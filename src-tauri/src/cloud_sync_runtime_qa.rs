use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::Duration;

use reqwest::header::{CONNECTION, CONTENT_TYPE};
use rusqlite::Connection;
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use crate::sync::storage;

const SCHEMA_VERSION: &str = "cloud_sync_desktop_installer_runtime_qa_v1";
const PRIVACY_BOUNDARY: &str =
    "metadata_only_no_original_media_no_protected_media_no_local_path_no_object_ref_no_signed_url";
const FORBIDDEN_PAYLOAD_KEYS: &[&str] = &[
    "originalPath",
    "original_path",
    "protectedCopyPath",
    "protected_copy_path",
    "localPath",
    "local_path",
    "objectRef",
    "object_ref",
    "signedUrl",
    "signed_url",
    "mediaBytes",
    "media_bytes",
];

pub fn run_from_env_if_requested() -> bool {
    let args = env::args().collect::<Vec<_>>();
    let explicit_arg = args
        .windows(2)
        .find(|pair| pair[0] == "--cloud-sync-runtime-qa")
        .map(|pair| pair[1].clone());
    let artifact_path = env::var("HIDDENSHIELD_DESKTOP_CLOUD_SYNC_QA_ARTIFACT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or(explicit_arg);
    let Some(artifact_path) = artifact_path else {
        return false;
    };

    let artifact_path = PathBuf::from(artifact_path);
    let backend_url = env::var("HIDDENSHIELD_CLOUD_SYNC_QA_BACKEND_URL")
        .or_else(|_| env::var("HIDDENSHIELD_CLOUD_URL"))
        .unwrap_or_else(|_| "http://127.0.0.1:43188".to_string());
    let run_id = env::var("HIDDENSHIELD_CLOUD_SYNC_RUNTIME_RUN_ID")
        .unwrap_or_else(|_| format!("{}", chrono::Utc::now().timestamp_millis()));
    let exe_path = env::current_exe()
        .ok()
        .map(|path| path.to_string_lossy().to_string());

    let result = run_desktop_cloud_sync_qa(&backend_url, &run_id, exe_path);
    let artifact = match result {
        Ok(artifact) => artifact,
        Err(error) => DesktopQaArtifact::blocked(&run_id, &backend_url, error),
    };
    if let Some(parent) = artifact_path.parent() {
        if let Err(error) = fs::create_dir_all(parent) {
            eprintln!("failed to create desktop QA artifact dir: {error}");
            process::exit(1);
        }
    }
    match serde_json::to_string_pretty(&artifact)
        .map(|text| format!("{text}\n"))
        .and_then(|text| fs::write(&artifact_path, text).map_err(serde_json::Error::io))
    {
        Ok(_) if artifact.ok => process::exit(0),
        Ok(_) => process::exit(2),
        Err(error) => {
            eprintln!("failed to write desktop QA artifact: {error}");
            process::exit(1);
        }
    }
}

fn run_desktop_cloud_sync_qa(
    backend_url: &str,
    run_id: &str,
    executable_path: Option<String>,
) -> Result<DesktopQaArtifact, String> {
    let client = QaHttpClient::new(backend_url)?;
    let started_at = chrono::Utc::now().to_rfc3339();
    client.health()?;

    let creator_identifier = format!("desktop-cloud-sync-{run_id}@hiddenshield.local");
    let creator_password = format!("qa-{run_id}");
    let desktop = client.create_session(
        &creator_identifier,
        &creator_password,
        &format!("desktop-installed-{run_id}"),
        "Desktop Installed Cloud Sync QA",
        "windows",
    )?;
    client.upgrade_to_creator(&desktop)?;
    let desktop = client.create_session(
        &creator_identifier,
        &creator_password,
        &format!("desktop-installed-{run_id}"),
        "Desktop Installed Cloud Sync QA",
        "windows",
    )?;
    let android_peer = client.create_session(
        &creator_identifier,
        &creator_password,
        &format!("android-peer-{run_id}"),
        "Android Peer Cloud Sync QA",
        "android",
    )?;

    let initial_pull = client.fetch_changes(&desktop, None)?;
    let desktop_event = metadata_event(run_id, "desktop", "image");
    assert_no_forbidden_payload_fields(
        desktop_event
            .get("payload")
            .ok_or_else(|| "desktop event missing payload".to_string())?,
    )?;
    let flush = client.push_events(&desktop, vec![desktop_event.clone()])?;
    let duplicate = client.push_events(&desktop, vec![desktop_event.clone()])?;
    let android_pull = client.fetch_changes(&android_peer, None)?;
    let desktop_entity_id = desktop_event
        .get("entityId")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let android_pulled_desktop = changes_contain_entity(&android_pull, &desktop_entity_id);

    let free_identifier = format!("desktop-cloud-sync-free-{run_id}@hiddenshield.local");
    let free = client.create_session(
        &free_identifier,
        &format!("free-{run_id}"),
        &format!("desktop-free-{run_id}"),
        "Desktop Free Cloud Sync QA",
        "windows",
    )?;
    let free_event = metadata_event(run_id, "desktop-free", "audio");
    assert_no_forbidden_payload_fields(
        free_event
            .get("payload")
            .ok_or_else(|| "free event missing payload".to_string())?,
    )?;
    let free_blocked = client.push_events_expect_status(&free, vec![free_event.clone()], 403)?;

    let queue = run_local_queue_diagnostics(run_id, &desktop_event, &free_event)?;
    let privacy = privacy_report(&[&desktop_event, &free_event]);
    let creator_ok = response_accepted(&flush, &desktop_event)
        && response_disposition(&duplicate, "duplicate")
        && android_pulled_desktop;
    let free_ok = free_blocked.status == 403
        && queue
            .get("freeAfterBlocked")
            .and_then(|value| value.get("lastErrorCode"))
            .and_then(Value::as_str)
            == Some("blocked_by_entitlement");
    let queue_ok = queue
        .get("creatorAfterFlush")
        .and_then(|value| value.get("synced"))
        .and_then(Value::as_u64)
        == Some(1)
        && queue.get("recoveredStale").and_then(Value::as_u64) == Some(1);
    let privacy_ok = privacy
        .get("forbiddenKeysPresent")
        .and_then(Value::as_array)
        .map(|items| items.is_empty())
        .unwrap_or(false);
    let ok = creator_ok && free_ok && queue_ok && privacy_ok;

    Ok(DesktopQaArtifact {
        schema_version: SCHEMA_VERSION.to_string(),
        run_id: run_id.to_string(),
        generated_at: chrono::Utc::now().to_rfc3339(),
        started_at,
        completed_at: Some(chrono::Utc::now().to_rfc3339()),
        ok,
        status: if ok { "ready" } else { "blocked" }.to_string(),
        backend_base_url: backend_url.to_string(),
        executable_path,
        completed_checks: json!({
            "feedbackBackendHealthy": true,
            "creatorInitialPull": initial_pull.get("changes").and_then(Value::as_array).is_some(),
            "creatorFlushAccepted": response_accepted(&flush, &desktop_event),
            "creatorDuplicateNotRetransmitted": response_disposition(&duplicate, "duplicate"),
            "creatorPeerPullReceived": android_pulled_desktop,
            "freeBlockedByEntitlement": free_ok,
            "queueDiagnosticsExported": queue_ok,
            "privacyWhitelistEnforced": privacy_ok
        }),
        creator_pull_flush_pull: json!({
            "initialPull": summarize_pull(&initial_pull),
            "flush": summarize_push(&flush),
            "duplicateFlush": summarize_push(&duplicate),
            "peerPull": summarize_pull(&android_pull),
            "peerPulledEntityId": desktop_entity_id,
            "peerPulledEntity": android_pulled_desktop
        }),
        free_blocked_by_entitlement: json!({
            "status": free_blocked.status,
            "body": free_blocked.body,
            "blocked": free_blocked.status == 403
        }),
        queue_diagnostics: queue,
        privacy: privacy,
        missing_checks: if ok {
            Vec::new()
        } else {
            vec![
                "Desktop installed automation channel did not satisfy all required sync assertions"
                    .to_string(),
            ]
        },
        privacy_boundary: PRIVACY_BOUNDARY.to_string(),
    })
}

fn run_local_queue_diagnostics(
    run_id: &str,
    desktop_event: &Value,
    free_event: &Value,
) -> Result<Value, String> {
    let db_path = env::temp_dir().join(format!(
        "hiddenshield-desktop-cloud-sync-qa-{run_id}.sqlite"
    ));
    let _ = fs::remove_file(&db_path);
    let conn = Connection::open(&db_path).map_err(|error| format!("open QA queue db: {error}"))?;
    storage::init_sync_storage(&conn).map_err(|error| format!("init QA queue db: {error}"))?;
    storage::enqueue_cloud_sync_event(
        &conn,
        "desktop-creator-queue",
        1,
        &desktop_event.to_string(),
    )
    .map_err(|error| format!("enqueue creator queue: {error}"))?;
    storage::mark_cloud_sync_queue_syncing(&conn, &["desktop-creator-queue".to_string()])
        .map_err(|error| format!("mark creator syncing: {error}"))?;
    storage::mark_cloud_sync_queue_synced(&conn, &["desktop-creator-queue".to_string()])
        .map_err(|error| format!("mark creator synced: {error}"))?;
    let creator_after_flush = queue_status(&conn)?;

    storage::enqueue_cloud_sync_event(&conn, "desktop-free-queue", 2, &free_event.to_string())
        .map_err(|error| format!("enqueue free queue: {error}"))?;
    storage::mark_uploadable_cloud_sync_queue_blocked_by_entitlement(
        &conn,
        "正式云同步从 Creator 开放，当前账户以后端权益快照为准，已阻断本次上传",
    )
    .map_err(|error| format!("mark free blocked: {error}"))?;
    let free_after_blocked = json!({
        "blocked": storage::count_cloud_sync_queue_by_status(&conn, "blocked").map_err(|error| error.to_string())?,
        "lastErrorCode": storage::latest_cloud_sync_queue_error_code(&conn).map_err(|error| error.to_string())?,
        "lastHttpStatus": storage::latest_cloud_sync_queue_http_status(&conn).map_err(|error| error.to_string())?,
        "blockedReason": storage::latest_cloud_sync_queue_blocked_reason(&conn).map_err(|error| error.to_string())?
    });

    storage::enqueue_cloud_sync_event(&conn, "desktop-stale-queue", 3, &desktop_event.to_string())
        .map_err(|error| format!("enqueue stale queue: {error}"))?;
    storage::mark_cloud_sync_queue_syncing(&conn, &["desktop-stale-queue".to_string()])
        .map_err(|error| format!("mark stale syncing: {error}"))?;
    let stale_at = (chrono::Utc::now() - chrono::Duration::minutes(20)).to_rfc3339();
    conn.execute(
        "UPDATE cloud_sync_queue SET lease_until = ?1, updated_at = ?1 WHERE id = 'desktop-stale-queue'",
        [stale_at],
    )
    .map_err(|error| format!("age stale queue: {error}"))?;
    let recovered = storage::recover_stale_cloud_syncing_queue(
        &conn,
        chrono::Utc::now() - chrono::Duration::minutes(10),
    )
    .map_err(|error| format!("recover stale queue: {error}"))?;

    Ok(json!({
        "databasePath": db_path.to_string_lossy(),
        "creatorAfterFlush": creator_after_flush,
        "freeAfterBlocked": free_after_blocked,
        "recoveredStale": recovered,
        "afterRecovery": queue_status(&conn)?
    }))
}

fn queue_status(conn: &Connection) -> Result<Value, String> {
    Ok(json!({
        "pending": storage::count_cloud_sync_queue_by_status(conn, "pending").map_err(|error| error.to_string())?,
        "syncing": storage::count_cloud_sync_queue_by_status(conn, "syncing").map_err(|error| error.to_string())?,
        "failed": storage::count_cloud_sync_queue_by_status(conn, "failed").map_err(|error| error.to_string())?,
        "blocked": storage::count_cloud_sync_queue_by_status(conn, "blocked").map_err(|error| error.to_string())?,
        "synced": storage::count_cloud_sync_queue_by_status(conn, "synced").map_err(|error| error.to_string())?,
        "lastErrorCode": storage::latest_cloud_sync_queue_error_code(conn).map_err(|error| error.to_string())?,
        "lastHttpStatus": storage::latest_cloud_sync_queue_http_status(conn).map_err(|error| error.to_string())?,
        "blockedReason": storage::latest_cloud_sync_queue_blocked_reason(conn).map_err(|error| error.to_string())?
    }))
}

fn metadata_event(run_id: &str, source: &str, kind: &str) -> Value {
    let entity_id = format!("{source}-record-{run_id}");
    json!({
        "clientEventId": format!("{source}-event-{run_id}"),
        "operation": "upsertVaultRecord",
        "entityType": "vaultRecord",
        "entityId": entity_id,
        "payload": {
            "id": entity_id,
            "kind": kind,
            "title": format!("{source}-{kind}-{run_id}"),
            "watermark_uid": long_uid(run_id, source),
            "revision": 1,
            "creator_display_name": format!("{source} QA Creator"),
            "sha256": format!("sha256:{}", digest_hex(&format!("{run_id}:{source}:{kind}"))),
            "created_at": chrono::Utc::now().to_rfc3339(),
            "payload_protocol_version": 3,
            "payload_bytes_length": 39,
            "media_payload_role": "v3_minimal_anchor",
            "watermark_id_issue_mode": "server_confirmed",
            "watermark_id_registry_status": "server_confirmed",
            "payload_auth_status": "verified",
            "protected_copy_name": format!("{source}-{kind}-protected"),
            "protected_copy_hash": format!("sha256:{}", digest_hex(&format!("protected:{run_id}:{source}:{kind}"))),
            "source": "desktop_installed_cloud_sync_qa",
            "sync_status": "pending"
        }
    })
}

fn long_uid(run_id: &str, source: &str) -> String {
    let digest = digest_hex(&format!("uid:{run_id}:{source}")).to_uppercase();
    format!(
        "HS-{}-{}-{}-{}",
        &digest[0..8],
        &digest[8..16],
        &digest[16..24],
        &digest[24..32]
    )
}

fn digest_hex(value: &str) -> String {
    hex::encode(Sha256::digest(value.as_bytes()))
}

fn assert_no_forbidden_payload_fields(payload: &Value) -> Result<(), String> {
    let object = payload
        .as_object()
        .ok_or_else(|| "payload is not an object".to_string())?;
    let present = FORBIDDEN_PAYLOAD_KEYS
        .iter()
        .filter(|key| object.contains_key(**key))
        .copied()
        .collect::<Vec<_>>();
    if present.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "forbidden payload keys present: {}",
            present.join(", ")
        ))
    }
}

fn privacy_report(events: &[&Value]) -> Value {
    let mut present = Vec::new();
    let mut payload_keys = Vec::new();
    for event in events {
        if let Some(payload) = event.get("payload").and_then(Value::as_object) {
            payload_keys.extend(payload.keys().cloned());
            for key in FORBIDDEN_PAYLOAD_KEYS {
                if payload.contains_key(*key) {
                    present.push((*key).to_string());
                }
            }
        }
    }
    payload_keys.sort();
    payload_keys.dedup();
    present.sort();
    present.dedup();
    json!({
        "forbiddenKeysPresent": present,
        "payloadKeys": payload_keys,
        "privacyBoundary": PRIVACY_BOUNDARY
    })
}

fn response_accepted(response: &Value, event: &Value) -> bool {
    let Some(event_id) = event.get("clientEventId").and_then(Value::as_str) else {
        return false;
    };
    response
        .get("acceptedEventIds")
        .and_then(Value::as_array)
        .map(|ids| ids.iter().any(|value| value.as_str() == Some(event_id)))
        .unwrap_or(false)
        || response
            .get("eventResults")
            .and_then(Value::as_array)
            .map(|items| {
                items.iter().any(|item| {
                    item.get("clientEventId").and_then(Value::as_str) == Some(event_id)
                        && item.get("disposition").and_then(Value::as_str) == Some("accepted")
                })
            })
            .unwrap_or(false)
}

fn response_disposition(response: &Value, disposition: &str) -> bool {
    response
        .get("eventResults")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .any(|item| item.get("disposition").and_then(Value::as_str) == Some(disposition))
        })
        .unwrap_or(false)
}

fn changes_contain_entity(response: &Value, entity_id: &str) -> bool {
    response
        .get("changes")
        .and_then(Value::as_array)
        .map(|changes| {
            changes.iter().any(|change| {
                change
                    .get("entity")
                    .and_then(|entity| entity.get("id"))
                    .and_then(Value::as_str)
                    == Some(entity_id)
            })
        })
        .unwrap_or(false)
}

fn summarize_push(response: &Value) -> Value {
    json!({
        "accepted": response.get("accepted").cloned().unwrap_or(Value::Null),
        "acceptedEventIds": response.get("acceptedEventIds").cloned().unwrap_or(Value::Null),
        "eventResults": response.get("eventResults").cloned().unwrap_or(Value::Null),
        "nextCursor": response.get("nextCursor").cloned().unwrap_or(Value::Null)
    })
}

fn summarize_pull(response: &Value) -> Value {
    json!({
        "nextCursor": response.get("nextCursor").cloned().unwrap_or(Value::Null),
        "changeCount": response.get("changes").and_then(Value::as_array).map(|items| items.len()).unwrap_or(0)
    })
}

#[derive(Debug)]
struct QaSession {
    access_token: String,
    account_id: String,
    workspace_id: String,
    device_id: String,
}

#[derive(Debug)]
struct QaHttpResponse {
    status: u16,
    body: Value,
}

struct QaHttpClient {
    base_url: String,
    http: reqwest::blocking::Client,
}

impl QaHttpClient {
    fn new(base_url: &str) -> Result<Self, String> {
        let base_url = base_url.trim().trim_end_matches('/').to_string();
        if base_url.is_empty() {
            return Err("backend url is empty".to_string());
        }
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .pool_max_idle_per_host(0)
            .build()
            .map_err(|error| format!("build HTTP client: {error}"))?;
        Ok(Self { base_url, http })
    }

    fn health(&self) -> Result<(), String> {
        let response = self
            .http
            .get(format!("{}/v1/health", self.base_url))
            .header(CONNECTION, "close")
            .send()
            .map_err(|error| format!("health request failed: {error}"))?;
        if !response.status().is_success() {
            return Err(format!("health failed: HTTP {}", response.status()));
        }
        let body = parse_response(response)?.body;
        if body.get("ok").and_then(Value::as_bool) != Some(true)
            || body.get("cloudSync").and_then(Value::as_bool) != Some(true)
        {
            return Err(format!("unexpected health body: {body}"));
        }
        Ok(())
    }

    fn create_session(
        &self,
        identifier: &str,
        password: &str,
        client_device_id: &str,
        name: &str,
        platform: &str,
    ) -> Result<QaSession, String> {
        let body = json!({
            "identifier": identifier,
            "password": password,
            "verificationCode": "000000",
            "device": {
                "clientDeviceId": client_device_id,
                "name": name,
                "platform": platform,
                "appVersion": "desktop-installed-cloud-sync-runtime-qa"
            },
            "localCreatorProfile": {
                "displayName": "Desktop Installed Cloud Sync QA",
                "creatorSeedRef": format!("qa-seed-{identifier}"),
                "seedEnvelopeVersion": 1
            }
        });
        let response = self.post("/v1/auth/sessions", None, &body)?;
        if response.status != 200 {
            return Err(format!(
                "create session failed: HTTP {} {}",
                response.status, response.body
            ));
        }
        Ok(QaSession {
            access_token: string_at(&response.body, &["accessToken"])?,
            account_id: string_at(&response.body, &["account", "id"])?,
            workspace_id: string_at(&response.body, &["workspace", "id"])?,
            device_id: string_at(&response.body, &["device", "id"])?,
        })
    }

    fn upgrade_to_creator(&self, session: &QaSession) -> Result<(), String> {
        let payment = self.post(
            "/v1/billing/payment-sessions",
            Some(&session.access_token),
            &json!({
                "accountId": session.account_id,
                "workspaceId": session.workspace_id,
                "planCode": "creator",
                "billingCycle": "monthly",
                "preferredProvider": "fixture"
            }),
        )?;
        if payment.status != 200 {
            return Err(format!(
                "fixture payment failed: HTTP {} {}",
                payment.status, payment.body
            ));
        }
        let payment_session_id = string_at(&payment.body, &["paymentSessionId"])?;
        let reconcile = self.post(
            &format!("/v1/billing/payment-sessions/{payment_session_id}/reconcile"),
            Some(&session.access_token),
            &json!({}),
        )?;
        if reconcile.status != 200 {
            return Err(format!(
                "fixture reconcile failed: HTTP {} {}",
                reconcile.status, reconcile.body
            ));
        }
        Ok(())
    }

    fn push_events(&self, session: &QaSession, events: Vec<Value>) -> Result<Value, String> {
        let response = self.post(
            "/v1/sync/events:batch",
            Some(&session.access_token),
            &json!({
                "deviceId": session.device_id,
                "workspaceId": session.workspace_id,
                "events": events
            }),
        )?;
        if response.status != 200 {
            return Err(format!(
                "push events failed: HTTP {} {}",
                response.status, response.body
            ));
        }
        Ok(response.body)
    }

    fn push_events_expect_status(
        &self,
        session: &QaSession,
        events: Vec<Value>,
        status: u16,
    ) -> Result<QaHttpResponse, String> {
        let response = self.post(
            "/v1/sync/events:batch",
            Some(&session.access_token),
            &json!({
                "deviceId": session.device_id,
                "workspaceId": session.workspace_id,
                "events": events
            }),
        )?;
        if response.status != status {
            return Err(format!(
                "expected HTTP {status}, got HTTP {} {}",
                response.status, response.body
            ));
        }
        Ok(response)
    }

    fn fetch_changes(&self, session: &QaSession, cursor: Option<&str>) -> Result<Value, String> {
        let mut path = format!("/v1/sync/changes?workspaceId={}", session.workspace_id);
        if let Some(cursor) = cursor.filter(|value| !value.trim().is_empty()) {
            path.push_str("&cursor=");
            path.push_str(cursor);
        }
        let response = self.get(&path, &session.access_token)?;
        if response.status != 200 {
            return Err(format!(
                "fetch changes failed: HTTP {} {}",
                response.status, response.body
            ));
        }
        Ok(response.body)
    }

    fn get(&self, path: &str, token: &str) -> Result<QaHttpResponse, String> {
        let response = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(token)
            .header(CONNECTION, "close")
            .send()
            .map_err(|error| format!("GET {path} failed: {error}"))?;
        parse_response(response)
    }

    fn post(
        &self,
        path: &str,
        token: Option<&str>,
        body: &Value,
    ) -> Result<QaHttpResponse, String> {
        let mut request = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .header(CONTENT_TYPE, "application/json")
            .header(CONNECTION, "close")
            .body(body.to_string());
        if let Some(token) = token {
            request = request.bearer_auth(token);
        }
        let response = request
            .send()
            .map_err(|error| format!("POST {path} failed: {error}"))?;
        parse_response(response)
    }
}

fn parse_response(response: reqwest::blocking::Response) -> Result<QaHttpResponse, String> {
    let status = response.status().as_u16();
    let text = response
        .text()
        .map_err(|error| format!("read HTTP response failed: {error}"))?;
    let body = if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).unwrap_or_else(|_| json!({ "raw": text }))
    };
    Ok(QaHttpResponse { status, body })
}

fn string_at(value: &Value, path: &[&str]) -> Result<String, String> {
    let mut current = value;
    for key in path {
        current = current
            .get(*key)
            .ok_or_else(|| format!("missing JSON key {}", path.join(".")))?;
    }
    current
        .as_str()
        .map(ToString::to_string)
        .ok_or_else(|| format!("JSON key {} is not a string", path.join(".")))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DesktopQaArtifact {
    #[serde(rename = "schemaVersion")]
    schema_version: String,
    run_id: String,
    generated_at: String,
    started_at: String,
    completed_at: Option<String>,
    ok: bool,
    status: String,
    backend_base_url: String,
    executable_path: Option<String>,
    completed_checks: Value,
    creator_pull_flush_pull: Value,
    free_blocked_by_entitlement: Value,
    queue_diagnostics: Value,
    privacy: Value,
    missing_checks: Vec<String>,
    privacy_boundary: String,
}

impl DesktopQaArtifact {
    fn blocked(run_id: &str, backend_url: &str, error: String) -> Self {
        Self {
            schema_version: SCHEMA_VERSION.to_string(),
            run_id: run_id.to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: Some(chrono::Utc::now().to_rfc3339()),
            ok: false,
            status: "blocked".to_string(),
            backend_base_url: backend_url.to_string(),
            executable_path: env::current_exe()
                .ok()
                .map(|path| path.to_string_lossy().to_string()),
            completed_checks: json!({ "error": error }),
            creator_pull_flush_pull: Value::Null,
            free_blocked_by_entitlement: Value::Null,
            queue_diagnostics: Value::Null,
            privacy: json!({ "privacyBoundary": PRIVACY_BOUNDARY }),
            missing_checks: vec![error],
            privacy_boundary: PRIVACY_BOUNDARY.to_string(),
        }
    }
}

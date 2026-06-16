use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::commands::vault::VaultRecord;
use crate::db::queries;

#[derive(Debug, Clone, Deserialize)]
pub struct MobileSyncQueueItem {
    #[serde(rename = "queueId")]
    pub queue_id: String,
    #[serde(rename = "recordId")]
    pub record_id: String,
    pub operation: String,
    #[serde(rename = "payloadType")]
    pub payload_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MobileSyncBatchRequest {
    pub items: Vec<MobileSyncQueueItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncQueueItem {
    pub id: String,
    pub record_id: u32,
    pub event_json: String,
    pub status: String,
    pub attempts: u32,
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

pub fn init_sync_storage(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
CREATE TABLE IF NOT EXISTS sync_events (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  received_at TEXT NOT NULL,
  queue_id TEXT NOT NULL,
  record_id TEXT NOT NULL,
  operation TEXT NOT NULL,
  payload_type TEXT NOT NULL,
  payload_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sync_events_received_at
ON sync_events(received_at DESC);

CREATE TABLE IF NOT EXISTS sync_evidence_records (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  mobile_record_id TEXT NOT NULL UNIQUE,
  received_at TEXT NOT NULL,
  suspect_file_name TEXT NOT NULL,
  asset_kind TEXT NOT NULL,
  watermark_uid TEXT NOT NULL,
  revision INTEGER NOT NULL,
  parent_watermark_uid TEXT,
  rewrite_reason TEXT,
  extracted_timestamp INTEGER,
  extracted_device_id_hex TEXT,
  extracted_file_hash_hex TEXT,
  payload_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sync_evidence_watermark
ON sync_evidence_records(watermark_uid);

CREATE INDEX IF NOT EXISTS idx_sync_evidence_received_at
ON sync_evidence_records(received_at DESC);

CREATE TABLE IF NOT EXISTS sync_resolutions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  resolved_at TEXT NOT NULL,
  queue_id TEXT NOT NULL,
  mobile_record_id TEXT NOT NULL,
  resolution_type TEXT NOT NULL,
  reason TEXT NOT NULL,
  watermark_uid TEXT NOT NULL,
  desktop_record_id INTEGER,
  desktop_hash TEXT,
  mobile_hash TEXT,
  desktop_revision INTEGER,
  mobile_revision INTEGER,
  inserted_record_id INTEGER,
  payload_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sync_resolutions_resolved_at
ON sync_resolutions(resolved_at DESC);

CREATE INDEX IF NOT EXISTS idx_sync_resolutions_watermark
ON sync_resolutions(watermark_uid);

CREATE TABLE IF NOT EXISTS cloud_sync_queue (
  id TEXT PRIMARY KEY,
  record_id INTEGER NOT NULL,
  event_json TEXT NOT NULL,
  status TEXT NOT NULL,
  attempts INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_cloud_sync_queue_status_created
ON cloud_sync_queue(status, created_at ASC);
",
    )?;
    Ok(())
}

pub fn enqueue_cloud_sync_event(
    conn: &Connection,
    queue_id: &str,
    record_id: u32,
    event_json: &str,
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    conn.execute(
        "
INSERT INTO cloud_sync_queue (
  id, record_id, event_json, status, attempts, last_error, created_at, updated_at
) VALUES (?1, ?2, ?3, 'pending', 0, NULL, ?4, ?4)
ON CONFLICT(id) DO UPDATE SET
  event_json = excluded.event_json,
  status = CASE
    WHEN cloud_sync_queue.status = 'synced' THEN 'synced'
    ELSE 'pending'
  END,
  updated_at = excluded.updated_at
",
        params![queue_id, record_id as i64, event_json, now],
    )?;
    Ok(())
}

pub fn list_pending_cloud_sync_queue(
    conn: &Connection,
    limit: usize,
) -> Result<Vec<CloudSyncQueueItem>, rusqlite::Error> {
    let mut stmt = conn.prepare(
        "
SELECT id, record_id, event_json, status, attempts, last_error, created_at, updated_at
FROM cloud_sync_queue
WHERE status IN ('pending', 'failed')
ORDER BY created_at ASC
LIMIT ?1
",
    )?;
    let rows = stmt.query_map(params![limit as i64], |row| {
        Ok(CloudSyncQueueItem {
            id: row.get(0)?,
            record_id: row.get::<_, i64>(1)? as u32,
            event_json: row.get(2)?,
            status: row.get(3)?,
            attempts: row.get::<_, i64>(4)? as u32,
            last_error: row.get(5)?,
            created_at: row.get(6)?,
            updated_at: row.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn mark_cloud_sync_queue_syncing(
    conn: &Connection,
    queue_ids: &[String],
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    for queue_id in queue_ids {
        conn.execute(
            "
UPDATE cloud_sync_queue
SET status = 'syncing',
    attempts = attempts + 1,
    last_error = NULL,
    updated_at = ?1
WHERE id = ?2
",
            params![now, queue_id],
        )?;
    }
    Ok(())
}

pub fn mark_cloud_sync_queue_synced(
    conn: &Connection,
    queue_ids: &[String],
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    for queue_id in queue_ids {
        conn.execute(
            "
UPDATE cloud_sync_queue
SET status = 'synced',
    last_error = NULL,
    updated_at = ?1
WHERE id = ?2
",
            params![now, queue_id],
        )?;
    }
    Ok(())
}

pub fn mark_cloud_sync_queue_failed(
    conn: &Connection,
    queue_ids: &[String],
    error: &str,
) -> Result<(), rusqlite::Error> {
    let now = chrono::Utc::now().to_rfc3339();
    for queue_id in queue_ids {
        conn.execute(
            "
UPDATE cloud_sync_queue
SET status = 'failed',
    last_error = ?1,
    updated_at = ?2
WHERE id = ?3
",
            params![error, now, queue_id],
        )?;
    }
    Ok(())
}

pub fn count_cloud_sync_queue_by_status(
    conn: &Connection,
    status: &str,
) -> Result<u64, rusqlite::Error> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM cloud_sync_queue WHERE status = ?1",
        params![status],
        |row| row.get(0),
    )?;
    Ok(count.max(0) as u64)
}

pub fn record_sync_event(
    conn: &Connection,
    item: &MobileSyncQueueItem,
) -> Result<i64, rusqlite::Error> {
    let tx = conn.unchecked_transaction()?;
    let payload_json = serde_json::to_string(&item.payload).unwrap_or_else(|_| "{}".to_string());
    tx.execute(
        "
INSERT INTO sync_events (
  received_at, queue_id, record_id, operation, payload_type, payload_json
) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
",
        params![
            chrono::Utc::now().to_rfc3339(),
            item.queue_id,
            item.record_id,
            item.operation,
            item.payload_type,
            payload_json,
        ],
    )?;
    let event_id = tx.last_insert_rowid();
    apply_queue_item(&tx, item)?;
    tx.commit()?;
    Ok(event_id)
}

fn apply_queue_item(
    tx: &rusqlite::Transaction<'_>,
    item: &MobileSyncQueueItem,
) -> Result<(), rusqlite::Error> {
    match (item.operation.as_str(), item.payload_type.as_str()) {
        ("upsertVaultRecord", "vault_record") => apply_vault_record(tx, item),
        ("upsertEvidenceRecord", "vault_record") | ("upsertEvidenceRecord", "evidence_record") => {
            apply_evidence_record(tx, item)
        }
        _ => Ok(()),
    }
}

fn apply_vault_record(
    tx: &rusqlite::Transaction<'_>,
    item: &MobileSyncQueueItem,
) -> Result<(), rusqlite::Error> {
    let record = mobile_payload_to_vault_record(item).map_err(|message| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )))
    })?;

    if let Some(existing) =
        find_vault_record_by_uid_hash(tx, &record.watermark_uid, &record.original_hash)?
    {
        if existing.revision == record.revision {
            return Ok(());
        }
        resolve_same_asset_revision(tx, item, &record, &existing)?;
        return Ok(());
    }

    if let Some(existing) = find_latest_vault_record_by_uid(tx, &record.watermark_uid)? {
        let inserted_id = queries::insert_record_tx(tx, &record)?;
        record_sync_resolution(
            tx,
            item,
            &record,
            Some(&existing),
            "variant_accepted",
            "same_watermark_uid_with_distinct_file_hash_was_accepted_as_asset_variant",
            Some(inserted_id),
        )?;
        return Ok(());
    }

    let inserted_id = queries::insert_record_tx(tx, &record)?;
    record_sync_resolution(
        tx,
        item,
        &record,
        None,
        "record_inserted",
        "new_mobile_vault_record_was_inserted",
        Some(inserted_id),
    )?;
    Ok(())
}

struct ExistingVaultRecord {
    id: i64,
    original_hash: String,
    revision: u32,
}

fn apply_evidence_record(
    tx: &rusqlite::Transaction<'_>,
    item: &MobileSyncQueueItem,
) -> Result<(), rusqlite::Error> {
    let evidence = mobile_payload_to_evidence_record(item).map_err(|message| {
        rusqlite::Error::ToSqlConversionFailure(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )))
    })?;
    let payload_json = serde_json::to_string(&item.payload).unwrap_or_else(|_| "{}".to_string());

    tx.execute(
        "
INSERT INTO sync_evidence_records (
  mobile_record_id, received_at, suspect_file_name, asset_kind,
  watermark_uid, revision, parent_watermark_uid, rewrite_reason,
  extracted_timestamp, extracted_device_id_hex, extracted_file_hash_hex,
  payload_json
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
ON CONFLICT(mobile_record_id) DO UPDATE SET
  received_at = excluded.received_at,
  suspect_file_name = excluded.suspect_file_name,
  asset_kind = excluded.asset_kind,
  watermark_uid = excluded.watermark_uid,
  revision = excluded.revision,
  parent_watermark_uid = excluded.parent_watermark_uid,
  rewrite_reason = excluded.rewrite_reason,
  extracted_timestamp = excluded.extracted_timestamp,
  extracted_device_id_hex = excluded.extracted_device_id_hex,
  extracted_file_hash_hex = excluded.extracted_file_hash_hex,
  payload_json = excluded.payload_json
",
        params![
            evidence.mobile_record_id,
            chrono::Utc::now().to_rfc3339(),
            evidence.suspect_file_name,
            evidence.asset_kind,
            evidence.watermark_uid,
            evidence.revision as i64,
            evidence.parent_watermark_uid,
            evidence.rewrite_reason,
            evidence.extracted_timestamp.map(|value| value as i64),
            evidence.extracted_device_id_hex,
            evidence.extracted_file_hash_hex,
            payload_json,
        ],
    )?;
    Ok(())
}

struct MobileEvidenceRecord {
    mobile_record_id: String,
    suspect_file_name: String,
    asset_kind: String,
    watermark_uid: String,
    revision: u32,
    parent_watermark_uid: Option<String>,
    rewrite_reason: Option<String>,
    extracted_timestamp: Option<u64>,
    extracted_device_id_hex: Option<String>,
    extracted_file_hash_hex: Option<String>,
}

fn find_vault_record_by_uid_hash(
    tx: &rusqlite::Transaction<'_>,
    watermark_uid: &str,
    original_hash: &str,
) -> Result<Option<ExistingVaultRecord>, rusqlite::Error> {
    tx.query_row(
        "SELECT id, original_hash, revision FROM vault_records
         WHERE watermark_uid = ?1 AND original_hash = ?2
         ORDER BY revision DESC, created_at DESC, id DESC
         LIMIT 1",
        params![watermark_uid, original_hash],
        |row| {
            Ok(ExistingVaultRecord {
                id: row.get(0)?,
                original_hash: row.get(1)?,
                revision: row.get::<_, i64>(2)? as u32,
            })
        },
    )
    .map(Some)
    .or_else(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
}

fn find_latest_vault_record_by_uid(
    tx: &rusqlite::Transaction<'_>,
    watermark_uid: &str,
) -> Result<Option<ExistingVaultRecord>, rusqlite::Error> {
    tx.query_row(
        "SELECT id, original_hash, revision FROM vault_records
         WHERE watermark_uid = ?1
         ORDER BY created_at DESC, id DESC
         LIMIT 1",
        params![watermark_uid],
        |row| {
            Ok(ExistingVaultRecord {
                id: row.get(0)?,
                original_hash: row.get(1)?,
                revision: row.get::<_, i64>(2)? as u32,
            })
        },
    )
    .map(Some)
    .or_else(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
}

fn resolve_same_asset_revision(
    tx: &rusqlite::Transaction<'_>,
    item: &MobileSyncQueueItem,
    mobile_record: &VaultRecord,
    existing: &ExistingVaultRecord,
) -> Result<(), rusqlite::Error> {
    if mobile_record.revision > existing.revision {
        tx.execute(
            "UPDATE vault_records
             SET revision = ?1,
                 parent_watermark_uid = COALESCE(?2, parent_watermark_uid),
                 rewrite_reason = COALESCE(?3, rewrite_reason)
             WHERE id = ?4",
            params![
                mobile_record.revision as i64,
                mobile_record.parent_watermark_uid,
                mobile_record.rewrite_reason,
                existing.id,
            ],
        )?;
        record_sync_resolution(
            tx,
            item,
            mobile_record,
            Some(existing),
            "revision_upgraded",
            "same_asset_hash_received_with_higher_mobile_revision",
            None,
        )?;
    } else {
        record_sync_resolution(
            tx,
            item,
            mobile_record,
            Some(existing),
            "stale_revision_ignored",
            "same_asset_hash_received_with_older_mobile_revision",
            None,
        )?;
    }
    Ok(())
}

fn record_sync_resolution(
    tx: &rusqlite::Transaction<'_>,
    item: &MobileSyncQueueItem,
    mobile_record: &VaultRecord,
    existing: Option<&ExistingVaultRecord>,
    resolution_type: &str,
    reason: &str,
    inserted_record_id: Option<i64>,
) -> Result<(), rusqlite::Error> {
    let payload_json = serde_json::to_string(&item.payload).unwrap_or_else(|_| "{}".to_string());
    tx.execute(
        "
INSERT INTO sync_resolutions (
  resolved_at, queue_id, mobile_record_id, resolution_type, reason,
  watermark_uid, desktop_record_id, desktop_hash, mobile_hash,
  desktop_revision, mobile_revision, inserted_record_id, payload_json
) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
",
        params![
            chrono::Utc::now().to_rfc3339(),
            item.queue_id,
            item.record_id,
            resolution_type,
            reason,
            mobile_record.watermark_uid,
            existing.map(|record| record.id),
            existing.map(|record| record.original_hash.as_str()),
            mobile_record.original_hash,
            existing.map(|record| record.revision as i64),
            mobile_record.revision as i64,
            inserted_record_id,
            payload_json,
        ],
    )?;
    Ok(())
}

fn mobile_payload_to_vault_record(item: &MobileSyncQueueItem) -> Result<VaultRecord, String> {
    let payload = item
        .payload
        .as_object()
        .ok_or_else(|| "payload must be an object".to_string())?;

    let watermark_uid = required_string(payload, "watermark_uid")?;
    let file_name = optional_string(payload, "title")
        .or_else(|| optional_string(payload, "file_name"))
        .unwrap_or_else(|| format!("mobile-{}", item.record_id));
    let original_hash = optional_string(payload, "sha256")
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| synthetic_hash(&item.record_id, &watermark_uid));
    let created_at =
        optional_string(payload, "created_at").unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let revision = payload
        .get("revision")
        .and_then(|value| value.as_u64())
        .unwrap_or(1)
        .clamp(1, u32::MAX as u64) as u32;

    let kind = optional_string(payload, "kind").unwrap_or_else(|| "unknown".to_string());
    let source = optional_string(payload, "source").unwrap_or_else(|| "mobile".to_string());
    let sync_status =
        optional_string(payload, "sync_status").unwrap_or_else(|| "pending".to_string());
    let custom_metadata = serde_json::json!({
        "mobileRecordId": item.record_id,
        "mobileQueueId": item.queue_id,
        "mobileKind": kind,
        "mobileSource": source,
        "mobileSyncStatus": sync_status
    })
    .to_string();

    Ok(VaultRecord {
        id: 0,
        original_hash,
        file_name,
        created_at,
        duration_secs: 0.0,
        resolution: kind,
        watermark_uid,
        thumbnail_path: None,
        output_douyin: None,
        output_bilibili: None,
        output_xhs: None,
        is_hdr_source: false,
        hw_encoder_used: None,
        process_time_ms: None,
        tsa_token_path: None,
        network_time: None,
        tsa_source: None,
        tsa_request_nonce: None,
        is_ai_generated: false,
        ai_training_permission: None,
        ai_generation_method: None,
        human_modification_level: None,
        authenticity_claim: None,
        custom_metadata: Some(custom_metadata),
        output_douyin_hash: None,
        output_bilibili_hash: None,
        output_xhs_hash: None,
        parent_watermark_uid: optional_string(payload, "parent_watermark_uid"),
        revision,
        rewrite_reason: optional_string(payload, "rewrite_reason"),
    })
}

fn mobile_payload_to_evidence_record(
    item: &MobileSyncQueueItem,
) -> Result<MobileEvidenceRecord, String> {
    let payload = item
        .payload
        .as_object()
        .ok_or_else(|| "payload must be an object".to_string())?;
    let watermark_uid = required_string(payload, "watermark_uid")?;
    let mobile_record_id = optional_string(payload, "id").unwrap_or_else(|| item.record_id.clone());
    let suspect_file_name = optional_string(payload, "title")
        .or_else(|| optional_string(payload, "file_name"))
        .unwrap_or_else(|| format!("mobile-evidence-{}", item.record_id));
    let asset_kind = optional_string(payload, "kind").unwrap_or_else(|| "unknown".to_string());
    let revision = payload
        .get("revision")
        .and_then(|value| value.as_u64())
        .unwrap_or(1)
        .clamp(1, u32::MAX as u64) as u32;

    Ok(MobileEvidenceRecord {
        mobile_record_id,
        suspect_file_name,
        asset_kind,
        watermark_uid,
        revision,
        parent_watermark_uid: optional_string(payload, "parent_watermark_uid"),
        rewrite_reason: optional_string(payload, "rewrite_reason"),
        extracted_timestamp: optional_u64(payload, "extracted_timestamp"),
        extracted_device_id_hex: optional_string(payload, "extracted_device_id_hex"),
        extracted_file_hash_hex: optional_string(payload, "extracted_file_hash_hex"),
    })
}

fn required_string(
    payload: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Result<String, String> {
    optional_string(payload, key).ok_or_else(|| format!("missing required field: {key}"))
}

fn optional_string(
    payload: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn optional_u64(payload: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<u64> {
    payload.get(key).and_then(|value| value.as_u64())
}

fn synthetic_hash(record_id: &str, watermark_uid: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"mobile-sync:");
    hasher.update(record_id.as_bytes());
    hasher.update(b":");
    hasher.update(watermark_uid.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn build_health_response() -> String {
    serde_json::json!({
        "ok": true,
        "desktopDeviceId": hostname::get()
            .ok()
            .and_then(|name| name.into_string().ok())
            .unwrap_or_else(|| "desktop".to_string()),
        "protocolVersion": 1
    })
    .to_string()
}

pub fn build_queue_item_response(queue_id: &str) -> String {
    serde_json::json!({
        "ok": true,
        "acceptedQueueId": queue_id
    })
    .to_string()
}

pub fn build_queue_batch_response(items: &[MobileSyncQueueItem]) -> String {
    serde_json::json!({
        "ok": true,
        "accepted": items.len(),
        "acceptedQueueIds": items.iter().map(|item| item.queue_id.as_str()).collect::<Vec<_>>()
    })
    .to_string()
}

pub fn build_changes_response(
    conn: &Connection,
    since: Option<&str>,
) -> Result<String, rusqlite::Error> {
    let mut records = queries::list_records(conn);
    if let Some(since) = since.filter(|value| !value.trim().is_empty()) {
        records.retain(|record| record.created_at.as_str() > since);
    }
    let evidence_records = list_evidence_changes(conn, since)?;
    let mut next_since = since.unwrap_or("").to_string();
    for created_at in records
        .iter()
        .map(|record| record.created_at.as_str())
        .chain(
            evidence_records
                .iter()
                .map(|record| record.received_at.as_str()),
        )
    {
        if created_at > next_since.as_str() {
            next_since = created_at.to_string();
        }
    }
    let mut changes = records
        .into_iter()
        .map(|record| {
            serde_json::json!({
                "id": format!("desktop-{}", record.id),
                "desktop_id": record.id,
                "kind": desktop_record_kind(&record),
                "title": record.file_name,
                "watermark_uid": record.watermark_uid,
                "revision": record.revision,
                "sha256": record.original_hash,
                "parent_watermark_uid": record.parent_watermark_uid,
                "rewrite_reason": record.rewrite_reason,
                "source": "desktop",
                "sync_status": "synced",
                "created_at": record.created_at
            })
        })
        .collect::<Vec<_>>();
    changes.extend(evidence_records.into_iter().map(|record| {
        serde_json::json!({
            "id": format!("desktop-evidence-{}", record.mobile_record_id),
            "kind": record.asset_kind,
            "title": record.suspect_file_name,
            "watermark_uid": record.watermark_uid,
            "revision": record.revision,
            "sha256": null,
            "parent_watermark_uid": record.parent_watermark_uid,
            "rewrite_reason": record.rewrite_reason,
            "extracted_timestamp": record.extracted_timestamp,
            "extracted_device_id_hex": record.extracted_device_id_hex,
            "extracted_file_hash_hex": record.extracted_file_hash_hex,
            "source": "verify",
            "sync_status": "synced",
            "created_at": record.received_at
        })
    }));
    changes.sort_by(|left, right| {
        let left_at = left["created_at"].as_str().unwrap_or("");
        let right_at = right["created_at"].as_str().unwrap_or("");
        left_at.cmp(right_at)
    });

    Ok(serde_json::json!({
        "ok": true,
        "nextSince": next_since,
        "changes": changes
    })
    .to_string())
}

struct EvidenceChangeRecord {
    mobile_record_id: String,
    received_at: String,
    suspect_file_name: String,
    asset_kind: String,
    watermark_uid: String,
    revision: u32,
    parent_watermark_uid: Option<String>,
    rewrite_reason: Option<String>,
    extracted_timestamp: Option<i64>,
    extracted_device_id_hex: Option<String>,
    extracted_file_hash_hex: Option<String>,
}

fn list_evidence_changes(
    conn: &Connection,
    since: Option<&str>,
) -> Result<Vec<EvidenceChangeRecord>, rusqlite::Error> {
    let mut sql = "SELECT mobile_record_id, received_at, suspect_file_name, asset_kind,
                          watermark_uid, revision, parent_watermark_uid, rewrite_reason,
                          extracted_timestamp, extracted_device_id_hex, extracted_file_hash_hex
                   FROM sync_evidence_records"
        .to_string();
    if since.filter(|value| !value.trim().is_empty()).is_some() {
        sql.push_str(" WHERE received_at > ?1");
    }
    sql.push_str(" ORDER BY received_at ASC, id ASC");

    let mut stmt = conn.prepare(&sql)?;
    let map_row = |row: &rusqlite::Row<'_>| {
        Ok(EvidenceChangeRecord {
            mobile_record_id: row.get(0)?,
            received_at: row.get(1)?,
            suspect_file_name: row.get(2)?,
            asset_kind: row.get(3)?,
            watermark_uid: row.get(4)?,
            revision: row.get::<_, i64>(5)? as u32,
            parent_watermark_uid: row.get(6)?,
            rewrite_reason: row.get(7)?,
            extracted_timestamp: row.get(8)?,
            extracted_device_id_hex: row.get(9)?,
            extracted_file_hash_hex: row.get(10)?,
        })
    };

    let rows = if let Some(since) = since.filter(|value| !value.trim().is_empty()) {
        stmt.query_map(params![since], map_row)?
    } else {
        stmt.query_map([], map_row)?
    };
    rows.collect()
}

fn desktop_record_kind(record: &VaultRecord) -> &'static str {
    match record.resolution.as_str() {
        "image" => "image",
        "audio" => "audio",
        _ => "video",
    }
}

pub fn build_error_response(error: &str) -> String {
    serde_json::json!({
        "ok": false,
        "error": error
    })
    .to_string()
}

pub fn count_sync_events(conn: &Connection) -> Result<u64, rusqlite::Error> {
    conn.query_row("SELECT COUNT(*) FROM sync_events", [], |row| {
        let count: i64 = row.get(0)?;
        Ok(count.max(0) as u64)
    })
}

pub fn latest_sync_event_at(conn: &Connection) -> Result<Option<String>, rusqlite::Error> {
    conn.query_row(
        "SELECT received_at FROM sync_events ORDER BY received_at DESC LIMIT 1",
        [],
        |row| row.get(0),
    )
    .or_else(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncResolutionSummary {
    pub resolved_at: String,
    pub resolution_type: String,
    pub reason: String,
    pub watermark_uid: String,
    pub desktop_hash: Option<String>,
    pub mobile_hash: Option<String>,
    pub desktop_revision: Option<u32>,
    pub mobile_revision: Option<u32>,
}

pub fn count_sync_resolutions(conn: &Connection) -> Result<u64, rusqlite::Error> {
    conn.query_row("SELECT COUNT(*) FROM sync_resolutions", [], |row| {
        let count: i64 = row.get(0)?;
        Ok(count.max(0) as u64)
    })
}

pub fn latest_sync_resolution(
    conn: &Connection,
) -> Result<Option<SyncResolutionSummary>, rusqlite::Error> {
    conn.query_row(
        "SELECT resolved_at, resolution_type, reason, watermark_uid, desktop_hash, mobile_hash,
                desktop_revision, mobile_revision
         FROM sync_resolutions
         ORDER BY resolved_at DESC, id DESC
         LIMIT 1",
        [],
        |row| {
            Ok(SyncResolutionSummary {
                resolved_at: row.get(0)?,
                resolution_type: row.get(1)?,
                reason: row.get(2)?,
                watermark_uid: row.get(3)?,
                desktop_hash: row.get(4)?,
                mobile_hash: row.get(5)?,
                desktop_revision: row.get::<_, Option<i64>>(6)?.map(|value| value as u32),
                mobile_revision: row.get::<_, Option<i64>>(7)?.map(|value| value as u32),
            })
        },
    )
    .map(Some)
    .or_else(|err| match err {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        other => Err(other),
    })
}

pub fn get_or_create_pairing_code(app_data_dir: &std::path::Path) -> Result<String, String> {
    let path = pairing_code_path(app_data_dir);
    if let Ok(existing) = std::fs::read_to_string(&path) {
        let code = existing.trim();
        if !code.is_empty() {
            return Ok(code.to_string());
        }
    }

    let code = new_pairing_code();
    save_pairing_code(app_data_dir, &code)?;
    Ok(code)
}

pub fn save_pairing_code(app_data_dir: &std::path::Path, code: &str) -> Result<(), String> {
    std::fs::create_dir_all(app_data_dir)
        .map_err(|e| format!("failed to create app data directory: {e}"))?;
    std::fs::write(pairing_code_path(app_data_dir), code)
        .map_err(|e| format!("failed to save pairing code: {e}"))?;
    Ok(())
}

pub fn pairing_code_matches(app_data_dir: &std::path::Path, received_code: &str) -> bool {
    let expected = match get_or_create_pairing_code(app_data_dir) {
        Ok(code) => code,
        Err(err) => {
            log::warn!("failed to load pairing code: {err}");
            return false;
        }
    };
    !received_code.trim().is_empty() && expected == received_code.trim()
}

pub fn new_pairing_code() -> String {
    let nanos = chrono::Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_default()
        .unsigned_abs();
    format!("{:06}", nanos % 1_000_000)
}

fn pairing_code_path(app_data_dir: &std::path::Path) -> std::path::PathBuf {
    app_data_dir.join("mobile_pairing_code.txt")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairing_code_is_six_digits() {
        let code = new_pairing_code();
        assert_eq!(code.len(), 6);
        assert!(code.chars().all(|ch| ch.is_ascii_digit()));
    }

    #[test]
    fn pairing_code_matches_saved_code() {
        let temp_dir = tempfile::tempdir().unwrap();
        save_pairing_code(temp_dir.path(), "123456").unwrap();

        assert!(pairing_code_matches(temp_dir.path(), "123456"));
        assert!(pairing_code_matches(temp_dir.path(), "123456  "));
        assert!(!pairing_code_matches(temp_dir.path(), "000000"));
    }

    #[test]
    fn record_sync_event_upserts_mobile_vault_record() {
        let conn = Connection::open_in_memory().unwrap();
        queries::init_db(&conn).unwrap();
        init_sync_storage(&conn).unwrap();
        let item = MobileSyncQueueItem {
            queue_id: "queue-1".to_string(),
            record_id: "mobile-record-1".to_string(),
            operation: "upsertVaultRecord".to_string(),
            payload_type: "vault_record".to_string(),
            payload: serde_json::json!({
                "id": "mobile-record-1",
                "kind": "image",
                "title": "mobile-image.png",
                "watermark_uid": "uid-mobile-1",
                "revision": 2,
                "sha256": "abc123",
                "parent_watermark_uid": "uid-parent",
                "rewrite_reason": "owner rewrite",
                "source": "write",
                "sync_status": "pending",
                "created_at": "2026-06-16T12:00:00.000Z"
            }),
        };

        record_sync_event(&conn, &item).unwrap();

        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM vault_records WHERE watermark_uid = 'uid-mobile-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(count, 1);

        let (file_name, original_hash, revision, parent_uid, rewrite_reason): (
            String,
            String,
            i64,
            String,
            String,
        ) = conn
            .query_row(
                "SELECT file_name, original_hash, revision, parent_watermark_uid, rewrite_reason
                 FROM vault_records WHERE watermark_uid = 'uid-mobile-1'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(file_name, "mobile-image.png");
        assert_eq!(original_hash, "abc123");
        assert_eq!(revision, 2);
        assert_eq!(parent_uid, "uid-parent");
        assert_eq!(rewrite_reason, "owner rewrite");
    }

    #[test]
    fn build_changes_response_returns_records_after_since() {
        let conn = Connection::open_in_memory().unwrap();
        queries::init_db(&conn).unwrap();
        init_sync_storage(&conn).unwrap();
        let first = MobileSyncQueueItem {
            queue_id: "queue-1".to_string(),
            record_id: "record-1".to_string(),
            operation: "upsertVaultRecord".to_string(),
            payload_type: "vault_record".to_string(),
            payload: serde_json::json!({
                "id": "record-1",
                "kind": "image",
                "title": "first.png",
                "watermark_uid": "uid-first",
                "sha256": "hash-first",
                "revision": 1,
                "created_at": "2026-06-16T12:00:00.000Z"
            }),
        };
        let second = MobileSyncQueueItem {
            queue_id: "queue-2".to_string(),
            record_id: "record-2".to_string(),
            payload: serde_json::json!({
                "id": "record-2",
                "kind": "audio",
                "title": "second.wav",
                "watermark_uid": "uid-second",
                "sha256": "hash-second",
                "revision": 1,
                "created_at": "2026-06-16T12:00:01.000Z"
            }),
            ..first.clone()
        };

        record_sync_event(&conn, &first).unwrap();
        record_sync_event(&conn, &second).unwrap();

        let body = build_changes_response(&conn, Some("2026-06-16T12:00:00.000Z")).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        let changes = parsed["changes"].as_array().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0]["watermark_uid"], "uid-second");
        assert_eq!(changes[0]["kind"], "audio");
        assert_eq!(parsed["nextSince"], "2026-06-16T12:00:01.000Z");
    }

    #[test]
    fn build_changes_response_includes_evidence_records() {
        let conn = Connection::open_in_memory().unwrap();
        queries::init_db(&conn).unwrap();
        init_sync_storage(&conn).unwrap();
        let item = MobileSyncQueueItem {
            queue_id: "queue-evidence".to_string(),
            record_id: "record-evidence".to_string(),
            operation: "upsertEvidenceRecord".to_string(),
            payload_type: "evidence_record".to_string(),
            payload: serde_json::json!({
                "id": "record-evidence",
                "kind": "image",
                "title": "suspect.png",
                "watermark_uid": "uid-evidence",
                "revision": 2,
                "parent_watermark_uid": "uid-parent",
                "rewrite_reason": "authorized rewrite",
                "extracted_timestamp": 123,
                "extracted_device_id_hex": "device",
                "extracted_file_hash_hex": "hash"
            }),
        };

        record_sync_event(&conn, &item).unwrap();

        let body = build_changes_response(&conn, None).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&body).unwrap();
        let changes = parsed["changes"].as_array().unwrap();
        assert_eq!(changes.len(), 1);
        assert_eq!(changes[0]["source"], "verify");
        assert_eq!(changes[0]["title"], "suspect.png");
        assert_eq!(changes[0]["extracted_timestamp"], 123);
        assert_eq!(changes[0]["extracted_device_id_hex"], "device");
        assert_eq!(changes[0]["extracted_file_hash_hex"], "hash");
    }

    #[test]
    fn record_sync_event_deduplicates_mobile_vault_record() {
        let conn = Connection::open_in_memory().unwrap();
        queries::init_db(&conn).unwrap();
        init_sync_storage(&conn).unwrap();
        let item = MobileSyncQueueItem {
            queue_id: "queue-1".to_string(),
            record_id: "mobile-record-1".to_string(),
            operation: "upsertVaultRecord".to_string(),
            payload_type: "vault_record".to_string(),
            payload: serde_json::json!({
                "id": "mobile-record-1",
                "kind": "audio",
                "title": "mobile-audio.wav",
                "watermark_uid": "uid-mobile-1",
                "sha256": "abc123",
                "created_at": "2026-06-16T12:00:00.000Z"
            }),
        };
        let retry = MobileSyncQueueItem {
            queue_id: "queue-2".to_string(),
            ..item.clone()
        };

        record_sync_event(&conn, &item).unwrap();
        record_sync_event(&conn, &retry).unwrap();

        let vault_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM vault_records WHERE watermark_uid = 'uid-mobile-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let event_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(vault_count, 1);
        assert_eq!(event_count, 2);
    }

    #[test]
    fn record_sync_event_accepts_same_uid_different_hash_as_variant() {
        let conn = Connection::open_in_memory().unwrap();
        queries::init_db(&conn).unwrap();
        init_sync_storage(&conn).unwrap();
        let first = MobileSyncQueueItem {
            queue_id: "queue-1".to_string(),
            record_id: "mobile-record-1".to_string(),
            operation: "upsertVaultRecord".to_string(),
            payload_type: "vault_record".to_string(),
            payload: serde_json::json!({
                "id": "mobile-record-1",
                "kind": "image",
                "title": "first.png",
                "watermark_uid": "uid-conflict",
                "sha256": "hash-one",
                "revision": 1,
                "created_at": "2026-06-16T12:00:00.000Z"
            }),
        };
        let second = MobileSyncQueueItem {
            queue_id: "queue-2".to_string(),
            record_id: "mobile-record-2".to_string(),
            payload: serde_json::json!({
                "id": "mobile-record-2",
                "kind": "image",
                "title": "second.png",
                "watermark_uid": "uid-conflict",
                "sha256": "hash-two",
                "revision": 1,
                "created_at": "2026-06-16T12:00:01.000Z"
            }),
            ..first.clone()
        };

        record_sync_event(&conn, &first).unwrap();
        record_sync_event(&conn, &second).unwrap();

        let vault_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM vault_records WHERE watermark_uid = 'uid-conflict'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let (resolution_type, desktop_hash, mobile_hash): (String, Option<String>, String) = conn
            .query_row(
                "SELECT resolution_type, desktop_hash, mobile_hash
                 FROM sync_resolutions
                 WHERE watermark_uid = 'uid-conflict'
                   AND resolution_type = 'variant_accepted'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(vault_count, 2);
        assert_eq!(resolution_type, "variant_accepted");
        assert_eq!(desktop_hash.as_deref(), Some("hash-one"));
        assert_eq!(mobile_hash, "hash-two");
    }

    #[test]
    fn record_sync_event_upgrades_same_asset_to_higher_revision() {
        let conn = Connection::open_in_memory().unwrap();
        queries::init_db(&conn).unwrap();
        init_sync_storage(&conn).unwrap();
        let first = MobileSyncQueueItem {
            queue_id: "queue-1".to_string(),
            record_id: "mobile-record-1".to_string(),
            operation: "upsertVaultRecord".to_string(),
            payload_type: "vault_record".to_string(),
            payload: serde_json::json!({
                "id": "mobile-record-1",
                "kind": "audio",
                "title": "first.wav",
                "watermark_uid": "uid-revision-conflict",
                "sha256": "same-hash",
                "revision": 1,
                "created_at": "2026-06-16T12:00:00.000Z"
            }),
        };
        let second = MobileSyncQueueItem {
            queue_id: "queue-2".to_string(),
            record_id: "mobile-record-2".to_string(),
            payload: serde_json::json!({
                "id": "mobile-record-2",
                "kind": "audio",
                "title": "second.wav",
                "watermark_uid": "uid-revision-conflict",
                "sha256": "same-hash",
                "revision": 2,
                "created_at": "2026-06-16T12:00:01.000Z"
            }),
            ..first.clone()
        };

        record_sync_event(&conn, &first).unwrap();
        record_sync_event(&conn, &second).unwrap();

        let vault_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM vault_records WHERE watermark_uid = 'uid-revision-conflict'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let stored_revision: i64 = conn
            .query_row(
                "SELECT revision FROM vault_records
                 WHERE watermark_uid = 'uid-revision-conflict'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let (resolution_type, desktop_revision, mobile_revision): (String, Option<i64>, i64) = conn
            .query_row(
                "SELECT resolution_type, desktop_revision, mobile_revision
                 FROM sync_resolutions
                 WHERE watermark_uid = 'uid-revision-conflict'
                   AND resolution_type = 'revision_upgraded'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(vault_count, 1);
        assert_eq!(stored_revision, 2);
        assert_eq!(resolution_type, "revision_upgraded");
        assert_eq!(desktop_revision, Some(1));
        assert_eq!(mobile_revision, 2);
    }

    #[test]
    fn record_sync_event_ignores_stale_same_asset_revision() {
        let conn = Connection::open_in_memory().unwrap();
        queries::init_db(&conn).unwrap();
        init_sync_storage(&conn).unwrap();
        let first = MobileSyncQueueItem {
            queue_id: "queue-1".to_string(),
            record_id: "mobile-record-1".to_string(),
            operation: "upsertVaultRecord".to_string(),
            payload_type: "vault_record".to_string(),
            payload: serde_json::json!({
                "id": "mobile-record-1",
                "kind": "audio",
                "title": "first.wav",
                "watermark_uid": "uid-stale-revision",
                "sha256": "same-hash",
                "revision": 3,
                "created_at": "2026-06-16T12:00:00.000Z"
            }),
        };
        let second = MobileSyncQueueItem {
            queue_id: "queue-2".to_string(),
            record_id: "mobile-record-2".to_string(),
            payload: serde_json::json!({
                "id": "mobile-record-2",
                "kind": "audio",
                "title": "second.wav",
                "watermark_uid": "uid-stale-revision",
                "sha256": "same-hash",
                "revision": 2,
                "created_at": "2026-06-16T12:00:01.000Z"
            }),
            ..first.clone()
        };

        record_sync_event(&conn, &first).unwrap();
        record_sync_event(&conn, &second).unwrap();

        let stored_revision: i64 = conn
            .query_row(
                "SELECT revision FROM vault_records WHERE watermark_uid = 'uid-stale-revision'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let resolution_type: String = conn
            .query_row(
                "SELECT resolution_type FROM sync_resolutions
                 WHERE watermark_uid = 'uid-stale-revision'
                   AND resolution_type = 'stale_revision_ignored'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_revision, 3);
        assert_eq!(resolution_type, "stale_revision_ignored");
    }

    #[test]
    fn record_sync_event_upserts_mobile_evidence_record() {
        let conn = Connection::open_in_memory().unwrap();
        queries::init_db(&conn).unwrap();
        init_sync_storage(&conn).unwrap();
        let item = MobileSyncQueueItem {
            queue_id: "queue-evidence-1".to_string(),
            record_id: "mobile-evidence-1".to_string(),
            operation: "upsertEvidenceRecord".to_string(),
            payload_type: "vault_record".to_string(),
            payload: serde_json::json!({
                "id": "mobile-evidence-1",
                "kind": "image",
                "title": "suspect.png",
                "watermark_uid": "uid-evidence-1",
                "revision": 2,
                "parent_watermark_uid": "uid-parent",
                "rewrite_reason": "authorized rewrite",
                "extracted_timestamp": 123,
                "extracted_device_id_hex": "device",
                "extracted_file_hash_hex": "hash"
            }),
        };

        record_sync_event(&conn, &item).unwrap();

        let (
            suspect_file_name,
            watermark_uid,
            revision,
            parent_uid,
            extracted_timestamp,
            device,
            file_hash,
        ): (String, String, i64, String, i64, String, String) = conn
            .query_row(
                "SELECT suspect_file_name, watermark_uid, revision,
                        parent_watermark_uid, extracted_timestamp,
                        extracted_device_id_hex, extracted_file_hash_hex
                 FROM sync_evidence_records
                 WHERE mobile_record_id = 'mobile-evidence-1'",
                [],
                |row| {
                    Ok((
                        row.get(0)?,
                        row.get(1)?,
                        row.get(2)?,
                        row.get(3)?,
                        row.get(4)?,
                        row.get(5)?,
                        row.get(6)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(suspect_file_name, "suspect.png");
        assert_eq!(watermark_uid, "uid-evidence-1");
        assert_eq!(revision, 2);
        assert_eq!(parent_uid, "uid-parent");
        assert_eq!(extracted_timestamp, 123);
        assert_eq!(device, "device");
        assert_eq!(file_hash, "hash");
    }

    #[test]
    fn record_sync_event_updates_existing_mobile_evidence_record() {
        let conn = Connection::open_in_memory().unwrap();
        queries::init_db(&conn).unwrap();
        init_sync_storage(&conn).unwrap();
        let first = MobileSyncQueueItem {
            queue_id: "queue-evidence-1".to_string(),
            record_id: "mobile-evidence-1".to_string(),
            operation: "upsertEvidenceRecord".to_string(),
            payload_type: "evidence_record".to_string(),
            payload: serde_json::json!({
                "id": "mobile-evidence-1",
                "kind": "image",
                "title": "suspect-old.png",
                "watermark_uid": "uid-evidence-1",
                "revision": 1,
                "extracted_timestamp": 123
            }),
        };
        let second = MobileSyncQueueItem {
            queue_id: "queue-evidence-2".to_string(),
            payload: serde_json::json!({
                "id": "mobile-evidence-1",
                "kind": "image",
                "title": "suspect-new.png",
                "watermark_uid": "uid-evidence-1",
                "revision": 2,
                "extracted_timestamp": 456
            }),
            ..first.clone()
        };

        record_sync_event(&conn, &first).unwrap();
        record_sync_event(&conn, &second).unwrap();

        let evidence_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_evidence_records", [], |row| {
                row.get(0)
            })
            .unwrap();
        let (title, revision, timestamp): (String, i64, i64) = conn
            .query_row(
                "SELECT suspect_file_name, revision, extracted_timestamp
                 FROM sync_evidence_records
                 WHERE mobile_record_id = 'mobile-evidence-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        let event_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM sync_events", [], |row| row.get(0))
            .unwrap();
        assert_eq!(evidence_count, 1);
        assert_eq!(title, "suspect-new.png");
        assert_eq!(revision, 2);
        assert_eq!(timestamp, 456);
        assert_eq!(event_count, 2);
    }
}

use std::collections::BTreeMap;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{DateTime, SecondsFormat, Utc};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db::billing::{self, EntitlementState, EntitlementStatus};
use crate::db::offline_license::{
    self as license_db, InstallationSecretStore, OfflineLicenseAuditEvent, StoredOfflineLicense,
    StoredRevocationList,
};
use crate::offline_license::{
    encode_activation_request_v1, parse_offline_license_v1, parse_revocation_list_v1,
    verify_offline_license_v1_signature, verify_revocation_list_v1_signature,
    ActivationRequestPayloadV1,
};

const LOCAL_BATCH_FEATURE: &str = "batch_processing";
const CLOUD_ONLY_FEATURES: [&str; 6] = [
    "cloud_sync",
    "cloud_batch_processing",
    "cloud_video_processing",
    "priority_queue",
    "team_workspace",
    "api_access",
];
const CLOCK_ROLLBACK_TOLERANCE_SECONDS: i64 = 300;
const FUTURE_ARTIFACT_TOLERANCE_SECONDS: i64 = 300;
const EMBEDDED_TRUST_POLICY_JSON: Option<&str> =
    option_env!("HIDDENSHIELD_OFFLINE_LICENSE_TRUST_POLICY_JSON");

#[cfg(any(test, feature = "internal-qa"))]
const K0_TEST_PUBLIC_KEY: [u8; 32] = [
    0xd7, 0x5a, 0x98, 0x01, 0x82, 0xb1, 0x0a, 0xb7, 0xd5, 0x4b, 0xfe, 0xd3, 0xc9, 0x64, 0x07, 0x3a,
    0x0e, 0xe1, 0x72, 0xf3, 0xda, 0xa6, 0x23, 0x25, 0xaf, 0x02, 0x1a, 0x68, 0xf7, 0x07, 0x51, 0x1a,
];

#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OfflineLicenseStatus {
    pub status: String,
    pub installation_id: String,
    pub installation_created_at: String,
    pub license_id: Option<String>,
    pub product_code: Option<String>,
    pub key_id: Option<String>,
    pub issued_at: Option<String>,
    pub not_before: Option<String>,
    pub expires_at: Option<String>,
    pub imported_at: Option<String>,
    pub revocation_list_sequence: Option<u64>,
    pub error_code: Option<String>,
    pub features: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Serialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OfflineActivationRequestExport {
    pub token: String,
    pub installation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmbeddedTrustPolicy {
    schema_version: u8,
    policy_type: String,
    keys: Vec<EmbeddedTrustedKey>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmbeddedTrustedKey {
    key_id: String,
    algorithm: String,
    public_key_base64_url: String,
    status: String,
    purposes: Vec<String>,
    not_before: String,
    not_after: String,
}

#[derive(Debug)]
struct EvaluatedOfflineLicense {
    status: String,
    license: Option<StoredOfflineLicense>,
    revocation_list_sequence: Option<u64>,
    error_code: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RevocationProgress {
    Idempotent,
    Newer,
}

pub fn get_offline_license_status(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
) -> Result<OfflineLicenseStatus, String> {
    get_offline_license_status_at(conn, secret_store, Utc::now())
}

pub fn get_offline_license_status_at(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    now: DateTime<Utc>,
) -> Result<OfflineLicenseStatus, String> {
    let mut evaluated = evaluate_offline_license(conn, secret_store, now)?;
    let identity = match license_db::get_or_create_installation_identity(conn, secret_store) {
        Ok(identity) => identity,
        Err(error) => {
            let stored = license_db::load_installation_identity(conn)?.ok_or(error.clone())?;
            evaluated.status = "invalid".to_string();
            evaluated.error_code = Some(error);
            license_db::InstallationIdentity {
                installation_id: stored.installation_id,
                created_at: stored.created_at,
            }
        }
    };
    let license = evaluated.license.as_ref();
    let is_active = evaluated.status == "active";
    Ok(OfflineLicenseStatus {
        status: evaluated.status,
        installation_id: identity.installation_id,
        installation_created_at: identity.created_at,
        license_id: license.map(|value| value.license_id.clone()),
        product_code: license.map(|value| value.product_code.clone()),
        key_id: license.map(|value| value.key_id.clone()),
        issued_at: license.map(|value| value.issued_at.clone()),
        not_before: license.map(|value| value.not_before.clone()),
        expires_at: license.map(|value| value.expires_at.clone()),
        imported_at: license.map(|value| value.imported_at.clone()),
        revocation_list_sequence: evaluated.revocation_list_sequence,
        error_code: evaluated.error_code,
        features: offline_feature_map(license.is_some() && is_active),
    })
}

pub fn export_activation_request(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
) -> Result<OfflineActivationRequestExport, String> {
    check_and_record_trusted_time(conn, secret_store, Utc::now())?;
    let identity = license_db::get_or_create_installation_identity(conn, secret_store)?;
    let mut nonce = [0u8; 16];
    let mut request_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce)
        .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
    getrandom::getrandom(&mut request_bytes)
        .map_err(|_| "offline_license_secure_storage_unavailable".to_string())?;
    let request_id = format!("req_{}", hex::encode(request_bytes));
    let payload = ActivationRequestPayloadV1 {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        created_at: timestamp(Utc::now()),
        installation_id: identity.installation_id.clone(),
        nonce: base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, nonce),
        platform: platform_name().to_string(),
        request_id: request_id.clone(),
        requested_product_code: "creator_offline".to_string(),
        schema_version: 1,
    };
    let token = encode_activation_request_v1(&payload)?;
    license_db::append_audit_event(
        conn,
        &OfflineLicenseAuditEvent {
            occurred_at: timestamp(Utc::now()),
            event_type: "activation_request_exported".to_string(),
            outcome: "accepted".to_string(),
            installation_id: Some(identity.installation_id.clone()),
            artifact_id: Some(request_id),
            key_id: None,
            detail_code: None,
        },
    )?;
    Ok(OfflineActivationRequestExport {
        token,
        installation_id: identity.installation_id,
        output_path: None,
    })
}

pub fn import_offline_license(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    token: &str,
) -> Result<OfflineLicenseStatus, String> {
    import_offline_license_at(conn, secret_store, token, Utc::now())
}

fn import_offline_license_at(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    token: &str,
    now: DateTime<Utc>,
) -> Result<OfflineLicenseStatus, String> {
    check_and_record_trusted_time(conn, secret_store, now)?;
    let identity = license_db::get_or_create_installation_identity(conn, secret_store)?;
    let result = validate_license_token(conn, secret_store, token, &identity.installation_id, now);
    match result {
        Ok(license) => {
            let transaction = conn
                .unchecked_transaction()
                .map_err(|error| format!("offline_license_db_error:{error}"))?;
            if let Some(previous) = license_db::load_offline_license(&transaction)? {
                if previous.license_id != license.license_id {
                    append_artifact_audit(
                        &transaction,
                        "license_replaced",
                        "accepted",
                        Some(&identity.installation_id),
                        Some(&previous.license_id),
                        Some(&previous.key_id),
                        Some(&license.license_id),
                    )?;
                }
            }
            license_db::save_offline_license(&transaction, &license)?;
            append_artifact_audit(
                &transaction,
                "license_imported",
                "accepted",
                Some(&identity.installation_id),
                Some(&license.license_id),
                Some(&license.key_id),
                None,
            )?;
            transaction
                .commit()
                .map_err(|error| format!("offline_license_db_error:{error}"))?;
            get_offline_license_status_at(conn, secret_store, now)
        }
        Err(error) => {
            let _ = append_artifact_audit(
                conn,
                "license_imported",
                "rejected",
                Some(&identity.installation_id),
                None,
                None,
                Some(&error),
            );
            Err(error)
        }
    }
}

pub fn clear_offline_license(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
) -> Result<OfflineLicenseStatus, String> {
    let now = Utc::now();
    check_and_record_trusted_time(conn, secret_store, now)?;
    let identity = license_db::get_or_create_installation_identity(conn, secret_store)?;
    let transaction = conn
        .unchecked_transaction()
        .map_err(|error| format!("offline_license_db_error:{error}"))?;
    let previous = license_db::load_offline_license(&transaction)?;
    license_db::clear_offline_license(&transaction)?;
    append_artifact_audit(
        &transaction,
        "license_cleared",
        "accepted",
        Some(&identity.installation_id),
        previous.as_ref().map(|license| license.license_id.as_str()),
        previous.as_ref().map(|license| license.key_id.as_str()),
        None,
    )?;
    transaction
        .commit()
        .map_err(|error| format!("offline_license_db_error:{error}"))?;
    get_offline_license_status_at(conn, secret_store, now)
}

pub fn import_revocation_list(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    token: &str,
) -> Result<OfflineLicenseStatus, String> {
    import_revocation_list_at(conn, secret_store, token, Utc::now())
}

fn import_revocation_list_at(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    token: &str,
    now: DateTime<Utc>,
) -> Result<OfflineLicenseStatus, String> {
    check_and_record_trusted_time(conn, secret_store, now)?;
    let identity = license_db::get_or_create_installation_identity(conn, secret_store)?;
    let result = validate_revocation_token(conn, secret_store, token, now);
    match result {
        Ok(list) => {
            record_revocation_high_water(secret_store, &list)?;
            let transaction = conn
                .unchecked_transaction()
                .map_err(|error| format!("offline_license_db_error:{error}"))?;
            license_db::save_revocation_list(&transaction, &list)?;
            append_artifact_audit(
                &transaction,
                "revocation_list_imported",
                "accepted",
                Some(&identity.installation_id),
                Some(&list.list_id),
                Some(&list.key_id),
                None,
            )?;
            transaction
                .commit()
                .map_err(|error| format!("offline_license_db_error:{error}"))?;
            get_offline_license_status_at(conn, secret_store, now)
        }
        Err(error) => {
            let _ = append_artifact_audit(
                conn,
                "revocation_list_imported",
                "rejected",
                Some(&identity.installation_id),
                None,
                None,
                Some(&error),
            );
            Err(error)
        }
    }
}

pub fn resolve_effective_entitlement(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
) -> Result<EntitlementState, String> {
    resolve_effective_entitlement_at(conn, secret_store, Utc::now())
}

pub fn resolve_effective_entitlement_at(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    now: DateTime<Utc>,
) -> Result<EntitlementState, String> {
    let mut effective = billing::get_entitlement_state(conn)
        .map_err(|error| format!("offline_license_db_error:{error}"))?;
    sanitize_cached_entitlement(&mut effective);

    let evaluated = evaluate_offline_license(conn, secret_store, now)?;
    if evaluated.status != "active" {
        return Ok(effective);
    }
    let Some(license) = evaluated.license else {
        return Ok(effective);
    };

    effective
        .features
        .insert(LOCAL_BATCH_FEATURE.to_string(), true);
    if !matches!(
        effective.status,
        EntitlementStatus::Trial | EntitlementStatus::Active | EntitlementStatus::Grace
    ) {
        effective.status = EntitlementStatus::Active;
        effective.plan_name = Some("图片/音频年费授权".to_string());
        effective.plan_code = "creator_offline".to_string();
        effective.billing_source = Some("offline_license".to_string());
        effective.subscription_id = Some(license.license_id.clone());
        effective.current_period_started_at = Some(license.not_before.clone());
        effective.current_period_ends_at = Some(license.expires_at.clone());
        effective.last_checked_at = Some(timestamp(now));
        effective.updated_at = timestamp(now);
    }
    Ok(effective)
}

fn evaluate_offline_license(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    now: DateTime<Utc>,
) -> Result<EvaluatedOfflineLicense, String> {
    check_and_record_trusted_time(conn, secret_store, now)?;
    let Some(stored) = license_db::load_offline_license(conn)? else {
        return Ok(EvaluatedOfflineLicense {
            status: "not_installed".to_string(),
            license: None,
            revocation_list_sequence: highest_revocation_sequence(conn)?,
            error_code: None,
        });
    };
    let identity = match license_db::get_or_create_installation_identity(conn, secret_store) {
        Ok(identity) => identity,
        Err(error) => {
            return Ok(invalid_evaluation(stored, error, None));
        }
    };
    match validate_stored_license(conn, secret_store, &stored, &identity.installation_id, now) {
        Ok(sequence) => Ok(EvaluatedOfflineLicense {
            status: "active".to_string(),
            license: Some(stored),
            revocation_list_sequence: sequence,
            error_code: None,
        }),
        Err(error) => {
            let status = match error.as_str() {
                "offline_license_expired" => "expired",
                "offline_license_not_yet_valid" => "not_yet_valid",
                "offline_license_revoked" => "revoked",
                _ => "invalid",
            };
            Ok(EvaluatedOfflineLicense {
                status: status.to_string(),
                license: Some(stored),
                revocation_list_sequence: highest_revocation_sequence(conn)?,
                error_code: Some(error),
            })
        }
    }
}

fn invalid_evaluation(
    license: StoredOfflineLicense,
    error: String,
    sequence: Option<u64>,
) -> EvaluatedOfflineLicense {
    EvaluatedOfflineLicense {
        status: "invalid".to_string(),
        license: Some(license),
        revocation_list_sequence: sequence,
        error_code: Some(error),
    }
}

fn validate_license_token(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    token: &str,
    installation_id: &str,
    now: DateTime<Utc>,
) -> Result<StoredOfflineLicense, String> {
    let parsed = parse_offline_license_v1(token)?;
    let public_key = trusted_public_key(&parsed.payload.key_id, "license", now)?;
    if !verify_offline_license_v1_signature(&parsed, &public_key)? {
        return Err("offline_license_signature_invalid".to_string());
    }
    if parsed.payload.installation_id != installation_id {
        return Err("offline_license_device_mismatch".to_string());
    }
    validate_license_time(
        &parsed.payload.issued_at,
        &parsed.payload.not_before,
        &parsed.payload.expires_at,
        now,
    )?;
    ensure_not_revoked(conn, secret_store, &parsed.payload.license_id, now)?;
    Ok(StoredOfflineLicense {
        signed_token: token.to_string(),
        token_sha256: license_db::token_sha256(token),
        license_id: parsed.payload.license_id,
        installation_id: parsed.payload.installation_id,
        product_code: parsed.payload.product_code,
        key_id: parsed.payload.key_id,
        issued_at: parsed.payload.issued_at,
        not_before: parsed.payload.not_before,
        expires_at: parsed.payload.expires_at,
        imported_at: timestamp(now),
    })
}

fn validate_stored_license(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    stored: &StoredOfflineLicense,
    installation_id: &str,
    now: DateTime<Utc>,
) -> Result<Option<u64>, String> {
    if license_db::token_sha256(&stored.signed_token) != stored.token_sha256 {
        return Err("offline_license_state_tampered".to_string());
    }
    let parsed = parse_offline_license_v1(&stored.signed_token)?;
    if parsed.payload.license_id != stored.license_id
        || parsed.payload.installation_id != stored.installation_id
        || parsed.payload.product_code != stored.product_code
        || parsed.payload.key_id != stored.key_id
        || parsed.payload.issued_at != stored.issued_at
        || parsed.payload.not_before != stored.not_before
        || parsed.payload.expires_at != stored.expires_at
    {
        return Err("offline_license_state_tampered".to_string());
    }
    let public_key = trusted_public_key(&parsed.payload.key_id, "license", now)?;
    if !verify_offline_license_v1_signature(&parsed, &public_key)? {
        return Err("offline_license_signature_invalid".to_string());
    }
    if parsed.payload.installation_id != installation_id {
        return Err("offline_license_device_mismatch".to_string());
    }
    validate_license_time(
        &parsed.payload.issued_at,
        &parsed.payload.not_before,
        &parsed.payload.expires_at,
        now,
    )?;
    let sequence = ensure_not_revoked(conn, secret_store, &parsed.payload.license_id, now)?;
    Ok(sequence)
}

fn validate_revocation_token(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    token: &str,
    now: DateTime<Utc>,
) -> Result<StoredRevocationList, String> {
    let parsed = parse_revocation_list_v1(token)?;
    let public_key = trusted_public_key(&parsed.payload.key_id, "revocation", now)?;
    if !verify_revocation_list_v1_signature(&parsed, &public_key)? {
        return Err("offline_license_revocation_signature_invalid".to_string());
    }
    let generated_at = parse_time(&parsed.payload.generated_at)?;
    if generated_at > now + chrono::Duration::seconds(FUTURE_ARTIFACT_TOLERANCE_SECONDS) {
        return Err("offline_license_artifact_from_future".to_string());
    }
    let token_sha256 = license_db::token_sha256(token);
    let database_existing = license_db::load_revocation_lists(conn)?
        .into_iter()
        .find(|list| list.key_id == parsed.payload.key_id);
    let database_progress = database_existing
        .as_ref()
        .map(|existing| {
            compare_revocation_high_water(
                parsed.payload.sequence,
                &token_sha256,
                existing.sequence,
                &existing.token_sha256,
            )
        })
        .transpose()?;
    let secure_anchor = secret_store.load_security_anchor()?;
    let secure_progress = secure_anchor
        .revocation_high_water
        .get(&parsed.payload.key_id)
        .map(|existing| {
            compare_revocation_high_water(
                parsed.payload.sequence,
                &token_sha256,
                existing.sequence,
                &existing.token_sha256,
            )
        })
        .transpose()?;
    if database_progress == Some(RevocationProgress::Idempotent)
        && secure_progress != Some(RevocationProgress::Newer)
    {
        return database_existing.ok_or_else(|| "offline_license_db_error".to_string());
    }
    Ok(StoredRevocationList {
        key_id: parsed.payload.key_id,
        signed_token: token.to_string(),
        token_sha256,
        list_id: parsed.payload.list_id,
        sequence: parsed.payload.sequence,
        generated_at: parsed.payload.generated_at,
        imported_at: timestamp(now),
    })
}

fn ensure_not_revoked(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    license_id: &str,
    now: DateTime<Utc>,
) -> Result<Option<u64>, String> {
    let mut highest = None;
    let stored_lists = license_db::load_revocation_lists(conn)?;
    let mut lists_by_key = BTreeMap::new();
    for stored in &stored_lists {
        if license_db::token_sha256(&stored.signed_token) != stored.token_sha256 {
            return Err("offline_license_revocation_state_tampered".to_string());
        }
        let parsed = parse_revocation_list_v1(&stored.signed_token)?;
        if parsed.payload.key_id != stored.key_id
            || parsed.payload.list_id != stored.list_id
            || parsed.payload.sequence != stored.sequence
            || parsed.payload.generated_at != stored.generated_at
        {
            return Err("offline_license_revocation_state_tampered".to_string());
        }
        let public_key = trusted_public_key(&parsed.payload.key_id, "revocation", now)?;
        if !verify_revocation_list_v1_signature(&parsed, &public_key)? {
            return Err("offline_license_revocation_signature_invalid".to_string());
        }
        highest = Some(highest.map_or(stored.sequence, |value: u64| value.max(stored.sequence)));
        if parsed
            .payload
            .revoked_license_ids
            .iter()
            .any(|revoked| revoked == license_id)
        {
            return Err("offline_license_revoked".to_string());
        }
        lists_by_key.insert(stored.key_id.clone(), stored.clone());
    }
    let mut anchor = secret_store.load_security_anchor()?;
    let mut anchor_changed = false;
    for (key_id, high_water) in &anchor.revocation_high_water {
        let stored = lists_by_key
            .get(key_id)
            .ok_or_else(|| "offline_license_revocation_state_rollback".to_string())?;
        if stored.sequence < high_water.sequence {
            return Err("offline_license_revocation_state_rollback".to_string());
        }
        if stored.sequence == high_water.sequence && stored.token_sha256 != high_water.token_sha256
        {
            return Err("offline_license_revocation_state_tampered".to_string());
        }
    }
    for stored in stored_lists {
        let anchored = anchor.revocation_high_water.get(&stored.key_id);
        if anchored.is_none_or(|value| stored.sequence > value.sequence) {
            anchor.revocation_high_water.insert(
                stored.key_id,
                license_db::RevocationHighWater {
                    sequence: stored.sequence,
                    token_sha256: stored.token_sha256,
                },
            );
            anchor_changed = true;
        }
    }
    if anchor_changed {
        secret_store.store_security_anchor(&anchor)?;
    }
    Ok(highest)
}

fn highest_revocation_sequence(conn: &Connection) -> Result<Option<u64>, String> {
    Ok(license_db::load_revocation_lists(conn)?
        .into_iter()
        .map(|list| list.sequence)
        .max())
}

fn validate_license_time(
    issued_at: &str,
    not_before: &str,
    expires_at: &str,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let issued_at = parse_time(issued_at)?;
    let not_before = parse_time(not_before)?;
    let expires_at = parse_time(expires_at)?;
    if issued_at > now || not_before > now {
        return Err("offline_license_not_yet_valid".to_string());
    }
    if expires_at <= now {
        return Err("offline_license_expired".to_string());
    }
    if issued_at > expires_at || not_before > expires_at {
        return Err("offline_license_time_invalid".to_string());
    }
    Ok(())
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| "offline_license_time_invalid".to_string())
}

fn sanitize_cached_entitlement(entitlement: &mut EntitlementState) {
    let active = matches!(
        entitlement.status,
        EntitlementStatus::Trial | EntitlementStatus::Active | EntitlementStatus::Grace
    );
    for (feature, _) in billing::default_entitlement_features() {
        let value = active && entitlement.features.get(&feature) == Some(&true);
        entitlement.features.insert(feature, value);
    }
    entitlement
        .features
        .insert("report_export".to_string(), false);
    if !active {
        entitlement.status = EntitlementStatus::Free;
        entitlement.plan_name = None;
        entitlement.plan_code = "free".to_string();
        entitlement.billing_source = None;
        entitlement.subscription_id = None;
    }
}

fn offline_feature_map(active: bool) -> BTreeMap<String, bool> {
    let mut features = billing::default_entitlement_features();
    features.insert(LOCAL_BATCH_FEATURE.to_string(), active);
    features.insert("report_export".to_string(), false);
    for feature in CLOUD_ONLY_FEATURES {
        features.insert(feature.to_string(), false);
    }
    features
}

fn trusted_public_key(key_id: &str, purpose: &str, now: DateTime<Utc>) -> Result<[u8; 32], String> {
    #[cfg(any(test, feature = "internal-qa"))]
    if key_id == "offline-test-k0" {
        return Ok(K0_TEST_PUBLIC_KEY);
    }

    let policy_json =
        EMBEDDED_TRUST_POLICY_JSON.ok_or_else(|| "offline_license_unknown_key".to_string())?;
    let policy: EmbeddedTrustPolicy = serde_json::from_str(policy_json)
        .map_err(|_| "offline_license_trust_policy_invalid".to_string())?;
    if policy.schema_version != 1 || policy.policy_type != "offline_license_trust_policy" {
        return Err("offline_license_trust_policy_invalid".to_string());
    }
    let key = policy
        .keys
        .into_iter()
        .find(|candidate| candidate.key_id == key_id)
        .ok_or_else(|| "offline_license_unknown_key".to_string())?;
    if key.algorithm != "Ed25519" {
        return Err("offline_license_trust_policy_invalid".to_string());
    }
    if key.status == "disabled" {
        return Err("offline_license_key_disabled".to_string());
    }
    if key.status != "active" && key.status != "verify_only" {
        return Err("offline_license_trust_policy_invalid".to_string());
    }
    if !key.purposes.iter().any(|candidate| candidate == purpose) {
        return Err("offline_license_key_purpose_invalid".to_string());
    }
    let not_before = parse_time(&key.not_before)?;
    let not_after = parse_time(&key.not_after)?;
    if now < not_before || now >= not_after {
        return Err("offline_license_key_inactive".to_string());
    }
    let public_key = URL_SAFE_NO_PAD
        .decode(key.public_key_base64_url)
        .map_err(|_| "offline_license_trust_policy_invalid".to_string())?;
    public_key
        .try_into()
        .map_err(|_| "offline_license_trust_policy_invalid".to_string())
}

fn compare_revocation_high_water(
    candidate_sequence: u64,
    candidate_digest: &str,
    stored_sequence: u64,
    stored_digest: &str,
) -> Result<RevocationProgress, String> {
    if candidate_sequence < stored_sequence {
        return Err("offline_license_revocation_replay".to_string());
    }
    if candidate_sequence == stored_sequence {
        if candidate_digest == stored_digest {
            return Ok(RevocationProgress::Idempotent);
        }
        return Err("offline_license_revocation_equivocation".to_string());
    }
    Ok(RevocationProgress::Newer)
}

fn record_revocation_high_water(
    secret_store: &dyn InstallationSecretStore,
    list: &StoredRevocationList,
) -> Result<(), String> {
    let mut anchor = secret_store.load_security_anchor()?;
    if let Some(existing) = anchor.revocation_high_water.get(&list.key_id) {
        compare_revocation_high_water(
            list.sequence,
            &list.token_sha256,
            existing.sequence,
            &existing.token_sha256,
        )?;
    }
    anchor.revocation_high_water.insert(
        list.key_id.clone(),
        license_db::RevocationHighWater {
            sequence: list.sequence,
            token_sha256: list.token_sha256.clone(),
        },
    );
    secret_store.store_security_anchor(&anchor)
}

fn check_and_record_trusted_time(
    conn: &Connection,
    secret_store: &dyn InstallationSecretStore,
    now: DateTime<Utc>,
) -> Result<(), String> {
    let database_highest = license_db::load_highest_observed_utc(conn)?
        .map(|value| parse_time(&value))
        .transpose()?;
    let mut anchor = secret_store.load_security_anchor()?;
    let secure_highest = anchor
        .highest_observed_utc
        .as_deref()
        .map(parse_time)
        .transpose()?;
    let highest = match (database_highest, secure_highest) {
        (Some(database), Some(secure)) => Some(database.max(secure)),
        (Some(database), None) => Some(database),
        (None, Some(secure)) => Some(secure),
        (None, None) => None,
    };
    if let Some(highest) = highest {
        if now + chrono::Duration::seconds(CLOCK_ROLLBACK_TOLERANCE_SECONDS) < highest {
            return Err("offline_license_clock_rollback".to_string());
        }
    }
    let next_highest = highest.map_or(now, |value| value.max(now));
    let next_text = timestamp(next_highest);
    license_db::save_highest_observed_utc(conn, &next_text, &timestamp(now))?;
    if anchor.highest_observed_utc.as_deref() != Some(next_text.as_str()) {
        anchor.highest_observed_utc = Some(next_text);
        secret_store.store_security_anchor(&anchor)?;
    }
    Ok(())
}

fn append_artifact_audit(
    conn: &Connection,
    event_type: &str,
    outcome: &str,
    installation_id: Option<&str>,
    artifact_id: Option<&str>,
    key_id: Option<&str>,
    detail_code: Option<&str>,
) -> Result<(), String> {
    license_db::append_audit_event(
        conn,
        &OfflineLicenseAuditEvent {
            occurred_at: timestamp(Utc::now()),
            event_type: event_type.to_string(),
            outcome: outcome.to_string(),
            installation_id: installation_id.map(str::to_string),
            artifact_id: artifact_id.map(str::to_string),
            key_id: key_id.map(str::to_string),
            detail_code: detail_code.map(str::to_string),
        },
    )
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn platform_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else {
        "linux"
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use ed25519_dalek::SigningKey;

    use super::*;
    use crate::db::offline_license::MemoryInstallationSecretStore;
    use crate::db::schema;
    use crate::offline_license::{
        encode_offline_license_v1, encode_revocation_list_v1, OfflineLicensePayloadV1,
        RevocationListPayloadV1,
    };

    fn setup() -> (Connection, std::sync::Arc<MemoryInstallationSecretStore>) {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();
        let store = MemoryInstallationSecretStore::with_secret(vec![7u8; 32]);
        (conn, store)
    }

    fn now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 15, 12, 0, 0).unwrap()
    }

    fn signing_key() -> SigningKey {
        SigningKey::from_bytes(
            &hex::decode("9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60")
                .unwrap()
                .try_into()
                .unwrap(),
        )
    }

    fn license_token(
        conn: &Connection,
        store: &dyn InstallationSecretStore,
        license_id: &str,
        expires_at: &str,
    ) -> String {
        let identity = license_db::get_or_create_installation_identity(conn, store).unwrap();
        encode_offline_license_v1(
            &OfflineLicensePayloadV1 {
                expires_at: expires_at.to_string(),
                installation_id: identity.installation_id,
                issued_at: "2026-07-15T00:00:00Z".to_string(),
                key_id: "offline-test-k0".to_string(),
                license_id: license_id.to_string(),
                not_before: "2026-07-15T00:00:00Z".to_string(),
                product_code: "creator_offline".to_string(),
                schema_version: 1,
            },
            &signing_key(),
        )
        .unwrap()
    }

    fn revocation_token(sequence: u64, list_id: &str, revoked_license_ids: Vec<String>) -> String {
        encode_revocation_list_v1(
            &RevocationListPayloadV1 {
                generated_at: "2026-07-15T01:00:00Z".to_string(),
                key_id: "offline-test-k0".to_string(),
                list_id: list_id.to_string(),
                list_type: "offline_license_revocations".to_string(),
                revoked_license_ids,
                schema_version: 1,
                sequence,
            },
            &signing_key(),
        )
        .unwrap()
    }

    #[test]
    fn db_feature_tamper_does_not_unlock_free_or_cloud_features() {
        let (conn, store) = setup();
        let mut tampered = billing::EntitlementState::default();
        tampered
            .features
            .insert("batch_processing".to_string(), true);
        tampered.features.insert("report_export".to_string(), true);
        tampered.features.insert("cloud_sync".to_string(), true);
        billing::save_entitlement_state(&conn, &tampered).unwrap();

        let effective = resolve_effective_entitlement_at(&conn, store.as_ref(), now()).unwrap();
        assert_eq!(effective.features.get("batch_processing"), Some(&false));
        assert_eq!(effective.features.get("report_export"), Some(&false));
        assert_eq!(effective.features.get("cloud_sync"), Some(&false));
    }

    #[test]
    fn copied_license_is_rejected_by_installation_binding() {
        let (conn, first_store) = setup();
        let token = license_token(
            &conn,
            first_store.as_ref(),
            "lic_k2_copied",
            "2027-07-15T00:00:00Z",
        );
        import_offline_license_at(&conn, first_store.as_ref(), &token, now()).unwrap();

        let copied_machine = MemoryInstallationSecretStore::with_secret(vec![8u8; 32]);
        let effective =
            resolve_effective_entitlement_at(&conn, copied_machine.as_ref(), now()).unwrap();
        assert_eq!(effective.features.get("batch_processing"), Some(&false));
        let status = get_offline_license_status_at(&conn, copied_machine.as_ref(), now()).unwrap();
        assert_eq!(status.status, "invalid");
        assert_eq!(
            status.error_code.as_deref(),
            Some("offline_license_installation_identity_mismatch")
        );
    }

    #[test]
    fn expired_license_does_not_unlock_local_features() {
        let (conn, store) = setup();
        let token = license_token(
            &conn,
            store.as_ref(),
            "lic_k2_expired",
            "2026-07-15T11:00:00Z",
        );
        let parsed = parse_offline_license_v1(&token).unwrap();
        license_db::save_offline_license(
            &conn,
            &StoredOfflineLicense {
                signed_token: token.clone(),
                token_sha256: license_db::token_sha256(&token),
                license_id: parsed.payload.license_id,
                installation_id: parsed.payload.installation_id,
                product_code: parsed.payload.product_code,
                key_id: parsed.payload.key_id,
                issued_at: parsed.payload.issued_at,
                not_before: parsed.payload.not_before,
                expires_at: parsed.payload.expires_at,
                imported_at: "2026-07-15T10:00:00Z".to_string(),
            },
        )
        .unwrap();

        let effective = resolve_effective_entitlement_at(&conn, store.as_ref(), now()).unwrap();
        assert_eq!(effective.features.get("batch_processing"), Some(&false));
        let status = get_offline_license_status_at(&conn, store.as_ref(), now()).unwrap();
        assert_eq!(status.status, "expired");
        assert_eq!(
            status.error_code.as_deref(),
            Some("offline_license_expired")
        );
    }

    #[test]
    fn signed_revocation_disables_an_imported_license() {
        let (conn, store) = setup();
        let token = license_token(
            &conn,
            store.as_ref(),
            "lic_k2_revoked",
            "2027-07-15T00:00:00Z",
        );
        import_offline_license_at(&conn, store.as_ref(), &token, now()).unwrap();
        let revocation_token =
            revocation_token(1, "rvl_k2_0001", vec!["lic_k2_revoked".to_string()]);
        let status =
            import_revocation_list_at(&conn, store.as_ref(), &revocation_token, now()).unwrap();
        assert_eq!(status.status, "revoked");
        let effective = resolve_effective_entitlement_at(&conn, store.as_ref(), now()).unwrap();
        assert_eq!(effective.features.get("report_export"), Some(&false));
    }

    #[test]
    fn offline_license_unlocks_only_local_batch_without_reports() {
        let (conn, store) = setup();
        let token = license_token(
            &conn,
            store.as_ref(),
            "lic_k2_active",
            "2027-07-15T00:00:00Z",
        );
        import_offline_license_at(&conn, store.as_ref(), &token, now()).unwrap();

        let effective = resolve_effective_entitlement_at(&conn, store.as_ref(), now()).unwrap();
        assert_eq!(effective.features.get("batch_processing"), Some(&true));
        assert_eq!(effective.features.get("report_export"), Some(&false));
        for feature in CLOUD_ONLY_FEATURES {
            assert_eq!(effective.features.get(feature), Some(&false), "{feature}");
        }

        let status = get_offline_license_status_at(&conn, store.as_ref(), now()).unwrap();
        assert_eq!(status.features.get("batch_processing"), Some(&true));
        assert_eq!(status.features.get("report_export"), Some(&false));
    }

    #[test]
    fn revocation_high_water_rejects_replay_and_equivocation() {
        let (conn, store) = setup();
        let first = revocation_token(7, "rvl_k4_0007", vec![]);
        import_revocation_list_at(&conn, store.as_ref(), &first, now()).unwrap();

        let idempotent = import_revocation_list_at(&conn, store.as_ref(), &first, now()).unwrap();
        assert_eq!(idempotent.revocation_list_sequence, Some(7));

        let replay = revocation_token(6, "rvl_k4_0006", vec![]);
        assert_eq!(
            import_revocation_list_at(&conn, store.as_ref(), &replay, now()).unwrap_err(),
            "offline_license_revocation_replay"
        );

        let equivocation = revocation_token(7, "rvl_k4_conflict", vec![]);
        assert_eq!(
            import_revocation_list_at(&conn, store.as_ref(), &equivocation, now()).unwrap_err(),
            "offline_license_revocation_equivocation"
        );
    }

    #[test]
    fn trusted_clock_high_water_rejects_large_rollback() {
        let (conn, store) = setup();
        let token = license_token(
            &conn,
            store.as_ref(),
            "lic_k4_clock",
            "2027-07-15T00:00:00Z",
        );
        import_offline_license_at(&conn, store.as_ref(), &token, now()).unwrap();

        let tolerated = now() - chrono::Duration::seconds(300);
        resolve_effective_entitlement_at(&conn, store.as_ref(), tolerated).unwrap();

        conn.execute("DELETE FROM offline_license_security_state", [])
            .unwrap();
        let rolled_back = now() - chrono::Duration::seconds(301);
        assert_eq!(
            resolve_effective_entitlement_at(&conn, store.as_ref(), rolled_back).unwrap_err(),
            "offline_license_clock_rollback"
        );
    }

    #[test]
    fn full_snapshot_rollback_is_known_limit_without_external_anchor() {
        let (current_conn, current_store) = setup();
        let token = license_token(
            &current_conn,
            current_store.as_ref(),
            "lic_k4_full_snapshot",
            "2027-07-15T00:00:00Z",
        );
        import_offline_license_at(&current_conn, current_store.as_ref(), &token, now()).unwrap();
        let snapshot_identity = license_db::load_installation_identity(&current_conn)
            .unwrap()
            .unwrap();
        let snapshot_license = license_db::load_offline_license(&current_conn)
            .unwrap()
            .unwrap();
        let snapshot_highest = license_db::load_highest_observed_utc(&current_conn)
            .unwrap()
            .unwrap();
        let (snapshot_secret, snapshot_anchor) = current_store.snapshot_state();
        resolve_effective_entitlement_at(
            &current_conn,
            current_store.as_ref(),
            now() + chrono::Duration::days(30),
        )
        .unwrap();

        let rolled_back_conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&rolled_back_conn).unwrap();
        let rolled_back_store =
            MemoryInstallationSecretStore::with_state(snapshot_secret, snapshot_anchor);
        rolled_back_conn
            .execute(
                "INSERT INTO installation_identity (
                    id, installation_id, salt_base64_url, secret_fingerprint_sha256,
                    created_at, updated_at
                 ) VALUES (1, ?1, ?2, ?3, ?4, ?5)",
                rusqlite::params![
                    snapshot_identity.installation_id,
                    snapshot_identity.salt_base64_url,
                    snapshot_identity.secret_fingerprint_sha256,
                    snapshot_identity.created_at,
                    snapshot_identity.updated_at,
                ],
            )
            .unwrap();
        license_db::save_offline_license(&rolled_back_conn, &snapshot_license).unwrap();
        license_db::save_highest_observed_utc(
            &rolled_back_conn,
            &snapshot_highest,
            &snapshot_highest,
        )
        .unwrap();
        let rolled_back =
            resolve_effective_entitlement_at(&rolled_back_conn, rolled_back_store.as_ref(), now())
                .unwrap();

        assert_eq!(rolled_back.status, EntitlementStatus::Active);
        assert_eq!(rolled_back.features.get("batch_processing"), Some(&true));
    }

    #[test]
    fn secure_revocation_anchor_detects_database_snapshot_rollback() {
        let (conn, store) = setup();
        let token = license_token(
            &conn,
            store.as_ref(),
            "lic_k4_snapshot",
            "2027-07-15T00:00:00Z",
        );
        import_offline_license_at(&conn, store.as_ref(), &token, now()).unwrap();
        let revocation = revocation_token(7, "rvl_k4_snapshot", vec![]);
        import_revocation_list_at(&conn, store.as_ref(), &revocation, now()).unwrap();

        conn.execute("DELETE FROM offline_revocation_lists", [])
            .unwrap();
        let status = get_offline_license_status_at(&conn, store.as_ref(), now()).unwrap();
        assert_eq!(status.status, "invalid");
        assert_eq!(
            status.error_code.as_deref(),
            Some("offline_license_revocation_state_rollback")
        );
    }

    #[test]
    fn replacing_a_license_appends_transfer_audit() {
        let (conn, store) = setup();
        let first = license_token(&conn, store.as_ref(), "lic_k4_old", "2027-07-15T00:00:00Z");
        let replacement =
            license_token(&conn, store.as_ref(), "lic_k4_new", "2027-07-15T00:00:00Z");
        import_offline_license_at(&conn, store.as_ref(), &first, now()).unwrap();
        import_offline_license_at(&conn, store.as_ref(), &replacement, now()).unwrap();

        let audit: (String, String, String) = conn
            .query_row(
                "SELECT event_type, artifact_id, detail_code
                 FROM offline_license_audit
                 WHERE event_type = 'license_replaced'
                 ORDER BY id DESC
                 LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            audit,
            (
                "license_replaced".to_string(),
                "lic_k4_old".to_string(),
                "lic_k4_new".to_string()
            )
        );
    }

    #[test]
    fn license_state_and_acceptance_audit_commit_atomically() {
        let (conn, store) = setup();
        let token = license_token(
            &conn,
            store.as_ref(),
            "lic_k4_atomic",
            "2027-07-15T00:00:00Z",
        );
        conn.execute_batch(
            "CREATE TRIGGER reject_offline_license_acceptance_audit
             BEFORE INSERT ON offline_license_audit
             WHEN NEW.outcome = 'accepted'
             BEGIN
                 SELECT RAISE(ABORT, 'qa_reject_acceptance_audit');
             END;",
        )
        .unwrap();

        assert!(import_offline_license_at(&conn, store.as_ref(), &token, now()).is_err());
        assert!(license_db::load_offline_license(&conn).unwrap().is_none());
    }
}

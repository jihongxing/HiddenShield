use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{Connection, Executor, PgConnection, Postgres, Row, Transaction};

pub const REASON_SESSION_STATE_INVALID: &str = "ai_session_state_invalid";
pub const REASON_CONFIRMATION_CONFLICT: &str = "ai_confirmation_conflict";
pub const REASON_SUBJECT_DIGEST_INVALID: &str = "ai_subject_digest_invalid";
pub const REASON_EVIDENCE_INVALID: &str = "ai_evidence_invalid";
pub const REASON_MARKER_REQUIREMENT_FAILED: &str = "ai_marker_requirement_failed";
pub const REASON_EXPLICIT_LABEL_REQUIREMENT_FAILED: &str = "ai_explicit_label_requirement_failed";
pub const REASON_LEDGER_WRITE_FAILED: &str = "ai_ledger_write_failed";
pub const REASON_AUDIT_WRITE_FAILED: &str = "audit_write_failed";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum ConfirmFailureInjection {
    None,
    Ledger,
    Audit,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmMarkingCommand {
    pub marking_session_id: String,
    pub transparency_manifest_id: String,
    pub ledger_entry_id: String,
    pub audit_event_id: String,
    pub watermark_uid: String,
    pub subject_digest: String,
    pub generation_mode: String,
    pub provider_id: String,
    pub system_name: String,
    pub system_version: String,
    pub model_id: Option<String>,
    pub model_version: Option<String>,
    pub generated_at: DateTime<Utc>,
    pub operations: Value,
    pub parent_subjects: Value,
    pub profile_statuses: Value,
    pub evidence: ConfirmEvidence,
    pub markers: Vec<ConfirmMarker>,
    pub explicit_label_receipts: Vec<ConfirmExplicitLabelReceipt>,
    pub write_after_read_verified: bool,
    pub failure_injection: ConfirmFailureInjection,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmEvidence {
    pub evidence_id: String,
    pub evidence_level: String,
    pub evidence_source: String,
    pub issuer_id: Option<String>,
    pub key_id: Option<String>,
    pub proof_type: String,
    pub signature_algorithm: Option<String>,
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmMarker {
    pub marker_binding_id: String,
    pub marker_type: String,
    pub marker_profile_id: String,
    pub marker_version: String,
    pub embed_status: String,
    pub verify_status: String,
    pub binding_digest: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfirmExplicitLabelReceipt {
    pub receipt_id: String,
    pub profile_id: String,
    pub required_surface: String,
    pub render_mode: String,
    pub rendered_asset_digest: Option<String>,
    pub placement: Value,
    pub locale: String,
    pub label_text: String,
    pub applied_at: DateTime<Utc>,
    pub applied_by: String,
    pub verification_status: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfirmMarkingOutcome {
    pub succeeded: bool,
    pub reason_code: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum ConfirmMarkingError {
    #[error("PostgreSQL confirm command failed: {0}")]
    Postgres(#[from] sqlx::Error),
}

pub async fn execute_postgres_confirm_marking_command(
    connection: &mut PgConnection,
    command: &ConfirmMarkingCommand,
) -> Result<ConfirmMarkingOutcome, ConfirmMarkingError> {
    if let Some(reason_code) = validate_static(command) {
        return Ok(rejected(reason_code));
    }
    let mut transaction = connection.begin().await?;
    let result = execute_in_transaction(&mut transaction, command).await;
    match result {
        Ok(outcome) if outcome.succeeded => {
            transaction.commit().await?;
            Ok(outcome)
        }
        Ok(outcome) => {
            transaction.rollback().await?;
            Ok(outcome)
        }
        Err(error) if command.failure_injection == ConfirmFailureInjection::Ledger => {
            transaction.rollback().await?;
            let _ = error;
            Ok(rejected(REASON_LEDGER_WRITE_FAILED))
        }
        Err(error) if command.failure_injection == ConfirmFailureInjection::Audit => {
            transaction.rollback().await?;
            let _ = error;
            Ok(rejected(REASON_AUDIT_WRITE_FAILED))
        }
        Err(error) => Err(error.into()),
    }
}

pub(crate) async fn execute_in_transaction(
    transaction: &mut Transaction<'_, Postgres>,
    command: &ConfirmMarkingCommand,
) -> Result<ConfirmMarkingOutcome, sqlx::Error> {
    if let Some(reason_code) = validate_static(command) {
        return Ok(rejected(reason_code));
    }
    let session = sqlx::query(
        "SELECT license_id, status, requested_profile_ids_json, claim_type, provider_content_id
         FROM ai_marking_sessions
         WHERE marking_session_id = $1
         FOR UPDATE",
    )
    .bind(&command.marking_session_id)
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(session) = session else {
        return Ok(rejected(REASON_SESSION_STATE_INVALID));
    };
    let status: String = session.get("status");
    if status == "confirmed" {
        return Ok(rejected(REASON_CONFIRMATION_CONFLICT));
    }
    if status != "ready_to_confirm" {
        return Ok(rejected(REASON_SESSION_STATE_INVALID));
    }
    let requested_profiles: Value = session.get("requested_profile_ids_json");
    if let Some(reason_code) = validate_profiles(&requested_profiles, command) {
        return Ok(rejected(reason_code));
    }
    let license_id: String = session.get("license_id");
    let claim_type: String = session.get("claim_type");
    let provider_content_id: Option<String> = session.get("provider_content_id");
    let now = Utc::now();
    let manifest_sha256 = manifest_digest(command);

    sqlx::query(
        "INSERT INTO ai_transparency_manifests (
            transparency_manifest_id, marking_session_id, watermark_uid, manifest_version, status,
            claim_type, modality, generation_mode, provider_id, system_name, system_version,
            model_id, model_version, operations_json, generated_at, provider_content_id,
            subject_digest_algorithm, subject_digest_scope, subject_digest, parent_subjects_json,
            profile_status_json, manifest_sha256, created_at, updated_at
         ) VALUES (
            $1, $2, $3, 1, 'active', $4, 'image', $5, $6, $7, $8, $9, $10, $11,
            $12, $13, 'sha256', 'protected_output', $14, $15, $16, $17, $18, $18
         )",
    )
    .bind(&command.transparency_manifest_id)
    .bind(&command.marking_session_id)
    .bind(&command.watermark_uid)
    .bind(&claim_type)
    .bind(&command.generation_mode)
    .bind(&command.provider_id)
    .bind(&command.system_name)
    .bind(&command.system_version)
    .bind(&command.model_id)
    .bind(&command.model_version)
    .bind(&command.operations)
    .bind(command.generated_at)
    .bind(provider_content_id)
    .bind(&command.subject_digest)
    .bind(&command.parent_subjects)
    .bind(&command.profile_statuses)
    .bind(manifest_sha256)
    .bind(now)
    .execute(&mut **transaction)
    .await?;

    sqlx::query(
        "INSERT INTO ai_claim_evidence (
            evidence_id, transparency_manifest_id, evidence_level, evidence_source, issuer_id,
            key_id, proof_type, subject_digest, signature_algorithm, signature,
            verification_status, verified_at, created_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'verified',$11,$11)",
    )
    .bind(&command.evidence.evidence_id)
    .bind(&command.transparency_manifest_id)
    .bind(&command.evidence.evidence_level)
    .bind(&command.evidence.evidence_source)
    .bind(&command.evidence.issuer_id)
    .bind(&command.evidence.key_id)
    .bind(&command.evidence.proof_type)
    .bind(&command.subject_digest)
    .bind(&command.evidence.signature_algorithm)
    .bind(&command.evidence.signature)
    .bind(now)
    .execute(&mut **transaction)
    .await?;

    for marker in &command.markers {
        sqlx::query(
            "INSERT INTO ai_marker_bindings (
                marker_binding_id, transparency_manifest_id, marker_type, marker_profile_id,
                marker_version, embed_status, verify_status, binding_digest, created_at
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)",
        )
        .bind(&marker.marker_binding_id)
        .bind(&command.transparency_manifest_id)
        .bind(&marker.marker_type)
        .bind(&marker.marker_profile_id)
        .bind(&marker.marker_version)
        .bind(&marker.embed_status)
        .bind(&marker.verify_status)
        .bind(&marker.binding_digest)
        .bind(now)
        .execute(&mut **transaction)
        .await?;
    }

    for receipt in &command.explicit_label_receipts {
        sqlx::query(
            "INSERT INTO ai_explicit_label_receipts (
                receipt_id, transparency_manifest_id, profile_id, required_surface, render_mode,
                rendered_asset_digest, placement_json, locale, label_text, applied_at, applied_by,
                verification_status, created_at
             ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)",
        )
        .bind(&receipt.receipt_id)
        .bind(&command.transparency_manifest_id)
        .bind(&receipt.profile_id)
        .bind(&receipt.required_surface)
        .bind(&receipt.render_mode)
        .bind(&receipt.rendered_asset_digest)
        .bind(&receipt.placement)
        .bind(&receipt.locale)
        .bind(&receipt.label_text)
        .bind(receipt.applied_at)
        .bind(&receipt.applied_by)
        .bind(&receipt.verification_status)
        .bind(now)
        .execute(&mut **transaction)
        .await?;
    }

    let quantity = if command.failure_injection == ConfirmFailureInjection::Ledger {
        2
    } else {
        1
    };
    sqlx::query(
        "INSERT INTO ai_marking_ledger (
            ledger_entry_id, license_id, marking_session_id, transparency_manifest_id,
            metering_unit, quantity, ledger_status, created_at
         ) VALUES ($1,$2,$3,$4,'confirmed_marked_image',$5,'pending',$6)",
    )
    .bind(&command.ledger_entry_id)
    .bind(&license_id)
    .bind(&command.marking_session_id)
    .bind(&command.transparency_manifest_id)
    .bind(quantity)
    .bind(now)
    .execute(&mut **transaction)
    .await?;

    sqlx::query(
        "UPDATE ai_marking_ledger
         SET ledger_status = 'committed', committed_at = $1
         WHERE ledger_entry_id = $2 AND ledger_status = 'pending'",
    )
    .bind(now)
    .bind(&command.ledger_entry_id)
    .execute(&mut **transaction)
    .await?;

    insert_audit(transaction, command, &license_id, now).await?;
    if command.failure_injection == ConfirmFailureInjection::Audit {
        insert_audit(transaction, command, &license_id, now).await?;
    }

    let updated = sqlx::query(
        "UPDATE ai_marking_sessions
         SET status = 'confirmed', confirmed_at = $1, updated_at = $1
         WHERE marking_session_id = $2 AND status = 'ready_to_confirm'",
    )
    .bind(now)
    .bind(&command.marking_session_id)
    .execute(&mut **transaction)
    .await?
    .rows_affected();
    if updated != 1 {
        return Ok(rejected(REASON_CONFIRMATION_CONFLICT));
    }
    Ok(ConfirmMarkingOutcome {
        succeeded: true,
        reason_code: None,
    })
}

async fn insert_audit(
    transaction: &mut Transaction<'_, Postgres>,
    command: &ConfirmMarkingCommand,
    license_id: &str,
    now: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    transaction
        .execute(
            sqlx::query(
                "INSERT INTO ai_marking_confirm_audit_events (
                    audit_event_id, marking_session_id, transparency_manifest_id,
                    license_id, outcome, subject_digest, occurred_at
                 ) VALUES ($1,$2,$3,$4,'confirmed',$5,$6)",
            )
            .bind(&command.audit_event_id)
            .bind(&command.marking_session_id)
            .bind(&command.transparency_manifest_id)
            .bind(license_id)
            .bind(&command.subject_digest)
            .bind(now),
        )
        .await?;
    Ok(())
}

fn validate_static(command: &ConfirmMarkingCommand) -> Option<&'static str> {
    if !is_lower_hex_digest(&command.subject_digest) {
        return Some(REASON_SUBJECT_DIGEST_INVALID);
    }
    if !command.write_after_read_verified {
        return Some(REASON_EVIDENCE_INVALID);
    }
    if command.evidence.evidence_level.is_empty()
        || command.evidence.proof_type.is_empty()
        || matches!(
            command.evidence.evidence_level.as_str(),
            "platform_signed" | "registry_signed" | "externally_verified"
        ) && (command.evidence.issuer_id.is_none()
            || command.evidence.key_id.is_none()
            || command.evidence.signature_algorithm.is_none()
            || command.evidence.signature.is_none())
    {
        return Some(REASON_EVIDENCE_INVALID);
    }
    if !command.markers.iter().any(|marker| {
        marker.marker_type == "blind_watermark"
            && marker.embed_status == "verified"
            && marker.verify_status == "verified"
    }) {
        return Some(REASON_MARKER_REQUIREMENT_FAILED);
    }
    if command.explicit_label_receipts.iter().any(|receipt| {
        receipt.verification_status != "verified"
            || (receipt.required_surface != "platform_ui"
                && receipt
                    .rendered_asset_digest
                    .as_deref()
                    .is_none_or(|digest| !is_lower_hex_digest(digest)))
    }) {
        return Some(REASON_EXPLICIT_LABEL_REQUIREMENT_FAILED);
    }
    None
}

fn validate_profiles(
    requested_profiles: &Value,
    command: &ConfirmMarkingCommand,
) -> Option<&'static str> {
    let Some(requested_profiles) = requested_profiles.as_array() else {
        return Some(REASON_MARKER_REQUIREMENT_FAILED);
    };
    for profile in requested_profiles {
        let Some(profile_id) = profile.as_str() else {
            return Some(REASON_MARKER_REQUIREMENT_FAILED);
        };
        let status_present = command.profile_statuses.as_array().is_some_and(|statuses| {
            statuses.iter().any(|status| {
                status.get("profileId").and_then(Value::as_str) == Some(profile_id)
                    && status.get("status").and_then(Value::as_str).is_some()
            })
        });
        let marker_present = command.markers.iter().any(|marker| {
            marker.marker_profile_id == profile_id && marker.verify_status == "verified"
        });
        let label_present = command.explicit_label_receipts.iter().any(|receipt| {
            receipt.profile_id == profile_id && receipt.verification_status == "verified"
        });
        if !status_present || (!marker_present && !label_present) {
            return Some(REASON_MARKER_REQUIREMENT_FAILED);
        }
    }
    None
}

fn manifest_digest(command: &ConfirmMarkingCommand) -> String {
    let bytes = serde_json::to_vec(command).expect("confirm command is serializable");
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_lower_hex_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn rejected(reason_code: &'static str) -> ConfirmMarkingOutcome {
    ConfirmMarkingOutcome {
        succeeded: false,
        reason_code: Some(reason_code.to_string()),
    }
}

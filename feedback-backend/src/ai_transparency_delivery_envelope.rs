use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{Connection, PgConnection, Postgres, Row, Transaction};
use uuid::Uuid;
use watermark_core::{
    canonical_json_sha256, seal_ai_delivery_envelope, AiConfirmedArtifactDeliveryEnvelope,
    AiDeliveryProfileIdentity, AI_DELIVERY_ENVELOPE_SCHEMA_VERSION,
};

pub const REASON_DELIVERY_NOT_READY: &str = "ai_delivery_envelope_not_ready";
pub const REASON_DELIVERY_PROFILE_IDENTITY_MISSING: &str =
    "ai_delivery_envelope_profile_identity_missing";
pub const REASON_DELIVERY_RECEIPT_MISSING: &str = "ai_delivery_envelope_receipt_missing";
pub const REASON_DELIVERY_REPLAY_CONFLICT: &str = "ai_delivery_envelope_replay_conflict";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InternalDeliveryEnvelopeOutcome {
    pub succeeded: bool,
    pub replayed: bool,
    pub reason_code: Option<String>,
    pub envelope: Option<AiConfirmedArtifactDeliveryEnvelope>,
    pub signer_receipt_json: Option<String>,
    pub artifact_finalize_receipt_json: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum InternalDeliveryEnvelopeError {
    #[error("PostgreSQL delivery envelope failed: {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("delivery envelope contract failed: {0}")]
    Contract(String),
}

pub async fn execute_postgres_confirmed_delivery_envelope(
    connection: &mut PgConnection,
    execution_id: &str,
) -> Result<InternalDeliveryEnvelopeOutcome, InternalDeliveryEnvelopeError> {
    let mut transaction = connection.begin().await?;
    let row = sqlx::query(
        "SELECT execution.execution_id, execution.marking_session_id, execution.license_id,
                execution.watermark_uid, execution.status AS signing_status,
                execution.artifact_status, execution.recovery_state,
                execution.worker_recovery_attempts, execution.recovery_control_version,
                execution.final_signed_png_sha256, execution.artifact_ref,
                execution.artifact_object_version, execution.signer_receipt_id,
                execution.signer_receipt_json, execution.artifact_finalize_receipt_id,
                execution.artifact_finalize_receipt_json, execution.artifact_finalized_at,
                execution.profile_entitlement_version, execution.profile_entitlement_digest,
                execution.technical_profile_ids_json, execution.regional_profile_id,
                manifest.transparency_manifest_id, manifest.claim_type,
                existing.envelope_json AS existing_envelope_json,
                existing.signer_receipt_sha256 AS existing_signer_receipt_sha256,
                existing.artifact_finalize_receipt_sha256 AS existing_finalize_receipt_sha256
         FROM ai_post_embed_signing_executions execution
         JOIN ai_transparency_manifests manifest
           ON manifest.marking_session_id = execution.marking_session_id
         LEFT JOIN ai_post_embed_delivery_envelopes existing
           ON existing.execution_id = execution.execution_id
         WHERE execution.execution_id = $1
         FOR UPDATE OF execution",
    )
    .bind(execution_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(row) = row else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_DELIVERY_NOT_READY));
    };
    if row.get::<String, _>("signing_status") != "confirmed"
        || row.get::<String, _>("artifact_status") != "finalized"
        || row.get::<String, _>("recovery_state") != "completed"
    {
        transaction.rollback().await?;
        return Ok(rejected(REASON_DELIVERY_NOT_READY));
    }
    let signer_receipt_json: Option<Value> = row.get("signer_receipt_json");
    let finalize_receipt_json: Option<Value> = row.get("artifact_finalize_receipt_json");
    let signer_receipt_id: Option<String> = row.get("signer_receipt_id");
    let finalize_receipt_id: Option<String> = row.get("artifact_finalize_receipt_id");
    let Some(signer_receipt_json) = signer_receipt_json else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_DELIVERY_RECEIPT_MISSING));
    };
    let Some(finalize_receipt_json) = finalize_receipt_json else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_DELIVERY_RECEIPT_MISSING));
    };
    let Some(signer_receipt_id) = signer_receipt_id else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_DELIVERY_RECEIPT_MISSING));
    };
    let Some(finalize_receipt_id) = finalize_receipt_id else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_DELIVERY_RECEIPT_MISSING));
    };
    let profile_entitlement_version: Option<i32> = row.get("profile_entitlement_version");
    let profile_entitlement_digest: Option<String> = row.get("profile_entitlement_digest");
    let technical_profile_ids_json: Option<Value> = row.get("technical_profile_ids_json");
    let regional_profile_id: Option<String> = row.get("regional_profile_id");
    let Some(profile_entitlement_version) = profile_entitlement_version else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_DELIVERY_PROFILE_IDENTITY_MISSING));
    };
    let Some(profile_entitlement_digest) = profile_entitlement_digest else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_DELIVERY_PROFILE_IDENTITY_MISSING));
    };
    let Some(technical_profile_ids_json) = technical_profile_ids_json else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_DELIVERY_PROFILE_IDENTITY_MISSING));
    };
    let Some(regional_profile_id) = regional_profile_id else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_DELIVERY_PROFILE_IDENTITY_MISSING));
    };
    let technical_profile_ids: Vec<String> = serde_json::from_value(technical_profile_ids_json)
        .map_err(|error| {
            InternalDeliveryEnvelopeError::Contract(format!(
                "decode technical profile identity: {error}"
            ))
        })?;
    let signer_receipt_text = serde_json::to_string(&signer_receipt_json).map_err(|error| {
        InternalDeliveryEnvelopeError::Contract(format!("encode signer receipt: {error}"))
    })?;
    let finalize_receipt_text = serde_json::to_string(&finalize_receipt_json).map_err(|error| {
        InternalDeliveryEnvelopeError::Contract(format!("encode finalize receipt: {error}"))
    })?;
    let signer_receipt_sha256 = canonical_json_sha256(&signer_receipt_text)
        .map_err(|error| InternalDeliveryEnvelopeError::Contract(error.to_string()))?;
    let finalize_receipt_sha256 = canonical_json_sha256(&finalize_receipt_text)
        .map_err(|error| InternalDeliveryEnvelopeError::Contract(error.to_string()))?;
    if let Some(existing_json) = row.get::<Option<Value>, _>("existing_envelope_json") {
        let envelope: AiConfirmedArtifactDeliveryEnvelope = serde_json::from_value(existing_json)
            .map_err(|error| {
            InternalDeliveryEnvelopeError::Contract(format!(
                "decode existing delivery envelope: {error}"
            ))
        })?;
        if row.get::<Option<String>, _>("existing_signer_receipt_sha256")
            != Some(signer_receipt_sha256)
            || row.get::<Option<String>, _>("existing_finalize_receipt_sha256")
                != Some(finalize_receipt_sha256)
        {
            transaction.rollback().await?;
            return Ok(rejected(REASON_DELIVERY_REPLAY_CONFLICT));
        }
        transaction.commit().await?;
        return Ok(success(
            envelope,
            signer_receipt_text,
            finalize_receipt_text,
            true,
        ));
    }
    let finalized_at: DateTime<Utc> = row.get("artifact_finalized_at");
    let envelope = seal_ai_delivery_envelope(AiConfirmedArtifactDeliveryEnvelope {
        schema_version: AI_DELIVERY_ENVELOPE_SCHEMA_VERSION.to_string(),
        delivery_envelope_id: format!("delivery-envelope-{}", Uuid::new_v4()),
        execution_id: row.get("execution_id"),
        marking_session_id: row.get("marking_session_id"),
        transparency_manifest_id: row.get("transparency_manifest_id"),
        license_id: row.get("license_id"),
        watermark_uid: row.get("watermark_uid"),
        media_type: "image/png".to_string(),
        claim_type: row.get("claim_type"),
        signing_status: row.get("signing_status"),
        artifact_status: row.get("artifact_status"),
        recovery_state: row.get("recovery_state"),
        worker_recovery_attempts: row.get("worker_recovery_attempts"),
        recovery_control_version: row.get("recovery_control_version"),
        final_file_sha256: row.get("final_signed_png_sha256"),
        artifact_ref: row.get("artifact_ref"),
        artifact_object_version: row.get("artifact_object_version"),
        signer_receipt_id,
        signer_receipt_sha256,
        artifact_finalize_receipt_id: finalize_receipt_id,
        artifact_finalize_receipt_sha256: finalize_receipt_sha256,
        profile_identity: AiDeliveryProfileIdentity {
            entitlement_version: profile_entitlement_version,
            entitlement_digest: profile_entitlement_digest,
            technical_profile_ids,
            regional_profile_id,
        },
        profile_identity_digest: String::new(),
        finalized_at: finalized_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        envelope_digest: String::new(),
    })
    .map_err(|error| InternalDeliveryEnvelopeError::Contract(error.to_string()))?;
    insert_delivery_envelope(&mut transaction, &envelope).await?;
    transaction.commit().await?;
    Ok(success(
        envelope,
        signer_receipt_text,
        finalize_receipt_text,
        false,
    ))
}

async fn insert_delivery_envelope(
    transaction: &mut Transaction<'_, Postgres>,
    envelope: &AiConfirmedArtifactDeliveryEnvelope,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_post_embed_delivery_envelopes (
            delivery_envelope_id, execution_id, schema_version, final_file_sha256,
            signer_receipt_id, signer_receipt_sha256, artifact_finalize_receipt_id,
            artifact_finalize_receipt_sha256, profile_identity_digest,
            recovery_control_version, envelope_digest, envelope_json, created_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,NOW())",
    )
    .bind(&envelope.delivery_envelope_id)
    .bind(&envelope.execution_id)
    .bind(&envelope.schema_version)
    .bind(&envelope.final_file_sha256)
    .bind(&envelope.signer_receipt_id)
    .bind(&envelope.signer_receipt_sha256)
    .bind(&envelope.artifact_finalize_receipt_id)
    .bind(&envelope.artifact_finalize_receipt_sha256)
    .bind(&envelope.profile_identity_digest)
    .bind(envelope.recovery_control_version)
    .bind(&envelope.envelope_digest)
    .bind(serde_json::to_value(envelope).expect("serializable delivery envelope"))
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn success(
    envelope: AiConfirmedArtifactDeliveryEnvelope,
    signer_receipt_json: String,
    artifact_finalize_receipt_json: String,
    replayed: bool,
) -> InternalDeliveryEnvelopeOutcome {
    InternalDeliveryEnvelopeOutcome {
        succeeded: true,
        replayed,
        reason_code: None,
        envelope: Some(envelope),
        signer_receipt_json: Some(signer_receipt_json),
        artifact_finalize_receipt_json: Some(artifact_finalize_receipt_json),
    }
}

fn rejected(reason_code: &str) -> InternalDeliveryEnvelopeOutcome {
    InternalDeliveryEnvelopeOutcome {
        succeeded: false,
        replayed: false,
        reason_code: Some(reason_code.to_string()),
        envelope: None,
        signer_receipt_json: None,
        artifact_finalize_receipt_json: None,
    }
}

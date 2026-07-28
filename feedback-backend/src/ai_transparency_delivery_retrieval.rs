use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use chrono::{Duration, SecondsFormat, Utc};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection, Postgres, Row, Transaction};
use std::time::Duration as StdDuration;
use uuid::Uuid;
use watermark_core::{
    seal_ai_delivery_retrieval_receipt, validate_ai_delivery_envelope,
    AiConfirmedArtifactDeliveryEnvelope, AiDeliveryRetrievalReceipt,
    AI_DELIVERY_RETRIEVAL_RECEIPT_SCHEMA_VERSION,
};

use crate::ai_transparency_change_command::{ActorAuthorizationInput, ChangeCommandPreflight};

pub const REASON_RETRIEVAL_AUTHORIZED: &str = "ai_delivery_retrieval_authorized";
pub const REASON_RETRIEVAL_SUCCEEDED: &str = "ai_delivery_retrieval_succeeded";
pub const REASON_RETRIEVAL_INVALID: &str = "ai_delivery_retrieval_invalid";
pub const REASON_RETRIEVAL_EXPIRED: &str = "ai_delivery_retrieval_expired";
pub const REASON_RETRIEVAL_ENTITLEMENT_INVALID: &str = "ai_delivery_retrieval_entitlement_invalid";
pub const REASON_RETRIEVAL_ARTIFACT_UNAVAILABLE: &str =
    "ai_delivery_retrieval_artifact_unavailable";
pub const REASON_RETRIEVAL_BRIDGE_REJECTED: &str = "ai_delivery_retrieval_bridge_rejected";
pub const REASON_RETRIEVAL_RATE_LIMITED: &str = "ai_delivery_retrieval_rate_limited";
pub const REASON_RETRIEVAL_SIZE_LIMIT_EXCEEDED: &str = "ai_delivery_retrieval_size_limit_exceeded";
pub const REASON_RETRIEVAL_CONTENT_TYPE_INVALID: &str =
    "ai_delivery_retrieval_content_type_invalid";
pub const REASON_RETRIEVAL_READ_TIMEOUT: &str = "ai_delivery_retrieval_read_timeout";
pub const REASON_AUTHORIZATION_REVOKED: &str = "ai_delivery_authorization_revoked";
pub const DELIVERY_MAX_DOWNLOAD_BYTES: i64 = 64 * 1024 * 1024;
pub const DELIVERY_REQUIRED_CONTENT_TYPE: &str = "image/png";
pub const DELIVERY_READ_TIMEOUT_MS: i32 = 5_000;
pub const DELIVERY_RATE_LIMIT_PER_MINUTE: i32 = 30;

#[derive(Debug, Clone)]
pub struct CreateDeliveryAuthorizationCommand {
    pub delivery_envelope_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub requester_snapshot_id: String,
    pub requester_token_hash: String,
    pub ttl_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryAuthorizationGrant {
    pub authorization_id: String,
    pub delivery_envelope_id: String,
    pub retrieval_token: String,
    pub expires_at: String,
    pub envelope_digest: String,
    pub max_download_bytes: i64,
    pub required_content_type: String,
    pub read_timeout_ms: i32,
    pub rate_limit_per_minute: i32,
}

#[derive(Debug, Clone)]
pub struct RetrieveDeliveryCommand {
    pub authorization_id: String,
    pub retrieval_token: String,
}

#[derive(Debug, Clone)]
pub struct RevokeDeliveryAuthorizationCommand {
    pub authorization_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub revoker_snapshot_id: String,
    pub revoker_token_hash: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryAuthorizationRevocationOutcome {
    pub succeeded: bool,
    pub replayed: bool,
    pub reason_code: Option<String>,
    pub authorization_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliveryDownloadBudget {
    pub max_bytes: i64,
    pub required_content_type: &'static str,
    pub read_timeout: StdDuration,
    pub rate_limit_per_minute: i32,
}

impl DeliveryDownloadBudget {
    pub fn production_v1() -> Self {
        Self {
            max_bytes: DELIVERY_MAX_DOWNLOAD_BYTES,
            required_content_type: DELIVERY_REQUIRED_CONTENT_TYPE,
            read_timeout: StdDuration::from_millis(DELIVERY_READ_TIMEOUT_MS as u64),
            rate_limit_per_minute: DELIVERY_RATE_LIMIT_PER_MINUTE,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeliveryArtifactObject {
    pub bytes: Vec<u8>,
    pub content_type: String,
    pub content_length_bytes: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliveryArtifactReadFailure {
    Unavailable,
    TimedOut,
}

pub trait DeliveryArtifactRetriever: Send + Sync {
    fn load_finalized_for_delivery(
        &self,
        execution_id: &str,
        artifact_ref: &str,
        budget: DeliveryDownloadBudget,
    ) -> Result<DeliveryArtifactObject, DeliveryArtifactReadFailure>;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VerifiedDeliveryRetrievalPackage {
    pub final_media_bytes: Vec<u8>,
    pub envelope_json: String,
    pub signer_receipt_json: String,
    pub artifact_finalize_receipt_json: String,
    pub retrieval_receipt: AiDeliveryRetrievalReceipt,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryRetrievalOutcome {
    pub succeeded: bool,
    pub reason_code: Option<String>,
    pub grant: Option<DeliveryAuthorizationGrant>,
    pub package: Option<VerifiedDeliveryRetrievalPackage>,
}

#[derive(Debug, thiserror::Error)]
pub enum DeliveryRetrievalError {
    #[error("PostgreSQL delivery retrieval failed: {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("delivery retrieval contract failed: {0}")]
    Contract(String),
}

pub async fn execute_postgres_create_delivery_authorization(
    connection: &mut PgConnection,
    command: &CreateDeliveryAuthorizationCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<DeliveryRetrievalOutcome, DeliveryRetrievalError> {
    if command.ttl_seconds < 60 || command.ttl_seconds > 900 {
        return Ok(rejected(REASON_RETRIEVAL_INVALID));
    }
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.requester_token_hash,
            required_role: "ai_transparency_delivery_operator",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "create_ai_delivery_retrieval_authorization",
        });
    if !decision.authorized {
        return Ok(rejected(
            decision.reason_code.as_deref().unwrap_or("iam_unavailable"),
        ));
    }
    let mut transaction = connection.begin().await?;
    let row = sqlx::query(
        "SELECT delivery.delivery_envelope_id, delivery.envelope_digest,
                delivery.artifact_finalize_receipt_sha256, delivery.envelope_json,
                execution.execution_id, execution.license_id, execution.final_signed_png_sha256,
                license.tenant_id, license.workspace_id, license.environment
         FROM ai_post_embed_delivery_envelopes delivery
         JOIN ai_post_embed_signing_executions execution
           ON execution.execution_id = delivery.execution_id
         JOIN ai_transparency_licenses license ON license.license_id = execution.license_id
         JOIN ai_transparency_actor_role_snapshots requester
           ON requester.actor_role_snapshot_id = $5
         WHERE delivery.delivery_envelope_id = $1
           AND license.status = 'active'
           AND license.effective_at <= NOW() AND license.expires_at > NOW()
           AND license.tenant_id = $2 AND license.workspace_id = $3
           AND license.environment = $4
           AND requester.role = 'ai_transparency_delivery_operator'
           AND requester.tenant_id = $2 AND requester.workspace_id = $3
           AND requester.environment = $4
           AND requester.source_expires_at > NOW()
         FOR SHARE OF delivery, execution, license, requester",
    )
    .bind(&command.delivery_envelope_id)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(&command.requester_snapshot_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(row) = row else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_RETRIEVAL_ENTITLEMENT_INVALID));
    };
    let envelope: AiConfirmedArtifactDeliveryEnvelope =
        serde_json::from_value(row.get("envelope_json")).map_err(|error| {
            DeliveryRetrievalError::Contract(format!("decode delivery envelope: {error}"))
        })?;
    if !profile_entitlements_active(
        &mut transaction,
        &row.get::<String, _>("license_id"),
        &envelope,
    )
    .await?
    {
        transaction.rollback().await?;
        return Ok(rejected(REASON_RETRIEVAL_ENTITLEMENT_INVALID));
    }
    let authorization_id = format!("delivery-auth-{}", Uuid::new_v4());
    let retrieval_token = new_retrieval_token();
    let token_hash = sha256_hex(retrieval_token.as_bytes());
    let expires_at = Utc::now() + Duration::seconds(command.ttl_seconds);
    sqlx::query(
        "INSERT INTO ai_delivery_retrieval_authorizations (
            authorization_id, delivery_envelope_id, license_id, tenant_id, workspace_id,
            environment, requester_snapshot_id, token_hash, envelope_digest,
            artifact_finalize_receipt_sha256, status, granted_at, expires_at, created_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'active',NOW(),$11,NOW())",
    )
    .bind(&authorization_id)
    .bind(&command.delivery_envelope_id)
    .bind(row.get::<String, _>("license_id"))
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(&command.requester_snapshot_id)
    .bind(token_hash)
    .bind(row.get::<String, _>("envelope_digest"))
    .bind(row.get::<String, _>("artifact_finalize_receipt_sha256"))
    .bind(expires_at)
    .execute(&mut *transaction)
    .await?;
    insert_download_audit(
        &mut transaction,
        Some(&authorization_id),
        &command.delivery_envelope_id,
        &row.get::<String, _>("execution_id"),
        "authorization_granted",
        "succeeded",
        REASON_RETRIEVAL_AUTHORIZED,
        &row.get::<String, _>("envelope_digest"),
        &row.get::<String, _>("final_signed_png_sha256"),
        json!({"ttlSeconds": command.ttl_seconds}),
    )
    .await?;
    transaction.commit().await?;
    Ok(DeliveryRetrievalOutcome {
        succeeded: true,
        reason_code: None,
        grant: Some(DeliveryAuthorizationGrant {
            authorization_id,
            delivery_envelope_id: command.delivery_envelope_id.clone(),
            retrieval_token,
            expires_at: expires_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            envelope_digest: envelope.envelope_digest,
            max_download_bytes: DELIVERY_MAX_DOWNLOAD_BYTES,
            required_content_type: DELIVERY_REQUIRED_CONTENT_TYPE.to_string(),
            read_timeout_ms: DELIVERY_READ_TIMEOUT_MS,
            rate_limit_per_minute: DELIVERY_RATE_LIMIT_PER_MINUTE,
        }),
        package: None,
    })
}

pub async fn execute_postgres_retrieve_delivery(
    connection: &mut PgConnection,
    command: &RetrieveDeliveryCommand,
    artifact_retriever: &dyn DeliveryArtifactRetriever,
) -> Result<DeliveryRetrievalOutcome, DeliveryRetrievalError> {
    let mut transaction = connection.begin().await?;
    let row = sqlx::query(
        "SELECT retrieval_auth.authorization_id, retrieval_auth.status,
                retrieval_auth.expires_at AS authorization_expires_at,
                retrieval_auth.token_hash,
                retrieval_auth.envelope_digest AS authorized_envelope_digest,
                retrieval_auth.artifact_finalize_receipt_sha256 AS authorized_finalize_digest,
                retrieval_auth.max_download_bytes, retrieval_auth.required_content_type,
                retrieval_auth.read_timeout_ms, retrieval_auth.rate_limit_per_minute,
                delivery.delivery_envelope_id, delivery.envelope_digest, delivery.envelope_json,
                delivery.artifact_finalize_receipt_sha256,
                execution.execution_id, execution.license_id, execution.artifact_ref,
                execution.final_signed_png_sha256, execution.signer_receipt_json,
                execution.artifact_finalize_receipt_json,
                license.status AS license_status,
                license.effective_at AS license_effective_at,
                license.expires_at AS license_expires_at
         FROM ai_delivery_retrieval_authorizations retrieval_auth
         JOIN ai_post_embed_delivery_envelopes delivery
           ON delivery.delivery_envelope_id = retrieval_auth.delivery_envelope_id
         JOIN ai_post_embed_signing_executions execution
           ON execution.execution_id = delivery.execution_id
         JOIN ai_transparency_licenses license ON license.license_id = execution.license_id
         WHERE retrieval_auth.authorization_id = $1
         FOR UPDATE OF retrieval_auth",
    )
    .bind(&command.authorization_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(row) = row else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_RETRIEVAL_INVALID));
    };
    let envelope: AiConfirmedArtifactDeliveryEnvelope =
        serde_json::from_value(row.get("envelope_json")).map_err(|error| {
            DeliveryRetrievalError::Contract(format!("decode delivery envelope: {error}"))
        })?;
    let authorization_status: String = row.get("status");
    let expired = row.get::<chrono::DateTime<Utc>, _>("authorization_expires_at") <= Utc::now();
    let invalid = authorization_status != "active"
        || sha256_hex(command.retrieval_token.as_bytes()) != row.get::<String, _>("token_hash")
        || row.get::<String, _>("license_status") != "active"
        || row.get::<chrono::DateTime<Utc>, _>("license_effective_at") > Utc::now()
        || row.get::<chrono::DateTime<Utc>, _>("license_expires_at") <= Utc::now()
        || row.get::<String, _>("authorized_envelope_digest")
            != row.get::<String, _>("envelope_digest")
        || row.get::<String, _>("authorized_finalize_digest")
            != row.get::<String, _>("artifact_finalize_receipt_sha256");
    if expired || invalid {
        let reason_code = if authorization_status == "revoked" {
            REASON_AUTHORIZATION_REVOKED
        } else if expired {
            REASON_RETRIEVAL_EXPIRED
        } else {
            REASON_RETRIEVAL_INVALID
        };
        if expired && authorization_status == "active" {
            sqlx::query(
                "UPDATE ai_delivery_retrieval_authorizations
                 SET status = 'expired'
                 WHERE authorization_id = $1 AND status = 'active'",
            )
            .bind(&command.authorization_id)
            .execute(&mut *transaction)
            .await?;
        }
        insert_download_audit(
            &mut transaction,
            Some(&command.authorization_id),
            &row.get::<String, _>("delivery_envelope_id"),
            &row.get::<String, _>("execution_id"),
            "retrieval_failed",
            "denied",
            reason_code,
            &row.get::<String, _>("envelope_digest"),
            &row.get::<String, _>("final_signed_png_sha256"),
            json!({}),
        )
        .await?;
        transaction.commit().await?;
        return Ok(rejected(reason_code));
    }
    if !profile_entitlements_active(
        &mut transaction,
        &row.get::<String, _>("license_id"),
        &envelope,
    )
    .await?
    {
        insert_download_audit(
            &mut transaction,
            Some(&command.authorization_id),
            &row.get::<String, _>("delivery_envelope_id"),
            &row.get::<String, _>("execution_id"),
            "retrieval_failed",
            "denied",
            REASON_RETRIEVAL_ENTITLEMENT_INVALID,
            &row.get::<String, _>("envelope_digest"),
            &row.get::<String, _>("final_signed_png_sha256"),
            json!({}),
        )
        .await?;
        transaction.commit().await?;
        return Ok(rejected(REASON_RETRIEVAL_ENTITLEMENT_INVALID));
    }
    let budget = DeliveryDownloadBudget {
        max_bytes: row.get("max_download_bytes"),
        required_content_type: DELIVERY_REQUIRED_CONTENT_TYPE,
        read_timeout: StdDuration::from_millis(row.get::<i32, _>("read_timeout_ms") as u64),
        rate_limit_per_minute: row.get("rate_limit_per_minute"),
    };
    if row.get::<String, _>("required_content_type") != budget.required_content_type
        || budget != DeliveryDownloadBudget::production_v1()
    {
        insert_download_audit(
            &mut transaction,
            Some(&command.authorization_id),
            &row.get::<String, _>("delivery_envelope_id"),
            &row.get::<String, _>("execution_id"),
            "retrieval_failed",
            "denied",
            REASON_RETRIEVAL_INVALID,
            &row.get::<String, _>("envelope_digest"),
            &row.get::<String, _>("final_signed_png_sha256"),
            json!({}),
        )
        .await?;
        transaction.commit().await?;
        return Ok(rejected(REASON_RETRIEVAL_INVALID));
    }
    if !claim_download_rate_limit(
        &mut transaction,
        &row.get::<String, _>("license_id"),
        budget.rate_limit_per_minute,
    )
    .await?
    {
        insert_download_audit(
            &mut transaction,
            Some(&command.authorization_id),
            &row.get::<String, _>("delivery_envelope_id"),
            &row.get::<String, _>("execution_id"),
            "retrieval_failed",
            "denied",
            REASON_RETRIEVAL_RATE_LIMITED,
            &row.get::<String, _>("envelope_digest"),
            &row.get::<String, _>("final_signed_png_sha256"),
            json!({"rateLimitPerMinute": budget.rate_limit_per_minute}),
        )
        .await?;
        transaction.commit().await?;
        return Ok(rejected(REASON_RETRIEVAL_RATE_LIMITED));
    }
    sqlx::query(
        "UPDATE ai_delivery_retrieval_authorizations
         SET status = 'consumed', consumed_at = NOW()
         WHERE authorization_id = $1 AND status = 'active'",
    )
    .bind(&command.authorization_id)
    .execute(&mut *transaction)
    .await?;
    insert_download_audit(
        &mut transaction,
        Some(&command.authorization_id),
        &row.get::<String, _>("delivery_envelope_id"),
        &row.get::<String, _>("execution_id"),
        "retrieval_claimed",
        "succeeded",
        "ai_delivery_retrieval_claimed",
        &row.get::<String, _>("envelope_digest"),
        &row.get::<String, _>("final_signed_png_sha256"),
        json!({
            "maxDownloadBytes": budget.max_bytes,
            "requiredContentType": budget.required_content_type,
            "readTimeoutMs": budget.read_timeout.as_millis(),
            "rateLimitPerMinute": budget.rate_limit_per_minute
        }),
    )
    .await?;
    transaction.commit().await?;

    let execution_id: String = row.get("execution_id");
    let artifact_ref: String = row.get("artifact_ref");
    let artifact = match artifact_retriever.load_finalized_for_delivery(
        &execution_id,
        &artifact_ref,
        budget,
    ) {
        Ok(artifact) => artifact,
        Err(DeliveryArtifactReadFailure::Unavailable) => {
            record_retrieval_failure(connection, &row, REASON_RETRIEVAL_ARTIFACT_UNAVAILABLE)
                .await?;
            return Ok(rejected(REASON_RETRIEVAL_ARTIFACT_UNAVAILABLE));
        }
        Err(DeliveryArtifactReadFailure::TimedOut) => {
            record_retrieval_failure(connection, &row, REASON_RETRIEVAL_READ_TIMEOUT).await?;
            return Ok(rejected(REASON_RETRIEVAL_READ_TIMEOUT));
        }
    };
    if artifact.content_length_bytes > budget.max_bytes
        || artifact.bytes.len() as i64 > budget.max_bytes
        || artifact.content_length_bytes != artifact.bytes.len() as i64
    {
        record_retrieval_failure(connection, &row, REASON_RETRIEVAL_SIZE_LIMIT_EXCEEDED).await?;
        return Ok(rejected(REASON_RETRIEVAL_SIZE_LIMIT_EXCEEDED));
    }
    if artifact.content_type != budget.required_content_type {
        record_retrieval_failure(connection, &row, REASON_RETRIEVAL_CONTENT_TYPE_INVALID).await?;
        return Ok(rejected(REASON_RETRIEVAL_CONTENT_TYPE_INVALID));
    }
    let final_media_bytes = artifact.bytes;
    let signer_receipt_json = serde_json::to_string(&row.get::<Value, _>("signer_receipt_json"))
        .map_err(|error| {
            DeliveryRetrievalError::Contract(format!("encode signer receipt: {error}"))
        })?;
    let finalize_receipt_json =
        serde_json::to_string(&row.get::<Value, _>("artifact_finalize_receipt_json")).map_err(
            |error| DeliveryRetrievalError::Contract(format!("encode finalize receipt: {error}")),
        )?;
    if validate_ai_delivery_envelope(
        &envelope,
        &final_media_bytes,
        &signer_receipt_json,
        &finalize_receipt_json,
    )
    .is_err()
    {
        record_retrieval_failure(connection, &row, REASON_RETRIEVAL_BRIDGE_REJECTED).await?;
        return Ok(rejected(REASON_RETRIEVAL_BRIDGE_REJECTED));
    }
    let retrieved_at = Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true);
    let receipt = seal_ai_delivery_retrieval_receipt(AiDeliveryRetrievalReceipt {
        schema_version: AI_DELIVERY_RETRIEVAL_RECEIPT_SCHEMA_VERSION.to_string(),
        retrieval_receipt_id: format!("delivery-retrieval-{}", Uuid::new_v4()),
        authorization_id: command.authorization_id.clone(),
        delivery_envelope_id: row.get("delivery_envelope_id"),
        execution_id,
        envelope_digest: envelope.envelope_digest.clone(),
        final_file_sha256: envelope.final_file_sha256.clone(),
        artifact_finalize_receipt_sha256: envelope.artifact_finalize_receipt_sha256.clone(),
        retrieved_at,
        receipt_digest: String::new(),
    })
    .map_err(|error| DeliveryRetrievalError::Contract(error.to_string()))?;
    let mut audit_transaction = connection.begin().await?;
    insert_download_audit(
        &mut audit_transaction,
        Some(&command.authorization_id),
        &receipt.delivery_envelope_id,
        &receipt.execution_id,
        "retrieval_succeeded",
        "succeeded",
        REASON_RETRIEVAL_SUCCEEDED,
        &receipt.envelope_digest,
        &receipt.final_file_sha256,
        json!({"retrievalReceiptDigest": receipt.receipt_digest}),
    )
    .await?;
    audit_transaction.commit().await?;
    Ok(DeliveryRetrievalOutcome {
        succeeded: true,
        reason_code: None,
        grant: None,
        package: Some(VerifiedDeliveryRetrievalPackage {
            final_media_bytes,
            envelope_json: serde_json::to_string(&envelope).map_err(|error| {
                DeliveryRetrievalError::Contract(format!("encode envelope: {error}"))
            })?,
            signer_receipt_json,
            artifact_finalize_receipt_json: finalize_receipt_json,
            retrieval_receipt: receipt,
        }),
    })
}

pub async fn execute_postgres_revoke_delivery_authorization(
    connection: &mut PgConnection,
    command: &RevokeDeliveryAuthorizationCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<DeliveryAuthorizationRevocationOutcome, DeliveryRetrievalError> {
    if command.reason.trim().is_empty() || command.reason.len() > 512 {
        return Ok(revoke_rejected(REASON_RETRIEVAL_INVALID));
    }
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.revoker_token_hash,
            required_role: "ai_transparency_security_approver",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "revoke_ai_delivery_retrieval_authorization",
        });
    if !decision.authorized {
        return Ok(revoke_rejected(
            decision.reason_code.as_deref().unwrap_or("iam_unavailable"),
        ));
    }
    let mut transaction = connection.begin().await?;
    let row = sqlx::query(
        "SELECT retrieval_auth.authorization_id, retrieval_auth.status,
                retrieval_auth.delivery_envelope_id, retrieval_auth.envelope_digest,
                execution.execution_id, execution.final_signed_png_sha256
         FROM ai_delivery_retrieval_authorizations retrieval_auth
         JOIN ai_post_embed_delivery_envelopes delivery
           ON delivery.delivery_envelope_id = retrieval_auth.delivery_envelope_id
         JOIN ai_post_embed_signing_executions execution
           ON execution.execution_id = delivery.execution_id
         JOIN ai_transparency_actor_role_snapshots revoker
           ON revoker.actor_role_snapshot_id = $5
         WHERE retrieval_auth.authorization_id = $1
           AND retrieval_auth.tenant_id = $2
           AND retrieval_auth.workspace_id = $3
           AND retrieval_auth.environment = $4
           AND revoker.role = 'ai_transparency_security_approver'
           AND revoker.tenant_id = $2 AND revoker.workspace_id = $3
           AND revoker.environment = $4
           AND revoker.source_expires_at > NOW()
         FOR UPDATE OF retrieval_auth, revoker",
    )
    .bind(&command.authorization_id)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(&command.revoker_snapshot_id)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(row) = row else {
        transaction.rollback().await?;
        return Ok(revoke_rejected(REASON_RETRIEVAL_INVALID));
    };
    let status: String = row.get("status");
    if status == "revoked" {
        transaction.rollback().await?;
        return Ok(DeliveryAuthorizationRevocationOutcome {
            succeeded: true,
            replayed: true,
            reason_code: None,
            authorization_id: Some(command.authorization_id.clone()),
        });
    }
    if status != "active" {
        transaction.rollback().await?;
        return Ok(revoke_rejected(REASON_RETRIEVAL_INVALID));
    }
    sqlx::query(
        "UPDATE ai_delivery_retrieval_authorizations
         SET status = 'revoked', revoked_at = NOW(), revoked_by_snapshot_id = $2,
             revoke_reason = $3
         WHERE authorization_id = $1 AND status = 'active'",
    )
    .bind(&command.authorization_id)
    .bind(&command.revoker_snapshot_id)
    .bind(command.reason.trim())
    .execute(&mut *transaction)
    .await?;
    insert_download_audit(
        &mut transaction,
        Some(&command.authorization_id),
        &row.get::<String, _>("delivery_envelope_id"),
        &row.get::<String, _>("execution_id"),
        "authorization_revoked",
        "succeeded",
        REASON_AUTHORIZATION_REVOKED,
        &row.get::<String, _>("envelope_digest"),
        &row.get::<String, _>("final_signed_png_sha256"),
        json!({
            "revokerSnapshotId": command.revoker_snapshot_id,
            "revokeReasonSha256": sha256_hex(command.reason.trim().as_bytes())
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(DeliveryAuthorizationRevocationOutcome {
        succeeded: true,
        replayed: false,
        reason_code: None,
        authorization_id: Some(command.authorization_id.clone()),
    })
}

async fn claim_download_rate_limit(
    transaction: &mut Transaction<'_, Postgres>,
    license_id: &str,
    limit: i32,
) -> Result<bool, sqlx::Error> {
    let claim_count: Option<i32> = sqlx::query_scalar(
        "INSERT INTO ai_delivery_download_rate_limit_windows (
            license_id, window_started_at, claim_count, updated_at
         ) VALUES ($1, date_trunc('minute', NOW()), 1, NOW())
         ON CONFLICT (license_id, window_started_at)
         DO UPDATE SET claim_count = ai_delivery_download_rate_limit_windows.claim_count + 1,
                       updated_at = NOW()
         WHERE ai_delivery_download_rate_limit_windows.claim_count < $2
         RETURNING claim_count",
    )
    .bind(license_id)
    .bind(limit)
    .fetch_optional(&mut **transaction)
    .await?;
    Ok(claim_count.is_some())
}

async fn profile_entitlements_active(
    transaction: &mut Transaction<'_, Postgres>,
    license_id: &str,
    envelope: &AiConfirmedArtifactDeliveryEnvelope,
) -> Result<bool, sqlx::Error> {
    let mut profile_ids = envelope.profile_identity.technical_profile_ids.clone();
    profile_ids.push(envelope.profile_identity.regional_profile_id.clone());
    let count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT entitlement.profile_id)
         FROM ai_profile_entitlements entitlement
         JOIN ai_profile_entitlement_versions versioned
           ON versioned.profile_entitlement_version_id = entitlement.current_version_id
         WHERE entitlement.license_id = $1
           AND entitlement.profile_id = ANY($2)
           AND entitlement.status = 'active'
           AND entitlement.effective_at <= NOW() AND entitlement.expires_at > NOW()
           AND versioned.status = 'active'
           AND versioned.effective_at <= NOW() AND versioned.expires_at > NOW()
           AND versioned.version = $3",
    )
    .bind(license_id)
    .bind(&profile_ids)
    .bind(envelope.profile_identity.entitlement_version)
    .fetch_one(&mut **transaction)
    .await?;
    Ok(count == profile_ids.len() as i64)
}

async fn insert_download_audit(
    transaction: &mut Transaction<'_, Postgres>,
    authorization_id: Option<&str>,
    delivery_envelope_id: &str,
    execution_id: &str,
    event_type: &str,
    outcome: &str,
    reason_code: &str,
    envelope_digest: &str,
    final_file_sha256: &str,
    details: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_delivery_download_audit_events (
            download_audit_event_id, authorization_id, delivery_envelope_id, execution_id,
            event_type, outcome, reason_code, envelope_digest, final_file_sha256,
            details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,NOW())",
    )
    .bind(format!("delivery-download-audit-{}", Uuid::new_v4()))
    .bind(authorization_id)
    .bind(delivery_envelope_id)
    .bind(execution_id)
    .bind(event_type)
    .bind(outcome)
    .bind(reason_code)
    .bind(envelope_digest)
    .bind(final_file_sha256)
    .bind(details)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn record_retrieval_failure(
    connection: &mut PgConnection,
    row: &sqlx::postgres::PgRow,
    reason_code: &str,
) -> Result<(), sqlx::Error> {
    let mut transaction = connection.begin().await?;
    insert_download_audit(
        &mut transaction,
        Some(&row.get::<String, _>("authorization_id")),
        &row.get::<String, _>("delivery_envelope_id"),
        &row.get::<String, _>("execution_id"),
        "retrieval_failed",
        "failed",
        reason_code,
        &row.get::<String, _>("envelope_digest"),
        &row.get::<String, _>("final_signed_png_sha256"),
        json!({}),
    )
    .await?;
    transaction.commit().await
}

fn new_retrieval_token() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn rejected(reason_code: &str) -> DeliveryRetrievalOutcome {
    DeliveryRetrievalOutcome {
        succeeded: false,
        reason_code: Some(reason_code.to_string()),
        grant: None,
        package: None,
    }
}

fn revoke_rejected(reason_code: &str) -> DeliveryAuthorizationRevocationOutcome {
    DeliveryAuthorizationRevocationOutcome {
        succeeded: false,
        replayed: false,
        reason_code: Some(reason_code.to_string()),
        authorization_id: None,
    }
}

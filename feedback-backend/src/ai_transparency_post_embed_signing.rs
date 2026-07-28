use std::collections::HashSet;

use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection, Row};
use uuid::Uuid;

use crate::ai_transparency_confirm_command::{
    execute_in_transaction as execute_confirm_in_transaction, ConfirmFailureInjection,
    ConfirmMarkingCommand,
};

pub const POST_EMBED_PROFILE_ID: &str = "c2pa_post_embed_signing_v1";
pub const ANCHOR_PROFILE_ID: &str = "hiddenshield_v3_image_anchor_v1";
pub const REASON_PRECHECK_REJECTED: &str = "ai_post_embed_precheck_rejected";
pub const REASON_SIGNER_REJECTED: &str = "ai_c2pa_signer_rejected";
pub const REASON_RECEIPT_HASH_MISMATCH: &str = "ai_c2pa_signer_receipt_hash_mismatch";
pub const REASON_C2PA_READBACK_FAILED: &str = "ai_c2pa_readback_failed";
pub const REASON_V3_READBACK_FAILED: &str = "ai_v3_readback_failed";
pub const REASON_CONFIRM_ROLLED_BACK: &str = "ai_confirmation_transaction_rolled_back";
pub const REASON_REPLAY_CONFLICT: &str = "ai_post_embed_replay_conflict";
pub const REASON_RESERVATION_IN_PROGRESS: &str = "ai_post_embed_reservation_in_progress";
pub const REASON_ARTIFACT_FINALIZE_PENDING: &str = "ai_artifact_finalize_pending";
pub const REASON_ARTIFACT_RECOVERY_FAILED: &str = "ai_artifact_recovery_failed";
pub const REASON_CRASH_INJECTED: &str = "ai_post_embed_crash_injected";
pub const ADAPTER_RECEIPT_CONTRACT_VERSION: &str = "hs-ai-production-adapter-receipts-v1";

#[derive(Debug, Clone)]
pub struct PostEmbedSigningProfile {
    pub profile_entitlement_version: i32,
    pub entitlement_digest: String,
    pub status: String,
    pub technical_profile_ids: Vec<String>,
    pub regional_profile_id: String,
    pub media_type: String,
    pub claim_type: String,
    pub issuer_mode: String,
    pub signing_order: String,
    pub allowed_signature_algorithms: Vec<String>,
    pub allow_ephemeral_signer: bool,
    pub valid_from: DateTime<Utc>,
    pub valid_until: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct PostEmbedAuthorizationReceipt {
    pub receipt_id: String,
    pub provider_id: String,
    pub operation: String,
    pub role: String,
    pub license_id: String,
    pub credential_id: String,
    pub marking_session_id: String,
    pub execution_id: String,
    pub profile_entitlement_digest: String,
    pub unsigned_v3_png_sha256: String,
    pub signer_credential_ref_digest: String,
    pub scope_digest: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostEmbedSignerReceipt {
    pub schema_version: String,
    pub signer_receipt_id: String,
    pub provider_id: String,
    pub operation: String,
    pub marking_session_id: String,
    pub execution_id: String,
    pub watermark_uid: String,
    pub profile_entitlement_digest: String,
    pub unsigned_v3_png_sha256: String,
    pub final_signed_png_sha256: String,
    pub c2pa_active_manifest_label: String,
    pub c2pa_claim_digest: String,
    pub certificate_chain_digest: String,
    pub signer_key_id: String,
    pub signer_key_version: String,
    pub signer_invocation_key: String,
    pub signer_result_ref: String,
    pub idempotency_disposition: String,
    pub billable_invocation_id: String,
    pub signature_algorithm: String,
    #[serde(skip_serializing)]
    pub certificate_chain_trusted: bool,
    pub signed_at: DateTime<Utc>,
    pub receipt_expires_at: DateTime<Utc>,
    pub provider_signature: String,
}

#[derive(Debug, Clone)]
pub struct PostEmbedSignerOutput {
    pub final_signed_png_bytes: Vec<u8>,
    pub receipt: PostEmbedSignerReceipt,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostEmbedArtifactReceipt {
    pub schema_version: String,
    pub artifact_receipt_id: String,
    pub provider_id: String,
    pub operation: String,
    pub execution_id: String,
    pub signer_invocation_key: String,
    pub artifact_ref: String,
    pub final_signed_png_sha256: String,
    pub object_version: String,
    pub idempotency_key: String,
    pub idempotency_disposition: String,
    pub durability_status: String,
    pub issued_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub provider_signature: String,
}

#[derive(Debug, Clone)]
pub struct PostEmbedArtifactStageOutput {
    pub receipt: PostEmbedArtifactReceipt,
}

#[derive(Debug, Clone)]
pub struct PostEmbedArtifactFinalizeOutput {
    pub final_signed_png_bytes: Vec<u8>,
    pub receipt: PostEmbedArtifactReceipt,
}

#[derive(Debug, Clone)]
pub struct PostEmbedReadbackResult {
    pub c2pa_active_manifest_present: bool,
    pub c2pa_hard_binding_valid: bool,
    pub c2pa_validation_findings: Vec<String>,
    pub watermark_uid: String,
    pub protocol_version: u8,
    pub payload_bytes_length: usize,
    pub payload_auth_status: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostEmbedSigningFailureInjection {
    None,
    ReceiptHashMismatch,
    C2paReadbackFailure,
    V3ReadbackFailure,
    ConfirmRollback,
    CrashAfterReservation,
    CrashAfterSigner,
    CrashAfterArtifactStage,
    CrashAfterConfirm,
}

#[derive(Debug, Clone)]
pub struct InternalPostEmbedSigningCommand {
    pub execution_id: String,
    pub idempotency_key: String,
    pub request_digest: String,
    pub credential_id: String,
    pub signer_credential_ref_digest: String,
    pub unsigned_v3_png_bytes: Vec<u8>,
    pub profile: PostEmbedSigningProfile,
    pub authorization_receipt: PostEmbedAuthorizationReceipt,
    pub confirm_command: ConfirmMarkingCommand,
    pub failure_injection: PostEmbedSigningFailureInjection,
}

#[derive(Debug, Clone)]
pub struct InternalPostEmbedSigningOutcome {
    pub succeeded: bool,
    pub replayed: bool,
    pub reason_code: Option<String>,
    pub final_signed_png_sha256: Option<String>,
    pub final_signed_png_bytes: Option<Vec<u8>>,
    pub signer_invoked: bool,
    pub orphan_signing_event_created: bool,
    pub artifact_recovery_performed: bool,
    pub artifact_pending: bool,
}

pub trait PostEmbedAuthorizationVerifier: Send + Sync {
    fn verify(&self, command: &InternalPostEmbedSigningCommand) -> Result<(), &'static str>;
}

pub trait PostEmbedC2paSigner: Send + Sync {
    fn sign(
        &self,
        command: &InternalPostEmbedSigningCommand,
        signer_invocation_key: &str,
    ) -> Result<PostEmbedSignerOutput, &'static str>;
}

pub trait PostEmbedReadbackVerifier: Send + Sync {
    fn verify(
        &self,
        command: &InternalPostEmbedSigningCommand,
        signed_png_bytes: &[u8],
    ) -> Result<PostEmbedReadbackResult, &'static str>;
}

pub trait PostEmbedArtifactStore: Send + Sync {
    fn stage(
        &self,
        command: &InternalPostEmbedSigningCommand,
        signer_invocation_key: &str,
        final_signed_png_sha256: &str,
        bytes: Vec<u8>,
    ) -> Result<PostEmbedArtifactStageOutput, &'static str>;
    fn finalize(
        &self,
        command: &InternalPostEmbedSigningCommand,
        signer_invocation_key: &str,
        artifact_ref: &str,
        final_signed_png_sha256: &str,
    ) -> Result<PostEmbedArtifactFinalizeOutput, &'static str>;
    fn quarantine(&self, execution_id: &str);
    fn load_finalized(&self, execution_id: &str, artifact_ref: &str) -> Option<Vec<u8>>;
}

#[derive(Debug, thiserror::Error)]
pub enum InternalPostEmbedSigningError {
    #[error("PostgreSQL post-embed signing command failed: {0}")]
    Postgres(#[from] sqlx::Error),
}

pub async fn execute_postgres_internal_post_embed_signing(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    authorization_verifier: &dyn PostEmbedAuthorizationVerifier,
    signer: &dyn PostEmbedC2paSigner,
    readback_verifier: &dyn PostEmbedReadbackVerifier,
    artifact_store: &dyn PostEmbedArtifactStore,
) -> Result<InternalPostEmbedSigningOutcome, InternalPostEmbedSigningError> {
    acquire_idempotency_lock(connection, &command.idempotency_key).await?;
    let result = execute_locked_post_embed_signing(
        connection,
        command,
        authorization_verifier,
        signer,
        readback_verifier,
        artifact_store,
    )
    .await;
    let unlock_result = release_idempotency_lock(connection, &command.idempotency_key).await;
    match (result, unlock_result) {
        (Ok(outcome), Ok(())) => Ok(outcome),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error.into()),
    }
}

async fn execute_locked_post_embed_signing(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    authorization_verifier: &dyn PostEmbedAuthorizationVerifier,
    signer: &dyn PostEmbedC2paSigner,
    readback_verifier: &dyn PostEmbedReadbackVerifier,
    artifact_store: &dyn PostEmbedArtifactStore,
) -> Result<InternalPostEmbedSigningOutcome, InternalPostEmbedSigningError> {
    let reservation_token = Uuid::new_v4().to_string();
    let signer_invocation_key = stable_signer_invocation_key(command);
    let mut recovering_reservation = false;
    if let Some(existing) = load_existing_execution(connection, &command.idempotency_key).await? {
        if existing.request_digest != command.request_digest {
            return Ok(rejected(REASON_REPLAY_CONFLICT, false, false));
        }
        match existing.status.as_str() {
            "confirmed" => {
                let Some(artifact_ref) = existing.artifact_ref.as_deref() else {
                    return Ok(rejected(REASON_ARTIFACT_RECOVERY_FAILED, false, false));
                };
                let Some(bytes) =
                    artifact_store.load_finalized(&existing.execution_id, artifact_ref)
                else {
                    return Ok(rejected(REASON_ARTIFACT_RECOVERY_FAILED, false, false));
                };
                return Ok(success(
                    existing.final_signed_png_sha256,
                    bytes,
                    false,
                    true,
                    false,
                ));
            }
            "artifact_pending" => {
                return finalize_artifact_pending(connection, command, artifact_store, false, true)
                    .await;
            }
            "signed_staged" => {
                return confirm_staged_execution(connection, command, artifact_store, false, true)
                    .await;
            }
            "orphaned" => return Ok(rejected(REASON_CONFIRM_ROLLED_BACK, false, true)),
            "reserved"
                if existing
                    .lease_expires_at
                    .is_some_and(|value| value > Utc::now()) =>
            {
                return Ok(rejected(REASON_RESERVATION_IN_PROGRESS, false, false));
            }
            "reserved" => {
                if validate_preconditions(connection, command).await?.is_none()
                    || authorization_verifier.verify(command).is_err()
                {
                    return Ok(rejected(REASON_PRECHECK_REJECTED, false, false));
                }
                if !reclaim_reservation(
                    connection,
                    command,
                    &reservation_token,
                    &signer_invocation_key,
                )
                .await?
                {
                    return Ok(rejected(REASON_RESERVATION_IN_PROGRESS, false, false));
                }
                recovering_reservation = true;
            }
            _ => return Ok(rejected(REASON_PRECHECK_REJECTED, false, false)),
        }
    } else {
        let Some(license_id) = validate_preconditions(connection, command).await? else {
            return Ok(rejected(REASON_PRECHECK_REJECTED, false, false));
        };
        if authorization_verifier.verify(command).is_err() {
            return Ok(rejected(REASON_PRECHECK_REJECTED, false, false));
        }
        reserve_execution(
            connection,
            command,
            &license_id,
            &reservation_token,
            &signer_invocation_key,
        )
        .await?;
    }
    if command.failure_injection == PostEmbedSigningFailureInjection::CrashAfterReservation {
        return Ok(crash_injected(false, false));
    }

    let signer_output = match signer.sign(command, &signer_invocation_key) {
        Ok(output) => output,
        Err(_) => {
            finish_failed_reservation(
                connection,
                command,
                &reservation_token,
                recovering_reservation,
                REASON_SIGNER_REJECTED,
            )
            .await?;
            return Ok(rejected(REASON_SIGNER_REJECTED, true, false));
        }
    };
    if command.failure_injection == PostEmbedSigningFailureInjection::CrashAfterSigner {
        return Ok(crash_injected(true, false));
    }
    let mut signer_output = signer_output;
    if command.failure_injection == PostEmbedSigningFailureInjection::ReceiptHashMismatch {
        signer_output.receipt.final_signed_png_sha256 =
            "9999999999999999999999999999999999999999999999999999999999999999".to_string();
    }
    let final_digest = sha256_hex(&signer_output.final_signed_png_bytes);
    if !validate_signer_receipt(
        command,
        &signer_output.receipt,
        &signer_invocation_key,
        &final_digest,
    ) {
        artifact_store.quarantine(&command.execution_id);
        finish_failed_reservation(
            connection,
            command,
            &reservation_token,
            recovering_reservation,
            REASON_RECEIPT_HASH_MISMATCH,
        )
        .await?;
        return Ok(rejected(REASON_RECEIPT_HASH_MISMATCH, true, false));
    }

    let mut readback =
        match readback_verifier.verify(command, &signer_output.final_signed_png_bytes) {
            Ok(readback) => readback,
            Err(_) => {
                artifact_store.quarantine(&command.execution_id);
                finish_failed_reservation(
                    connection,
                    command,
                    &reservation_token,
                    recovering_reservation,
                    REASON_C2PA_READBACK_FAILED,
                )
                .await?;
                return Ok(rejected(REASON_C2PA_READBACK_FAILED, true, false));
            }
        };
    if command.failure_injection == PostEmbedSigningFailureInjection::C2paReadbackFailure {
        readback.c2pa_hard_binding_valid = false;
    }
    if command.failure_injection == PostEmbedSigningFailureInjection::V3ReadbackFailure {
        readback.payload_auth_status = "invalid".to_string();
    }
    if !readback.c2pa_active_manifest_present
        || !readback.c2pa_hard_binding_valid
        || !readback.c2pa_validation_findings.is_empty()
    {
        artifact_store.quarantine(&command.execution_id);
        finish_failed_reservation(
            connection,
            command,
            &reservation_token,
            recovering_reservation,
            REASON_C2PA_READBACK_FAILED,
        )
        .await?;
        return Ok(rejected(REASON_C2PA_READBACK_FAILED, true, false));
    }
    if readback.watermark_uid != command.confirm_command.watermark_uid
        || readback.protocol_version != 3
        || readback.payload_bytes_length != 39
        || readback.payload_auth_status != "verified"
    {
        artifact_store.quarantine(&command.execution_id);
        finish_failed_reservation(
            connection,
            command,
            &reservation_token,
            recovering_reservation,
            REASON_V3_READBACK_FAILED,
        )
        .await?;
        return Ok(rejected(REASON_V3_READBACK_FAILED, true, false));
    }
    let artifact_stage = match artifact_store.stage(
        command,
        &signer_invocation_key,
        &final_digest,
        signer_output.final_signed_png_bytes,
    ) {
        Ok(output) => output,
        Err(_) => {
            finish_failed_reservation(
                connection,
                command,
                &reservation_token,
                recovering_reservation,
                REASON_PRECHECK_REJECTED,
            )
            .await?;
            return Ok(rejected(REASON_PRECHECK_REJECTED, true, false));
        }
    };
    if !validate_artifact_receipt(
        command,
        &artifact_stage.receipt,
        &signer_invocation_key,
        &final_digest,
        "stage",
        "staged",
    ) {
        artifact_store.quarantine(&command.execution_id);
        finish_failed_reservation(
            connection,
            command,
            &reservation_token,
            recovering_reservation,
            REASON_PRECHECK_REJECTED,
        )
        .await?;
        return Ok(rejected(REASON_PRECHECK_REJECTED, true, false));
    }
    if command.failure_injection == PostEmbedSigningFailureInjection::CrashAfterArtifactStage {
        return Ok(crash_injected(true, false));
    }
    let staged_persisted = mark_signed_staged(
        connection,
        command,
        &reservation_token,
        &signer_output.receipt,
        &final_digest,
        &artifact_stage.receipt,
    )
    .await;
    match staged_persisted {
        Ok(true) => {}
        Ok(false) => {
            artifact_store.quarantine(&command.execution_id);
            return Ok(rejected(REASON_RESERVATION_IN_PROGRESS, true, false));
        }
        Err(error) => {
            artifact_store.quarantine(&command.execution_id);
            return Err(error.into());
        }
    }

    confirm_staged_execution(
        connection,
        command,
        artifact_store,
        true,
        recovering_reservation,
    )
    .await
}

struct ExistingExecution {
    execution_id: String,
    request_digest: String,
    status: String,
    final_signed_png_sha256: Option<String>,
    artifact_ref: Option<String>,
    lease_expires_at: Option<DateTime<Utc>>,
}

async fn load_existing_execution(
    connection: &mut PgConnection,
    idempotency_key: &str,
) -> Result<Option<ExistingExecution>, sqlx::Error> {
    Ok(sqlx::query(
        "SELECT execution_id, request_digest, status, final_signed_png_sha256,
                artifact_ref, lease_expires_at
         FROM ai_post_embed_signing_executions WHERE idempotency_key = $1",
    )
    .bind(idempotency_key)
    .fetch_optional(&mut *connection)
    .await?
    .map(|row| ExistingExecution {
        execution_id: row.get("execution_id"),
        request_digest: row.get("request_digest"),
        status: row.get("status"),
        final_signed_png_sha256: row.get("final_signed_png_sha256"),
        artifact_ref: row.get("artifact_ref"),
        lease_expires_at: row.get("lease_expires_at"),
    }))
}

async fn acquire_idempotency_lock(
    connection: &mut PgConnection,
    idempotency_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("SELECT pg_advisory_lock(hashtextextended($1, 20260727))")
        .bind(idempotency_key)
        .execute(&mut *connection)
        .await?;
    Ok(())
}

async fn release_idempotency_lock(
    connection: &mut PgConnection,
    idempotency_key: &str,
) -> Result<(), sqlx::Error> {
    let released =
        sqlx::query_scalar::<_, bool>("SELECT pg_advisory_unlock(hashtextextended($1, 20260727))")
            .bind(idempotency_key)
            .fetch_one(&mut *connection)
            .await?;
    if !released {
        return Err(sqlx::Error::Protocol(
            "post-embed idempotency advisory lock was not held".to_string(),
        ));
    }
    Ok(())
}

async fn validate_preconditions(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
) -> Result<Option<String>, sqlx::Error> {
    if !is_digest(&command.request_digest)
        || command.profile.status != "active"
        || command.profile.allow_ephemeral_signer
        || command.profile.media_type != "image/png"
        || command.profile.claim_type != "ai_generated"
        || command.profile.issuer_mode != "production_platform"
        || command.profile.signing_order != "watermark_then_c2pa"
        || command.profile.valid_from > Utc::now()
        || command.profile.valid_until <= Utc::now()
    {
        return Ok(None);
    }
    let technical_profiles: HashSet<_> = command
        .profile
        .technical_profile_ids
        .iter()
        .cloned()
        .collect();
    if !technical_profiles.contains(ANCHOR_PROFILE_ID)
        || !technical_profiles.contains(POST_EMBED_PROFILE_ID)
    {
        return Ok(None);
    }
    let row = sqlx::query(
        "SELECT license_id, status, claim_type, requested_profile_ids_json
         FROM ai_marking_sessions WHERE marking_session_id = $1",
    )
    .bind(&command.confirm_command.marking_session_id)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(row) = row else {
        return Ok(None);
    };
    if row.get::<String, _>("status") != "ready_to_confirm"
        || row.get::<String, _>("claim_type") != "ai_generated"
    {
        return Ok(None);
    }
    let requested: Value = row.get("requested_profile_ids_json");
    let requested: HashSet<String> = requested
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    if !requested.contains(ANCHOR_PROFILE_ID)
        || !requested.contains(POST_EMBED_PROFILE_ID)
        || !requested.contains(&command.profile.regional_profile_id)
    {
        return Ok(None);
    }
    let license_id: String = row.get("license_id");
    if command.authorization_receipt.license_id != license_id
        || command.authorization_receipt.credential_id != command.credential_id
        || command.authorization_receipt.marking_session_id
            != command.confirm_command.marking_session_id
        || command.authorization_receipt.execution_id != command.execution_id
        || command.authorization_receipt.profile_entitlement_digest
            != command.profile.entitlement_digest
        || command.authorization_receipt.unsigned_v3_png_sha256
            != sha256_hex(&command.unsigned_v3_png_bytes)
        || command.authorization_receipt.signer_credential_ref_digest
            != command.signer_credential_ref_digest
    {
        return Ok(None);
    }
    let license_and_credential_valid = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1
            FROM ai_transparency_licenses license
            JOIN ai_sdk_credential_bindings credential
              ON credential.license_id = license.license_id
            WHERE license.license_id = $1
              AND license.environment = 'production'
              AND license.status = 'active'
              AND license.effective_at <= NOW()
              AND license.expires_at > NOW()
              AND credential.credential_id = $2
              AND credential.status = 'active'
              AND credential.environment = 'production'
              AND (credential.expires_at IS NULL OR credential.expires_at > NOW())
              AND credential.scopes_json ? 'ai_transparency:post_embed_sign'
              AND credential.issuer_modes_json ? 'production_platform'
              AND credential.key_hash IS NOT NULL
              AND credential.hash_secret_version IS NOT NULL
              AND credential.custody_key_id IS NOT NULL
        )",
    )
    .bind(&license_id)
    .bind(&command.credential_id)
    .fetch_one(&mut *connection)
    .await?;
    if !license_and_credential_valid {
        return Ok(None);
    }
    let required_profiles = [
        ANCHOR_PROFILE_ID,
        POST_EMBED_PROFILE_ID,
        command.profile.regional_profile_id.as_str(),
    ];
    let active_profile_count = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*)
         FROM ai_profile_entitlements entitlement
         JOIN ai_profile_entitlement_versions versioned
           ON versioned.profile_entitlement_version_id = entitlement.current_version_id
          AND versioned.license_id = entitlement.license_id
          AND versioned.profile_id = entitlement.profile_id
         WHERE entitlement.license_id = $1
           AND entitlement.profile_id = ANY($2)
           AND entitlement.status = 'active'
           AND entitlement.effective_at <= NOW()
           AND entitlement.expires_at > NOW()
           AND entitlement.current_version = $3
           AND versioned.version = $3
           AND versioned.status = 'active'
           AND versioned.effective_at <= NOW()
           AND versioned.expires_at > NOW()",
    )
    .bind(&license_id)
    .bind(required_profiles.as_slice())
    .bind(command.profile.profile_entitlement_version)
    .fetch_one(&mut *connection)
    .await?;
    if active_profile_count != required_profiles.len() as i64 {
        return Ok(None);
    }
    Ok(Some(license_id))
}

fn validate_signer_receipt(
    command: &InternalPostEmbedSigningCommand,
    receipt: &PostEmbedSignerReceipt,
    signer_invocation_key: &str,
    final_digest: &str,
) -> bool {
    receipt.schema_version == "hs-ai-production-c2pa-signer-receipt-v1"
        && receipt.operation == "c2pa_post_embed_sign"
        && receipt.provider_id == command.authorization_receipt.provider_id
        && receipt.marking_session_id == command.confirm_command.marking_session_id
        && receipt.execution_id == command.execution_id
        && receipt.watermark_uid == command.confirm_command.watermark_uid
        && receipt.profile_entitlement_digest == command.profile.entitlement_digest
        && receipt.unsigned_v3_png_sha256 == sha256_hex(&command.unsigned_v3_png_bytes)
        && receipt.final_signed_png_sha256 == final_digest
        && receipt.signer_invocation_key == signer_invocation_key
        && !receipt.signer_result_ref.is_empty()
        && !receipt.billable_invocation_id.is_empty()
        && matches!(
            receipt.idempotency_disposition.as_str(),
            "created" | "replayed"
        )
        && receipt.signed_at <= Utc::now()
        && receipt.receipt_expires_at > Utc::now()
        && !receipt.provider_signature.is_empty()
        && receipt.certificate_chain_trusted
        && command
            .profile
            .allowed_signature_algorithms
            .contains(&receipt.signature_algorithm)
}

fn validate_artifact_receipt(
    command: &InternalPostEmbedSigningCommand,
    receipt: &PostEmbedArtifactReceipt,
    signer_invocation_key: &str,
    final_digest: &str,
    operation: &str,
    durability_status: &str,
) -> bool {
    receipt.schema_version == "hs-ai-production-post-embed-artifact-receipt-v1"
        && !receipt.artifact_receipt_id.is_empty()
        && !receipt.provider_id.is_empty()
        && receipt.operation == operation
        && receipt.execution_id == command.execution_id
        && receipt.signer_invocation_key == signer_invocation_key
        && !receipt.artifact_ref.is_empty()
        && receipt.final_signed_png_sha256 == final_digest
        && !receipt.object_version.is_empty()
        && receipt.idempotency_key == command.idempotency_key
        && matches!(
            receipt.idempotency_disposition.as_str(),
            "created" | "replayed"
        )
        && receipt.durability_status == durability_status
        && receipt.issued_at <= Utc::now()
        && receipt.expires_at > Utc::now()
        && !receipt.provider_signature.is_empty()
}

async fn reserve_execution(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    license_id: &str,
    reservation_token: &str,
    signer_invocation_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_post_embed_signing_executions (
            execution_id, marking_session_id, license_id, idempotency_key, request_digest,
            watermark_uid, unsigned_v3_png_sha256, final_signed_png_sha256,
            signer_receipt_id, status, reason_code, reservation_token, lease_owner,
            lease_expires_at, signer_invocation_key, artifact_status,
            adapter_receipt_contract_version, profile_entitlement_version,
            profile_entitlement_digest, technical_profile_ids_json, regional_profile_id,
            created_at, updated_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,NULL,NULL,'reserved',NULL,$8,$9,
            NOW() + INTERVAL '5 minutes',$10,'none',$11,$12,$13,$14,$15,NOW(),NOW())",
    )
    .bind(&command.execution_id)
    .bind(&command.confirm_command.marking_session_id)
    .bind(license_id)
    .bind(&command.idempotency_key)
    .bind(&command.request_digest)
    .bind(&command.confirm_command.watermark_uid)
    .bind(sha256_hex(&command.unsigned_v3_png_bytes))
    .bind(reservation_token)
    .bind(&command.authorization_receipt.provider_id)
    .bind(signer_invocation_key)
    .bind(ADAPTER_RECEIPT_CONTRACT_VERSION)
    .bind(command.profile.profile_entitlement_version)
    .bind(&command.profile.entitlement_digest)
    .bind(json!(command.profile.technical_profile_ids))
    .bind(&command.profile.regional_profile_id)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

async fn reclaim_reservation(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    reservation_token: &str,
    signer_invocation_key: &str,
) -> Result<bool, sqlx::Error> {
    let updated = sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET reservation_token = $1, lease_owner = $2,
             lease_expires_at = NOW() + INTERVAL '5 minutes', updated_at = NOW()
         WHERE execution_id = $3
           AND idempotency_key = $4
           AND request_digest = $5
           AND status = 'reserved'
           AND lease_expires_at <= NOW()
           AND signer_invocation_key = $6",
    )
    .bind(reservation_token)
    .bind(&command.authorization_receipt.provider_id)
    .bind(&command.execution_id)
    .bind(&command.idempotency_key)
    .bind(&command.request_digest)
    .bind(signer_invocation_key)
    .execute(&mut *connection)
    .await?
    .rows_affected();
    Ok(updated == 1)
}

async fn delete_reservation(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    reservation_token: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "DELETE FROM ai_post_embed_signing_executions
         WHERE execution_id = $1 AND reservation_token = $2 AND status = 'reserved'",
    )
    .bind(&command.execution_id)
    .bind(reservation_token)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

async fn finish_failed_reservation(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    reservation_token: &str,
    recovering_reservation: bool,
    reason_code: &str,
) -> Result<(), sqlx::Error> {
    if !recovering_reservation {
        return delete_reservation(connection, command, reservation_token).await;
    }
    sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET lease_expires_at = NOW(), reason_code = $1, updated_at = NOW()
         WHERE execution_id = $2 AND reservation_token = $3 AND status = 'reserved'",
    )
    .bind(reason_code)
    .bind(&command.execution_id)
    .bind(reservation_token)
    .execute(&mut *connection)
    .await?;
    Ok(())
}

async fn mark_signed_staged(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    reservation_token: &str,
    receipt: &PostEmbedSignerReceipt,
    final_digest: &str,
    artifact_receipt: &PostEmbedArtifactReceipt,
) -> Result<bool, sqlx::Error> {
    let signer_receipt_json = serde_json::to_value(receipt).expect("serializable signer receipt");
    let artifact_receipt_json =
        serde_json::to_value(artifact_receipt).expect("serializable artifact receipt");
    let updated = sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET final_signed_png_sha256 = $1, signer_receipt_id = $2,
             signer_result_ref = $3, signer_billable_invocation_id = $4,
             signer_idempotency_disposition = $5, signer_receipt_json = $6,
             artifact_ref = $7, artifact_stage_receipt_id = $8,
             artifact_stage_receipt_json = $9, artifact_object_version = $10,
             artifact_status = 'staged', status = 'signed_staged',
             reason_code = NULL, updated_at = NOW()
         WHERE execution_id = $11
           AND reservation_token = $12
           AND status = 'reserved'",
    )
    .bind(final_digest)
    .bind(&receipt.signer_receipt_id)
    .bind(&receipt.signer_result_ref)
    .bind(&receipt.billable_invocation_id)
    .bind(&receipt.idempotency_disposition)
    .bind(signer_receipt_json)
    .bind(&artifact_receipt.artifact_ref)
    .bind(&artifact_receipt.artifact_receipt_id)
    .bind(artifact_receipt_json)
    .bind(&artifact_receipt.object_version)
    .bind(&command.execution_id)
    .bind(reservation_token)
    .execute(&mut *connection)
    .await?
    .rows_affected();
    Ok(updated == 1)
}

async fn confirm_staged_execution(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    artifact_store: &dyn PostEmbedArtifactStore,
    signer_invoked: bool,
    replayed: bool,
) -> Result<InternalPostEmbedSigningOutcome, InternalPostEmbedSigningError> {
    let mut transaction = connection.begin().await?;
    let row = sqlx::query(
        "SELECT status, final_signed_png_sha256, signer_receipt_id, artifact_ref
         FROM ai_post_embed_signing_executions
         WHERE execution_id = $1 AND request_digest = $2
         FOR UPDATE",
    )
    .bind(&command.execution_id)
    .bind(&command.request_digest)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(row) = row else {
        transaction.rollback().await?;
        return Ok(rejected(REASON_PRECHECK_REJECTED, signer_invoked, false));
    };
    let status: String = row.get("status");
    if status == "artifact_pending" {
        transaction.rollback().await?;
        return finalize_artifact_pending(
            connection,
            command,
            artifact_store,
            signer_invoked,
            replayed,
        )
        .await;
    }
    if status != "signed_staged" {
        transaction.rollback().await?;
        return Ok(rejected(
            REASON_RESERVATION_IN_PROGRESS,
            signer_invoked,
            false,
        ));
    }
    let final_digest: String = row.get("final_signed_png_sha256");
    let signer_receipt_id: String = row.get("signer_receipt_id");
    let artifact_ref: String = row.get("artifact_ref");
    let mut confirm_command = command.confirm_command.clone();
    confirm_command.subject_digest = final_digest.clone();
    confirm_command.write_after_read_verified = true;
    confirm_command.failure_injection =
        if command.failure_injection == PostEmbedSigningFailureInjection::ConfirmRollback {
            ConfirmFailureInjection::Ledger
        } else {
            ConfirmFailureInjection::None
        };
    let confirm_result = execute_confirm_in_transaction(&mut transaction, &confirm_command).await;
    match confirm_result {
        Ok(outcome) if outcome.succeeded => {
            let metering_held = sqlx::query(
                "UPDATE ai_marking_ledger
                 SET ledger_status = 'pending', committed_at = NULL
                 WHERE ledger_entry_id = $1 AND ledger_status = 'committed'",
            )
            .bind(&confirm_command.ledger_entry_id)
            .execute(&mut *transaction)
            .await?
            .rows_affected();
            if metering_held != 1 {
                transaction.rollback().await?;
                artifact_store.quarantine(&command.execution_id);
                persist_orphan(connection, command, REASON_CONFIRM_ROLLED_BACK).await?;
                return Ok(rejected(REASON_CONFIRM_ROLLED_BACK, signer_invoked, true));
            }
            sqlx::query(
                "UPDATE ai_post_embed_signing_executions
                 SET status = 'artifact_pending', artifact_status = 'pending_finalize',
                     lease_owner = NULL, lease_expires_at = NULL, updated_at = NOW()
                 WHERE execution_id = $1 AND status = 'signed_staged'",
            )
            .bind(&command.execution_id)
            .execute(&mut *transaction)
            .await?;
            insert_signing_audit(
                &mut transaction,
                command,
                "confirm_committed_artifact_pending",
                "confirm_committed_artifact_pending",
                Some(&final_digest),
                json!({
                    "signerReceiptId": signer_receipt_id,
                    "artifactRef": artifact_ref,
                    "c2paReadback": "verified",
                    "v3Readback": "verified"
                }),
            )
            .await?;
            transaction.commit().await?;
            if command.failure_injection == PostEmbedSigningFailureInjection::CrashAfterConfirm {
                return Ok(crash_injected(signer_invoked, true));
            }
            finalize_artifact_pending(
                connection,
                command,
                artifact_store,
                signer_invoked,
                replayed,
            )
            .await
        }
        Ok(outcome) => {
            transaction.rollback().await?;
            artifact_store.quarantine(&command.execution_id);
            persist_orphan(
                connection,
                command,
                outcome
                    .reason_code
                    .as_deref()
                    .unwrap_or(REASON_CONFIRM_ROLLED_BACK),
            )
            .await?;
            Ok(rejected(REASON_CONFIRM_ROLLED_BACK, signer_invoked, true))
        }
        Err(_) => {
            transaction.rollback().await?;
            artifact_store.quarantine(&command.execution_id);
            persist_orphan(connection, command, REASON_CONFIRM_ROLLED_BACK).await?;
            Ok(rejected(REASON_CONFIRM_ROLLED_BACK, signer_invoked, true))
        }
    }
}

async fn finalize_artifact_pending(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    artifact_store: &dyn PostEmbedArtifactStore,
    signer_invoked: bool,
    replayed: bool,
) -> Result<InternalPostEmbedSigningOutcome, InternalPostEmbedSigningError> {
    let row = sqlx::query(
        "SELECT status, final_signed_png_sha256, artifact_ref, signer_invocation_key
         FROM ai_post_embed_signing_executions
         WHERE execution_id = $1 AND request_digest = $2",
    )
    .bind(&command.execution_id)
    .bind(&command.request_digest)
    .fetch_optional(&mut *connection)
    .await?;
    let Some(row) = row else {
        return Ok(rejected(REASON_PRECHECK_REJECTED, signer_invoked, false));
    };
    if row.get::<String, _>("status") != "artifact_pending" {
        return Ok(rejected(
            REASON_RESERVATION_IN_PROGRESS,
            signer_invoked,
            false,
        ));
    }
    let final_digest: String = row.get("final_signed_png_sha256");
    let artifact_ref: String = row.get("artifact_ref");
    let signer_invocation_key: String = row.get("signer_invocation_key");
    let finalized = match artifact_store.finalize(
        command,
        &signer_invocation_key,
        &artifact_ref,
        &final_digest,
    ) {
        Ok(output) => output,
        Err(_) => {
            record_artifact_recovery_failure(connection, command, &final_digest, &artifact_ref)
                .await?;
            return Ok(artifact_pending(signer_invoked, replayed));
        }
    };
    if sha256_hex(&finalized.final_signed_png_bytes) != final_digest
        || !validate_artifact_receipt(
            command,
            &finalized.receipt,
            &signer_invocation_key,
            &final_digest,
            "finalize",
            "finalized",
        )
        || finalized.receipt.artifact_ref != artifact_ref
    {
        record_artifact_recovery_failure(connection, command, &final_digest, &artifact_ref).await?;
        return Ok(artifact_pending(signer_invoked, replayed));
    }
    let finalize_receipt_json =
        serde_json::to_value(&finalized.receipt).expect("serializable artifact finalize receipt");
    let mut transaction = connection.begin().await?;
    let updated = sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET status = 'confirmed', artifact_status = 'finalized',
             artifact_finalize_receipt_id = $1, artifact_finalize_receipt_json = $2,
             artifact_finalized_at = NOW(), reason_code = NULL,
             recovery_state = CASE
                 WHEN recovery_state = 'leased' THEN 'leased'
                 ELSE 'completed'
             END,
             recovery_lease_owner = CASE
                 WHEN recovery_state = 'leased' THEN recovery_lease_owner
                 ELSE NULL
             END,
             recovery_lease_expires_at = CASE
                 WHEN recovery_state = 'leased' THEN recovery_lease_expires_at
                 ELSE NULL
             END,
             last_recovery_reason = CASE
                 WHEN recovery_state = 'leased' THEN last_recovery_reason
                 ELSE 'ai_post_embed_recovery_not_required'
             END,
             updated_at = NOW()
         WHERE execution_id = $3 AND request_digest = $4 AND status = 'artifact_pending'",
    )
    .bind(&finalized.receipt.artifact_receipt_id)
    .bind(finalize_receipt_json)
    .bind(&command.execution_id)
    .bind(&command.request_digest)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if updated != 1 {
        transaction.rollback().await?;
        return Ok(rejected(
            REASON_RESERVATION_IN_PROGRESS,
            signer_invoked,
            false,
        ));
    }
    let metering_committed = sqlx::query(
        "UPDATE ai_marking_ledger
         SET ledger_status = 'committed', committed_at = NOW()
         WHERE ledger_entry_id = $1 AND ledger_status = 'pending'",
    )
    .bind(&command.confirm_command.ledger_entry_id)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if metering_committed != 1 {
        transaction.rollback().await?;
        return Ok(artifact_pending(signer_invoked, replayed));
    }
    insert_signing_audit(
        &mut transaction,
        command,
        "artifact_finalized",
        "artifact_finalized",
        Some(&final_digest),
        json!({
            "artifactRef": artifact_ref,
            "artifactFinalizeReceiptId": finalized.receipt.artifact_receipt_id,
            "artifactObjectVersion": finalized.receipt.object_version
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(success(
        Some(final_digest),
        finalized.final_signed_png_bytes,
        signer_invoked,
        replayed,
        replayed,
    ))
}

async fn record_artifact_recovery_failure(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    final_digest: &str,
    artifact_ref: &str,
) -> Result<(), sqlx::Error> {
    let mut transaction = connection.begin().await?;
    let attempt = sqlx::query_scalar::<_, i32>(
        "UPDATE ai_post_embed_signing_executions
         SET recovery_attempts = recovery_attempts + 1, updated_at = NOW()
         WHERE execution_id = $1 AND request_digest = $2 AND status = 'artifact_pending'
         RETURNING recovery_attempts",
    )
    .bind(&command.execution_id)
    .bind(&command.request_digest)
    .fetch_one(&mut *transaction)
    .await?;
    insert_signing_audit(
        &mut transaction,
        command,
        "artifact_recovery_failed",
        &format!("artifact_recovery_failed_{attempt}"),
        Some(final_digest),
        json!({"artifactRef": artifact_ref, "attempt": attempt}),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn persist_orphan(
    connection: &mut PgConnection,
    command: &InternalPostEmbedSigningCommand,
    reason_code: &str,
) -> Result<(), sqlx::Error> {
    let mut transaction = connection.begin().await?;
    let final_digest = sqlx::query_scalar::<_, String>(
        "UPDATE ai_post_embed_signing_executions
         SET status = 'orphaned', artifact_status = 'quarantined',
             reason_code = $1, lease_owner = NULL, lease_expires_at = NULL,
             recovery_state = CASE
                 WHEN recovery_state = 'leased' THEN 'leased'
                 ELSE 'completed'
             END,
             recovery_lease_owner = CASE
                 WHEN recovery_state = 'leased' THEN recovery_lease_owner
                 ELSE NULL
             END,
             recovery_lease_expires_at = CASE
                 WHEN recovery_state = 'leased' THEN recovery_lease_expires_at
                 ELSE NULL
             END,
             last_recovery_reason = CASE
                 WHEN recovery_state = 'leased' THEN last_recovery_reason
                 ELSE $1
             END,
             updated_at = NOW()
         WHERE execution_id = $2 AND status = 'signed_staged'
         RETURNING final_signed_png_sha256",
    )
    .bind(reason_code)
    .bind(&command.execution_id)
    .fetch_one(&mut *transaction)
    .await?;
    insert_signing_audit(
        &mut transaction,
        command,
        "orphan_signing",
        "orphan_signing",
        Some(&final_digest),
        json!({"reasonCode": reason_code}),
    )
    .await?;
    transaction.commit().await?;
    Ok(())
}

async fn insert_signing_audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &InternalPostEmbedSigningCommand,
    event_type: &str,
    event_key: &str,
    subject_digest: Option<&str>,
    details: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_post_embed_signing_audit_events (
            audit_event_id, execution_id, event_type, subject_digest, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(format!(
        "audit-post-embed-{}-{event_key}",
        command.execution_id
    ))
    .bind(&command.execution_id)
    .bind(event_type)
    .bind(subject_digest)
    .bind(details)
    .bind(Utc::now())
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn rejected(
    reason_code: &str,
    signer_invoked: bool,
    orphan_signing_event_created: bool,
) -> InternalPostEmbedSigningOutcome {
    InternalPostEmbedSigningOutcome {
        succeeded: false,
        replayed: false,
        reason_code: Some(reason_code.to_string()),
        final_signed_png_sha256: None,
        final_signed_png_bytes: None,
        signer_invoked,
        orphan_signing_event_created,
        artifact_recovery_performed: false,
        artifact_pending: false,
    }
}

fn artifact_pending(signer_invoked: bool, replayed: bool) -> InternalPostEmbedSigningOutcome {
    InternalPostEmbedSigningOutcome {
        succeeded: false,
        replayed,
        reason_code: Some(REASON_ARTIFACT_FINALIZE_PENDING.to_string()),
        final_signed_png_sha256: None,
        final_signed_png_bytes: None,
        signer_invoked,
        orphan_signing_event_created: false,
        artifact_recovery_performed: replayed,
        artifact_pending: true,
    }
}

fn crash_injected(signer_invoked: bool, artifact_pending: bool) -> InternalPostEmbedSigningOutcome {
    InternalPostEmbedSigningOutcome {
        succeeded: false,
        replayed: false,
        reason_code: Some(REASON_CRASH_INJECTED.to_string()),
        final_signed_png_sha256: None,
        final_signed_png_bytes: None,
        signer_invoked,
        orphan_signing_event_created: false,
        artifact_recovery_performed: false,
        artifact_pending,
    }
}

fn success(
    final_signed_png_sha256: Option<String>,
    bytes: Vec<u8>,
    signer_invoked: bool,
    replayed: bool,
    artifact_recovery_performed: bool,
) -> InternalPostEmbedSigningOutcome {
    InternalPostEmbedSigningOutcome {
        succeeded: true,
        replayed,
        reason_code: None,
        final_signed_png_sha256,
        final_signed_png_bytes: Some(bytes),
        signer_invoked,
        orphan_signing_event_created: false,
        artifact_recovery_performed,
        artifact_pending: false,
    }
}

pub fn stable_signer_invocation_key(command: &InternalPostEmbedSigningCommand) -> String {
    let mut digest = Sha256::new();
    digest.update(b"hiddenshield-post-embed-signer-invocation-v1\0");
    digest.update(command.idempotency_key.as_bytes());
    digest.update(b"\0");
    digest.update(command.request_digest.as_bytes());
    digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .as_bytes()
            .iter()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
}

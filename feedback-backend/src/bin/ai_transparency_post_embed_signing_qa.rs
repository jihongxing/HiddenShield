#[cfg(feature = "postgres")]
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::PathBuf,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::Duration as StdDuration,
};

#[cfg(feature = "postgres")]
use chrono::{Duration, Utc};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_change_command::{
    ActorAuthorizationDecision, ActorAuthorizationInput, ApprovalReferenceAdapter,
    ApprovalReferenceDecision, ApprovalReferenceInput, ChangeCommandPreflight,
    InternalIamAuthorizationAdapter,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_confirm_command::{
    ConfirmEvidence, ConfirmExplicitLabelReceipt, ConfirmFailureInjection, ConfirmMarker,
    ConfirmMarkingCommand,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_dead_letter_command::{
    canonical_dead_letter_requeue_digest, execute_postgres_dead_letter_requeue,
    inspect_postgres_dead_letter, DeadLetterInspectCommand, DeadLetterRequeueCommand,
    DeadLetterRequeueExecutionHook, DeadLetterRequeueMode, NoopDeadLetterRequeueExecutionHook,
    REASON_AUDIT_WRITE_FAILED as DEAD_LETTER_REASON_AUDIT_WRITE_FAILED,
    REASON_IDEMPOTENCY_REPLAY as DEAD_LETTER_REASON_IDEMPOTENCY_REPLAY,
    REASON_REQUEST_DIGEST_MISMATCH, REASON_TARGET_STATE_CONFLICT,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_delivery_envelope::{
    execute_postgres_confirmed_delivery_envelope, REASON_DELIVERY_NOT_READY,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_delivery_observability::{
    execute_postgres_cleanup_delivery_security_windows,
    execute_postgres_generate_delivery_security_summary, CleanupDeliverySecurityWindowsCommand,
    DeliverySecuritySummaryMode, GenerateDeliverySecuritySummaryCommand,
    ALERT_AVAILABILITY_WARNING_COUNT, ALERT_FAILURE_RATIO_MIN_ATTEMPTS,
    ALERT_FAILURE_RATIO_WARNING_PERCENT, ALERT_RATE_LIMITED_WARNING_COUNT,
    ALERT_REVOKED_ACCESS_CRITICAL_COUNT, DELIVERY_RATE_WINDOW_RETENTION_HOURS,
    DELIVERY_SECURITY_CLEANUP_BATCH_LIMIT, DELIVERY_SECURITY_MAX_EXPORT_WINDOW_MINUTES,
    DELIVERY_SECURITY_METRIC_RETENTION_DAYS, DELIVERY_SECURITY_MONITORING_WINDOW_MINUTES,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_delivery_retrieval::{
    execute_postgres_create_delivery_authorization, execute_postgres_retrieve_delivery,
    execute_postgres_revoke_delivery_authorization, CreateDeliveryAuthorizationCommand,
    DeliveryArtifactObject, DeliveryArtifactReadFailure, DeliveryArtifactRetriever,
    DeliveryAuthorizationGrant, DeliveryDownloadBudget, RetrieveDeliveryCommand,
    RevokeDeliveryAuthorizationCommand, DELIVERY_MAX_DOWNLOAD_BYTES,
    DELIVERY_RATE_LIMIT_PER_MINUTE, DELIVERY_READ_TIMEOUT_MS, DELIVERY_REQUIRED_CONTENT_TYPE,
    REASON_AUTHORIZATION_REVOKED, REASON_RETRIEVAL_ARTIFACT_UNAVAILABLE,
    REASON_RETRIEVAL_BRIDGE_REJECTED, REASON_RETRIEVAL_CONTENT_TYPE_INVALID,
    REASON_RETRIEVAL_ENTITLEMENT_INVALID, REASON_RETRIEVAL_EXPIRED, REASON_RETRIEVAL_INVALID,
    REASON_RETRIEVAL_RATE_LIMITED, REASON_RETRIEVAL_READ_TIMEOUT,
    REASON_RETRIEVAL_SIZE_LIMIT_EXCEEDED,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_delivery_security_incident::{
    canonical_delivery_security_incident_change_digest,
    ensure_postgres_delivery_security_cleanup_schedule,
    execute_postgres_delivery_security_incident_change, run_postgres_due_delivery_security_cleanup,
    DeliverySecurityIncidentChangeCommand, DeliverySecurityIncidentChangeMode,
    DeliverySecurityIncidentDesiredStatus, EnsureDeliverySecurityCleanupScheduleCommand,
    RunDeliverySecurityCleanupScheduleCommand, DELIVERY_SECURITY_CLEANUP_INTERVAL_MINUTES,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_delivery_security_notification::{
    claim_postgres_delivery_security_notifications, enqueue_delivery_security_notification,
    inspect_postgres_delivery_security_incident, list_postgres_delivery_security_incidents,
    replay_postgres_delivery_security_notification, ClaimDeliverySecurityNotificationsCommand,
    EnqueueDeliverySecurityNotificationInput, InspectDeliverySecurityIncidentCommand,
    ListDeliverySecurityIncidentsCommand, ReplayDeliverySecurityNotificationCommand,
    INCIDENT_LIST_MAX_LIMIT, NOTIFICATION_OUTBOX_LEASE_MINUTES, NOTIFICATION_OUTBOX_MAX_CLAIM,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_notification_delivery::{
    bind_postgres_notification_destination, complete_postgres_notification_delivery,
    destination_policy_digest, fail_postgres_notification_delivery,
    recover_postgres_notification_dead_letter, validate_destination_policy,
    BindNotificationDestinationCommand, CompleteNotificationDeliveryCommand,
    FailNotificationDeliveryCommand, NotificationAdapterRequest, NotificationDestinationPolicyV1,
    NotificationProviderAdapter, RecoverNotificationDeadLetterCommand, ZeroSendNotificationAdapter,
    DESTINATION_POLICY_SCHEMA_VERSION,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_post_embed_recovery::{
    run_postgres_post_embed_recovery_batch, PostEmbedRecoveryCommandLoader,
    PostEmbedRecoveryWorkerConfig, RECOVERY_REASON_LOADER_FAILED,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_post_embed_signing::{
    execute_postgres_internal_post_embed_signing, InternalPostEmbedSigningCommand,
    PostEmbedArtifactFinalizeOutput, PostEmbedArtifactReceipt, PostEmbedArtifactStageOutput,
    PostEmbedArtifactStore, PostEmbedAuthorizationReceipt, PostEmbedAuthorizationVerifier,
    PostEmbedC2paSigner, PostEmbedReadbackResult, PostEmbedReadbackVerifier, PostEmbedSignerOutput,
    PostEmbedSignerReceipt, PostEmbedSigningFailureInjection, PostEmbedSigningProfile,
    ANCHOR_PROFILE_ID, POST_EMBED_PROFILE_ID, REASON_ARTIFACT_FINALIZE_PENDING,
    REASON_C2PA_READBACK_FAILED, REASON_CONFIRM_ROLLED_BACK, REASON_CRASH_INJECTED,
    REASON_RECEIPT_HASH_MISMATCH, REASON_SIGNER_REJECTED, REASON_V3_READBACK_FAILED,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::database::{
    POSTGRES_P10_AI_TRANSPARENCY_ADAPTER_RECEIPTS_UP_SQL,
    POSTGRES_P11_AI_TRANSPARENCY_RECOVERY_WORKER_UP_SQL,
    POSTGRES_P12_AI_TRANSPARENCY_DEAD_LETTER_REQUEUE_UP_SQL,
    POSTGRES_P13_AI_TRANSPARENCY_CONFIRMED_DELIVERY_ENVELOPE_UP_SQL,
    POSTGRES_P14_AI_TRANSPARENCY_DELIVERY_RETRIEVAL_UP_SQL,
    POSTGRES_P15_AI_TRANSPARENCY_DELIVERY_REVOKE_RESOURCE_BUDGET_UP_SQL,
    POSTGRES_P16_AI_TRANSPARENCY_DELIVERY_SECURITY_OBSERVABILITY_UP_SQL,
    POSTGRES_P17_AI_TRANSPARENCY_DELIVERY_SECURITY_INCIDENT_RUNNER_UP_SQL,
    POSTGRES_P18_AI_TRANSPARENCY_DELIVERY_SECURITY_NOTIFICATION_OUTBOX_UP_SQL,
    POSTGRES_P19_AI_TRANSPARENCY_NOTIFICATION_DELIVERY_GATE_UP_SQL,
    POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL, POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL,
    POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
    POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_UP_SQL,
    POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_UP_SQL,
    POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_UP_SQL,
    POSTGRES_P8_AI_TRANSPARENCY_POST_EMBED_SIGNING_UP_SQL,
    POSTGRES_P9_AI_TRANSPARENCY_SIGNING_RESERVATION_UP_SQL,
};
#[cfg(feature = "postgres")]
use serde::Serialize;
#[cfg(feature = "postgres")]
use serde_json::{json, Value};
#[cfg(feature = "postgres")]
use sha2::{Digest, Sha256};
#[cfg(feature = "postgres")]
use sqlx::{Connection as _, Row};
#[cfg(feature = "postgres")]
use watermark_core::{validate_ai_delivery_envelope, validate_ai_delivery_import};

#[cfg(feature = "postgres")]
struct AllowGovernance;

#[cfg(feature = "postgres")]
impl InternalIamAuthorizationAdapter for AllowGovernance {
    fn verify_actor_authorization(
        &self,
        _input: &ActorAuthorizationInput<'_>,
    ) -> ActorAuthorizationDecision {
        ActorAuthorizationDecision {
            authorized: true,
            reason_code: None,
            verification_receipt_id: Some("iam-receipt-dead-letter-qa".to_string()),
        }
    }
}

#[cfg(feature = "postgres")]
impl ApprovalReferenceAdapter for AllowGovernance {
    fn verify_approval_reference(
        &self,
        _input: &ApprovalReferenceInput<'_>,
    ) -> ApprovalReferenceDecision {
        ApprovalReferenceDecision {
            verified: true,
            reason_code: None,
            verification_receipt_id: Some("security-reference-receipt-dead-letter-qa".to_string()),
        }
    }
}

#[cfg(feature = "postgres")]
struct BlockingDeadLetterRequeueHook {
    locked: Arc<std::sync::Barrier>,
    release: Arc<std::sync::Barrier>,
}

#[cfg(feature = "postgres")]
impl DeadLetterRequeueExecutionHook for BlockingDeadLetterRequeueHook {
    fn after_dead_letter_locked(&self) {
        self.locked.wait();
        self.release.wait();
    }
}

#[cfg(feature = "postgres")]
const REGIONAL_PROFILE_ID: &str = "cn_aigc_label_2025_image_export_v1";
#[cfg(feature = "postgres")]
const AUTH_PROVIDER_ID: &str = "provider-production-c2pa-qa";
#[cfg(feature = "postgres")]
const SIGNER_KEY_REF_DIGEST: &str =
    "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
#[cfg(feature = "postgres")]
const ENTITLEMENT_DIGEST: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
#[cfg(feature = "postgres")]
const SCOPE_DIGEST: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";

#[cfg(feature = "postgres")]
struct AllowAuthorization;

#[cfg(feature = "postgres")]
impl PostEmbedAuthorizationVerifier for AllowAuthorization {
    fn verify(&self, command: &InternalPostEmbedSigningCommand) -> Result<(), &'static str> {
        let receipt = &command.authorization_receipt;
        if receipt.provider_id != AUTH_PROVIDER_ID
            || receipt.operation != "ai_transparency_post_embed_c2pa_sign"
            || receipt.role != "ai_transparency_production_signer"
            || receipt.scope_digest != SCOPE_DIGEST
            || receipt.issued_at > Utc::now()
            || receipt.expires_at <= Utc::now()
        {
            return Err("authorization receipt rejected");
        }
        Ok(())
    }
}

#[cfg(feature = "postgres")]
struct StaticRecoveryCommandLoader {
    command: InternalPostEmbedSigningCommand,
}

#[cfg(feature = "postgres")]
impl PostEmbedRecoveryCommandLoader for StaticRecoveryCommandLoader {
    fn load(&self, execution_id: &str) -> Result<InternalPostEmbedSigningCommand, &'static str> {
        if self.command.execution_id != execution_id {
            return Err("recovery execution mismatch");
        }
        Ok(self.command.clone())
    }
}

#[cfg(feature = "postgres")]
struct RejectingRecoveryCommandLoader;

#[cfg(feature = "postgres")]
impl PostEmbedRecoveryCommandLoader for RejectingRecoveryCommandLoader {
    fn load(&self, _execution_id: &str) -> Result<InternalPostEmbedSigningCommand, &'static str> {
        Err("controlled recovery loader failure")
    }
}

#[cfg(feature = "postgres")]
struct ControlledSigner {
    reject: bool,
    delay_ms: u64,
    requests: Arc<AtomicUsize>,
    billable_invocations: Arc<AtomicUsize>,
    results: Arc<Mutex<HashMap<String, PostEmbedSignerOutput>>>,
}

#[cfg(feature = "postgres")]
impl ControlledSigner {
    fn new(reject: bool, delay_ms: u64) -> Self {
        Self {
            reject,
            delay_ms,
            requests: Arc::new(AtomicUsize::new(0)),
            billable_invocations: Arc::new(AtomicUsize::new(0)),
            results: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn request_count(&self) -> usize {
        self.requests.load(Ordering::SeqCst)
    }

    fn billable_invocation_count(&self) -> usize {
        self.billable_invocations.load(Ordering::SeqCst)
    }
}

#[cfg(feature = "postgres")]
impl PostEmbedC2paSigner for ControlledSigner {
    fn sign(
        &self,
        command: &InternalPostEmbedSigningCommand,
        signer_invocation_key: &str,
    ) -> Result<PostEmbedSignerOutput, &'static str> {
        self.requests.fetch_add(1, Ordering::SeqCst);
        if self.delay_ms > 0 {
            thread::sleep(StdDuration::from_millis(self.delay_ms));
        }
        if self.reject {
            return Err("controlled signer rejection");
        }
        if let Some(mut cached) = self
            .results
            .lock()
            .map_err(|_| "signer result lock poisoned")?
            .get(signer_invocation_key)
            .cloned()
        {
            cached.receipt.idempotency_disposition = "replayed".to_string();
            return Ok(cached);
        }
        self.billable_invocations.fetch_add(1, Ordering::SeqCst);
        let mut final_bytes = command.unsigned_v3_png_bytes.clone();
        final_bytes.extend_from_slice(b":c2pa-post-embed-signed");
        let final_digest = sha256_hex(&final_bytes);
        let output = PostEmbedSignerOutput {
            final_signed_png_bytes: final_bytes,
            receipt: PostEmbedSignerReceipt {
                schema_version: "hs-ai-production-c2pa-signer-receipt-v1".to_string(),
                signer_receipt_id: format!("sign-result-{}", command.execution_id),
                provider_id: AUTH_PROVIDER_ID.to_string(),
                operation: "c2pa_post_embed_sign".to_string(),
                marking_session_id: command.confirm_command.marking_session_id.clone(),
                execution_id: command.execution_id.clone(),
                watermark_uid: command.confirm_command.watermark_uid.clone(),
                profile_entitlement_digest: command.profile.entitlement_digest.clone(),
                unsigned_v3_png_sha256: sha256_hex(&command.unsigned_v3_png_bytes),
                final_signed_png_sha256: final_digest,
                c2pa_active_manifest_label: format!("urn:c2pa:manifest:{}", command.execution_id),
                c2pa_claim_digest:
                    "1111111111111111111111111111111111111111111111111111111111111111".to_string(),
                certificate_chain_digest:
                    "2222222222222222222222222222222222222222222222222222222222222222".to_string(),
                signer_key_id: "kms-production-c2pa-key-qa".to_string(),
                signer_key_version: "1".to_string(),
                signer_invocation_key: signer_invocation_key.to_string(),
                signer_result_ref: format!("signer-result://qa/{signer_invocation_key}"),
                idempotency_disposition: "created".to_string(),
                billable_invocation_id: format!("billable-{signer_invocation_key}"),
                signature_algorithm: "es256".to_string(),
                certificate_chain_trusted: true,
                signed_at: Utc::now(),
                receipt_expires_at: Utc::now() + Duration::minutes(5),
                provider_signature: format!(
                    "qa-signer-provider-signature-{}",
                    command.execution_id
                ),
            },
        };
        self.results
            .lock()
            .map_err(|_| "signer result lock poisoned")?
            .insert(signer_invocation_key.to_string(), output.clone());
        Ok(output)
    }
}

#[cfg(feature = "postgres")]
struct ControlledReadback;

#[cfg(feature = "postgres")]
impl PostEmbedReadbackVerifier for ControlledReadback {
    fn verify(
        &self,
        command: &InternalPostEmbedSigningCommand,
        _signed_png_bytes: &[u8],
    ) -> Result<PostEmbedReadbackResult, &'static str> {
        Ok(PostEmbedReadbackResult {
            c2pa_active_manifest_present: true,
            c2pa_hard_binding_valid: true,
            c2pa_validation_findings: Vec::new(),
            watermark_uid: command.confirm_command.watermark_uid.clone(),
            protocol_version: 3,
            payload_bytes_length: 39,
            payload_auth_status: "verified".to_string(),
        })
    }
}

#[cfg(feature = "postgres")]
#[derive(Default)]
struct InMemoryArtifactStore {
    staged: Mutex<HashMap<String, Vec<u8>>>,
    committed: Mutex<HashMap<String, Vec<u8>>>,
    quarantined: Mutex<HashSet<String>>,
    finalize_failures_remaining: AtomicUsize,
    stage_requests: AtomicUsize,
    unique_stage_writes: AtomicUsize,
    load_requests: AtomicUsize,
    delivery_failure: Mutex<Option<DeliveryArtifactReadFailure>>,
    delivery_content_type: Mutex<Option<String>>,
    delivery_content_length: Mutex<Option<i64>>,
}

#[cfg(feature = "postgres")]
impl InMemoryArtifactStore {
    fn is_quarantined(&self, execution_id: &str) -> bool {
        self.quarantined
            .lock()
            .expect("artifact quarantine lock")
            .contains(execution_id)
    }

    fn committed_count(&self) -> usize {
        self.committed
            .lock()
            .expect("artifact committed lock")
            .len()
    }

    fn fail_next_finalize(&self) {
        self.finalize_failures_remaining
            .fetch_add(1, Ordering::SeqCst);
    }

    fn stage_request_count(&self) -> usize {
        self.stage_requests.load(Ordering::SeqCst)
    }

    fn unique_stage_write_count(&self) -> usize {
        self.unique_stage_writes.load(Ordering::SeqCst)
    }

    fn load_request_count(&self) -> usize {
        self.load_requests.load(Ordering::SeqCst)
    }

    fn set_delivery_failure(&self, failure: Option<DeliveryArtifactReadFailure>) {
        *self.delivery_failure.lock().expect("delivery failure lock") = failure;
    }

    fn set_delivery_content_type(&self, content_type: Option<&str>) {
        *self
            .delivery_content_type
            .lock()
            .expect("delivery content type lock") = content_type.map(ToString::to_string);
    }

    fn set_delivery_content_length(&self, content_length: Option<i64>) {
        *self
            .delivery_content_length
            .lock()
            .expect("delivery content length lock") = content_length;
    }
}

#[cfg(feature = "postgres")]
impl PostEmbedArtifactStore for InMemoryArtifactStore {
    fn stage(
        &self,
        command: &InternalPostEmbedSigningCommand,
        signer_invocation_key: &str,
        final_signed_png_sha256: &str,
        bytes: Vec<u8>,
    ) -> Result<PostEmbedArtifactStageOutput, &'static str> {
        self.stage_requests.fetch_add(1, Ordering::SeqCst);
        let mut staged = self.staged.lock().map_err(|_| "stage lock poisoned")?;
        let disposition = if let Some(existing) = staged.get(&command.execution_id) {
            if existing != &bytes {
                return Err("idempotent artifact stage payload mismatch");
            }
            "replayed"
        } else {
            staged.insert(command.execution_id.clone(), bytes);
            self.unique_stage_writes.fetch_add(1, Ordering::SeqCst);
            "created"
        };
        Ok(PostEmbedArtifactStageOutput {
            receipt: artifact_receipt(
                command,
                signer_invocation_key,
                final_signed_png_sha256,
                "stage",
                "staged",
                disposition,
            ),
        })
    }

    fn finalize(
        &self,
        command: &InternalPostEmbedSigningCommand,
        signer_invocation_key: &str,
        artifact_ref: &str,
        final_signed_png_sha256: &str,
    ) -> Result<PostEmbedArtifactFinalizeOutput, &'static str> {
        if self.finalize_failures_remaining.load(Ordering::SeqCst) > 0 {
            self.finalize_failures_remaining
                .fetch_sub(1, Ordering::SeqCst);
            return Err("controlled durable artifact finalize failure");
        }
        if artifact_ref != format!("memory://post-embed/{}", command.execution_id) {
            return Err("artifact ref mismatch");
        }
        if let Some(bytes) = self
            .committed
            .lock()
            .map_err(|_| "commit lock poisoned")?
            .get(&command.execution_id)
            .cloned()
        {
            return Ok(PostEmbedArtifactFinalizeOutput {
                final_signed_png_bytes: bytes,
                receipt: artifact_receipt(
                    command,
                    signer_invocation_key,
                    final_signed_png_sha256,
                    "finalize",
                    "finalized",
                    "replayed",
                ),
            });
        }
        let bytes = self
            .staged
            .lock()
            .map_err(|_| "stage lock poisoned")?
            .remove(&command.execution_id)
            .ok_or("staged artifact missing")?;
        self.committed
            .lock()
            .map_err(|_| "commit lock poisoned")?
            .insert(command.execution_id.clone(), bytes.clone());
        Ok(PostEmbedArtifactFinalizeOutput {
            final_signed_png_bytes: bytes,
            receipt: artifact_receipt(
                command,
                signer_invocation_key,
                final_signed_png_sha256,
                "finalize",
                "finalized",
                "created",
            ),
        })
    }

    fn quarantine(&self, execution_id: &str) {
        if let Ok(mut staged) = self.staged.lock() {
            staged.remove(execution_id);
        }
        if let Ok(mut quarantined) = self.quarantined.lock() {
            quarantined.insert(execution_id.to_string());
        }
    }

    fn load_finalized(&self, execution_id: &str, artifact_ref: &str) -> Option<Vec<u8>> {
        self.load_requests.fetch_add(1, Ordering::SeqCst);
        if artifact_ref != format!("memory://post-embed/{execution_id}") {
            return None;
        }
        self.committed
            .lock()
            .ok()
            .and_then(|committed| committed.get(execution_id).cloned())
    }
}

#[cfg(feature = "postgres")]
impl DeliveryArtifactRetriever for InMemoryArtifactStore {
    fn load_finalized_for_delivery(
        &self,
        execution_id: &str,
        artifact_ref: &str,
        _budget: DeliveryDownloadBudget,
    ) -> Result<DeliveryArtifactObject, DeliveryArtifactReadFailure> {
        if let Some(failure) = *self.delivery_failure.lock().expect("delivery failure lock") {
            self.load_requests.fetch_add(1, Ordering::SeqCst);
            return Err(failure);
        }
        let bytes = PostEmbedArtifactStore::load_finalized(self, execution_id, artifact_ref)
            .ok_or(DeliveryArtifactReadFailure::Unavailable)?;
        let content_type = self
            .delivery_content_type
            .lock()
            .expect("delivery content type lock")
            .clone()
            .unwrap_or_else(|| DELIVERY_REQUIRED_CONTENT_TYPE.to_string());
        let content_length_bytes = self
            .delivery_content_length
            .lock()
            .expect("delivery content length lock")
            .unwrap_or(bytes.len() as i64);
        Ok(DeliveryArtifactObject {
            bytes,
            content_type,
            content_length_bytes,
        })
    }
}

#[cfg(feature = "postgres")]
fn artifact_receipt(
    command: &InternalPostEmbedSigningCommand,
    signer_invocation_key: &str,
    final_signed_png_sha256: &str,
    operation: &str,
    durability_status: &str,
    idempotency_disposition: &str,
) -> PostEmbedArtifactReceipt {
    let issued_at = Utc::now();
    PostEmbedArtifactReceipt {
        schema_version: "hs-ai-production-post-embed-artifact-receipt-v1".to_string(),
        artifact_receipt_id: format!("artifact-{operation}-receipt-{}", command.execution_id),
        provider_id: "provider-production-object-store-qa".to_string(),
        operation: operation.to_string(),
        execution_id: command.execution_id.clone(),
        signer_invocation_key: signer_invocation_key.to_string(),
        artifact_ref: format!("memory://post-embed/{}", command.execution_id),
        final_signed_png_sha256: final_signed_png_sha256.to_string(),
        object_version: format!("object-version-{}", command.execution_id),
        idempotency_key: command.idempotency_key.clone(),
        idempotency_disposition: idempotency_disposition.to_string(),
        durability_status: durability_status.to_string(),
        issued_at,
        expires_at: issued_at + Duration::minutes(5),
        provider_signature: format!(
            "qa-object-store-signature-{operation}-{}",
            command.execution_id
        ),
    }
}

#[cfg(feature = "postgres")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScenarioResult {
    fixture_type: String,
    succeeded: bool,
    replayed: bool,
    reason_code: Option<String>,
    signer_invocations: usize,
    signer_billable_invocations: usize,
    artifact_stage_requests: usize,
    unique_artifact_stage_writes: usize,
    execution_status: Option<String>,
    artifact_status: Option<String>,
    recovery_attempts: Option<i32>,
    worker_recovery_state: Option<String>,
    worker_recovery_attempts: Option<i32>,
    recovery_audit_count: i64,
    signing_execution_count: i64,
    signing_audit_count: i64,
    confirm_audit_count: i64,
    manifest_count: i64,
    committed_ledger_count: i64,
    committed_ledger_quantity: i64,
    artifact_returned: bool,
    committed_artifact_count: usize,
    quarantined: bool,
}

#[cfg(feature = "postgres")]
#[derive(Debug)]
struct DatabaseSnapshot {
    execution_status: Option<String>,
    artifact_status: Option<String>,
    recovery_attempts: Option<i32>,
    worker_recovery_state: Option<String>,
    worker_recovery_attempts: Option<i32>,
    recovery_audit_count: i64,
    adapter_receipt_contract_version: Option<String>,
    signer_billable_invocation_id: Option<String>,
    artifact_stage_receipt_id: Option<String>,
    artifact_finalize_receipt_id: Option<String>,
    signer_receipt_contract_complete: Option<bool>,
    artifact_stage_receipt_contract_complete: Option<bool>,
    artifact_finalize_receipt_contract_complete: Option<bool>,
    signing_execution_count: i64,
    signing_audit_count: i64,
    confirm_audit_count: i64,
    manifest_count: i64,
    committed_ledger_count: i64,
    committed_ledger_quantity: i64,
}

#[cfg(feature = "postgres")]
fn main() {
    let result = thread::Builder::new()
        .name("ai-transparency-post-embed-qa".to_string())
        .stack_size(32 * 1024 * 1024)
        .spawn(|| {
            let runtime = tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .map_err(|error| error.to_string())?;
            runtime
                .block_on(run_qa())
                .map_err(|error| error.to_string())
        })
        .expect("spawn post-embed QA thread")
        .join()
        .expect("join post-embed QA thread");
    if let Err(error) = result {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

#[cfg(feature = "postgres")]
async fn run_qa() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| "missing disposable PostgreSQL URL for post-embed signing QA")?;
    if !safe_smoke_url(&database_url) {
        return Err(
            "refusing post-embed signing QA against non-disposable URL; require localhost/127.0.0.1 and hiddenshield_migrate_smoke"
                .into(),
        );
    }
    let pool = sqlx::PgPool::connect(&database_url).await?;
    reset_schema(&pool).await?;
    assert_fixture_contracts()?;

    let mut results = Vec::new();
    results.push(
        run_scenario(
            &pool,
            "success",
            false,
            PostEmbedSigningFailureInjection::None,
            None,
        )
        .await?,
    );
    results.push(
        run_scenario(
            &pool,
            "signer_rejected",
            true,
            PostEmbedSigningFailureInjection::None,
            Some(REASON_SIGNER_REJECTED),
        )
        .await?,
    );
    results.push(
        run_scenario(
            &pool,
            "receipt_hash_mismatch",
            false,
            PostEmbedSigningFailureInjection::ReceiptHashMismatch,
            Some(REASON_RECEIPT_HASH_MISMATCH),
        )
        .await?,
    );
    results.push(
        run_scenario(
            &pool,
            "c2pa_readback_failure",
            false,
            PostEmbedSigningFailureInjection::C2paReadbackFailure,
            Some(REASON_C2PA_READBACK_FAILED),
        )
        .await?,
    );
    results.push(
        run_scenario(
            &pool,
            "v3_readback_failure",
            false,
            PostEmbedSigningFailureInjection::V3ReadbackFailure,
            Some(REASON_V3_READBACK_FAILED),
        )
        .await?,
    );
    results.push(
        run_scenario(
            &pool,
            "confirm_rollback",
            false,
            PostEmbedSigningFailureInjection::ConfirmRollback,
            Some(REASON_CONFIRM_ROLLED_BACK),
        )
        .await?,
    );
    results.push(run_duplicate_replay(&pool).await?);
    results.push(run_concurrent_reservation(&pool).await?);
    results.push(run_artifact_finalize_recovery(&pool).await?);
    results.push(
        run_crash_recovery(
            &pool,
            "crash_after_reservation",
            PostEmbedSigningFailureInjection::CrashAfterReservation,
            "reserved",
            0,
            0,
            1,
            1,
        )
        .await?,
    );
    results.push(run_recovery_worker_reserved(&pool).await?);
    results.push(run_recovery_worker_artifact_pending(&pool).await?);
    results.push(run_recovery_worker_dead_letter(&pool).await?);
    results.push(run_recovery_worker_concurrent_claim(&pool).await?);
    results.push(Box::pin(run_dead_letter_inspect_requeue(&pool)).await?);
    results.push(Box::pin(run_dead_letter_audit_failure_rollback(&pool)).await?);
    results.push(Box::pin(run_confirmed_delivery_envelope(&pool)).await?);
    results.push(Box::pin(run_delivery_envelope_recovery_not_completed(&pool)).await?);
    Box::pin(run_delivery_retrieval_gate(&pool)).await?;
    Box::pin(run_delivery_revoke_resource_budget_gate(&pool)).await?;
    Box::pin(run_delivery_security_observability_gate(&pool)).await?;
    results.push(
        run_crash_recovery(
            &pool,
            "crash_after_signer",
            PostEmbedSigningFailureInjection::CrashAfterSigner,
            "reserved",
            1,
            0,
            2,
            1,
        )
        .await?,
    );
    results.push(
        run_crash_recovery(
            &pool,
            "crash_after_artifact_stage",
            PostEmbedSigningFailureInjection::CrashAfterArtifactStage,
            "reserved",
            1,
            1,
            2,
            2,
        )
        .await?,
    );
    results.push(
        run_crash_recovery(
            &pool,
            "crash_after_confirm",
            PostEmbedSigningFailureInjection::CrashAfterConfirm,
            "artifact_pending",
            1,
            1,
            1,
            1,
        )
        .await?,
    );

    println!("{}", serde_json::to_string_pretty(&results)?);
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("ai_transparency_post_embed_signing_qa requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
async fn run_scenario(
    pool: &sqlx::PgPool,
    fixture_type: &str,
    signer_rejects: bool,
    failure_injection: PostEmbedSigningFailureInjection,
    expected_reason: Option<&str>,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    seed_production_state(pool, fixture_type).await?;
    let command = command(fixture_type, failure_injection);
    let signer = ControlledSigner::new(signer_rejects, 0);
    let invocations = Arc::clone(&signer.requests);
    let store = InMemoryArtifactStore::default();
    let mut connection = pool.acquire().await?;
    let outcome = execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    let snapshot = snapshot(pool, fixture_type).await?;
    let expected_success = expected_reason.is_none();
    if outcome.succeeded != expected_success
        || outcome.reason_code.as_deref() != expected_reason
        || outcome.replayed
    {
        return Err(format!("unexpected {fixture_type} outcome: {outcome:?}").into());
    }
    assert_scenario(
        fixture_type,
        failure_injection,
        signer_rejects,
        &outcome,
        &snapshot,
        &store,
        invocations.load(Ordering::SeqCst),
    )?;
    assert_runtime_matches_fixture(
        fixture_type,
        &outcome,
        &snapshot,
        &store,
        invocations.load(Ordering::SeqCst),
        None,
    )?;
    Ok(ScenarioResult {
        fixture_type: fixture_type.to_string(),
        succeeded: outcome.succeeded,
        replayed: outcome.replayed,
        reason_code: outcome.reason_code,
        signer_invocations: invocations.load(Ordering::SeqCst),
        signer_billable_invocations: signer.billable_invocation_count(),
        artifact_stage_requests: store.stage_request_count(),
        unique_artifact_stage_writes: store.unique_stage_write_count(),
        execution_status: snapshot.execution_status,
        artifact_status: snapshot.artifact_status,
        recovery_attempts: snapshot.recovery_attempts,
        worker_recovery_state: snapshot.worker_recovery_state,
        worker_recovery_attempts: snapshot.worker_recovery_attempts,
        recovery_audit_count: snapshot.recovery_audit_count,
        signing_execution_count: snapshot.signing_execution_count,
        signing_audit_count: snapshot.signing_audit_count,
        confirm_audit_count: snapshot.confirm_audit_count,
        manifest_count: snapshot.manifest_count,
        committed_ledger_count: snapshot.committed_ledger_count,
        committed_ledger_quantity: snapshot.committed_ledger_quantity,
        artifact_returned: outcome.final_signed_png_bytes.is_some(),
        committed_artifact_count: store.committed_count(),
        quarantined: store.is_quarantined(&command.execution_id),
    })
}

#[cfg(feature = "postgres")]
async fn run_duplicate_replay(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "duplicate_replay";
    seed_production_state(pool, fixture_type).await?;
    let command = command(fixture_type, PostEmbedSigningFailureInjection::None);
    let signer = ControlledSigner::new(false, 0);
    let invocations = Arc::clone(&signer.requests);
    let store = InMemoryArtifactStore::default();
    let mut connection = pool.acquire().await?;
    let first = execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    let before = snapshot(pool, fixture_type).await?;
    let second = execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    let after = snapshot(pool, fixture_type).await?;
    if !first.succeeded
        || !second.succeeded
        || !second.replayed
        || second.final_signed_png_bytes.is_none()
        || invocations.load(Ordering::SeqCst) != 1
        || before.signing_execution_count != after.signing_execution_count
        || before.signing_audit_count != after.signing_audit_count
        || before.confirm_audit_count != after.confirm_audit_count
        || before.manifest_count != after.manifest_count
        || before.committed_ledger_count != after.committed_ledger_count
    {
        return Err(format!(
            "duplicate replay was not idempotent: first={first:?}, second={second:?}, before={before:?}, after={after:?}"
        )
        .into());
    }
    assert_runtime_matches_fixture(
        fixture_type,
        &second,
        &after,
        &store,
        invocations.load(Ordering::SeqCst),
        Some(&before),
    )?;
    Ok(ScenarioResult {
        fixture_type: fixture_type.to_string(),
        succeeded: second.succeeded,
        replayed: second.replayed,
        reason_code: second.reason_code,
        signer_invocations: invocations.load(Ordering::SeqCst),
        signer_billable_invocations: signer.billable_invocation_count(),
        artifact_stage_requests: store.stage_request_count(),
        unique_artifact_stage_writes: store.unique_stage_write_count(),
        execution_status: after.execution_status,
        artifact_status: after.artifact_status,
        recovery_attempts: after.recovery_attempts,
        worker_recovery_state: after.worker_recovery_state,
        worker_recovery_attempts: after.worker_recovery_attempts,
        recovery_audit_count: after.recovery_audit_count,
        signing_execution_count: after.signing_execution_count,
        signing_audit_count: after.signing_audit_count,
        confirm_audit_count: after.confirm_audit_count,
        manifest_count: after.manifest_count,
        committed_ledger_count: after.committed_ledger_count,
        committed_ledger_quantity: after.committed_ledger_quantity,
        artifact_returned: second.final_signed_png_bytes.is_some(),
        committed_artifact_count: store.committed_count(),
        quarantined: store.is_quarantined(&command.execution_id),
    })
}

#[cfg(feature = "postgres")]
async fn run_concurrent_reservation(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "concurrent_reservation";
    seed_production_state(pool, fixture_type).await?;
    let command = command(fixture_type, PostEmbedSigningFailureInjection::None);
    let signer = Arc::new(ControlledSigner::new(false, 300));
    let invocations = Arc::clone(&signer.requests);
    let store = Arc::new(InMemoryArtifactStore::default());

    let first_pool = pool.clone();
    let first_command = command.clone();
    let first_signer = Arc::clone(&signer);
    let first_store = Arc::clone(&store);
    let first = tokio::spawn(async move {
        let mut connection = first_pool
            .acquire()
            .await
            .map_err(|error| error.to_string())?;
        execute_postgres_internal_post_embed_signing(
            &mut connection,
            &first_command,
            &AllowAuthorization,
            first_signer.as_ref(),
            &ControlledReadback,
            first_store.as_ref(),
        )
        .await
        .map_err(|error| error.to_string())
    });

    for _ in 0..100 {
        if invocations.load(Ordering::SeqCst) == 1 {
            break;
        }
        tokio::time::sleep(StdDuration::from_millis(10)).await;
    }
    if invocations.load(Ordering::SeqCst) != 1 {
        return Err("first concurrent signer invocation did not start".into());
    }
    let reserved = snapshot(pool, fixture_type).await?;
    if reserved.execution_status.as_deref() != Some("reserved")
        || reserved.artifact_status.as_deref() != Some("none")
        || reserved.signing_execution_count != 1
        || reserved.confirm_audit_count != 0
        || reserved.committed_ledger_count != 0
    {
        return Err(format!("concurrent reservation was not durable: {reserved:?}").into());
    }

    let second_pool = pool.clone();
    let second_command = command.clone();
    let second_signer = Arc::clone(&signer);
    let second_store = Arc::clone(&store);
    let second = tokio::spawn(async move {
        let mut connection = second_pool
            .acquire()
            .await
            .map_err(|error| error.to_string())?;
        execute_postgres_internal_post_embed_signing(
            &mut connection,
            &second_command,
            &AllowAuthorization,
            second_signer.as_ref(),
            &ControlledReadback,
            second_store.as_ref(),
        )
        .await
        .map_err(|error| error.to_string())
    });

    let first_outcome = first.await??;
    let second_outcome = second.await??;
    let snapshot = snapshot(pool, fixture_type).await?;
    if !first_outcome.succeeded
        || first_outcome.replayed
        || !second_outcome.succeeded
        || !second_outcome.replayed
        || invocations.load(Ordering::SeqCst) != 1
        || snapshot.execution_status.as_deref() != Some("confirmed")
        || snapshot.signing_execution_count != 1
        || snapshot.signing_audit_count != 2
        || snapshot.confirm_audit_count != 1
        || snapshot.committed_ledger_count != 1
        || snapshot.committed_ledger_quantity != 1
        || store.committed_count() != 1
    {
        return Err(format!(
            "concurrent reservation gate failed: first={first_outcome:?}, second={second_outcome:?}, snapshot={snapshot:?}"
        )
        .into());
    }
    Ok(ScenarioResult {
        fixture_type: fixture_type.to_string(),
        succeeded: second_outcome.succeeded,
        replayed: second_outcome.replayed,
        reason_code: second_outcome.reason_code,
        signer_invocations: invocations.load(Ordering::SeqCst),
        signer_billable_invocations: signer.billable_invocation_count(),
        artifact_stage_requests: store.stage_request_count(),
        unique_artifact_stage_writes: store.unique_stage_write_count(),
        execution_status: snapshot.execution_status,
        artifact_status: snapshot.artifact_status,
        recovery_attempts: snapshot.recovery_attempts,
        worker_recovery_state: snapshot.worker_recovery_state,
        worker_recovery_attempts: snapshot.worker_recovery_attempts,
        recovery_audit_count: snapshot.recovery_audit_count,
        signing_execution_count: snapshot.signing_execution_count,
        signing_audit_count: snapshot.signing_audit_count,
        confirm_audit_count: snapshot.confirm_audit_count,
        manifest_count: snapshot.manifest_count,
        committed_ledger_count: snapshot.committed_ledger_count,
        committed_ledger_quantity: snapshot.committed_ledger_quantity,
        artifact_returned: second_outcome.final_signed_png_bytes.is_some(),
        committed_artifact_count: store.committed_count(),
        quarantined: store.is_quarantined(&command.execution_id),
    })
}

#[cfg(feature = "postgres")]
async fn run_artifact_finalize_recovery(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "artifact_finalize_recovery";
    seed_production_state(pool, fixture_type).await?;
    let command = command(fixture_type, PostEmbedSigningFailureInjection::None);
    let signer = ControlledSigner::new(false, 0);
    let invocations = Arc::clone(&signer.requests);
    let store = InMemoryArtifactStore::default();
    store.fail_next_finalize();
    let mut connection = pool.acquire().await?;
    let first = execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    let pending = snapshot(pool, fixture_type).await?;
    if first.succeeded
        || first.reason_code.as_deref() != Some(REASON_ARTIFACT_FINALIZE_PENDING)
        || !first.artifact_pending
        || first.final_signed_png_bytes.is_some()
        || pending.execution_status.as_deref() != Some("artifact_pending")
        || pending.artifact_status.as_deref() != Some("pending_finalize")
        || pending.recovery_attempts != Some(1)
        || pending.signing_audit_count != 2
        || pending.confirm_audit_count != 1
        || pending.committed_ledger_count != 0
        || pending.committed_ledger_quantity != 0
        || store.committed_count() != 0
        || invocations.load(Ordering::SeqCst) != 1
    {
        return Err(format!(
            "artifact pending gate failed: outcome={first:?}, snapshot={pending:?}"
        )
        .into());
    }

    let recovered = execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    let snapshot = snapshot(pool, fixture_type).await?;
    if !recovered.succeeded
        || !recovered.replayed
        || !recovered.artifact_recovery_performed
        || recovered.signer_invoked
        || recovered.final_signed_png_bytes.is_none()
        || invocations.load(Ordering::SeqCst) != 1
        || snapshot.execution_status.as_deref() != Some("confirmed")
        || snapshot.artifact_status.as_deref() != Some("finalized")
        || snapshot.recovery_attempts != Some(1)
        || snapshot.signing_execution_count != 1
        || snapshot.signing_audit_count != 3
        || snapshot.confirm_audit_count != 1
        || snapshot.committed_ledger_count != 1
        || snapshot.committed_ledger_quantity != 1
        || store.committed_count() != 1
    {
        return Err(format!(
            "artifact recovery gate failed: outcome={recovered:?}, snapshot={snapshot:?}"
        )
        .into());
    }
    Ok(ScenarioResult {
        fixture_type: fixture_type.to_string(),
        succeeded: recovered.succeeded,
        replayed: recovered.replayed,
        reason_code: recovered.reason_code,
        signer_invocations: invocations.load(Ordering::SeqCst),
        signer_billable_invocations: signer.billable_invocation_count(),
        artifact_stage_requests: store.stage_request_count(),
        unique_artifact_stage_writes: store.unique_stage_write_count(),
        execution_status: snapshot.execution_status,
        artifact_status: snapshot.artifact_status,
        recovery_attempts: snapshot.recovery_attempts,
        worker_recovery_state: snapshot.worker_recovery_state,
        worker_recovery_attempts: snapshot.worker_recovery_attempts,
        recovery_audit_count: snapshot.recovery_audit_count,
        signing_execution_count: snapshot.signing_execution_count,
        signing_audit_count: snapshot.signing_audit_count,
        confirm_audit_count: snapshot.confirm_audit_count,
        manifest_count: snapshot.manifest_count,
        committed_ledger_count: snapshot.committed_ledger_count,
        committed_ledger_quantity: snapshot.committed_ledger_quantity,
        artifact_returned: recovered.final_signed_png_bytes.is_some(),
        committed_artifact_count: store.committed_count(),
        quarantined: store.is_quarantined(&command.execution_id),
    })
}

#[cfg(feature = "postgres")]
#[allow(clippy::too_many_arguments)]
async fn run_crash_recovery(
    pool: &sqlx::PgPool,
    fixture_type: &str,
    failure_injection: PostEmbedSigningFailureInjection,
    expected_initial_status: &str,
    expected_initial_signer_requests: usize,
    expected_initial_stage_requests: usize,
    expected_final_signer_requests: usize,
    expected_final_stage_requests: usize,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    seed_production_state(pool, fixture_type).await?;
    let command = command(fixture_type, failure_injection);
    let signer = ControlledSigner::new(false, 0);
    let store = InMemoryArtifactStore::default();
    let mut connection = pool.acquire().await?;
    let crashed = execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    let crashed_snapshot = snapshot(pool, fixture_type).await?;
    if crashed.succeeded
        || crashed.reason_code.as_deref() != Some(REASON_CRASH_INJECTED)
        || crashed.final_signed_png_bytes.is_some()
        || crashed_snapshot.execution_status.as_deref() != Some(expected_initial_status)
        || crashed_snapshot.adapter_receipt_contract_version.as_deref()
            != Some("hs-ai-production-adapter-receipts-v1")
        || signer.request_count() != expected_initial_signer_requests
        || signer.billable_invocation_count() != usize::from(expected_initial_signer_requests > 0)
        || store.stage_request_count() != expected_initial_stage_requests
        || store.unique_stage_write_count() != usize::from(expected_initial_stage_requests > 0)
        || crashed_snapshot.committed_ledger_count != 0
        || crashed_snapshot.committed_ledger_quantity != 0
    {
        return Err(format!(
            "{fixture_type} crash-state gate failed: outcome={crashed:?}, snapshot={crashed_snapshot:?}, signerRequests={}, billable={}, stageRequests={}, uniqueStageWrites={}",
            signer.request_count(),
            signer.billable_invocation_count(),
            store.stage_request_count(),
            store.unique_stage_write_count()
        )
        .into());
    }
    if expected_initial_status == "reserved" {
        sqlx::query(
            "UPDATE ai_post_embed_signing_executions
             SET lease_expires_at = NOW() - INTERVAL '1 second'
             WHERE execution_id = $1 AND status = 'reserved'",
        )
        .bind(format!("execution-{fixture_type}"))
        .execute(&mut *connection)
        .await?;
    }
    let mut recovery_command = command.clone();
    recovery_command.failure_injection = PostEmbedSigningFailureInjection::None;
    let recovered = execute_postgres_internal_post_embed_signing(
        &mut connection,
        &recovery_command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    let recovered_snapshot = snapshot(pool, fixture_type).await?;
    if !recovered.succeeded
        || !recovered.replayed
        || !recovered.artifact_recovery_performed
        || recovered.final_signed_png_bytes.is_none()
        || signer.request_count() != expected_final_signer_requests
        || signer.billable_invocation_count() != 1
        || store.stage_request_count() != expected_final_stage_requests
        || store.unique_stage_write_count() != 1
        || store.committed_count() != 1
        || recovered_snapshot.execution_status.as_deref() != Some("confirmed")
        || recovered_snapshot.artifact_status.as_deref() != Some("finalized")
        || recovered_snapshot.signing_execution_count != 1
        || recovered_snapshot.signing_audit_count != 2
        || recovered_snapshot.confirm_audit_count != 1
        || recovered_snapshot.manifest_count != 1
        || recovered_snapshot.committed_ledger_count != 1
        || recovered_snapshot.committed_ledger_quantity != 1
        || recovered_snapshot.signer_billable_invocation_id.is_none()
        || recovered_snapshot.artifact_stage_receipt_id.is_none()
        || recovered_snapshot.artifact_finalize_receipt_id.is_none()
        || recovered_snapshot.signer_receipt_contract_complete != Some(true)
        || recovered_snapshot.artifact_stage_receipt_contract_complete != Some(true)
        || recovered_snapshot.artifact_finalize_receipt_contract_complete != Some(true)
    {
        return Err(format!(
            "{fixture_type} recovery gate failed: outcome={recovered:?}, snapshot={recovered_snapshot:?}, signerRequests={}, billable={}, stageRequests={}, uniqueStageWrites={}",
            signer.request_count(),
            signer.billable_invocation_count(),
            store.stage_request_count(),
            store.unique_stage_write_count()
        )
        .into());
    }
    assert_runtime_matches_fixture(
        fixture_type,
        &recovered,
        &recovered_snapshot,
        &store,
        signer.request_count(),
        Some(&crashed_snapshot),
    )?;
    Ok(ScenarioResult {
        fixture_type: fixture_type.to_string(),
        succeeded: recovered.succeeded,
        replayed: recovered.replayed,
        reason_code: recovered.reason_code,
        signer_invocations: signer.request_count(),
        signer_billable_invocations: signer.billable_invocation_count(),
        artifact_stage_requests: store.stage_request_count(),
        unique_artifact_stage_writes: store.unique_stage_write_count(),
        execution_status: recovered_snapshot.execution_status,
        artifact_status: recovered_snapshot.artifact_status,
        recovery_attempts: recovered_snapshot.recovery_attempts,
        worker_recovery_state: recovered_snapshot.worker_recovery_state,
        worker_recovery_attempts: recovered_snapshot.worker_recovery_attempts,
        recovery_audit_count: recovered_snapshot.recovery_audit_count,
        signing_execution_count: recovered_snapshot.signing_execution_count,
        signing_audit_count: recovered_snapshot.signing_audit_count,
        confirm_audit_count: recovered_snapshot.confirm_audit_count,
        manifest_count: recovered_snapshot.manifest_count,
        committed_ledger_count: recovered_snapshot.committed_ledger_count,
        committed_ledger_quantity: recovered_snapshot.committed_ledger_quantity,
        artifact_returned: recovered.final_signed_png_bytes.is_some(),
        committed_artifact_count: store.committed_count(),
        quarantined: store.is_quarantined(&command.execution_id),
    })
}

#[cfg(feature = "postgres")]
fn recovery_worker_config(worker_id: &str, max_attempts: i32) -> PostEmbedRecoveryWorkerConfig {
    PostEmbedRecoveryWorkerConfig {
        worker_id: worker_id.to_string(),
        batch_size: 1,
        artifact_pending_timeout: Duration::seconds(1),
        recovery_lease_duration: Duration::seconds(30),
        base_backoff: Duration::seconds(1),
        max_backoff: Duration::seconds(4),
        max_attempts,
    }
}

#[cfg(feature = "postgres")]
async fn run_recovery_worker_reserved(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "recovery_worker_expired_reserved";
    seed_production_state(pool, fixture_type).await?;
    let mut command = command(
        fixture_type,
        PostEmbedSigningFailureInjection::CrashAfterReservation,
    );
    let signer = ControlledSigner::new(false, 0);
    let store = InMemoryArtifactStore::default();
    let mut connection = pool.acquire().await?;
    let crashed = execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    if crashed.reason_code.as_deref() != Some(REASON_CRASH_INJECTED) {
        return Err("reserved recovery worker setup did not crash after reservation".into());
    }
    sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET lease_expires_at = NOW() - INTERVAL '1 second',
             next_recovery_at = NOW() - INTERVAL '1 second'
         WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .execute(pool)
    .await?;
    command.failure_injection = PostEmbedSigningFailureInjection::None;
    let loader = StaticRecoveryCommandLoader { command };
    let outcome = run_postgres_post_embed_recovery_batch(
        pool,
        &recovery_worker_config("worker-reserved", 3),
        &loader,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    let snapshot = snapshot(pool, fixture_type).await?;
    if outcome.claimed != 1
        || outcome.succeeded != 1
        || outcome.retry_scheduled != 0
        || outcome.dead_lettered != 0
        || signer.request_count() != 1
        || signer.billable_invocation_count() != 1
        || snapshot.execution_status.as_deref() != Some("confirmed")
        || snapshot.worker_recovery_state.as_deref() != Some("completed")
        || snapshot.worker_recovery_attempts != Some(1)
        || snapshot.recovery_audit_count != 2
        || snapshot.committed_ledger_count != 1
    {
        return Err(format!(
            "expired reserved worker recovery failed: outcome={outcome:?}, snapshot={snapshot:?}"
        )
        .into());
    }
    Ok(worker_scenario_result(
        fixture_type,
        &snapshot,
        &signer,
        &store,
        true,
        true,
        None,
    ))
}

#[cfg(feature = "postgres")]
async fn run_recovery_worker_artifact_pending(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "recovery_worker_artifact_pending_timeout";
    seed_production_state(pool, fixture_type).await?;
    let command = command(fixture_type, PostEmbedSigningFailureInjection::None);
    let signer = ControlledSigner::new(false, 0);
    let store = InMemoryArtifactStore::default();
    store.fail_next_finalize();
    let mut connection = pool.acquire().await?;
    let pending = execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    if pending.reason_code.as_deref() != Some(REASON_ARTIFACT_FINALIZE_PENDING) {
        return Err("artifact pending worker setup did not preserve pending artifact".into());
    }
    sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET updated_at = NOW() - INTERVAL '2 seconds',
             next_recovery_at = NOW() - INTERVAL '1 second'
         WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .execute(pool)
    .await?;
    let loader = StaticRecoveryCommandLoader { command };
    let outcome = run_postgres_post_embed_recovery_batch(
        pool,
        &recovery_worker_config("worker-artifact", 3),
        &loader,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    let snapshot = snapshot(pool, fixture_type).await?;
    if outcome.claimed != 1
        || outcome.succeeded != 1
        || signer.request_count() != 1
        || signer.billable_invocation_count() != 1
        || store.unique_stage_write_count() != 1
        || snapshot.execution_status.as_deref() != Some("confirmed")
        || snapshot.worker_recovery_state.as_deref() != Some("completed")
        || snapshot.worker_recovery_attempts != Some(1)
        || snapshot.recovery_attempts != Some(1)
        || snapshot.recovery_audit_count != 2
        || snapshot.confirm_audit_count != 1
        || snapshot.committed_ledger_count != 1
    {
        return Err(format!(
            "artifact pending worker recovery failed: outcome={outcome:?}, snapshot={snapshot:?}"
        )
        .into());
    }
    Ok(worker_scenario_result(
        fixture_type,
        &snapshot,
        &signer,
        &store,
        true,
        true,
        None,
    ))
}

#[cfg(feature = "postgres")]
async fn run_recovery_worker_dead_letter(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "recovery_worker_backoff_dead_letter";
    seed_production_state(pool, fixture_type).await?;
    let command = command(
        fixture_type,
        PostEmbedSigningFailureInjection::CrashAfterReservation,
    );
    let signer = ControlledSigner::new(false, 0);
    let store = InMemoryArtifactStore::default();
    let mut connection = pool.acquire().await?;
    execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET lease_expires_at = NOW() - INTERVAL '1 second',
             next_recovery_at = NOW() - INTERVAL '1 second'
         WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .execute(pool)
    .await?;
    let config = recovery_worker_config("worker-dead-letter", 3);
    for expected_attempt in 1..=3 {
        let outcome = run_postgres_post_embed_recovery_batch(
            pool,
            &config,
            &RejectingRecoveryCommandLoader,
            &AllowAuthorization,
            &signer,
            &ControlledReadback,
            &store,
        )
        .await?;
        let expected_dead_letter = expected_attempt == 3;
        if outcome.claimed != 1
            || outcome.succeeded != 0
            || outcome.retry_scheduled != usize::from(!expected_dead_letter)
            || outcome.dead_lettered != usize::from(expected_dead_letter)
            || outcome.items[0].attempt != expected_attempt
            || outcome.items[0].reason_code != RECOVERY_REASON_LOADER_FAILED
            || outcome.items[0].next_attempt_at.is_some() == expected_dead_letter
        {
            return Err(
                format!("recovery backoff attempt {expected_attempt} failed: {outcome:?}").into(),
            );
        }
        if !expected_dead_letter {
            sqlx::query(
                "UPDATE ai_post_embed_signing_executions
                 SET next_recovery_at = NOW() - INTERVAL '1 second'
                 WHERE execution_id = $1",
            )
            .bind(&command.execution_id)
            .execute(pool)
            .await?;
        }
    }
    let snapshot = snapshot(pool, fixture_type).await?;
    if snapshot.execution_status.as_deref() != Some("reserved")
        || snapshot.worker_recovery_state.as_deref() != Some("dead_letter")
        || snapshot.worker_recovery_attempts != Some(3)
        || snapshot.recovery_audit_count != 6
        || snapshot.committed_ledger_count != 0
        || signer.request_count() != 0
    {
        return Err(format!("recovery dead-letter state mismatch: {snapshot:?}").into());
    }
    let update_audit = sqlx::query(
        "UPDATE ai_post_embed_recovery_audit_events
         SET reason_code = 'mutated'
         WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .execute(pool)
    .await;
    let delete_audit =
        sqlx::query("DELETE FROM ai_post_embed_recovery_audit_events WHERE execution_id = $1")
            .bind(&command.execution_id)
            .execute(pool)
            .await;
    if update_audit.is_ok() || delete_audit.is_ok() {
        return Err("recovery audit append-only trigger accepted mutation".into());
    }
    Ok(worker_scenario_result(
        fixture_type,
        &snapshot,
        &signer,
        &store,
        false,
        false,
        Some(RECOVERY_REASON_LOADER_FAILED),
    ))
}

#[cfg(feature = "postgres")]
async fn run_recovery_worker_concurrent_claim(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "recovery_worker_concurrent_claim";
    seed_production_state(pool, fixture_type).await?;
    let mut command = command(
        fixture_type,
        PostEmbedSigningFailureInjection::CrashAfterReservation,
    );
    let signer = ControlledSigner::new(false, 150);
    let store = InMemoryArtifactStore::default();
    let mut connection = pool.acquire().await?;
    execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET lease_expires_at = NOW() - INTERVAL '1 second',
             next_recovery_at = NOW() - INTERVAL '1 second'
         WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .execute(pool)
    .await?;
    command.failure_injection = PostEmbedSigningFailureInjection::None;
    let loader = StaticRecoveryCommandLoader { command };
    let worker_a = recovery_worker_config("worker-concurrent-a", 3);
    let worker_b = recovery_worker_config("worker-concurrent-b", 3);
    let (outcome_a, outcome_b) = tokio::join!(
        run_postgres_post_embed_recovery_batch(
            pool,
            &worker_a,
            &loader,
            &AllowAuthorization,
            &signer,
            &ControlledReadback,
            &store,
        ),
        run_postgres_post_embed_recovery_batch(
            pool,
            &worker_b,
            &loader,
            &AllowAuthorization,
            &signer,
            &ControlledReadback,
            &store,
        )
    );
    let outcome_a = outcome_a?;
    let outcome_b = outcome_b?;
    let snapshot = snapshot(pool, fixture_type).await?;
    if outcome_a.claimed + outcome_b.claimed != 1
        || outcome_a.succeeded + outcome_b.succeeded != 1
        || signer.request_count() != 1
        || signer.billable_invocation_count() != 1
        || store.unique_stage_write_count() != 1
        || snapshot.worker_recovery_state.as_deref() != Some("completed")
        || snapshot.worker_recovery_attempts != Some(1)
        || snapshot.recovery_audit_count != 2
        || snapshot.committed_ledger_count != 1
    {
        return Err(format!(
            "concurrent recovery claim failed: a={outcome_a:?}, b={outcome_b:?}, snapshot={snapshot:?}"
        )
        .into());
    }
    Ok(worker_scenario_result(
        fixture_type,
        &snapshot,
        &signer,
        &store,
        true,
        true,
        None,
    ))
}

#[cfg(feature = "postgres")]
async fn run_dead_letter_inspect_requeue(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "dead_letter_inspect_requeue_worker_conflict";
    let (mut signing_command, signer, store) =
        create_dead_letter_execution(pool, fixture_type).await?;
    let preflight = ChangeCommandPreflight {
        iam: &AllowGovernance,
        references: &AllowGovernance,
    };
    let inspect_command = DeadLetterInspectCommand {
        execution_id: signing_command.execution_id.clone(),
        tenant_id: format!("tenant-{fixture_type}"),
        workspace_id: format!("workspace-{fixture_type}"),
        environment: "production".to_string(),
        actor_snapshot_id: format!("actor-snapshot-{fixture_type}-auditor"),
        actor_token_hash: "auditor-token-hash".to_string(),
    };
    let mut inspect_connection = pool.acquire().await?;
    let inspection =
        inspect_postgres_dead_letter(&mut inspect_connection, &inspect_command, &preflight)
            .await?
            .ok_or("dead-letter inspection unexpectedly returned not_found")?;
    drop(inspect_connection);
    if inspection.recovery_state != "dead_letter"
        || inspection.worker_recovery_attempts != 3
        || inspection.recovery_control_version != 1
        || inspection.last_recovery_reason != RECOVERY_REASON_LOADER_FAILED
    {
        return Err(format!("dead-letter inspection mismatch: {inspection:?}").into());
    }
    let inspection_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_post_embed_dead_letter_inspection_audit_events
         WHERE execution_id = $1 AND outcome = 'succeeded'",
    )
    .bind(&signing_command.execution_id)
    .fetch_one(pool)
    .await?;
    if inspection_audit_count != 1 {
        return Err("dead-letter inspection audit was not appended".into());
    }
    let update_inspection_audit = sqlx::query(
        "UPDATE ai_post_embed_dead_letter_inspection_audit_events
         SET reason_code = 'mutated' WHERE execution_id = $1",
    )
    .bind(&signing_command.execution_id)
    .execute(pool)
    .await;
    let delete_inspection_audit = sqlx::query(
        "DELETE FROM ai_post_embed_dead_letter_inspection_audit_events
         WHERE execution_id = $1",
    )
    .bind(&signing_command.execution_id)
    .execute(pool)
    .await;
    if update_inspection_audit.is_ok() || delete_inspection_audit.is_ok() {
        return Err("dead-letter inspection audit accepted mutation".into());
    }

    let mut invalid_digest =
        dead_letter_requeue_command(fixture_type, DeadLetterRequeueMode::SubmitRequest, false);
    invalid_digest.request_digest = "invalid-digest".to_string();
    let mut invalid_connection = pool.acquire().await?;
    let invalid_outcome = execute_postgres_dead_letter_requeue(
        &mut invalid_connection,
        &invalid_digest,
        &preflight,
        &NoopDeadLetterRequeueExecutionHook,
    )
    .await?;
    drop(invalid_connection);
    if invalid_outcome.succeeded
        || invalid_outcome.reason_code.as_deref() != Some(REASON_REQUEST_DIGEST_MISMATCH)
    {
        return Err(format!("invalid digest was not rejected: {invalid_outcome:?}").into());
    }

    let mut same_actor =
        dead_letter_requeue_command(fixture_type, DeadLetterRequeueMode::ApproveRequest, false);
    same_actor.approver_actor_id = same_actor.requester_actor_id.clone();
    same_actor.request_digest = canonical_dead_letter_requeue_digest(&same_actor);
    let mut same_actor_connection = pool.acquire().await?;
    let same_actor_outcome = execute_postgres_dead_letter_requeue(
        &mut same_actor_connection,
        &same_actor,
        &preflight,
        &NoopDeadLetterRequeueExecutionHook,
    )
    .await?;
    drop(same_actor_connection);
    if same_actor_outcome.succeeded
        || same_actor_outcome.reason_code.as_deref() != Some(REASON_TARGET_STATE_CONFLICT)
    {
        return Err(format!("same-actor approval was not rejected: {same_actor_outcome:?}").into());
    }
    let pre_submit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_transparency_change_requests
         WHERE target_scope_key = $1",
    )
    .bind(format!(
        "post_embed_recovery:{}",
        signing_command.execution_id
    ))
    .fetch_one(pool)
    .await?;
    if pre_submit_count != 0 {
        return Err("preflight rejection leaked change request writes".into());
    }

    let submit_command =
        dead_letter_requeue_command(fixture_type, DeadLetterRequeueMode::SubmitRequest, false);
    let mut submit_connection = pool.acquire().await?;
    let submit_outcome = execute_postgres_dead_letter_requeue(
        &mut submit_connection,
        &submit_command,
        &preflight,
        &NoopDeadLetterRequeueExecutionHook,
    )
    .await?;
    drop(submit_connection);
    if !submit_outcome.succeeded || submit_outcome.request_status != "pending_review" {
        return Err(format!("dead-letter submit failed: {submit_outcome:?}").into());
    }
    let mut replay_connection = pool.acquire().await?;
    let replay_outcome = execute_postgres_dead_letter_requeue(
        &mut replay_connection,
        &submit_command,
        &preflight,
        &NoopDeadLetterRequeueExecutionHook,
    )
    .await?;
    drop(replay_connection);
    if replay_outcome.succeeded
        || replay_outcome.reason_code.as_deref() != Some(DEAD_LETTER_REASON_IDEMPOTENCY_REPLAY)
    {
        return Err(format!("dead-letter duplicate submit mismatch: {replay_outcome:?}").into());
    }
    let pending_snapshot = snapshot(pool, fixture_type).await?;
    if pending_snapshot.worker_recovery_state.as_deref() != Some("dead_letter")
        || pending_snapshot.worker_recovery_attempts != Some(3)
    {
        return Err("submit mutated the dead-letter projection before approval".into());
    }

    let execute_command = dead_letter_requeue_command(
        fixture_type,
        DeadLetterRequeueMode::ExecuteApprovedRequest,
        false,
    );
    let mut early_execute_connection = pool.acquire().await?;
    let early_execute = execute_postgres_dead_letter_requeue(
        &mut early_execute_connection,
        &execute_command,
        &preflight,
        &NoopDeadLetterRequeueExecutionHook,
    )
    .await?;
    drop(early_execute_connection);
    if early_execute.succeeded
        || early_execute.reason_code.as_deref() != Some(REASON_TARGET_STATE_CONFLICT)
    {
        return Err(format!("unapproved requeue execution was accepted: {early_execute:?}").into());
    }

    let approve_command =
        dead_letter_requeue_command(fixture_type, DeadLetterRequeueMode::ApproveRequest, false);
    let mut approve_connection = pool.acquire().await?;
    let approve_outcome = execute_postgres_dead_letter_requeue(
        &mut approve_connection,
        &approve_command,
        &preflight,
        &NoopDeadLetterRequeueExecutionHook,
    )
    .await?;
    drop(approve_connection);
    if !approve_outcome.succeeded || approve_outcome.request_status != "approved" {
        return Err(format!("dead-letter approval failed: {approve_outcome:?}").into());
    }

    let locked = Arc::new(std::sync::Barrier::new(2));
    let release = Arc::new(std::sync::Barrier::new(2));
    let hook = Arc::new(BlockingDeadLetterRequeueHook {
        locked: Arc::clone(&locked),
        release: Arc::clone(&release),
    });
    let execution_database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))?;
    let execution_thread = thread::spawn(move || -> Result<_, String> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        runtime.block_on(async move {
            let mut connection = sqlx::PgConnection::connect(&execution_database_url)
                .await
                .map_err(|error| error.to_string())?;
            let execution_preflight = ChangeCommandPreflight {
                iam: &AllowGovernance,
                references: &AllowGovernance,
            };
            execute_postgres_dead_letter_requeue(
                &mut connection,
                &execute_command,
                &execution_preflight,
                hook.as_ref(),
            )
            .await
            .map_err(|error| error.to_string())
        })
    });
    locked.wait();
    signing_command.failure_injection = PostEmbedSigningFailureInjection::None;
    let loader = StaticRecoveryCommandLoader {
        command: signing_command.clone(),
    };
    let worker_config = recovery_worker_config("worker-dead-letter-requeue-conflict", 3);
    let conflict_outcome = run_postgres_post_embed_recovery_batch(
        pool,
        &worker_config,
        &loader,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    if conflict_outcome.claimed != 0 {
        return Err(
            format!("worker claimed row locked by approved requeue: {conflict_outcome:?}").into(),
        );
    }
    release.wait();
    let execution_outcome = execution_thread
        .join()
        .map_err(|_| "dead-letter requeue execution thread panicked")?
        .map_err(|error| format!("dead-letter requeue execution failed: {error}"))?;
    if !execution_outcome.succeeded || execution_outcome.recovery_control_version != 2 {
        return Err(format!("approved dead-letter execution failed: {execution_outcome:?}").into());
    }

    let recovery_outcome = run_postgres_post_embed_recovery_batch(
        pool,
        &worker_config,
        &loader,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    let final_snapshot = snapshot(pool, fixture_type).await?;
    let governance = sqlx::query(
        "SELECT request.status,
                (SELECT COUNT(*) FROM ai_transparency_change_approvals approval
                 WHERE approval.change_request_id = request.change_request_id) AS approval_count,
                (SELECT COUNT(*) FROM ai_transparency_change_executions execution
                 WHERE execution.change_request_id = request.change_request_id) AS execution_count,
                (SELECT ARRAY_AGG(audit.sequence ORDER BY audit.sequence)
                 FROM ai_transparency_change_audit_events audit
                 WHERE audit.change_request_id = request.change_request_id) AS audit_sequence,
                signing.recovery_control_version,
                signing.last_requeue_change_request_id
         FROM ai_transparency_change_requests request
         JOIN ai_post_embed_signing_executions signing
           ON signing.execution_id = request.target_id
         WHERE request.change_request_id = $1",
    )
    .bind(&submit_command.change_request_id)
    .fetch_one(pool)
    .await?;
    let audit_sequence: Vec<i32> = governance.get("audit_sequence");
    if recovery_outcome.claimed != 1
        || recovery_outcome.succeeded != 1
        || signer.request_count() != 1
        || signer.billable_invocation_count() != 1
        || store.unique_stage_write_count() != 1
        || final_snapshot.worker_recovery_state.as_deref() != Some("completed")
        || final_snapshot.worker_recovery_attempts != Some(1)
        || final_snapshot.committed_ledger_count != 1
        || governance.get::<String, _>("status") != "succeeded"
        || governance.get::<i64, _>("approval_count") != 1
        || governance.get::<i64, _>("execution_count") != 1
        || audit_sequence != vec![1, 2, 3, 4, 5]
        || governance.get::<i32, _>("recovery_control_version") != 2
        || governance.get::<Option<String>, _>("last_requeue_change_request_id")
            != Some(submit_command.change_request_id.clone())
    {
        return Err(format!(
            "dead-letter requeue governance mismatch: recovery={recovery_outcome:?}, \
             snapshot={final_snapshot:?}, audit={audit_sequence:?}"
        )
        .into());
    }
    Ok(worker_scenario_result(
        fixture_type,
        &final_snapshot,
        &signer,
        &store,
        true,
        false,
        None,
    ))
}

#[cfg(feature = "postgres")]
async fn run_dead_letter_audit_failure_rollback(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "dead_letter_requeue_audit_failure_rollback";
    let (_signing_command, signer, store) =
        create_dead_letter_execution(pool, fixture_type).await?;
    let preflight = ChangeCommandPreflight {
        iam: &AllowGovernance,
        references: &AllowGovernance,
    };
    for mode in [
        DeadLetterRequeueMode::SubmitRequest,
        DeadLetterRequeueMode::ApproveRequest,
    ] {
        let command = dead_letter_requeue_command(fixture_type, mode, false);
        let mut connection = pool.acquire().await?;
        let outcome = execute_postgres_dead_letter_requeue(
            &mut connection,
            &command,
            &preflight,
            &NoopDeadLetterRequeueExecutionHook,
        )
        .await?;
        if !outcome.succeeded {
            return Err(format!("rollback setup command failed: {outcome:?}").into());
        }
    }
    let execute_command = dead_letter_requeue_command(
        fixture_type,
        DeadLetterRequeueMode::ExecuteApprovedRequest,
        true,
    );
    let mut connection = pool.acquire().await?;
    let outcome = execute_postgres_dead_letter_requeue(
        &mut connection,
        &execute_command,
        &preflight,
        &NoopDeadLetterRequeueExecutionHook,
    )
    .await?;
    let snapshot = snapshot(pool, fixture_type).await?;
    let governance = sqlx::query(
        "SELECT request.status,
                (SELECT COUNT(*) FROM ai_transparency_change_executions execution
                 WHERE execution.change_request_id = request.change_request_id) AS execution_count,
                (SELECT ARRAY_AGG(audit.sequence ORDER BY audit.sequence)
                 FROM ai_transparency_change_audit_events audit
                 WHERE audit.change_request_id = request.change_request_id) AS audit_sequence,
                signing.recovery_control_version,
                signing.last_requeue_change_request_id
         FROM ai_transparency_change_requests request
         JOIN ai_post_embed_signing_executions signing
           ON signing.execution_id = request.target_id
         WHERE request.change_request_id = $1",
    )
    .bind(&execute_command.change_request_id)
    .fetch_one(pool)
    .await?;
    let audit_sequence: Vec<i32> = governance.get("audit_sequence");
    if outcome.succeeded
        || outcome.reason_code.as_deref() != Some(DEAD_LETTER_REASON_AUDIT_WRITE_FAILED)
        || snapshot.worker_recovery_state.as_deref() != Some("dead_letter")
        || snapshot.worker_recovery_attempts != Some(3)
        || governance.get::<String, _>("status") != "approved"
        || governance.get::<i64, _>("execution_count") != 0
        || audit_sequence != vec![1, 2]
        || governance.get::<i32, _>("recovery_control_version") != 1
        || governance
            .get::<Option<String>, _>("last_requeue_change_request_id")
            .is_some()
    {
        return Err(format!(
            "dead-letter audit rollback mismatch: outcome={outcome:?}, \
             snapshot={snapshot:?}, audit={audit_sequence:?}"
        )
        .into());
    }
    Ok(worker_scenario_result(
        fixture_type,
        &snapshot,
        &signer,
        &store,
        false,
        false,
        Some(DEAD_LETTER_REASON_AUDIT_WRITE_FAILED),
    ))
}

#[cfg(feature = "postgres")]
async fn run_confirmed_delivery_envelope(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "confirmed_delivery_envelope";
    seed_production_state(pool, fixture_type).await?;
    let command = command(fixture_type, PostEmbedSigningFailureInjection::None);
    let signer = ControlledSigner::new(false, 0);
    let store = InMemoryArtifactStore::default();
    let mut signing_connection = pool.acquire().await?;
    let signing_outcome = execute_postgres_internal_post_embed_signing(
        &mut signing_connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    drop(signing_connection);
    let final_bytes = signing_outcome
        .final_signed_png_bytes
        .ok_or("confirmed signing did not return final bytes")?;
    let mut envelope_connection = pool.acquire().await?;
    let created = execute_postgres_confirmed_delivery_envelope(
        &mut envelope_connection,
        &command.execution_id,
    )
    .await?;
    drop(envelope_connection);
    let envelope = created
        .envelope
        .clone()
        .ok_or("delivery envelope was not created")?;
    let signer_receipt_json = created
        .signer_receipt_json
        .as_deref()
        .ok_or("delivery signer receipt missing")?;
    let finalize_receipt_json = created
        .artifact_finalize_receipt_json
        .as_deref()
        .ok_or("delivery finalize receipt missing")?;
    validate_ai_delivery_envelope(
        &envelope,
        &final_bytes,
        signer_receipt_json,
        finalize_receipt_json,
    )?;
    let mut replay_connection = pool.acquire().await?;
    let replay =
        execute_postgres_confirmed_delivery_envelope(&mut replay_connection, &command.execution_id)
            .await?;
    drop(replay_connection);
    let delivery_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_post_embed_delivery_envelopes WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .fetch_one(pool)
    .await?;
    let update_delivery = sqlx::query(
        "UPDATE ai_post_embed_delivery_envelopes
         SET envelope_digest = repeat('a', 64) WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .execute(pool)
    .await;
    let delete_delivery =
        sqlx::query("DELETE FROM ai_post_embed_delivery_envelopes WHERE execution_id = $1")
            .bind(&command.execution_id)
            .execute(pool)
            .await;
    let mut profile_mismatch = envelope.clone();
    profile_mismatch.profile_identity.regional_profile_id = "eu_ai_act_image_v1".to_string();
    let profile_error = validate_ai_delivery_envelope(
        &profile_mismatch,
        &final_bytes,
        signer_receipt_json,
        finalize_receipt_json,
    )
    .unwrap_err();
    let snapshot = snapshot(pool, fixture_type).await?;
    if !created.succeeded
        || created.replayed
        || !replay.succeeded
        || !replay.replayed
        || delivery_count != 1
        || update_delivery.is_ok()
        || delete_delivery.is_ok()
        || profile_error.code != "ai_delivery_envelope_profile_identity_digest_mismatch"
        || snapshot.execution_status.as_deref() != Some("confirmed")
        || snapshot.artifact_status.as_deref() != Some("finalized")
        || snapshot.worker_recovery_state.as_deref() != Some("completed")
    {
        return Err(format!(
            "confirmed delivery envelope mismatch: created={created:?}, replay={replay:?}, \
             snapshot={snapshot:?}, profile_error={profile_error}"
        )
        .into());
    }
    Ok(worker_scenario_result(
        fixture_type,
        &snapshot,
        &signer,
        &store,
        true,
        true,
        None,
    ))
}

#[cfg(feature = "postgres")]
async fn run_delivery_retrieval_gate(
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture_type = "delivery_retrieval_gate";
    seed_production_state(pool, fixture_type).await?;
    let signing_command = command(fixture_type, PostEmbedSigningFailureInjection::None);
    let signer = ControlledSigner::new(false, 0);
    let store = InMemoryArtifactStore::default();
    let mut signing_connection = pool.acquire().await?;
    let signing_outcome = execute_postgres_internal_post_embed_signing(
        &mut signing_connection,
        &signing_command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    if !signing_outcome.succeeded {
        return Err(format!("delivery retrieval signing setup failed: {signing_outcome:?}").into());
    }
    drop(signing_connection);
    let mut envelope_connection = pool.acquire().await?;
    let delivery = execute_postgres_confirmed_delivery_envelope(
        &mut envelope_connection,
        &signing_command.execution_id,
    )
    .await?;
    drop(envelope_connection);
    let envelope = delivery
        .envelope
        .clone()
        .ok_or("delivery retrieval envelope missing")?;
    let preflight = ChangeCommandPreflight {
        iam: &AllowGovernance,
        references: &AllowGovernance,
    };
    let authorization_command = CreateDeliveryAuthorizationCommand {
        delivery_envelope_id: envelope.delivery_envelope_id.clone(),
        tenant_id: format!("tenant-{fixture_type}"),
        workspace_id: format!("workspace-{fixture_type}"),
        environment: "production".to_string(),
        requester_snapshot_id: format!("actor-snapshot-{fixture_type}-delivery-operator"),
        requester_token_hash: sha256_hex(b"delivery-retrieval-operator-token"),
        ttl_seconds: 120,
    };
    let mut wrong_role_command = authorization_command.clone();
    wrong_role_command.requester_snapshot_id = format!("actor-snapshot-{fixture_type}");
    let mut wrong_role_connection = pool.acquire().await?;
    let wrong_role = execute_postgres_create_delivery_authorization(
        &mut wrong_role_connection,
        &wrong_role_command,
        &preflight,
    )
    .await?;
    drop(wrong_role_connection);
    if wrong_role.succeeded || wrong_role.grant.is_some() {
        return Err(format!(
            "delivery retrieval requester snapshot role mismatch was accepted: {wrong_role:?}"
        )
        .into());
    }
    let mut authorization_connection = pool.acquire().await?;
    let authorization = execute_postgres_create_delivery_authorization(
        &mut authorization_connection,
        &authorization_command,
        &preflight,
    )
    .await?;
    drop(authorization_connection);
    let grant = authorization
        .grant
        .clone()
        .ok_or("delivery retrieval authorization grant missing")?;
    let stored_token_hash: String = sqlx::query_scalar(
        "SELECT token_hash FROM ai_delivery_retrieval_authorizations
         WHERE authorization_id = $1",
    )
    .bind(&grant.authorization_id)
    .fetch_one(pool)
    .await?;
    if stored_token_hash != sha256_hex(grant.retrieval_token.as_bytes())
        || stored_token_hash == grant.retrieval_token
    {
        return Err("delivery retrieval token custody mismatch".into());
    }

    let retrieve_command = RetrieveDeliveryCommand {
        authorization_id: grant.authorization_id.clone(),
        retrieval_token: grant.retrieval_token.clone(),
    };
    let mut connection_a = pool.acquire().await?;
    let mut connection_b = pool.acquire().await?;
    let (outcome_a, outcome_b) = tokio::join!(
        execute_postgres_retrieve_delivery(&mut connection_a, &retrieve_command, &store),
        execute_postgres_retrieve_delivery(&mut connection_b, &retrieve_command, &store)
    );
    let outcome_a = outcome_a?;
    let outcome_b = outcome_b?;
    let successes = [&outcome_a, &outcome_b]
        .into_iter()
        .filter(|outcome| outcome.succeeded)
        .count();
    let package = outcome_a
        .package
        .as_ref()
        .or(outcome_b.package.as_ref())
        .ok_or("delivery retrieval success package missing")?;
    let package_envelope = serde_json::from_str(&package.envelope_json)?;
    validate_ai_delivery_import(
        &package_envelope,
        &package.final_media_bytes,
        &package.signer_receipt_json,
        &package.artifact_finalize_receipt_json,
        &package.retrieval_receipt,
    )?;
    if successes != 1 || store.load_request_count() != 1 {
        return Err(format!(
            "delivery retrieval concurrency mismatch: a={outcome_a:?}, b={outcome_b:?}, loads={}",
            store.load_request_count()
        )
        .into());
    }

    let mut replay_connection = pool.acquire().await?;
    let replay =
        execute_postgres_retrieve_delivery(&mut replay_connection, &retrieve_command, &store)
            .await?;
    drop(replay_connection);
    if replay.succeeded || replay.reason_code.as_deref() != Some(REASON_RETRIEVAL_INVALID) {
        return Err(format!("delivery retrieval replay was not rejected: {replay:?}").into());
    }

    let mut invalid_authorization_connection = pool.acquire().await?;
    let invalid_authorization = execute_postgres_create_delivery_authorization(
        &mut invalid_authorization_connection,
        &authorization_command,
        &preflight,
    )
    .await?;
    drop(invalid_authorization_connection);
    let invalid_grant = invalid_authorization
        .grant
        .ok_or("invalid-token authorization grant missing")?;
    let mut invalid_connection = pool.acquire().await?;
    let invalid = execute_postgres_retrieve_delivery(
        &mut invalid_connection,
        &RetrieveDeliveryCommand {
            authorization_id: invalid_grant.authorization_id.clone(),
            retrieval_token: "invalid-retrieval-token".to_string(),
        },
        &store,
    )
    .await?;
    drop(invalid_connection);
    let invalid_status: String = sqlx::query_scalar(
        "SELECT status FROM ai_delivery_retrieval_authorizations WHERE authorization_id = $1",
    )
    .bind(&invalid_grant.authorization_id)
    .fetch_one(pool)
    .await?;
    if invalid.succeeded
        || invalid.reason_code.as_deref() != Some(REASON_RETRIEVAL_INVALID)
        || invalid_status != "active"
    {
        return Err(format!(
            "delivery retrieval invalid-token fail-closed mismatch: {invalid:?}, status={invalid_status}"
        )
        .into());
    }

    let mut expired_authorization_connection = pool.acquire().await?;
    let expired_authorization = execute_postgres_create_delivery_authorization(
        &mut expired_authorization_connection,
        &authorization_command,
        &preflight,
    )
    .await?;
    drop(expired_authorization_connection);
    let expired_grant = expired_authorization
        .grant
        .ok_or("expired authorization grant missing")?;
    sqlx::query(
        "UPDATE ai_delivery_retrieval_authorizations
         SET granted_at = NOW() - INTERVAL '2 minutes',
             expires_at = NOW() - INTERVAL '1 second'
         WHERE authorization_id = $1",
    )
    .bind(&expired_grant.authorization_id)
    .execute(pool)
    .await?;
    let mut expired_connection = pool.acquire().await?;
    let expired = execute_postgres_retrieve_delivery(
        &mut expired_connection,
        &RetrieveDeliveryCommand {
            authorization_id: expired_grant.authorization_id.clone(),
            retrieval_token: expired_grant.retrieval_token,
        },
        &store,
    )
    .await?;
    drop(expired_connection);
    let expired_status: String = sqlx::query_scalar(
        "SELECT status FROM ai_delivery_retrieval_authorizations WHERE authorization_id = $1",
    )
    .bind(&expired_grant.authorization_id)
    .fetch_one(pool)
    .await?;
    if expired.succeeded
        || expired.reason_code.as_deref() != Some(REASON_RETRIEVAL_EXPIRED)
        || expired_status != "expired"
    {
        return Err(format!(
            "delivery retrieval expiry mismatch: {expired:?}, status={expired_status}"
        )
        .into());
    }

    let mut entitlement_authorization_connection = pool.acquire().await?;
    let entitlement_authorization = execute_postgres_create_delivery_authorization(
        &mut entitlement_authorization_connection,
        &authorization_command,
        &preflight,
    )
    .await?;
    drop(entitlement_authorization_connection);
    let entitlement_grant = entitlement_authorization
        .grant
        .ok_or("entitlement-revoked authorization grant missing")?;
    sqlx::query(
        "UPDATE ai_profile_entitlements
         SET status = 'suspended', updated_at = NOW()
         WHERE license_id = $1 AND profile_id = $2",
    )
    .bind(format!("license-{fixture_type}"))
    .bind(ANCHOR_PROFILE_ID)
    .execute(pool)
    .await?;
    let loads_before_entitlement = store.load_request_count();
    let mut entitlement_connection = pool.acquire().await?;
    let entitlement_rejected = execute_postgres_retrieve_delivery(
        &mut entitlement_connection,
        &RetrieveDeliveryCommand {
            authorization_id: entitlement_grant.authorization_id,
            retrieval_token: entitlement_grant.retrieval_token,
        },
        &store,
    )
    .await?;
    drop(entitlement_connection);
    if entitlement_rejected.succeeded
        || entitlement_rejected.reason_code.as_deref() != Some(REASON_RETRIEVAL_ENTITLEMENT_INVALID)
        || store.load_request_count() != loads_before_entitlement
    {
        return Err(format!(
            "delivery retrieval revoked entitlement mismatch: {entitlement_rejected:?}"
        )
        .into());
    }
    sqlx::query(
        "UPDATE ai_profile_entitlements
         SET status = 'active', updated_at = NOW()
         WHERE license_id = $1 AND profile_id = $2",
    )
    .bind(format!("license-{fixture_type}"))
    .bind(ANCHOR_PROFILE_ID)
    .execute(pool)
    .await?;

    let mut unavailable_authorization_connection = pool.acquire().await?;
    let unavailable_authorization = execute_postgres_create_delivery_authorization(
        &mut unavailable_authorization_connection,
        &authorization_command,
        &preflight,
    )
    .await?;
    drop(unavailable_authorization_connection);
    let unavailable_grant = unavailable_authorization
        .grant
        .ok_or("artifact-unavailable authorization grant missing")?;
    let original_bytes = package.final_media_bytes.clone();
    store
        .committed
        .lock()
        .expect("artifact committed lock")
        .remove(&signing_command.execution_id);
    let mut unavailable_connection = pool.acquire().await?;
    let unavailable = execute_postgres_retrieve_delivery(
        &mut unavailable_connection,
        &RetrieveDeliveryCommand {
            authorization_id: unavailable_grant.authorization_id.clone(),
            retrieval_token: unavailable_grant.retrieval_token,
        },
        &store,
    )
    .await?;
    drop(unavailable_connection);
    let unavailable_status: String = sqlx::query_scalar(
        "SELECT status FROM ai_delivery_retrieval_authorizations WHERE authorization_id = $1",
    )
    .bind(&unavailable_grant.authorization_id)
    .fetch_one(pool)
    .await?;
    if unavailable.succeeded
        || unavailable.package.is_some()
        || unavailable.reason_code.as_deref() != Some(REASON_RETRIEVAL_ARTIFACT_UNAVAILABLE)
        || unavailable_status != "consumed"
    {
        return Err(format!(
            "delivery retrieval artifact unavailable mismatch: {unavailable:?}, status={unavailable_status}"
        )
        .into());
    }
    store
        .committed
        .lock()
        .expect("artifact committed lock")
        .insert(signing_command.execution_id.clone(), original_bytes.clone());

    let mut tampered_authorization_connection = pool.acquire().await?;
    let tampered_authorization = execute_postgres_create_delivery_authorization(
        &mut tampered_authorization_connection,
        &authorization_command,
        &preflight,
    )
    .await?;
    drop(tampered_authorization_connection);
    let tampered_grant = tampered_authorization
        .grant
        .ok_or("tampered-bytes authorization grant missing")?;
    store
        .committed
        .lock()
        .expect("artifact committed lock")
        .insert(
            signing_command.execution_id.clone(),
            b"tampered-finalized-bytes".to_vec(),
        );
    let mut tampered_connection = pool.acquire().await?;
    let tampered = execute_postgres_retrieve_delivery(
        &mut tampered_connection,
        &RetrieveDeliveryCommand {
            authorization_id: tampered_grant.authorization_id.clone(),
            retrieval_token: tampered_grant.retrieval_token,
        },
        &store,
    )
    .await?;
    drop(tampered_connection);
    let tampered_status: String = sqlx::query_scalar(
        "SELECT status FROM ai_delivery_retrieval_authorizations WHERE authorization_id = $1",
    )
    .bind(&tampered_grant.authorization_id)
    .fetch_one(pool)
    .await?;
    if tampered.succeeded
        || tampered.package.is_some()
        || tampered.reason_code.as_deref() != Some(REASON_RETRIEVAL_BRIDGE_REJECTED)
        || tampered_status != "consumed"
    {
        return Err(format!(
            "delivery retrieval tampered bytes mismatch: {tampered:?}, status={tampered_status}"
        )
        .into());
    }
    store
        .committed
        .lock()
        .expect("artifact committed lock")
        .insert(signing_command.execution_id.clone(), original_bytes);

    let audit_types: Vec<String> = sqlx::query_scalar(
        "SELECT event_type FROM ai_delivery_download_audit_events
         WHERE delivery_envelope_id = $1 ORDER BY occurred_at, download_audit_event_id",
    )
    .bind(&envelope.delivery_envelope_id)
    .fetch_all(pool)
    .await?;
    if audit_types
        .iter()
        .filter(|event| *event == "authorization_granted")
        .count()
        != 6
        || audit_types
            .iter()
            .filter(|event| *event == "retrieval_claimed")
            .count()
            != 3
        || audit_types
            .iter()
            .filter(|event| *event == "retrieval_succeeded")
            .count()
            != 1
        || audit_types
            .iter()
            .filter(|event| *event == "retrieval_failed")
            .count()
            != 7
    {
        return Err(format!("delivery retrieval audit sequence mismatch: {audit_types:?}").into());
    }
    let audit_id: String = sqlx::query_scalar(
        "SELECT download_audit_event_id FROM ai_delivery_download_audit_events
         WHERE delivery_envelope_id = $1 LIMIT 1",
    )
    .bind(&envelope.delivery_envelope_id)
    .fetch_one(pool)
    .await?;
    if sqlx::query(
        "UPDATE ai_delivery_download_audit_events
         SET reason_code = 'tampered' WHERE download_audit_event_id = $1",
    )
    .bind(&audit_id)
    .execute(pool)
    .await
    .is_ok()
        || sqlx::query(
            "DELETE FROM ai_delivery_download_audit_events WHERE download_audit_event_id = $1",
        )
        .bind(&audit_id)
        .execute(pool)
        .await
        .is_ok()
    {
        return Err("delivery retrieval audit append-only trigger accepted mutation".into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_delivery_revoke_resource_budget_gate(
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let contract_fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/contracts/ai-transparency-delivery-retrieval/resource-budget-v1.fixture.json"
    ))?;
    if contract_fixture["authorization"]["maxDownloadBytes"].as_i64()
        != Some(DELIVERY_MAX_DOWNLOAD_BYTES)
        || contract_fixture["authorization"]["requiredContentType"].as_str()
            != Some(DELIVERY_REQUIRED_CONTENT_TYPE)
        || contract_fixture["authorization"]["readTimeoutMs"].as_i64()
            != Some(DELIVERY_READ_TIMEOUT_MS as i64)
        || contract_fixture["authorization"]["rateLimitPerMinute"].as_i64()
            != Some(DELIVERY_RATE_LIMIT_PER_MINUTE as i64)
    {
        return Err("delivery resource budget fixture mismatch".into());
    }
    let fixture_type = "delivery_revoke_resource_budget";
    seed_production_state(pool, fixture_type).await?;
    let signing_command = command(fixture_type, PostEmbedSigningFailureInjection::None);
    let signer = ControlledSigner::new(false, 0);
    let store = InMemoryArtifactStore::default();
    let mut signing_connection = pool.acquire().await?;
    let signing_outcome = execute_postgres_internal_post_embed_signing(
        &mut signing_connection,
        &signing_command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    if !signing_outcome.succeeded {
        return Err(format!("delivery budget signing setup failed: {signing_outcome:?}").into());
    }
    drop(signing_connection);
    let mut envelope_connection = pool.acquire().await?;
    let delivery = execute_postgres_confirmed_delivery_envelope(
        &mut envelope_connection,
        &signing_command.execution_id,
    )
    .await?;
    drop(envelope_connection);
    let envelope = delivery
        .envelope
        .ok_or("delivery budget envelope missing")?;
    let preflight = ChangeCommandPreflight {
        iam: &AllowGovernance,
        references: &AllowGovernance,
    };
    let authorization_command = CreateDeliveryAuthorizationCommand {
        delivery_envelope_id: envelope.delivery_envelope_id.clone(),
        tenant_id: format!("tenant-{fixture_type}"),
        workspace_id: format!("workspace-{fixture_type}"),
        environment: "production".to_string(),
        requester_snapshot_id: format!("actor-snapshot-{fixture_type}-delivery-operator"),
        requester_token_hash: sha256_hex(b"delivery-budget-operator-token"),
        ttl_seconds: 120,
    };
    let revocation_command = |authorization_id: String| RevokeDeliveryAuthorizationCommand {
        authorization_id,
        tenant_id: format!("tenant-{fixture_type}"),
        workspace_id: format!("workspace-{fixture_type}"),
        environment: "production".to_string(),
        revoker_snapshot_id: format!("actor-snapshot-{fixture_type}-approver"),
        revoker_token_hash: sha256_hex(b"delivery-budget-security-token"),
        reason: "internal security revocation QA".to_string(),
    };

    let revoked_grant =
        issue_delivery_authorization(pool, &authorization_command, &preflight).await?;
    if revoked_grant.max_download_bytes != DELIVERY_MAX_DOWNLOAD_BYTES
        || revoked_grant.required_content_type != DELIVERY_REQUIRED_CONTENT_TYPE
        || revoked_grant.read_timeout_ms != DELIVERY_READ_TIMEOUT_MS
        || revoked_grant.rate_limit_per_minute != DELIVERY_RATE_LIMIT_PER_MINUTE
    {
        return Err(
            format!("delivery authorization budget grant mismatch: {revoked_grant:?}").into(),
        );
    }
    let stored_budget: (i64, String, i32, i32) = sqlx::query_as(
        "SELECT max_download_bytes, required_content_type, read_timeout_ms,
                rate_limit_per_minute
         FROM ai_delivery_retrieval_authorizations WHERE authorization_id = $1",
    )
    .bind(&revoked_grant.authorization_id)
    .fetch_one(pool)
    .await?;
    if stored_budget
        != (
            DELIVERY_MAX_DOWNLOAD_BYTES,
            DELIVERY_REQUIRED_CONTENT_TYPE.to_string(),
            DELIVERY_READ_TIMEOUT_MS,
            DELIVERY_RATE_LIMIT_PER_MINUTE,
        )
    {
        return Err(format!("stored delivery budget mismatch: {stored_budget:?}").into());
    }
    let revoke_command = revocation_command(revoked_grant.authorization_id.clone());
    let mut wrong_role_revoke = revoke_command.clone();
    wrong_role_revoke.revoker_snapshot_id =
        format!("actor-snapshot-{fixture_type}-delivery-operator");
    let mut wrong_role_revoke_connection = pool.acquire().await?;
    let wrong_role_revoke_outcome = execute_postgres_revoke_delivery_authorization(
        &mut wrong_role_revoke_connection,
        &wrong_role_revoke,
        &preflight,
    )
    .await?;
    drop(wrong_role_revoke_connection);
    if wrong_role_revoke_outcome.succeeded {
        return Err(format!(
            "delivery authorization revoke accepted wrong role: {wrong_role_revoke_outcome:?}"
        )
        .into());
    }
    let mut revoke_connection = pool.acquire().await?;
    let revoked = execute_postgres_revoke_delivery_authorization(
        &mut revoke_connection,
        &revoke_command,
        &preflight,
    )
    .await?;
    drop(revoke_connection);
    let mut replay_connection = pool.acquire().await?;
    let revoke_replay = execute_postgres_revoke_delivery_authorization(
        &mut replay_connection,
        &revoke_command,
        &preflight,
    )
    .await?;
    drop(replay_connection);
    let loads_before_revoked = store.load_request_count();
    let mut revoked_retrieve_connection = pool.acquire().await?;
    let revoked_retrieve = execute_postgres_retrieve_delivery(
        &mut revoked_retrieve_connection,
        &RetrieveDeliveryCommand {
            authorization_id: revoked_grant.authorization_id.clone(),
            retrieval_token: revoked_grant.retrieval_token,
        },
        &store,
    )
    .await?;
    drop(revoked_retrieve_connection);
    if !revoked.succeeded
        || revoked.replayed
        || !revoke_replay.succeeded
        || !revoke_replay.replayed
        || revoked_retrieve.succeeded
        || revoked_retrieve.package.is_some()
        || revoked_retrieve.reason_code.as_deref() != Some(REASON_AUTHORIZATION_REVOKED)
        || store.load_request_count() != loads_before_revoked
    {
        return Err(format!(
            "delivery authorization revoke mismatch: revoked={revoked:?}, replay={revoke_replay:?}, retrieve={revoked_retrieve:?}"
        )
        .into());
    }

    let concurrent_grant =
        issue_delivery_authorization(pool, &authorization_command, &preflight).await?;
    let concurrent_revoke = revocation_command(concurrent_grant.authorization_id.clone());
    let concurrent_retrieve = RetrieveDeliveryCommand {
        authorization_id: concurrent_grant.authorization_id.clone(),
        retrieval_token: concurrent_grant.retrieval_token,
    };
    let mut concurrent_revoke_connection = pool.acquire().await?;
    let mut concurrent_retrieve_connection = pool.acquire().await?;
    let loads_before_conflict = store.load_request_count();
    let (revoke_outcome, retrieve_outcome) = tokio::join!(
        execute_postgres_revoke_delivery_authorization(
            &mut concurrent_revoke_connection,
            &concurrent_revoke,
            &preflight
        ),
        execute_postgres_retrieve_delivery(
            &mut concurrent_retrieve_connection,
            &concurrent_retrieve,
            &store
        )
    );
    let revoke_outcome = revoke_outcome?;
    let retrieve_outcome = retrieve_outcome?;
    let successful_commands =
        usize::from(revoke_outcome.succeeded) + usize::from(retrieve_outcome.succeeded);
    let concurrent_status: String = sqlx::query_scalar(
        "SELECT status FROM ai_delivery_retrieval_authorizations WHERE authorization_id = $1",
    )
    .bind(&concurrent_grant.authorization_id)
    .fetch_one(pool)
    .await?;
    if successful_commands != 1
        || !matches!(concurrent_status.as_str(), "revoked" | "consumed")
        || retrieve_outcome.package.is_some() != retrieve_outcome.succeeded
        || store.load_request_count() - loads_before_conflict
            != usize::from(retrieve_outcome.succeeded)
    {
        return Err(format!(
            "delivery revoke/retrieve concurrency mismatch: revoke={revoke_outcome:?}, retrieve={retrieve_outcome:?}, status={concurrent_status}"
        )
        .into());
    }

    let oversized_grant =
        issue_delivery_authorization(pool, &authorization_command, &preflight).await?;
    store.set_delivery_content_length(Some(DELIVERY_MAX_DOWNLOAD_BYTES + 1));
    let mut oversized_connection = pool.acquire().await?;
    let oversized = execute_postgres_retrieve_delivery(
        &mut oversized_connection,
        &RetrieveDeliveryCommand {
            authorization_id: oversized_grant.authorization_id.clone(),
            retrieval_token: oversized_grant.retrieval_token,
        },
        &store,
    )
    .await?;
    drop(oversized_connection);
    store.set_delivery_content_length(None);
    assert_consumed_failure(
        pool,
        &oversized_grant.authorization_id,
        &oversized,
        REASON_RETRIEVAL_SIZE_LIMIT_EXCEEDED,
    )
    .await?;

    let content_type_grant =
        issue_delivery_authorization(pool, &authorization_command, &preflight).await?;
    store.set_delivery_content_type(Some("application/octet-stream"));
    let mut content_type_connection = pool.acquire().await?;
    let content_type = execute_postgres_retrieve_delivery(
        &mut content_type_connection,
        &RetrieveDeliveryCommand {
            authorization_id: content_type_grant.authorization_id.clone(),
            retrieval_token: content_type_grant.retrieval_token,
        },
        &store,
    )
    .await?;
    drop(content_type_connection);
    store.set_delivery_content_type(None);
    assert_consumed_failure(
        pool,
        &content_type_grant.authorization_id,
        &content_type,
        REASON_RETRIEVAL_CONTENT_TYPE_INVALID,
    )
    .await?;

    let timeout_grant =
        issue_delivery_authorization(pool, &authorization_command, &preflight).await?;
    store.set_delivery_failure(Some(DeliveryArtifactReadFailure::TimedOut));
    let mut timeout_connection = pool.acquire().await?;
    let timeout = execute_postgres_retrieve_delivery(
        &mut timeout_connection,
        &RetrieveDeliveryCommand {
            authorization_id: timeout_grant.authorization_id.clone(),
            retrieval_token: timeout_grant.retrieval_token,
        },
        &store,
    )
    .await?;
    drop(timeout_connection);
    store.set_delivery_failure(None);
    assert_consumed_failure(
        pool,
        &timeout_grant.authorization_id,
        &timeout,
        REASON_RETRIEVAL_READ_TIMEOUT,
    )
    .await?;

    let rate_limited_grant =
        issue_delivery_authorization(pool, &authorization_command, &preflight).await?;
    sqlx::query(
        "INSERT INTO ai_delivery_download_rate_limit_windows (
            license_id, window_started_at, claim_count, updated_at
         ) VALUES ($1, date_trunc('minute', NOW()), $2, NOW())
         ON CONFLICT (license_id, window_started_at)
         DO UPDATE SET claim_count = EXCLUDED.claim_count, updated_at = NOW()",
    )
    .bind(format!("license-{fixture_type}"))
    .bind(DELIVERY_RATE_LIMIT_PER_MINUTE)
    .execute(pool)
    .await?;
    let loads_before_rate_limit = store.load_request_count();
    let mut rate_limit_connection = pool.acquire().await?;
    let rate_limited = execute_postgres_retrieve_delivery(
        &mut rate_limit_connection,
        &RetrieveDeliveryCommand {
            authorization_id: rate_limited_grant.authorization_id.clone(),
            retrieval_token: rate_limited_grant.retrieval_token,
        },
        &store,
    )
    .await?;
    drop(rate_limit_connection);
    let rate_limited_status: String = sqlx::query_scalar(
        "SELECT status FROM ai_delivery_retrieval_authorizations WHERE authorization_id = $1",
    )
    .bind(&rate_limited_grant.authorization_id)
    .fetch_one(pool)
    .await?;
    if rate_limited.succeeded
        || rate_limited.package.is_some()
        || rate_limited.reason_code.as_deref() != Some(REASON_RETRIEVAL_RATE_LIMITED)
        || rate_limited_status != "active"
        || store.load_request_count() != loads_before_rate_limit
    {
        return Err(format!(
            "delivery rate limit mismatch: outcome={rate_limited:?}, status={rate_limited_status}"
        )
        .into());
    }

    let revoke_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_download_audit_events
         WHERE authorization_id = $1 AND event_type = 'authorization_revoked'",
    )
    .bind(&revoked_grant.authorization_id)
    .fetch_one(pool)
    .await?;
    if revoke_audit_count != 1 {
        return Err(format!("delivery revoke audit count mismatch: {revoke_audit_count}").into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_delivery_security_observability_gate(
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/contracts/ai-transparency-delivery-retrieval/security-observability-v1.fixture.json"
    ))?;
    if fixture["retention"]["rateLimitWindowHours"].as_i64()
        != Some(DELIVERY_RATE_WINDOW_RETENTION_HOURS)
        || fixture["retention"]["metricSnapshotDays"].as_i64()
            != Some(DELIVERY_SECURITY_METRIC_RETENTION_DAYS)
        || fixture["windows"]["monitoringMinutes"].as_i64()
            != Some(DELIVERY_SECURITY_MONITORING_WINDOW_MINUTES)
        || fixture["windows"]["auditExportMaximumMinutes"].as_i64()
            != Some(DELIVERY_SECURITY_MAX_EXPORT_WINDOW_MINUTES)
        || fixture["windows"]["cleanupBatchLimit"].as_i64()
            != Some(DELIVERY_SECURITY_CLEANUP_BATCH_LIMIT)
        || fixture["alerts"]["revokedAccessCriticalCount"].as_i64()
            != Some(ALERT_REVOKED_ACCESS_CRITICAL_COUNT)
        || fixture["alerts"]["rateLimitedWarningCount"].as_i64()
            != Some(ALERT_RATE_LIMITED_WARNING_COUNT)
        || fixture["alerts"]["availabilityFailureWarningCount"].as_i64()
            != Some(ALERT_AVAILABILITY_WARNING_COUNT)
        || fixture["alerts"]["failureRatioWarningPercent"].as_i64()
            != Some(ALERT_FAILURE_RATIO_WARNING_PERCENT)
        || fixture["alerts"]["failureRatioMinimumAttempts"].as_i64()
            != Some(ALERT_FAILURE_RATIO_MIN_ATTEMPTS)
    {
        return Err("delivery security observability fixture mismatch".into());
    }
    let fixture_type = "delivery_revoke_resource_budget";
    let preflight = ChangeCommandPreflight {
        iam: &AllowGovernance,
        references: &AllowGovernance,
    };
    let monitoring_command = GenerateDeliverySecuritySummaryCommand {
        tenant_id: format!("tenant-{fixture_type}"),
        workspace_id: format!("workspace-{fixture_type}"),
        environment: "production".to_string(),
        requester_snapshot_id: format!("actor-snapshot-{fixture_type}-auditor"),
        requester_token_hash: sha256_hex(b"delivery-security-auditor-token"),
        mode: DeliverySecuritySummaryMode::Monitoring15m,
        window_minutes: DELIVERY_SECURITY_MONITORING_WINDOW_MINUTES,
    };
    let mut monitoring_connection = pool.acquire().await?;
    let monitoring = execute_postgres_generate_delivery_security_summary(
        &mut monitoring_connection,
        &monitoring_command,
        &preflight,
    )
    .await?;
    drop(monitoring_connection);
    let monitoring_summary = monitoring
        .summary
        .as_ref()
        .ok_or("delivery monitoring summary missing")?;
    if !monitoring.succeeded
        || monitoring_summary.alert_status != "critical"
        || !monitoring_summary
            .alert_codes
            .contains(&"delivery_integrity_failure".to_string())
        || monitoring_summary.size_limit_count < 1
        || monitoring_summary.content_type_invalid_count < 1
        || monitoring_summary.read_timeout_count < 1
        || monitoring_summary.rate_limited_count < 1
    {
        return Err(format!("delivery monitoring summary mismatch: {monitoring:?}").into());
    }
    let retention_days: f64 = sqlx::query_scalar(
        "SELECT (EXTRACT(EPOCH FROM (retention_expires_at - created_at)) / 86400.0)
                ::DOUBLE PRECISION
         FROM ai_delivery_security_observability_snapshots WHERE summary_id = $1",
    )
    .bind(&monitoring_summary.summary_id)
    .fetch_one(pool)
    .await?;
    if retention_days != DELIVERY_SECURITY_METRIC_RETENTION_DAYS as f64 {
        return Err(format!("delivery metric retention mismatch: {retention_days}").into());
    }
    if sqlx::query(
        "UPDATE ai_delivery_security_observability_snapshots
         SET alert_status = 'ok' WHERE summary_id = $1",
    )
    .bind(&monitoring_summary.summary_id)
    .execute(pool)
    .await
    .is_ok()
        || sqlx::query(
            "DELETE FROM ai_delivery_security_observability_snapshots WHERE summary_id = $1",
        )
        .bind(&monitoring_summary.summary_id)
        .execute(pool)
        .await
        .is_ok()
    {
        return Err("delivery metric retention guard accepted premature mutation".into());
    }

    let export_command = GenerateDeliverySecuritySummaryCommand {
        mode: DeliverySecuritySummaryMode::AuditExport,
        window_minutes: DELIVERY_SECURITY_MAX_EXPORT_WINDOW_MINUTES,
        ..monitoring_command.clone()
    };
    let mut export_connection = pool.acquire().await?;
    let export = execute_postgres_generate_delivery_security_summary(
        &mut export_connection,
        &export_command,
        &preflight,
    )
    .await?;
    drop(export_connection);
    let export_summary = export
        .summary
        .as_ref()
        .ok_or("audit export summary missing")?;
    if !export.succeeded
        || export_summary.alert_status != "not_evaluated"
        || !export_summary.alert_codes.is_empty()
    {
        return Err(format!("delivery aggregate audit export mismatch: {export:?}").into());
    }
    let snapshots_before_invalid: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_observability_snapshots
         WHERE tenant_id = $1 AND workspace_id = $2 AND environment = 'production'",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .fetch_one(pool)
    .await?;
    let invalid_export_command = GenerateDeliverySecuritySummaryCommand {
        window_minutes: DELIVERY_SECURITY_MAX_EXPORT_WINDOW_MINUTES + 1,
        ..export_command.clone()
    };
    let mut invalid_export_connection = pool.acquire().await?;
    let invalid_export = execute_postgres_generate_delivery_security_summary(
        &mut invalid_export_connection,
        &invalid_export_command,
        &preflight,
    )
    .await?;
    drop(invalid_export_connection);
    let mut wrong_role_command = monitoring_command.clone();
    wrong_role_command.requester_snapshot_id =
        format!("actor-snapshot-{fixture_type}-delivery-operator");
    let mut wrong_role_connection = pool.acquire().await?;
    let wrong_role = execute_postgres_generate_delivery_security_summary(
        &mut wrong_role_connection,
        &wrong_role_command,
        &preflight,
    )
    .await?;
    drop(wrong_role_connection);
    let snapshots_after_invalid: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_observability_snapshots
         WHERE tenant_id = $1 AND workspace_id = $2 AND environment = 'production'",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .fetch_one(pool)
    .await?;
    if invalid_export.succeeded
        || invalid_export.summary.is_some()
        || wrong_role.succeeded
        || wrong_role.summary.is_some()
        || snapshots_after_invalid != snapshots_before_invalid
    {
        return Err(format!(
            "delivery observability export boundary mismatch: invalid={invalid_export:?}, wrong_role={wrong_role:?}"
        )
        .into());
    }

    for hours_ago in [48_i64, 49_i64] {
        sqlx::query(
            "INSERT INTO ai_delivery_download_rate_limit_windows (
                license_id, window_started_at, claim_count, updated_at
             ) VALUES (
                $1, date_trunc('minute', NOW() - ($2 * INTERVAL '1 hour')), 1, NOW()
             )",
        )
        .bind(format!("license-{fixture_type}"))
        .bind(hours_ago)
        .execute(pool)
        .await?;
    }
    sqlx::query(
        "INSERT INTO ai_delivery_security_observability_snapshots (
            summary_id, tenant_id, workspace_id, environment, mode,
            window_started_at, window_ended_at,
            authorization_granted_count, authorization_revoked_count,
            retrieval_claimed_count, retrieval_succeeded_count, retrieval_failed_count,
            rate_limited_count, revoked_access_count, size_limit_count,
            content_type_invalid_count, read_timeout_count, artifact_unavailable_count,
            bridge_rejected_count, alert_status, alert_codes_json, summary_digest,
            requested_by_snapshot_id, created_at, retention_expires_at
         ) VALUES (
            'delivery-security-summary-expired-qa',$1,$2,'production','monitoring_15m',
            NOW() - INTERVAL '92 days', NOW() - INTERVAL '91 days',
            0,0,0,0,0,0,0,0,0,0,0,0,'ok','[]'::jsonb,repeat('a',64),$3,
            NOW() - INTERVAL '91 days',
            (NOW() - INTERVAL '91 days') + INTERVAL '90 days'
         )",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .bind(&monitoring_command.requester_snapshot_id)
    .execute(pool)
    .await?;
    let cleanup_command = CleanupDeliverySecurityWindowsCommand {
        tenant_id: monitoring_command.tenant_id.clone(),
        workspace_id: monitoring_command.workspace_id.clone(),
        environment: monitoring_command.environment.clone(),
        executor_snapshot_id: format!("actor-snapshot-{fixture_type}-executor"),
        executor_token_hash: sha256_hex(b"delivery-security-system-executor-token"),
    };
    let mut cleanup_connection_a = pool.acquire().await?;
    let mut cleanup_connection_b = pool.acquire().await?;
    let (cleanup_a, cleanup_b) = tokio::join!(
        execute_postgres_cleanup_delivery_security_windows(
            &mut cleanup_connection_a,
            &cleanup_command,
            &preflight
        ),
        execute_postgres_cleanup_delivery_security_windows(
            &mut cleanup_connection_b,
            &cleanup_command,
            &preflight
        )
    );
    let cleanup_a = cleanup_a?;
    let cleanup_b = cleanup_b?;
    if !cleanup_a.succeeded
        || !cleanup_b.succeeded
        || cleanup_a.deleted_rate_windows + cleanup_b.deleted_rate_windows != 2
        || cleanup_a.deleted_metric_snapshots + cleanup_b.deleted_metric_snapshots != 1
    {
        return Err(format!(
            "delivery security cleanup concurrency mismatch: a={cleanup_a:?}, b={cleanup_b:?}"
        )
        .into());
    }
    let expired_summary_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_observability_snapshots
         WHERE summary_id = 'delivery-security-summary-expired-qa'",
    )
    .fetch_one(pool)
    .await?;
    let old_rate_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_download_rate_limit_windows rate_window
         JOIN ai_transparency_licenses license ON license.license_id = rate_window.license_id
         WHERE license.tenant_id = $1 AND license.workspace_id = $2
           AND rate_window.window_started_at
               < date_trunc('minute', NOW() - INTERVAL '24 hours')",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .fetch_one(pool)
    .await?;
    if expired_summary_count != 0 || old_rate_count != 0 {
        return Err(format!(
            "delivery security cleanup left expired data: summaries={expired_summary_count}, rate_windows={old_rate_count}"
        )
        .into());
    }
    let operations_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_operations_audit_events
         WHERE tenant_id = $1 AND workspace_id = $2 AND environment = 'production'",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .fetch_one(pool)
    .await?;
    if operations_audit_count != 4 {
        return Err(format!(
            "delivery security operations audit count mismatch: {operations_audit_count}"
        )
        .into());
    }
    let operations_audit_id: String = sqlx::query_scalar(
        "SELECT operation_audit_event_id
         FROM ai_delivery_security_operations_audit_events
         WHERE tenant_id = $1 LIMIT 1",
    )
    .bind(&monitoring_command.tenant_id)
    .fetch_one(pool)
    .await?;
    if sqlx::query(
        "UPDATE ai_delivery_security_operations_audit_events
         SET outcome = 'failed' WHERE operation_audit_event_id = $1",
    )
    .bind(&operations_audit_id)
    .execute(pool)
    .await
    .is_ok()
        || sqlx::query(
            "DELETE FROM ai_delivery_security_operations_audit_events
             WHERE operation_audit_event_id = $1",
        )
        .bind(&operations_audit_id)
        .execute(pool)
        .await
        .is_ok()
    {
        return Err("delivery security operations audit accepted mutation".into());
    }
    run_delivery_security_incident_and_runner_gate(pool, &preflight, &monitoring_command).await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_delivery_security_incident_and_runner_gate(
    pool: &sqlx::PgPool,
    preflight: &ChangeCommandPreflight<'_>,
    monitoring_command: &GenerateDeliverySecuritySummaryCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/contracts/ai-transparency-delivery-security/incident-runner-v1.fixture.json"
    ))?;
    if fixture["incident"]["projectionSource"].as_str() != Some("monitoring_15m")
        || fixture["incident"]["ackRequiresFourEyes"].as_bool() != Some(true)
        || fixture["incident"]["resolveRequiresFourEyes"].as_bool() != Some(true)
        || fixture["cleanupRunner"]["intervalMinutes"].as_i64()
            != Some(DELIVERY_SECURITY_CLEANUP_INTERVAL_MINUTES.into())
        || fixture["notificationAdapters"]["pagerDuty"].as_str() != Some("suspended")
        || fixture["notificationAdapters"]["email"].as_str() != Some("suspended")
        || fixture["notificationAdapters"]["sms"].as_str() != Some("suspended")
    {
        return Err("delivery security incident/runner fixture mismatch".into());
    }

    let active_incident = sqlx::query(
        "SELECT incident_id, status, occurrence_count, control_version
         FROM ai_delivery_security_incidents
         WHERE tenant_id = $1 AND workspace_id = $2 AND environment = $3
           AND status <> 'resolved'",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .bind(&monitoring_command.environment)
    .fetch_one(pool)
    .await?;
    let incident_id: String = active_incident.get("incident_id");
    if active_incident.get::<String, _>("status") != "open"
        || active_incident.get::<i64, _>("occurrence_count") != 1
        || active_incident.get::<i32, _>("control_version") != 1
    {
        return Err("initial delivery security incident projection mismatch".into());
    }

    let mut summary_connection_a = pool.acquire().await?;
    let mut summary_connection_b = pool.acquire().await?;
    let (summary_a, summary_b) = tokio::join!(
        execute_postgres_generate_delivery_security_summary(
            &mut summary_connection_a,
            monitoring_command,
            preflight
        ),
        execute_postgres_generate_delivery_security_summary(
            &mut summary_connection_b,
            monitoring_command,
            preflight
        )
    );
    let summary_a = summary_a?;
    let summary_b = summary_b?;
    drop(summary_connection_a);
    drop(summary_connection_b);
    if !summary_a.succeeded || !summary_b.succeeded {
        return Err("concurrent delivery security summary projection failed".into());
    }
    let active_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_incidents
         WHERE tenant_id = $1 AND workspace_id = $2 AND environment = $3
           AND status <> 'resolved'",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .bind(&monitoring_command.environment)
    .fetch_one(pool)
    .await?;
    let occurrence_count: i64 = sqlx::query_scalar(
        "SELECT occurrence_count FROM ai_delivery_security_incidents WHERE incident_id = $1",
    )
    .bind(&incident_id)
    .fetch_one(pool)
    .await?;
    if active_count != 1 || occurrence_count != 3 {
        return Err(format!(
            "concurrent incident projection mismatch: active={active_count}, occurrences={occurrence_count}"
        )
        .into());
    }

    let fixture_type = "delivery_revoke_resource_budget";
    let mut acknowledge = incident_change_command(
        fixture_type,
        &incident_id,
        "ack",
        DeliverySecurityIncidentDesiredStatus::Acknowledged,
        1,
    );
    let invalid_change_count_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_transparency_change_requests
         WHERE target_id = $1 AND target_type = 'delivery_security_incident'",
    )
    .bind(&incident_id)
    .fetch_one(pool)
    .await?;
    let mut invalid_digest = acknowledge.clone();
    invalid_digest.change_request_id =
        format!("{}-invalid-digest", invalid_digest.change_request_id);
    invalid_digest.idempotency_key = format!("{}-invalid-digest", invalid_digest.idempotency_key);
    invalid_digest.request_digest = "0".repeat(64);
    let mut invalid_connection = pool.acquire().await?;
    let invalid = execute_postgres_delivery_security_incident_change(
        &mut invalid_connection,
        &invalid_digest,
        preflight,
    )
    .await?;
    drop(invalid_connection);
    let invalid_change_count_after: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_transparency_change_requests
         WHERE target_id = $1 AND target_type = 'delivery_security_incident'",
    )
    .bind(&incident_id)
    .fetch_one(pool)
    .await?;
    if invalid.succeeded || invalid_change_count_after != invalid_change_count_before {
        return Err(format!("incident digest mismatch was not zero-write: {invalid:?}").into());
    }

    acknowledge.mode = DeliverySecurityIncidentChangeMode::SubmitRequest;
    let mut submit_connection = pool.acquire().await?;
    let submitted = execute_postgres_delivery_security_incident_change(
        &mut submit_connection,
        &acknowledge,
        preflight,
    )
    .await?;
    drop(submit_connection);
    if !submitted.succeeded {
        return Err(format!("incident ack submit failed: {submitted:?}").into());
    }
    let mut same_actor = acknowledge.clone();
    same_actor.mode = DeliverySecurityIncidentChangeMode::ApproveRequest;
    same_actor.approver_actor_id = same_actor.requester_actor_id.clone();
    let mut same_actor_connection = pool.acquire().await?;
    let same_actor_outcome = execute_postgres_delivery_security_incident_change(
        &mut same_actor_connection,
        &same_actor,
        preflight,
    )
    .await?;
    drop(same_actor_connection);
    let approval_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_transparency_change_approvals WHERE change_request_id = $1",
    )
    .bind(&acknowledge.change_request_id)
    .fetch_one(pool)
    .await?;
    if same_actor_outcome.succeeded || approval_count != 0 {
        return Err(format!(
            "incident same-actor approval was not zero-write: {same_actor_outcome:?}"
        )
        .into());
    }
    execute_incident_change_lifecycle(pool, preflight, &mut acknowledge).await?;
    let stale = incident_change_command(
        fixture_type,
        &incident_id,
        "stale-resolve",
        DeliverySecurityIncidentDesiredStatus::Resolved,
        1,
    );
    let mut stale_connection = pool.acquire().await?;
    let stale_outcome = execute_postgres_delivery_security_incident_change(
        &mut stale_connection,
        &stale,
        preflight,
    )
    .await?;
    drop(stale_connection);
    let stale_request_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_transparency_change_requests WHERE change_request_id = $1",
    )
    .bind(&stale.change_request_id)
    .fetch_one(pool)
    .await?;
    if stale_outcome.succeeded || stale_request_count != 0 {
        return Err(format!("incident stale version was not zero-write: {stale_outcome:?}").into());
    }
    let mut resolve = incident_change_command(
        fixture_type,
        &incident_id,
        "resolve",
        DeliverySecurityIncidentDesiredStatus::Resolved,
        2,
    );
    execute_incident_change_lifecycle(pool, preflight, &mut resolve).await?;

    let mut replay_connection = pool.acquire().await?;
    resolve.mode = DeliverySecurityIncidentChangeMode::ExecuteApprovedRequest;
    let replay = execute_postgres_delivery_security_incident_change(
        &mut replay_connection,
        &resolve,
        preflight,
    )
    .await?;
    drop(replay_connection);
    if !replay.succeeded
        || replay.reason_code.as_deref() != Some("idempotency_replay")
        || replay.control_version != 3
    {
        return Err(format!("incident execution replay mismatch: {replay:?}").into());
    }

    let resolved_status: (String, i32) = sqlx::query_as(
        "SELECT status, control_version
         FROM ai_delivery_security_incidents WHERE incident_id = $1",
    )
    .bind(&incident_id)
    .fetch_one(pool)
    .await?;
    if resolved_status != ("resolved".to_string(), 3) {
        return Err(format!("incident resolve projection mismatch: {resolved_status:?}").into());
    }
    let change_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_transparency_change_audit_events
         WHERE change_request_id IN ($1,$2)",
    )
    .bind(&acknowledge.change_request_id)
    .bind(&resolve.change_request_id)
    .fetch_one(pool)
    .await?;
    if change_audit_count != 10 {
        return Err(
            format!("incident replay duplicated change audit: {change_audit_count}").into(),
        );
    }
    let incident_audit_id: String = sqlx::query_scalar(
        "SELECT incident_audit_event_id
         FROM ai_delivery_security_incident_audit_events
         WHERE incident_id = $1 LIMIT 1",
    )
    .bind(&incident_id)
    .fetch_one(pool)
    .await?;
    if sqlx::query(
        "UPDATE ai_delivery_security_incident_audit_events
         SET status = 'resolved' WHERE incident_audit_event_id = $1",
    )
    .bind(&incident_audit_id)
    .execute(pool)
    .await
    .is_ok()
    {
        return Err("delivery security incident audit accepted mutation".into());
    }

    let mut recurrence_connection = pool.acquire().await?;
    let recurrence = execute_postgres_generate_delivery_security_summary(
        &mut recurrence_connection,
        monitoring_command,
        preflight,
    )
    .await?;
    drop(recurrence_connection);
    if !recurrence.succeeded {
        return Err("resolved incident recurrence summary failed".into());
    }
    let incident_counts: (i64, i64) = sqlx::query_as(
        "SELECT
            COUNT(*) FILTER (WHERE status = 'resolved')::BIGINT,
            COUNT(*) FILTER (WHERE status <> 'resolved')::BIGINT
         FROM ai_delivery_security_incidents
         WHERE tenant_id = $1 AND workspace_id = $2 AND environment = $3",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .bind(&monitoring_command.environment)
    .fetch_one(pool)
    .await?;
    if incident_counts != (1, 1) {
        return Err(format!("resolved incident recurrence mismatch: {incident_counts:?}").into());
    }

    let ensure_command = EnsureDeliverySecurityCleanupScheduleCommand {
        tenant_id: monitoring_command.tenant_id.clone(),
        workspace_id: monitoring_command.workspace_id.clone(),
        environment: monitoring_command.environment.clone(),
        executor_snapshot_id: format!("actor-snapshot-{fixture_type}-executor"),
        executor_token_hash: sha256_hex(b"delivery-security-system-executor-token"),
        interval_minutes: DELIVERY_SECURITY_CLEANUP_INTERVAL_MINUTES,
    };
    let mut ensure_connection = pool.acquire().await?;
    let ensured = ensure_postgres_delivery_security_cleanup_schedule(
        &mut ensure_connection,
        &ensure_command,
        preflight,
    )
    .await?;
    drop(ensure_connection);
    if !ensured.succeeded || ensured.schedule_id.is_none() {
        return Err(format!("delivery cleanup schedule ensure mismatch: {ensured:?}").into());
    }
    let runner_a = RunDeliverySecurityCleanupScheduleCommand {
        tenant_id: ensure_command.tenant_id.clone(),
        workspace_id: ensure_command.workspace_id.clone(),
        environment: ensure_command.environment.clone(),
        executor_snapshot_id: ensure_command.executor_snapshot_id.clone(),
        executor_token_hash: ensure_command.executor_token_hash.clone(),
        runner_id: "delivery-security-runner-a".to_string(),
    };
    let runner_b = RunDeliverySecurityCleanupScheduleCommand {
        runner_id: "delivery-security-runner-b".to_string(),
        ..runner_a.clone()
    };
    let (run_a, run_b) = tokio::join!(
        run_postgres_due_delivery_security_cleanup(pool, &runner_a, preflight),
        run_postgres_due_delivery_security_cleanup(pool, &runner_b, preflight)
    );
    let run_a = run_a?;
    let run_b = run_b?;
    if !run_a.succeeded
        || !run_b.succeeded
        || i32::from(run_a.claimed) + i32::from(run_b.claimed) != 1
    {
        return Err(
            format!("delivery cleanup runner claim mismatch: a={run_a:?}, b={run_b:?}").into(),
        );
    }
    let schedule = sqlx::query(
        "SELECT status, run_count, consecutive_failures, last_outcome
         FROM ai_delivery_security_cleanup_schedules WHERE schedule_id = $1",
    )
    .bind(ensured.schedule_id.as_deref().unwrap_or_default())
    .fetch_one(pool)
    .await?;
    if schedule.get::<String, _>("status") != "active"
        || schedule.get::<i64, _>("run_count") != 1
        || schedule.get::<i32, _>("consecutive_failures") != 0
        || schedule.get::<Option<String>, _>("last_outcome").as_deref() != Some("succeeded")
    {
        return Err("delivery cleanup runner final projection mismatch".into());
    }
    let runner_audit_id: String = sqlx::query_scalar(
        "SELECT runner_audit_event_id
         FROM ai_delivery_security_cleanup_runner_audit_events
         WHERE schedule_id = $1 LIMIT 1",
    )
    .bind(ensured.schedule_id.as_deref().unwrap_or_default())
    .fetch_one(pool)
    .await?;
    if sqlx::query(
        "UPDATE ai_delivery_security_cleanup_runner_audit_events
         SET outcome = 'failed' WHERE runner_audit_event_id = $1",
    )
    .bind(&runner_audit_id)
    .execute(pool)
    .await
    .is_ok()
    {
        return Err("delivery cleanup runner audit accepted mutation".into());
    }
    run_delivery_security_incident_inspect_outbox_gate(
        pool,
        preflight,
        monitoring_command,
        &incident_id,
    )
    .await?;
    run_notification_delivery_gate(pool, preflight).await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_delivery_security_incident_inspect_outbox_gate(
    pool: &sqlx::PgPool,
    preflight: &ChangeCommandPreflight<'_>,
    monitoring_command: &GenerateDeliverySecuritySummaryCommand,
    resolved_incident_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/contracts/ai-transparency-delivery-security/incident-inspect-notification-outbox-v1.fixture.json"
    ))?;
    if fixture["inspection"]["requiredRole"].as_str() != Some("ai_transparency_readonly_auditor")
        || fixture["inspection"]["listMaximum"].as_i64() != Some(INCIDENT_LIST_MAX_LIMIT.into())
        || fixture["outbox"]["leaseMinutes"].as_i64() != Some(NOTIFICATION_OUTBOX_LEASE_MINUTES)
        || fixture["outbox"]["claimMaximum"].as_i64() != Some(NOTIFICATION_OUTBOX_MAX_CLAIM.into())
        || fixture["outbox"]["providerSuccessStatusExists"].as_bool() != Some(false)
        || fixture["externalAdapters"]["providerReceiptAccepted"].as_bool() != Some(false)
    {
        return Err("delivery security incident inspect/outbox fixture mismatch".into());
    }

    let fixture_type = "delivery_revoke_resource_budget";
    let inspect_command = InspectDeliverySecurityIncidentCommand {
        incident_id: resolved_incident_id.to_string(),
        tenant_id: monitoring_command.tenant_id.clone(),
        workspace_id: monitoring_command.workspace_id.clone(),
        environment: monitoring_command.environment.clone(),
        actor_snapshot_id: format!("actor-snapshot-{fixture_type}-auditor"),
        actor_token_hash: sha256_hex(b"delivery-security-auditor-token"),
    };
    let mut inspect_connection = pool.acquire().await?;
    let inspected = inspect_postgres_delivery_security_incident(
        &mut inspect_connection,
        &inspect_command,
        preflight,
    )
    .await?;
    drop(inspect_connection);
    if !inspected.succeeded {
        return Err(format!("delivery security incident inspect mismatch: {inspected:?}").into());
    }
    let incident = inspected
        .incident
        .ok_or("delivery security incident inspect returned no incident")?;
    if incident.status != "resolved"
        || incident.control_version != 3
        || incident.pending_notification_count != 3
    {
        return Err(format!(
            "delivery security incident inspect projection mismatch: {incident:?}"
        )
        .into());
    }

    let list_command = ListDeliverySecurityIncidentsCommand {
        tenant_id: monitoring_command.tenant_id.clone(),
        workspace_id: monitoring_command.workspace_id.clone(),
        environment: monitoring_command.environment.clone(),
        actor_snapshot_id: format!("actor-snapshot-{fixture_type}-auditor"),
        actor_token_hash: sha256_hex(b"delivery-security-auditor-token"),
        status: None,
        limit: INCIDENT_LIST_MAX_LIMIT,
    };
    let mut list_connection = pool.acquire().await?;
    let listed =
        list_postgres_delivery_security_incidents(&mut list_connection, &list_command, preflight)
            .await?;
    drop(list_connection);
    if !listed.succeeded || listed.incidents.len() != 2 {
        return Err(format!("delivery security incident list mismatch: {listed:?}").into());
    }
    let mut denied_list = list_command.clone();
    denied_list.actor_snapshot_id = format!("actor-snapshot-{fixture_type}-delivery-operator");
    let mut denied_connection = pool.acquire().await?;
    let denied =
        list_postgres_delivery_security_incidents(&mut denied_connection, &denied_list, preflight)
            .await?;
    drop(denied_connection);
    if denied.succeeded || !denied.incidents.is_empty() {
        return Err(
            format!("delivery security incident wrong-role list mismatch: {denied:?}").into(),
        );
    }

    let outbox_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_notification_outbox
         WHERE tenant_id = $1 AND workspace_id = $2 AND environment = $3",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .bind(&monitoring_command.environment)
    .fetch_one(pool)
    .await?;
    let distinct_dedupe_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT dedupe_key) FROM ai_delivery_security_notification_outbox
         WHERE tenant_id = $1 AND workspace_id = $2 AND environment = $3",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .bind(&monitoring_command.environment)
    .fetch_one(pool)
    .await?;
    if outbox_count != 4 || distinct_dedupe_count != outbox_count {
        return Err(format!(
            "delivery security outbox dedupe mismatch: total={outbox_count}, distinct={distinct_dedupe_count}"
        )
        .into());
    }
    let resolved_incident = listed
        .incidents
        .iter()
        .find(|incident| incident.incident_id == resolved_incident_id)
        .ok_or("resolved incident missing from list")?;
    let mut duplicate_enqueue_transaction = pool.begin().await?;
    let duplicate_enqueue = enqueue_delivery_security_notification(
        &mut duplicate_enqueue_transaction,
        &EnqueueDeliverySecurityNotificationInput {
            incident_id: resolved_incident_id,
            tenant_id: &monitoring_command.tenant_id,
            workspace_id: &monitoring_command.workspace_id,
            environment: &monitoring_command.environment,
            event_type: "incident_resolved",
            priority: "info",
            incident_status: "resolved",
            severity: &resolved_incident.severity,
            alert_codes: &resolved_incident.alert_codes,
            occurrence_count: resolved_incident.occurrence_count,
            control_version: resolved_incident.control_version,
            actor_snapshot_id: &inspect_command.actor_snapshot_id,
        },
    )
    .await?;
    duplicate_enqueue_transaction.commit().await?;
    let outbox_count_after_duplicate: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_notification_outbox
         WHERE tenant_id = $1 AND workspace_id = $2 AND environment = $3",
    )
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .bind(&monitoring_command.environment)
    .fetch_one(pool)
    .await?;
    if !duplicate_enqueue.replayed || outbox_count_after_duplicate != outbox_count {
        return Err(format!(
            "delivery security outbox duplicate enqueue mismatch: {duplicate_enqueue:?}"
        )
        .into());
    }

    let selected_notification_id: String = sqlx::query_scalar(
        "SELECT notification_id FROM ai_delivery_security_notification_outbox
         WHERE incident_id = $1 AND event_type = 'incident_opened'",
    )
    .bind(resolved_incident_id)
    .fetch_one(pool)
    .await?;
    sqlx::query(
        "UPDATE ai_delivery_security_notification_outbox
         SET available_at = CASE
             WHEN notification_id = $1 THEN NOW()
             ELSE NOW() + INTERVAL '1 day'
         END
         WHERE tenant_id = $2 AND workspace_id = $3 AND environment = $4",
    )
    .bind(&selected_notification_id)
    .bind(&monitoring_command.tenant_id)
    .bind(&monitoring_command.workspace_id)
    .bind(&monitoring_command.environment)
    .execute(pool)
    .await?;

    let claim_a = ClaimDeliverySecurityNotificationsCommand {
        tenant_id: monitoring_command.tenant_id.clone(),
        workspace_id: monitoring_command.workspace_id.clone(),
        environment: monitoring_command.environment.clone(),
        executor_snapshot_id: format!("actor-snapshot-{fixture_type}-executor"),
        executor_token_hash: sha256_hex(b"delivery-security-system-executor-token"),
        runner_id: "notification-runner-a".to_string(),
        limit: 1,
    };
    let claim_b = ClaimDeliverySecurityNotificationsCommand {
        runner_id: "notification-runner-b".to_string(),
        ..claim_a.clone()
    };
    let mut claim_connection_a = pool.acquire().await?;
    let mut claim_connection_b = pool.acquire().await?;
    let (claimed_a, claimed_b) = tokio::join!(
        claim_postgres_delivery_security_notifications(
            &mut claim_connection_a,
            &claim_a,
            preflight
        ),
        claim_postgres_delivery_security_notifications(
            &mut claim_connection_b,
            &claim_b,
            preflight
        )
    );
    let claimed_a = claimed_a?;
    let claimed_b = claimed_b?;
    drop(claim_connection_a);
    drop(claim_connection_b);
    if !claimed_a.succeeded
        || !claimed_b.succeeded
        || claimed_a.notifications.len() + claimed_b.notifications.len() != 1
    {
        return Err(format!(
            "delivery security outbox concurrent claim mismatch: a={claimed_a:?}, b={claimed_b:?}"
        )
        .into());
    }
    let (claimed, lease_owner) = if let Some(notification) = claimed_a.notifications.first() {
        (notification, claim_a.runner_id.as_str())
    } else {
        (
            claimed_b
                .notifications
                .first()
                .ok_or("missing claimed notification")?,
            claim_b.runner_id.as_str(),
        )
    };
    if claimed.notification_id != selected_notification_id
        || claimed.delivery_attempt_count != 1
        || claimed.reclaimed_expired_lease
    {
        return Err(format!("delivery security outbox claimed item mismatch: {claimed:?}").into());
    }

    let replay_command = ReplayDeliverySecurityNotificationCommand {
        notification_id: claimed.notification_id.clone(),
        tenant_id: monitoring_command.tenant_id.clone(),
        workspace_id: monitoring_command.workspace_id.clone(),
        environment: monitoring_command.environment.clone(),
        executor_snapshot_id: format!("actor-snapshot-{fixture_type}-executor"),
        executor_token_hash: sha256_hex(b"delivery-security-system-executor-token"),
        lease_owner: lease_owner.to_string(),
        idempotency_key: "notification-replay-qa".to_string(),
    };
    let mut replay_connection = pool.acquire().await?;
    let replayed = replay_postgres_delivery_security_notification(
        &mut replay_connection,
        &replay_command,
        preflight,
    )
    .await?;
    drop(replay_connection);
    if !replayed.succeeded || replayed.replayed || replayed.replay_count != 1 {
        return Err(format!("delivery security outbox replay mismatch: {replayed:?}").into());
    }
    let mut replay_idempotency_connection = pool.acquire().await?;
    let replay_idempotency = replay_postgres_delivery_security_notification(
        &mut replay_idempotency_connection,
        &replay_command,
        preflight,
    )
    .await?;
    drop(replay_idempotency_connection);
    if !replay_idempotency.succeeded
        || !replay_idempotency.replayed
        || replay_idempotency.replay_count != 1
    {
        return Err(format!(
            "delivery security outbox replay idempotency mismatch: {replay_idempotency:?}"
        )
        .into());
    }

    let reclaim_command = ClaimDeliverySecurityNotificationsCommand {
        runner_id: "notification-runner-reclaim".to_string(),
        ..claim_a.clone()
    };
    let mut second_claim_connection = pool.acquire().await?;
    let second_claim = claim_postgres_delivery_security_notifications(
        &mut second_claim_connection,
        &reclaim_command,
        preflight,
    )
    .await?;
    drop(second_claim_connection);
    if second_claim.notifications.len() != 1
        || second_claim.notifications[0].delivery_attempt_count != 2
    {
        return Err(
            format!("delivery security outbox second claim mismatch: {second_claim:?}").into(),
        );
    }
    sqlx::query(
        "UPDATE ai_delivery_security_notification_outbox
         SET lease_expires_at = NOW() - INTERVAL '1 second'
         WHERE notification_id = $1",
    )
    .bind(&selected_notification_id)
    .execute(pool)
    .await?;
    let expired_replay_command = ReplayDeliverySecurityNotificationCommand {
        notification_id: selected_notification_id.clone(),
        tenant_id: monitoring_command.tenant_id.clone(),
        workspace_id: monitoring_command.workspace_id.clone(),
        environment: monitoring_command.environment.clone(),
        executor_snapshot_id: format!("actor-snapshot-{fixture_type}-executor"),
        executor_token_hash: sha256_hex(b"delivery-security-system-executor-token"),
        lease_owner: reclaim_command.runner_id.clone(),
        idempotency_key: "notification-expired-replay-qa".to_string(),
    };
    let mut expired_replay_connection = pool.acquire().await?;
    let expired_replay = replay_postgres_delivery_security_notification(
        &mut expired_replay_connection,
        &expired_replay_command,
        preflight,
    )
    .await?;
    drop(expired_replay_connection);
    if expired_replay.succeeded || expired_replay.replay_count != 1 {
        return Err(format!(
            "delivery security outbox expired replay was not rejected: {expired_replay:?}"
        )
        .into());
    }
    let expired_reclaim_command = ClaimDeliverySecurityNotificationsCommand {
        runner_id: "notification-runner-expired-reclaim".to_string(),
        ..claim_a
    };
    let mut expired_reclaim_connection = pool.acquire().await?;
    let expired_reclaim = claim_postgres_delivery_security_notifications(
        &mut expired_reclaim_connection,
        &expired_reclaim_command,
        preflight,
    )
    .await?;
    drop(expired_reclaim_connection);
    if expired_reclaim.notifications.len() != 1
        || !expired_reclaim.notifications[0].reclaimed_expired_lease
        || expired_reclaim.notifications[0].delivery_attempt_count != 3
    {
        return Err(format!(
            "delivery security outbox expired lease reclaim mismatch: {expired_reclaim:?}"
        )
        .into());
    }

    let invalid_status_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_notification_outbox
         WHERE status NOT IN ('pending','leased','retry_scheduled')",
    )
    .fetch_one(pool)
    .await?;
    if invalid_status_count != 0 {
        return Err("delivery security outbox contains provider success status".into());
    }
    let inspection_audit_id: String = sqlx::query_scalar(
        "SELECT inspection_audit_event_id
         FROM ai_delivery_security_incident_inspection_audit_events LIMIT 1",
    )
    .fetch_one(pool)
    .await?;
    let outbox_audit_id: String = sqlx::query_scalar(
        "SELECT outbox_audit_event_id
         FROM ai_delivery_security_notification_outbox_audit_events LIMIT 1",
    )
    .fetch_one(pool)
    .await?;
    if sqlx::query(
        "UPDATE ai_delivery_security_incident_inspection_audit_events
         SET outcome = 'denied' WHERE inspection_audit_event_id = $1",
    )
    .bind(&inspection_audit_id)
    .execute(pool)
    .await
    .is_ok()
        || sqlx::query(
            "DELETE FROM ai_delivery_security_notification_outbox_audit_events
             WHERE outbox_audit_event_id = $1",
        )
        .bind(&outbox_audit_id)
        .execute(pool)
        .await
        .is_ok()
    {
        return Err("delivery security inspect/outbox audit accepted mutation".into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_notification_delivery_gate(
    pool: &sqlx::PgPool,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../docs/contracts/ai-transparency-delivery-security/notification-delivery-gate-v1.fixture.json"
    ))?;
    if fixture["destinationPolicy"]["boundBeforeAdapterInvocation"].as_bool() != Some(true)
        || fixture["providerReceipt"]["maximumLifetimeSeconds"].as_i64() != Some(900)
        || fixture["completion"]["invalidReceiptWrites"].as_i64() != Some(0)
        || fixture["externalAdapters"]["zeroSend"].as_str() != Some("sandbox_simulation_only")
    {
        return Err("notification delivery gate fixture mismatch".into());
    }

    let tenant_id = "tenant-notification-delivery-gate";
    let workspace_id = "workspace-notification-delivery-gate";
    let environment = "sandbox";
    let actor_snapshot_id = "actor-snapshot-notification-delivery-gate-executor";
    sqlx::query(
        "INSERT INTO ai_transparency_actor_role_snapshots (
            actor_role_snapshot_id, actor_id, actor_type, role,
            tenant_id, workspace_id, environment, role_binding_id,
            role_binding_version, source_identity_system, authentication_level,
            captured_at, source_expires_at, snapshot_sha256
         ) VALUES ($1,$2,'system','system_executor',$3,$4,$5,$6,1,
                   'hiddenshield_internal_iam','workload_identity',NOW(),
                   NOW() + INTERVAL '1 hour',$7)",
    )
    .bind(actor_snapshot_id)
    .bind("notification-delivery-gate-executor")
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(environment)
    .bind("notification-delivery-gate-role-binding")
    .bind(sha256_hex(b"notification-delivery-gate-snapshot"))
    .execute(pool)
    .await?;

    let incident_id = "delivery-security-incident-notification-delivery-gate";
    let incident_key = "delivery-security-notification-delivery-gate";
    let summary_digest = sha256_hex(b"notification-delivery-gate-summary");
    sqlx::query(
        "INSERT INTO ai_delivery_security_incidents (
            incident_id, tenant_id, workspace_id, environment,
            incident_key, active_incident_key, severity, status,
            alert_codes_json, occurrence_count, first_summary_id,
            first_summary_digest, latest_summary_id, latest_summary_digest,
            control_version, opened_at, updated_at
         ) VALUES ($1,$2,$3,$4,$5,$5,'warning','open',$6,1,$7,$8,$7,$8,1,NOW(),NOW())",
    )
    .bind(incident_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(environment)
    .bind(incident_key)
    .bind(json!(["delivery_rate_limited_warning"]))
    .bind("delivery-security-summary-notification-delivery-gate")
    .bind(&summary_digest)
    .execute(pool)
    .await?;

    let policy = NotificationDestinationPolicyV1 {
        schema_version: DESTINATION_POLICY_SCHEMA_VERSION.to_string(),
        policy_id: "notification-zero-send-sandbox-v1".to_string(),
        version: 1,
        environment: environment.to_string(),
        enabled: true,
        adapter_kind: "zero_send".to_string(),
        delivery_mode: "simulation".to_string(),
        destination_ref: "internal://notification-gate/zero-send".to_string(),
        event_types: vec![
            "incident_opened".to_string(),
            "incident_acknowledged".to_string(),
        ],
        minimum_priority: "info".to_string(),
        max_delivery_attempts: 3,
        retry_base_seconds: 1,
    };
    let mut production_zero_send = policy.clone();
    production_zero_send.environment = "production".to_string();
    if !validate_destination_policy(&policy, environment)
        || validate_destination_policy(&production_zero_send, "production")
    {
        return Err("notification destination policy validation mismatch".into());
    }

    let first = enqueue_notification_gate_item(
        pool,
        incident_id,
        tenant_id,
        workspace_id,
        environment,
        actor_snapshot_id,
        "incident_opened",
        1,
    )
    .await?;
    let claim_command = ClaimDeliverySecurityNotificationsCommand {
        tenant_id: tenant_id.to_string(),
        workspace_id: workspace_id.to_string(),
        environment: environment.to_string(),
        executor_snapshot_id: actor_snapshot_id.to_string(),
        executor_token_hash: sha256_hex(b"notification-delivery-gate-token"),
        runner_id: "notification-delivery-gate-runner-a".to_string(),
        limit: 1,
    };
    let mut claim_connection = pool.acquire().await?;
    let claimed = claim_postgres_delivery_security_notifications(
        &mut claim_connection,
        &claim_command,
        preflight,
    )
    .await?;
    drop(claim_connection);
    if claimed.notifications.len() != 1
        || claimed.notifications[0].notification_id != first.notification_id
    {
        return Err(format!("notification completion claim mismatch: {claimed:?}").into());
    }
    let claimed = claimed.notifications[0].clone();
    let bind_command = BindNotificationDestinationCommand {
        notification_id: claimed.notification_id.clone(),
        tenant_id: tenant_id.to_string(),
        workspace_id: workspace_id.to_string(),
        environment: environment.to_string(),
        executor_snapshot_id: actor_snapshot_id.to_string(),
        executor_token_hash: sha256_hex(b"notification-delivery-gate-token"),
        lease_owner: claim_command.runner_id.clone(),
        policy: policy.clone(),
    };
    let mut bind_connection = pool.acquire().await?;
    let bound =
        bind_postgres_notification_destination(&mut bind_connection, &bind_command, preflight)
            .await?;
    drop(bind_connection);
    if !bound.succeeded || bound.adapter_invocation_key.is_none() {
        return Err(format!("notification destination bind mismatch: {bound:?}").into());
    }
    let adapter = ZeroSendNotificationAdapter;
    let adapter_request = NotificationAdapterRequest {
        notification_id: claimed.notification_id.clone(),
        payload: claimed.payload.clone(),
        payload_digest: claimed.payload_digest.clone(),
        destination_policy: policy.clone(),
        destination_policy_digest: destination_policy_digest(&policy),
        adapter_invocation_key: bound.adapter_invocation_key.clone().unwrap_or_default(),
    };
    let receipt = adapter.deliver(&adapter_request).await?;
    let receipt_count_before: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_notification_provider_receipts
         WHERE notification_id = $1",
    )
    .bind(&claimed.notification_id)
    .fetch_one(pool)
    .await?;
    let mut mismatched_receipt = receipt.clone();
    mismatched_receipt.payload_digest = sha256_hex(b"mismatched-notification-payload");
    let invalid_complete = CompleteNotificationDeliveryCommand {
        notification_id: claimed.notification_id.clone(),
        tenant_id: tenant_id.to_string(),
        workspace_id: workspace_id.to_string(),
        environment: environment.to_string(),
        executor_snapshot_id: actor_snapshot_id.to_string(),
        executor_token_hash: sha256_hex(b"notification-delivery-gate-token"),
        lease_owner: claim_command.runner_id.clone(),
        completion_idempotency_key: "notification-completion-gate-v1".to_string(),
        receipt: mismatched_receipt,
    };
    let mut invalid_connection = pool.acquire().await?;
    let invalid = complete_postgres_notification_delivery(
        &mut invalid_connection,
        &invalid_complete,
        preflight,
    )
    .await?;
    drop(invalid_connection);
    let receipt_count_after_invalid: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_notification_provider_receipts
         WHERE notification_id = $1",
    )
    .bind(&claimed.notification_id)
    .fetch_one(pool)
    .await?;
    let status_after_invalid: String = sqlx::query_scalar(
        "SELECT status FROM ai_delivery_security_notification_outbox WHERE notification_id = $1",
    )
    .bind(&claimed.notification_id)
    .fetch_one(pool)
    .await?;
    if invalid.succeeded
        || receipt_count_before != receipt_count_after_invalid
        || status_after_invalid != "leased"
    {
        return Err(format!("invalid provider receipt was not fail-closed: {invalid:?}").into());
    }
    let complete_command = CompleteNotificationDeliveryCommand {
        receipt,
        ..invalid_complete
    };
    let mut complete_connection = pool.acquire().await?;
    let completed = complete_postgres_notification_delivery(
        &mut complete_connection,
        &complete_command,
        preflight,
    )
    .await?;
    drop(complete_connection);
    let mut replay_connection = pool.acquire().await?;
    let replayed = complete_postgres_notification_delivery(
        &mut replay_connection,
        &complete_command,
        preflight,
    )
    .await?;
    drop(replay_connection);
    let receipt_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_delivery_security_notification_provider_receipts
         WHERE notification_id = $1",
    )
    .bind(&claimed.notification_id)
    .fetch_one(pool)
    .await?;
    if !completed.succeeded
        || completed.replayed
        || !replayed.succeeded
        || !replayed.replayed
        || receipt_count != 1
    {
        return Err(format!(
            "notification completion/replay mismatch: completed={completed:?}, replayed={replayed:?}"
        )
        .into());
    }

    let second = enqueue_notification_gate_item(
        pool,
        incident_id,
        tenant_id,
        workspace_id,
        environment,
        actor_snapshot_id,
        "incident_acknowledged",
        2,
    )
    .await?;
    let dead_letter_claim = ClaimDeliverySecurityNotificationsCommand {
        runner_id: "notification-delivery-gate-runner-dead-letter".to_string(),
        ..claim_command.clone()
    };
    let mut dead_letter_claim_connection = pool.acquire().await?;
    let dead_letter_claimed = claim_postgres_delivery_security_notifications(
        &mut dead_letter_claim_connection,
        &dead_letter_claim,
        preflight,
    )
    .await?;
    drop(dead_letter_claim_connection);
    if dead_letter_claimed.notifications.len() != 1
        || dead_letter_claimed.notifications[0].notification_id != second.notification_id
    {
        return Err(
            format!("notification dead-letter claim mismatch: {dead_letter_claimed:?}").into(),
        );
    }
    let mut dead_letter_policy = policy.clone();
    dead_letter_policy.policy_id = "notification-zero-send-dead-letter-v1".to_string();
    dead_letter_policy.max_delivery_attempts = 1;
    let dead_letter_bind = BindNotificationDestinationCommand {
        notification_id: second.notification_id.clone(),
        lease_owner: dead_letter_claim.runner_id.clone(),
        policy: dead_letter_policy,
        ..bind_command.clone()
    };
    let mut dead_letter_bind_connection = pool.acquire().await?;
    let dead_letter_bound = bind_postgres_notification_destination(
        &mut dead_letter_bind_connection,
        &dead_letter_bind,
        preflight,
    )
    .await?;
    drop(dead_letter_bind_connection);
    if !dead_letter_bound.succeeded {
        return Err(format!(
            "notification dead-letter destination bind mismatch: {dead_letter_bound:?}"
        )
        .into());
    }
    let fail_command = FailNotificationDeliveryCommand {
        notification_id: second.notification_id.clone(),
        tenant_id: tenant_id.to_string(),
        workspace_id: workspace_id.to_string(),
        environment: environment.to_string(),
        executor_snapshot_id: actor_snapshot_id.to_string(),
        executor_token_hash: sha256_hex(b"notification-delivery-gate-token"),
        lease_owner: dead_letter_claim.runner_id.clone(),
        failure_idempotency_key: "notification-failure-gate-v1".to_string(),
        failure_code: "provider_unavailable".to_string(),
        retryable: true,
    };
    let mut fail_connection = pool.acquire().await?;
    let failed =
        fail_postgres_notification_delivery(&mut fail_connection, &fail_command, preflight).await?;
    drop(fail_connection);
    if !failed.succeeded || failed.status != "dead_letter" {
        return Err(format!("notification dead-letter transition mismatch: {failed:?}").into());
    }
    let recover_command = RecoverNotificationDeadLetterCommand {
        notification_id: second.notification_id.clone(),
        tenant_id: tenant_id.to_string(),
        workspace_id: workspace_id.to_string(),
        environment: environment.to_string(),
        executor_snapshot_id: actor_snapshot_id.to_string(),
        executor_token_hash: sha256_hex(b"notification-delivery-gate-token"),
        recovery_idempotency_key: "notification-recovery-gate-v1".to_string(),
    };
    let mut recover_connection = pool.acquire().await?;
    let recovered = recover_postgres_notification_dead_letter(
        &mut recover_connection,
        &recover_command,
        preflight,
    )
    .await?;
    drop(recover_connection);
    let mut recover_replay_connection = pool.acquire().await?;
    let recovery_replayed = recover_postgres_notification_dead_letter(
        &mut recover_replay_connection,
        &recover_command,
        preflight,
    )
    .await?;
    drop(recover_replay_connection);
    if !recovered.succeeded
        || recovered.status != "retry_scheduled"
        || !recovery_replayed.succeeded
        || !recovery_replayed.replayed
    {
        return Err(format!(
            "notification dead-letter recovery mismatch: recovered={recovered:?}, replay={recovery_replayed:?}"
        )
        .into());
    }
    let recovery_claim = ClaimDeliverySecurityNotificationsCommand {
        runner_id: "notification-delivery-gate-runner-recovery-a".to_string(),
        ..claim_command.clone()
    };
    let mut recovery_claim_connection = pool.acquire().await?;
    let recovery_claimed = claim_postgres_delivery_security_notifications(
        &mut recovery_claim_connection,
        &recovery_claim,
        preflight,
    )
    .await?;
    drop(recovery_claim_connection);
    if recovery_claimed.notifications.len() != 1 {
        return Err(format!("notification recovery claim mismatch: {recovery_claimed:?}").into());
    }
    sqlx::query(
        "UPDATE ai_delivery_security_notification_outbox
         SET lease_expires_at = NOW() - INTERVAL '1 second'
         WHERE notification_id = $1",
    )
    .bind(&second.notification_id)
    .execute(pool)
    .await?;
    let expired_reclaim_command = ClaimDeliverySecurityNotificationsCommand {
        runner_id: "notification-delivery-gate-runner-recovery-b".to_string(),
        ..claim_command
    };
    let mut expired_reclaim_connection = pool.acquire().await?;
    let expired_reclaimed = claim_postgres_delivery_security_notifications(
        &mut expired_reclaim_connection,
        &expired_reclaim_command,
        preflight,
    )
    .await?;
    drop(expired_reclaim_connection);
    if expired_reclaimed.notifications.len() != 1
        || !expired_reclaimed.notifications[0].reclaimed_expired_lease
        || expired_reclaimed.notifications[0].recovery_count != 2
    {
        return Err(
            format!("notification expired lease recovery mismatch: {expired_reclaimed:?}").into(),
        );
    }
    let receipt_record_id: String = sqlx::query_scalar(
        "SELECT provider_receipt_record_id
         FROM ai_delivery_security_notification_provider_receipts
         WHERE notification_id = $1",
    )
    .bind(&first.notification_id)
    .fetch_one(pool)
    .await?;
    if sqlx::query(
        "UPDATE ai_delivery_security_notification_provider_receipts
         SET provider_outcome = 'delivered' WHERE provider_receipt_record_id = $1",
    )
    .bind(&receipt_record_id)
    .execute(pool)
    .await
    .is_ok()
    {
        return Err("notification provider receipt accepted mutation".into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
#[allow(clippy::too_many_arguments)]
async fn enqueue_notification_gate_item(
    pool: &sqlx::PgPool,
    incident_id: &str,
    tenant_id: &str,
    workspace_id: &str,
    environment: &str,
    actor_snapshot_id: &str,
    event_type: &str,
    control_version: i32,
) -> Result<
    hiddenshield_feedback_backend::ai_transparency_delivery_security_notification::NotificationOutboxEnqueueOutcome,
    Box<dyn std::error::Error>,
>{
    let mut connection = pool.acquire().await?;
    let mut transaction = connection.begin().await?;
    let outcome = enqueue_delivery_security_notification(
        &mut transaction,
        &EnqueueDeliverySecurityNotificationInput {
            incident_id,
            tenant_id,
            workspace_id,
            environment,
            event_type,
            priority: "warning",
            incident_status: "open",
            severity: "warning",
            alert_codes: &["delivery_rate_limited_warning".to_string()],
            occurrence_count: 1,
            control_version,
            actor_snapshot_id,
        },
    )
    .await?;
    transaction.commit().await?;
    Ok(outcome)
}

#[cfg(feature = "postgres")]
fn incident_change_command(
    fixture_type: &str,
    incident_id: &str,
    suffix: &str,
    desired_status: DeliverySecurityIncidentDesiredStatus,
    expected_control_version: i32,
) -> DeliverySecurityIncidentChangeCommand {
    let mut command = DeliverySecurityIncidentChangeCommand {
        mode: DeliverySecurityIncidentChangeMode::SubmitRequest,
        desired_status,
        change_request_id: format!("change-{fixture_type}-incident-{suffix}"),
        approval_id: format!("approval-{fixture_type}-incident-{suffix}"),
        change_execution_id: format!("execution-{fixture_type}-incident-{suffix}"),
        incident_id: incident_id.to_string(),
        target_scope_key: format!("delivery_security_incident:{incident_id}"),
        tenant_id: format!("tenant-{fixture_type}"),
        workspace_id: format!("workspace-{fixture_type}"),
        environment: "production".to_string(),
        expected_control_version,
        desired_control_version: expected_control_version + 1,
        security_review_reference: format!("security-review-{fixture_type}-incident-{suffix}"),
        requester_snapshot_id: format!("actor-snapshot-{fixture_type}"),
        requester_actor_id: format!("actor-{fixture_type}-requester"),
        requester_token_hash: sha256_hex(b"delivery-security-requester-token"),
        approver_snapshot_id: format!("actor-snapshot-{fixture_type}-approver"),
        approver_actor_id: format!("actor-{fixture_type}-approver"),
        approver_role: "ai_transparency_security_approver".to_string(),
        approver_token_hash: sha256_hex(b"delivery-security-approver-token"),
        executor_snapshot_id: format!("actor-snapshot-{fixture_type}-executor"),
        executor_token_hash: sha256_hex(b"delivery-security-system-executor-token"),
        request_digest: String::new(),
        idempotency_key: format!("incident-{fixture_type}-{suffix}"),
    };
    command.request_digest = canonical_delivery_security_incident_change_digest(&command);
    command
}

#[cfg(feature = "postgres")]
async fn execute_incident_change_lifecycle(
    pool: &sqlx::PgPool,
    preflight: &ChangeCommandPreflight<'_>,
    command: &mut DeliverySecurityIncidentChangeCommand,
) -> Result<(), Box<dyn std::error::Error>> {
    for mode in [
        DeliverySecurityIncidentChangeMode::SubmitRequest,
        DeliverySecurityIncidentChangeMode::ApproveRequest,
        DeliverySecurityIncidentChangeMode::ExecuteApprovedRequest,
    ] {
        command.mode = mode;
        let mut connection = pool.acquire().await?;
        let outcome =
            execute_postgres_delivery_security_incident_change(&mut connection, command, preflight)
                .await?;
        if !outcome.succeeded {
            return Err(format!("incident lifecycle failed in {mode:?}: {outcome:?}").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn issue_delivery_authorization(
    pool: &sqlx::PgPool,
    command: &CreateDeliveryAuthorizationCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<DeliveryAuthorizationGrant, Box<dyn std::error::Error>> {
    let mut connection = pool.acquire().await?;
    let outcome =
        execute_postgres_create_delivery_authorization(&mut connection, command, preflight).await?;
    match outcome.grant {
        Some(grant) => Ok(grant),
        None => Err(format!("delivery authorization grant missing: {outcome:?}").into()),
    }
}

#[cfg(feature = "postgres")]
async fn assert_consumed_failure(
    pool: &sqlx::PgPool,
    authorization_id: &str,
    outcome: &hiddenshield_feedback_backend::ai_transparency_delivery_retrieval::DeliveryRetrievalOutcome,
    expected_reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let status: String = sqlx::query_scalar(
        "SELECT status FROM ai_delivery_retrieval_authorizations WHERE authorization_id = $1",
    )
    .bind(authorization_id)
    .fetch_one(pool)
    .await?;
    if outcome.succeeded
        || outcome.package.is_some()
        || outcome.reason_code.as_deref() != Some(expected_reason)
        || status != "consumed"
    {
        return Err(format!(
            "delivery consumed failure mismatch: outcome={outcome:?}, status={status}, expected={expected_reason}"
        )
        .into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_delivery_envelope_recovery_not_completed(
    pool: &sqlx::PgPool,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let fixture_type = "delivery_envelope_recovery_not_completed";
    seed_production_state(pool, fixture_type).await?;
    let command = command(fixture_type, PostEmbedSigningFailureInjection::None);
    let signer = ControlledSigner::new(false, 0);
    let store = InMemoryArtifactStore::default();
    let mut signing_connection = pool.acquire().await?;
    execute_postgres_internal_post_embed_signing(
        &mut signing_connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    drop(signing_connection);
    sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET recovery_state = 'eligible' WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .execute(pool)
    .await?;
    let mut envelope_connection = pool.acquire().await?;
    let outcome = execute_postgres_confirmed_delivery_envelope(
        &mut envelope_connection,
        &command.execution_id,
    )
    .await?;
    drop(envelope_connection);
    let envelope_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_post_embed_delivery_envelopes WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .fetch_one(pool)
    .await?;
    let snapshot = snapshot(pool, fixture_type).await?;
    if outcome.succeeded
        || outcome.reason_code.as_deref() != Some(REASON_DELIVERY_NOT_READY)
        || envelope_count != 0
        || snapshot.worker_recovery_state.as_deref() != Some("eligible")
    {
        return Err(format!(
            "delivery recovery fail-closed mismatch: outcome={outcome:?}, snapshot={snapshot:?}"
        )
        .into());
    }
    Ok(worker_scenario_result(
        fixture_type,
        &snapshot,
        &signer,
        &store,
        false,
        false,
        Some(REASON_DELIVERY_NOT_READY),
    ))
}

#[cfg(feature = "postgres")]
async fn create_dead_letter_execution(
    pool: &sqlx::PgPool,
    scenario_id: &str,
) -> Result<
    (
        InternalPostEmbedSigningCommand,
        ControlledSigner,
        InMemoryArtifactStore,
    ),
    Box<dyn std::error::Error>,
> {
    seed_production_state(pool, scenario_id).await?;
    let command = command(
        scenario_id,
        PostEmbedSigningFailureInjection::CrashAfterReservation,
    );
    let signer = ControlledSigner::new(false, 0);
    let store = InMemoryArtifactStore::default();
    let mut connection = pool.acquire().await?;
    execute_postgres_internal_post_embed_signing(
        &mut connection,
        &command,
        &AllowAuthorization,
        &signer,
        &ControlledReadback,
        &store,
    )
    .await?;
    sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET lease_expires_at = NOW() - INTERVAL '1 second',
             next_recovery_at = NOW() - INTERVAL '1 second'
         WHERE execution_id = $1",
    )
    .bind(&command.execution_id)
    .execute(pool)
    .await?;
    let worker_config = recovery_worker_config(&format!("worker-{scenario_id}-dead-letter"), 3);
    for expected_attempt in 1..=3 {
        let outcome = run_postgres_post_embed_recovery_batch(
            pool,
            &worker_config,
            &RejectingRecoveryCommandLoader,
            &AllowAuthorization,
            &signer,
            &ControlledReadback,
            &store,
        )
        .await?;
        if outcome.claimed != 1 || outcome.items[0].attempt != expected_attempt {
            return Err(format!(
                "failed to create dead-letter attempt {expected_attempt}: {outcome:?}"
            )
            .into());
        }
        if expected_attempt < 3 {
            sqlx::query(
                "UPDATE ai_post_embed_signing_executions
                 SET next_recovery_at = NOW() - INTERVAL '1 second'
                 WHERE execution_id = $1",
            )
            .bind(&command.execution_id)
            .execute(pool)
            .await?;
        }
    }
    Ok((command, signer, store))
}

#[cfg(feature = "postgres")]
fn dead_letter_requeue_command(
    scenario_id: &str,
    mode: DeadLetterRequeueMode,
    inject_audit_failure: bool,
) -> DeadLetterRequeueCommand {
    let mut command = DeadLetterRequeueCommand {
        mode,
        change_request_id: format!("dead-letter-change-{scenario_id}"),
        approval_id: format!("dead-letter-approval-{scenario_id}"),
        change_execution_id: format!("dead-letter-execution-{scenario_id}"),
        target_execution_id: format!("execution-{scenario_id}"),
        target_scope_key: format!("post_embed_recovery:execution-{scenario_id}"),
        tenant_id: format!("tenant-{scenario_id}"),
        workspace_id: format!("workspace-{scenario_id}"),
        environment: "production".to_string(),
        expected_control_version: 1,
        desired_control_version: 2,
        security_review_reference: format!("security-review-{scenario_id}"),
        requester_snapshot_id: format!("actor-snapshot-{scenario_id}"),
        requester_actor_id: format!("actor-{scenario_id}-requester"),
        requester_token_hash: "requester-token-hash".to_string(),
        approver_snapshot_id: format!("actor-snapshot-{scenario_id}-approver"),
        approver_actor_id: format!("actor-{scenario_id}-approver"),
        approver_role: "ai_transparency_security_approver".to_string(),
        approver_token_hash: "approver-token-hash".to_string(),
        executor_snapshot_id: format!("actor-snapshot-{scenario_id}-executor"),
        executor_token_hash: "executor-token-hash".to_string(),
        request_digest: String::new(),
        idempotency_key: format!("dead-letter-idempotency-{scenario_id}"),
        inject_audit_failure,
    };
    command.request_digest = canonical_dead_letter_requeue_digest(&command);
    command
}

#[cfg(feature = "postgres")]
fn worker_scenario_result(
    fixture_type: &str,
    snapshot: &DatabaseSnapshot,
    signer: &ControlledSigner,
    store: &InMemoryArtifactStore,
    succeeded: bool,
    replayed: bool,
    reason_code: Option<&str>,
) -> ScenarioResult {
    ScenarioResult {
        fixture_type: fixture_type.to_string(),
        succeeded,
        replayed,
        reason_code: reason_code.map(str::to_string),
        signer_invocations: signer.request_count(),
        signer_billable_invocations: signer.billable_invocation_count(),
        artifact_stage_requests: store.stage_request_count(),
        unique_artifact_stage_writes: store.unique_stage_write_count(),
        execution_status: snapshot.execution_status.clone(),
        artifact_status: snapshot.artifact_status.clone(),
        recovery_attempts: snapshot.recovery_attempts,
        worker_recovery_state: snapshot.worker_recovery_state.clone(),
        worker_recovery_attempts: snapshot.worker_recovery_attempts,
        recovery_audit_count: snapshot.recovery_audit_count,
        signing_execution_count: snapshot.signing_execution_count,
        signing_audit_count: snapshot.signing_audit_count,
        confirm_audit_count: snapshot.confirm_audit_count,
        manifest_count: snapshot.manifest_count,
        committed_ledger_count: snapshot.committed_ledger_count,
        committed_ledger_quantity: snapshot.committed_ledger_quantity,
        artifact_returned: succeeded,
        committed_artifact_count: store.committed_count(),
        quarantined: false,
    }
}

#[cfg(feature = "postgres")]
fn assert_scenario(
    fixture_type: &str,
    failure_injection: PostEmbedSigningFailureInjection,
    signer_rejects: bool,
    outcome: &hiddenshield_feedback_backend::ai_transparency_post_embed_signing::InternalPostEmbedSigningOutcome,
    snapshot: &DatabaseSnapshot,
    store: &InMemoryArtifactStore,
    signer_invocations: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if outcome.succeeded {
        if signer_invocations != 1
            || snapshot.execution_status.as_deref() != Some("confirmed")
            || snapshot.signing_execution_count != 1
            || snapshot.signing_audit_count != 2
            || snapshot.confirm_audit_count != 1
            || snapshot.manifest_count != 1
            || snapshot.committed_ledger_count != 1
            || outcome.final_signed_png_bytes.is_none()
            || store.committed_count() != 1
            || store.is_quarantined(&format!("execution-{fixture_type}"))
        {
            return Err(format!("success assertions failed: {snapshot:?}").into());
        }
        return Ok(());
    }
    if signer_rejects {
        if signer_invocations != 1
            || snapshot.signing_execution_count != 0
            || snapshot.signing_audit_count != 0
            || snapshot.confirm_audit_count != 0
            || snapshot.manifest_count != 0
            || snapshot.committed_ledger_count != 0
            || outcome.final_signed_png_bytes.is_some()
        {
            return Err(format!("signer rejection leaked writes: {snapshot:?}").into());
        }
        return Ok(());
    }
    if failure_injection == PostEmbedSigningFailureInjection::ConfirmRollback {
        if signer_invocations != 1
            || snapshot.execution_status.as_deref() != Some("orphaned")
            || snapshot.signing_execution_count != 1
            || snapshot.signing_audit_count != 1
            || snapshot.confirm_audit_count != 0
            || snapshot.manifest_count != 0
            || snapshot.committed_ledger_count != 0
            || outcome.final_signed_png_bytes.is_some()
            || !outcome.orphan_signing_event_created
            || !store.is_quarantined(&format!("execution-{fixture_type}"))
        {
            return Err(format!("confirm rollback assertions failed: {snapshot:?}").into());
        }
        return Ok(());
    }
    if signer_invocations != 1
        || snapshot.signing_execution_count != 0
        || snapshot.signing_audit_count != 0
        || snapshot.confirm_audit_count != 0
        || snapshot.manifest_count != 0
        || snapshot.committed_ledger_count != 0
        || outcome.final_signed_png_bytes.is_some()
        || !store.is_quarantined(&format!("execution-{fixture_type}"))
    {
        return Err(format!("pre-confirm failure leaked writes: {snapshot:?}").into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn seed_production_state(pool: &sqlx::PgPool, scenario_id: &str) -> Result<(), sqlx::Error> {
    let license_id = format!("license-{scenario_id}");
    let credential_id = format!("credential-{scenario_id}");
    let session_id = format!("session-{scenario_id}");
    sqlx::query(
        "INSERT INTO ai_transparency_licenses (
            license_id, tenant_id, workspace_id, environment, status, issuer_mode,
            deployment_mode, public_verification_required, metering_plan_id,
            effective_at, expires_at, created_at, updated_at
         ) VALUES ($1,$2,$3,'production','active','hiddenshield_managed','hosted',
            TRUE,'post-embed-qa',NOW() - INTERVAL '1 minute',
            NOW() + INTERVAL '1 day',NOW(),NOW())",
    )
    .bind(&license_id)
    .bind(format!("tenant-{scenario_id}"))
    .bind(format!("workspace-{scenario_id}"))
    .execute(pool)
    .await?;
    for (snapshot_suffix, actor_suffix, actor_type, role) in [
        (
            "requester",
            "requester",
            "human",
            "ai_transparency_requester",
        ),
        (
            "approver",
            "approver",
            "human",
            "ai_transparency_security_approver",
        ),
        ("executor", "executor", "system", "system_executor"),
        (
            "auditor",
            "auditor",
            "human",
            "ai_transparency_readonly_auditor",
        ),
        (
            "delivery-operator",
            "delivery-operator",
            "human",
            "ai_transparency_delivery_operator",
        ),
    ] {
        let snapshot_id = if snapshot_suffix == "requester" {
            format!("actor-snapshot-{scenario_id}")
        } else {
            format!("actor-snapshot-{scenario_id}-{snapshot_suffix}")
        };
        sqlx::query(
            "INSERT INTO ai_transparency_actor_role_snapshots (
                actor_role_snapshot_id, actor_id, actor_type, role, tenant_id, workspace_id,
                environment, role_binding_id, role_binding_version, source_identity_system,
                authentication_level, captured_at, source_expires_at, snapshot_sha256
             ) VALUES ($1,$2,$3,$4,$5,$6,'production',$7,1,
                'hiddenshield_internal_iam','mfa',NOW(),NOW() + INTERVAL '1 day',$8)",
        )
        .bind(&snapshot_id)
        .bind(format!("actor-{scenario_id}-{actor_suffix}"))
        .bind(actor_type)
        .bind(role)
        .bind(format!("tenant-{scenario_id}"))
        .bind(format!("workspace-{scenario_id}"))
        .bind(format!("role-binding-{scenario_id}-{snapshot_suffix}"))
        .bind(sha256_hex(snapshot_id.as_bytes()))
        .execute(pool)
        .await?;
    }
    for (profile_id, profile_kind) in [
        (ANCHOR_PROFILE_ID, "technical"),
        (POST_EMBED_PROFILE_ID, "technical"),
        (REGIONAL_PROFILE_ID, "regulatory"),
    ] {
        let profile_key = profile_id.replace('_', "-");
        let change_request_id = format!("change-{scenario_id}-{profile_key}");
        let version_id = format!("profile-version-{scenario_id}-{profile_key}");
        sqlx::query(
            "INSERT INTO ai_transparency_change_requests (
                change_request_id, operation, target_type, target_id, target_scope_key,
                tenant_id, workspace_id, environment, desired_next_version, desired_state_json,
                request_reason, legal_review_reference, security_review_reference,
                requester_snapshot_id, request_digest_version, request_digest, idempotency_key,
                status, expires_at, evidence_quality, production_eligibility, created_at, updated_at
             ) VALUES ($1,'grant_profile_entitlement','profile_entitlement',$2,$3,$4,$5,
                'production',1,$6,'post-embed signing QA entitlement','legal-gate-approved',
                'security-gate-approved',$7,'hs-ai-change-request-digest-v1',$8,$9,'succeeded',
                NOW() + INTERVAL '1 day','native_four_eyes',TRUE,NOW(),NOW())",
        )
        .bind(&change_request_id)
        .bind(profile_id)
        .bind(format!("license-{scenario_id}:{profile_id}"))
        .bind(format!("tenant-{scenario_id}"))
        .bind(format!("workspace-{scenario_id}"))
        .bind(json!({"profileId": profile_id, "version": 1, "status": "active"}))
        .bind(format!("actor-snapshot-{scenario_id}"))
        .bind(sha256_hex(change_request_id.as_bytes()))
        .bind(format!("profile-request-{scenario_id}-{profile_key}"))
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO ai_profile_entitlement_versions (
                profile_entitlement_version_id, license_id, profile_id, version, profile_kind,
                status, effective_at, expires_at, terms_version, legal_review_reference,
                security_review_reference, source_change_request_id, created_at
             ) VALUES ($1,$2,$3,1,$4,'active',NOW() - INTERVAL '1 minute',
                NOW() + INTERVAL '1 day','terms-v1',$5,$6,$7,NOW())",
        )
        .bind(&version_id)
        .bind(&license_id)
        .bind(profile_id)
        .bind(profile_kind)
        .bind((profile_kind == "regulatory").then_some("legal-gate-approved"))
        .bind((profile_kind == "technical").then_some("security-gate-approved"))
        .bind(&change_request_id)
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO ai_profile_entitlements (
                license_id, profile_id, profile_kind, status, effective_at, expires_at,
                terms_version, approved_by, created_at, updated_at, current_version_id,
                current_version, projection_updated_at
             ) VALUES ($1,$2,$3,'active',NOW() - INTERVAL '1 minute',
                NOW() + INTERVAL '1 day','terms-v1','post-embed-qa',NOW(),NOW(),$4,1,NOW())",
        )
        .bind(&license_id)
        .bind(profile_id)
        .bind(profile_kind)
        .bind(&version_id)
        .execute(pool)
        .await?;
    }
    sqlx::query(
        "INSERT INTO ai_sdk_credential_bindings (
            credential_id, license_id, api_key_id, scopes_json, status, expires_at, created_at,
            key_prefix, key_hash, hash_secret_version, environment, issuer_modes_json,
            custody_key_id, issued_at
         ) VALUES ($1,$2,$3,$4,'active',NOW() + INTERVAL '1 day',NOW(),
            $5,$6,'qa-pepper-v1','production',$7,'qa-kms-key',NOW())",
    )
    .bind(&credential_id)
    .bind(&license_id)
    .bind(format!("api-key-{scenario_id}"))
    .bind(json!([
        "ai_transparency:mark",
        "ai_transparency:post_embed_sign"
    ]))
    .bind(format!("hsqa-{scenario_id}"))
    .bind(sha256_hex(format!("key-{scenario_id}").as_bytes()))
    .bind(json!(["production_platform"]))
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO ai_marking_sessions (
            marking_session_id, license_id, tenant_id, workspace_id, environment,
            idempotency_key, requested_profile_ids_json, claim_type, provider_content_id,
            status, expires_at, created_at, updated_at
         ) VALUES ($1,$2,$3,$4,'production',$5,$6,'ai_generated',$7,
            'ready_to_confirm',NOW() + INTERVAL '30 minutes',NOW(),NOW())",
    )
    .bind(&session_id)
    .bind(&license_id)
    .bind(format!("tenant-{scenario_id}"))
    .bind(format!("workspace-{scenario_id}"))
    .bind(format!("session-idempotency-{scenario_id}"))
    .bind(json!([
        ANCHOR_PROFILE_ID,
        POST_EMBED_PROFILE_ID,
        REGIONAL_PROFILE_ID
    ]))
    .bind(format!("provider-content-{scenario_id}"))
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
fn command(
    scenario_id: &str,
    failure_injection: PostEmbedSigningFailureInjection,
) -> InternalPostEmbedSigningCommand {
    let now = Utc::now();
    let unsigned_v3_png_bytes = format!("png-v3-watermarked-{scenario_id}").into_bytes();
    let watermark_uid = format!("HS-POST-EMBED-{}", scenario_id.to_ascii_uppercase());
    InternalPostEmbedSigningCommand {
        execution_id: format!("execution-{scenario_id}"),
        idempotency_key: format!("post-embed-idempotency-{scenario_id}"),
        request_digest: sha256_hex(format!("request-{scenario_id}").as_bytes()),
        credential_id: format!("credential-{scenario_id}"),
        signer_credential_ref_digest: SIGNER_KEY_REF_DIGEST.to_string(),
        unsigned_v3_png_bytes: unsigned_v3_png_bytes.clone(),
        profile: PostEmbedSigningProfile {
            profile_entitlement_version: 1,
            entitlement_digest: ENTITLEMENT_DIGEST.to_string(),
            status: "active".to_string(),
            technical_profile_ids: vec![
                ANCHOR_PROFILE_ID.to_string(),
                POST_EMBED_PROFILE_ID.to_string(),
            ],
            regional_profile_id: REGIONAL_PROFILE_ID.to_string(),
            media_type: "image/png".to_string(),
            claim_type: "ai_generated".to_string(),
            issuer_mode: "production_platform".to_string(),
            signing_order: "watermark_then_c2pa".to_string(),
            allowed_signature_algorithms: vec!["es256".to_string()],
            allow_ephemeral_signer: false,
            valid_from: now - Duration::minutes(1),
            valid_until: now + Duration::days(1),
        },
        authorization_receipt: PostEmbedAuthorizationReceipt {
            receipt_id: format!("authorization-receipt-{scenario_id}"),
            provider_id: AUTH_PROVIDER_ID.to_string(),
            operation: "ai_transparency_post_embed_c2pa_sign".to_string(),
            role: "ai_transparency_production_signer".to_string(),
            license_id: format!("license-{scenario_id}"),
            credential_id: format!("credential-{scenario_id}"),
            marking_session_id: format!("session-{scenario_id}"),
            execution_id: format!("execution-{scenario_id}"),
            profile_entitlement_digest: ENTITLEMENT_DIGEST.to_string(),
            unsigned_v3_png_sha256: sha256_hex(&unsigned_v3_png_bytes),
            signer_credential_ref_digest: SIGNER_KEY_REF_DIGEST.to_string(),
            scope_digest: SCOPE_DIGEST.to_string(),
            issued_at: now - Duration::seconds(5),
            expires_at: now + Duration::minutes(5),
        },
        confirm_command: confirm_command(scenario_id, &watermark_uid, now),
        failure_injection,
    }
}

#[cfg(feature = "postgres")]
fn confirm_command(
    scenario_id: &str,
    watermark_uid: &str,
    now: chrono::DateTime<Utc>,
) -> ConfirmMarkingCommand {
    ConfirmMarkingCommand {
        marking_session_id: format!("session-{scenario_id}"),
        transparency_manifest_id: format!("manifest-{scenario_id}"),
        ledger_entry_id: format!("ledger-{scenario_id}"),
        audit_event_id: format!("confirm-audit-{scenario_id}"),
        watermark_uid: watermark_uid.to_string(),
        subject_digest: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            .to_string(),
        generation_mode: "text_to_image".to_string(),
        provider_id: "internal-platform-qa".to_string(),
        system_name: "HiddenShield Post Embed QA".to_string(),
        system_version: "2026.07.27".to_string(),
        model_id: Some("qa-model".to_string()),
        model_version: Some("1".to_string()),
        generated_at: now,
        operations: json!([]),
        parent_subjects: json!([]),
        profile_statuses: json!([
            {"profileId": ANCHOR_PROFILE_ID, "status": "verified"},
            {"profileId": POST_EMBED_PROFILE_ID, "status": "verified"},
            {"profileId": REGIONAL_PROFILE_ID, "status": "verified"}
        ]),
        evidence: ConfirmEvidence {
            evidence_id: format!("evidence-{scenario_id}"),
            evidence_level: "platform_signed".to_string(),
            evidence_source: "post_embed_signer_receipt".to_string(),
            issuer_id: Some(AUTH_PROVIDER_ID.to_string()),
            key_id: Some("kms-production-c2pa-key-qa".to_string()),
            proof_type: "c2pa_manifest_and_v3_readback".to_string(),
            signature_algorithm: Some("es256".to_string()),
            signature: Some(format!("controlled-signature-{scenario_id}")),
        },
        markers: vec![
            ConfirmMarker {
                marker_binding_id: format!("marker-v3-{scenario_id}"),
                marker_type: "blind_watermark".to_string(),
                marker_profile_id: ANCHOR_PROFILE_ID.to_string(),
                marker_version: "3".to_string(),
                embed_status: "verified".to_string(),
                verify_status: "verified".to_string(),
                binding_digest: Some(
                    "3333333333333333333333333333333333333333333333333333333333333333".to_string(),
                ),
            },
            ConfirmMarker {
                marker_binding_id: format!("marker-c2pa-{scenario_id}"),
                marker_type: "c2pa".to_string(),
                marker_profile_id: POST_EMBED_PROFILE_ID.to_string(),
                marker_version: "1".to_string(),
                embed_status: "verified".to_string(),
                verify_status: "verified".to_string(),
                binding_digest: Some(
                    "4444444444444444444444444444444444444444444444444444444444444444".to_string(),
                ),
            },
        ],
        explicit_label_receipts: vec![ConfirmExplicitLabelReceipt {
            receipt_id: format!("label-receipt-{scenario_id}"),
            profile_id: REGIONAL_PROFILE_ID.to_string(),
            required_surface: "platform_ui".to_string(),
            render_mode: "visible_label".to_string(),
            rendered_asset_digest: None,
            placement: json!({"surface": "generation_result"}),
            locale: "zh-CN".to_string(),
            label_text: "AI生成".to_string(),
            applied_at: now,
            applied_by: "internal-platform-qa".to_string(),
            verification_status: "verified".to_string(),
        }],
        write_after_read_verified: false,
        failure_injection: ConfirmFailureInjection::None,
    }
}

#[cfg(feature = "postgres")]
async fn snapshot(pool: &sqlx::PgPool, scenario_id: &str) -> Result<DatabaseSnapshot, sqlx::Error> {
    let execution_id = format!("execution-{scenario_id}");
    let session_id = format!("session-{scenario_id}");
    let counts = sqlx::query(
        "SELECT
            (SELECT status FROM ai_post_embed_signing_executions WHERE execution_id = $1) execution_status,
            (SELECT artifact_status FROM ai_post_embed_signing_executions WHERE execution_id = $1) artifact_status,
            (SELECT recovery_attempts FROM ai_post_embed_signing_executions WHERE execution_id = $1) recovery_attempts,
            (SELECT recovery_state FROM ai_post_embed_signing_executions WHERE execution_id = $1) worker_recovery_state,
            (SELECT worker_recovery_attempts FROM ai_post_embed_signing_executions WHERE execution_id = $1) worker_recovery_attempts,
            (SELECT COUNT(*) FROM ai_post_embed_recovery_audit_events WHERE execution_id = $1) recovery_audits,
            (SELECT adapter_receipt_contract_version FROM ai_post_embed_signing_executions WHERE execution_id = $1) adapter_receipt_contract_version,
            (SELECT signer_billable_invocation_id FROM ai_post_embed_signing_executions WHERE execution_id = $1) signer_billable_invocation_id,
            (SELECT artifact_stage_receipt_id FROM ai_post_embed_signing_executions WHERE execution_id = $1) artifact_stage_receipt_id,
            (SELECT artifact_finalize_receipt_id FROM ai_post_embed_signing_executions WHERE execution_id = $1) artifact_finalize_receipt_id,
            (SELECT
                signer_receipt_json ->> 'schemaVersion' = 'hs-ai-production-c2pa-signer-receipt-v1'
                AND signer_receipt_json ?& ARRAY[
                    'signerInvocationKey', 'signerResultRef', 'idempotencyDisposition',
                    'billableInvocationId', 'signedAt', 'receiptExpiresAt', 'providerSignature'
                ]
                AND NOT signer_receipt_json ? 'certificateChainTrusted'
             FROM ai_post_embed_signing_executions WHERE execution_id = $1) signer_receipt_contract_complete,
            (SELECT
                artifact_stage_receipt_json ->> 'schemaVersion' = 'hs-ai-production-post-embed-artifact-receipt-v1'
                AND artifact_stage_receipt_json ->> 'operation' = 'stage'
                AND artifact_stage_receipt_json ->> 'durabilityStatus' = 'staged'
                AND artifact_stage_receipt_json ?& ARRAY[
                    'signerInvocationKey', 'artifactRef', 'finalSignedPngSha256',
                    'objectVersion', 'idempotencyKey', 'providerSignature'
                ]
             FROM ai_post_embed_signing_executions WHERE execution_id = $1) artifact_stage_receipt_contract_complete,
            (SELECT
                artifact_finalize_receipt_json ->> 'schemaVersion' = 'hs-ai-production-post-embed-artifact-receipt-v1'
                AND artifact_finalize_receipt_json ->> 'operation' = 'finalize'
                AND artifact_finalize_receipt_json ->> 'durabilityStatus' = 'finalized'
                AND artifact_finalize_receipt_json ?& ARRAY[
                    'signerInvocationKey', 'artifactRef', 'finalSignedPngSha256',
                    'objectVersion', 'idempotencyKey', 'providerSignature'
                ]
             FROM ai_post_embed_signing_executions WHERE execution_id = $1) artifact_finalize_receipt_contract_complete,
            (SELECT COUNT(*) FROM ai_post_embed_signing_executions WHERE execution_id = $1) signing_executions,
            (SELECT COUNT(*) FROM ai_post_embed_signing_audit_events WHERE execution_id = $1) signing_audits,
            (SELECT COUNT(*) FROM ai_marking_confirm_audit_events WHERE marking_session_id = $2) confirm_audits,
            (SELECT COUNT(*) FROM ai_transparency_manifests WHERE marking_session_id = $2) manifests,
            (SELECT COUNT(*) FROM ai_marking_ledger
                WHERE marking_session_id = $2 AND ledger_status = 'committed') committed_ledger,
            (SELECT COALESCE(SUM(quantity), 0) FROM ai_marking_ledger
                WHERE marking_session_id = $2 AND ledger_status = 'committed') committed_quantity",
    )
    .bind(&execution_id)
    .bind(&session_id)
    .fetch_one(pool)
    .await?;
    Ok(DatabaseSnapshot {
        execution_status: counts.get("execution_status"),
        artifact_status: counts.get("artifact_status"),
        recovery_attempts: counts.get("recovery_attempts"),
        worker_recovery_state: counts.get("worker_recovery_state"),
        worker_recovery_attempts: counts.get("worker_recovery_attempts"),
        recovery_audit_count: counts.get("recovery_audits"),
        adapter_receipt_contract_version: counts.get("adapter_receipt_contract_version"),
        signer_billable_invocation_id: counts.get("signer_billable_invocation_id"),
        artifact_stage_receipt_id: counts.get("artifact_stage_receipt_id"),
        artifact_finalize_receipt_id: counts.get("artifact_finalize_receipt_id"),
        signer_receipt_contract_complete: counts.get("signer_receipt_contract_complete"),
        artifact_stage_receipt_contract_complete: counts
            .get("artifact_stage_receipt_contract_complete"),
        artifact_finalize_receipt_contract_complete: counts
            .get("artifact_finalize_receipt_contract_complete"),
        signing_execution_count: counts.get("signing_executions"),
        signing_audit_count: counts.get("signing_audits"),
        confirm_audit_count: counts.get("confirm_audits"),
        manifest_count: counts.get("manifests"),
        committed_ledger_count: counts.get("committed_ledger"),
        committed_ledger_quantity: counts.get("committed_quantity"),
    })
}

#[cfg(feature = "postgres")]
async fn reset_schema(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(pool)
        .await?;
    for migration in [
        POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL,
        POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL,
        POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
        POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_UP_SQL,
        POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_UP_SQL,
        POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_UP_SQL,
        POSTGRES_P8_AI_TRANSPARENCY_POST_EMBED_SIGNING_UP_SQL,
        POSTGRES_P9_AI_TRANSPARENCY_SIGNING_RESERVATION_UP_SQL,
        POSTGRES_P10_AI_TRANSPARENCY_ADAPTER_RECEIPTS_UP_SQL,
        POSTGRES_P11_AI_TRANSPARENCY_RECOVERY_WORKER_UP_SQL,
        POSTGRES_P12_AI_TRANSPARENCY_DEAD_LETTER_REQUEUE_UP_SQL,
        POSTGRES_P13_AI_TRANSPARENCY_CONFIRMED_DELIVERY_ENVELOPE_UP_SQL,
        POSTGRES_P14_AI_TRANSPARENCY_DELIVERY_RETRIEVAL_UP_SQL,
        POSTGRES_P15_AI_TRANSPARENCY_DELIVERY_REVOKE_RESOURCE_BUDGET_UP_SQL,
        POSTGRES_P16_AI_TRANSPARENCY_DELIVERY_SECURITY_OBSERVABILITY_UP_SQL,
        POSTGRES_P17_AI_TRANSPARENCY_DELIVERY_SECURITY_INCIDENT_RUNNER_UP_SQL,
        POSTGRES_P18_AI_TRANSPARENCY_DELIVERY_SECURITY_NOTIFICATION_OUTBOX_UP_SQL,
        POSTGRES_P19_AI_TRANSPARENCY_NOTIFICATION_DELIVERY_GATE_UP_SQL,
    ] {
        sqlx::raw_sql(migration).execute(pool).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn assert_fixture_contracts() -> Result<(), Box<dyn std::error::Error>> {
    for (file_name, expected_type) in [
        ("success-v1.fixture.json", "success"),
        ("signer-rejected-v1.fixture.json", "signer_rejected"),
        (
            "receipt-hash-mismatch-v1.fixture.json",
            "receipt_hash_mismatch",
        ),
        (
            "c2pa-readback-failure-v1.fixture.json",
            "c2pa_readback_failure",
        ),
        ("v3-readback-failure-v1.fixture.json", "v3_readback_failure"),
        ("confirm-rollback-v1.fixture.json", "confirm_rollback"),
        ("duplicate-replay-v1.fixture.json", "duplicate_replay"),
        (
            "concurrent-reservation-v1.fixture.json",
            "concurrent_reservation",
        ),
        (
            "artifact-finalize-recovery-v1.fixture.json",
            "artifact_finalize_recovery",
        ),
        (
            "crash-after-reservation-v1.fixture.json",
            "crash_after_reservation",
        ),
        ("crash-after-signer-v1.fixture.json", "crash_after_signer"),
        (
            "crash-after-artifact-stage-v1.fixture.json",
            "crash_after_artifact_stage",
        ),
        ("crash-after-confirm-v1.fixture.json", "crash_after_confirm"),
    ] {
        let fixture: Value = serde_json::from_slice(&fs::read(fixture_dir().join(file_name))?)?;
        if fixture.get("fixtureType").and_then(Value::as_str) != Some(expected_type) {
            return Err(format!("{file_name} fixtureType mismatch").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn assert_runtime_matches_fixture(
    fixture_type: &str,
    outcome: &hiddenshield_feedback_backend::ai_transparency_post_embed_signing::InternalPostEmbedSigningOutcome,
    snapshot: &DatabaseSnapshot,
    store: &InMemoryArtifactStore,
    signer_invocations: usize,
    replay_before: Option<&DatabaseSnapshot>,
) -> Result<(), Box<dyn std::error::Error>> {
    let fixture: Value = serde_json::from_slice(&fs::read(fixture_path_for_type(fixture_type))?)?;
    let expected = fixture
        .get("expected")
        .ok_or_else(|| format!("{fixture_type} fixture missing expected"))?;
    if let Some(expected_artifact) = expected.get("artifactReturned").and_then(Value::as_bool) {
        if outcome.final_signed_png_bytes.is_some() != expected_artifact {
            return Err(format!("{fixture_type} artifactReturned mismatch").into());
        }
    }
    if let Some(expected_quarantine) = expected
        .get("signedBytesQuarantined")
        .and_then(Value::as_bool)
    {
        let execution_id = format!("execution-{fixture_type}");
        if store.is_quarantined(&execution_id) != expected_quarantine {
            return Err(format!("{fixture_type} signedBytesQuarantined mismatch").into());
        }
    }
    if let Some(expected_orphan) = expected
        .get("orphanSigningEventCreated")
        .and_then(Value::as_bool)
    {
        if outcome.orphan_signing_event_created != expected_orphan {
            return Err(format!("{fixture_type} orphanSigningEventCreated mismatch").into());
        }
    }
    if let Some(expected_replay) = expected
        .get("replayedExistingProjection")
        .and_then(Value::as_bool)
    {
        if outcome.replayed != expected_replay {
            return Err(format!("{fixture_type} replayedExistingProjection mismatch").into());
        }
    }
    if let Some(expected_confirmed) = expected
        .get("committedConfirmedMarkedImageCount")
        .and_then(Value::as_i64)
    {
        if snapshot.committed_ledger_count != expected_confirmed {
            return Err(
                format!("{fixture_type} committedConfirmedMarkedImageCount mismatch").into(),
            );
        }
    }
    if let Some(expected_confirm_writes) = expected.get("confirmWrites").and_then(Value::as_i64) {
        let confirm_writes = snapshot.manifest_count
            + snapshot.confirm_audit_count
            + snapshot.committed_ledger_count;
        if confirm_writes != expected_confirm_writes {
            return Err(format!("{fixture_type} confirmWrites mismatch").into());
        }
    }
    if let Some(expected_metering) = expected
        .get("customerMeteringQuantity")
        .and_then(Value::as_i64)
    {
        let actual_metering = replay_before
            .map(|before| snapshot.committed_ledger_quantity - before.committed_ledger_quantity)
            .unwrap_or(snapshot.committed_ledger_quantity);
        if actual_metering != expected_metering {
            return Err(format!("{fixture_type} customerMeteringQuantity mismatch").into());
        }
    }
    if expected
        .get("secondSignerReceiptCreated")
        .and_then(Value::as_bool)
        == Some(false)
        && signer_invocations != 1
    {
        return Err(format!("{fixture_type} secondSignerReceiptCreated mismatch").into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../docs/contracts/ai-transparency-post-embed-signing")
}

#[cfg(feature = "postgres")]
fn fixture_path_for_type(fixture_type: &str) -> PathBuf {
    let file_name = match fixture_type {
        "success" => "success-v1.fixture.json",
        "signer_rejected" => "signer-rejected-v1.fixture.json",
        "receipt_hash_mismatch" => "receipt-hash-mismatch-v1.fixture.json",
        "c2pa_readback_failure" => "c2pa-readback-failure-v1.fixture.json",
        "v3_readback_failure" => "v3-readback-failure-v1.fixture.json",
        "confirm_rollback" => "confirm-rollback-v1.fixture.json",
        "duplicate_replay" => "duplicate-replay-v1.fixture.json",
        "crash_after_reservation" => "crash-after-reservation-v1.fixture.json",
        "crash_after_signer" => "crash-after-signer-v1.fixture.json",
        "crash_after_artifact_stage" => "crash-after-artifact-stage-v1.fixture.json",
        "crash_after_confirm" => "crash-after-confirm-v1.fixture.json",
        _ => return fixture_dir().join("unknown.fixture.json"),
    };
    fixture_dir().join(file_name)
}

#[cfg(feature = "postgres")]
fn safe_smoke_url(database_url: &str) -> bool {
    let lower = database_url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_migrate_smoke")
}

#[cfg(feature = "postgres")]
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

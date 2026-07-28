use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::ai_transparency_change_command::{
    ActorAuthorizationInput, ApprovalReferenceInput, ApprovalReferenceType, ChangeCommandPreflight,
};

pub const DEAD_LETTER_REQUEUE_OPERATION: &str = "requeue_post_embed_dead_letter";
pub const DEAD_LETTER_TARGET_TYPE: &str = "post_embed_recovery";
pub const DEAD_LETTER_REQUEST_DIGEST_VERSION: &str =
    "hs-ai-post-embed-dead-letter-requeue-digest-v1";
pub const REASON_DEAD_LETTER_INSPECTED: &str = "ai_post_embed_dead_letter_inspected";
pub const REASON_DEAD_LETTER_NOT_FOUND: &str = "ai_post_embed_dead_letter_not_found";
pub const REASON_DEAD_LETTER_REQUEUED: &str = "ai_post_embed_dead_letter_requeued";
pub const REASON_REQUEST_DIGEST_MISMATCH: &str = "request_digest_mismatch";
pub const REASON_TARGET_STATE_CONFLICT: &str = "target_state_conflict";
pub const REASON_TARGET_VERSION_CONFLICT: &str = "target_version_conflict";
pub const REASON_IDEMPOTENCY_REPLAY: &str = "idempotency_replay";
pub const REASON_CONFLICTING_REQUEST_EXISTS: &str = "conflicting_request_exists";
pub const REASON_AUDIT_WRITE_FAILED: &str = "audit_write_failed";

#[derive(Debug, Clone)]
pub struct DeadLetterInspectCommand {
    pub execution_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub actor_snapshot_id: String,
    pub actor_token_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeadLetterInspection {
    pub execution_id: String,
    pub signing_status: String,
    pub recovery_state: String,
    pub worker_recovery_attempts: i32,
    pub recovery_control_version: i32,
    pub last_recovery_reason: String,
    pub dead_lettered_at: DateTime<Utc>,
    pub last_requeue_change_request_id: Option<String>,
    pub requeued_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeadLetterRequeueMode {
    SubmitRequest,
    ApproveRequest,
    ExecuteApprovedRequest,
}

#[derive(Debug, Clone)]
pub struct DeadLetterRequeueCommand {
    pub mode: DeadLetterRequeueMode,
    pub change_request_id: String,
    pub approval_id: String,
    pub change_execution_id: String,
    pub target_execution_id: String,
    pub target_scope_key: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub expected_control_version: i32,
    pub desired_control_version: i32,
    pub security_review_reference: String,
    pub requester_snapshot_id: String,
    pub requester_actor_id: String,
    pub requester_token_hash: String,
    pub approver_snapshot_id: String,
    pub approver_actor_id: String,
    pub approver_role: String,
    pub approver_token_hash: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
    pub request_digest: String,
    pub idempotency_key: String,
    pub inject_audit_failure: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeadLetterRequeueOutcome {
    pub succeeded: bool,
    pub request_status: String,
    pub reason_code: Option<String>,
    pub recovery_control_version: i32,
}

pub trait DeadLetterRequeueExecutionHook: Send + Sync {
    fn after_dead_letter_locked(&self);
}

pub struct NoopDeadLetterRequeueExecutionHook;

impl DeadLetterRequeueExecutionHook for NoopDeadLetterRequeueExecutionHook {
    fn after_dead_letter_locked(&self) {}
}

#[derive(Debug, thiserror::Error)]
pub enum DeadLetterCommandError {
    #[error("PostgreSQL dead-letter command failed: {0}")]
    Postgres(#[from] sqlx::Error),
}

pub async fn inspect_postgres_dead_letter(
    connection: &mut PgConnection,
    command: &DeadLetterInspectCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<Option<DeadLetterInspection>, DeadLetterCommandError> {
    let authorization = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.actor_token_hash,
            required_role: "ai_transparency_readonly_auditor",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "inspect_post_embed_dead_letter",
        });
    let mut transaction = connection.begin().await?;
    if !authorization.authorized {
        insert_inspection_audit(
            &mut transaction,
            command,
            "denied",
            authorization
                .reason_code
                .as_deref()
                .unwrap_or("iam_unavailable"),
        )
        .await?;
        transaction.commit().await?;
        return Ok(None);
    }
    let row = sqlx::query(
        "SELECT execution.execution_id, execution.status, execution.recovery_state,
                execution.worker_recovery_attempts, execution.recovery_control_version,
                execution.last_recovery_reason, execution.dead_lettered_at,
                execution.last_requeue_change_request_id, execution.requeued_at
         FROM ai_post_embed_signing_executions execution
         JOIN ai_transparency_licenses license ON license.license_id = execution.license_id
         WHERE execution.execution_id = $1
           AND execution.recovery_state = 'dead_letter'
           AND license.tenant_id = $2
           AND license.workspace_id = $3
           AND license.environment = $4",
    )
    .bind(&command.execution_id)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .fetch_optional(&mut *transaction)
    .await?;
    let inspection = row.map(|row| DeadLetterInspection {
        execution_id: row.get("execution_id"),
        signing_status: row.get("status"),
        recovery_state: row.get("recovery_state"),
        worker_recovery_attempts: row.get("worker_recovery_attempts"),
        recovery_control_version: row.get("recovery_control_version"),
        last_recovery_reason: row.get("last_recovery_reason"),
        dead_lettered_at: row.get("dead_lettered_at"),
        last_requeue_change_request_id: row.get("last_requeue_change_request_id"),
        requeued_at: row.get("requeued_at"),
    });
    insert_inspection_audit(
        &mut transaction,
        command,
        if inspection.is_some() {
            "succeeded"
        } else {
            "not_found"
        },
        if inspection.is_some() {
            REASON_DEAD_LETTER_INSPECTED
        } else {
            REASON_DEAD_LETTER_NOT_FOUND
        },
    )
    .await?;
    transaction.commit().await?;
    Ok(inspection)
}

pub async fn execute_postgres_dead_letter_requeue(
    connection: &mut PgConnection,
    command: &DeadLetterRequeueCommand,
    preflight: &ChangeCommandPreflight<'_>,
    hook: &dyn DeadLetterRequeueExecutionHook,
) -> Result<DeadLetterRequeueOutcome, DeadLetterCommandError> {
    if let Some(outcome) = validate_requeue_preflight(command, preflight) {
        return Ok(outcome);
    }
    let mut transaction = connection.begin().await?;
    let outcome = match command.mode {
        DeadLetterRequeueMode::SubmitRequest => {
            submit_requeue_request(&mut transaction, command).await
        }
        DeadLetterRequeueMode::ApproveRequest => {
            approve_requeue_request(&mut transaction, command).await
        }
        DeadLetterRequeueMode::ExecuteApprovedRequest => {
            execute_approved_requeue(&mut transaction, command, hook).await
        }
    };
    match outcome {
        Ok(outcome) if outcome.succeeded => {
            transaction.commit().await?;
            Ok(outcome)
        }
        Ok(outcome) => {
            transaction.rollback().await?;
            Ok(outcome)
        }
        Err(_) if command.inject_audit_failure => {
            transaction.rollback().await?;
            Ok(failed_requeue(
                REASON_AUDIT_WRITE_FAILED,
                command.expected_control_version,
            ))
        }
        Err(error) => {
            transaction.rollback().await?;
            Err(error)
        }
    }
}

pub fn canonical_dead_letter_requeue_digest(command: &DeadLetterRequeueCommand) -> String {
    let canonical = json!([
        DEAD_LETTER_REQUEST_DIGEST_VERSION,
        DEAD_LETTER_REQUEUE_OPERATION,
        DEAD_LETTER_TARGET_TYPE,
        command.target_execution_id,
        command.target_scope_key,
        command.tenant_id,
        command.workspace_id,
        command.environment,
        command.expected_control_version,
        command.desired_control_version,
        desired_requeue_state(command),
        command.security_review_reference,
        command.requester_actor_id,
        command.requester_snapshot_id
    ]);
    sha256_hex(
        serde_json::to_string(&canonical)
            .expect("dead-letter requeue canonical JSON")
            .as_bytes(),
    )
}

fn validate_requeue_preflight(
    command: &DeadLetterRequeueCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Option<DeadLetterRequeueOutcome> {
    if command.target_scope_key != format!("post_embed_recovery:{}", command.target_execution_id)
        || command.expected_control_version < 1
        || command.desired_control_version != command.expected_control_version + 1
        || command.request_digest != canonical_dead_letter_requeue_digest(command)
        || command.environment != "production"
    {
        return Some(failed_requeue(
            REASON_REQUEST_DIGEST_MISMATCH,
            command.expected_control_version,
        ));
    }
    let actor_checks: &[(&str, &str)] = match command.mode {
        DeadLetterRequeueMode::SubmitRequest => &[(
            command.requester_token_hash.as_str(),
            "ai_transparency_requester",
        )],
        DeadLetterRequeueMode::ApproveRequest => &[(
            command.approver_token_hash.as_str(),
            "ai_transparency_security_approver",
        )],
        DeadLetterRequeueMode::ExecuteApprovedRequest => {
            &[(command.executor_token_hash.as_str(), "system_executor")]
        }
    };
    for (token_hash, required_role) in actor_checks {
        let decision = preflight
            .iam
            .verify_actor_authorization(&ActorAuthorizationInput {
                token_hash,
                required_role,
                tenant_id: &command.tenant_id,
                workspace_id: &command.workspace_id,
                environment: &command.environment,
                operation: DEAD_LETTER_REQUEUE_OPERATION,
            });
        if !decision.authorized {
            return Some(failed_requeue(
                decision.reason_code.as_deref().unwrap_or("iam_unavailable"),
                command.expected_control_version,
            ));
        }
    }
    if command.mode == DeadLetterRequeueMode::ApproveRequest {
        if command.approver_role != "ai_transparency_security_approver"
            || command.requester_actor_id == command.approver_actor_id
        {
            return Some(failed_requeue(
                REASON_TARGET_STATE_CONFLICT,
                command.expected_control_version,
            ));
        }
        let decision = preflight
            .references
            .verify_approval_reference(&ApprovalReferenceInput {
                reference_type: ApprovalReferenceType::SecurityReview,
                reference_id: &command.security_review_reference,
                tenant_id: &command.tenant_id,
                workspace_id: &command.workspace_id,
                environment: &command.environment,
                operation: DEAD_LETTER_REQUEUE_OPERATION,
            });
        if !decision.verified {
            return Some(failed_requeue(
                decision
                    .reason_code
                    .as_deref()
                    .unwrap_or("reference_unavailable"),
                command.expected_control_version,
            ));
        }
    }
    None
}

async fn submit_requeue_request(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeadLetterRequeueCommand,
) -> Result<DeadLetterRequeueOutcome, DeadLetterCommandError> {
    acquire_target_lock(transaction, &command.target_scope_key).await?;
    if idempotency_exists(transaction, command).await? {
        return Ok(failed_requeue(
            REASON_IDEMPOTENCY_REPLAY,
            command.expected_control_version,
        ));
    }
    if inflight_request_exists(transaction, command).await? {
        return Ok(failed_requeue(
            REASON_CONFLICTING_REQUEST_EXISTS,
            command.expected_control_version,
        ));
    }
    let current_version = dead_letter_control_version(transaction, command).await?;
    if current_version != Some(command.expected_control_version) {
        return Ok(failed_requeue(
            if current_version.is_some() {
                REASON_TARGET_VERSION_CONFLICT
            } else {
                REASON_TARGET_STATE_CONFLICT
            },
            current_version.unwrap_or(0),
        ));
    }
    sqlx::query(
        "INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_id, target_scope_key,
            tenant_id, workspace_id, environment, expected_target_version,
            desired_next_version, desired_state_json, request_reason,
            security_review_reference, requester_snapshot_id, request_digest_version,
            request_digest, idempotency_key, status, expires_at, evidence_quality,
            production_eligibility, created_at, updated_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,
            'internal dead-letter requeue request',$12,$13,$14,$15,$16,
            'pending_review',NOW() + INTERVAL '1 day','native_four_eyes',FALSE,NOW(),NOW())",
    )
    .bind(&command.change_request_id)
    .bind(DEAD_LETTER_REQUEUE_OPERATION)
    .bind(DEAD_LETTER_TARGET_TYPE)
    .bind(&command.target_execution_id)
    .bind(&command.target_scope_key)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(command.expected_control_version)
    .bind(command.desired_control_version)
    .bind(desired_requeue_state(command))
    .bind(&command.security_review_reference)
    .bind(&command.requester_snapshot_id)
    .bind(DEAD_LETTER_REQUEST_DIGEST_VERSION)
    .bind(&command.request_digest)
    .bind(&command.idempotency_key)
    .execute(&mut **transaction)
    .await?;
    insert_change_audit(
        transaction,
        command,
        1,
        "change_request_submitted",
        None,
        "pending_review",
        &command.requester_snapshot_id,
        "request_submitted",
        None,
        None,
        json!({"desiredRecoveryState": "retry_scheduled"}),
    )
    .await?;
    Ok(success_requeue(
        "pending_review",
        command.expected_control_version,
    ))
}

async fn approve_requeue_request(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeadLetterRequeueCommand,
) -> Result<DeadLetterRequeueOutcome, DeadLetterCommandError> {
    acquire_target_lock(transaction, &command.target_scope_key).await?;
    let request_status = matching_request_status(transaction, command).await?;
    if request_status.as_deref() != Some("pending_review") {
        return Ok(failed_requeue(
            REASON_TARGET_STATE_CONFLICT,
            command.expected_control_version,
        ));
    }
    sqlx::query(
        "INSERT INTO ai_transparency_change_approvals (
            approval_id, change_request_id, decision, approver_snapshot_id,
            requester_actor_id, approver_actor_id, approver_role, decision_reason,
            policy_version, request_digest, decided_at
         ) VALUES ($1,$2,'approved',$3,$4,$5,$6,
            'approved post-embed dead-letter requeue',
            'ai-transparency-approval-v1',$7,NOW())",
    )
    .bind(&command.approval_id)
    .bind(&command.change_request_id)
    .bind(&command.approver_snapshot_id)
    .bind(&command.requester_actor_id)
    .bind(&command.approver_actor_id)
    .bind(&command.approver_role)
    .bind(&command.request_digest)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "UPDATE ai_transparency_change_requests
         SET status = 'approved', updated_at = NOW()
         WHERE change_request_id = $1 AND status = 'pending_review'",
    )
    .bind(&command.change_request_id)
    .execute(&mut **transaction)
    .await?;
    insert_change_audit(
        transaction,
        command,
        2,
        "approval_granted",
        Some("pending_review"),
        "approved",
        &command.approver_snapshot_id,
        "approval_granted",
        None,
        None,
        json!({"securityReviewReference": command.security_review_reference}),
    )
    .await?;
    Ok(success_requeue(
        "approved",
        command.expected_control_version,
    ))
}

async fn execute_approved_requeue(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeadLetterRequeueCommand,
    hook: &dyn DeadLetterRequeueExecutionHook,
) -> Result<DeadLetterRequeueOutcome, DeadLetterCommandError> {
    acquire_target_lock(transaction, &command.target_scope_key).await?;
    if execution_exists(transaction, command).await? {
        return Ok(failed_requeue(
            REASON_TARGET_STATE_CONFLICT,
            command.expected_control_version,
        ));
    }
    if matching_request_status(transaction, command)
        .await?
        .as_deref()
        != Some("approved")
    {
        return Ok(failed_requeue(
            REASON_TARGET_STATE_CONFLICT,
            command.expected_control_version,
        ));
    }
    let row = sqlx::query(
        "SELECT execution.recovery_control_version, execution.worker_recovery_attempts,
                execution.last_recovery_reason
         FROM ai_post_embed_signing_executions execution
         JOIN ai_transparency_licenses license ON license.license_id = execution.license_id
         WHERE execution.execution_id = $1
           AND execution.recovery_state = 'dead_letter'
           AND license.tenant_id = $2
           AND license.workspace_id = $3
           AND license.environment = $4
         FOR UPDATE OF execution",
    )
    .bind(&command.target_execution_id)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .fetch_optional(&mut **transaction)
    .await?;
    let Some(row) = row else {
        return Ok(failed_requeue(
            REASON_TARGET_STATE_CONFLICT,
            command.expected_control_version,
        ));
    };
    let current_version: i32 = row.get("recovery_control_version");
    if current_version != command.expected_control_version {
        return Ok(failed_requeue(
            REASON_TARGET_VERSION_CONFLICT,
            current_version,
        ));
    }
    let previous_attempts: i32 = row.get("worker_recovery_attempts");
    let previous_reason: String = row.get("last_recovery_reason");
    sqlx::query(
        "INSERT INTO ai_transparency_change_executions (
            execution_id, change_request_id, executor_snapshot_id, status,
            target_version_before, target_version_after, started_at
         ) VALUES ($1,$2,$3,'executing',$4,$5,NOW())",
    )
    .bind(&command.change_execution_id)
    .bind(&command.change_request_id)
    .bind(&command.executor_snapshot_id)
    .bind(command.expected_control_version)
    .bind(command.desired_control_version)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "UPDATE ai_transparency_change_requests
         SET status = 'executing', updated_at = NOW()
         WHERE change_request_id = $1",
    )
    .bind(&command.change_request_id)
    .execute(&mut **transaction)
    .await?;
    insert_change_audit(
        transaction,
        command,
        3,
        "execution_started",
        Some("approved"),
        "executing",
        &command.executor_snapshot_id,
        "execution_started",
        Some(command.expected_control_version),
        Some(command.desired_control_version),
        json!({}),
    )
    .await?;
    hook.after_dead_letter_locked();
    let updated = sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET recovery_state = 'retry_scheduled',
             worker_recovery_attempts = 0,
             next_recovery_at = NOW(),
             recovery_lease_owner = NULL,
             recovery_lease_expires_at = NULL,
             last_recovery_reason = $1,
             dead_lettered_at = NULL,
             recovery_control_version = $2,
             last_requeue_change_request_id = $3,
             requeued_at = NOW(),
             lease_expires_at = CASE WHEN status = 'reserved' THEN NOW() ELSE lease_expires_at END
         WHERE execution_id = $4
           AND recovery_state = 'dead_letter'
           AND recovery_control_version = $5",
    )
    .bind(REASON_DEAD_LETTER_REQUEUED)
    .bind(command.desired_control_version)
    .bind(&command.change_request_id)
    .bind(&command.target_execution_id)
    .bind(command.expected_control_version)
    .execute(&mut **transaction)
    .await?
    .rows_affected();
    if updated != 1 {
        return Ok(failed_requeue(
            REASON_TARGET_VERSION_CONFLICT,
            current_version,
        ));
    }
    insert_change_audit(
        transaction,
        command,
        4,
        "target_state_changed",
        Some("executing"),
        "executing",
        &command.executor_snapshot_id,
        REASON_DEAD_LETTER_REQUEUED,
        Some(command.expected_control_version),
        Some(command.desired_control_version),
        json!({
            "previousRecoveryState": "dead_letter",
            "desiredRecoveryState": "retry_scheduled",
            "previousWorkerRecoveryAttempts": previous_attempts,
            "previousReasonCode": previous_reason
        }),
    )
    .await?;
    sqlx::query(
        "UPDATE ai_transparency_change_executions
         SET status = 'succeeded', target_version_after = $2, finished_at = NOW()
         WHERE execution_id = $1",
    )
    .bind(&command.change_execution_id)
    .bind(command.desired_control_version)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "UPDATE ai_transparency_change_requests
         SET status = 'succeeded', updated_at = NOW()
         WHERE change_request_id = $1",
    )
    .bind(&command.change_request_id)
    .execute(&mut **transaction)
    .await?;
    insert_change_audit(
        transaction,
        command,
        5,
        "execution_succeeded",
        Some("executing"),
        "succeeded",
        &command.executor_snapshot_id,
        "execution_succeeded",
        Some(command.expected_control_version),
        Some(command.desired_control_version),
        json!({}),
    )
    .await?;
    if command.inject_audit_failure {
        insert_change_audit(
            transaction,
            command,
            5,
            "execution_succeeded",
            Some("executing"),
            "succeeded",
            &command.executor_snapshot_id,
            "forced_duplicate_audit",
            Some(command.expected_control_version),
            Some(command.desired_control_version),
            json!({}),
        )
        .await?;
    }
    Ok(success_requeue(
        "succeeded",
        command.desired_control_version,
    ))
}

async fn insert_inspection_audit(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeadLetterInspectCommand,
    outcome: &str,
    reason_code: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_post_embed_dead_letter_inspection_audit_events (
            inspection_audit_event_id, execution_id, actor_snapshot_id, outcome,
            reason_code, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,NOW())",
    )
    .bind(format!("dead-letter-inspect-{}", Uuid::new_v4()))
    .bind(&command.execution_id)
    .bind(&command.actor_snapshot_id)
    .bind(outcome)
    .bind(reason_code)
    .bind(json!({"operation": "inspect_post_embed_dead_letter"}))
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_change_audit(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeadLetterRequeueCommand,
    sequence: i32,
    event_type: &str,
    from_state: Option<&str>,
    to_state: &str,
    actor_snapshot_id: &str,
    reason_code: &str,
    target_version_before: Option<i32>,
    target_version_after: Option<i32>,
    details: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_transparency_change_audit_events (
            audit_event_id, change_request_id, sequence, event_type, from_state, to_state,
            actor_snapshot_id, target_type, target_id, target_version_before,
            target_version_after, reason_code, request_digest, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,NOW())",
    )
    .bind(format!("{}-audit-{sequence}", command.change_request_id))
    .bind(&command.change_request_id)
    .bind(sequence)
    .bind(event_type)
    .bind(from_state)
    .bind(to_state)
    .bind(actor_snapshot_id)
    .bind(DEAD_LETTER_TARGET_TYPE)
    .bind(&command.target_execution_id)
    .bind(target_version_before)
    .bind(target_version_after)
    .bind(reason_code)
    .bind(&command.request_digest)
    .bind(details)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

async fn acquire_target_lock(
    transaction: &mut Transaction<'_, Postgres>,
    target_scope_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_transparency_change_target_locks (target_scope_key, updated_at)
         VALUES ($1,NOW()) ON CONFLICT(target_scope_key) DO NOTHING",
    )
    .bind(target_scope_key)
    .execute(&mut **transaction)
    .await?;
    sqlx::query(
        "SELECT target_scope_key FROM ai_transparency_change_target_locks
         WHERE target_scope_key = $1 FOR UPDATE",
    )
    .bind(target_scope_key)
    .fetch_one(&mut **transaction)
    .await?;
    Ok(())
}

async fn dead_letter_control_version(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeadLetterRequeueCommand,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT execution.recovery_control_version
         FROM ai_post_embed_signing_executions execution
         JOIN ai_transparency_licenses license ON license.license_id = execution.license_id
         WHERE execution.execution_id = $1
           AND execution.recovery_state = 'dead_letter'
           AND license.tenant_id = $2
           AND license.workspace_id = $3
           AND license.environment = $4",
    )
    .bind(&command.target_execution_id)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .fetch_optional(&mut **transaction)
    .await
}

async fn matching_request_status(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeadLetterRequeueCommand,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT status
         FROM ai_transparency_change_requests
         WHERE change_request_id = $1
           AND operation = $2
           AND target_type = $3
           AND target_id = $4
           AND target_scope_key = $5
           AND request_digest = $6
           AND expected_target_version = $7
           AND desired_next_version = $8",
    )
    .bind(&command.change_request_id)
    .bind(DEAD_LETTER_REQUEUE_OPERATION)
    .bind(DEAD_LETTER_TARGET_TYPE)
    .bind(&command.target_execution_id)
    .bind(&command.target_scope_key)
    .bind(&command.request_digest)
    .bind(command.expected_control_version)
    .bind(command.desired_control_version)
    .fetch_optional(&mut **transaction)
    .await
}

async fn idempotency_exists(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeadLetterRequeueCommand,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_requests
            WHERE requester_snapshot_id = $1 AND idempotency_key = $2
         )",
    )
    .bind(&command.requester_snapshot_id)
    .bind(&command.idempotency_key)
    .fetch_one(&mut **transaction)
    .await
}

async fn inflight_request_exists(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeadLetterRequeueCommand,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_requests
            WHERE target_scope_key = $1
              AND status IN ('pending_review', 'approved', 'executing')
         )",
    )
    .bind(&command.target_scope_key)
    .fetch_one(&mut **transaction)
    .await
}

async fn execution_exists(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeadLetterRequeueCommand,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_executions
            WHERE execution_id = $1 OR change_request_id = $2
         )",
    )
    .bind(&command.change_execution_id)
    .bind(&command.change_request_id)
    .fetch_one(&mut **transaction)
    .await
}

fn desired_requeue_state(command: &DeadLetterRequeueCommand) -> Value {
    json!({
        "schemaVersion": "hs-ai-post-embed-dead-letter-requeue-desired-state-v1",
        "executionId": command.target_execution_id,
        "recoveryState": "retry_scheduled",
        "resetWorkerRecoveryAttempts": true,
        "nextRecoveryAt": "immediate",
        "expectedRecoveryControlVersion": command.expected_control_version,
        "desiredRecoveryControlVersion": command.desired_control_version
    })
}

fn success_requeue(
    request_status: &str,
    recovery_control_version: i32,
) -> DeadLetterRequeueOutcome {
    DeadLetterRequeueOutcome {
        succeeded: true,
        request_status: request_status.to_string(),
        reason_code: None,
        recovery_control_version,
    }
}

fn failed_requeue(reason_code: &str, recovery_control_version: i32) -> DeadLetterRequeueOutcome {
    DeadLetterRequeueOutcome {
        succeeded: false,
        request_status: "failed".to_string(),
        reason_code: Some(reason_code.to_string()),
        recovery_control_version,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

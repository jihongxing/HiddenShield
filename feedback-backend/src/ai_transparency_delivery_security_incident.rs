use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::ai_transparency_change_command::{
    ActorAuthorizationInput, ApprovalReferenceInput, ApprovalReferenceType, ChangeCommandPreflight,
};
use crate::ai_transparency_delivery_observability::{
    execute_postgres_cleanup_delivery_security_windows, CleanupDeliverySecurityWindowsCommand,
};
use crate::ai_transparency_delivery_security_notification::{
    enqueue_delivery_security_notification, EnqueueDeliverySecurityNotificationInput,
};

pub const DELIVERY_SECURITY_INCIDENT_TARGET_TYPE: &str = "delivery_security_incident";
pub const DELIVERY_SECURITY_INCIDENT_DIGEST_VERSION: &str =
    "hs-ai-delivery-security-incident-change-digest-v1";
pub const ACK_DELIVERY_SECURITY_INCIDENT_OPERATION: &str = "ack_delivery_security_incident";
pub const RESOLVE_DELIVERY_SECURITY_INCIDENT_OPERATION: &str = "resolve_delivery_security_incident";
pub const DELIVERY_SECURITY_CLEANUP_INTERVAL_MINUTES: i32 = 15;
pub const DELIVERY_SECURITY_CLEANUP_LEASE_MINUTES: i64 = 5;
pub const DELIVERY_SECURITY_CLEANUP_BASE_BACKOFF_MINUTES: i64 = 1;
pub const DELIVERY_SECURITY_CLEANUP_MAX_BACKOFF_MINUTES: i64 = 60;

pub const REASON_INCIDENT_OPENED: &str = "ai_delivery_security_incident_opened";
pub const REASON_INCIDENT_EVIDENCE_MERGED: &str = "ai_delivery_security_incident_evidence_merged";
pub const REASON_INCIDENT_ACKNOWLEDGED: &str = "ai_delivery_security_incident_acknowledged";
pub const REASON_INCIDENT_RESOLVED: &str = "ai_delivery_security_incident_resolved";
pub const REASON_INCIDENT_INVALID: &str = "ai_delivery_security_incident_invalid";
pub const REASON_REQUEST_DIGEST_MISMATCH: &str = "request_digest_mismatch";
pub const REASON_TARGET_STATE_CONFLICT: &str = "target_state_conflict";
pub const REASON_TARGET_VERSION_CONFLICT: &str = "target_version_conflict";
pub const REASON_IDEMPOTENCY_REPLAY: &str = "idempotency_replay";
pub const REASON_CONFLICTING_REQUEST_EXISTS: &str = "conflicting_request_exists";
pub const REASON_CLEANUP_SCHEDULED: &str = "ai_delivery_security_cleanup_scheduled";
pub const REASON_CLEANUP_CLAIMED: &str = "ai_delivery_security_cleanup_claimed";
pub const REASON_CLEANUP_SUCCEEDED: &str = "ai_delivery_security_cleanup_runner_succeeded";
pub const REASON_CLEANUP_FAILED: &str = "ai_delivery_security_cleanup_runner_failed";

#[derive(Debug, Clone)]
pub struct DeliverySecurityIncidentProjectionInput<'a> {
    pub tenant_id: &'a str,
    pub workspace_id: &'a str,
    pub environment: &'a str,
    pub summary_id: &'a str,
    pub summary_digest: &'a str,
    pub severity: &'a str,
    pub alert_codes: &'a [String],
    pub actor_snapshot_id: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySecurityIncidentProjection {
    pub incident_id: String,
    pub incident_key: String,
    pub status: String,
    pub severity: String,
    pub occurrence_count: i64,
    pub control_version: i32,
    pub event_type: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliverySecurityIncidentChangeMode {
    SubmitRequest,
    ApproveRequest,
    ExecuteApprovedRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeliverySecurityIncidentDesiredStatus {
    Acknowledged,
    Resolved,
}

impl DeliverySecurityIncidentDesiredStatus {
    fn as_str(self) -> &'static str {
        match self {
            Self::Acknowledged => "acknowledged",
            Self::Resolved => "resolved",
        }
    }

    fn operation(self) -> &'static str {
        match self {
            Self::Acknowledged => ACK_DELIVERY_SECURITY_INCIDENT_OPERATION,
            Self::Resolved => RESOLVE_DELIVERY_SECURITY_INCIDENT_OPERATION,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeliverySecurityIncidentChangeCommand {
    pub mode: DeliverySecurityIncidentChangeMode,
    pub desired_status: DeliverySecurityIncidentDesiredStatus,
    pub change_request_id: String,
    pub approval_id: String,
    pub change_execution_id: String,
    pub incident_id: String,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySecurityIncidentChangeOutcome {
    pub succeeded: bool,
    pub request_status: String,
    pub incident_status: String,
    pub reason_code: Option<String>,
    pub control_version: i32,
}

#[derive(Debug, Clone)]
pub struct EnsureDeliverySecurityCleanupScheduleCommand {
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
    pub interval_minutes: i32,
}

#[derive(Debug, Clone)]
pub struct RunDeliverySecurityCleanupScheduleCommand {
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
    pub runner_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySecurityCleanupScheduleOutcome {
    pub succeeded: bool,
    pub reason_code: Option<String>,
    pub schedule_id: Option<String>,
    pub run_id: Option<String>,
    pub claimed: bool,
    pub deleted_rate_windows: i64,
    pub deleted_metric_snapshots: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum DeliverySecurityIncidentError {
    #[error("PostgreSQL delivery security incident command failed: {0}")]
    Postgres(#[from] sqlx::Error),
}

pub async fn project_delivery_security_incident(
    transaction: &mut Transaction<'_, Postgres>,
    input: &DeliverySecurityIncidentProjectionInput<'_>,
) -> Result<Option<DeliverySecurityIncidentProjection>, sqlx::Error> {
    if !matches!(input.severity, "warning" | "critical") || input.alert_codes.is_empty() {
        return Ok(None);
    }
    let mut alert_codes = input.alert_codes.to_vec();
    alert_codes.sort();
    alert_codes.dedup();
    let incident_key = sha256_hex(
        serde_json::to_string(&json!([
            input.tenant_id,
            input.workspace_id,
            input.environment,
            alert_codes
        ]))
        .expect("delivery security incident key JSON")
        .as_bytes(),
    );
    let previous_severity: Option<String> = sqlx::query_scalar(
        "SELECT severity FROM ai_delivery_security_incidents
         WHERE active_incident_key = $1
         FOR UPDATE",
    )
    .bind(&incident_key)
    .fetch_optional(&mut **transaction)
    .await?;
    let candidate_id = format!("delivery-security-incident-{}", Uuid::new_v4());
    let row = sqlx::query(
        "INSERT INTO ai_delivery_security_incidents (
            incident_id, tenant_id, workspace_id, environment, incident_key,
            active_incident_key, severity, status, alert_codes_json, occurrence_count,
            first_summary_id, first_summary_digest, latest_summary_id, latest_summary_digest,
            control_version, opened_at, updated_at
         ) VALUES ($1,$2,$3,$4,$5,$5,$6,'open',$7,1,$8,$9,$8,$9,1,NOW(),NOW())
         ON CONFLICT(active_incident_key) DO UPDATE SET
            severity = CASE
                WHEN ai_delivery_security_incidents.severity = 'critical'
                  OR EXCLUDED.severity = 'critical' THEN 'critical'
                ELSE 'warning'
            END,
            alert_codes_json = EXCLUDED.alert_codes_json,
            occurrence_count = ai_delivery_security_incidents.occurrence_count + 1,
            latest_summary_id = EXCLUDED.latest_summary_id,
            latest_summary_digest = EXCLUDED.latest_summary_digest,
            updated_at = NOW()
         RETURNING incident_id, incident_key, status, severity, occurrence_count, control_version",
    )
    .bind(&candidate_id)
    .bind(input.tenant_id)
    .bind(input.workspace_id)
    .bind(input.environment)
    .bind(&incident_key)
    .bind(input.severity)
    .bind(json!(alert_codes))
    .bind(input.summary_id)
    .bind(input.summary_digest)
    .fetch_one(&mut **transaction)
    .await?;
    let occurrence_count: i64 = row.get("occurrence_count");
    let event_type = if occurrence_count == 1 {
        "opened"
    } else {
        "evidence_merged"
    };
    let incident_id: String = row.get("incident_id");
    let status: String = row.get("status");
    let severity: String = row.get("severity");
    let control_version: i32 = row.get("control_version");
    insert_incident_audit(
        transaction,
        &incident_id,
        event_type,
        input.actor_snapshot_id,
        None,
        Some(input.summary_id),
        &severity,
        &status,
        control_version,
        if occurrence_count == 1 {
            REASON_INCIDENT_OPENED
        } else {
            REASON_INCIDENT_EVIDENCE_MERGED
        },
        json!({
            "summaryDigest": input.summary_digest,
            "occurrenceCount": occurrence_count,
            "notificationAdaptersInvoked": false
        }),
    )
    .await?;
    let became_critical = previous_severity.as_deref() == Some("warning") && severity == "critical";
    if event_type == "opened" || became_critical {
        enqueue_delivery_security_notification(
            transaction,
            &EnqueueDeliverySecurityNotificationInput {
                incident_id: &incident_id,
                tenant_id: input.tenant_id,
                workspace_id: input.workspace_id,
                environment: input.environment,
                event_type: if event_type == "opened" {
                    "incident_opened"
                } else {
                    "incident_became_critical"
                },
                priority: &severity,
                incident_status: &status,
                severity: &severity,
                alert_codes: input.alert_codes,
                occurrence_count,
                control_version,
                actor_snapshot_id: input.actor_snapshot_id,
            },
        )
        .await?;
    }
    Ok(Some(DeliverySecurityIncidentProjection {
        incident_id,
        incident_key: row.get("incident_key"),
        status,
        severity,
        occurrence_count,
        control_version,
        event_type: event_type.to_string(),
    }))
}

pub fn canonical_delivery_security_incident_change_digest(
    command: &DeliverySecurityIncidentChangeCommand,
) -> String {
    sha256_hex(
        serde_json::to_string(&json!([
            DELIVERY_SECURITY_INCIDENT_DIGEST_VERSION,
            command.desired_status.operation(),
            DELIVERY_SECURITY_INCIDENT_TARGET_TYPE,
            command.incident_id,
            command.target_scope_key,
            command.tenant_id,
            command.workspace_id,
            command.environment,
            command.expected_control_version,
            command.desired_control_version,
            command.desired_status.as_str(),
            command.security_review_reference,
            command.requester_actor_id,
            command.requester_snapshot_id
        ]))
        .expect("delivery security incident canonical JSON")
        .as_bytes(),
    )
}

pub async fn execute_postgres_delivery_security_incident_change(
    connection: &mut PgConnection,
    command: &DeliverySecurityIncidentChangeCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<DeliverySecurityIncidentChangeOutcome, DeliverySecurityIncidentError> {
    if let Some(outcome) = validate_incident_change_preflight(command, preflight) {
        return Ok(outcome);
    }
    let mut transaction = connection.begin().await?;
    let outcome = match command.mode {
        DeliverySecurityIncidentChangeMode::SubmitRequest => {
            submit_incident_change(&mut transaction, command).await?
        }
        DeliverySecurityIncidentChangeMode::ApproveRequest => {
            approve_incident_change(&mut transaction, command).await?
        }
        DeliverySecurityIncidentChangeMode::ExecuteApprovedRequest => {
            execute_approved_incident_change(&mut transaction, command).await?
        }
    };
    if outcome.succeeded {
        transaction.commit().await?;
    } else {
        transaction.rollback().await?;
    }
    Ok(outcome)
}

pub async fn ensure_postgres_delivery_security_cleanup_schedule(
    connection: &mut PgConnection,
    command: &EnsureDeliverySecurityCleanupScheduleCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<DeliverySecurityCleanupScheduleOutcome, DeliverySecurityIncidentError> {
    if command.interval_minutes != DELIVERY_SECURITY_CLEANUP_INTERVAL_MINUTES {
        return Ok(cleanup_schedule_rejected(REASON_INCIDENT_INVALID));
    }
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.executor_token_hash,
            required_role: "system_executor",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "ensure_ai_delivery_security_cleanup_schedule",
        });
    if !decision.authorized {
        return Ok(cleanup_schedule_rejected(
            decision.reason_code.as_deref().unwrap_or("iam_unavailable"),
        ));
    }
    let mut transaction = connection.begin().await?;
    if !snapshot_matches_scope(
        &mut transaction,
        &command.executor_snapshot_id,
        "system_executor",
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
    )
    .await?
    {
        transaction.rollback().await?;
        return Ok(cleanup_schedule_rejected(REASON_INCIDENT_INVALID));
    }
    let candidate_id = format!("delivery-security-cleanup-schedule-{}", Uuid::new_v4());
    let row = sqlx::query(
        "INSERT INTO ai_delivery_security_cleanup_schedules (
            schedule_id, tenant_id, workspace_id, environment, interval_minutes,
            status, next_run_at, created_by_snapshot_id, created_at, updated_at
         ) VALUES ($1,$2,$3,$4,$5,'active',NOW(),$6,NOW(),NOW())
         ON CONFLICT(tenant_id, workspace_id, environment) DO UPDATE SET
            interval_minutes = EXCLUDED.interval_minutes,
            status = CASE
                WHEN ai_delivery_security_cleanup_schedules.status = 'leased'
                    THEN ai_delivery_security_cleanup_schedules.status
                ELSE 'active'
            END,
            updated_at = NOW()
         RETURNING schedule_id, (xmax = 0) created",
    )
    .bind(&candidate_id)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(command.interval_minutes)
    .bind(&command.executor_snapshot_id)
    .fetch_one(&mut *transaction)
    .await?;
    let schedule_id: String = row.get("schedule_id");
    let created: bool = row.get("created");
    insert_cleanup_runner_audit(
        &mut transaction,
        &schedule_id,
        "schedule-configuration",
        "scheduler-control",
        &command.executor_snapshot_id,
        if created {
            "schedule_created"
        } else {
            "schedule_updated"
        },
        "scheduled",
        REASON_CLEANUP_SCHEDULED,
        0,
        0,
        json!({
            "intervalMinutes": command.interval_minutes,
            "notificationAdaptersRequired": false
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(DeliverySecurityCleanupScheduleOutcome {
        succeeded: true,
        reason_code: None,
        schedule_id: Some(schedule_id),
        run_id: None,
        claimed: false,
        deleted_rate_windows: 0,
        deleted_metric_snapshots: 0,
    })
}

pub async fn run_postgres_due_delivery_security_cleanup(
    pool: &PgPool,
    command: &RunDeliverySecurityCleanupScheduleCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<DeliverySecurityCleanupScheduleOutcome, DeliverySecurityIncidentError> {
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.executor_token_hash,
            required_role: "system_executor",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "run_ai_delivery_security_cleanup_schedule",
        });
    if !decision.authorized {
        return Ok(cleanup_schedule_rejected(
            decision.reason_code.as_deref().unwrap_or("iam_unavailable"),
        ));
    }
    let mut claim_transaction = pool.begin().await?;
    if !snapshot_matches_scope(
        &mut claim_transaction,
        &command.executor_snapshot_id,
        "system_executor",
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
    )
    .await?
    {
        claim_transaction.rollback().await?;
        return Ok(cleanup_schedule_rejected(REASON_INCIDENT_INVALID));
    }
    let run_id = format!("delivery-security-cleanup-run-{}", Uuid::new_v4());
    let claimed = sqlx::query(
        "WITH candidate AS (
            SELECT schedule_id
            FROM ai_delivery_security_cleanup_schedules
            WHERE tenant_id = $1 AND workspace_id = $2 AND environment = $3
              AND (
                (status = 'active' AND next_run_at <= NOW())
                OR
                (status = 'leased' AND lease_expires_at <= NOW())
              )
            FOR UPDATE SKIP LOCKED
            LIMIT 1
         )
         UPDATE ai_delivery_security_cleanup_schedules schedule
         SET status = 'leased',
             lease_owner = $4,
             lease_expires_at = NOW() + INTERVAL '5 minutes',
             last_started_at = NOW(),
             updated_at = NOW()
         FROM candidate
         WHERE schedule.schedule_id = candidate.schedule_id
         RETURNING schedule.schedule_id, schedule.interval_minutes,
                   schedule.consecutive_failures",
    )
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(&command.runner_id)
    .fetch_optional(&mut *claim_transaction)
    .await?;
    let Some(claimed) = claimed else {
        claim_transaction.rollback().await?;
        return Ok(DeliverySecurityCleanupScheduleOutcome {
            succeeded: true,
            reason_code: None,
            schedule_id: None,
            run_id: None,
            claimed: false,
            deleted_rate_windows: 0,
            deleted_metric_snapshots: 0,
        });
    };
    let schedule_id: String = claimed.get("schedule_id");
    let interval_minutes: i32 = claimed.get("interval_minutes");
    let previous_failures: i32 = claimed.get("consecutive_failures");
    insert_cleanup_runner_audit(
        &mut claim_transaction,
        &schedule_id,
        &run_id,
        &command.runner_id,
        &command.executor_snapshot_id,
        "claimed",
        "running",
        REASON_CLEANUP_CLAIMED,
        0,
        0,
        json!({"leaseMinutes": DELIVERY_SECURITY_CLEANUP_LEASE_MINUTES}),
    )
    .await?;
    claim_transaction.commit().await?;

    let cleanup_command = CleanupDeliverySecurityWindowsCommand {
        tenant_id: command.tenant_id.clone(),
        workspace_id: command.workspace_id.clone(),
        environment: command.environment.clone(),
        executor_snapshot_id: command.executor_snapshot_id.clone(),
        executor_token_hash: command.executor_token_hash.clone(),
    };
    let mut cleanup_connection = pool.acquire().await?;
    let cleanup = execute_postgres_cleanup_delivery_security_windows(
        &mut cleanup_connection,
        &cleanup_command,
        preflight,
    )
    .await;
    let mut finalize_transaction = pool.begin().await?;
    match cleanup {
        Ok(outcome) if outcome.succeeded => {
            let updated = sqlx::query(
                "UPDATE ai_delivery_security_cleanup_schedules
                 SET status = 'active', lease_owner = NULL, lease_expires_at = NULL,
                     consecutive_failures = 0, run_count = run_count + 1,
                     next_run_at = NOW() + ($1 * INTERVAL '1 minute'),
                     last_finished_at = NOW(), last_outcome = 'succeeded',
                     last_reason_code = $2, last_deleted_rate_windows = $3,
                     last_deleted_metric_snapshots = $4, updated_at = NOW()
                 WHERE schedule_id = $5 AND status = 'leased' AND lease_owner = $6",
            )
            .bind(interval_minutes)
            .bind(REASON_CLEANUP_SUCCEEDED)
            .bind(outcome.deleted_rate_windows)
            .bind(outcome.deleted_metric_snapshots)
            .bind(&schedule_id)
            .bind(&command.runner_id)
            .execute(&mut *finalize_transaction)
            .await?
            .rows_affected();
            if updated != 1 {
                finalize_transaction.rollback().await?;
                return Err(sqlx::Error::Protocol(
                    "delivery security cleanup runner lost lease".to_string(),
                )
                .into());
            }
            insert_cleanup_runner_audit(
                &mut finalize_transaction,
                &schedule_id,
                &run_id,
                &command.runner_id,
                &command.executor_snapshot_id,
                "succeeded",
                "succeeded",
                REASON_CLEANUP_SUCCEEDED,
                outcome.deleted_rate_windows,
                outcome.deleted_metric_snapshots,
                json!({"nextRunMinutes": interval_minutes}),
            )
            .await?;
            finalize_transaction.commit().await?;
            Ok(DeliverySecurityCleanupScheduleOutcome {
                succeeded: true,
                reason_code: None,
                schedule_id: Some(schedule_id),
                run_id: Some(run_id),
                claimed: true,
                deleted_rate_windows: outcome.deleted_rate_windows,
                deleted_metric_snapshots: outcome.deleted_metric_snapshots,
            })
        }
        result => {
            let reason_code = match &result {
                Ok(outcome) => outcome
                    .reason_code
                    .as_deref()
                    .unwrap_or(REASON_CLEANUP_FAILED),
                Err(_) => REASON_CLEANUP_FAILED,
            };
            let failures = previous_failures.saturating_add(1);
            let backoff_minutes = cleanup_backoff_minutes(failures);
            let updated = sqlx::query(
                "UPDATE ai_delivery_security_cleanup_schedules
                 SET status = 'active', lease_owner = NULL, lease_expires_at = NULL,
                     consecutive_failures = $1, run_count = run_count + 1,
                     next_run_at = NOW() + ($2 * INTERVAL '1 minute'),
                     last_finished_at = NOW(), last_outcome = 'failed',
                     last_reason_code = $3, last_deleted_rate_windows = 0,
                     last_deleted_metric_snapshots = 0, updated_at = NOW()
                 WHERE schedule_id = $4 AND status = 'leased' AND lease_owner = $5",
            )
            .bind(failures)
            .bind(backoff_minutes)
            .bind(reason_code)
            .bind(&schedule_id)
            .bind(&command.runner_id)
            .execute(&mut *finalize_transaction)
            .await?
            .rows_affected();
            if updated != 1 {
                finalize_transaction.rollback().await?;
                return Err(sqlx::Error::Protocol(
                    "delivery security cleanup runner lost failed lease".to_string(),
                )
                .into());
            }
            insert_cleanup_runner_audit(
                &mut finalize_transaction,
                &schedule_id,
                &run_id,
                &command.runner_id,
                &command.executor_snapshot_id,
                "failed",
                "failed",
                reason_code,
                0,
                0,
                json!({"backoffMinutes": backoff_minutes, "consecutiveFailures": failures}),
            )
            .await?;
            finalize_transaction.commit().await?;
            Ok(DeliverySecurityCleanupScheduleOutcome {
                succeeded: false,
                reason_code: Some(reason_code.to_string()),
                schedule_id: Some(schedule_id),
                run_id: Some(run_id),
                claimed: true,
                deleted_rate_windows: 0,
                deleted_metric_snapshots: 0,
            })
        }
    }
}

fn validate_incident_change_preflight(
    command: &DeliverySecurityIncidentChangeCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Option<DeliverySecurityIncidentChangeOutcome> {
    if command.environment != "production"
        || command.target_scope_key != format!("delivery_security_incident:{}", command.incident_id)
        || command.expected_control_version < 1
        || command.desired_control_version != command.expected_control_version + 1
        || command.request_digest != canonical_delivery_security_incident_change_digest(command)
    {
        return Some(incident_change_rejected(
            command,
            REASON_REQUEST_DIGEST_MISMATCH,
            command.expected_control_version,
        ));
    }
    let (token_hash, role) = match command.mode {
        DeliverySecurityIncidentChangeMode::SubmitRequest => (
            command.requester_token_hash.as_str(),
            "ai_transparency_requester",
        ),
        DeliverySecurityIncidentChangeMode::ApproveRequest => (
            command.approver_token_hash.as_str(),
            "ai_transparency_security_approver",
        ),
        DeliverySecurityIncidentChangeMode::ExecuteApprovedRequest => {
            (command.executor_token_hash.as_str(), "system_executor")
        }
    };
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash,
            required_role: role,
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: command.desired_status.operation(),
        });
    if !decision.authorized {
        return Some(incident_change_rejected(
            command,
            decision.reason_code.as_deref().unwrap_or("iam_unavailable"),
            command.expected_control_version,
        ));
    }
    if command.mode == DeliverySecurityIncidentChangeMode::ApproveRequest {
        if command.approver_role != "ai_transparency_security_approver"
            || command.requester_actor_id == command.approver_actor_id
        {
            return Some(incident_change_rejected(
                command,
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
                operation: command.desired_status.operation(),
            });
        if !decision.verified {
            return Some(incident_change_rejected(
                command,
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

async fn submit_incident_change(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeliverySecurityIncidentChangeCommand,
) -> Result<DeliverySecurityIncidentChangeOutcome, sqlx::Error> {
    acquire_target_lock(transaction, &command.target_scope_key).await?;
    if let Some(row) = sqlx::query(
        "SELECT request_digest, status
         FROM ai_transparency_change_requests
         WHERE requester_snapshot_id = $1 AND idempotency_key = $2",
    )
    .bind(&command.requester_snapshot_id)
    .bind(&command.idempotency_key)
    .fetch_optional(&mut **transaction)
    .await?
    {
        let matches = row.get::<String, _>("request_digest") == command.request_digest;
        return Ok(if matches {
            incident_change_success(
                command,
                row.get::<String, _>("status").as_str(),
                REASON_IDEMPOTENCY_REPLAY,
                command.expected_control_version,
            )
        } else {
            incident_change_rejected(
                command,
                REASON_CONFLICTING_REQUEST_EXISTS,
                command.expected_control_version,
            )
        });
    }
    if !snapshot_matches_scope(
        transaction,
        &command.requester_snapshot_id,
        "ai_transparency_requester",
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
    )
    .await?
    {
        return Ok(incident_change_rejected(
            command,
            REASON_INCIDENT_INVALID,
            command.expected_control_version,
        ));
    }
    let current = incident_state(transaction, command, false).await?;
    let Some((status, version)) = current else {
        return Ok(incident_change_rejected(
            command,
            REASON_TARGET_STATE_CONFLICT,
            0,
        ));
    };
    if version != command.expected_control_version {
        return Ok(incident_change_rejected(
            command,
            REASON_TARGET_VERSION_CONFLICT,
            version,
        ));
    }
    if !valid_incident_transition(status.as_str(), command.desired_status) {
        return Ok(incident_change_rejected(
            command,
            REASON_TARGET_STATE_CONFLICT,
            version,
        ));
    }
    let inflight: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_requests
            WHERE target_scope_key = $1
              AND status IN ('pending_review', 'approved', 'executing')
         )",
    )
    .bind(&command.target_scope_key)
    .fetch_one(&mut **transaction)
    .await?;
    if inflight {
        return Ok(incident_change_rejected(
            command,
            REASON_CONFLICTING_REQUEST_EXISTS,
            version,
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
            'internal delivery security incident state change',$12,$13,$14,$15,$16,
            'pending_review',NOW() + INTERVAL '1 day','native_four_eyes',FALSE,NOW(),NOW())",
    )
    .bind(&command.change_request_id)
    .bind(command.desired_status.operation())
    .bind(DELIVERY_SECURITY_INCIDENT_TARGET_TYPE)
    .bind(&command.incident_id)
    .bind(&command.target_scope_key)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(command.expected_control_version)
    .bind(command.desired_control_version)
    .bind(desired_incident_state(command))
    .bind(&command.security_review_reference)
    .bind(&command.requester_snapshot_id)
    .bind(DELIVERY_SECURITY_INCIDENT_DIGEST_VERSION)
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
        json!({"desiredIncidentStatus": command.desired_status.as_str()}),
    )
    .await?;
    Ok(incident_change_success(
        command,
        "pending_review",
        "request_submitted",
        version,
    ))
}

async fn approve_incident_change(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeliverySecurityIncidentChangeCommand,
) -> Result<DeliverySecurityIncidentChangeOutcome, sqlx::Error> {
    acquire_target_lock(transaction, &command.target_scope_key).await?;
    if !snapshot_matches_scope(
        transaction,
        &command.approver_snapshot_id,
        "ai_transparency_security_approver",
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
    )
    .await?
    {
        return Ok(incident_change_rejected(
            command,
            REASON_INCIDENT_INVALID,
            command.expected_control_version,
        ));
    }
    let request_status = matching_request_status(transaction, command).await?;
    if request_status.as_deref() == Some("approved") {
        return Ok(incident_change_success(
            command,
            "approved",
            REASON_IDEMPOTENCY_REPLAY,
            command.expected_control_version,
        ));
    }
    if request_status.as_deref() != Some("pending_review") {
        return Ok(incident_change_rejected(
            command,
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
            'approved delivery security incident state change',
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
    Ok(incident_change_success(
        command,
        "approved",
        "approval_granted",
        command.expected_control_version,
    ))
}

async fn execute_approved_incident_change(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeliverySecurityIncidentChangeCommand,
) -> Result<DeliverySecurityIncidentChangeOutcome, sqlx::Error> {
    acquire_target_lock(transaction, &command.target_scope_key).await?;
    if !snapshot_matches_scope(
        transaction,
        &command.executor_snapshot_id,
        "system_executor",
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
    )
    .await?
    {
        return Ok(incident_change_rejected(
            command,
            REASON_INCIDENT_INVALID,
            command.expected_control_version,
        ));
    }
    let existing: bool = sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_change_executions
            WHERE execution_id = $1 OR change_request_id = $2
         )",
    )
    .bind(&command.change_execution_id)
    .bind(&command.change_request_id)
    .fetch_one(&mut **transaction)
    .await?;
    if existing {
        let current = incident_state(transaction, command, false).await?;
        if current.as_ref()
            == Some(&(
                command.desired_status.as_str().to_string(),
                command.desired_control_version,
            ))
        {
            return Ok(incident_change_success(
                command,
                "succeeded",
                REASON_IDEMPOTENCY_REPLAY,
                command.desired_control_version,
            ));
        }
        return Ok(incident_change_rejected(
            command,
            REASON_TARGET_STATE_CONFLICT,
            command.expected_control_version,
        ));
    }
    if matching_request_status(transaction, command)
        .await?
        .as_deref()
        != Some("approved")
    {
        return Ok(incident_change_rejected(
            command,
            REASON_TARGET_STATE_CONFLICT,
            command.expected_control_version,
        ));
    }
    let current = incident_state(transaction, command, true).await?;
    let Some((current_status, current_version)) = current else {
        return Ok(incident_change_rejected(
            command,
            REASON_TARGET_STATE_CONFLICT,
            0,
        ));
    };
    if current_version != command.expected_control_version {
        return Ok(incident_change_rejected(
            command,
            REASON_TARGET_VERSION_CONFLICT,
            current_version,
        ));
    }
    if !valid_incident_transition(&current_status, command.desired_status) {
        return Ok(incident_change_rejected(
            command,
            REASON_TARGET_STATE_CONFLICT,
            current_version,
        ));
    }
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
    let updated = match command.desired_status {
        DeliverySecurityIncidentDesiredStatus::Acknowledged => sqlx::query(
            "UPDATE ai_delivery_security_incidents
                 SET status = 'acknowledged', control_version = $1,
                     acknowledged_by_change_request_id = $2, acknowledged_at = NOW(),
                     updated_at = NOW()
                 WHERE incident_id = $3 AND tenant_id = $4 AND workspace_id = $5
                   AND environment = $6 AND status = 'open' AND control_version = $7",
        )
        .bind(command.desired_control_version)
        .bind(&command.change_request_id)
        .bind(&command.incident_id)
        .bind(&command.tenant_id)
        .bind(&command.workspace_id)
        .bind(&command.environment)
        .bind(command.expected_control_version)
        .execute(&mut **transaction)
        .await?
        .rows_affected(),
        DeliverySecurityIncidentDesiredStatus::Resolved => sqlx::query(
            "UPDATE ai_delivery_security_incidents
                 SET status = 'resolved', active_incident_key = NULL, control_version = $1,
                     resolved_by_change_request_id = $2, resolved_at = NOW(), updated_at = NOW()
                 WHERE incident_id = $3 AND tenant_id = $4 AND workspace_id = $5
                   AND environment = $6 AND status IN ('open','acknowledged')
                   AND control_version = $7",
        )
        .bind(command.desired_control_version)
        .bind(&command.change_request_id)
        .bind(&command.incident_id)
        .bind(&command.tenant_id)
        .bind(&command.workspace_id)
        .bind(&command.environment)
        .bind(command.expected_control_version)
        .execute(&mut **transaction)
        .await?
        .rows_affected(),
    };
    if updated != 1 {
        return Ok(incident_change_rejected(
            command,
            REASON_TARGET_VERSION_CONFLICT,
            current_version,
        ));
    }
    let incident = sqlx::query(
        "SELECT severity, alert_codes_json, occurrence_count
         FROM ai_delivery_security_incidents WHERE incident_id = $1",
    )
    .bind(&command.incident_id)
    .fetch_one(&mut **transaction)
    .await?;
    insert_incident_audit(
        transaction,
        &command.incident_id,
        command.desired_status.as_str(),
        &command.executor_snapshot_id,
        Some(&command.change_request_id),
        None,
        incident.get("severity"),
        command.desired_status.as_str(),
        command.desired_control_version,
        match command.desired_status {
            DeliverySecurityIncidentDesiredStatus::Acknowledged => REASON_INCIDENT_ACKNOWLEDGED,
            DeliverySecurityIncidentDesiredStatus::Resolved => REASON_INCIDENT_RESOLVED,
        },
        json!({"previousStatus": current_status}),
    )
    .await?;
    let alert_codes: Vec<String> =
        serde_json::from_value(incident.get("alert_codes_json")).unwrap_or_default();
    enqueue_delivery_security_notification(
        transaction,
        &EnqueueDeliverySecurityNotificationInput {
            incident_id: &command.incident_id,
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            event_type: match command.desired_status {
                DeliverySecurityIncidentDesiredStatus::Acknowledged => "incident_acknowledged",
                DeliverySecurityIncidentDesiredStatus::Resolved => "incident_resolved",
            },
            priority: match command.desired_status {
                DeliverySecurityIncidentDesiredStatus::Acknowledged => "warning",
                DeliverySecurityIncidentDesiredStatus::Resolved => "info",
            },
            incident_status: command.desired_status.as_str(),
            severity: incident.get("severity"),
            alert_codes: &alert_codes,
            occurrence_count: incident.get("occurrence_count"),
            control_version: command.desired_control_version,
            actor_snapshot_id: &command.executor_snapshot_id,
        },
    )
    .await?;
    insert_change_audit(
        transaction,
        command,
        4,
        "target_state_changed",
        Some("executing"),
        "executing",
        &command.executor_snapshot_id,
        match command.desired_status {
            DeliverySecurityIncidentDesiredStatus::Acknowledged => REASON_INCIDENT_ACKNOWLEDGED,
            DeliverySecurityIncidentDesiredStatus::Resolved => REASON_INCIDENT_RESOLVED,
        },
        Some(command.expected_control_version),
        Some(command.desired_control_version),
        json!({
            "previousIncidentStatus": current_status,
            "desiredIncidentStatus": command.desired_status.as_str()
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
    Ok(incident_change_success(
        command,
        "succeeded",
        "execution_succeeded",
        command.desired_control_version,
    ))
}

fn valid_incident_transition(
    current_status: &str,
    desired_status: DeliverySecurityIncidentDesiredStatus,
) -> bool {
    matches!(
        (current_status, desired_status),
        ("open", DeliverySecurityIncidentDesiredStatus::Acknowledged)
            | ("open", DeliverySecurityIncidentDesiredStatus::Resolved)
            | (
                "acknowledged",
                DeliverySecurityIncidentDesiredStatus::Resolved
            )
    )
}

async fn incident_state(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeliverySecurityIncidentChangeCommand,
    lock: bool,
) -> Result<Option<(String, i32)>, sqlx::Error> {
    let sql = if lock {
        "SELECT status, control_version
         FROM ai_delivery_security_incidents
         WHERE incident_id = $1 AND tenant_id = $2 AND workspace_id = $3
           AND environment = $4
         FOR UPDATE"
    } else {
        "SELECT status, control_version
         FROM ai_delivery_security_incidents
         WHERE incident_id = $1 AND tenant_id = $2 AND workspace_id = $3
           AND environment = $4"
    };
    Ok(sqlx::query(sql)
        .bind(&command.incident_id)
        .bind(&command.tenant_id)
        .bind(&command.workspace_id)
        .bind(&command.environment)
        .fetch_optional(&mut **transaction)
        .await?
        .map(|row| (row.get("status"), row.get("control_version"))))
}

async fn matching_request_status(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeliverySecurityIncidentChangeCommand,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT status FROM ai_transparency_change_requests
         WHERE change_request_id = $1 AND operation = $2 AND target_type = $3
           AND target_id = $4 AND target_scope_key = $5 AND request_digest = $6
           AND expected_target_version = $7 AND desired_next_version = $8",
    )
    .bind(&command.change_request_id)
    .bind(command.desired_status.operation())
    .bind(DELIVERY_SECURITY_INCIDENT_TARGET_TYPE)
    .bind(&command.incident_id)
    .bind(&command.target_scope_key)
    .bind(&command.request_digest)
    .bind(command.expected_control_version)
    .bind(command.desired_control_version)
    .fetch_optional(&mut **transaction)
    .await
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

async fn snapshot_matches_scope(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot_id: &str,
    role: &str,
    tenant_id: &str,
    workspace_id: &str,
    environment: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_actor_role_snapshots
            WHERE actor_role_snapshot_id = $1 AND role = $2
              AND tenant_id = $3 AND workspace_id = $4 AND environment = $5
              AND source_expires_at > NOW()
         )",
    )
    .bind(snapshot_id)
    .bind(role)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(environment)
    .fetch_one(&mut **transaction)
    .await
}

#[allow(clippy::too_many_arguments)]
async fn insert_incident_audit(
    transaction: &mut Transaction<'_, Postgres>,
    incident_id: &str,
    event_type: &str,
    actor_snapshot_id: &str,
    change_request_id: Option<&str>,
    summary_id: Option<&str>,
    severity: &str,
    status: &str,
    control_version: i32,
    reason_code: &str,
    details: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_delivery_security_incident_audit_events (
            incident_audit_event_id, incident_id, event_type, actor_snapshot_id,
            change_request_id, summary_id, severity, status, control_version,
            reason_code, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,NOW())",
    )
    .bind(format!(
        "delivery-security-incident-audit-{}",
        Uuid::new_v4()
    ))
    .bind(incident_id)
    .bind(event_type)
    .bind(actor_snapshot_id)
    .bind(change_request_id)
    .bind(summary_id)
    .bind(severity)
    .bind(status)
    .bind(control_version)
    .bind(reason_code)
    .bind(details)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_change_audit(
    transaction: &mut Transaction<'_, Postgres>,
    command: &DeliverySecurityIncidentChangeCommand,
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
    .bind(DELIVERY_SECURITY_INCIDENT_TARGET_TYPE)
    .bind(&command.incident_id)
    .bind(target_version_before)
    .bind(target_version_after)
    .bind(reason_code)
    .bind(&command.request_digest)
    .bind(details)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_cleanup_runner_audit(
    transaction: &mut Transaction<'_, Postgres>,
    schedule_id: &str,
    run_id: &str,
    runner_id: &str,
    actor_snapshot_id: &str,
    event_type: &str,
    outcome: &str,
    reason_code: &str,
    deleted_rate_windows: i64,
    deleted_metric_snapshots: i64,
    details: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_delivery_security_cleanup_runner_audit_events (
            runner_audit_event_id, schedule_id, run_id, runner_id, actor_snapshot_id,
            event_type, outcome, reason_code, deleted_rate_windows,
            deleted_metric_snapshots, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,NOW())",
    )
    .bind(format!("delivery-security-runner-audit-{}", Uuid::new_v4()))
    .bind(schedule_id)
    .bind(run_id)
    .bind(runner_id)
    .bind(actor_snapshot_id)
    .bind(event_type)
    .bind(outcome)
    .bind(reason_code)
    .bind(deleted_rate_windows)
    .bind(deleted_metric_snapshots)
    .bind(details)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn desired_incident_state(command: &DeliverySecurityIncidentChangeCommand) -> Value {
    json!({
        "schemaVersion": "hs-ai-delivery-security-incident-desired-state-v1",
        "incidentId": command.incident_id,
        "status": command.desired_status.as_str(),
        "expectedControlVersion": command.expected_control_version,
        "desiredControlVersion": command.desired_control_version
    })
}

fn incident_change_success(
    command: &DeliverySecurityIncidentChangeCommand,
    request_status: &str,
    reason_code: &str,
    control_version: i32,
) -> DeliverySecurityIncidentChangeOutcome {
    DeliverySecurityIncidentChangeOutcome {
        succeeded: true,
        request_status: request_status.to_string(),
        incident_status: if request_status == "succeeded" {
            command.desired_status.as_str().to_string()
        } else {
            String::new()
        },
        reason_code: (reason_code == REASON_IDEMPOTENCY_REPLAY).then(|| reason_code.to_string()),
        control_version,
    }
}

fn incident_change_rejected(
    _command: &DeliverySecurityIncidentChangeCommand,
    reason_code: &str,
    control_version: i32,
) -> DeliverySecurityIncidentChangeOutcome {
    DeliverySecurityIncidentChangeOutcome {
        succeeded: false,
        request_status: "rejected".to_string(),
        incident_status: String::new(),
        reason_code: Some(reason_code.to_string()),
        control_version,
    }
}

fn cleanup_schedule_rejected(reason_code: &str) -> DeliverySecurityCleanupScheduleOutcome {
    DeliverySecurityCleanupScheduleOutcome {
        succeeded: false,
        reason_code: Some(reason_code.to_string()),
        schedule_id: None,
        run_id: None,
        claimed: false,
        deleted_rate_windows: 0,
        deleted_metric_snapshots: 0,
    }
}

fn cleanup_backoff_minutes(failures: i32) -> i64 {
    let exponent = u32::try_from(failures.saturating_sub(1))
        .unwrap_or(0)
        .min(30);
    DELIVERY_SECURITY_CLEANUP_BASE_BACKOFF_MINUTES
        .saturating_mul(1_i64.checked_shl(exponent).unwrap_or(i64::MAX))
        .min(DELIVERY_SECURITY_CLEANUP_MAX_BACKOFF_MINUTES)
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

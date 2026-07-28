use chrono::{Duration, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection, Row};
use uuid::Uuid;

use crate::ai_transparency_change_command::{ActorAuthorizationInput, ChangeCommandPreflight};
use crate::ai_transparency_delivery_security_incident::{
    project_delivery_security_incident, DeliverySecurityIncidentProjectionInput,
};

pub const DELIVERY_RATE_WINDOW_RETENTION_HOURS: i64 = 24;
pub const DELIVERY_SECURITY_METRIC_RETENTION_DAYS: i64 = 90;
pub const DELIVERY_SECURITY_MONITORING_WINDOW_MINUTES: i64 = 15;
pub const DELIVERY_SECURITY_MAX_EXPORT_WINDOW_MINUTES: i64 = 31 * 24 * 60;
pub const DELIVERY_SECURITY_CLEANUP_BATCH_LIMIT: i64 = 1_000;

pub const ALERT_RATE_LIMITED_WARNING_COUNT: i64 = 5;
pub const ALERT_REVOKED_ACCESS_CRITICAL_COUNT: i64 = 3;
pub const ALERT_AVAILABILITY_WARNING_COUNT: i64 = 3;
pub const ALERT_FAILURE_RATIO_WARNING_PERCENT: i64 = 20;
pub const ALERT_FAILURE_RATIO_MIN_ATTEMPTS: i64 = 10;

pub const REASON_SUMMARY_INVALID: &str = "ai_delivery_security_summary_invalid";
pub const REASON_SUMMARY_AUTHORIZED: &str = "ai_delivery_security_summary_authorized";
pub const REASON_CLEANUP_SUCCEEDED: &str = "ai_delivery_rate_limit_cleanup_succeeded";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliverySecuritySummaryMode {
    Monitoring15m,
    AuditExport,
}

impl DeliverySecuritySummaryMode {
    fn as_str(self) -> &'static str {
        match self {
            Self::Monitoring15m => "monitoring_15m",
            Self::AuditExport => "audit_export",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenerateDeliverySecuritySummaryCommand {
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub requester_snapshot_id: String,
    pub requester_token_hash: String,
    pub mode: DeliverySecuritySummaryMode,
    pub window_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySecuritySummary {
    pub summary_id: String,
    pub mode: DeliverySecuritySummaryMode,
    pub window_started_at: String,
    pub window_ended_at: String,
    pub authorization_granted_count: i64,
    pub authorization_revoked_count: i64,
    pub retrieval_claimed_count: i64,
    pub retrieval_succeeded_count: i64,
    pub retrieval_failed_count: i64,
    pub rate_limited_count: i64,
    pub revoked_access_count: i64,
    pub size_limit_count: i64,
    pub content_type_invalid_count: i64,
    pub read_timeout_count: i64,
    pub artifact_unavailable_count: i64,
    pub bridge_rejected_count: i64,
    pub alert_status: String,
    pub alert_codes: Vec<String>,
    pub summary_digest: String,
    pub retention_expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySecuritySummaryOutcome {
    pub succeeded: bool,
    pub reason_code: Option<String>,
    pub summary: Option<DeliverySecuritySummary>,
}

#[derive(Debug, Clone)]
pub struct CleanupDeliverySecurityWindowsCommand {
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySecurityCleanupOutcome {
    pub succeeded: bool,
    pub reason_code: Option<String>,
    pub deleted_rate_windows: i64,
    pub deleted_metric_snapshots: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum DeliverySecurityObservabilityError {
    #[error("PostgreSQL delivery security observability failed: {0}")]
    Postgres(#[from] sqlx::Error),
    #[error("delivery security observability contract failed: {0}")]
    Contract(String),
}

pub async fn execute_postgres_generate_delivery_security_summary(
    connection: &mut PgConnection,
    command: &GenerateDeliverySecuritySummaryCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<DeliverySecuritySummaryOutcome, DeliverySecurityObservabilityError> {
    if !valid_summary_window(command.mode, command.window_minutes) {
        return Ok(summary_rejected(REASON_SUMMARY_INVALID));
    }
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.requester_token_hash,
            required_role: "ai_transparency_readonly_auditor",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: match command.mode {
                DeliverySecuritySummaryMode::Monitoring15m => {
                    "generate_ai_delivery_security_monitoring_summary"
                }
                DeliverySecuritySummaryMode::AuditExport => {
                    "export_ai_delivery_security_audit_summary"
                }
            },
        });
    if !decision.authorized {
        return Ok(summary_rejected(
            decision.reason_code.as_deref().unwrap_or("iam_unavailable"),
        ));
    }
    let mut transaction = connection.begin().await?;
    if !snapshot_matches_scope(
        &mut transaction,
        &command.requester_snapshot_id,
        "ai_transparency_readonly_auditor",
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
    )
    .await?
    {
        transaction.rollback().await?;
        return Ok(summary_rejected(REASON_SUMMARY_INVALID));
    }
    let row = sqlx::query(
        "SELECT
            COUNT(*) FILTER (WHERE audit.event_type = 'authorization_granted')::BIGINT authorization_granted_count,
            COUNT(*) FILTER (WHERE audit.event_type = 'authorization_revoked')::BIGINT authorization_revoked_count,
            COUNT(*) FILTER (WHERE audit.event_type = 'retrieval_claimed')::BIGINT retrieval_claimed_count,
            COUNT(*) FILTER (WHERE audit.event_type = 'retrieval_succeeded')::BIGINT retrieval_succeeded_count,
            COUNT(*) FILTER (WHERE audit.event_type = 'retrieval_failed')::BIGINT retrieval_failed_count,
            COUNT(*) FILTER (WHERE audit.reason_code = 'ai_delivery_retrieval_rate_limited')::BIGINT rate_limited_count,
            COUNT(*) FILTER (WHERE audit.reason_code = 'ai_delivery_authorization_revoked')::BIGINT revoked_access_count,
            COUNT(*) FILTER (WHERE audit.reason_code = 'ai_delivery_retrieval_size_limit_exceeded')::BIGINT size_limit_count,
            COUNT(*) FILTER (WHERE audit.reason_code = 'ai_delivery_retrieval_content_type_invalid')::BIGINT content_type_invalid_count,
            COUNT(*) FILTER (WHERE audit.reason_code = 'ai_delivery_retrieval_read_timeout')::BIGINT read_timeout_count,
            COUNT(*) FILTER (WHERE audit.reason_code = 'ai_delivery_retrieval_artifact_unavailable')::BIGINT artifact_unavailable_count,
            COUNT(*) FILTER (WHERE audit.reason_code = 'ai_delivery_retrieval_bridge_rejected')::BIGINT bridge_rejected_count
         FROM ai_delivery_download_audit_events audit
         JOIN ai_delivery_retrieval_authorizations auth_scope
           ON auth_scope.authorization_id = audit.authorization_id
         WHERE auth_scope.tenant_id = $1
           AND auth_scope.workspace_id = $2
           AND auth_scope.environment = $3
           AND audit.occurred_at >= NOW() - ($4 * INTERVAL '1 minute')
           AND audit.occurred_at <= NOW()",
    )
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(command.window_minutes)
    .fetch_one(&mut *transaction)
    .await?;
    let now = Utc::now();
    let window_started_at = now - Duration::minutes(command.window_minutes);
    let counts = SummaryCounts::from_row(&row);
    let (alert_status, alert_codes) = evaluate_alerts(command.mode, &counts);
    let summary_id = format!("delivery-security-summary-{}", Uuid::new_v4());
    let retention_expires_at = now + Duration::days(DELIVERY_SECURITY_METRIC_RETENTION_DAYS);
    let summary_digest = summary_digest(
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
        command.mode,
        window_started_at,
        now,
        &counts,
        &alert_status,
        &alert_codes,
    );
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
            $1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,
            $20,$21,$22,$23,$24,$25
         )",
    )
    .bind(&summary_id)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(command.mode.as_str())
    .bind(window_started_at)
    .bind(now)
    .bind(counts.authorization_granted)
    .bind(counts.authorization_revoked)
    .bind(counts.retrieval_claimed)
    .bind(counts.retrieval_succeeded)
    .bind(counts.retrieval_failed)
    .bind(counts.rate_limited)
    .bind(counts.revoked_access)
    .bind(counts.size_limit)
    .bind(counts.content_type_invalid)
    .bind(counts.read_timeout)
    .bind(counts.artifact_unavailable)
    .bind(counts.bridge_rejected)
    .bind(&alert_status)
    .bind(json!(alert_codes))
    .bind(&summary_digest)
    .bind(&command.requester_snapshot_id)
    .bind(now)
    .bind(retention_expires_at)
    .execute(&mut *transaction)
    .await?;
    let incident_projection = if command.mode == DeliverySecuritySummaryMode::Monitoring15m {
        project_delivery_security_incident(
            &mut transaction,
            &DeliverySecurityIncidentProjectionInput {
                tenant_id: &command.tenant_id,
                workspace_id: &command.workspace_id,
                environment: &command.environment,
                summary_id: &summary_id,
                summary_digest: &summary_digest,
                severity: &alert_status,
                alert_codes: &alert_codes,
                actor_snapshot_id: &command.requester_snapshot_id,
            },
        )
        .await?
    } else {
        None
    };
    insert_operations_audit(
        &mut transaction,
        command,
        match command.mode {
            DeliverySecuritySummaryMode::Monitoring15m => "delivery_security_summary_generated",
            DeliverySecuritySummaryMode::AuditExport => "delivery_security_audit_summary_exported",
        },
        &command.requester_snapshot_id,
        0,
        0,
        json!({
            "summaryId": summary_id,
            "summaryDigest": summary_digest,
            "windowMinutes": command.window_minutes,
            "aggregateOnly": true,
            "rawAuditExported": false,
            "incidentId": incident_projection.as_ref().map(|incident| &incident.incident_id),
            "incidentEventType": incident_projection.as_ref().map(|incident| &incident.event_type),
            "notificationAdaptersInvoked": false
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(DeliverySecuritySummaryOutcome {
        succeeded: true,
        reason_code: None,
        summary: Some(DeliverySecuritySummary {
            summary_id,
            mode: command.mode,
            window_started_at: window_started_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            window_ended_at: now.to_rfc3339_opts(SecondsFormat::Millis, true),
            authorization_granted_count: counts.authorization_granted,
            authorization_revoked_count: counts.authorization_revoked,
            retrieval_claimed_count: counts.retrieval_claimed,
            retrieval_succeeded_count: counts.retrieval_succeeded,
            retrieval_failed_count: counts.retrieval_failed,
            rate_limited_count: counts.rate_limited,
            revoked_access_count: counts.revoked_access,
            size_limit_count: counts.size_limit,
            content_type_invalid_count: counts.content_type_invalid,
            read_timeout_count: counts.read_timeout,
            artifact_unavailable_count: counts.artifact_unavailable,
            bridge_rejected_count: counts.bridge_rejected,
            alert_status,
            alert_codes,
            summary_digest,
            retention_expires_at: retention_expires_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }),
    })
}

pub async fn execute_postgres_cleanup_delivery_security_windows(
    connection: &mut PgConnection,
    command: &CleanupDeliverySecurityWindowsCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<DeliverySecurityCleanupOutcome, DeliverySecurityObservabilityError> {
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.executor_token_hash,
            required_role: "system_executor",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "cleanup_ai_delivery_security_windows",
        });
    if !decision.authorized {
        return Ok(cleanup_rejected(
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
        return Ok(cleanup_rejected(REASON_SUMMARY_INVALID));
    }
    let deleted_rate_windows: i64 = sqlx::query_scalar(
        "WITH candidates AS (
            SELECT rate_window.license_id, rate_window.window_started_at
            FROM ai_delivery_download_rate_limit_windows rate_window
            JOIN ai_transparency_licenses license
              ON license.license_id = rate_window.license_id
            WHERE license.tenant_id = $1
              AND license.workspace_id = $2
              AND license.environment = $3
              AND rate_window.window_started_at
                  < date_trunc('minute', NOW() - INTERVAL '24 hours')
            ORDER BY rate_window.window_started_at
            LIMIT $4
            FOR UPDATE OF rate_window SKIP LOCKED
         ),
         deleted AS (
            DELETE FROM ai_delivery_download_rate_limit_windows rate_window
            USING candidates
            WHERE rate_window.license_id = candidates.license_id
              AND rate_window.window_started_at = candidates.window_started_at
            RETURNING 1
         )
         SELECT COUNT(*)::BIGINT FROM deleted",
    )
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(DELIVERY_SECURITY_CLEANUP_BATCH_LIMIT)
    .fetch_one(&mut *transaction)
    .await?;
    let deleted_metric_snapshots: i64 = sqlx::query_scalar(
        "WITH candidates AS (
            SELECT summary_id
            FROM ai_delivery_security_observability_snapshots
            WHERE tenant_id = $1 AND workspace_id = $2 AND environment = $3
              AND retention_expires_at <= NOW()
            ORDER BY retention_expires_at
            LIMIT $4
            FOR UPDATE SKIP LOCKED
         ),
         deleted AS (
            DELETE FROM ai_delivery_security_observability_snapshots summary
            USING candidates
            WHERE summary.summary_id = candidates.summary_id
            RETURNING 1
         )
         SELECT COUNT(*)::BIGINT FROM deleted",
    )
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(DELIVERY_SECURITY_CLEANUP_BATCH_LIMIT)
    .fetch_one(&mut *transaction)
    .await?;
    let audit_command = GenerateDeliverySecuritySummaryCommand {
        tenant_id: command.tenant_id.clone(),
        workspace_id: command.workspace_id.clone(),
        environment: command.environment.clone(),
        requester_snapshot_id: command.executor_snapshot_id.clone(),
        requester_token_hash: String::new(),
        mode: DeliverySecuritySummaryMode::Monitoring15m,
        window_minutes: DELIVERY_SECURITY_MONITORING_WINDOW_MINUTES,
    };
    insert_operations_audit(
        &mut transaction,
        &audit_command,
        "delivery_rate_limit_cleanup",
        &command.executor_snapshot_id,
        deleted_rate_windows as i32,
        deleted_metric_snapshots as i32,
        json!({
            "rateWindowRetentionHours": DELIVERY_RATE_WINDOW_RETENTION_HOURS,
            "metricRetentionDays": DELIVERY_SECURITY_METRIC_RETENTION_DAYS,
            "batchLimit": DELIVERY_SECURITY_CLEANUP_BATCH_LIMIT
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(DeliverySecurityCleanupOutcome {
        succeeded: true,
        reason_code: None,
        deleted_rate_windows,
        deleted_metric_snapshots,
    })
}

#[derive(Debug, Clone)]
struct SummaryCounts {
    authorization_granted: i64,
    authorization_revoked: i64,
    retrieval_claimed: i64,
    retrieval_succeeded: i64,
    retrieval_failed: i64,
    rate_limited: i64,
    revoked_access: i64,
    size_limit: i64,
    content_type_invalid: i64,
    read_timeout: i64,
    artifact_unavailable: i64,
    bridge_rejected: i64,
}

impl SummaryCounts {
    fn from_row(row: &sqlx::postgres::PgRow) -> Self {
        Self {
            authorization_granted: row.get("authorization_granted_count"),
            authorization_revoked: row.get("authorization_revoked_count"),
            retrieval_claimed: row.get("retrieval_claimed_count"),
            retrieval_succeeded: row.get("retrieval_succeeded_count"),
            retrieval_failed: row.get("retrieval_failed_count"),
            rate_limited: row.get("rate_limited_count"),
            revoked_access: row.get("revoked_access_count"),
            size_limit: row.get("size_limit_count"),
            content_type_invalid: row.get("content_type_invalid_count"),
            read_timeout: row.get("read_timeout_count"),
            artifact_unavailable: row.get("artifact_unavailable_count"),
            bridge_rejected: row.get("bridge_rejected_count"),
        }
    }
}

fn valid_summary_window(mode: DeliverySecuritySummaryMode, window_minutes: i64) -> bool {
    match mode {
        DeliverySecuritySummaryMode::Monitoring15m => {
            window_minutes == DELIVERY_SECURITY_MONITORING_WINDOW_MINUTES
        }
        DeliverySecuritySummaryMode::AuditExport => (DELIVERY_SECURITY_MONITORING_WINDOW_MINUTES
            ..=DELIVERY_SECURITY_MAX_EXPORT_WINDOW_MINUTES)
            .contains(&window_minutes),
    }
}

fn evaluate_alerts(
    mode: DeliverySecuritySummaryMode,
    counts: &SummaryCounts,
) -> (String, Vec<String>) {
    if mode == DeliverySecuritySummaryMode::AuditExport {
        return ("not_evaluated".to_string(), Vec::new());
    }
    let mut warnings = Vec::new();
    let mut criticals = Vec::new();
    if counts.size_limit + counts.content_type_invalid + counts.bridge_rejected >= 1 {
        criticals.push("delivery_integrity_failure".to_string());
    }
    if counts.revoked_access >= ALERT_REVOKED_ACCESS_CRITICAL_COUNT {
        criticals.push("revoked_authorization_access_burst".to_string());
    }
    if counts.rate_limited >= ALERT_RATE_LIMITED_WARNING_COUNT {
        warnings.push("delivery_rate_limit_pressure".to_string());
    }
    if counts.read_timeout + counts.artifact_unavailable >= ALERT_AVAILABILITY_WARNING_COUNT {
        warnings.push("delivery_artifact_availability_degraded".to_string());
    }
    let attempts = counts.retrieval_succeeded + counts.retrieval_failed;
    if attempts >= ALERT_FAILURE_RATIO_MIN_ATTEMPTS
        && counts.retrieval_failed * 100 >= attempts * ALERT_FAILURE_RATIO_WARNING_PERCENT
    {
        warnings.push("delivery_failure_ratio_elevated".to_string());
    }
    if !criticals.is_empty() {
        criticals.extend(warnings);
        ("critical".to_string(), criticals)
    } else if !warnings.is_empty() {
        ("warning".to_string(), warnings)
    } else {
        ("ok".to_string(), Vec::new())
    }
}

fn summary_digest(
    tenant_id: &str,
    workspace_id: &str,
    environment: &str,
    mode: DeliverySecuritySummaryMode,
    window_started_at: chrono::DateTime<Utc>,
    window_ended_at: chrono::DateTime<Utc>,
    counts: &SummaryCounts,
    alert_status: &str,
    alert_codes: &[String],
) -> String {
    sha256_hex(
        serde_json::to_string(&json!([
            tenant_id,
            workspace_id,
            environment,
            mode.as_str(),
            window_started_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            window_ended_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            counts.authorization_granted,
            counts.authorization_revoked,
            counts.retrieval_claimed,
            counts.retrieval_succeeded,
            counts.retrieval_failed,
            counts.rate_limited,
            counts.revoked_access,
            counts.size_limit,
            counts.content_type_invalid,
            counts.read_timeout,
            counts.artifact_unavailable,
            counts.bridge_rejected,
            alert_status,
            alert_codes
        ]))
        .expect("serializable delivery security summary")
        .as_bytes(),
    )
}

async fn snapshot_matches_scope(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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

async fn insert_operations_audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    command: &GenerateDeliverySecuritySummaryCommand,
    operation: &str,
    actor_snapshot_id: &str,
    affected_rate_windows: i32,
    affected_metric_snapshots: i32,
    details: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_delivery_security_operations_audit_events (
            operation_audit_event_id, tenant_id, workspace_id, environment,
            operation, outcome, actor_snapshot_id, affected_rate_windows,
            affected_metric_snapshots, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,'succeeded',$6,$7,$8,$9,NOW())",
    )
    .bind(format!("delivery-security-operation-{}", Uuid::new_v4()))
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(operation)
    .bind(actor_snapshot_id)
    .bind(affected_rate_windows)
    .bind(affected_metric_snapshots)
    .bind(details)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn summary_rejected(reason_code: &str) -> DeliverySecuritySummaryOutcome {
    DeliverySecuritySummaryOutcome {
        succeeded: false,
        reason_code: Some(reason_code.to_string()),
        summary: None,
    }
}

fn cleanup_rejected(reason_code: &str) -> DeliverySecurityCleanupOutcome {
    DeliverySecurityCleanupOutcome {
        succeeded: false,
        reason_code: Some(reason_code.to_string()),
        deleted_rate_windows: 0,
        deleted_metric_snapshots: 0,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

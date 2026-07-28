use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::ai_transparency_change_command::{ActorAuthorizationInput, ChangeCommandPreflight};

pub const NOTIFICATION_OUTBOX_LEASE_MINUTES: i64 = 5;
pub const NOTIFICATION_OUTBOX_MAX_CLAIM: i32 = 100;
pub const INCIDENT_LIST_MAX_LIMIT: i32 = 100;

pub const REASON_INCIDENT_INSPECTED: &str = "ai_delivery_security_incident_inspected";
pub const REASON_INCIDENT_LISTED: &str = "ai_delivery_security_incidents_listed";
pub const REASON_INCIDENT_NOT_FOUND: &str = "ai_delivery_security_incident_not_found";
pub const REASON_INSPECTION_DENIED: &str = "ai_delivery_security_incident_inspection_denied";
pub const REASON_OUTBOX_ENQUEUED: &str = "ai_delivery_security_notification_enqueued";
pub const REASON_OUTBOX_DEDUPE_REPLAY: &str = "ai_delivery_security_notification_dedupe_replay";
pub const REASON_OUTBOX_CLAIMED: &str = "ai_delivery_security_notification_claimed";
pub const REASON_OUTBOX_EXPIRED_LEASE_RECLAIMED: &str =
    "ai_delivery_security_notification_expired_lease_reclaimed";
pub const REASON_OUTBOX_REPLAY_SCHEDULED: &str =
    "ai_delivery_security_notification_replay_scheduled";
pub const REASON_OUTBOX_REPLAY_IDEMPOTENCY: &str =
    "ai_delivery_security_notification_replay_idempotency";
pub const REASON_OUTBOX_INVALID: &str = "ai_delivery_security_notification_invalid";
pub const REASON_OUTBOX_LEASE_CONFLICT: &str = "ai_delivery_security_notification_lease_conflict";

#[derive(Debug, Clone)]
pub struct InspectDeliverySecurityIncidentCommand {
    pub incident_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub actor_snapshot_id: String,
    pub actor_token_hash: String,
}

#[derive(Debug, Clone)]
pub struct ListDeliverySecurityIncidentsCommand {
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub actor_snapshot_id: String,
    pub actor_token_hash: String,
    pub status: Option<String>,
    pub limit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySecurityIncidentView {
    pub incident_id: String,
    pub severity: String,
    pub status: String,
    pub alert_codes: Vec<String>,
    pub occurrence_count: i64,
    pub first_summary_digest: String,
    pub latest_summary_digest: String,
    pub control_version: i32,
    pub pending_notification_count: i64,
    pub opened_at: String,
    pub acknowledged_at: Option<String>,
    pub resolved_at: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySecurityIncidentInspectionOutcome {
    pub succeeded: bool,
    pub reason_code: Option<String>,
    pub incident: Option<DeliverySecurityIncidentView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DeliverySecurityIncidentListOutcome {
    pub succeeded: bool,
    pub reason_code: Option<String>,
    pub incidents: Vec<DeliverySecurityIncidentView>,
}

#[derive(Debug, Clone)]
pub struct EnqueueDeliverySecurityNotificationInput<'a> {
    pub incident_id: &'a str,
    pub tenant_id: &'a str,
    pub workspace_id: &'a str,
    pub environment: &'a str,
    pub event_type: &'a str,
    pub priority: &'a str,
    pub incident_status: &'a str,
    pub severity: &'a str,
    pub alert_codes: &'a [String],
    pub occurrence_count: i64,
    pub control_version: i32,
    pub actor_snapshot_id: &'a str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationOutboxEnqueueOutcome {
    pub notification_id: String,
    pub dedupe_key: String,
    pub payload_digest: String,
    pub replayed: bool,
}

#[derive(Debug, Clone)]
pub struct ClaimDeliverySecurityNotificationsCommand {
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
    pub runner_id: String,
    pub limit: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimedDeliverySecurityNotification {
    pub notification_id: String,
    pub incident_id: String,
    pub event_type: String,
    pub priority: String,
    pub payload: Value,
    pub payload_digest: String,
    pub delivery_attempt_count: i32,
    pub replay_count: i32,
    pub recovery_count: i32,
    pub lease_expires_at: String,
    pub reclaimed_expired_lease: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClaimDeliverySecurityNotificationsOutcome {
    pub succeeded: bool,
    pub reason_code: Option<String>,
    pub notifications: Vec<ClaimedDeliverySecurityNotification>,
}

#[derive(Debug, Clone)]
pub struct ReplayDeliverySecurityNotificationCommand {
    pub notification_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
    pub lease_owner: String,
    pub idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ReplayDeliverySecurityNotificationOutcome {
    pub succeeded: bool,
    pub replayed: bool,
    pub reason_code: Option<String>,
    pub notification_id: String,
    pub status: String,
    pub replay_count: i32,
}

#[derive(Debug, thiserror::Error)]
pub enum DeliverySecurityNotificationError {
    #[error("PostgreSQL delivery security notification command failed: {0}")]
    Postgres(#[from] sqlx::Error),
}

pub async fn inspect_postgres_delivery_security_incident(
    connection: &mut PgConnection,
    command: &InspectDeliverySecurityIncidentCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<DeliverySecurityIncidentInspectionOutcome, DeliverySecurityNotificationError> {
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.actor_token_hash,
            required_role: "ai_transparency_readonly_auditor",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "inspect_delivery_security_incident",
        });
    let mut transaction = connection.begin().await?;
    if !decision.authorized
        || !snapshot_matches_scope(
            &mut transaction,
            &command.actor_snapshot_id,
            "ai_transparency_readonly_auditor",
            &command.tenant_id,
            &command.workspace_id,
            &command.environment,
        )
        .await?
    {
        insert_inspection_audit(
            &mut transaction,
            &command.tenant_id,
            &command.workspace_id,
            &command.environment,
            Some(&command.incident_id),
            "inspect_delivery_security_incident",
            &command.actor_snapshot_id,
            "denied",
            decision
                .reason_code
                .as_deref()
                .unwrap_or(REASON_INSPECTION_DENIED),
            0,
            json!({"scopeBound": true}),
        )
        .await?;
        transaction.commit().await?;
        return Ok(DeliverySecurityIncidentInspectionOutcome {
            succeeded: false,
            reason_code: Some(
                decision
                    .reason_code
                    .unwrap_or_else(|| REASON_INSPECTION_DENIED.to_string()),
            ),
            incident: None,
        });
    }
    let row = sqlx::query(
        "SELECT incident.incident_id, incident.severity, incident.status,
                incident.alert_codes_json, incident.occurrence_count,
                incident.first_summary_digest, incident.latest_summary_digest,
                incident.control_version, incident.opened_at, incident.acknowledged_at,
                incident.resolved_at, incident.updated_at,
                (
                    SELECT COUNT(*)::BIGINT
                    FROM ai_delivery_security_notification_outbox outbox
                    WHERE outbox.incident_id = incident.incident_id
                      AND outbox.status IN ('pending','leased','retry_scheduled')
                ) pending_notification_count
         FROM ai_delivery_security_incidents incident
         WHERE incident.incident_id = $1 AND incident.tenant_id = $2
           AND incident.workspace_id = $3 AND incident.environment = $4",
    )
    .bind(&command.incident_id)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .fetch_optional(&mut *transaction)
    .await?;
    let incident = row.map(incident_view_from_row);
    insert_inspection_audit(
        &mut transaction,
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
        Some(&command.incident_id),
        "inspect_delivery_security_incident",
        &command.actor_snapshot_id,
        if incident.is_some() {
            "succeeded"
        } else {
            "not_found"
        },
        if incident.is_some() {
            REASON_INCIDENT_INSPECTED
        } else {
            REASON_INCIDENT_NOT_FOUND
        },
        i32::from(incident.is_some()),
        json!({"aggregateOnly": true}),
    )
    .await?;
    transaction.commit().await?;
    Ok(DeliverySecurityIncidentInspectionOutcome {
        succeeded: incident.is_some(),
        reason_code: incident
            .is_none()
            .then(|| REASON_INCIDENT_NOT_FOUND.to_string()),
        incident,
    })
}

pub async fn list_postgres_delivery_security_incidents(
    connection: &mut PgConnection,
    command: &ListDeliverySecurityIncidentsCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<DeliverySecurityIncidentListOutcome, DeliverySecurityNotificationError> {
    let invalid_request = command.limit < 1
        || command.limit > INCIDENT_LIST_MAX_LIMIT
        || command
            .status
            .as_deref()
            .is_some_and(|status| !matches!(status, "open" | "acknowledged" | "resolved"));
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.actor_token_hash,
            required_role: "ai_transparency_readonly_auditor",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "list_delivery_security_incidents",
        });
    let mut transaction = connection.begin().await?;
    if invalid_request
        || !decision.authorized
        || !snapshot_matches_scope(
            &mut transaction,
            &command.actor_snapshot_id,
            "ai_transparency_readonly_auditor",
            &command.tenant_id,
            &command.workspace_id,
            &command.environment,
        )
        .await?
    {
        insert_inspection_audit(
            &mut transaction,
            &command.tenant_id,
            &command.workspace_id,
            &command.environment,
            None,
            "list_delivery_security_incidents",
            &command.actor_snapshot_id,
            "denied",
            decision
                .reason_code
                .as_deref()
                .unwrap_or(REASON_INSPECTION_DENIED),
            0,
            json!({"scopeBound": true}),
        )
        .await?;
        transaction.commit().await?;
        return Ok(DeliverySecurityIncidentListOutcome {
            succeeded: false,
            reason_code: Some(
                decision
                    .reason_code
                    .unwrap_or_else(|| REASON_INSPECTION_DENIED.to_string()),
            ),
            incidents: Vec::new(),
        });
    }
    let rows = sqlx::query(
        "SELECT incident.incident_id, incident.severity, incident.status,
                incident.alert_codes_json, incident.occurrence_count,
                incident.first_summary_digest, incident.latest_summary_digest,
                incident.control_version, incident.opened_at, incident.acknowledged_at,
                incident.resolved_at, incident.updated_at,
                (
                    SELECT COUNT(*)::BIGINT
                    FROM ai_delivery_security_notification_outbox outbox
                    WHERE outbox.incident_id = incident.incident_id
                      AND outbox.status IN ('pending','leased','retry_scheduled')
                ) pending_notification_count
         FROM ai_delivery_security_incidents incident
         WHERE incident.tenant_id = $1 AND incident.workspace_id = $2
           AND incident.environment = $3
           AND ($4::TEXT IS NULL OR incident.status = $4)
         ORDER BY incident.updated_at DESC, incident.incident_id ASC
         LIMIT $5",
    )
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(command.status.as_deref())
    .bind(command.limit)
    .fetch_all(&mut *transaction)
    .await?;
    let incidents = rows
        .into_iter()
        .map(incident_view_from_row)
        .collect::<Vec<_>>();
    insert_inspection_audit(
        &mut transaction,
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
        None,
        "list_delivery_security_incidents",
        &command.actor_snapshot_id,
        "succeeded",
        REASON_INCIDENT_LISTED,
        i32::try_from(incidents.len()).unwrap_or(INCIDENT_LIST_MAX_LIMIT),
        json!({"statusFilter": command.status, "limit": command.limit, "aggregateOnly": true}),
    )
    .await?;
    transaction.commit().await?;
    Ok(DeliverySecurityIncidentListOutcome {
        succeeded: true,
        reason_code: None,
        incidents,
    })
}

pub async fn enqueue_delivery_security_notification(
    transaction: &mut Transaction<'_, Postgres>,
    input: &EnqueueDeliverySecurityNotificationInput<'_>,
) -> Result<NotificationOutboxEnqueueOutcome, sqlx::Error> {
    let mut alert_codes = input.alert_codes.to_vec();
    alert_codes.sort();
    alert_codes.dedup();
    let dedupe_key = sha256_hex(
        serde_json::to_string(&json!([
            input.incident_id,
            input.event_type,
            input.control_version
        ]))
        .expect("delivery security notification dedupe JSON")
        .as_bytes(),
    );
    let payload = json!({
        "schemaVersion": "hs-ai-delivery-security-notification-payload-v1",
        "incidentId": input.incident_id,
        "scope": {
            "tenantId": input.tenant_id,
            "workspaceId": input.workspace_id,
            "environment": input.environment
        },
        "eventType": input.event_type,
        "priority": input.priority,
        "incidentStatus": input.incident_status,
        "severity": input.severity,
        "alertCodes": alert_codes,
        "occurrenceCount": input.occurrence_count,
        "controlVersion": input.control_version
    });
    let payload_digest = sha256_hex(
        serde_json::to_string(&json!([
            "hs-ai-delivery-security-notification-payload-v1",
            input.incident_id,
            input.tenant_id,
            input.workspace_id,
            input.environment,
            input.event_type,
            input.priority,
            input.incident_status,
            input.severity,
            alert_codes,
            input.occurrence_count,
            input.control_version
        ]))
        .expect("delivery security notification payload JSON")
        .as_bytes(),
    );
    let candidate_id = format!("delivery-security-notification-{}", Uuid::new_v4());
    let inserted = sqlx::query(
        "INSERT INTO ai_delivery_security_notification_outbox (
            notification_id, incident_id, tenant_id, workspace_id, environment,
            event_type, priority, dedupe_key, payload_json, payload_digest,
            status, available_at, last_reason_code, created_at, updated_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,'pending',NOW(),$11,NOW(),NOW())
         ON CONFLICT(dedupe_key) DO NOTHING
         RETURNING notification_id",
    )
    .bind(&candidate_id)
    .bind(input.incident_id)
    .bind(input.tenant_id)
    .bind(input.workspace_id)
    .bind(input.environment)
    .bind(input.event_type)
    .bind(input.priority)
    .bind(&dedupe_key)
    .bind(&payload)
    .bind(&payload_digest)
    .bind(REASON_OUTBOX_ENQUEUED)
    .fetch_optional(&mut **transaction)
    .await?;
    let (notification_id, replayed): (String, bool) = if let Some(row) = inserted {
        (row.get::<String, _>("notification_id"), false)
    } else {
        (
            sqlx::query_scalar::<_, String>(
                "SELECT notification_id
                 FROM ai_delivery_security_notification_outbox
                 WHERE dedupe_key = $1",
            )
            .bind(&dedupe_key)
            .fetch_one(&mut **transaction)
            .await?,
            true,
        )
    };
    let row = sqlx::query(
        "SELECT delivery_attempt_count, replay_count, status
         FROM ai_delivery_security_notification_outbox WHERE notification_id = $1",
    )
    .bind(&notification_id)
    .fetch_one(&mut **transaction)
    .await?;
    insert_outbox_audit(
        transaction,
        &notification_id,
        input.incident_id,
        if replayed {
            "dedupe_replay"
        } else {
            "enqueued"
        },
        input.actor_snapshot_id,
        None,
        row.get("status"),
        if replayed {
            REASON_OUTBOX_DEDUPE_REPLAY
        } else {
            REASON_OUTBOX_ENQUEUED
        },
        row.get("delivery_attempt_count"),
        row.get("replay_count"),
        json!({
            "dedupeKey": dedupe_key,
            "payloadDigest": payload_digest,
            "providerConfigured": false,
            "deliveryClaimed": false
        }),
    )
    .await?;
    Ok(NotificationOutboxEnqueueOutcome {
        notification_id,
        dedupe_key,
        payload_digest,
        replayed,
    })
}

pub async fn claim_postgres_delivery_security_notifications(
    connection: &mut PgConnection,
    command: &ClaimDeliverySecurityNotificationsCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<ClaimDeliverySecurityNotificationsOutcome, DeliverySecurityNotificationError> {
    if command.limit < 1 || command.limit > NOTIFICATION_OUTBOX_MAX_CLAIM {
        return Ok(claim_rejected(REASON_OUTBOX_INVALID));
    }
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.executor_token_hash,
            required_role: "system_executor",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "claim_ai_delivery_security_notification_outbox",
        });
    if !decision.authorized {
        return Ok(claim_rejected(
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
        return Ok(claim_rejected(REASON_OUTBOX_INVALID));
    }
    let rows = sqlx::query(
        "WITH candidates AS (
            SELECT notification_id, status previous_status
            FROM ai_delivery_security_notification_outbox
            WHERE tenant_id = $1 AND workspace_id = $2 AND environment = $3
              AND (
                (status IN ('pending','retry_scheduled') AND available_at <= NOW())
                OR
                (status = 'leased' AND lease_expires_at <= NOW())
              )
            ORDER BY available_at ASC, created_at ASC
            LIMIT $4
            FOR UPDATE SKIP LOCKED
         )
         UPDATE ai_delivery_security_notification_outbox outbox
         SET status = 'leased', lease_owner = $5,
             lease_expires_at = NOW() + INTERVAL '5 minutes',
             delivery_attempt_count = outbox.delivery_attempt_count + 1,
             recovery_count = outbox.recovery_count
                 + CASE WHEN candidates.previous_status = 'leased' THEN 1 ELSE 0 END,
             last_reason_code = $6, updated_at = NOW()
         FROM candidates
         WHERE outbox.notification_id = candidates.notification_id
         RETURNING outbox.notification_id, outbox.incident_id, outbox.event_type,
                   outbox.priority, outbox.payload_json, outbox.payload_digest,
                   outbox.delivery_attempt_count, outbox.replay_count,
                   outbox.recovery_count,
                   outbox.lease_expires_at, candidates.previous_status",
    )
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(command.limit)
    .bind(&command.runner_id)
    .bind(REASON_OUTBOX_CLAIMED)
    .fetch_all(&mut *transaction)
    .await?;
    let mut notifications = Vec::with_capacity(rows.len());
    for row in rows {
        let previous_status: String = row.get("previous_status");
        let reclaimed_expired_lease = previous_status == "leased";
        let notification = ClaimedDeliverySecurityNotification {
            notification_id: row.get("notification_id"),
            incident_id: row.get("incident_id"),
            event_type: row.get("event_type"),
            priority: row.get("priority"),
            payload: row.get("payload_json"),
            payload_digest: row.get("payload_digest"),
            delivery_attempt_count: row.get("delivery_attempt_count"),
            replay_count: row.get("replay_count"),
            recovery_count: row.get("recovery_count"),
            lease_expires_at: row
                .get::<chrono::DateTime<chrono::Utc>, _>("lease_expires_at")
                .to_rfc3339(),
            reclaimed_expired_lease,
        };
        insert_outbox_audit(
            &mut transaction,
            &notification.notification_id,
            &notification.incident_id,
            if reclaimed_expired_lease {
                "expired_lease_reclaimed"
            } else {
                "claimed"
            },
            &command.executor_snapshot_id,
            Some(&command.runner_id),
            "leased",
            if reclaimed_expired_lease {
                REASON_OUTBOX_EXPIRED_LEASE_RECLAIMED
            } else {
                REASON_OUTBOX_CLAIMED
            },
            notification.delivery_attempt_count,
            notification.replay_count,
            json!({
                "leaseMinutes": NOTIFICATION_OUTBOX_LEASE_MINUTES,
                "providerInvoked": false
            }),
        )
        .await?;
        notifications.push(notification);
    }
    transaction.commit().await?;
    Ok(ClaimDeliverySecurityNotificationsOutcome {
        succeeded: true,
        reason_code: None,
        notifications,
    })
}

pub async fn replay_postgres_delivery_security_notification(
    connection: &mut PgConnection,
    command: &ReplayDeliverySecurityNotificationCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<ReplayDeliverySecurityNotificationOutcome, DeliverySecurityNotificationError> {
    if command.idempotency_key.trim().is_empty() || command.lease_owner.trim().is_empty() {
        return Ok(replay_rejected(command, REASON_OUTBOX_INVALID, 0));
    }
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: &command.executor_token_hash,
            required_role: "system_executor",
            tenant_id: &command.tenant_id,
            workspace_id: &command.workspace_id,
            environment: &command.environment,
            operation: "replay_ai_delivery_security_notification_outbox",
        });
    if !decision.authorized {
        return Ok(replay_rejected(
            command,
            decision.reason_code.as_deref().unwrap_or("iam_unavailable"),
            0,
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
        return Ok(replay_rejected(command, REASON_OUTBOX_INVALID, 0));
    }
    let row = sqlx::query(
        "SELECT incident_id, status, lease_owner, delivery_attempt_count,
                replay_count, last_replay_idempotency_key,
                COALESCE(lease_expires_at > NOW(), FALSE) lease_valid
         FROM ai_delivery_security_notification_outbox
         WHERE notification_id = $1 AND tenant_id = $2 AND workspace_id = $3
           AND environment = $4
         FOR UPDATE",
    )
    .bind(&command.notification_id)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(row) = row else {
        transaction.rollback().await?;
        return Ok(replay_rejected(command, REASON_OUTBOX_INVALID, 0));
    };
    let incident_id: String = row.get("incident_id");
    let status: String = row.get("status");
    let replay_count: i32 = row.get("replay_count");
    let delivery_attempt_count: i32 = row.get("delivery_attempt_count");
    let last_replay_idempotency_key: Option<String> = row.get("last_replay_idempotency_key");
    if status == "retry_scheduled"
        && last_replay_idempotency_key.as_deref() == Some(command.idempotency_key.as_str())
    {
        insert_outbox_audit(
            &mut transaction,
            &command.notification_id,
            &incident_id,
            "replay_idempotency_replay",
            &command.executor_snapshot_id,
            Some(&command.lease_owner),
            "retry_scheduled",
            REASON_OUTBOX_REPLAY_IDEMPOTENCY,
            delivery_attempt_count,
            replay_count,
            json!({"providerInvoked": false}),
        )
        .await?;
        transaction.commit().await?;
        return Ok(ReplayDeliverySecurityNotificationOutcome {
            succeeded: true,
            replayed: true,
            reason_code: Some(REASON_OUTBOX_REPLAY_IDEMPOTENCY.to_string()),
            notification_id: command.notification_id.clone(),
            status,
            replay_count,
        });
    }
    if status != "leased"
        || !row.get::<bool, _>("lease_valid")
        || row.get::<Option<String>, _>("lease_owner").as_deref()
            != Some(command.lease_owner.as_str())
    {
        transaction.rollback().await?;
        return Ok(replay_rejected(
            command,
            REASON_OUTBOX_LEASE_CONFLICT,
            replay_count,
        ));
    }
    let updated = sqlx::query(
        "UPDATE ai_delivery_security_notification_outbox
         SET status = 'retry_scheduled', available_at = NOW(),
             lease_owner = NULL, lease_expires_at = NULL,
             replay_count = replay_count + 1,
             last_replay_idempotency_key = $1,
             last_reason_code = $2, updated_at = NOW()
         WHERE notification_id = $3 AND status = 'leased' AND lease_owner = $4
         RETURNING replay_count",
    )
    .bind(&command.idempotency_key)
    .bind(REASON_OUTBOX_REPLAY_SCHEDULED)
    .bind(&command.notification_id)
    .bind(&command.lease_owner)
    .fetch_one(&mut *transaction)
    .await?;
    let replay_count: i32 = updated.get("replay_count");
    insert_outbox_audit(
        &mut transaction,
        &command.notification_id,
        &incident_id,
        "replay_scheduled",
        &command.executor_snapshot_id,
        Some(&command.lease_owner),
        "retry_scheduled",
        REASON_OUTBOX_REPLAY_SCHEDULED,
        delivery_attempt_count,
        replay_count,
        json!({
            "providerInvoked": false,
            "providerReceiptAccepted": false,
            "availableImmediately": true
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(ReplayDeliverySecurityNotificationOutcome {
        succeeded: true,
        replayed: false,
        reason_code: None,
        notification_id: command.notification_id.clone(),
        status: "retry_scheduled".to_string(),
        replay_count,
    })
}

fn incident_view_from_row(row: sqlx::postgres::PgRow) -> DeliverySecurityIncidentView {
    DeliverySecurityIncidentView {
        incident_id: row.get("incident_id"),
        severity: row.get("severity"),
        status: row.get("status"),
        alert_codes: serde_json::from_value(row.get("alert_codes_json")).unwrap_or_default(),
        occurrence_count: row.get("occurrence_count"),
        first_summary_digest: row.get("first_summary_digest"),
        latest_summary_digest: row.get("latest_summary_digest"),
        control_version: row.get("control_version"),
        pending_notification_count: row.get("pending_notification_count"),
        opened_at: row
            .get::<chrono::DateTime<chrono::Utc>, _>("opened_at")
            .to_rfc3339(),
        acknowledged_at: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("acknowledged_at")
            .map(|value| value.to_rfc3339()),
        resolved_at: row
            .get::<Option<chrono::DateTime<chrono::Utc>>, _>("resolved_at")
            .map(|value| value.to_rfc3339()),
        updated_at: row
            .get::<chrono::DateTime<chrono::Utc>, _>("updated_at")
            .to_rfc3339(),
    }
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
async fn insert_inspection_audit(
    transaction: &mut Transaction<'_, Postgres>,
    tenant_id: &str,
    workspace_id: &str,
    environment: &str,
    incident_id: Option<&str>,
    operation: &str,
    actor_snapshot_id: &str,
    outcome: &str,
    reason_code: &str,
    returned_count: i32,
    details: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_delivery_security_incident_inspection_audit_events (
            inspection_audit_event_id, tenant_id, workspace_id, environment,
            incident_id, operation, actor_snapshot_id, outcome, reason_code,
            returned_count, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,NOW())",
    )
    .bind(format!("delivery-security-inspection-{}", Uuid::new_v4()))
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(environment)
    .bind(incident_id)
    .bind(operation)
    .bind(actor_snapshot_id)
    .bind(outcome)
    .bind(reason_code)
    .bind(returned_count)
    .bind(details)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_outbox_audit(
    transaction: &mut Transaction<'_, Postgres>,
    notification_id: &str,
    incident_id: &str,
    event_type: &str,
    actor_snapshot_id: &str,
    runner_id: Option<&str>,
    outcome: &str,
    reason_code: &str,
    delivery_attempt_count: i32,
    replay_count: i32,
    details: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_delivery_security_notification_outbox_audit_events (
            outbox_audit_event_id, notification_id, incident_id, event_type,
            actor_snapshot_id, runner_id, outcome, reason_code,
            delivery_attempt_count, replay_count, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,NOW())",
    )
    .bind(format!("delivery-security-outbox-audit-{}", Uuid::new_v4()))
    .bind(notification_id)
    .bind(incident_id)
    .bind(event_type)
    .bind(actor_snapshot_id)
    .bind(runner_id)
    .bind(outcome)
    .bind(reason_code)
    .bind(delivery_attempt_count)
    .bind(replay_count)
    .bind(details)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn claim_rejected(reason_code: &str) -> ClaimDeliverySecurityNotificationsOutcome {
    ClaimDeliverySecurityNotificationsOutcome {
        succeeded: false,
        reason_code: Some(reason_code.to_string()),
        notifications: Vec::new(),
    }
}

fn replay_rejected(
    command: &ReplayDeliverySecurityNotificationCommand,
    reason_code: &str,
    replay_count: i32,
) -> ReplayDeliverySecurityNotificationOutcome {
    ReplayDeliverySecurityNotificationOutcome {
        succeeded: false,
        replayed: false,
        reason_code: Some(reason_code.to_string()),
        notification_id: command.notification_id.clone(),
        status: String::new(),
        replay_count,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sqlx::{Connection, PgConnection, Row};
use uuid::Uuid;

use crate::ai_transparency_change_command::{ActorAuthorizationInput, ChangeCommandPreflight};

pub const DESTINATION_POLICY_SCHEMA_VERSION: &str = "hs-ai-delivery-security-destination-policy-v1";
pub const PROVIDER_RECEIPT_SCHEMA_VERSION: &str = "hs-ai-delivery-security-provider-receipt-v1";
pub const NOTIFICATION_DELIVERY_MAX_ATTEMPTS: i32 = 20;
pub const NOTIFICATION_RECEIPT_MAX_LIFETIME_SECONDS: i64 = 900;

pub const REASON_DESTINATION_BOUND: &str = "ai_delivery_security_notification_destination_bound";
pub const REASON_DESTINATION_POLICY_INVALID: &str =
    "ai_delivery_security_notification_destination_policy_invalid";
pub const REASON_PROVIDER_RECEIPT_INVALID: &str =
    "ai_delivery_security_notification_provider_receipt_invalid";
pub const REASON_NOTIFICATION_COMPLETED: &str = "ai_delivery_security_notification_completed";
pub const REASON_COMPLETION_IDEMPOTENCY_REPLAY: &str =
    "ai_delivery_security_notification_completion_idempotency_replay";
pub const REASON_NOTIFICATION_DELIVERY_FAILED: &str =
    "ai_delivery_security_notification_delivery_failed";
pub const REASON_NOTIFICATION_FAILURE_IDEMPOTENCY_REPLAY: &str =
    "ai_delivery_security_notification_failure_idempotency_replay";
pub const REASON_NOTIFICATION_DEAD_LETTERED: &str =
    "ai_delivery_security_notification_dead_lettered";
pub const REASON_NOTIFICATION_RECOVERED: &str = "ai_delivery_security_notification_recovered";
pub const REASON_NOTIFICATION_RECOVERY_IDEMPOTENCY_REPLAY: &str =
    "ai_delivery_security_notification_recovery_idempotency_replay";
pub const REASON_NOTIFICATION_LEASE_CONFLICT: &str =
    "ai_delivery_security_notification_lease_conflict";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDestinationPolicyV1 {
    pub schema_version: String,
    pub policy_id: String,
    pub version: i32,
    pub environment: String,
    pub enabled: bool,
    pub adapter_kind: String,
    pub delivery_mode: String,
    pub destination_ref: String,
    pub event_types: Vec<String>,
    pub minimum_priority: String,
    pub max_delivery_attempts: i32,
    pub retry_base_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationProviderReceiptV1 {
    pub schema_version: String,
    pub receipt_id: String,
    pub adapter_kind: String,
    pub adapter_invocation_key: String,
    pub notification_id: String,
    pub payload_digest: String,
    pub destination_policy_digest: String,
    pub outcome: String,
    pub delivery_claimed: bool,
    pub provider_reference: Option<String>,
    pub issued_at: String,
    pub expires_at: String,
}

#[derive(Debug, Clone)]
pub struct NotificationAdapterRequest {
    pub notification_id: String,
    pub payload: Value,
    pub payload_digest: String,
    pub destination_policy: NotificationDestinationPolicyV1,
    pub destination_policy_digest: String,
    pub adapter_invocation_key: String,
}

pub trait NotificationProviderAdapter: Send + Sync {
    fn deliver<'a>(
        &'a self,
        request: &'a NotificationAdapterRequest,
    ) -> Pin<Box<dyn Future<Output = Result<NotificationProviderReceiptV1, String>> + Send + 'a>>;
}

#[derive(Debug, Default)]
pub struct ZeroSendNotificationAdapter;

impl NotificationProviderAdapter for ZeroSendNotificationAdapter {
    fn deliver<'a>(
        &'a self,
        request: &'a NotificationAdapterRequest,
    ) -> Pin<Box<dyn Future<Output = Result<NotificationProviderReceiptV1, String>> + Send + 'a>>
    {
        Box::pin(async move {
            if request.destination_policy.environment != "sandbox"
                || request.destination_policy.adapter_kind != "zero_send"
                || request.destination_policy.delivery_mode != "simulation"
            {
                return Err("zero_send_policy_scope_mismatch".to_string());
            }
            let issued_at = Utc::now();
            Ok(NotificationProviderReceiptV1 {
                schema_version: PROVIDER_RECEIPT_SCHEMA_VERSION.to_string(),
                receipt_id: format!("zero-send-receipt-{}", Uuid::new_v4()),
                adapter_kind: "zero_send".to_string(),
                adapter_invocation_key: request.adapter_invocation_key.clone(),
                notification_id: request.notification_id.clone(),
                payload_digest: request.payload_digest.clone(),
                destination_policy_digest: request.destination_policy_digest.clone(),
                outcome: "simulated".to_string(),
                delivery_claimed: false,
                provider_reference: None,
                issued_at: issued_at.to_rfc3339(),
                expires_at: (issued_at + Duration::minutes(5)).to_rfc3339(),
            })
        })
    }
}

#[derive(Debug, Clone)]
pub struct BindNotificationDestinationCommand {
    pub notification_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
    pub lease_owner: String,
    pub policy: NotificationDestinationPolicyV1,
}

#[derive(Debug, Clone)]
pub struct CompleteNotificationDeliveryCommand {
    pub notification_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
    pub lease_owner: String,
    pub completion_idempotency_key: String,
    pub receipt: NotificationProviderReceiptV1,
}

#[derive(Debug, Clone)]
pub struct FailNotificationDeliveryCommand {
    pub notification_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
    pub lease_owner: String,
    pub failure_idempotency_key: String,
    pub failure_code: String,
    pub retryable: bool,
}

#[derive(Debug, Clone)]
pub struct RecoverNotificationDeadLetterCommand {
    pub notification_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub executor_snapshot_id: String,
    pub executor_token_hash: String,
    pub recovery_idempotency_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct NotificationDeliveryOutcome {
    pub succeeded: bool,
    pub replayed: bool,
    pub reason_code: Option<String>,
    pub notification_id: String,
    pub status: String,
    pub delivery_attempt_count: i32,
    pub recovery_count: i32,
    pub adapter_invocation_key: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum NotificationDeliveryError {
    #[error("PostgreSQL notification delivery command failed: {0}")]
    Postgres(#[from] sqlx::Error),
}

pub fn destination_policy_digest(policy: &NotificationDestinationPolicyV1) -> String {
    sha256_hex(
        serde_json::to_string(policy)
            .expect("destination policy serialization")
            .as_bytes(),
    )
}

pub fn provider_receipt_digest(receipt: &NotificationProviderReceiptV1) -> String {
    sha256_hex(
        serde_json::to_string(receipt)
            .expect("provider receipt serialization")
            .as_bytes(),
    )
}

pub fn adapter_invocation_key(
    notification_id: &str,
    delivery_attempt_count: i32,
    payload_digest: &str,
    policy_digest: &str,
) -> String {
    sha256_hex(
        serde_json::to_string(&json!([
            "hs-ai-delivery-security-adapter-invocation-v1",
            notification_id,
            delivery_attempt_count,
            payload_digest,
            policy_digest
        ]))
        .expect("adapter invocation key serialization")
        .as_bytes(),
    )
}

pub async fn bind_postgres_notification_destination(
    connection: &mut PgConnection,
    command: &BindNotificationDestinationCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<NotificationDeliveryOutcome, NotificationDeliveryError> {
    if let Some(reason) = authorize(
        preflight,
        command,
        "bind_ai_delivery_notification_destination",
    ) {
        return Ok(rejected(&command.notification_id, &reason));
    }
    if !validate_destination_policy(&command.policy, &command.environment) {
        return Ok(rejected(
            &command.notification_id,
            REASON_DESTINATION_POLICY_INVALID,
        ));
    }
    let policy_digest = destination_policy_digest(&command.policy);
    let mut transaction = connection.begin().await?;
    if !snapshot_matches_scope(
        &mut transaction,
        &command.executor_snapshot_id,
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
    )
    .await?
    {
        transaction.rollback().await?;
        return Ok(rejected(
            &command.notification_id,
            REASON_DESTINATION_POLICY_INVALID,
        ));
    }
    let row = sqlx::query(
        "SELECT incident_id, event_type, priority, payload_digest, status,
                lease_owner, COALESCE(lease_expires_at > NOW(), FALSE) lease_valid,
                delivery_attempt_count, replay_count, recovery_count, destination_policy_digest
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
        return Ok(rejected(
            &command.notification_id,
            REASON_NOTIFICATION_LEASE_CONFLICT,
        ));
    };
    let attempt_count: i32 = row.get("delivery_attempt_count");
    let recovery_count: i32 = row.get("recovery_count");
    if row.get::<String, _>("status") != "leased"
        || !row.get::<bool, _>("lease_valid")
        || row.get::<Option<String>, _>("lease_owner").as_deref()
            != Some(command.lease_owner.as_str())
        || !command
            .policy
            .event_types
            .contains(&row.get::<String, _>("event_type"))
        || priority_rank(&row.get::<String, _>("priority"))
            < priority_rank(&command.policy.minimum_priority)
    {
        transaction.rollback().await?;
        return Ok(rejected(
            &command.notification_id,
            REASON_NOTIFICATION_LEASE_CONFLICT,
        ));
    }
    let existing_digest: Option<String> = row.get("destination_policy_digest");
    if existing_digest
        .as_deref()
        .is_some_and(|value| value != policy_digest)
    {
        transaction.rollback().await?;
        return Ok(rejected(
            &command.notification_id,
            REASON_DESTINATION_POLICY_INVALID,
        ));
    }
    sqlx::query(
        "UPDATE ai_delivery_security_notification_outbox
         SET destination_policy_id = $1, destination_policy_version = $2,
             destination_policy_digest = $3, destination_policy_json = $4,
             adapter_kind = $5, max_delivery_attempts = $6,
             last_reason_code = $7, updated_at = NOW()
         WHERE notification_id = $8",
    )
    .bind(&command.policy.policy_id)
    .bind(command.policy.version)
    .bind(&policy_digest)
    .bind(json!(command.policy))
    .bind(&command.policy.adapter_kind)
    .bind(command.policy.max_delivery_attempts)
    .bind(REASON_DESTINATION_BOUND)
    .bind(&command.notification_id)
    .execute(&mut *transaction)
    .await?;
    let invocation_key = adapter_invocation_key(
        &command.notification_id,
        attempt_count,
        row.get("payload_digest"),
        &policy_digest,
    );
    insert_audit(
        &mut transaction,
        &command.notification_id,
        row.get("incident_id"),
        "destination_bound",
        &command.executor_snapshot_id,
        Some(&command.lease_owner),
        "leased",
        REASON_DESTINATION_BOUND,
        attempt_count,
        row.get("replay_count"),
        json!({
            "destinationPolicyId": command.policy.policy_id,
            "destinationPolicyVersion": command.policy.version,
            "destinationPolicyDigest": policy_digest,
            "adapterKind": command.policy.adapter_kind,
            "deliveryMode": command.policy.delivery_mode,
            "adapterInvocationKey": invocation_key
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(NotificationDeliveryOutcome {
        succeeded: true,
        replayed: existing_digest.is_some(),
        reason_code: None,
        notification_id: command.notification_id.clone(),
        status: "leased".to_string(),
        delivery_attempt_count: attempt_count,
        recovery_count,
        adapter_invocation_key: Some(invocation_key),
    })
}

pub async fn complete_postgres_notification_delivery(
    connection: &mut PgConnection,
    command: &CompleteNotificationDeliveryCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<NotificationDeliveryOutcome, NotificationDeliveryError> {
    if let Some(reason) = authorize(preflight, command, "complete_ai_delivery_notification") {
        return Ok(rejected(&command.notification_id, &reason));
    }
    let mut transaction = connection.begin().await?;
    if !snapshot_matches_scope(
        &mut transaction,
        &command.executor_snapshot_id,
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
    )
    .await?
    {
        transaction.rollback().await?;
        return Ok(rejected(
            &command.notification_id,
            REASON_PROVIDER_RECEIPT_INVALID,
        ));
    }
    let row = sqlx::query(
        "SELECT incident_id, status, lease_owner,
                COALESCE(lease_expires_at > NOW(), FALSE) lease_valid,
                delivery_attempt_count, replay_count, recovery_count,
                payload_digest, destination_policy_digest, adapter_kind,
                completion_idempotency_key, provider_receipt_digest
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
        return Ok(rejected(
            &command.notification_id,
            REASON_NOTIFICATION_LEASE_CONFLICT,
        ));
    };
    let attempt_count: i32 = row.get("delivery_attempt_count");
    let replay_count: i32 = row.get("replay_count");
    let recovery_count: i32 = row.get("recovery_count");
    let receipt_digest = provider_receipt_digest(&command.receipt);
    if row.get::<String, _>("status") == "completed"
        && row
            .get::<Option<String>, _>("completion_idempotency_key")
            .as_deref()
            == Some(command.completion_idempotency_key.as_str())
        && row
            .get::<Option<String>, _>("provider_receipt_digest")
            .as_deref()
            == Some(receipt_digest.as_str())
    {
        insert_audit(
            &mut transaction,
            &command.notification_id,
            row.get("incident_id"),
            "completion_idempotency_replay",
            &command.executor_snapshot_id,
            Some(&command.lease_owner),
            "completed",
            REASON_COMPLETION_IDEMPOTENCY_REPLAY,
            attempt_count,
            replay_count,
            json!({"providerInvoked": false, "receiptDigest": receipt_digest}),
        )
        .await?;
        transaction.commit().await?;
        return Ok(NotificationDeliveryOutcome {
            succeeded: true,
            replayed: true,
            reason_code: Some(REASON_COMPLETION_IDEMPOTENCY_REPLAY.to_string()),
            notification_id: command.notification_id.clone(),
            status: "completed".to_string(),
            delivery_attempt_count: attempt_count,
            recovery_count,
            adapter_invocation_key: Some(command.receipt.adapter_invocation_key.clone()),
        });
    }
    let payload_digest: String = row.get("payload_digest");
    let policy_digest: Option<String> = row.get("destination_policy_digest");
    let adapter_kind: Option<String> = row.get("adapter_kind");
    let expected_invocation_key = policy_digest.as_deref().map(|digest| {
        adapter_invocation_key(
            &command.notification_id,
            attempt_count,
            &payload_digest,
            digest,
        )
    });
    if row.get::<String, _>("status") != "leased"
        || !row.get::<bool, _>("lease_valid")
        || row.get::<Option<String>, _>("lease_owner").as_deref()
            != Some(command.lease_owner.as_str())
        || !valid_receipt(
            &command.receipt,
            &command.notification_id,
            &payload_digest,
            policy_digest.as_deref(),
            adapter_kind.as_deref(),
            expected_invocation_key.as_deref(),
        )
    {
        transaction.rollback().await?;
        return Ok(rejected(
            &command.notification_id,
            REASON_PROVIDER_RECEIPT_INVALID,
        ));
    }
    sqlx::query(
        "INSERT INTO ai_delivery_security_notification_provider_receipts (
            provider_receipt_record_id, notification_id, delivery_attempt_count,
            adapter_kind, adapter_invocation_key, destination_policy_digest,
            payload_digest, provider_receipt_id, provider_outcome, delivery_claimed,
            receipt_json, receipt_digest, accepted_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,NOW())",
    )
    .bind(format!("notification-provider-receipt-{}", Uuid::new_v4()))
    .bind(&command.notification_id)
    .bind(attempt_count)
    .bind(&command.receipt.adapter_kind)
    .bind(&command.receipt.adapter_invocation_key)
    .bind(&command.receipt.destination_policy_digest)
    .bind(&command.receipt.payload_digest)
    .bind(&command.receipt.receipt_id)
    .bind(&command.receipt.outcome)
    .bind(command.receipt.delivery_claimed)
    .bind(json!(command.receipt))
    .bind(&receipt_digest)
    .execute(&mut *transaction)
    .await?;
    sqlx::query(
        "UPDATE ai_delivery_security_notification_outbox
         SET status = 'completed', lease_owner = NULL, lease_expires_at = NULL,
             completion_idempotency_key = $1, provider_receipt_id = $2,
             provider_receipt_digest = $3, completed_at = NOW(),
             last_failure_code = NULL, last_reason_code = $4, updated_at = NOW()
         WHERE notification_id = $5",
    )
    .bind(&command.completion_idempotency_key)
    .bind(&command.receipt.receipt_id)
    .bind(&receipt_digest)
    .bind(REASON_NOTIFICATION_COMPLETED)
    .bind(&command.notification_id)
    .execute(&mut *transaction)
    .await?;
    insert_audit(
        &mut transaction,
        &command.notification_id,
        row.get("incident_id"),
        "completed",
        &command.executor_snapshot_id,
        Some(&command.lease_owner),
        "completed",
        REASON_NOTIFICATION_COMPLETED,
        attempt_count,
        replay_count,
        json!({
            "providerReceiptId": command.receipt.receipt_id,
            "providerReceiptDigest": receipt_digest,
            "providerOutcome": command.receipt.outcome,
            "deliveryClaimed": command.receipt.delivery_claimed
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(NotificationDeliveryOutcome {
        succeeded: true,
        replayed: false,
        reason_code: None,
        notification_id: command.notification_id.clone(),
        status: "completed".to_string(),
        delivery_attempt_count: attempt_count,
        recovery_count,
        adapter_invocation_key: expected_invocation_key,
    })
}

pub async fn fail_postgres_notification_delivery(
    connection: &mut PgConnection,
    command: &FailNotificationDeliveryCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<NotificationDeliveryOutcome, NotificationDeliveryError> {
    if let Some(reason) = authorize(preflight, command, "fail_ai_delivery_notification") {
        return Ok(rejected(&command.notification_id, &reason));
    }
    if command.failure_code.trim().is_empty() || command.failure_idempotency_key.trim().is_empty() {
        return Ok(rejected(
            &command.notification_id,
            REASON_PROVIDER_RECEIPT_INVALID,
        ));
    }
    let mut transaction = connection.begin().await?;
    if !snapshot_matches_scope(
        &mut transaction,
        &command.executor_snapshot_id,
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
    )
    .await?
    {
        transaction.rollback().await?;
        return Ok(rejected(
            &command.notification_id,
            REASON_NOTIFICATION_LEASE_CONFLICT,
        ));
    }
    let row = sqlx::query(
        "SELECT incident_id, status, lease_owner,
                COALESCE(lease_expires_at > NOW(), FALSE) lease_valid,
                delivery_attempt_count, replay_count, recovery_count,
                max_delivery_attempts, destination_policy_json,
                destination_policy_digest, last_failure_idempotency_key
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
        return Ok(rejected(
            &command.notification_id,
            REASON_NOTIFICATION_LEASE_CONFLICT,
        ));
    };
    let attempt_count: i32 = row.get("delivery_attempt_count");
    let replay_count: i32 = row.get("replay_count");
    let recovery_count: i32 = row.get("recovery_count");
    if row
        .get::<Option<String>, _>("last_failure_idempotency_key")
        .as_deref()
        == Some(command.failure_idempotency_key.as_str())
    {
        let status: String = row.get("status");
        insert_audit(
            &mut transaction,
            &command.notification_id,
            row.get("incident_id"),
            "failure_idempotency_replay",
            &command.executor_snapshot_id,
            Some(&command.lease_owner),
            &status,
            REASON_NOTIFICATION_FAILURE_IDEMPOTENCY_REPLAY,
            attempt_count,
            replay_count,
            json!({"providerInvoked": false, "failureCode": command.failure_code}),
        )
        .await?;
        transaction.commit().await?;
        return Ok(NotificationDeliveryOutcome {
            succeeded: true,
            replayed: true,
            reason_code: Some(REASON_NOTIFICATION_FAILURE_IDEMPOTENCY_REPLAY.to_string()),
            notification_id: command.notification_id.clone(),
            status,
            delivery_attempt_count: attempt_count,
            recovery_count,
            adapter_invocation_key: None,
        });
    }
    if row.get::<String, _>("status") != "leased"
        || !row.get::<bool, _>("lease_valid")
        || row.get::<Option<String>, _>("lease_owner").as_deref()
            != Some(command.lease_owner.as_str())
        || row
            .get::<Option<String>, _>("destination_policy_digest")
            .is_none()
    {
        transaction.rollback().await?;
        return Ok(rejected(
            &command.notification_id,
            REASON_NOTIFICATION_LEASE_CONFLICT,
        ));
    }
    let max_attempts: i32 = row.get("max_delivery_attempts");
    let dead_letter = !command.retryable || attempt_count >= max_attempts;
    let policy: NotificationDestinationPolicyV1 =
        serde_json::from_value(row.get("destination_policy_json"))
            .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    let backoff_seconds = (policy.retry_base_seconds
        * 2_i64.pow(attempt_count.saturating_sub(1).min(10) as u32))
    .min(3600);
    let status = if dead_letter {
        "dead_letter"
    } else {
        "retry_scheduled"
    };
    sqlx::query(
        "UPDATE ai_delivery_security_notification_outbox
         SET status = $1, available_at = CASE WHEN $1 = 'retry_scheduled'
                 THEN NOW() + make_interval(secs => $2) ELSE available_at END,
             lease_owner = NULL, lease_expires_at = NULL,
             dead_lettered_at = CASE WHEN $1 = 'dead_letter' THEN NOW() ELSE NULL END,
             last_failure_idempotency_key = $3, last_failure_code = $4,
             last_reason_code = $5, updated_at = NOW()
         WHERE notification_id = $6",
    )
    .bind(status)
    .bind(backoff_seconds as f64)
    .bind(&command.failure_idempotency_key)
    .bind(&command.failure_code)
    .bind(if dead_letter {
        REASON_NOTIFICATION_DEAD_LETTERED
    } else {
        REASON_NOTIFICATION_DELIVERY_FAILED
    })
    .bind(&command.notification_id)
    .execute(&mut *transaction)
    .await?;
    insert_audit(
        &mut transaction,
        &command.notification_id,
        row.get("incident_id"),
        if dead_letter {
            "dead_lettered"
        } else {
            "delivery_failed"
        },
        &command.executor_snapshot_id,
        Some(&command.lease_owner),
        status,
        if dead_letter {
            REASON_NOTIFICATION_DEAD_LETTERED
        } else {
            REASON_NOTIFICATION_DELIVERY_FAILED
        },
        attempt_count,
        replay_count,
        json!({
            "failureCode": command.failure_code,
            "retryable": command.retryable,
            "maxDeliveryAttempts": max_attempts,
            "backoffSeconds": if dead_letter { 0 } else { backoff_seconds }
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(NotificationDeliveryOutcome {
        succeeded: true,
        replayed: false,
        reason_code: None,
        notification_id: command.notification_id.clone(),
        status: status.to_string(),
        delivery_attempt_count: attempt_count,
        recovery_count,
        adapter_invocation_key: None,
    })
}

pub async fn recover_postgres_notification_dead_letter(
    connection: &mut PgConnection,
    command: &RecoverNotificationDeadLetterCommand,
    preflight: &ChangeCommandPreflight<'_>,
) -> Result<NotificationDeliveryOutcome, NotificationDeliveryError> {
    if let Some(reason) = authorize(preflight, command, "recover_ai_delivery_notification") {
        return Ok(rejected(&command.notification_id, &reason));
    }
    let mut transaction = connection.begin().await?;
    if !snapshot_matches_scope(
        &mut transaction,
        &command.executor_snapshot_id,
        &command.tenant_id,
        &command.workspace_id,
        &command.environment,
    )
    .await?
    {
        transaction.rollback().await?;
        return Ok(rejected(
            &command.notification_id,
            REASON_NOTIFICATION_LEASE_CONFLICT,
        ));
    }
    let row = sqlx::query(
        "SELECT incident_id, status, delivery_attempt_count, replay_count,
                recovery_count, last_recovery_idempotency_key
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
        return Ok(rejected(
            &command.notification_id,
            REASON_NOTIFICATION_LEASE_CONFLICT,
        ));
    };
    let attempt_count: i32 = row.get("delivery_attempt_count");
    let replay_count: i32 = row.get("replay_count");
    let recovery_count: i32 = row.get("recovery_count");
    if row
        .get::<Option<String>, _>("last_recovery_idempotency_key")
        .as_deref()
        == Some(command.recovery_idempotency_key.as_str())
    {
        let status: String = row.get("status");
        insert_audit(
            &mut transaction,
            &command.notification_id,
            row.get("incident_id"),
            "recovery_idempotency_replay",
            &command.executor_snapshot_id,
            None,
            &status,
            REASON_NOTIFICATION_RECOVERY_IDEMPOTENCY_REPLAY,
            attempt_count,
            replay_count,
            json!({"providerInvoked": false, "recoveryCount": recovery_count}),
        )
        .await?;
        transaction.commit().await?;
        return Ok(NotificationDeliveryOutcome {
            succeeded: true,
            replayed: true,
            reason_code: Some(REASON_NOTIFICATION_RECOVERY_IDEMPOTENCY_REPLAY.to_string()),
            notification_id: command.notification_id.clone(),
            status,
            delivery_attempt_count: attempt_count,
            recovery_count,
            adapter_invocation_key: None,
        });
    }
    if row.get::<String, _>("status") != "dead_letter"
        || command.recovery_idempotency_key.trim().is_empty()
    {
        transaction.rollback().await?;
        return Ok(rejected(
            &command.notification_id,
            REASON_NOTIFICATION_LEASE_CONFLICT,
        ));
    }
    let updated_recovery_count = recovery_count + 1;
    sqlx::query(
        "UPDATE ai_delivery_security_notification_outbox
         SET status = 'retry_scheduled', available_at = NOW(),
             dead_lettered_at = NULL, last_failure_code = NULL,
             last_recovery_idempotency_key = $1, recovery_count = recovery_count + 1,
             last_reason_code = $2, updated_at = NOW()
         WHERE notification_id = $3",
    )
    .bind(&command.recovery_idempotency_key)
    .bind(REASON_NOTIFICATION_RECOVERED)
    .bind(&command.notification_id)
    .execute(&mut *transaction)
    .await?;
    insert_audit(
        &mut transaction,
        &command.notification_id,
        row.get("incident_id"),
        "replay_scheduled",
        &command.executor_snapshot_id,
        None,
        "retry_scheduled",
        REASON_NOTIFICATION_RECOVERED,
        attempt_count,
        replay_count,
        json!({
            "deadLetterRecovery": true,
            "recoveryCount": updated_recovery_count,
            "providerInvoked": false
        }),
    )
    .await?;
    transaction.commit().await?;
    Ok(NotificationDeliveryOutcome {
        succeeded: true,
        replayed: false,
        reason_code: None,
        notification_id: command.notification_id.clone(),
        status: "retry_scheduled".to_string(),
        delivery_attempt_count: attempt_count,
        recovery_count: updated_recovery_count,
        adapter_invocation_key: None,
    })
}

trait NotificationScopeCommand {
    fn tenant_id(&self) -> &str;
    fn workspace_id(&self) -> &str;
    fn environment(&self) -> &str;
    fn executor_token_hash(&self) -> &str;
}

impl NotificationScopeCommand for BindNotificationDestinationCommand {
    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
    fn workspace_id(&self) -> &str {
        &self.workspace_id
    }
    fn environment(&self) -> &str {
        &self.environment
    }
    fn executor_token_hash(&self) -> &str {
        &self.executor_token_hash
    }
}

impl NotificationScopeCommand for CompleteNotificationDeliveryCommand {
    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
    fn workspace_id(&self) -> &str {
        &self.workspace_id
    }
    fn environment(&self) -> &str {
        &self.environment
    }
    fn executor_token_hash(&self) -> &str {
        &self.executor_token_hash
    }
}

impl NotificationScopeCommand for FailNotificationDeliveryCommand {
    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
    fn workspace_id(&self) -> &str {
        &self.workspace_id
    }
    fn environment(&self) -> &str {
        &self.environment
    }
    fn executor_token_hash(&self) -> &str {
        &self.executor_token_hash
    }
}

impl NotificationScopeCommand for RecoverNotificationDeadLetterCommand {
    fn tenant_id(&self) -> &str {
        &self.tenant_id
    }
    fn workspace_id(&self) -> &str {
        &self.workspace_id
    }
    fn environment(&self) -> &str {
        &self.environment
    }
    fn executor_token_hash(&self) -> &str {
        &self.executor_token_hash
    }
}

fn authorize(
    preflight: &ChangeCommandPreflight<'_>,
    command: &impl NotificationScopeCommand,
    operation: &str,
) -> Option<String> {
    let decision = preflight
        .iam
        .verify_actor_authorization(&ActorAuthorizationInput {
            token_hash: command.executor_token_hash(),
            required_role: "system_executor",
            tenant_id: command.tenant_id(),
            workspace_id: command.workspace_id(),
            environment: command.environment(),
            operation,
        });
    if decision.authorized {
        None
    } else {
        Some(
            decision
                .reason_code
                .unwrap_or_else(|| "iam_unavailable".to_string()),
        )
    }
}

pub fn validate_destination_policy(
    policy: &NotificationDestinationPolicyV1,
    environment: &str,
) -> bool {
    policy.schema_version == DESTINATION_POLICY_SCHEMA_VERSION
        && !policy.policy_id.trim().is_empty()
        && policy.version >= 1
        && policy.environment == environment
        && policy.enabled
        && matches!(
            policy.adapter_kind.as_str(),
            "zero_send" | "pagerduty" | "email" | "sms"
        )
        && matches!(policy.delivery_mode.as_str(), "simulation" | "external")
        && !policy.destination_ref.trim().is_empty()
        && !policy.event_types.is_empty()
        && policy.event_types.iter().all(|event| {
            matches!(
                event.as_str(),
                "incident_opened"
                    | "incident_became_critical"
                    | "incident_acknowledged"
                    | "incident_resolved"
            )
        })
        && matches!(
            policy.minimum_priority.as_str(),
            "info" | "warning" | "critical"
        )
        && (1..=NOTIFICATION_DELIVERY_MAX_ATTEMPTS).contains(&policy.max_delivery_attempts)
        && (1..=3600).contains(&policy.retry_base_seconds)
        && ((policy.adapter_kind == "zero_send"
            && policy.delivery_mode == "simulation"
            && environment == "sandbox")
            || (policy.adapter_kind != "zero_send" && policy.delivery_mode == "external"))
}

fn valid_receipt(
    receipt: &NotificationProviderReceiptV1,
    notification_id: &str,
    payload_digest: &str,
    policy_digest: Option<&str>,
    adapter_kind: Option<&str>,
    invocation_key: Option<&str>,
) -> bool {
    let Ok(issued_at) = DateTime::parse_from_rfc3339(&receipt.issued_at) else {
        return false;
    };
    let Ok(expires_at) = DateTime::parse_from_rfc3339(&receipt.expires_at) else {
        return false;
    };
    let now = Utc::now();
    receipt.schema_version == PROVIDER_RECEIPT_SCHEMA_VERSION
        && !receipt.receipt_id.trim().is_empty()
        && receipt.notification_id == notification_id
        && receipt.payload_digest == payload_digest
        && Some(receipt.destination_policy_digest.as_str()) == policy_digest
        && Some(receipt.adapter_kind.as_str()) == adapter_kind
        && Some(receipt.adapter_invocation_key.as_str()) == invocation_key
        && issued_at <= now
        && expires_at > now
        && expires_at - issued_at <= Duration::seconds(NOTIFICATION_RECEIPT_MAX_LIFETIME_SECONDS)
        && ((receipt.outcome == "simulated"
            && receipt.adapter_kind == "zero_send"
            && !receipt.delivery_claimed
            && receipt.provider_reference.is_none())
            || (receipt.outcome == "delivered"
                && receipt.adapter_kind != "zero_send"
                && receipt.delivery_claimed
                && receipt
                    .provider_reference
                    .as_deref()
                    .is_some_and(|value| !value.is_empty())))
}

fn priority_rank(priority: &str) -> i32 {
    match priority {
        "critical" => 3,
        "warning" => 2,
        "info" => 1,
        _ => 0,
    }
}

async fn snapshot_matches_scope(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    snapshot_id: &str,
    tenant_id: &str,
    workspace_id: &str,
    environment: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT EXISTS(
            SELECT 1 FROM ai_transparency_actor_role_snapshots
            WHERE actor_role_snapshot_id = $1 AND role = 'system_executor'
              AND tenant_id = $2 AND workspace_id = $3 AND environment = $4
              AND source_expires_at > NOW()
         )",
    )
    .bind(snapshot_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(environment)
    .fetch_one(&mut **transaction)
    .await
}

#[allow(clippy::too_many_arguments)]
async fn insert_audit(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
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

fn rejected(notification_id: &str, reason_code: &str) -> NotificationDeliveryOutcome {
    NotificationDeliveryOutcome {
        succeeded: false,
        replayed: false,
        reason_code: Some(reason_code.to_string()),
        notification_id: notification_id.to_string(),
        status: "rejected".to_string(),
        delivery_attempt_count: 0,
        recovery_count: 0,
        adapter_invocation_key: None,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
use std::{future::Future, pin::Pin};

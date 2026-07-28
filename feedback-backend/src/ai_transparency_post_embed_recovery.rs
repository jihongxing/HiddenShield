use chrono::{DateTime, Duration, Utc};
use serde::Serialize;
use serde_json::{json, Value};
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::ai_transparency_post_embed_signing::{
    execute_postgres_internal_post_embed_signing, InternalPostEmbedSigningCommand,
    PostEmbedArtifactStore, PostEmbedAuthorizationVerifier, PostEmbedC2paSigner,
    PostEmbedReadbackVerifier,
};

pub const RECOVERY_REASON_CLAIMED: &str = "ai_post_embed_recovery_claimed";
pub const RECOVERY_REASON_SUCCEEDED: &str = "ai_post_embed_recovery_succeeded";
pub const RECOVERY_REASON_LOADER_FAILED: &str = "ai_post_embed_recovery_loader_failed";
pub const RECOVERY_REASON_COMMAND_ERROR: &str = "ai_post_embed_recovery_command_error";
pub const RECOVERY_REASON_COMMAND_REJECTED: &str = "ai_post_embed_recovery_command_rejected";

#[derive(Debug, Clone)]
pub struct PostEmbedRecoveryWorkerConfig {
    pub worker_id: String,
    pub batch_size: u32,
    pub artifact_pending_timeout: Duration,
    pub recovery_lease_duration: Duration,
    pub base_backoff: Duration,
    pub max_backoff: Duration,
    pub max_attempts: i32,
}

impl PostEmbedRecoveryWorkerConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.worker_id.trim().is_empty()
            || self.batch_size == 0
            || self.batch_size > 1_000
            || self.artifact_pending_timeout.num_seconds() <= 0
            || self.recovery_lease_duration.num_seconds() <= 0
            || self.base_backoff.num_seconds() <= 0
            || self.max_backoff < self.base_backoff
            || self.max_attempts <= 0
        {
            return Err("invalid post-embed recovery worker configuration");
        }
        Ok(())
    }
}

pub trait PostEmbedRecoveryCommandLoader: Send + Sync {
    fn load(&self, execution_id: &str) -> Result<InternalPostEmbedSigningCommand, &'static str>;
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostEmbedRecoveryItemOutcome {
    pub execution_id: String,
    pub source_status: String,
    pub attempt: i32,
    pub result: String,
    pub reason_code: String,
    pub next_attempt_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PostEmbedRecoveryBatchOutcome {
    pub worker_id: String,
    pub claimed: usize,
    pub succeeded: usize,
    pub retry_scheduled: usize,
    pub dead_lettered: usize,
    pub items: Vec<PostEmbedRecoveryItemOutcome>,
}

struct ClaimedRecovery {
    execution_id: String,
    source_status: String,
    attempt: i32,
}

pub async fn run_postgres_post_embed_recovery_batch(
    pool: &PgPool,
    config: &PostEmbedRecoveryWorkerConfig,
    command_loader: &dyn PostEmbedRecoveryCommandLoader,
    authorization_verifier: &dyn PostEmbedAuthorizationVerifier,
    signer: &dyn PostEmbedC2paSigner,
    readback_verifier: &dyn PostEmbedReadbackVerifier,
    artifact_store: &dyn PostEmbedArtifactStore,
) -> Result<PostEmbedRecoveryBatchOutcome, sqlx::Error> {
    config
        .validate()
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    let mut batch = PostEmbedRecoveryBatchOutcome {
        worker_id: config.worker_id.clone(),
        claimed: 0,
        succeeded: 0,
        retry_scheduled: 0,
        dead_lettered: 0,
        items: Vec::new(),
    };
    for _ in 0..config.batch_size {
        let Some(claimed) = claim_next(pool, config).await? else {
            break;
        };
        batch.claimed += 1;
        let command = match command_loader.load(&claimed.execution_id) {
            Ok(command) if command.execution_id == claimed.execution_id => command,
            _ => {
                let item = finish_failure(
                    pool,
                    config,
                    &claimed,
                    RECOVERY_REASON_LOADER_FAILED,
                    json!({"phase": "command_load"}),
                )
                .await?;
                record_result(&mut batch, item);
                continue;
            }
        };
        let execution = match pool.acquire().await {
            Ok(mut connection) => {
                execute_postgres_internal_post_embed_signing(
                    &mut connection,
                    &command,
                    authorization_verifier,
                    signer,
                    readback_verifier,
                    artifact_store,
                )
                .await
            }
            Err(error) => return Err(error),
        };
        let item = match execution {
            Ok(outcome) if outcome.succeeded => finish_success(pool, config, &claimed).await?,
            Ok(outcome) => {
                let reason = outcome
                    .reason_code
                    .as_deref()
                    .unwrap_or(RECOVERY_REASON_COMMAND_REJECTED);
                finish_failure(
                    pool,
                    config,
                    &claimed,
                    reason,
                    json!({
                        "phase": "command",
                        "artifactPending": outcome.artifact_pending,
                        "signerInvoked": outcome.signer_invoked
                    }),
                )
                .await?
            }
            Err(_) => {
                finish_failure(
                    pool,
                    config,
                    &claimed,
                    RECOVERY_REASON_COMMAND_ERROR,
                    json!({"phase": "command", "errorClass": "postgres_command_error"}),
                )
                .await?
            }
        };
        record_result(&mut batch, item);
    }
    Ok(batch)
}

fn record_result(batch: &mut PostEmbedRecoveryBatchOutcome, item: PostEmbedRecoveryItemOutcome) {
    match item.result.as_str() {
        "succeeded" => batch.succeeded += 1,
        "retry_scheduled" => batch.retry_scheduled += 1,
        "dead_letter" => batch.dead_lettered += 1,
        _ => {}
    }
    batch.items.push(item);
}

async fn claim_next(
    pool: &PgPool,
    config: &PostEmbedRecoveryWorkerConfig,
) -> Result<Option<ClaimedRecovery>, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let row = sqlx::query(
        "WITH candidate AS (
            SELECT execution_id
            FROM ai_post_embed_signing_executions
            WHERE status IN ('reserved', 'artifact_pending')
              AND (
                (
                    recovery_state IN ('eligible', 'retry_scheduled')
                    AND next_recovery_at <= NOW()
                )
                OR (
                    recovery_state = 'leased'
                    AND recovery_lease_expires_at <= NOW()
                )
              )
              AND (
                (status = 'reserved' AND lease_expires_at <= NOW())
                OR (
                    status = 'artifact_pending'
                    AND updated_at <= NOW() - ($1::BIGINT * INTERVAL '1 second')
                )
              )
            ORDER BY next_recovery_at ASC, updated_at ASC, execution_id ASC
            FOR UPDATE SKIP LOCKED
            LIMIT 1
         )
         UPDATE ai_post_embed_signing_executions execution
         SET recovery_state = 'leased',
             worker_recovery_attempts = worker_recovery_attempts + 1,
             recovery_lease_owner = $2,
             recovery_lease_expires_at = NOW() + ($3::BIGINT * INTERVAL '1 second'),
             last_recovery_reason = $4,
             dead_lettered_at = NULL
         FROM candidate
         WHERE execution.execution_id = candidate.execution_id
         RETURNING execution.execution_id, execution.status, execution.worker_recovery_attempts",
    )
    .bind(config.artifact_pending_timeout.num_seconds())
    .bind(&config.worker_id)
    .bind(config.recovery_lease_duration.num_seconds())
    .bind(RECOVERY_REASON_CLAIMED)
    .fetch_optional(&mut *transaction)
    .await?;
    let Some(row) = row else {
        transaction.rollback().await?;
        return Ok(None);
    };
    let claimed = ClaimedRecovery {
        execution_id: row.get("execution_id"),
        source_status: row.get("status"),
        attempt: row.get("worker_recovery_attempts"),
    };
    insert_recovery_audit(
        &mut transaction,
        &claimed,
        &config.worker_id,
        "claimed",
        RECOVERY_REASON_CLAIMED,
        None,
        json!({"sourceStatus": claimed.source_status}),
    )
    .await?;
    transaction.commit().await?;
    Ok(Some(claimed))
}

async fn finish_success(
    pool: &PgPool,
    config: &PostEmbedRecoveryWorkerConfig,
    claimed: &ClaimedRecovery,
) -> Result<PostEmbedRecoveryItemOutcome, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let updated = sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET recovery_state = 'completed',
             recovery_lease_owner = NULL,
             recovery_lease_expires_at = NULL,
             next_recovery_at = NOW(),
             last_recovery_reason = $1,
             dead_lettered_at = NULL
         WHERE execution_id = $2
           AND recovery_state = 'leased'
           AND recovery_lease_owner = $3",
    )
    .bind(RECOVERY_REASON_SUCCEEDED)
    .bind(&claimed.execution_id)
    .bind(&config.worker_id)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if updated != 1 {
        transaction.rollback().await?;
        return Err(sqlx::Error::Protocol(
            "post-embed recovery success lost worker lease".to_string(),
        ));
    }
    insert_recovery_audit(
        &mut transaction,
        claimed,
        &config.worker_id,
        "succeeded",
        RECOVERY_REASON_SUCCEEDED,
        None,
        json!({"finalStatus": "confirmed"}),
    )
    .await?;
    transaction.commit().await?;
    Ok(PostEmbedRecoveryItemOutcome {
        execution_id: claimed.execution_id.clone(),
        source_status: claimed.source_status.clone(),
        attempt: claimed.attempt,
        result: "succeeded".to_string(),
        reason_code: RECOVERY_REASON_SUCCEEDED.to_string(),
        next_attempt_at: None,
    })
}

async fn finish_failure(
    pool: &PgPool,
    config: &PostEmbedRecoveryWorkerConfig,
    claimed: &ClaimedRecovery,
    reason_code: &str,
    details: Value,
) -> Result<PostEmbedRecoveryItemOutcome, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let status = sqlx::query_scalar::<_, String>(
        "SELECT status
         FROM ai_post_embed_signing_executions
         WHERE execution_id = $1
         FOR UPDATE",
    )
    .bind(&claimed.execution_id)
    .fetch_one(&mut *transaction)
    .await?;
    let dead_letter = claimed.attempt >= config.max_attempts
        || !matches!(status.as_str(), "reserved" | "artifact_pending");
    let next_attempt_at = if dead_letter {
        None
    } else {
        Some(Utc::now() + retry_backoff(config, claimed.attempt))
    };
    let recovery_state = if dead_letter {
        "dead_letter"
    } else {
        "retry_scheduled"
    };
    let updated = sqlx::query(
        "UPDATE ai_post_embed_signing_executions
         SET recovery_state = $1,
             recovery_lease_owner = NULL,
             recovery_lease_expires_at = NULL,
             next_recovery_at = COALESCE($2, next_recovery_at),
             last_recovery_reason = $3,
             dead_lettered_at = CASE WHEN $1 = 'dead_letter' THEN NOW() ELSE NULL END,
             lease_expires_at = CASE WHEN status = 'reserved' THEN NOW() ELSE lease_expires_at END
         WHERE execution_id = $4
           AND recovery_state = 'leased'
           AND recovery_lease_owner = $5",
    )
    .bind(recovery_state)
    .bind(next_attempt_at)
    .bind(reason_code)
    .bind(&claimed.execution_id)
    .bind(&config.worker_id)
    .execute(&mut *transaction)
    .await?
    .rows_affected();
    if updated != 1 {
        transaction.rollback().await?;
        return Err(sqlx::Error::Protocol(
            "post-embed recovery failure lost worker lease".to_string(),
        ));
    }
    let event_type = if dead_letter {
        "dead_letter"
    } else {
        "retry_scheduled"
    };
    insert_recovery_audit(
        &mut transaction,
        claimed,
        &config.worker_id,
        event_type,
        reason_code,
        next_attempt_at,
        details,
    )
    .await?;
    transaction.commit().await?;
    Ok(PostEmbedRecoveryItemOutcome {
        execution_id: claimed.execution_id.clone(),
        source_status: claimed.source_status.clone(),
        attempt: claimed.attempt,
        result: recovery_state.to_string(),
        reason_code: reason_code.to_string(),
        next_attempt_at,
    })
}

fn retry_backoff(config: &PostEmbedRecoveryWorkerConfig, attempt: i32) -> Duration {
    let exponent = u32::try_from(attempt.saturating_sub(1))
        .unwrap_or(0)
        .min(30);
    let multiplier = 1_i64.checked_shl(exponent).unwrap_or(i64::MAX);
    let seconds = config
        .base_backoff
        .num_seconds()
        .saturating_mul(multiplier)
        .min(config.max_backoff.num_seconds());
    Duration::seconds(seconds)
}

async fn insert_recovery_audit(
    transaction: &mut Transaction<'_, Postgres>,
    claimed: &ClaimedRecovery,
    worker_id: &str,
    event_type: &str,
    reason_code: &str,
    next_attempt_at: Option<DateTime<Utc>>,
    details: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_post_embed_recovery_audit_events (
            recovery_audit_event_id, execution_id, worker_id, attempt, event_type,
            reason_code, next_attempt_at, details_json, occurred_at
         ) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,NOW())",
    )
    .bind(format!("recovery-audit-{}", Uuid::new_v4()))
    .bind(&claimed.execution_id)
    .bind(worker_id)
    .bind(claimed.attempt)
    .bind(event_type)
    .bind(reason_code)
    .bind(next_attempt_at)
    .bind(details)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

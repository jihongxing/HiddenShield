use std::future::Future;
use std::sync::Arc;

use chrono::Utc;
use sha2::{Digest, Sha256};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

use crate::repository::CloudSyncRepository;
use crate::schema::{
    CloudSyncBatchRequest, CloudSyncBatchResult, CloudSyncChange, CloudSyncChangesResult,
    CloudSyncEventDisposition,
};
use crate::storage::StorageError;

#[derive(Clone)]
pub struct PostgresCloudSyncRepository {
    pool: PgPool,
    runtime: Arc<tokio::runtime::Runtime>,
}

impl PostgresCloudSyncRepository {
    pub fn new(pool: PgPool) -> Self {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("postgres sync repository runtime must be available"),
        );
        Self { pool, runtime }
    }

    pub fn connect(database_url: &str, max_connections: u32) -> Result<Self, StorageError> {
        let runtime = Arc::new(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|error| {
                    StorageError::BadRequest(format!("postgres_runtime_unavailable:{error}"))
                })?,
        );
        let pool = runtime.block_on(async move {
            PgPoolOptions::new()
                .max_connections(max_connections)
                .connect(database_url)
                .await
        })?;
        Ok(Self { pool, runtime })
    }

    fn run<T, F>(&self, future: F) -> Result<T, StorageError>
    where
        F: Future<Output = Result<T, StorageError>>,
    {
        self.runtime.block_on(future)
    }
}

impl CloudSyncRepository for PostgresCloudSyncRepository {
    fn push_cloud_events_batch(
        &self,
        access_token: &str,
        request: &CloudSyncBatchRequest,
    ) -> Result<CloudSyncBatchResult, StorageError> {
        self.run(push_cloud_events_batch_pg(
            self.pool.clone(),
            access_token.to_string(),
            request.clone(),
        ))
    }

    fn get_cloud_changes(
        &self,
        access_token: &str,
        workspace_id: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<CloudSyncChangesResult, StorageError> {
        self.run(get_cloud_changes_pg(
            self.pool.clone(),
            access_token.to_string(),
            workspace_id.map(str::to_string),
            cursor.map(str::to_string),
        ))
    }
}

async fn push_cloud_events_batch_pg(
    pool: PgPool,
    access_token: String,
    request: CloudSyncBatchRequest,
) -> Result<CloudSyncBatchResult, StorageError> {
    let session = authenticate_pg(&pool, &access_token).await?;
    let device_id = request.device_id.trim();
    if device_id.is_empty() {
        return Err(StorageError::BadRequest("deviceId is required".to_string()));
    }
    if session.device_id != device_id {
        return Err(StorageError::Unauthorized);
    }
    let workspace_id = request.workspace_id.trim();
    if workspace_id.is_empty() {
        return Err(StorageError::BadRequest(
            "workspaceId is required".to_string(),
        ));
    }
    if !session_workspace_matches_pg(&pool, &session.account_id, workspace_id).await? {
        return Err(StorageError::Forbidden);
    }
    ensure_cloud_sync_entitled_pg(&pool, &session.account_id).await?;
    if request.events.is_empty() {
        return Err(StorageError::BadRequest(
            "events must not be empty".to_string(),
        ));
    }

    let mut tx = pool.begin().await?;
    let mut accepted_event_ids = Vec::new();
    let mut event_results = Vec::new();
    for event in &request.events {
        let client_event_id = event.client_event_id.trim();
        let entity_type = event.entity_type.trim();
        let entity_id = event.entity_id.trim();
        if client_event_id.is_empty() || entity_type.is_empty() || entity_id.is_empty() {
            event_results.push(CloudSyncEventDisposition {
                client_event_id: client_event_id.to_string(),
                disposition: "rejected_invalid_event".to_string(),
                payload_hash: None,
                entity_revision: None,
                message: Some("clientEventId, entityType and entityId are required".to_string()),
            });
            continue;
        }
        let payload_hash = cloud_sync_payload_hash(&event.payload);
        let entity_revision = cloud_sync_entity_revision(&event.payload);
        let existing = sqlx::query(
            "SELECT payload_hash, entity_revision FROM cloud_sync_events
             WHERE account_id = $1 AND device_id = $2 AND client_event_id = $3
             LIMIT 1",
        )
        .bind(&session.account_id)
        .bind(&session.device_id)
        .bind(client_event_id)
        .fetch_optional(&mut *tx)
        .await?;
        if let Some(row) = existing {
            let existing_hash: Option<String> = row.try_get("payload_hash")?;
            let existing_revision: Option<i64> = row.try_get("entity_revision")?;
            if existing_hash
                .as_deref()
                .is_some_and(|hash| hash != payload_hash)
            {
                event_results.push(CloudSyncEventDisposition {
                    client_event_id: client_event_id.to_string(),
                    disposition: "conflict_payload_changed".to_string(),
                    payload_hash: Some(payload_hash),
                    entity_revision,
                    message: Some(
                        "same clientEventId was received with a different payload hash".to_string(),
                    ),
                });
                continue;
            }
            accepted_event_ids.push(client_event_id.to_string());
            event_results.push(CloudSyncEventDisposition {
                client_event_id: client_event_id.to_string(),
                disposition: "duplicate".to_string(),
                payload_hash: existing_hash.or(Some(payload_hash)),
                entity_revision: existing_revision.or(entity_revision),
                message: None,
            });
            continue;
        }
        {
            sqlx::query(
                "INSERT INTO cloud_sync_events (
                    account_id, device_id, client_event_id, operation, entity_type,
                    entity_id, payload_json, payload_hash, entity_revision, created_at
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                ON CONFLICT(account_id, device_id, client_event_id) DO NOTHING",
            )
            .bind(&session.account_id)
            .bind(&session.device_id)
            .bind(client_event_id)
            .bind(event.operation.trim())
            .bind(entity_type)
            .bind(entity_id)
            .bind(event.payload.clone())
            .bind(&payload_hash)
            .bind(entity_revision)
            .bind(Utc::now())
            .execute(&mut *tx)
            .await?;
        }
        accepted_event_ids.push(client_event_id.to_string());
        event_results.push(CloudSyncEventDisposition {
            client_event_id: client_event_id.to_string(),
            disposition: "accepted".to_string(),
            payload_hash: Some(payload_hash),
            entity_revision,
            message: None,
        });
    }
    tx.commit().await?;

    let next_cursor = account_cursor_pg(&pool, &session.account_id).await?;
    Ok(CloudSyncBatchResult {
        accepted: accepted_event_ids.len() as u32,
        accepted_event_ids,
        next_cursor,
        resolutions: serde_json::json!([]),
        event_results,
    })
}

async fn get_cloud_changes_pg(
    pool: PgPool,
    access_token: String,
    workspace_id: Option<String>,
    cursor: Option<String>,
) -> Result<CloudSyncChangesResult, StorageError> {
    let session = authenticate_pg(&pool, &access_token).await?;
    let workspace_id = workspace_id.unwrap_or_default();
    let workspace_id = workspace_id.trim();
    if workspace_id.is_empty() {
        return Err(StorageError::BadRequest(
            "workspaceId is required".to_string(),
        ));
    }
    if !session_workspace_matches_pg(&pool, &session.account_id, workspace_id).await? {
        return Err(StorageError::Forbidden);
    }
    ensure_cloud_sync_entitled_pg(&pool, &session.account_id).await?;
    let stored_device_cursor =
        device_cursor_pg(&pool, &session.account_id, &session.device_id).await?;
    let client_since_sequence = sequence_from_cursor(cursor.as_deref());
    let stored_since_sequence = sequence_from_cursor(stored_device_cursor.as_deref());
    let since_sequence = if stored_device_cursor.is_some() {
        client_since_sequence.min(stored_since_sequence)
    } else {
        0
    };

    let rows = sqlx::query(
        "SELECT sequence, device_id, operation, entity_type, payload_json
         FROM cloud_sync_events
         WHERE account_id = $1 AND sequence > $2
         ORDER BY sequence ASC",
    )
    .bind(&session.account_id)
    .bind(since_sequence)
    .fetch_all(&pool)
    .await?;
    let changes = rows
        .into_iter()
        .map(|row| {
            let sequence: i64 = row.try_get("sequence")?;
            let operation: String = row.try_get("operation")?;
            Ok(CloudSyncChange {
                cursor: Some(cursor_from_sequence(sequence as u64)),
                entity_type: row.try_get("entity_type")?,
                operation: cloud_operation(&operation),
                source_device: Some(row.try_get("device_id")?),
                entity: row.try_get("payload_json")?,
            })
        })
        .collect::<Result<Vec<_>, sqlx::Error>>()?;

    let next_cursor = account_cursor_pg(&pool, &session.account_id)
        .await?
        .unwrap_or_else(|| {
            cursor
                .clone()
                .unwrap_or_else(|| cursor_from_sequence(since_sequence as u64))
        });
    upsert_device_cursor_pg(&pool, &session.account_id, &session.device_id, &next_cursor).await?;

    Ok(CloudSyncChangesResult {
        next_cursor,
        changes,
    })
}

async fn authenticate_pg(pool: &PgPool, access_token: &str) -> Result<SessionRecord, StorageError> {
    let access_token = access_token.trim();
    if access_token.is_empty() {
        return Err(StorageError::Unauthorized);
    }
    let row = sqlx::query(
        "SELECT account_id, device_id, revoked_at
         FROM cloud_sessions
         WHERE access_token = $1",
    )
    .bind(access_token)
    .fetch_optional(pool)
    .await?
    .ok_or(StorageError::Unauthorized)?;
    let session = SessionRecord {
        account_id: row.try_get("account_id")?,
        device_id: row.try_get("device_id")?,
        revoked_at: row.try_get("revoked_at")?,
    };
    if session.revoked_at.is_some() {
        return Err(StorageError::Unauthorized);
    }
    Ok(session)
}

async fn session_workspace_matches_pg(
    pool: &PgPool,
    account_id: &str,
    workspace_id: &str,
) -> Result<bool, StorageError> {
    let stored_workspace_id: Option<String> =
        sqlx::query_scalar("SELECT workspace_id FROM cloud_accounts WHERE id = $1")
            .bind(account_id.trim())
            .fetch_optional(pool)
            .await?;
    Ok(stored_workspace_id.as_deref() == Some(workspace_id))
}

async fn ensure_cloud_sync_entitled_pg(
    pool: &PgPool,
    account_id: &str,
) -> Result<(), StorageError> {
    let features: serde_json::Value =
        sqlx::query_scalar("SELECT entitlement_features_json FROM cloud_accounts WHERE id = $1")
            .bind(account_id.trim())
            .fetch_one(pool)
            .await?;
    if features
        .get("cloud_sync")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        Ok(())
    } else {
        Err(StorageError::Forbidden)
    }
}

async fn account_cursor_pg(
    pool: &PgPool,
    account_id: &str,
) -> Result<Option<String>, StorageError> {
    let cursor: Option<i64> =
        sqlx::query_scalar("SELECT MAX(sequence) FROM cloud_sync_events WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(pool)
            .await?;
    Ok(cursor.map(|value| cursor_from_sequence(value as u64)))
}

async fn device_cursor_pg(
    pool: &PgPool,
    account_id: &str,
    device_id: &str,
) -> Result<Option<String>, StorageError> {
    let cursor: Option<String> = sqlx::query_scalar(
        "SELECT cursor
         FROM cloud_device_cursors
         WHERE account_id = $1 AND device_id = $2",
    )
    .bind(account_id)
    .bind(device_id)
    .fetch_optional(pool)
    .await?;
    Ok(cursor)
}

fn cloud_sync_payload_hash(payload: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(payload).unwrap_or_else(|_| b"{}".to_vec());
    format!("sha256:{}", hex_lower(&Sha256::digest(bytes)))
}

fn cloud_sync_entity_revision(payload: &serde_json::Value) -> Option<i64> {
    payload
        .get("revision")
        .and_then(|value| value.as_i64().or_else(|| value.as_u64().map(|n| n as i64)))
        .filter(|revision| *revision > 0)
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

async fn upsert_device_cursor_pg(
    pool: &PgPool,
    account_id: &str,
    device_id: &str,
    cursor: &str,
) -> Result<(), StorageError> {
    sqlx::query(
        "INSERT INTO cloud_device_cursors (
            account_id, device_id, cursor, updated_at
        ) VALUES ($1, $2, $3, $4)
        ON CONFLICT(account_id, device_id) DO UPDATE SET
            cursor = excluded.cursor,
            updated_at = excluded.updated_at",
    )
    .bind(account_id)
    .bind(device_id)
    .bind(cursor)
    .bind(Utc::now())
    .execute(pool)
    .await?;
    Ok(())
}

#[derive(Debug, Clone)]
struct SessionRecord {
    account_id: String,
    device_id: String,
    revoked_at: Option<chrono::DateTime<Utc>>,
}

fn sequence_from_cursor(cursor: Option<&str>) -> i64 {
    let Some(cursor) = cursor.map(str::trim).filter(|value| !value.is_empty()) else {
        return 0;
    };
    let Some(value) = cursor.strip_prefix("cursor_") else {
        return 0;
    };
    value.parse::<i64>().unwrap_or(0).max(0)
}

fn cursor_from_sequence(value: u64) -> String {
    format!("cursor_{value}")
}

fn cloud_operation(operation: &str) -> String {
    if operation.starts_with("upsert") {
        "upsert".to_string()
    } else {
        operation.to_string()
    }
}

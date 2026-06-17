use std::path::Path;
use std::sync::Mutex;

use chrono::{Duration, Utc};
use rusqlite::{params, params_from_iter, Connection, TransactionBehavior};

use crate::schema::{
    AnonymousEventOutcome, AnonymousFeedbackBatch, AnonymousFeedbackBatchAck,
    AnonymousFeedbackEvent, AnonymousFeedbackStatsQuery, AnonymousFeedbackStatsResponse,
    CloudAccount, CloudAccountSession, CloudCreatorProfile, CloudDevice, CloudEntitlement,
    CloudSyncBatchRequest, CloudSyncBatchResult, CloudSyncChange, CloudSyncChangesResult,
    CloudWorkspace, FeedbackStatRow, FeedbackTotals, StatsDimension,
};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("invalid retention days")]
    InvalidRetentionDays,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("bad request: {0}")]
    BadRequest(String),
}

pub struct Storage {
    conn: Mutex<Connection>,
    retention_days: i64,
}

impl Storage {
    pub fn open(path: impl AsRef<Path>, retention_days: i64) -> Result<Self, StorageError> {
        if retention_days <= 0 {
            return Err(StorageError::InvalidRetentionDays);
        }
        let conn = Connection::open(path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        init_schema(&conn)?;
        let storage = Self {
            conn: Mutex::new(conn),
            retention_days,
        };
        storage.cleanup_old_events()?;
        Ok(storage)
    }

    pub fn ingest_batch(
        &self,
        batch: &AnonymousFeedbackBatch,
    ) -> Result<AnonymousFeedbackBatchAck, StorageError> {
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let accepted_at = Utc::now();

        tx.execute(
            "INSERT INTO feedback_batches (
                request_id, install_id, session_id, app_version, sent_at, received_at,
                received_events, inserted_events, duplicate_events
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 0)",
            params![
                request_id,
                batch.install_id,
                batch.session_id,
                batch.app_version,
                batch.sent_at.to_rfc3339(),
                accepted_at.to_rfc3339(),
            ],
        )?;

        let mut inserted_events = 0usize;
        let mut duplicate_events = 0usize;
        for event in &batch.events {
            if insert_anonymous_event(&tx, event)? {
                inserted_events += 1;
            } else {
                duplicate_events += 1;
            }
        }

        tx.execute(
            "UPDATE feedback_batches
             SET received_events = ?2, inserted_events = ?3, duplicate_events = ?4
             WHERE request_id = ?1",
            params![
                request_id,
                batch.events.len() as i64,
                inserted_events as i64,
                duplicate_events as i64
            ],
        )?;

        tx.commit()?;
        Ok(AnonymousFeedbackBatchAck {
            request_id,
            received_events: batch.events.len(),
            inserted_events,
            duplicate_events,
            accepted_at,
        })
    }

    pub fn cleanup_old_events(&self) -> Result<usize, StorageError> {
        let cutoff = (Utc::now() - Duration::days(self.retention_days)).to_rfc3339();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let deleted = conn.execute(
            "DELETE FROM feedback_events WHERE occurred_at < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }

    pub fn query_stats(
        &self,
        query: &AnonymousFeedbackStatsQuery,
    ) -> Result<AnonymousFeedbackStatsResponse, StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let dimension = query.dimension.clone().unwrap_or_default();
        let (dimension_expr, label_expr) = dimension_sql(&dimension);
        let (where_sql, values) = build_filters(query);

        let totals_sql = format!(
            "SELECT
                COUNT(*) AS total_events,
                SUM(CASE WHEN outcome = 'success' THEN 1 ELSE 0 END) AS success_events,
                SUM(CASE WHEN outcome = 'failure' THEN 1 ELSE 0 END) AS failure_events,
                SUM(CASE WHEN outcome = 'crash' THEN 1 ELSE 0 END) AS crash_events,
                SUM(CASE WHEN outcome = 'diagnostic' THEN 1 ELSE 0 END) AS diagnostic_events,
                AVG(duration_ms) AS avg_duration_ms,
                MAX(occurred_at) AS last_event_at
             FROM feedback_events {}",
            where_sql
        );
        let mut totals_stmt = conn.prepare(&totals_sql)?;
        let totals = totals_stmt.query_row(params_from_iter(values.clone()), |row| {
            Ok(FeedbackTotals {
                total_events: row.get::<_, i64>(0).unwrap_or_default() as u64,
                success_events: row.get::<_, Option<i64>>(1)?.unwrap_or_default() as u64,
                failure_events: row.get::<_, Option<i64>>(2)?.unwrap_or_default() as u64,
                crash_events: row.get::<_, Option<i64>>(3)?.unwrap_or_default() as u64,
                diagnostic_events: row.get::<_, Option<i64>>(4)?.unwrap_or_default() as u64,
                avg_duration_ms: row.get::<_, Option<f64>>(5)?,
                last_event_at: row
                    .get::<_, Option<String>>(6)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            })
        })?;

        let grouped_sql = format!(
            "SELECT
                {label_expr} AS label,
                COUNT(*) AS total_events,
                SUM(CASE WHEN outcome = 'success' THEN 1 ELSE 0 END) AS success_events,
                SUM(CASE WHEN outcome = 'failure' THEN 1 ELSE 0 END) AS failure_events,
                SUM(CASE WHEN outcome = 'crash' THEN 1 ELSE 0 END) AS crash_events,
                SUM(CASE WHEN outcome = 'diagnostic' THEN 1 ELSE 0 END) AS diagnostic_events,
                AVG(duration_ms) AS avg_duration_ms,
                MAX(occurred_at) AS last_event_at
             FROM feedback_events
             {}
             GROUP BY {dimension_expr}
             ORDER BY {dimension_expr} ASC",
            where_sql
        );

        let mut stmt = conn.prepare(&grouped_sql)?;
        let rows = stmt
            .query_map(params_from_iter(values), |row| {
                Ok(FeedbackStatRow {
                    label: row.get::<_, String>(0)?,
                    total_events: row.get::<_, i64>(1).unwrap_or_default() as u64,
                    success_events: row.get::<_, Option<i64>>(2)?.unwrap_or_default() as u64,
                    failure_events: row.get::<_, Option<i64>>(3)?.unwrap_or_default() as u64,
                    crash_events: row.get::<_, Option<i64>>(4)?.unwrap_or_default() as u64,
                    diagnostic_events: row.get::<_, Option<i64>>(5)?.unwrap_or_default() as u64,
                    avg_duration_ms: row.get::<_, Option<f64>>(6)?,
                    last_event_at: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(AnonymousFeedbackStatsResponse {
            dimension: dimension.as_str().to_string(),
            totals,
            rows,
            generated_at: Utc::now(),
        })
    }

    pub fn continue_account(
        &self,
        request: &crate::schema::ContinueAccountRequest,
    ) -> Result<CloudAccountSession, StorageError> {
        let identifier = normalize_identifier(&request.identifier)?;
        let display_name = request
            .local_creator_profile
            .display_name
            .trim()
            .to_string();
        let creator_display_name = if display_name.is_empty() {
            identifier.clone()
        } else {
            display_name
        };
        let creator_seed_ref = request.local_creator_profile.creator_seed_ref.trim();
        let creator_seed_ref = if creator_seed_ref.is_empty() {
            "local-seed-ref".to_string()
        } else {
            creator_seed_ref.to_string()
        };

        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let now = Utc::now().to_rfc3339();
        let account = ensure_account(
            &tx,
            &identifier,
            &creator_display_name,
            &creator_seed_ref,
            request.local_creator_profile.seed_envelope_version,
            &now,
        )?;
        let device = ensure_device(&tx, &account.id, request, &now)?;
        let session = create_session(&tx, &account.id, &device.id, &now)?;
        tx.commit()?;

        Ok(CloudAccountSession {
            access_token: session.access_token,
            refresh_token: session.refresh_token,
            account: CloudAccount {
                id: account.id,
                display_name: account.display_name,
            },
            workspace: CloudWorkspace {
                id: account.workspace_id,
                name: account.workspace_name,
            },
            device: CloudDevice {
                id: device.id,
                name: Some(device.name),
                platform: Some(device.platform),
                registered: true,
            },
            creator_profile: CloudCreatorProfile {
                id: account.creator_profile_id,
                display_name: account.creator_display_name,
                is_default: true,
            },
            entitlement: CloudEntitlement {
                id: account.entitlement_id,
                plan_name: Some(account.entitlement_plan_name),
                plan_code: account.entitlement_plan_code,
                status: account.entitlement_status,
                features: serde_json::from_str(&account.entitlement_features_json)
                    .unwrap_or_else(|_| serde_json::json!({ "cloud_sync": true })),
            },
        })
    }

    pub fn push_cloud_events_batch(
        &self,
        access_token: &str,
        request: &CloudSyncBatchRequest,
    ) -> Result<CloudSyncBatchResult, StorageError> {
        let session = self.authenticate(access_token)?;
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
        if !self.session_workspace_matches(&session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        if request.events.is_empty() {
            return Err(StorageError::BadRequest(
                "events must not be empty".to_string(),
            ));
        }

        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut accepted_event_ids = Vec::new();

        for event in &request.events {
            let client_event_id = event.client_event_id.trim();
            let entity_type = event.entity_type.trim();
            let entity_id = event.entity_id.trim();
            if client_event_id.is_empty() || entity_type.is_empty() || entity_id.is_empty() {
                continue;
            }

            tx.execute(
                "INSERT OR IGNORE INTO cloud_sync_events (
                    account_id, device_id, client_event_id, operation, entity_type,
                    entity_id, payload_json, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    session.account_id,
                    session.device_id,
                    client_event_id,
                    event.operation.trim(),
                    entity_type,
                    entity_id,
                    serde_json::to_string(&event.payload).unwrap_or_else(|_| "{}".to_string()),
                    Utc::now().to_rfc3339(),
                ],
            )?;
            accepted_event_ids.push(client_event_id.to_string());
        }

        tx.commit()?;
        let next_cursor = account_cursor_with_conn(&conn, &session.account_id)?;
        Ok(CloudSyncBatchResult {
            accepted: accepted_event_ids.len() as u32,
            accepted_event_ids,
            next_cursor,
            resolutions: serde_json::json!([]),
        })
    }

    pub fn get_cloud_changes(
        &self,
        access_token: &str,
        workspace_id: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<CloudSyncChangesResult, StorageError> {
        let session = self.authenticate(access_token)?;
        let workspace_id = workspace_id.unwrap_or_default().trim();
        if workspace_id.is_empty() {
            return Err(StorageError::BadRequest(
                "workspaceId is required".to_string(),
            ));
        }
        if !self.session_workspace_matches(&session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        let since_sequence = sequence_from_cursor(cursor);

        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT sequence, device_id, operation, entity_type, entity_id, payload_json
             FROM cloud_sync_events
             WHERE account_id = ?1 AND sequence > ?2
             ORDER BY sequence ASC",
        )?;
        let rows = stmt.query_map(params![session.account_id, since_sequence], |row| {
            let payload_json: String = row.get(5)?;
            Ok(CloudSyncChange {
                cursor: Some(cursor_from_sequence(row.get::<_, i64>(0)? as u64)),
                entity_type: row.get(3)?,
                operation: cloud_operation(&row.get::<_, String>(2)?),
                source_device: Some(row.get(1)?),
                entity: serde_json::from_str(&payload_json)
                    .unwrap_or_else(|_| serde_json::json!({})),
            })
        })?;
        let changes = rows.collect::<Result<Vec<_>, _>>()?;

        let next_cursor =
            account_cursor_with_conn(&conn, &session.account_id)?.unwrap_or_else(|| {
                cursor
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| cursor_from_sequence(since_sequence as u64))
            });
        upsert_device_cursor(&conn, &session.account_id, &session.device_id, &next_cursor)?;

        Ok(CloudSyncChangesResult {
            next_cursor,
            changes,
        })
    }

    fn authenticate(&self, access_token: &str) -> Result<SessionRecord, StorageError> {
        let access_token = access_token.trim();
        if access_token.is_empty() {
            return Err(StorageError::Unauthorized);
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT account_id, device_id, revoked_at
             FROM cloud_sessions
             WHERE access_token = ?1",
        )?;
        let session = stmt.query_row(params![access_token], |row| {
            Ok(SessionRecord {
                account_id: row.get(0)?,
                device_id: row.get(1)?,
                revoked_at: row.get(2)?,
            })
        });
        match session {
            Ok(session) if session.revoked_at.is_none() => Ok(session),
            _ => Err(StorageError::Unauthorized),
        }
    }

    fn session_workspace_matches(
        &self,
        account_id: &str,
        workspace_id: &str,
    ) -> Result<bool, StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let stored_workspace_id = conn
            .query_row(
                "SELECT workspace_id FROM cloud_accounts WHERE id = ?1",
                params![account_id],
                |row| row.get::<_, String>(0),
            )
            .optional()?;
        Ok(stored_workspace_id.as_deref() == Some(workspace_id))
    }
}

#[derive(Debug, Clone)]
struct CloudAccountRow {
    id: String,
    display_name: String,
    workspace_id: String,
    workspace_name: String,
    creator_profile_id: String,
    creator_display_name: String,
    entitlement_id: String,
    entitlement_plan_name: String,
    entitlement_plan_code: String,
    entitlement_status: String,
    entitlement_features_json: String,
}

#[derive(Debug, Clone)]
struct CloudDeviceRow {
    id: String,
    name: String,
    platform: String,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    account_id: String,
    device_id: String,
    revoked_at: Option<String>,
}

fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS feedback_events (
            event_id TEXT PRIMARY KEY,
            occurred_at TEXT NOT NULL,
            install_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            app_version TEXT NOT NULL,
            feature_name TEXT NOT NULL,
            outcome TEXT NOT NULL,
            media_type TEXT NOT NULL,
            file_size_bucket TEXT NOT NULL,
            duration_ms INTEGER,
            error_code TEXT,
            diagnostic_note TEXT,
            stack_summary TEXT,
            pipeline_id TEXT,
            ingested_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS feedback_batches (
            request_id TEXT PRIMARY KEY,
            install_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            app_version TEXT NOT NULL,
            sent_at TEXT NOT NULL,
            received_at TEXT NOT NULL,
            received_events INTEGER NOT NULL,
            inserted_events INTEGER NOT NULL,
            duplicate_events INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_feedback_events_occurred_at ON feedback_events(occurred_at);
        CREATE INDEX IF NOT EXISTS idx_feedback_events_app_version ON feedback_events(app_version);
        CREATE INDEX IF NOT EXISTS idx_feedback_events_feature_name ON feedback_events(feature_name);
        CREATE INDEX IF NOT EXISTS idx_feedback_events_error_code ON feedback_events(error_code);
        CREATE INDEX IF NOT EXISTS idx_feedback_events_media_type ON feedback_events(media_type);
        CREATE INDEX IF NOT EXISTS idx_feedback_events_outcome ON feedback_events(outcome);

        CREATE TABLE IF NOT EXISTS cloud_accounts (
            id TEXT PRIMARY KEY,
            identifier TEXT NOT NULL UNIQUE,
            display_name TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            workspace_name TEXT NOT NULL,
            creator_profile_id TEXT NOT NULL,
            creator_display_name TEXT NOT NULL,
            creator_seed_ref TEXT NOT NULL,
            seed_envelope_version INTEGER NOT NULL,
            entitlement_id TEXT NOT NULL,
            entitlement_plan_name TEXT NOT NULL,
            entitlement_plan_code TEXT NOT NULL,
            entitlement_status TEXT NOT NULL,
            entitlement_features_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS cloud_devices (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            client_device_id TEXT NOT NULL,
            name TEXT NOT NULL,
            platform TEXT NOT NULL,
            app_version TEXT NOT NULL,
            public_key TEXT,
            registered INTEGER NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(account_id, client_device_id)
        );

        CREATE TABLE IF NOT EXISTS cloud_sessions (
            access_token TEXT PRIMARY KEY,
            refresh_token TEXT NOT NULL,
            account_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            revoked_at TEXT
        );

        CREATE TABLE IF NOT EXISTS cloud_sync_events (
            sequence INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            client_event_id TEXT NOT NULL,
            operation TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE(account_id, device_id, client_event_id)
        );

        CREATE TABLE IF NOT EXISTS cloud_device_cursors (
            account_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            cursor TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY(account_id, device_id)
        );

        CREATE INDEX IF NOT EXISTS idx_cloud_sync_events_account_sequence
        ON cloud_sync_events(account_id, sequence ASC);
        "#,
    )
}

fn insert_anonymous_event(
    tx: &rusqlite::Transaction<'_>,
    event: &AnonymousFeedbackEvent,
) -> Result<bool, rusqlite::Error> {
    let changed = tx.execute(
        "INSERT OR IGNORE INTO feedback_events (
            event_id, occurred_at, install_id, session_id, app_version, feature_name,
            outcome, media_type, file_size_bucket, duration_ms, error_code,
            diagnostic_note, stack_summary, pipeline_id, ingested_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            event.event_id,
            event.occurred_at.to_rfc3339(),
            event.install_id,
            event.session_id,
            event.app_version,
            event.feature_name,
            outcome_to_str(&event.outcome),
            event.media_type,
            event.file_size_bucket,
            event.duration_ms.map(|v| v as i64),
            event.error_code,
            event.diagnostic_note,
            event.stack_summary,
            event.pipeline_id,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(changed > 0)
}

fn ensure_account(
    tx: &rusqlite::Transaction<'_>,
    identifier: &str,
    creator_display_name: &str,
    creator_seed_ref: &str,
    seed_envelope_version: u32,
    now: &str,
) -> Result<CloudAccountRow, rusqlite::Error> {
    let account_id = tx
        .query_row(
            "SELECT id FROM cloud_accounts WHERE identifier = ?1",
            params![identifier],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    let account_id = account_id.unwrap_or_else(|| format!("acct_{}", short_id(identifier)));
    let workspace_id = format!("ws_{}", short_id(&account_id));
    let creator_profile_id = format!("creator_{}", short_id(&account_id));
    let entitlement_id = format!("ent_{}", short_id(&account_id));
    let display_name = identifier.to_string();
    let workspace_name = "个人空间".to_string();
    let entitlement_plan_name = "免费版".to_string();
    let entitlement_plan_code = "free".to_string();
    let entitlement_status = "free".to_string();
    let entitlement_features_json = serde_json::json!({
        "cloud_sync": true,
        "batch_processing": false,
        "cloud_video_processing": false
    })
    .to_string();

    tx.execute(
        "INSERT INTO cloud_accounts (
            id, identifier, display_name, workspace_id, workspace_name,
            creator_profile_id, creator_display_name, creator_seed_ref, seed_envelope_version,
            entitlement_id, entitlement_plan_name, entitlement_plan_code, entitlement_status,
            entitlement_features_json, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
        ON CONFLICT(identifier) DO UPDATE SET
            display_name = excluded.display_name,
            creator_display_name = excluded.creator_display_name,
            creator_seed_ref = excluded.creator_seed_ref,
            seed_envelope_version = excluded.seed_envelope_version,
            updated_at = excluded.updated_at",
        params![
            account_id,
            identifier,
            display_name,
            workspace_id,
            workspace_name,
            creator_profile_id,
            creator_display_name,
            creator_seed_ref,
            seed_envelope_version as i64,
            entitlement_id,
            entitlement_plan_name,
            entitlement_plan_code,
            entitlement_status,
            entitlement_features_json,
            now,
            now,
        ],
    )?;

    tx.query_row(
        "SELECT id, display_name, workspace_id, workspace_name, creator_profile_id,
                creator_display_name, entitlement_id, entitlement_plan_name,
                entitlement_plan_code, entitlement_status, entitlement_features_json
         FROM cloud_accounts WHERE identifier = ?1",
        params![identifier],
        |row| {
            Ok(CloudAccountRow {
                id: row.get(0)?,
                display_name: row.get(1)?,
                workspace_id: row.get(2)?,
                workspace_name: row.get(3)?,
                creator_profile_id: row.get(4)?,
                creator_display_name: row.get(5)?,
                entitlement_id: row.get(6)?,
                entitlement_plan_name: row.get(7)?,
                entitlement_plan_code: row.get(8)?,
                entitlement_status: row.get(9)?,
                entitlement_features_json: row.get(10)?,
            })
        },
    )
}

fn ensure_device(
    tx: &rusqlite::Transaction<'_>,
    account_id: &str,
    request: &crate::schema::ContinueAccountRequest,
    now: &str,
) -> Result<CloudDeviceRow, rusqlite::Error> {
    let device_id = request.device.client_device_id.trim();
    let name = request.device.name.trim();
    let platform = request.device.platform.trim();
    let app_version = request.device.app_version.trim();
    let public_key = request.device.public_key.clone();
    let device_id = if device_id.is_empty() {
        format!("device_{}", short_id(account_id))
    } else {
        device_id.to_string()
    };
    let device_name = if name.is_empty() {
        "当前设备".to_string()
    } else {
        name.to_string()
    };
    let platform = if platform.is_empty() {
        "unknown".to_string()
    } else {
        platform.to_string()
    };
    let app_version = if app_version.is_empty() {
        "0.1.0".to_string()
    } else {
        app_version.to_string()
    };

    tx.execute(
        "INSERT INTO cloud_devices (
            id, account_id, client_device_id, name, platform, app_version,
            public_key, registered, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, ?8, ?9)
        ON CONFLICT(account_id, client_device_id) DO UPDATE SET
            name = excluded.name,
            platform = excluded.platform,
            app_version = excluded.app_version,
            public_key = excluded.public_key,
            registered = excluded.registered,
            updated_at = excluded.updated_at",
        params![
            device_id,
            account_id,
            request.device.client_device_id.trim(),
            device_name,
            platform,
            app_version,
            public_key,
            now,
            now,
        ],
    )?;

    tx.query_row(
        "SELECT id, name, platform FROM cloud_devices
         WHERE account_id = ?1 AND client_device_id = ?2",
        params![account_id, request.device.client_device_id.trim()],
        |row| {
            Ok(CloudDeviceRow {
                id: row.get(0)?,
                name: row.get(1)?,
                platform: row.get(2)?,
            })
        },
    )
}

fn create_session(
    tx: &rusqlite::Transaction<'_>,
    account_id: &str,
    device_id: &str,
    now: &str,
) -> Result<SessionTokenRow, rusqlite::Error> {
    let access_token = format!("mock_{}_{}_{}", account_id, device_id, short_id(now));
    let refresh_token = format!("refresh_{}_{}", account_id, device_id);
    tx.execute(
        "INSERT INTO cloud_sessions (
            access_token, refresh_token, account_id, device_id, created_at, revoked_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, NULL)",
        params![access_token, refresh_token, account_id, device_id, now],
    )?;
    Ok(SessionTokenRow {
        access_token,
        refresh_token,
    })
}

fn upsert_device_cursor(
    tx: &rusqlite::Connection,
    account_id: &str,
    device_id: &str,
    cursor: &str,
) -> Result<(), rusqlite::Error> {
    let now = Utc::now().to_rfc3339();
    tx.execute(
        "INSERT INTO cloud_device_cursors (
            account_id, device_id, cursor, updated_at
        ) VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(account_id, device_id) DO UPDATE SET
            cursor = excluded.cursor,
            updated_at = excluded.updated_at",
        params![account_id, device_id, cursor, now],
    )?;
    Ok(())
}

fn account_cursor_with_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
) -> Result<Option<String>, StorageError> {
    let cursor = conn
        .query_row(
            "SELECT MAX(sequence) FROM cloud_sync_events WHERE account_id = ?1",
            params![account_id],
            |row| row.get::<_, Option<i64>>(0),
        )?
        .map(|value| cursor_from_sequence(value as u64));
    Ok(cursor)
}

fn outcome_to_str(outcome: &AnonymousEventOutcome) -> &'static str {
    match outcome {
        AnonymousEventOutcome::Success => "success",
        AnonymousEventOutcome::Failure => "failure",
        AnonymousEventOutcome::Crash => "crash",
        AnonymousEventOutcome::Diagnostic => "diagnostic",
    }
}

fn cloud_operation(operation: &str) -> String {
    if operation.starts_with("upsert") {
        "upsert".to_string()
    } else {
        operation.to_string()
    }
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

fn short_id(input: &str) -> String {
    let mut hash = 2166136261u32;
    for byte in input.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")
}

fn normalize_identifier(input: &str) -> Result<String, StorageError> {
    let identifier = input.trim().to_lowercase();
    if identifier.is_empty() {
        return Err(StorageError::BadRequest(
            "identifier is required".to_string(),
        ));
    }
    Ok(identifier)
}

trait OptionalRowExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalRowExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

#[derive(Debug, Clone)]
struct SessionTokenRow {
    access_token: String,
    refresh_token: String,
}

fn dimension_sql(dimension: &StatsDimension) -> (&'static str, &'static str) {
    match dimension {
        StatsDimension::Day => ("substr(occurred_at, 1, 10)", "substr(occurred_at, 1, 10)"),
        StatsDimension::Version => ("app_version", "app_version"),
        StatsDimension::Feature => ("feature_name", "feature_name"),
        StatsDimension::ErrorCode => (
            "coalesce(nullif(error_code, ''), 'none')",
            "coalesce(nullif(error_code, ''), 'none')",
        ),
        StatsDimension::MediaType => ("media_type", "media_type"),
        StatsDimension::Outcome => ("outcome", "outcome"),
    }
}

fn build_filters(query: &AnonymousFeedbackStatsQuery) -> (String, Vec<rusqlite::types::Value>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();

    if let Some(from) = query.from {
        clauses.push("occurred_at >= ?".to_string());
        values.push(rusqlite::types::Value::Text(from.to_rfc3339()));
    }
    if let Some(to) = query.to {
        clauses.push("occurred_at <= ?".to_string());
        values.push(rusqlite::types::Value::Text(to.to_rfc3339()));
    }
    if let Some(ref app_version) = query.app_version {
        clauses.push("app_version = ?".to_string());
        values.push(rusqlite::types::Value::Text(app_version.clone()));
    }
    if let Some(ref feature_name) = query.feature_name {
        clauses.push("feature_name = ?".to_string());
        values.push(rusqlite::types::Value::Text(feature_name.clone()));
    }
    if let Some(ref media_type) = query.media_type {
        clauses.push("media_type = ?".to_string());
        values.push(rusqlite::types::Value::Text(media_type.clone()));
    }
    if let Some(ref error_code) = query.error_code {
        clauses.push("error_code = ?".to_string());
        values.push(rusqlite::types::Value::Text(error_code.clone()));
    }
    if let Some(ref outcome) = query.outcome {
        clauses.push("outcome = ?".to_string());
        values.push(rusqlite::types::Value::Text(
            outcome_to_str(outcome).to_string(),
        ));
    }

    if clauses.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn sample_event(event_id: &str, outcome: AnonymousEventOutcome) -> AnonymousFeedbackEvent {
        AnonymousFeedbackEvent {
            event_id: event_id.to_string(),
            occurred_at: Utc::now(),
            install_id: "inst-1".to_string(),
            session_id: "sess-1".to_string(),
            app_version: "0.1.0".to_string(),
            feature_name: "watermark_video".to_string(),
            outcome,
            media_type: "video".to_string(),
            file_size_bucket: "50-200mb".to_string(),
            duration_ms: Some(1234),
            error_code: None,
            diagnostic_note: None,
            stack_summary: None,
            pipeline_id: Some("pipe-1".to_string()),
        }
    }

    fn sample_continue_request(
        identifier: &str,
        device_id: &str,
    ) -> crate::schema::ContinueAccountRequest {
        crate::schema::ContinueAccountRequest {
            identifier: identifier.to_string(),
            verification_code: "000000".to_string(),
            device: crate::schema::ContinueAccountDevice {
                client_device_id: device_id.to_string(),
                name: "Device".to_string(),
                platform: "contract".to_string(),
                app_version: "0.1.0".to_string(),
                public_key: None,
            },
            local_creator_profile: crate::schema::ContinueAccountCreatorProfile {
                display_name: "Creator".to_string(),
                creator_seed_ref: "seed-ref".to_string(),
                seed_envelope_version: 1,
            },
        }
    }

    #[test]
    fn ingests_events_and_dedupes() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let batch = AnonymousFeedbackBatch {
            install_id: "inst-1".to_string(),
            session_id: "sess-1".to_string(),
            app_version: "0.1.0".to_string(),
            sent_at: Utc::now(),
            events: vec![
                sample_event("evt-1", AnonymousEventOutcome::Success),
                sample_event("evt-1", AnonymousEventOutcome::Success),
            ],
        };

        let ack = storage.ingest_batch(&batch).unwrap();
        assert_eq!(ack.received_events, 2);
        assert_eq!(ack.inserted_events, 1);
        assert_eq!(ack.duplicate_events, 1);
    }

    #[test]
    fn stats_group_by_day() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let batch = AnonymousFeedbackBatch {
            install_id: "inst-1".to_string(),
            session_id: "sess-1".to_string(),
            app_version: "0.1.0".to_string(),
            sent_at: Utc::now(),
            events: vec![sample_event("evt-2", AnonymousEventOutcome::Failure)],
        };
        storage.ingest_batch(&batch).unwrap();

        let stats = storage
            .query_stats(&AnonymousFeedbackStatsQuery::default())
            .unwrap();
        assert_eq!(stats.totals.total_events, 1);
        assert_eq!(stats.totals.failure_events, 1);
        assert_eq!(stats.rows.len(), 1);
    }

    #[test]
    fn continue_account_returns_session_and_persists() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        assert_eq!(session.account.display_name, "alice@example.com");
        assert_eq!(session.workspace.name, "个人空间");
        assert_eq!(session.device.id, "dev-1");
        assert_eq!(session.creator_profile.is_default, true);
        assert_eq!(session.entitlement.plan_code, "free");
    }

    #[test]
    fn push_and_pull_cloud_events_round_trip() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let batch = CloudSyncBatchRequest {
            device_id: "dev-1".to_string(),
            workspace_id: session.workspace.id.clone(),
            events: vec![crate::schema::CloudSyncEvent {
                client_event_id: "evt-1".to_string(),
                operation: "upsertVaultRecord".to_string(),
                entity_type: "vaultRecord".to_string(),
                entity_id: "record-1".to_string(),
                payload: serde_json::json!({
                    "id": "record-1",
                    "kind": "image",
                    "title": "demo.png",
                    "watermark_uid": "uid-1",
                    "revision": 1,
                    "sha256": "hash-1",
                    "created_at": "2026-06-16T12:00:00Z"
                }),
            }],
        };
        let result = storage
            .push_cloud_events_batch(&session.access_token, &batch)
            .unwrap();
        assert_eq!(result.accepted, 1);
        assert_eq!(result.accepted_event_ids, vec!["evt-1".to_string()]);

        let changes = storage
            .get_cloud_changes(&session.access_token, Some(&session.workspace.id), None)
            .unwrap();
        assert_eq!(changes.changes.len(), 1);
        assert_eq!(changes.changes[0].entity_type, "vaultRecord");
        assert_eq!(changes.changes[0].operation, "upsert");
        assert_eq!(changes.changes[0].entity["watermark_uid"], "uid-1");
    }

    #[test]
    fn push_cloud_events_rejects_device_and_workspace_mismatch() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let event = crate::schema::CloudSyncEvent {
            client_event_id: "evt-guard".to_string(),
            operation: "upsertVaultRecord".to_string(),
            entity_type: "vaultRecord".to_string(),
            entity_id: "record-guard".to_string(),
            payload: serde_json::json!({
                "id": "record-guard",
                "kind": "image",
                "title": "guard.png",
                "watermark_uid": "uid-guard",
                "revision": 1,
                "created_at": "2026-06-16T12:00:00Z"
            }),
        };
        let wrong_device = CloudSyncBatchRequest {
            device_id: "dev-other".to_string(),
            workspace_id: session.workspace.id.clone(),
            events: vec![event.clone()],
        };
        assert!(matches!(
            storage.push_cloud_events_batch(&session.access_token, &wrong_device),
            Err(StorageError::Unauthorized)
        ));

        let wrong_workspace = CloudSyncBatchRequest {
            device_id: "dev-1".to_string(),
            workspace_id: "ws-other".to_string(),
            events: vec![event],
        };
        assert!(matches!(
            storage.push_cloud_events_batch(&session.access_token, &wrong_workspace),
            Err(StorageError::Forbidden)
        ));

        assert!(matches!(
            storage.get_cloud_changes(&session.access_token, Some("ws-other"), None),
            Err(StorageError::Forbidden)
        ));
    }
}

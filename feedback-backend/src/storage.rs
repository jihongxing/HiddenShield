use std::path::Path;
use std::sync::Mutex;

use chrono::{Duration, Utc};
use rusqlite::{params, params_from_iter, Connection, TransactionBehavior};

use crate::schema::{
    AnonymousFeedbackBatch, AnonymousFeedbackBatchAck, AnonymousFeedbackEvent,
    AnonymousFeedbackStatsQuery, AnonymousFeedbackStatsResponse, AnonymousEventOutcome,
    FeedbackStatRow, FeedbackTotals, StatsDimension,
};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("invalid retention days")]
    InvalidRetentionDays,
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
            if insert_event(&tx, event)? {
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
        "#,
    )
}

fn insert_event(
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

fn outcome_to_str(outcome: &AnonymousEventOutcome) -> &'static str {
    match outcome {
        AnonymousEventOutcome::Success => "success",
        AnonymousEventOutcome::Failure => "failure",
        AnonymousEventOutcome::Crash => "crash",
        AnonymousEventOutcome::Diagnostic => "diagnostic",
    }
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
        values.push(rusqlite::types::Value::Text(outcome_to_str(outcome).to_string()));
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
    use crate::schema::{AnonymousFeedbackEvent, AnonymousFeedbackBatch, AnonymousEventOutcome};
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

        let stats = storage.query_stats(&AnonymousFeedbackStatsQuery::default()).unwrap();
        assert_eq!(stats.totals.total_events, 1);
        assert_eq!(stats.totals.failure_events, 1);
        assert_eq!(stats.rows.len(), 1);
    }
}

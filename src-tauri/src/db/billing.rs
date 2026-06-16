use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use serde::{Deserialize, Serialize};

use crate::commands::vault::VaultRecord;

#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum EntitlementStatus {
    #[default]
    Free,
    Trial,
    Active,
    Grace,
    Expired,
}

impl EntitlementStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Free => "free",
            Self::Trial => "trial",
            Self::Active => "active",
            Self::Grace => "grace",
            Self::Expired => "expired",
        }
    }

    pub fn from_db(value: &str) -> Self {
        match value {
            "trial" => Self::Trial,
            "active" => Self::Active,
            "grace" => Self::Grace,
            "expired" => Self::Expired,
            _ => Self::Free,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntitlementState {
    pub status: EntitlementStatus,
    pub plan_name: Option<String>,
    pub billing_source: Option<String>,
    pub subscription_id: Option<String>,
    pub trial_started_at: Option<String>,
    pub trial_ends_at: Option<String>,
    pub current_period_started_at: Option<String>,
    pub current_period_ends_at: Option<String>,
    pub grace_ends_at: Option<String>,
    pub last_checked_at: Option<String>,
    pub updated_at: String,
}

impl Default for EntitlementState {
    fn default() -> Self {
        Self {
            status: EntitlementStatus::Free,
            plan_name: None,
            billing_source: None,
            subscription_id: None,
            trial_started_at: None,
            trial_ends_at: None,
            current_period_started_at: None,
            current_period_ends_at: None,
            grace_ends_at: None,
            last_checked_at: None,
            updated_at: Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLedgerEntry {
    pub occurred_at: String,
    pub feature_name: String,
    pub media_type: String,
    pub file_size_bucket: String,
    pub quantity: i64,
    pub event_type: String,
    pub entitlement_status: EntitlementStatus,
    pub billing_source: Option<String>,
    pub plan_name: Option<String>,
    pub subscription_id: Option<String>,
    pub pipeline_id: Option<String>,
    pub vault_record_id: Option<i64>,
    pub app_version: String,
}

impl UsageLedgerEntry {
    pub fn success(
        feature_name: impl Into<String>,
        media_type: impl Into<String>,
        file_size_bytes: u64,
        entitlement: &EntitlementState,
        pipeline_id: Option<String>,
    ) -> Self {
        Self {
            occurred_at: Utc::now().to_rfc3339(),
            feature_name: feature_name.into(),
            media_type: media_type.into(),
            file_size_bucket: bucket_file_size(file_size_bytes).to_string(),
            quantity: 1,
            event_type: "success".to_string(),
            entitlement_status: entitlement.status.clone(),
            billing_source: entitlement.billing_source.clone(),
            plan_name: entitlement.plan_name.clone(),
            subscription_id: entitlement.subscription_id.clone(),
            pipeline_id,
            vault_record_id: None,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLedgerSummary {
    pub total_units: i64,
    pub total_events: i64,
    pub image_units: i64,
    pub video_units: i64,
    pub audio_units: i64,
    pub last_used_at: Option<String>,
    pub last_feature_name: Option<String>,
    pub entitlement: EntitlementState,
}

fn row_to_entitlement(row: &rusqlite::Row<'_>) -> Result<EntitlementState, rusqlite::Error> {
    Ok(EntitlementState {
        status: EntitlementStatus::from_db(&row.get::<_, String>(0)?),
        plan_name: row.get(1)?,
        billing_source: row.get(2)?,
        subscription_id: row.get(3)?,
        trial_started_at: row.get(4)?,
        trial_ends_at: row.get(5)?,
        current_period_started_at: row.get(6)?,
        current_period_ends_at: row.get(7)?,
        grace_ends_at: row.get(8)?,
        last_checked_at: row.get(9)?,
        updated_at: row.get(10)?,
    })
}

fn entitlement_columns() -> &'static str {
    "status, plan_name, billing_source, subscription_id, trial_started_at, trial_ends_at, current_period_started_at, current_period_ends_at, grace_ends_at, last_checked_at, updated_at"
}

/// Return the current entitlement state, seeding the default free state if needed.
pub fn get_entitlement_state(conn: &Connection) -> Result<EntitlementState, rusqlite::Error> {
    let current = conn
        .query_row(
            &format!(
                "SELECT {} FROM entitlement_state WHERE id = 1",
                entitlement_columns()
            ),
            [],
            row_to_entitlement,
        )
        .optional()?;

    if let Some(state) = current {
        return Ok(state);
    }

    let default_state = EntitlementState::default();
    save_entitlement_state(conn, &default_state)?;
    Ok(default_state)
}

/// Replace the current entitlement state snapshot.
pub fn save_entitlement_state(
    conn: &Connection,
    state: &EntitlementState,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO entitlement_state (
            id, status, plan_name, billing_source, subscription_id,
            trial_started_at, trial_ends_at, current_period_started_at,
            current_period_ends_at, grace_ends_at, last_checked_at, updated_at
        ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        ON CONFLICT(id) DO UPDATE SET
            status = excluded.status,
            plan_name = excluded.plan_name,
            billing_source = excluded.billing_source,
            subscription_id = excluded.subscription_id,
            trial_started_at = excluded.trial_started_at,
            trial_ends_at = excluded.trial_ends_at,
            current_period_started_at = excluded.current_period_started_at,
            current_period_ends_at = excluded.current_period_ends_at,
            grace_ends_at = excluded.grace_ends_at,
            last_checked_at = excluded.last_checked_at,
            updated_at = excluded.updated_at",
        params![
            state.status.as_str(),
            state.plan_name,
            state.billing_source,
            state.subscription_id,
            state.trial_started_at,
            state.trial_ends_at,
            state.current_period_started_at,
            state.current_period_ends_at,
            state.grace_ends_at,
            state.last_checked_at,
            state.updated_at,
        ],
    )?;
    Ok(())
}

/// Insert a usage ledger row.
pub fn append_usage_entry(
    conn: &Connection,
    entry: &UsageLedgerEntry,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO usage_ledger (
            occurred_at, feature_name, media_type, file_size_bucket, quantity,
            event_type, entitlement_status, billing_source, plan_name,
            subscription_id, pipeline_id, vault_record_id, app_version
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            entry.occurred_at,
            entry.feature_name,
            entry.media_type,
            entry.file_size_bucket,
            entry.quantity,
            entry.event_type,
            entry.entitlement_status.as_str(),
            entry.billing_source,
            entry.plan_name,
            entry.subscription_id,
            entry.pipeline_id,
            entry.vault_record_id,
            entry.app_version,
        ],
    )?;
    Ok(())
}

pub fn append_usage_entry_tx(
    tx: &Transaction<'_>,
    entry: &UsageLedgerEntry,
) -> Result<(), rusqlite::Error> {
    tx.execute(
        "INSERT INTO usage_ledger (
            occurred_at, feature_name, media_type, file_size_bucket, quantity,
            event_type, entitlement_status, billing_source, plan_name,
            subscription_id, pipeline_id, vault_record_id, app_version
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            entry.occurred_at,
            entry.feature_name,
            entry.media_type,
            entry.file_size_bucket,
            entry.quantity,
            entry.event_type,
            entry.entitlement_status.as_str(),
            entry.billing_source,
            entry.plan_name,
            entry.subscription_id,
            entry.pipeline_id,
            entry.vault_record_id,
            entry.app_version,
        ],
    )?;
    Ok(())
}

/// Atomically persist a successful vault record and its corresponding usage entry.
pub fn insert_record_and_usage(
    conn: &mut Connection,
    record: &VaultRecord,
    mut usage_entry: UsageLedgerEntry,
) -> Result<i64, rusqlite::Error> {
    let tx = conn.transaction()?;
    let record_id = crate::db::queries::insert_record_tx(&tx, record)?;
    usage_entry.vault_record_id = Some(record_id);
    append_usage_entry_tx(&tx, &usage_entry)?;
    tx.commit()?;
    Ok(record_id)
}

/// Return a lightweight usage summary for UI / diagnostics.
pub fn get_usage_summary(conn: &Connection) -> Result<UsageLedgerSummary, rusqlite::Error> {
    let entitlement = get_entitlement_state(conn)?;

    let counts = conn.query_row(
        "SELECT
            COALESCE(SUM(quantity), 0) AS total_units,
            COUNT(*) AS total_events,
            COALESCE(SUM(CASE WHEN media_type = 'image' THEN quantity ELSE 0 END), 0) AS image_units,
            COALESCE(SUM(CASE WHEN media_type = 'video' THEN quantity ELSE 0 END), 0) AS video_units,
            COALESCE(SUM(CASE WHEN media_type = 'audio' THEN quantity ELSE 0 END), 0) AS audio_units
         FROM usage_ledger",
        [],
        |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, i64>(4)?,
            ))
        },
    )?;

    let latest = conn
        .query_row(
            "SELECT occurred_at, feature_name
             FROM usage_ledger
             ORDER BY occurred_at DESC, id DESC
             LIMIT 1",
            [],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;

    Ok(UsageLedgerSummary {
        total_units: counts.0,
        total_events: counts.1,
        image_units: counts.2,
        video_units: counts.3,
        audio_units: counts.4,
        last_used_at: latest.as_ref().map(|(occurred_at, _)| occurred_at.clone()),
        last_feature_name: latest.as_ref().map(|(_, feature)| feature.clone()),
        entitlement,
    })
}

/// Convert bytes to a coarse file-size bucket for anonymous usage stats.
pub fn bucket_file_size(bytes: u64) -> &'static str {
    const MB: u64 = 1024 * 1024;
    if bytes <= 10 * MB {
        "0-10mb"
    } else if bytes <= 50 * MB {
        "10-50mb"
    } else if bytes <= 200 * MB {
        "50-200mb"
    } else if bytes <= 500 * MB {
        "200-500mb"
    } else {
        "500mb+"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    #[test]
    fn default_entitlement_state_is_seeded() {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();

        let state = get_entitlement_state(&conn).unwrap();
        assert_eq!(state.status, EntitlementStatus::Free);
    }

    #[test]
    fn usage_summary_reflects_inserted_rows() {
        let mut conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();

        let entitlement = get_entitlement_state(&conn).unwrap();
        let record = VaultRecord {
            id: 0,
            original_hash: "aa11".to_string(),
            file_name: "sample.mp4".to_string(),
            created_at: "2026-05-03T12:00:00Z".to_string(),
            duration_secs: 1.0,
            resolution: "1920x1080".to_string(),
            watermark_uid: "HS-TEST".to_string(),
            thumbnail_path: None,
            output_douyin: None,
            output_bilibili: None,
            output_xhs: None,
            is_hdr_source: false,
            hw_encoder_used: None,
            process_time_ms: None,
            tsa_token_path: None,
            network_time: None,
            tsa_source: None,
            tsa_request_nonce: None,
            is_ai_generated: false,
            ai_training_permission: None,
            ai_generation_method: None,
            human_modification_level: None,
            authenticity_claim: None,
            custom_metadata: None,
            output_douyin_hash: None,
            output_bilibili_hash: None,
            output_xhs_hash: None,
            parent_watermark_uid: None,
            revision: 1,
            rewrite_reason: None,
        };

        let usage = UsageLedgerEntry::success(
            "watermark_video",
            "video",
            42 * 1024 * 1024,
            &entitlement,
            Some("pipe-1".to_string()),
        );
        let record_id = insert_record_and_usage(&mut conn, &record, usage).unwrap();
        assert!(record_id > 0);

        let summary = get_usage_summary(&conn).unwrap();
        assert_eq!(summary.total_units, 1);
        assert_eq!(summary.total_events, 1);
        assert_eq!(summary.video_units, 1);
    }

    #[test]
    fn entitlement_state_round_trips_all_statuses() {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();

        let statuses = [
            EntitlementStatus::Free,
            EntitlementStatus::Trial,
            EntitlementStatus::Active,
            EntitlementStatus::Grace,
            EntitlementStatus::Expired,
        ];

        for (index, status) in statuses.into_iter().enumerate() {
            let state = EntitlementState {
                status: status.clone(),
                plan_name: Some(format!("plan-{index}")),
                billing_source: Some("manual".to_string()),
                subscription_id: Some(format!("sub-{index}")),
                trial_started_at: Some("2026-05-03T00:00:00Z".to_string()),
                trial_ends_at: Some("2026-06-03T00:00:00Z".to_string()),
                current_period_started_at: Some("2026-05-03T00:00:00Z".to_string()),
                current_period_ends_at: Some("2026-06-03T00:00:00Z".to_string()),
                grace_ends_at: Some("2026-06-10T00:00:00Z".to_string()),
                last_checked_at: Some("2026-05-03T12:00:00Z".to_string()),
                updated_at: format!("2026-05-03T12:00:{index:02}Z"),
            };
            save_entitlement_state(&conn, &state).unwrap();

            let loaded = get_entitlement_state(&conn).unwrap();
            assert_eq!(loaded.status, state.status);
            assert_eq!(loaded.plan_name, state.plan_name);
            assert_eq!(loaded.subscription_id, state.subscription_id);

            let summary = get_usage_summary(&conn).unwrap();
            assert_eq!(summary.entitlement.status, state.status);
        }
    }
}

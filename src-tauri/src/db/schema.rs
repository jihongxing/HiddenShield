/// Current schema version. Increment when adding migrations.
#[allow(dead_code)]
pub const CURRENT_VERSION: u32 = 20;

/// Base schema (version 0 → 1): initial vault_records table.
pub const VAULT_RECORDS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS vault_records (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  original_hash TEXT NOT NULL,
  file_name TEXT NOT NULL,
  file_type TEXT NOT NULL DEFAULT 'video',
  created_at TEXT NOT NULL,
  duration_secs REAL,
  resolution TEXT,
  watermark_uid TEXT NOT NULL,
  thumbnail_path TEXT,
  output_douyin TEXT,
  output_bilibili TEXT,
  output_xhs TEXT,
  is_hdr_source INTEGER DEFAULT 0,
  hw_encoder_used TEXT,
  process_time_ms INTEGER
);

CREATE INDEX IF NOT EXISTS idx_vault_hash ON vault_records(original_hash);
CREATE INDEX IF NOT EXISTS idx_vault_created ON vault_records(created_at);
CREATE INDEX IF NOT EXISTS idx_vault_watermark ON vault_records(watermark_uid);
"#;

/// Run all necessary migrations to bring the database from its current version
/// to `CURRENT_VERSION`. Uses `PRAGMA user_version` to track state.
pub fn run_migrations(conn: &rusqlite::Connection) -> Result<(), rusqlite::Error> {
    let current: u32 = conn.pragma_query_value(None, "user_version", |row| row.get(0))?;

    if current < 1 {
        // Fresh database or pre-migration database — apply base schema
        conn.execute_batch(VAULT_RECORDS_SCHEMA)?;
        set_user_version(conn, 1)?;
    }

    if current < 2 {
        // Add TSA (Trusted Timestamp Authority) and network time columns
        conn.execute_batch(
            "ALTER TABLE vault_records ADD COLUMN tsa_token_path TEXT;
             ALTER TABLE vault_records ADD COLUMN network_time TEXT;
             ALTER TABLE vault_records ADD COLUMN tsa_source TEXT;",
        )?;
        set_user_version(conn, 2)?;
    }

    if current < 3 {
        conn.execute_batch("ALTER TABLE vault_records ADD COLUMN tsa_request_nonce TEXT;")?;
        set_user_version(conn, 3)?;
    }

    if current < 4 {
        // Add AI content identification fields (vendor-agnostic)
        conn.execute_batch(
            "ALTER TABLE vault_records ADD COLUMN is_ai_generated INTEGER NOT NULL DEFAULT 0;
             ALTER TABLE vault_records ADD COLUMN ai_training_permission TEXT;
             ALTER TABLE vault_records ADD COLUMN ai_generation_method TEXT;
             ALTER TABLE vault_records ADD COLUMN human_modification_level TEXT;
             ALTER TABLE vault_records ADD COLUMN authenticity_claim TEXT;
             ALTER TABLE vault_records ADD COLUMN custom_metadata TEXT;",
        )?;
        // Add indexes for AI content queries
        conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_ai_generated ON vault_records(is_ai_generated);
             CREATE INDEX IF NOT EXISTS idx_ai_generation_method ON vault_records(ai_generation_method);
             CREATE INDEX IF NOT EXISTS idx_ai_training_permission ON vault_records(ai_training_permission);",
        )?;
        set_user_version(conn, 4)?;
    }

    if current < 5 {
        // Add output file hash fields for asset binding verification
        conn.execute_batch(
            "ALTER TABLE vault_records ADD COLUMN output_douyin_hash TEXT;
             ALTER TABLE vault_records ADD COLUMN output_bilibili_hash TEXT;
             ALTER TABLE vault_records ADD COLUMN output_xhs_hash TEXT;",
        )?;
        set_user_version(conn, 5)?;
    }

    if current < 6 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS entitlement_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                status TEXT NOT NULL,
                plan_name TEXT,
                billing_source TEXT,
                subscription_id TEXT,
                trial_started_at TEXT,
                trial_ends_at TEXT,
                current_period_started_at TEXT,
                current_period_ends_at TEXT,
                grace_ends_at TEXT,
                last_checked_at TEXT,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS usage_ledger (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                occurred_at TEXT NOT NULL,
                feature_name TEXT NOT NULL,
                media_type TEXT NOT NULL,
                file_size_bucket TEXT NOT NULL,
                quantity INTEGER NOT NULL DEFAULT 1,
                event_type TEXT NOT NULL DEFAULT 'success',
                entitlement_status TEXT NOT NULL,
                billing_source TEXT,
                plan_name TEXT,
                subscription_id TEXT,
                pipeline_id TEXT,
                vault_record_id INTEGER,
                app_version TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_usage_ledger_occurred_at ON usage_ledger(occurred_at);
            CREATE INDEX IF NOT EXISTS idx_usage_ledger_feature ON usage_ledger(feature_name);
            CREATE INDEX IF NOT EXISTS idx_usage_ledger_media_type ON usage_ledger(media_type);

            INSERT OR IGNORE INTO entitlement_state (
                id, status, plan_name, billing_source, subscription_id,
                trial_started_at, trial_ends_at, current_period_started_at,
                current_period_ends_at, grace_ends_at, last_checked_at, updated_at
            ) VALUES (
                1, 'free', NULL, NULL, NULL,
                NULL, NULL, NULL,
                NULL, NULL, NULL, CURRENT_TIMESTAMP
            );",
        )?;
        set_user_version(conn, 6)?;
    }

    if current < 7 {
        conn.execute_batch(
            "ALTER TABLE vault_records ADD COLUMN parent_watermark_uid TEXT;
             ALTER TABLE vault_records ADD COLUMN revision INTEGER NOT NULL DEFAULT 1;
             ALTER TABLE vault_records ADD COLUMN rewrite_reason TEXT;

             CREATE INDEX IF NOT EXISTS idx_vault_parent_watermark
             ON vault_records(parent_watermark_uid);",
        )?;
        set_user_version(conn, 7)?;
    }

    if current < 8 {
        conn.execute_batch(
            "ALTER TABLE vault_records ADD COLUMN write_verification_status TEXT;
             ALTER TABLE vault_records ADD COLUMN write_verification_message TEXT;
             ALTER TABLE vault_records ADD COLUMN write_verification_at TEXT;

             CREATE INDEX IF NOT EXISTS idx_vault_write_verification_status
             ON vault_records(write_verification_status);",
        )?;
        set_user_version(conn, 8)?;
    }

    if current < 9 {
        conn.execute_batch(
            "ALTER TABLE entitlement_state ADD COLUMN plan_code TEXT;
             ALTER TABLE entitlement_state ADD COLUMN features_json TEXT NOT NULL DEFAULT '{}';
             UPDATE entitlement_state
             SET plan_code = COALESCE(plan_code, 'free'),
                 features_json = CASE
                    WHEN features_json IS NULL OR features_json = '{}' THEN '{\"cloud_sync\":false,\"batch_processing\":false,\"report_export\":false,\"cloud_batch_processing\":false,\"cloud_video_processing\":false,\"priority_queue\":false,\"team_workspace\":false,\"api_access\":false}'
                    ELSE features_json
                 END;
             ALTER TABLE usage_ledger ADD COLUMN plan_code TEXT;",
        )?;
        set_user_version(conn, 9)?;
    }

    if current < 10 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS local_batch_jobs (
                id TEXT PRIMARY KEY,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                entitlement_plan_code TEXT NOT NULL,
                entitlement_status TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS local_batch_items (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                input_ref TEXT NOT NULL,
                file_name TEXT NOT NULL,
                media_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                attempts INTEGER NOT NULL DEFAULT 0,
                last_error TEXT,
                output_ref TEXT,
                vault_record_id INTEGER,
                write_verification_status TEXT,
                write_verification_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(job_id) REFERENCES local_batch_jobs(id) ON DELETE CASCADE
            );

            CREATE INDEX IF NOT EXISTS idx_local_batch_jobs_updated
            ON local_batch_jobs(updated_at DESC);

            CREATE INDEX IF NOT EXISTS idx_local_batch_items_job
            ON local_batch_items(job_id, updated_at ASC);",
        )?;
        set_user_version(conn, 10)?;
    }

    if current < 11 {
        conn.execute_batch(
            "UPDATE entitlement_state
             SET features_json = REPLACE(features_json, '\"cloud_sync\":true', '\"cloud_sync\":false')
             WHERE status = 'free' AND plan_code = 'free' AND features_json LIKE '%\"cloud_sync\":true%';",
        )?;
        set_user_version(conn, 11)?;
    }

    if current < 12 {
        conn.execute_batch(
            "ALTER TABLE vault_records ADD COLUMN video_notary_id TEXT;
             ALTER TABLE vault_records ADD COLUMN video_notary_at TEXT;
             ALTER TABLE vault_records ADD COLUMN video_notary_receipt_signature TEXT;
             ALTER TABLE vault_records ADD COLUMN video_notary_usage_ledger_id TEXT;
             ALTER TABLE vault_records ADD COLUMN video_fingerprint_root TEXT;
             ALTER TABLE vault_records ADD COLUMN video_bundle_sha256 TEXT;
             ALTER TABLE vault_records ADD COLUMN video_bundle_bytes INTEGER;
             ALTER TABLE vault_records ADD COLUMN video_bundle_scene_count INTEGER;
             ALTER TABLE vault_records ADD COLUMN video_bundle_elapsed_ms INTEGER;
             ALTER TABLE vault_records ADD COLUMN video_frame_sample_policy TEXT;

             CREATE INDEX IF NOT EXISTS idx_vault_video_notary_id
             ON vault_records(video_notary_id);",
        )?;
        set_user_version(conn, 12)?;
    }

    if current < 13 {
        conn.execute_batch("ALTER TABLE vault_records ADD COLUMN creator_display_name TEXT;")?;
        set_user_version(conn, 13)?;
    }

    if current < 14 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS report_purchase_grants (
                grant_id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                workspace_id TEXT NOT NULL,
                creator_profile_id TEXT NOT NULL,
                vault_record_id TEXT NOT NULL,
                product_code TEXT NOT NULL,
                price_cents INTEGER NOT NULL,
                currency TEXT NOT NULL,
                status TEXT NOT NULL,
                granted_at TEXT NOT NULL,
                revoked_at TEXT,
                updated_at TEXT NOT NULL,
                UNIQUE(account_id, workspace_id, vault_record_id, product_code)
            );

            CREATE INDEX IF NOT EXISTS idx_report_purchase_grants_record
            ON report_purchase_grants(account_id, workspace_id, vault_record_id, status);",
        )?;
        set_user_version(conn, 14)?;
    }

    if current < 15 {
        conn.execute_batch(
            "ALTER TABLE vault_records ADD COLUMN protected_copy_name TEXT;
             ALTER TABLE vault_records ADD COLUMN protected_copy_path TEXT;
             ALTER TABLE vault_records ADD COLUMN protected_copy_hash TEXT;
             ALTER TABLE vault_records ADD COLUMN output_strategy TEXT NOT NULL DEFAULT 'minimal_required_change';
             ALTER TABLE vault_records ADD COLUMN work_source_declaration TEXT NOT NULL DEFAULT 'unspecified';
             ALTER TABLE vault_records ADD COLUMN training_permission_declaration TEXT NOT NULL DEFAULT 'prohibited';
             ALTER TABLE vault_records ADD COLUMN creation_method_declaration TEXT NOT NULL DEFAULT 'unspecified';
             ALTER TABLE vault_records ADD COLUMN human_edit_level_declaration TEXT NOT NULL DEFAULT 'unspecified';
             ALTER TABLE vault_records ADD COLUMN authenticity_claim_declaration TEXT NOT NULL DEFAULT 'unspecified';
             ALTER TABLE vault_records ADD COLUMN custom_rights_statement TEXT;

             UPDATE vault_records
             SET protected_copy_path = COALESCE(protected_copy_path, output_douyin, output_bilibili, output_xhs),
                 protected_copy_hash = COALESCE(protected_copy_hash, output_douyin_hash, output_bilibili_hash, output_xhs_hash),
                 protected_copy_name = COALESCE(
                    protected_copy_name,
                    CASE
                        WHEN COALESCE(output_douyin, output_bilibili, output_xhs) IS NOT NULL
                        THEN '历史保护副本'
                        ELSE NULL
                    END
                 ),
                 work_source_declaration = CASE
                    WHEN is_ai_generated = 1 THEN 'ai_generated'
                    ELSE COALESCE(NULLIF(work_source_declaration, ''), 'unspecified')
                 END,
                 training_permission_declaration = COALESCE(NULLIF(ai_training_permission, ''), training_permission_declaration, 'prohibited'),
                 creation_method_declaration = COALESCE(NULLIF(ai_generation_method, ''), creation_method_declaration, 'unspecified'),
                 human_edit_level_declaration = COALESCE(NULLIF(human_modification_level, ''), human_edit_level_declaration, 'unspecified'),
                 authenticity_claim_declaration = COALESCE(NULLIF(authenticity_claim, ''), authenticity_claim_declaration, 'unspecified'),
                 custom_rights_statement = COALESCE(NULLIF(custom_metadata, ''), custom_rights_statement);

             CREATE INDEX IF NOT EXISTS idx_vault_protected_copy_hash
             ON vault_records(protected_copy_hash);
             CREATE INDEX IF NOT EXISTS idx_vault_output_strategy
             ON vault_records(output_strategy);",
        )?;
        set_user_version(conn, 15)?;
    }

    if current < 16 {
        conn.execute_batch(
            "ALTER TABLE vault_records ADD COLUMN payload_protocol_version INTEGER NOT NULL DEFAULT 2;
             ALTER TABLE vault_records ADD COLUMN payload_bytes_length INTEGER NOT NULL DEFAULT 119;
             ALTER TABLE vault_records ADD COLUMN watermark_id_issue_mode TEXT NOT NULL DEFAULT 'offline_generated';
             ALTER TABLE vault_records ADD COLUMN watermark_id_registry_status TEXT NOT NULL DEFAULT 'pending_registration';
             ALTER TABLE vault_records ADD COLUMN watermark_id_registry_receipt TEXT;
             ALTER TABLE vault_records ADD COLUMN payload_auth_status TEXT NOT NULL DEFAULT 'verified';

             CREATE INDEX IF NOT EXISTS idx_vault_registry_status
             ON vault_records(watermark_id_registry_status);
             CREATE INDEX IF NOT EXISTS idx_vault_payload_protocol
             ON vault_records(payload_protocol_version);",
        )?;
        set_user_version(conn, 16)?;
    }

    if current < 17 {
        conn.execute_batch(
            "ALTER TABLE vault_records ADD COLUMN video_visual_task_id TEXT;
             ALTER TABLE vault_records ADD COLUMN video_visual_completed_at TEXT;
             ALTER TABLE vault_records ADD COLUMN video_visual_strategy_digest TEXT;
             ALTER TABLE vault_records ADD COLUMN video_visual_self_check_confidence REAL;
             ALTER TABLE vault_records ADD COLUMN video_visual_self_check_threshold REAL;
             ALTER TABLE vault_records ADD COLUMN video_visual_checked_frames INTEGER;
             ALTER TABLE vault_records ADD COLUMN video_visual_media_hash TEXT;
             ALTER TABLE vault_records ADD COLUMN video_visual_receipt_hash TEXT;
             ALTER TABLE vault_records ADD COLUMN video_visual_output_bytes INTEGER;
             ALTER TABLE vault_records ADD COLUMN video_visual_output_content_type TEXT;

             CREATE INDEX IF NOT EXISTS idx_vault_video_visual_task
             ON vault_records(video_visual_task_id);
             CREATE INDEX IF NOT EXISTS idx_vault_video_visual_media_hash
             ON vault_records(video_visual_media_hash);",
        )?;
        set_user_version(conn, 17)?;
    }

    if current < 18 {
        backfill_vault_record_file_types(conn)?;
        set_user_version(conn, 18)?;
    }

    if current < 19 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS installation_identity (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                installation_id TEXT NOT NULL UNIQUE,
                salt_base64_url TEXT NOT NULL,
                secret_fingerprint_sha256 TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS offline_license_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                signed_token TEXT NOT NULL,
                token_sha256 TEXT NOT NULL,
                license_id TEXT NOT NULL,
                installation_id TEXT NOT NULL,
                product_code TEXT NOT NULL,
                key_id TEXT NOT NULL,
                issued_at TEXT NOT NULL,
                not_before TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                imported_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS offline_revocation_lists (
                key_id TEXT PRIMARY KEY,
                signed_token TEXT NOT NULL,
                token_sha256 TEXT NOT NULL,
                list_id TEXT NOT NULL,
                sequence INTEGER NOT NULL,
                generated_at TEXT NOT NULL,
                imported_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS offline_license_audit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                occurred_at TEXT NOT NULL,
                event_type TEXT NOT NULL,
                outcome TEXT NOT NULL,
                installation_id TEXT,
                artifact_id TEXT,
                key_id TEXT,
                detail_code TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_offline_license_audit_occurred_at
            ON offline_license_audit(occurred_at);

            CREATE TRIGGER IF NOT EXISTS offline_license_audit_no_update
            BEFORE UPDATE ON offline_license_audit
            BEGIN
                SELECT RAISE(ABORT, 'offline_license_audit_append_only');
            END;

            CREATE TRIGGER IF NOT EXISTS offline_license_audit_no_delete
            BEFORE DELETE ON offline_license_audit
            BEGIN
                SELECT RAISE(ABORT, 'offline_license_audit_append_only');
            END;",
        )?;
        set_user_version(conn, 19)?;
    }

    if current < 20 {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS offline_license_security_state (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                highest_observed_utc TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );",
        )?;
        set_user_version(conn, 20)?;
    }

    Ok(())
}

pub fn backfill_vault_record_file_types(
    conn: &rusqlite::Connection,
) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        "
        UPDATE vault_records
        SET file_type = 'image'
        WHERE file_type = 'video'
          AND video_notary_id IS NULL
          AND video_fingerprint_root IS NULL
          AND video_visual_task_id IS NULL
          AND video_visual_media_hash IS NULL
          AND NOT (
            lower(COALESCE(file_name, '')) LIKE '%.mp4'
            OR lower(COALESCE(file_name, '')) LIKE '%.mov'
            OR lower(COALESCE(file_name, '')) LIKE '%.webm'
            OR lower(COALESCE(file_name, '')) LIKE '%.avi'
            OR lower(COALESCE(file_name, '')) LIKE '%.mkv'
            OR lower(COALESCE(file_name, '')) LIKE '%.m4v'
          )
          AND (
            lower(COALESCE(file_name, '')) LIKE '%.jpg'
            OR lower(COALESCE(file_name, '')) LIKE '%.jpeg'
            OR lower(COALESCE(file_name, '')) LIKE '%.png'
            OR lower(COALESCE(file_name, '')) LIKE '%.bmp'
            OR lower(COALESCE(file_name, '')) LIKE '%.tiff'
            OR lower(COALESCE(file_name, '')) LIKE '%.webp'
            OR lower(COALESCE(file_name, '')) LIKE '%.gif'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.jpg'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.jpeg'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.png'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.bmp'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.tiff'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.webp'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.gif'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.jpg'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.jpeg'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.png'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.bmp'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.tiff'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.webp'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.gif'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.jpg'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.jpeg'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.png'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.bmp'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.tiff'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.webp'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.gif'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.jpg'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.jpeg'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.png'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.bmp'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.tiff'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.webp'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.gif'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.jpg'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.jpeg'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.png'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.bmp'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.tiff'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.webp'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.gif'
          );

        UPDATE vault_records
        SET file_type = 'audio'
        WHERE file_type = 'video'
          AND video_notary_id IS NULL
          AND video_fingerprint_root IS NULL
          AND video_visual_task_id IS NULL
          AND video_visual_media_hash IS NULL
          AND NOT (
            lower(COALESCE(file_name, '')) LIKE '%.mp4'
            OR lower(COALESCE(file_name, '')) LIKE '%.mov'
            OR lower(COALESCE(file_name, '')) LIKE '%.webm'
            OR lower(COALESCE(file_name, '')) LIKE '%.avi'
            OR lower(COALESCE(file_name, '')) LIKE '%.mkv'
            OR lower(COALESCE(file_name, '')) LIKE '%.m4v'
          )
          AND (
            lower(COALESCE(file_name, '')) LIKE '%.wav'
            OR lower(COALESCE(file_name, '')) LIKE '%.mp3'
            OR lower(COALESCE(file_name, '')) LIKE '%.flac'
            OR lower(COALESCE(file_name, '')) LIKE '%.aac'
            OR lower(COALESCE(file_name, '')) LIKE '%.ogg'
            OR lower(COALESCE(file_name, '')) LIKE '%.m4a'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.wav'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.mp3'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.flac'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.aac'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.ogg'
            OR lower(COALESCE(protected_copy_name, '')) LIKE '%.m4a'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.wav'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.mp3'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.flac'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.aac'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.ogg'
            OR lower(COALESCE(protected_copy_path, '')) LIKE '%.m4a'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.wav'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.mp3'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.flac'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.aac'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.ogg'
            OR lower(COALESCE(output_douyin, '')) LIKE '%.m4a'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.wav'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.mp3'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.flac'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.aac'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.ogg'
            OR lower(COALESCE(output_bilibili, '')) LIKE '%.m4a'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.wav'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.mp3'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.flac'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.aac'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.ogg'
            OR lower(COALESCE(output_xhs, '')) LIKE '%.m4a'
          );
        ",
    )
}

fn set_user_version(conn: &rusqlite::Connection, version: u32) -> Result<(), rusqlite::Error> {
    conn.pragma_update(None, "user_version", version)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    #[test]
    fn migration_18_backfills_legacy_media_file_types_without_touching_video_receipts() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        conn.execute_batch(
            "
            INSERT INTO vault_records (
                original_hash, file_name, file_type, created_at, watermark_uid,
                protected_copy_name, protected_copy_path
            ) VALUES
                ('hash-image', 'legacy-batch-image.png', 'video', '2026-07-04T01:00:00Z', 'HS-IMAGE', 'legacy-batch-image_watermarked.png', 'C:/qa/legacy-batch-image_watermarked.png'),
                ('hash-audio', 'legacy-batch-audio.wav', 'video', '2026-07-04T01:01:00Z', 'HS-AUDIO', 'legacy-batch-audio_watermarked.wav', 'C:/qa/legacy-batch-audio_watermarked.wav'),
                ('hash-video', 'legacy-l1-video.mp4', 'video', '2026-07-04T01:02:00Z', 'HS-VIDEO', 'legacy-l1-video_protected.mp4', 'C:/qa/legacy-l1-video_protected.mp4'),
                ('hash-l2', 'operator-renamed.png', 'video', '2026-07-04T01:03:00Z', 'HS-L2', NULL, NULL);

            UPDATE vault_records
            SET video_notary_id = 'vfn_legacy', video_fingerprint_root = 'sha256:l2-root'
            WHERE watermark_uid = 'HS-L2';
            ",
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 17u32).unwrap();

        run_migrations(&conn).unwrap();

        let mut stmt = conn
            .prepare("SELECT watermark_uid, file_type FROM vault_records ORDER BY watermark_uid")
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(
            rows,
            vec![
                ("HS-AUDIO".to_string(), "audio".to_string()),
                ("HS-IMAGE".to_string(), "image".to_string()),
                ("HS-L2".to_string(), "video".to_string()),
                ("HS-VIDEO".to_string(), "video".to_string()),
            ]
        );
        let version: u32 = conn
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_VERSION);
    }

    #[test]
    fn migration_19_creates_offline_license_tables_and_append_only_audit() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO offline_license_audit (
                occurred_at, event_type, outcome, detail_code
             ) VALUES ('2026-07-15T00:00:00Z', 'license_import', 'accepted', 'ok')",
            [],
        )
        .unwrap();

        let update_error = conn
            .execute(
                "UPDATE offline_license_audit SET outcome = 'changed' WHERE id = 1",
                [],
            )
            .unwrap_err();
        assert!(update_error
            .to_string()
            .contains("offline_license_audit_append_only"));

        let delete_error = conn
            .execute("DELETE FROM offline_license_audit WHERE id = 1", [])
            .unwrap_err();
        assert!(delete_error
            .to_string()
            .contains("offline_license_audit_append_only"));
    }

    #[test]
    fn migration_20_creates_offline_license_clock_high_water_state() {
        let conn = Connection::open_in_memory().unwrap();
        run_migrations(&conn).unwrap();

        conn.execute(
            "INSERT INTO offline_license_security_state (
                id, highest_observed_utc, updated_at
             ) VALUES (1, '2026-07-15T12:00:00Z', '2026-07-15T12:00:00Z')",
            [],
        )
        .unwrap();

        let highest: String = conn
            .query_row(
                "SELECT highest_observed_utc
                 FROM offline_license_security_state
                 WHERE id = 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(highest, "2026-07-15T12:00:00Z");
    }
}

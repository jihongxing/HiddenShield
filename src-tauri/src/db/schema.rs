/// Current schema version. Increment when adding migrations.
#[allow(dead_code)]
pub const CURRENT_VERSION: u32 = 5;

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

    Ok(())
}

fn set_user_version(conn: &rusqlite::Connection, version: u32) -> Result<(), rusqlite::Error> {
    conn.pragma_update(None, "user_version", version)
}

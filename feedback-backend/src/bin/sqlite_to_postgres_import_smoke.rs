#[cfg(feature = "postgres")]
use rusqlite::{params, Connection};

#[cfg(feature = "postgres")]
use sha2::{Digest, Sha256};

#[cfg(feature = "postgres")]
use sqlx::{Executor, Row};

#[cfg(feature = "postgres")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use hiddenshield_feedback_backend::database::{
        POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL,
    };
    use sqlx::PgPool;

    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| {
            "missing HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or DATABASE_URL for disposable Postgres import smoke"
        })?;

    if !is_safe_import_smoke_url(&database_url) {
        return Err(
            "refusing to run import smoke against non-disposable database URL; include localhost/127.0.0.1 and hiddenshield_import_smoke in the URL"
                .into(),
        );
    }

    let sqlite = build_sqlite_fixture()?;
    let pool = PgPool::connect(&database_url).await?;
    let required_tables = migration_tables();

    assert_tables_absent(&pool, &required_tables).await?;
    execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL).await?;

    let source_checks = collect_table_checks(&sqlite, &pool, "after_first_import_before").await?;
    assert_source_not_imported_yet(&source_checks)?;
    import_sqlite_fixture(&sqlite, &pool).await?;
    let imported_checks = collect_table_checks(&sqlite, &pool, "after_first_import").await?;
    assert_table_checks_match(&imported_checks)?;

    import_sqlite_fixture(&sqlite, &pool).await?;
    let idempotent_checks = collect_table_checks(&sqlite, &pool, "after_second_import").await?;
    assert_table_checks_match(&idempotent_checks)?;
    assert_idempotent_counts(&imported_checks, &idempotent_checks)?;

    let logical_reference_checks = assert_logical_references(&pool).await?;
    let unique_constraint_checks = assert_unique_constraints(&pool).await?;

    execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?;
    assert_tables_absent(&pool, &required_tables).await?;

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "migration": "sqlite_to_postgres_p4_import_smoke",
            "source": "in_memory_sqlite_fixture",
            "tablesChecked": imported_checks.len(),
            "totalRowsImported": imported_checks.iter().map(|check| check.postgres_count).sum::<i64>(),
            "rowCountChecks": imported_checks,
            "idempotentRerun": "row_counts_unchanged",
            "hashAggregate": "primary_key_hash_match",
            "logicalReferenceChecks": logical_reference_checks,
            "uniqueConstraintChecks": unique_constraint_checks,
            "rollback": "empty_schema_verified",
            "safety": {
                "productionDatabaseAllowed": false,
                "formalUiMockReleaseDefaultPath": "not_switched",
                "sqliteSource": "local_fixture_only"
            }
        })
    );
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("sqlite_to_postgres_import_smoke requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
#[derive(Debug, serde::Serialize)]
struct TableCheck {
    phase: &'static str,
    table: &'static str,
    source_count: i64,
    postgres_count: i64,
    source_hash: String,
    postgres_hash: String,
}

#[cfg(feature = "postgres")]
fn is_safe_import_smoke_url(database_url: &str) -> bool {
    let lower = database_url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_import_smoke")
}

#[cfg(feature = "postgres")]
fn migration_tables() -> [&'static str; 11] {
    [
        "schema_migrations",
        "cloud_accounts",
        "cloud_devices",
        "cloud_sessions",
        "auth_challenges",
        "auth_attempts",
        "cloud_sync_events",
        "cloud_device_cursors",
        "watermark_id_registry",
        "watermark_id_reissue_jobs",
        "rights_manifests",
    ]
}

#[cfg(feature = "postgres")]
async fn execute_sql_batch(pool: &sqlx::PgPool, sql: &str) -> Result<(), sqlx::Error> {
    for statement in sql.split(';') {
        let statement = statement.trim();
        if !statement.is_empty() {
            pool.execute(statement).await?;
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn build_sqlite_fixture() -> Result<Connection, rusqlite::Error> {
    let conn = Connection::open_in_memory()?;
    conn.execute_batch(
        r#"
        CREATE TABLE cloud_accounts (
            id TEXT PRIMARY KEY,
            identifier TEXT NOT NULL UNIQUE,
            password_hash TEXT,
            password_salt TEXT,
            password_hash_algorithm TEXT NOT NULL DEFAULT 'sha256',
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
        CREATE TABLE cloud_devices (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            client_device_id TEXT NOT NULL,
            name TEXT NOT NULL,
            platform TEXT NOT NULL,
            app_version TEXT NOT NULL,
            public_key TEXT,
            registered INTEGER NOT NULL,
            auto_sync_enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(account_id, client_device_id)
        );
        CREATE TABLE cloud_sessions (
            access_token TEXT PRIMARY KEY,
            refresh_token TEXT NOT NULL,
            account_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            revoked_at TEXT,
            expires_at TEXT,
            refresh_expires_at TEXT,
            last_used_at TEXT,
            token_family_id TEXT
        );
        CREATE TABLE auth_challenges (
            challenge_id TEXT PRIMARY KEY,
            identifier TEXT NOT NULL,
            purpose TEXT NOT NULL,
            client_device_id TEXT NOT NULL,
            code_hash TEXT NOT NULL,
            code_salt TEXT NOT NULL,
            delivery_channel TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            consumed_at TEXT,
            created_at TEXT NOT NULL,
            plain_code_for_delivery TEXT
        );
        CREATE TABLE auth_attempts (
            attempt_id TEXT PRIMARY KEY,
            identifier TEXT NOT NULL,
            client_device_id TEXT,
            attempt_type TEXT NOT NULL,
            outcome TEXT NOT NULL,
            reason TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
        CREATE TABLE cloud_sync_events (
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
        CREATE TABLE cloud_device_cursors (
            account_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            cursor TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY(account_id, device_id)
        );
        CREATE TABLE watermark_id_registry (
            registry_id TEXT PRIMARY KEY,
            request_id TEXT,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            creator_profile_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            watermark_uid TEXT NOT NULL UNIQUE,
            watermark_id_issue_mode TEXT NOT NULL,
            registry_status TEXT NOT NULL,
            registry_receipt TEXT NOT NULL,
            registry_proof_hash TEXT NOT NULL,
            media_type TEXT NOT NULL,
            payload_protocol_version INTEGER NOT NULL,
            payload_bytes_length INTEGER NOT NULL,
            parent_watermark_uid TEXT,
            revision INTEGER NOT NULL,
            original_hash TEXT,
            protected_copy_hash TEXT,
            write_verification_status TEXT,
            confirmed_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(account_id, request_id)
        );
        CREATE TABLE watermark_id_reissue_jobs (
            job_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            creator_profile_id TEXT NOT NULL,
            previous_watermark_uid TEXT NOT NULL,
            replacement_watermark_uid TEXT NOT NULL,
            reason TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        CREATE TABLE rights_manifests (
            id TEXT PRIMARY KEY,
            rights_manifest_id TEXT NOT NULL UNIQUE,
            watermark_uid TEXT NOT NULL,
            manifest_version INTEGER NOT NULL,
            status TEXT NOT NULL,
            training_policy TEXT NOT NULL,
            work_source_declaration TEXT NOT NULL,
            creation_method_declaration TEXT NOT NULL,
            human_edit_level_declaration TEXT NOT NULL,
            authenticity_claim_declaration TEXT NOT NULL,
            tdm_reservation TEXT NOT NULL DEFAULT 'not_declared',
            search_indexing_policy TEXT NOT NULL DEFAULT 'not_declared',
            embedding_policy TEXT NOT NULL DEFAULT 'not_declared',
            commercial_training_policy TEXT NOT NULL DEFAULT 'not_declared',
            custom_terms_url TEXT,
            custom_terms_hash TEXT,
            standard_mappings_json TEXT NOT NULL,
            manifest_sha256 TEXT NOT NULL,
            signed_by TEXT NOT NULL,
            signature TEXT NOT NULL,
            effective_at TEXT NOT NULL,
            superseded_by_rights_manifest_id TEXT,
            revoked_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        "#,
    )?;
    seed_sqlite_fixture(&conn)?;
    Ok(conn)
}

#[cfg(feature = "postgres")]
fn seed_sqlite_fixture(conn: &Connection) -> Result<(), rusqlite::Error> {
    let now = "2026-07-03T00:00:00Z";
    conn.execute(
        "INSERT INTO cloud_accounts VALUES (?1, ?2, ?3, ?4, 'sha256', ?5, ?6, ?7, ?8, ?9, ?10, 1, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
        params![
            "acct_creator_001",
            "creator@example.test",
            "hash",
            "salt",
            "Creator Test",
            "workspace_001",
            "Creator Workspace",
            "creator_profile_001",
            "Creator Test",
            "seed-ref-fixture",
            "entitlement_001",
            "Creator",
            "creator",
            "active",
            r#"{"cloud_sync":true,"report_export":true}"#,
            now,
            now
        ],
    )?;
    for (id, client_device_id, name, platform) in [
        (
            "device_desktop_001",
            "desktop-local-001",
            "Desktop",
            "windows",
        ),
        ("device_mobile_001", "mobile-local-001", "Mobile", "android"),
    ] {
        conn.execute(
            "INSERT INTO cloud_devices VALUES (?1, 'acct_creator_001', ?2, ?3, ?4, '0.1.0', NULL, 1, 1, ?5, ?5)",
            params![id, client_device_id, name, platform, now],
        )?;
    }
    conn.execute(
        "INSERT INTO cloud_sessions VALUES ('access_001', 'refresh_001', 'acct_creator_001', 'device_desktop_001', ?1, NULL, ?1, ?1, ?1, 'family_001')",
        params![now],
    )?;
    conn.execute(
        "INSERT INTO auth_challenges VALUES ('challenge_001', 'creator@example.test', 'login', 'desktop-local-001', 'code_hash', 'code_salt', 'fixture', ?1, NULL, ?1, '123456')",
        params![now],
    )?;
    conn.execute(
        "INSERT INTO auth_attempts VALUES ('attempt_001', 'creator@example.test', 'desktop-local-001', 'challenge', 'accepted', 'fixture', ?1)",
        params![now],
    )?;
    for (client_event_id, entity_id, payload) in [
        (
            "event_desktop_001",
            "vault_record_001",
            r#"{"watermarkUid":"HS-F552CEF2-9CBD6243-2D10EB78-CC4A61F0","mediaType":"image"}"#,
        ),
        (
            "event_mobile_001",
            "vault_record_002",
            r#"{"watermarkUid":"HS-4110B32E-81781234-ABCDEF90-12345678","mediaType":"audio"}"#,
        ),
    ] {
        conn.execute(
            "INSERT INTO cloud_sync_events (account_id, device_id, client_event_id, operation, entity_type, entity_id, payload_json, created_at)
             VALUES ('acct_creator_001', 'device_desktop_001', ?1, 'upsert', 'vault_record', ?2, ?3, ?4)",
            params![client_event_id, entity_id, payload, now],
        )?;
    }
    for (device_id, cursor) in [("device_desktop_001", "2"), ("device_mobile_001", "2")] {
        conn.execute(
            "INSERT INTO cloud_device_cursors VALUES ('acct_creator_001', ?1, ?2, ?3)",
            params![device_id, cursor, now],
        )?;
    }
    conn.execute(
        "INSERT INTO watermark_id_registry VALUES ('registry_001', 'request_001', 'acct_creator_001', 'workspace_001', 'creator_profile_001', 'device_desktop_001', 'HS-F552CEF2-9CBD6243-2D10EB78-CC4A61F0', 'server_reserved', 'server_confirmed', 'receipt_001', 'proof_001', 'image', 3, 39, NULL, 1, 'orig_hash_001', 'protected_hash_001', 'verified', ?1, ?1, ?1)",
        params![now],
    )?;
    conn.execute(
        "INSERT INTO watermark_id_registry VALUES ('registry_002', 'request_002', 'acct_creator_001', 'workspace_001', 'creator_profile_001', 'device_mobile_001', 'HS-4110B32E-81781234-ABCDEF90-12345678', 'offline_reconciled', 'offline_confirmed', 'receipt_002', 'proof_002', 'audio', 3, 39, 'HS-F552CEF2-9CBD6243-2D10EB78-CC4A61F0', 2, 'orig_hash_002', 'protected_hash_002', 'verified', ?1, ?1, ?1)",
        params![now],
    )?;
    conn.execute(
        "INSERT INTO watermark_id_reissue_jobs VALUES ('reissue_001', 'acct_creator_001', 'workspace_001', 'creator_profile_001', 'HS-OLDOLD00-OLDOLD00-OLDOLD00-OLDOLD00', 'HS-F552CEF2-9CBD6243-2D10EB78-CC4A61F0', 'duplicate_uid', 'completed', ?1, ?1)",
        params![now],
    )?;
    conn.execute(
        "INSERT INTO rights_manifests VALUES ('rights_row_001', 'rights_manifest_001', 'HS-F552CEF2-9CBD6243-2D10EB78-CC4A61F0', 1, 'active', 'no_training', 'human_created', 'camera_or_editor', 'substantial', 'creator_attested', 'reserved', 'allowed', 'not_declared', 'prohibited', NULL, NULL, ?1, 'manifest_sha256_001', 'fixture_signer', 'fixture_signature', ?2, NULL, NULL, ?2, ?2)",
        params![r#"{"tdm":"reserved"}"#, now],
    )?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_sqlite_fixture(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    import_cloud_accounts(sqlite, pool).await?;
    import_cloud_devices(sqlite, pool).await?;
    import_cloud_sessions(sqlite, pool).await?;
    import_auth_challenges(sqlite, pool).await?;
    import_auth_attempts(sqlite, pool).await?;
    import_cloud_sync_events(sqlite, pool).await?;
    import_cloud_device_cursors(sqlite, pool).await?;
    import_watermark_id_registry(sqlite, pool).await?;
    import_watermark_id_reissue_jobs(sqlite, pool).await?;
    import_rights_manifests(sqlite, pool).await?;
    sqlx::query("SELECT setval(pg_get_serial_sequence('cloud_sync_events', 'sequence'), COALESCE((SELECT MAX(sequence) FROM cloud_sync_events), 1), true)")
        .execute(pool)
        .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_cloud_accounts(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = sqlite.prepare(
        "SELECT id, identifier, password_hash, password_salt, password_hash_algorithm, display_name, workspace_id, workspace_name, creator_profile_id, creator_display_name, creator_seed_ref, seed_envelope_version, entitlement_id, entitlement_plan_name, entitlement_plan_code, entitlement_status, entitlement_features_json, created_at, updated_at FROM cloud_accounts",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, i64>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, String>(14)?,
            row.get::<_, String>(15)?,
            json_value(row.get::<_, String>(16)?),
            parse_ts(row.get::<_, String>(17)?)?,
            parse_ts(row.get::<_, String>(18)?)?,
        ))
    })?;
    for row in rows {
        let row = row?;
        sqlx::query(
            "INSERT INTO cloud_accounts VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3)
        .bind(row.4)
        .bind(row.5)
        .bind(row.6)
        .bind(row.7)
        .bind(row.8)
        .bind(row.9)
        .bind(row.10)
        .bind(row.11 as i32)
        .bind(row.12)
        .bind(row.13)
        .bind(row.14)
        .bind(row.15)
        .bind(row.16)
        .bind(row.17)
        .bind(row.18)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_cloud_devices(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = sqlite.prepare(
        "SELECT id, account_id, client_device_id, name, platform, app_version, public_key, registered, auto_sync_enabled, created_at, updated_at FROM cloud_devices",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, Option<String>>(6)?,
            row.get::<_, i64>(7)? != 0,
            row.get::<_, i64>(8)? != 0,
            parse_ts(row.get::<_, String>(9)?)?,
            parse_ts(row.get::<_, String>(10)?)?,
        ))
    })?;
    for row in rows {
        let row = row?;
        sqlx::query(
            "INSERT INTO cloud_devices VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3)
        .bind(row.4)
        .bind(row.5)
        .bind(row.6)
        .bind(row.7)
        .bind(row.8)
        .bind(row.9)
        .bind(row.10)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_cloud_sessions(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = sqlite.prepare(
        "SELECT access_token, refresh_token, account_id, device_id, created_at, revoked_at, expires_at, refresh_expires_at, last_used_at, token_family_id FROM cloud_sessions",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            parse_ts(row.get::<_, String>(4)?)?,
            parse_optional_ts(row.get::<_, Option<String>>(5)?)?,
            parse_optional_ts(row.get::<_, Option<String>>(6)?)?,
            parse_optional_ts(row.get::<_, Option<String>>(7)?)?,
            parse_optional_ts(row.get::<_, Option<String>>(8)?)?,
            row.get::<_, Option<String>>(9)?,
        ))
    })?;
    for row in rows {
        let row = row?;
        sqlx::query(
            "INSERT INTO cloud_sessions VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             ON CONFLICT (access_token) DO NOTHING",
        )
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3)
        .bind(row.4)
        .bind(row.5)
        .bind(row.6)
        .bind(row.7)
        .bind(row.8)
        .bind(row.9)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_auth_challenges(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = sqlite.prepare(
        "SELECT challenge_id, identifier, purpose, client_device_id, code_hash, code_salt, delivery_channel, expires_at, consumed_at, created_at, plain_code_for_delivery FROM auth_challenges",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            parse_ts(row.get::<_, String>(7)?)?,
            parse_optional_ts(row.get::<_, Option<String>>(8)?)?,
            parse_ts(row.get::<_, String>(9)?)?,
            row.get::<_, Option<String>>(10)?,
        ))
    })?;
    for row in rows {
        let row = row?;
        sqlx::query(
            "INSERT INTO auth_challenges VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)
             ON CONFLICT (challenge_id) DO NOTHING",
        )
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3)
        .bind(row.4)
        .bind(row.5)
        .bind(row.6)
        .bind(row.7)
        .bind(row.8)
        .bind(row.9)
        .bind(row.10)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_auth_attempts(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = sqlite.prepare(
        "SELECT attempt_id, identifier, client_device_id, attempt_type, outcome, reason, created_at FROM auth_attempts",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            parse_ts(row.get::<_, String>(6)?)?,
        ))
    })?;
    for row in rows {
        let row = row?;
        sqlx::query(
            "INSERT INTO auth_attempts VALUES ($1,$2,$3,$4,$5,$6,$7)
             ON CONFLICT (attempt_id) DO NOTHING",
        )
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3)
        .bind(row.4)
        .bind(row.5)
        .bind(row.6)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_cloud_sync_events(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = sqlite.prepare(
        "SELECT sequence, account_id, device_id, client_event_id, operation, entity_type, entity_id, payload_json, created_at FROM cloud_sync_events ORDER BY sequence",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            json_value(row.get::<_, String>(7)?),
            parse_ts(row.get::<_, String>(8)?)?,
        ))
    })?;
    for row in rows {
        let row = row?;
        sqlx::query(
            "INSERT INTO cloud_sync_events (sequence, account_id, device_id, client_event_id, operation, entity_type, entity_id, payload_json, created_at)
             VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)
             ON CONFLICT (account_id, device_id, client_event_id) DO NOTHING",
        )
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3)
        .bind(row.4)
        .bind(row.5)
        .bind(row.6)
        .bind(row.7)
        .bind(row.8)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_cloud_device_cursors(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = sqlite
        .prepare("SELECT account_id, device_id, cursor, updated_at FROM cloud_device_cursors")?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            parse_ts(row.get::<_, String>(3)?)?,
        ))
    })?;
    for row in rows {
        let row = row?;
        sqlx::query(
            "INSERT INTO cloud_device_cursors VALUES ($1,$2,$3,$4)
             ON CONFLICT (account_id, device_id) DO UPDATE
             SET cursor = EXCLUDED.cursor, updated_at = EXCLUDED.updated_at",
        )
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_watermark_id_registry(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = sqlite.prepare(
        "SELECT registry_id, request_id, account_id, workspace_id, creator_profile_id, device_id, watermark_uid, watermark_id_issue_mode, registry_status, registry_receipt, registry_proof_hash, media_type, payload_protocol_version, payload_bytes_length, parent_watermark_uid, revision, original_hash, protected_copy_hash, write_verification_status, confirmed_at, created_at, updated_at FROM watermark_id_registry",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, i64>(12)?,
            row.get::<_, i64>(13)?,
            row.get::<_, Option<String>>(14)?,
            row.get::<_, i64>(15)?,
            row.get::<_, Option<String>>(16)?,
            row.get::<_, Option<String>>(17)?,
            row.get::<_, Option<String>>(18)?,
            parse_optional_ts(row.get::<_, Option<String>>(19)?)?,
            parse_ts(row.get::<_, String>(20)?)?,
            parse_ts(row.get::<_, String>(21)?)?,
        ))
    })?;
    for row in rows {
        let row = row?;
        sqlx::query(
            "INSERT INTO watermark_id_registry VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22)
             ON CONFLICT (registry_id) DO NOTHING",
        )
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3)
        .bind(row.4)
        .bind(row.5)
        .bind(row.6)
        .bind(row.7)
        .bind(row.8)
        .bind(row.9)
        .bind(row.10)
        .bind(row.11)
        .bind(row.12 as i32)
        .bind(row.13 as i32)
        .bind(row.14)
        .bind(row.15 as i32)
        .bind(row.16)
        .bind(row.17)
        .bind(row.18)
        .bind(row.19)
        .bind(row.20)
        .bind(row.21)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_watermark_id_reissue_jobs(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = sqlite.prepare(
        "SELECT job_id, account_id, workspace_id, creator_profile_id, previous_watermark_uid, replacement_watermark_uid, reason, status, created_at, updated_at FROM watermark_id_reissue_jobs",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            parse_ts(row.get::<_, String>(8)?)?,
            parse_ts(row.get::<_, String>(9)?)?,
        ))
    })?;
    for row in rows {
        let row = row?;
        sqlx::query(
            "INSERT INTO watermark_id_reissue_jobs VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10)
             ON CONFLICT (job_id) DO NOTHING",
        )
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3)
        .bind(row.4)
        .bind(row.5)
        .bind(row.6)
        .bind(row.7)
        .bind(row.8)
        .bind(row.9)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn import_rights_manifests(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stmt = sqlite.prepare(
        "SELECT id, rights_manifest_id, watermark_uid, manifest_version, status, training_policy, work_source_declaration, creation_method_declaration, human_edit_level_declaration, authenticity_claim_declaration, tdm_reservation, search_indexing_policy, embedding_policy, commercial_training_policy, custom_terms_url, custom_terms_hash, standard_mappings_json, manifest_sha256, signed_by, signature, effective_at, superseded_by_rights_manifest_id, revoked_at, created_at, updated_at FROM rights_manifests",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            row.get::<_, String>(9)?,
            row.get::<_, String>(10)?,
            row.get::<_, String>(11)?,
            row.get::<_, String>(12)?,
            row.get::<_, String>(13)?,
            row.get::<_, Option<String>>(14)?,
            row.get::<_, Option<String>>(15)?,
            json_value(row.get::<_, String>(16)?),
            row.get::<_, String>(17)?,
            row.get::<_, String>(18)?,
            row.get::<_, String>(19)?,
            parse_ts(row.get::<_, String>(20)?)?,
            row.get::<_, Option<String>>(21)?,
            parse_optional_ts(row.get::<_, Option<String>>(22)?)?,
            parse_ts(row.get::<_, String>(23)?)?,
            parse_ts(row.get::<_, String>(24)?)?,
        ))
    })?;
    for row in rows {
        let row = row?;
        sqlx::query(
            "INSERT INTO rights_manifests VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,$24,$25)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(row.0)
        .bind(row.1)
        .bind(row.2)
        .bind(row.3 as i32)
        .bind(row.4)
        .bind(row.5)
        .bind(row.6)
        .bind(row.7)
        .bind(row.8)
        .bind(row.9)
        .bind(row.10)
        .bind(row.11)
        .bind(row.12)
        .bind(row.13)
        .bind(row.14)
        .bind(row.15)
        .bind(row.16)
        .bind(row.17)
        .bind(row.18)
        .bind(row.19)
        .bind(row.20)
        .bind(row.21)
        .bind(row.22)
        .bind(row.23)
        .bind(row.24)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn collect_table_checks(
    sqlite: &Connection,
    pool: &sqlx::PgPool,
    phase: &'static str,
) -> Result<Vec<TableCheck>, Box<dyn std::error::Error>> {
    let specs = [
        ("cloud_accounts", "id"),
        ("cloud_devices", "id"),
        ("cloud_sessions", "access_token"),
        ("auth_challenges", "challenge_id"),
        ("auth_attempts", "attempt_id"),
        ("cloud_sync_events", "sequence || ':' || client_event_id"),
        ("cloud_device_cursors", "account_id || ':' || device_id"),
        ("watermark_id_registry", "registry_id"),
        ("watermark_id_reissue_jobs", "job_id"),
        ("rights_manifests", "id"),
    ];
    let mut checks = Vec::new();
    for (table, key_expr) in specs {
        let source_count = sqlite_count(sqlite, table)?;
        let postgres_count = postgres_count(pool, table).await?;
        let source_hash = sqlite_hash(sqlite, table, key_expr)?;
        let postgres_hash = postgres_hash(pool, table, key_expr).await?;
        checks.push(TableCheck {
            phase,
            table,
            source_count,
            postgres_count,
            source_hash,
            postgres_hash,
        });
    }
    Ok(checks)
}

#[cfg(feature = "postgres")]
fn assert_source_not_imported_yet(checks: &[TableCheck]) -> Result<(), Box<dyn std::error::Error>> {
    if checks.iter().any(|check| check.postgres_count != 0) {
        return Err("expected empty Postgres tables before import".into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn assert_table_checks_match(checks: &[TableCheck]) -> Result<(), Box<dyn std::error::Error>> {
    for check in checks {
        if check.source_count != check.postgres_count {
            return Err(format!(
                "{} row count mismatch: source={} postgres={}",
                check.table, check.source_count, check.postgres_count
            )
            .into());
        }
        if check.source_hash != check.postgres_hash {
            return Err(format!(
                "{} hash mismatch: source={} postgres={}",
                check.table, check.source_hash, check.postgres_hash
            )
            .into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn assert_idempotent_counts(
    first: &[TableCheck],
    second: &[TableCheck],
) -> Result<(), Box<dyn std::error::Error>> {
    for (first, second) in first.iter().zip(second.iter()) {
        if first.postgres_count != second.postgres_count
            || first.postgres_hash != second.postgres_hash
        {
            return Err(format!("{} import rerun changed row count or hash", first.table).into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_logical_references(
    pool: &sqlx::PgPool,
) -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
    let checks = [
        (
            "devices_account_exists",
            "SELECT COUNT(*) FROM cloud_devices d LEFT JOIN cloud_accounts a ON a.id = d.account_id WHERE a.id IS NULL",
        ),
        (
            "sessions_account_device_exists",
            "SELECT COUNT(*) FROM cloud_sessions s LEFT JOIN cloud_accounts a ON a.id = s.account_id LEFT JOIN cloud_devices d ON d.id = s.device_id WHERE a.id IS NULL OR d.id IS NULL",
        ),
        (
            "sync_events_account_device_exists",
            "SELECT COUNT(*) FROM cloud_sync_events e LEFT JOIN cloud_accounts a ON a.id = e.account_id LEFT JOIN cloud_devices d ON d.id = e.device_id WHERE a.id IS NULL OR d.id IS NULL",
        ),
        (
            "cursors_account_device_exists",
            "SELECT COUNT(*) FROM cloud_device_cursors c LEFT JOIN cloud_accounts a ON a.id = c.account_id LEFT JOIN cloud_devices d ON d.id = c.device_id WHERE a.id IS NULL OR d.id IS NULL",
        ),
        (
            "registry_account_device_exists",
            "SELECT COUNT(*) FROM watermark_id_registry r LEFT JOIN cloud_accounts a ON a.id = r.account_id LEFT JOIN cloud_devices d ON d.id = r.device_id WHERE a.id IS NULL OR d.id IS NULL",
        ),
        (
            "reissue_replacement_exists",
            "SELECT COUNT(*) FROM watermark_id_reissue_jobs j LEFT JOIN watermark_id_registry r ON r.watermark_uid = j.replacement_watermark_uid WHERE r.watermark_uid IS NULL",
        ),
    ];
    let mut passed = Vec::new();
    for (name, sql) in checks {
        let missing: i64 = sqlx::query_scalar(sql).fetch_one(pool).await?;
        if missing != 0 {
            return Err(format!("logical reference check failed: {name} missing={missing}").into());
        }
        passed.push(name);
    }
    Ok(passed)
}

#[cfg(feature = "postgres")]
async fn assert_unique_constraints(
    pool: &sqlx::PgPool,
) -> Result<Vec<&'static str>, Box<dyn std::error::Error>> {
    expect_unique_violation(
        pool,
        "duplicate_cloud_sync_client_event_id",
        "INSERT INTO cloud_sync_events (sequence, account_id, device_id, client_event_id, operation, entity_type, entity_id, payload_json, created_at)
         VALUES (100, 'acct_creator_001', 'device_desktop_001', 'event_desktop_001', 'upsert', 'vault_record', 'duplicate', '{}'::jsonb, now())",
    )
    .await?;
    expect_unique_violation(
        pool,
        "duplicate_active_rights_manifest",
        "INSERT INTO rights_manifests (id, rights_manifest_id, watermark_uid, manifest_version, status, training_policy, work_source_declaration, creation_method_declaration, human_edit_level_declaration, authenticity_claim_declaration, standard_mappings_json, manifest_sha256, signed_by, signature, effective_at, created_at, updated_at)
         VALUES ('rights_row_duplicate', 'rights_manifest_duplicate', 'HS-F552CEF2-9CBD6243-2D10EB78-CC4A61F0', 2, 'active', 'no_training', 'human_created', 'camera_or_editor', 'substantial', 'creator_attested', '{}'::jsonb, 'manifest_sha256_duplicate', 'fixture_signer', 'fixture_signature', now(), now(), now())",
    )
    .await?;
    Ok(vec![
        "duplicate_cloud_sync_client_event_id",
        "duplicate_active_rights_manifest",
    ])
}

#[cfg(feature = "postgres")]
async fn expect_unique_violation(
    pool: &sqlx::PgPool,
    name: &str,
    sql: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match sqlx::query(sql).execute(pool).await {
        Ok(_) => Err(format!("expected unique violation for {name}").into()),
        Err(sqlx::Error::Database(error)) if error.code().as_deref() == Some("23505") => Ok(()),
        Err(error) => Err(format!("expected unique violation for {name}, got {error}").into()),
    }
}

#[cfg(feature = "postgres")]
fn sqlite_count(sqlite: &Connection, table: &str) -> Result<i64, rusqlite::Error> {
    sqlite.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
}

#[cfg(feature = "postgres")]
async fn postgres_count(pool: &sqlx::PgPool, table: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
}

#[cfg(feature = "postgres")]
fn sqlite_hash(
    sqlite: &Connection,
    table: &str,
    key_expr: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut stmt = sqlite.prepare(&format!(
        "SELECT CAST({key_expr} AS TEXT) AS key FROM {table} ORDER BY key"
    ))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut keys = Vec::new();
    for row in rows {
        keys.push(row?);
    }
    Ok(hash_lines(&keys))
}

#[cfg(feature = "postgres")]
async fn postgres_hash(
    pool: &sqlx::PgPool,
    table: &str,
    key_expr: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let rows = sqlx::query(&format!(
        "SELECT CAST({key_expr} AS TEXT) AS key FROM {table} ORDER BY key"
    ))
    .fetch_all(pool)
    .await?;
    let keys = rows
        .iter()
        .map(|row| row.try_get::<String, _>("key"))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(hash_lines(&keys))
}

#[cfg(feature = "postgres")]
fn hash_lines(values: &[String]) -> String {
    let mut hasher = Sha256::new();
    for value in values {
        hasher.update(value.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(feature = "postgres")]
fn json_value(value: String) -> serde_json::Value {
    serde_json::from_str(&value).expect("fixture JSON must be valid")
}

#[cfg(feature = "postgres")]
fn parse_ts(value: String) -> Result<chrono::DateTime<chrono::Utc>, rusqlite::Error> {
    chrono::DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))
}

#[cfg(feature = "postgres")]
fn parse_optional_ts(
    value: Option<String>,
) -> Result<Option<chrono::DateTime<chrono::Utc>>, rusqlite::Error> {
    value.map(parse_ts).transpose()
}

#[cfg(feature = "postgres")]
async fn assert_tables_absent(
    pool: &sqlx::PgPool,
    tables: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for table in tables {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = $1
            )",
        )
        .bind(table)
        .fetch_one(pool)
        .await?;
        if exists {
            return Err(format!("expected disposable schema to not contain table {table}").into());
        }
    }
    Ok(())
}

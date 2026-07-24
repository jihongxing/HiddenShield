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
            "missing HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or DATABASE_URL for disposable Postgres smoke"
        })?;

    if !is_safe_smoke_url(&database_url) {
        return Err(
            "refusing to run smoke against non-disposable database URL; include localhost/127.0.0.1 and hiddenshield_migrate_smoke in the URL"
                .into(),
        );
    }

    let pool = PgPool::connect(&database_url).await?;
    let required_tables = [
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
    ];
    let required_indexes = [
        "idx_auth_challenges_identifier_created",
        "idx_auth_attempts_identifier_created",
        "idx_cloud_sync_events_account_sequence",
        "idx_watermark_id_registry_account_workspace",
        "idx_watermark_id_registry_parent",
        "idx_watermark_id_reissue_jobs_account",
        "idx_rights_manifests_one_active",
        "idx_rights_manifests_watermark",
        "idx_rights_manifests_watermark_status",
        "idx_rights_manifests_watermark_version",
        "idx_rights_manifests_status_updated",
    ];

    assert_tables_absent(&pool, &required_tables).await?;
    execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL).await?;
    assert_tables_present(&pool, &required_tables).await?;
    assert_indexes_present(&pool, &required_indexes).await?;
    assert_column_type(&pool, "cloud_sync_events", "sequence", "bigint").await?;
    assert_column_type(&pool, "cloud_sync_events", "payload_json", "jsonb").await?;
    assert_column_type(&pool, "cloud_devices", "registered", "boolean").await?;
    assert_column_type(
        &pool,
        "cloud_accounts",
        "created_at",
        "timestamp with time zone",
    )
    .await?;
    assert_partial_index(
        &pool,
        "idx_rights_manifests_one_active",
        "WHERE (status = 'active'",
    )
    .await?;

    execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?;
    assert_tables_absent(&pool, &required_tables).await?;
    assert_indexes_absent(&pool, &required_indexes).await?;

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "migration": "0001_auth_sync_registry",
            "upTablesChecked": required_tables.len(),
            "indexesChecked": required_indexes.len(),
            "rollback": "empty_schema_verified"
        })
    );
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("postgres_migrate_smoke requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
fn is_safe_smoke_url(database_url: &str) -> bool {
    let lower = database_url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_migrate_smoke")
}

#[cfg(feature = "postgres")]
async fn execute_sql_batch(pool: &sqlx::PgPool, sql: &str) -> Result<(), sqlx::Error> {
    for statement in sql.split(';') {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        pool.execute(statement).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_tables_present(
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
        if !exists {
            return Err(format!("expected table {table} to exist after migration up").into());
        }
    }
    Ok(())
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

#[cfg(feature = "postgres")]
async fn assert_indexes_present(
    pool: &sqlx::PgPool,
    indexes: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for index in indexes {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM pg_indexes
                WHERE schemaname = 'public' AND indexname = $1
            )",
        )
        .bind(index)
        .fetch_one(pool)
        .await?;
        if !exists {
            return Err(format!("expected index {index} to exist after migration up").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_indexes_absent(
    pool: &sqlx::PgPool,
    indexes: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for index in indexes {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM pg_indexes
                WHERE schemaname = 'public' AND indexname = $1
            )",
        )
        .bind(index)
        .fetch_one(pool)
        .await?;
        if exists {
            return Err(
                format!("expected index {index} to be dropped after migration down").into(),
            );
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_column_type(
    pool: &sqlx::PgPool,
    table: &str,
    column: &str,
    expected_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let actual: Option<String> = sqlx::query_scalar(
        "SELECT data_type FROM information_schema.columns
         WHERE table_schema = 'public' AND table_name = $1 AND column_name = $2",
    )
    .bind(table)
    .bind(column)
    .fetch_optional(pool)
    .await?;
    match actual {
        Some(actual) if actual == expected_type => Ok(()),
        Some(actual) => {
            Err(format!("expected {table}.{column} to be {expected_type}, got {actual}").into())
        }
        None => Err(format!("missing column {table}.{column}").into()),
    }
}

#[cfg(feature = "postgres")]
async fn assert_partial_index(
    pool: &sqlx::PgPool,
    index: &str,
    expected_fragment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let row = sqlx::query(
        "SELECT indexdef FROM pg_indexes WHERE schemaname = 'public' AND indexname = $1",
    )
    .bind(index)
    .fetch_one(pool)
    .await?;
    let indexdef: String = row.try_get("indexdef")?;
    if !indexdef.contains(expected_fragment) {
        return Err(format!(
            "partial index {index} missing fragment {expected_fragment}: {indexdef}"
        )
        .into());
    }
    Ok(())
}

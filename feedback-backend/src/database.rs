use std::path::PathBuf;

use clap::ValueEnum;
use rusqlite::Connection;

#[cfg(feature = "postgres")]
pub type PostgresPool = sqlx::Pool<sqlx::Postgres>;

#[cfg(feature = "postgres")]
pub const POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL: &str =
    include_str!("../migrations/postgres/0001_auth_sync_registry.up.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0001_auth_sync_registry.down.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL: &str =
    include_str!("../migrations/postgres/0002_ai_transparency_schema.up.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0002_ai_transparency_schema.down.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL: &str =
    include_str!("../migrations/postgres/0003_ai_transparency_approval_state_machine.up.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0003_ai_transparency_approval_state_machine.down.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_UP_SQL: &str =
    include_str!("../migrations/postgres/0004_ai_transparency_confirm_audit.up.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0004_ai_transparency_confirm_audit.down.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_UP_SQL: &str =
    include_str!("../migrations/postgres/0005_ai_transparency_credential_custody.up.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0005_ai_transparency_credential_custody.down.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_UP_SQL: &str =
    include_str!("../migrations/postgres/0006_ai_transparency_credential_lifecycle.up.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0006_ai_transparency_credential_lifecycle.down.sql");

pub const POSTGRES_P8_AI_TRANSPARENCY_POST_EMBED_SIGNING_UP_SQL: &str =
    include_str!("../migrations/postgres/0007_ai_transparency_post_embed_signing.up.sql");

pub const POSTGRES_P8_AI_TRANSPARENCY_POST_EMBED_SIGNING_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0007_ai_transparency_post_embed_signing.down.sql");

pub const POSTGRES_P9_AI_TRANSPARENCY_SIGNING_RESERVATION_UP_SQL: &str = include_str!(
    "../migrations/postgres/0008_ai_transparency_signing_reservation_artifact_recovery.up.sql"
);

pub const POSTGRES_P9_AI_TRANSPARENCY_SIGNING_RESERVATION_DOWN_SQL: &str = include_str!(
    "../migrations/postgres/0008_ai_transparency_signing_reservation_artifact_recovery.down.sql"
);

pub const POSTGRES_P10_AI_TRANSPARENCY_ADAPTER_RECEIPTS_UP_SQL: &str = include_str!(
    "../migrations/postgres/0009_ai_transparency_adapter_receipts_crash_recovery.up.sql"
);

pub const POSTGRES_P10_AI_TRANSPARENCY_ADAPTER_RECEIPTS_DOWN_SQL: &str = include_str!(
    "../migrations/postgres/0009_ai_transparency_adapter_receipts_crash_recovery.down.sql"
);

pub const POSTGRES_P11_AI_TRANSPARENCY_RECOVERY_WORKER_UP_SQL: &str =
    include_str!("../migrations/postgres/0010_ai_transparency_post_embed_recovery_worker.up.sql");

pub const POSTGRES_P11_AI_TRANSPARENCY_RECOVERY_WORKER_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0010_ai_transparency_post_embed_recovery_worker.down.sql");

pub const POSTGRES_P12_AI_TRANSPARENCY_DEAD_LETTER_REQUEUE_UP_SQL: &str =
    include_str!("../migrations/postgres/0011_ai_transparency_dead_letter_requeue_command.up.sql");

pub const POSTGRES_P12_AI_TRANSPARENCY_DEAD_LETTER_REQUEUE_DOWN_SQL: &str = include_str!(
    "../migrations/postgres/0011_ai_transparency_dead_letter_requeue_command.down.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P13_AI_TRANSPARENCY_CONFIRMED_DELIVERY_ENVELOPE_UP_SQL: &str =
    include_str!("../migrations/postgres/0012_ai_transparency_confirmed_delivery_envelope.up.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P13_AI_TRANSPARENCY_CONFIRMED_DELIVERY_ENVELOPE_DOWN_SQL: &str = include_str!(
    "../migrations/postgres/0012_ai_transparency_confirmed_delivery_envelope.down.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P14_AI_TRANSPARENCY_DELIVERY_RETRIEVAL_UP_SQL: &str = include_str!(
    "../migrations/postgres/0013_ai_transparency_delivery_authorization_retrieval.up.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P14_AI_TRANSPARENCY_DELIVERY_RETRIEVAL_DOWN_SQL: &str = include_str!(
    "../migrations/postgres/0013_ai_transparency_delivery_authorization_retrieval.down.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P15_AI_TRANSPARENCY_DELIVERY_REVOKE_RESOURCE_BUDGET_UP_SQL: &str = include_str!(
    "../migrations/postgres/0014_ai_transparency_delivery_revoke_resource_budget.up.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P15_AI_TRANSPARENCY_DELIVERY_REVOKE_RESOURCE_BUDGET_DOWN_SQL: &str = include_str!(
    "../migrations/postgres/0014_ai_transparency_delivery_revoke_resource_budget.down.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P16_AI_TRANSPARENCY_DELIVERY_SECURITY_OBSERVABILITY_UP_SQL: &str = include_str!(
    "../migrations/postgres/0015_ai_transparency_delivery_security_observability.up.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P16_AI_TRANSPARENCY_DELIVERY_SECURITY_OBSERVABILITY_DOWN_SQL: &str = include_str!(
    "../migrations/postgres/0015_ai_transparency_delivery_security_observability.down.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P17_AI_TRANSPARENCY_DELIVERY_SECURITY_INCIDENT_RUNNER_UP_SQL: &str = include_str!(
    "../migrations/postgres/0016_ai_transparency_delivery_security_incident_runner.up.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P17_AI_TRANSPARENCY_DELIVERY_SECURITY_INCIDENT_RUNNER_DOWN_SQL: &str = include_str!(
    "../migrations/postgres/0016_ai_transparency_delivery_security_incident_runner.down.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P18_AI_TRANSPARENCY_DELIVERY_SECURITY_NOTIFICATION_OUTBOX_UP_SQL: &str = include_str!(
    "../migrations/postgres/0017_ai_transparency_delivery_security_notification_outbox.up.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P18_AI_TRANSPARENCY_DELIVERY_SECURITY_NOTIFICATION_OUTBOX_DOWN_SQL: &str = include_str!(
    "../migrations/postgres/0017_ai_transparency_delivery_security_notification_outbox.down.sql"
);
#[cfg(feature = "postgres")]
pub const POSTGRES_P19_AI_TRANSPARENCY_NOTIFICATION_DELIVERY_GATE_UP_SQL: &str =
    include_str!("../migrations/postgres/0018_ai_transparency_notification_delivery_gate.up.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P19_AI_TRANSPARENCY_NOTIFICATION_DELIVERY_GATE_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0018_ai_transparency_notification_delivery_gate.down.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P20_AI_TRANSPARENCY_PLATFORM_API_UP_SQL: &str =
    include_str!("../migrations/postgres/0019_ai_transparency_platform_api.up.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P20_AI_TRANSPARENCY_PLATFORM_API_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0019_ai_transparency_platform_api.down.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P21_AI_TRANSPARENCY_PUBLIC_RESOLVER_UP_SQL: &str =
    include_str!("../migrations/postgres/0020_ai_transparency_public_resolver.up.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P21_AI_TRANSPARENCY_PUBLIC_RESOLVER_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0020_ai_transparency_public_resolver.down.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P22_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_INTAKE_UP_SQL: &str =
    include_str!("../migrations/postgres/0021_ai_transparency_external_evidence_intake.up.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P22_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_INTAKE_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0021_ai_transparency_external_evidence_intake.down.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P23_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_REVIEW_UP_SQL: &str =
    include_str!("../migrations/postgres/0022_ai_transparency_external_evidence_review.up.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P23_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_REVIEW_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0022_ai_transparency_external_evidence_review.down.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P24_CLOUD_COPYRIGHT_MULTITENANT_CORE_UP_SQL: &str =
    include_str!("../migrations/postgres/0023_cloud_copyright_multitenant_core.up.sql");
#[cfg(feature = "postgres")]
pub const POSTGRES_P24_CLOUD_COPYRIGHT_MULTITENANT_CORE_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0023_cloud_copyright_multitenant_core.down.sql");

pub const SQLITE_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL: &str =
    include_str!("../migrations/sqlite/0003_ai_transparency_approval_state_machine.up.sql");

pub const SQLITE_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_DOWN_SQL: &str =
    include_str!("../migrations/sqlite/0003_ai_transparency_approval_state_machine.down.sql");

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum DatabaseBackendKind {
    Sqlite,
    Postgres,
}

impl DatabaseBackendKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sqlite => "sqlite",
            Self::Postgres => "postgres",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DatabaseRuntimeMode {
    Local,
    Test,
    Staging,
    Production,
}

impl DatabaseRuntimeMode {
    pub fn parse(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "prod" | "production" => Self::Production,
            "stage" | "staging" => Self::Staging,
            "test" | "ci" => Self::Test,
            _ => Self::Local,
        }
    }

    pub fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DatabaseConfig {
    pub backend: DatabaseBackendKind,
    pub sqlite_path: Option<PathBuf>,
    pub postgres_url: Option<String>,
    pub runtime_mode: DatabaseRuntimeMode,
}

impl DatabaseConfig {
    pub fn sqlite(path: impl Into<PathBuf>, deployment_env: impl AsRef<str>) -> Self {
        Self {
            backend: DatabaseBackendKind::Sqlite,
            sqlite_path: Some(path.into()),
            postgres_url: None,
            runtime_mode: DatabaseRuntimeMode::parse(deployment_env.as_ref()),
        }
    }

    pub fn postgres(url: impl Into<String>, deployment_env: impl AsRef<str>) -> Self {
        Self {
            backend: DatabaseBackendKind::Postgres,
            sqlite_path: None,
            postgres_url: Some(url.into()),
            runtime_mode: DatabaseRuntimeMode::parse(deployment_env.as_ref()),
        }
    }

    pub fn from_server_args(
        backend: DatabaseBackendKind,
        sqlite_path: PathBuf,
        postgres_url: Option<String>,
        deployment_env: impl AsRef<str>,
    ) -> Self {
        let runtime_mode = DatabaseRuntimeMode::parse(deployment_env.as_ref());
        match backend {
            DatabaseBackendKind::Sqlite => Self {
                backend,
                sqlite_path: Some(sqlite_path),
                postgres_url: None,
                runtime_mode,
            },
            DatabaseBackendKind::Postgres => Self {
                backend,
                sqlite_path: None,
                postgres_url,
                runtime_mode,
            },
        }
    }

    pub fn validate(&self) -> Result<(), DatabaseConfigError> {
        match self.backend {
            DatabaseBackendKind::Sqlite => {
                if self.runtime_mode.is_production() {
                    return Err(DatabaseConfigError::SqliteForbiddenInProduction);
                }
                if self.sqlite_path.is_none() {
                    return Err(DatabaseConfigError::MissingSqlitePath);
                }
            }
            DatabaseBackendKind::Postgres => {
                let Some(url) = self.postgres_url.as_deref() else {
                    return Err(DatabaseConfigError::MissingPostgresUrl);
                };
                if !url.starts_with("postgres://") && !url.starts_with("postgresql://") {
                    return Err(DatabaseConfigError::InvalidPostgresUrl);
                }
            }
        }
        Ok(())
    }

    #[cfg(feature = "postgres")]
    pub fn postgres_pool_options(max_connections: u32) -> sqlx::postgres::PgPoolOptions {
        sqlx::postgres::PgPoolOptions::new().max_connections(max_connections)
    }
}

#[cfg(feature = "postgres")]
pub fn postgres_schema_smoke_sql() -> &'static [&'static str] {
    &[
        POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL,
        POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL,
        POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
        POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_UP_SQL,
        POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_UP_SQL,
        POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_UP_SQL,
        POSTGRES_P8_AI_TRANSPARENCY_POST_EMBED_SIGNING_UP_SQL,
        POSTGRES_P9_AI_TRANSPARENCY_SIGNING_RESERVATION_UP_SQL,
    ]
}

pub fn apply_sqlite_ai_transparency_approval_state_machine(
    conn: &Connection,
) -> Result<(), rusqlite::Error> {
    ensure_sqlite_column(
        conn,
        "ai_profile_entitlements",
        "current_version_id",
        "TEXT",
    )?;
    ensure_sqlite_column(
        conn,
        "ai_profile_entitlements",
        "current_version",
        "INTEGER",
    )?;
    ensure_sqlite_column(
        conn,
        "ai_profile_entitlements",
        "projection_updated_at",
        "TEXT",
    )?;
    conn.execute_batch(SQLITE_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL)
}

fn ensure_sqlite_column(
    conn: &Connection,
    table: &str,
    column: &str,
    column_type: &str,
) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let existing: String = row.get(1)?;
        if existing == column {
            return Ok(());
        }
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {column_type}"),
        [],
    )?;
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum DatabaseConfigError {
    #[error("SQLite backend is forbidden when HIDDENSHIELD_DEPLOYMENT_ENV=production")]
    SqliteForbiddenInProduction,
    #[error("SQLite backend requires a db path")]
    MissingSqlitePath,
    #[error("PostgreSQL backend requires HIDDENSHIELD_DATABASE_URL")]
    MissingPostgresUrl,
    #[error("PostgreSQL database URL must start with postgres:// or postgresql://")]
    InvalidPostgresUrl,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_is_valid_for_local_runtime() {
        let config = DatabaseConfig::sqlite("feedback.sqlite", "local");
        assert_eq!(config.backend, DatabaseBackendKind::Sqlite);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn sqlite_is_forbidden_for_production_runtime() {
        let config = DatabaseConfig::sqlite("feedback.sqlite", "production");
        assert!(matches!(
            config.validate(),
            Err(DatabaseConfigError::SqliteForbiddenInProduction)
        ));
    }

    #[test]
    fn postgres_requires_database_url() {
        let config = DatabaseConfig::from_server_args(
            DatabaseBackendKind::Postgres,
            "ignored.sqlite".into(),
            None,
            "staging",
        );
        assert!(matches!(
            config.validate(),
            Err(DatabaseConfigError::MissingPostgresUrl)
        ));
    }

    #[test]
    fn postgres_accepts_postgresql_url() {
        let config = DatabaseConfig::postgres("postgresql://localhost/hiddenshield", "production");
        assert_eq!(config.backend, DatabaseBackendKind::Postgres);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn sqlite_approval_migration_is_additive_and_idempotent() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE ai_transparency_licenses (license_id TEXT PRIMARY KEY);
             CREATE TABLE ai_profile_entitlements (
                license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
                profile_id TEXT NOT NULL,
                profile_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                effective_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                terms_version TEXT NOT NULL,
                approved_by TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (license_id, profile_id)
             );",
        )
        .unwrap();

        apply_sqlite_ai_transparency_approval_state_machine(&conn).unwrap();
        apply_sqlite_ai_transparency_approval_state_machine(&conn).unwrap();

        for table in [
            "ai_transparency_actor_role_snapshots",
            "ai_transparency_change_requests",
            "ai_profile_entitlement_versions",
            "ai_transparency_change_approvals",
            "ai_transparency_change_executions",
            "ai_transparency_change_audit_events",
            "ai_transparency_change_target_locks",
        ] {
            let exists: bool = conn
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                    [table],
                    |row| row.get(0),
                )
                .unwrap();
            assert!(exists, "missing SQLite migration table {table}");
        }

        let columns: Vec<String> = conn
            .prepare("PRAGMA table_info(ai_profile_entitlements)")
            .unwrap()
            .query_map([], |row| row.get(1))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        for column in [
            "current_version_id",
            "current_version",
            "projection_updated_at",
        ] {
            assert!(columns.iter().any(|existing| existing == column));
        }
    }

    #[test]
    fn sqlite_approval_audit_is_append_only_and_down_drops_new_tables() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE ai_transparency_licenses (license_id TEXT PRIMARY KEY);
             CREATE TABLE ai_profile_entitlements (
                license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
                profile_id TEXT NOT NULL,
                profile_kind TEXT NOT NULL,
                status TEXT NOT NULL,
                effective_at TEXT NOT NULL,
                expires_at TEXT NOT NULL,
                terms_version TEXT NOT NULL,
                approved_by TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                PRIMARY KEY (license_id, profile_id)
             );",
        )
        .unwrap();
        apply_sqlite_ai_transparency_approval_state_machine(&conn).unwrap();
        conn.execute_batch(
            "INSERT INTO ai_transparency_actor_role_snapshots (
                actor_role_snapshot_id, actor_id, actor_type, role, tenant_id, workspace_id,
                environment, role_binding_id, role_binding_version, source_identity_system,
                authentication_level, captured_at, source_expires_at, snapshot_sha256
             ) VALUES (
                'actor', 'actor', 'human', 'ai_transparency_requester', 'tenant', 'workspace',
                'sandbox', 'binding', 1, 'hiddenshield_internal_iam', 'mfa',
                '2026-07-27T00:00:00Z', '2026-07-28T00:00:00Z',
                'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'
             );
             INSERT INTO ai_transparency_change_requests (
                change_request_id, operation, target_type, target_scope_key, tenant_id, workspace_id,
                environment, desired_state_json, request_reason, requester_snapshot_id,
                request_digest_version, request_digest, idempotency_key, status, expires_at,
                created_at, updated_at
             ) VALUES (
                'request', 'create_license', 'license', 'license:tenant:workspace:sandbox',
                'tenant', 'workspace', 'sandbox', '{}', 'test', 'actor',
                'hs-ai-change-request-digest-v1',
                'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
                'idem', 'pending_review', '2026-07-28T00:00:00Z',
                '2026-07-27T00:00:00Z', '2026-07-27T00:00:00Z'
             );
             INSERT INTO ai_transparency_change_audit_events (
                audit_event_id, change_request_id, sequence, event_type, to_state,
                actor_snapshot_id, target_type, reason_code, request_digest, details_json, occurred_at
             ) VALUES (
                'audit', 'request', 1, 'change_request_submitted', 'pending_review', 'actor',
                'license', 'submitted',
                'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
                '{}', '2026-07-27T00:00:00Z'
             );",
        )
        .unwrap();

        assert!(conn
            .execute(
                "UPDATE ai_transparency_change_audit_events SET reason_code = 'changed'",
                [],
            )
            .is_err());
        assert!(conn
            .execute("DELETE FROM ai_transparency_change_audit_events", [])
            .is_err());

        conn.execute_batch(SQLITE_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_DOWN_SQL)
            .unwrap();
        let remaining: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master
                 WHERE type = 'table' AND name LIKE 'ai_transparency_change_%'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0);
    }

    #[cfg(feature = "postgres")]
    #[test]
    fn postgres_schema_smoke_covers_p1_tables() {
        let sql = postgres_schema_smoke_sql().join("\n");
        for table in [
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
        ] {
            assert!(sql.contains(table), "missing Postgres smoke table {table}");
        }
        assert!(sql.contains("JSONB"));
        assert!(sql.contains("TIMESTAMPTZ"));
        assert!(sql.contains("BIGSERIAL"));
        assert!(POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL.contains("DROP TABLE IF EXISTS"));
    }
}

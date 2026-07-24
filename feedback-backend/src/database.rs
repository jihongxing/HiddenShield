use std::path::PathBuf;

use clap::ValueEnum;

#[cfg(feature = "postgres")]
pub type PostgresPool = sqlx::Pool<sqlx::Postgres>;

#[cfg(feature = "postgres")]
pub const POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL: &str =
    include_str!("../migrations/postgres/0001_auth_sync_registry.up.sql");

#[cfg(feature = "postgres")]
pub const POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL: &str =
    include_str!("../migrations/postgres/0001_auth_sync_registry.down.sql");

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
    &[POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL]
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

import { readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  design: readFileSync('docs/云版权库PostgreSQL迁移设计.md', 'utf8'),
  roadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  boundary: readFileSync('docs/当前真实能力边界说明.md', 'utf8'),
  backendDatabase: readFileSync('feedback-backend/src/database.rs', 'utf8'),
  backendCargo: readFileSync('feedback-backend/Cargo.toml', 'utf8'),
  backendRepository: readFileSync('feedback-backend/src/repository.rs', 'utf8'),
  backendPostgresAuth: readFileSync('feedback-backend/src/postgres_auth.rs', 'utf8'),
  backendPostgresSync: readFileSync('feedback-backend/src/postgres_sync.rs', 'utf8'),
  backendPostgresRegistry: readFileSync('feedback-backend/src/postgres_registry.rs', 'utf8'),
  backendAuthPostgresQa: readFileSync('feedback-backend/src/bin/auth_postgres_runtime_qa.rs', 'utf8'),
  backendSyncPostgresQa: readFileSync('feedback-backend/src/bin/cloud_sync_postgres_runtime_qa.rs', 'utf8'),
  backendRegistryPostgresQa: readFileSync('feedback-backend/src/bin/watermark_registry_postgres_runtime_qa.rs', 'utf8'),
  backendPostgresImportSmoke: readFileSync('feedback-backend/src/bin/sqlite_to_postgres_import_smoke.rs', 'utf8'),
  postgresMigrationUp: readFileSync('feedback-backend/migrations/postgres/0001_auth_sync_registry.up.sql', 'utf8'),
  postgresMigrationDown: readFileSync('feedback-backend/migrations/postgres/0001_auth_sync_registry.down.sql', 'utf8'),
  authPostgresQaScript: readFileSync('scripts/run-auth-postgres-runtime-qa.mjs', 'utf8'),
  syncPostgresQaScript: readFileSync('scripts/run-cloud-sync-postgres-runtime-qa.mjs', 'utf8'),
  registryPostgresQaScript: readFileSync('scripts/run-watermark-registry-postgres-runtime-qa.mjs', 'utf8'),
  postgresRuntimeAggregateScript: readFileSync('scripts/run-cloud-postgres-runtime-qa.mjs', 'utf8'),
  postgresImportSmokeScript: readFileSync('scripts/run-postgres-import-smoke.mjs', 'utf8'),
  postgresProductionReadinessGate: readFileSync('scripts/verify-cloud-postgres-production-readiness-gate.mjs', 'utf8'),
  postgresSqliteShutdownGate: readFileSync('scripts/verify-cloud-postgres-sqlite-shutdown-gate.mjs', 'utf8'),
  backendStorage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  backendLib: readFileSync('feedback-backend/src/lib.rs', 'utf8'),
};

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

const requiredDesignTokens = [
  'Phase P0 评审结论',
  'P0 评审通过',
  'cloud_accounts',
  'cloud_devices',
  'cloud_sessions',
  'auth_challenges',
  'auth_attempts',
  'cloud_sync_events',
  'cloud_device_cursors',
  'watermark_id_registry',
  'watermark_id_reissue_jobs',
  'rights_manifests',
  'enterprise_api_keys',
  'enterprise_quota_balances',
  'enterprise_quota_ledger',
  'enterprise_api_audit_events',
  'enterprise_rate_limit_windows',
  'enterprise_admin_audit_events',
  'billing_payment_sessions',
  'report_purchase_sessions',
  'report_purchase_grants',
  'billing_customers',
  'subscriptions',
  'subscription_events',
  'entitlements',
  'admin_audit_events',
  'cloud_video_tasks',
  'cloud_usage_ledger',
  'video_fingerprint_notaries',
  'team_workspaces',
  'team_members',
  'team_shared_library_records',
  'team_audit_logs',
  'feedback_events',
  'feedback_batches',
  'Creator 云同步 push',
  'Enterprise batch scan',
  'quota 幂等重试',
  '支付 webhook',
  'L3 task claim',
  '恢复演练',
];

for (const token of requiredDesignTokens) {
  assert(sources.design.includes(token), `PostgreSQL migration design missing required token: ${token}`);
}

assert(
    sources.packageJson.includes('"cloud:db-portability-contract"') &&
    sources.packageJson.includes('verify-cloud-db-portability-contract.mjs') &&
    sources.packageJson.includes('"cloud:db-portability:postgres-check"') &&
    sources.packageJson.includes('"cloud:postgres-migration-contract"') &&
    sources.packageJson.includes('"auth:postgres-runtime-qa"') &&
    sources.packageJson.includes('run-auth-postgres-runtime-qa.mjs') &&
    sources.packageJson.includes('"cloud:sync-postgres-runtime-qa"') &&
    sources.packageJson.includes('run-cloud-sync-postgres-runtime-qa.mjs') &&
    sources.packageJson.includes('"watermark:registry-postgres-runtime-qa"') &&
    sources.packageJson.includes('run-watermark-registry-postgres-runtime-qa.mjs') &&
    sources.packageJson.includes('"cloud:postgres-runtime-qa"') &&
    sources.packageJson.includes('run-cloud-postgres-runtime-qa.mjs') &&
    sources.packageJson.includes('"cloud:postgres-import-smoke"') &&
    sources.packageJson.includes('run-postgres-import-smoke.mjs') &&
    sources.packageJson.includes('"cloud:postgres-production-readiness-gate"') &&
    sources.packageJson.includes('verify-cloud-postgres-production-readiness-gate.mjs') &&
    sources.packageJson.includes('"cloud:postgres-sqlite-shutdown-gate"') &&
    sources.packageJson.includes('verify-cloud-postgres-sqlite-shutdown-gate.mjs'),
  'package.json must expose cloud:db-portability-contract',
);

assert(
  sources.backendLib.includes('pub mod database;') &&
    sources.backendLib.includes('pub mod repository;') &&
    sources.backendLib.includes('DatabaseBackendKind') &&
    sources.backendLib.includes('HIDDENSHIELD_DATABASE_BACKEND') &&
    sources.backendLib.includes('HIDDENSHIELD_DATABASE_URL') &&
    sources.backendLib.includes('HIDDENSHIELD_DEPLOYMENT_ENV') &&
    sources.backendLib.includes('Storage::open_with_database_config'),
  'feedback-backend lib must expose database backend args and open storage through DatabaseConfig',
);

assert(
  sources.backendCargo.includes('[features]') &&
    sources.backendCargo.includes('postgres = ["dep:sqlx"]') &&
    sources.backendCargo.includes('sqlx = {') &&
    sources.backendCargo.includes('runtime-tokio-rustls') &&
    sources.backendCargo.includes('"postgres"') &&
    sources.backendCargo.includes('optional = true'),
  'feedback-backend Cargo.toml must gate sqlx behind postgres feature',
);

assert(
  sources.backendRepository.includes('pub trait AuthRepository') &&
    sources.backendRepository.includes('pub trait CloudSyncRepository') &&
    sources.backendRepository.includes('pub trait WatermarkRegistryRepository') &&
    sources.backendRepository.includes('impl AuthRepository for Storage') &&
    sources.backendRepository.includes('impl CloudSyncRepository for Storage') &&
    sources.backendRepository.includes('impl WatermarkRegistryRepository for Storage') &&
    sources.backendRepository.includes('continue_account') &&
    sources.backendRepository.includes('create_auth_session') &&
    sources.backendRepository.includes('refresh_auth_session') &&
    sources.backendRepository.includes('list_devices') &&
    sources.backendRepository.includes('revoke_device') &&
    sources.backendRepository.includes('push_cloud_events_batch') &&
    sources.backendRepository.includes('get_cloud_changes') &&
    sources.backendRepository.includes('reserve_watermark_id') &&
    sources.backendRepository.includes('confirm_watermark_id') &&
    sources.backendRepository.includes('reconcile_watermark_id') &&
    sources.backendRepository.includes('reissue_watermark_id'),
  'repository traits must cover auth, sync and registry slices and delegate to current SQLite Storage',
);

assert(
    sources.backendLib.includes('pub mod postgres_auth;') &&
    sources.backendLib.includes('pub mod postgres_sync;') &&
    sources.backendLib.includes('pub mod postgres_registry;') &&
    sources.backendPostgresAuth.includes('pub struct PostgresAuthRepository') &&
    sources.backendPostgresAuth.includes('impl AuthRepository for PostgresAuthRepository') &&
    sources.backendPostgresAuth.includes('create_auth_challenge_pg') &&
    sources.backendPostgresAuth.includes('create_auth_session_pg') &&
    sources.backendPostgresAuth.includes('refresh_auth_session_pg') &&
    sources.backendPostgresAuth.includes('logout_auth_session_pg') &&
    sources.backendPostgresAuth.includes('list_devices_pg') &&
    sources.backendPostgresAuth.includes('revoke_device_pg') &&
    !sources.backendPostgresAuth.includes('impl CloudSyncRepository for PostgresAuthRepository') &&
    !sources.backendPostgresAuth.includes('impl WatermarkRegistryRepository for PostgresAuthRepository'),
  'P3.1 must implement only AuthRepository for Postgres and must not enable sync/registry write paths',
);

assert(
  sources.backendPostgresSync.includes('pub struct PostgresCloudSyncRepository') &&
    sources.backendPostgresSync.includes('impl CloudSyncRepository for PostgresCloudSyncRepository') &&
    sources.backendPostgresSync.includes('push_cloud_events_batch_pg') &&
    sources.backendPostgresSync.includes('get_cloud_changes_pg') &&
    sources.backendPostgresSync.includes('ensure_cloud_sync_entitled_pg') &&
    sources.backendPostgresSync.includes('session_workspace_matches_pg') &&
    sources.backendPostgresSync.includes('upsert_device_cursor_pg') &&
    !sources.backendPostgresSync.includes('impl WatermarkRegistryRepository for PostgresCloudSyncRepository') &&
    !sources.backendPostgresSync.includes('reserve_watermark_id') &&
    !sources.backendPostgresSync.includes('confirm_watermark_id'),
  'P3.2 must implement only CloudSyncRepository for Postgres and must not enable registry write paths',
);

assert(
  sources.backendPostgresRegistry.includes('pub struct PostgresWatermarkRegistryRepository') &&
    sources.backendPostgresRegistry.includes('impl WatermarkRegistryRepository for PostgresWatermarkRegistryRepository') &&
    sources.backendPostgresRegistry.includes('reserve_watermark_id_pg') &&
    sources.backendPostgresRegistry.includes('confirm_watermark_id_pg') &&
    sources.backendPostgresRegistry.includes('reconcile_watermark_id_pg') &&
    sources.backendPostgresRegistry.includes('reissue_watermark_id_pg') &&
    !sources.backendPostgresRegistry.includes('impl CloudSyncRepository for PostgresWatermarkRegistryRepository') &&
    !sources.backendPostgresRegistry.includes('push_cloud_events_batch') &&
    !sources.backendPostgresRegistry.includes('get_cloud_changes'),
  'P3.3 must implement only WatermarkRegistryRepository for Postgres and must not enable sync write paths',
);

assert(
  sources.backendAuthPostgresQa.includes('auth_postgres_runtime_qa') &&
    sources.backendAuthPostgresQa.includes('PostgresAuthRepository') &&
    sources.backendAuthPostgresQa.includes('challengeFixtureCode') &&
    sources.backendAuthPostgresQa.includes('refreshRotation') &&
    sources.backendAuthPostgresQa.includes('deviceRevoke') &&
    sources.backendAuthPostgresQa.includes('"syncRepositoryWritePath": "not_executed"') &&
    sources.backendAuthPostgresQa.includes('"registryRepositoryWritePath": "not_executed"') &&
    sources.backendAuthPostgresQa.includes('hiddenshield_auth_runtime_qa'),
  'auth Postgres runtime QA must cover challenge/session/refresh/device and keep sync/registry not executed',
);

assert(
  sources.authPostgresQaScript.includes('auth_postgres_runtime_qa_artifact_v1') &&
    sources.authPostgresQaScript.includes('hiddenshield_auth_runtime_qa') &&
    sources.authPostgresQaScript.includes('productionDatabaseAllowed') &&
    sources.authPostgresQaScript.includes("syncRepositoryWritePath !== 'not_executed'") &&
    sources.authPostgresQaScript.includes("registryRepositoryWritePath !== 'not_executed'"),
  'auth Postgres runtime QA script must write a safety artifact and reject sync/registry execution',
);

assert(
  sources.backendSyncPostgresQa.includes('cloud_sync_postgres_runtime_qa') &&
    sources.backendSyncPostgresQa.includes('PostgresCloudSyncRepository') &&
    sources.backendSyncPostgresQa.includes('duplicateClientEventIdIdempotent') &&
    sources.backendSyncPostgresQa.includes('freePushForbidden') &&
    sources.backendSyncPostgresQa.includes('wrongDeviceRejected') &&
    sources.backendSyncPostgresQa.includes('"registryRepositoryWritePath": "not_executed"') &&
    sources.backendSyncPostgresQa.includes('hiddenshield_sync_runtime_qa'),
  'cloud sync Postgres runtime QA must cover idempotency, cursor resume, entitlement and registry not executed',
);

assert(
  sources.syncPostgresQaScript.includes('cloud_sync_postgres_runtime_qa_artifact_v1') &&
    sources.syncPostgresQaScript.includes('hiddenshield_sync_runtime_qa') &&
    sources.syncPostgresQaScript.includes('productionDatabaseAllowed') &&
    sources.syncPostgresQaScript.includes("registryRepositoryWritePath !== 'not_executed'"),
  'cloud sync Postgres runtime QA script must write a safety artifact and reject registry execution',
);

assert(
  sources.backendRegistryPostgresQa.includes('watermark_registry_postgres_runtime_qa') &&
    sources.backendRegistryPostgresQa.includes('PostgresWatermarkRegistryRepository') &&
    sources.backendRegistryPostgresQa.includes('idempotentReserveByRequestId') &&
    sources.backendRegistryPostgresQa.includes('offlineReconcile') &&
    sources.backendRegistryPostgresQa.includes('conflictDetection') &&
    sources.backendRegistryPostgresQa.includes('reissueCreated') &&
    sources.backendRegistryPostgresQa.includes('"syncRepositoryWritePath": "not_executed"') &&
    sources.backendRegistryPostgresQa.includes('"formalUiMockReleaseDefaultPath": "not_switched"') &&
    sources.backendRegistryPostgresQa.includes('hiddenshield_registry_runtime_qa'),
  'watermark registry Postgres runtime QA must cover reserve/confirm/reconcile/conflict/reissue and keep default paths untouched',
);

assert(
  sources.registryPostgresQaScript.includes('watermark_registry_postgres_runtime_qa_artifact_v1') &&
    sources.registryPostgresQaScript.includes('hiddenshield_registry_runtime_qa') &&
    sources.registryPostgresQaScript.includes('productionDatabaseAllowed') &&
    sources.registryPostgresQaScript.includes("syncRepositoryWritePath !== 'not_executed'") &&
    sources.registryPostgresQaScript.includes("formalUiMockReleaseDefaultPath !== 'not_switched'"),
  'watermark registry Postgres runtime QA script must write a safety artifact and reject default-path switching',
);

assert(
  sources.postgresRuntimeAggregateScript.includes('cloud_postgres_runtime_qa_aggregate_v1') &&
    sources.postgresRuntimeAggregateScript.includes('auth:postgres-runtime-qa') &&
    sources.postgresRuntimeAggregateScript.includes('cloud:sync-postgres-runtime-qa') &&
    sources.postgresRuntimeAggregateScript.includes('watermark:registry-postgres-runtime-qa') &&
    sources.postgresRuntimeAggregateScript.includes('aggregateDoesNotSwitchDefaultPath') &&
    sources.postgresRuntimeAggregateScript.includes('formalUiMockReleaseDefaultPath') &&
    sources.postgresRuntimeAggregateScript.includes('productionDatabaseAllowed') &&
    sources.postgresRuntimeAggregateScript.includes('P4_sqlite_to_postgres_import_smoke'),
  'P3.4 aggregate Postgres runtime QA must chain auth/sync/registry gates and preserve safety boundaries',
);

assert(
  sources.backendPostgresImportSmoke.includes('sqlite_to_postgres_p4_import_smoke') &&
    sources.backendPostgresImportSmoke.includes('hiddenshield_import_smoke') &&
    sources.backendPostgresImportSmoke.includes('import_sqlite_fixture') &&
    sources.backendPostgresImportSmoke.includes('idempotentRerun') &&
    sources.backendPostgresImportSmoke.includes('row_counts_unchanged') &&
    sources.backendPostgresImportSmoke.includes('hashAggregate') &&
    sources.backendPostgresImportSmoke.includes('primary_key_hash_match') &&
    sources.backendPostgresImportSmoke.includes('logicalReferenceChecks') &&
    sources.backendPostgresImportSmoke.includes('uniqueConstraintChecks') &&
    sources.backendPostgresImportSmoke.includes('formalUiMockReleaseDefaultPath') &&
    sources.backendPostgresImportSmoke.includes('productionDatabaseAllowed') &&
    sources.backendPostgresImportSmoke.includes('cloud_accounts') &&
    sources.backendPostgresImportSmoke.includes('cloud_sync_events') &&
    sources.backendPostgresImportSmoke.includes('watermark_id_registry') &&
    sources.backendPostgresImportSmoke.includes('rights_manifests'),
  'P4 import smoke must verify SQLite fixture import into disposable Postgres with counts, hashes, references and uniqueness checks',
);

assert(
  sources.postgresImportSmokeScript.includes('postgres_import_smoke_artifact_v1') &&
    sources.postgresImportSmokeScript.includes('sqlite_to_postgres_import_smoke') &&
    sources.postgresImportSmokeScript.includes('hiddenshield_import_smoke') &&
    sources.postgresImportSmokeScript.includes('productionDatabaseAllowed') &&
    sources.postgresImportSmokeScript.includes('formalUiMockReleaseDefaultPath') &&
    sources.postgresImportSmokeScript.includes('row_counts_unchanged') &&
    sources.postgresImportSmokeScript.includes('tmp-ui-qa/postgres-import'),
  'P4 import smoke script must write a safety artifact and require a disposable local Postgres database',
);

assert(
  sources.postgresProductionReadinessGate.includes('cloud_postgres_production_readiness_gate_v1') &&
    sources.postgresProductionReadinessGate.includes('HIDDENSHIELD_POSTGRES_REQUIRE_PRODUCTION_READY') &&
    sources.postgresProductionReadinessGate.includes('cloud_postgres_load_gate_artifact_v1') &&
    sources.postgresProductionReadinessGate.includes('cloud_postgres_restore_drill_artifact_v1') &&
    sources.postgresProductionReadinessGate.includes('cloud_postgres_observability_artifact_v1') &&
    sources.postgresProductionReadinessGate.includes('cloud_postgres_cutover_runbook_artifact_v1') &&
    sources.postgresProductionReadinessGate.includes('cloud_postgres_release_owner_signoff_v1') &&
    sources.postgresProductionReadinessGate.includes('productionDatabaseAllowed') &&
    sources.postgresProductionReadinessGate.includes('formalUiMockReleaseDefaultPath') &&
    sources.postgresProductionReadinessGate.includes('blocked'),
  'P5 production readiness gate must machine-block staging load, restore, observability, cutover and owner signoff gaps',
);

assert(
  sources.postgresSqliteShutdownGate.includes('cloud_postgres_sqlite_shutdown_gate_v1') &&
    sources.postgresSqliteShutdownGate.includes('HIDDENSHIELD_POSTGRES_REQUIRE_SQLITE_SHUTDOWN_READY') &&
    sources.postgresSqliteShutdownGate.includes('HIDDENSHIELD_POSTGRES_PRODUCTION_READINESS_ARTIFACT') &&
    sources.postgresSqliteShutdownGate.includes('SqliteForbiddenInProduction') &&
    sources.postgresSqliteShutdownGate.includes('sqlite_dev_test_adapter_still_available') &&
    sources.postgresSqliteShutdownGate.includes('productionDatabaseAllowed') &&
    sources.postgresSqliteShutdownGate.includes('formalUiMockReleaseDefaultPath') &&
    sources.postgresSqliteShutdownGate.includes('blocked'),
  'P6 SQLite shutdown gate must require P5 readiness while preserving SQLite dev/test adapter',
);

assert(
  sources.backendDatabase.includes('pub enum DatabaseBackendKind') &&
    sources.backendDatabase.includes('Sqlite') &&
    sources.backendDatabase.includes('Postgres') &&
    sources.backendDatabase.includes('pub struct DatabaseConfig') &&
    sources.backendDatabase.includes('cfg(feature = "postgres")') &&
    sources.backendDatabase.includes('PostgresPool') &&
    sources.backendDatabase.includes('PgPoolOptions') &&
    sources.backendDatabase.includes('postgres_schema_smoke_sql') &&
    sources.backendDatabase.includes('POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL') &&
    sources.backendDatabase.includes('include_str!("../migrations/postgres/0001_auth_sync_registry.up.sql")') &&
    sources.backendDatabase.includes('schema_migrations') &&
    sources.backendDatabase.includes('cloud_sync_events') &&
    sources.backendDatabase.includes('watermark_id_registry') &&
    sources.backendDatabase.includes('rights_manifests') &&
    sources.backendDatabase.includes('SqliteForbiddenInProduction') &&
    sources.backendDatabase.includes('MissingPostgresUrl') &&
    sources.backendDatabase.includes('postgres://') &&
    sources.backendDatabase.includes('postgresql://'),
  'feedback-backend database module must define SQLite/PostgreSQL backend config and production guard',
);

assert(
  sources.postgresMigrationUp.includes('CREATE TABLE IF NOT EXISTS cloud_accounts') &&
    sources.postgresMigrationUp.includes('CREATE TABLE IF NOT EXISTS cloud_sync_events') &&
    sources.postgresMigrationUp.includes('CREATE TABLE IF NOT EXISTS watermark_id_registry') &&
    sources.postgresMigrationUp.includes('CREATE TABLE IF NOT EXISTS rights_manifests') &&
    sources.postgresMigrationUp.includes('JSONB') &&
    sources.postgresMigrationUp.includes('TIMESTAMPTZ') &&
    sources.postgresMigrationUp.includes('BIGSERIAL') &&
    sources.postgresMigrationDown.includes('DROP TABLE IF EXISTS cloud_accounts') &&
    sources.postgresMigrationDown.includes('DROP TABLE IF EXISTS cloud_sync_events') &&
    sources.postgresMigrationDown.includes('DROP TABLE IF EXISTS watermark_id_registry') &&
    sources.postgresMigrationDown.includes('DROP TABLE IF EXISTS rights_manifests'),
  'db portability contract must verify real Postgres migration files for auth, sync and registry slices',
);

assert(
  sources.backendStorage.includes('open_with_database_config') &&
    sources.backendStorage.includes('DatabaseBackendKind::Postgres') &&
    sources.backendStorage.includes('PostgresAdapterNotImplemented') &&
    sources.backendStorage.includes('Connection::open(sqlite_path)') &&
    sources.backendStorage.includes('init_schema(&conn)') &&
    sources.backendStorage.includes('journal_mode') &&
    sources.backendStorage.includes('foreign_keys'),
  'SQLite adapter must remain the working dev/test backend while PostgreSQL adapter stays an explicit skeleton',
);

assert(
  sources.roadmap.includes('数据库抽象层与 `cloud:db-portability-contract`') &&
    sources.boundary.includes('SQLite 单文件') &&
    sources.boundary.includes('PostgreSQL 迁移'),
  'Roadmap and capability boundary must keep PostgreSQL production boundary visible',
);

const postgresCheck = spawnSync(
  'cargo',
  ['check', '--manifest-path', 'feedback-backend/Cargo.toml', '--features', 'postgres'],
  { encoding: 'utf8', shell: process.platform === 'win32' },
);

if (postgresCheck.status !== 0) {
  process.stdout.write(postgresCheck.stdout ?? '');
  process.stderr.write(postgresCheck.stderr ?? '');
  throw new Error('cargo check --features postgres failed');
}

console.log('cloud db portability contract passed');

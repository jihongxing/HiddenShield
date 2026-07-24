import { existsSync, readFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';

const upPath = 'feedback-backend/migrations/postgres/0001_auth_sync_registry.up.sql';
const downPath = 'feedback-backend/migrations/postgres/0001_auth_sync_registry.down.sql';

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  design: readFileSync('docs/云版权库PostgreSQL迁移设计.md', 'utf8'),
  roadmap: readFileSync('docs/商业化落地Roadmap.md', 'utf8'),
  database: readFileSync('feedback-backend/src/database.rs', 'utf8'),
  smokeBin: readFileSync('feedback-backend/src/bin/postgres_migrate_smoke.rs', 'utf8'),
  smokeScript: readFileSync('scripts/run-postgres-migrate-smoke.mjs', 'utf8'),
  portabilityContract: readFileSync('scripts/verify-cloud-db-portability-contract.mjs', 'utf8'),
  up: existsSync(upPath) ? readFileSync(upPath, 'utf8') : '',
  down: existsSync(downPath) ? readFileSync(downPath, 'utf8') : '',
};

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

assert(existsSync(upPath), `${upPath} must exist`);
assert(existsSync(downPath), `${downPath} must exist`);

const requiredTables = [
  'schema_migrations',
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
];

for (const table of requiredTables) {
  assert(
    sources.up.includes(`CREATE TABLE IF NOT EXISTS ${table}`),
    `up migration must create ${table}`,
  );
  assert(
    sources.down.includes(`DROP TABLE IF EXISTS ${table}`),
    `down migration must drop ${table}`,
  );
}

const requiredIndexes = [
  'idx_auth_challenges_identifier_created',
  'idx_auth_attempts_identifier_created',
  'idx_cloud_sync_events_account_sequence',
  'idx_watermark_id_registry_account_workspace',
  'idx_watermark_id_registry_parent',
  'idx_watermark_id_reissue_jobs_account',
  'idx_rights_manifests_one_active',
  'idx_rights_manifests_watermark',
  'idx_rights_manifests_watermark_status',
  'idx_rights_manifests_watermark_version',
  'idx_rights_manifests_status_updated',
];

for (const index of requiredIndexes) {
  assert(sources.up.includes(`CREATE`) && sources.up.includes(index), `up migration must create ${index}`);
  assert(sources.down.includes(`DROP INDEX IF EXISTS ${index}`), `down migration must drop ${index}`);
}

for (const token of [
  'JSONB',
  'TIMESTAMPTZ',
  'BIGSERIAL',
  'BOOLEAN',
  "WHERE status = 'active'",
  'UNIQUE(account_id, device_id, client_event_id)',
  'UNIQUE(account_id, request_id)',
  'UNIQUE(watermark_uid, manifest_version)',
]) {
  assert(sources.up.includes(token), `up migration missing PostgreSQL token: ${token}`);
}

for (const forbidden of ['AUTOINCREMENT', 'INTEGER PRIMARY KEY AUTOINCREMENT', 'PRAGMA ', 'WITHOUT ROWID']) {
  assert(!sources.up.toUpperCase().includes(forbidden), `up migration must not contain SQLite token: ${forbidden}`);
}

assert(
  sources.database.includes('include_str!("../migrations/postgres/0001_auth_sync_registry.up.sql")') &&
    sources.database.includes('include_str!("../migrations/postgres/0001_auth_sync_registry.down.sql")') &&
    sources.database.includes('POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL') &&
    sources.database.includes('POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL'),
  'database.rs must source Postgres smoke SQL from migration files',
);

assert(
  !sources.database.includes('CREATE TABLE IF NOT EXISTS cloud_accounts (id TEXT PRIMARY KEY') &&
    !sources.database.includes('CREATE TABLE IF NOT EXISTS cloud_sync_events (sequence BIGSERIAL'),
  'database.rs must not keep inline Postgres CREATE TABLE smoke SQL',
);

assert(
  sources.packageJson.includes('"cloud:postgres-migration-contract"') &&
    sources.packageJson.includes('verify-cloud-postgres-migration-contract.mjs') &&
    sources.packageJson.includes('"cloud:postgres-migrate-smoke"') &&
    sources.packageJson.includes('run-postgres-migrate-smoke.mjs'),
  'package.json must expose cloud:postgres-migration-contract',
);

assert(
  sources.smokeBin.includes('POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL') &&
    sources.smokeBin.includes('POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL') &&
    sources.smokeBin.includes('HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL') &&
    sources.smokeBin.includes('is_safe_smoke_url') &&
    sources.smokeBin.includes('hiddenshield_migrate_smoke') &&
    sources.smokeBin.includes('assert_tables_present') &&
    sources.smokeBin.includes('assert_indexes_present') &&
    sources.smokeBin.includes('assert_partial_index') &&
    sources.smokeBin.includes('empty_schema_verified'),
  'postgres_migrate_smoke bin must execute up/down against a disposable database and verify schema',
);

assert(
  sources.smokeScript.includes('HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL') &&
    sources.smokeScript.includes('detectContainerRuntime') &&
    sources.smokeScript.includes('tmp-ui-qa/postgres-migration') &&
    sources.smokeScript.includes('postgres_migration_smoke_artifact_v1') &&
    sources.smokeScript.includes('writeArtifact') &&
    sources.smokeScript.includes('parseSmokeJson') &&
    sources.smokeScript.includes('upTablesChecked') &&
    sources.smokeScript.includes('indexesChecked') &&
    sources.smokeScript.includes('empty_schema_verified') &&
    sources.smokeScript.includes("'podman'") &&
    sources.smokeScript.includes('docker') &&
    sources.smokeScript.includes('postgres:16-alpine') &&
    sources.smokeScript.includes('hiddenshield_migrate_smoke') &&
    sources.smokeScript.includes('postgres_migrate_smoke') &&
    sources.smokeScript.includes('rm') &&
    sources.smokeScript.includes('requires either HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or Podman/Docker'),
  'postgres migrate smoke script must prepare disposable Postgres via env URL or Podman/Docker',
);

assert(
  sources.portabilityContract.includes('cloud:postgres-migration-contract') ||
    sources.portabilityContract.includes('0001_auth_sync_registry.up.sql'),
  'db portability contract must be aware of real Postgres migration files',
);

assert(
  sources.design.includes('P2 评审结论') &&
    sources.design.includes('0001_auth_sync_registry.up.sql') &&
    sources.design.includes('0001_auth_sync_registry.down.sql') &&
    sources.design.includes('cloud:postgres-migration-contract'),
  'migration design must record P2 migration contract boundary',
);

assert(
  sources.roadmap.includes('cloud:postgres-migration-contract') &&
    sources.roadmap.includes('feedback-backend/migrations/postgres'),
  'roadmap must record P2 Postgres migration contract',
);

const postgresCheck = spawnSync(
  'cargo',
  ['test', '--manifest-path', 'feedback-backend/Cargo.toml', 'database::tests', '--lib', '--features', 'postgres'],
  { encoding: 'utf8', shell: process.platform === 'win32' },
);

if (postgresCheck.status !== 0) {
  process.stdout.write(postgresCheck.stdout ?? '');
  process.stderr.write(postgresCheck.stderr ?? '');
  throw new Error('cargo test database::tests --features postgres failed');
}

console.log('cloud postgres migration contract passed');

import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const requireReady = process.env.HIDDENSHIELD_POSTGRES_REQUIRE_SQLITE_SHUTDOWN_READY === '1';
const runId = `cloud-postgres-sqlite-shutdown-gate-${Date.now()}`;
const artifactDir = resolve('tmp-ui-qa/postgres-sqlite-shutdown');

const sources = {
  database: readFileSync('feedback-backend/src/database.rs', 'utf8'),
  storage: readFileSync('feedback-backend/src/storage.rs', 'utf8'),
  packageJson: readFileSync('package.json', 'utf8'),
};

const structuralChecks = [
  {
    key: 'sqlite_forbidden_in_production',
    ok:
      sources.database.includes('SqliteForbiddenInProduction') &&
      sources.database.includes('runtime_mode.is_production()') &&
      sources.database.includes('DatabaseBackendKind::Sqlite'),
  },
  {
    key: 'postgres_url_required_for_postgres_backend',
    ok:
      sources.database.includes('MissingPostgresUrl') &&
      sources.database.includes('InvalidPostgresUrl') &&
      sources.database.includes('postgres://') &&
      sources.database.includes('postgresql://'),
  },
  {
    key: 'sqlite_dev_test_adapter_still_available',
    ok:
      sources.storage.includes('Connection::open(sqlite_path)') &&
      sources.storage.includes('init_schema(&conn)') &&
      sources.packageJson.includes('"cloud:backend"') &&
      sources.packageJson.includes('--db-path feedback-backend/cloud.sqlite'),
  },
];

const productionReadiness = validateProductionReadinessArtifact();
const structuralOk = structuralChecks.every((check) => check.ok);
const blocked = !structuralOk || productionReadiness.status !== 'accepted';

const artifact = {
  schemaVersion: 'cloud_postgres_sqlite_shutdown_gate_v1',
  runId,
  generatedAt: new Date().toISOString(),
  ok: !blocked,
  status: blocked ? 'blocked' : 'passed',
  requiredReadyMode: requireReady,
  productionDatabaseAllowed: false,
  formalUiMockReleaseDefaultPath: 'not_switched',
  checks: {
    structuralChecks,
    productionReadiness,
  },
  blockedReasons: [
    ...structuralChecks
      .filter((check) => !check.ok)
      .map((check) => ({ key: check.key, reason: 'structural_check_failed' })),
    ...(productionReadiness.status === 'accepted'
      ? []
      : [{ key: 'production_readiness', reason: productionReadiness.status }]),
  ],
  nextPhaseCandidate: blocked ? 'complete_P5_production_readiness_before_P6_shutdown' : 'remove_sqlite_from_production_runbook',
};

mkdirSync(artifactDir, { recursive: true });
const artifactPath = resolve(artifactDir, `${runId}.json`);
writeFileSync(artifactPath, `${JSON.stringify(artifact, null, 2)}\n`, 'utf8');
console.log(`Cloud Postgres SQLite shutdown gate artifact: ${artifactPath}`);
console.log(`Cloud Postgres SQLite shutdown status: ${artifact.status.toUpperCase()}`);

if (requireReady && blocked) {
  process.exit(1);
}

function validateProductionReadinessArtifact() {
  const path = process.env.HIDDENSHIELD_POSTGRES_PRODUCTION_READINESS_ARTIFACT;
  if (!path) {
    return {
      env: 'HIDDENSHIELD_POSTGRES_PRODUCTION_READINESS_ARTIFACT',
      path: null,
      status: 'missing_env',
      expectedSchemaVersion: 'cloud_postgres_production_readiness_gate_v1',
    };
  }
  const absolutePath = resolve(path);
  if (!existsSync(absolutePath)) {
    return {
      env: 'HIDDENSHIELD_POSTGRES_PRODUCTION_READINESS_ARTIFACT',
      path: normalizePath(absolutePath),
      status: 'missing_file',
      expectedSchemaVersion: 'cloud_postgres_production_readiness_gate_v1',
    };
  }
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(absolutePath, 'utf8'));
  } catch (error) {
    return {
      env: 'HIDDENSHIELD_POSTGRES_PRODUCTION_READINESS_ARTIFACT',
      path: normalizePath(absolutePath),
      status: 'invalid_json',
      error: String(error),
      expectedSchemaVersion: 'cloud_postgres_production_readiness_gate_v1',
    };
  }
  if (parsed.schemaVersion !== 'cloud_postgres_production_readiness_gate_v1') {
    return {
      env: 'HIDDENSHIELD_POSTGRES_PRODUCTION_READINESS_ARTIFACT',
      path: normalizePath(absolutePath),
      status: 'schema_mismatch',
      actualSchemaVersion: parsed.schemaVersion ?? null,
      expectedSchemaVersion: 'cloud_postgres_production_readiness_gate_v1',
    };
  }
  if (parsed.ok !== true || parsed.status !== 'passed') {
    return {
      env: 'HIDDENSHIELD_POSTGRES_PRODUCTION_READINESS_ARTIFACT',
      path: normalizePath(absolutePath),
      status: 'production_readiness_not_passed',
      artifactOk: parsed.ok ?? null,
      artifactStatus: parsed.status ?? null,
    };
  }
  return {
    env: 'HIDDENSHIELD_POSTGRES_PRODUCTION_READINESS_ARTIFACT',
    path: normalizePath(absolutePath),
    status: 'accepted',
    artifactRunId: parsed.runId ?? null,
  };
}

function normalizePath(path) {
  return path.replaceAll('\\', '/');
}

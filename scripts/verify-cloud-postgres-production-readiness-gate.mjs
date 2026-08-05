import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const requireReady = process.env.HIDDENSHIELD_POSTGRES_REQUIRE_PRODUCTION_READY === '1';
const runId = `cloud-postgres-production-readiness-gate-${Date.now()}`;
const artifactDir = resolve('tmp-ui-qa/postgres-production-readiness');

const requiredArtifacts = [
  {
    key: 'formalHttpGateArtifact',
    env: 'HIDDENSHIELD_POSTGRES_FORMAL_HTTP_GATE_ARTIFACT',
    schemaVersion: 'cloud_postgres_formal_http_gate_v1',
    description: 'Formal backend binary starts with PostgreSQL and passes the unchanged cloud contract/E2E plus registry HTTP round trip',
  },
  {
    key: 'stagingLoadArtifact',
    env: 'HIDDENSHIELD_POSTGRES_STAGING_LOAD_ARTIFACT',
    schemaVersion: 'cloud_postgres_load_gate_artifact_v1',
    description: 'P5 staging load test covering Creator sync push/pull and Enterprise API pressure thresholds',
  },
  {
    key: 'backupRestoreArtifact',
    env: 'HIDDENSHIELD_POSTGRES_BACKUP_RESTORE_ARTIFACT',
    schemaVersion: 'cloud_postgres_restore_drill_artifact_v1',
    description: 'P5 backup, PITR and restore drill artifact from staging',
  },
  {
    key: 'observabilityArtifact',
    env: 'HIDDENSHIELD_POSTGRES_OBSERVABILITY_ARTIFACT',
    schemaVersion: 'cloud_postgres_observability_artifact_v1',
    description: 'P5 slow query, lock wait, deadlock and pool saturation monitoring artifact',
  },
  {
    key: 'cutoverRunbookArtifact',
    env: 'HIDDENSHIELD_POSTGRES_CUTOVER_RUNBOOK_ARTIFACT',
    schemaVersion: 'cloud_postgres_cutover_runbook_artifact_v1',
    description: 'P5 production cutover and rollback window runbook reviewed by release owner',
  },
  {
    key: 'releaseOwnerSignoffArtifact',
    env: 'HIDDENSHIELD_POSTGRES_RELEASE_OWNER_SIGNOFF_ARTIFACT',
    schemaVersion: 'cloud_postgres_release_owner_signoff_v1',
    description: 'P5 release owner signoff after staging load and restore drills pass',
  },
];

const results = requiredArtifacts.map((item) => validateArtifact(item));
const missingOrInvalid = results.filter((item) => item.status !== 'accepted');
const blocked = missingOrInvalid.length > 0;

const artifact = {
  schemaVersion: 'cloud_postgres_production_readiness_gate_v1',
  runId,
  generatedAt: new Date().toISOString(),
  ok: !blocked,
  status: blocked ? 'blocked' : 'passed',
  requiredReadyMode: requireReady,
  productionDatabaseAllowed: false,
  formalUiMockReleaseDefaultPath: 'not_switched',
  checks: results,
  blockedReasons: missingOrInvalid.map((item) => ({
    key: item.key,
    env: item.env,
    reason: item.status,
    description: item.description,
  })),
  nextPhaseCandidate: blocked ? 'provide_formal_http_and_real_staging_operational_artifacts' : 'P6_sqlite_production_path_shutdown',
};

mkdirSync(artifactDir, { recursive: true });
const artifactPath = resolve(artifactDir, `${runId}.json`);
writeFileSync(artifactPath, `${JSON.stringify(artifact, null, 2)}\n`, 'utf8');
console.log(`Cloud Postgres production readiness gate artifact: ${artifactPath}`);
console.log(`Cloud Postgres production readiness status: ${artifact.status.toUpperCase()}`);

if (requireReady && blocked) {
  process.exit(1);
}

function validateArtifact(item) {
  const path = process.env[item.env];
  if (!path) {
    return { ...item, path: null, status: 'missing_env' };
  }
  const absolutePath = resolve(path);
  if (!existsSync(absolutePath)) {
    return { ...item, path: normalizePath(absolutePath), status: 'missing_file' };
  }
  let parsed;
  try {
    parsed = JSON.parse(readFileSync(absolutePath, 'utf8'));
  } catch (error) {
    return { ...item, path: normalizePath(absolutePath), status: 'invalid_json', error: String(error) };
  }
  if (parsed.schemaVersion !== item.schemaVersion) {
    return {
      ...item,
      path: normalizePath(absolutePath),
      status: 'schema_mismatch',
      actualSchemaVersion: parsed.schemaVersion ?? null,
    };
  }
  const semanticFailure = validateArtifactSemantics(item, parsed);
  if (semanticFailure) {
    return {
      ...item,
      path: normalizePath(absolutePath),
      status: semanticFailure,
      environmentClass: parsed.environmentClass ?? null,
      reviewStatus: parsed.reviewStatus ?? null,
      decision: parsed.decision ?? null,
    };
  }
  if (parsed.ok !== true || parsed.status !== 'passed') {
    return {
      ...item,
      path: normalizePath(absolutePath),
      status: 'artifact_not_passing',
      artifactStatus: parsed.status ?? null,
      artifactOk: parsed.ok ?? null,
    };
  }
  return {
    ...item,
    path: normalizePath(absolutePath),
    status: 'accepted',
    artifactRunId: parsed.runId ?? null,
  };
}

function validateArtifactSemantics(item, parsed) {
  if (
    ['stagingLoadArtifact', 'backupRestoreArtifact', 'observabilityArtifact'].includes(item.key) &&
    !['external_staging', 'local_podman_staging_equivalent'].includes(parsed.environmentClass)
  ) {
    return 'environment_class_not_accepted';
  }
  if (
    ['stagingLoadArtifact', 'backupRestoreArtifact', 'observabilityArtifact'].includes(item.key) &&
    parsed.scope !== 'cloud_copyright_core_auth_sync_registry'
  ) {
    return 'scope_mismatch';
  }
  if (
    item.key === 'cutoverRunbookArtifact' &&
    (parsed.releaseOwnerReviewed !== true || parsed.reviewStatus !== 'approved')
  ) {
    return 'release_owner_review_missing';
  }
  if (
    item.key === 'releaseOwnerSignoffArtifact' &&
    (parsed.humanAttestation !== true ||
      parsed.decision !== 'approved' ||
      typeof parsed.signedBy !== 'string' ||
      parsed.signedBy.trim() === '' ||
      typeof parsed.signedAt !== 'string' ||
      parsed.signedAt.trim() === '')
  ) {
    return 'release_owner_signoff_missing';
  }
  return null;
}

function normalizePath(path) {
  return path.replaceAll('\\', '/');
}

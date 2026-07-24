import { spawn } from 'node:child_process';
import { readdirSync, readFileSync, mkdirSync, writeFileSync, statSync } from 'node:fs';
import { join, resolve } from 'node:path';

const runId = `cloud-postgres-runtime-qa-${Date.now()}`;
const artifactDir = resolve('tmp-ui-qa/postgres-runtime-aggregate');

const gates = [
  {
    name: 'auth:postgres-runtime-qa',
    artifactDir: 'tmp-ui-qa/postgres-auth-runtime',
    prefix: 'auth-postgres-runtime-qa-',
    schemaVersion: 'auth_postgres_runtime_qa_artifact_v1',
    safetyChecks: {
      syncRepositoryWritePath: 'not_executed',
      registryRepositoryWritePath: 'not_executed',
    },
  },
  {
    name: 'cloud:sync-postgres-runtime-qa',
    artifactDir: 'tmp-ui-qa/postgres-sync-runtime',
    prefix: 'cloud-sync-postgres-runtime-qa-',
    schemaVersion: 'cloud_sync_postgres_runtime_qa_artifact_v1',
    safetyChecks: {
      registryRepositoryWritePath: 'not_executed',
    },
  },
  {
    name: 'watermark:registry-postgres-runtime-qa',
    artifactDir: 'tmp-ui-qa/postgres-registry-runtime',
    prefix: 'watermark-registry-postgres-runtime-qa-',
    schemaVersion: 'watermark_registry_postgres_runtime_qa_artifact_v1',
    safetyChecks: {
      syncRepositoryWritePath: 'not_executed',
      formalUiMockReleaseDefaultPath: 'not_switched',
    },
  },
];

const startedAt = new Date();
const results = [];

for (const gate of gates) {
  const before = latestArtifact(gate);
  await runNpmScript(gate.name);
  const after = latestArtifact(gate);
  if (!after || after.path === before?.path) {
    throw new Error(`${gate.name} did not create a new artifact`);
  }
  const artifact = JSON.parse(readFileSync(after.path, 'utf8'));
  validateGateArtifact(gate, artifact, after.path);
  results.push({
    name: gate.name,
    artifactPath: relativePath(after.path),
    schemaVersion: artifact.schemaVersion,
    ok: artifact.ok,
    runtime: artifact.runtime,
    image: artifact.image,
    databaseName: artifact.databaseName,
    checks: artifact.checks,
    safety: artifact.safety,
    cleanup: artifact.cleanup,
  });
}

mkdirSync(artifactDir, { recursive: true });
const aggregate = {
  schemaVersion: 'cloud_postgres_runtime_qa_aggregate_v1',
  runId,
  generatedAt: new Date().toISOString(),
  startedAt: startedAt.toISOString(),
  ok: true,
  gates: results,
  safety: {
    productionDatabaseAllowed: false,
    formalUiMockReleaseDefaultPath: 'not_switched',
    aggregateDoesNotSwitchDefaultPath: true,
  },
  nextPhaseCandidate: 'P4_sqlite_to_postgres_import_smoke',
};
const artifactPath = resolve(artifactDir, `${runId}.json`);
writeFileSync(artifactPath, `${JSON.stringify(aggregate, null, 2)}\n`, 'utf8');
console.log(`Cloud Postgres runtime QA aggregate artifact: ${artifactPath}`);

function validateGateArtifact(gate, artifact, path) {
  assert(artifact.schemaVersion === gate.schemaVersion, `${gate.name} artifact schema mismatch: ${path}`);
  assert(artifact.ok === true, `${gate.name} artifact not ok: ${path}`);
  assert(artifact.safety?.productionDatabaseAllowed === false, `${gate.name} must not allow production DB`);
  assert(artifact.cleanup?.status === 'removed', `${gate.name} disposable container cleanup must be removed`);
  for (const [key, expected] of Object.entries(gate.safetyChecks)) {
    const actual = artifact.safety?.[key] ?? artifact.checks?.[key];
    assert(actual === expected, `${gate.name} expected ${key}=${expected}, got ${actual}`);
  }
}

function latestArtifact(gate) {
  let entries = [];
  try {
    entries = readdirSync(gate.artifactDir)
      .filter((name) => name.startsWith(gate.prefix) && name.endsWith('.json'))
      .map((name) => {
        const path = resolve(gate.artifactDir, name);
        return { path, mtimeMs: statSync(path).mtimeMs };
      })
      .sort((a, b) => b.mtimeMs - a.mtimeMs);
  } catch {
    return null;
  }
  return entries[0] ?? null;
}

function runNpmScript(scriptName) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command('npm'), ['run', scriptName], {
      stdio: 'inherit',
      shell: process.platform === 'win32',
    });
    child.on('exit', (code) => {
      if (code === 0) {
        resolvePromise();
      } else {
        reject(new Error(`${scriptName} failed with exit code ${code}`));
      }
    });
    child.on('error', reject);
  });
}

function command(bin) {
  if (process.platform === 'win32' && bin === 'npm') {
    return 'npm.cmd';
  }
  return bin;
}

function relativePath(path) {
  return path.replace(resolve('.'), '').replace(/^[/\\]/, '').replaceAll('\\', '/');
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

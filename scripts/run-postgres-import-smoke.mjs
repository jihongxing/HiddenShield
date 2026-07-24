import { spawn, spawnSync } from 'node:child_process';
import { randomInt } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const image = process.env.HIDDENSHIELD_POSTGRES_TEST_IMAGE || 'postgres:16-alpine';
const password = process.env.HIDDENSHIELD_POSTGRES_TEST_PASSWORD || 'hiddenshield';
const dbName = 'hiddenshield_import_smoke';
const externalUrl = process.env.HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL || process.env.DATABASE_URL;
const runId = `postgres-import-smoke-${Date.now()}`;
const artifactDir = resolve('tmp-ui-qa/postgres-import');

if (externalUrl) {
  const smoke = await runCargoSmoke(externalUrl);
  writeArtifact({
    runId,
    runtime: { kind: 'external_database_url', version: null },
    image: null,
    containerName: null,
    port: null,
    databaseName: dbName,
    smoke,
    cleanup: { attempted: false, status: 'not_applicable_external_database_url' },
  });
  process.exit(0);
}

const containerRuntime = detectContainerRuntime();

if (!containerRuntime) {
  console.error(
    'cloud:postgres-import-smoke requires either HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or Podman/Docker.',
  );
  console.error(
    'Use a disposable database URL containing localhost/127.0.0.1 and hiddenshield_import_smoke.',
  );
  process.exit(2);
}

const containerName = `hiddenshield-postgres-import-${Date.now()}-${randomInt(1000, 9999)}`;
const port = String(randomInt(35433, 45432));
const url = `postgres://postgres:${password}@127.0.0.1:${port}/${dbName}`;
let smoke = null;
let cleanup = { attempted: false, status: 'not_started' };

try {
  const runResult = await runCapture(containerRuntime.kind, [
    'run',
    '--rm',
    '--detach',
    '--name',
    containerName,
    '-e',
    `POSTGRES_PASSWORD=${password}`,
    '-e',
    `POSTGRES_DB=${dbName}`,
    '-p',
    `${port}:5432`,
    image,
  ]);
  const containerId = runResult.stdout.trim();
  if (containerId) {
    console.log(containerId);
  }
  smoke = await waitForPostgresAndRunSmoke(url);
} finally {
  cleanup = { attempted: true, status: 'started' };
  const cleanupResult = await runCapture(containerRuntime.kind, ['rm', '--force', containerName], {
    allowFailure: true,
  });
  cleanup = {
    attempted: true,
    status: cleanupResult.code === 0 ? 'removed' : 'remove_failed',
    stdout: cleanupResult.stdout.trim(),
    stderr: cleanupResult.stderr.trim(),
  };
  if (cleanup.stdout) {
    console.log(cleanup.stdout);
  }
  if (smoke) {
    writeArtifact({
      runId,
      runtime: containerRuntime,
      image,
      containerName,
      port,
      databaseName: dbName,
      smoke,
      cleanup,
    });
  }
}

async function waitForPostgresAndRunSmoke(url) {
  const deadline = Date.now() + 60_000;
  let lastOutput = '';
  while (Date.now() < deadline) {
    const result = spawnSync(
      command('cargo'),
      [
        'run',
        '--quiet',
        '--manifest-path',
        'feedback-backend/Cargo.toml',
        '--features',
        'postgres',
        '--bin',
        'sqlite_to_postgres_import_smoke',
      ],
      {
        env: { ...process.env, HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL: url },
        encoding: 'utf8',
        shell: process.platform === 'win32',
      },
    );
    if (result.status === 0) {
      process.stdout.write(result.stdout ?? '');
      return parseSmokeJson(result.stdout ?? '');
    }
    lastOutput = `${result.stdout ?? ''}${result.stderr ?? ''}`;
    if (!lastOutput.includes('error communicating with database') && !lastOutput.includes('Connection refused')) {
      throw new Error(lastOutput || 'postgres import smoke failed before readiness');
    }
    await sleep(1500);
  }
  throw new Error(`timed out waiting for disposable Postgres: ${lastOutput}`);
}

async function runCargoSmoke(url) {
  const result = await runCapture(
    'cargo',
    [
      'run',
      '--manifest-path',
      'feedback-backend/Cargo.toml',
      '--features',
      'postgres',
      '--bin',
      'sqlite_to_postgres_import_smoke',
    ],
    {
      env: { ...process.env, HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL: url },
    },
  );
  process.stdout.write(result.stdout);
  process.stderr.write(result.stderr);
  return parseSmokeJson(result.stdout);
}

function runCapture(bin, args, options = {}) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command(bin), args, {
      env: options.env ?? process.env,
      shell: process.platform === 'win32',
    });
    let stdout = '';
    let stderr = '';
    child.stdout?.on('data', (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr?.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    child.on('exit', (code) => {
      if (code === 0 || options.allowFailure) {
        resolvePromise({ code, stdout, stderr });
      } else {
        process.stdout.write(stdout);
        process.stderr.write(stderr);
        reject(new Error(`${bin} ${args.join(' ')} failed with exit code ${code}`));
      }
    });
    child.on('error', reject);
  });
}

function command(bin) {
  if (process.platform !== 'win32') {
    return bin;
  }
  if (bin === 'cargo') {
    return 'cargo.exe';
  }
  if (bin === 'podman') {
    return 'podman.exe';
  }
  if (bin === 'docker') {
    return 'docker.exe';
  }
  return bin;
}

function sleep(ms) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, ms));
}

function detectContainerRuntime() {
  for (const candidate of ['podman', 'docker']) {
    const result = spawnSync(command(candidate), ['--version'], {
      encoding: 'utf8',
      shell: process.platform === 'win32',
    });
    if (result.status === 0) {
      console.log(`Using ${candidate} for disposable Postgres import smoke.`);
      return {
        kind: candidate,
        version: (result.stdout || result.stderr || '').trim(),
      };
    }
  }
  return null;
}

function parseSmokeJson(stdout) {
  const line = stdout
    .split(/\r?\n/)
    .map((value) => value.trim())
    .filter((value) => value.startsWith('{') && value.endsWith('}'))
    .at(-1);
  if (!line) {
    throw new Error(`postgres import smoke did not emit JSON: ${stdout}`);
  }
  return JSON.parse(line);
}

function writeArtifact({ runId, runtime, image, containerName, port, databaseName, smoke, cleanup }) {
  if (smoke.idempotentRerun !== 'row_counts_unchanged') {
    throw new Error(`postgres import smoke idempotency check failed: ${smoke.idempotentRerun}`);
  }
  if (smoke.rollback !== 'empty_schema_verified') {
    throw new Error(`postgres import smoke rollback check failed: ${smoke.rollback}`);
  }
  if (smoke.safety?.productionDatabaseAllowed !== false) {
    throw new Error('postgres import smoke must reject production database usage');
  }
  mkdirSync(artifactDir, { recursive: true });
  const artifact = {
    schemaVersion: 'postgres_import_smoke_artifact_v1',
    runId,
    generatedAt: new Date().toISOString(),
    migration: smoke.migration,
    ok: smoke.ok === true,
    runtime,
    image,
    containerName,
    port,
    databaseName,
    checks: {
      source: smoke.source,
      tablesChecked: smoke.tablesChecked,
      totalRowsImported: smoke.totalRowsImported,
      rowCountChecks: smoke.rowCountChecks,
      idempotentRerun: smoke.idempotentRerun,
      hashAggregate: smoke.hashAggregate,
      logicalReferenceChecks: smoke.logicalReferenceChecks,
      uniqueConstraintChecks: smoke.uniqueConstraintChecks,
      rollback: smoke.rollback,
    },
    safety: {
      requiresDisposableDatabaseName: 'hiddenshield_import_smoke',
      allowsOnlyLocalhost: true,
      productionDatabaseAllowed: false,
      formalUiMockReleaseDefaultPath: smoke.safety?.formalUiMockReleaseDefaultPath,
      sqliteSource: smoke.safety?.sqliteSource,
    },
    cleanup,
  };
  const path = resolve(artifactDir, `${runId}.json`);
  writeFileSync(path, `${JSON.stringify(artifact, null, 2)}\n`, 'utf8');
  console.log(`Postgres import smoke artifact: ${path}`);
}

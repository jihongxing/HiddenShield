import { spawn, spawnSync } from 'node:child_process';
import { randomInt } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

const image = process.env.HIDDENSHIELD_POSTGRES_TEST_IMAGE || 'postgres:16-alpine';
const password = process.env.HIDDENSHIELD_POSTGRES_TEST_PASSWORD || 'hiddenshield';
const dbName = 'hiddenshield_registry_runtime_qa';
const externalUrl = process.env.HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL || process.env.DATABASE_URL;
const runId = `watermark-registry-postgres-runtime-qa-${Date.now()}`;
const artifactDir = resolve('tmp-ui-qa/postgres-registry-runtime');

if (externalUrl) {
  const qa = await runCargoQa(externalUrl);
  writeArtifact({
    runId,
    runtime: { kind: 'external_database_url', version: null },
    image: null,
    containerName: null,
    port: null,
    databaseName: dbName,
    qa,
    cleanup: { attempted: false, status: 'not_applicable_external_database_url' },
  });
  process.exit(0);
}

const containerRuntime = detectContainerRuntime();

if (!containerRuntime) {
  console.error(
    'watermark:registry-postgres-runtime-qa requires either HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or Podman/Docker.',
  );
  console.error(
    'Use a disposable database URL containing localhost/127.0.0.1 and hiddenshield_registry_runtime_qa.',
  );
  process.exit(2);
}

const containerName = `hiddenshield-registry-postgres-qa-${Date.now()}-${randomInt(1000, 9999)}`;
const port = String(randomInt(25432, 35432));
const url = `postgres://postgres:${password}@127.0.0.1:${port}/${dbName}`;
let qa = null;
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
  qa = await waitForPostgresAndRunQa(url);
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
  if (qa) {
    writeArtifact({
      runId,
      runtime: containerRuntime,
      image,
      containerName,
      port,
      databaseName: dbName,
      qa,
      cleanup,
    });
  }
}

async function waitForPostgresAndRunQa(url) {
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
        'watermark_registry_postgres_runtime_qa',
      ],
      {
        env: {
          ...process.env,
          HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL: url,
          HIDDENSHIELD_REGISTRY_POSTGRES_QA_RUN_ID: runId,
        },
        encoding: 'utf8',
        shell: process.platform === 'win32',
      },
    );
    if (result.status === 0) {
      process.stdout.write(result.stdout ?? '');
      return parseQaJson(result.stdout ?? '');
    }
    lastOutput = `${result.stdout ?? ''}${result.stderr ?? ''}`;
    if (
      !lastOutput.includes('error communicating with database') &&
      !lastOutput.includes('Connection refused')
    ) {
      throw new Error(lastOutput || 'watermark registry Postgres runtime QA failed before readiness');
    }
    await sleep(1500);
  }
  throw new Error(`timed out waiting for disposable Postgres: ${lastOutput}`);
}

async function runCargoQa(url) {
  const result = await runCapture(
    'cargo',
    [
      'run',
      '--manifest-path',
      'feedback-backend/Cargo.toml',
      '--features',
      'postgres',
      '--bin',
      'watermark_registry_postgres_runtime_qa',
    ],
    {
      env: {
        ...process.env,
        HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL: url,
        HIDDENSHIELD_REGISTRY_POSTGRES_QA_RUN_ID: runId,
      },
    },
  );
  process.stdout.write(result.stdout);
  process.stderr.write(result.stderr);
  return parseQaJson(result.stdout);
}

function runCapture(bin, args, options = {}) {
  return new Promise((resolve, reject) => {
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
        resolve({ code, stdout, stderr });
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
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function detectContainerRuntime() {
  for (const candidate of ['podman', 'docker']) {
    const result = spawnSync(command(candidate), ['--version'], {
      encoding: 'utf8',
      shell: process.platform === 'win32',
    });
    if (result.status === 0) {
      console.log(`Using ${candidate} for watermark registry Postgres runtime QA.`);
      return {
        kind: candidate,
        version: (result.stdout || result.stderr || '').trim(),
      };
    }
  }
  return null;
}

function parseQaJson(stdout) {
  const line = stdout
    .split(/\r?\n/)
    .map((value) => value.trim())
    .filter((value) => value.startsWith('{') && value.endsWith('}'))
    .at(-1);
  if (!line) {
    throw new Error(`watermark registry Postgres runtime QA did not emit JSON: ${stdout}`);
  }
  return JSON.parse(line);
}

function writeArtifact({ runId, runtime, image, containerName, port, databaseName, qa, cleanup }) {
  if (qa?.ok !== true) {
    throw new Error(`watermark registry Postgres runtime QA failed: ${JSON.stringify(qa)}`);
  }
  if (qa?.checks?.syncRepositoryWritePath !== 'not_executed') {
    throw new Error('watermark registry Postgres runtime QA must not execute sync repository writes');
  }
  if (qa?.checks?.formalUiMockReleaseDefaultPath !== 'not_switched') {
    throw new Error('watermark registry Postgres runtime QA must not switch formal default paths');
  }
  mkdirSync(artifactDir, { recursive: true });
  const artifact = {
    schemaVersion: 'watermark_registry_postgres_runtime_qa_artifact_v1',
    runId,
    generatedAt: new Date().toISOString(),
    ok: true,
    runtime,
    image,
    containerName,
    port,
    databaseName,
    repository: qa.repository,
    adapter: qa.adapter,
    checks: qa.checks,
    safety: {
      requiresDisposableDatabaseName: dbName,
      allowsOnlyLocalhost: true,
      productionDatabaseAllowed: qa.productionDatabaseAllowed === true ? true : false,
      syncRepositoryWritePath: qa.checks.syncRepositoryWritePath,
      formalUiMockReleaseDefaultPath: qa.checks.formalUiMockReleaseDefaultPath,
    },
    cleanup,
  };
  if (artifact.safety.productionDatabaseAllowed) {
    throw new Error('watermark registry Postgres runtime QA artifact must never allow production database');
  }
  const path = resolve(artifactDir, `${runId}.json`);
  writeFileSync(path, `${JSON.stringify(artifact, null, 2)}\n`, 'utf8');
  console.log(`Watermark registry Postgres runtime QA artifact: ${path}`);
}

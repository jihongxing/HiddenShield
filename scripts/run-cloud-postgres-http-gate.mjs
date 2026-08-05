import { spawn, spawnSync } from 'node:child_process';
import { randomInt } from 'node:crypto';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { createServer } from 'node:net';
import { resolve } from 'node:path';

const rootDir = resolve('.');
const requestedImage = process.env.HIDDENSHIELD_POSTGRES_TEST_IMAGE;
const password = process.env.HIDDENSHIELD_POSTGRES_TEST_PASSWORD || 'hiddenshield';
const databaseName = 'hiddenshield_http_gate';
const externalUrl =
  process.env.HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL || process.env.DATABASE_URL;
const runId = `cloud-postgres-http-gate-${Date.now()}`;
const artifactDir = resolve('tmp-ui-qa/postgres-http-gate');
const backendExecutable = resolve(
  'feedback-backend',
  'target',
  'debug',
  process.platform === 'win32'
    ? 'hiddenshield-feedback-backend.exe'
    : 'hiddenshield-feedback-backend',
);

let backend;
let containerRuntime;
let containerName;
let databaseUrl = externalUrl;
let image = requestedImage || null;
let cleanup = { backend: 'not_started', database: 'not_started' };
const checks = [];

try {
  if (!databaseUrl) {
    containerRuntime = detectContainerRuntime();
    if (!containerRuntime) {
      throw new Error(
        'cloud:postgres-http-gate requires HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or Podman/Docker',
      );
    }
    image = image || detectLocalPostgresImage(containerRuntime.kind) || 'postgres:16-alpine';
    const port = String(randomInt(35433, 45432));
    containerName = `hiddenshield-postgres-http-gate-${Date.now()}-${randomInt(1000, 9999)}`;
    databaseUrl = `postgres://postgres:${password}@127.0.0.1:${port}/${databaseName}`;
    await runCapture(containerRuntime.kind, [
      'run',
      '--rm',
      '--detach',
      '--name',
      containerName,
      '-e',
      `POSTGRES_PASSWORD=${password}`,
      '-e',
      `POSTGRES_DB=${databaseName}`,
      '-p',
      `${port}:5432`,
      image,
    ]);
  }

  assertSafeDatabaseUrl(databaseUrl);
  await waitForPostgres(databaseUrl);
  await runCargo([
    'run',
    '--quiet',
    '--manifest-path',
    'feedback-backend/Cargo.toml',
    '--features',
    'postgres',
    '--bin',
    'postgres_http_schema',
    '--',
    'reset',
  ], {
    HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL: databaseUrl,
  });
  checks.push({ key: 'postgres_schema_reset', status: 'passed' });

  await runCargo([
    'build',
    '--manifest-path',
    'feedback-backend/Cargo.toml',
    '--features',
    'postgres',
    '--bin',
    'hiddenshield-feedback-backend',
  ]);
  if (!existsSync(backendExecutable)) {
    throw new Error(`formal backend executable not found: ${backendExecutable}`);
  }
  checks.push({ key: 'formal_backend_build', status: 'passed' });

  const cloudUrl = await localBackendUrl();
  backend = spawn(
    backendExecutable,
    [
      '--bind-addr',
      cloudUrl.replace('http://', ''),
      '--database-backend',
      'postgres',
      '--database-url',
      databaseUrl,
      '--deployment-env',
      'staging',
    ],
    {
      cwd: rootDir,
      env: {
        ...process.env,
        HIDDENSHIELD_POSTGRES_HTTP_QA_ENTITLEMENT_GRANT: '1',
        HIDDENSHIELD_POSTGRES_HTTP_QA_INTERNAL_TOKEN: 'local-http-gate-internal-token',
      },
      stdio: ['ignore', 'pipe', 'pipe'],
    },
  );
  backend.stdout.on('data', (chunk) => writePrefixed('postgres-http-backend', chunk));
  backend.stderr.on('data', (chunk) => writePrefixed('postgres-http-backend', chunk));
  await waitForCloud(cloudUrl);
  checks.push({ key: 'formal_postgres_http_startup', status: 'passed' });

  const qaEnvironment = {
    HIDDENSHIELD_CLOUD_URL: cloudUrl,
    HIDDENSHIELD_CLOUD_QA_ENTITLEMENT_MODE: 'postgres_http_gate',
    HIDDENSHIELD_CLOUD_QA_INTERNAL_TOKEN: 'local-http-gate-internal-token',
  };
  await runNodeScript('scripts/verify-cloud-sync-contract.mjs', qaEnvironment);
  checks.push({ key: 'cloud_contract_same_script', status: 'passed' });
  await runNodeScript('scripts/verify-cloud-sync-e2e.mjs', qaEnvironment);
  checks.push({ key: 'cloud_e2e_same_script', status: 'passed' });
  await runNodeScript('scripts/verify-cloud-postgres-http-registry.mjs', qaEnvironment);
  checks.push({ key: 'watermark_registry_http_round_trip', status: 'passed' });

  console.log('Cloud PostgreSQL formal HTTP Gate OK');
} finally {
  if (backend && backend.exitCode == null) {
    backend.kill();
    await waitForBackendExit();
    cleanup.backend = 'stopped';
  }
  if (databaseUrl) {
    try {
      await runCargo([
        'run',
        '--quiet',
        '--manifest-path',
        'feedback-backend/Cargo.toml',
        '--features',
        'postgres',
        '--bin',
        'postgres_http_schema',
        '--',
        'down',
      ], {
        HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL: databaseUrl,
      });
      cleanup.database = 'schema_removed';
    } catch (error) {
      cleanup.database = `schema_cleanup_failed:${error}`;
    }
  }
  if (containerRuntime && containerName) {
    const result = await runCapture(
      containerRuntime.kind,
      ['rm', '--force', containerName],
      { allowFailure: true },
    );
    cleanup.database = result.code === 0 ? 'container_removed' : 'container_remove_failed';
  }
  writeArtifact();
}

function writeArtifact() {
  mkdirSync(artifactDir, { recursive: true });
  const passed = checks.length === 6 && checks.every((check) => check.status === 'passed');
  const artifact = {
    schemaVersion: 'cloud_postgres_formal_http_gate_v1',
    runId,
    generatedAt: new Date().toISOString(),
    ok: passed,
    status: passed ? 'passed' : 'failed',
    formalBackendBinary: normalizePath(backendExecutable),
    databaseBackend: 'postgres',
    databaseName,
    productionDatabaseAllowed: false,
    qaEntitlementGrant: 'explicit_gate_only',
    checks,
    cleanup,
  };
  const path = resolve(artifactDir, `${runId}.json`);
  writeFileSync(path, `${JSON.stringify(artifact, null, 2)}\n`, 'utf8');
  console.log(`Cloud PostgreSQL formal HTTP Gate artifact: ${path}`);
}

async function waitForPostgres(url) {
  const deadline = Date.now() + 90_000;
  let lastError = '';
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
        'postgres_http_schema',
        '--',
        'up',
      ],
      {
        cwd: rootDir,
        env: {
          ...process.env,
          HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL: url,
        },
        encoding: 'utf8',
        shell: process.platform === 'win32',
      },
    );
    if (result.status === 0) {
      return;
    }
    lastError = `${result.stdout ?? ''}${result.stderr ?? ''}`;
    if (
      !lastError.includes('Connection refused') &&
      !lastError.includes('error communicating with database') &&
      !lastError.includes('database system is starting up') &&
      !lastError.includes('ConnectionReset') &&
      !lastError.includes('10054')
    ) {
      throw new Error(lastError || 'PostgreSQL readiness probe failed');
    }
    await delay(1000);
  }
  throw new Error(`timed out waiting for disposable PostgreSQL: ${lastError}`);
}

async function waitForCloud(cloudUrl) {
  const deadline = Date.now() + 120_000;
  while (Date.now() < deadline) {
    if (backend.exitCode != null) {
      throw new Error(`formal PostgreSQL backend exited early with code ${backend.exitCode}`);
    }
    try {
      const response = await fetch(`${cloudUrl}/v1/health`);
      if (response.ok) {
        return;
      }
    } catch (_) {
      // Keep waiting while the formal backend establishes its repository pools.
    }
    await delay(500);
  }
  throw new Error(`formal PostgreSQL backend did not become healthy: ${cloudUrl}`);
}

function runCargo(args, extraEnvironment = {}) {
  return runCapture('cargo', args, {
    env: { ...process.env, ...extraEnvironment },
  });
}

function runNodeScript(scriptPath, extraEnvironment) {
  return runCapture('node', [scriptPath], {
    env: { ...process.env, ...extraEnvironment },
    inherit: true,
  });
}

function runCapture(bin, args, options = {}) {
  return new Promise((resolvePromise, reject) => {
    const child = spawn(command(bin), args, {
      cwd: rootDir,
      env: options.env ?? process.env,
      shell: process.platform === 'win32',
      stdio: options.inherit ? 'inherit' : ['ignore', 'pipe', 'pipe'],
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
        reject(new Error(`${bin} ${args.join(' ')} failed (${code})\n${stdout}${stderr}`));
      }
    });
    child.on('error', reject);
  });
}

function detectContainerRuntime() {
  for (const candidate of ['podman', 'docker']) {
    const result = spawnSync(command(candidate), ['--version'], {
      encoding: 'utf8',
      shell: process.platform === 'win32',
    });
    if (result.status === 0) {
      return {
        kind: candidate,
        version: (result.stdout || result.stderr || '').trim(),
      };
    }
  }
  return null;
}

function detectLocalPostgresImage(runtime) {
  const result = spawnSync(command(runtime), ['images', '--format', '{{.Repository}}:{{.Tag}}'], {
    encoding: 'utf8',
    shell: process.platform === 'win32',
  });
  if (result.status !== 0) {
    return null;
  }
  const images = result.stdout
    .split(/\r?\n/)
    .map((value) => value.trim())
    .filter(Boolean);
  return (
    images.find((value) => value === 'localhost/postgres:16') ||
    images.find((value) => /(?:^|\/)postgres:16(?:-|$)/.test(value)) ||
    images.find((value) => /(?:^|\/)postgres:/.test(value)) ||
    null
  );
}

async function localBackendUrl() {
  const port = await new Promise((resolvePromise, reject) => {
    const server = createServer();
    server.unref();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      server.close(() => {
        if (!address || typeof address === 'string') {
          reject(new Error('failed to allocate backend port'));
          return;
        }
        resolvePromise(address.port);
      });
    });
  });
  return `http://127.0.0.1:${port}`;
}

async function waitForBackendExit() {
  if (!backend || backend.exitCode != null) {
    return;
  }
  await new Promise((resolvePromise) => {
    const timer = setTimeout(resolvePromise, 5000);
    backend.once('exit', () => {
      clearTimeout(timer);
      resolvePromise();
    });
  });
}

function assertSafeDatabaseUrl(url) {
  const lower = url.toLowerCase();
  if (
    (!lower.includes('localhost') && !lower.includes('127.0.0.1')) ||
    !lower.includes(databaseName)
  ) {
    throw new Error(
      `refusing PostgreSQL HTTP Gate database URL without localhost/127.0.0.1 and ${databaseName}`,
    );
  }
}

function writePrefixed(label, chunk) {
  for (const line of chunk.toString().split(/\r?\n/)) {
    if (line) {
      console.log(`[${label}] ${line}`);
    }
  }
}

function command(name) {
  if (process.platform !== 'win32') {
    return name;
  }
  if (name === 'cargo') {
    return 'cargo.exe';
  }
  if (name === 'node') {
    return 'node.exe';
  }
  return name;
}

function normalizePath(path) {
  return path.replaceAll('\\', '/');
}

function delay(milliseconds) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, milliseconds));
}

import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const cloudUrl = (process.env.HIDDENSHIELD_CLOUD_URL ?? 'http://127.0.0.1:43188').replace(/\/$/, '');
const cloudUri = new URL(cloudUrl);
const bindAddr = `${cloudUri.hostname}:${cloudUri.port || '80'}`;
const tempDir = await mkdtemp(join(tmpdir(), 'hiddenshield-cloud-ci-'));
const dbPath = join(tempDir, 'cloud-ci.sqlite');
let backend;

try {
  console.log(`Cloud sync CI endpoint: ${cloudUrl}`);
  console.log(`Cloud sync CI database: ${dbPath}`);

  backend = spawn(command('cargo'), [
    'run',
    '--manifest-path',
    'feedback-backend/Cargo.toml',
    '--',
    '--bind-addr',
    bindAddr,
    '--db-path',
    dbPath,
  ], {
    cwd: rootDir,
    env: process.env,
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  backend.stdout.on('data', (chunk) => writePrefixed('cloud-backend', chunk));
  backend.stderr.on('data', (chunk) => writePrefixed('cloud-backend', chunk));

  await waitForCloud();
  await runNodeScript('scripts/verify-cloud-sync-contract.mjs');
  await runNodeScript('scripts/verify-cloud-sync-e2e.mjs');
  console.log('Cloud sync CI OK');
} finally {
  if (backend && !backend.killed) {
    backend.kill();
    await waitForBackendExit();
  }
  await removeTempDir();
}

async function waitForCloud() {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 60_000) {
    if (backend.exitCode != null) {
      throw new Error(`cloud backend exited early with code ${backend.exitCode}`);
    }
    try {
      const response = await fetch(`${cloudUrl}/v1/health`);
      if (response.ok) {
        console.log('Cloud backend is healthy');
        return;
      }
    } catch (_) {
      // Keep waiting while cargo compiles and starts the server.
    }
    await delay(500);
  }
  throw new Error(`cloud backend did not become healthy within 60s: ${cloudUrl}`);
}

async function runNodeScript(scriptPath) {
  await new Promise((resolvePromise, reject) => {
    const child = spawn(command('node'), [scriptPath], {
      cwd: rootDir,
      env: {
        ...process.env,
        HIDDENSHIELD_CLOUD_URL: cloudUrl,
      },
      stdio: 'inherit',
    });
    child.on('exit', (code) => {
      if (code === 0) {
        resolvePromise();
      } else {
        reject(new Error(`${scriptPath} exited with code ${code}`));
      }
    });
    child.on('error', reject);
  });
}

async function waitForBackendExit() {
  if (!backend || backend.exitCode != null) {
    return;
  }
  await new Promise((resolvePromise) => {
    const timer = setTimeout(resolvePromise, 5_000);
    backend.once('exit', () => {
      clearTimeout(timer);
      resolvePromise();
    });
  });
}

async function removeTempDir() {
  for (let attempt = 0; attempt < 5; attempt += 1) {
    try {
      await rm(tempDir, { recursive: true, force: true });
      return;
    } catch (error) {
      if (attempt === 4 || error?.code !== 'EBUSY') {
        throw error;
      }
      await delay(500);
    }
  }
}

function writePrefixed(label, chunk) {
  for (const line of chunk.toString().split(/\r?\n/)) {
    if (line.length > 0) {
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

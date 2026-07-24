import { spawn } from 'node:child_process';
import { mkdtemp, rm } from 'node:fs/promises';
import { createServer as createHttpServer } from 'node:http';
import { createServer } from 'node:net';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const authUrl = (process.env.HIDDENSHIELD_CLOUD_URL ?? await localBackendUrl()).replace(/\/$/, '');
const authUri = new URL(authUrl);
const bindAddr = `${authUri.hostname}:${authUri.port || '80'}`;
const tempDir = await mkdtemp(join(tmpdir(), 'hiddenshield-auth-contract-'));
const dbPath = join(tempDir, 'auth-contract.sqlite');
const targetDir = join(tempDir, 'target');
let backend;
let otpServer;
const otpDeliveries = [];

try {
  const otpPort = await findAvailablePort();
  const otpEndpoint = `http://127.0.0.1:${otpPort}/otp`;
  otpServer = await startOtpDeliveryServer(otpPort);
  console.log(`Auth contract endpoint: ${authUrl}`);
  console.log(`Auth contract database: ${dbPath}`);
  console.log(`Auth OTP delivery endpoint: ${otpEndpoint}`);

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
    env: {
      ...process.env,
      CARGO_TARGET_DIR: targetDir,
      HIDDENSHIELD_AUTH_OTP_DELIVERY_ENDPOINT: otpEndpoint,
      HIDDENSHIELD_AUTH_OTP_DELIVERY_CHANNEL: 'email',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
  });

  backend.stdout.on('data', (chunk) => writePrefixed('auth-backend', chunk));
  backend.stderr.on('data', (chunk) => writePrefixed('auth-backend', chunk));

  await waitForBackend();
  await runNodeScript('scripts/verify-auth-contract.mjs', {
    HIDDENSHIELD_AUTH_OTP_DELIVERY_ENDPOINT: otpEndpoint,
  });
  console.log('Auth contract runner OK');
} finally {
  if (backend && !backend.killed) {
    backend.kill();
    await waitForBackendExit();
  }
  if (otpServer) {
    await closeOtpDeliveryServer();
  }
  await removeTempDir();
}

async function waitForBackend() {
  const startedAt = Date.now();
  while (Date.now() - startedAt < 300_000) {
    if (backend.exitCode != null) {
      throw new Error(`auth backend exited early with code ${backend.exitCode}`);
    }
    try {
      const response = await fetch(`${authUrl}/v1/health`);
      if (response.ok) {
        console.log('Auth backend is healthy');
        return;
      }
    } catch (_) {
      // Keep waiting while cargo compiles and starts the server.
    }
    await delay(500);
  }
  throw new Error(`auth backend did not become healthy within 300s: ${authUrl}`);
}

async function runNodeScript(scriptPath, extraEnv = {}) {
  await new Promise((resolvePromise, reject) => {
    const child = spawn(command('node'), [scriptPath], {
      cwd: rootDir,
      env: {
        ...process.env,
        HIDDENSHIELD_CLOUD_URL: authUrl,
        ...extraEnv,
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

async function startOtpDeliveryServer(port) {
  const server = createHttpServer(async (request, response) => {
    if (request.method === 'GET' && request.url === '/deliveries') {
      response.writeHead(200, { 'content-type': 'application/json' });
      response.end(JSON.stringify({ deliveries: otpDeliveries }));
      return;
    }
    if (request.method !== 'POST' || request.url !== '/otp') {
      response.writeHead(404);
      response.end();
      return;
    }
    let body = '';
    for await (const chunk of request) {
      body += chunk;
    }
    try {
      otpDeliveries.push(JSON.parse(body));
      response.writeHead(204);
      response.end();
    } catch {
      response.writeHead(400);
      response.end();
    }
  });
  await new Promise((resolvePromise, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', resolvePromise);
  });
  return server;
}

async function closeOtpDeliveryServer() {
  await new Promise((resolvePromise) => {
    otpServer.close(resolvePromise);
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

async function localBackendUrl() {
  const port = await findAvailablePort();
  return `http://127.0.0.1:${port}`;
}

async function findAvailablePort() {
  return await new Promise((resolvePromise, reject) => {
    const server = createServer();
    server.unref();
    server.on('error', reject);
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      server.close(() => {
        if (!address || typeof address === 'string') {
          reject(new Error('failed to allocate an available local port'));
          return;
        }
        resolvePromise(address.port);
      });
    });
  });
}

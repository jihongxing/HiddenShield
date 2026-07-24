#!/usr/bin/env node
import { spawn, spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import net from 'node:net';

const runId = process.env.HIDDENSHIELD_PUBLIC_RIGHTS_IOS_QA_RUN_ID ?? `${Date.now()}`;
const configuredEndpoint = process.env.HIDDENSHIELD_QA_BACKEND_URL?.replace(/\/$/, '') ?? null;
const shouldStartBackend = !configuredEndpoint;
const port = shouldStartBackend ? await freePort() : Number(new URL(configuredEndpoint).port || 80);
const baseUrl = configuredEndpoint ?? `http://127.0.0.1:${port}`;
const outputDir = resolve('tmp-ui-qa', 'ios-public-rights-v3-runtime', runId);
const qaJsonPath = join(outputDir, `ios-public-rights-v3-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `ios-public-rights-v3-runtime-qa-${runId}.md`);
const commandPath = join(outputDir, 'ios-public-rights-v3-runtime-command.txt');
const tmpRoot = join(tmpdir(), `hiddenshield-ios-public-rights-v3-runtime-${runId}`);
const dbPath = join(tmpRoot, 'cloud.sqlite');

mkdirSync(outputDir, { recursive: true });
mkdirSync(tmpRoot, { recursive: true });

let backend;
const childProcesses = [];
try {
  const deviceId = process.env.HIDDENSHIELD_IOS_DEVICE_ID ?? discoverIosDeviceId();
  if (shouldStartBackend) {
    backend = spawn(
      'cargo',
      [
        'run',
        '--manifest-path',
        'feedback-backend/Cargo.toml',
        '--',
        '--bind-addr',
        `127.0.0.1:${port}`,
        '--db-path',
        dbPath,
      ],
      { cwd: process.cwd(), stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true },
    );
    childProcesses.push(backend);
    backend.stdout.on('data', (chunk) => process.stdout.write(`[backend] ${chunk}`));
    backend.stderr.on('data', (chunk) => process.stderr.write(`[backend] ${chunk}`));
  }
  await waitForHealth(baseUrl);
  const args = [
    'run',
    '-d',
    deviceId,
    '-t',
    'tool/ios_public_rights_v3_runtime_qa.dart',
    '--dart-define',
    `HIDDENSHIELD_QA_BACKEND_URL=${baseUrl}`,
    '--dart-define',
    `HIDDENSHIELD_QA_RUN_ID=${runId}`,
  ];
  writeFileSync(commandPath, `flutter ${args.join(' ')}\n`, 'utf8');
  const child = spawn('flutter', args, {
    cwd: resolve('mobile_app'),
    stdio: ['ignore', 'pipe', 'pipe'],
    shell: process.platform === 'win32',
    env: {
      ...process.env,
      FLUTTER_STORAGE_BASE_URL:
        process.env.FLUTTER_STORAGE_BASE_URL ?? 'https://storage.flutter-io.cn',
      PUB_HOSTED_URL: process.env.PUB_HOSTED_URL ?? 'https://pub.flutter-io.cn',
    },
  });
  childProcesses.push(child);
  const mobileResult = await waitForIosQaResult(child, 600_000);
  const result = {
    runId,
    status: 'passed',
    platform: 'ios',
    deviceId,
    baseUrl,
    startedBackend: shouldStartBackend,
    dbPath: shouldStartBackend ? dbPath : null,
    command: `flutter ${args.join(' ')}`,
    result: mobileResult,
    completedAt: new Date().toISOString(),
  };
  writeEvidence(result);
  console.log(`iOS public rights V3 runtime QA OK: ${qaMdPath}`);
} catch (error) {
  const result = {
    runId,
    status: 'blocked_or_failed',
    platform: 'ios',
    baseUrl,
    startedBackend: shouldStartBackend,
    dbPath: shouldStartBackend ? dbPath : null,
    error: String(error?.message ?? error),
    completedAt: new Date().toISOString(),
  };
  writeEvidence(result);
  throw error;
} finally {
  for (const child of childProcesses.reverse()) {
    await stopChild(child);
  }
}

function discoverIosDeviceId() {
  const result = spawnSync('flutter', ['devices'], {
    cwd: process.cwd(),
    encoding: 'utf8',
    shell: process.platform === 'win32',
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(`flutter devices failed with status ${result.status}`);
  }
  const lines = String(result.stdout ?? '')
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  const line = lines.find(
    (candidate) =>
      /iPhone|iPad|Simulator|iOS/i.test(candidate) &&
      !/No devices|Found \d+ connected/i.test(candidate),
  );
  if (!line) {
    throw new Error(
      `No iOS device found. Run on macOS with Xcode/iOS Simulator or set HIDDENSHIELD_IOS_DEVICE_ID. flutter devices output:\n${lines.join('\n')}`,
    );
  }
  const match = line.match(/\(([^)]+)\)$/);
  if (match?.[1]) return match[1];
  const parts = line.split('•').map((part) => part.trim()).filter(Boolean);
  return parts[0] ?? line;
}

function waitForIosQaResult(child, timeoutMs) {
  return new Promise((resolve, reject) => {
    let settled = false;
    let output = '';
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      reject(new Error(`flutter iOS public rights V3 QA timed out after ${timeoutMs}ms\n${tail(output)}`));
    }, timeoutMs);
    const finish = (fn, value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      fn(value);
    };
    const onChunk = (chunk, stream) => {
      const text = chunk.toString('utf8');
      output += text;
      stream.write(text);
      const errorLine = text
        .split(/\r?\n/)
        .find((line) => line.includes('HIDDENSHIELD_IOS_PUBLIC_RIGHTS_QA_ERROR'));
      if (errorLine) {
        finish(reject, new Error(`iOS QA reported error: ${errorLine}\n${tail(output)}`));
        return;
      }
      const resultLine = text
        .split(/\r?\n/)
        .find((line) => line.includes('HIDDENSHIELD_IOS_PUBLIC_RIGHTS_QA_RESULT'));
      if (!resultLine) return;
      const jsonStart = resultLine.indexOf('{');
      if (jsonStart < 0) {
        finish(reject, new Error(`iOS QA result line must contain JSON: ${resultLine}`));
        return;
      }
      const result = JSON.parse(resultLine.slice(jsonStart));
      if (result.passed !== true) {
        finish(reject, new Error(`iOS QA result must pass: ${resultLine}`));
        return;
      }
      finish(resolve, result);
    };
    child.stdout.on('data', (chunk) => onChunk(chunk, process.stdout));
    child.stderr.on('data', (chunk) => onChunk(chunk, process.stderr));
    child.on('exit', (code, signal) => {
      if (settled) return;
      finish(reject, new Error(`flutter iOS public rights V3 QA exited before result: code=${code} signal=${signal}\n${tail(output)}`));
    });
    child.on('error', (error) => finish(reject, error));
  });
}

async function waitForHealth(endpoint) {
  const deadline = Date.now() + 120_000;
  let lastError = 'not started';
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`${endpoint}/v1/health`);
      const body = await response.json().catch(() => ({}));
      if (response.status === 200 && body.ok === true) return;
      lastError = `health ${response.status}`;
    } catch (error) {
      lastError = error.message;
    }
    await sleep(1000);
  }
  throw new Error(`backend did not become healthy: ${lastError}`);
}

function writeEvidence(result) {
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
}

function renderMarkdown(result) {
  const lines = [
    '# iOS Public Rights / V3 Runtime QA',
    '',
    `- runId: \`${result.runId}\``,
    `- status: \`${result.status}\``,
    `- platform: \`${result.platform}\``,
    `- backend: \`${result.baseUrl}\``,
    `- completedAt: \`${result.completedAt}\``,
    '',
  ];
  if (result.result) {
    lines.push(
      '## Checks',
      '',
      `- watermarkUid: \`${result.result.watermarkUid}\``,
      `- publicRightsJsonPass: \`${result.result.publicRightsJsonPass}\``,
      `- publicMetadataJsonPass: \`${result.result.publicMetadataJsonPass}\``,
      `- embeddedImagePass: \`${result.result.embeddedImagePass}\``,
      `- v3DefaultWriteReadPass: \`${result.result.v3DefaultWriteReadPass}\``,
      `- payload: \`V${result.result.payloadProtocolVersion}/${result.result.payloadBytesLength}\``,
      `- anchorProtocol: \`${result.result.anchorProtocol}\``,
      '',
    );
  }
  if (result.error) {
    lines.push('## Blocker / Failure', '', result.error, '');
  }
  lines.push(
    '## Next Step',
    '',
    result.status === 'passed'
      ? 'Attach this evidence to the public-rights completion gate and keep legalConclusion=false in all user-facing copy.'
      : 'Run on macOS with Xcode and an iOS Simulator or real device, or set HIDDENSHIELD_IOS_DEVICE_ID and a device-reachable HIDDENSHIELD_QA_BACKEND_URL.',
    '',
  );
  return `${lines.join('\n')}\n`;
}

async function stopChild(child) {
  if (!child) return;
  if (process.platform === 'win32' && child.pid) {
    try {
      spawnSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
    } catch {
      // Process may already be gone.
    }
  }
  if (child.killed || child.exitCode !== null || child.signalCode !== null) return;
  child.kill();
  await Promise.race([
    new Promise((resolve) => child.once('exit', resolve)),
    sleep(5000),
  ]);
}

function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      server.close(() => resolve(address.port));
    });
    server.on('error', reject);
  });
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function tail(value, max = 4000) {
  const text = String(value ?? '');
  return text.length > max ? text.slice(text.length - max) : text;
}

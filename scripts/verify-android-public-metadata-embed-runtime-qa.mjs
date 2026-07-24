import { execFileSync, spawn, spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import net from 'node:net';

const runId =
  process.env.HIDDENSHIELD_ANDROID_PUBLIC_METADATA_EMBED_QA_RUN_ID ?? `${Date.now()}`;
const endpoint = process.env.HIDDENSHIELD_CLOUD_URL?.replace(/\/$/, '') ?? null;
const shouldStartBackend = !endpoint;
const port = shouldStartBackend ? await freePort() : Number(new URL(endpoint).port || 80);
const baseUrl = endpoint ?? `http://127.0.0.1:${port}`;
const mobileBaseUrl =
  process.env.HIDDENSHIELD_QA_MOBILE_BACKEND_URL ??
  (shouldStartBackend ? `http://127.0.0.1:${port}` : baseUrl);
const adbSerial = process.env.HIDDENSHIELD_QA_ADB_SERIAL ?? 'emulator-5554';
const packageName = 'com.hiddenshield.hidden_shield_mobile';
const activityName = `${packageName}/.MainActivity`;
const tmpRoot = join(tmpdir(), `hiddenshield-android-public-metadata-embed-${runId}`);
const dbPath = join(tmpRoot, 'cloud.sqlite');
const outputDir = resolve('tmp-ui-qa', 'android-public-metadata-embed', runId);
const pulledDir = join(outputDir, 'pulled');
const deviceDir = `/data/data/${packageName}/files/HiddenShieldPublicMetadataEmbed/${runId}`;
const deviceResult = `${deviceDir}/android-public-metadata-embed-result.json`;
const qaJsonPath = join(outputDir, `android-public-metadata-embed-qa-${runId}.json`);
const qaMdPath = join(outputDir, `android-public-metadata-embed-qa-${runId}.md`);
const mobileScreenshotPath = join(outputDir, `android-public-metadata-embed-${runId}.png`);

mkdirSync(tmpRoot, { recursive: true });
mkdirSync(outputDir, { recursive: true });
mkdirSync(pulledDir, { recursive: true });

let backend;
try {
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
    backend.stdout.on('data', (chunk) => process.stdout.write(`[backend] ${chunk}`));
    backend.stderr.on('data', (chunk) => process.stderr.write(`[backend] ${chunk}`));
  }

  await waitForHealth(baseUrl);
  ensureAdbReverse(port);
  appMkdir(deviceDir);

  const flutterEnv = {
    ...process.env,
    FLUTTER_STORAGE_BASE_URL:
      process.env.FLUTTER_STORAGE_BASE_URL ?? 'https://storage.flutter-io.cn',
    PUB_HOSTED_URL: process.env.PUB_HOSTED_URL ?? 'https://pub.flutter-io.cn',
  };
  run(
    'flutter',
    [
      'build',
      'apk',
      '--debug',
      '--target-platform',
      'android-x64',
      '-t',
      'tool/public_metadata_embed_runtime_qa.dart',
      `--dart-define=HIDDENSHIELD_QA_BACKEND_URL=${mobileBaseUrl}`,
      `--dart-define=HIDDENSHIELD_QA_RUN_ID=${runId}`,
      `--dart-define=HIDDENSHIELD_QA_OUTPUT_DIR=${deviceDir}`,
    ],
    { cwd: resolve('mobile_app'), env: flutterEnv },
  );
  adb(['install', '-r', resolve('mobile_app', 'build', 'app', 'outputs', 'flutter-apk', 'app-debug.apk')]);
  adb(['shell', 'am', 'force-stop', packageName]);
  adb(['shell', 'am', 'start', '-n', activityName]);
  waitForDeviceFile(deviceResult, 180_000);
  writeFileSync(mobileScreenshotPath, adbBuffer(['exec-out', 'screencap', '-p']));

  const resultPath = join(pulledDir, 'android-public-metadata-embed-result.json');
  pullFromAppFile(deviceResult, resultPath);
  const mobileResult = JSON.parse(readFileSync(resultPath, 'utf8'));
  for (const row of mobileResult.rows ?? []) {
    pullFromAppFile(row.embeddedPath, join(pulledDir, basename(row.embeddedPath)));
  }
  pullFromAppFile(mobileResult.metadataPath, join(pulledDir, basename(mobileResult.metadataPath)));

  const result = {
    runId,
    baseUrl,
    mobileBaseUrl,
    adbSerial,
    startedBackend: shouldStartBackend,
    dbPath: shouldStartBackend ? dbPath : null,
    outputDir,
    deviceDir,
    screenshot: mobileScreenshotPath,
    mobileResult,
    completedAt: new Date().toISOString(),
    pass: mobileResult.pass === true && mobileResult.rows.every((row) => row.pass === true),
  };
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
  if (!result.pass) {
    throw new Error(`Android public metadata embedded image QA failed: ${qaJsonPath}`);
  }
  console.log('Android public metadata embedded image runtime QA OK');
  console.log(`QA JSON: ${qaJsonPath}`);
  console.log(`QA Markdown: ${qaMdPath}`);
  console.log(`Mobile screenshot: ${mobileScreenshotPath}`);
} finally {
  await stopChild(backend);
}

function run(command, args, options = {}) {
  const useShell = process.platform === 'win32' && command === 'flutter';
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? process.cwd(),
    env: options.env ?? process.env,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    shell: useShell,
    windowsHide: true,
  });
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    if (result.error) console.error(result.error);
    throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
  }
}

function adb(args) {
  run('adb', ['-s', adbSerial, ...args]);
}

function adbBuffer(args) {
  const result = spawnSync('adb', ['-s', adbSerial, ...args], {
    maxBuffer: 64 * 1024 * 1024,
    windowsHide: true,
  });
  if (result.stderr?.length) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    throw new Error(`adb ${args.join(' ')} failed with status ${result.status}`);
  }
  return result.stdout;
}

function ensureAdbReverse(hostPort) {
  adb(['reverse', `tcp:${hostPort}`, `tcp:${hostPort}`]);
}

function waitForDeviceFile(path, timeoutMs) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const result = spawnSync(
      'adb',
      ['-s', adbSerial, 'shell', runAsCommand(`test -f ${shellQuote(path)}`)],
      { encoding: 'utf8', windowsHide: true },
    );
    if (result.status === 0) return;
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1000);
  }
  throw new Error(`timed out waiting for device file ${path}`);
}

function pullFromAppFile(appPath, localPath) {
  const result = spawnSync(
    'adb',
    ['-s', adbSerial, 'exec-out', 'run-as', packageName, 'cat', appPath],
    { maxBuffer: 64 * 1024 * 1024, windowsHide: true },
  );
  if (result.stderr?.length) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    throw new Error(`adb exec-out run-as cat ${appPath} failed with status ${result.status}`);
  }
  mkdirSync(dirname(localPath), { recursive: true });
  writeFileSync(localPath, result.stdout);
}

function appMkdir(path) {
  adb(['shell', runAsCommand(`mkdir -p ${shellQuote(path)}`)]);
}

function runAsCommand(command) {
  return `run-as ${packageName} sh -c ${shellQuote(command)}`;
}

function shellQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

function basename(path) {
  return path.split('/').filter(Boolean).pop() ?? 'file';
}

async function waitForHealth(url) {
  const deadline = Date.now() + 120_000;
  let lastError = 'not started';
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`${url}/v1/health`);
      const body = await response.json();
      if (response.status === 200 && body.ok === true) return;
      lastError = `HTTP ${response.status}`;
    } catch (error) {
      lastError = error.message;
    }
    await sleep(1000);
  }
  throw new Error(`backend health timed out: ${lastError}`);
}

async function stopChild(child) {
  if (!child) return;
  if (process.platform === 'win32' && child.pid) {
    try {
      execFileSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
    } catch {
      // Process may have already exited.
    }
    return;
  }
  if (child.killed || child.exitCode !== null || child.signalCode !== null) return;
  child.kill();
  await Promise.race([new Promise((resolveStop) => child.once('exit', resolveStop)), sleep(5000)]);
}

function freePort() {
  return new Promise((resolvePort, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      server.close(() => resolvePort(address.port));
    });
    server.on('error', reject);
  });
}

function sleep(ms) {
  return new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
}

function renderMarkdown(result) {
  const rows = result.mobileResult.rows ?? [];
  return `# HiddenShield Android 图片嵌入元数据副本运行态 QA

- Run ID: \`${result.runId}\`
- 后端: ${result.baseUrl}
- Android 后端地址: ${result.mobileBaseUrl}
- ADB: \`${result.adbSerial}\`
- 设备目录: \`${result.deviceDir}\`
- 本机证据目录: \`${result.outputDir}\`
- 截图: \`${result.screenshot}\`
- 完成时间: ${result.completedAt}

| 格式 | watermarkUid | manifestHash | 容器 | namespace | UID | manifestHash | legalConclusion=false | 结果 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
${rows
  .map(
    (row) =>
      `| ${row.format} | \`${row.watermarkUid}\` | \`${row.manifestHash}\` | ${mark(row.byteChecks.hasContainer)} | ${mark(row.byteChecks.hasNamespace)} | ${mark(row.byteChecks.hasWatermarkUid)} | ${mark(row.byteChecks.hasManifestHash)} | ${mark(row.byteChecks.hasLegalConclusionFalse)} | ${row.pass ? 'PASS' : 'FAIL'} |`,
  )
  .join('\n')}

## 结论

Android 原生 Flutter 运行态已复用 registry metadata 与 Dart 公开元数据嵌入器，完成 PNG iTXt 与 JPEG APP1 字节级检查；\`legalConclusion=false\` 保持为硬边界。当前移动端版权库历史记录未持久化保护副本路径，因此用户入口需等待“重新选择保护副本并导出”交互设计后再开放。
`;
}

function mark(value) {
  return value ? 'PASS' : 'FAIL';
}

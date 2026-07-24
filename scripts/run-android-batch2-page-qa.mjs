import { execFileSync, spawn, spawnSync } from 'node:child_process';
import {
  existsSync,
  mkdirSync,
  readFileSync,
  writeFileSync,
} from 'node:fs';
import { basename, dirname, join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import net from 'node:net';

const runId =
  process.env.HIDDENSHIELD_ANDROID_BATCH2_QA_RUN_ID ?? `${Date.now()}`;
const endpoint = process.env.HIDDENSHIELD_CLOUD_URL?.replace(/\/$/, '') ?? null;
const shouldStartBackend = !endpoint;
const port = shouldStartBackend ? await freePort() : Number(new URL(endpoint).port || 80);
const baseUrl = endpoint ?? `http://127.0.0.1:${port}`;
const mobileBaseUrl =
  process.env.HIDDENSHIELD_QA_MOBILE_BACKEND_URL ??
  (baseUrl.startsWith('http://127.0.0.1:') ? baseUrl : endpoint);
const adbSerial = process.env.HIDDENSHIELD_QA_ADB_SERIAL ?? 'emulator-5554';
const packageName = 'com.hiddenshield.hidden_shield_mobile';
const activityName = `${packageName}/.MainActivity`;
const tmpRoot = join(tmpdir(), `hiddenshield-android-batch2-${runId}`);
const dbPath = join(tmpRoot, 'cloud.sqlite');
const outputDir = resolve(
  'tmp-ui-qa',
  'desktop-batch2-qa',
  'android-batch2-page-qa',
  runId,
);
const pulledDir = join(outputDir, 'pulled');
const screenshotDir = join(outputDir, 'screenshots');
const apkPath = resolve('mobile_app', 'build', 'app', 'outputs', 'flutter-apk', 'app-debug.apk');
const deviceDir = `/data/data/${packageName}/files/HiddenShieldAndroidBatch2/${runId}`;
const deviceReady = `${deviceDir}/android-batch2-ready-${runId}.txt`;
const deviceResult = `${deviceDir}/android-batch2-page-qa-${runId}.json`;
const localResult = join(pulledDir, `android-batch2-page-qa-${runId}.json`);
const summaryPath = join(outputDir, `android-batch2-page-qa-summary-${runId}.json`);
const markdownPath = join(outputDir, `android-batch2-page-qa-summary-${runId}.md`);

mkdirSync(tmpRoot, { recursive: true });
mkdirSync(outputDir, { recursive: true });
mkdirSync(pulledDir, { recursive: true });
mkdirSync(screenshotDir, { recursive: true });

let backend;
try {
  if (shouldStartBackend) {
    backend = spawn(
      'cargo',
      [
        'run',
        '--manifest-path',
        'feedback-backend/Cargo.toml',
        '--bin',
        'hiddenshield-feedback-backend',
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
  buildApk();
  installAndStart();

  waitForDeviceFile(deviceReady, 420_000);
  screenshot('01-result-top.png');
  swipeUp();
  await sleep(600);
  screenshot('02-result-middle.png');
  swipeUp();
  await sleep(600);
  screenshot('03-result-bottom.png');

  scrollToTop();
  scrollToText('保存或分享保护副本');
  screenshot('04-protected-copy-entry.png');
  tapByText('保存或分享保护副本', { fallbackX: 540, fallbackY: 710 });
  await sleep(2500);
  screenshot('05-protected-copy-share-sheet.png');
  backToApp();

  scrollToText('分享公开元数据 JSON');
  screenshot('06-public-metadata-entry.png');
  tapByText('分享公开元数据 JSON', { fallbackX: 540, fallbackY: 1460 });
  await sleep(2500);
  screenshot('07-public-metadata-share-sheet.png');
  backToApp();

  scrollToText('关闭后端成熟错误');
  screenshot('08-backend-off-mature-error.png');

  pullFromAppFile(deviceResult, localResult);
  const artifact = JSON.parse(readFileSync(localResult, 'utf8'));
  const pulledFiles = pullReferencedArtifacts(artifact);
  const screenshots = collectScreenshots();
  const summary = {
    schemaVersion: 'android_batch2_page_qa_summary_v1',
    runId,
    generatedAt: new Date().toISOString(),
    adbSerial,
    baseUrl,
    mobileBaseUrl,
    startedBackend: shouldStartBackend,
    dbPath: shouldStartBackend ? dbPath : null,
    outputDir,
    artifactPath: localResult,
    markdownPath,
    screenshots,
    pulledFiles,
    artifact,
    pass:
      artifact.ok === true &&
      existsSync(screenshots.protectedCopyShareSheet) &&
      existsSync(screenshots.publicMetadataShareSheet),
  };
  writeFileSync(summaryPath, `${JSON.stringify(summary, null, 2)}\n`, 'utf8');
  writeFileSync(markdownPath, renderMarkdown(summary), 'utf8');
  console.log('Android Batch 2 page QA complete');
  console.log(`Summary JSON: ${summaryPath}`);
  console.log(`Summary Markdown: ${markdownPath}`);
  if (!summary.pass) {
    throw new Error(`Android Batch 2 page QA blocked: ${summaryPath}`);
  }
} finally {
  try {
    adb(['shell', 'am', 'force-stop', packageName]);
  } catch {
    // Device may not be connected after a blocked run.
  }
  await stopChild(backend);
}

function buildApk() {
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
      'tool/android_batch2_page_qa.dart',
      `--dart-define=HIDDENSHIELD_ANDROID_BATCH2_QA_BACKEND_URL=${mobileBaseUrl}`,
      `--dart-define=HIDDENSHIELD_ANDROID_BATCH2_QA_RUN_ID=${runId}`,
      `--dart-define=HIDDENSHIELD_ANDROID_BATCH2_QA_OUTPUT_DIR=${deviceDir}`,
    ],
    { cwd: resolve('mobile_app'), env: flutterEnv },
  );
}

function installAndStart() {
  ensureAdbReverse(port);
  adb(['install', '-r', apkPath]);
  adb(['shell', 'am', 'force-stop', packageName]);
  adb(['shell', 'pm', 'clear', packageName]);
  appMkdir(deviceDir);
  adb(['shell', 'am', 'start', '-S', '-n', activityName]);
}

function pullReferencedArtifacts(artifact) {
  const files = {};
  const refs = {
    protectedImage:
      artifact.protectedCopyShare?.protectedPath ??
      artifact.imageWrite?.protectedPath,
    protectedAudio: artifact.audioWrite?.protectedPath,
    formalReport: artifact.formalReportDraft?.reportPath,
    publicMetadataJson: artifact.publicMetadataExportEntry?.jsonPath,
  };
  for (const [key, value] of Object.entries(refs)) {
    if (!value || typeof value !== 'string') continue;
    const localPath = join(pulledDir, basename(value));
    pullFromAppFile(value, localPath);
    files[key] = localPath;
  }
  return files;
}

function collectScreenshots() {
  return {
    resultTop: join(screenshotDir, '01-result-top.png'),
    resultMiddle: join(screenshotDir, '02-result-middle.png'),
    resultBottom: join(screenshotDir, '03-result-bottom.png'),
    protectedCopyEntry: join(screenshotDir, '04-protected-copy-entry.png'),
    protectedCopyShareSheet: join(screenshotDir, '05-protected-copy-share-sheet.png'),
    publicMetadataEntry: join(screenshotDir, '06-public-metadata-entry.png'),
    publicMetadataShareSheet: join(screenshotDir, '07-public-metadata-share-sheet.png'),
    backendOffMatureError: join(screenshotDir, '08-backend-off-mature-error.png'),
  };
}

function screenshot(fileName) {
  writeFileSync(join(screenshotDir, fileName), adbBuffer(['exec-out', 'screencap', '-p']));
}

function scrollToText(text) {
  for (let i = 0; i < 7; i += 1) {
    const dump = safeUiDump();
    if (findBounds(dump, text)) return true;
    swipeUp();
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 500);
  }
  return false;
}

function tapByText(text, fallback) {
  const dump = safeUiDump();
  const bounds = findBounds(dump, text);
  if (bounds) {
    adb(['shell', 'input', 'tap', String(bounds.x), String(bounds.y)]);
    return;
  }
  adb(['shell', 'input', 'tap', String(fallback.fallbackX), String(fallback.fallbackY)]);
}

function safeUiDump() {
  try {
    return adbText(['exec-out', 'uiautomator', 'dump', '/dev/tty']);
  } catch {
    return '';
  }
}

function findBounds(xml, text) {
  const escaped = escapeRegExp(text);
  const pattern = new RegExp(
    `(?:text|content-desc)="${escaped}"[\\s\\S]*?bounds="\\[(\\d+),(\\d+)\\]\\[(\\d+),(\\d+)\\]"`,
  );
  const match = xml.match(pattern);
  if (!match) return null;
  const left = Number(match[1]);
  const top = Number(match[2]);
  const right = Number(match[3]);
  const bottom = Number(match[4]);
  return {
    x: Math.round((left + right) / 2),
    y: Math.round((top + bottom) / 2),
  };
}

function swipeUp() {
  adb(['shell', 'input', 'swipe', '540', '1700', '540', '420', '450']);
}

function swipeDown() {
  adb(['shell', 'input', 'swipe', '540', '420', '540', '1700', '450']);
}

function scrollToTop() {
  for (let i = 0; i < 7; i += 1) {
    swipeDown();
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 250);
  }
}

function backToApp() {
  adb(['shell', 'input', 'keyevent', 'BACK']);
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1000);
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

function adbText(args) {
  return adbBuffer(args).toString('utf8');
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

function escapeRegExp(value) {
  return String(value).replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
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
      // Already stopped.
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

function renderMarkdown(summary) {
  const rows = summary.artifact.orderedSteps
    .map(
      (step) =>
        `| ${step.label} | ${step.pass ? 'PASS' : 'FAIL'} | ${step.detail} |`,
    )
    .join('\n');
  return `# HiddenShield Android Batch 2 页面级 QA

- Run ID: \`${summary.runId}\`
- 后端: ${summary.baseUrl}
- Android 后端地址: ${summary.mobileBaseUrl}
- ADB: \`${summary.adbSerial}\`
- 证据目录: \`${summary.outputDir}\`
- 原始 artifact: \`${summary.artifactPath}\`
- 完成时间: ${summary.generatedAt}

| 页面 / 场景 | 结果 | 证据摘要 |
| --- | --- | --- |
${rows}

## 截图

- 顶部结果: \`${summary.screenshots.resultTop}\`
- 中段结果: \`${summary.screenshots.resultMiddle}\`
- 底部结果: \`${summary.screenshots.resultBottom}\`
- 保护副本分享入口: \`${summary.screenshots.protectedCopyEntry}\`
- 保护副本系统分享面板: \`${summary.screenshots.protectedCopyShareSheet}\`
- 公开元数据入口: \`${summary.screenshots.publicMetadataEntry}\`
- 公开元数据系统分享面板: \`${summary.screenshots.publicMetadataShareSheet}\`
- 关闭后端成熟错误: \`${summary.screenshots.backendOffMatureError}\`

## 结论

Android Batch 2 剩余页面级 QA ${summary.pass ? '通过' : '阻断'}。本轮只验证 Android 原生端页面/运行态，不替代 iOS、真实支付、生产 C2PA/TSA、生产 PostgreSQL 或 L3 可售 SLA。
`;
}

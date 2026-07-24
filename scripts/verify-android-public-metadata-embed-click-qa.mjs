import { execFileSync, spawn, spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import net from 'node:net';

const rootRunId =
  process.env.HIDDENSHIELD_ANDROID_PUBLIC_METADATA_EMBED_CLICK_QA_RUN_ID ?? `${Date.now()}`;
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
const tmpRoot = join(tmpdir(), `hiddenshield-android-public-metadata-click-${rootRunId}`);
const dbPath = join(tmpRoot, 'cloud.sqlite');
const outputDir = resolve('tmp-ui-qa', 'android-public-metadata-embed-click', rootRunId);
const pulledDir = join(outputDir, 'pulled');
const apkPath = resolve('mobile_app', 'build', 'app', 'outputs', 'flutter-apk', 'app-debug.apk');
const qaJsonPath = join(outputDir, `android-public-metadata-embed-click-qa-${rootRunId}.json`);
const qaMdPath = join(outputDir, `android-public-metadata-embed-click-qa-${rootRunId}.md`);

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

  const rows = [];
  for (const format of ['png', 'jpeg']) {
    await waitForHealth(baseUrl);
    ensureAdbReverse(port);
    rows.push(await runClickQaCase(format));
  }

  const result = {
    runId: rootRunId,
    baseUrl,
    mobileBaseUrl,
    adbSerial,
    startedBackend: shouldStartBackend,
    dbPath: shouldStartBackend ? dbPath : null,
    outputDir,
    completedAt: new Date().toISOString(),
    rows,
    pass: rows.every((row) => row.pass === true),
  };
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
  if (!result.pass) {
    throw new Error(`Android public metadata embedded image click QA failed: ${qaJsonPath}`);
  }
  console.log('Android public metadata embedded image click QA OK');
  console.log(`QA JSON: ${qaJsonPath}`);
  console.log(`QA Markdown: ${qaMdPath}`);
} finally {
  await stopChild(backend);
}

async function runClickQaCase(format) {
  const runId = `${rootRunId}-${format}`;
  const caseOutputDir = join(outputDir, format);
  const casePullDir = join(pulledDir, format);
  const deviceDir = `/data/data/${packageName}/files/HiddenShieldPublicMetadataEmbedClick/${runId}`;
  const deviceReady = `${deviceDir}/android-click-ready-${runId}.txt`;
  const deviceResult = `${deviceDir}/android-click-result-${runId}.json`;
  const screenshotReady = join(caseOutputDir, `android-public-metadata-click-ready-${runId}.png`);
  const screenshotAfterClick = join(caseOutputDir, `android-public-metadata-click-after-${runId}.png`);
  const pulledResultPath = join(casePullDir, `android-click-result-${runId}.json`);

  mkdirSync(caseOutputDir, { recursive: true });
  mkdirSync(casePullDir, { recursive: true });

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
      'tool/public_metadata_embed_click_qa.dart',
      `--dart-define=HIDDENSHIELD_QA_BACKEND_URL=${mobileBaseUrl}`,
      `--dart-define=HIDDENSHIELD_QA_RUN_ID=${runId}`,
      `--dart-define=HIDDENSHIELD_QA_OUTPUT_DIR=${deviceDir}`,
      `--dart-define=HIDDENSHIELD_QA_IMAGE_FORMAT=${format}`,
    ],
    { cwd: resolve('mobile_app'), env: flutterEnv },
  );
  await waitForHealth(baseUrl);
  ensureAdbReverse(port);
  adb(['install', '-r', apkPath]);
  adb(['shell', 'am', 'force-stop', packageName]);
  adb(['shell', 'pm', 'clear', packageName]);
  appMkdir(deviceDir);
  adb(['shell', 'am', 'start', '-S', '-n', activityName]);
  waitForDeviceFile(deviceReady, 180_000);
  writeFileSync(screenshotReady, adbBuffer(['exec-out', 'screencap', '-p']));

  tapExportButton();
  waitForDeviceFile(deviceResult, 180_000);
  await sleep(3000);
  writeFileSync(screenshotAfterClick, adbBuffer(['exec-out', 'screencap', '-p']));

  pullFromAppFile(deviceResult, pulledResultPath);
  const mobileResult = JSON.parse(readFileSync(pulledResultPath, 'utf8'));
  const embeddedLocalPath = join(casePullDir, basename(mobileResult.embeddedPath));
  const protectedLocalPath = join(casePullDir, basename(mobileResult.protectedPath));
  pullFromAppFile(mobileResult.embeddedPath, embeddedLocalPath);
  pullFromAppFile(mobileResult.protectedPath, protectedLocalPath);
  adb(['shell', 'am', 'force-stop', packageName]);
  adb(['shell', 'input', 'keyevent', 'HOME']);
  const embeddedBytes = readFileSync(embeddedLocalPath);
  const embeddedText = embeddedBytes.toString('latin1');
  const byteContains = {
    watermarkUid: embeddedText.includes(mobileResult.watermarkUid),
    manifestHash: embeddedText.includes(mobileResult.manifestHash),
    legalConclusionFalse:
      embeddedText.includes('legalConclusion="false"') ||
      embeddedText.includes('&quot;legalConclusion&quot;:false') ||
      embeddedText.includes('&quot;hiddenShield:LegalConclusion&quot;:false'),
  };

  return {
    runId,
    format,
    deviceDir,
    deviceReady,
    deviceResult,
    screenshots: {
      ready: screenshotReady,
      afterClick: screenshotAfterClick,
    },
    pulled: {
      result: pulledResultPath,
      embedded: embeddedLocalPath,
      protected: protectedLocalPath,
    },
    mobileResult,
    byteContains,
    pass:
      mobileResult.pass === true &&
      mobileResult.legalConclusion === false &&
      mobileResult.byteChecks?.hasWatermarkUid === true &&
      mobileResult.byteChecks?.hasManifestHash === true &&
      mobileResult.byteChecks?.hasLegalConclusionFalse === true &&
      byteContains.watermarkUid === true &&
      byteContains.manifestHash === true &&
      byteContains.legalConclusionFalse === true,
  };
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

function tapExportButton() {
  const dump = adbText(['exec-out', 'uiautomator', 'dump', '/dev/tty']);
  const bounds = findBounds(dump, '导出嵌入元数据图片副本');
  if (bounds) {
    adb(['shell', 'input', 'tap', String(bounds.x), String(bounds.y)]);
    return;
  }
  adb(['shell', 'input', 'tap', '540', '640']);
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

function adbText(args) {
  return adbBuffer(args).toString('utf8');
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
  return `# HiddenShield Android 嵌入元数据图片副本点击 QA

- Run ID: \`${result.runId}\`
- 后端: ${result.baseUrl}
- Android 后端地址: ${result.mobileBaseUrl}
- ADB: \`${result.adbSerial}\`
- 本机证据目录: \`${result.outputDir}\`
- 完成时间: ${result.completedAt}

| 格式 | watermarkUid | manifestHash | 详情页截图 | 点击后截图 | 字节 UID | 字节 manifestHash | legalConclusion=false | 结果 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
${result.rows
  .map(
    (row) =>
      `| ${row.format} | \`${row.mobileResult.watermarkUid}\` | \`${row.mobileResult.manifestHash}\` | \`${row.screenshots.ready}\` | \`${row.screenshots.afterClick}\` | ${mark(row.byteContains.watermarkUid)} | ${mark(row.byteContains.manifestHash)} | ${mark(row.byteContains.legalConclusionFalse)} | ${row.pass ? 'PASS' : 'FAIL'} |`,
  )
  .join('\n')}

## 结论

Android 原生端已完成端到端点击 QA：进入图片版权库详情 QA 页面，点击“导出嵌入元数据图片副本”，对真实 PNG/JPEG 保护副本导出嵌入副本，并从分享前产物字节确认 \`watermarkUid\`、\`manifestHash\`、\`legalConclusion=false\` 均存在。该 QA 不替代 iOS Simulator 或真机 QA。
`;
}

function mark(value) {
  return value ? 'PASS' : 'FAIL';
}

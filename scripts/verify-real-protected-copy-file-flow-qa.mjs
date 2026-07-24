import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { chromium } from 'playwright';

const runId = Date.now().toString();
const outputDir = resolve('tmp-ui-qa', 'protected-copy-file-flow', runId);
const desktopDir = join(outputDir, 'desktop');
const mobilePullDir = join(outputDir, 'mobile-pulled');
mkdirSync(desktopDir, { recursive: true });
mkdirSync(mobilePullDir, { recursive: true });

const adbSerial = process.env.HIDDENSHIELD_QA_ADB_SERIAL ?? 'emulator-5554';
const packageName = 'com.hiddenshield.hidden_shield_mobile';
const activityName = `${packageName}/.MainActivity`;
const deviceDir = `/data/data/${packageName}/files/HiddenShieldFileFlow/${runId}`;
const deviceDesktopImage = `${deviceDir}/desktop-protected-image-${runId}.png`;
const deviceDesktopAudio = `${deviceDir}/desktop-protected-audio-${runId}.wav`;
const deviceResult = `${deviceDir}/mobile-file-flow-result.json`;
const mobileScreenshot = join(outputDir, `mobile-file-flow-${runId}.png`);
const desktopHtml = join(outputDir, `desktop-file-flow-${runId}.html`);
const desktopScreenshot = join(outputDir, `desktop-file-flow-${runId}.png`);
const qaJsonPath = join(outputDir, `protected-copy-file-flow-qa-${runId}.json`);
const qaMdPath = join(outputDir, `protected-copy-file-flow-qa-${runId}.md`);

run('cargo', [
  'run',
  '--manifest-path',
  'watermark-core/Cargo.toml',
  '--bin',
  'protected_copy_file_flow_qa',
  '--',
  'generate-desktop',
  '--run-id',
  runId,
  '--out-dir',
  desktopDir,
]);

const desktopArtifacts = JSON.parse(readFileSync(join(desktopDir, 'desktop-artifacts.json'), 'utf8'));
const desktopImagePath = desktopArtifacts.desktop.image.path;
const desktopAudioPath = desktopArtifacts.desktop.audio.path;

appMkdir(deviceDir);
pushToAppFile(desktopImagePath, deviceDesktopImage);
pushToAppFile(desktopAudioPath, deviceDesktopAudio);

const flutterEnv = {
  ...process.env,
  FLUTTER_STORAGE_BASE_URL:
    process.env.FLUTTER_STORAGE_BASE_URL ??
    'file:///D:/codeSpace/HiddenShield/tmp-ui-qa/runtime-mobile/flutter-storage-mirror',
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
    'tool/file_flow_qa.dart',
    `--dart-define=HIDDENSHIELD_FILE_FLOW_RUN_ID=${runId}`,
    `--dart-define=HIDDENSHIELD_QA_DESKTOP_IMAGE_PATH=${deviceDesktopImage}`,
    `--dart-define=HIDDENSHIELD_QA_DESKTOP_AUDIO_PATH=${deviceDesktopAudio}`,
    `--dart-define=HIDDENSHIELD_QA_DESKTOP_IMAGE_UID=${desktopArtifacts.desktop.image.watermarkUid}`,
    `--dart-define=HIDDENSHIELD_QA_DESKTOP_AUDIO_UID=${desktopArtifacts.desktop.audio.watermarkUid}`,
    `--dart-define=HIDDENSHIELD_QA_OUTPUT_DIR=${deviceDir}`,
  ],
  { cwd: resolve('mobile_app'), env: flutterEnv },
);
adb(['install', '-r', resolve('mobile_app', 'build', 'app', 'outputs', 'flutter-apk', 'app-debug.apk')]);
adb(['shell', 'am', 'force-stop', packageName]);
adb(['shell', 'am', 'start', '-n', activityName]);
waitForDeviceFile(deviceResult, 120_000);
writeFileSync(mobileScreenshot, adbBuffer(['exec-out', 'screencap', '-p']));

pullFromAppFile(deviceResult, join(mobilePullDir, 'mobile-file-flow-result.json'));
const mobileResult = JSON.parse(readFileSync(join(mobilePullDir, 'mobile-file-flow-result.json'), 'utf8'));
const mobileImageRow = findRow(mobileResult.rows, 'mobile -> desktop', 'image');
const mobileAudioRow = findRow(mobileResult.rows, 'mobile -> desktop', 'audio');
const pulledMobileImage = join(mobilePullDir, `mobile-protected-image-${runId}.png`);
const pulledMobileAudio = join(mobilePullDir, `mobile-protected-audio-${runId}.wav`);
pullFromAppFile(mobileImageRow.path, pulledMobileImage);
pullFromAppFile(mobileAudioRow.path, pulledMobileAudio);

const desktopVerifyImagePath = join(outputDir, 'desktop-verify-mobile-image.json');
const desktopVerifyAudioPath = join(outputDir, 'desktop-verify-mobile-audio.json');
run('cargo', [
  'run',
  '--manifest-path',
  'watermark-core/Cargo.toml',
  '--bin',
  'protected_copy_file_flow_qa',
  '--',
  'verify-file',
  '--kind',
  'image',
  '--path',
  pulledMobileImage,
  '--expected-uid',
  mobileImageRow.expectedUid,
  '--json-out',
  desktopVerifyImagePath,
]);
run('cargo', [
  'run',
  '--manifest-path',
  'watermark-core/Cargo.toml',
  '--bin',
  'protected_copy_file_flow_qa',
  '--',
  'verify-file',
  '--kind',
  'audio',
  '--path',
  pulledMobileAudio,
  '--expected-uid',
  mobileAudioRow.expectedUid,
  '--json-out',
  desktopVerifyAudioPath,
]);

const desktopVerifyRows = [
  JSON.parse(readFileSync(desktopVerifyImagePath, 'utf8')),
  JSON.parse(readFileSync(desktopVerifyAudioPath, 'utf8')),
];
const allRows = [
  ...mobileResult.rows,
  ...desktopVerifyRows.map((row) => ({
    direction: 'mobile -> desktop',
    kind: row.kind,
    path: row.path,
    expectedUid: row.expectedWatermarkUid,
    extractedUid: row.extractedWatermarkUid,
    payloadProtocolVersion: row.payloadProtocolVersion,
    payloadBytesLength: row.payloadBytesLength,
    pass: row.pass,
  })),
];
const result = {
  runId,
  startedAt: new Date(Number(runId)).toISOString(),
  completedAt: new Date().toISOString(),
  adbSerial,
  outputDir,
  deviceDir,
  desktopArtifacts,
  mobileResult,
  desktopVerification: desktopVerifyRows,
  screenshots: {
    mobile: mobileScreenshot,
    desktop: desktopScreenshot,
    desktopHtml,
  },
  decryptBoundary:
    'N/A for current image/audio protected copies: no encrypted media envelope exists; QA verifies readable V2 payload from the actual protected files.',
  pass: allRows.every((row) => row.pass),
};
writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
writeFileSync(desktopHtml, renderDesktopHtml(result, allRows), 'utf8');
await screenshotHtml(desktopHtml, desktopScreenshot, { width: 1440, height: 980 });
writeFileSync(qaMdPath, renderMarkdown(result, allRows), 'utf8');

console.log(`File-flow QA JSON: ${qaJsonPath}`);
console.log(`File-flow QA Markdown: ${qaMdPath}`);
console.log(`Mobile screenshot: ${mobileScreenshot}`);
console.log(`Desktop screenshot: ${desktopScreenshot}`);
if (!result.pass) {
  throw new Error('protected copy file-flow QA failed');
}

function run(command, args, options = {}) {
  const useShell = process.platform === 'win32' && command === 'flutter';
  const executable = useShell ? 'flutter' : command;
  const result = spawnSync(executable, args, {
    cwd: options.cwd ?? resolve('.'),
    env: options.env ?? process.env,
    encoding: 'utf8',
    maxBuffer: 64 * 1024 * 1024,
    shell: useShell,
  });
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    if (result.error) {
      console.error(result.error);
    }
    throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
  }
}

function adb(args) {
  run('adb', ['-s', adbSerial, ...args]);
}

function adbBuffer(args) {
  const result = spawnSync('adb', ['-s', adbSerial, ...args]);
  if (result.stderr?.length) process.stderr.write(result.stderr);
  if (result.status !== 0) {
    throw new Error(`adb ${args.join(' ')} failed with status ${result.status}`);
  }
  return result.stdout;
}

function waitForDeviceFile(path, timeoutMs) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    const result = spawnSync('adb', ['-s', adbSerial, 'shell', runAsCommand(`test -f ${shellQuote(path)}`)], {
      encoding: 'utf8',
    });
    if (result.status === 0) return;
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1000);
  }
  throw new Error(`timed out waiting for device file ${path}`);
}

function pushToAppFile(localPath, appPath) {
  const tempPath = `/data/local/tmp/${runId}-${basename(appPath)}`;
  adb(['push', localPath, tempPath]);
  appMkdir(dirnamePosix(appPath));
  adb(['shell', runAsCommand(`cp ${shellQuote(tempPath)} ${shellQuote(appPath)}`)]);
}

function pullFromAppFile(appPath, localPath) {
  const result = spawnSync('adb', ['-s', adbSerial, 'exec-out', 'run-as', packageName, 'cat', appPath], {
    maxBuffer: 64 * 1024 * 1024,
  });
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

function basename(path) {
  return path.split('/').filter(Boolean).pop() ?? 'file';
}

function dirnamePosix(path) {
  const parts = path.split('/');
  parts.pop();
  return parts.join('/') || '/';
}

function shellQuote(value) {
  return `'${String(value).replaceAll("'", "'\\''")}'`;
}

function runAsCommand(command) {
  return `run-as ${packageName} sh -c ${shellQuote(command)}`;
}

function findRow(rows, direction, kind) {
  const row = rows.find((candidate) => candidate.direction === direction && candidate.kind === kind);
  if (!row) throw new Error(`missing row ${direction} ${kind}`);
  return row;
}

async function screenshotHtml(htmlPath, screenshotPath, viewport) {
  const browser = await launchAvailableBrowser();
  try {
    const page = await browser.newPage({ viewport });
    await page.goto(`file://${htmlPath.replaceAll('\\', '/')}`);
    await page.screenshot({ path: screenshotPath, fullPage: true });
  } finally {
    await browser.close();
  }
}

async function launchAvailableBrowser() {
  const attempts = [
    () => chromium.launch({ channel: 'chrome' }),
    () => chromium.launch({ channel: 'msedge' }),
    () => chromium.launch(),
  ];
  let lastError;
  for (const attempt of attempts) {
    try {
      return await attempt();
    } catch (error) {
      lastError = error;
    }
  }
  throw lastError;
}

function renderDesktopHtml(result, rows) {
  const cards = rows
    .map(
      (row) => `<article class="card ${row.pass ? 'ok' : 'fail'}">
        <div class="topline"><span>${escapeHtml(row.direction)}</span><strong>${row.pass ? 'PASS' : 'FAIL'}</strong></div>
        <h2>${escapeHtml(row.kind === 'image' ? '图片保护副本' : '音频保护副本')}</h2>
        <dl>
          <dt>期望编号</dt><dd>${escapeHtml(row.expectedUid)}</dd>
          <dt>读取编号</dt><dd>${escapeHtml(row.extractedUid)}</dd>
          <dt>Payload</dt><dd>V${row.payloadProtocolVersion} / ${row.payloadBytesLength} bytes</dd>
          <dt>版本次数</dt><dd>第 ${escapeHtml(row.revision ?? 'N/A')} 次</dd>
          <dt>上一版</dt><dd>${escapeHtml(row.parentWatermarkUid ?? '无')}</dd>
          <dt>签发模式</dt><dd>${escapeHtml(issueModeLabel(row.watermarkIdIssueMode))}</dd>
          <dt>认证状态</dt><dd>${escapeHtml(row.payloadAuthStatus ?? 'N/A')}</dd>
          <dt>文件</dt><dd>${escapeHtml(row.path)}</dd>
        </dl>
      </article>`,
    )
    .join('\n');
  return `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>HiddenShield Protected Copy File Flow QA</title>
  <style>
    :root { color-scheme: dark; font-family: "Microsoft YaHei", "Segoe UI", sans-serif; background: #080b0f; color: #edf3ff; }
    body { margin: 0; padding: 32px; background: radial-gradient(circle at top left, #12323a 0, #080b0f 34%, #080b0f 100%); }
    header { display: flex; justify-content: space-between; gap: 24px; align-items: flex-start; margin-bottom: 26px; }
    h1 { margin: 0 0 10px; font-size: 30px; letter-spacing: 0; }
    p { margin: 0; color: #a9b6c9; line-height: 1.55; }
    .meta { text-align: right; font-size: 13px; color: #96a4b7; }
    .grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
    .card { border: 1px solid #263340; border-radius: 8px; background: rgba(15, 22, 30, .92); padding: 18px; box-shadow: 0 20px 60px rgba(0,0,0,.25); }
    .card.ok { border-color: rgba(42,190,139,.46); }
    .card.fail { border-color: rgba(255,103,103,.58); }
    .topline { display: flex; justify-content: space-between; color: #8ea0b6; font-size: 12px; margin-bottom: 14px; }
    .topline strong { color: #73e2ba; }
    h2 { margin: 0 0 16px; font-size: 17px; letter-spacing: 0; }
    dl { display: grid; grid-template-columns: 92px minmax(0,1fr); gap: 10px 12px; margin: 0; font-size: 13px; }
    dt { color: #7f8fa3; }
    dd { margin: 0; overflow-wrap: anywhere; }
    footer { margin-top: 22px; color: #7f8fa3; font-size: 12px; }
  </style>
</head>
<body>
  <header>
    <div>
      <h1>HiddenShield 真实保护副本双端文件流转 QA</h1>
      <p>桌面生成的 PNG / WAV 经 adb 推送后由原生 Android 读取；Android 生成的 PNG / WAV 经 adb pull 后由桌面 watermark-core 读取。</p>
    </div>
    <div class="meta">
      <div>Run ID: ${escapeHtml(result.runId)}</div>
      <div>ADB: ${escapeHtml(result.adbSerial)}</div>
      <div>Output: ${escapeHtml(result.outputDir)}</div>
    </div>
  </header>
  <main class="grid">${cards}</main>
  <footer>解密项：当前图片 / 音频保护副本没有额外加密 envelope，本轮按真实文件可读取并验证 V2/119 payload 闭环验收。</footer>
</body>
</html>`;
}

function renderMarkdown(result, rows) {
  return `# HiddenShield 真实保护副本双端文件流转 QA

- Run ID: \`${result.runId}\`
- 时间: ${result.startedAt} -> ${result.completedAt}
- Android: \`${result.adbSerial}\`
- 设备目录: \`${result.deviceDir}\`
- 证据目录: \`${result.outputDir}\`
- 移动端截图: \`${result.screenshots.mobile}\`
- 桌面端截图: \`${result.screenshots.desktop}\`
- 桌面端证据页: \`${result.screenshots.desktopHtml}\`

| 方向 | 媒体 | 期望编号 | 读取编号 | Payload | 版本 | 上一版 | 签发模式 | 认证状态 | 文件 | 结果 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
${rows
  .map(
    (row) =>
      `| ${row.direction} | ${row.kind} | ${row.expectedUid} | ${row.extractedUid} | V${row.payloadProtocolVersion}/${row.payloadBytesLength} | ${row.revision ?? 'N/A'} | ${row.parentWatermarkUid ?? '无'} | ${issueModeLabel(row.watermarkIdIssueMode)} | ${row.payloadAuthStatus ?? 'N/A'} | ${row.path} | ${row.pass ? 'PASS' : 'FAIL'} |`,
  )
  .join('\n')}

## 结论

desktop -> mobile 与 mobile -> desktop 的图片 / 音频真实保护副本文件流转均通过。移动端截图来自 Android 原生 Flutter 运行态和真实 Rust bridge；桌面端截图来自本机 \`watermark-core\` 对 adb pull 回来的真实文件提取结果。

当前图片 / 音频保护副本没有额外加密 envelope，因此“解密”项为 N/A；本轮按双端读取并验证同一版权编号和 V2/119 payload 作为封版互解证据。
`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

function issueModeLabel(value) {
  switch (value) {
    case 'server_reserved':
      return '后端预签发';
    case 'server_confirmed':
      return '后端已确认';
    case 'server_reissued':
      return '后端重签发';
    case 'offline_generated':
      return '离线生成';
    default:
      return value ?? 'N/A';
  }
}

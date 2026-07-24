import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { chromium } from 'playwright';

const runId = Date.now().toString();
const outputDir = resolve('tmp-ui-qa', 'v3-readonly-candidate-runtime', runId);
const desktopDir = join(outputDir, 'desktop');
const mobilePullDir = join(outputDir, 'android-pulled');
mkdirSync(desktopDir, { recursive: true });
mkdirSync(mobilePullDir, { recursive: true });

const adbSerial = process.env.HIDDENSHIELD_QA_ADB_SERIAL ?? 'emulator-5554';
const packageName = 'com.hiddenshield.hidden_shield_mobile';
const activityName = `${packageName}/.MainActivity`;
const deviceDir = `/data/data/${packageName}/files/HiddenShieldV3ReadonlyCandidate/${runId}`;
const deviceImage = `${deviceDir}/v3-readonly-candidate-image.png`;
const deviceAudio = `${deviceDir}/v3-readonly-candidate-audio.wav`;
const deviceResult = `${deviceDir}/android-v3-readonly-candidate-result.json`;
const androidScreenshot = join(outputDir, `android-v3-readonly-candidate-${runId}.png`);
const desktopHtml = join(outputDir, `desktop-v3-readonly-candidate-${runId}.html`);
const desktopScreenshot = join(outputDir, `desktop-v3-readonly-candidate-${runId}.png`);
const qaJsonPath = join(outputDir, `v3-readonly-candidate-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `v3-readonly-candidate-runtime-qa-${runId}.md`);

run('cargo', [
  'run',
  '--manifest-path',
  'src-tauri/Cargo.toml',
  '--features',
  'internal-qa',
  '--example',
  'v3_readonly_candidate_runtime_qa',
  '--',
  '--out-dir',
  desktopDir,
]);

const desktop = JSON.parse(
  readFileSync(join(desktopDir, 'desktop-v3-readonly-candidate-runtime.json'), 'utf8'),
);
assertCandidateRow(desktop.desktop.image, 'desktop image');
assertCandidateRow(desktop.desktop.audio, 'desktop audio');
assert(desktop.defaultV3WriteEnabled === true, 'default V3 write must be enabled');
assert(
  desktop.defaultWatermarkServiceExtractV3Enabled === true,
  'default WatermarkService V3 extract must be enabled',
);

appMkdir(deviceDir);
pushToAppFile(desktop.fixtures.imagePath, deviceImage);
pushToAppFile(desktop.fixtures.audioPath, deviceAudio);

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
    'tool/v3_readonly_candidate_runtime_qa.dart',
    `--dart-define=HIDDENSHIELD_V3_CANDIDATE_RUN_ID=${runId}`,
    `--dart-define=HIDDENSHIELD_V3_CANDIDATE_IMAGE_PATH=${deviceImage}`,
    `--dart-define=HIDDENSHIELD_V3_CANDIDATE_AUDIO_PATH=${deviceAudio}`,
    `--dart-define=HIDDENSHIELD_V3_CANDIDATE_IMAGE_UID=${desktop.desktop.image.watermarkUid}`,
    `--dart-define=HIDDENSHIELD_V3_CANDIDATE_AUDIO_UID=${desktop.desktop.audio.watermarkUid}`,
    `--dart-define=HIDDENSHIELD_V3_CANDIDATE_OUTPUT_DIR=${deviceDir}`,
  ],
  { cwd: resolve('mobile_app'), env: flutterEnv },
);
adb(['install', '-r', resolve('mobile_app', 'build', 'app', 'outputs', 'flutter-apk', 'app-debug.apk')]);
adb(['shell', 'am', 'force-stop', packageName]);
adb(['shell', 'am', 'start', '-n', activityName]);
waitForDeviceFile(deviceResult, 120_000);
writeFileSync(androidScreenshot, adbBuffer(['exec-out', 'screencap', '-p']));

pullFromAppFile(deviceResult, join(mobilePullDir, 'android-v3-readonly-candidate-result.json'));
const android = JSON.parse(
  readFileSync(join(mobilePullDir, 'android-v3-readonly-candidate-result.json'), 'utf8'),
);
for (const row of android.rows) {
  assertAndroidRow(row, `android ${row.kind}`);
}

const rows = [
  normalizeDesktopRow(desktop.desktop.image),
  normalizeDesktopRow(desktop.desktop.audio),
  ...android.rows.map(normalizeAndroidRow),
];
const result = {
  runId,
  startedAt: new Date(Number(runId)).toISOString(),
  completedAt: new Date().toISOString(),
  adbSerial,
  outputDir,
  deviceDir,
  desktop,
  android,
  screenshots: {
    android: androidScreenshot,
    desktop: desktopScreenshot,
    desktopHtml,
  },
  boundary:
    'V3 readonly candidate runtime QA uses real PNG/WAV files with formal image sync packet and audio recovery packet carrier lanes. It keeps explicit readonly candidate readers while default V3 writes and default WatermarkService/read() V3 extraction are enabled.',
  pass: rows.every((row) => row.pass),
};

writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
writeFileSync(desktopHtml, renderDesktopHtml(result, rows), 'utf8');
await screenshotHtml(desktopHtml, desktopScreenshot, { width: 1440, height: 920 });
writeFileSync(qaMdPath, renderMarkdown(result, rows), 'utf8');

console.log(`V3 readonly candidate runtime QA JSON: ${qaJsonPath}`);
console.log(`V3 readonly candidate runtime QA Markdown: ${qaMdPath}`);
console.log(`Android screenshot: ${androidScreenshot}`);
console.log(`Desktop screenshot: ${desktopScreenshot}`);
if (!result.pass) {
  throw new Error('V3 readonly candidate runtime QA failed');
}

function run(command, args, options = {}) {
  const useShell = process.platform === 'win32' && command === 'flutter';
  const result = spawnSync(command, args, {
    cwd: options.cwd ?? resolve('.'),
    env: options.env ?? process.env,
    encoding: 'utf8',
    maxBuffer: 128 * 1024 * 1024,
    shell: useShell,
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

function assertCandidateRow(row, label) {
  assert(row.watermarkUid?.startsWith('HS-'), `${label} watermarkUid must be present`);
  assert(row.payloadProtocolVersion === 3, `${label} payloadProtocolVersion must be 3`);
  assert(row.payloadBytesLength === 39, `${label} payloadBytesLength must be 39`);
  assert(row.payloadAuthStatus === 'verified', `${label} payloadAuthStatus must be verified`);
  assert(row.watermarkIdIssueMode === 'registry_resolved', `${label} issue mode must be registry_resolved`);
  assert(row.mediaPayloadRole === 'v3_minimal_anchor', `${label} media payload role must be v3_minimal_anchor`);
  assert(
    row.defaultExtractStatus === 'default_v3_contract_guarded',
    `${label} default extract must stay V3-only contract guarded`,
  );
}

function assertAndroidRow(row, label) {
  assert(row.watermarkUid === row.expectedUid, `${label} expected uid mismatch`);
  assert(row.payloadProtocolVersion === 3, `${label} payloadProtocolVersion must be 3`);
  assert(row.payloadBytesLength === 39, `${label} payloadBytesLength must be 39`);
  assert(row.payloadAuthStatus === 'verified', `${label} payloadAuthStatus must be verified`);
  assert(row.watermarkIdIssueMode === 'registry_resolved', `${label} issue mode must be registry_resolved`);
  assert(row.mediaPayloadRole === 'v3_minimal_anchor', `${label} media payload role must be v3_minimal_anchor`);
  assert(
    row.defaultReadStatus === 'default_v3_contract_guarded',
    `${label} default read must stay V3-only contract guarded`,
  );
  assert(row.pass === true, `${label} pass must be true`);
}

function normalizeDesktopRow(row) {
  return {
    bridge: 'desktop',
    kind: row.mediaKind,
    path: row.path,
    expectedUid: row.watermarkUid,
    watermarkUid: row.watermarkUid,
    payloadProtocolVersion: row.payloadProtocolVersion,
    payloadBytesLength: row.payloadBytesLength,
    payloadAuthStatus: row.payloadAuthStatus,
    watermarkIdIssueMode: row.watermarkIdIssueMode,
    mediaPayloadRole: row.mediaPayloadRole,
    defaultStatus: row.defaultExtractStatus,
    pass: true,
  };
}

function normalizeAndroidRow(row) {
  return {
    bridge: 'android_native',
    kind: row.kind,
    path: row.path,
    expectedUid: row.expectedUid,
    watermarkUid: row.watermarkUid,
    payloadProtocolVersion: row.payloadProtocolVersion,
    payloadBytesLength: row.payloadBytesLength,
    payloadAuthStatus: row.payloadAuthStatus,
    watermarkIdIssueMode: row.watermarkIdIssueMode,
    mediaPayloadRole: row.mediaPayloadRole,
    defaultStatus: row.defaultReadStatus,
    pass: row.pass === true,
  };
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
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
        <div class="topline"><span>${escapeHtml(row.bridge)}</span><strong>${row.pass ? 'PASS' : 'FAIL'}</strong></div>
        <h2>${escapeHtml(row.kind === 'image' ? '图片真实媒体' : '音频真实媒体')}</h2>
        <dl>
          <dt>期望编号</dt><dd>${escapeHtml(row.expectedUid)}</dd>
          <dt>读取编号</dt><dd>${escapeHtml(row.watermarkUid)}</dd>
          <dt>Payload</dt><dd>V${row.payloadProtocolVersion} / ${row.payloadBytesLength} bytes</dd>
          <dt>认证状态</dt><dd>${escapeHtml(row.payloadAuthStatus)}</dd>
          <dt>签发模式</dt><dd>${escapeHtml(row.watermarkIdIssueMode)}</dd>
          <dt>载荷角色</dt><dd>${escapeHtml(row.mediaPayloadRole)}</dd>
          <dt>默认读取</dt><dd>${escapeHtml(row.defaultStatus)}</dd>
          <dt>文件</dt><dd>${escapeHtml(row.path)}</dd>
        </dl>
      </article>`,
    )
    .join('\n');
  return `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>HiddenShield V3 Readonly Candidate Runtime QA</title>
  <style>
    :root { color-scheme: dark; font-family: "Microsoft YaHei", "Segoe UI", sans-serif; background: #080b0f; color: #edf3ff; }
    body { margin: 0; padding: 32px; background: #080b0f; }
    header { display: flex; justify-content: space-between; gap: 24px; align-items: flex-start; margin-bottom: 26px; }
    h1 { margin: 0 0 10px; font-size: 30px; letter-spacing: 0; }
    p { margin: 0; color: #a9b6c9; line-height: 1.55; max-width: 780px; }
    .meta { text-align: right; font-size: 13px; color: #96a4b7; }
    .grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
    .card { border: 1px solid #263340; border-radius: 8px; background: rgba(15, 22, 30, .96); padding: 18px; box-shadow: 0 20px 60px rgba(0,0,0,.25); }
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
      <h1>HiddenShield V3 只读候选真实媒体运行态 QA</h1>
      <p>真实 PNG / WAV 文件携带正式图片 sync packet 与音频 recovery packet 中的 V3/39 minimal anchor；桌面和 Android 保留显式 readonly candidate reader 作为迁移桥，默认读取已只接受 V3/39。</p>
    </div>
    <div class="meta">
      <div>Run ID: ${escapeHtml(result.runId)}</div>
      <div>ADB: ${escapeHtml(result.adbSerial)}</div>
      <div>Output: ${escapeHtml(result.outputDir)}</div>
    </div>
  </header>
  <main class="grid">${cards}</main>
  <footer>${escapeHtml(result.boundary)}</footer>
</body>
</html>`;
}

function renderMarkdown(result, rows) {
  return `# HiddenShield V3 readonly candidate 真实媒体运行态 QA

- Run ID: \`${result.runId}\`
- 时间: ${result.startedAt} -> ${result.completedAt}
- Android: \`${result.adbSerial}\`
- 设备目录: \`${result.deviceDir}\`
- 证据目录: \`${result.outputDir}\`
- Android 截图: \`${result.screenshots.android}\`
- 桌面证据截图: \`${result.screenshots.desktop}\`
- 桌面证据页: \`${result.screenshots.desktopHtml}\`

| Bridge | 媒体 | 期望编号 | 读取编号 | Payload | Auth | Issue mode | Role | 默认读取 | 结果 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
${rows
  .map(
    (row) =>
      `| ${row.bridge} | ${row.kind} | ${row.expectedUid} | ${row.watermarkUid} | V${row.payloadProtocolVersion}/${row.payloadBytesLength} | ${row.payloadAuthStatus} | ${row.watermarkIdIssueMode} | ${row.mediaPayloadRole} | ${row.defaultStatus} | ${row.pass ? 'PASS' : 'FAIL'} |`,
  )
  .join('\n')}

## 边界

${result.boundary}
`;
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

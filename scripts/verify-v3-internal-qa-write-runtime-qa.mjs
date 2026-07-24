import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { chromium } from 'playwright';

const runId = Date.now().toString();
const outputDir = resolve('tmp-ui-qa', 'v3-internal-qa-write-runtime', runId);
const desktopDir = join(outputDir, 'desktop');
const mobilePullDir = join(outputDir, 'android-pulled');
mkdirSync(desktopDir, { recursive: true });
mkdirSync(mobilePullDir, { recursive: true });

const adbSerial = process.env.HIDDENSHIELD_QA_ADB_SERIAL ?? 'emulator-5554';
const packageName = 'com.hiddenshield.hidden_shield_mobile';
const activityName = `${packageName}/.MainActivity`;
const deviceDir = `/data/data/${packageName}/files/HiddenShieldV3InternalQaWrite/${runId}`;
const deviceImageSource = `${deviceDir}/desktop-source-image.png`;
const deviceAudioSource = `${deviceDir}/desktop-source-audio.wav`;
const deviceResult = `${deviceDir}/android-v3-internal-qa-write-result.json`;
const androidScreenshot = join(outputDir, `android-v3-internal-qa-write-${runId}.png`);
const desktopHtml = join(outputDir, `desktop-v3-internal-qa-write-${runId}.html`);
const desktopScreenshot = join(outputDir, `desktop-v3-internal-qa-write-${runId}.png`);
const qaJsonPath = join(outputDir, `v3-internal-qa-write-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `v3-internal-qa-write-runtime-qa-${runId}.md`);

const sources = {
  packageJson: readFileSync('package.json', 'utf8'),
  desktopCargo: readFileSync('src-tauri/Cargo.toml', 'utf8'),
  desktopQaBin: readFileSync('src-tauri/examples/v3_internal_qa_write_runtime_qa.rs', 'utf8'),
  desktopScheduler: readFileSync('src-tauri/src/pipeline/scheduler.rs', 'utf8'),
  mobileRustApi: readFileSync('mobile_app/rust/src/api.rs', 'utf8'),
  mobileBridge: readFileSync('mobile_app/lib/bridge/rust_watermark_bridge.dart', 'utf8'),
  mobileQaTool: readFileSync('mobile_app/tool/v3_internal_qa_write_runtime_qa.dart', 'utf8'),
  mobileGeneratedApi: readFileSync('mobile_app/lib/src/rust/api.dart', 'utf8'),
  migrationContract: readFileSync('docs/V3跨端fixture与迁移桥接报告字段冻结合同.md', 'utf8'),
};

assert(
  sources.packageJson.includes('"rights:v3-internal-qa-write-runtime-qa"') &&
    sources.packageJson.includes('verify-v3-internal-qa-write-runtime-qa.mjs'),
  'package.json must expose rights:v3-internal-qa-write-runtime-qa',
);
assert(
  sources.desktopCargo.includes('v3_internal_qa_write_runtime_qa') &&
    sources.desktopQaBin.includes('embed_v3_internal_qa_media') &&
    sources.desktopQaBin.includes('default_write') &&
    sources.desktopQaBin.includes('v3_minimal_anchor_verified'),
  'desktop QA bin must cover internal_qa V3 writing and default V3 writing',
);
assert(
  sources.mobileRustApi.includes('embed_v3_internal_qa_for_mobile') &&
    sources.mobileGeneratedApi.includes('embedV3InternalQaForMobile') &&
    sources.mobileQaTool.includes('embedV3InternalQaForMobile') &&
    sources.mobileQaTool.includes('bridge.write') &&
    sources.mobileQaTool.includes('defaultMobileWriteV3Enabled'),
  'Android native QA must expose controlled internal_qa writing and default write verification',
);
assert(
  !sources.desktopScheduler.includes('embed_v3_internal_qa_media') &&
    !sources.mobileBridge.includes('embedV3InternalQaForMobile'),
  'formal desktop scheduler and mobile default bridge.write must not call V3 internal QA writing',
);
assert(
  sources.migrationContract.includes('`off`') &&
    sources.migrationContract.includes('`internal_qa`') &&
    sources.migrationContract.includes('默认正式路径') &&
    sources.migrationContract.includes('V3/39') &&
    sources.migrationContract.includes('`force_v2_rollback`'),
  'migration contract must retain feature gate rollback boundary',
);

run('cargo', [
  'run',
  '--manifest-path',
  'src-tauri/Cargo.toml',
  '--features',
  'internal-qa',
  '--example',
  'v3_internal_qa_write_runtime_qa',
  '--',
  '--run-id',
  runId,
  '--out-dir',
  desktopDir,
]);

const desktop = JSON.parse(
  readFileSync(join(desktopDir, 'desktop-v3-internal-qa-write-runtime.json'), 'utf8'),
);
for (const row of desktop.desktop.rows) {
  assertRuntimeRow(row, `desktop ${row.writePath} ${row.mediaKind}`);
}
assert(desktop.defaultV3WriteEnabled === true, 'desktop default V3 write must be enabled');

appMkdir(deviceDir);
pushToAppFile(desktop.desktop.source.imagePath, deviceImageSource);
pushToAppFile(desktop.desktop.source.audioPath, deviceAudioSource);

const imageV3 = desktop.desktop.rows.find((row) => row.writePath === 'internal_qa' && row.mediaKind === 'image');
const audioV3 = desktop.desktop.rows.find((row) => row.writePath === 'internal_qa' && row.mediaKind === 'audio');

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
    'tool/v3_internal_qa_write_runtime_qa.dart',
    `--dart-define=HIDDENSHIELD_V3_INTERNAL_QA_RUN_ID=${runId}`,
    `--dart-define=HIDDENSHIELD_V3_INTERNAL_QA_IMAGE_SOURCE_PATH=${deviceImageSource}`,
    `--dart-define=HIDDENSHIELD_V3_INTERNAL_QA_AUDIO_SOURCE_PATH=${deviceAudioSource}`,
    `--dart-define=HIDDENSHIELD_V3_INTERNAL_QA_IMAGE_UID=${imageV3.watermarkUid}`,
    `--dart-define=HIDDENSHIELD_V3_INTERNAL_QA_AUDIO_UID=${audioV3.watermarkUid}`,
    `--dart-define=HIDDENSHIELD_V3_INTERNAL_QA_OUTPUT_DIR=${deviceDir}`,
  ],
  { cwd: resolve('mobile_app'), env: flutterEnv },
);
adb(['install', '-r', resolve('mobile_app', 'build', 'app', 'outputs', 'flutter-apk', 'app-debug.apk')]);
adb(['shell', 'am', 'force-stop', packageName]);
adb(['shell', 'am', 'start', '-n', activityName]);
waitForDeviceFile(deviceResult, 120_000);
writeFileSync(androidScreenshot, adbBuffer(['exec-out', 'screencap', '-p']));

pullFromAppFile(deviceResult, join(mobilePullDir, 'android-v3-internal-qa-write-result.json'));
const android = JSON.parse(
  readFileSync(join(mobilePullDir, 'android-v3-internal-qa-write-result.json'), 'utf8'),
);
for (const row of android.rows) {
  assertRuntimeRow(row, `android ${row.writePath} ${row.kind}`);
}
assert(android.defaultV3WriteEnabled === true, 'android default V3 write must be enabled');
assert(android.defaultMobileWriteV3Enabled === true, 'android mobile default write V3 must be enabled');

const rows = [
  ...desktop.desktop.rows.map(normalizeDesktopRow),
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
    'Desktop and Android native QA explicitly call internal_qa V3 writing to produce QA-only V3/39 image/audio artifacts, then verify formal default write paths also produce V3/39. V2 is covered only by force_v2_rollback.',
  pass: rows.every((row) => row.pass),
};

writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
writeFileSync(desktopHtml, renderDesktopHtml(result, rows), 'utf8');
await screenshotHtml(desktopHtml, desktopScreenshot, { width: 1440, height: 960 });
writeFileSync(qaMdPath, renderMarkdown(result, rows), 'utf8');

console.log(`V3 internal QA write runtime QA JSON: ${qaJsonPath}`);
console.log(`V3 internal QA write runtime QA Markdown: ${qaMdPath}`);
console.log(`Android screenshot: ${androidScreenshot}`);
console.log(`Desktop screenshot: ${desktopScreenshot}`);
if (!result.pass) throw new Error('V3 internal QA write runtime QA failed');

function assertRuntimeRow(row, label) {
  const kind = row.mediaKind ?? row.kind;
  assert(['image', 'audio'].includes(kind), `${label} media kind must be image/audio`);
  assert(row.watermarkUid?.startsWith('HS-'), `${label} watermarkUid must be present`);
  assert(row.payloadAuthStatus === 'verified', `${label} payloadAuthStatus must be verified`);
  assert(row.pass === true, `${label} pass must be true`);
  if (row.writePath === 'internal_qa') {
    assert(row.payloadProtocolVersion === 3, `${label} internal_qa must write V3`);
    assert(row.payloadBytesLength === 39, `${label} internal_qa must be V3/39`);
    assert(row.watermarkIdIssueMode === 'registry_resolved', `${label} issue mode must be registry_resolved`);
    assert(row.mediaPayloadRole === 'v3_minimal_anchor', `${label} role must be v3_minimal_anchor`);
    assert(
      row.defaultWritePathStatus === 'not_used_internal_qa_only',
      `${label} default write path status must stay internal-only`,
    );
  } else {
    assert(row.writePath === 'default_write', `${label} non-internal row must be default_write`);
    assert(row.payloadProtocolVersion === 3, `${label} default write must be V3`);
    assert(row.payloadBytesLength === 39, `${label} default write must be V3/39`);
    assert(row.mediaPayloadRole === 'v3_minimal_anchor', `${label} role must be v3_minimal_anchor`);
    assert(
      row.defaultWritePathStatus === 'v3_minimal_anchor_verified',
      `${label} default write path status must be V3 verified`,
    );
  }
}

function normalizeDesktopRow(row) {
  return {
    bridge: 'desktop',
    writePath: row.writePath,
    kind: row.mediaKind,
    path: row.path,
    watermarkUid: row.watermarkUid,
    payloadProtocolVersion: row.payloadProtocolVersion,
    payloadBytesLength: row.payloadBytesLength,
    payloadAuthStatus: row.payloadAuthStatus,
    watermarkIdIssueMode: row.watermarkIdIssueMode,
    mediaPayloadRole: row.mediaPayloadRole,
    defaultWritePathStatus: row.defaultWritePathStatus,
    pass: row.pass === true,
  };
}

function normalizeAndroidRow(row) {
  return {
    bridge: 'android_native',
    writePath: row.writePath,
    kind: row.kind,
    path: row.path,
    watermarkUid: row.watermarkUid,
    payloadProtocolVersion: row.payloadProtocolVersion,
    payloadBytesLength: row.payloadBytesLength,
    payloadAuthStatus: row.payloadAuthStatus,
    watermarkIdIssueMode: row.watermarkIdIssueMode,
    mediaPayloadRole: row.mediaPayloadRole,
    defaultWritePathStatus: row.defaultWritePathStatus,
    pass: row.pass === true,
  };
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
        <div class="topline"><span>${escapeHtml(row.bridge)} · ${escapeHtml(row.writePath)}</span><strong>${row.pass ? 'PASS' : 'FAIL'}</strong></div>
        <h2>${escapeHtml(row.kind === 'image' ? '图片真实媒体' : '音频真实媒体')}</h2>
        <dl>
          <dt>版权编号</dt><dd>${escapeHtml(row.watermarkUid)}</dd>
          <dt>Payload</dt><dd>V${row.payloadProtocolVersion} / ${row.payloadBytesLength} bytes</dd>
          <dt>认证状态</dt><dd>${escapeHtml(row.payloadAuthStatus)}</dd>
          <dt>签发模式</dt><dd>${escapeHtml(row.watermarkIdIssueMode)}</dd>
          <dt>载荷角色</dt><dd>${escapeHtml(row.mediaPayloadRole)}</dd>
          <dt>默认路径</dt><dd>${escapeHtml(row.defaultWritePathStatus)}</dd>
          <dt>文件</dt><dd>${escapeHtml(row.path)}</dd>
        </dl>
      </article>`,
    )
    .join('\n');
  return `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>HiddenShield V3 Internal QA Write Runtime QA</title>
  <style>
    :root { color-scheme: dark; font-family: "Microsoft YaHei", "Segoe UI", sans-serif; background: #080b0f; color: #edf3ff; }
    body { margin: 0; padding: 32px; background: #080b0f; }
    header { display: flex; justify-content: space-between; gap: 24px; align-items: flex-start; margin-bottom: 26px; }
    h1 { margin: 0 0 10px; font-size: 30px; letter-spacing: 0; }
    p { margin: 0; color: #a9b6c9; line-height: 1.55; max-width: 820px; }
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
      <h1>HiddenShield V3 internal_qa 写入运行态 QA</h1>
      <p>桌面端和 Android 原生端显式调用内部 QA V3 写入 gate 生成 V3/39 图片 / 音频样本；同一运行态再验证默认写入路径也输出 V3/39。</p>
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
  return `# HiddenShield V3 internal_qa 写入运行态 QA

- Run ID: \`${result.runId}\`
- 时间: ${result.startedAt} -> ${result.completedAt}
- Android: \`${result.adbSerial}\`
- 设备目录: \`${result.deviceDir}\`
- 证据目录: \`${result.outputDir}\`
- Android 截图: \`${result.screenshots.android}\`
- 桌面证据截图: \`${result.screenshots.desktop}\`
- 桌面证据页: \`${result.screenshots.desktopHtml}\`

| Bridge | 写入路径 | 媒体 | 版权编号 | Payload | Auth | Issue mode | Role | 默认路径状态 | 结果 |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
${rows
  .map(
    (row) =>
      `| ${row.bridge} | ${row.writePath} | ${row.kind} | ${row.watermarkUid} | V${row.payloadProtocolVersion}/${row.payloadBytesLength} | ${row.payloadAuthStatus} | ${row.watermarkIdIssueMode} | ${row.mediaPayloadRole} | ${row.defaultWritePathStatus} | ${row.pass ? 'PASS' : 'FAIL'} |`,
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

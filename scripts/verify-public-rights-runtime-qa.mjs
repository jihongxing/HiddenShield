import { createHash } from 'node:crypto';
import { execFileSync, spawn } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import net from 'node:net';
import { chromium } from 'playwright';

const runId = process.env.HIDDENSHIELD_PUBLIC_RIGHTS_QA_RUN_ID ?? `${Date.now()}`;
const endpoint = process.env.HIDDENSHIELD_CLOUD_URL?.replace(/\/$/, '') ?? null;
const shouldStartBackend = !endpoint;
const port = shouldStartBackend ? await freePort() : Number(new URL(endpoint).port || 80);
const baseUrl = endpoint ?? `http://127.0.0.1:${port}`;
const mobileBaseUrl =
  process.env.HIDDENSHIELD_QA_MOBILE_BACKEND_URL ??
  (shouldStartBackend ? `http://127.0.0.1:${port}` : baseUrl);
const tmpRoot = join(tmpdir(), `hiddenshield-public-rights-runtime-qa-${runId}`);
const dbPath = join(tmpRoot, 'cloud.sqlite');
const outputDir = resolve('tmp-ui-qa', 'public-rights-runtime', runId);
const qaJsonPath = join(outputDir, `public-rights-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `public-rights-runtime-qa-${runId}.md`);
const desktopHtmlPath = join(outputDir, `desktop-public-rights-${runId}.html`);
const desktopScreenshotPath = join(outputDir, `desktop-public-rights-${runId}.png`);
const mobileScreenshotPath = join(outputDir, `mobile-public-rights-${runId}.png`);
const adbSerial = process.env.HIDDENSHIELD_QA_ADB_SERIAL ?? 'emulator-5554';
const avdName = process.env.HIDDENSHIELD_QA_AVD ?? 'HiddenShield_QA_API36';

mkdirSync(tmpRoot, { recursive: true });
mkdirSync(outputDir, { recursive: true });

let backend;
let startedEmulator = false;
const childProcesses = [];
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
    childProcesses.push(backend);
    backend.stdout.on('data', (chunk) => process.stdout.write(`[backend] ${chunk}`));
    backend.stderr.on('data', (chunk) => process.stderr.write(`[backend] ${chunk}`));
  }

  await waitForHealth(baseUrl);
  const desktop = await runDesktopRightsQa(baseUrl);
  writeFileSync(desktopHtmlPath, renderDesktopHtml({ runId, baseUrl, rows: desktop.rows }), 'utf8');
  await screenshotHtml(desktopHtmlPath, desktopScreenshotPath, { width: 1440, height: 1040 });

  const mobile = await runMobileRightsQa();
  const result = {
    runId,
    baseUrl,
    mobileBaseUrl,
    startedBackend: shouldStartBackend,
    dbPath: shouldStartBackend ? dbPath : null,
    desktop: {
      rows: desktop.rows,
      screenshot: desktopScreenshotPath,
      evidenceHtml: desktopHtmlPath,
    },
    mobile,
    screenshots: {
      desktop: desktopScreenshotPath,
      mobile: mobileScreenshotPath,
    },
    completedAt: new Date().toISOString(),
  };
  assert(
    desktop.rows.every((row) => row.pass) && mobile.status === 'passed',
    'public rights runtime QA must pass both desktop and mobile',
  );
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
  console.log('Public rights runtime QA OK');
  console.log(`QA JSON: ${qaJsonPath}`);
  console.log(`QA Markdown: ${qaMdPath}`);
  console.log(`Desktop screenshot: ${desktopScreenshotPath}`);
  console.log(`Mobile screenshot: ${mobileScreenshotPath}`);
} finally {
  for (const child of childProcesses.reverse()) {
    await stopChild(child);
  }
}

async function runDesktopRightsQa(endpoint) {
  const session = await ensureCreatorSession(endpoint, {
    identifier: `desktop-public-rights-${runId}@hiddenshield.local`,
    password: `desktop-public-rights-${runId}`,
    deviceId: `desktop-public-rights-${runId}`,
    deviceName: 'Desktop Public Rights Runtime QA',
    platform: 'windows',
    creatorDisplayName: '桌面端公开权利 QA 创作者',
  });
  const cases = [
    {
      mediaKind: 'image',
      mediaType: 'image',
      title: `desktop-image-public-rights-${runId}.png`,
      trainingPermissionDeclaration: 'commercial_allowed',
      workSourceDeclaration: 'ai_assisted',
      creationMethodDeclaration: 'text_to_image',
    },
    {
      mediaKind: 'audio',
      mediaType: 'audio',
      title: `desktop-audio-public-rights-${runId}.wav`,
      trainingPermissionDeclaration: 'prohibited',
      workSourceDeclaration: 'human_created',
      creationMethodDeclaration: 'audio_recording',
    },
  ];
  const rows = [];
  for (const item of cases) {
    rows.push(await writeSyncAndQueryRights(endpoint, session, item));
  }
  return { rows };
}

async function writeSyncAndQueryRights(endpoint, session, item) {
  const originalHash = `sha256:${sha256(`${item.mediaKind}:original:${runId}`)}`;
  const protectedCopyHash = `sha256:${sha256(`${item.mediaKind}:protected:${runId}`)}`;
  const reserve = await request(
    endpoint,
    'POST',
    '/v1/watermark-ids/reserve',
    {
      requestId: `desktop-${item.mediaKind}-public-rights-${runId}`,
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      mediaType: item.mediaType,
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      parentWatermarkUid: null,
      revision: 1,
      originalHash,
    },
    session.accessToken,
  );
  assert(reserve.status === 200, `${item.mediaKind} reserve must succeed`);
  const confirm = await request(
    endpoint,
    'POST',
    '/v1/watermark-ids/confirm',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      watermarkUid: reserve.body.watermarkUid,
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      originalHash,
      protectedCopyHash,
      writeVerificationStatus: 'verified',
    },
    session.accessToken,
  );
  assert(confirm.status === 200, `${item.mediaKind} confirm must succeed`);

  const payload = {
    id: `desktop-${item.mediaKind}-record-${runId}`,
    kind: item.mediaKind,
    title: item.title,
    watermark_uid: confirm.body.watermarkUid,
    revision: confirm.body.revision,
    creator_display_name: session.creatorProfile.displayName,
    sha256: originalHash,
    protected_copy_name: item.title.replace(/\.(png|wav)$/i, '.protected.$1'),
    protected_copy_hash: protectedCopyHash,
    payload_protocol_version: confirm.body.payloadProtocolVersion,
    payload_bytes_length: confirm.body.payloadBytesLength,
    watermark_id_issue_mode: confirm.body.watermarkIdIssueMode,
    watermark_id_registry_status: confirm.body.registryStatus,
    watermark_id_registry_receipt: confirm.body.registryReceipt,
    payload_auth_status: 'verified',
    output_strategy: 'minimal_required_change',
    work_source_declaration: item.workSourceDeclaration,
    training_permission_declaration: item.trainingPermissionDeclaration,
    creation_method_declaration: item.creationMethodDeclaration,
    human_edit_level_declaration: 'light',
    authenticity_claim_declaration: 'synthetic',
    custom_rights_statement: 'desktop public rights runtime QA',
    source: 'write',
    sync_status: 'synced',
    created_at: new Date().toISOString(),
  };
  const pushed = await request(
    endpoint,
    'POST',
    '/v1/sync/events:batch',
    {
      deviceId: session.device.id,
      workspaceId: session.workspace.id,
      events: [
        {
          clientEventId: `desktop-${item.mediaKind}-rights-event-${runId}`,
          operation: 'upsertVaultRecord',
          entityType: 'vaultRecord',
          entityId: payload.id,
          payload,
        },
      ],
    },
    session.accessToken,
  );
  assert(pushed.status === 200, `${item.mediaKind} sync push must succeed`);

  const rights = await request(
    endpoint,
    'GET',
    `/v1/public/rights/${encodeURIComponent(payload.watermark_uid)}`,
  );
  assert(rights.status === 200, `${item.mediaKind} public rights query must succeed`);
  const expectedPolicy = expectedPublicTrainingPolicy(item.trainingPermissionDeclaration);
  const pass =
    rights.body.scanStatus === 'registry_active' &&
    rights.body.trainingPermission.policy === expectedPolicy &&
    rights.body.registry.anchorProtocol === 'v2_migration_anchor' &&
    rights.body.trainingPermission.legalConclusion === false &&
    Boolean(rights.body.rightsManifest?.manifestVersion);
  return {
    mediaKind: item.mediaKind,
    title: item.title,
    watermarkUid: payload.watermark_uid,
    localTraining: item.trainingPermissionDeclaration,
    publicTrainingPolicy: rights.body.trainingPermission.policy,
    publicTrainingLabel: rights.body.trainingPermission.label,
    scanStatus: rights.body.scanStatus,
    anchorProtocol: rights.body.registry.anchorProtocol,
    manifestVersion: rights.body.rightsManifest?.manifestVersion ?? 0,
    legalConclusion: rights.body.trainingPermission.legalConclusion,
    pass,
  };
}

async function runMobileRightsQa() {
  await ensureEmulator();
  if (!process.env.HIDDENSHIELD_QA_MOBILE_BACKEND_URL && shouldStartBackend) {
    ensureAdbReverse(port);
  }
  try {
    adbText(['-s', adbSerial, 'shell', 'am', 'force-stop', 'com.hiddenshield.hidden_shield_mobile']);
  } catch {
    // The app may not be installed yet.
  }
  const args = [
    'run',
    '-d',
    adbSerial,
    '-t',
    'tool/public_rights_runtime_qa.dart',
    '--dart-define',
    `HIDDENSHIELD_QA_BACKEND_URL=${mobileBaseUrl}`,
    '--dart-define',
    `HIDDENSHIELD_QA_RUN_ID=${runId}`,
  ];
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
  const mobileResult = await waitForMobileQaResult(child, 420_000);
  await sleep(2500);
  const png = execFileSync('adb', ['-s', adbSerial, 'exec-out', 'screencap', '-p']);
  writeFileSync(mobileScreenshotPath, png);
  await stopChild(child);
  cleanupFlutterQaProcesses();
  return {
    status: 'passed',
    command: `flutter ${args.join(' ')}`,
    screenshot: mobileScreenshotPath,
    rows: mobileResult.rows ?? [],
  };
}

function ensureAdbReverse(hostPort) {
  adbText(['-s', adbSerial, 'reverse', `tcp:${hostPort}`, `tcp:${hostPort}`]);
}

function waitForMobileQaResult(child, timeoutMs) {
  return new Promise((resolve, reject) => {
    let settled = false;
    let output = '';
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      reject(new Error(`flutter mobile public rights QA timed out after ${timeoutMs}ms\n${tail(output)}`));
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
        .find((line) => line.includes('HIDDENSHIELD_PUBLIC_RIGHTS_QA_ERROR'));
      if (errorLine) {
        finish(reject, new Error(`mobile QA reported error: ${errorLine}\n${tail(output)}`));
        return;
      }
      const resultLine = text
        .split(/\r?\n/)
        .find((line) => line.includes('HIDDENSHIELD_PUBLIC_RIGHTS_QA_RESULT'));
      if (!resultLine) return;
      const jsonStart = resultLine.indexOf('{');
      assert(jsonStart >= 0, `mobile QA result line must contain JSON: ${resultLine}`);
      const result = JSON.parse(resultLine.slice(jsonStart));
      assert(result.passed === true, `mobile QA result must pass: ${resultLine}`);
      finish(resolve, result);
    };
    child.stdout.on('data', (chunk) => onChunk(chunk, process.stdout));
    child.stderr.on('data', (chunk) => onChunk(chunk, process.stderr));
    child.on('exit', (code, signal) => {
      if (settled) return;
      finish(reject, new Error(`flutter mobile public rights QA exited before result: code=${code} signal=${signal}\n${tail(output)}`));
    });
    child.on('error', (error) => finish(reject, error));
  });
}

async function ensureEmulator() {
  const devices = adbText(['devices']);
  if (devices.includes(`${adbSerial}\tdevice`)) return;
  const emulatorPath = process.env.HIDDENSHIELD_ANDROID_EMULATOR_EXE ??
    join(process.env.ANDROID_HOME ?? '', 'emulator', 'emulator.exe');
  spawn(emulatorPath, ['-avd', avdName, '-no-snapshot-save'], {
    cwd: process.cwd(),
    detached: true,
    stdio: 'ignore',
    windowsHide: true,
  }).unref();
  startedEmulator = true;
  const deadline = Date.now() + 180_000;
  while (Date.now() < deadline) {
    await sleep(3000);
    const current = adbText(['devices']);
    if (current.includes(`${adbSerial}\tdevice`)) {
      try {
        adbText(['-s', adbSerial, 'shell', 'settings', 'put', 'global', 'window_animation_scale', '0']);
        adbText(['-s', adbSerial, 'shell', 'settings', 'put', 'global', 'transition_animation_scale', '0']);
        adbText(['-s', adbSerial, 'shell', 'settings', 'put', 'global', 'animator_duration_scale', '0']);
      } catch {
        // Non-fatal for screenshots.
      }
      return;
    }
  }
  throw new Error(`Android emulator ${avdName} did not become ready`);
}

async function ensureCreatorSession(endpoint, input) {
  const response = await request(endpoint, 'POST', '/v1/auth/sessions', {
    identifier: input.identifier,
    password: input.password,
    verificationCode: '000000',
    device: {
      clientDeviceId: input.deviceId,
      name: input.deviceName,
      platform: input.platform,
      appVersion: 'public-rights-runtime-qa',
    },
    localCreatorProfile: {
      displayName: input.creatorDisplayName,
      creatorSeedRef: `seed-ref-${input.identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, `${input.platform} auth/sessions must succeed`);
  let session = response.body;
  if (session.entitlement?.features?.cloud_sync !== true) {
    const payment = await request(
      endpoint,
      'POST',
      '/v1/billing/payment-sessions',
      {
        accountId: session.account.id,
        workspaceId: session.workspace.id,
        planCode: 'creator',
        billingCycle: 'monthly',
        preferredProvider: 'fixture',
      },
      session.accessToken,
    );
    assert(payment.status === 200, `${input.platform} fixture payment must succeed`);
    const reconcile = await request(
      endpoint,
      'POST',
      `/v1/billing/payment-sessions/${payment.body.paymentSessionId}/reconcile`,
      {},
      session.accessToken,
    );
    assert(reconcile.status === 200, `${input.platform} fixture reconcile must succeed`);
    const refreshed = await request(endpoint, 'GET', '/v1/me', undefined, session.accessToken);
    assert(refreshed.status === 200, `${input.platform} me must succeed`);
    session = { ...session, entitlement: refreshed.body.entitlement };
  }
  return session;
}

async function request(endpoint, method, path, body, token) {
  const headers = {};
  if (body !== undefined) headers['content-type'] = 'application/json';
  if (token) headers.authorization = `Bearer ${token}`;
  const response = await fetch(`${endpoint}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let parsed = {};
  try {
    parsed = text ? JSON.parse(text) : {};
  } catch {
    parsed = { raw: text };
  }
  return { status: response.status, body: parsed };
}

async function waitForHealth(endpoint) {
  const deadline = Date.now() + 120_000;
  let lastError = 'not started';
  while (Date.now() < deadline) {
    try {
      const response = await request(endpoint, 'GET', '/v1/health');
      if (response.status === 200 && response.body.ok === true) return;
      lastError = `health ${response.status}`;
    } catch (error) {
      lastError = error.message;
    }
    await sleep(1000);
  }
  throw new Error(`backend did not become healthy: ${lastError}`);
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

function renderDesktopHtml({ runId, baseUrl, rows }) {
  const cards = rows.map(renderRightsCard).join('\n');
  return `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>HiddenShield Public Rights Runtime QA</title>
  <style>
    :root { color-scheme: dark; font-family: "Microsoft YaHei", "Segoe UI", sans-serif; background: #080b0f; color: #edf3ff; }
    body { margin: 0; padding: 32px; background: #080b0f; }
    header { display: flex; justify-content: space-between; gap: 24px; align-items: flex-start; margin-bottom: 26px; }
    h1 { margin: 0 0 10px; font-size: 30px; letter-spacing: 0; }
    p { margin: 0; color: #a9b6c9; line-height: 1.55; }
    .meta { text-align: right; font-size: 13px; color: #96a4b7; }
    .grid { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 16px; }
    .card { border: 1px solid #263340; border-radius: 8px; background: #0f161e; padding: 18px; }
    .card.ok { border-color: rgba(42, 190, 139, .46); }
    .topline { display: flex; justify-content: space-between; color: #8ea0b6; font-size: 12px; margin-bottom: 14px; }
    .topline strong { color: #73e2ba; }
    h2 { margin: 0 0 16px; font-size: 17px; line-height: 1.35; letter-spacing: 0; }
    dl { display: grid; grid-template-columns: 106px minmax(0, 1fr); gap: 10px 12px; margin: 0; font-size: 13px; }
    dt { color: #7f8fa3; }
    dd { margin: 0; color: #e7edf7; overflow-wrap: anywhere; }
    footer { margin-top: 22px; color: #7f8fa3; font-size: 12px; }
  </style>
</head>
<body>
  <header>
    <div>
      <h1>HiddenShield 桌面端公开权利信号 QA</h1>
      <p>桌面端图片 / 音频记录完成真实后端登记、云同步声明入库，并通过公开 rights registry 查询训练许可快照。</p>
    </div>
    <div class="meta">
      <div>Run ID: ${escapeHtml(runId)}</div>
      <div>Backend: ${escapeHtml(baseUrl)}</div>
      <div>Boundary: 创作者声明与 registry 快照，不直接判断是否可训练</div>
    </div>
  </header>
  <main class="grid">${cards}</main>
  <footer>该 QA 不上传原始媒体或保护副本文件，只同步版权库元数据和作品声明字段。</footer>
</body>
</html>`;
}

function renderRightsCard(row) {
  return `<article class="card ${row.pass ? 'ok' : 'fail'}">
    <div class="topline"><span>${escapeHtml(row.mediaKind === 'image' ? '图片写入' : '音频写入')}</span><strong>${row.pass ? 'PASS' : 'FAIL'}</strong></div>
    <h2>${escapeHtml(row.title)}</h2>
    <dl>
      <dt>版权编号</dt><dd>${escapeHtml(row.watermarkUid)}</dd>
      <dt>本地训练许可</dt><dd>${escapeHtml(trainingLabel(row.localTraining))}</dd>
      <dt>公开训练许可</dt><dd>${escapeHtml(row.publicTrainingLabel)}</dd>
      <dt>扫描状态</dt><dd>${escapeHtml(scanStatusLabel(row.scanStatus))}</dd>
      <dt>锚点协议</dt><dd>${escapeHtml(anchorProtocolLabel(row.anchorProtocol))}</dd>
      <dt>Manifest</dt><dd>v${row.manifestVersion}</dd>
      <dt>法律结论</dt><dd>${row.legalConclusion ? '是' : '否'}</dd>
    </dl>
  </article>`;
}

function renderMarkdown(result) {
  return `# HiddenShield 公开权利信号真实后端运行态 QA

- Run ID: \`${result.runId}\`
- 后端: ${result.baseUrl}
- 移动端后端地址: ${result.mobileBaseUrl}
- 完成时间: ${result.completedAt}

## 桌面端

- 截图: \`${result.desktop.screenshot}\`
- 证据页: \`${result.desktop.evidenceHtml}\`

| 媒体 | 本地训练许可 | 公开训练许可 | 扫描状态 | 锚点协议 | Manifest | 结果 |
| --- | --- | --- | --- | --- | --- | --- |
${result.desktop.rows.map((row) => `| ${row.mediaKind} | ${trainingLabel(row.localTraining)} | ${row.publicTrainingLabel} | ${row.scanStatus} | ${row.anchorProtocol} | v${row.manifestVersion} | ${row.pass ? 'PASS' : 'FAIL'} |`).join('\n')}

## 移动端

- 状态: ${result.mobile.status}
- 截图: \`${result.mobile.screenshot}\`
- 命令: \`${result.mobile.command}\`

## 结论

桌面端与 Android 原生端均已完成图片 / 音频记录的真实后端登记、云同步声明入库和公开 rights registry 查询；公开权利信号与本地训练许可声明一致，且 \`legalConclusion=false\`。
`;
}

function expectedPublicTrainingPolicy(local) {
  return {
    commercial_allowed: 'commercial_training_allowed',
    non_commercial_allowed: 'non_commercial_research_allowed',
    separate_authorization_required: 'separate_license_required',
    prohibited: 'no_ai_training',
  }[local] ?? 'no_ai_training';
}

function trainingLabel(value) {
  return {
    commercial_allowed: '允许商业训练',
    prohibited: '禁止模型训练',
  }[value] ?? value;
}

function scanStatusLabel(value) {
  return {
    registry_active: 'registry 已生效',
    watermark_only: '仅识别到水印锚点',
  }[value] ?? value;
}

function anchorProtocolLabel(value) {
  return {
    v2_migration_anchor: 'V2 迁移桥接锚点',
    v3_minimal_anchor: 'V3 最小媒体锚点',
  }[value] ?? value;
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function adbText(args) {
  return execFileSync('adb', args, { encoding: 'utf8' });
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function stopChild(child) {
  if (!child) return;
  if (process.platform === 'win32' && child.pid) {
    try {
      execFileSync('taskkill', ['/PID', String(child.pid), '/T', '/F'], { stdio: 'ignore' });
    } catch {
      // The process may have exited after the QA result was printed.
    }
  }
  if (child.killed || child.exitCode !== null || child.signalCode !== null) return;
  child.kill();
  await Promise.race([
    new Promise((resolve) => child.once('exit', resolve)),
    sleep(5000),
  ]);
}

function cleanupFlutterQaProcesses() {
  if (process.platform !== 'win32') return;
  const cleanupScript = join(tmpRoot, 'cleanup-public-rights-flutter.ps1');
  writeFileSync(
    cleanupScript,
    [
      '$current = $PID',
      'Get-CimInstance Win32_Process |',
      "  Where-Object { $_.CommandLine -like '*public_rights_runtime_qa.dart*' -and $_.ProcessId -ne $current } |",
      '  ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }',
    ].join('\n'),
    'utf8',
  );
  try {
    execFileSync('powershell', ['-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', cleanupScript], {
      stdio: 'ignore',
    });
  } catch {
    // Best-effort cleanup only. The QA evidence has already been written.
  }
}

function tail(value, max = 4000) {
  return value.length > max ? value.slice(value.length - max) : value;
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

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

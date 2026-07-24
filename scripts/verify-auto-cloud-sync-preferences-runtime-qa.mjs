import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn } from 'node:child_process';
import net from 'node:net';
import { chromium } from 'playwright';

const runId = process.env.HIDDENSHIELD_AUTO_SYNC_QA_RUN_ID ?? `${Date.now()}`;
const endpoint = process.env.HIDDENSHIELD_CLOUD_URL?.replace(/\/$/, '') ?? null;
const shouldStartBackend = !endpoint;
const port = shouldStartBackend ? await freePort() : Number(new URL(endpoint).port || 80);
const baseUrl = endpoint ?? `http://127.0.0.1:${port}`;
const tmpRoot = join(tmpdir(), `hiddenshield-auto-sync-runtime-qa-${runId}`);
const dbPath = join(tmpRoot, 'cloud.sqlite');
const targetDir =
  process.env.HIDDENSHIELD_AUTO_SYNC_QA_TARGET_DIR ??
  resolve('tmp-ui-qa', 'auto-cloud-sync-target');
const outputDir = resolve('tmp-ui-qa', 'auto-cloud-sync');
const qaJsonPath = join(outputDir, `auto-cloud-sync-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `auto-cloud-sync-runtime-qa-${runId}.md`);
const desktopHtmlPath = join(outputDir, `desktop-auto-cloud-sync-${runId}.html`);
const desktopScreenshotPath = join(outputDir, `desktop-auto-cloud-sync-${runId}.png`);
const mobileHtmlPath = join(outputDir, `mobile-auto-cloud-sync-${runId}.html`);
const mobileScreenshotPath = join(outputDir, `mobile-auto-cloud-sync-${runId}.png`);

mkdirSync(tmpRoot, { recursive: true });
mkdirSync(outputDir, { recursive: true });

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
      {
        cwd: process.cwd(),
        env: { ...process.env, CARGO_TARGET_DIR: targetDir },
        stdio: ['ignore', 'pipe', 'pipe'],
        windowsHide: true,
      },
    );
    backend.stdout.on('data', (chunk) => process.stdout.write(`[backend] ${chunk}`));
    backend.stderr.on('data', (chunk) => process.stderr.write(`[backend] ${chunk}`));
  }

  await waitForHealth(baseUrl);
  const result = await runQa(baseUrl);
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(desktopHtmlPath, renderDesktopHtml(result), 'utf8');
  writeFileSync(mobileHtmlPath, renderMobileHtml(result), 'utf8');
  await screenshotHtml(desktopHtmlPath, desktopScreenshotPath, { width: 1440, height: 1040 });
  await screenshotHtml(mobileHtmlPath, mobileScreenshotPath, { width: 430, height: 932 });
  result.screenshots = {
    desktop: desktopScreenshotPath,
    mobile: mobileScreenshotPath,
    desktopHtml: desktopHtmlPath,
    mobileHtml: mobileHtmlPath,
  };
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');

  console.log('Auto cloud sync preference runtime QA OK');
  console.log(`QA JSON: ${qaJsonPath}`);
  console.log(`QA Markdown: ${qaMdPath}`);
  console.log(`Desktop screenshot: ${desktopScreenshotPath}`);
  console.log(`Mobile screenshot: ${mobileScreenshotPath}`);
} finally {
  if (backend && !backend.killed) {
    backend.kill();
  }
}

async function runQa(endpoint) {
  const identifier = `auto-sync-runtime-${runId}@example.com`;
  const password = 'auto-sync-runtime-password';
  const creatorDisplayName = 'Auto Sync Runtime Creator';
  const desktopClientDeviceId = `desktop-auto-sync-${runId}`;
  const mobileClientDeviceId = `mobile-auto-sync-${runId}`;
  const startedAt = new Date().toISOString();

  const firstDesktop = await continueAccount(endpoint, {
    identifier,
    password,
    deviceId: desktopClientDeviceId,
    name: 'Runtime QA Desktop',
    platform: 'windows',
    creatorDisplayName,
  });
  await upgradeToCreator(endpoint, firstDesktop);

  const desktop = await continueAccount(endpoint, {
    identifier,
    password,
    deviceId: desktopClientDeviceId,
    name: 'Runtime QA Desktop',
    platform: 'windows',
    creatorDisplayName,
  });
  const mobile = await continueAccount(endpoint, {
    identifier,
    password,
    deviceId: mobileClientDeviceId,
    name: 'Runtime QA Mobile',
    platform: 'android',
    creatorDisplayName,
  });

  assert(desktop.account.id === mobile.account.id, 'desktop and mobile must share account');
  assert(desktop.workspace.id === mobile.workspace.id, 'desktop and mobile must share workspace');
  assert(desktop.syncPolicy === 'auto_cloud_vault', 'desktop Creator default must auto sync');
  assert(mobile.syncPolicy === 'auto_cloud_vault', 'mobile Creator default must auto sync');

  const mobileBaseline = await pullChanges(endpoint, mobile);
  const desktopBaseline = await pullChanges(endpoint, desktop);

  const desktopAutoRecord = await pushVaultRecord(endpoint, desktop, {
    clientEventId: `desktop-auto-${runId}`,
    recordId: `desktop-auto-record-${runId}`,
    title: `desktop-auto-${runId}.png`,
    watermarkUid: `qa-desktop-auto-${runId}`,
  });
  const mobileAutoPull = await pullChanges(endpoint, mobile, mobileBaseline.nextCursor);
  const desktopAutoOnMobile = findChange(mobileAutoPull, desktopAutoRecord.recordId);
  assert(Boolean(desktopAutoOnMobile), 'mobile automatic pull must see desktop record');

  const paused = await updateSyncPreferences(endpoint, mobile, false);
  assert(paused.syncPolicy === 'manual_local_only', 'mobile pause must return manual_local_only');
  const mobileAfterPause = await continueAccount(endpoint, {
    identifier,
    password,
    deviceId: mobileClientDeviceId,
    name: 'Runtime QA Mobile',
    platform: 'android',
    creatorDisplayName,
  });
  assert(
    mobileAfterPause.syncPolicy === 'manual_local_only',
    'mobile auth/sessions must preserve manual_local_only',
  );

  const mobileManualRecord = await pushVaultRecord(endpoint, mobileAfterPause, {
    clientEventId: `mobile-manual-${runId}`,
    recordId: `mobile-manual-record-${runId}`,
    title: `mobile-manual-${runId}.png`,
    watermarkUid: `qa-mobile-manual-${runId}`,
  });
  const desktopManualPull = await pullChanges(endpoint, desktop, desktopBaseline.nextCursor);
  const mobileManualOnDesktop = findChange(desktopManualPull, mobileManualRecord.recordId);
  assert(Boolean(mobileManualOnDesktop), 'manual push while paused must remain allowed');

  const desktopPausedRecord = await pushVaultRecord(endpoint, desktop, {
    clientEventId: `desktop-paused-${runId}`,
    recordId: `desktop-paused-record-${runId}`,
    title: `desktop-paused-${runId}.png`,
    watermarkUid: `qa-desktop-paused-${runId}`,
  });
  const pausedManualPull = await pullChanges(endpoint, mobileAfterPause, mobileAutoPull.nextCursor);
  const desktopPausedOnMobileManual = findChange(pausedManualPull, desktopPausedRecord.recordId);
  assert(
    Boolean(desktopPausedOnMobileManual),
    'manual pull while paused must remain allowed for Creator',
  );

  const resumed = await updateSyncPreferences(endpoint, mobileAfterPause, true);
  assert(resumed.syncPolicy === 'auto_cloud_vault', 'mobile resume must return auto_cloud_vault');
  const mobileAfterResume = await continueAccount(endpoint, {
    identifier,
    password,
    deviceId: mobileClientDeviceId,
    name: 'Runtime QA Mobile',
    platform: 'android',
    creatorDisplayName,
  });
  assert(
    mobileAfterResume.syncPolicy === 'auto_cloud_vault',
    'mobile auth/sessions must preserve resumed auto_cloud_vault',
  );

  const desktopResumedRecord = await pushVaultRecord(endpoint, desktop, {
    clientEventId: `desktop-resumed-${runId}`,
    recordId: `desktop-resumed-record-${runId}`,
    title: `desktop-resumed-${runId}.png`,
    watermarkUid: `qa-desktop-resumed-${runId}`,
  });
  const mobileResumePull = await pullChanges(endpoint, mobileAfterResume, pausedManualPull.nextCursor);
  const desktopResumedOnMobile = findChange(mobileResumePull, desktopResumedRecord.recordId);
  assert(Boolean(desktopResumedOnMobile), 'mobile resumed automatic pull must see desktop record');

  const steps = [
    {
      key: 'creator-default',
      title: 'Creator 默认自动同步',
      desktopPolicy: desktop.syncPolicy,
      mobilePolicy: mobile.syncPolicy,
      evidence: `desktop 写入 ${desktopAutoRecord.recordId} 后，mobile pull 读取到同一记录。`,
      pass: true,
    },
    {
      key: 'mobile-paused',
      title: '移动端暂停自动同步',
      desktopPolicy: desktop.syncPolicy,
      mobilePolicy: paused.syncPolicy,
      evidence: 'PATCH /v1/me/sync-preferences 返回 manual_local_only，重新 auth/sessions 后仍保持暂停。',
      pass: true,
    },
    {
      key: 'manual-while-paused',
      title: '暂停后手动同步仍可用',
      desktopPolicy: desktop.syncPolicy,
      mobilePolicy: mobileAfterPause.syncPolicy,
      evidence: `mobile 暂停后手动 push ${mobileManualRecord.recordId}，desktop pull 可读取；mobile 手动 pull 也可读取 ${desktopPausedRecord.recordId}。`,
      pass: true,
    },
    {
      key: 'mobile-resumed',
      title: '移动端恢复自动同步',
      desktopPolicy: desktop.syncPolicy,
      mobilePolicy: resumed.syncPolicy,
      evidence: `恢复后重新 auth/sessions 为 auto_cloud_vault，并读取到 ${desktopResumedRecord.recordId}。`,
      pass: true,
    },
  ];

  return {
    runId,
    startedAt,
    completedAt: new Date().toISOString(),
    backendBaseUrl: endpoint,
    account: {
      identifier,
      accountId: desktop.account.id,
      workspaceId: desktop.workspace.id,
      creatorProfileId: desktop.creatorProfile.id,
    },
    devices: {
      desktop: {
        clientDeviceId: desktopClientDeviceId,
        cloudDeviceId: desktop.device.id,
        syncPolicy: desktop.syncPolicy,
      },
      mobile: {
        clientDeviceId: mobileClientDeviceId,
        cloudDeviceId: mobile.device.id,
        initialSyncPolicy: mobile.syncPolicy,
        pausedSyncPolicy: paused.syncPolicy,
        resumedSyncPolicy: resumed.syncPolicy,
      },
    },
    records: {
      desktopAuto: desktopAutoRecord,
      mobileManualWhilePaused: mobileManualRecord,
      desktopWhileMobilePaused: desktopPausedRecord,
      desktopAfterResume: desktopResumedRecord,
    },
    cursors: {
      mobileBaseline: mobileBaseline.nextCursor,
      desktopBaseline: desktopBaseline.nextCursor,
      mobileAutoPull: mobileAutoPull.nextCursor,
      desktopManualPull: desktopManualPull.nextCursor,
      pausedManualPull: pausedManualPull.nextCursor,
      mobileResumePull: mobileResumePull.nextCursor,
    },
    steps,
    privacyBoundary:
      'QA only syncs account/device/workspace metadata and vault record metadata. It does not upload original media, protected-copy files, local paths, recoverable media content, or creator seed plaintext.',
    screenshots: {
      desktop: desktopScreenshotPath,
      mobile: mobileScreenshotPath,
      desktopHtml: desktopHtmlPath,
      mobileHtml: mobileHtmlPath,
    },
    pass: steps.every((step) => step.pass),
  };
}

async function upgradeToCreator(endpoint, session) {
  if (session.entitlement?.features?.cloud_sync === true) {
    return;
  }
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
  assert(payment.status === 200, 'fixture Creator payment session must succeed');
  const reconcile = await request(
    endpoint,
    'POST',
    `/v1/billing/payment-sessions/${payment.body.paymentSessionId}/reconcile`,
    {},
    session.accessToken,
  );
  assert(
    reconcile.status === 200 && reconcile.body?.entitlement?.features?.cloud_sync === true,
    'fixture Creator reconcile must enable cloud_sync',
  );
}

async function continueAccount(
  endpoint,
  { identifier, password, deviceId, name, platform, creatorDisplayName },
) {
  const response = await request(endpoint, 'POST', '/v1/auth/sessions', {
    identifier,
    password,
    verificationCode: password,
    device: {
      clientDeviceId: deviceId,
      name,
      platform,
      appVersion: 'auto-sync-runtime-qa',
    },
    localCreatorProfile: {
      displayName: creatorDisplayName,
      creatorSeedRef: `qa-seed-${identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, `auth/sessions failed for ${deviceId}`);
  return response.body;
}

async function updateSyncPreferences(endpoint, session, autoSyncEnabled) {
  const response = await request(
    endpoint,
    'PATCH',
    '/v1/me/sync-preferences',
    {
      autoSyncEnabled,
      reason: autoSyncEnabled ? 'user_resumed' : 'user_paused',
    },
    session.accessToken,
  );
  assert(response.status === 200, `sync preference update failed: ${response.status}`);
  return response.body;
}

async function pushVaultRecord(endpoint, session, { clientEventId, recordId, title, watermarkUid }) {
  const payload = {
    id: recordId,
    kind: 'image',
    title,
    watermark_uid: watermarkUid,
    revision: 1,
    sha256: `sha256-${recordId}`,
    protected_copy_name: title.replace('.png', '.protected.png'),
    protected_copy_hash: `protected-${recordId}`,
    payload_protocol_version: 2,
    payload_bytes_length: 119,
    watermark_id_issue_mode: 'server_reserved',
    watermark_id_registry_status: 'server_confirmed',
    payload_auth_status: 'verified',
    output_strategy: 'minimal_required_change',
    work_source_declaration: 'original',
    training_permission_declaration: 'prohibited',
    source: 'write',
    sync_status: 'synced',
    created_at: new Date().toISOString(),
  };
  const response = await request(
    endpoint,
    'POST',
    '/v1/sync/events:batch',
    {
      deviceId: session.device.id,
      workspaceId: session.workspace.id,
      events: [
        {
          clientEventId,
          operation: 'upsertVaultRecord',
          entityType: 'vaultRecord',
          entityId: recordId,
          payload,
        },
      ],
    },
    session.accessToken,
  );
  assert(response.status === 200, `push ${recordId} failed: ${response.status}`);
  assert(response.body.acceptedEventIds?.includes(clientEventId), `push ${recordId} not accepted`);
  return { recordId, clientEventId, title, watermarkUid, payload, nextCursor: response.body.nextCursor };
}

async function pullChanges(endpoint, session, cursor) {
  const path = new URL('/v1/sync/changes', endpoint);
  path.searchParams.set('workspaceId', session.workspace.id);
  if (cursor) {
    path.searchParams.set('cursor', cursor);
  }
  const response = await request(
    endpoint,
    'GET',
    `${path.pathname}${path.search}`,
    undefined,
    session.accessToken,
  );
  assert(response.status === 200, `pull changes failed: ${response.status}`);
  return response.body;
}

function findChange(changesResult, recordId) {
  return changesResult.changes?.find((change) => change.entity?.id === recordId);
}

async function waitForHealth(endpoint) {
  for (let i = 0; i < 300; i += 1) {
    try {
      const health = await request(endpoint, 'GET', '/v1/health');
      if (health.status === 200 && health.body?.ok === true) {
        return;
      }
    } catch {
      // keep waiting
    }
    await delay(1000);
  }
  throw new Error(`backend did not become ready at ${endpoint}`);
}

async function request(endpoint, method, path, body, token) {
  const headers = {};
  if (body !== undefined) {
    headers['content-type'] = 'application/json';
  }
  if (token) {
    headers.authorization = `Bearer ${token}`;
  }
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

function renderDesktopHtml(result) {
  const rows = result.steps
    .map(
      (step) => `
        <tr>
          <td>${escapeHtml(step.title)}</td>
          <td><span class="pill ${policyClass(step.desktopPolicy)}">${escapeHtml(step.desktopPolicy)}</span></td>
          <td><span class="pill ${policyClass(step.mobilePolicy)}">${escapeHtml(step.mobilePolicy)}</span></td>
          <td>${escapeHtml(step.evidence)}</td>
          <td><span class="ok">通过</span></td>
        </tr>`,
    )
    .join('');
  return `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>HiddenShield 自动云同步运行态 QA</title>
  <style>${baseCss()}${desktopCss()}</style>
</head>
<body>
  <main class="shell">
    <aside class="nav">
      <strong>HiddenShield</strong>
      <span>工作台</span>
      <span class="active">账户与云同步</span>
      <span>版权库</span>
      <span>设置</span>
    </aside>
    <section class="stage">
      <header>
        <div>
          <p class="eyebrow">运行态 QA</p>
          <h1>Creator 自动云同步暂停 / 恢复</h1>
          <p>${escapeHtml(result.account.identifier)} · ${escapeHtml(result.backendBaseUrl)}</p>
        </div>
        <span class="status">全部通过</span>
      </header>
      <div class="metrics">
        <div><span>账户</span><strong>${escapeHtml(result.account.accountId)}</strong></div>
        <div><span>工作区</span><strong>${escapeHtml(result.account.workspaceId)}</strong></div>
        <div><span>桌面设备</span><strong>${escapeHtml(result.devices.desktop.syncPolicy)}</strong></div>
        <div><span>移动设备</span><strong>${escapeHtml(result.devices.mobile.resumedSyncPolicy)}</strong></div>
      </div>
      <table>
        <thead><tr><th>验收项</th><th>桌面端</th><th>移动端</th><th>运行证据</th><th>结果</th></tr></thead>
        <tbody>${rows}</tbody>
      </table>
    </section>
    <aside class="context">
      <p class="eyebrow">隐私边界</p>
      <p>${escapeHtml(result.privacyBoundary)}</p>
      <hr />
      <p class="eyebrow">关键记录</p>
      <ul>
        <li>${escapeHtml(result.records.desktopAuto.recordId)}</li>
        <li>${escapeHtml(result.records.mobileManualWhilePaused.recordId)}</li>
        <li>${escapeHtml(result.records.desktopAfterResume.recordId)}</li>
      </ul>
    </aside>
  </main>
</body>
</html>`;
}

function renderMobileHtml(result) {
  const cards = result.steps
    .map(
      (step) => `
        <article class="mobile-card">
          <div>
            <strong>${escapeHtml(step.title)}</strong>
            <span class="ok">通过</span>
          </div>
          <p>${escapeHtml(step.evidence)}</p>
          <div class="policy-row">
            <span class="pill ${policyClass(step.desktopPolicy)}">${escapeHtml(step.desktopPolicy)}</span>
            <span class="pill ${policyClass(step.mobilePolicy)}">${escapeHtml(step.mobilePolicy)}</span>
          </div>
        </article>`,
    )
    .join('');
  return `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>HiddenShield 移动端自动云同步 QA</title>
  <style>${baseCss()}${mobileCss()}</style>
</head>
<body>
  <main class="phone">
    <header>
      <p class="eyebrow">账户与同步</p>
      <h1>自动云同步</h1>
      <span class="status">Creator</span>
    </header>
    <section class="summary">
      <span>当前设备</span>
      <strong>${escapeHtml(result.devices.mobile.resumedSyncPolicy)}</strong>
      <p>暂停态 ${escapeHtml(result.devices.mobile.pausedSyncPolicy)} 已验证，恢复后可继续自动拉取版权库。</p>
    </section>
    ${cards}
    <nav><span>工作台</span><span>验证</span><span class="active">版权库</span><span>设置</span></nav>
  </main>
</body>
</html>`;
}

function renderMarkdown(result) {
  const rows = result.steps
    .map(
      (step) =>
        `| ${step.title} | ${step.desktopPolicy} | ${step.mobilePolicy} | ${step.evidence} | 通过 |`,
    )
    .join('\n');
  return `# HiddenShield 自动云同步暂停 / 恢复运行态 QA

- Run ID: \`${result.runId}\`
- Backend: \`${result.backendBaseUrl}\`
- Account: \`${result.account.identifier}\`
- Desktop screenshot: \`${result.screenshots.desktop}\`
- Mobile screenshot: \`${result.screenshots.mobile}\`

## 验收结果

| 验收项 | 桌面端策略 | 移动端策略 | 运行证据 | 结果 |
| --- | --- | --- | --- | --- |
${rows}

## 隐私边界

${result.privacyBoundary}

## 结论

通过。Creator 默认自动云同步、当前设备暂停为 \`manual_local_only\`、暂停期间手动同步仍可用、恢复为 \`auto_cloud_vault\` 均已用真实后端运行态验证。
`;
}

async function screenshotHtml(htmlPath, screenshotPath, viewport) {
  const browser = await chromium.launch({
    headless: true,
    executablePath: findBrowserExecutable(),
  });
  try {
    const page = await browser.newPage({ viewport });
    await page.goto(`file://${htmlPath.replace(/\\/g, '/')}`);
    await page.screenshot({ path: screenshotPath, fullPage: true });
  } finally {
    await browser.close();
  }
}

function findBrowserExecutable() {
  if (process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH) {
    return process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE_PATH;
  }
  if (process.platform !== 'win32') {
    return undefined;
  }
  const candidates = [
    'C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe',
    'C:\\Program Files (x86)\\Google\\Chrome\\Application\\chrome.exe',
    'C:\\Program Files\\Microsoft\\Edge\\Application\\msedge.exe',
    'C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe',
  ];
  return candidates.find((candidate) => existsSync(candidate));
}

function baseCss() {
  return `
    :root {
      color-scheme: dark;
      --bg: #07110f;
      --panel: #0c1c18;
      --panel-2: #102620;
      --text: #edf8f2;
      --muted: #91aaa0;
      --line: rgba(255,255,255,.11);
      --accent: #70e0b8;
      --warn: #ffd166;
      --danger: #ff7a90;
    }
    * { box-sizing: border-box; }
    body { margin: 0; background: var(--bg); color: var(--text); font-family: "Microsoft YaHei", "Segoe UI", sans-serif; letter-spacing: 0; }
    h1 { margin: 0; font-size: 30px; font-weight: 720; }
    p { color: var(--muted); line-height: 1.65; }
    .eyebrow { margin: 0 0 6px; color: var(--accent); font-size: 13px; }
    .status, .ok { color: #06120f; background: var(--accent); border-radius: 6px; padding: 6px 10px; font-weight: 700; }
    .pill { display: inline-flex; min-height: 28px; align-items: center; border-radius: 6px; padding: 4px 8px; border: 1px solid var(--line); color: var(--text); white-space: nowrap; }
    .pill.auto { border-color: rgba(112,224,184,.7); color: var(--accent); }
    .pill.manual { border-color: rgba(255,209,102,.7); color: var(--warn); }
    .pill.blocked { border-color: rgba(255,122,144,.7); color: var(--danger); }
  `;
}

function desktopCss() {
  return `
    .shell { min-height: 100vh; display: grid; grid-template-columns: 220px minmax(0, 1fr) 300px; }
    .nav, .context { background: #081612; border-right: 1px solid var(--line); padding: 24px; }
    .context { border-right: 0; border-left: 1px solid var(--line); }
    .nav { display: flex; flex-direction: column; gap: 14px; }
    .nav strong { margin-bottom: 18px; font-size: 20px; }
    .nav span { color: var(--muted); padding: 10px 12px; border-radius: 6px; }
    .nav .active { color: var(--text); background: var(--panel-2); }
    .stage { padding: 28px; }
    header { display: flex; justify-content: space-between; gap: 20px; align-items: flex-start; margin-bottom: 24px; }
    .metrics { display: grid; grid-template-columns: repeat(4, 1fr); gap: 12px; margin-bottom: 20px; }
    .metrics div { background: var(--panel); border: 1px solid var(--line); border-radius: 8px; padding: 14px; min-width: 0; }
    .metrics span { display: block; color: var(--muted); margin-bottom: 8px; }
    .metrics strong { display: block; overflow-wrap: anywhere; font-size: 15px; }
    table { width: 100%; border-collapse: collapse; background: var(--panel); border: 1px solid var(--line); border-radius: 8px; overflow: hidden; }
    th, td { padding: 14px; text-align: left; border-bottom: 1px solid var(--line); vertical-align: top; }
    th { color: var(--muted); font-weight: 600; }
    td { color: var(--text); line-height: 1.55; }
    ul { padding-left: 18px; color: var(--muted); line-height: 1.8; overflow-wrap: anywhere; }
    hr { border: 0; border-top: 1px solid var(--line); margin: 20px 0; }
  `;
}

function mobileCss() {
  return `
    body { background: #020806; }
    .phone { min-height: 100vh; padding: 20px 16px 82px; background: linear-gradient(180deg, #07110f, #0b1915 36%, #06100d); }
    header { display: grid; grid-template-columns: 1fr auto; gap: 10px; align-items: start; margin-bottom: 16px; }
    header h1 { grid-column: 1 / 2; font-size: 28px; }
    .summary, .mobile-card { background: var(--panel); border: 1px solid var(--line); border-radius: 8px; padding: 14px; margin-bottom: 12px; }
    .summary span { color: var(--muted); font-size: 13px; }
    .summary strong { display: block; margin-top: 4px; font-size: 20px; color: var(--accent); }
    .mobile-card > div:first-child { display: flex; justify-content: space-between; gap: 12px; align-items: center; }
    .mobile-card strong { font-size: 16px; }
    .mobile-card p { margin: 10px 0 12px; }
    .policy-row { display: flex; flex-wrap: wrap; gap: 8px; }
    nav { position: fixed; left: 0; right: 0; bottom: 0; display: grid; grid-template-columns: repeat(4, 1fr); gap: 1px; background: #081612; border-top: 1px solid var(--line); }
    nav span { text-align: center; color: var(--muted); padding: 14px 4px; font-size: 13px; }
    nav .active { color: var(--accent); }
  `;
}

function policyClass(policy) {
  if (policy === 'auto_cloud_vault') return 'auto';
  if (policy === 'manual_local_only') return 'manual';
  return 'blocked';
}

function escapeHtml(value) {
  return String(value ?? '')
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;');
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

function delay(ms) {
  return new Promise((resolveDelay) => setTimeout(resolveDelay, ms));
}

function freePort() {
  return new Promise((resolvePort, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const port = typeof address === 'object' && address ? address.port : null;
      server.close(() => {
        if (port) {
          resolvePort(port);
        } else {
          reject(new Error('cannot allocate free port'));
        }
      });
    });
    server.on('error', reject);
  });
}

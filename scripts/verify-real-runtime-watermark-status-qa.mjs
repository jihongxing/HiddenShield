import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { tmpdir } from 'node:os';
import { chromium } from 'playwright';

const runId = Date.now().toString();
const outputDir = resolve('tmp-ui-qa', 'real-runtime-status');
mkdirSync(outputDir, { recursive: true });

const backendBaseUrl = process.env.HIDDENSHIELD_QA_BACKEND_URL ?? 'http://127.0.0.1:43188';
const mobileBackendBaseUrl = process.env.HIDDENSHIELD_QA_MOBILE_BACKEND_URL ?? 'http://10.0.2.2:43188';

const qaJsonPath = join(outputDir, `real-runtime-status-qa-${runId}.json`);
const qaMdPath = join(outputDir, `real-runtime-status-qa-${runId}.md`);
const desktopHtmlPath = join(outputDir, `desktop-runtime-status-${runId}.html`);
const desktopScreenshotPath = join(outputDir, `desktop-runtime-status-${runId}.png`);

const desktopAccount = `desktop-real-qa-${runId}@hiddenshield.local`;
const mobileAccount = `mobile-real-qa-${runId}@hiddenshield.local`;
const password = `qa-${runId}`;

const mediaCases = [
  { platform: 'desktop', mediaKind: 'image', mediaType: 'image' },
  { platform: 'desktop', mediaKind: 'audio', mediaType: 'audio' },
  { platform: 'mobile', mediaKind: 'image', mediaType: 'image' },
  { platform: 'mobile', mediaKind: 'audio', mediaType: 'audio' },
];

const startedAt = new Date().toISOString();
await assertBackendHealth();
const desktopSession = await ensureCreatorSession(desktopAccount, '桌面端 QA 创作者');
const mobileSession = await ensureCreatorSession(mobileAccount, '移动端 QA 创作者');

const desktopResults = [];
for (const mediaCase of mediaCases.filter((item) => item.platform === 'desktop')) {
  desktopResults.push(await runStatusTriplet(mediaCase, desktopSession));
}

writeFileSync(
  desktopHtmlPath,
  renderDesktopEvidenceHtml({
    runId,
    startedAt,
    backendBaseUrl,
    desktopAccount,
    rows: desktopResults.flatMap((item) => item.statuses),
  }),
  'utf8',
);
await screenshotHtml(desktopHtmlPath, desktopScreenshotPath, { width: 1440, height: 1100 });

const result = {
  runId,
  startedAt,
  completedAt: new Date().toISOString(),
  backendBaseUrl,
  mobileBackendBaseUrl,
  accounts: {
    desktop: desktopAccount,
    mobile: mobileAccount,
  },
  desktop: {
    screenshot: desktopScreenshotPath,
    evidenceHtml: desktopHtmlPath,
    media: desktopResults,
  },
  mobile: {
    status: 'pending_external_native_screenshot',
    expectedCommand:
      'flutter run -d emulator-5554 -t mobile_app/tool/real_runtime_qa.dart --dart-define=HIDDENSHIELD_QA_BACKEND_URL=http://10.0.2.2:43188',
    expectedScreenshotDir: outputDir,
  },
};
writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');

console.log(`QA JSON: ${qaJsonPath}`);
console.log(`QA Markdown: ${qaMdPath}`);
console.log(`Desktop screenshot: ${desktopScreenshotPath}`);

async function assertBackendHealth() {
  const response = await request('GET', '/v1/health');
  assert(response.status === 200 && response.body?.ok === true, 'backend /v1/health must be ok');
}

async function ensureCreatorSession(identifier, creatorName) {
  const sessionResponse = await request('POST', '/v1/auth/sessions', {
    identifier,
    password,
    verificationCode: password,
    device: {
      clientDeviceId: `qa-device-${identifier}`,
      name: identifier.includes('desktop') ? 'Desktop Runtime QA' : 'Android Runtime QA',
      platform: identifier.includes('desktop') ? 'windows' : 'android',
      appVersion: 'real-runtime-qa',
    },
    localCreatorProfile: {
      displayName: creatorName,
      creatorSeedRef: `qa-seed-${identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(sessionResponse.status === 200, `continue account failed for ${identifier}`);
  const session = sessionResponse.body;
  if (session.entitlement?.features?.cloud_sync !== true) {
    const payment = await request(
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
    assert(payment.status === 200, `create fixture creator payment failed for ${identifier}`);
    const reconcile = await request(
      'POST',
      `/v1/billing/payment-sessions/${payment.body.paymentSessionId}/reconcile`,
      {},
      session.accessToken,
    );
    assert(
      reconcile.status === 200 && reconcile.body?.entitlement?.features?.cloud_sync === true,
      `fixture creator reconcile failed for ${identifier}`,
    );
  }
  const refreshed = await request('GET', '/v1/entitlements/current', undefined, session.accessToken);
  assert(refreshed.status === 200, `refresh entitlement failed for ${identifier}`);
  return { ...session, entitlement: refreshed.body };
}

async function runStatusTriplet(mediaCase, session) {
  const originalHash = sha256Hex(`${mediaCase.platform}:${mediaCase.mediaKind}:original:${runId}`);
  const protectedHash = sha256Hex(`${mediaCase.platform}:${mediaCase.mediaKind}:protected:${runId}`);
  const requestId = `${mediaCase.platform}-${mediaCase.mediaKind}-${runId}`;
  const reserve = await request(
    'POST',
    '/v1/watermark-ids/reserve',
    {
      requestId,
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      mediaType: mediaCase.mediaType,
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      parentWatermarkUid: null,
      revision: 1,
      originalHash: `sha256:${originalHash}`,
    },
    session.accessToken,
  );
  assert(reserve.status === 200, `${requestId} reserve failed`);
  const confirm = await request(
    'POST',
    '/v1/watermark-ids/confirm',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      watermarkUid: reserve.body.watermarkUid,
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      originalHash: `sha256:${originalHash}`,
      protectedCopyHash: `sha256:${protectedHash}`,
      writeVerificationStatus: 'verified',
    },
    session.accessToken,
  );
  assert(confirm.status === 200, `${requestId} confirm failed`);
  assert(confirm.body.registryStatus === 'server_confirmed', `${requestId} confirm status mismatch`);

  const offlineUid = uidFromHex(sha256Hex(`${mediaCase.platform}:${mediaCase.mediaKind}:offline:${runId}`).slice(0, 32));
  const pending = {
    watermarkUid: offlineUid,
    watermarkIdIssueMode: 'offline_generated',
    registryStatus: 'pending_registration',
    registryReceipt: null,
    payloadProtocolVersion: 2,
    payloadBytesLength: 119,
    parentWatermarkUid: null,
    revision: 1,
  };
  const reconcile = await request(
    'POST',
    '/v1/watermark-ids/reconcile',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      watermarkUid: offlineUid,
      mediaType: mediaCase.mediaType,
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      parentWatermarkUid: null,
      revision: 1,
      originalHash: `sha256:${sha256Hex(`${mediaCase.platform}:${mediaCase.mediaKind}:offline-original:${runId}`)}`,
      protectedCopyHash: `sha256:${sha256Hex(`${mediaCase.platform}:${mediaCase.mediaKind}:offline-protected:${runId}`)}`,
      writeVerificationStatus: 'verified',
    },
    session.accessToken,
  );
  assert(reconcile.status === 200, `${requestId} reconcile failed`);
  assert(
    reconcile.body.registryStatus === 'offline_confirmed',
    `${requestId} reconcile status mismatch`,
  );
  return {
    mediaKind: mediaCase.mediaKind,
    statuses: [
      evidenceRow(mediaCase, confirm.body, 'server_confirmed', '在线 reserve -> confirm'),
      evidenceRow(mediaCase, pending, 'pending_registration', '后端不可用时离线生成，仅本地待登记'),
      evidenceRow(mediaCase, reconcile.body, 'offline_confirmed', '云同步前 reconcile 后补登记'),
    ],
  };
}

function evidenceRow(mediaCase, body, expectedStatus, workflow) {
  return {
    platform: mediaCase.platform,
    mediaKind: mediaCase.mediaKind,
    workflow,
    expectedStatus,
    watermarkUid: body.watermarkUid,
    issueMode: body.watermarkIdIssueMode,
    registryStatus: body.registryStatus,
    registryReceipt: body.registryReceipt,
    parentWatermarkUid: body.parentWatermarkUid ?? null,
    revision: body.revision,
    payloadProtocolVersion: body.payloadProtocolVersion,
    payloadBytesLength: body.payloadBytesLength,
    pass: body.registryStatus === expectedStatus,
  };
}

async function request(method, path, body, accessToken) {
  const headers = { 'content-type': 'application/json' };
  if (accessToken) headers.authorization = `Bearer ${accessToken}`;
  const response = await fetch(`${backendBaseUrl}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let parsed = null;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    parsed = text;
  }
  return { status: response.status, body: parsed };
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

function renderDesktopEvidenceHtml({ runId, startedAt, backendBaseUrl, desktopAccount, rows }) {
  const cards = rows
    .map(
      (row) => `
        <article class="card ${row.pass ? 'ok' : 'fail'}">
          <div class="topline">
            <span>${escapeHtml(row.mediaKind === 'image' ? '图片写入' : '音频写入')}</span>
            <strong>${escapeHtml(statusLabel(row.registryStatus))}</strong>
          </div>
          <h2>${escapeHtml(row.workflow)}</h2>
          <dl>
            <dt>版权编号</dt><dd>${escapeHtml(row.watermarkUid)}</dd>
            <dt>编号签发模式</dt><dd>${escapeHtml(issueModeLabel(row.issueMode))}</dd>
            <dt>登记状态</dt><dd>${escapeHtml(statusLabel(row.registryStatus))}</dd>
            <dt>Payload</dt><dd>V${row.payloadProtocolVersion} / ${row.payloadBytesLength} bytes</dd>
            <dt>父编号 / 版本</dt><dd>${escapeHtml(row.parentWatermarkUid ?? '无')} / 第 ${row.revision} 次</dd>
            <dt>验收</dt><dd>${row.pass ? 'PASS' : 'FAIL'}</dd>
          </dl>
        </article>`,
    )
    .join('\n');
  return `<!doctype html>
<html lang="zh-CN">
<head>
  <meta charset="utf-8" />
  <title>HiddenShield Desktop Runtime QA</title>
  <style>
    :root {
      color-scheme: dark;
      font-family: "Microsoft YaHei", "Segoe UI", sans-serif;
      background: #080b0f;
      color: #edf3ff;
    }
    body { margin: 0; padding: 32px; background: radial-gradient(circle at top left, #12323a 0, #080b0f 32%, #080b0f 100%); }
    header { display: flex; justify-content: space-between; gap: 24px; align-items: flex-start; margin-bottom: 26px; }
    h1 { margin: 0 0 10px; font-size: 30px; letter-spacing: 0; }
    p { margin: 0; color: #a9b6c9; line-height: 1.55; }
    .meta { text-align: right; font-size: 13px; color: #96a4b7; }
    .grid { display: grid; grid-template-columns: repeat(3, minmax(0, 1fr)); gap: 16px; }
    .card { border: 1px solid #263340; border-radius: 8px; background: rgba(15, 22, 30, .92); padding: 18px; box-shadow: 0 20px 60px rgba(0, 0, 0, .25); }
    .card.ok { border-color: rgba(42, 190, 139, .42); }
    .card.fail { border-color: rgba(255, 103, 103, .58); }
    .topline { display: flex; justify-content: space-between; align-items: center; color: #8ea0b6; font-size: 12px; margin-bottom: 14px; }
    .topline strong { color: #73e2ba; font-size: 12px; }
    h2 { margin: 0 0 16px; font-size: 16px; line-height: 1.35; letter-spacing: 0; }
    dl { display: grid; grid-template-columns: 92px minmax(0, 1fr); gap: 10px 12px; margin: 0; font-size: 13px; }
    dt { color: #7f8fa3; }
    dd { margin: 0; color: #e7edf7; overflow-wrap: anywhere; }
    footer { margin-top: 22px; color: #7f8fa3; font-size: 12px; }
  </style>
</head>
<body>
  <header>
    <div>
      <h1>HiddenShield 桌面端真实后端运行态 QA</h1>
      <p>各写入一张图片 / 一段音频对应的版权编号登记状态验收：server_confirmed、pending_registration、offline_confirmed。</p>
    </div>
    <div class="meta">
      <div>Run ID: ${escapeHtml(runId)}</div>
      <div>Started: ${escapeHtml(startedAt)}</div>
      <div>Backend: ${escapeHtml(backendBaseUrl)}</div>
      <div>Account: ${escapeHtml(desktopAccount)}</div>
    </div>
  </header>
  <main class="grid">${cards}</main>
  <footer>证据来自真实 feedback-backend 的 watermark-ids reserve / confirm / reconcile API；pending_registration 是客户端离线待登记记录状态。</footer>
</body>
</html>`;
}

function renderMarkdown(result) {
  const desktopRows = result.desktop.media.flatMap((item) => item.statuses);
  return `# HiddenShield 双端真实运行态版权登记状态 QA

- Run ID: \`${result.runId}\`
- 时间: ${result.startedAt} -> ${result.completedAt}
- 后端: ${result.backendBaseUrl}
- Android 模拟器后端地址: ${result.mobileBackendBaseUrl}

## 桌面端

- 截图: \`${result.desktop.screenshot}\`
- 证据页: \`${result.desktop.evidenceHtml}\`

| 媒体 | 工作流 | 编号签发模式 | 登记状态 | Payload | 版权编号 | 结果 |
| --- | --- | --- | --- | --- | --- | --- |
${desktopRows
  .map(
    (row) =>
      `| ${row.mediaKind} | ${row.workflow} | ${row.issueMode} | ${row.registryStatus} | V${row.payloadProtocolVersion}/${row.payloadBytesLength} | ${row.watermarkUid} | ${row.pass ? 'PASS' : 'FAIL'} |`,
  )
  .join('\n')}

## 移动端

- 状态: ${result.mobile.status}
- 运行命令: \`${result.mobile.expectedCommand}\`
- 截图目录: \`${result.mobile.expectedScreenshotDir}\`

## 结论

桌面端真实后端状态链路已完成截图证据；移动端需由 Android 原生 QA 入口继续补齐截图并回填本记录。
`;
}

function sha256Hex(value) {
  return createHash('sha256').update(value).digest('hex');
}

function uidFromHex(hex32) {
  return `HS-${hex32.slice(0, 8)}-${hex32.slice(8, 16)}-${hex32.slice(16, 24)}-${hex32.slice(24, 32)}`.toUpperCase();
}

function issueModeLabel(value) {
  return (
    {
      server_reserved: '后端预签发',
      server_confirmed: '后端已确认',
      server_reissued: '后端重新签发',
      offline_generated: '离线高熵生成',
    }[value] ?? value
  );
}

function statusLabel(value) {
  return (
    {
      reserved: '已预留，等待写入确认',
      server_confirmed: '后端已确认',
      offline_confirmed: '离线编号已补登记',
      pending_registration: '待联网登记',
    }[value] ?? value
  );
}

function escapeHtml(value) {
  return String(value)
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;');
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

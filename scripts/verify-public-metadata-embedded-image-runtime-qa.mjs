import { createHash } from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import net from 'node:net';

const runId = process.env.HIDDENSHIELD_PUBLIC_METADATA_EMBED_QA_RUN_ID ?? `${Date.now()}`;
const endpoint = process.env.HIDDENSHIELD_CLOUD_URL?.replace(/\/$/, '') ?? null;
const shouldStartBackend = !endpoint;
const port = shouldStartBackend ? await freePort() : Number(new URL(endpoint).port || 80);
const baseUrl = endpoint ?? `http://127.0.0.1:${port}`;
const tmpRoot = join(tmpdir(), `hiddenshield-public-metadata-embedded-image-qa-${runId}`);
const dbPath = join(tmpRoot, 'cloud.sqlite');
const outputDir = resolve('tmp-ui-qa', 'public-metadata-embedded-image', runId);
const protectedDir = join(outputDir, 'protected');
const metadataDir = join(outputDir, 'metadata');
const embeddedDir = join(outputDir, 'embedded');
const qaJsonPath = join(outputDir, `public-metadata-embedded-image-qa-${runId}.json`);
const qaMdPath = join(outputDir, `public-metadata-embedded-image-qa-${runId}.md`);

mkdirSync(tmpRoot, { recursive: true });
mkdirSync(protectedDir, { recursive: true });
mkdirSync(metadataDir, { recursive: true });
mkdirSync(embeddedDir, { recursive: true });

let backend;
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
  const session = await ensureCreatorSession(baseUrl);
  const rows = [];
  for (const imageCase of [
    { format: 'png', mediaType: 'image/png', trainingPermissionDeclaration: 'commercial_allowed' },
    { format: 'jpeg', mediaType: 'image/jpeg', trainingPermissionDeclaration: 'restricted' },
  ]) {
    rows.push(await runEmbeddedImageCase(session, imageCase));
  }
  const result = {
    runId,
    baseUrl,
    startedBackend: shouldStartBackend,
    dbPath: shouldStartBackend ? dbPath : null,
    outputDir,
    rows,
    pass: rows.every((row) => row.pass),
    completedAt: new Date().toISOString(),
  };
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
  if (!result.pass) {
    throw new Error(`public metadata embedded image runtime QA failed: ${qaJsonPath}`);
  }
  console.log('Public metadata embedded image runtime QA OK');
  console.log(`QA JSON: ${qaJsonPath}`);
  console.log(`QA Markdown: ${qaMdPath}`);
} finally {
  for (const child of childProcesses.reverse()) {
    await stopChild(child);
  }
}

async function runEmbeddedImageCase(session, imageCase) {
  const label = `desktop-${imageCase.format}-public-metadata-${runId}`;
  const originalHash = `sha256:${sha256(`${label}:original`)}`;
  const protectedCopyHash = `sha256:${sha256(`${label}:protected`)}`;
  const reserve = await request(
    baseUrl,
    'POST',
    '/v1/watermark-ids/reserve',
    {
      requestId: `${label}-reserve`,
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      mediaType: 'image',
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      parentWatermarkUid: null,
      revision: 1,
      originalHash,
    },
    session.accessToken,
  );
  assert(reserve.status === 200, `${imageCase.format} reserve must succeed`);
  const confirm = await request(
    baseUrl,
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
  assert(confirm.status === 200, `${imageCase.format} confirm must succeed`);
  const protectedPath = join(
    protectedDir,
    `${label}.protected.${imageCase.format === 'jpeg' ? 'jpg' : 'png'}`,
  );
  const protectedJson = runJson('cargo', [
    'run',
    '--quiet',
    '--manifest-path',
    'watermark-core/Cargo.toml',
    '--bin',
    'protected_copy_file_flow_qa',
    '--',
    'generate-image',
    '--run-id',
    runId,
    '--watermark-uid',
    confirm.body.watermarkUid,
    '--format',
    imageCase.format,
    '--output',
    protectedPath,
  ]);
  const payload = {
    id: `${label}-record`,
    kind: 'image',
    title: `${label}.${imageCase.format === 'jpeg' ? 'jpg' : 'png'}`,
    watermark_uid: confirm.body.watermarkUid,
    revision: confirm.body.revision,
    creator_display_name: session.creatorProfile.displayName,
    sha256: originalHash,
    protected_copy_name: protectedPath.split(/[\\/]/).pop(),
    protected_copy_hash: protectedCopyHash,
    payload_protocol_version: confirm.body.payloadProtocolVersion,
    payload_bytes_length: confirm.body.payloadBytesLength,
    watermark_id_issue_mode: confirm.body.watermarkIdIssueMode,
    watermark_id_registry_status: confirm.body.registryStatus,
    watermark_id_registry_receipt: confirm.body.registryReceipt,
    payload_auth_status: 'verified',
    output_strategy: 'minimal_required_change',
    work_source_declaration: 'ai_assisted',
    training_permission_declaration: imageCase.trainingPermissionDeclaration,
    creation_method_declaration: 'text_to_image',
    human_edit_level_declaration: 'light',
    authenticity_claim_declaration: 'synthetic',
    custom_rights_statement: `public metadata embedded image QA ${imageCase.format}`,
    source: 'write',
    sync_status: 'synced',
    created_at: new Date().toISOString(),
  };
  const pushed = await request(
    baseUrl,
    'POST',
    '/v1/sync/events:batch',
    {
      deviceId: session.device.id,
      workspaceId: session.workspace.id,
      events: [
        {
          clientEventId: `${label}-sync`,
          operation: 'upsertVaultRecord',
          entityType: 'vaultRecord',
          entityId: payload.id,
          payload,
        },
      ],
    },
    session.accessToken,
  );
  assert(pushed.status === 200, `${imageCase.format} sync push must succeed`);
  const metadata = await request(
    baseUrl,
    'GET',
    `/v1/public/rights/${encodeURIComponent(confirm.body.watermarkUid)}/metadata`,
  );
  assert(metadata.status === 200, `${imageCase.format} metadata export must succeed`);
  assert(metadata.body.legalConclusion === false, `${imageCase.format} legalConclusion must be false`);
  const metadataPath = join(metadataDir, `${label}.metadata.json`);
  writeFileSync(metadataPath, `${JSON.stringify(metadata.body, null, 2)}\n`, 'utf8');
  const embeddedPath = join(
    embeddedDir,
    `${label}.embedded.${imageCase.format === 'jpeg' ? 'jpg' : 'png'}`,
  );
  const checkJsonPath = join(embeddedDir, `${label}.checks.json`);
  const checks = runJson('cargo', [
    'run',
    '--quiet',
    '--manifest-path',
    'src-tauri/Cargo.toml',
    '--features',
    'internal-qa',
    '--example',
    'public_metadata_embed_qa',
    '--',
    '--source',
    protectedPath,
    '--metadata',
    metadataPath,
    '--output',
    embeddedPath,
    '--format',
    imageCase.format,
    '--json-out',
    checkJsonPath,
  ]);
  const byteChecks = checks.checks ?? {};
  const pass = Object.values(byteChecks).every((value) => value === true);
  return {
    format: imageCase.format,
    watermarkUid: confirm.body.watermarkUid,
    manifestHash: metadata.body.manifestHash,
    trainingPermissionDeclaration: imageCase.trainingPermissionDeclaration,
    publicMetadataLegalConclusion: metadata.body.legalConclusion,
    protectedPath,
    embeddedPath,
    metadataPath,
    checkJsonPath,
    protectedCopy: protectedJson,
    byteChecks,
    pass,
  };
}

async function ensureCreatorSession(endpoint) {
  const identifier = `public-metadata-embed-${runId}@hiddenshield.local`;
  const password = `public-metadata-embed-${runId}`;
  const response = await request(endpoint, 'POST', '/v1/auth/sessions', {
    identifier,
    password,
    verificationCode: password,
    device: {
      clientDeviceId: `public-metadata-embed-device-${runId}`,
      name: 'Desktop Public Metadata Embed QA',
      platform: 'windows',
      appVersion: 'public-metadata-embed-runtime-qa',
    },
    localCreatorProfile: {
      displayName: '公开元数据嵌入 QA 创作者',
      creatorSeedRef: `public-metadata-embed-seed-${runId}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, 'auth session must succeed');
  const session = response.body;
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
    assert(payment.status === 200, 'fixture creator payment must succeed');
    const reconcile = await request(
      endpoint,
      'POST',
      `/v1/billing/payment-sessions/${payment.body.paymentSessionId}/reconcile`,
      {},
      session.accessToken,
    );
    assert(
      reconcile.status === 200 && reconcile.body?.entitlement?.features?.cloud_sync === true,
      'fixture creator reconcile must enable cloud sync',
    );
  }
  return session;
}

async function waitForHealth(endpoint) {
  const deadline = Date.now() + 60_000;
  let lastError = null;
  while (Date.now() < deadline) {
    try {
      const response = await request(endpoint, 'GET', '/v1/health');
      if (response.status === 200 && response.body?.ok === true) return;
      lastError = new Error(`health returned ${response.status}`);
    } catch (error) {
      lastError = error;
    }
    await sleep(500);
  }
  throw lastError ?? new Error('backend health timed out');
}

async function request(endpoint, method, path, body, accessToken) {
  const headers = { 'content-type': 'application/json' };
  if (accessToken) headers.authorization = `Bearer ${accessToken}`;
  const response = await fetch(`${endpoint}${path}`, {
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

function runJson(command, args) {
  const result = spawnSync(command, args, {
    cwd: process.cwd(),
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(
      `${command} ${args.join(' ')} failed with ${result.status}\nSTDOUT:\n${result.stdout}\nSTDERR:\n${result.stderr}`,
    );
  }
  return JSON.parse(result.stdout || readFileSync(args[args.indexOf('--json-out') + 1], 'utf8'));
}

async function freePort() {
  return new Promise((resolvePort, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const selected = typeof address === 'object' && address ? address.port : null;
      server.close(() => {
        if (selected) resolvePort(selected);
        else reject(new Error('failed to allocate free port'));
      });
    });
    server.on('error', reject);
  });
}

async function stopChild(child) {
  if (!child || child.killed) return;
  child.kill();
  await sleep(300);
}

function sleep(ms) {
  return new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function renderMarkdown(result) {
  const lines = [
    '# HiddenShield 公开元数据嵌入图片副本运行态 QA',
    '',
    `- Run ID: \`${result.runId}\``,
    `- 后端: ${result.baseUrl}`,
    `- 完成时间: ${result.completedAt}`,
    '',
    '| 格式 | watermarkUid | manifestHash | 容器 | namespace | C2PA active manifest | UID | manifestHash | legalConclusion=false | 结果 |',
    '| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |',
  ];
  for (const row of result.rows) {
    lines.push(
      `| ${row.format} | \`${row.watermarkUid}\` | \`${row.manifestHash}\` | ${mark(row.byteChecks.hasContainer)} | ${mark(row.byteChecks.hasNamespace)} | ${mark(row.byteChecks.hasC2paActiveManifest)} | ${mark(row.byteChecks.hasWatermarkUid)} | ${mark(row.byteChecks.hasManifestHash)} | ${mark(row.byteChecks.hasLegalConclusionFalse)} | ${row.pass ? 'PASS' : 'FAIL'} |`,
    );
  }
  lines.push(
    '',
    '## 产物',
    '',
    ...result.rows.flatMap((row) => [
      `- ${row.format} 保护副本: \`${row.protectedPath}\``,
      `- ${row.format} 嵌入副本: \`${row.embeddedPath}\``,
      `- ${row.format} 字节检查: \`${row.checkJsonPath}\``,
    ]),
    '',
    '## 结论',
    '',
    result.pass
      ? 'PNG iTXt 与 JPEG APP1 中均已通过字节级检查，且官方 C2PA Reader 可读取 active manifest；确认包含 watermarkUid、manifestHash 和 legalConclusion=false。'
      : '存在未通过的字节级检查。',
  );
  return `${lines.join('\n')}\n`;
}

function mark(value) {
  return value ? 'PASS' : 'FAIL';
}

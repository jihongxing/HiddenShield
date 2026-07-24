import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import net from 'node:net';

const runId = process.env.HIDDENSHIELD_DUAL_RUNTIME_QA_RUN_ID ?? `${Date.now()}`;
const endpoint = process.env.HIDDENSHIELD_CLOUD_URL?.replace(/\/$/, '') ?? null;
const shouldStartBackend = !endpoint;
const port = shouldStartBackend ? await freePort() : Number(new URL(endpoint).port || 80);
const baseUrl = endpoint ?? `http://127.0.0.1:${port}`;
const tmpRoot = join(tmpdir(), `hiddenshield-dual-runtime-qa-${runId}`);
const dbPath = join(tmpRoot, 'cloud.sqlite');
const outputDir = join(process.cwd(), 'tmp-ui-qa', 'dual-runtime');
const qaJsonPath = join(outputDir, `dual-vault-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `dual-vault-runtime-qa-${runId}.md`);
mkdirSync(tmpRoot, { recursive: true });
mkdirSync(outputDir, { recursive: true });

const imageRecordKeys = [
  'id',
  'kind',
  'title',
  'watermark_uid',
  'revision',
  'creator_display_name',
  'trusted_time_status',
  'trusted_time_source',
  'trusted_time_at',
  'third_party_verification_status',
  'third_party_verification_provider',
  'third_party_verification_path',
  'sha256',
  'parent_watermark_uid',
  'rewrite_reason',
  'write_verification_status',
  'write_verification_message',
  'write_verification_at',
  'protected_copy_name',
  'protected_copy_hash',
  'payload_protocol_version',
  'payload_bytes_length',
  'media_payload_role',
  'watermark_id_issue_mode',
  'watermark_id_registry_status',
  'watermark_id_registry_receipt',
  'payload_auth_status',
  'output_strategy',
  'work_source_declaration',
  'training_permission_declaration',
  'creation_method_declaration',
  'human_edit_level_declaration',
  'authenticity_claim_declaration',
  'custom_rights_statement',
  'source',
  'sync_status',
  'created_at',
];

const fieldLabels = [
  [['文件名', '原文件'], 'title'],
  ['版权编号', 'watermark_uid'],
  ['版本次数', 'revision'],
  ['创作者身份', 'creator_display_name'],
  ['作品指纹', 'sha256'],
  ['保护副本名称', 'protected_copy_name'],
  ['保护副本摘要', 'protected_copy_hash'],
  ['输出策略', 'output_strategy'],
  ['完成后验证', 'write_verification_status'],
  ['验证说明', 'write_verification_message'],
  ['验证时间', 'write_verification_at'],
  ['Payload 协议', 'payload_protocol_version'],
  ['媒体载荷角色', 'media_payload_role'],
  ['编号签发模式', 'watermark_id_issue_mode'],
  ['登记状态', 'watermark_id_registry_status'],
  ['登记收据', 'watermark_id_registry_receipt'],
  ['Payload 认证状态', 'payload_auth_status'],
  ['第三方验证', 'third_party_verification_status'],
  ['可信时间', 'trusted_time_status'],
  ['时间来源', 'trusted_time_source'],
  ['作品来源声明', 'work_source_declaration'],
  ['训练许可声明', 'training_permission_declaration'],
  ['创作方式声明', 'creation_method_declaration'],
  ['人工编辑声明', 'human_edit_level_declaration'],
  ['真实性声明', 'authenticity_claim_declaration'],
  ['自定义版权声明', 'custom_rights_statement'],
];

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
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
  console.log(`Dual vault runtime QA OK`);
  console.log(`QA JSON: ${qaJsonPath}`);
  console.log(`QA Markdown: ${qaMdPath}`);
} finally {
  if (backend && !backend.killed) {
    backend.kill();
  }
}

async function runQa(endpoint) {
  const identifier = `dual-runtime-${runId}@example.com`;
  const password = 'dual-runtime-password';
  const creatorDisplayName = 'Dual Runtime Creator';
  const desktopClientDeviceId = `desktop-runtime-${runId}`;
  const mobileClientDeviceId = `mobile-runtime-${runId}`;

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
  await upgradeToCreator(endpoint, desktop);

  assert(desktop.account.id === mobile.account.id, 'desktop and mobile must share one account');
  assert(desktop.workspace.id === mobile.workspace.id, 'desktop and mobile must share one workspace');
  assert(
    desktop.creatorProfile.id === mobile.creatorProfile.id,
    'desktop and mobile must share one creator profile',
  );

  const desktopBaseline = await changes(endpoint, desktop);
  const mobileBaseline = await changes(endpoint, mobile);

  const desktopPayload = await writeImageRecordThroughBackend({
    endpoint,
    session: desktop,
    clientDeviceId: desktopClientDeviceId,
    origin: 'desktop',
    title: `desktop-runtime-${runId}.png`,
    eventId: `desktop-event-${runId}`,
    recordId: `desktop-record-${runId}`,
    payloadProtocolVersion: 2,
    payloadBytesLength: 119,
  });
  const mobilePulled = await changes(endpoint, mobile, mobileBaseline.nextCursor);
  const desktopToMobileChange = findChange(mobilePulled, desktopPayload.id);
  assert(Boolean(desktopToMobileChange), 'mobile must pull desktop-written image record');
  assert(
    desktopToMobileChange.sourceDevice === desktopClientDeviceId,
    'mobile pull must identify desktop source device',
  );
  const desktopToMobile = verifyDirection(
    'desktop->mobile',
    desktopToMobileChange.entity,
    desktopPayload,
  );

  const mobilePayload = await writeImageRecordThroughBackend({
    endpoint,
    session: mobile,
    clientDeviceId: mobileClientDeviceId,
    origin: 'mobile',
    title: `mobile-runtime-v3-bridge-${runId}.png`,
    eventId: `mobile-event-${runId}`,
    recordId: `mobile-record-${runId}`,
    payloadProtocolVersion: 3,
    payloadBytesLength: 39,
  });
  const desktopPulled = await changes(endpoint, desktop, desktopBaseline.nextCursor);
  const mobileToDesktopChange = findChange(desktopPulled, mobilePayload.id);
  assert(Boolean(mobileToDesktopChange), 'desktop must pull mobile-written image record');
  assert(
    mobileToDesktopChange.sourceDevice === mobileClientDeviceId,
    'desktop pull must identify mobile source device',
  );
  const mobileToDesktop = verifyDirection(
    'mobile->desktop',
    mobileToDesktopChange.entity,
    mobilePayload,
  );

  return {
    runId,
    endpoint,
    startedBackend: shouldStartBackend,
    dbPath: shouldStartBackend ? dbPath : null,
    accountId: desktop.account.id,
    workspaceId: desktop.workspace.id,
    creatorProfileId: desktop.creatorProfile.id,
    desktopDeviceId: desktopClientDeviceId,
    mobileDeviceId: mobileClientDeviceId,
    directions: [desktopToMobile, mobileToDesktop],
  };
}

async function writeImageRecordThroughBackend({
  endpoint,
  session,
  clientDeviceId,
  origin,
  title,
  eventId,
  recordId,
  payloadProtocolVersion,
  payloadBytesLength,
}) {
  const imageBytes = Buffer.from(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAFgwJ/lv6w9QAAAABJRU5ErkJggg==',
    'base64',
  );
  const protectedBytes = Buffer.from(`${origin}:${runId}:protected-copy`);
  const originalHash = `sha256:${sha256(imageBytes)}-${origin}`;
  const protectedCopyHash = `sha256:${sha256(protectedBytes)}-${origin}`;

  const reserved = await request(
    endpoint,
    'POST',
    '/v1/watermark-ids/reserve',
    {
      requestId: `${origin}-reserve-${runId}`,
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      mediaType: 'image',
      payloadProtocolVersion,
      payloadBytesLength,
      parentWatermarkUid: null,
      revision: 1,
      originalHash,
    },
    session.accessToken,
  );
  assert(reserved.status === 200, `${origin} reserve must return 200`);

  const confirmed = await request(
    endpoint,
    'POST',
    '/v1/watermark-ids/confirm',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      watermarkUid: reserved.body.watermarkUid,
      payloadProtocolVersion,
      payloadBytesLength,
      originalHash,
      protectedCopyHash,
      writeVerificationStatus: 'verified',
    },
    session.accessToken,
  );
  assert(confirmed.status === 200, `${origin} confirm must return 200`);
  assert(
    confirmed.body.watermarkUid === reserved.body.watermarkUid,
    `${origin} confirm must preserve reserved watermark uid`,
  );
  assert(
    confirmed.body.registryStatus === 'server_confirmed',
    `${origin} registry must become server_confirmed`,
  );

  const createdAt = new Date().toISOString();
  const verificationAt = new Date(Date.now() + 1000).toISOString();
  const trustedAt = new Date(Date.now() + 2000).toISOString();
  const payload = {
    id: recordId,
    kind: 'image',
    title,
    watermark_uid: confirmed.body.watermarkUid,
    revision: confirmed.body.revision,
    creator_display_name: session.creatorProfile.displayName,
    trusted_time_status: '已记录网络授时',
    trusted_time_source: 'freetsa.org',
    trusted_time_at: trustedAt,
    third_party_verification_status: '已记录网络授时',
    third_party_verification_provider: 'freetsa.org',
    third_party_verification_path: 'HiddenShield 后端 HTTP Date',
    sha256: originalHash,
    parent_watermark_uid: confirmed.body.parentWatermarkUid ?? null,
    rewrite_reason: null,
    write_verification_status: 'verified',
    write_verification_message: '完成后验证已通过，保护副本可取证。',
    write_verification_at: verificationAt,
    protected_copy_name: title.replace(/\.png$/i, '.protected.png'),
    protected_copy_hash: protectedCopyHash,
    payload_protocol_version: confirmed.body.payloadProtocolVersion,
    payload_bytes_length: confirmed.body.payloadBytesLength,
    media_payload_role: mediaPayloadRoleForProtocol(confirmed.body.payloadProtocolVersion),
    watermark_id_issue_mode: confirmed.body.watermarkIdIssueMode,
    watermark_id_registry_status: confirmed.body.registryStatus,
    watermark_id_registry_receipt: confirmed.body.registryReceipt,
    payload_auth_status: 'verified',
    output_strategy: 'minimal_required_change',
    work_source_declaration: origin === 'desktop' ? 'human_created' : 'ai_assisted',
    training_permission_declaration: 'prohibited',
    creation_method_declaration: origin === 'desktop' ? 'camera_original' : 'digital_creation',
    human_edit_level_declaration: 'minor_adjustment',
    authenticity_claim_declaration: 'creator_declared',
    custom_rights_statement: `${origin} runtime QA rights statement`,
    source: 'write',
    sync_status: 'synced',
    created_at: createdAt,
  };
  assertNoLocalMediaFields(payload, `${origin} sync payload`);

  const pushed = await request(
    endpoint,
    'POST',
    '/v1/sync/events:batch',
    {
      deviceId: clientDeviceId,
      workspaceId: session.workspace.id,
      events: [
        {
          clientEventId: eventId,
          operation: 'upsertVaultRecord',
          entityType: 'vaultRecord',
          entityId: recordId,
          payload,
        },
      ],
    },
    session.accessToken,
  );
  assert(pushed.status === 200, `${origin} sync push must return 200`);
  assert(
    pushed.body.acceptedEventIds?.includes(eventId),
    `${origin} sync push must accept event id`,
  );

  return payload;
}

function verifyDirection(direction, pulledEntity, expectedPayload) {
  assertNoLocalMediaFields(pulledEntity, `${direction} pulled entity`);
  for (const key of imageRecordKeys) {
    assert(
      Object.prototype.hasOwnProperty.call(pulledEntity, key),
      `${direction} pulled entity must include ${key}`,
    );
    assert(
      JSON.stringify(pulledEntity[key]) === JSON.stringify(expectedPayload[key]),
      `${direction} pulled entity must preserve ${key}`,
    );
  }

  const detail = buildVaultDetail(pulledEntity);
  const summary = buildCopyrightSummary(pulledEntity);
  const formalReport = buildFormalReport(pulledEntity);

  for (const [label, key] of fieldLabels) {
    const value = displayValueForKey(key, pulledEntity[key]);
    const labels = Array.isArray(label) ? label : [label];
    assert(
      labels.some((item) => detail.includes(item)),
      `${direction} detail must include ${labels.join(' / ')}`,
    );
    assert(
      labels.some((item) => summary.includes(item)),
      `${direction} summary must include ${labels.join(' / ')}`,
    );
    assert(
      labels.some((item) => formalReport.includes(item)),
      `${direction} formal report must include ${labels.join(' / ')}`,
    );
    if (value !== '无') {
      assert(
        detail.includes(value) || summary.includes(value) || formalReport.includes(value),
        `${direction} outputs must include value for ${labels.join(' / ')}`,
      );
    }
  }

  return {
    direction,
    recordId: pulledEntity.id,
    watermarkUid: pulledEntity.watermark_uid,
    registryStatus: pulledEntity.watermark_id_registry_status,
    detail,
    summary,
    formalReport,
    checkedKeys: imageRecordKeys,
  };
}

function buildVaultDetail(record) {
  return [
    `版权库详情`,
    `文件名: ${record.title}`,
    `版权编号: ${record.watermark_uid}`,
    `版本次数: 第 ${record.revision} 次`,
    `创作者身份: ${record.creator_display_name}`,
    `作品指纹: ${record.sha256}`,
    `保护副本名称: ${record.protected_copy_name}`,
    `保护副本摘要: ${record.protected_copy_hash}`,
    `输出策略: ${outputStrategyLabel(record.output_strategy)}`,
    `完成后验证: ${verificationLabel(record.write_verification_status)}`,
    `验证说明: ${record.write_verification_message}`,
    `验证时间: ${record.write_verification_at}`,
    `Payload 协议: V${record.payload_protocol_version} / ${record.payload_bytes_length} bytes`,
    `媒体载荷角色: ${mediaPayloadRoleLabel(record.media_payload_role)}`,
    `编号签发模式: ${issueModeLabel(record.watermark_id_issue_mode)}`,
    `登记状态: ${registryStatusLabel(record.watermark_id_registry_status)}`,
    `登记收据: ${record.watermark_id_registry_receipt}`,
    `Payload 认证状态: ${payloadAuthStatusLabel(record.payload_auth_status)}`,
    `第三方验证: ${record.third_party_verification_status}`,
    `可信时间: ${record.trusted_time_at}`,
    `时间来源: ${record.trusted_time_source}`,
    `作品来源声明: ${workSourceLabel(record.work_source_declaration)}`,
    `训练许可声明: ${trainingPermissionLabel(record.training_permission_declaration)}`,
    `创作方式声明: ${record.creation_method_declaration}`,
    `人工编辑声明: ${record.human_edit_level_declaration}`,
    `真实性声明: ${authenticityLabel(record.authenticity_claim_declaration)}`,
    `自定义版权声明: ${record.custom_rights_statement}`,
  ].join('\n');
}

function buildCopyrightSummary(record) {
  return [
    `【隐盾版权存证】`,
    `版权编号: ${record.watermark_uid}`,
    `版本次数: 第 ${record.revision} 次`,
    `创作者身份: ${record.creator_display_name}`,
    `完成后验证: ${verificationLabel(record.write_verification_status)}`,
    `验证说明: ${record.write_verification_message}`,
    `验证时间: ${record.write_verification_at}`,
    `Payload 协议: V${record.payload_protocol_version} / ${record.payload_bytes_length} bytes`,
    `媒体载荷角色: ${mediaPayloadRoleLabel(record.media_payload_role)}`,
    `编号签发模式: ${issueModeLabel(record.watermark_id_issue_mode)}`,
    `登记状态: ${registryStatusLabel(record.watermark_id_registry_status)}`,
    `登记收据: ${record.watermark_id_registry_receipt}`,
    `Payload 认证状态: ${payloadAuthStatusLabel(record.payload_auth_status)}`,
    `第三方验证: ${record.third_party_verification_status}`,
    `可信时间: ${record.trusted_time_at}`,
    `时间来源: ${record.trusted_time_source}`,
    `原文件: ${record.title}`,
    `作品指纹: ${record.sha256}`,
    `保护副本名称: ${record.protected_copy_name}`,
    `保护副本摘要: ${record.protected_copy_hash}`,
    `输出策略: ${outputStrategyLabel(record.output_strategy)}`,
    `处理时间: ${record.created_at}`,
    `作品来源声明: ${workSourceLabel(record.work_source_declaration)}`,
    `训练许可声明: ${trainingPermissionLabel(record.training_permission_declaration)}`,
    `创作方式声明: ${record.creation_method_declaration}`,
    `人工编辑声明: ${record.human_edit_level_declaration}`,
    `真实性声明: ${authenticityLabel(record.authenticity_claim_declaration)}`,
    `自定义版权声明: ${record.custom_rights_statement}`,
  ].join('\n');
}

function buildFormalReport(record) {
  return [
    `# HiddenShield 正式版权报告`,
    ``,
    `## 版权记录`,
    `- 文件名: ${record.title}`,
    `- 版权编号: ${record.watermark_uid}`,
    `- 版本次数: 第 ${record.revision} 次`,
    `- 创作者身份: ${record.creator_display_name}`,
    `- 作品指纹: ${record.sha256}`,
    `- 保护副本名称: ${record.protected_copy_name}`,
    `- 保护副本摘要: ${record.protected_copy_hash}`,
    `- 输出策略: ${outputStrategyLabel(record.output_strategy)}`,
    `- 完成后验证: ${verificationLabel(record.write_verification_status)}`,
    `- 验证说明: ${record.write_verification_message}`,
    `- 验证时间: ${record.write_verification_at}`,
    `- Payload 协议: V${record.payload_protocol_version} / ${record.payload_bytes_length} bytes`,
    `- 媒体载荷角色: ${mediaPayloadRoleLabel(record.media_payload_role)}`,
    `- 编号签发模式: ${issueModeLabel(record.watermark_id_issue_mode)}`,
    `- 登记状态: ${registryStatusLabel(record.watermark_id_registry_status)}`,
    `- 登记收据: ${record.watermark_id_registry_receipt}`,
    `- Payload 认证状态: ${payloadAuthStatusLabel(record.payload_auth_status)}`,
    `## 可信时间`,
    `- 第三方验证: ${record.third_party_verification_status}`,
    `- 验证服务: ${record.third_party_verification_provider}`,
    `- 验证路径: ${record.third_party_verification_path}`,
    `- 可信时间: ${record.trusted_time_status}`,
    `- 时间来源: ${record.trusted_time_source}`,
    `- 记录时间: ${record.trusted_time_at}`,
    `## 作品声明与授权策略`,
    `- 作品来源声明: ${workSourceLabel(record.work_source_declaration)}`,
    `- 训练许可声明: ${trainingPermissionLabel(record.training_permission_declaration)}`,
    `- 创作方式声明: ${record.creation_method_declaration}`,
    `- 人工编辑声明: ${record.human_edit_level_declaration}`,
    `- 真实性声明: ${authenticityLabel(record.authenticity_claim_declaration)}`,
    `- 自定义版权声明: ${record.custom_rights_statement}`,
    `## 隐私边界`,
    `- 不包含原始媒体文件`,
    `- 不包含加水印后的媒体文件`,
    `- 不包含本地媒体文件路径`,
  ].join('\n');
}

async function continueAccount(endpoint, { identifier, password, deviceId, name, platform, creatorDisplayName }) {
  const response = await request(endpoint, 'POST', '/v1/auth/sessions', {
    identifier,
    password,
    verificationCode: '000000',
    device: {
      clientDeviceId: deviceId,
      name,
      platform,
      appVersion: 'runtime-qa',
    },
    localCreatorProfile: {
      displayName: creatorDisplayName,
      creatorSeedRef: `seed-ref-${identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, `${name} auth/sessions must return 200`);
  assert(Boolean(response.body.accessToken), `${name} must return access token`);
  return response.body;
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

async function changes(endpoint, session, cursor) {
  const params = new URLSearchParams({ workspaceId: session.workspace.id });
  if (cursor) {
    params.set('cursor', cursor);
  }
  const response = await request(endpoint, 'GET', `/v1/sync/changes?${params}`, null, session.accessToken);
  assert(response.status === 200, 'changes must return 200');
  assert(Boolean(response.body.nextCursor), 'changes must return nextCursor');
  return response.body;
}

function findChange(result, entityId) {
  return result.changes?.find((change) => change.entity?.id === entityId);
}

async function request(endpoint, method, path, body, token) {
  const headers = {};
  if (body != null) {
    headers['content-type'] = 'application/json';
  }
  if (token) {
    headers.authorization = `Bearer ${token}`;
  }
  const response = await fetch(`${endpoint}${path}`, {
    method,
    headers,
    body: body == null ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let parsed;
  try {
    parsed = text ? JSON.parse(text) : {};
  } catch {
    parsed = { raw: text };
  }
  return { status: response.status, body: parsed };
}

async function waitForHealth(endpoint) {
  const deadline = Date.now() + 120_000;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const response = await request(endpoint, 'GET', '/v1/health');
      if (response.status === 200 && response.body.cloudSync === true) {
        return;
      }
      lastError = `health returned ${response.status}`;
    } catch (error) {
      lastError = error.message;
    }
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }
  throw new Error(`backend did not become healthy at ${endpoint}: ${lastError}`);
}

function renderMarkdown(result) {
  const lines = [
    `# HiddenShield 双端版权字段运行态 QA`,
    ``,
    `- Run ID: ${result.runId}`,
    `- Backend: ${result.endpoint}`,
    `- Account: ${result.accountId}`,
    `- Workspace: ${result.workspaceId}`,
    `- Creator Profile: ${result.creatorProfileId}`,
    ``,
  ];
  for (const direction of result.directions) {
    lines.push(
      `## ${direction.direction}`,
      ``,
      `- Record ID: ${direction.recordId}`,
      `- Watermark UID: ${direction.watermarkUid}`,
      `- Registry Status: ${direction.registryStatus}`,
      `- Checked Keys: ${direction.checkedKeys.length}`,
      ``,
      `### 版权库详情`,
      '```text',
      direction.detail,
      '```',
      ``,
      `### 复制摘要`,
      '```text',
      direction.summary,
      '```',
      ``,
      `### 正式报告`,
      '```markdown',
      direction.formalReport,
      '```',
      ``,
    );
  }
  return `${lines.join('\n')}\n`;
}

function assertNoLocalMediaFields(value, label) {
  const forbiddenKeys = new Set([
    'path',
    'filePath',
    'file_path',
    'localPath',
    'local_path',
    'protectedCopyPath',
    'protected_copy_path',
    'sourcePath',
    'source_path',
    'outputPath',
    'output_path',
    'originalPath',
    'original_path',
    'mediaBytes',
    'media_bytes',
    'imageBytes',
    'image_bytes',
    'originalMedia',
    'original_media',
    'outputMedia',
    'output_media',
  ]);
  const pathLikePatterns = [/^[a-zA-Z]:[\\/]/, /^file:\/\//, /^\/Users\//, /^\/home\//, /^\/tmp\//];
  visit(value, []);

  function visit(node, trail) {
    if (Array.isArray(node)) {
      node.forEach((item, index) => visit(item, [...trail, String(index)]));
      return;
    }
    if (node && typeof node === 'object') {
      for (const [key, child] of Object.entries(node)) {
        assert(!forbiddenKeys.has(key), `${label} must not contain ${key}`);
        visit(child, [...trail, key]);
      }
      return;
    }
    if (typeof node === 'string') {
      assert(
        !pathLikePatterns.some((pattern) => pattern.test(node)),
        `${label} must not contain local path value at ${trail.join('.')}`,
      );
    }
  }
}

function sha256(bytes) {
  return createHash('sha256').update(bytes).digest('hex');
}

function reportValue(value) {
  if (value == null || value === '') return '无';
  return String(value);
}

function displayValueForKey(key, value) {
  if (key === 'output_strategy') return outputStrategyLabel(value);
  if (key === 'write_verification_status') return verificationLabel(value);
  if (key === 'media_payload_role') return mediaPayloadRoleLabel(value);
  if (key === 'watermark_id_issue_mode') return issueModeLabel(value);
  if (key === 'watermark_id_registry_status') return registryStatusLabel(value);
  if (key === 'payload_auth_status') return payloadAuthStatusLabel(value);
  if (key === 'work_source_declaration') return workSourceLabel(value);
  if (key === 'training_permission_declaration') return trainingPermissionLabel(value);
  if (key === 'authenticity_claim_declaration') return authenticityLabel(value);
  if (key === 'payload_protocol_version') return `V${value}`;
  return reportValue(value);
}

function mediaPayloadRoleForProtocol(protocolVersion) {
  return Number(protocolVersion) >= 3 ? 'v3_minimal_anchor' : 'v2_full_record';
}

function mediaPayloadRoleLabel(value) {
  return {
    v2_full_record: 'V2 完整载荷',
    v3_minimal_anchor: 'V3 最小锚点',
  }[value] ?? reportValue(value);
}

function outputStrategyLabel(value) {
  return value === 'minimal_required_change' ? '最小必要变更' : reportValue(value);
}

function verificationLabel(value) {
  return value === 'verified' ? '已通过' : reportValue(value);
}

function issueModeLabel(value) {
  return {
    server_reserved: '后端预签发',
    server_confirmed: '后端已确认',
    server_reissued: '后端重新签发',
    offline_generated: '本地离线生成',
  }[value] ?? reportValue(value);
}

function registryStatusLabel(value) {
  return {
    reserved: '已预留，等待写入确认',
    server_confirmed: '后端已确认',
    offline_confirmed: '离线编号已补登记',
    pending_registration: '待联网登记',
  }[value] ?? reportValue(value);
}

function payloadAuthStatusLabel(value) {
  return value === 'verified' ? '已通过' : reportValue(value);
}

function workSourceLabel(value) {
  return {
    human_created: '人类原创',
    ai_assisted: 'AI 辅助创作',
    ai_generated: 'AI 生成',
    unspecified: '未声明',
  }[value] ?? reportValue(value);
}

function trainingPermissionLabel(value) {
  return {
    prohibited: '禁止用于模型训练',
    allowed: '允许用于模型训练',
    unspecified: '未声明',
  }[value] ?? reportValue(value);
}

function authenticityLabel(value) {
  return {
    creator_declared: '创作者声明',
    unspecified: '未声明',
  }[value] ?? reportValue(value);
}

function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const port = address.port;
      server.close(() => resolve(port));
    });
    server.on('error', reject);
  });
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(`Dual vault runtime QA failed: ${message}`);
  }
}

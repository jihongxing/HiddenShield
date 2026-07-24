import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import net from 'node:net';

const runId = process.env.HIDDENSHIELD_L3_CROSS_END_RUNTIME_QA_RUN_ID ?? `${Date.now()}`;
const endpoint = process.env.HIDDENSHIELD_CLOUD_URL?.replace(/\/$/, '') ?? null;
const shouldStartBackend = !endpoint;
const port = shouldStartBackend ? await freePort() : Number(new URL(endpoint).port || 80);
const baseUrl = endpoint ?? `http://127.0.0.1:${port}`;
const tmpRoot = join(tmpdir(), `hiddenshield-l3-cross-end-runtime-qa-${runId}`);
const dbPath = join(tmpRoot, 'cloud.sqlite');
const outputDir = join(process.cwd(), 'tmp-ui-qa', 'l3-video-visual-cross-end-runtime');
const qaJsonPath = join(outputDir, `l3-video-visual-cross-end-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `l3-video-visual-cross-end-runtime-qa-${runId}.md`);
mkdirSync(tmpRoot, { recursive: true });
mkdirSync(outputDir, { recursive: true });

const l3VaultRecordKeys = [
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
  'video_visual_task_id',
  'video_visual_completed_at',
  'video_visual_strategy_digest',
  'video_visual_self_check_confidence',
  'video_visual_self_check_threshold',
  'video_visual_checked_frames',
  'video_visual_media_hash',
  'video_visual_receipt_hash',
  'video_visual_output_bytes',
  'video_visual_output_content_type',
  'source',
  'sync_status',
  'created_at',
];

const l3ReceiptKeys = [
  'video_visual_task_id',
  'video_visual_completed_at',
  'video_visual_strategy_digest',
  'video_visual_self_check_confidence',
  'video_visual_self_check_threshold',
  'video_visual_checked_frames',
  'video_visual_media_hash',
  'video_visual_receipt_hash',
  'video_visual_output_bytes',
  'video_visual_output_content_type',
];

const l3FieldLabels = [
  ['L3 视频画面盲水印', null],
  [['任务编号', 'L3 任务'], 'video_visual_task_id'],
  ['完成时间', 'video_visual_completed_at'],
  ['策略摘要', 'video_visual_strategy_digest'],
  ['自检置信度', 'video_visual_self_check_confidence'],
  ['自检阈值', 'video_visual_self_check_threshold'],
  ['检查帧数', 'video_visual_checked_frames'],
  [['成品摘要', '成品媒体摘要'], 'video_visual_media_hash'],
  [['Worker 收据', 'Worker 收据摘要'], 'video_visual_receipt_hash'],
  ['成品字节数', 'video_visual_output_bytes'],
  ['成品内容类型', 'video_visual_output_content_type'],
];

let backend;
try {
  if (shouldStartBackend) {
    backend = spawn(
      command('cargo'),
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
  console.log('L3 video visual cross-end runtime QA OK');
  console.log(`QA JSON: ${qaJsonPath}`);
  console.log(`QA Markdown: ${qaMdPath}`);
} finally {
  if (backend && !backend.killed) {
    backend.kill();
  }
}

async function runQa(endpoint) {
  const identifier = `l3-cross-end-runtime-${runId}@example.com`;
  const password = 'l3-cross-end-runtime-password';
  const creatorDisplayName = 'L3 Cross-End Runtime Creator';
  const desktopClientDeviceId = `desktop-l3-runtime-${runId}`;
  const mobileClientDeviceId = `mobile-l3-runtime-${runId}`;

  const desktop = await continueAccount(endpoint, {
    identifier,
    password,
    deviceId: desktopClientDeviceId,
    name: 'L3 Runtime QA Desktop',
    platform: 'windows',
    creatorDisplayName,
  });
  const mobile = await continueAccount(endpoint, {
    identifier,
    password,
    deviceId: mobileClientDeviceId,
    name: 'L3 Runtime QA Mobile',
    platform: 'android',
    creatorDisplayName,
  });
  await upgradeToStudio(endpoint, desktop);

  assert(desktop.account.id === mobile.account.id, 'desktop and mobile must share one account');
  assert(desktop.workspace.id === mobile.workspace.id, 'desktop and mobile must share one workspace');
  assert(
    desktop.creatorProfile.id === mobile.creatorProfile.id,
    'desktop and mobile must share one creator profile',
  );

  const desktopBaseline = await changes(endpoint, desktop);
  const mobileBaseline = await changes(endpoint, mobile);

  const desktopPayload = await writeL3RecordThroughBackend({
    endpoint,
    session: desktop,
    clientDeviceId: desktopClientDeviceId,
    origin: 'desktop',
    eventId: `desktop-l3-event-${runId}`,
    recordId: `desktop-l3-record-${runId}`,
    title: `desktop-l3-${runId}.l3-watermarked.mp4`,
    confidence: 0.982451,
    threshold: 0.95,
    checkedFrames: 12,
    outputBytes: 234567,
  });
  const mobilePulled = await changes(endpoint, mobile, mobileBaseline.nextCursor);
  const desktopToMobileChange = findChange(mobilePulled, desktopPayload.id);
  assert(Boolean(desktopToMobileChange), 'mobile must pull desktop-written L3 video visual record');
  assert(
    desktopToMobileChange.sourceDevice === desktopClientDeviceId,
    'mobile pull must identify desktop source device',
  );
  const desktopToMobile = verifyDirection({
    direction: 'desktop->mobile',
    reader: 'mobile',
    pulledEntity: desktopToMobileChange.entity,
    expectedPayload: desktopPayload,
  });

  const mobilePayload = await writeL3RecordThroughBackend({
    endpoint,
    session: mobile,
    clientDeviceId: mobileClientDeviceId,
    origin: 'mobile',
    eventId: `mobile-l3-event-${runId}`,
    recordId: `mobile-l3-record-${runId}`,
    title: `mobile-l3-${runId}.l3-watermarked.mp4`,
    confidence: 0.991337,
    threshold: 0.97,
    checkedFrames: 16,
    outputBytes: 345678,
  });
  const desktopPulled = await changes(endpoint, desktop, desktopBaseline.nextCursor);
  const mobileToDesktopChange = findChange(desktopPulled, mobilePayload.id);
  assert(Boolean(mobileToDesktopChange), 'desktop must pull mobile-written L3 video visual record');
  assert(
    mobileToDesktopChange.sourceDevice === mobileClientDeviceId,
    'desktop pull must identify mobile source device',
  );
  const mobileToDesktop = verifyDirection({
    direction: 'mobile->desktop',
    reader: 'desktop',
    pulledEntity: mobileToDesktopChange.entity,
    expectedPayload: mobilePayload,
  });

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

async function writeL3RecordThroughBackend({
  endpoint,
  session,
  clientDeviceId,
  origin,
  eventId,
  recordId,
  title,
  confidence,
  threshold,
  checkedFrames,
  outputBytes,
}) {
  const originalHash = `sha256:${sha256Text(`${origin}:${runId}:l3-source-video`)}`;
  const mediaHash = `sha256:${sha256Text(`${origin}:${runId}:l3-watermarked-mp4`)}`;
  const strategyDigest = `sha256:${sha256Text(`${origin}:${runId}:l3-strategy`)}`;
  const workerReceiptHash = `sha256:${sha256Text(`${origin}:${runId}:l3-worker-receipt`)}`;

  const reserved = await request(
    endpoint,
    'POST',
    '/v1/watermark-ids/reserve',
    {
      requestId: `${origin}-l3-reserve-${runId}`,
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      mediaType: 'video_visual',
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      parentWatermarkUid: null,
      revision: 1,
      originalHash,
    },
    session.accessToken,
  );
  assert(reserved.status === 200, `${origin} video_visual reserve must return 200`);
  assert(reserved.body.watermarkUid?.startsWith('HS-'), `${origin} reserve must return HS uid`);

  const confirmed = await request(
    endpoint,
    'POST',
    '/v1/watermark-ids/confirm',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      watermarkUid: reserved.body.watermarkUid,
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      originalHash,
      protectedCopyHash: mediaHash,
      writeVerificationStatus: 'verified',
    },
    session.accessToken,
  );
  assert(confirmed.status === 200, `${origin} video_visual confirm must return 200`);
  assert(confirmed.body.watermarkUid === reserved.body.watermarkUid, `${origin} confirm must preserve uid`);
  assert(confirmed.body.registryStatus === 'server_confirmed', `${origin} registry must be server_confirmed`);

  const createdAt = new Date().toISOString();
  const completedAt = new Date(Date.now() + 1000).toISOString();
  const verificationAt = new Date(Date.now() + 2000).toISOString();
  const trustedAt = new Date(Date.now() + 3000).toISOString();
  const payload = {
    id: recordId,
    kind: 'video',
    title,
    watermark_uid: confirmed.body.watermarkUid,
    revision: confirmed.body.revision,
    creator_display_name: session.creatorProfile.displayName,
    trusted_time_status: '已记录网络授时',
    trusted_time_source: 'HiddenShield backend runtime QA',
    trusted_time_at: trustedAt,
    third_party_verification_status: '已记录网络授时',
    third_party_verification_provider: 'HiddenShield backend runtime QA',
    third_party_verification_path: 'HiddenShield 后端 HTTP Date',
    sha256: originalHash,
    parent_watermark_uid: confirmed.body.parentWatermarkUid ?? null,
    rewrite_reason: null,
    write_verification_status: 'verified',
    write_verification_message: 'L3 succeeded task 自检收据已通过，成品摘要与 worker receipt 已固化。',
    write_verification_at: verificationAt,
    protected_copy_name: title,
    protected_copy_hash: mediaHash,
    payload_protocol_version: confirmed.body.payloadProtocolVersion,
    payload_bytes_length: confirmed.body.payloadBytesLength,
    media_payload_role: 'v2_full_record',
    watermark_id_issue_mode: confirmed.body.watermarkIdIssueMode,
    watermark_id_registry_status: confirmed.body.registryStatus,
    watermark_id_registry_receipt: confirmed.body.registryReceipt,
    payload_auth_status: 'verified',
    output_strategy: 'cloud_l3_video_visual_watermark',
    work_source_declaration: 'unspecified',
    training_permission_declaration: 'prohibited',
    creation_method_declaration: 'unspecified',
    human_edit_level_declaration: 'unspecified',
    authenticity_claim_declaration: 'creator_declared',
    custom_rights_statement: `${origin} L3 cross-end runtime QA receipt metadata only`,
    video_visual_task_id: `l3task_${origin}_${runId}`,
    video_visual_completed_at: completedAt,
    video_visual_strategy_digest: strategyDigest,
    video_visual_self_check_confidence: confidence,
    video_visual_self_check_threshold: threshold,
    video_visual_checked_frames: checkedFrames,
    video_visual_media_hash: mediaHash,
    video_visual_receipt_hash: workerReceiptHash,
    video_visual_output_bytes: outputBytes,
    video_visual_output_content_type: 'video/mp4',
    source: 'write',
    sync_status: 'synced',
    created_at: createdAt,
  };
  assertNoLocalMediaFields(payload, `${origin} L3 sync payload`);

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
  assert(pushed.status === 200, `${origin} L3 sync push must return 200`);
  assert(pushed.body.acceptedEventIds?.includes(eventId), `${origin} L3 sync push must accept event id`);

  return payload;
}

function verifyDirection({ direction, reader, pulledEntity, expectedPayload }) {
  assertNoLocalMediaFields(pulledEntity, `${direction} pulled entity`);
  for (const key of l3VaultRecordKeys) {
    assert(
      Object.prototype.hasOwnProperty.call(pulledEntity, key),
      `${direction} pulled entity must include ${key}`,
    );
    assert(
      JSON.stringify(pulledEntity[key]) === JSON.stringify(expectedPayload[key]),
      `${direction} pulled entity must preserve ${key}`,
    );
  }

  assert(pulledEntity.kind === 'video', `${direction} pulled entity must remain video kind`);
  assert(
    pulledEntity.output_strategy === 'cloud_l3_video_visual_watermark',
    `${direction} pulled entity must keep L3 output strategy`,
  );
  assert(
    pulledEntity.video_visual_self_check_confidence >= pulledEntity.video_visual_self_check_threshold,
    `${direction} pulled receipt must keep confidence >= threshold`,
  );
  assert(pulledEntity.video_visual_checked_frames > 0, `${direction} pulled receipt must keep checked frames`);

  const detail = buildVaultDetail(reader, pulledEntity);
  const formalReport = buildFormalReport(reader, pulledEntity);
  for (const [label, key] of l3FieldLabels) {
    const labels = Array.isArray(label) ? label : [label];
    assert(
      labels.some((item) => detail.includes(item)),
      `${direction} ${reader} vault detail must include ${labels.join(' / ')}`,
    );
    assert(
      labels.some((item) => formalReport.includes(item)),
      `${direction} ${reader} formal report must include ${labels.join(' / ')}`,
    );
    if (key) {
      const value = displayValueForKey(key, pulledEntity[key], reader);
      assert(detail.includes(value), `${direction} ${reader} vault detail must include value for ${key}`);
      assert(formalReport.includes(value), `${direction} ${reader} formal report must include value for ${key}`);
    }
  }

  return {
    direction,
    reader,
    recordId: pulledEntity.id,
    watermarkUid: pulledEntity.watermark_uid,
    registryStatus: pulledEntity.watermark_id_registry_status,
    taskId: pulledEntity.video_visual_task_id,
    checkedReceiptKeys: l3ReceiptKeys,
    checkedVaultKeys: l3VaultRecordKeys,
    detail,
    formalReport,
  };
}

function buildVaultDetail(reader, record) {
  const confidence = displayNumber(record.video_visual_self_check_confidence, reader);
  const threshold = displayNumber(record.video_visual_self_check_threshold, reader);
  const frames = String(record.video_visual_checked_frames);
  if (reader === 'desktop') {
    return [
      '版权库详情',
      'L3 视频画面盲水印',
      `L3 任务: ${record.video_visual_task_id}`,
      `L3 完成时间: ${record.video_visual_completed_at}`,
      `L3 策略摘要: ${record.video_visual_strategy_digest}`,
      `L3 自检置信度: ${confidence}`,
      `L3 自检阈值: ${threshold}`,
      `L3 检查帧数: ${frames}`,
      `L3 成品摘要: ${record.video_visual_media_hash}`,
      `L3 Worker 收据: ${record.video_visual_receipt_hash}`,
      `L3 成品字节数: ${record.video_visual_output_bytes}`,
      `L3 成品内容类型: ${record.video_visual_output_content_type}`,
    ].join('\n');
  }
  return [
    '版权库详情',
    'L3 视频画面盲水印',
    `任务编号: ${record.video_visual_task_id}`,
    `完成时间: ${record.video_visual_completed_at}`,
    `策略摘要: ${record.video_visual_strategy_digest}`,
    `自检置信度: ${confidence}`,
    `自检阈值: ${threshold}`,
    `检查帧数: ${frames}`,
    `成品摘要: ${record.video_visual_media_hash}`,
    `Worker 收据: ${record.video_visual_receipt_hash}`,
    `成品字节数: ${record.video_visual_output_bytes}`,
    `成品内容类型: ${record.video_visual_output_content_type}`,
  ].join('\n');
}

function buildFormalReport(reader, record) {
  const confidence = displayNumber(record.video_visual_self_check_confidence, reader);
  const threshold = displayNumber(record.video_visual_self_check_threshold, reader);
  return [
    '# HiddenShield 正式版权报告',
    '',
    '## 版权记录',
    `- 文件名: ${record.title}`,
    `- 版权编号: ${record.watermark_uid}`,
    `- 保护副本摘要: ${record.protected_copy_hash}`,
    '',
    reader === 'desktop' ? '### L3 视频画面盲水印' : '## L3 视频画面盲水印',
    '',
    `- 任务编号: ${record.video_visual_task_id}`,
    `- 完成时间: ${record.video_visual_completed_at}`,
    `- 策略摘要: ${record.video_visual_strategy_digest}`,
    `- 自检置信度: ${confidence}`,
    `- 自检阈值: ${threshold}`,
    `- 检查帧数: ${record.video_visual_checked_frames}`,
    `- 成品媒体摘要: ${record.video_visual_media_hash}`,
    `- Worker 收据摘要: ${record.video_visual_receipt_hash}`,
    `- 成品字节数: ${record.video_visual_output_bytes}`,
    `- 成品内容类型: ${record.video_visual_output_content_type}`,
    '',
    '## 隐私边界',
    '- 不包含原始媒体文件',
    '- 不包含加水印后的媒体文件',
    '- 不包含本地媒体文件路径',
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
      appVersion: 'l3-cross-end-runtime-qa',
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

async function upgradeToStudio(endpoint, session) {
  if (session.entitlement?.features?.cloud_sync === true && session.entitlement?.features?.cloud_video_processing === true) {
    return;
  }
  const payment = await request(
    endpoint,
    'POST',
    '/v1/billing/payment-sessions',
    {
      accountId: session.account.id,
      workspaceId: session.workspace.id,
      planCode: 'studio',
      billingCycle: 'monthly',
      preferredProvider: 'fixture',
    },
    session.accessToken,
  );
  assert(payment.status === 200, 'fixture Studio payment session must succeed');
  const webhook = await request(
    endpoint,
    'POST',
    '/v1/billing/webhooks/fixture',
    {
      providerEventId: `fixture-l3-cross-end-${runId}`,
      providerOrderId: payment.body.providerOrderId,
      providerTransactionId: `fixture-l3-cross-end-txn-${runId}`,
      accountId: session.account.id,
      workspaceId: session.workspace.id,
      planCode: 'studio',
      billingCycle: 'monthly',
      amountCents: 6900,
      currency: 'CNY',
      eventType: 'payment.succeeded',
      occurredAt: new Date().toISOString(),
      rawPayloadJson: {
        provider: 'fixture',
        eventType: 'payment.succeeded',
        providerOrderId: payment.body.providerOrderId,
      },
    },
    session.accessToken,
  );
  assert(webhook.status === 200, 'fixture Studio webhook must succeed');
  const entitlement = await request(endpoint, 'GET', '/v1/entitlements/current', null, session.accessToken);
  assert(entitlement.status === 200, 'Studio entitlement refresh must return 200');
  assert(entitlement.body.features?.cloud_sync === true, 'Studio entitlement must enable cloud_sync');
  assert(
    entitlement.body.features?.cloud_video_processing === true,
    'Studio entitlement must enable cloud_video_processing',
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
    '# HiddenShield L3 视频画面盲水印双端运行态同步 QA',
    '',
    `- Run ID: ${result.runId}`,
    `- Backend: ${result.endpoint}`,
    `- Account: ${result.accountId}`,
    `- Workspace: ${result.workspaceId}`,
    `- Creator Profile: ${result.creatorProfileId}`,
    `- Desktop Device: ${result.desktopDeviceId}`,
    `- Mobile Device: ${result.mobileDeviceId}`,
    '',
  ];
  for (const direction of result.directions) {
    lines.push(
      `## ${direction.direction}`,
      '',
      `- Opposite Endpoint Reader: ${direction.reader}`,
      `- Record ID: ${direction.recordId}`,
      `- Watermark UID: ${direction.watermarkUid}`,
      `- Registry Status: ${direction.registryStatus}`,
      `- L3 Task ID: ${direction.taskId}`,
      `- Checked Receipt Keys: ${direction.checkedReceiptKeys.join(', ')}`,
      '',
      '### 版权库详情',
      '```text',
      direction.detail,
      '```',
      '',
      '### 正式报告',
      '```markdown',
      direction.formalReport,
      '```',
      '',
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
    'videoBytes',
    'video_bytes',
    'originalMedia',
    'original_media',
    'outputMedia',
    'output_media',
    'outputMediaStorageRef',
    'output_media_storage_ref',
    'signedDownloadUrl',
    'signed_download_url',
    'signedUploadUrl',
    'signed_upload_url',
    'downloadToken',
    'download_token',
    'uploadToken',
    'upload_token',
  ]);
  const forbiddenStringPatterns = [
    /^[a-zA-Z]:[\\/]/,
    /^file:\/\//,
    /^\/Users\//,
    /^\/home\//,
    /^\/tmp\//,
    /^object:\/\/l3-(upload|output)\//,
    /output-download\?token=/,
    /video-object-store\/upload\?token=/,
    /hs_l3(dl|up)_v1\./,
  ];
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
        !forbiddenStringPatterns.some((pattern) => pattern.test(node)),
        `${label} must not contain forbidden media/object/signed value at ${trail.join('.')}`,
      );
    }
  }
}

function displayValueForKey(key, value, reader) {
  if (key === 'video_visual_self_check_confidence' || key === 'video_visual_self_check_threshold') {
    return displayNumber(value, reader);
  }
  return String(value);
}

function displayNumber(value, reader) {
  const number = Number(value);
  if (reader === 'mobile') {
    return number.toFixed(6);
  }
  return String(number);
}

function sha256Text(value) {
  return createHash('sha256').update(value).digest('hex');
}

function freePort() {
  return new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const selectedPort = address.port;
      server.close(() => resolve(selectedPort));
    });
    server.on('error', reject);
  });
}

function command(name) {
  if (process.platform !== 'win32') {
    return name;
  }
  if (name === 'cargo') {
    return 'cargo.exe';
  }
  return name;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(`L3 video visual cross-end runtime QA failed: ${message}`);
  }
}

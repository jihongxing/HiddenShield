import { createHash, createHmac } from 'node:crypto';

const endpoint = (process.env.HIDDENSHIELD_CLOUD_URL ?? 'http://127.0.0.1:43188').replace(/\/$/, '');
const runId = process.env.HIDDENSHIELD_CLOUD_VIDEO_E2E_RUN_ID ?? `${Date.now()}`;
const identifier = process.env.HIDDENSHIELD_CLOUD_IDENTIFIER ?? `video-e2e-${runId}@example.com`;
const password = process.env.HIDDENSHIELD_CLOUD_PASSWORD ?? 'video-e2e-password';
const adminToken = process.env.HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN ?? 'cloud-video-ci-admin-token';
const deviceId = `video-e2e-device-${runId}`;

console.log(`HiddenShield cloud video L2 E2E: ${endpoint}`);
console.log(`identifier: ${identifier}`);

const health = await request('GET', '/v1/health');
assert(health.status === 200, 'health endpoint must return 200');

const session = await continueAccount();
await assertRejectedBoundaries(session);
await assertSuccessfulNotary(session);
await assertCloudVideoTaskFlow(session);

console.log('Cloud video L2 E2E OK');

async function continueAccount() {
  const response = await request('POST', '/v1/auth/sessions', {
    identifier,
    password,
    verificationCode: '000000',
    device: {
      clientDeviceId: deviceId,
      name: 'Video E2E Device',
      platform: 'contract',
      appVersion: 'video-e2e-test',
    },
    localCreatorProfile: {
      displayName: 'Video E2E Creator',
      creatorSeedRef: `video-seed-ref-${identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, 'auth/sessions must return 200');
  assert(Boolean(response.body.accessToken), 'auth/sessions must return accessToken');
  assert(Boolean(response.body.workspace?.id), 'auth/sessions must return workspace.id');
  assert(Boolean(response.body.creatorProfile?.id), 'auth/sessions must return creatorProfile.id');
  return response.body;
}

async function assertRejectedBoundaries(session) {
  const base = notaryRequest(session);

  const missingToken = await request('POST', '/v1/video-fingerprints/notaries', base);
  assert(missingToken.status === 401, 'L2 notary without token must return 401');

  const originalVideo = structuredClone(base);
  originalVideo.uploadManifest.containsOriginalVideo = true;
  const originalVideoResponse = await request(
    'POST',
    '/v1/video-fingerprints/notaries',
    originalVideo,
    session.accessToken,
  );
  assert(originalVideoResponse.status === 400, 'L2 notary with original video must return 400');
  assert(
    originalVideoResponse.body.message === 'original_video_forbidden',
    'L2 notary original video rejection must use original_video_forbidden',
  );

  const localPath = structuredClone(base);
  localPath.uploadManifest.containsLocalPaths = true;
  const localPathResponse = await request(
    'POST',
    '/v1/video-fingerprints/notaries',
    localPath,
    session.accessToken,
  );
  assert(localPathResponse.status === 400, 'L2 notary with local paths must return 400');
  assert(
    localPathResponse.body.message === 'local_path_forbidden',
    'L2 notary local path rejection must use local_path_forbidden',
  );

  const missingCrop = structuredClone(base);
  missingCrop.cropWindowFingerprintRoot = '';
  missingCrop.cropWindowCount = 0;
  const missingCropResponse = await request(
    'POST',
    '/v1/video-fingerprints/notaries',
    missingCrop,
    session.accessToken,
  );
  assert(missingCropResponse.status === 400, 'L2 notary without crop windows must return 400');
  assert(
    missingCropResponse.body.message === 'crop_windows_required',
    'L2 notary crop rejection must use crop_windows_required',
  );

  const wrongWorkspace = structuredClone(base);
  wrongWorkspace.workspaceId = `${session.workspace.id}-other`;
  const wrongWorkspaceResponse = await request(
    'POST',
    '/v1/video-fingerprints/notaries',
    wrongWorkspace,
    session.accessToken,
  );
  assert(wrongWorkspaceResponse.status === 403, 'L2 notary with wrong workspace must return 403');

  console.log('L2 notary rejection checks passed');
}

async function assertSuccessfulNotary(session) {
  const payload = notaryRequest(session);
  assertNoLocalMediaFields(payload, 'successful L2 request');
  const response = await request(
    'POST',
    '/v1/video-fingerprints/notaries',
    payload,
    session.accessToken,
  );
  assert(response.status === 200, 'L2 notary success must return 200');
  assert(response.body.schemaVersion === 'video_fingerprint_notary_receipt_v1', 'receipt schema must match');
  assert(Boolean(response.body.notaryId), 'receipt must return notaryId');
  assert(response.body.watermarkUid === payload.watermarkUid, 'receipt must preserve watermarkUid');
  assert(response.body.sourceHash === payload.sourceHash, 'receipt must preserve sourceHash');
  assert(response.body.fingerprintRoot === payload.fingerprintRoot, 'receipt must preserve fingerprintRoot');
  assert(Boolean(response.body.serverReceiptSignature), 'receipt must include serverReceiptSignature');
  assert(Boolean(response.body.usageLedgerId), 'receipt must include usageLedgerId');
  assertNoLocalMediaFields(response.body, 'successful L2 receipt');
  console.log(`L2 notary receipt: ${response.body.notaryId}`);
}

async function assertCloudVideoTaskFlow(session) {
  const base = cloudVideoTaskRequest(session);
  const missingToken = await request('POST', '/v1/video-tasks', base);
  assert(missingToken.status === 401, 'L3 task without token must return 401');

  const forbidden = await request('POST', '/v1/video-tasks', base, session.accessToken);
  assert(forbidden.status === 403, 'L3 task without entitlement must return 403');

  const entitle = await request(
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
  assert(entitle.status === 200, 'studio payment session must return 200');
  await request('POST', '/v1/billing/webhooks/fixture', {
    providerEventId: `fixture-l3-${runId}`,
    providerOrderId: entitle.body.providerOrderId,
    providerTransactionId: `fixture-l3-txn-${runId}`,
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
      providerOrderId: entitle.body.providerOrderId,
    },
  }, session.accessToken);

  const entitlement = await request('GET', '/v1/entitlements/current', null, session.accessToken);
  assert(entitlement.status === 200, 'entitlement refresh must return 200');
  assert(
    entitlement.body.features?.cloud_video_processing === true,
    'studio entitlement must enable cloud_video_processing',
  );

  const created = await request('POST', '/v1/video-tasks', base, session.accessToken);
  assert(created.status === 200, 'L3 task creation must return 200 after entitlement upgrade');
  assert(created.body.schemaVersion === 'cloud_video_task_v1', 'L3 task schema must match');
  assert(created.body.status === 'draft', 'L3 task must start in draft');

  const listed = await request(
    'GET',
    `/v1/video-tasks?workspaceId=${encodeURIComponent(session.workspace.id)}&status=draft&limit=10`,
    null,
    session.accessToken,
  );
  assert(listed.status === 200, 'L3 task list must return 200');
  assert(Array.isArray(listed.body.tasks) && listed.body.tasks.length >= 1, 'L3 task list must include the created task');

  const fetched = await request(
    'GET',
    `/v1/video-tasks/${created.body.taskId}`,
    null,
    session.accessToken,
  );
  assert(fetched.status === 200, 'L3 task fetch must return 200');
  assert(fetched.body.taskId === created.body.taskId, 'L3 task fetch must round-trip taskId');

  const failed = await request(
    'PATCH',
    `/v1/video-tasks/${created.body.taskId}/status`,
    {
      status: 'failed',
      failureCode: 'server_rejected',
    },
    session.accessToken,
  );
  assert(failed.status === 200, 'L3 task failed update must return 200');
  assert(failed.body.status === 'failed', 'L3 task must transition to failed');
  assert(failed.body.usageLedgerId == null, 'failed L3 task must not charge usage');

  const second = await request('POST', '/v1/video-tasks', base, session.accessToken);
  assert(second.status === 200, 'second L3 task creation must return 200');
  const claim = await claimL3Task('worker-cloud-video-e2e');
  assert(claim.body.task.taskId === second.body.taskId, 'worker claim must return the second L3 task');
  assert(claim.body.task.status === 'running', 'worker claim must mark task running');
  assert(claim.body.task.attemptCount === 1, 'first worker claim must start attempt count');
  assert(Boolean(claim.body.leaseToken), 'worker claim must return a one-time lease token');
  const forgedCompletion = await request(
    'PATCH',
    `/v1/video-tasks/${second.body.taskId}/status`,
    {
      status: 'succeeded',
      strategyDigest: 'sha256:strategy',
      selfCheckThreshold: 0.9,
      selfCheckConfidence: 0.95,
      checkedFrames: 8,
      watermarkedMediaHash: `sha256:watermarked-l3-${runId}`,
      serverReceiptSignature: 'sig:server-receipt',
    },
    session.accessToken,
  );
  assert(forgedCompletion.status === 400, 'user bearer L3 succeeded update must be rejected');
  const completion = {
    strategyDigest: 'sha256:strategy',
    selfCheckThreshold: 0.9,
    selfCheckConfidence: 0.95,
    checkedFrames: 8,
    watermarkedMediaHash: `sha256:watermarked-l3-${runId}`,
    ...workerReceiptFields(second.body.taskId, `sha256:watermarked-l3-${runId}`),
    serverReceiptSignature: '',
    workerId: claim.body.workerId,
    attemptId: claim.body.attemptId,
    leaseToken: claim.body.leaseToken,
  };
  completion.serverReceiptSignature = l3CompletionSignature(adminToken, second.body.taskId, completion);
  const succeeded = await request(
    'POST',
    `/internal/video-tasks/${second.body.taskId}/completion`,
    completion,
    adminToken,
  );
  assert(succeeded.status === 200, 'L3 task succeeded update must return 200');
  assert(succeeded.body.status === 'succeeded', 'L3 task must transition to succeeded');
  assert(succeeded.body.selfCheckConfidence >= succeeded.body.selfCheckThreshold, 'succeeded L3 task must persist a passing watermark-core self-check');
  assert(succeeded.body.checkedFrames >= 1, 'succeeded L3 task must persist checked frame count');
  assert(Boolean(succeeded.body.strategyDigest), 'succeeded L3 task must persist strategy digest');
  assert(Boolean(succeeded.body.watermarkedMediaHash), 'succeeded L3 task must persist watermarked media hash');
  assert(succeeded.body.outputMediaStorageRef === completion.outputMediaStorageRef, 'succeeded L3 task must persist output media ref');
  assert(succeeded.body.workerReceiptHash === completion.workerReceiptHash, 'succeeded L3 task must persist worker receipt hash');
  assert(succeeded.body.workerReceipt?.schemaVersion === 'l3_worker_receipt_v1', 'succeeded L3 task must persist worker receipt');
  assert(Boolean(succeeded.body.usageLedgerId), 'succeeded L3 task must charge usage');
  assert(succeeded.body.workerId === claim.body.workerId, 'succeeded L3 task must preserve worker id');
  assert(succeeded.body.attemptId === claim.body.attemptId, 'succeeded L3 task must preserve attempt id');
}

async function claimL3Task(workerId) {
  const response = await request(
    'POST',
    '/internal/video-tasks/claim',
    {
      workerId,
      capabilityLevel: 'hybrid_visual_watermark',
      leaseSeconds: 900,
    },
    adminToken,
  );
  assert(response.status === 200, 'internal worker claim must return 200');
  return response;
}

function l3CompletionSignature(secret, taskId, completion) {
  const payload = [
    'hidden-shield:l3-completion:v1',
    taskId.trim(),
    completion.strategyDigest.trim(),
    completion.selfCheckThreshold.toFixed(6),
    completion.selfCheckConfidence.toFixed(6),
    String(completion.checkedFrames),
    completion.watermarkedMediaHash.trim(),
    completion.outputMediaStorageRef.trim(),
    String(completion.outputMediaBytes),
    completion.outputMediaContentType.trim(),
    completion.workerReceiptHash.trim(),
    completion.workerId.trim(),
    completion.attemptId.trim(),
    completion.leaseToken.trim(),
  ].join('\n');
  const digest = createHmac('sha256', secret).update(payload).digest('hex');
  return `hmac-sha256:l3-completion-v1:${digest}`;
}

function workerReceiptFields(taskId, mediaHash) {
  const workerReceipt = {
    algorithmSource: 'watermark-core',
    output: {
      bytes: 4096,
      contentType: 'video/mp4',
      sha256: mediaHash,
      storageRef: `object://l3-output/${taskId}/fixture.mp4`,
    },
    schemaVersion: 'l3_worker_receipt_v1',
    taskId,
    workerId: 'worker-cloud-video-e2e',
  };
  return {
    outputMediaStorageRef: workerReceipt.output.storageRef,
    outputMediaBytes: workerReceipt.output.bytes,
    outputMediaContentType: workerReceipt.output.contentType,
    workerReceipt,
    workerReceiptHash: sha256Text(stableStringify(workerReceipt)),
  };
}

function stableStringify(value) {
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

function sha256Text(text) {
  return `sha256:${createHash('sha256').update(text).digest('hex')}`;
}

function notaryRequest(session) {
  return {
    schemaVersion: 'video_fingerprint_notary_request_v1',
    workspaceId: session.workspace.id,
    creatorProfileId: session.creatorProfile.id,
    watermarkUid: `wm-video-e2e-${runId}`,
    sourceHash: `sha256:source-${runId}`,
    durationMs: 125000,
    frameSamplePolicy: 'uniform_8_frames_v1',
    sceneCount: 8,
    fingerprintSchemaVersion: 'video_fingerprint_v1',
    globalFrameFingerprints: [
      {
        sceneIndex: 0,
        timestampMs: 1000,
        phash: '0000000000000001',
        colorHash: '0000000000000002',
        edgeHash: '0000000000000003',
        motionSummary: 'static-frame-v1',
      },
    ],
    localBlockFingerprintRoot: `sha256:local-block-root-${runId}`,
    localBlockCount: 912,
    cropWindowFingerprintRoot: `sha256:crop-window-root-${runId}`,
    cropWindowCount: 56,
    fingerprintRoot: `sha256:fingerprint-root-${runId}`,
    clientSignature: `ed25519:client-signature-${runId}`,
    uploadManifest: {
      schemaVersion: 'video_upload_manifest_v1',
      containsOriginalVideo: false,
      containsWatermarkedVideo: false,
      containsLocalPaths: false,
      containsProxy: false,
      items: [
        {
          kind: 'video_fingerprint_bundle',
          sha256: `sha256:bundle-${runId}`,
          bytes: 48212,
        },
      ],
    },
  };
}

function cloudVideoTaskRequest(session) {
  return {
    schemaVersion: 'cloud_video_task_v1',
    workspaceId: session.workspace.id,
    creatorProfileId: session.creatorProfile.id,
    capabilityLevel: 'hybrid_visual_watermark',
    watermarkUid: `wm-video-l3-${runId}`,
    sourceHash: `sha256:source-l3-${runId}`,
    durationMs: 125000,
    targetProfiles: ['douyin_9_16_h264_high_crf18_720p'],
    uploadManifest: {
      schemaVersion: 'video_upload_manifest_v1',
      containsOriginalVideo: false,
      containsWatermarkedVideo: false,
      containsLocalPaths: false,
      containsProxy: false,
      items: [
        {
          kind: 'video_fingerprint_bundle',
          sha256: `sha256:bundle-l3-${runId}`,
          bytes: 48212,
        },
      ],
    },
  };
}

async function request(method, path, body, token) {
  const headers = {};
  if (body != null) {
    headers['content-type'] = 'application/json';
  }
  if (token) {
    headers.authorization = `Bearer ${token}`;
  }

  let response;
  try {
    response = await fetch(`${endpoint}${path}`, {
      method,
      headers,
      body: body == null ? undefined : JSON.stringify(body),
    });
  } catch (error) {
    console.error(`Cannot reach ${endpoint}${path}: ${error}`);
    console.error('Start the cloud backend with: npm run cloud:backend');
    process.exit(1);
  }

  const text = await response.text();
  let parsed;
  try {
    parsed = text ? JSON.parse(text) : {};
  } catch {
    parsed = { raw: text };
  }
  return { status: response.status, body: parsed };
}

function assertNoLocalMediaFields(value, label) {
  const forbiddenKeys = new Set([
    'path',
    'filePath',
    'file_path',
    'localPath',
    'local_path',
    'sourcePath',
    'source_path',
    'outputPath',
    'output_path',
    'originalPath',
    'original_path',
    'videoBytes',
    'video_bytes',
    'originalVideo',
    'original_video',
    'watermarkedVideo',
    'watermarked_video',
  ]);
  const pathLikePatterns = [
    /^[a-zA-Z]:[\\/]/,
    /^file:\/\//,
    /^\/Users\//,
    /^\/home\//,
    /^\/var\//,
    /^\/tmp\//,
  ];

  visit(value, []);

  function visit(node, trail) {
    if (Array.isArray(node)) {
      node.forEach((item, index) => visit(item, [...trail, String(index)]));
      return;
    }
    if (node && typeof node === 'object') {
      for (const [key, child] of Object.entries(node)) {
        const nextTrail = [...trail, key];
        assert(
          !forbiddenKeys.has(key),
          `${label} must not contain local/media field: ${nextTrail.join('.')}`,
        );
        visit(child, nextTrail);
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

function assert(condition, message) {
  if (!condition) {
    console.error(`Cloud video L2 E2E failed: ${message}`);
    process.exit(1);
  }
}

import { createHash, createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';

const endpoint = (process.env.HIDDENSHIELD_CLOUD_URL ?? 'http://127.0.0.1:43188').replace(/\/$/, '');
const runId = process.env.HIDDENSHIELD_L3_REAL_WORKER_QA_RUN_ID ?? `${Date.now()}`;
const identifier = process.env.HIDDENSHIELD_L3_REAL_WORKER_IDENTIFIER ?? `l3-real-worker-${runId}@example.com`;
const password = process.env.HIDDENSHIELD_L3_REAL_WORKER_PASSWORD ?? 'l3-real-worker-password';
const adminToken = process.env.HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN ?? 'cloud-video-ci-admin-token';
const deviceId = `l3-real-worker-device-${runId}`;
let sourceHash = null;
let sourceBytes = 0;
let objectUploadStorageRef = null;

console.log(`HiddenShield L3 real worker first-pass E2E: ${endpoint}`);
console.log(`identifier: ${identifier}`);

const tempDir = await mkdtemp(join(tmpdir(), 'hiddenshield-l3-real-worker-'));
const objectStoreDir = process.env.HIDDENSHIELD_L3_OBJECT_STORE_DIR ?? join(tempDir, 'object-store');
try {
  const health = await request('GET', '/v1/health');
  assert(health.status === 200, 'health endpoint must return 200');

  const session = await continueAccount();
  await enableStudio(session);
  await createUserObjectUploadProxy(session);
  const reserved = await reserveVideoVisualUid(session);
  const task = await createRealWorkerTask(session, reserved);
  const claim = await claimL3Task('watermark-core-l3-real-worker-first-pass');
  assert(claim.body.task.taskId === task.taskId, 'real worker claim must pick created task');
  assert(claim.body.task.status === 'running', 'real worker claim must mark task running');
  assert(claim.body.task.workerId === claim.body.workerId, 'claim must persist worker id');
  assert(claim.body.task.attemptId === claim.body.attemptId, 'claim must persist attempt id');
  assert(claim.body.task.attemptCount === 1, 'first claim must persist attempt count');
  assert(Boolean(claim.body.leaseToken), 'claim must return a lease token');
  const secondClaim = await request(
    'POST',
    '/internal/video-tasks/claim',
    {
      workerId: 'watermark-core-l3-real-worker-replay',
      capabilityLevel: 'hybrid_visual_watermark',
      leaseSeconds: 900,
    },
    adminToken,
  );
  assert(secondClaim.status === 400, 'running leased task must not be claimed twice');
  assert(secondClaim.body.message === 'cloud_video_task_queue_empty', 'running leased task must be hidden from queue');
  const worker = await runRealWorker(claim.body.task, reserved);

  assert(worker.schemaVersion === 'l3_real_worker_first_pass_v1', 'real worker schema must match');
  assert(worker.algorithmSource === 'watermark-core', 'real worker must report watermark-core');
  assert(worker.watermarkUid === reserved.watermarkUid, 'worker task watermarkUid must be reserved uid');
  assert(worker.payloadWatermarkUid === reserved.watermarkUid, 'core payload must bind reserved uid');
  assert(worker.manifestBinding?.kind === 'l3_user_object_upload_proxy', 'worker must parse user object upload kind');
  assert(worker.manifestBinding?.storageRef?.startsWith('object://l3-upload/'), 'worker must preserve object upload storage ref');
  assert(worker.transcodeSandbox?.engine === 'ffmpeg', 'worker must use ffmpeg sandbox');
  assert(worker.transcodeSandbox?.cleanup === true, 'worker must clean sandbox directory');
  assert(worker.manifestBinding?.objectStoreRead === true, 'worker must read the uploaded object-store bytes');
  assert(worker.outputPackaging?.downloadableObjectStoreObject === true, 'worker must package downloadable object-store output');
  assert(worker.outputMediaStorageRef?.startsWith('object://l3-output/'), 'worker must return object output storage ref');
  assert(worker.outputMediaBytes > 0, 'worker must return output media bytes');
  assert(worker.outputMediaContentType === 'video/mp4', 'worker must return mp4 content type');
  assert(worker.workerReceiptHash?.startsWith('sha256:'), 'worker must return worker receipt hash');
  assert(worker.workerReceipt?.schemaVersion === 'l3_worker_receipt_v1', 'worker must return persisted receipt payload');
  const canonicalWorkerReceipt = stableValue(worker.workerReceipt);
  const canonicalWorkerReceiptHash = sha256Text(stableStringify(canonicalWorkerReceipt));
  const outputPath = objectStoragePath(worker.outputMediaStorageRef);
  const outputInfo = await stat(outputPath);
  assert(outputInfo.size === worker.outputMediaBytes, 'worker output bytes must match object output file');
  assert(sha256Hex(await readFile(outputPath)) === worker.watermarkedMediaHash, 'worker media hash must match output file');
  assert(worker.privacyBoundary?.objectUploadOnly === true, 'worker must stay object-upload only');
  assert(worker.privacyBoundary?.noLocalPathInReceipt === true, 'worker receipt must not expose local paths');
  assert(worker.selfCheckConfidence >= worker.selfCheckThreshold, 'worker self-check must pass threshold');
  assert(worker.checkedFrames > 0, 'worker must check frames');
  assert(worker.watermarkedMediaHash?.startsWith('sha256:'), 'worker must return watermarked media hash');

  const confirmed = await confirmReservedUid(session, reserved, worker);
  assert(confirmed.status === 200, 'registry confirm must return 200');
  assert(confirmed.body.watermarkUid === reserved.watermarkUid, 'registry confirm must preserve reserved uid');
  assert(confirmed.body.registryStatus === 'server_confirmed', 'registry confirm must mark uid server_confirmed');

  const completion = {
    strategyDigest: worker.strategyDigest,
    selfCheckThreshold: worker.selfCheckThreshold,
    selfCheckConfidence: worker.selfCheckConfidence,
    checkedFrames: worker.checkedFrames,
    watermarkedMediaHash: worker.watermarkedMediaHash,
    outputMediaStorageRef: worker.outputMediaStorageRef,
    outputMediaBytes: worker.outputMediaBytes,
    outputMediaContentType: worker.outputMediaContentType,
    workerReceiptHash: canonicalWorkerReceiptHash,
    workerReceipt: canonicalWorkerReceipt,
    serverReceiptSignature: '',
    workerId: claim.body.workerId,
    attemptId: claim.body.attemptId,
    leaseToken: claim.body.leaseToken,
  };
  completion.serverReceiptSignature = l3CompletionSignature(adminToken, task.taskId, completion);
  const staleCompletion = {
    ...completion,
    attemptId: `${claim.body.attemptId}-stale`,
    serverReceiptSignature: '',
  };
  staleCompletion.serverReceiptSignature = l3CompletionSignature(adminToken, task.taskId, staleCompletion);
  const stale = await request(
    'POST',
    `/internal/video-tasks/${task.taskId}/completion`,
    staleCompletion,
    adminToken,
  );
  assert(stale.status === 400, 'stale attempt completion must be rejected');
  assert(
    stale.body.message === 'cloud_video_task_completion_stale_attempt',
    `stale attempt must use stable replay-protection error, got ${JSON.stringify(stale.body)}`,
  );
  const completed = await request(
    'POST',
    `/internal/video-tasks/${task.taskId}/completion`,
    completion,
    adminToken,
  );
  assert(completed.status === 200, 'trusted completion must return 200');
  assert(completed.body.status === 'succeeded', 'trusted completion must mark task succeeded');
  assert(completed.body.watermarkUid === reserved.watermarkUid, 'completed task must preserve reserved uid');
  assert(completed.body.watermarkedMediaHash === worker.watermarkedMediaHash, 'completed task must persist worker media hash');
  assert(completed.body.outputMediaStorageRef === worker.outputMediaStorageRef, 'completed task must persist output storage ref');
  assert(completed.body.outputMediaBytes === worker.outputMediaBytes, 'completed task must persist output media bytes');
  assert(completed.body.outputMediaContentType === 'video/mp4', 'completed task must persist output content type');
  assert(completed.body.workerReceiptHash === completion.workerReceiptHash, 'completed task must persist worker receipt hash');
  assert(completed.body.workerReceipt?.schemaVersion === 'l3_worker_receipt_v1', 'completed task must persist worker receipt');
  assert(Boolean(completed.body.usageLedgerId), 'trusted completion must charge video_minutes');
  assert(completed.body.workerId === claim.body.workerId, 'completed task must persist worker id');
  assert(completed.body.attemptId === claim.body.attemptId, 'completed task must persist attempt id');
  await assertOutputDownloadAuthorization(session, task.taskId, completed.body);

  const replay = await request(
    'POST',
    `/internal/video-tasks/${task.taskId}/completion`,
    completion,
    adminToken,
  );
  assert(replay.status === 400, 'duplicate completion must be rejected');
  assert(replay.body.message === 'cloud_video_task_already_succeeded', 'duplicate completion must use stable already-succeeded error');
  const afterReplay = await request('GET', `/v1/video-tasks/${task.taskId}`, null, session.accessToken);
  assert(afterReplay.status === 200, 'post-replay fetch must return 200');
  assert(afterReplay.body.usageLedgerId === completed.body.usageLedgerId, 'duplicate completion must not create a second ledger');

  await assertFailureAttribution(session);

  console.log(`L3 real worker first-pass task: ${task.taskId}`);
  console.log(`L3 real worker reserved uid: ${reserved.watermarkUid}`);
  console.log(`L3 real worker strategy: ${worker.strategyDigest}`);
  console.log('HiddenShield L3 real worker first-pass E2E OK');
} finally {
  await rm(tempDir, { recursive: true, force: true });
}

async function continueAccount() {
  const response = await request('POST', '/v1/auth/sessions', {
    identifier,
    password,
    verificationCode: '000000',
    device: {
      clientDeviceId: deviceId,
      name: 'L3 Real Worker E2E Device',
      platform: 'contract',
      appVersion: 'l3-real-worker-e2e-test',
    },
    localCreatorProfile: {
      displayName: 'L3 Real Worker E2E Creator',
      creatorSeedRef: `l3-real-worker-seed-ref-${identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, 'auth/sessions must return 200');
  return response.body;
}

async function createUserObjectUploadProxy(session) {
  const proxyPath = join(tempDir, 'source-proxy.mp4');
  await spawnCapture(command('ffmpeg'), [
    '-hide_banner',
    '-loglevel',
    'error',
    '-y',
    '-f',
    'lavfi',
    '-i',
    'testsrc2=size=1024x1024:rate=1:duration=4',
    '-frames:v',
    '4',
    '-c:v',
    'libx264',
    '-preset',
    'ultrafast',
    '-crf',
    '18',
    '-pix_fmt',
    'yuv420p',
    proxyPath,
  ]);
  const bytes = await readFile(proxyPath);
  sourceHash = sha256Hex(bytes);
  sourceBytes = bytes.length;
  const authorization = await request(
    'POST',
    '/v1/video-tasks/object-upload-authorizations',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      sha256: sourceHash,
      bytes: sourceBytes,
      contentType: 'video/mp4',
      objectKind: 'l3_user_object_upload_proxy',
      ttlSeconds: 300,
    },
    session.accessToken,
  );
  assert(authorization.status === 200, `object upload authorization must succeed: ${JSON.stringify(authorization.body)}`);
  assert(authorization.body.schemaVersion === 'l3_object_upload_authorization_v1', 'object upload authorization schema must be versioned');
  assert(authorization.body.storageRef?.startsWith('object://l3-upload/'), 'object upload authorization must return object storage ref');
  assert(authorization.body.uploadMethod === 'PUT', 'object upload authorization must use PUT');
  assert(authorization.body.uploadToken?.startsWith('hs_l3up_v1.'), 'object upload authorization must return signed token');
  assert(
    authorization.body.signedUploadUrl?.startsWith('/v1/video-object-store/upload?token=hs_l3up_v1.'),
    'object upload authorization must return signed object-store upload URL',
  );
  assert(
    authorization.body.privacyBoundary === 'signed_object_upload_only_no_local_path_no_raw_video_sync',
    'object upload authorization must preserve privacy boundary',
  );
  const uploaded = await uploadBytes(authorization.body.signedUploadUrl, bytes);
  assert(uploaded.status === 200, `signed object upload must store bytes: ${JSON.stringify(uploaded.body)}`);
  assert(uploaded.body.storageRef === authorization.body.storageRef, 'object upload response must preserve storage ref');
  assert(uploaded.body.sha256 === sourceHash, 'object upload response must preserve source hash');
  assert(uploaded.body.bytes === sourceBytes, 'object upload response must preserve byte count');
  objectUploadStorageRef = uploaded.body.storageRef;
  const storedPath = objectStoragePath(objectUploadStorageRef);
  assert(sha256Hex(await readFile(storedPath)) === sourceHash, 'object store bytes must match uploaded proxy');
}

async function enableStudio(session) {
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
  const webhook = await request('POST', '/v1/billing/webhooks/fixture', {
    providerEventId: `fixture-l3-real-worker-${runId}`,
    providerOrderId: entitle.body.providerOrderId,
    providerTransactionId: `fixture-l3-real-worker-txn-${runId}`,
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
  assert(webhook.status === 200, 'studio fixture webhook must return 200');
}

async function reserveVideoVisualUid(session) {
  return await reserveVideoVisualUidWithRequest(session, `l3-real-worker-reserve-${runId}`);
}

async function reserveVideoVisualUidWithRequest(session, requestId) {
  const response = await request(
    'POST',
    '/v1/watermark-ids/reserve',
    {
      requestId,
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      mediaType: 'video_visual',
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      parentWatermarkUid: null,
      revision: 1,
      originalHash: sourceHash,
    },
    session.accessToken,
  );
  assert(response.status === 200, 'video_visual reserve must return 200');
  assert(response.body.watermarkUid?.startsWith('HS-'), 'reserve must return HS uid');
  assert(response.body.registryStatus === 'reserved', 'reserve must start reserved');
  return response.body;
}

async function createRealWorkerTask(session, reserved) {
  const payload = {
    schemaVersion: 'cloud_video_task_v1',
    workspaceId: session.workspace.id,
    creatorProfileId: session.creatorProfile.id,
    capabilityLevel: 'hybrid_visual_watermark',
    watermarkUid: reserved.watermarkUid,
    sourceHash,
    durationMs: 125000,
    targetProfiles: ['internal_l3_real_worker_first_pass_h264'],
    uploadManifest: {
      schemaVersion: 'video_upload_manifest_v1',
      containsOriginalVideo: false,
      containsWatermarkedVideo: false,
      containsLocalPaths: false,
      containsProxy: true,
      items: [
        {
          kind: 'l3_user_object_upload_proxy',
          sha256: sourceHash,
          bytes: sourceBytes,
          storageRef: objectUploadStorageRef,
          sandboxProfile: 'l3_ffmpeg_transcode_sandbox_v1',
          transcodeProfile: 'h264_controlled_proxy_v1',
          width: 1024,
          height: 1024,
          frameCount: 4,
        },
      ],
    },
  };
  const response = await request('POST', '/v1/video-tasks', payload, session.accessToken);
  assert(response.status === 200, 'real worker L3 task creation must return 200');
  assert(response.body.watermarkUid === reserved.watermarkUid, 'task must preserve reserved uid');
  assert(response.body.uploadManifest?.items?.[0]?.storageRef === payload.uploadManifest.items[0].storageRef, 'task must persist object storageRef');
  return response.body;
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
  assert(response.status === 200, 'real worker claim must return 200');
  return response;
}

async function assertFailureAttribution(session) {
  const retryableTask = await createFailureTask(session, 'retryable');
  const pendingDownload = await request(
    'POST',
    `/v1/video-tasks/${retryableTask.taskId}/output-download-authorizations`,
    { ttlSeconds: 300 },
    session.accessToken,
  );
  assert(pendingDownload.status === 400, 'pending L3 task must not authorize output download');
  assert(pendingDownload.body.message === 'cloud_video_task_output_not_ready', 'pending download denial must use stable error code');
  const retryClaim = await claimL3Task('watermark-core-l3-real-worker-failure');
  assert(retryClaim.body.task.taskId === retryableTask.taskId, 'retryable failure claim must pick retryable task');
  const retryFailure = await request(
    'POST',
    `/internal/video-tasks/${retryableTask.taskId}/failure`,
    {
      workerId: retryClaim.body.workerId,
      attemptId: retryClaim.body.attemptId,
      leaseToken: retryClaim.body.leaseToken,
      failureCode: 'sandbox_transcode_failed',
      failureStage: 'transcode_sandbox',
      failureMessage: 'object fixture forced retryable sandbox failure',
      retryable: true,
    },
    adminToken,
  );
  assert(retryFailure.status === 200, 'retryable worker failure must return 200');
  assert(retryFailure.body.status === 'queued', 'retryable worker failure must requeue task');
  assert(retryFailure.body.lastFailureCode === 'sandbox_transcode_failed', 'retryable failure must persist attribution code');
  assert(retryFailure.body.lastFailureStage === 'transcode_sandbox', 'retryable failure must persist failure stage');
  assert(retryFailure.body.usageLedgerId == null, 'retryable failure must not charge video_minutes');

  const retryClaimAgain = await claimL3Task('watermark-core-l3-real-worker-retry');
  assert(retryClaimAgain.body.task.taskId === retryableTask.taskId, 'requeued task must be claimable again');
  assert(retryClaimAgain.body.task.attemptCount === 2, 'requeued task must increment attempt count');
  const staleFailure = await request(
    'POST',
    `/internal/video-tasks/${retryableTask.taskId}/failure`,
    {
      workerId: retryClaim.body.workerId,
      attemptId: retryClaim.body.attemptId,
      leaseToken: retryClaim.body.leaseToken,
      failureCode: 'worker_receipt_invalid',
      failureStage: 'receipt',
      retryable: false,
    },
    adminToken,
  );
  assert(staleFailure.status === 400, 'old failure attempt must not overwrite a requeued claim');
  assert(staleFailure.body.message === 'cloud_video_task_completion_stale_attempt', 'old failure attempt must use replay-protection error');

  const fatalFailure = await request(
    'POST',
    `/internal/video-tasks/${retryableTask.taskId}/failure`,
    {
      workerId: retryClaimAgain.body.workerId,
      attemptId: retryClaimAgain.body.attemptId,
      leaseToken: retryClaimAgain.body.leaseToken,
      failureCode: 'manifest_invalid',
      failureStage: 'manifest_parse',
      failureMessage: 'object fixture forced fatal manifest failure',
      retryable: false,
    },
    adminToken,
  );
  assert(fatalFailure.status === 200, 'fatal worker failure must return 200');
  assert(fatalFailure.body.status === 'failed', 'fatal worker failure must mark task failed');
  assert(fatalFailure.body.failureCode === 'manifest_invalid', 'fatal failure must persist public failure code');
  assert(fatalFailure.body.lastFailureCode === 'manifest_invalid', 'fatal failure must persist last failure code');
  assert(fatalFailure.body.usageLedgerId == null, 'fatal worker failure must not charge video_minutes');
}

async function assertOutputDownloadAuthorization(session, taskId, completedTask) {
  const authorization = await request(
    'POST',
    `/v1/video-tasks/${taskId}/output-download-authorizations`,
    { ttlSeconds: 300 },
    session.accessToken,
  );
  assert(authorization.status === 200, `download authorization must succeed: ${JSON.stringify(authorization.body)}`);
  assert(authorization.body.schemaVersion === 'l3_output_download_authorization_v1', 'download authorization schema must be versioned');
  assert(authorization.body.status === 'authorized', 'download authorization must be authorized');
  assert(authorization.body.outputMediaStorageRef === completedTask.outputMediaStorageRef, 'download authorization must bind output storage ref');
  assert(authorization.body.outputMediaBytes === completedTask.outputMediaBytes, 'download authorization must bind output size');
  assert(authorization.body.outputMediaContentType === 'video/mp4', 'download authorization must bind content type');
  assert(authorization.body.watermarkedMediaHash === completedTask.watermarkedMediaHash, 'download authorization must bind media hash');
  assert(authorization.body.workerReceiptHash === completedTask.workerReceiptHash, 'download authorization must bind receipt hash');
  assert(authorization.body.downloadMethod === 'GET', 'download authorization must expose GET method');
  assert(authorization.body.downloadToken?.startsWith('hs_l3dl_v1.'), 'download authorization must return signed token');
  assert(
    authorization.body.signedDownloadUrl?.startsWith(`/v1/video-tasks/${taskId}/output-download?token=hs_l3dl_v1.`),
    'download authorization must return signed output URL',
  );
  assert(
    authorization.body.privacyBoundary === 'signed_download_authorization_only_no_local_path_no_raw_upload',
    'download authorization must preserve privacy boundary',
  );
  const resolved = await downloadBytes(authorization.body.signedDownloadUrl);
  assert(resolved.status === 200, `signed download must return bytes with status 200`);
  assert(resolved.headers.get('content-type')?.startsWith('video/mp4'), 'signed download must return mp4 content type');
  assert(resolved.headers.get('x-hiddenshield-watermarked-media-hash') === completedTask.watermarkedMediaHash, 'signed download must bind media hash header');
  assert(resolved.bytes.length === completedTask.outputMediaBytes, 'signed download bytes must match completed output size');
  assert(sha256Hex(resolved.bytes) === completedTask.watermarkedMediaHash, 'signed download bytes must match completed media hash');
  const tamperedUrl = authorization.body.signedDownloadUrl.replace(/.$/, (last) => (last === 'a' ? 'b' : 'a'));
  const tampered = await request('GET', tamperedUrl);
  assert(tampered.status === 403, 'tampered signed download token must be rejected');
}

async function createFailureTask(session, suffix) {
  const failureReserved = await reserveVideoVisualUidWithRequest(
    session,
    `l3-real-worker-reserve-${runId}-${suffix}`,
  );
  return await createRealWorkerTask(session, failureReserved);
}

async function runRealWorker(task, reserved) {
  const taskPath = join(tempDir, `task-${task.taskId}.json`);
  await writeFile(taskPath, JSON.stringify(task, null, 2), 'utf8');
  const stdout = await spawnCapture(command('cargo'), [
    'run',
    '--quiet',
    '--manifest-path',
    'watermark-core/Cargo.toml',
    '--bin',
    'l3_real_worker_first_pass',
    '--',
    '--task-json',
    taskPath,
    '--registry-proof-hash',
    reserved.registryProofHash,
    '--creator-identity',
    'L3 Real Worker E2E Creator',
    '--object-store-dir',
    objectStoreDir,
  ]);
  return JSON.parse(stdout);
}

async function confirmReservedUid(session, reserved, worker) {
  return await request(
    'POST',
    '/v1/watermark-ids/confirm',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      watermarkUid: reserved.watermarkUid,
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      originalHash: sourceHash,
      protectedCopyHash: worker.watermarkedMediaHash,
      writeVerificationStatus: 'passed',
    },
    session.accessToken,
  );
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

function objectStoragePath(storageRef) {
  assert(storageRef.startsWith('object://'), 'object storage ref must start with object://');
  const relative = storageRef.slice('object://'.length);
  const segments = relative.split('/');
  assert(
    segments.every((segment) => segment && segment !== '.' && segment !== '..' && !segment.includes('\\') && !segment.includes(':')),
    'object storage ref must be relative and safe',
  );
  return join(objectStoreDir, ...segments);
}

function sha256Hex(bytes) {
  return `sha256:${createHash('sha256').update(bytes).digest('hex')}`;
}

function sha256Text(text) {
  return `sha256:${createHash('sha256').update(text).digest('hex')}`;
}

function stableValue(value) {
  if (Array.isArray(value)) return value.map(stableValue);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map((key) => [key, stableValue(value[key])]));
  }
  return value;
}

function stableStringify(value) {
  if (Array.isArray(value)) return `[${value.map(stableStringify).join(',')}]`;
  if (value && typeof value === 'object') {
    return `{${Object.keys(value).sort().map((key) => `${JSON.stringify(key)}:${stableStringify(value[key])}`).join(',')}}`;
  }
  return JSON.stringify(value);
}

async function request(method, path, body, token) {
  const response = await fetch(`${endpoint}${path}`, {
    method,
    headers: {
      ...(body == null ? {} : { 'content-type': 'application/json' }),
      ...(token ? { authorization: `Bearer ${token}` } : {}),
    },
    body: body == null ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let parsed = null;
  if (text.trim()) {
    parsed = JSON.parse(text);
  }
  return { status: response.status, body: parsed };
}

async function uploadBytes(path, bytes) {
  const response = await fetch(`${endpoint}${path}`, {
    method: 'PUT',
    headers: {
      'content-type': 'video/mp4',
    },
    body: bytes,
  });
  const text = await response.text();
  return { status: response.status, body: text.trim() ? JSON.parse(text) : null };
}

async function downloadBytes(path) {
  const response = await fetch(`${endpoint}${path}`, {
    method: 'GET',
  });
  const bytes = Buffer.from(await response.arrayBuffer());
  return { status: response.status, headers: response.headers, bytes };
}

async function spawnCapture(cmd, args) {
  return await new Promise((resolve, reject) => {
    const child = spawn(cmd, args, {
      cwd: process.cwd(),
      env: process.env,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', (chunk) => {
      stdout += chunk.toString();
    });
    child.stderr.on('data', (chunk) => {
      stderr += chunk.toString();
    });
    child.on('exit', (code) => {
      if (code === 0) {
        resolve(stdout);
      } else {
        reject(new Error(`${cmd} ${args.join(' ')} exited with ${code}\n${stderr}`));
      }
    });
    child.on('error', reject);
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
    console.error(`L3 real worker first-pass E2E failed: ${message}`);
    process.exit(1);
  }
}

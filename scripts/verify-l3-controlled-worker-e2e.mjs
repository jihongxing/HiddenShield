import { createHash, createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';

const endpoint = (process.env.HIDDENSHIELD_CLOUD_URL ?? 'http://127.0.0.1:43188').replace(/\/$/, '');
const runId = process.env.HIDDENSHIELD_L3_WORKER_QA_RUN_ID ?? `${Date.now()}`;
const identifier = process.env.HIDDENSHIELD_L3_WORKER_IDENTIFIER ?? `l3-worker-${runId}@example.com`;
const password = process.env.HIDDENSHIELD_L3_WORKER_PASSWORD ?? 'l3-worker-password';
const adminToken = process.env.HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN ?? 'cloud-video-ci-admin-token';
const deviceId = `l3-worker-device-${runId}`;

console.log(`HiddenShield L3 controlled worker E2E: ${endpoint}`);
console.log(`identifier: ${identifier}`);

const health = await request('GET', '/v1/health');
assert(health.status === 200, 'health endpoint must return 200');

const session = await continueAccount();
await enableStudio(session);
const task = await createControlledTask(session);
const claim = await claimL3Task('watermark-core-controlled-l3-fixture');
assert(claim.body.task.taskId === task.taskId, 'controlled worker claim must pick created task');
assert(claim.body.task.status === 'running', 'controlled worker claim must mark task running');
assert(claim.body.task.workerId === claim.body.workerId, 'claim record must persist worker id');
assert(claim.body.task.attemptId === claim.body.attemptId, 'claim record must persist attempt id');
assert(claim.body.task.attemptCount === 1, 'claim record must persist first attempt count');
assert(Boolean(claim.body.leaseToken), 'claim must return lease token only in claim response');
const worker = await runControlledWorker(claim.body.task);
assert(worker.schemaVersion === 'l3_controlled_worker_fixture_v1', 'controlled worker schema must match');
assert(worker.algorithmSource === 'watermark-core', 'controlled worker must report watermark-core as algorithm source');
assert(worker.privacyBoundary?.fixtureOnly === true, 'controlled worker must be fixture-only');
assert(worker.watermarkUid === task.watermarkUid, 'controlled worker must echo task watermarkUid');
assert(worker.payloadWatermarkUid?.startsWith('HS-'), 'controlled worker must expose core payload watermark uid');
assert(worker.selfCheckConfidence >= worker.selfCheckThreshold, 'controlled worker self-check must pass threshold');
assert(worker.checkedFrames > 0, 'controlled worker must check at least one frame');
assert(worker.strategyDigest?.startsWith('sha256:'), 'controlled worker must return strategy digest');
assert(worker.watermarkedMediaHash?.startsWith('sha256:'), 'controlled worker must return watermarked media hash');

const forged = await request(
  'PATCH',
  `/v1/video-tasks/${task.taskId}/status`,
  {
    status: 'succeeded',
    strategyDigest: worker.strategyDigest,
    selfCheckThreshold: worker.selfCheckThreshold,
    selfCheckConfidence: worker.selfCheckConfidence,
    checkedFrames: worker.checkedFrames,
    watermarkedMediaHash: worker.watermarkedMediaHash,
    serverReceiptSignature: 'forged-by-user',
  },
  session.accessToken,
);
assert(forged.status === 400, 'user bearer must not complete L3 task');
assert(
  forged.body.message === 'cloud_video_task_completion_requires_trusted_worker',
  'user bearer completion must use stable trusted worker error',
);

const completion = {
  strategyDigest: worker.strategyDigest,
  selfCheckThreshold: worker.selfCheckThreshold,
  selfCheckConfidence: worker.selfCheckConfidence,
  checkedFrames: worker.checkedFrames,
  watermarkedMediaHash: worker.watermarkedMediaHash,
  ...workerReceiptFields(task.taskId, worker.watermarkedMediaHash, claim.body.workerId),
  serverReceiptSignature: '',
  workerId: claim.body.workerId,
  attemptId: claim.body.attemptId,
  leaseToken: claim.body.leaseToken,
};
completion.serverReceiptSignature = l3CompletionSignature(adminToken, task.taskId, completion);
const staleCompletion = {
  ...completion,
  leaseToken: `${claim.body.leaseToken}-stale`,
  serverReceiptSignature: '',
};
staleCompletion.serverReceiptSignature = l3CompletionSignature(adminToken, task.taskId, staleCompletion);
const stale = await request(
  'POST',
  `/internal/video-tasks/${task.taskId}/completion`,
  staleCompletion,
  adminToken,
);
assert(stale.status === 400, 'stale lease completion must be rejected');
assert(
  stale.body.message === 'cloud_video_task_completion_stale_attempt',
  'stale lease completion must use stable replay-protection error',
);
const completed = await request(
  'POST',
  `/internal/video-tasks/${task.taskId}/completion`,
  completion,
  adminToken,
);
assert(completed.status === 200, 'trusted worker completion must return 200');
assert(completed.body.status === 'succeeded', 'trusted worker completion must mark task succeeded');
assert(completed.body.strategyDigest === worker.strategyDigest, 'task must persist worker strategy digest');
assert(completed.body.watermarkedMediaHash === worker.watermarkedMediaHash, 'task must persist worker media hash');
assert(completed.body.outputMediaStorageRef === completion.outputMediaStorageRef, 'task must persist output storage ref');
assert(completed.body.workerReceiptHash === completion.workerReceiptHash, 'task must persist worker receipt hash');
assert(completed.body.workerReceipt?.schemaVersion === 'l3_worker_receipt_v1', 'task must persist worker receipt');
assert(completed.body.selfCheckConfidence >= completed.body.selfCheckThreshold, 'task must persist passing self-check');
assert(Boolean(completed.body.usageLedgerId), 'trusted worker completion must charge video_minutes');
assert(completed.body.workerId === claim.body.workerId, 'task must persist worker id');
assert(completed.body.attemptId === claim.body.attemptId, 'task must persist attempt id');

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
assert(afterReplay.body.usageLedgerId === completed.body.usageLedgerId, 'duplicate completion must not create a new ledger');

console.log(`L3 controlled worker task: ${task.taskId}`);
console.log(`L3 controlled worker strategy: ${worker.strategyDigest}`);
console.log('HiddenShield L3 controlled worker E2E OK');

async function continueAccount() {
  const response = await request('POST', '/v1/auth/sessions', {
    identifier,
    password,
    verificationCode: '000000',
    device: {
      clientDeviceId: deviceId,
      name: 'L3 Worker E2E Device',
      platform: 'contract',
      appVersion: 'l3-worker-e2e-test',
    },
    localCreatorProfile: {
      displayName: 'L3 Worker E2E Creator',
      creatorSeedRef: `l3-worker-seed-ref-${identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, 'auth/sessions must return 200');
  return response.body;
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
    providerEventId: `fixture-l3-worker-${runId}`,
    providerOrderId: entitle.body.providerOrderId,
    providerTransactionId: `fixture-l3-worker-txn-${runId}`,
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

async function createControlledTask(session) {
  const sourceHash = `sha256:${'1'.repeat(64)}`;
  const payload = {
    schemaVersion: 'cloud_video_task_v1',
    workspaceId: session.workspace.id,
    creatorProfileId: session.creatorProfile.id,
    capabilityLevel: 'hybrid_visual_watermark',
    watermarkUid: `wm-l3-worker-${runId}`,
    sourceHash,
    durationMs: 125000,
    targetProfiles: ['internal_l3_controlled_fixture_2k_h264'],
    uploadManifest: {
      schemaVersion: 'video_upload_manifest_v1',
      containsOriginalVideo: false,
      containsWatermarkedVideo: false,
      containsLocalPaths: false,
      containsProxy: false,
      items: [
        {
          kind: 'l3_controlled_worker_fixture',
          sha256: sourceHash,
          bytes: 0,
        },
      ],
    },
  };
  const response = await request('POST', '/v1/video-tasks', payload, session.accessToken);
  assert(response.status === 200, 'controlled L3 task creation must return 200');
  assert(response.body.status === 'draft', 'controlled L3 task must start in draft');
  return response.body;
}

async function runControlledWorker(task) {
  const stdout = await spawnCapture(command('cargo'), [
    'run',
    '--quiet',
    '--manifest-path',
    'watermark-core/Cargo.toml',
    '--bin',
    'l3_controlled_worker_fixture',
    '--',
    '--task-id',
    task.taskId,
    '--watermark-uid',
    task.watermarkUid,
    '--source-hash',
    task.sourceHash,
    '--duration-ms',
    String(task.durationMs),
  ]);
  return JSON.parse(stdout);
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
  assert(response.status === 200, 'controlled worker claim must return 200');
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

function workerReceiptFields(taskId, mediaHash, workerId) {
  const workerReceipt = {
    algorithmSource: 'watermark-core',
    output: {
      bytes: 2048,
      contentType: 'video/mp4',
      sha256: mediaHash,
      storageRef: `object://l3-output/${taskId}/controlled-fixture.mp4`,
    },
    schemaVersion: 'l3_worker_receipt_v1',
    taskId,
    workerId,
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
  return name === 'cargo' ? 'cargo.exe' : name;
}

function assert(condition, message) {
  if (!condition) {
    console.error(`L3 controlled worker E2E failed: ${message}`);
    process.exit(1);
  }
}

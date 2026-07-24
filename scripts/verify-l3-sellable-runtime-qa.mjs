import { createHash, createHmac } from 'node:crypto';
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { mkdtemp, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import net from 'node:net';

const runId = process.env.HIDDENSHIELD_L3_SELLABLE_QA_RUN_ID ?? `${Date.now()}`;
const endpointEnv = process.env.HIDDENSHIELD_CLOUD_URL?.replace(/\/$/, '') ?? null;
const shouldStartBackend = !endpointEnv;
const port = shouldStartBackend ? await freePort() : Number(new URL(endpointEnv).port || 80);
const endpoint = endpointEnv ?? `http://127.0.0.1:${port}`;
const adminToken = process.env.HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN ?? 'cloud-video-ci-admin-token';
const tempDir = await mkdtemp(join(tmpdir(), `hiddenshield-l3-sellable-${runId}-`));
const dbPath = join(tempDir, 'cloud.sqlite');
const objectStoreDir = process.env.HIDDENSHIELD_L3_OBJECT_STORE_DIR ?? join(tempDir, 'l3-object-store');
const outputDir = join(process.cwd(), 'tmp-ui-qa', 'l3-video-visual-sellable-runtime');
const qaJsonPath = join(outputDir, `l3-video-visual-sellable-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `l3-video-visual-sellable-runtime-qa-${runId}.md`);
mkdirSync(outputDir, { recursive: true });

const samples = [
  {
    id: 'desktop_square_motion_mp4',
    expectedOutcome: 'succeeded',
    creator: 'desktop',
    opposite: 'mobile',
    width: 1024,
    height: 1024,
    rate: 1,
    durationSeconds: 4,
    lavfi: 'testsrc2=size=1024x1024:rate=1:duration=4',
    targetProfile: 'sellable_runtime_desktop_square_h264',
  },
  {
    id: 'mobile_square_detail_mp4',
    expectedOutcome: 'succeeded',
    creator: 'mobile',
    opposite: 'desktop',
    width: 1024,
    height: 1024,
    rate: 1,
    durationSeconds: 4,
    lavfi: 'smptebars=size=1024x1024:rate=1:duration=4',
    targetProfile: 'sellable_runtime_mobile_square_h264',
  },
  {
    id: 'desktop_landscape_motion_mp4',
    expectedOutcome: 'succeeded',
    creator: 'desktop',
    opposite: 'mobile',
    width: 1280,
    height: 720,
    rate: 1,
    durationSeconds: 4,
    lavfi: 'testsrc2=size=1280x720:rate=1:duration=4',
    targetProfile: 'sellable_runtime_desktop_landscape_h264',
  },
  {
    id: 'mobile_square_small_high_fps_strategy_invalid',
    expectedOutcome: 'input_rejected',
    creator: 'mobile',
    opposite: 'desktop',
    width: 512,
    height: 512,
    rate: 2,
    durationSeconds: 4,
    lavfi: 'testsrc2=size=512x512:rate=2:duration=4',
    targetProfile: 'sellable_runtime_small_high_fps_capacity_boundary',
  },
  {
    id: 'desktop_vertical_9x16_motion_mp4',
    expectedOutcome: 'succeeded',
    creator: 'desktop',
    opposite: 'mobile',
    width: 608,
    height: 1080,
    rate: 1,
    durationSeconds: 4,
    lavfi: 'testsrc2=size=608x1080:rate=1:duration=4',
    targetProfile: 'sellable_runtime_desktop_vertical_9x16_h264',
  },
  {
    id: 'mobile_landscape_1080p_motion_mp4',
    expectedOutcome: 'succeeded',
    creator: 'mobile',
    opposite: 'desktop',
    width: 1920,
    height: 1080,
    rate: 1,
    durationSeconds: 4,
    lavfi: 'testsrc2=size=1920x1080:rate=1:duration=4',
    targetProfile: 'sellable_runtime_mobile_landscape_1080p_h264',
  },
  {
    id: 'desktop_real_motion_fixture_mp4',
    expectedOutcome: 'succeeded',
    creator: 'desktop',
    opposite: 'mobile',
    width: 1280,
    height: 720,
    rate: 1,
    durationSeconds: 4,
    fixturePath: join(process.cwd(), 'tmp-ui-qa', 'manual-test', 'original-video-input.mp4'),
    fixtureFallback: 'controlled_motion_proxy',
    lavfi: 'mandelbrot=size=1280x720:rate=1',
    targetProfile: 'sellable_runtime_real_motion_fixture_h264',
  },
  {
    id: 'mobile_subtitle_dense_mp4',
    expectedOutcome: 'succeeded',
    creator: 'mobile',
    opposite: 'desktop',
    width: 1280,
    height: 720,
    rate: 1,
    durationSeconds: 4,
    lavfi: "testsrc2=size=1280x720:rate=1:duration=4,drawtext=text='HiddenShield L3 SUBTITLE DENSE SAMPLE':x=20:y=h-80:fontsize=36:fontcolor=white:box=1:boxcolor=black@0.75",
    targetProfile: 'sellable_runtime_subtitle_dense_h264',
  },
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
        '--commercial-metrics-admin-token',
        adminToken,
      ],
      {
        cwd: process.cwd(),
        env: {
          ...process.env,
          HIDDENSHIELD_L3_OBJECT_STORE_DIR: objectStoreDir,
        },
        stdio: ['ignore', 'pipe', 'pipe'],
        windowsHide: true,
      },
    );
    backend.stdout.on('data', (chunk) => process.stdout.write(`[backend] ${chunk}`));
    backend.stderr.on('data', (chunk) => process.stderr.write(`[backend] ${chunk}`));
  }

  await waitForHealth();
  const result = await runQa();
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
  console.log('L3 video visual sellable runtime QA OK');
  console.log(`QA JSON: ${qaJsonPath}`);
  console.log(`QA Markdown: ${qaMdPath}`);
} finally {
  if (backend && !backend.killed) {
    backend.kill();
    await waitForBackendExit();
  }
  await removeTempDir();
}

async function runQa() {
  const identifier = `l3-sellable-${runId}@example.com`;
  const password = 'l3-sellable-runtime-password';
  const desktopDeviceId = `desktop-l3-sellable-${runId}`;
  const mobileDeviceId = `mobile-l3-sellable-${runId}`;
  const desktop = await continueAccount({
    identifier,
    password,
    deviceId: desktopDeviceId,
    name: 'L3 Sellable Runtime Desktop',
    platform: 'windows',
  });
  const mobile = await continueAccount({
    identifier,
    password,
    deviceId: mobileDeviceId,
    name: 'L3 Sellable Runtime Mobile',
    platform: 'android',
  });
  await enableStudio(desktop);
  const desktopBaseline = await changes(desktop);
  const mobileBaseline = await changes(mobile);
  const sessions = { desktop, mobile };
  const cursors = {
    desktop: desktopBaseline.nextCursor,
    mobile: mobileBaseline.nextCursor,
  };
  const devices = {
    desktop: desktopDeviceId,
    mobile: mobileDeviceId,
  };

  const sampleResults = [];
  for (const sample of samples) {
    console.log(`[l3-sellable] ${sample.id} (${sample.width}x${sample.height}@${sample.rate}fps) expecting ${sample.expectedOutcome}`);
    const created = await createUploadTask(samples.indexOf(sample), sample, sessions[sample.creator]);
    if (sample.expectedOutcome === 'input_rejected') {
      const rejected = await rejectCapacityInsufficientTask(created);
      sampleResults.push({
        sampleId: sample.id,
        creator: sample.creator,
        opposite: sample.opposite,
        expectedOutcome: sample.expectedOutcome,
        rejectionCode: rejected.rejectionCode,
        rejectionStage: 'task_create_capacity_preflight',
        watermarkUid: created.reserved.watermarkUid,
        sourceHash: created.sourceHash,
        sourceKind: created.sourceKind,
        uploadedBytes: created.sourceBytes,
        taskId: null,
        usageLedgerId: null,
        privacyBoundary: 'signed_object_upload_only_no_local_path_no_raw_video_sync',
      });
      continue;
    }
    if (sample.expectedOutcome === 'strategy_invalid') {
      const failed = await runWorkerToStableStrategyInvalid(created);
      sampleResults.push({
        sampleId: sample.id,
        creator: sample.creator,
        opposite: sample.opposite,
        expectedOutcome: sample.expectedOutcome,
        taskId: failed.taskId,
        watermarkUid: created.task.watermarkUid,
        sourceHash: created.sourceHash,
        sourceKind: created.sourceKind,
        uploadedBytes: created.sourceBytes,
        failureCode: failed.failureCode,
        failureStage: failed.failureStage,
        failureMessage: failed.failureMessage,
        usageLedgerId: failed.usageLedgerId,
        privacyBoundary: 'signed_object_upload_only_no_local_path_no_raw_video_sync',
      });
      continue;
    }
    const completed = await runWorkerToSucceeded(created, sessions[sample.creator], sample);
    const vaultPayload = await downloadAndBuildVaultPayload({
      sample,
      session: sessions[sample.creator],
      task: completed.task,
      downloaded: completed.downloaded,
      registry: completed.registry,
    });
    const eventId = `${sample.creator}-l3-sellable-${sample.id}-${runId}`;
    await pushVaultRecord({
      session: sessions[sample.creator],
      deviceId: devices[sample.creator],
      eventId,
      payload: vaultPayload,
    });
    const pulled = await changes(sessions[sample.opposite], cursors[sample.opposite]);
    cursors[sample.opposite] = pulled.nextCursor;
    const received = pulled.changes?.find((change) => change.entity?.id === vaultPayload.id);
    assert(Boolean(received), `${sample.opposite} must pull ${sample.creator} L3 saved vault record`);
    const projection = verifyOppositeRead({
      sample,
      reader: sample.opposite,
      entity: received.entity,
      expected: vaultPayload,
    });
    sampleResults.push({
      sampleId: sample.id,
      creator: sample.creator,
      opposite: sample.opposite,
      expectedOutcome: sample.expectedOutcome,
      taskId: completed.task.taskId,
      watermarkUid: completed.task.watermarkUid,
      sourceHash: created.sourceHash,
      sourceKind: created.sourceKind,
      outputHash: completed.task.watermarkedMediaHash,
      uploadedBytes: created.sourceBytes,
      downloadedBytes: completed.downloaded.bytes.length,
      checkedFrames: completed.task.checkedFrames,
      confidence: completed.task.selfCheckConfidence,
      threshold: completed.task.selfCheckThreshold,
      usageLedgerId: completed.task.usageLedgerId,
      registryStatus: completed.registry.registryStatus,
      privacyBoundary: 'signed_object_upload_only_no_local_path_no_raw_video_sync',
      projection,
    });
  }

  return {
    runId,
    endpoint,
    startedBackend: shouldStartBackend,
    objectStoreDir: shouldStartBackend ? objectStoreDir : 'provided-by-environment',
    accountId: desktop.account.id,
    workspaceId: desktop.workspace.id,
    creatorProfileId: desktop.creatorProfile.id,
    desktopDeviceId,
    mobileDeviceId,
    acceptanceChecklist: sellableChecklist(),
    samples: sampleResults,
  };
}

async function createUploadTask(index, sample, session) {
  const filePath = join(tempDir, `${sample.id}.mp4`);
  const sourceKind = sample.fixturePath && existsSync(sample.fixturePath)
    ? `fixture:${sample.fixturePath}`
    : (sample.fixtureFallback ?? 'generated_lavfi');
  if (sample.fixturePath && existsSync(sample.fixturePath)) {
    await spawnCapture(command('ffmpeg'), [
      '-hide_banner',
      '-loglevel',
      'error',
      '-y',
      '-i',
      sample.fixturePath,
      '-vf',
      `scale=${sample.width}:${sample.height},fps=${sample.rate}`,
      '-frames:v',
      String(sample.durationSeconds * sample.rate),
      '-c:v',
      'libx264',
      '-preset',
      'ultrafast',
      '-crf',
      '18',
      '-pix_fmt',
      'yuv420p',
      filePath,
    ]);
  } else {
    await spawnCapture(command('ffmpeg'), [
      '-hide_banner',
      '-loglevel',
      'error',
      '-y',
      '-f',
      'lavfi',
      '-i',
      sample.lavfi,
      '-frames:v',
      String(sample.durationSeconds * sample.rate),
      '-c:v',
      'libx264',
      '-preset',
      'ultrafast',
      '-crf',
      '18',
      '-pix_fmt',
      'yuv420p',
      filePath,
    ]);
  }
  const bytes = await readFile(filePath);
  const sourceHash = sha256Hex(bytes);
  const auth = await request(
    'POST',
    '/v1/video-tasks/object-upload-authorizations',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      sha256: sourceHash,
      bytes: bytes.length,
      contentType: 'video/mp4',
      objectKind: 'l3_user_object_upload_proxy',
      ttlSeconds: 900,
    },
    session.accessToken,
  );
  assert(auth.status === 200, `${sample.id} upload authorization must succeed`);
  assert(auth.body.privacyBoundary === 'signed_object_upload_only_no_local_path_no_raw_video_sync', `${sample.id} upload privacy boundary must be explicit`);
  const uploaded = await uploadBytes(auth.body.signedUploadUrl, bytes);
  assert(uploaded.status === 200, `${sample.id} signed upload must succeed`);
  assert(uploaded.body.sha256 === sourceHash, `${sample.id} upload response must preserve source hash`);
  assert(uploaded.body.bytes === bytes.length, `${sample.id} upload response must preserve bytes`);
  assert(uploaded.body.storageRef?.startsWith('object://l3-upload/'), `${sample.id} must use object upload storage`);
  assert(sha256Hex(await readFile(objectStoragePath(uploaded.body.storageRef))) === sourceHash, `${sample.id} object store bytes must match`);

  const reserved = await reserveVideoVisualUid(session, `${sample.id}-${runId}-${index}`, sourceHash);
  const taskPayload = {
    schemaVersion: 'cloud_video_task_v1',
    workspaceId: session.workspace.id,
    creatorProfileId: session.creatorProfile.id,
    capabilityLevel: 'hybrid_visual_watermark',
    watermarkUid: reserved.watermarkUid,
    sourceHash,
    durationMs: sample.durationSeconds * 1000,
    targetProfiles: [sample.targetProfile],
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
          bytes: bytes.length,
          storageRef: uploaded.body.storageRef,
          sandboxProfile: 'l3_ffmpeg_transcode_sandbox_v1',
          transcodeProfile: 'h264_controlled_proxy_v1',
          width: sample.width,
          height: sample.height,
          frameCount: sample.durationSeconds * sample.rate,
        },
      ],
    },
  };
  const task = await request('POST', '/v1/video-tasks', taskPayload, session.accessToken);
  if (sample.expectedOutcome === 'input_rejected') {
    assert(task.status === 400, `${sample.id} capacity-insufficient task creation must be rejected`);
    assert(
      task.body.message === 'l3_strategy_capacity_insufficient',
      `${sample.id} rejection must use stable l3_strategy_capacity_insufficient code`,
    );
    return {
      task: null,
      reserved,
      sourceHash,
      sourceBytes: bytes.length,
      sourceKind,
      rejectedTask: task.body,
    };
  }
  assert(task.status === 200, `${sample.id} task creation must succeed`);
  assert(['draft', 'queued'].includes(task.body.status), `${sample.id} created task must be claimable draft/queued`);
  assert(task.body.watermarkUid === reserved.watermarkUid, `${sample.id} task must bind reserved uid`);
  assert(task.body.uploadManifest?.containsLocalPaths === false, `${sample.id} manifest must reject local paths`);
  return {
    task: task.body,
    reserved,
    sourceHash,
    sourceBytes: bytes.length,
    sourceKind,
  };
}

async function rejectCapacityInsufficientTask(created) {
  assert(created.task == null, 'capacity-insufficient sample must not create a task');
  assert(
    created.rejectedTask?.message === 'l3_strategy_capacity_insufficient',
    'capacity-insufficient sample must be rejected by product input preflight',
  );
  return {
    rejectionCode: created.rejectedTask.message,
  };
}

async function runWorkerToSucceeded(created, session, sample) {
  const claim = await request(
    'POST',
    '/internal/video-tasks/claim',
    {
      workerId: `sellable-runtime-worker-${runId}`,
      capabilityLevel: 'hybrid_visual_watermark',
      leaseSeconds: 900,
    },
    adminToken,
  );
  assert(claim.status === 200, `${created.task.taskId} claim must succeed`);
  assert(claim.body.task.taskId === created.task.taskId, `${created.task.taskId} claim must pick created task`);
  let worker;
  try {
    worker = await runRealWorker(claim.body.task, created.reserved);
  } catch (error) {
    throw new Error(`${sample.id} worker failed: ${error.message}`);
  }
  assert(worker.algorithmSource === 'watermark-core', `${created.task.taskId} worker must use watermark-core`);
  assert(worker.payloadWatermarkUid === created.reserved.watermarkUid, `${created.task.taskId} payload uid must match registry-reserved uid`);
  assert(worker.outputMediaStorageRef?.startsWith('object://l3-output/'), `${created.task.taskId} worker output must be object storage`);
  assert(worker.selfCheckConfidence >= worker.selfCheckThreshold, `${created.task.taskId} self-check must pass threshold`);
  const canonicalReceipt = stableValue(worker.workerReceipt);
  const canonicalReceiptHash = sha256Text(stableStringify(canonicalReceipt));
  const outputInfo = await stat(objectStoragePath(worker.outputMediaStorageRef));
  assert(outputInfo.size === worker.outputMediaBytes, `${created.task.taskId} output file size must match worker result`);
  assert(sha256Hex(await readFile(objectStoragePath(worker.outputMediaStorageRef))) === worker.watermarkedMediaHash, `${created.task.taskId} output hash must match worker result`);

  const registry = await request(
    'POST',
    '/v1/watermark-ids/confirm',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      watermarkUid: created.reserved.watermarkUid,
      payloadProtocolVersion: 2,
      payloadBytesLength: 119,
      originalHash: created.sourceHash,
      protectedCopyHash: worker.watermarkedMediaHash,
      writeVerificationStatus: 'verified',
    },
    session.accessToken,
  );
  assert(registry.status === 200, `${created.task.taskId} registry confirm must succeed`);
  assert(registry.body.registryStatus === 'server_confirmed', `${created.task.taskId} registry must be server_confirmed`);

  const completion = {
    strategyDigest: worker.strategyDigest,
    selfCheckThreshold: worker.selfCheckThreshold,
    selfCheckConfidence: worker.selfCheckConfidence,
    checkedFrames: worker.checkedFrames,
    watermarkedMediaHash: worker.watermarkedMediaHash,
    outputMediaStorageRef: worker.outputMediaStorageRef,
    outputMediaBytes: worker.outputMediaBytes,
    outputMediaContentType: worker.outputMediaContentType,
    workerReceiptHash: canonicalReceiptHash,
    workerReceipt: canonicalReceipt,
    serverReceiptSignature: '',
    workerId: claim.body.workerId,
    attemptId: claim.body.attemptId,
    leaseToken: claim.body.leaseToken,
  };
  completion.serverReceiptSignature = l3CompletionSignature(adminToken, created.task.taskId, completion);
  const completed = await request('POST', `/internal/video-tasks/${created.task.taskId}/completion`, completion, adminToken);
  assert(completed.status === 200, `${created.task.taskId} trusted completion must succeed`);
  assert(completed.body.status === 'succeeded', `${created.task.taskId} must become succeeded`);
  assert(Boolean(completed.body.usageLedgerId), `${created.task.taskId} completion must charge video_minutes`);

  const downloaded = await downloadSucceededOutput(session, completed.body);
  return { task: completed.body, registry: registry.body, downloaded };
}

async function runWorkerToStableStrategyInvalid(created) {
  const claim = await request(
    'POST',
    '/internal/video-tasks/claim',
    {
      workerId: `sellable-runtime-worker-${runId}`,
      capabilityLevel: 'hybrid_visual_watermark',
      leaseSeconds: 900,
    },
    adminToken,
  );
  assert(claim.status === 200, `${created.task.taskId} capacity-boundary claim must succeed`);
  assert(claim.body.task.taskId === created.task.taskId, `${created.task.taskId} capacity-boundary claim must pick created task`);
  const workerFailure = await runRealWorkerExpectingFailure(claim.body.task, created.reserved);
  assert(
    workerFailure.failureCode === 'strategy_invalid',
    `${created.task.taskId} capacity-boundary worker failure must map to strategy_invalid`,
  );
  assert(
    workerFailure.failureMessage.includes('DCT mid-band frame bitstream exceeds block capacity'),
    `${created.task.taskId} strategy_invalid must preserve capacity explanation`,
  );
  const failure = await request(
    'POST',
    `/internal/video-tasks/${created.task.taskId}/failure`,
    {
      workerId: claim.body.workerId,
      attemptId: claim.body.attemptId,
      leaseToken: claim.body.leaseToken,
      failureCode: workerFailure.failureCode,
      failureStage: workerFailure.failureStage,
      failureMessage: workerFailure.failureMessage,
      retryable: false,
    },
    adminToken,
  );
  assert(failure.status === 200, `${created.task.taskId} strategy_invalid failure must persist`);
  assert(failure.body.status === 'failed', `${created.task.taskId} strategy_invalid must mark task failed`);
  assert(failure.body.failureCode === 'strategy_invalid', `${created.task.taskId} public failureCode must be strategy_invalid`);
  assert(failure.body.lastFailureCode === 'strategy_invalid', `${created.task.taskId} lastFailureCode must be strategy_invalid`);
  assert(failure.body.lastFailureStage === 'strategy_capacity', `${created.task.taskId} lastFailureStage must be strategy_capacity`);
  assert(failure.body.usageLedgerId == null, `${created.task.taskId} strategy_invalid must not charge video_minutes`);
  return {
    taskId: failure.body.taskId,
    failureCode: failure.body.failureCode,
    failureStage: failure.body.lastFailureStage,
    failureMessage: workerFailure.failureMessage,
    usageLedgerId: failure.body.usageLedgerId,
  };
}

async function downloadSucceededOutput(session, task) {
  const auth = await request(
    'POST',
    `/v1/video-tasks/${task.taskId}/output-download-authorizations`,
    { ttlSeconds: 900 },
    session.accessToken,
  );
  assert(auth.status === 200, `${task.taskId} download authorization must succeed`);
  assert(auth.body.outputMediaStorageRef === task.outputMediaStorageRef, `${task.taskId} download auth must bind output ref`);
  assert(auth.body.watermarkedMediaHash === task.watermarkedMediaHash, `${task.taskId} download auth must bind output hash`);
  const downloaded = await downloadBytes(auth.body.signedDownloadUrl);
  assert(downloaded.status === 200, `${task.taskId} signed download must return MP4 bytes`);
  assert(downloaded.headers.get('content-type')?.startsWith('video/mp4'), `${task.taskId} download content-type must be video/mp4`);
  assert(downloaded.bytes.length === task.outputMediaBytes, `${task.taskId} downloaded bytes must match completed task`);
  assert(sha256Hex(downloaded.bytes) === task.watermarkedMediaHash, `${task.taskId} downloaded hash must match completed task`);
  return downloaded;
}

async function downloadAndBuildVaultPayload({ sample, session, task, downloaded, registry }) {
  assert(isL3VideoVisualTaskCapability(task.capabilityLevel), `${task.taskId} client validator must accept backend L3 capability`);
  assert(task.selfCheckConfidence >= task.selfCheckThreshold, `${task.taskId} client validator must require confidence >= threshold`);
  assert(task.checkedFrames > 0, `${task.taskId} client validator must require checkedFrames`);
  assert(task.outputMediaStorageRef?.startsWith('object://l3-output/'), `${task.taskId} client validator must require object output`);
  assert(sha256Hex(downloaded.bytes) === task.watermarkedMediaHash, `${task.taskId} client save must verify downloaded media hash`);
  const completedAt = task.completedAt ?? new Date().toISOString();
  const payload = {
    id: `${sample.creator}-l3-sellable-record-${sample.id}-${runId}`,
    kind: 'video',
    title: `${sample.id}.l3-watermarked.mp4`,
    watermark_uid: task.watermarkUid,
    revision: 1,
    creator_display_name: session.creatorProfile.displayName,
    trusted_time_status: '后端 trusted worker completion',
    trusted_time_source: 'HiddenShield L3 sellable runtime QA',
    trusted_time_at: completedAt,
    third_party_verification_status: 'worker_receipt_verified',
    third_party_verification_provider: 'HiddenShield worker receipt',
    third_party_verification_path: 'trusted worker/admin completion',
    sha256: task.sourceHash,
    parent_watermark_uid: null,
    rewrite_reason: null,
    write_verification_status: 'verified',
    write_verification_message: 'L3 trusted worker 自检 succeeded，签名下载 MP4 哈希复核已通过。',
    write_verification_at: completedAt,
    protected_copy_name: `${task.taskId}.l3-watermarked.mp4`,
    protected_copy_hash: task.watermarkedMediaHash,
    payload_protocol_version: registry.payloadProtocolVersion,
    payload_bytes_length: registry.payloadBytesLength,
    media_payload_role: 'v2_full_record',
    watermark_id_issue_mode: registry.watermarkIdIssueMode,
    watermark_id_registry_status: registry.registryStatus,
    watermark_id_registry_receipt: registry.registryReceipt,
    payload_auth_status: 'verified',
    output_strategy: 'cloud_l3_video_visual_watermark',
    work_source_declaration: 'unspecified',
    training_permission_declaration: 'prohibited',
    creation_method_declaration: 'unspecified',
    human_edit_level_declaration: 'unspecified',
    authenticity_claim_declaration: 'creator_declared',
    custom_rights_statement: 'L3 sellable runtime QA: receipt metadata only, no object refs or signed URLs.',
    video_visual_task_id: task.taskId,
    video_visual_completed_at: completedAt,
    video_visual_strategy_digest: task.strategyDigest,
    video_visual_self_check_confidence: task.selfCheckConfidence,
    video_visual_self_check_threshold: task.selfCheckThreshold,
    video_visual_checked_frames: task.checkedFrames,
    video_visual_media_hash: task.watermarkedMediaHash,
    video_visual_receipt_hash: task.workerReceiptHash,
    video_visual_output_bytes: task.outputMediaBytes,
    video_visual_output_content_type: task.outputMediaContentType,
    source: 'write',
    sync_status: 'synced',
    created_at: completedAt,
  };
  assertNoLocalMediaFields(payload, `${sample.id} saved vault payload`);
  return payload;
}

async function pushVaultRecord({ session, deviceId, eventId, payload }) {
  const pushed = await request(
    'POST',
    '/v1/sync/events:batch',
    {
      deviceId,
      workspaceId: session.workspace.id,
      events: [
        {
          clientEventId: eventId,
          operation: 'upsertVaultRecord',
          entityType: 'vaultRecord',
          entityId: payload.id,
          payload,
        },
      ],
    },
    session.accessToken,
  );
  assert(pushed.status === 200, `${payload.id} sync push must succeed`);
  assert(pushed.body.acceptedEventIds?.includes(eventId), `${payload.id} sync event must be accepted`);
}

function verifyOppositeRead({ sample, reader, entity, expected }) {
  assertNoLocalMediaFields(entity, `${sample.id} ${reader} pulled entity`);
  for (const key of Object.keys(expected)) {
    assert(JSON.stringify(entity[key]) === JSON.stringify(expected[key]), `${sample.id} ${reader} must preserve ${key}`);
  }
  assert(entity.video_visual_self_check_confidence >= entity.video_visual_self_check_threshold, `${sample.id} ${reader} must preserve confidence threshold`);
  const detail = buildVaultDetail(reader, entity);
  const report = buildFormalReport(reader, entity);
  for (const token of [
    'L3 视频画面盲水印',
    entity.video_visual_task_id,
    entity.video_visual_strategy_digest,
    String(entity.video_visual_checked_frames),
    entity.video_visual_media_hash,
    entity.video_visual_receipt_hash,
    String(entity.video_visual_output_bytes),
    entity.video_visual_output_content_type,
  ]) {
    assert(detail.includes(token), `${sample.id} ${reader} detail must include ${token}`);
    assert(report.includes(token), `${sample.id} ${reader} report must include ${token}`);
  }
  assertNoLocalMediaFields({ detail, report }, `${sample.id} ${reader} projection`);
  return { detail, report };
}

function buildVaultDetail(reader, record) {
  return [
    `${reader} 版权库详情`,
    'L3 视频画面盲水印',
    `任务编号: ${record.video_visual_task_id}`,
    `完成时间: ${record.video_visual_completed_at}`,
    `策略摘要: ${record.video_visual_strategy_digest}`,
    `自检置信度: ${record.video_visual_self_check_confidence}`,
    `自检阈值: ${record.video_visual_self_check_threshold}`,
    `检查帧数: ${record.video_visual_checked_frames}`,
    `成品媒体摘要: ${record.video_visual_media_hash}`,
    `Worker 收据摘要: ${record.video_visual_receipt_hash}`,
    `成品字节数: ${record.video_visual_output_bytes}`,
    `成品内容类型: ${record.video_visual_output_content_type}`,
  ].join('\n');
}

function buildFormalReport(reader, record) {
  return [
    '# HiddenShield 正式版权报告',
    `- Endpoint: ${reader}`,
    `- 文件名: ${record.title}`,
    `- 版权编号: ${record.watermark_uid}`,
    '## L3 视频画面盲水印',
    `- 任务编号: ${record.video_visual_task_id}`,
    `- 完成时间: ${record.video_visual_completed_at}`,
    `- 策略摘要: ${record.video_visual_strategy_digest}`,
    `- 自检置信度: ${record.video_visual_self_check_confidence}`,
    `- 自检阈值: ${record.video_visual_self_check_threshold}`,
    `- 检查帧数: ${record.video_visual_checked_frames}`,
    `- 成品媒体摘要: ${record.video_visual_media_hash}`,
    `- Worker 收据摘要: ${record.video_visual_receipt_hash}`,
    `- 成品字节数: ${record.video_visual_output_bytes}`,
    `- 成品内容类型: ${record.video_visual_output_content_type}`,
    '## 隐私边界',
    '- 不包含原始媒体文件',
    '- 不包含加水印后的媒体文件',
    '- 不包含本地媒体文件路径',
    '- 不包含对象存储引用、签名上传 URL 或签名下载 URL',
  ].join('\n');
}

async function continueAccount({ identifier, password, deviceId, name, platform }) {
  const response = await request('POST', '/v1/auth/sessions', {
    identifier,
    password,
    verificationCode: '000000',
    device: {
      clientDeviceId: deviceId,
      name,
      platform,
      appVersion: 'l3-sellable-runtime-qa',
    },
    localCreatorProfile: {
      displayName: 'L3 Sellable Runtime Creator',
      creatorSeedRef: `l3-sellable-runtime-seed-${identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, `${name} login must succeed`);
  return response.body;
}

async function enableStudio(session) {
  const payment = await request(
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
  assert(payment.status === 200, 'Studio fixture payment session must succeed');
  const webhook = await request(
    'POST',
    '/v1/billing/webhooks/fixture',
    {
      providerEventId: `fixture-l3-sellable-${runId}`,
      providerOrderId: payment.body.providerOrderId,
      providerTransactionId: `fixture-l3-sellable-txn-${runId}`,
      accountId: session.account.id,
      workspaceId: session.workspace.id,
      planCode: 'studio',
      billingCycle: 'monthly',
      amountCents: 6900,
      currency: 'CNY',
      eventType: 'payment.succeeded',
      occurredAt: new Date().toISOString(),
      rawPayloadJson: { provider: 'fixture', eventType: 'payment.succeeded' },
    },
    session.accessToken,
  );
  assert(webhook.status === 200, 'Studio fixture webhook must succeed');
}

async function reserveVideoVisualUid(session, requestId, sourceHash) {
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
  assert(response.status === 200, `${requestId} reserve must succeed`);
  assert(response.body.watermarkUid?.startsWith('HS-'), `${requestId} reserve must return HS uid`);
  return response.body;
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
    'L3 Sellable Runtime Creator',
    '--object-store-dir',
    objectStoreDir,
  ]);
  return JSON.parse(stdout);
}

async function runRealWorkerExpectingFailure(task, reserved) {
  const taskPath = join(tempDir, `task-${task.taskId}.json`);
  await writeFile(taskPath, JSON.stringify(task, null, 2), 'utf8');
  const result = await spawnCaptureResult(command('cargo'), [
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
    'L3 Sellable Runtime Creator',
    '--object-store-dir',
    objectStoreDir,
  ]);
  assert(result.code !== 0, `${task.taskId} capacity-boundary worker must fail`);
  const output = `${result.stdout}\n${result.stderr}`;
  if (
    output.includes('strategy_invalid') &&
    output.includes('DCT mid-band frame bitstream exceeds block capacity')
  ) {
    return {
      failureCode: 'strategy_invalid',
      failureStage: 'strategy_capacity',
      failureMessage: 'strategy_invalid: DCT mid-band frame bitstream exceeds block capacity',
    };
  }
  return {
    failureCode: 'core_strategy_failed',
    failureStage: 'strategy_build',
    failureMessage: output.slice(0, 512),
  };
}

async function changes(session, cursor) {
  const params = new URLSearchParams({ workspaceId: session.workspace.id });
  if (cursor) params.set('cursor', cursor);
  const response = await request('GET', `/v1/sync/changes?${params}`, null, session.accessToken);
  assert(response.status === 200, 'sync changes must return 200');
  return response.body;
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
  let parsed = {};
  if (text.trim()) {
    parsed = JSON.parse(text);
  }
  return { status: response.status, body: parsed, headers: response.headers };
}

async function uploadBytes(path, bytes) {
  const response = await fetch(`${endpoint}${path}`, {
    method: 'PUT',
    headers: { 'content-type': 'video/mp4' },
    body: bytes,
  });
  const text = await response.text();
  return { status: response.status, body: text.trim() ? JSON.parse(text) : {} };
}

async function downloadBytes(path) {
  const response = await fetch(`${endpoint}${path}`, { method: 'GET' });
  return {
    status: response.status,
    headers: response.headers,
    bytes: Buffer.from(await response.arrayBuffer()),
  };
}

async function waitForHealth() {
  const deadline = Date.now() + 120_000;
  let lastError = 'not started';
  while (Date.now() < deadline) {
    try {
      const response = await request('GET', '/v1/health');
      if (response.status === 200) return;
      lastError = `status ${response.status}`;
    } catch (error) {
      lastError = error.message;
    }
    await new Promise((resolve) => setTimeout(resolve, 500));
  }
  throw new Error(`backend did not become healthy at ${endpoint}: ${lastError}`);
}

async function waitForBackendExit() {
  if (!backend || backend.exitCode != null) return;
  await new Promise((resolve) => {
    const timer = setTimeout(resolve, 5_000);
    backend.once('exit', () => {
      clearTimeout(timer);
      resolve();
    });
  });
}

async function removeTempDir() {
  for (let attempt = 0; attempt < 5; attempt += 1) {
    try {
      await rm(tempDir, { recursive: true, force: true });
      return;
    } catch (error) {
      if (attempt === 4 || error?.code !== 'EBUSY') throw error;
      await new Promise((resolve) => setTimeout(resolve, 500));
    }
  }
}

async function spawnCapture(cmd, args) {
  const result = await spawnCaptureResult(cmd, args);
  if (result.code === 0) return result.stdout;
  throw new Error(`${cmd} ${args.join(' ')} exited with ${result.code}\n${result.stderr}`);
}

async function spawnCaptureResult(cmd, args) {
  return await new Promise((resolve, reject) => {
    const child = spawn(cmd, args, {
      cwd: process.cwd(),
      env: process.env,
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
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
      resolve({ code, stdout, stderr });
    });
    child.on('error', reject);
  });
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
  return `hmac-sha256:l3-completion-v1:${createHmac('sha256', secret).update(payload).digest('hex')}`;
}

function objectStoragePath(storageRef) {
  assert(storageRef.startsWith('object://'), 'object storage ref must start with object://');
  const segments = storageRef.slice('object://'.length).split('/');
  assert(segments.every((segment) => segment && segment !== '.' && segment !== '..' && !segment.includes('\\') && !segment.includes(':')), 'object storage ref must be safe');
  return join(objectStoreDir, ...segments);
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
    'mediaBytes',
    'media_bytes',
  ]);
  const forbiddenPatterns = [
    /^[a-zA-Z]:[\\/]/,
    /^file:\/\//,
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
      assert(!forbiddenPatterns.some((pattern) => pattern.test(node)), `${label} must not leak media/object/signed value at ${trail.join('.')}`);
    }
  }
}

function sellableChecklist() {
  return [
    'Studio / Enterprise entitlement gates creation and download.',
    'MP4 source uploads through signed object upload only.',
    'Registry-reserved video_visual UID binds into watermark-core payload.',
    'Trusted worker produces strategyDigest, checkedFrames, confidence, output hash, and receipt hash.',
    'video_minutes is charged only after trusted completion succeeds.',
    'Desktop and mobile both save succeeded tasks into video_visual_* vault records.',
    'Opposite endpoint reads the same receipt fields through cloud sync.',
    'Vault/report/sync exclude object refs, signed URLs, local paths, and media bytes.',
  ];
}

function isL3VideoVisualTaskCapability(value) {
  return value === 'video_visual' || value === 'hybrid_visual_watermark';
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

function renderMarkdown(result) {
  const lines = [
    '# HiddenShield L3 可售验收运行态 QA',
    '',
    `- Run ID: ${result.runId}`,
    `- Backend: ${result.endpoint}`,
    `- Account: ${result.accountId}`,
    `- Workspace: ${result.workspaceId}`,
    `- Creator Profile: ${result.creatorProfileId}`,
    `- Desktop Device: ${result.desktopDeviceId}`,
    `- Mobile Device: ${result.mobileDeviceId}`,
    '',
    '## 验收清单',
    '',
    ...result.acceptanceChecklist.map((item) => `- ${item}`),
    '',
    '## MP4 样本池',
    '',
  ];
  for (const sample of result.samples) {
    if (sample.expectedOutcome === 'input_rejected') {
      lines.push(
        `### ${sample.sampleId}`,
        '',
        `- Creator Endpoint: ${sample.creator}`,
        `- Expected Outcome: ${sample.expectedOutcome}`,
        `- Task ID: ${sample.taskId ?? 'none'}`,
        `- Watermark UID: ${sample.watermarkUid}`,
        `- Source Hash: ${sample.sourceHash}`,
        `- Source Kind: ${sample.sourceKind}`,
        `- Uploaded Bytes: ${sample.uploadedBytes}`,
        `- Rejection Code: ${sample.rejectionCode}`,
        `- Rejection Stage: ${sample.rejectionStage}`,
        `- Usage Ledger: ${sample.usageLedgerId ?? 'none'}`,
        `- Privacy Boundary: ${sample.privacyBoundary}`,
        '',
      );
      continue;
    }
    if (sample.expectedOutcome === 'strategy_invalid') {
      lines.push(
        `### ${sample.sampleId}`,
        '',
        `- Creator Endpoint: ${sample.creator}`,
        `- Expected Outcome: ${sample.expectedOutcome}`,
        `- Task ID: ${sample.taskId}`,
        `- Watermark UID: ${sample.watermarkUid}`,
        `- Source Hash: ${sample.sourceHash}`,
        `- Source Kind: ${sample.sourceKind}`,
        `- Uploaded Bytes: ${sample.uploadedBytes}`,
        `- Failure Code: ${sample.failureCode}`,
        `- Failure Stage: ${sample.failureStage}`,
        `- Failure Message: ${sample.failureMessage}`,
        `- Usage Ledger: ${sample.usageLedgerId ?? 'none'}`,
        `- Privacy Boundary: ${sample.privacyBoundary}`,
        '',
      );
      continue;
    }
    lines.push(
      `### ${sample.sampleId}`,
      '',
      `- Creator Endpoint: ${sample.creator}`,
      `- Opposite Endpoint: ${sample.opposite}`,
      `- Expected Outcome: ${sample.expectedOutcome}`,
      `- Task ID: ${sample.taskId}`,
      `- Watermark UID: ${sample.watermarkUid}`,
      `- Source Hash: ${sample.sourceHash}`,
      `- Source Kind: ${sample.sourceKind}`,
      `- Output Hash: ${sample.outputHash}`,
      `- Uploaded Bytes: ${sample.uploadedBytes}`,
      `- Downloaded Bytes: ${sample.downloadedBytes}`,
      `- Self Check: ${sample.confidence} / ${sample.threshold}`,
      `- Checked Frames: ${sample.checkedFrames}`,
      `- Usage Ledger: ${sample.usageLedgerId}`,
      `- Registry Status: ${sample.registryStatus}`,
      `- Privacy Boundary: ${sample.privacyBoundary}`,
      '',
      '#### Opposite Vault Detail',
      '```text',
      sample.projection.detail,
      '```',
      '',
      '#### Opposite Formal Report',
      '```markdown',
      sample.projection.report,
      '```',
      '',
    );
  }
  return `${lines.join('\n')}\n`;
}

async function freePort() {
  return await new Promise((resolve, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      server.close(() => resolve(address.port));
    });
    server.on('error', reject);
  });
}

function command(name) {
  if (process.platform !== 'win32') return name;
  if (name === 'cargo') return 'cargo.exe';
  if (name === 'ffmpeg') return 'ffmpeg.exe';
  return name;
}

function assert(condition, message) {
  if (!condition) {
    console.error(`L3 sellable runtime QA failed: ${message}`);
    process.exit(1);
  }
}

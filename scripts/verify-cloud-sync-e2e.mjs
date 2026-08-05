const endpoint = (process.env.HIDDENSHIELD_CLOUD_URL ?? 'http://127.0.0.1:43188').replace(/\/$/, '');
const runId = process.env.HIDDENSHIELD_CLOUD_E2E_RUN_ID ?? `${Date.now()}`;
const identifier = process.env.HIDDENSHIELD_CLOUD_IDENTIFIER ?? `e2e-${runId}@example.com`;
const password = process.env.HIDDENSHIELD_CLOUD_PASSWORD ?? 'e2e-password';
const creatorDisplayName = process.env.HIDDENSHIELD_CLOUD_CREATOR ?? 'E2E Creator';

const desktopDeviceId = `desktop-e2e-${runId}`;
const mobileDeviceId = `mobile-e2e-${runId}`;
const desktopRecordId = `desktop-record-${runId}`;
const mobileRecordId = `mobile-evidence-${runId}`;
const desktopEventId = `desktop-event-${runId}`;
const mobileEventId = `mobile-event-${runId}`;

console.log(`HiddenShield cloud sync E2E: ${endpoint}`);
console.log(`identifier: ${identifier}`);

const health = await request('GET', '/v1/health');
assert(health.status === 200, 'health endpoint must return 200');
assert(health.body.cloudSync === true, 'health endpoint must expose cloudSync=true');

const desktopSession = await continueAccount({
  deviceId: desktopDeviceId,
  name: 'E2E Desktop',
  platform: 'windows',
});
const mobileSession = await continueAccount({
  deviceId: mobileDeviceId,
  name: 'E2E Mobile',
  platform: 'android',
});

assert(
  desktopSession.account.id === mobileSession.account.id,
  'desktop and mobile must resolve to the same account',
);
assert(
  desktopSession.workspace.id === mobileSession.workspace.id,
  'desktop and mobile must resolve to the same workspace',
);
assert(
  desktopSession.creatorProfile.id === mobileSession.creatorProfile.id,
  'desktop and mobile must resolve to the same creator profile',
);
assert(
  desktopSession.device.id !== mobileSession.device.id,
    'desktop and mobile must register as separate devices',
);
assert(
  desktopSession.entitlement.features?.cloud_sync === false &&
    mobileSession.entitlement.features?.cloud_sync === false,
  'free cloud_sync entitlement must be disabled on both clients',
);
assert(
  desktopSession.entitlement.features?.batch_processing === false &&
    mobileSession.entitlement.features?.batch_processing === false,
  'unpaid batch_processing entitlement must be disabled on both clients',
);
assert(
  desktopSession.entitlement.planKey === 'base_unpaid' &&
    mobileSession.entitlement.planKey === 'base_unpaid',
  'unpaid entitlement must map to base_unpaid on both clients',
);
const freeChanges = await request(
  'GET',
  `/v1/sync/changes?workspaceId=${encodeURIComponent(desktopSession.workspace.id)}`,
  null,
  desktopSession.accessToken,
);
assert(freeChanges.status === 403, 'free changes must be blocked before Creator upgrade');

await upgradeToCreator(desktopSession);
const creatorDesktopSession = await continueAccount({
  deviceId: desktopDeviceId,
  name: 'E2E Desktop',
  platform: 'windows',
});
const creatorMobileSession = await continueAccount({
  deviceId: mobileDeviceId,
  name: 'E2E Mobile',
  platform: 'android',
});
assert(
  creatorDesktopSession.entitlement.features?.cloud_sync === true &&
    creatorMobileSession.entitlement.features?.cloud_sync === true,
  'image/audio annual cloud_sync entitlement must be enabled on both clients',
);
assert(
  creatorDesktopSession.entitlement.features?.batch_processing === true &&
    creatorMobileSession.entitlement.features?.batch_processing === true,
  'image/audio annual batch_processing entitlement must be enabled on both clients',
);
assert(
  creatorDesktopSession.entitlement.planKey === 'image_audio_annual' &&
    creatorMobileSession.entitlement.planKey === 'image_audio_annual',
  'annual entitlement must map to image_audio_annual on both clients',
);
assert(
  creatorDesktopSession.syncPolicy === 'auto_cloud_vault' &&
    creatorMobileSession.syncPolicy === 'auto_cloud_vault',
  'creator devices must default to auto_cloud_vault',
);

const desktopBaseline = await changes(creatorDesktopSession);
const mobileBaseline = await changes(creatorMobileSession);
console.log(`baseline cursors: desktop=${desktopBaseline.nextCursor} mobile=${mobileBaseline.nextCursor}`);

await assertRejectedSyncBoundaries(creatorDesktopSession);

await pushBatch(creatorDesktopSession, desktopDeviceId, [
  {
    clientEventId: desktopEventId,
    operation: 'upsertVaultRecord',
    entityType: 'vaultRecord',
    entityId: desktopRecordId,
    payload: {
      id: desktopRecordId,
      kind: 'image',
      title: 'desktop-image.png',
      watermark_uid: `wm-desktop-${runId}`,
      revision: 1,
      sha256: `sha256-desktop-${runId}`,
      parent_watermark_uid: null,
      rewrite_reason: null,
      source: 'write',
      created_at: new Date().toISOString(),
    },
  },
]);

const mobilePullAfterDesktop = await changes(
  creatorMobileSession,
  mobileBaseline.nextCursor,
);
const desktopRecord = findChange(mobilePullAfterDesktop, desktopRecordId);
assert(Boolean(desktopRecord), 'mobile must pull the desktop uploaded record');
assert(desktopRecord.entityType === 'vaultRecord', 'desktop upload must remain a vaultRecord');
assert(desktopRecord.operation === 'upsert', 'desktop upload must pull as an upsert');
assert(
  desktopRecord.entity.watermark_uid === `wm-desktop-${runId}`,
  'desktop watermark_uid must round-trip',
);
assertNoLocalMediaFields(desktopRecord.entity, 'desktop pulled record');

await pushBatch(creatorMobileSession, mobileDeviceId, [
  {
    clientEventId: mobileEventId,
    operation: 'upsertEvidenceRecord',
    entityType: 'evidenceRecord',
    entityId: mobileRecordId,
    payload: {
      id: mobileRecordId,
      kind: 'audio',
      title: 'mobile-audio.wav',
      watermark_uid: `wm-mobile-${runId}`,
      revision: 2,
      parent_watermark_uid: `wm-desktop-${runId}`,
      rewrite_reason: 'mobile verification e2e',
      extracted_timestamp: 1894944000,
      extracted_device_id_hex: '01020304',
      extracted_file_hash_hex: `hash-mobile-${runId}`,
      source: 'verify',
      created_at: new Date().toISOString(),
    },
  },
]);

const desktopPullAfterMobile = await changes(
  creatorDesktopSession,
  desktopBaseline.nextCursor,
);
const mobileRecord = findChange(desktopPullAfterMobile, mobileRecordId);
assert(Boolean(mobileRecord), 'desktop must pull the mobile uploaded evidence record');
assert(mobileRecord.entityType === 'evidenceRecord', 'mobile upload must remain an evidenceRecord');
assert(mobileRecord.operation === 'upsert', 'mobile upload must pull as an upsert');
assert(
  mobileRecord.entity.parent_watermark_uid === `wm-desktop-${runId}`,
  'mobile evidence parent watermark uid must round-trip',
);
assert(
  mobileRecord.entity.extracted_file_hash_hex === `hash-mobile-${runId}`,
  'mobile evidence file hash must round-trip',
);
assertNoLocalMediaFields(mobileRecord.entity, 'mobile pulled evidence record');

console.log('Cloud sync E2E OK');

async function continueAccount({ deviceId, name, platform }) {
  const response = await request('POST', '/v1/auth/sessions', {
    identifier,
    password,
    verificationCode: '000000',
    device: {
      clientDeviceId: deviceId,
      name,
      platform,
      appVersion: 'e2e-test',
    },
    localCreatorProfile: {
      displayName: creatorDisplayName,
      creatorSeedRef: `seed-ref-${identifier}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, `${name} auth/sessions must return 200`);
  assert(Boolean(response.body.accessToken), `${name} auth/sessions must return accessToken`);
  assert(Boolean(response.body.account?.id), `${name} auth/sessions must return account.id`);
  assert(Boolean(response.body.workspace?.id), `${name} auth/sessions must return workspace.id`);
  assert(Boolean(response.body.device?.id), `${name} auth/sessions must return device.id`);
  assert(Boolean(response.body.creatorProfile?.id), `${name} auth/sessions must return creatorProfile.id`);
  console.log(`${name}: account=${response.body.account.id} device=${response.body.device.id}`);
  return response.body;
}

async function upgradeToCreator(session) {
  if (process.env.HIDDENSHIELD_CLOUD_QA_ENTITLEMENT_MODE === 'postgres_http_gate') {
    const grant = await request('POST', '/internal/qa/entitlements/cloud-sync', {
      accountId: session.account.id,
      workspaceId: session.workspace.id,
    });
    assert(grant.status === 200, 'PostgreSQL HTTP Gate entitlement grant must return 200');
    assert(grant.body.features?.cloud_sync === true, 'PostgreSQL HTTP Gate grant must enable cloud_sync');
    return;
  }

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
  assert(payment.status === 200, 'fixture creator payment session must return 200');
  const event = await request('POST', '/v1/billing/webhooks/fixture', {
    providerEventId: `fixture-e2e-${runId}`,
    providerOrderId: payment.body.providerOrderId,
    providerTransactionId: `fixture-e2e-txn-${runId}`,
    accountId: session.account.id,
    workspaceId: session.workspace.id,
    planCode: 'creator',
    billingCycle: 'monthly',
    amountCents: 1900,
    currency: 'CNY',
    eventType: 'payment.succeeded',
    occurredAt: new Date().toISOString(),
    rawPayloadJson: {
      provider: 'fixture',
      eventType: 'payment.succeeded',
      providerOrderId: payment.body.providerOrderId,
    },
  });
  assert(event.status === 200, 'fixture creator billing event must return 200');
}

async function pushBatch(session, deviceId, events) {
  for (const event of events) {
    assertNoLocalMediaFields(event.payload, `${event.clientEventId} payload`);
  }

  const response = await request(
    'POST',
    '/v1/sync/events:batch',
    { deviceId, workspaceId: session.workspace.id, events },
    session.accessToken,
  );
  assert(response.status === 200, `${deviceId} events:batch must return 200`);
  for (const event of events) {
    assert(
      response.body.acceptedEventIds?.includes(event.clientEventId),
      `${deviceId} events:batch must accept ${event.clientEventId}`,
    );
  }
  console.log(`${deviceId}: pushed ${events.length} event(s)`);
}

async function assertRejectedSyncBoundaries(session) {
  const event = {
    clientEventId: `rejected-event-${runId}`,
    operation: 'upsertVaultRecord',
    entityType: 'vaultRecord',
    entityId: `rejected-record-${runId}`,
    payload: {
      id: `rejected-record-${runId}`,
      kind: 'image',
      title: 'rejected.png',
      watermark_uid: `wm-rejected-${runId}`,
      revision: 1,
      source: 'write',
      created_at: new Date().toISOString(),
    },
  };

  const missingToken = await request('POST', '/v1/sync/events:batch', {
    deviceId: session.device.id,
    workspaceId: session.workspace.id,
    events: [{ ...event, clientEventId: `${event.clientEventId}-missing-token` }],
  });
  assert(missingToken.status === 401, 'events:batch without token must return 401');

  const wrongDevice = await request(
    'POST',
    '/v1/sync/events:batch',
    {
      deviceId: `${session.device.id}-other`,
      workspaceId: session.workspace.id,
      events: [{ ...event, clientEventId: `${event.clientEventId}-wrong-device` }],
    },
    session.accessToken,
  );
  assert(wrongDevice.status === 401, 'events:batch with mismatched device must return 401');

  const wrongWorkspace = await request(
    'POST',
    '/v1/sync/events:batch',
    {
      deviceId: session.device.id,
      workspaceId: `${session.workspace.id}-other`,
      events: [{ ...event, clientEventId: `${event.clientEventId}-wrong-workspace` }],
    },
    session.accessToken,
  );
  assert(wrongWorkspace.status === 403, 'events:batch with mismatched workspace must return 403');

  const wrongWorkspaceChanges = await request(
    'GET',
    `/v1/sync/changes?workspaceId=${encodeURIComponent(`${session.workspace.id}-other`)}`,
    null,
    session.accessToken,
  );
  assert(wrongWorkspaceChanges.status === 403, 'changes with mismatched workspace must return 403');

  console.log('sync boundary rejection checks passed');
}

async function changes(session, cursor) {
  const params = new URLSearchParams({ workspaceId: session.workspace.id });
  if (cursor) {
    params.set('cursor', cursor);
  }
  const response = await request('GET', `/v1/sync/changes?${params}`, null, session.accessToken);
  assert(response.status === 200, 'changes must return 200');
  assert(Boolean(response.body.nextCursor), 'changes must return nextCursor');
  return response.body;
}

function findChange(result, entityId) {
  return result.changes?.find((change) => change.entity?.id === entityId);
}

async function request(method, path, body, token) {
  const headers = {};
  if (body != null) {
    headers['content-type'] = 'application/json';
  }
  if (token) {
    headers.authorization = `Bearer ${token}`;
  }
  if (path === '/internal/qa/entitlements/cloud-sync') {
    headers['x-hiddenshield-internal-token'] =
      process.env.HIDDENSHIELD_CLOUD_QA_INTERNAL_TOKEN ?? '';
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
    'mediaBytes',
    'media_bytes',
    'imageBytes',
    'image_bytes',
    'audioBytes',
    'audio_bytes',
    'videoBytes',
    'video_bytes',
    'originalMedia',
    'original_media',
    'outputMedia',
    'output_media',
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
    console.error(`Cloud sync E2E failed: ${message}`);
    process.exit(1);
  }
}

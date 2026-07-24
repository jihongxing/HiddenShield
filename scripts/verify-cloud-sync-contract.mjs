const endpoint = (process.env.HIDDENSHIELD_CLOUD_URL ?? 'http://127.0.0.1:43188').replace(/\/$/, '');
const runId = Date.now();
const identifier = process.env.HIDDENSHIELD_CLOUD_IDENTIFIER ?? `cloud-sync-contract-${runId}@example.com`;
const password = process.env.HIDDENSHIELD_CLOUD_PASSWORD ?? 'contract-password';
const deviceId = process.env.HIDDENSHIELD_CLOUD_DEVICE_ID ?? `contract-device-${runId}`;
const recordId = `contract-record-${runId}`;
const queueId = `contract-event-${runId}`;

console.log(`HiddenShield cloud sync contract check: ${endpoint}`);

const health = await request('GET', '/v1/health');
console.log(`health: ${health.status} ${JSON.stringify(health.body)}`);
assert(health.status === 200, 'health endpoint must return 200');
assert(Boolean(health.body.cloudSync), 'health endpoint must expose cloudSync');

const session = await request('POST', '/v1/auth/sessions', {
  identifier,
  password,
  verificationCode: '000000',
  device: {
    clientDeviceId: deviceId,
    name: 'Contract Test Device',
    platform: 'contract',
    appVersion: 'contract-test',
  },
  localCreatorProfile: {
    displayName: 'Contract Creator',
    creatorSeedRef: 'contract-seed-ref',
    seedEnvelopeVersion: 1,
  },
});
console.log(`auth/sessions: ${session.status} account=${session.body.account?.id}`);
assert(session.status === 200, 'auth/sessions must return 200');
assert(Boolean(session.body.accessToken), 'auth/sessions must return accessToken');
assert(Boolean(session.body.account?.id), 'auth/sessions must return account.id');
assert(Boolean(session.body.workspace?.id), 'auth/sessions must return workspace.id');
assert(Boolean(session.body.device?.id), 'auth/sessions must return device.id');
assert(Boolean(session.body.creatorProfile?.id), 'auth/sessions must return creatorProfile.id');
assert(
  session.body.entitlement?.features?.cloud_sync === false,
  'free entitlement must disable cloud_sync',
);
assert(
  session.body.syncPolicy === 'blocked_by_entitlement',
  'free auth session must return syncPolicy=blocked_by_entitlement',
);
assertEntitlementFeatures(session.body.entitlement?.features);

const token = session.body.accessToken;
const freeSync = await request(
  'POST',
  '/v1/sync/events:batch',
  {
    deviceId,
    workspaceId: session.body.workspace.id,
    events: [
      {
        clientEventId: `${queueId}-free-blocked`,
        operation: 'upsertVaultRecord',
        entityType: 'vaultRecord',
        entityId: `${recordId}-free-blocked`,
        payload: { id: `${recordId}-free-blocked`, watermark_uid: 'free-blocked' },
      },
    ],
  },
  token,
);
assert(freeSync.status === 403, 'free cloud sync push must return 403');

const freeResume = await request(
  'PATCH',
  '/v1/me/sync-preferences',
  { autoSyncEnabled: true, reason: 'user_resumed' },
  token,
);
assert(freeResume.status === 403, 'free auto cloud sync resume must return 403');

const payment = await request(
  'POST',
  '/v1/billing/payment-sessions',
  {
    accountId: session.body.account.id,
    workspaceId: session.body.workspace.id,
    planCode: 'creator',
    billingCycle: 'monthly',
    preferredProvider: 'fixture',
  },
  token,
);
assert(payment.status === 200, 'fixture creator payment session must return 200');
const fixtureEvent = await request('POST', '/v1/billing/webhooks/fixture', {
  providerEventId: `fixture-cloud-sync-${Date.now()}`,
  providerOrderId: payment.body.providerOrderId,
  providerTransactionId: `fixture-txn-${Date.now()}`,
  accountId: session.body.account.id,
  workspaceId: session.body.workspace.id,
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
assert(fixtureEvent.status === 200, 'fixture creator payment event must return 200');

const creatorSession = await request('POST', '/v1/auth/sessions', {
  identifier,
  password,
  verificationCode: '000000',
  device: {
    clientDeviceId: deviceId,
    name: 'Contract Test Device',
    platform: 'contract',
    appVersion: 'contract-test',
  },
  localCreatorProfile: {
    displayName: 'Contract Creator',
    creatorSeedRef: 'contract-seed-ref',
    seedEnvelopeVersion: 1,
  },
});
assert(creatorSession.status === 200, 'creator auth/sessions must return 200');
assert(
  creatorSession.body.entitlement?.features?.cloud_sync === true,
  'creator entitlement must enable cloud_sync',
);
assert(
  creatorSession.body.syncPolicy === 'auto_cloud_vault',
  'creator auth session must return syncPolicy=auto_cloud_vault',
);

const paused = await request(
  'PATCH',
  '/v1/me/sync-preferences',
  { autoSyncEnabled: false, reason: 'user_paused' },
  creatorSession.body.accessToken,
);
assert(paused.status === 200, 'creator pause auto cloud sync must return 200');
assert(paused.body.autoSyncEnabled === false, 'pause response must disable autoSyncEnabled');
assert(paused.body.syncPolicy === 'manual_local_only', 'pause response must return syncPolicy=manual_local_only');
assert(paused.body.entitlement?.features?.cloud_sync === true, 'pause response must preserve cloud_sync entitlement');

const pausedSession = await request('POST', '/v1/auth/sessions', {
  identifier,
  password,
  verificationCode: '000000',
  device: {
    clientDeviceId: deviceId,
    name: 'Contract Test Device',
    platform: 'contract',
    appVersion: 'contract-test',
  },
  localCreatorProfile: {
    displayName: 'Contract Creator',
    creatorSeedRef: 'contract-seed-ref',
    seedEnvelopeVersion: 1,
  },
});
assert(pausedSession.status === 200, 'paused creator auth/sessions must return 200');
assert(
  pausedSession.body.syncPolicy === 'manual_local_only',
  'paused device auth session must preserve syncPolicy=manual_local_only',
);

const resumed = await request(
  'PATCH',
  '/v1/me/sync-preferences',
  { autoSyncEnabled: true, reason: 'user_resumed' },
  pausedSession.body.accessToken,
);
assert(resumed.status === 200, 'creator resume auto cloud sync must return 200');
assert(resumed.body.autoSyncEnabled === true, 'resume response must enable autoSyncEnabled');
assert(resumed.body.syncPolicy === 'auto_cloud_vault', 'resume response must return syncPolicy=auto_cloud_vault');

const batch = await request(
  'POST',
  '/v1/sync/events:batch',
  {
    deviceId,
    workspaceId: session.body.workspace.id,
    events: [
      {
        clientEventId: queueId,
        operation: 'upsertVaultRecord',
        entityType: 'vaultRecord',
        entityId: recordId,
        payload: {
          id: recordId,
          kind: 'image',
          title: 'contract.png',
          watermark_uid: 'contract-watermark',
          revision: 1,
          sha256: 'contract-sha256',
          source: 'write',
          created_at: new Date().toISOString(),
        },
      },
    ],
  },
  creatorSession.body.accessToken,
);
console.log(`events:batch: ${batch.status} accepted=${batch.body.accepted}`);
assert(batch.status === 200, 'events:batch must return 200');
assert(batch.body.acceptedEventIds?.includes(queueId), 'events:batch must accept the client event id');

const missingToken = await request(
  'POST',
  '/v1/sync/events:batch',
  {
    deviceId,
    workspaceId: session.body.workspace.id,
    events: [
      {
        clientEventId: `${queueId}-missing-token`,
        operation: 'upsertVaultRecord',
        entityType: 'vaultRecord',
        entityId: `${recordId}-missing-token`,
        payload: { id: `${recordId}-missing-token`, watermark_uid: 'missing-token' },
      },
    ],
  },
);
assert(missingToken.status === 401, 'events:batch without token must return 401');

const wrongDevice = await request(
  'POST',
  '/v1/sync/events:batch',
  {
    deviceId: `${deviceId}-other`,
    workspaceId: session.body.workspace.id,
    events: [
      {
        clientEventId: `${queueId}-wrong-device`,
        operation: 'upsertVaultRecord',
        entityType: 'vaultRecord',
        entityId: `${recordId}-wrong-device`,
        payload: { id: `${recordId}-wrong-device`, watermark_uid: 'wrong-device' },
      },
    ],
  },
  token,
);
assert(wrongDevice.status === 401, 'events:batch with mismatched device must return 401');

const wrongWorkspace = await request(
  'POST',
  '/v1/sync/events:batch',
  {
    deviceId,
    workspaceId: `${session.body.workspace.id}-other`,
    events: [
      {
        clientEventId: `${queueId}-wrong-workspace`,
        operation: 'upsertVaultRecord',
        entityType: 'vaultRecord',
        entityId: `${recordId}-wrong-workspace`,
        payload: { id: `${recordId}-wrong-workspace`, watermark_uid: 'wrong-workspace' },
      },
    ],
  },
  token,
);
assert(wrongWorkspace.status === 403, 'events:batch with mismatched workspace must return 403');

const wrongWorkspaceChanges = await request(
  'GET',
  `/v1/sync/changes?workspaceId=${encodeURIComponent(`${session.body.workspace.id}-other`)}`,
  null,
  token,
);
assert(wrongWorkspaceChanges.status === 403, 'changes with mismatched workspace must return 403');

const changes = await request(
  'GET',
  `/v1/sync/changes?workspaceId=${encodeURIComponent(session.body.workspace.id)}`,
  null,
  token,
);
console.log(`changes: ${changes.status} count=${changes.body.changes?.length ?? 0}`);
assert(changes.status === 200, 'changes must return 200');
assert(Boolean(changes.body.nextCursor), 'changes must return nextCursor');
const synced = changes.body.changes?.find((item) => item.entity?.id === recordId);
assert(Boolean(synced), 'changes must include the pushed record');
assert(synced.entityType === 'vaultRecord', 'change entityType must be vaultRecord');
assert(synced.operation === 'upsert', 'change operation must be upsert');
assert(synced.entity.watermark_uid === 'contract-watermark', 'change entity must preserve watermark_uid');

const emptyChanges = await request(
  'GET',
  `/v1/sync/changes?workspaceId=${encodeURIComponent(session.body.workspace.id)}&cursor=${encodeURIComponent(changes.body.nextCursor)}`,
  null,
  token,
);
console.log(`changes after cursor: ${emptyChanges.status} count=${emptyChanges.body.changes?.length ?? 0}`);
assert(emptyChanges.status === 200, 'changes after cursor must return 200');
assert((emptyChanges.body.changes?.length ?? 0) === 0, 'changes after nextCursor must be empty');

console.log('Cloud sync contract OK');

function assertEntitlementFeatures(features) {
  const expected = {
    cloud_sync: false,
    batch_processing: false,
    report_export: false,
    cloud_batch_processing: false,
    cloud_video_processing: false,
    priority_queue: false,
    team_workspace: false,
    api_access: false,
  };
  for (const [key, value] of Object.entries(expected)) {
    assert(
      features?.[key] === value,
      `entitlement.features.${key} must be ${value}`,
    );
  }
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

function assert(condition, message) {
  if (!condition) {
    console.error(`Contract check failed: ${message}`);
    process.exit(1);
  }
}

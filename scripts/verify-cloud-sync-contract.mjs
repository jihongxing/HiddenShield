const endpoint = (process.env.HIDDENSHIELD_CLOUD_URL ?? 'http://127.0.0.1:43188').replace(/\/$/, '');
const identifier = process.env.HIDDENSHIELD_CLOUD_IDENTIFIER ?? 'alice@example.com';
const deviceId = process.env.HIDDENSHIELD_CLOUD_DEVICE_ID ?? `contract-device-${Date.now()}`;
const recordId = `contract-record-${Date.now()}`;
const queueId = `contract-event-${Date.now()}`;

console.log(`HiddenShield cloud sync contract check: ${endpoint}`);

const health = await request('GET', '/v1/health');
console.log(`health: ${health.status} ${JSON.stringify(health.body)}`);
assert(health.status === 200, 'health endpoint must return 200');
assert(Boolean(health.body.cloudSync), 'health endpoint must expose cloudSync');

const session = await request('POST', '/v1/auth/continue', {
  identifier,
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
console.log(`auth/continue: ${session.status} account=${session.body.account?.id}`);
assert(session.status === 200, 'auth/continue must return 200');
assert(Boolean(session.body.accessToken), 'auth/continue must return accessToken');
assert(Boolean(session.body.account?.id), 'auth/continue must return account.id');
assert(Boolean(session.body.workspace?.id), 'auth/continue must return workspace.id');
assert(Boolean(session.body.device?.id), 'auth/continue must return device.id');
assert(Boolean(session.body.creatorProfile?.id), 'auth/continue must return creatorProfile.id');
assert(Boolean(session.body.entitlement?.features?.cloud_sync), 'entitlement must enable cloud_sync');

const token = session.body.accessToken;
const batch = await request(
  'POST',
  '/v1/sync/events:batch',
  {
    deviceId,
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
  token,
);
console.log(`events:batch: ${batch.status} accepted=${batch.body.accepted}`);
assert(batch.status === 200, 'events:batch must return 200');
assert(batch.body.acceptedEventIds?.includes(queueId), 'events:batch must accept the client event id');

const changes = await request('GET', '/v1/sync/changes', null, token);
console.log(`changes: ${changes.status} count=${changes.body.changes?.length ?? 0}`);
assert(changes.status === 200, 'changes must return 200');
assert(Boolean(changes.body.nextCursor), 'changes must return nextCursor');
const synced = changes.body.changes?.find((item) => item.entity?.id === recordId);
assert(Boolean(synced), 'changes must include the pushed record');
assert(synced.entityType === 'vaultRecord', 'change entityType must be vaultRecord');
assert(synced.operation === 'upsert', 'change operation must be upsert');
assert(synced.entity.watermark_uid === 'contract-watermark', 'change entity must preserve watermark_uid');

const emptyChanges = await request('GET', `/v1/sync/changes?cursor=${encodeURIComponent(changes.body.nextCursor)}`, null, token);
console.log(`changes after cursor: ${emptyChanges.status} count=${emptyChanges.body.changes?.length ?? 0}`);
assert(emptyChanges.status === 200, 'changes after cursor must return 200');
assert((emptyChanges.body.changes?.length ?? 0) === 0, 'changes after nextCursor must be empty');

console.log('Cloud sync contract OK');

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

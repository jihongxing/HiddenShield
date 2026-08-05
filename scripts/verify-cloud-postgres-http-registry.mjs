const endpoint = (process.env.HIDDENSHIELD_CLOUD_URL || 'http://127.0.0.1:43188').replace(/\/$/, '');
const runId = Date.now();
const sessionResponse = await request('POST', '/v1/auth/sessions', {
  identifier: `postgres-http-registry-${runId}@example.test`,
  password: `Registry-${runId}-Password`,
  verificationCode: '000000',
  device: {
    clientDeviceId: `postgres-http-registry-device-${runId}`,
    name: 'PostgreSQL HTTP Registry Gate',
    platform: 'desktop',
    appVersion: 'postgres-http-gate',
  },
  localCreatorProfile: {
    displayName: 'PostgreSQL HTTP Registry Gate',
    creatorSeedRef: `postgres-http-registry-seed-${runId}`,
    seedEnvelopeVersion: 1,
  },
});
assert(sessionResponse.status === 200, 'registry gate auth session must return 200');
const session = sessionResponse.body;

const reserved = await request(
  'POST',
  '/v1/watermark-ids/reserve',
  {
    requestId: `postgres-http-registry-request-${runId}`,
    workspaceId: session.workspace.id,
    creatorProfileId: session.creatorProfile.id,
    mediaType: 'image',
    payloadProtocolVersion: 3,
    payloadBytesLength: 39,
    parentWatermarkUid: null,
    revision: 1,
    originalHash: `sha256:postgres-http-original-${runId}`,
  },
  session.accessToken,
);
assert(reserved.status === 200, 'registry reserve must return 200');
assert(reserved.body.registryStatus === 'reserved', 'registry reserve must persist reserved status');
assert(/^HS-[0-9A-F]{8}(?:-[0-9A-F]{8}){3}$/.test(reserved.body.watermarkUid), 'registry reserve must return formal UID');

const confirmed = await request(
  'POST',
  '/v1/watermark-ids/confirm',
  {
    workspaceId: session.workspace.id,
    creatorProfileId: session.creatorProfile.id,
    watermarkUid: reserved.body.watermarkUid,
    payloadProtocolVersion: 3,
    payloadBytesLength: 39,
    originalHash: `sha256:postgres-http-original-${runId}`,
    protectedCopyHash: `sha256:postgres-http-protected-${runId}`,
    writeVerificationStatus: 'verified',
  },
  session.accessToken,
);
assert(confirmed.status === 200, 'registry confirm must return 200');
assert(confirmed.body.registryStatus === 'server_confirmed', 'registry confirm must persist server_confirmed status');
assert(confirmed.body.watermarkUid === reserved.body.watermarkUid, 'registry confirm UID must match reserve');

console.log(`PostgreSQL HTTP registry gate OK: ${confirmed.body.watermarkUid}`);

async function request(method, path, body, token) {
  const headers = {};
  if (body !== undefined) {
    headers['content-type'] = 'application/json';
  }
  if (token) {
    headers.authorization = `Bearer ${token}`;
  }
  const response = await fetch(`${endpoint}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let parsed = {};
  if (text) {
    parsed = JSON.parse(text);
  }
  return { status: response.status, body: parsed };
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

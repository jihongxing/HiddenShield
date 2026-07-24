const endpoint = (process.env.HIDDENSHIELD_CLOUD_URL ?? 'http://127.0.0.1:43188').replace(/\/$/, '');
const runId = Date.now();
const identifier = process.env.HIDDENSHIELD_AUTH_IDENTIFIER ?? `auth-contract-${runId}@example.com`;
const passwordIdentifier = process.env.HIDDENSHIELD_AUTH_PASSWORD_IDENTIFIER ?? `auth-password-${runId}@example.com`;
const password = process.env.HIDDENSHIELD_AUTH_PASSWORD ?? 'contract-password';
const deviceId = process.env.HIDDENSHIELD_AUTH_DEVICE_ID ?? `auth-device-${runId}`;
const passwordDeviceId = `${deviceId}-password`;
const otpDeliveryEndpoint = process.env.HIDDENSHIELD_AUTH_OTP_DELIVERY_ENDPOINT ?? '';

console.log(`HiddenShield auth contract check: ${endpoint}`);

const health = await request('GET', '/v1/health');
assert(health.status === 200, 'health endpoint must return 200');

const challenge = await request('POST', '/v1/auth/challenges', {
  identifier,
  purpose: 'register_or_login',
  clientDeviceId: deviceId,
});
assert(challenge.status === 200, 'auth/challenges must return 200');
assert(Boolean(challenge.body.challengeId), 'challenge must return challengeId');
assert(Boolean(challenge.body.expiresAt), 'challenge must return expiresAt');
let verificationCode;
if (otpDeliveryEndpoint) {
  assert(challenge.body.deliveryChannel !== 'fixture', 'configured delivery must not use fixture channel');
  assert(challenge.body.fixtureCode == null, 'configured delivery must not expose fixtureCode');
  const delivery = await waitForOtpDelivery(challenge.body.challengeId);
  assert(delivery.identifier === identifier, 'OTP delivery must include identifier');
  assert(delivery.clientDeviceId === deviceId, 'OTP delivery must include clientDeviceId');
  assert(Boolean(delivery.verificationCode), 'OTP delivery must include verificationCode');
  verificationCode = delivery.verificationCode;
} else {
  assert(challenge.body.deliveryChannel === 'fixture', 'challenge fixture delivery channel must be explicit');
  assert(challenge.body.fixtureCode === '000000', 'fixture challenge must expose only the local test code');
  verificationCode = challenge.body.fixtureCode ?? '000000';
}

const session = await request('POST', '/v1/auth/sessions', {
  identifier,
  challengeId: challenge.body.challengeId,
  verificationCode,
  device: devicePayload(deviceId),
  localCreatorProfile: creatorPayload(),
});
assert(session.status === 200, 'auth/sessions via challenge must return 200');
assert(Boolean(session.body.accessToken), 'session must return accessToken');
assert(Boolean(session.body.refreshToken), 'session must return refreshToken');
assert(session.body.device?.id === deviceId, 'session must bind the requested device');
assert(session.body.entitlement?.features?.cloud_sync === false, 'free session must disable cloud_sync');
assert(session.body.syncPolicy === 'blocked_by_entitlement', 'free session must be blocked_by_entitlement');

const reusedChallenge = await request('POST', '/v1/auth/sessions', {
  identifier,
  challengeId: challenge.body.challengeId,
  verificationCode,
  device: devicePayload(deviceId),
  localCreatorProfile: creatorPayload(),
});
assert(reusedChallenge.status === 401, 'consumed challenge must not be reusable');

const me = await request('GET', '/v1/me', null, session.body.accessToken);
assert(me.status === 200, 'me must return 200 with access token');
assert(me.body.account?.id === session.body.account.id, 'me must return same account');
assert(me.body.device?.id === session.body.device.id, 'me must return same device');
assert(me.body.syncPolicy === 'blocked_by_entitlement', 'me must return syncPolicy');

const refreshed = await request('POST', '/v1/auth/refresh', {
  refreshToken: session.body.refreshToken,
  deviceId: session.body.device.id,
});
assert(refreshed.status === 200, 'auth/refresh must return 200');
assert(refreshed.body.accessToken !== session.body.accessToken, 'refresh must rotate access token');
assert(refreshed.body.refreshToken !== session.body.refreshToken, 'refresh must rotate refresh token');

const reusedRefresh = await request('POST', '/v1/auth/refresh', {
  refreshToken: session.body.refreshToken,
  deviceId: session.body.device.id,
});
assert(reusedRefresh.status === 401, 'old refresh token must be revoked after rotation');

const oldAccessMe = await request('GET', '/v1/me', null, session.body.accessToken);
assert(oldAccessMe.status === 401, 'old access token must be revoked after refresh rotation');

const passwordSession = await request('POST', '/v1/auth/sessions', {
  identifier: passwordIdentifier,
  password,
  device: devicePayload(passwordDeviceId),
  localCreatorProfile: creatorPayload(),
});
assert(passwordSession.status === 200, 'auth/sessions via password must return 200');

const payment = await request(
  'POST',
  '/v1/billing/payment-sessions',
  {
    accountId: passwordSession.body.account.id,
    workspaceId: passwordSession.body.workspace.id,
    planCode: 'creator',
    billingCycle: 'monthly',
    preferredProvider: 'fixture',
  },
  passwordSession.body.accessToken,
);
assert(payment.status === 200, 'fixture creator payment session must return 200');

const fixtureEvent = await request('POST', '/v1/billing/webhooks/fixture', {
  providerEventId: `fixture-auth-${runId}`,
  providerOrderId: payment.body.providerOrderId,
  providerTransactionId: `fixture-auth-txn-${runId}`,
  accountId: passwordSession.body.account.id,
  workspaceId: passwordSession.body.workspace.id,
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
assert(fixtureEvent.status === 200, 'fixture billing event must return 200');

const creatorSession = await request('POST', '/v1/auth/sessions', {
  identifier: passwordIdentifier,
  password,
  device: devicePayload(`${passwordDeviceId}-desktop`),
  localCreatorProfile: creatorPayload(),
});
assert(creatorSession.status === 200, 'creator auth/sessions must return 200');
assert(creatorSession.body.entitlement?.features?.cloud_sync === true, 'creator must enable cloud_sync');
assert(creatorSession.body.syncPolicy === 'auto_cloud_vault', 'creator must default to auto_cloud_vault');

const mobileCreatorSession = await request('POST', '/v1/auth/sessions', {
  identifier: passwordIdentifier,
  password,
  device: devicePayload(`${passwordDeviceId}-mobile`),
  localCreatorProfile: creatorPayload(),
});
assert(mobileCreatorSession.status === 200, 'second creator device auth/sessions must return 200');

const devices = await request('GET', '/v1/devices', null, creatorSession.body.accessToken);
assert(devices.status === 200, 'devices list must return 200');
assert(devices.body.devices?.length >= 2, 'devices list must include both creator devices');
assert(
  devices.body.devices.some((device) => device.id === creatorSession.body.device.id && device.isCurrent),
  'devices list must mark current device',
);
assert(
  devices.body.devices.some((device) => device.id === mobileCreatorSession.body.device.id && !device.isCurrent),
  'devices list must include other device',
);

const renamed = await request(
  'PATCH',
  `/v1/devices/${encodeURIComponent(mobileCreatorSession.body.device.id)}`,
  { name: 'Revoked Mobile Contract Device' },
  creatorSession.body.accessToken,
);
assert(renamed.status === 200, 'device rename must return 200');
assert(renamed.body.name === 'Revoked Mobile Contract Device', 'device rename must persist name');

const revokeCurrent = await request(
  'DELETE',
  `/v1/devices/${encodeURIComponent(creatorSession.body.device.id)}`,
  null,
  creatorSession.body.accessToken,
);
assert(revokeCurrent.status === 400, 'current device revoke through device list must be rejected');

const revokedDevice = await request(
  'DELETE',
  `/v1/devices/${encodeURIComponent(mobileCreatorSession.body.device.id)}`,
  null,
  creatorSession.body.accessToken,
);
assert(revokedDevice.status === 200, 'other device revoke must return 200');
assert(revokedDevice.body.revokedSessionCount >= 1, 'other device revoke must close sessions');

const revokedDeviceMe = await request('GET', '/v1/me', null, mobileCreatorSession.body.accessToken);
assert(revokedDeviceMe.status === 401, 'revoked device access token must be unauthorized');

const paused = await request(
  'PATCH',
  '/v1/me/sync-preferences',
  { autoSyncEnabled: false, reason: 'user_paused' },
  creatorSession.body.accessToken,
);
assert(paused.status === 200, 'creator pause sync preference must return 200');
assert(paused.body.syncPolicy === 'manual_local_only', 'pause must return manual_local_only');

const pausedMe = await request('GET', '/v1/me', null, creatorSession.body.accessToken);
assert(pausedMe.status === 200, 'me after pause must return 200');
assert(pausedMe.body.syncPolicy === 'manual_local_only', 'me must preserve manual_local_only');

const pausedRefresh = await request('POST', '/v1/auth/refresh', {
  refreshToken: creatorSession.body.refreshToken,
  deviceId: creatorSession.body.device.id,
});
assert(pausedRefresh.status === 200, 'refresh after pause must return 200');
assert(pausedRefresh.body.syncPolicy === 'manual_local_only', 'refresh must preserve manual_local_only');

const logout = await request('POST', '/v1/auth/logout', {
  refreshToken: pausedRefresh.body.refreshToken,
  deviceId: pausedRefresh.body.device.id,
});
assert(logout.status === 200 && logout.body.ok === true, 'auth/logout must return ok');

const afterLogoutMe = await request('GET', '/v1/me', null, pausedRefresh.body.accessToken);
assert(afterLogoutMe.status === 401, 'me after logout must return 401');

console.log('Auth contract OK');

function devicePayload(id) {
  return {
    clientDeviceId: id,
    name: 'Auth Contract Device',
    platform: 'contract',
    appVersion: 'contract-test',
  };
}

function creatorPayload() {
  return {
    displayName: 'Auth Contract Creator',
    creatorSeedRef: 'auth-contract-seed-ref',
    seedEnvelopeVersion: 1,
  };
}

async function waitForOtpDelivery(challengeId) {
  const deliveriesUrl = otpDeliveryEndpoint.replace(/\/otp$/, '/deliveries');
  const startedAt = Date.now();
  while (Date.now() - startedAt < 10_000) {
    const response = await fetch(deliveriesUrl);
    if (response.ok) {
      const body = await response.json();
      const delivery = (body.deliveries ?? []).find((item) => item.challengeId === challengeId);
      if (delivery) return delivery;
    }
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  console.error(`Auth contract failed: OTP delivery not received for ${challengeId}`);
  process.exit(1);
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
    console.error(`Auth contract failed: ${message}`);
    process.exit(1);
  }
}

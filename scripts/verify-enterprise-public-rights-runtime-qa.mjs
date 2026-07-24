#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';

const runId = String(Date.now());
const adminToken = `enterprise-public-rights-admin-${runId}`;
const hashSecret = `enterprise-public-rights-hash-secret-${runId}`;
const proxySecret = `enterprise-public-rights-proxy-${runId}`;
const tmpRoot = join(tmpdir(), `hiddenshield-enterprise-public-rights-qa-${runId}`);
const dbPath = join(tmpRoot, 'feedback.sqlite');
const outputDir = resolve('tmp-ui-qa', 'enterprise-public-rights-runtime', runId);
const qaJsonPath = join(outputDir, `enterprise-public-rights-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `enterprise-public-rights-runtime-qa-${runId}.md`);
mkdirSync(tmpRoot, { recursive: true });
mkdirSync(outputDir, { recursive: true });

const port = 43360 + Number(runId.slice(-3));
const baseUrl = `http://127.0.0.1:${port}`;
const backend = spawnBackend();

try {
  await waitForHealth();
  const session = await ensureCreatorSession();
  const watermarkUid = await createRightsRecord(session);
  const accountId = `acct_enterprise_public_${runId}`;
  const workspaceId = `ws_enterprise_public_${runId}`;
  const issued = runCliJson([
    'issue-api-key',
    '--json',
    JSON.stringify({
      accountId,
      workspaceId,
      name: 'External public rights runtime QA',
      scopes: ['public_rights:batch_read'],
      createdByAccountId: 'admin_runtime_qa',
      reason: 'external public rights runtime qa',
      deliveryChannel: 'secure_note',
      recipientRef: `qa-ticket-${runId}`,
    }),
  ]);
  const now = new Date();
  const quota = runCliJson([
    'init-quota-balance',
    '--json',
    JSON.stringify({
      accountId,
      workspaceId,
      quotaType: 'public_rights_scan_units',
      periodStart: new Date(now.getTime() - 60_000).toISOString(),
      periodEnd: new Date(now.getTime() + 3_600_000).toISOString(),
      includedUnits: 5,
      overageAllowed: false,
      currency: 'USD',
    }),
  ]);
  const unauthorized = await request('POST', '/v1/enterprise/public-rights/batch', {
    watermarkUids: [watermarkUid],
  });
  assert(unauthorized.status === 401, 'missing Enterprise API key must be rejected');

  const success = await request(
    'POST',
    '/v1/enterprise/public-rights/batch',
    {
      watermarkUids: [watermarkUid, `wm_missing_${runId}`],
      idempotencyKey: `enterprise-public-rights-${runId}`,
      clientLabel: 'runtime-qa',
    },
    issued.cleartextApiKey,
    trustedProxyHeaders('203.0.113.20'),
  );
  assert(success.status === 200, `Enterprise batch must succeed: ${JSON.stringify(success.body)}`);
  assert(success.body.gateway?.quotaChargedUnits === 2, 'success must charge two scan units');
  assert(
    typeof success.body.gateway?.clientFingerprintHash === 'string' &&
      success.body.gateway.clientFingerprintHash.startsWith('sha256:'),
    'success must expose hash-only client fingerprint',
  );
  assert(
    success.body.gateway?.trustedProxyStatus === 'trusted_proxy_x_forwarded_for',
    'success must record trusted proxy fingerprint source',
  );
  assert(success.body.gateway?.legalConclusion === false, 'gateway legalConclusion must stay false');
  assert(success.body.batch?.results?.[0]?.status === 'ok', 'known watermark must resolve');
  assert(
    ['not_found', 'watermark_uid_invalid'].includes(success.body.batch?.results?.[1]?.errorCode),
    'missing or malformed watermark must be a record-level error',
  );

  const quotaDenied = await request(
    'POST',
    '/v1/enterprise/public-rights/batch',
    {
      watermarkUids: [watermarkUid, watermarkUid, watermarkUid, watermarkUid],
      idempotencyKey: `enterprise-public-rights-quota-${runId}`,
    },
    issued.cleartextApiKey,
    trustedProxyHeaders('203.0.113.20'),
  );
  assert(quotaDenied.status === 400, 'quota exhaustion must be rejected before another debit');

  const result = {
    runId,
    baseUrl,
    watermarkUid,
    apiKeyId: issued.apiKey.apiKeyId,
    quotaBalanceId: quota.quotaBalanceId,
    unauthorizedStatus: unauthorized.status,
    successStatus: success.status,
    quotaDeniedStatus: quotaDenied.status,
    quotaChargedUnits: success.body.gateway.quotaChargedUnits,
    legalConclusion: success.body.gateway.legalConclusion,
    firstResultStatus: success.body.batch.results[0].status,
    secondResultErrorCode: success.body.batch.results[1].errorCode,
    completedAt: new Date().toISOString(),
  };
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
  console.log(`Enterprise public rights runtime QA OK: ${qaMdPath}`);
} finally {
  await stopBackend(backend);
  rmSync(tmpRoot, { recursive: true, force: true });
}

async function createRightsRecord(session) {
  const originalHash = `sha256:${sha256(`enterprise-public-rights:${runId}:original`)}`;
  const protectedCopyHash = `sha256:${sha256(`enterprise-public-rights:${runId}:protected`)}`;
  const reserve = await request(
    'POST',
    '/v1/watermark-ids/reserve',
    {
      requestId: `enterprise-public-rights-reserve-${runId}`,
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      mediaType: 'image',
      payloadProtocolVersion: 3,
      payloadBytesLength: 39,
      parentWatermarkUid: null,
      revision: 1,
      originalHash,
    },
    session.accessToken,
  );
  assert(reserve.status === 200, 'reserve must succeed');
  const confirm = await request(
    'POST',
    '/v1/watermark-ids/confirm',
    {
      workspaceId: session.workspace.id,
      creatorProfileId: session.creatorProfile.id,
      watermarkUid: reserve.body.watermarkUid,
      payloadProtocolVersion: 3,
      payloadBytesLength: 39,
      originalHash,
      protectedCopyHash,
      writeVerificationStatus: 'verified',
    },
    session.accessToken,
  );
  assert(confirm.status === 200, 'confirm must succeed');
  const pushed = await request(
    'POST',
    '/v1/sync/events:batch',
    {
      deviceId: session.device.id,
      workspaceId: session.workspace.id,
      events: [
        {
          clientEventId: `enterprise-public-rights-sync-${runId}`,
          operation: 'upsertVaultRecord',
          entityType: 'vaultRecord',
          entityId: `enterprise-public-rights-record-${runId}`,
          payload: {
            id: `enterprise-public-rights-record-${runId}`,
            kind: 'image',
            title: 'enterprise-public-rights.png',
            watermark_uid: reserve.body.watermarkUid,
            revision: 1,
            sha256: originalHash,
            protected_copy_hash: protectedCopyHash,
            payload_protocol_version: 3,
            payload_bytes_length: 39,
            payload_auth_status: 'verified',
            work_source_declaration: 'ai_assisted',
            training_permission_declaration: 'commercial_allowed',
            creation_method_declaration: 'text_to_image',
            human_edit_level_declaration: 'light',
            authenticity_claim_declaration: 'synthetic',
            created_at: new Date().toISOString(),
          },
        },
      ],
    },
    session.accessToken,
  );
  assert(pushed.status === 200, 'sync must succeed');
  return reserve.body.watermarkUid;
}

async function ensureCreatorSession() {
  const response = await request('POST', '/v1/auth/sessions', {
    identifier: `enterprise-public-rights-${runId}@hiddenshield.local`,
    password: `enterprise-public-rights-${runId}`,
    verificationCode: `enterprise-public-rights-${runId}`,
    device: {
      clientDeviceId: `enterprise-public-rights-device-${runId}`,
      name: 'Enterprise Public Rights Runtime QA',
      platform: 'windows',
      appVersion: 'enterprise-public-rights-runtime-qa',
    },
    localCreatorProfile: {
      displayName: 'Enterprise Public Rights QA',
      creatorSeedRef: `enterprise-public-rights-seed-${runId}`,
      seedEnvelopeVersion: 1,
    },
  });
  assert(response.status === 200, 'auth must succeed');
  const session = response.body;
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
  assert(payment.status === 200, 'fixture payment must succeed');
  const reconcile = await request(
    'POST',
    `/v1/billing/payment-sessions/${payment.body.paymentSessionId}/reconcile`,
    {},
    session.accessToken,
  );
  assert(reconcile.status === 200, 'fixture reconcile must succeed');
  return session;
}

function spawnBackend() {
  const build = spawnSync('cargo', ['build', '--manifest-path', 'feedback-backend/Cargo.toml'], {
    cwd: process.cwd(),
    encoding: 'utf8',
    windowsHide: true,
  });
  if (build.status !== 0) {
    throw new Error(`backend build failed:\n${build.stdout}\n${build.stderr}`);
  }
  const backendBin = resolve(
    'feedback-backend',
    'target',
    'debug',
    process.platform === 'win32'
      ? 'hiddenshield-feedback-backend.exe'
      : 'hiddenshield-feedback-backend',
  );
  return spawn(
    backendBin,
    ['--bind-addr', `127.0.0.1:${port}`, '--db-path', dbPath],
    {
      cwd: process.cwd(),
      stdio: ['ignore', 'pipe', 'pipe'],
      windowsHide: true,
      env: {
        ...process.env,
        HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN: adminToken,
        HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET: hashSecret,
        HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET_VERSION: 'runtime-qa-v1',
        HIDDENSHIELD_TRUSTED_PROXY_SHARED_SECRET: proxySecret,
        HIDDENSHIELD_ENTERPRISE_REQUIRE_TRUSTED_PROXY: 'true',
      },
    },
  );
}

function runCliJson(args) {
  const result = spawnSync('node', ['scripts/enterprise-internal-admin.mjs', ...args], {
    cwd: process.cwd(),
    encoding: 'utf8',
    windowsHide: true,
    env: {
      ...process.env,
      HIDDENSHIELD_INTERNAL_ADMIN_BASE_URL: baseUrl,
      HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN: adminToken,
    },
  });
  if (result.status !== 0) {
    throw new Error(`enterprise admin CLI failed:\n${result.stdout}\n${result.stderr}`);
  }
  return JSON.parse(result.stdout);
}

function trustedProxyHeaders(ip) {
  return {
    'x-hiddenshield-proxy-secret': proxySecret,
    'x-forwarded-for': `${ip}, 10.0.0.1`,
  };
}

async function request(method, path, body, token, extraHeaders = {}) {
  const headers = { 'content-type': 'application/json' };
  if (token) headers.authorization = `Bearer ${token}`;
  Object.assign(headers, extraHeaders);
  const response = await fetch(`${baseUrl}${path}`, {
    method,
    headers,
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let parsed = null;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    parsed = text;
  }
  return { status: response.status, body: parsed };
}

async function waitForHealth() {
  const deadline = Date.now() + 60_000;
  let lastError;
  while (Date.now() < deadline) {
    try {
      const response = await request('GET', '/v1/health');
      if (response.status === 200 && response.body?.ok === true) return;
    } catch (error) {
      lastError = error;
    }
    await sleep(500);
  }
  throw lastError ?? new Error('backend health timed out');
}

async function stopBackend(child) {
  if (!child || child.killed) return;
  child.kill();
  await sleep(300);
}

function renderMarkdown(result) {
  return [
    '# Enterprise Public Rights Runtime QA',
    '',
    `- runId: ${result.runId}`,
    `- watermarkUid: ${result.watermarkUid}`,
    `- apiKeyId: ${result.apiKeyId}`,
    `- quotaBalanceId: ${result.quotaBalanceId}`,
    `- unauthorizedStatus: ${result.unauthorizedStatus}`,
    `- successStatus: ${result.successStatus}`,
    `- quotaDeniedStatus: ${result.quotaDeniedStatus}`,
    `- quotaChargedUnits: ${result.quotaChargedUnits}`,
    `- legalConclusion: ${result.legalConclusion}`,
    `- firstResultStatus: ${result.firstResultStatus}`,
    `- secondResultErrorCode: ${result.secondResultErrorCode}`,
    `- completedAt: ${result.completedAt}`,
    '',
  ].join('\n');
}

function sleep(ms) {
  return new Promise((resolveSleep) => setTimeout(resolveSleep, ms));
}

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

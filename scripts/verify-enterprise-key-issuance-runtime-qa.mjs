#!/usr/bin/env node
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { spawn, spawnSync } from 'node:child_process';

const runId = String(Date.now());
const adminToken = `enterprise-issue-admin-${runId}`;
const hashSecret = `enterprise-issue-hash-secret-${runId}`;
const tmpRoot = join(tmpdir(), `hiddenshield-enterprise-key-issue-qa-${runId}`);
const dbPath = join(tmpRoot, 'feedback.sqlite');
const outputDir = resolve('tmp-ui-qa', 'enterprise-key-issuance-runtime', runId);
const qaJsonPath = join(outputDir, `enterprise-key-issuance-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `enterprise-key-issuance-runtime-qa-${runId}.md`);
mkdirSync(tmpRoot, { recursive: true });
mkdirSync(outputDir, { recursive: true });

const port = 43240 + Number(runId.slice(-3));
const baseUrl = `http://127.0.0.1:${port}`;
const backend = spawnBackend();

try {
  await waitForHealth();
  const payload = {
    accountId: `acct_issue_${runId}`,
    workspaceId: `ws_issue_${runId}`,
    name: 'Runtime QA scanner',
    scopes: ['public_rights:read', 'public_rights:batch_read'],
    createdByAccountId: 'admin_runtime_qa',
    reason: 'runtime qa issuance',
    deliveryChannel: 'secure_note',
    recipientRef: `qa-ticket-${runId}`,
  };
  const issued = runCliJson(['issue-api-key', '--json', JSON.stringify(payload)]);
  assert(issued.cleartextApiKey?.startsWith('hsent_live_'), 'issue response must include one-time cleartext API key');
  assert(issued.shownOnce === true, 'issue response must mark shownOnce=true');
  assert(issued.hashAlgorithm === 'hmac-sha256:v1:runtime-qa-v1', 'issue response must include hash algorithm and version');
  assert(issued.apiKey?.apiKeyId, 'issue response must include apiKey metadata');
  assert(issued.apiKey?.keyPrefix === issued.keyPrefix, 'issue response keyPrefix must match apiKey metadata');

  const rotated = runCliJson([
    'rotate-api-key',
    '--api-key-id',
    issued.apiKey.apiKeyId,
    '--json',
    JSON.stringify({
      createdByAccountId: 'admin_runtime_qa',
      reason: 'runtime qa scheduled rotation',
      gracePeriodHours: 1,
      deliveryChannel: 'secure_note',
      recipientRef: `qa-rotate-ticket-${runId}`,
    }),
  ]);
  assert(rotated.cleartextApiKey?.startsWith('hsent_live_'), 'rotate response must include one-time new cleartext API key');
  assert(rotated.shownOnce === true, 'rotate response must mark shownOnce=true');
  assert(rotated.oldApiKey?.status === 'paused', 'rotate response must pause old API key');
  assert(rotated.newApiKey?.status === 'active', 'rotate response must create active new API key');
  assert(rotated.newApiKey?.apiKeyId !== issued.apiKey.apiKeyId, 'rotate response must create a different key id');

  const beforeSweep = runCliJson([
    'revoke-expired-rotations',
    '--json',
    JSON.stringify({
      reason: 'runtime qa sweep before deadline',
      now: new Date(new Date(rotated.rotationDeadlineAt).getTime() - 1000).toISOString(),
      limit: 10,
    }),
  ]);
  assert(beforeSweep.processed === 0, 'sweep before rotationDeadlineAt must not revoke old key');

  const expiredSweep = runCliJson([
    'revoke-expired-rotations',
    '--json',
    JSON.stringify({
      reason: 'runtime qa grace period complete',
      now: new Date(new Date(rotated.rotationDeadlineAt).getTime() + 1000).toISOString(),
      limit: 10,
    }),
  ]);
  assert(expiredSweep.processed === 1, 'expired sweep must process one old rotated key');
  assert(expiredSweep.revoked === 1, 'expired sweep must revoke one old rotated key');
  assert(expiredSweep.items?.[0]?.oldApiKeyId === issued.apiKey.apiKeyId, 'expired sweep item must reference old key');
  assert(expiredSweep.items?.[0]?.newApiKeyId === rotated.newApiKey.apiKeyId, 'expired sweep item must reference new key');

  const list = runCliJson(['list-api-keys', '--account-id', payload.accountId, '--status', 'active']);
  const getOld = runCliJson(['get-api-key', '--api-key-id', issued.apiKey.apiKeyId]);
  const getNew = runCliJson(['get-api-key', '--api-key-id', rotated.newApiKey.apiKeyId]);
  const issueAudit = runCliJson(['list-admin-audit-events', '--operation', 'issue_api_key', '--outcome', 'succeeded', '--limit', '10']);
  const rotateAudit = runCliJson(['list-admin-audit-events', '--operation', 'rotate_api_key', '--outcome', 'succeeded', '--limit', '10']);
  const revokeAudit = runCliJson(['list-admin-audit-events', '--operation', 'revoke_api_key', '--outcome', 'succeeded', '--limit', '10']);
  const sweepAudit = runCliJson(['list-admin-audit-events', '--operation', 'revoke_expired_rotations', '--outcome', 'succeeded', '--limit', '10']);
  const combinedFollowup = JSON.stringify({ list, getOld, getNew, issueAudit, rotateAudit, revokeAudit, sweepAudit });
  assert(!combinedFollowup.includes(issued.cleartextApiKey), 'list/get/audit must not contain issued cleartext API key');
  assert(!combinedFollowup.includes(rotated.cleartextApiKey), 'list/get/audit must not contain rotated cleartext API key');
  assert(!combinedFollowup.includes('keyHash'), 'list/get/audit must not expose keyHash');
  assert(getOld.status === 'revoked', 'old key must be revoked after explicit revoke step');
  assert(getNew.status === 'active', 'new rotated key must remain active');
  assert(issueAudit.events?.some((event) => event.apiKeyId === issued.apiKey.apiKeyId), 'issue_api_key audit event must be recorded');
  assert(issueAudit.events.every((event) => event.endpoint === '/internal/enterprise/api-key-issuances'), 'issue audit endpoint must be internal issuance route');
  assert(rotateAudit.events?.some((event) => event.apiKeyId === issued.apiKey.apiKeyId && event.targetId === rotated.newApiKey.apiKeyId), 'rotate_api_key audit event must link old and new keys');
  assert(rotateAudit.events.every((event) => event.endpoint === '/internal/enterprise/api-keys/:api_key_id/rotate'), 'rotate audit endpoint must be internal route');
  assert(revokeAudit.events?.some((event) => event.apiKeyId === issued.apiKey.apiKeyId), 'revoke_api_key audit event must complete old key chain');
  assert(sweepAudit.events?.some((event) => Number(event.details?.revoked) === 1), 'revoke_expired_rotations audit event must summarize revoked count');

  const externalRouteProbe = await fetch(`${baseUrl}/v1/enterprise/public-rights/batch`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ watermarkUids: [`wm_probe_${runId}`] }),
  });
  assert(externalRouteProbe.status === 401, 'external batch route must reject requests without an API key');

  const result = {
    runId,
    baseUrl,
    issuedApiKeyId: issued.apiKey.apiKeyId,
    rotatedApiKeyId: rotated.newApiKey.apiKeyId,
    keyPrefix: rotated.keyPrefix,
    hashAlgorithm: rotated.hashAlgorithm,
    shownOnce: issued.shownOnce && rotated.shownOnce,
    oldKeyFinalStatus: getOld.status,
    newKeyFinalStatus: getNew.status,
    sweepBeforeDeadlineProcessed: beforeSweep.processed,
    expiredSweepProcessed: expiredSweep.processed,
    expiredSweepRevoked: expiredSweep.revoked,
    followupContainsCleartext:
      combinedFollowup.includes(issued.cleartextApiKey) || combinedFollowup.includes(rotated.cleartextApiKey),
    followupContainsKeyHash: combinedFollowup.includes('keyHash'),
    issueAuditEvents: issueAudit.events?.length ?? 0,
    rotateAuditEvents: rotateAudit.events?.length ?? 0,
    revokeAuditEvents: revokeAudit.events?.length ?? 0,
    sweepAuditEvents: sweepAudit.events?.length ?? 0,
    externalRouteStatus: externalRouteProbe.status,
  };
  writeFileSync(qaJsonPath, JSON.stringify(result, null, 2));
  writeFileSync(
    qaMdPath,
    [
      '# Enterprise API Key Issuance Runtime QA',
      '',
      `- runId: ${runId}`,
      `- baseUrl: ${baseUrl}`,
      `- issuedApiKeyId: ${issued.apiKey.apiKeyId}`,
      `- rotatedApiKeyId: ${rotated.newApiKey.apiKeyId}`,
      `- keyPrefix: ${rotated.keyPrefix}`,
      `- hashAlgorithm: ${rotated.hashAlgorithm}`,
      `- shownOnce: ${result.shownOnce}`,
      `- oldKeyFinalStatus: ${result.oldKeyFinalStatus}`,
      `- newKeyFinalStatus: ${result.newKeyFinalStatus}`,
      `- sweepBeforeDeadlineProcessed: ${result.sweepBeforeDeadlineProcessed}`,
      `- expiredSweepProcessed: ${result.expiredSweepProcessed}`,
      `- expiredSweepRevoked: ${result.expiredSweepRevoked}`,
      `- follow-up list/get/audit contains cleartext: ${result.followupContainsCleartext}`,
      `- follow-up list/get/audit contains keyHash: ${result.followupContainsKeyHash}`,
      `- issue_api_key audit events: ${result.issueAuditEvents}`,
      `- rotate_api_key audit events: ${result.rotateAuditEvents}`,
      `- revoke_api_key audit events: ${result.revokeAuditEvents}`,
      `- revoke_expired_rotations audit events: ${result.sweepAuditEvents}`,
      `- external /v1/enterprise/public-rights/batch probe status: ${result.externalRouteStatus}`,
    ].join('\n'),
  );
  console.log(`Enterprise key issuance runtime QA OK: ${qaMdPath}`);
} finally {
  await stopBackend(backend);
  rmSync(tmpRoot, { recursive: true, force: true });
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
  const command = [
    backendBin,
    '--bind-addr',
    `127.0.0.1:${port}`,
    '--db-path',
    dbPath,
    '--commercial-metrics-admin-token',
    adminToken,
    '--enterprise-api-key-hash-secret',
    hashSecret,
    '--enterprise-api-key-hash-secret-version',
    'runtime-qa-v1',
  ];
  const child = spawn(command[0], command.slice(1), {
    cwd: process.cwd(),
    env: { ...process.env, RUST_BACKTRACE: '1' },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
  let stderr = '';
  child.stderr.on('data', (chunk) => {
    stderr += String(chunk);
  });
  child.on('exit', (code) => {
    if (code !== null && code !== 0) {
      console.error(stderr);
    }
  });
  return child;
}

async function stopBackend(child) {
  if (!child || child.exitCode !== null || child.killed) {
    return;
  }
  const exited = new Promise((resolveStop) => {
    child.once('exit', () => resolveStop(true));
  });
  child.kill();
  await Promise.race([
    exited,
    new Promise((resolveStop) => setTimeout(() => resolveStop(false), 5000)),
  ]);
}

async function waitForHealth() {
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`${baseUrl}/healthz`);
      if (response.ok) {
        return;
      }
    } catch {
      // Keep polling until the backend binds.
    }
    await new Promise((resolveDelay) => setTimeout(resolveDelay, 500));
  }
  throw new Error(`backend did not become healthy at ${baseUrl}`);
}

function runCliJson(args) {
  const result = spawnSync('node', ['scripts/enterprise-internal-admin.mjs', ...args], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      HIDDENSHIELD_INTERNAL_ADMIN_BASE_URL: baseUrl,
      HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN: adminToken,
    },
    encoding: 'utf8',
    windowsHide: true,
  });
  if (result.status !== 0) {
    throw new Error(`CLI failed: ${args.join(' ')}\n${result.stdout}\n${result.stderr}`);
  }
  return JSON.parse(result.stdout);
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

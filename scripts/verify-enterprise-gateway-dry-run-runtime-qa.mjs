import { execFileSync, spawn } from 'node:child_process';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import net from 'node:net';

const runId = process.env.HIDDENSHIELD_ENTERPRISE_GATEWAY_DRY_RUN_QA_RUN_ID ?? `${Date.now()}`;
const adminToken =
  process.env.HIDDENSHIELD_ENTERPRISE_GATEWAY_DRY_RUN_ADMIN_TOKEN ?? 'secret-admin-token';
const endpoint = process.env.HIDDENSHIELD_ENTERPRISE_GATEWAY_DRY_RUN_BASE_URL?.replace(/\/$/, '');
const shouldStartBackend = !endpoint;
const port = shouldStartBackend ? await freePort() : Number(new URL(endpoint).port || 80);
const baseUrl = endpoint ?? `http://127.0.0.1:${port}`;
const tmpRoot = join(tmpdir(), `hiddenshield-enterprise-gateway-dry-run-qa-${runId}`);
const dbPath = join(tmpRoot, 'cloud.sqlite');
const outputDir = resolve('tmp-ui-qa', 'enterprise-gateway-dry-run-runtime', runId);
const qaJsonPath = join(outputDir, `enterprise-gateway-dry-run-runtime-qa-${runId}.json`);
const qaMdPath = join(outputDir, `enterprise-gateway-dry-run-runtime-qa-${runId}.md`);

mkdirSync(tmpRoot, { recursive: true });
mkdirSync(outputDir, { recursive: true });

let backend;
try {
  if (shouldStartBackend) {
    backend = spawn(
      'cargo',
      [
        'run',
        '--manifest-path',
        'feedback-backend/Cargo.toml',
        '--bin',
        'hiddenshield-feedback-backend',
        '--',
        '--bind-addr',
        `127.0.0.1:${port}`,
        '--db-path',
        dbPath,
        '--commercial-metrics-admin-token',
        adminToken,
      ],
      { cwd: process.cwd(), stdio: ['ignore', 'pipe', 'pipe'], windowsHide: true },
    );
    backend.stdout.on('data', (chunk) => process.stdout.write(`[backend] ${chunk}`));
    backend.stderr.on('data', (chunk) => process.stderr.write(`[backend] ${chunk}`));
  }

  await waitForHealth(baseUrl);
  const cases = buildCases();
  const results = [];
  for (const item of cases) {
    const decision = runCliJson(['dry-run-gateway', '--json', JSON.stringify(item.payload)]);
    assertDecision(item, decision);
    results.push({
      key: item.key,
      label: item.label,
      requestId: item.payload.requestId,
      expected: {
        allowed: item.allowed,
        statusCode: item.statusCode,
        errorCode: item.errorCode,
      },
      decision,
    });
  }

  const audit = runCliJson(['list-admin-audit-events', '--operation', 'dry_run_gateway', '--limit', '20']);
  assert(audit.returned >= cases.length, 'dry-run gateway admin audit must contain every runtime case');
  for (const item of cases) {
    const event = audit.events.find((candidate) => candidate.targetId === item.payload.requestId);
    assert(event, `${item.key} must write dry_run_gateway admin audit event`);
    assert(event.endpoint === '/internal/enterprise/gateway-dry-run', `${item.key} audit endpoint must be internal`);
    assert(event.outcome === 'succeeded', `${item.key} audit outcome must be succeeded`);
    assert(
      event.details?.statusCode === item.statusCode,
      `${item.key} audit status code must match dry-run decision`,
    );
    assert(
      (event.details?.errorCode ?? null) === item.errorCode,
      `${item.key} audit error code must match dry-run decision`,
    );
    assert(event.details?.legalConclusion === false, `${item.key} audit must not carry legal conclusion`);
  }

  const result = {
    runId,
    baseUrl,
    startedBackend: shouldStartBackend,
    dbPath: shouldStartBackend ? dbPath : null,
    cases: results,
    audit: {
      returned: audit.returned,
      matchedRequestIds: cases.map((item) => item.payload.requestId),
    },
    completedAt: new Date().toISOString(),
  };
  writeFileSync(qaJsonPath, `${JSON.stringify(result, null, 2)}\n`, 'utf8');
  writeFileSync(qaMdPath, renderMarkdown(result), 'utf8');
  console.log('Enterprise gateway dry-run runtime QA OK');
  console.log(`QA JSON: ${qaJsonPath}`);
  console.log(`QA Markdown: ${qaMdPath}`);
} finally {
  if (backend && !backend.killed) {
    await stopChild(backend);
  }
  if (shouldStartBackend) {
    await removeBestEffort(tmpRoot);
  }
}

function buildCases() {
  const base = {
    auth: {
      apiKeyId: `eak_dry_run_${runId}`,
      accountId: 'acct_enterprise',
      workspaceId: 'ws_enterprise',
      keyPrefix: 'hsent_live',
      scopes: ['public_rights:batch_read'],
      status: 'active',
      apiAccess: true,
    },
    requiredScope: 'public_rights:batch_read',
    endpoint: '/v1/enterprise/public-rights/batch',
    method: 'POST',
    requestId: `req_${runId}_success`,
    itemCount: 2,
    quotaType: 'public_rights_scan_units',
    quotaIncludedUnits: 100,
    quotaUsedUnits: 10,
    quotaReservedUnits: 0,
    quotaOverageAllowed: false,
    rateLimit: {
      policyId: 'enterprise_public_rights_default',
      requestsPerMinute: 60,
      itemsPerMinute: 600,
      burstRequests: 10,
      retryAfterSeconds: 60,
    },
    clientFingerprint: {
      fingerprintHash: 'sha256:dry-run-client-fingerprint',
      source: 'trusted_proxy_x_hiddenshield_client_fingerprint',
      trustedProxy: true,
      rateLimitSubject: `eak_dry_run_${runId}:sha256:dry-run-client-fingerprint`,
    },
    currentWindowRequests: 1,
    currentWindowItems: 10,
    chargeOnNotFound: false,
    chargeMetadataExport: false,
  };

  return [
    {
      key: 'success',
      label: 'success path',
      payload: clone(base),
      allowed: true,
      statusCode: 200,
      errorCode: null,
      quotaDecision: 'passed',
      chargeableUnits: 2,
      ledgerStatus: 'committed',
    },
    {
      key: 'scope_denied',
      label: 'scope denied',
      payload: withPatch(base, {
        requestId: `req_${runId}_scope_denied`,
        requiredScope: 'public_rights:metadata_export',
      }),
      allowed: false,
      statusCode: 403,
      errorCode: 'scope_denied',
      quotaDecision: 'not_evaluated',
      chargeableUnits: 0,
      ledgerStatus: 'skipped',
    },
    {
      key: 'api_access_disabled',
      label: 'api_access disabled',
      payload: withPatch(base, {
        requestId: `req_${runId}_api_access_disabled`,
        auth: { ...base.auth, apiAccess: false },
      }),
      allowed: false,
      statusCode: 403,
      errorCode: 'api_access_disabled',
      quotaDecision: 'not_evaluated',
      chargeableUnits: 0,
      ledgerStatus: 'skipped',
    },
    {
      key: 'rate_limited',
      label: 'rate limited',
      payload: withPatch(base, {
        requestId: `req_${runId}_rate_limited`,
        currentWindowRequests: 70,
      }),
      allowed: false,
      statusCode: 429,
      errorCode: 'rate_limited',
      quotaDecision: 'not_evaluated',
      chargeableUnits: 0,
      ledgerStatus: 'skipped',
    },
    {
      key: 'quota_exhausted',
      label: 'quota exhausted',
      payload: withPatch(base, {
        requestId: `req_${runId}_quota_exhausted`,
        quotaIncludedUnits: 11,
        quotaUsedUnits: 10,
        quotaReservedUnits: 0,
      }),
      allowed: false,
      statusCode: 402,
      errorCode: 'quota_exhausted',
      quotaDecision: 'failed:quota_exhausted',
      chargeableUnits: 0,
      ledgerStatus: 'skipped',
    },
    {
      key: 'api_key_revoked',
      label: 'revoked API key',
      payload: withPatch(base, {
        requestId: `req_${runId}_api_key_revoked`,
        auth: { ...base.auth, status: 'revoked' },
      }),
      allowed: false,
      statusCode: 403,
      errorCode: 'api_key_revoked',
      quotaDecision: 'not_evaluated',
      chargeableUnits: 0,
      ledgerStatus: 'skipped',
    },
  ];
}

function assertDecision(item, decision) {
  assert(decision.allowed === item.allowed, `${item.key} allowed must be ${item.allowed}`);
  assert(decision.statusCode === item.statusCode, `${item.key} statusCode must be ${item.statusCode}`);
  assert((decision.errorCode ?? null) === item.errorCode, `${item.key} errorCode must match`);
  assert(decision.legalConclusion === false, `${item.key} legalConclusion must stay false`);
  assert(decision.audit?.legalConclusion === false, `${item.key} audit legalConclusion must stay false`);
  assert(decision.audit?.requestId === item.payload.requestId, `${item.key} audit requestId must match`);
  assert(decision.audit?.endpoint === item.payload.endpoint, `${item.key} audit endpoint must preserve simulated endpoint`);
  assert(decision.quota?.chargeableUnits === item.chargeableUnits, `${item.key} chargeableUnits must match`);
  assert(decision.quota?.ledgerStatus === item.ledgerStatus, `${item.key} ledgerStatus must match`);
  assert(decision.quotaDecision === item.quotaDecision, `${item.key} quotaDecision must match`);
  assert(
    Array.isArray(decision.requiredSteps) &&
      decision.requiredSteps.includes('authenticate_api_key') &&
      decision.requiredSteps.includes('record_api_audit_event'),
    `${item.key} must return required gateway steps`,
  );
}

function runCliJson(args) {
  const output = execFileSync('node', ['scripts/enterprise-internal-admin.mjs', ...args], {
    cwd: process.cwd(),
    env: {
      ...process.env,
      HIDDENSHIELD_INTERNAL_ADMIN_BASE_URL: baseUrl,
      HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN: adminToken,
    },
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'pipe'],
  });
  return JSON.parse(output);
}

function withPatch(value, patch) {
  return { ...clone(value), ...patch };
}

function clone(value) {
  return JSON.parse(JSON.stringify(value));
}

async function waitForHealth(url) {
  const deadline = Date.now() + 60_000;
  while (Date.now() < deadline) {
    try {
      const response = await fetch(`${url}/healthz`);
      if (response.ok) {
        return;
      }
    } catch {
      // Retry until the backend finishes compiling and starts listening.
    }
    await sleep(500);
  }
  throw new Error(`Backend did not become healthy at ${url}`);
}

function freePort() {
  return new Promise((resolvePromise, reject) => {
    const server = net.createServer();
    server.listen(0, '127.0.0.1', () => {
      const address = server.address();
      const port = typeof address === 'object' && address ? address.port : null;
      server.close(() => {
        if (port) {
          resolvePromise(port);
        } else {
          reject(new Error('Unable to allocate free port'));
        }
      });
    });
    server.on('error', reject);
  });
}

function sleep(ms) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, ms));
}

function stopChild(child) {
  return new Promise((resolvePromise) => {
    if (!child || child.killed || child.exitCode !== null) {
      resolvePromise();
      return;
    }
    const timeout = setTimeout(() => resolvePromise(), 5_000);
    child.once('exit', () => {
      clearTimeout(timeout);
      resolvePromise();
    });
    child.kill();
  });
}

async function removeBestEffort(path) {
  for (let attempt = 0; attempt < 5; attempt += 1) {
    try {
      rmSync(path, { recursive: true, force: true });
      return;
    } catch (error) {
      if (error?.code !== 'EBUSY' && error?.code !== 'EPERM') {
        throw error;
      }
      await sleep(250 * (attempt + 1));
    }
  }
  console.warn(`warning: temporary QA directory is still locked and was left for OS cleanup: ${path}`);
}

function renderMarkdown(result) {
  const rows = result.cases
    .map(
      (item) =>
        `| ${item.key} | ${item.decision.allowed} | ${item.decision.statusCode} | ${
          item.decision.errorCode ?? ''
        } | ${item.decision.quota.ledgerStatus} | ${item.decision.quota.chargeableUnits} |`,
    )
    .join('\n');
  return `# Enterprise Gateway Dry-run Runtime QA

- Run ID: ${result.runId}
- Base URL: ${result.baseUrl}
- Started backend: ${result.startedBackend}
- Completed at: ${result.completedAt}
- Audit events matched: ${result.audit.matchedRequestIds.length}

| Case | Allowed | Status | Error | Ledger | Units |
| --- | --- | --- | --- | --- | --- |
${rows}
`;
}

function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

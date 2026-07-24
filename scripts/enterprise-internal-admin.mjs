#!/usr/bin/env node
import { readFileSync } from 'node:fs';

const DEFAULT_BASE_URL = 'http://127.0.0.1:8787';

const command = process.argv[2];
const args = parseArgs(process.argv.slice(3));

if (!command || command === '--help' || command === '-h' || args.help) {
  printHelp();
  process.exit(command ? 0 : 1);
}

const baseUrl = (args.baseUrl || process.env.HIDDENSHIELD_INTERNAL_ADMIN_BASE_URL || DEFAULT_BASE_URL)
  .replace(/\/+$/, '');
const adminToken = args.adminToken || process.env.HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN;

if (!adminToken) {
  fail('Missing admin token. Pass --admin-token or set HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN.');
}

const payload = args.json ? JSON.parse(args.json) : args.jsonFile ? JSON.parse(readFileSync(args.jsonFile, 'utf8')) : null;

switch (command) {
  case 'issue-api-key':
    await postInternal(`${baseUrl}/internal/enterprise/api-key-issuances`, requirePayload(payload, command));
    break;
  case 'create-api-key':
    await postInternal(`${baseUrl}/internal/enterprise/api-keys`, requirePayload(payload, command));
    break;
  case 'list-api-keys':
    await getInternal(`${baseUrl}/internal/enterprise/api-keys${buildApiKeyListQuery(args)}`);
    break;
  case 'get-api-key':
    await getInternal(`${baseUrl}/internal/enterprise/api-keys/${encodeURIComponent(requireArg(args.apiKeyId, '--api-key-id'))}`);
    break;
  case 'pause-api-key':
    await postInternal(
      `${baseUrl}/internal/enterprise/api-keys/${encodeURIComponent(requireArg(args.apiKeyId, '--api-key-id'))}/pause`,
      { reason: requireArg(args.reason, '--reason') },
    );
    break;
  case 'rotate-api-key':
    await postInternal(
      `${baseUrl}/internal/enterprise/api-keys/${encodeURIComponent(requireArg(args.apiKeyId, '--api-key-id'))}/rotate`,
      requirePayload(payload, command),
    );
    break;
  case 'revoke-expired-rotations':
    await postInternal(`${baseUrl}/internal/enterprise/api-key-rotations/revoke-expired`, requirePayload(payload, command));
    break;
  case 'revoke-api-key':
    await postInternal(
      `${baseUrl}/internal/enterprise/api-keys/${encodeURIComponent(requireArg(args.apiKeyId, '--api-key-id'))}/revoke`,
      { reason: requireArg(args.reason, '--reason') },
    );
    break;
  case 'init-quota-balance':
    await postInternal(`${baseUrl}/internal/enterprise/quota-balances`, requirePayload(payload, command));
    break;
  case 'list-admin-audit-events':
    await getInternal(`${baseUrl}/internal/enterprise/admin-audit-events${buildAdminAuditEventListQuery(args)}`);
    break;
  case 'dry-run-gateway':
    await postInternal(`${baseUrl}/internal/enterprise/gateway-dry-run`, requirePayload(payload, command));
    break;
  default:
    fail(`Unknown command: ${command}`);
}

async function getInternal(url) {
  if (url.includes('/v1/enterprise/')) {
    fail('Refusing to call external Enterprise API route from internal admin CLI.');
  }
  const response = await fetch(url, {
    method: 'GET',
    headers: {
      authorization: `Bearer ${adminToken}`,
    },
  });
  await printResponse(response);
}

async function postInternal(url, body) {
  if (url.includes('/v1/enterprise/')) {
    fail('Refusing to call external Enterprise API route from internal admin CLI.');
  }
  const response = await fetch(url, {
    method: 'POST',
    headers: {
      authorization: `Bearer ${adminToken}`,
      'content-type': 'application/json',
    },
    body: JSON.stringify(body),
  });
  await printResponse(response);
}

async function printResponse(response) {
  const text = await response.text();
  let parsed = null;
  try {
    parsed = text ? JSON.parse(text) : null;
  } catch {
    parsed = { raw: text };
  }
  if (!response.ok) {
    console.error(JSON.stringify(parsed, null, 2));
    process.exit(response.status);
  }
  console.log(JSON.stringify(parsed, null, 2));
}

function parseArgs(values) {
  const parsed = {};
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value === '--help' || value === '-h') {
      parsed.help = true;
    } else if (value === '--base-url') {
      parsed.baseUrl = values[++index];
    } else if (value === '--admin-token') {
      parsed.adminToken = values[++index];
    } else if (value === '--json') {
      parsed.json = values[++index];
    } else if (value === '--json-file') {
      parsed.jsonFile = values[++index];
    } else if (value === '--api-key-id') {
      parsed.apiKeyId = values[++index];
    } else if (value === '--account-id') {
      parsed.accountId = values[++index];
    } else if (value === '--workspace-id') {
      parsed.workspaceId = values[++index];
    } else if (value === '--status') {
      parsed.status = values[++index];
    } else if (value === '--operation') {
      parsed.operation = values[++index];
    } else if (value === '--outcome') {
      parsed.outcome = values[++index];
    } else if (value === '--from-occurred-at') {
      parsed.fromOccurredAt = values[++index];
    } else if (value === '--to-occurred-at') {
      parsed.toOccurredAt = values[++index];
    } else if (value === '--limit') {
      parsed.limit = values[++index];
    } else if (value === '--reason') {
      parsed.reason = values[++index];
    } else {
      fail(`Unknown argument: ${value}`);
    }
  }
  return parsed;
}

function requirePayload(payload, commandName) {
  if (!payload || typeof payload !== 'object' || Array.isArray(payload)) {
    fail(`${commandName} requires --json or --json-file with a JSON object payload.`);
  }
  return payload;
}

function buildApiKeyListQuery(values) {
  const params = new URLSearchParams();
  appendQuery(params, 'accountId', values.accountId);
  appendQuery(params, 'workspaceId', values.workspaceId);
  appendQuery(params, 'status', values.status);
  appendQuery(params, 'limit', values.limit);
  const query = params.toString();
  return query ? `?${query}` : '';
}

function buildAdminAuditEventListQuery(values) {
  const params = new URLSearchParams();
  appendQuery(params, 'operation', values.operation);
  appendQuery(params, 'outcome', values.outcome);
  appendQuery(params, 'accountId', values.accountId);
  appendQuery(params, 'apiKeyId', values.apiKeyId);
  appendQuery(params, 'fromOccurredAt', values.fromOccurredAt);
  appendQuery(params, 'toOccurredAt', values.toOccurredAt);
  appendQuery(params, 'limit', values.limit);
  const query = params.toString();
  return query ? `?${query}` : '';
}

function appendQuery(params, key, value) {
  if (value !== undefined && String(value).trim() !== '') {
    params.set(key, String(value).trim());
  }
}

function requireArg(value, label) {
  if (value === undefined || String(value).trim() === '') {
    fail(`Missing required ${label}.`);
  }
  return String(value).trim();
}

function fail(message) {
  console.error(`enterprise-internal-admin: ${message}`);
  process.exit(1);
}

function printHelp() {
  console.log(`HiddenShield Enterprise internal admin CLI

Usage:
  node scripts/enterprise-internal-admin.mjs create-api-key --json-file payload.json
  node scripts/enterprise-internal-admin.mjs issue-api-key --json '{"accountId":"acct","workspaceId":"ws","name":"Vendor scanner","scopes":["public_rights:read"],"createdByAccountId":"admin","reason":"customer onboarding","deliveryChannel":"secure_note","recipientRef":"ticket-123"}'
  node scripts/enterprise-internal-admin.mjs list-api-keys --account-id acct --status active
  node scripts/enterprise-internal-admin.mjs get-api-key --api-key-id eak_xxx
  node scripts/enterprise-internal-admin.mjs pause-api-key --api-key-id eak_xxx --reason "contract review"
  node scripts/enterprise-internal-admin.mjs rotate-api-key --api-key-id eak_xxx --json '{"createdByAccountId":"admin","reason":"scheduled rotation","gracePeriodHours":24,"deliveryChannel":"secure_note","recipientRef":"ticket-456"}'
  node scripts/enterprise-internal-admin.mjs revoke-expired-rotations --json '{"reason":"scheduled rotation grace period complete","limit":100}'
  node scripts/enterprise-internal-admin.mjs revoke-api-key --api-key-id eak_xxx --reason "customer offboarded"
  node scripts/enterprise-internal-admin.mjs init-quota-balance --json '{"accountId":"acct","workspaceId":"ws","quotaType":"public_rights_scan_units","periodStart":"2026-07-01T00:00:00Z","periodEnd":"2026-08-01T00:00:00Z","includedUnits":10000,"overageAllowed":false,"overageUnitPriceCents":null,"currency":"CNY"}'
  node scripts/enterprise-internal-admin.mjs list-admin-audit-events --operation create_api_key --outcome succeeded --limit 50
  node scripts/enterprise-internal-admin.mjs dry-run-gateway --json-file gateway-dry-run.json

Environment:
  HIDDENSHIELD_INTERNAL_ADMIN_BASE_URL      default ${DEFAULT_BASE_URL}
  HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN
  HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET
  HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET_VERSION

Notes:
  - Calls only /internal/enterprise/... admin endpoints.
  - issue-api-key is the only command that returns a cleartext API key, and it returns it once.
  - create-api-key only registers existing keyHash/keyPrefix generated by a separate custody process.
  - list-admin-audit-events is read-only and does not write another Enterprise admin audit event.
  - dry-run-gateway calls only the internal gateway dry-run endpoint and does not write quota ledger or open /v1/enterprise/... routes.
`);
}

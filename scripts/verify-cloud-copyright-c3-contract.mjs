import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile, readdir } from "node:fs/promises";

const root = "docs/contracts/cloud-copyright";
const identity = await readJson(`${root}/c3-identity-receipt-v1.fixture.json`);
const failures = await readJson(`${root}/c3-identity-receipt-failures-v1.fixture.json`);
const boundaries = await readJson(`${root}/c3-identity-receipt-boundaries-v1.fixture.json`);
const transport = await readJson(`${root}/c3-transport-fail-closed-v1.fixture.json`);
const schema = await readJson(`${root}/cloud-copyright-c3-contract-v1.schema.json`);
const rlsPolicy = await readFile(`${root}/c3-rls-policy-v1.sql`, "utf8");
const executableRlsPolicy = rlsPolicy.replace(/^--.*$/gm, "");
const backendRouter = await readFile("feedback-backend/src/lib.rs", "utf8");
const repository = await readFile("feedback-backend/src/cloud_copyright.rs", "utf8");

assert.equal(schema.$id, "https://hiddenshield.local/contracts/cloud-copyright/cloud-copyright-c3-contract-v1.schema.json");
for (const fixture of [identity, failures, boundaries, transport]) {
  assert.equal(fixture.schemaVersion, 1);
  assert.ok(schema.properties.fixtureType.enum.includes(fixture.fixtureType));
}

assert.equal(identity.canonicalization, "hs-cloud-copyright-identity-receipt-digest-v1");
assert.equal(
  identity.receiptDigest,
  receiptDigest(identity.receipt),
  "identity receipt digest must bind canonical receipt claims excluding signature",
);
for (const field of ["providerId", "receiptId", "audience", "operation", "requestId", "requestDigest", "issuedAt", "expiresAt"]) {
  assert.equal(typeof identity.receipt[field], "string", `missing receipt ${field}`);
}
for (const field of ["actorId", "accountId", "workspaceId", "deviceId", "membershipId"]) {
  assert.equal(typeof identity.receipt.actor[field], "string", `missing receipt actor.${field}`);
}
assert.ok(identity.expected.scopeKeys.includes("app.receipt_digest"));

assert.equal(boundaries.canonicalization, identity.canonicalization);
assert.equal(boundaries.cases.length, 6);
for (const caseId of [
  "key_order_independent_digest",
  "maximum_lifetime_boundary",
  "lifetime_exceeds_limit",
  "clock_skew_boundary",
  "clock_skew_exceeded",
  "receipt_replay_identity",
]) {
  assert.ok(boundaries.cases.some((item) => item.caseId === caseId), `missing receipt boundary ${caseId}`);
}
for (const item of boundaries.cases.filter((candidate) => candidate.expected.admitted === false)) {
  assert.equal(item.expected.databaseWrites, 0, `${item.caseId} must fail before database writes`);
  assert.equal(typeof item.expected.reasonCode, "string");
}

assert.equal(failures.expected.admitted, false);
assert.equal(failures.expected.databaseWrites, 0);
assert.equal(failures.expected.localState, "draft_retained");
assert.equal(failures.expected.retryAllowed, false);
for (const expectedCase of [
  "invalid_signature",
  "expired_receipt",
  "scope_mismatch",
  "request_digest_mismatch",
  "provider_unavailable",
]) {
  assert.ok(failures.cases.some((item) => item.caseId === expectedCase), `missing failure ${expectedCase}`);
}

assert.equal(transport.recordSchema, "cloud-copyright-record-v1");
for (const endpoint of [transport.expected.desktop, transport.expected.android]) {
  assert.equal(endpoint.outboxState, "draft_retained");
  assert.equal(endpoint.autoRetry, false);
  assert.match(endpoint.userMessage, /未标记为已备份/);
}
assert.equal(
  transport.expected.desktop.userMessage,
  transport.expected.android.userMessage,
  "desktop and Android fail-closed user wording must match",
);

lintRlsPolicy(rlsPolicy);

assert.match(backendRouter, /\.route\("\/v1\/sync\/events:batch"/);
assert.match(backendRouter, /\.route\("\/v1\/sync\/changes"/);
assert.doesNotMatch(backendRouter, /\/(?:v1|internal)\/cloud-copyright/);
assert.doesNotMatch(repository, /\baxum::|\bRouter\b|\.route\(/);

const postgresMigrations = await readdir("feedback-backend/migrations/postgres");
assert.equal(
  postgresMigrations.some((name) => /^0024_.*cloud_copyright/i.test(name)),
  false,
  "C3 contract freeze must not create 0024 migration",
);

console.log(JSON.stringify({
  ok: true,
  contractVersion: "cloud-copyright-c3-contract-v1",
  fixtures: 4,
  guarantees: ["canonical_identity_digest", "receipt_boundary_fail_closed", "cross_end_draft_retention", "rls_static_safety"],
}));

export function lintRlsPolicy(rlsPolicy) {
  const executableRlsPolicy = rlsPolicy.replace(/^--.*$/gm, "");
  for (const table of [
  "cloud_copyright_records",
  "cloud_copyright_changes",
  "cloud_copyright_events",
  "cloud_copyright_audit_events",
  "cloud_copyright_workspace_cursors",
]) {
  assert.match(rlsPolicy, new RegExp(`ALTER TABLE ${table} ENABLE ROW LEVEL SECURITY;`));
  assert.match(rlsPolicy, new RegExp(`ALTER TABLE ${table} FORCE ROW LEVEL SECURITY;`));
}
for (const scopeKey of ["app.account_id", "app.workspace_id", "app.device_id", "app.membership_id"]) {
  assert.ok(rlsPolicy.includes(scopeKey), `missing RLS scope ${scopeKey}`);
}
assert.match(rlsPolicy, /set_config\(\$1, \$2, true\)/);
  assert.doesNotMatch(executableRlsPolicy, /(?<!NO)BYPASSRLS/);
  assert.doesNotMatch(executableRlsPolicy, /\bSET ROLE\b/);
  assert.doesNotMatch(executableRlsPolicy, /\bGRANT\b[\s\S]*\bPUBLIC\b/);
  assert.doesNotMatch(executableRlsPolicy, /set_config\([^)]*,\s*false\)/);
}

function receiptDigest(receipt) {
  const { signature: _signature, ...claims } = receipt;
  return `sha256:${createHash("sha256").update(canonicalize(claims)).digest("hex")}`;
}

function canonicalize(value) {
  if (Array.isArray(value)) {
    return `[${value.map(canonicalize).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalize(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

import assert from "node:assert/strict";
import { readFile, readdir } from "node:fs/promises";

const root = "docs/contracts/cloud-copyright";
const fixtureNames = [
  "c2-transport-mapping-v1.fixture.json",
  "c2-postgres-request-scope-v1.fixture.json",
  "c2-internal-api-admission-v1.fixture.json",
];

const schema = await readJson(`${root}/cloud-copyright-c2-contract-v1.schema.json`);
const fixtures = Object.fromEntries(
  await Promise.all(fixtureNames.map(async (name) => [name, await readJson(`${root}/${name}`)])),
);

assert.equal(schema.$id, "https://hiddenshield.local/contracts/cloud-copyright/cloud-copyright-c2-contract-v1.schema.json");
for (const fixture of Object.values(fixtures)) {
  assert.equal(fixture.schemaVersion, 1, "C2 fixture schema version must be 1");
  assert.ok(schema.properties.fixtureType.enum.includes(fixture.fixtureType), "fixture type must be declared");
}

const transport = fixtures["c2-transport-mapping-v1.fixture.json"];
assert.equal(transport.recordSchema, "cloud-copyright-record-v1");
for (const endpoint of [transport.desktop, transport.mobile]) {
  assert.equal(typeof endpoint.outboxEnvelope.workspaceId, "string");
  assert.equal(typeof endpoint.outboxEnvelope.recordId, "string");
  assert.equal(typeof endpoint.outboxEnvelope.baseRecordVersion, "number");
  assert.equal(typeof endpoint.outboxEnvelope.idempotencyKey, "string");
  assert.match(endpoint.outboxEnvelope.requestDigest, /^sha256:/);
}
for (const error of ["conflict_version_changed", "blocked_by_membership_revoked"]) {
  assert.equal(transport.expectedResult[error].localState, "draft_retained");
  assert.equal(transport.expectedResult[error].retryAllowed, false);
}

const scope = fixtures["c2-postgres-request-scope-v1.fixture.json"];
assert.equal(scope.transactionScope.source, "verified_internal_identity");
assert.equal(scope.transactionScope.scopeLifetime, "transaction_local");
for (const name of ["app.account_id", "app.workspace_id", "app.device_id", "app.membership_id", "app.request_id"]) {
  assert.ok(scope.transactionScope.setConfig.includes(name), `missing request scope ${name}`);
}
assert.equal(scope.rlsPolicies.length, 4);
assert.ok(scope.failureCases.every((item) => item.endsWith("zero_writes")));

const admission = fixtures["c2-internal-api-admission-v1.fixture.json"];
assert.equal(admission.endpointClass, "internal_only");
assert.equal(admission.operations.length, 3);
for (const claim of ["actor_id", "workspace_id", "membership_id", "receipt_digest"]) {
  assert.ok(admission.requiredClaims.includes(claim), `missing admission claim ${claim}`);
}
for (const forbidden of ["public_router_registration", "desktop_direct_call", "mobile_direct_call", "public_sdk_export"]) {
  assert.ok(admission.forbiddenExposure.includes(forbidden), `missing forbidden exposure ${forbidden}`);
}

const migrations = await readdir("feedback-backend/migrations/postgres");
assert.equal(
  migrations.some((name) => /^0024_.*cloud_copyright/i.test(name)),
  false,
  "C2 contract freeze must not create a 0024 cloud copyright migration",
);

console.log(JSON.stringify({
  ok: true,
  contractVersion: "cloud-copyright-c2-contract-v1",
  fixtures: fixtureNames.length,
  guarantees: ["shared_transport_mapping", "transaction_local_rls_scope", "internal_api_no_public_exposure"],
}));

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

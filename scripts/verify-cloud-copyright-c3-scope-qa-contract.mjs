import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const root = "docs/contracts/cloud-copyright";
const schema = await readJson(`${root}/cloud-copyright-c3-postgres-scope-qa-artifact-v1.schema.json`);
const fixture = await readJson(`${root}/c3-postgres-scope-qa-artifact-v1.fixture.json`);

assert.equal(schema.$id, "https://hiddenshield.local/contracts/cloud-copyright/cloud-copyright-c3-postgres-scope-qa-artifact-v1.schema.json");
assert.equal(fixture.schemaVersion, 1);
assert.equal(fixture.artifactKind, "cloud_copyright_postgres_scope_qa");
assert.equal(fixture.evidenceClass, "fixture_contract_only");
assert.equal(fixture.database.kind, "postgres");
assert.equal(fixture.database.connections, 2);
assert.equal(fixture.database.sqliteAllowed, false);
assert.equal(fixture.database.mockDatabaseAllowed, false);
assert.match(fixture.database.databaseNamePattern, /^hiddenshield_migrate_smoke/);
assert.equal(fixture.receiptProtection.rawIdentityReceipt, "excluded");
assert.equal(fixture.receiptProtection.providerResponse, "excluded");
assert.match(fixture.receiptProtection.scopeDigest, /^sha256:/);
assert.equal(fixture.retention.classification, "internal_security_qa");
assert.equal(fixture.retention.retainDays, 90);
assert.equal(fixture.retention.exportBoundary, "internal_audited_export_only");

const scenarioIds = [
  "valid_scoped_read_write",
  "missing_scope",
  "account_workspace_mismatch",
  "device_membership_mismatch",
  "role_denied",
  "revoked_after_receipt",
  "expired_invalid_unavailable_receipt",
  "service_actor_reason_gate",
  "pool_scope_bleed",
  "two_workspace_concurrency",
  "audit_failure_rollback",
  "direct_sql_denial",
];
assert.equal(fixture.scenarios.length, scenarioIds.length);
for (const scenarioId of scenarioIds) {
  const scenario = fixture.scenarios.find((candidate) => candidate.scenarioId === scenarioId);
  assert.ok(scenario, `missing scope QA scenario ${scenarioId}`);
  for (const key of ["recordDelta", "changeDelta", "eventDelta", "auditDelta", "cursorDelta"]) {
    assert.equal(typeof scenario.expected[key], "number", `${scenarioId} missing ${key}`);
  }
  if (!["valid_scoped_read_write", "two_workspace_concurrency"].includes(scenarioId)) {
    for (const key of ["recordDelta", "changeDelta", "eventDelta", "auditDelta", "cursorDelta"]) {
      assert.equal(scenario.expected[key], 0, `${scenarioId} must have zero business write delta`);
    }
  }
}

console.log(JSON.stringify({
  ok: true,
  contractVersion: "cloud-copyright-c3-postgres-scope-qa-artifact-v1",
  scenarios: scenarioIds.length,
  evidenceClass: fixture.evidenceClass,
  guarantees: ["postgres_only", "two_connections", "receipt_redaction", "fail_closed_zero_writes"],
}));

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

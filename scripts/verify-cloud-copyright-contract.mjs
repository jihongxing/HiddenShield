import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

const root = "docs/contracts/cloud-copyright";
const fixtureNames = [
  "copyright-record-v1.fixture.json",
  "workspace-membership-rbac-v1.fixture.json",
  "change-batch-v1.fixture.json",
  "conflict-version-changed-v1.fixture.json",
  "membership-revoked-v1.fixture.json",
  "forbidden-sync-data-rejection-v1.fixture.json",
];
const forbiddenFields = new Set([
  "originalPath",
  "original_path",
  "protectedCopyPath",
  "protected_copy_path",
  "localPath",
  "local_path",
  "mediaBytes",
  "media_bytes",
  "creatorSeed",
  "creator_seed",
  "accessToken",
  "access_token",
  "refreshToken",
  "refresh_token",
  "privateKey",
  "private_key",
]);

const schema = await readJson(`${root}/cloud-copyright-contract-v1.schema.json`);
const fixtures = Object.fromEntries(
  await Promise.all(
    fixtureNames.map(async (name) => [name, await readJson(`${root}/${name}`)]),
  ),
);

assertSchema(schema);
assertFixtureEnvelopes(schema, fixtures);
assertRecordAndCrossEndContract(fixtures["copyright-record-v1.fixture.json"]);
assertRbacContract(fixtures["workspace-membership-rbac-v1.fixture.json"]);
assertChangeBatchContract(fixtures["change-batch-v1.fixture.json"]);
assertConflictContract(fixtures["conflict-version-changed-v1.fixture.json"]);
assertRevocationContract(fixtures["membership-revoked-v1.fixture.json"]);
assertPrivacyContract(fixtures["forbidden-sync-data-rejection-v1.fixture.json"]);

console.log(
  JSON.stringify({
    ok: true,
    contractVersion: "cloud-copyright-contract-v1",
    fixtures: fixtureNames.length,
    guarantees: [
      "desktop_mobile_round_trip",
      "workspace_rbac",
      "idempotent_change_batch",
      "conflict_fail_closed",
      "revocation_fail_closed",
      "forbidden_data_rejected",
    ],
  }),
);

async function readJson(path) {
  return JSON.parse(await readFile(path, "utf8"));
}

function assertSchema(candidate) {
  assert.equal(
    candidate.$id,
    "https://hiddenshield.local/contracts/cloud-copyright/cloud-copyright-contract-v1.schema.json",
  );
  assert.equal(candidate.properties?.schemaVersion?.const, 1);
  assert.equal(candidate.properties?.fixtureType?.enum?.length, fixtureNames.length);
  assert.equal(candidate.oneOf?.length, fixtureNames.length);
}

function assertFixtureEnvelopes(candidate, fixtureMap) {
  const types = new Set(candidate.properties.fixtureType.enum);
  for (const [name, fixture] of Object.entries(fixtureMap)) {
    assert.equal(fixture.schemaVersion, 1, `${name}: schemaVersion must be 1`);
    assert(types.has(fixture.fixtureType), `${name}: unsupported fixture type`);
    const branch = candidate.oneOf.find(
      (entry) => entry.properties?.fixtureType?.const === fixture.fixtureType,
    );
    assert(branch, `${name}: schema branch missing`);
    for (const field of branch.required ?? []) {
      assert(Object.hasOwn(fixture, field), `${name}: missing ${field}`);
    }
  }
}

function assertRecordAndCrossEndContract(fixture) {
  assert.equal(fixture.recordSchemaVersion, "cloud-copyright-record-v1");
  const record = fixture.record;
  for (const field of [
    "recordId",
    "workspaceId",
    "ownerAccountId",
    "creatorProfileId",
    "originDeviceId",
    "watermarkUid",
    "watermarkRevision",
    "originalHash",
    "protectedCopyHash",
    "evidenceDigest",
    "recordVersion",
    "etag",
  ]) {
    assert(record[field] !== undefined && record[field] !== null, `record missing ${field}`);
  }
  assert.equal(record.classification, "private_metadata");
  assert.equal(record.visibility, "workspace_members");
  assertNoForbiddenFields(record, "copyright record");

  assert.equal(fixture.crossEndCases.length, 2);
  const canonicalRecordDigest = `sha256:${createHash("sha256")
    .update(canonicalJson(record))
    .digest("hex")}`;
  const caseIds = new Set(fixture.crossEndCases.map((item) => item.caseId));
  assert(caseIds.has("desktop_written_mobile_read"));
  assert(caseIds.has("mobile_written_desktop_read"));
  for (const testCase of fixture.crossEndCases) {
    assert.notEqual(testCase.writer, testCase.reader, `${testCase.caseId}: must cross endpoints`);
    assert.equal(testCase.expectedRecordId, record.recordId);
    assert.equal(testCase.expectedWatermarkUid, record.watermarkUid);
    assert.equal(testCase.expectedRecordVersion, record.recordVersion);
    assert.equal(testCase.expectedMetadataDigest, canonicalRecordDigest);
  }
}

function assertRbacContract(fixture) {
  assert.equal(fixture.workspace.workspaceType, "team");
  assert.equal(fixture.workspace.status, "active");
  const roles = ["owner", "admin", "editor", "viewer"].sort();
  assert.deepEqual(
    fixture.memberships.map((member) => member.role).sort(),
    roles,
  );
  assert.deepEqual(fixture.permissionMatrix.viewer, ["read_record"]);
  assert(fixture.permissionMatrix.editor.includes("write_record"));
  assert(!fixture.permissionMatrix.editor.includes("invite_member"));
  for (const privilegedAction of ["invite_member", "change_role", "remove_member", "export_audit"]) {
    assert(fixture.permissionMatrix.owner.includes(privilegedAction));
    assert(fixture.permissionMatrix.admin.includes(privilegedAction));
    assert(!fixture.permissionMatrix.viewer.includes(privilegedAction));
  }
}

function assertChangeBatchContract(fixture) {
  assert.equal(fixture.changes.length, 1);
  const change = fixture.changes[0];
  assert.equal(change.operation, "upsert_record");
  assert.match(change.idempotencyKey, /^desktop:/);
  assert.equal(change.baseRecordVersion, 6);
  assert.equal(fixture.expectedDispositions.length, 2);
  assert.equal(fixture.expectedDispositions[0].status, "accepted");
  assert.equal(fixture.expectedDispositions[0].auditAppended, true);
  assert.equal(fixture.expectedDispositions[1].status, "duplicate");
  assert.equal(fixture.expectedDispositions[1].auditAppended, false);
  assert.equal(
    fixture.expectedDispositions[0].recordVersion,
    fixture.expectedDispositions[1].recordVersion,
  );
}

function assertConflictContract(fixture) {
  assert.equal(fixture.change.baseRecordVersion, 6);
  assert.equal(fixture.change.currentRecordVersion, 7);
  assert.equal(fixture.expectedError.code, "conflict_version_changed");
  assert.equal(fixture.expectedError.remoteWriteCommitted, false);
  assert.equal(fixture.expectedError.auditAppended, false);
  assert.equal(fixture.localResolution.preserveLocalDraft, true);
  assert.equal(fixture.localResolution.requiresExplicitResolution, true);
}

function assertRevocationContract(fixture) {
  assert.equal(fixture.membership.status, "removed");
  assert.equal(fixture.expectedError.code, "blocked_by_membership_revoked");
  assert.equal(fixture.expectedError.remoteWriteCommitted, false);
  assert.equal(fixture.expectedError.auditAppended, false);
  assert.equal(fixture.localResolution.preserveLocalDraft, true);
  assert.equal(fixture.localResolution.requiresWorkspaceReauthorization, true);
}

function assertPrivacyContract(fixture) {
  for (const field of fixture.forbiddenFields) {
    assert(forbiddenFields.has(field), `forbidden field not frozen: ${field}`);
  }
  assert(
    Object.keys(fixture.recordCandidate).some((field) => fixture.forbiddenFields.includes(field)),
    "rejection fixture must contain an actual forbidden field",
  );
  assert.equal(fixture.expectedError.code, "rejected_forbidden_sync_data");
  assert.equal(fixture.expectedError.remoteWriteCommitted, false);
  assert.equal(fixture.expectedError.auditAppended, false);
}

function assertNoForbiddenFields(value, label) {
  if (Array.isArray(value)) {
    for (const entry of value) {
      assertNoForbiddenFields(entry, label);
    }
    return;
  }
  if (!value || typeof value !== "object") {
    return;
  }
  for (const [field, nested] of Object.entries(value)) {
    assert(!forbiddenFields.has(field), `${label} must not contain ${field}`);
    assertNoForbiddenFields(nested, label);
  }
}

function canonicalJson(value) {
  if (Array.isArray(value)) {
    return `[${value.map(canonicalJson).join(",")}]`;
  }
  if (value && typeof value === "object") {
    return `{${Object.keys(value)
      .sort()
      .map((key) => `${JSON.stringify(key)}:${canonicalJson(value[key])}`)
      .join(",")}}`;
  }
  return JSON.stringify(value);
}

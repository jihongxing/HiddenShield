import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const fixtureDir = path.join(root, "docs", "fixtures", "offline-license-k4");
const trustPolicy = JSON.parse(
  fs.readFileSync(path.join(fixtureDir, "trust-policy-v1.json"), "utf8"),
);
const vectors = JSON.parse(
  fs.readFileSync(path.join(fixtureDir, "security-policy-v1.json"), "utf8"),
);

const timestampPattern = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/;
const keyIdPattern = /^[a-z0-9][a-z0-9._-]{2,63}$/;
const publicKeyPattern = /^[A-Za-z0-9_-]{43}$/;
const digestPattern = /^[a-f0-9]{64}$/;

assert.equal(trustPolicy.schemaVersion, 1);
assert.equal(trustPolicy.policyType, "offline_license_trust_policy");
assert.deepEqual(trustPolicy.licenseSchemaRange, { minimum: 1, maximum: 1 });
assert.equal(trustPolicy.releaseIntegrity.requireOsPackageSignature, true);
assert.equal(
  trustPolicy.releaseIntegrity.allowInProcessSelfHashAsAuthority,
  false,
);
assert.ok(
  trustPolicy.clockPolicy.rollbackToleranceSeconds >= 0 &&
    trustPolicy.clockPolicy.rollbackToleranceSeconds <= 900,
);
assert.ok(
  trustPolicy.clockPolicy.futureArtifactToleranceSeconds >= 0 &&
    trustPolicy.clockPolicy.futureArtifactToleranceSeconds <= 900,
);

const sortedKeyIds = trustPolicy.keys.map((key) => key.keyId).toSorted();
assert.deepEqual(
  trustPolicy.keys.map((key) => key.keyId),
  sortedKeyIds,
  "trusted key ring must be sorted by keyId",
);
assert.equal(new Set(sortedKeyIds).size, sortedKeyIds.length);

const keyRing = new Map();
for (const key of trustPolicy.keys) {
  assert.match(key.keyId, keyIdPattern);
  assert.equal(key.algorithm, "Ed25519");
  assert.match(key.publicKeyBase64Url, publicKeyPattern);
  assert.ok(["active", "verify_only", "disabled"].includes(key.status));
  assert.ok(key.purposes.length > 0);
  assert.equal(new Set(key.purposes).size, key.purposes.length);
  assert.ok(
    key.purposes.every((purpose) =>
      ["license", "revocation"].includes(purpose),
    ),
  );
  assert.match(key.notBefore, timestampPattern);
  assert.match(key.notAfter, timestampPattern);
  assert.ok(Date.parse(key.notBefore) < Date.parse(key.notAfter));
  keyRing.set(key.keyId, key);
}

assert.equal(vectors.vectorVersion, 1);
assert.match(vectors.trustedNow, timestampPattern);
assert.match(vectors.highestObservedUtc, timestampPattern);
assert.match(vectors.revocationHighWater.keyId, keyIdPattern);
assert.match(vectors.revocationHighWater.payloadSha256, digestPattern);

function evaluate(testCase) {
  const key = keyRing.get(testCase.keyId);
  if (!key) return "offline_license_unknown_key";
  if (key.status === "disabled") return "offline_license_key_disabled";

  const purpose =
    testCase.operation === "import_revocation" ? "revocation" : "license";
  if (!key.purposes.includes(purpose)) {
    return "offline_license_key_purpose_invalid";
  }

  const observedNow = Date.parse(testCase.observedNow ?? vectors.trustedNow);
  const highestObserved = Date.parse(vectors.highestObservedUtc);
  if (
    observedNow + trustPolicy.clockPolicy.rollbackToleranceSeconds * 1000 <
    highestObserved
  ) {
    return "offline_license_clock_rollback";
  }

  if (
    Date.parse(testCase.artifactTime) >
    observedNow +
      trustPolicy.clockPolicy.futureArtifactToleranceSeconds * 1000
  ) {
    return "offline_license_artifact_from_future";
  }

  if (testCase.operation === "import_revocation") {
    assert.match(testCase.payloadSha256, digestPattern);
    const highWater = vectors.revocationHighWater;
    if (testCase.sequence < highWater.sequence) {
      return "offline_license_revocation_replay";
    }
    if (
      testCase.sequence === highWater.sequence &&
      testCase.payloadSha256 !== highWater.payloadSha256
    ) {
      return "offline_license_revocation_equivocation";
    }
  }

  return "ok";
}

for (const testCase of vectors.cases) {
  assert.equal(evaluate(testCase), testCase.expected, testCase.id);
}

console.log(
  `offline license K4 contract passed: ${trustPolicy.keys.length} keys, ${vectors.cases.length} policy vectors`,
);

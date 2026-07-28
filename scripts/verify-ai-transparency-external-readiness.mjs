import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const manifestPath =
  process.argv[2] ??
  "docs/ai-transparency-external-readiness/external-readiness.template.json";
const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
const schema = JSON.parse(
  await readFile(
    "docs/ai-transparency-external-readiness/external-readiness-v1.schema.json",
    "utf8",
  ),
);

assert.equal(
  schema.$id,
  "https://hiddenshield.internal/contracts/ai-transparency/external-readiness-v1.schema.json",
);
assert.equal(schema.additionalProperties, false);
assert.equal(manifest.schemaVersion, "hs-ai-external-readiness-v1");
assert.ok(["configuration_required", "ready_for_internal_review"].includes(manifest.status));
for (const section of ["provider", "designPartner", "approvals"]) {
  assert.equal(schema.properties[section].additionalProperties, false);
  for (const field of schema.properties[section].required) {
    assert.notEqual(manifest[section][field], undefined, `${section}.${field} is required`);
  }
}

const references = [];
collectStrings(manifest, references);
assert.equal(
  references.some((value) => /(?:api[_-]?key|token|private[_-]?key|password|secret=)/i.test(value)),
  false,
  "manifest must not contain raw secret material",
);

for (const field of [
  "iamReceiptUrl",
  "iamIssuer",
  "iamJwksUrl",
  "kmsHealthUrl",
  "signerEndpoint",
]) {
  assertHttpsPlaceholder(manifest.provider[field], `provider.${field}`);
}
for (const field of ["sandboxApiBaseUrl", "resolverBaseUrl"]) {
  assertHttpsPlaceholder(manifest.designPartner[field], `designPartner.${field}`);
}
for (const field of [
  "workloadIdentityRef",
  "signerCredentialRef",
  "objectStoreCredentialRef",
  "notificationCredentialRef",
]) {
  assert.match(manifest.provider[field], /^secret:\/\//, `provider.${field} must use secret://`);
}
assert.match(
  manifest.designPartner.credentialSecretRef,
  /^secret:\/\//,
  "designPartner.credentialSecretRef must use secret://",
);
assert.match(manifest.provider.recoveryRunbookRef, /^runbook:\/\//);
assert.equal(manifest.designPartner.acceptanceEvidenceRef, null);

const configurationRequired =
  manifest.status === "configuration_required" &&
  references.some((value) => value.includes("replace-me"));
assert.equal(
  configurationRequired,
  true,
  "template must remain configuration_required until external references are supplied",
);

console.log(JSON.stringify({
  ok: true,
  schemaVersion: manifest.schemaVersion,
  status: manifest.status,
  productionConfigurationActivated: false,
  partnerAcceptance: "not_started",
}));

function assertHttpsPlaceholder(value, field) {
  assert.equal(typeof value, "string", `${field} must be a string`);
  assert.match(value, /^https:\/\//, `${field} must use HTTPS`);
  assert.match(value, /replace-me/, `${field} must remain a placeholder in the template`);
}

function collectStrings(value, output) {
  if (typeof value === "string") {
    output.push(value);
    return;
  }
  if (Array.isArray(value)) {
    value.forEach((entry) => collectStrings(entry, output));
    return;
  }
  if (value && typeof value === "object") {
    Object.values(value).forEach((entry) => collectStrings(entry, output));
  }
}

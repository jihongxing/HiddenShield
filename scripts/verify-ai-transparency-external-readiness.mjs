import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const { manifestPath, mode, expectInvalid } = parseArguments(process.argv.slice(2));

try {
  const manifest = JSON.parse(await readFile(manifestPath, "utf8"));
  const schema = JSON.parse(
    await readFile(
      "docs/ai-transparency-external-readiness/external-readiness-v1.schema.json",
      "utf8",
    ),
  );

  validateManifest({ manifest, schema, mode });
  if (expectInvalid) {
    throw new Error("fixture unexpectedly passed validation");
  }

  console.log(JSON.stringify({
    ok: true,
    schemaVersion: manifest.schemaVersion,
    mode,
    status: manifest.status,
    productionConfigurationActivated: false,
    partnerAcceptance: "not_started",
  }));
} catch (error) {
  if (!expectInvalid) throw error;
  console.log(JSON.stringify({
    ok: true,
    mode,
    expectedInvalid: true,
    error: error.message,
    productionConfigurationActivated: false,
  }));
}

function validateManifest({ manifest, schema, mode }) {
  assert.equal(
    schema.$id,
    "https://hiddenshield.internal/contracts/ai-transparency/external-readiness-v1.schema.json",
  );
  assert.equal(schema.additionalProperties, false);
  assert.equal(manifest.schemaVersion, "hs-ai-external-readiness-v1");
  assert.equal(
    manifest.status,
    mode === "template" ? "configuration_required" : "ready_for_internal_review",
  );
  for (const section of ["provider", "designPartner", "approvals"]) {
    assert.equal(schema.properties[section].additionalProperties, false);
    for (const field of schema.properties[section].required) {
      assert.notEqual(manifest[section][field], undefined, `${section}.${field} is required`);
    }
  }

  const references = [];
  collectStrings(manifest, references);
  assert.equal(
    references.some((value) => /(?:-----BEGIN|(?:api[_-]?key|token|private[_-]?key|password)\s*[:=]|secret\s*=)/i.test(value)),
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
    assertHttps(manifest.provider[field], `provider.${field}`, mode);
  }
  for (const field of ["sandboxApiBaseUrl", "resolverBaseUrl"]) {
    assertHttps(manifest.designPartner[field], `designPartner.${field}`, mode);
  }
  for (const field of [
    "workloadIdentityRef",
    "signerCredentialRef",
    "objectStoreCredentialRef",
    "notificationCredentialRef",
  ]) {
    assertReference(manifest.provider[field], `provider.${field}`, "secret", mode);
  }
  assertReference(
    manifest.designPartner.credentialSecretRef,
    "designPartner.credentialSecretRef",
    "secret",
    mode,
  );
  assertReference(manifest.provider.activePepperRef, "provider.activePepperRef", "kms", mode);
  for (const value of manifest.provider.retainedPepperRefs) {
    assertReference(value, "provider.retainedPepperRefs", "kms", mode);
  }
  assertReference(manifest.provider.recoveryRunbookRef, "provider.recoveryRunbookRef", "runbook", mode);
  assertReference(manifest.designPartner.partnerBundleRef, "designPartner.partnerBundleRef", "partner", mode);
  assertReference(
    manifest.designPartner.dataProcessingApprovalRef,
    "designPartner.dataProcessingApprovalRef",
    "partner",
    mode,
  );
  for (const field of ["securityApprovalRef", "legalApprovalRef", "productApprovalRef"]) {
    assertReference(manifest.approvals[field], `approvals.${field}`, "approval", mode);
  }

  assert.match(manifest.provider.iamAudience, /^[A-Za-z0-9._:-]{8,}$/);
  if (mode === "review") {
    assert.ok(["gcp_kms", "aws_kms", "azure_key_vault", "pkcs11"].includes(manifest.provider.kmsProvider));
  } else {
    assert.match(manifest.provider.kmsProvider, /replace-me/);
  }
  assert.equal(manifest.designPartner.acceptanceEvidenceRef, null);

  if (mode === "template") {
    assert.equal(
      references.some((value) => value.includes("replace-me")),
      true,
      "template must retain placeholders",
    );
  } else {
    assert.equal(
      references.some((value) => /replace[-_]?me|placeholder/i.test(value)),
      false,
      "review manifest must not contain placeholders",
    );
  }
}

function assertHttps(value, field, mode) {
  assert.equal(typeof value, "string", `${field} must be a string`);
  assert.match(value, /^https:\/\//, `${field} must use HTTPS`);
  const hostname = new URL(value).hostname.toLowerCase();
  if (mode === "template") {
    assert.match(value, /replace-me/, `${field} must retain a placeholder`);
    return;
  }
  assert.equal(
    ["localhost", "127.0.0.1", "::1", "example.com", "example.net", "example.org"].includes(hostname) ||
      hostname.endsWith(".example") ||
      hostname.endsWith(".invalid") ||
      hostname.endsWith(".test"),
    false,
    `${field} must be a non-placeholder HTTPS endpoint`,
  );
}

function assertReference(value, field, kind, mode) {
  const patterns = {
    secret: /^secret:\/\/[^/\s]+\/.+/,
    kms: /^(?:gcp-kms|aws-kms|azure-key-vault|pkcs11|kms):\/\/[^/\s]+\/.+/,
    runbook: /^runbook:\/\/[^/\s]+\/.+/,
    partner: /^partner:\/\/[^/\s]+\/.+/,
    approval: /^approval:\/\/[^/\s]+\/.+/,
  };
  assert.equal(typeof value, "string", `${field} must be a string`);
  assert.match(value, patterns[kind], `${field} has an invalid ${kind} reference`);
  if (mode === "review") {
    assert.doesNotMatch(value, /replace[-_]?me|placeholder/i, `${field} must be configured`);
  }
}

function parseArguments(args) {
  let manifestPath = "docs/ai-transparency-external-readiness/external-readiness.template.json";
  let mode = "template";
  let expectInvalid = false;
  for (let index = 0; index < args.length; index += 1) {
    if (args[index] === "--mode") {
      mode = args[++index];
    } else if (args[index] === "--expect-invalid") {
      expectInvalid = true;
    } else if (!args[index].startsWith("--")) {
      manifestPath = args[index];
    } else {
      throw new Error(`unknown argument: ${args[index]}`);
    }
  }
  assert.ok(["template", "review"].includes(mode), "--mode must be template or review");
  return { manifestPath, mode, expectInvalid };
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

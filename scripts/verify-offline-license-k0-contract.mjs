import { readFileSync } from "node:fs";
import {
  decodeBase64Url,
  deriveInstallationIdV1,
  parseActivationRequestV1,
  parseOfflineLicenseV1,
  parseRevocationListV1,
  validateOfflineArtifactV1,
  verifyActivationRequestV1Checksum,
  verifyOfflineLicenseV1Signature,
  verifyRevocationListV1Signature,
} from "../src/lib/offline-license.ts";

const root = "docs/fixtures/offline-license-k0";
const licenseFixture = readJson(`${root}/hslic1-ed25519-v1.json`);
const requestFixture = readJson(`${root}/hsreq1-v1-valid.json`);
const revocationFixture = readJson(`${root}/hsrvl1-ed25519-v1-valid.json`);
const errorVectors = readJson(`${root}/offline-license-errors-v1.json`);
const installationIdentity = readJson(
  `${root}/installation-identity-v1.json`,
);
const licenseSchema = readJson(
  "docs/contracts/offline-license/license-payload-v1.schema.json",
);
const licenseTokenSchema = readJson(
  "docs/contracts/offline-license/hslic1-token-v1.schema.json",
);
const requestSchema = readJson(
  "docs/contracts/offline-license/activation-request-payload-v1.schema.json",
);
const requestTokenSchema = readJson(
  "docs/contracts/offline-license/hsreq1-token-v1.schema.json",
);
const revocationSchema = readJson(
  "docs/contracts/offline-license/revocation-list-payload-v1.schema.json",
);
const revocationTokenSchema = readJson(
  "docs/contracts/offline-license/hsrvl1-token-v1.schema.json",
);

assertTokenContract(licenseFixture, licenseTokenSchema, "HSLIC1");
assertTokenContract(requestFixture, requestTokenSchema, "HSREQ1");
assertTokenContract(revocationFixture, revocationTokenSchema, "HSRVL1");
assert(
  licenseSchema.additionalProperties === false &&
    licenseSchema.properties.productCode.const === "creator_offline" &&
    licenseSchema.properties.schemaVersion.const === 1,
  "license schema must freeze v1 and creator_offline",
);
assert(
  requestSchema.additionalProperties === false &&
    requestSchema.properties.requestedProductCode.const ===
      "creator_offline" &&
    requestSchema.properties.schemaVersion.const === 1,
  "request schema must freeze v1 and creator_offline",
);
assert(
  revocationSchema.additionalProperties === false &&
    revocationSchema.properties.listType.const ===
      "offline_license_revocations" &&
    revocationSchema.properties.sequence.minimum === 1 &&
    revocationSchema.properties.schemaVersion.const === 1,
  "revocation schema must freeze type, sequence, and schema v1",
);

const publicKey = decodeBase64Url(licenseFixture.publicKeyBase64Url);
const license = parseOfflineLicenseV1(licenseFixture.token);
assert(
  new TextDecoder().decode(license.payloadBytes) ===
    licenseFixture.canonicalPayload,
  "TypeScript license parser must preserve canonical payload bytes",
);
assertExpectedFields(license.payload, licenseFixture.expected);
assert(
  await verifyOfflineLicenseV1Signature(license, publicKey),
  "TypeScript must verify the fixed license signature",
);

const request = parseActivationRequestV1(requestFixture.token);
assert(
  new TextDecoder().decode(request.payloadBytes) ===
    requestFixture.canonicalPayload,
  "TypeScript request parser must preserve canonical payload bytes",
);
assertExpectedFields(request.payload, requestFixture.expected);
assert(
  await verifyActivationRequestV1Checksum(request),
  "TypeScript must verify the fixed request checksum",
);

const revocation = parseRevocationListV1(revocationFixture.token);
assert(
  new TextDecoder().decode(revocation.payloadBytes) ===
    revocationFixture.canonicalPayload,
  "TypeScript revocation parser must preserve canonical payload bytes",
);
assertExpectedFields(revocation.payload, revocationFixture.expected);
assert(
  await verifyRevocationListV1Signature(revocation, publicKey),
  "TypeScript must verify the fixed revocation signature",
);

const sources = {
  license: licenseFixture,
  activation_request: requestFixture,
  revocation_list: revocationFixture,
};
for (const vector of errorVectors.cases) {
  const source = sources[vector.source];
  const mutated = mutateVector(source, vector.mutation);
  let actualError = null;
  try {
    await validateOfflineArtifactV1(
      vector.source,
      mutated.token,
      mutated.publicKeyBase64Url
        ? decodeBase64Url(mutated.publicKeyBase64Url)
        : undefined,
    );
  } catch (error) {
    actualError = error instanceof Error ? error.message : String(error);
  }
  assert(
    actualError === vector.expectedError,
    `${vector.caseId} expected ${vector.expectedError}, got ${actualError}`,
  );
}

assert(
  (await deriveInstallationIdV1(
    decodeBase64Url(installationIdentity.testOnlySecretBase64Url),
    decodeBase64Url(installationIdentity.saltBase64Url),
  )) === installationIdentity.expectedInstallationId,
  "TypeScript installation identity must match the shared vector",
);

console.log(
  `Offline license K0 contract OK (${errorVectors.cases.length} shared error vectors)`,
);

function readJson(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

function assertTokenContract(fixture, schema, prefix) {
  assert(
    fixture.token.length === fixture.expected.tokenLength,
    `${prefix} fixture token length must match`,
  );
  assert(
    fixture.token.length >= schema.minLength &&
      fixture.token.length <= schema.maxLength &&
      new RegExp(schema.pattern).test(fixture.token),
    `${prefix} token must match its frozen schema`,
  );
}

function assertExpectedFields(actual, expected) {
  for (const [key, value] of Object.entries(expected)) {
    if (
      key === "tokenLength" ||
      key === "signatureValid" ||
      key === "checksumValid"
    ) {
      continue;
    }
    assert(
      JSON.stringify(actual[key]) === JSON.stringify(value),
      `${key} must match the shared expected result`,
    );
  }
}

function mutateVector(source, mutation) {
  let token = source.token;
  let publicKeyBase64Url = source.publicKeyBase64Url;
  if (mutation.kind === "replace_prefix") {
    const segments = token.split(".");
    segments[0] = mutation.value;
    token = segments.join(".");
  } else if (mutation.kind === "replace_payload") {
    const segments = token.split(".");
    const payload = Buffer.from(segments[1], "base64url").toString("utf8");
    const mutatedPayload = replaceExactlyOnce(
      payload,
      mutation.from,
      mutation.to,
    );
    segments[1] = Buffer.from(mutatedPayload, "utf8").toString("base64url");
    token = segments.join(".");
  } else if (mutation.kind === "replace_trailer") {
    const segments = token.split(".");
    segments[2] = mutation.value;
    token = segments.join(".");
  } else if (mutation.kind === "replace_public_key") {
    publicKeyBase64Url = mutation.value;
  } else {
    throw new Error(`unknown mutation ${mutation.kind}`);
  }
  return { token, publicKeyBase64Url };
}

function replaceExactlyOnce(value, from, to) {
  const first = value.indexOf(from);
  const last = value.lastIndexOf(from);
  assert(first >= 0 && first === last, `mutation source must occur once: ${from}`);
  return `${value.slice(0, first)}${to}${value.slice(first + from.length)}`;
}

function assert(condition, message) {
  if (!condition) {
    console.error(`Offline license K0 contract failed: ${message}`);
    process.exit(1);
  }
}

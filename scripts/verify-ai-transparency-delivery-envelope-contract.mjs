import assert from "node:assert/strict";
import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const fixturePath = path.join(
  root,
  "docs",
  "contracts",
  "ai-transparency-delivery-envelope",
  "success-v1.fixture.json",
);
const fixture = JSON.parse(fs.readFileSync(fixturePath, "utf8"));

const sha256 = (value) =>
  crypto.createHash("sha256").update(value).digest("hex");

const sortJson = (value) => {
  if (Array.isArray(value)) {
    return value.map(sortJson);
  }
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, sortJson(value[key])]),
    );
  }
  return value;
};

const canonicalJson = (value) => JSON.stringify(sortJson(value));
const mediaBytes = Buffer.from(fixture.finalMediaUtf8, "utf8");
const envelope = fixture.envelope;
const profile = envelope.profileIdentity;

assert.equal(
  envelope.schemaVersion,
  "hs-ai-confirmed-artifact-delivery-envelope-v1",
);
assert.equal(envelope.signingStatus, "confirmed");
assert.equal(envelope.artifactStatus, "finalized");
assert.equal(envelope.recoveryState, "completed");
assert.equal(envelope.finalFileSha256, sha256(mediaBytes));
assert.equal(
  envelope.signerReceiptSha256,
  sha256(canonicalJson(fixture.signerReceipt)),
);
assert.equal(
  envelope.artifactFinalizeReceiptSha256,
  sha256(canonicalJson(fixture.artifactFinalizeReceipt)),
);
assert.equal(
  envelope.profileIdentityDigest,
  sha256(
    JSON.stringify([
      profile.entitlementVersion,
      profile.entitlementDigest,
      profile.technicalProfileIds,
      profile.regionalProfileId,
    ]),
  ),
);
assert.equal(
  envelope.envelopeDigest,
  sha256(
    JSON.stringify([
      envelope.schemaVersion,
      envelope.deliveryEnvelopeId,
      envelope.executionId,
      envelope.markingSessionId,
      envelope.transparencyManifestId,
      envelope.licenseId,
      envelope.watermarkUid,
      envelope.mediaType,
      envelope.claimType,
      envelope.signingStatus,
      envelope.artifactStatus,
      envelope.recoveryState,
      envelope.workerRecoveryAttempts,
      envelope.recoveryControlVersion,
      envelope.finalFileSha256,
      envelope.artifactRef,
      envelope.artifactObjectVersion,
      envelope.signerReceiptId,
      envelope.signerReceiptSha256,
      envelope.artifactFinalizeReceiptId,
      envelope.artifactFinalizeReceiptSha256,
      profile.entitlementVersion,
      profile.entitlementDigest,
      profile.technicalProfileIds,
      profile.regionalProfileId,
      envelope.profileIdentityDigest,
      envelope.finalizedAt,
    ]),
  ),
);

const desktopSource = fs.readFileSync(
  path.join(root, "src-tauri", "src", "commands", "delivery_envelope.rs"),
  "utf8",
);
const mobileSource = fs.readFileSync(
  path.join(root, "mobile_app", "rust", "src", "api.rs"),
  "utf8",
);
for (const [name, source] of [
  ["desktop", desktopSource],
  ["mobile", mobileSource],
]) {
  assert.match(
    source,
    /validate_ai_delivery_envelope\(/,
    `${name} bridge must call watermark-core delivery validation`,
  );
  assert.doesNotMatch(
    source,
    /fn\s+ai_delivery_envelope_digest/,
    `${name} bridge must not implement a second envelope digest`,
  );
}

console.log(
  "AI Transparency confirmed/finalized delivery envelope contract passed (shared Desktop/mobile fixture)",
);

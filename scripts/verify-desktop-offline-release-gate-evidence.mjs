import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

const summary = JSON.parse(
  readFileSync(
    "artifacts/desktop-offline-release-gate/20260721/summary.json",
    "utf8",
  ),
);
const trustPolicy = JSON.parse(
  readFileSync("config/offline-license-trust-policy.production.json", "utf8"),
);

assert.equal(summary.schemaVersion, 5);
assert.equal(summary.date, "2026-07-21");
assert.equal(summary.status, "passed_with_rc_environment_limitations");
assert.equal(summary.releaseGates.rc.status, "passed");
assert.equal(summary.releaseGates.ga.status, "in_progress");
assert(summary.package.installerEvidence.includes("20260721-rc-final"));
assert(summary.package.authenticodeEvidence.includes("20260721171515"));
assert.equal(summary.licenseLifecycle.active.status, "active");
assert.equal(summary.licenseLifecycle.expired.errorCode, "offline_license_expired");
assert.equal(summary.licenseLifecycle.restartRevoked.status, "revoked");
assert.equal(
  summary.mapping.batch_processing,
  true,
);
assert.equal(
  summary.mapping.report_export,
  false,
);
assert.equal(
  summary.licenseLifecycle.restartRevoked.features.batch_processing,
  false,
);
assert.equal(
  summary.licenseLifecycle.restartRevoked.features.report_export,
  false,
);
assert.equal(summary.secretHandling.privateKeyCommitted, false);
assert.equal(summary.secretHandling.tokensCommitted, false);
assert(
  trustPolicy.keys.some(
    (key) =>
      key.keyId === summary.licenseLifecycle.active.keyId &&
      key.status === "active" &&
      key.purposes.includes("license") &&
      key.purposes.includes("revocation"),
  ),
  "runtime issuer key must exist in the internal QA trust policy",
);

console.log("Desktop offline release Gate evidence OK");

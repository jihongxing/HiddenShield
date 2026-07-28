import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import test from "node:test";

import {
  buildResolverUrl,
  REQUIRED_ACCEPTANCE_SCENARIOS,
  validateDesignPartnerSandboxKit,
} from "../src/index.mjs";

const template = JSON.parse(
  await readFile(
    new URL("../templates/design-partner-sandbox-kit.template.json", import.meta.url),
    "utf8",
  ),
);

test("frozen template is valid but configuration is required", () => {
  const result = validateDesignPartnerSandboxKit(template);
  assert.equal(result.valid, true);
  assert.equal(result.readiness, "configuration_required");
  assert.equal(result.warnings.length, 2);
});

test("fully evidenced partner bundle reaches sandbox accepted", () => {
  const bundle = structuredClone(template);
  bundle.packageStatus = "approved_for_sandbox";
  bundle.onboarding.partnerId = "partner_acme_001";
  bundle.onboarding.partnerLegalNameRef = "crm://partners/acme";
  bundle.onboarding.technicalContactRef = "iam://partners/acme/technical";
  bundle.onboarding.securityContactRef = "iam://partners/acme/security";
  bundle.onboarding.credentialSecretRef = "secret://partners/acme/sandbox";
  bundle.onboarding.sandboxApiBaseUrl = "https://sandbox-api.acme-aigc.com";
  bundle.onboarding.resolverBaseUrl = "https://sandbox-resolver.acme-aigc.com";
  bundle.onboarding.approvalReferences = {
    partnerTechnicalSignoffRef: "approval://partners/acme/technical/2026-07-28",
    partnerSecuritySignoffRef: "approval://partners/acme/security/2026-07-28",
    hiddenShieldEngineeringApprovalRef: "approval://hiddenshield/engineering/acme/2026-07-28",
    hiddenShieldCommercialApprovalRef: "approval://hiddenshield/commercial/acme/2026-07-28",
  };
  bundle.acceptanceMatrix.scenarios = bundle.acceptanceMatrix.scenarios.map(
    (scenario) => ({
      ...scenario,
      status: "passed",
      evidenceRef: `evidence://sha256/${createHash("sha256")
        .update(scenario.scenarioId)
        .digest("hex")}`,
    }),
  );
  const result = validateDesignPartnerSandboxKit(bundle);
  assert.equal(result.valid, true);
  assert.equal(result.readiness, "sandbox_accepted");
});

test("placeholder partner references cannot reach sandbox accepted", () => {
  const bundle = structuredClone(template);
  bundle.packageStatus = "approved_for_sandbox";
  bundle.onboarding.sandboxApiBaseUrl = "https://sandbox-api.acme-aigc.com";
  bundle.onboarding.resolverBaseUrl = "https://sandbox-resolver.acme-aigc.com";
  bundle.acceptanceMatrix.scenarios = bundle.acceptanceMatrix.scenarios.map(
    (scenario) => ({
      ...scenario,
      status: "passed",
      evidenceRef: `evidence://sha256/${createHash("sha256")
        .update(scenario.scenarioId)
        .digest("hex")}`,
    }),
  );
  const result = validateDesignPartnerSandboxKit(bundle);
  assert.equal(result.valid, true);
  assert.equal(result.readiness, "configuration_required");
});

test("mutable evidence reference fails closed", () => {
  const bundle = structuredClone(template);
  bundle.acceptanceMatrix.scenarios[0].status = "passed";
  bundle.acceptanceMatrix.scenarios[0].evidenceRef = "evidence://partner/latest";
  const result = validateDesignPartnerSandboxKit(bundle);
  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /immutable evidenceRef/);
});

test("raw credential material fails closed", () => {
  const bundle = structuredClone(template);
  bundle.onboarding.credential = "hsai_live_raw_secret_must_not_be_here";
  const result = validateDesignPartnerSandboxKit(bundle);
  assert.equal(result.valid, false);
  assert.match(result.errors.join("\n"), /external reference/);
});

test("acceptance matrix contains every mandatory scenario", () => {
  const scenarioIds = new Set(
    template.acceptanceMatrix.scenarios.map((scenario) => scenario.scenarioId),
  );
  for (const scenarioId of REQUIRED_ACCEPTANCE_SCENARIOS) {
    assert.equal(scenarioIds.has(scenarioId), true);
  }
});

test("resolver link builder accepts exactly one public identifier", () => {
  assert.equal(
    buildResolverUrl({
      resolverBaseUrl: "https://resolver.partner.test",
      watermarkUid: "HS-01234567-89ABCDEF-01234567-89ABCDEF",
    }),
    "https://resolver.partner.test/v1/ai-transparency/public/resolve/watermarks/HS-01234567-89ABCDEF-01234567-89ABCDEF",
  );
  assert.throws(
    () =>
      buildResolverUrl({
        resolverBaseUrl: "https://resolver.partner.test",
        watermarkUid: "HS-01234567-89ABCDEF-01234567-89ABCDEF",
        manifestId: "manifest-1",
      }),
    /exactly one/,
  );
});

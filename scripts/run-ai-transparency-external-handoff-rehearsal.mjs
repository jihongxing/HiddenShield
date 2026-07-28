import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";

const template = JSON.parse(
  await readFile(
    "docs/ai-transparency-external-readiness/phase-gates/phase-c-d-evidence.template.json",
    "utf8",
  ),
);

assert.equal(template.schemaVersion, "hs-ai-phase-c-d-evidence-v1");
assert.equal(template.status, "configuration_required");
assert.equal(template.releaseBoundary.productionActivation, false);
assert.equal(template.releaseBoundary.publicResolverDeployment, false);
assert.equal(template.releaseBoundary.sdkPublication, false);
assert.equal(template.releaseBoundary.legalConclusion, false);
assert.equal(template.phaseCDesignPartner.acceptanceEvidenceRefs.length, 0);
assert.deepEqual(
  Object.values(template.phaseDProviders).filter((value) => value === null).length,
  5,
);

const commands = [
  ["scripts/verify-ai-transparency-external-readiness.mjs"],
  ["scripts/verify-ai-transparency-external-evidence-intake-contract.mjs"],
  ["packages/ai-transparency-design-partner-kit/bin/preflight.mjs", "packages/ai-transparency-design-partner-kit/templates/design-partner-sandbox-kit.template.json"],
];
for (const args of commands) {
  execFileSync(process.execPath, args, { stdio: "pipe" });
}

console.log(JSON.stringify({
  ok: true,
  executionMode: "synthetic_external_handoff_rehearsal",
  acceptanceStatus: "not_real_partner_or_provider_acceptance",
  readiness: "configuration_required",
  phaseCGate: "blocked_external",
  phaseDGate: "blocked_external",
  productionActivation: false,
}));

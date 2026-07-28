import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

import { REQUIRED_ACCEPTANCE_SCENARIOS } from "../src/index.mjs";
import { runSyntheticSandboxQa } from "../src/synthetic-sandbox-qa.mjs";

const template = JSON.parse(
  await readFile(
    new URL("../templates/design-partner-sandbox-kit.template.json", import.meta.url),
    "utf8"
  )
);

test("synthetic QA exercises the partner flow without claiming acceptance", async () => {
  const report = await runSyntheticSandboxQa(template);

  assert.equal(report.contractVersion, "hs-ai-synthetic-sandbox-qa-v1");
  assert.equal(report.executionMode, "synthetic_non_acceptance");
  assert.equal(report.acceptanceStatus, "not_real_partner_acceptance");
  assert.equal(report.readiness, "configuration_required");
  assert.deepEqual(report.scenarioIds, REQUIRED_ACCEPTANCE_SCENARIOS);
  assert.deepEqual(report.sdkCallOrder, ["admit", "session", "mark", "confirm", "confirm"]);
  assert.equal(report.metering.replayed, true);
  assert.equal(report.resolver.legalConclusion, false);
  assert.equal(report.latency.notPartnerLatencyEvidence, true);
  assert.doesNotMatch(JSON.stringify(report), /hs_synthetic_runtime_only/);
});

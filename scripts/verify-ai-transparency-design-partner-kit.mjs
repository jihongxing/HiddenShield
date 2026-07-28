import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

import {
  REQUIRED_ACCEPTANCE_SCENARIOS,
  validateDesignPartnerSandboxKit,
} from "../packages/ai-transparency-design-partner-kit/src/index.mjs";

const root = "packages/ai-transparency-design-partner-kit";
const packageJson = JSON.parse(await readFile(`${root}/package.json`, "utf8"));
const schema = JSON.parse(
  await readFile(`${root}/schemas/design-partner-sandbox-kit-v1.schema.json`, "utf8"),
);
const template = JSON.parse(
  await readFile(`${root}/templates/design-partner-sandbox-kit.template.json`, "utf8"),
);
const readme = await readFile(`${root}/README.md`, "utf8");
const onboarding = await readFile(`${root}/ONBOARDING.md`, "utf8");
const profileMapping = await readFile(`${root}/PROFILE_MAPPING.md`, "utf8");
const acceptance = await readFile(`${root}/ACCEPTANCE_MATRIX.md`, "utf8");
const example = await readFile(`${root}/examples/server-mark-and-resolve.mjs`, "utf8");

assert.equal(packageJson.private, true);
assert.equal(
  packageJson.name,
  "@hiddenshield/ai-transparency-design-partner-kit",
);
assert.equal(
  schema.$id,
  "https://hiddenshield.internal/contracts/ai-transparency/design-partner-sandbox-kit-v1.schema.json",
);
assert.equal(schema.additionalProperties, false);
assert.equal(template.onboarding.environment, "sandbox");
assert.match(template.onboarding.credentialSecretRef, /^secret:\/\//);
assert.equal(template.onboarding.acknowledgements.noProductionCredential, true);
assert.match(
  template.onboarding.approvalReferences.partnerTechnicalSignoffRef,
  /replace-me/,
);
assert.equal(template.resolverLink.requiresAuthorization, false);
assert.equal(template.resolverLink.metered, false);
assert.equal(template.resolverLink.legalConclusion, false);
assert.deepEqual(
  template.acceptanceMatrix.scenarios.map((scenario) => scenario.scenarioId),
  REQUIRED_ACCEPTANCE_SCENARIOS,
);

const preflight = validateDesignPartnerSandboxKit(template);
assert.equal(preflight.valid, true);
assert.equal(preflight.readiness, "configuration_required");
assert.equal(schema.properties.acceptanceMatrix.properties.scenarios.maxItems, 12);
assert.equal(
  schema.properties.acceptanceMatrix.properties.scenarios.items.properties
    .evidenceRef.pattern,
  "^evidence://sha256/[a-f0-9]{64}$",
);
assert.equal(
  template.acceptanceMatrix.scenarios.every(
    (scenario) => scenario.status === "not_run" && scenario.evidenceRef === null,
  ),
  true,
);
assert.match(readme, /No production credential/);
assert.match(onboarding, /blocked_external/);
assert.match(profileMapping, /does not determine legal applicability/);
assert.match(acceptance, /`?blocked_external`? is not acceptance/i);
assert.match(example, /createAiTransparencyPlatformFacade/);
assert.match(example, /buildResolverUrl/);
assert.doesNotMatch(example, /hsai_live_/);

console.log(JSON.stringify({
  ok: true,
  packageName: packageJson.name,
  schemaVersion: template.schemaVersion,
  mandatoryScenarios: REQUIRED_ACCEPTANCE_SCENARIOS.length,
  readiness: preflight.readiness,
  productionCredentialIncluded: false,
}));

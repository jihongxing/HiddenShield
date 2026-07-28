import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const root = "packages/ai-transparency-design-partner-kit";
const source = await readFile(`${root}/src/synthetic-sandbox-qa.mjs`, "utf8");
const documentation = await readFile(`${root}/SYNTHETIC_QA.md`, "utf8");

assert.match(source, /synthetic_non_acceptance/);
assert.match(source, /not_real_partner_acceptance/);
assert.match(source, /configuration_required/);
assert.doesNotMatch(source, /sandbox_accepted/);
assert.doesNotMatch(source, /\bfetch\s*\(/);
assert.doesNotMatch(source, /watermark-core/);
assert.doesNotMatch(source, /postgres/i);
assert.match(documentation, /never produces `sandbox_accepted`/);
assert.match(documentation, /without network, PostgreSQL, `watermark-core`/);

console.log(JSON.stringify({
  ok: true,
  contractVersion: "hs-ai-synthetic-sandbox-qa-v1",
  acceptanceStatus: "not_real_partner_acceptance"
}));

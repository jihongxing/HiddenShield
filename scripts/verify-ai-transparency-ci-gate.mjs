import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const packageJson = JSON.parse(await readFile("package.json", "utf8"));
const workflow = (await readFile(".github/workflows/ci.yml", "utf8")).replace(
  /\r\n/g,
  "\n"
);
const command = packageJson.scripts["ai-transparency:ci"];

assert.equal(typeof command, "string");
assert.match(command, /ai-transparency:sdk-contract/);
assert.match(command, /ai-transparency:sdk-test/);
assert.match(command, /ai-transparency:design-partner-kit/);
assert.match(command, /ai-transparency:synthetic-sandbox-qa/);
assert.match(command, /ai-transparency:external-readiness/);
assert.match(command, /ai-transparency:external-evidence-intake/);
assert.match(command, /ai-transparency:postgres-qa-contract/);
assert.match(command, /ai-transparency:external-handoff-rehearsal/);
assert.match(workflow, /\n  ai-transparency-contract:\n/);
assert.match(workflow, /name: AI Transparency contract gate/);
assert.match(workflow, /run: npm run ai-transparency:ci/);

console.log(
  JSON.stringify({
    ok: true,
    job: "ai-transparency-contract",
    command: "ai-transparency:ci",
    syntheticSandboxQaRequired: true
  })
);

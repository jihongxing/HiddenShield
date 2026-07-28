import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const root = "docs/contracts/ai-transparency-external-evidence-intake";
const success = JSON.parse(await readFile(`${root}/success-v1.fixture.json`, "utf8"));
const rejected = JSON.parse(await readFile(`${root}/reject-placeholder-v1.fixture.json`, "utf8"));
const moduleSource = await readFile("feedback-backend/src/ai_transparency_external_evidence_intake.rs", "utf8");
const migration = await readFile(
  "feedback-backend/migrations/postgres/0021_ai_transparency_external_evidence_intake.up.sql",
  "utf8",
);

assert.equal(success.schemaVersion, "hs-ai-external-evidence-intake-v1");
assert.equal(success.evidenceReference, `evidence://sha256/${success.evidenceSha256}`);
assert.equal(success.resultStatus, "received_for_review");
assert.equal(success.productionActivation, false);
assert.equal(success.partnerAcceptance, false);
assert.match(rejected.sourceReference, /replace-me/);
assert.match(moduleSource, /verify_actor_authorization/);
assert.match(moduleSource, /verify_approval_reference/);
assert.match(migration, /append_only/);

console.log(JSON.stringify({ ok: true, resultStatus: success.resultStatus }));

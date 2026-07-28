import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const root = "docs/contracts/ai-transparency-external-evidence-intake";
const success = JSON.parse(await readFile(`${root}/success-v1.fixture.json`, "utf8"));
const rejected = JSON.parse(await readFile(`${root}/reject-placeholder-v1.fixture.json`, "utf8"));
const acceptedReview = JSON.parse(await readFile(`${root}/accept-for-gate-v1.fixture.json`, "utf8"));
const rejectedReview = JSON.parse(await readFile(`${root}/reject-v1.fixture.json`, "utf8"));
const sameReviewer = JSON.parse(await readFile(`${root}/same-reviewer-rejection-v1.fixture.json`, "utf8"));
const expiredEvidence = JSON.parse(await readFile(`${root}/expired-evidence-rejection-v1.fixture.json`, "utf8"));
const deniedReference = JSON.parse(await readFile(`${root}/reference-denied-v1.fixture.json`, "utf8"));
const auditRollback = JSON.parse(await readFile(`${root}/audit-rollback-v1.fixture.json`, "utf8"));
const moduleSource = await readFile("feedback-backend/src/ai_transparency_external_evidence_intake.rs", "utf8");
const migration = await readFile(
  "feedback-backend/migrations/postgres/0021_ai_transparency_external_evidence_intake.up.sql",
  "utf8",
);
const reviewMigration = await readFile(
  "feedback-backend/migrations/postgres/0022_ai_transparency_external_evidence_review.up.sql",
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
assert.equal(acceptedReview.decision, "accepted_for_gate");
assert.equal(rejectedReview.decision, "rejected");
assert.equal(acceptedReview.productionActivation, false);
assert.equal(acceptedReview.partnerAcceptance, false);
assert.equal(sameReviewer.expectedError, "reviewer_or_evidence_window");
assert.equal(expiredEvidence.expectedError, "reviewer_or_evidence_window");
assert.equal(deniedReference.referenceVerified, false);
assert.equal(auditRollback.auditWrite, "injected_failure");
assert.equal(auditRollback.expectedDecisionRowsWritten, 0);
assert.equal(auditRollback.expectedAuditRowsWritten, 0);
assert.match(reviewMigration, /evidence_intake_id TEXT NOT NULL UNIQUE/);
assert.match(reviewMigration, /append_only/);
assert.match(moduleSource, /tenant_id: &tenant_id/);
assert.match(moduleSource, /ApprovalReferenceType::SecurityReview/);

console.log(JSON.stringify({ ok: true, resultStatus: success.resultStatus }));

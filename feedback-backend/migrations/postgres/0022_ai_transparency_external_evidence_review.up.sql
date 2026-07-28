CREATE TABLE ai_transparency_external_evidence_review_decisions (
    evidence_review_decision_id TEXT PRIMARY KEY,
    evidence_intake_id TEXT NOT NULL UNIQUE REFERENCES ai_transparency_external_evidence_intakes(evidence_intake_id),
    decision TEXT NOT NULL,
    reviewer_snapshot_id TEXT NOT NULL REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    review_reference TEXT NOT NULL,
    reason_digest TEXT NOT NULL,
    decided_at TIMESTAMPTZ NOT NULL,
    CHECK(decision IN ('accepted_for_gate', 'rejected')),
    CHECK(review_reference ~ '^approval://[^[:space:]]+$'),
    CHECK(reason_digest ~ '^[a-f0-9]{64}$')
);

CREATE TABLE ai_transparency_external_evidence_review_audit_events (
    evidence_review_audit_event_id TEXT PRIMARY KEY,
    evidence_review_decision_id TEXT NOT NULL REFERENCES ai_transparency_external_evidence_review_decisions(evidence_review_decision_id),
    event_type TEXT NOT NULL,
    actor_snapshot_id TEXT NOT NULL REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    event_digest TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(event_type IN ('evidence_accepted_for_gate', 'evidence_rejected')),
    CHECK(event_digest ~ '^[a-f0-9]{64}$')
);

CREATE OR REPLACE FUNCTION reject_ai_transparency_external_evidence_review_mutation()
RETURNS TRIGGER AS $$ BEGIN RAISE EXCEPTION 'ai_transparency_external_evidence_reviews_are_append_only'; END; $$ LANGUAGE plpgsql;
CREATE TRIGGER trg_ai_external_evidence_review_decisions_append_only BEFORE UPDATE OR DELETE ON ai_transparency_external_evidence_review_decisions FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_external_evidence_review_mutation();
CREATE TRIGGER trg_ai_external_evidence_review_audit_append_only BEFORE UPDATE OR DELETE ON ai_transparency_external_evidence_review_audit_events FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_external_evidence_review_mutation();

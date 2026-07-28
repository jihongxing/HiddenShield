CREATE TABLE IF NOT EXISTS ai_transparency_external_evidence_intakes (
    evidence_intake_id TEXT PRIMARY KEY,
    source_kind TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    source_reference TEXT NOT NULL,
    evidence_reference TEXT NOT NULL,
    evidence_sha256 TEXT NOT NULL,
    signer_reference TEXT NOT NULL,
    contract_reference TEXT NOT NULL,
    security_review_reference TEXT NOT NULL,
    submitter_snapshot_id TEXT NOT NULL REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    valid_from TIMESTAMPTZ NOT NULL,
    valid_until TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL,
    status TEXT NOT NULL DEFAULT 'received_for_review',
    created_at TIMESTAMPTZ NOT NULL,
    CHECK(source_kind IN ('provider_recovery', 'design_partner_sandbox')),
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(source_reference ~ '^(provider|partner)://[^[:space:]]+$'),
    CHECK(evidence_reference ~ '^evidence://sha256/[a-f0-9]{64}$'),
    CHECK(evidence_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK(evidence_reference = 'evidence://sha256/' || evidence_sha256),
    CHECK(signer_reference ~ '^(approval|receipt)://[^[:space:]]+$'),
    CHECK(contract_reference ~ '^approval://[^[:space:]]+$'),
    CHECK(security_review_reference ~ '^approval://[^[:space:]]+$'),
    CHECK(valid_until > valid_from),
    CHECK(status = 'received_for_review'),
    UNIQUE(source_kind, source_reference, evidence_sha256)
);

CREATE INDEX IF NOT EXISTS idx_ai_external_evidence_intakes_scope_received
ON ai_transparency_external_evidence_intakes(tenant_id, workspace_id, environment, received_at DESC);

CREATE TABLE IF NOT EXISTS ai_transparency_external_evidence_intake_audit_events (
    evidence_intake_audit_event_id TEXT PRIMARY KEY,
    evidence_intake_id TEXT NOT NULL REFERENCES ai_transparency_external_evidence_intakes(evidence_intake_id),
    event_type TEXT NOT NULL,
    actor_snapshot_id TEXT NOT NULL REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    event_digest TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(event_type = 'evidence_received'),
    CHECK(event_digest ~ '^[a-f0-9]{64}$')
);

CREATE INDEX IF NOT EXISTS idx_ai_external_evidence_intake_audit_events_intake
ON ai_transparency_external_evidence_intake_audit_events(evidence_intake_id, occurred_at);

CREATE OR REPLACE FUNCTION reject_ai_transparency_external_evidence_intake_mutation()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'ai_transparency_external_evidence_intakes_are_append_only';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_ai_external_evidence_intakes_append_only
BEFORE UPDATE OR DELETE ON ai_transparency_external_evidence_intakes
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_external_evidence_intake_mutation();

CREATE TRIGGER trg_ai_external_evidence_intake_audit_events_append_only
BEFORE UPDATE OR DELETE ON ai_transparency_external_evidence_intake_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_external_evidence_intake_mutation();

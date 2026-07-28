ALTER TABLE ai_transparency_actor_role_snapshots
DROP CONSTRAINT IF EXISTS ai_transparency_actor_role_snapshots_role_check;

ALTER TABLE ai_transparency_actor_role_snapshots
ADD CONSTRAINT ai_transparency_actor_role_snapshots_role_check
CHECK(role IN (
    'ai_transparency_requester',
    'ai_transparency_commercial_approver',
    'ai_transparency_compliance_approver',
    'ai_transparency_security_approver',
    'ai_transparency_readonly_auditor',
    'ai_transparency_delivery_operator',
    'system_executor'
));

CREATE TABLE IF NOT EXISTS ai_delivery_retrieval_authorizations (
    authorization_id TEXT PRIMARY KEY,
    delivery_envelope_id TEXT NOT NULL
        REFERENCES ai_post_embed_delivery_envelopes(delivery_envelope_id),
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    requester_snapshot_id TEXT NOT NULL
        REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    token_hash TEXT NOT NULL UNIQUE,
    envelope_digest TEXT NOT NULL,
    artifact_finalize_receipt_sha256 TEXT NOT NULL,
    status TEXT NOT NULL,
    granted_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(token_hash ~ '^[a-f0-9]{64}$'),
    CHECK(envelope_digest ~ '^[a-f0-9]{64}$'),
    CHECK(artifact_finalize_receipt_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK(status IN ('active', 'consumed', 'expired', 'revoked')),
    CHECK(expires_at > granted_at),
    CHECK(status <> 'consumed' OR consumed_at IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_retrieval_authorization_expiry
ON ai_delivery_retrieval_authorizations(status, expires_at);

CREATE TABLE IF NOT EXISTS ai_delivery_download_audit_events (
    download_audit_event_id TEXT PRIMARY KEY,
    authorization_id TEXT
        REFERENCES ai_delivery_retrieval_authorizations(authorization_id),
    delivery_envelope_id TEXT NOT NULL,
    execution_id TEXT NOT NULL,
    event_type TEXT NOT NULL,
    outcome TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    envelope_digest TEXT NOT NULL,
    final_file_sha256 TEXT NOT NULL,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(event_type IN ('authorization_granted', 'retrieval_claimed', 'retrieval_succeeded', 'retrieval_failed')),
    CHECK(outcome IN ('succeeded', 'denied', 'failed')),
    CHECK(reason_code <> ''),
    CHECK(envelope_digest ~ '^[a-f0-9]{64}$'),
    CHECK(final_file_sha256 ~ '^[a-f0-9]{64}$')
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_download_audit_envelope_time
ON ai_delivery_download_audit_events(delivery_envelope_id, occurred_at ASC);

DROP TRIGGER IF EXISTS trg_ai_delivery_download_audit_append_only
ON ai_delivery_download_audit_events;

CREATE TRIGGER trg_ai_delivery_download_audit_append_only
BEFORE UPDATE OR DELETE ON ai_delivery_download_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

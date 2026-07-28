ALTER TABLE ai_delivery_retrieval_authorizations
ADD COLUMN IF NOT EXISTS max_download_bytes BIGINT NOT NULL DEFAULT 67108864;

ALTER TABLE ai_delivery_retrieval_authorizations
ADD COLUMN IF NOT EXISTS required_content_type TEXT NOT NULL DEFAULT 'image/png';

ALTER TABLE ai_delivery_retrieval_authorizations
ADD COLUMN IF NOT EXISTS read_timeout_ms INTEGER NOT NULL DEFAULT 5000;

ALTER TABLE ai_delivery_retrieval_authorizations
ADD COLUMN IF NOT EXISTS rate_limit_per_minute INTEGER NOT NULL DEFAULT 30;

ALTER TABLE ai_delivery_retrieval_authorizations
ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ;

ALTER TABLE ai_delivery_retrieval_authorizations
ADD COLUMN IF NOT EXISTS revoked_by_snapshot_id TEXT
    REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id);

ALTER TABLE ai_delivery_retrieval_authorizations
ADD COLUMN IF NOT EXISTS revoke_reason TEXT;

ALTER TABLE ai_delivery_retrieval_authorizations
ADD CONSTRAINT ai_delivery_retrieval_authorizations_resource_budget_check
CHECK(
    max_download_bytes = 67108864
    AND required_content_type = 'image/png'
    AND read_timeout_ms = 5000
    AND rate_limit_per_minute = 30
);

ALTER TABLE ai_delivery_retrieval_authorizations
ADD CONSTRAINT ai_delivery_retrieval_authorizations_revocation_check
CHECK(
    status <> 'revoked'
    OR (
        revoked_at IS NOT NULL
        AND revoked_by_snapshot_id IS NOT NULL
        AND revoke_reason IS NOT NULL
        AND revoke_reason <> ''
    )
);

ALTER TABLE ai_delivery_download_audit_events
DROP CONSTRAINT IF EXISTS ai_delivery_download_audit_events_event_type_check;

ALTER TABLE ai_delivery_download_audit_events
ADD CONSTRAINT ai_delivery_download_audit_events_event_type_check
CHECK(event_type IN (
    'authorization_granted',
    'authorization_revoked',
    'retrieval_claimed',
    'retrieval_succeeded',
    'retrieval_failed'
));

CREATE TABLE IF NOT EXISTS ai_delivery_download_rate_limit_windows (
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    window_started_at TIMESTAMPTZ NOT NULL,
    claim_count INTEGER NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY(license_id, window_started_at),
    CHECK(claim_count >= 1 AND claim_count <= 30),
    CHECK(window_started_at = date_trunc('minute', window_started_at))
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_download_rate_limit_updated
ON ai_delivery_download_rate_limit_windows(updated_at);

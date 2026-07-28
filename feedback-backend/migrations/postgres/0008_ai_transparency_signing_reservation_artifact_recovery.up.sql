ALTER TABLE ai_post_embed_signing_executions
ADD COLUMN IF NOT EXISTS reservation_token TEXT,
ADD COLUMN IF NOT EXISTS lease_owner TEXT,
ADD COLUMN IF NOT EXISTS lease_expires_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS signer_invocation_key TEXT,
ADD COLUMN IF NOT EXISTS artifact_ref TEXT,
ADD COLUMN IF NOT EXISTS artifact_status TEXT NOT NULL DEFAULT 'none',
ADD COLUMN IF NOT EXISTS artifact_finalized_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS recovery_attempts INTEGER NOT NULL DEFAULT 0;

UPDATE ai_post_embed_signing_executions
SET artifact_ref = COALESCE(artifact_ref, 'legacy-post-embed://' || execution_id),
    artifact_status = 'finalized',
    artifact_finalized_at = COALESCE(artifact_finalized_at, updated_at)
WHERE status = 'confirmed';

UPDATE ai_post_embed_signing_executions
SET artifact_status = 'quarantined'
WHERE status = 'orphaned';

ALTER TABLE ai_post_embed_signing_executions
DROP CONSTRAINT IF EXISTS ai_post_embed_signing_executions_status_check;

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_executions_status_check
CHECK(status IN ('reserved', 'signed_staged', 'artifact_pending', 'confirmed', 'orphaned'));

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_artifact_status_check
CHECK(artifact_status IN ('none', 'staged', 'pending_finalize', 'finalized', 'quarantined'));

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_reservation_shape_check
CHECK(
    status <> 'reserved'
    OR (
        reservation_token IS NOT NULL
        AND lease_owner IS NOT NULL
        AND lease_expires_at IS NOT NULL
        AND signer_invocation_key IS NOT NULL
        AND artifact_status = 'none'
    )
);

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_staged_shape_check
CHECK(
    status <> 'signed_staged'
    OR (
        reservation_token IS NOT NULL
        AND signer_invocation_key IS NOT NULL
        AND final_signed_png_sha256 IS NOT NULL
        AND signer_receipt_id IS NOT NULL
        AND artifact_ref IS NOT NULL
        AND artifact_status = 'staged'
    )
);

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_pending_shape_check
CHECK(
    status <> 'artifact_pending'
    OR (
        final_signed_png_sha256 IS NOT NULL
        AND signer_receipt_id IS NOT NULL
        AND artifact_ref IS NOT NULL
        AND artifact_status = 'pending_finalize'
    )
);

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_finalized_shape_check
CHECK(
    status <> 'confirmed'
    OR (
        artifact_ref IS NOT NULL
        AND artifact_status = 'finalized'
        AND artifact_finalized_at IS NOT NULL
    )
);

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_recovery_attempts_check
CHECK(recovery_attempts >= 0);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_post_embed_signing_invocation_key
ON ai_post_embed_signing_executions(signer_invocation_key)
WHERE signer_invocation_key IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_ai_post_embed_signing_active_lease
ON ai_post_embed_signing_executions(lease_expires_at)
WHERE status = 'reserved';

CREATE INDEX IF NOT EXISTS idx_ai_post_embed_signing_artifact_pending
ON ai_post_embed_signing_executions(updated_at)
WHERE status = 'artifact_pending';

ALTER TABLE ai_post_embed_signing_audit_events
DROP CONSTRAINT IF EXISTS ai_post_embed_signing_audit_events_event_type_check;

ALTER TABLE ai_post_embed_signing_audit_events
ADD CONSTRAINT ai_post_embed_signing_audit_events_event_type_check
CHECK(event_type IN (
    'confirmed',
    'confirm_committed_artifact_pending',
    'artifact_finalized',
    'artifact_recovery_failed',
    'orphan_signing'
));

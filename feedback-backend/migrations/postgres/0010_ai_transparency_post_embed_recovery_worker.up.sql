ALTER TABLE ai_post_embed_signing_executions
ADD COLUMN IF NOT EXISTS recovery_state TEXT NOT NULL DEFAULT 'eligible',
ADD COLUMN IF NOT EXISTS worker_recovery_attempts INTEGER NOT NULL DEFAULT 0,
ADD COLUMN IF NOT EXISTS next_recovery_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
ADD COLUMN IF NOT EXISTS recovery_lease_owner TEXT,
ADD COLUMN IF NOT EXISTS recovery_lease_expires_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS last_recovery_reason TEXT,
ADD COLUMN IF NOT EXISTS dead_lettered_at TIMESTAMPTZ;

UPDATE ai_post_embed_signing_executions
SET recovery_state = 'completed',
    next_recovery_at = updated_at
WHERE status IN ('confirmed', 'orphaned');

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_recovery_state_check
CHECK(recovery_state IN ('eligible', 'leased', 'retry_scheduled', 'dead_letter', 'completed'));

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_worker_recovery_attempts_check
CHECK(worker_recovery_attempts >= 0);

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_recovery_lease_shape_check
CHECK(
    recovery_state <> 'leased'
    OR (
        recovery_lease_owner IS NOT NULL
        AND recovery_lease_expires_at IS NOT NULL
    )
);

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_recovery_dead_letter_shape_check
CHECK(
    recovery_state <> 'dead_letter'
    OR (
        dead_lettered_at IS NOT NULL
        AND last_recovery_reason IS NOT NULL
    )
);

CREATE INDEX IF NOT EXISTS idx_ai_post_embed_recovery_due
ON ai_post_embed_signing_executions(next_recovery_at, updated_at)
WHERE recovery_state IN ('eligible', 'retry_scheduled')
  AND status IN ('reserved', 'artifact_pending');

CREATE INDEX IF NOT EXISTS idx_ai_post_embed_recovery_lease_expiry
ON ai_post_embed_signing_executions(recovery_lease_expires_at)
WHERE recovery_state = 'leased';

CREATE TABLE IF NOT EXISTS ai_post_embed_recovery_audit_events (
    recovery_audit_event_id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL REFERENCES ai_post_embed_signing_executions(execution_id),
    worker_id TEXT NOT NULL,
    attempt INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    next_attempt_at TIMESTAMPTZ,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(attempt >= 1),
    CHECK(event_type IN ('claimed', 'succeeded', 'retry_scheduled', 'dead_letter')),
    CHECK(reason_code <> '')
);

CREATE INDEX IF NOT EXISTS idx_ai_post_embed_recovery_audit_execution_time
ON ai_post_embed_recovery_audit_events(execution_id, occurred_at ASC);

DROP TRIGGER IF EXISTS trg_ai_post_embed_recovery_audit_append_only
ON ai_post_embed_recovery_audit_events;

CREATE TRIGGER trg_ai_post_embed_recovery_audit_append_only
BEFORE UPDATE OR DELETE ON ai_post_embed_recovery_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

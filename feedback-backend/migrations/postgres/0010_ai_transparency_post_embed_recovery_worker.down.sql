DROP TRIGGER IF EXISTS trg_ai_post_embed_recovery_audit_append_only
ON ai_post_embed_recovery_audit_events;

DROP INDEX IF EXISTS idx_ai_post_embed_recovery_audit_execution_time;
DROP TABLE IF EXISTS ai_post_embed_recovery_audit_events;

DROP INDEX IF EXISTS idx_ai_post_embed_recovery_lease_expiry;
DROP INDEX IF EXISTS idx_ai_post_embed_recovery_due;

ALTER TABLE ai_post_embed_signing_executions
DROP CONSTRAINT IF EXISTS ai_post_embed_recovery_dead_letter_shape_check,
DROP CONSTRAINT IF EXISTS ai_post_embed_recovery_lease_shape_check,
DROP CONSTRAINT IF EXISTS ai_post_embed_worker_recovery_attempts_check,
DROP CONSTRAINT IF EXISTS ai_post_embed_recovery_state_check;

ALTER TABLE ai_post_embed_signing_executions
DROP COLUMN IF EXISTS dead_lettered_at,
DROP COLUMN IF EXISTS last_recovery_reason,
DROP COLUMN IF EXISTS recovery_lease_expires_at,
DROP COLUMN IF EXISTS recovery_lease_owner,
DROP COLUMN IF EXISTS next_recovery_at,
DROP COLUMN IF EXISTS worker_recovery_attempts,
DROP COLUMN IF EXISTS recovery_state;

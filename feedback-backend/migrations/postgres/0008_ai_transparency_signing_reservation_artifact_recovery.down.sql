ALTER TABLE ai_post_embed_signing_audit_events
DROP CONSTRAINT IF EXISTS ai_post_embed_signing_audit_events_event_type_check;

ALTER TABLE ai_post_embed_signing_audit_events
ADD CONSTRAINT ai_post_embed_signing_audit_events_event_type_check
CHECK(event_type IN ('confirmed', 'orphan_signing'));

DROP INDEX IF EXISTS idx_ai_post_embed_signing_artifact_pending;
DROP INDEX IF EXISTS idx_ai_post_embed_signing_active_lease;
DROP INDEX IF EXISTS idx_ai_post_embed_signing_invocation_key;

ALTER TABLE ai_post_embed_signing_executions
DROP CONSTRAINT IF EXISTS ai_post_embed_signing_recovery_attempts_check,
DROP CONSTRAINT IF EXISTS ai_post_embed_signing_finalized_shape_check,
DROP CONSTRAINT IF EXISTS ai_post_embed_signing_pending_shape_check,
DROP CONSTRAINT IF EXISTS ai_post_embed_signing_staged_shape_check,
DROP CONSTRAINT IF EXISTS ai_post_embed_signing_reservation_shape_check,
DROP CONSTRAINT IF EXISTS ai_post_embed_signing_artifact_status_check,
DROP CONSTRAINT IF EXISTS ai_post_embed_signing_executions_status_check;

ALTER TABLE ai_post_embed_signing_executions
DROP COLUMN IF EXISTS recovery_attempts,
DROP COLUMN IF EXISTS artifact_finalized_at,
DROP COLUMN IF EXISTS artifact_status,
DROP COLUMN IF EXISTS artifact_ref,
DROP COLUMN IF EXISTS signer_invocation_key,
DROP COLUMN IF EXISTS lease_expires_at,
DROP COLUMN IF EXISTS lease_owner,
DROP COLUMN IF EXISTS reservation_token;

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_executions_status_check
CHECK(status IN ('confirmed', 'orphaned'));

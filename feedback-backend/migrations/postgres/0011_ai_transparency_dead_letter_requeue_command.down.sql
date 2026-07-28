DROP TRIGGER IF EXISTS trg_ai_post_embed_dead_letter_inspection_append_only
ON ai_post_embed_dead_letter_inspection_audit_events;

DROP INDEX IF EXISTS idx_ai_post_embed_dead_letter_inspection_execution_time;
DROP TABLE IF EXISTS ai_post_embed_dead_letter_inspection_audit_events;

ALTER TABLE ai_transparency_change_requests
DROP CONSTRAINT IF EXISTS ai_transparency_change_requests_request_digest_version_check;

ALTER TABLE ai_transparency_change_requests
ADD CONSTRAINT ai_transparency_change_requests_request_digest_version_check
CHECK(request_digest_version = 'hs-ai-change-request-digest-v1');

ALTER TABLE ai_post_embed_signing_executions
DROP CONSTRAINT IF EXISTS ai_post_embed_recovery_control_version_check;

ALTER TABLE ai_post_embed_signing_executions
DROP COLUMN IF EXISTS requeued_at,
DROP COLUMN IF EXISTS last_requeue_change_request_id,
DROP COLUMN IF EXISTS recovery_control_version;

ALTER TABLE ai_transparency_change_audit_events
DROP CONSTRAINT IF EXISTS ai_transparency_change_audit_events_target_type_check;

ALTER TABLE ai_transparency_change_audit_events
ADD CONSTRAINT ai_transparency_change_audit_events_target_type_check
CHECK(target_type IN ('license', 'profile_entitlement'));

ALTER TABLE ai_transparency_change_requests
DROP CONSTRAINT IF EXISTS ai_transparency_change_requests_target_type_check;

ALTER TABLE ai_transparency_change_requests
ADD CONSTRAINT ai_transparency_change_requests_target_type_check
CHECK(target_type IN ('license', 'profile_entitlement'));

ALTER TABLE ai_transparency_change_requests
DROP CONSTRAINT IF EXISTS ai_transparency_change_requests_operation_check;

ALTER TABLE ai_transparency_change_requests
ADD CONSTRAINT ai_transparency_change_requests_operation_check
CHECK(operation IN (
    'create_license',
    'renew_license',
    'suspend_license',
    'revoke_license',
    'grant_profile_entitlement',
    'renew_profile_entitlement',
    'suspend_profile_entitlement',
    'revoke_profile_entitlement'
));

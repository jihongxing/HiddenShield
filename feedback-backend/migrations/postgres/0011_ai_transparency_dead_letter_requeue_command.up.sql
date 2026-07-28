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
    'revoke_profile_entitlement',
    'requeue_post_embed_dead_letter'
));

ALTER TABLE ai_transparency_change_requests
DROP CONSTRAINT IF EXISTS ai_transparency_change_requests_target_type_check;

ALTER TABLE ai_transparency_change_requests
ADD CONSTRAINT ai_transparency_change_requests_target_type_check
CHECK(target_type IN ('license', 'profile_entitlement', 'post_embed_recovery'));

ALTER TABLE ai_transparency_change_requests
DROP CONSTRAINT IF EXISTS ai_transparency_change_requests_request_digest_version_check;

ALTER TABLE ai_transparency_change_requests
ADD CONSTRAINT ai_transparency_change_requests_request_digest_version_check
CHECK(request_digest_version IN (
    'hs-ai-change-request-digest-v1',
    'hs-ai-post-embed-dead-letter-requeue-digest-v1'
));

ALTER TABLE ai_transparency_change_audit_events
DROP CONSTRAINT IF EXISTS ai_transparency_change_audit_events_target_type_check;

ALTER TABLE ai_transparency_change_audit_events
ADD CONSTRAINT ai_transparency_change_audit_events_target_type_check
CHECK(target_type IN ('license', 'profile_entitlement', 'post_embed_recovery'));

ALTER TABLE ai_post_embed_signing_executions
ADD COLUMN IF NOT EXISTS recovery_control_version INTEGER NOT NULL DEFAULT 1,
ADD COLUMN IF NOT EXISTS last_requeue_change_request_id TEXT
    REFERENCES ai_transparency_change_requests(change_request_id),
ADD COLUMN IF NOT EXISTS requeued_at TIMESTAMPTZ;

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_recovery_control_version_check
CHECK(recovery_control_version >= 1);

CREATE TABLE IF NOT EXISTS ai_post_embed_dead_letter_inspection_audit_events (
    inspection_audit_event_id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL,
    actor_snapshot_id TEXT NOT NULL
        REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    outcome TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(outcome IN ('succeeded', 'denied', 'not_found')),
    CHECK(reason_code <> '')
);

CREATE INDEX IF NOT EXISTS idx_ai_post_embed_dead_letter_inspection_execution_time
ON ai_post_embed_dead_letter_inspection_audit_events(execution_id, occurred_at ASC);

DROP TRIGGER IF EXISTS trg_ai_post_embed_dead_letter_inspection_append_only
ON ai_post_embed_dead_letter_inspection_audit_events;

CREATE TRIGGER trg_ai_post_embed_dead_letter_inspection_append_only
BEFORE UPDATE OR DELETE ON ai_post_embed_dead_letter_inspection_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

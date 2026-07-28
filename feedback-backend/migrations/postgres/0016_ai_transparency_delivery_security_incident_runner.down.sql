DROP TRIGGER IF EXISTS trg_ai_delivery_security_cleanup_runner_audit_append_only
ON ai_delivery_security_cleanup_runner_audit_events;
DROP TABLE IF EXISTS ai_delivery_security_cleanup_runner_audit_events;
DROP TABLE IF EXISTS ai_delivery_security_cleanup_schedules;

DROP TRIGGER IF EXISTS trg_ai_delivery_security_incident_audit_append_only
ON ai_delivery_security_incident_audit_events;
DROP TABLE IF EXISTS ai_delivery_security_incident_audit_events;
DROP TABLE IF EXISTS ai_delivery_security_incidents;

ALTER TABLE ai_transparency_change_audit_events
DROP CONSTRAINT IF EXISTS ai_transparency_change_audit_events_target_type_check;

ALTER TABLE ai_transparency_change_audit_events
ADD CONSTRAINT ai_transparency_change_audit_events_target_type_check
CHECK(target_type IN ('license', 'profile_entitlement', 'post_embed_recovery'));

ALTER TABLE ai_transparency_change_requests
DROP CONSTRAINT IF EXISTS ai_transparency_change_requests_request_digest_version_check;

ALTER TABLE ai_transparency_change_requests
ADD CONSTRAINT ai_transparency_change_requests_request_digest_version_check
CHECK(request_digest_version IN (
    'hs-ai-change-request-digest-v1',
    'hs-ai-post-embed-dead-letter-requeue-digest-v1'
));

ALTER TABLE ai_transparency_change_requests
DROP CONSTRAINT IF EXISTS ai_transparency_change_requests_target_type_check;

ALTER TABLE ai_transparency_change_requests
ADD CONSTRAINT ai_transparency_change_requests_target_type_check
CHECK(target_type IN ('license', 'profile_entitlement', 'post_embed_recovery'));

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

DROP TRIGGER IF EXISTS trg_ai_platform_api_audit_append_only
ON ai_platform_api_audit_events;

DROP TABLE IF EXISTS ai_platform_api_audit_events;
DROP TABLE IF EXISTS ai_platform_marking_submissions;
DROP TABLE IF EXISTS ai_platform_marking_sessions;
DROP TABLE IF EXISTS ai_platform_profile_admissions;

ALTER TABLE ai_runtime_credential_audit_events
DROP CONSTRAINT IF EXISTS ai_runtime_credential_audit_events_operation_check;

ALTER TABLE ai_runtime_credential_audit_events
ADD CONSTRAINT ai_runtime_credential_audit_events_operation_check
CHECK(operation IN (
    'issue_production_credential',
    'create_ready_marking_session',
    'rotate_production_credential',
    'revoke_production_credential'
));

ALTER TABLE ai_marking_sessions
DROP CONSTRAINT IF EXISTS ai_marking_sessions_status_check;

ALTER TABLE ai_marking_sessions
ADD CONSTRAINT ai_marking_sessions_status_check
CHECK(status IN (
    'reserved',
    'processing',
    'ready_to_confirm',
    'confirmed',
    'failed',
    'cancelled',
    'expired'
));

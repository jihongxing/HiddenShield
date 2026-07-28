DROP TRIGGER IF EXISTS trg_ai_delivery_download_audit_append_only
ON ai_delivery_download_audit_events;

DROP INDEX IF EXISTS idx_ai_delivery_download_audit_envelope_time;
DROP TABLE IF EXISTS ai_delivery_download_audit_events;

DROP INDEX IF EXISTS idx_ai_delivery_retrieval_authorization_expiry;
DROP TABLE IF EXISTS ai_delivery_retrieval_authorizations;

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
    'system_executor'
));

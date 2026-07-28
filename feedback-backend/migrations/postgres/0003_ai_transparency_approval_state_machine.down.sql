DROP TABLE IF EXISTS ai_transparency_change_target_locks;
DROP TRIGGER IF EXISTS trg_ai_change_audit_no_delete ON ai_transparency_change_audit_events;
DROP TRIGGER IF EXISTS trg_ai_change_audit_no_update ON ai_transparency_change_audit_events;
DROP FUNCTION IF EXISTS reject_ai_transparency_change_audit_mutation();
DROP TABLE IF EXISTS ai_transparency_change_audit_events;
DROP TABLE IF EXISTS ai_transparency_change_executions;
DROP TABLE IF EXISTS ai_transparency_change_approvals;

ALTER TABLE ai_profile_entitlements DROP COLUMN IF EXISTS projection_updated_at;
ALTER TABLE ai_profile_entitlements DROP COLUMN IF EXISTS current_version;
ALTER TABLE ai_profile_entitlements DROP COLUMN IF EXISTS current_version_id;

DROP TABLE IF EXISTS ai_profile_entitlement_versions;
DROP TABLE IF EXISTS ai_transparency_change_requests;
DROP TABLE IF EXISTS ai_transparency_actor_role_snapshots;

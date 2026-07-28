DROP TRIGGER IF EXISTS trg_ai_credential_lifecycle_audit_append_only
ON ai_credential_lifecycle_audit_events;

DROP INDEX IF EXISTS idx_ai_credential_lifecycle_audit_license_time;
DROP TABLE IF EXISTS ai_credential_lifecycle_audit_events;

DROP INDEX IF EXISTS idx_ai_sdk_credentials_rotated_from;

ALTER TABLE ai_sdk_credential_bindings
DROP COLUMN IF EXISTS rotated_at,
DROP COLUMN IF EXISTS rotated_from_credential_id;

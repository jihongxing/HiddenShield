DROP TRIGGER IF EXISTS trg_ai_runtime_credential_audit_append_only
ON ai_runtime_credential_audit_events;

DROP INDEX IF EXISTS idx_ai_runtime_credential_audit_license_time;
DROP TABLE IF EXISTS ai_runtime_credential_audit_events;

DROP INDEX IF EXISTS idx_ai_sdk_credentials_key_hash;
DROP INDEX IF EXISTS idx_ai_sdk_credentials_key_prefix;

ALTER TABLE ai_sdk_credential_bindings
DROP COLUMN IF EXISTS revoked_reason,
DROP COLUMN IF EXISTS revoked_at,
DROP COLUMN IF EXISTS last_used_at,
DROP COLUMN IF EXISTS issued_at,
DROP COLUMN IF EXISTS custody_key_id,
DROP COLUMN IF EXISTS issuer_modes_json,
DROP COLUMN IF EXISTS environment,
DROP COLUMN IF EXISTS hash_secret_version,
DROP COLUMN IF EXISTS key_hash,
DROP COLUMN IF EXISTS key_prefix;

ALTER TABLE ai_sdk_credential_bindings
ADD COLUMN IF NOT EXISTS key_prefix TEXT,
ADD COLUMN IF NOT EXISTS key_hash TEXT,
ADD COLUMN IF NOT EXISTS hash_secret_version TEXT,
ADD COLUMN IF NOT EXISTS environment TEXT,
ADD COLUMN IF NOT EXISTS issuer_modes_json JSONB,
ADD COLUMN IF NOT EXISTS custody_key_id TEXT,
ADD COLUMN IF NOT EXISTS issued_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS last_used_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS revoked_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS revoked_reason TEXT;

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_sdk_credentials_key_prefix
ON ai_sdk_credential_bindings(key_prefix)
WHERE key_prefix IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_sdk_credentials_key_hash
ON ai_sdk_credential_bindings(key_hash)
WHERE key_hash IS NOT NULL;

CREATE TABLE IF NOT EXISTS ai_runtime_credential_audit_events (
    audit_event_id TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    credential_id TEXT NOT NULL REFERENCES ai_sdk_credential_bindings(credential_id),
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    marking_session_id TEXT REFERENCES ai_marking_sessions(marking_session_id),
    custody_authorization_receipt_id TEXT NOT NULL,
    custody_key_id TEXT NOT NULL,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(operation IN ('issue_production_credential', 'create_ready_marking_session'))
);

CREATE INDEX IF NOT EXISTS idx_ai_runtime_credential_audit_license_time
ON ai_runtime_credential_audit_events(license_id, occurred_at DESC);

DROP TRIGGER IF EXISTS trg_ai_runtime_credential_audit_append_only
ON ai_runtime_credential_audit_events;

CREATE TRIGGER trg_ai_runtime_credential_audit_append_only
BEFORE UPDATE OR DELETE ON ai_runtime_credential_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

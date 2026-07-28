ALTER TABLE ai_sdk_credential_bindings
ADD COLUMN IF NOT EXISTS rotated_from_credential_id TEXT REFERENCES ai_sdk_credential_bindings(credential_id),
ADD COLUMN IF NOT EXISTS rotated_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_ai_sdk_credentials_rotated_from
ON ai_sdk_credential_bindings(rotated_from_credential_id)
WHERE rotated_from_credential_id IS NOT NULL;

CREATE TABLE IF NOT EXISTS ai_credential_lifecycle_audit_events (
    audit_event_id TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    previous_credential_id TEXT REFERENCES ai_sdk_credential_bindings(credential_id),
    resulting_credential_id TEXT REFERENCES ai_sdk_credential_bindings(credential_id),
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    custody_authorization_receipt_id TEXT NOT NULL,
    custody_key_id TEXT NOT NULL,
    reason TEXT,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(operation IN ('rotate_production_credential', 'revoke_production_credential'))
);

CREATE INDEX IF NOT EXISTS idx_ai_credential_lifecycle_audit_license_time
ON ai_credential_lifecycle_audit_events(license_id, occurred_at DESC);

DROP TRIGGER IF EXISTS trg_ai_credential_lifecycle_audit_append_only
ON ai_credential_lifecycle_audit_events;

CREATE TRIGGER trg_ai_credential_lifecycle_audit_append_only
BEFORE UPDATE OR DELETE ON ai_credential_lifecycle_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

ALTER TABLE ai_marking_sessions
DROP CONSTRAINT IF EXISTS ai_marking_sessions_status_check;

ALTER TABLE ai_marking_sessions
ADD CONSTRAINT ai_marking_sessions_status_check
CHECK(status IN (
    'reserved',
    'processing',
    'ready_to_upload',
    'ready_to_confirm',
    'confirmed',
    'failed',
    'cancelled',
    'expired'
));

ALTER TABLE ai_runtime_credential_audit_events
DROP CONSTRAINT IF EXISTS ai_runtime_credential_audit_events_operation_check;

ALTER TABLE ai_runtime_credential_audit_events
ADD CONSTRAINT ai_runtime_credential_audit_events_operation_check
CHECK(operation IN (
    'issue_production_credential',
    'create_ready_marking_session',
    'create_upload_marking_session',
    'rotate_production_credential',
    'revoke_production_credential'
));

CREATE TABLE IF NOT EXISTS ai_platform_profile_admissions (
    admission_id TEXT PRIMARY KEY,
    credential_id TEXT NOT NULL REFERENCES ai_sdk_credential_bindings(credential_id),
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    issuer_mode TEXT NOT NULL,
    regulatory_profile_id TEXT NOT NULL,
    technical_profile_ids_json JSONB NOT NULL,
    requested_profile_ids_json JSONB NOT NULL,
    entitlement_version_id TEXT NOT NULL,
    entitlement_digest TEXT NOT NULL,
    status TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    CHECK(environment = 'production'),
    CHECK(status IN ('admitted', 'expired', 'revoked')),
    CHECK(entitlement_digest ~ '^[a-f0-9]{64}$')
);

CREATE INDEX IF NOT EXISTS idx_ai_platform_admissions_license_status
ON ai_platform_profile_admissions(license_id, status, expires_at);

CREATE TABLE IF NOT EXISTS ai_platform_marking_sessions (
    marking_session_id TEXT PRIMARY KEY REFERENCES ai_marking_sessions(marking_session_id),
    admission_id TEXT NOT NULL REFERENCES ai_platform_profile_admissions(admission_id),
    watermark_uid TEXT NOT NULL UNIQUE,
    generation_event_id TEXT NOT NULL,
    subject_reference TEXT NOT NULL,
    content_type TEXT NOT NULL,
    entitlement_digest TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    CHECK(content_type = 'image/png'),
    CHECK(entitlement_digest ~ '^[a-f0-9]{64}$')
);

CREATE TABLE IF NOT EXISTS ai_platform_marking_submissions (
    submission_id TEXT PRIMARY KEY,
    marking_session_id TEXT NOT NULL UNIQUE REFERENCES ai_marking_sessions(marking_session_id),
    admission_id TEXT NOT NULL REFERENCES ai_platform_profile_admissions(admission_id),
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    watermark_uid TEXT NOT NULL,
    original_file_sha256 TEXT NOT NULL,
    marked_file_sha256 TEXT NOT NULL,
    confirmation_token_hash TEXT NOT NULL,
    marker_evidence_digest TEXT NOT NULL,
    explicit_label_receipt_digest TEXT NOT NULL,
    confirm_command_json JSONB NOT NULL,
    confirm_idempotency_key TEXT,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    confirmed_at TIMESTAMPTZ,
    CHECK(status IN ('ready_to_confirm', 'confirmed', 'failed')),
    CHECK(original_file_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK(marked_file_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK(confirmation_token_hash ~ '^[a-f0-9]{64}$'),
    CHECK(marker_evidence_digest ~ '^[a-f0-9]{64}$'),
    CHECK(explicit_label_receipt_digest ~ '^[a-f0-9]{64}$')
);

CREATE TABLE IF NOT EXISTS ai_platform_api_audit_events (
    audit_event_id TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    outcome TEXT NOT NULL,
    admission_id TEXT REFERENCES ai_platform_profile_admissions(admission_id),
    marking_session_id TEXT REFERENCES ai_marking_sessions(marking_session_id),
    license_id TEXT REFERENCES ai_transparency_licenses(license_id),
    reason_code TEXT,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(operation IN ('admit_profile', 'create_session', 'mark_image', 'confirm_image')),
    CHECK(outcome IN ('succeeded', 'replayed', 'denied', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_ai_platform_api_audit_license_time
ON ai_platform_api_audit_events(license_id, occurred_at DESC);

DROP TRIGGER IF EXISTS trg_ai_platform_api_audit_append_only
ON ai_platform_api_audit_events;

CREATE TRIGGER trg_ai_platform_api_audit_append_only
BEFORE UPDATE OR DELETE ON ai_platform_api_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

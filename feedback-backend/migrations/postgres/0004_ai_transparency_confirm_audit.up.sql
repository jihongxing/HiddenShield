CREATE TABLE IF NOT EXISTS ai_marking_confirm_audit_events (
    audit_event_id TEXT PRIMARY KEY,
    marking_session_id TEXT NOT NULL UNIQUE REFERENCES ai_marking_sessions(marking_session_id),
    transparency_manifest_id TEXT NOT NULL UNIQUE REFERENCES ai_transparency_manifests(transparency_manifest_id),
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    outcome TEXT NOT NULL,
    subject_digest TEXT NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(outcome = 'confirmed'),
    CHECK(subject_digest ~ '^[a-f0-9]{64}$')
);

CREATE INDEX IF NOT EXISTS idx_ai_marking_confirm_audit_license_time
ON ai_marking_confirm_audit_events(license_id, occurred_at DESC);

DROP TRIGGER IF EXISTS trg_ai_marking_confirm_audit_append_only
ON ai_marking_confirm_audit_events;

CREATE TRIGGER trg_ai_marking_confirm_audit_append_only
BEFORE UPDATE OR DELETE ON ai_marking_confirm_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

CREATE TABLE IF NOT EXISTS ai_post_embed_signing_executions (
    execution_id TEXT PRIMARY KEY,
    marking_session_id TEXT NOT NULL UNIQUE REFERENCES ai_marking_sessions(marking_session_id),
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    idempotency_key TEXT NOT NULL UNIQUE,
    request_digest TEXT NOT NULL,
    watermark_uid TEXT NOT NULL,
    unsigned_v3_png_sha256 TEXT NOT NULL,
    final_signed_png_sha256 TEXT,
    signer_receipt_id TEXT,
    status TEXT NOT NULL,
    reason_code TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CHECK(request_digest ~ '^[a-f0-9]{64}$'),
    CHECK(unsigned_v3_png_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK(final_signed_png_sha256 IS NULL OR final_signed_png_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK(status IN ('confirmed', 'orphaned')),
    CHECK(status <> 'confirmed' OR (
        final_signed_png_sha256 IS NOT NULL
        AND signer_receipt_id IS NOT NULL
        AND reason_code IS NULL
    )),
    CHECK(status <> 'orphaned' OR reason_code IS NOT NULL)
);

CREATE INDEX IF NOT EXISTS idx_ai_post_embed_signing_license_time
ON ai_post_embed_signing_executions(license_id, created_at DESC);

CREATE TABLE IF NOT EXISTS ai_post_embed_signing_audit_events (
    audit_event_id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL REFERENCES ai_post_embed_signing_executions(execution_id),
    event_type TEXT NOT NULL,
    subject_digest TEXT,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(event_type IN ('confirmed', 'orphan_signing')),
    CHECK(subject_digest IS NULL OR subject_digest ~ '^[a-f0-9]{64}$')
);

CREATE INDEX IF NOT EXISTS idx_ai_post_embed_signing_audit_execution_time
ON ai_post_embed_signing_audit_events(execution_id, occurred_at ASC);

DROP TRIGGER IF EXISTS trg_ai_post_embed_signing_audit_append_only
ON ai_post_embed_signing_audit_events;

CREATE TRIGGER trg_ai_post_embed_signing_audit_append_only
BEFORE UPDATE OR DELETE ON ai_post_embed_signing_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

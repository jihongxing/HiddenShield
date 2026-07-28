ALTER TABLE ai_post_embed_signing_executions
ADD COLUMN IF NOT EXISTS profile_entitlement_version INTEGER,
ADD COLUMN IF NOT EXISTS profile_entitlement_digest TEXT,
ADD COLUMN IF NOT EXISTS technical_profile_ids_json JSONB,
ADD COLUMN IF NOT EXISTS regional_profile_id TEXT;

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_profile_identity_shape_check
CHECK(
    profile_entitlement_version IS NULL
    OR (
        profile_entitlement_version >= 1
        AND profile_entitlement_digest ~ '^[a-f0-9]{64}$'
        AND jsonb_typeof(technical_profile_ids_json) = 'array'
        AND jsonb_array_length(technical_profile_ids_json) >= 1
        AND regional_profile_id IS NOT NULL
        AND regional_profile_id <> ''
    )
);

CREATE TABLE IF NOT EXISTS ai_post_embed_delivery_envelopes (
    delivery_envelope_id TEXT PRIMARY KEY,
    execution_id TEXT NOT NULL UNIQUE
        REFERENCES ai_post_embed_signing_executions(execution_id),
    schema_version TEXT NOT NULL,
    final_file_sha256 TEXT NOT NULL,
    signer_receipt_id TEXT NOT NULL,
    signer_receipt_sha256 TEXT NOT NULL,
    artifact_finalize_receipt_id TEXT NOT NULL,
    artifact_finalize_receipt_sha256 TEXT NOT NULL,
    profile_identity_digest TEXT NOT NULL,
    recovery_control_version INTEGER NOT NULL,
    envelope_digest TEXT NOT NULL UNIQUE,
    envelope_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    CHECK(schema_version = 'hs-ai-confirmed-artifact-delivery-envelope-v1'),
    CHECK(final_file_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK(signer_receipt_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK(artifact_finalize_receipt_sha256 ~ '^[a-f0-9]{64}$'),
    CHECK(profile_identity_digest ~ '^[a-f0-9]{64}$'),
    CHECK(envelope_digest ~ '^[a-f0-9]{64}$'),
    CHECK(recovery_control_version >= 1)
);

CREATE INDEX IF NOT EXISTS idx_ai_post_embed_delivery_created
ON ai_post_embed_delivery_envelopes(created_at DESC);

DROP TRIGGER IF EXISTS trg_ai_post_embed_delivery_envelope_append_only
ON ai_post_embed_delivery_envelopes;

CREATE TRIGGER trg_ai_post_embed_delivery_envelope_append_only
BEFORE UPDATE OR DELETE ON ai_post_embed_delivery_envelopes
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

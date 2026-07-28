DROP TRIGGER IF EXISTS trg_ai_post_embed_delivery_envelope_append_only
ON ai_post_embed_delivery_envelopes;

DROP INDEX IF EXISTS idx_ai_post_embed_delivery_created;
DROP TABLE IF EXISTS ai_post_embed_delivery_envelopes;

ALTER TABLE ai_post_embed_signing_executions
DROP CONSTRAINT IF EXISTS ai_post_embed_profile_identity_shape_check;

ALTER TABLE ai_post_embed_signing_executions
DROP COLUMN IF EXISTS regional_profile_id,
DROP COLUMN IF EXISTS technical_profile_ids_json,
DROP COLUMN IF EXISTS profile_entitlement_digest,
DROP COLUMN IF EXISTS profile_entitlement_version;

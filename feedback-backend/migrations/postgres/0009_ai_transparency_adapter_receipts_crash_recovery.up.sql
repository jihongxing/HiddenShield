ALTER TABLE ai_post_embed_signing_executions
ADD COLUMN IF NOT EXISTS adapter_receipt_contract_version TEXT,
ADD COLUMN IF NOT EXISTS signer_result_ref TEXT,
ADD COLUMN IF NOT EXISTS signer_billable_invocation_id TEXT,
ADD COLUMN IF NOT EXISTS signer_idempotency_disposition TEXT,
ADD COLUMN IF NOT EXISTS signer_receipt_json JSONB,
ADD COLUMN IF NOT EXISTS artifact_stage_receipt_id TEXT,
ADD COLUMN IF NOT EXISTS artifact_stage_receipt_json JSONB,
ADD COLUMN IF NOT EXISTS artifact_finalize_receipt_id TEXT,
ADD COLUMN IF NOT EXISTS artifact_finalize_receipt_json JSONB,
ADD COLUMN IF NOT EXISTS artifact_object_version TEXT;

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_adapter_contract_version_check
CHECK(
    adapter_receipt_contract_version IS NULL
    OR adapter_receipt_contract_version = 'hs-ai-production-adapter-receipts-v1'
);

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_signer_disposition_check
CHECK(
    signer_idempotency_disposition IS NULL
    OR signer_idempotency_disposition IN ('created', 'replayed')
);

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_adapter_receipt_shape_check
CHECK(
    adapter_receipt_contract_version IS NULL
    OR (
        status = 'reserved'
        OR (
            signer_result_ref IS NOT NULL
            AND signer_billable_invocation_id IS NOT NULL
            AND signer_idempotency_disposition IS NOT NULL
            AND signer_receipt_json IS NOT NULL
            AND artifact_stage_receipt_id IS NOT NULL
            AND artifact_stage_receipt_json IS NOT NULL
            AND artifact_object_version IS NOT NULL
        )
    )
);

ALTER TABLE ai_post_embed_signing_executions
ADD CONSTRAINT ai_post_embed_signing_finalize_receipt_shape_check
CHECK(
    adapter_receipt_contract_version IS NULL
    OR status <> 'confirmed'
    OR (
        artifact_finalize_receipt_id IS NOT NULL
        AND artifact_finalize_receipt_json IS NOT NULL
    )
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_post_embed_signing_billable_invocation
ON ai_post_embed_signing_executions(signer_billable_invocation_id)
WHERE signer_billable_invocation_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_post_embed_signing_artifact_stage_receipt
ON ai_post_embed_signing_executions(artifact_stage_receipt_id)
WHERE artifact_stage_receipt_id IS NOT NULL;

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_post_embed_signing_artifact_finalize_receipt
ON ai_post_embed_signing_executions(artifact_finalize_receipt_id)
WHERE artifact_finalize_receipt_id IS NOT NULL;

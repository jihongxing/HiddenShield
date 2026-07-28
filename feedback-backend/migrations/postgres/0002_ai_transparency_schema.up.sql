-- HiddenShield PostgreSQL P3: AI Transparency schema contract v1.
-- This migration creates storage only. It does not expose API, SDK, billing, or detector behavior.

CREATE TABLE IF NOT EXISTS ai_transparency_licenses (
    license_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    status TEXT NOT NULL,
    issuer_mode TEXT NOT NULL,
    deployment_mode TEXT NOT NULL,
    public_verification_required BOOLEAN NOT NULL,
    metering_plan_id TEXT NOT NULL,
    effective_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(status IN ('active', 'suspended', 'expired', 'revoked')),
    CHECK(issuer_mode IN ('hiddenshield_managed', 'platform_managed', 'customer_byok')),
    CHECK(deployment_mode IN ('hosted', 'private')),
    CHECK(expires_at > effective_at),
    CHECK(environment <> 'production' OR public_verification_required = TRUE)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_transparency_licenses_one_active
ON ai_transparency_licenses(tenant_id, workspace_id, environment)
WHERE status = 'active';

CREATE TABLE IF NOT EXISTS ai_profile_entitlements (
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    profile_id TEXT NOT NULL,
    profile_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    effective_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    terms_version TEXT NOT NULL,
    approved_by TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY(license_id, profile_id),
    CHECK(profile_kind IN ('regulatory', 'technical')),
    CHECK(status IN ('active', 'suspended', 'expired', 'revoked')),
    CHECK(expires_at > effective_at)
);

CREATE INDEX IF NOT EXISTS idx_ai_profile_entitlements_license_status
ON ai_profile_entitlements(license_id, status, expires_at);

CREATE TABLE IF NOT EXISTS ai_sdk_credential_bindings (
    credential_id TEXT PRIMARY KEY,
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    api_key_id TEXT NOT NULL UNIQUE,
    scopes_json JSONB NOT NULL,
    status TEXT NOT NULL,
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    CHECK(status IN ('active', 'suspended', 'revoked'))
);

CREATE INDEX IF NOT EXISTS idx_ai_sdk_credential_bindings_license_status
ON ai_sdk_credential_bindings(license_id, status);

CREATE TABLE IF NOT EXISTS ai_marking_sessions (
    marking_session_id TEXT PRIMARY KEY,
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    requested_profile_ids_json JSONB NOT NULL,
    claim_type TEXT NOT NULL,
    provider_content_id TEXT,
    status TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    confirmed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(license_id, idempotency_key),
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(claim_type IN ('ai_generated', 'ai_manipulated')),
    CHECK(status IN ('reserved', 'processing', 'ready_to_confirm', 'confirmed', 'failed', 'cancelled', 'expired'))
);

CREATE INDEX IF NOT EXISTS idx_ai_marking_sessions_license_status
ON ai_marking_sessions(license_id, status, created_at DESC);

CREATE TABLE IF NOT EXISTS ai_transparency_manifests (
    transparency_manifest_id TEXT PRIMARY KEY,
    marking_session_id TEXT NOT NULL UNIQUE REFERENCES ai_marking_sessions(marking_session_id),
    watermark_uid TEXT NOT NULL,
    manifest_version INTEGER NOT NULL,
    status TEXT NOT NULL,
    claim_type TEXT NOT NULL,
    modality TEXT NOT NULL,
    generation_mode TEXT NOT NULL,
    provider_id TEXT NOT NULL,
    system_name TEXT NOT NULL,
    system_version TEXT NOT NULL,
    model_id TEXT,
    model_version TEXT,
    operations_json JSONB NOT NULL,
    generated_at TIMESTAMPTZ NOT NULL,
    provider_content_id TEXT,
    subject_digest_algorithm TEXT NOT NULL,
    subject_digest_scope TEXT NOT NULL,
    subject_digest TEXT NOT NULL,
    parent_subjects_json JSONB NOT NULL,
    profile_status_json JSONB NOT NULL DEFAULT '[]'::jsonb,
    manifest_sha256 TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(watermark_uid, manifest_version),
    CHECK(status IN ('active', 'superseded', 'revoked', 'disputed')),
    CHECK(claim_type IN ('ai_generated', 'ai_manipulated')),
    CHECK(modality = 'image'),
    CHECK(subject_digest_algorithm = 'sha256'),
    CHECK(subject_digest_scope = 'protected_output'),
    CHECK(subject_digest ~ '^[a-f0-9]{64}$'),
    CHECK(manifest_sha256 ~ '^[a-f0-9]{64}$')
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_transparency_manifests_one_active
ON ai_transparency_manifests(watermark_uid)
WHERE status = 'active';

CREATE INDEX IF NOT EXISTS idx_ai_transparency_manifests_watermark_status
ON ai_transparency_manifests(watermark_uid, status, manifest_version DESC);

CREATE TABLE IF NOT EXISTS ai_claim_evidence (
    evidence_id TEXT PRIMARY KEY,
    transparency_manifest_id TEXT NOT NULL REFERENCES ai_transparency_manifests(transparency_manifest_id),
    evidence_level TEXT NOT NULL,
    evidence_source TEXT NOT NULL,
    issuer_id TEXT,
    key_id TEXT,
    proof_type TEXT NOT NULL,
    subject_digest TEXT NOT NULL,
    signature_algorithm TEXT,
    signature TEXT,
    verification_status TEXT NOT NULL,
    verified_at TIMESTAMPTZ,
    failure_code TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    CHECK(evidence_level IN ('self_declared', 'device_signed', 'registry_signed', 'platform_signed', 'externally_verified', 'unsupported_proof', 'invalid_proof')),
    CHECK(subject_digest ~ '^[a-f0-9]{64}$'),
    CHECK(
        evidence_level NOT IN ('platform_signed', 'registry_signed', 'externally_verified')
        OR (issuer_id IS NOT NULL AND key_id IS NOT NULL AND signature_algorithm IS NOT NULL AND signature IS NOT NULL)
    ),
    CHECK(
        evidence_level NOT IN ('unsupported_proof', 'invalid_proof')
        OR failure_code IS NOT NULL
    )
);

CREATE INDEX IF NOT EXISTS idx_ai_claim_evidence_manifest
ON ai_claim_evidence(transparency_manifest_id, created_at DESC);

CREATE TABLE IF NOT EXISTS ai_marker_bindings (
    marker_binding_id TEXT PRIMARY KEY,
    transparency_manifest_id TEXT NOT NULL REFERENCES ai_transparency_manifests(transparency_manifest_id),
    marker_type TEXT NOT NULL,
    marker_profile_id TEXT NOT NULL,
    marker_version TEXT NOT NULL,
    detector_scheme TEXT,
    detector_endpoint TEXT,
    signpost TEXT,
    embed_status TEXT NOT NULL,
    verify_status TEXT NOT NULL,
    binding_digest TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(transparency_manifest_id, marker_type, marker_profile_id),
    CHECK(marker_type IN ('c2pa', 'xmp', 'iptc', 'json_ld', 'blind_watermark', 'explicit_label'))
);

CREATE INDEX IF NOT EXISTS idx_ai_marker_bindings_manifest
ON ai_marker_bindings(transparency_manifest_id, marker_type);

CREATE TABLE IF NOT EXISTS ai_explicit_label_receipts (
    receipt_id TEXT PRIMARY KEY,
    transparency_manifest_id TEXT NOT NULL REFERENCES ai_transparency_manifests(transparency_manifest_id),
    profile_id TEXT NOT NULL,
    required_surface TEXT NOT NULL,
    render_mode TEXT NOT NULL,
    rendered_asset_digest TEXT,
    placement_json JSONB NOT NULL,
    locale TEXT NOT NULL,
    label_text TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL,
    applied_by TEXT NOT NULL,
    verification_status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(transparency_manifest_id, profile_id, required_surface),
    CHECK(required_surface IN ('platform_ui', 'exported_file', 'both')),
    CHECK(
        required_surface = 'platform_ui'
        OR (rendered_asset_digest IS NOT NULL AND rendered_asset_digest ~ '^[a-f0-9]{64}$')
    )
);

CREATE INDEX IF NOT EXISTS idx_ai_explicit_label_receipts_manifest
ON ai_explicit_label_receipts(transparency_manifest_id, profile_id);

CREATE TABLE IF NOT EXISTS ai_marking_ledger (
    ledger_entry_id TEXT PRIMARY KEY,
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    marking_session_id TEXT NOT NULL UNIQUE REFERENCES ai_marking_sessions(marking_session_id),
    transparency_manifest_id TEXT NOT NULL UNIQUE REFERENCES ai_transparency_manifests(transparency_manifest_id),
    metering_unit TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    ledger_status TEXT NOT NULL,
    committed_at TIMESTAMPTZ,
    reversal_reason TEXT,
    created_at TIMESTAMPTZ NOT NULL,
    CHECK(metering_unit = 'confirmed_marked_image'),
    CHECK(quantity = 1),
    CHECK(ledger_status IN ('pending', 'committed', 'reversed', 'no_charge'))
);

CREATE INDEX IF NOT EXISTS idx_ai_marking_ledger_license_status
ON ai_marking_ledger(license_id, ledger_status, created_at DESC);

CREATE TABLE IF NOT EXISTS ai_transparency_admin_audit_events (
    audit_event_id TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    outcome TEXT NOT NULL,
    endpoint TEXT NOT NULL,
    license_id TEXT,
    tenant_id TEXT,
    workspace_id TEXT,
    requested_profile_ids_json JSONB NOT NULL,
    reason_code TEXT NOT NULL,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(operation IN ('get_license', 'check_profile_entitlements')),
    CHECK(outcome IN ('succeeded', 'denied', 'failed'))
);

CREATE INDEX IF NOT EXISTS idx_ai_transparency_admin_audit_events_license_time
ON ai_transparency_admin_audit_events(license_id, occurred_at DESC);

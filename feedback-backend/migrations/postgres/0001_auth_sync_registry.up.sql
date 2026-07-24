-- HiddenShield PostgreSQL P2 migration: auth, cloud sync, and watermark registry.
-- This migration is intentionally limited to the first repository slices.

CREATE TABLE IF NOT EXISTS schema_migrations (
    version BIGINT PRIMARY KEY,
    name TEXT NOT NULL,
    checksum TEXT NOT NULL,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS cloud_accounts (
    id TEXT PRIMARY KEY,
    identifier TEXT NOT NULL UNIQUE,
    password_hash TEXT,
    password_salt TEXT,
    password_hash_algorithm TEXT NOT NULL DEFAULT 'sha256',
    display_name TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    workspace_name TEXT NOT NULL,
    creator_profile_id TEXT NOT NULL,
    creator_display_name TEXT NOT NULL,
    creator_seed_ref TEXT NOT NULL,
    seed_envelope_version INTEGER NOT NULL,
    entitlement_id TEXT NOT NULL,
    entitlement_plan_name TEXT NOT NULL,
    entitlement_plan_code TEXT NOT NULL,
    entitlement_status TEXT NOT NULL,
    entitlement_features_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS cloud_devices (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    client_device_id TEXT NOT NULL,
    name TEXT NOT NULL,
    platform TEXT NOT NULL,
    app_version TEXT NOT NULL,
    public_key TEXT,
    registered BOOLEAN NOT NULL,
    auto_sync_enabled BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(account_id, client_device_id)
);

CREATE TABLE IF NOT EXISTS cloud_sessions (
    access_token TEXT PRIMARY KEY,
    refresh_token TEXT NOT NULL,
    account_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ,
    refresh_expires_at TIMESTAMPTZ,
    last_used_at TIMESTAMPTZ,
    token_family_id TEXT
);

CREATE TABLE IF NOT EXISTS auth_challenges (
    challenge_id TEXT PRIMARY KEY,
    identifier TEXT NOT NULL,
    purpose TEXT NOT NULL,
    client_device_id TEXT NOT NULL,
    code_hash TEXT NOT NULL,
    code_salt TEXT NOT NULL,
    delivery_channel TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    plain_code_for_delivery TEXT
);

CREATE TABLE IF NOT EXISTS auth_attempts (
    attempt_id TEXT PRIMARY KEY,
    identifier TEXT NOT NULL,
    client_device_id TEXT,
    attempt_type TEXT NOT NULL,
    outcome TEXT NOT NULL,
    reason TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_auth_challenges_identifier_created
ON auth_challenges(identifier, created_at DESC);

CREATE INDEX IF NOT EXISTS idx_auth_attempts_identifier_created
ON auth_attempts(identifier, created_at DESC);

CREATE TABLE IF NOT EXISTS cloud_sync_events (
    sequence BIGSERIAL PRIMARY KEY,
    account_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    client_event_id TEXT NOT NULL,
    operation TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    payload_json JSONB NOT NULL,
    payload_hash TEXT,
    entity_revision BIGINT,
    created_at TIMESTAMPTZ NOT NULL,
    UNIQUE(account_id, device_id, client_event_id)
);

CREATE TABLE IF NOT EXISTS cloud_device_cursors (
    account_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    cursor TEXT NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    PRIMARY KEY(account_id, device_id)
);

CREATE INDEX IF NOT EXISTS idx_cloud_sync_events_account_sequence
ON cloud_sync_events(account_id, sequence ASC);

CREATE TABLE IF NOT EXISTS watermark_id_registry (
    registry_id TEXT PRIMARY KEY,
    request_id TEXT,
    account_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    creator_profile_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    watermark_uid TEXT NOT NULL UNIQUE,
    watermark_id_issue_mode TEXT NOT NULL,
    registry_status TEXT NOT NULL,
    registry_receipt TEXT NOT NULL,
    registry_proof_hash TEXT NOT NULL,
    media_type TEXT NOT NULL,
    payload_protocol_version INTEGER NOT NULL,
    payload_bytes_length INTEGER NOT NULL,
    parent_watermark_uid TEXT,
    revision INTEGER NOT NULL,
    original_hash TEXT,
    protected_copy_hash TEXT,
    write_verification_status TEXT,
    confirmed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(account_id, request_id)
);

CREATE INDEX IF NOT EXISTS idx_watermark_id_registry_account_workspace
ON watermark_id_registry(account_id, workspace_id, updated_at DESC);

CREATE INDEX IF NOT EXISTS idx_watermark_id_registry_parent
ON watermark_id_registry(parent_watermark_uid, revision);

CREATE TABLE IF NOT EXISTS watermark_id_reissue_jobs (
    job_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    creator_profile_id TEXT NOT NULL,
    previous_watermark_uid TEXT NOT NULL,
    replacement_watermark_uid TEXT NOT NULL,
    reason TEXT NOT NULL,
    status TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_watermark_id_reissue_jobs_account
ON watermark_id_reissue_jobs(account_id, workspace_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS rights_manifests (
    id TEXT PRIMARY KEY,
    rights_manifest_id TEXT NOT NULL UNIQUE,
    watermark_uid TEXT NOT NULL,
    manifest_version INTEGER NOT NULL,
    status TEXT NOT NULL,
    training_policy TEXT NOT NULL,
    work_source_declaration TEXT NOT NULL,
    creation_method_declaration TEXT NOT NULL,
    human_edit_level_declaration TEXT NOT NULL,
    authenticity_claim_declaration TEXT NOT NULL,
    tdm_reservation TEXT NOT NULL DEFAULT 'not_declared',
    search_indexing_policy TEXT NOT NULL DEFAULT 'not_declared',
    embedding_policy TEXT NOT NULL DEFAULT 'not_declared',
    commercial_training_policy TEXT NOT NULL DEFAULT 'not_declared',
    custom_terms_url TEXT,
    custom_terms_hash TEXT,
    standard_mappings_json JSONB NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    signed_by TEXT NOT NULL,
    signature TEXT NOT NULL,
    effective_at TIMESTAMPTZ NOT NULL,
    superseded_by_rights_manifest_id TEXT,
    revoked_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(watermark_uid, manifest_version),
    CHECK(status IN ('active', 'superseded', 'revoked', 'disputed')),
    CHECK(status != 'active' OR (manifest_sha256 != '' AND signature != '')),
    CHECK(custom_terms_hash IS NULL OR custom_terms_url IS NOT NULL)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_rights_manifests_one_active
ON rights_manifests(watermark_uid)
WHERE status = 'active';

CREATE INDEX IF NOT EXISTS idx_rights_manifests_watermark
ON rights_manifests(watermark_uid);

CREATE INDEX IF NOT EXISTS idx_rights_manifests_watermark_status
ON rights_manifests(watermark_uid, status);

CREATE INDEX IF NOT EXISTS idx_rights_manifests_watermark_version
ON rights_manifests(watermark_uid, manifest_version DESC);

CREATE INDEX IF NOT EXISTS idx_rights_manifests_status_updated
ON rights_manifests(status, updated_at DESC);

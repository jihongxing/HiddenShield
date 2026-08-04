CREATE TABLE IF NOT EXISTS cloud_copyright_workspaces (
    workspace_id TEXT PRIMARY KEY,
    owner_account_id TEXT NOT NULL REFERENCES cloud_accounts(id),
    workspace_type TEXT NOT NULL CHECK (workspace_type IN ('personal', 'team')),
    status TEXT NOT NULL CHECK (status IN ('active', 'suspended', 'archived')),
    membership_version BIGINT NOT NULL DEFAULT 1 CHECK (membership_version >= 1),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_cloud_copyright_one_active_personal_workspace
ON cloud_copyright_workspaces(owner_account_id)
WHERE workspace_type = 'personal' AND status = 'active';

CREATE TABLE IF NOT EXISTS cloud_copyright_workspace_memberships (
    membership_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES cloud_copyright_workspaces(workspace_id),
    account_id TEXT NOT NULL REFERENCES cloud_accounts(id),
    role TEXT NOT NULL CHECK (role IN ('owner', 'admin', 'editor', 'viewer')),
    status TEXT NOT NULL CHECK (status IN ('invited', 'active', 'removed')),
    membership_version BIGINT NOT NULL DEFAULT 1 CHECK (membership_version >= 1),
    invited_by_account_id TEXT REFERENCES cloud_accounts(id),
    joined_at TIMESTAMPTZ,
    removed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(workspace_id, account_id)
);

CREATE INDEX IF NOT EXISTS idx_cloud_copyright_memberships_account_workspace
ON cloud_copyright_workspace_memberships(account_id, workspace_id)
WHERE status = 'active';

CREATE TABLE IF NOT EXISTS cloud_copyright_creator_profiles (
    creator_profile_id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES cloud_accounts(id),
    display_name TEXT NOT NULL,
    seed_envelope_ref TEXT NOT NULL,
    seed_envelope_version INTEGER NOT NULL CHECK (seed_envelope_version >= 1),
    status TEXT NOT NULL CHECK (status IN ('active', 'archived')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_cloud_copyright_creator_profiles_account
ON cloud_copyright_creator_profiles(account_id, status, updated_at DESC);

CREATE TABLE IF NOT EXISTS cloud_copyright_records (
    record_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES cloud_copyright_workspaces(workspace_id),
    owner_account_id TEXT NOT NULL REFERENCES cloud_accounts(id),
    creator_profile_id TEXT NOT NULL REFERENCES cloud_copyright_creator_profiles(creator_profile_id),
    origin_device_id TEXT NOT NULL REFERENCES cloud_devices(id),
    record_kind TEXT NOT NULL CHECK (record_kind IN ('image', 'audio', 'video', 'other')),
    watermark_uid TEXT NOT NULL,
    watermark_revision BIGINT NOT NULL CHECK (watermark_revision >= 1),
    parent_watermark_uid TEXT,
    original_hash TEXT NOT NULL,
    protected_copy_hash TEXT NOT NULL,
    evidence_digest TEXT NOT NULL,
    write_verification_status TEXT NOT NULL,
    rights_declaration_json JSONB NOT NULL,
    classification TEXT NOT NULL CHECK (classification = 'private_metadata'),
    visibility TEXT NOT NULL CHECK (visibility = 'workspace_members'),
    record_version BIGINT NOT NULL CHECK (record_version >= 1),
    etag TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    deleted_at TIMESTAMPTZ,
    UNIQUE(workspace_id, watermark_uid, watermark_revision)
);

CREATE INDEX IF NOT EXISTS idx_cloud_copyright_records_workspace_updated
ON cloud_copyright_records(workspace_id, updated_at DESC, record_id);

CREATE TABLE IF NOT EXISTS cloud_copyright_changes (
    change_id TEXT PRIMARY KEY,
    workspace_id TEXT NOT NULL REFERENCES cloud_copyright_workspaces(workspace_id),
    device_id TEXT NOT NULL REFERENCES cloud_devices(id),
    record_id TEXT NOT NULL REFERENCES cloud_copyright_records(record_id),
    idempotency_key TEXT NOT NULL,
    request_digest TEXT NOT NULL,
    operation TEXT NOT NULL CHECK (operation IN ('upsert_record', 'tombstone_record')),
    base_record_version BIGINT NOT NULL CHECK (base_record_version >= 1),
    status TEXT NOT NULL CHECK (status IN ('accepted', 'duplicate')),
    record_version BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    UNIQUE(workspace_id, device_id, idempotency_key)
);

CREATE INDEX IF NOT EXISTS idx_cloud_copyright_changes_workspace_record
ON cloud_copyright_changes(workspace_id, record_id, created_at DESC);

CREATE TABLE IF NOT EXISTS cloud_copyright_events (
    sequence BIGSERIAL PRIMARY KEY,
    event_id TEXT NOT NULL UNIQUE,
    workspace_id TEXT NOT NULL REFERENCES cloud_copyright_workspaces(workspace_id),
    record_id TEXT NOT NULL REFERENCES cloud_copyright_records(record_id),
    change_id TEXT NOT NULL REFERENCES cloud_copyright_changes(change_id),
    event_type TEXT NOT NULL CHECK (event_type IN ('record_upserted', 'record_tombstoned')),
    record_version BIGINT NOT NULL,
    payload_json JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_cloud_copyright_events_workspace_sequence
ON cloud_copyright_events(workspace_id, sequence ASC);

CREATE TABLE IF NOT EXISTS cloud_copyright_audit_events (
    sequence BIGSERIAL PRIMARY KEY,
    audit_event_id TEXT NOT NULL UNIQUE,
    workspace_id TEXT NOT NULL REFERENCES cloud_copyright_workspaces(workspace_id),
    actor_account_id TEXT NOT NULL REFERENCES cloud_accounts(id),
    actor_membership_id TEXT NOT NULL REFERENCES cloud_copyright_workspace_memberships(membership_id),
    actor_device_id TEXT REFERENCES cloud_devices(id),
    action TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT NOT NULL,
    request_digest TEXT NOT NULL,
    previous_event_hash TEXT,
    event_hash TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_cloud_copyright_audit_workspace_sequence
ON cloud_copyright_audit_events(workspace_id, sequence ASC);

CREATE OR REPLACE FUNCTION cloud_copyright_reject_append_only_mutation()
RETURNS trigger
LANGUAGE plpgsql
AS $$
BEGIN
    RAISE EXCEPTION 'cloud copyright append-only rows cannot be updated or deleted';
END;
$$;

CREATE TRIGGER cloud_copyright_events_append_only
BEFORE UPDATE OR DELETE ON cloud_copyright_events
FOR EACH ROW EXECUTE FUNCTION cloud_copyright_reject_append_only_mutation();

CREATE TRIGGER cloud_copyright_audit_events_append_only
BEFORE UPDATE OR DELETE ON cloud_copyright_audit_events
FOR EACH ROW EXECUTE FUNCTION cloud_copyright_reject_append_only_mutation();

CREATE TABLE IF NOT EXISTS cloud_copyright_workspace_cursors (
    workspace_id TEXT NOT NULL REFERENCES cloud_copyright_workspaces(workspace_id),
    device_id TEXT NOT NULL REFERENCES cloud_devices(id),
    cursor_sequence BIGINT NOT NULL DEFAULT 0 CHECK (cursor_sequence >= 0),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY(workspace_id, device_id)
);

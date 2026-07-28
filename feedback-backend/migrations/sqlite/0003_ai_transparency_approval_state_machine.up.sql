CREATE TABLE IF NOT EXISTS ai_transparency_actor_role_snapshots (
    actor_role_snapshot_id TEXT PRIMARY KEY,
    actor_id TEXT NOT NULL,
    actor_type TEXT NOT NULL,
    role TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    role_binding_id TEXT NOT NULL,
    role_binding_version INTEGER NOT NULL,
    source_identity_system TEXT NOT NULL,
    authentication_level TEXT NOT NULL,
    captured_at TEXT NOT NULL,
    source_expires_at TEXT NOT NULL,
    snapshot_sha256 TEXT NOT NULL,
    CHECK(actor_type IN ('human', 'service', 'system')),
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(role IN ('ai_transparency_requester', 'ai_transparency_commercial_approver', 'ai_transparency_compliance_approver', 'ai_transparency_security_approver', 'ai_transparency_readonly_auditor', 'system_executor')),
    CHECK(source_identity_system = 'hiddenshield_internal_iam'),
    CHECK(role_binding_version >= 1),
    CHECK(length(snapshot_sha256) = 64 AND snapshot_sha256 NOT GLOB '*[^0-9a-f]*'),
    CHECK((role = 'system_executor' AND actor_type = 'system') OR (role <> 'system_executor' AND actor_type = 'human'))
);

CREATE INDEX IF NOT EXISTS idx_ai_actor_role_snapshots_actor_scope
ON ai_transparency_actor_role_snapshots(actor_id, tenant_id, workspace_id, environment, captured_at DESC);

CREATE TABLE IF NOT EXISTS ai_transparency_change_requests (
    change_request_id TEXT PRIMARY KEY,
    operation TEXT NOT NULL,
    target_type TEXT NOT NULL,
    target_id TEXT,
    target_scope_key TEXT NOT NULL,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    expected_target_version INTEGER,
    desired_next_version INTEGER,
    desired_state_json TEXT NOT NULL,
    request_reason TEXT NOT NULL,
    contract_reference TEXT,
    legal_review_reference TEXT,
    security_review_reference TEXT,
    requester_snapshot_id TEXT NOT NULL REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    request_digest_version TEXT NOT NULL,
    request_digest TEXT NOT NULL,
    idempotency_key TEXT NOT NULL,
    status TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    supersedes_change_request_id TEXT REFERENCES ai_transparency_change_requests(change_request_id),
    evidence_quality TEXT NOT NULL DEFAULT 'native_four_eyes',
    production_eligibility INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(requester_snapshot_id, idempotency_key),
    CHECK(operation IN ('create_license', 'renew_license', 'suspend_license', 'revoke_license', 'grant_profile_entitlement', 'renew_profile_entitlement', 'suspend_profile_entitlement', 'revoke_profile_entitlement')),
    CHECK(target_type IN ('license', 'profile_entitlement')),
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(expected_target_version IS NULL OR expected_target_version >= 1),
    CHECK(desired_next_version IS NULL OR desired_next_version >= 1),
    CHECK(request_reason <> ''),
    CHECK(request_digest_version = 'hs-ai-change-request-digest-v1'),
    CHECK(length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*'),
    CHECK(status IN ('draft', 'pending_review', 'approved', 'executing', 'succeeded', 'rejected', 'expired', 'cancelled', 'failed', 'conflict')),
    CHECK(evidence_quality IN ('native_four_eyes', 'migrated_legacy_without_four_eyes')),
    CHECK(evidence_quality <> 'migrated_legacy_without_four_eyes' OR production_eligibility = 0)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_change_requests_one_inflight_target
ON ai_transparency_change_requests(target_scope_key)
WHERE status IN ('pending_review', 'approved', 'executing');

CREATE INDEX IF NOT EXISTS idx_ai_change_requests_scope_status
ON ai_transparency_change_requests(tenant_id, workspace_id, environment, status, created_at DESC);

CREATE TABLE IF NOT EXISTS ai_profile_entitlement_versions (
    profile_entitlement_version_id TEXT PRIMARY KEY,
    license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
    profile_id TEXT NOT NULL,
    version INTEGER NOT NULL,
    previous_version_id TEXT REFERENCES ai_profile_entitlement_versions(profile_entitlement_version_id),
    profile_kind TEXT NOT NULL,
    status TEXT NOT NULL,
    effective_at TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    terms_version TEXT NOT NULL,
    legal_review_reference TEXT,
    security_review_reference TEXT,
    source_change_request_id TEXT NOT NULL UNIQUE REFERENCES ai_transparency_change_requests(change_request_id),
    created_at TEXT NOT NULL,
    superseded_at TEXT,
    UNIQUE(license_id, profile_id, version),
    CHECK(version >= 1),
    CHECK(profile_kind IN ('regulatory', 'technical')),
    CHECK(status IN ('active', 'suspended', 'expired', 'revoked', 'superseded')),
    CHECK(version = 1 OR previous_version_id IS NOT NULL),
    CHECK(profile_kind <> 'regulatory' OR legal_review_reference IS NOT NULL),
    CHECK(profile_kind <> 'technical' OR security_review_reference IS NOT NULL)
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_profile_entitlement_versions_one_active
ON ai_profile_entitlement_versions(license_id, profile_id)
WHERE status = 'active';

CREATE INDEX IF NOT EXISTS idx_ai_profile_entitlement_versions_history
ON ai_profile_entitlement_versions(license_id, profile_id, version DESC);

CREATE TABLE IF NOT EXISTS ai_transparency_change_approvals (
    approval_id TEXT PRIMARY KEY,
    change_request_id TEXT NOT NULL UNIQUE REFERENCES ai_transparency_change_requests(change_request_id),
    decision TEXT NOT NULL,
    approver_snapshot_id TEXT NOT NULL REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    requester_actor_id TEXT NOT NULL,
    approver_actor_id TEXT NOT NULL,
    approver_role TEXT NOT NULL,
    decision_reason TEXT NOT NULL,
    policy_version TEXT NOT NULL,
    request_digest TEXT NOT NULL,
    decided_at TEXT NOT NULL,
    CHECK(decision IN ('approved', 'rejected')),
    CHECK(requester_actor_id <> approver_actor_id),
    CHECK(decision_reason <> ''),
    CHECK(length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*')
);

CREATE TABLE IF NOT EXISTS ai_transparency_change_executions (
    execution_id TEXT PRIMARY KEY,
    change_request_id TEXT NOT NULL UNIQUE REFERENCES ai_transparency_change_requests(change_request_id),
    executor_snapshot_id TEXT NOT NULL REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    status TEXT NOT NULL,
    target_version_before INTEGER,
    target_version_after INTEGER,
    resulting_entitlement_version_id TEXT REFERENCES ai_profile_entitlement_versions(profile_entitlement_version_id),
    reason_code TEXT,
    started_at TEXT NOT NULL,
    finished_at TEXT,
    CHECK(status IN ('executing', 'succeeded', 'failed', 'conflict')),
    CHECK(target_version_before IS NULL OR target_version_before >= 1),
    CHECK(target_version_after IS NULL OR target_version_after >= 1),
    CHECK(status <> 'succeeded' OR (finished_at IS NOT NULL AND target_version_after IS NOT NULL)),
    CHECK(status NOT IN ('failed', 'conflict') OR reason_code IS NOT NULL)
);

CREATE TABLE IF NOT EXISTS ai_transparency_change_audit_events (
    audit_event_id TEXT PRIMARY KEY,
    change_request_id TEXT NOT NULL REFERENCES ai_transparency_change_requests(change_request_id),
    sequence INTEGER NOT NULL,
    event_type TEXT NOT NULL,
    from_state TEXT,
    to_state TEXT NOT NULL,
    actor_snapshot_id TEXT NOT NULL REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    target_type TEXT NOT NULL,
    target_id TEXT,
    target_version_before INTEGER,
    target_version_after INTEGER,
    reason_code TEXT NOT NULL,
    request_digest TEXT NOT NULL,
    details_json TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    UNIQUE(change_request_id, sequence),
    UNIQUE(change_request_id, event_type, to_state, target_version_after),
    CHECK(sequence >= 1),
    CHECK(event_type IN ('change_request_drafted', 'change_request_submitted', 'change_request_cancelled', 'approval_granted', 'approval_rejected', 'approval_expired', 'execution_started', 'execution_succeeded', 'execution_failed', 'execution_conflict', 'target_state_changed')),
    CHECK(target_type IN ('license', 'profile_entitlement')),
    CHECK(reason_code <> ''),
    CHECK(length(request_digest) = 64 AND request_digest NOT GLOB '*[^0-9a-f]*')
);

CREATE INDEX IF NOT EXISTS idx_ai_change_audit_events_request_sequence
ON ai_transparency_change_audit_events(change_request_id, sequence);

CREATE TRIGGER IF NOT EXISTS trg_ai_change_audit_no_update
BEFORE UPDATE ON ai_transparency_change_audit_events
BEGIN
    SELECT RAISE(ABORT, 'ai_transparency_change_audit_events is append-only');
END;

CREATE TRIGGER IF NOT EXISTS trg_ai_change_audit_no_delete
BEFORE DELETE ON ai_transparency_change_audit_events
BEGIN
    SELECT RAISE(ABORT, 'ai_transparency_change_audit_events is append-only');
END;

CREATE TABLE IF NOT EXISTS ai_transparency_change_target_locks (
    target_scope_key TEXT PRIMARY KEY,
    updated_at TEXT NOT NULL
);

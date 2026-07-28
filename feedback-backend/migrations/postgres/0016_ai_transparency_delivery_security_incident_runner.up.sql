ALTER TABLE ai_transparency_change_requests
DROP CONSTRAINT IF EXISTS ai_transparency_change_requests_operation_check;

ALTER TABLE ai_transparency_change_requests
ADD CONSTRAINT ai_transparency_change_requests_operation_check
CHECK(operation IN (
    'create_license',
    'renew_license',
    'suspend_license',
    'revoke_license',
    'grant_profile_entitlement',
    'renew_profile_entitlement',
    'suspend_profile_entitlement',
    'revoke_profile_entitlement',
    'requeue_post_embed_dead_letter',
    'ack_delivery_security_incident',
    'resolve_delivery_security_incident'
));

ALTER TABLE ai_transparency_change_requests
DROP CONSTRAINT IF EXISTS ai_transparency_change_requests_target_type_check;

ALTER TABLE ai_transparency_change_requests
ADD CONSTRAINT ai_transparency_change_requests_target_type_check
CHECK(target_type IN (
    'license',
    'profile_entitlement',
    'post_embed_recovery',
    'delivery_security_incident'
));

ALTER TABLE ai_transparency_change_requests
DROP CONSTRAINT IF EXISTS ai_transparency_change_requests_request_digest_version_check;

ALTER TABLE ai_transparency_change_requests
ADD CONSTRAINT ai_transparency_change_requests_request_digest_version_check
CHECK(request_digest_version IN (
    'hs-ai-change-request-digest-v1',
    'hs-ai-post-embed-dead-letter-requeue-digest-v1',
    'hs-ai-delivery-security-incident-change-digest-v1'
));

ALTER TABLE ai_transparency_change_audit_events
DROP CONSTRAINT IF EXISTS ai_transparency_change_audit_events_target_type_check;

ALTER TABLE ai_transparency_change_audit_events
ADD CONSTRAINT ai_transparency_change_audit_events_target_type_check
CHECK(target_type IN (
    'license',
    'profile_entitlement',
    'post_embed_recovery',
    'delivery_security_incident'
));

CREATE TABLE IF NOT EXISTS ai_delivery_security_incidents (
    incident_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    incident_key TEXT NOT NULL,
    active_incident_key TEXT UNIQUE,
    severity TEXT NOT NULL,
    status TEXT NOT NULL,
    alert_codes_json JSONB NOT NULL,
    occurrence_count BIGINT NOT NULL,
    first_summary_id TEXT NOT NULL,
    first_summary_digest TEXT NOT NULL,
    latest_summary_id TEXT NOT NULL,
    latest_summary_digest TEXT NOT NULL,
    control_version INTEGER NOT NULL,
    acknowledged_by_change_request_id TEXT
        REFERENCES ai_transparency_change_requests(change_request_id),
    acknowledged_at TIMESTAMPTZ,
    resolved_by_change_request_id TEXT
        REFERENCES ai_transparency_change_requests(change_request_id),
    resolved_at TIMESTAMPTZ,
    opened_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(severity IN ('warning', 'critical')),
    CHECK(status IN ('open', 'acknowledged', 'resolved')),
    CHECK(occurrence_count >= 1),
    CHECK(first_summary_digest ~ '^[a-f0-9]{64}$'),
    CHECK(latest_summary_digest ~ '^[a-f0-9]{64}$'),
    CHECK(control_version >= 1),
    CHECK(
        (status = 'resolved' AND active_incident_key IS NULL AND resolved_at IS NOT NULL)
        OR
        (status <> 'resolved' AND active_incident_key = incident_key AND resolved_at IS NULL)
    ),
    CHECK(
        (status = 'open' AND acknowledged_at IS NULL)
        OR
        (status IN ('acknowledged', 'resolved'))
    ),
    CHECK(
        (acknowledged_by_change_request_id IS NULL AND acknowledged_at IS NULL)
        OR
        (acknowledged_by_change_request_id IS NOT NULL AND acknowledged_at IS NOT NULL)
    ),
    CHECK(
        status <> 'acknowledged'
        OR acknowledged_by_change_request_id IS NOT NULL
    ),
    CHECK(
        (resolved_by_change_request_id IS NULL AND resolved_at IS NULL)
        OR
        (resolved_by_change_request_id IS NOT NULL AND resolved_at IS NOT NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_incidents_scope_status
ON ai_delivery_security_incidents(
    tenant_id, workspace_id, environment, status, updated_at DESC
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_incidents_latest_summary
ON ai_delivery_security_incidents(latest_summary_id);

CREATE TABLE IF NOT EXISTS ai_delivery_security_incident_audit_events (
    incident_audit_event_id TEXT PRIMARY KEY,
    incident_id TEXT NOT NULL REFERENCES ai_delivery_security_incidents(incident_id),
    event_type TEXT NOT NULL,
    actor_snapshot_id TEXT NOT NULL
        REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    change_request_id TEXT
        REFERENCES ai_transparency_change_requests(change_request_id),
    summary_id TEXT,
    severity TEXT NOT NULL,
    status TEXT NOT NULL,
    control_version INTEGER NOT NULL,
    reason_code TEXT NOT NULL,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(event_type IN ('opened', 'evidence_merged', 'acknowledged', 'resolved')),
    CHECK(severity IN ('warning', 'critical')),
    CHECK(status IN ('open', 'acknowledged', 'resolved')),
    CHECK(control_version >= 1),
    CHECK(reason_code <> '')
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_incident_audit_incident_time
ON ai_delivery_security_incident_audit_events(incident_id, occurred_at ASC);

CREATE TRIGGER trg_ai_delivery_security_incident_audit_append_only
BEFORE UPDATE OR DELETE ON ai_delivery_security_incident_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

CREATE TABLE IF NOT EXISTS ai_delivery_security_cleanup_schedules (
    schedule_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    interval_minutes INTEGER NOT NULL,
    status TEXT NOT NULL,
    next_run_at TIMESTAMPTZ NOT NULL,
    lease_owner TEXT,
    lease_expires_at TIMESTAMPTZ,
    consecutive_failures INTEGER NOT NULL DEFAULT 0,
    run_count BIGINT NOT NULL DEFAULT 0,
    last_started_at TIMESTAMPTZ,
    last_finished_at TIMESTAMPTZ,
    last_outcome TEXT,
    last_reason_code TEXT,
    last_deleted_rate_windows BIGINT NOT NULL DEFAULT 0,
    last_deleted_metric_snapshots BIGINT NOT NULL DEFAULT 0,
    created_by_snapshot_id TEXT NOT NULL
        REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    UNIQUE(tenant_id, workspace_id, environment),
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(interval_minutes BETWEEN 5 AND 1440),
    CHECK(status IN ('active', 'leased', 'paused')),
    CHECK(consecutive_failures >= 0),
    CHECK(run_count >= 0),
    CHECK(last_deleted_rate_windows >= 0),
    CHECK(last_deleted_metric_snapshots >= 0),
    CHECK(last_outcome IS NULL OR last_outcome IN ('succeeded', 'failed')),
    CHECK(
        (status = 'leased' AND lease_owner IS NOT NULL AND lease_expires_at IS NOT NULL)
        OR
        (status <> 'leased' AND lease_owner IS NULL AND lease_expires_at IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_cleanup_schedules_due
ON ai_delivery_security_cleanup_schedules(status, next_run_at, lease_expires_at);

CREATE TABLE IF NOT EXISTS ai_delivery_security_cleanup_runner_audit_events (
    runner_audit_event_id TEXT PRIMARY KEY,
    schedule_id TEXT NOT NULL
        REFERENCES ai_delivery_security_cleanup_schedules(schedule_id),
    run_id TEXT NOT NULL,
    runner_id TEXT NOT NULL,
    actor_snapshot_id TEXT NOT NULL
        REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    event_type TEXT NOT NULL,
    outcome TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    deleted_rate_windows BIGINT NOT NULL,
    deleted_metric_snapshots BIGINT NOT NULL,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(event_type IN ('schedule_created', 'schedule_updated', 'claimed', 'succeeded', 'failed')),
    CHECK(outcome IN ('scheduled', 'running', 'succeeded', 'failed')),
    CHECK(reason_code <> ''),
    CHECK(deleted_rate_windows >= 0),
    CHECK(deleted_metric_snapshots >= 0)
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_cleanup_runner_schedule_time
ON ai_delivery_security_cleanup_runner_audit_events(schedule_id, occurred_at ASC);

CREATE TRIGGER trg_ai_delivery_security_cleanup_runner_audit_append_only
BEFORE UPDATE OR DELETE ON ai_delivery_security_cleanup_runner_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

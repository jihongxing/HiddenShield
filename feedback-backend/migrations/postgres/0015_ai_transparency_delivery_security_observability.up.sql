CREATE TABLE IF NOT EXISTS ai_delivery_security_observability_snapshots (
    summary_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    mode TEXT NOT NULL,
    window_started_at TIMESTAMPTZ NOT NULL,
    window_ended_at TIMESTAMPTZ NOT NULL,
    authorization_granted_count BIGINT NOT NULL,
    authorization_revoked_count BIGINT NOT NULL,
    retrieval_claimed_count BIGINT NOT NULL,
    retrieval_succeeded_count BIGINT NOT NULL,
    retrieval_failed_count BIGINT NOT NULL,
    rate_limited_count BIGINT NOT NULL,
    revoked_access_count BIGINT NOT NULL,
    size_limit_count BIGINT NOT NULL,
    content_type_invalid_count BIGINT NOT NULL,
    read_timeout_count BIGINT NOT NULL,
    artifact_unavailable_count BIGINT NOT NULL,
    bridge_rejected_count BIGINT NOT NULL,
    alert_status TEXT NOT NULL,
    alert_codes_json JSONB NOT NULL,
    summary_digest TEXT NOT NULL,
    requested_by_snapshot_id TEXT NOT NULL
        REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    created_at TIMESTAMPTZ NOT NULL,
    retention_expires_at TIMESTAMPTZ NOT NULL,
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(mode IN ('monitoring_15m', 'audit_export')),
    CHECK(window_ended_at > window_started_at),
    CHECK(alert_status IN ('ok', 'warning', 'critical', 'not_evaluated')),
    CHECK(summary_digest ~ '^[a-f0-9]{64}$'),
    CHECK(retention_expires_at = created_at + INTERVAL '90 days')
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_summary_scope_time
ON ai_delivery_security_observability_snapshots(
    tenant_id, workspace_id, environment, window_ended_at DESC
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_summary_retention
ON ai_delivery_security_observability_snapshots(retention_expires_at);

CREATE OR REPLACE FUNCTION guard_ai_delivery_security_summary_mutation()
RETURNS trigger AS $$
BEGIN
    IF TG_OP = 'UPDATE' THEN
        RAISE EXCEPTION 'delivery security summary snapshots are immutable';
    END IF;
    IF TG_OP = 'DELETE' AND OLD.retention_expires_at > NOW() THEN
        RAISE EXCEPTION 'delivery security summary retention has not expired';
    END IF;
    RETURN OLD;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_ai_delivery_security_summary_guard
ON ai_delivery_security_observability_snapshots;

CREATE TRIGGER trg_ai_delivery_security_summary_guard
BEFORE UPDATE OR DELETE ON ai_delivery_security_observability_snapshots
FOR EACH ROW EXECUTE FUNCTION guard_ai_delivery_security_summary_mutation();

CREATE TABLE IF NOT EXISTS ai_delivery_security_operations_audit_events (
    operation_audit_event_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    operation TEXT NOT NULL,
    outcome TEXT NOT NULL,
    actor_snapshot_id TEXT NOT NULL
        REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    affected_rate_windows INTEGER NOT NULL,
    affected_metric_snapshots INTEGER NOT NULL,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(operation IN (
        'delivery_security_summary_generated',
        'delivery_security_audit_summary_exported',
        'delivery_rate_limit_cleanup'
    )),
    CHECK(outcome IN ('succeeded', 'denied', 'failed')),
    CHECK(affected_rate_windows >= 0),
    CHECK(affected_metric_snapshots >= 0)
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_operations_scope_time
ON ai_delivery_security_operations_audit_events(
    tenant_id, workspace_id, environment, occurred_at DESC
);

DROP TRIGGER IF EXISTS trg_ai_delivery_security_operations_audit_append_only
ON ai_delivery_security_operations_audit_events;

CREATE TRIGGER trg_ai_delivery_security_operations_audit_append_only
BEFORE UPDATE OR DELETE ON ai_delivery_security_operations_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

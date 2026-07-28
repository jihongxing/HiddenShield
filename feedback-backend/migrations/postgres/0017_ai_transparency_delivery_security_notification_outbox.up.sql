CREATE TABLE IF NOT EXISTS ai_delivery_security_incident_inspection_audit_events (
    inspection_audit_event_id TEXT PRIMARY KEY,
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    incident_id TEXT,
    operation TEXT NOT NULL,
    actor_snapshot_id TEXT NOT NULL
        REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    outcome TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    returned_count INTEGER NOT NULL,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(operation IN (
        'inspect_delivery_security_incident',
        'list_delivery_security_incidents'
    )),
    CHECK(outcome IN ('succeeded', 'denied', 'not_found')),
    CHECK(reason_code <> ''),
    CHECK(returned_count >= 0)
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_incident_inspection_scope_time
ON ai_delivery_security_incident_inspection_audit_events(
    tenant_id, workspace_id, environment, occurred_at DESC
);

CREATE TRIGGER trg_ai_delivery_security_incident_inspection_append_only
BEFORE UPDATE OR DELETE ON ai_delivery_security_incident_inspection_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

CREATE TABLE IF NOT EXISTS ai_delivery_security_notification_outbox (
    notification_id TEXT PRIMARY KEY,
    incident_id TEXT NOT NULL
        REFERENCES ai_delivery_security_incidents(incident_id),
    tenant_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    environment TEXT NOT NULL,
    event_type TEXT NOT NULL,
    priority TEXT NOT NULL,
    dedupe_key TEXT NOT NULL UNIQUE,
    payload_json JSONB NOT NULL,
    payload_digest TEXT NOT NULL,
    status TEXT NOT NULL,
    available_at TIMESTAMPTZ NOT NULL,
    lease_owner TEXT,
    lease_expires_at TIMESTAMPTZ,
    delivery_attempt_count INTEGER NOT NULL DEFAULT 0,
    replay_count INTEGER NOT NULL DEFAULT 0,
    last_replay_idempotency_key TEXT,
    last_reason_code TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL,
    CHECK(environment IN ('sandbox', 'production')),
    CHECK(event_type IN (
        'incident_opened',
        'incident_became_critical',
        'incident_acknowledged',
        'incident_resolved'
    )),
    CHECK(priority IN ('info', 'warning', 'critical')),
    CHECK(payload_digest ~ '^[a-f0-9]{64}$'),
    CHECK(status IN ('pending', 'leased', 'retry_scheduled')),
    CHECK(delivery_attempt_count >= 0),
    CHECK(replay_count >= 0),
    CHECK(last_reason_code <> ''),
    CHECK(
        (status = 'leased' AND lease_owner IS NOT NULL AND lease_expires_at IS NOT NULL)
        OR
        (status <> 'leased' AND lease_owner IS NULL AND lease_expires_at IS NULL)
    )
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_notification_outbox_due
ON ai_delivery_security_notification_outbox(
    tenant_id, workspace_id, environment, status, available_at, lease_expires_at
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_notification_outbox_incident
ON ai_delivery_security_notification_outbox(incident_id, created_at ASC);

CREATE TABLE IF NOT EXISTS ai_delivery_security_notification_outbox_audit_events (
    outbox_audit_event_id TEXT PRIMARY KEY,
    notification_id TEXT NOT NULL
        REFERENCES ai_delivery_security_notification_outbox(notification_id),
    incident_id TEXT NOT NULL
        REFERENCES ai_delivery_security_incidents(incident_id),
    event_type TEXT NOT NULL,
    actor_snapshot_id TEXT NOT NULL
        REFERENCES ai_transparency_actor_role_snapshots(actor_role_snapshot_id),
    runner_id TEXT,
    outcome TEXT NOT NULL,
    reason_code TEXT NOT NULL,
    delivery_attempt_count INTEGER NOT NULL,
    replay_count INTEGER NOT NULL,
    details_json JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    CHECK(event_type IN (
        'enqueued',
        'dedupe_replay',
        'claimed',
        'expired_lease_reclaimed',
        'replay_scheduled',
        'replay_idempotency_replay'
    )),
    CHECK(outcome IN ('pending', 'leased', 'retry_scheduled')),
    CHECK(reason_code <> ''),
    CHECK(delivery_attempt_count >= 0),
    CHECK(replay_count >= 0)
);

CREATE INDEX IF NOT EXISTS idx_ai_delivery_security_notification_outbox_audit_item_time
ON ai_delivery_security_notification_outbox_audit_events(notification_id, occurred_at ASC);

CREATE TRIGGER trg_ai_delivery_security_notification_outbox_audit_append_only
BEFORE UPDATE OR DELETE ON ai_delivery_security_notification_outbox_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

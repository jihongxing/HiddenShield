ALTER TABLE ai_delivery_security_notification_outbox
    ADD COLUMN destination_policy_id TEXT,
    ADD COLUMN destination_policy_version INTEGER,
    ADD COLUMN destination_policy_digest TEXT,
    ADD COLUMN destination_policy_json JSONB,
    ADD COLUMN adapter_kind TEXT,
    ADD COLUMN max_delivery_attempts INTEGER NOT NULL DEFAULT 5,
    ADD COLUMN recovery_count INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN completion_idempotency_key TEXT,
    ADD COLUMN last_failure_idempotency_key TEXT,
    ADD COLUMN last_recovery_idempotency_key TEXT,
    ADD COLUMN provider_receipt_id TEXT,
    ADD COLUMN provider_receipt_digest TEXT,
    ADD COLUMN completed_at TIMESTAMPTZ,
    ADD COLUMN dead_lettered_at TIMESTAMPTZ,
    ADD COLUMN last_failure_code TEXT;

ALTER TABLE ai_delivery_security_notification_outbox
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_status_check;

ALTER TABLE ai_delivery_security_notification_outbox
    ADD CONSTRAINT ai_delivery_security_notification_outbox_status_check
    CHECK(status IN ('pending', 'leased', 'retry_scheduled', 'completed', 'dead_letter')),
    ADD CONSTRAINT ai_delivery_security_notification_outbox_destination_policy_check
    CHECK(
        (
            destination_policy_id IS NULL
            AND destination_policy_version IS NULL
            AND destination_policy_digest IS NULL
            AND destination_policy_json IS NULL
            AND adapter_kind IS NULL
        )
        OR
        (
            destination_policy_id IS NOT NULL
            AND destination_policy_version >= 1
            AND destination_policy_digest ~ '^[a-f0-9]{64}$'
            AND destination_policy_json IS NOT NULL
            AND adapter_kind IN ('zero_send', 'pagerduty', 'email', 'sms')
        )
    ),
    ADD CONSTRAINT ai_delivery_security_notification_outbox_attempt_budget_check
    CHECK(max_delivery_attempts BETWEEN 1 AND 20),
    ADD CONSTRAINT ai_delivery_security_notification_outbox_recovery_count_check
    CHECK(recovery_count >= 0),
    ADD CONSTRAINT ai_delivery_security_notification_outbox_receipt_digest_check
    CHECK(provider_receipt_digest IS NULL OR provider_receipt_digest ~ '^[a-f0-9]{64}$'),
    ADD CONSTRAINT ai_delivery_security_notification_outbox_terminal_state_check
    CHECK(
        (
            status = 'completed'
            AND completed_at IS NOT NULL
            AND dead_lettered_at IS NULL
            AND completion_idempotency_key IS NOT NULL
            AND provider_receipt_id IS NOT NULL
            AND provider_receipt_digest IS NOT NULL
            AND destination_policy_digest IS NOT NULL
        )
        OR
        (
            status = 'dead_letter'
            AND completed_at IS NULL
            AND dead_lettered_at IS NOT NULL
            AND provider_receipt_id IS NULL
            AND provider_receipt_digest IS NULL
            AND destination_policy_digest IS NOT NULL
        )
        OR
        (
            status IN ('pending', 'leased', 'retry_scheduled')
            AND completed_at IS NULL
            AND dead_lettered_at IS NULL
            AND provider_receipt_id IS NULL
            AND provider_receipt_digest IS NULL
        )
    );

CREATE INDEX idx_ai_delivery_security_notification_completion_idempotency
ON ai_delivery_security_notification_outbox(completion_idempotency_key)
WHERE completion_idempotency_key IS NOT NULL;

CREATE INDEX idx_ai_delivery_security_notification_dead_letter
ON ai_delivery_security_notification_outbox(
    tenant_id, workspace_id, environment, dead_lettered_at
)
WHERE status = 'dead_letter';

CREATE TABLE ai_delivery_security_notification_provider_receipts (
    provider_receipt_record_id TEXT PRIMARY KEY,
    notification_id TEXT NOT NULL
        REFERENCES ai_delivery_security_notification_outbox(notification_id),
    delivery_attempt_count INTEGER NOT NULL,
    adapter_kind TEXT NOT NULL,
    adapter_invocation_key TEXT NOT NULL UNIQUE,
    destination_policy_digest TEXT NOT NULL,
    payload_digest TEXT NOT NULL,
    provider_receipt_id TEXT NOT NULL,
    provider_outcome TEXT NOT NULL,
    delivery_claimed BOOLEAN NOT NULL,
    receipt_json JSONB NOT NULL,
    receipt_digest TEXT NOT NULL,
    accepted_at TIMESTAMPTZ NOT NULL,
    CHECK(delivery_attempt_count >= 1),
    CHECK(adapter_kind IN ('zero_send', 'pagerduty', 'email', 'sms')),
    CHECK(destination_policy_digest ~ '^[a-f0-9]{64}$'),
    CHECK(payload_digest ~ '^[a-f0-9]{64}$'),
    CHECK(provider_outcome IN ('simulated', 'delivered')),
    CHECK(receipt_digest ~ '^[a-f0-9]{64}$'),
    CHECK(
        (provider_outcome = 'simulated' AND adapter_kind = 'zero_send' AND delivery_claimed = FALSE)
        OR
        (provider_outcome = 'delivered' AND adapter_kind <> 'zero_send' AND delivery_claimed = TRUE)
    )
);

CREATE UNIQUE INDEX idx_ai_delivery_security_notification_provider_receipt_id
ON ai_delivery_security_notification_provider_receipts(adapter_kind, provider_receipt_id);

CREATE INDEX idx_ai_delivery_security_notification_provider_receipt_item
ON ai_delivery_security_notification_provider_receipts(notification_id, accepted_at);

CREATE TRIGGER trg_ai_delivery_security_notification_provider_receipt_append_only
BEFORE UPDATE OR DELETE ON ai_delivery_security_notification_provider_receipts
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

ALTER TABLE ai_delivery_security_notification_outbox_audit_events
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_audit_event_type_check,
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_audit_ev_outcome_check,
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_audit_events_event_typ,
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_audit_events_outcome_c;

ALTER TABLE ai_delivery_security_notification_outbox_audit_events
    ADD CONSTRAINT ai_delivery_notification_audit_event_type_v2_check
    CHECK(event_type IN (
        'enqueued',
        'dedupe_replay',
        'claimed',
        'expired_lease_reclaimed',
        'destination_bound',
        'completed',
        'completion_idempotency_replay',
        'delivery_failed',
        'failure_idempotency_replay',
        'dead_lettered',
        'recovery_idempotency_replay',
        'replay_scheduled',
        'replay_idempotency_replay'
    )),
    ADD CONSTRAINT ai_delivery_notification_audit_outcome_v2_check
    CHECK(outcome IN ('pending', 'leased', 'retry_scheduled', 'completed', 'dead_letter'));

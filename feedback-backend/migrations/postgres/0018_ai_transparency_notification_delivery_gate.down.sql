DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM ai_delivery_security_notification_outbox
        WHERE status IN ('completed', 'dead_letter')
    ) THEN
        RAISE EXCEPTION
            'refusing 0018 rollback while completed/dead-letter notification rows exist';
    END IF;
END
$$;

ALTER TABLE ai_delivery_security_notification_outbox_audit_events
    DROP CONSTRAINT IF EXISTS ai_delivery_notification_audit_event_type_v2_check,
    DROP CONSTRAINT IF EXISTS ai_delivery_notification_audit_outcome_v2_check;

ALTER TABLE ai_delivery_security_notification_outbox_audit_events
    ADD CONSTRAINT ai_delivery_security_notification_outbox_audit_event_type_check
    CHECK(event_type IN (
        'enqueued',
        'dedupe_replay',
        'claimed',
        'expired_lease_reclaimed',
        'replay_scheduled',
        'replay_idempotency_replay'
    )),
    ADD CONSTRAINT ai_delivery_security_notification_outbox_audit_ev_outcome_check
    CHECK(outcome IN ('pending', 'leased', 'retry_scheduled'));

DROP TRIGGER IF EXISTS trg_ai_delivery_security_notification_provider_receipt_append_only
ON ai_delivery_security_notification_provider_receipts;
DROP INDEX IF EXISTS idx_ai_delivery_security_notification_provider_receipt_item;
DROP INDEX IF EXISTS idx_ai_delivery_security_notification_provider_receipt_id;
DROP TABLE IF EXISTS ai_delivery_security_notification_provider_receipts;

DROP INDEX IF EXISTS idx_ai_delivery_security_notification_dead_letter;
DROP INDEX IF EXISTS idx_ai_delivery_security_notification_completion_idempotency;

ALTER TABLE ai_delivery_security_notification_outbox
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_terminal_state_check,
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_receipt_digest_check,
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_recovery_count_check,
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_attempt_budget_check,
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_destination_policy_check,
    DROP CONSTRAINT IF EXISTS ai_delivery_security_notification_outbox_status_check;

ALTER TABLE ai_delivery_security_notification_outbox
    ADD CONSTRAINT ai_delivery_security_notification_outbox_status_check
    CHECK(status IN ('pending', 'leased', 'retry_scheduled'));

ALTER TABLE ai_delivery_security_notification_outbox
    DROP COLUMN IF EXISTS last_failure_code,
    DROP COLUMN IF EXISTS dead_lettered_at,
    DROP COLUMN IF EXISTS completed_at,
    DROP COLUMN IF EXISTS provider_receipt_digest,
    DROP COLUMN IF EXISTS provider_receipt_id,
    DROP COLUMN IF EXISTS last_recovery_idempotency_key,
    DROP COLUMN IF EXISTS last_failure_idempotency_key,
    DROP COLUMN IF EXISTS completion_idempotency_key,
    DROP COLUMN IF EXISTS recovery_count,
    DROP COLUMN IF EXISTS max_delivery_attempts,
    DROP COLUMN IF EXISTS adapter_kind,
    DROP COLUMN IF EXISTS destination_policy_json,
    DROP COLUMN IF EXISTS destination_policy_digest,
    DROP COLUMN IF EXISTS destination_policy_version,
    DROP COLUMN IF EXISTS destination_policy_id;

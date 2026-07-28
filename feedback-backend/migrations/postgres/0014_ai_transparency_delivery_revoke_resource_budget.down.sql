DROP INDEX IF EXISTS idx_ai_delivery_download_rate_limit_updated;
DROP TABLE IF EXISTS ai_delivery_download_rate_limit_windows;

DROP TRIGGER IF EXISTS trg_ai_delivery_download_audit_append_only
ON ai_delivery_download_audit_events;

DELETE FROM ai_delivery_download_audit_events
WHERE event_type = 'authorization_revoked';

ALTER TABLE ai_delivery_download_audit_events
DROP CONSTRAINT IF EXISTS ai_delivery_download_audit_events_event_type_check;

ALTER TABLE ai_delivery_download_audit_events
ADD CONSTRAINT ai_delivery_download_audit_events_event_type_check
CHECK(event_type IN (
    'authorization_granted',
    'retrieval_claimed',
    'retrieval_succeeded',
    'retrieval_failed'
));

CREATE TRIGGER trg_ai_delivery_download_audit_append_only
BEFORE UPDATE OR DELETE ON ai_delivery_download_audit_events
FOR EACH ROW EXECUTE FUNCTION reject_ai_transparency_change_audit_mutation();

UPDATE ai_delivery_retrieval_authorizations
SET status = 'expired'
WHERE status = 'revoked';

ALTER TABLE ai_delivery_retrieval_authorizations
DROP CONSTRAINT IF EXISTS ai_delivery_retrieval_authorizations_revocation_check;

ALTER TABLE ai_delivery_retrieval_authorizations
DROP CONSTRAINT IF EXISTS ai_delivery_retrieval_authorizations_resource_budget_check;

ALTER TABLE ai_delivery_retrieval_authorizations
DROP COLUMN IF EXISTS revoke_reason;

ALTER TABLE ai_delivery_retrieval_authorizations
DROP COLUMN IF EXISTS revoked_by_snapshot_id;

ALTER TABLE ai_delivery_retrieval_authorizations
DROP COLUMN IF EXISTS revoked_at;

ALTER TABLE ai_delivery_retrieval_authorizations
DROP COLUMN IF EXISTS rate_limit_per_minute;

ALTER TABLE ai_delivery_retrieval_authorizations
DROP COLUMN IF EXISTS read_timeout_ms;

ALTER TABLE ai_delivery_retrieval_authorizations
DROP COLUMN IF EXISTS required_content_type;

ALTER TABLE ai_delivery_retrieval_authorizations
DROP COLUMN IF EXISTS max_download_bytes;

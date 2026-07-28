-- HiddenShield PostgreSQL P3 rollback: AI Transparency schema contract v1.

DROP INDEX IF EXISTS idx_ai_marking_ledger_license_status;
DROP INDEX IF EXISTS idx_ai_explicit_label_receipts_manifest;
DROP INDEX IF EXISTS idx_ai_marker_bindings_manifest;
DROP INDEX IF EXISTS idx_ai_claim_evidence_manifest;
DROP INDEX IF EXISTS idx_ai_transparency_manifests_watermark_status;
DROP INDEX IF EXISTS idx_ai_transparency_manifests_one_active;
DROP INDEX IF EXISTS idx_ai_marking_sessions_license_status;
DROP INDEX IF EXISTS idx_ai_sdk_credential_bindings_license_status;
DROP INDEX IF EXISTS idx_ai_profile_entitlements_license_status;
DROP INDEX IF EXISTS idx_ai_transparency_licenses_one_active;

DROP TABLE IF EXISTS ai_transparency_admin_audit_events;
DROP TABLE IF EXISTS ai_marking_ledger;
DROP TABLE IF EXISTS ai_explicit_label_receipts;
DROP TABLE IF EXISTS ai_marker_bindings;
DROP TABLE IF EXISTS ai_claim_evidence;
DROP TABLE IF EXISTS ai_transparency_manifests;
DROP TABLE IF EXISTS ai_marking_sessions;
DROP TABLE IF EXISTS ai_sdk_credential_bindings;
DROP TABLE IF EXISTS ai_profile_entitlements;
DROP TABLE IF EXISTS ai_transparency_licenses;

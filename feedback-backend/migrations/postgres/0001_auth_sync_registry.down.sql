-- HiddenShield PostgreSQL P2 rollback: auth, cloud sync, and watermark registry.
-- Drop indexes before tables and keep schema_migrations last.

DROP INDEX IF EXISTS idx_rights_manifests_status_updated;
DROP INDEX IF EXISTS idx_rights_manifests_watermark_version;
DROP INDEX IF EXISTS idx_rights_manifests_watermark_status;
DROP INDEX IF EXISTS idx_rights_manifests_watermark;
DROP INDEX IF EXISTS idx_rights_manifests_one_active;
DROP INDEX IF EXISTS idx_watermark_id_reissue_jobs_account;
DROP INDEX IF EXISTS idx_watermark_id_registry_parent;
DROP INDEX IF EXISTS idx_watermark_id_registry_account_workspace;
DROP INDEX IF EXISTS idx_cloud_sync_events_account_sequence;
DROP INDEX IF EXISTS idx_auth_attempts_identifier_created;
DROP INDEX IF EXISTS idx_auth_challenges_identifier_created;

DROP TABLE IF EXISTS rights_manifests;
DROP TABLE IF EXISTS watermark_id_reissue_jobs;
DROP TABLE IF EXISTS watermark_id_registry;
DROP TABLE IF EXISTS cloud_device_cursors;
DROP TABLE IF EXISTS cloud_sync_events;
DROP TABLE IF EXISTS auth_attempts;
DROP TABLE IF EXISTS auth_challenges;
DROP TABLE IF EXISTS cloud_sessions;
DROP TABLE IF EXISTS cloud_devices;
DROP TABLE IF EXISTS cloud_accounts;
DROP TABLE IF EXISTS schema_migrations;

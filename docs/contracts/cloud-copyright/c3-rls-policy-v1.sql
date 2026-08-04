-- C3 contract template only. This file is not a PostgreSQL migration.
-- All runtime roles must be NOSUPERUSER NOBYPASSRLS and cannot own cloud copyright tables.

CREATE ROLE hiddenshield_cloud_copyright_app
  LOGIN NOSUPERUSER NOBYPASSRLS NOINHERIT;

CREATE ROLE hiddenshield_cloud_copyright_internal_service
  LOGIN NOSUPERUSER NOBYPASSRLS NOINHERIT;

ALTER TABLE cloud_copyright_records ENABLE ROW LEVEL SECURITY;
ALTER TABLE cloud_copyright_records FORCE ROW LEVEL SECURITY;
ALTER TABLE cloud_copyright_changes ENABLE ROW LEVEL SECURITY;
ALTER TABLE cloud_copyright_changes FORCE ROW LEVEL SECURITY;
ALTER TABLE cloud_copyright_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE cloud_copyright_events FORCE ROW LEVEL SECURITY;
ALTER TABLE cloud_copyright_audit_events ENABLE ROW LEVEL SECURITY;
ALTER TABLE cloud_copyright_audit_events FORCE ROW LEVEL SECURITY;
ALTER TABLE cloud_copyright_workspace_cursors ENABLE ROW LEVEL SECURITY;
ALTER TABLE cloud_copyright_workspace_cursors FORCE ROW LEVEL SECURITY;

CREATE POLICY cloud_copyright_records_workspace_scope
ON cloud_copyright_records
USING (
  workspace_id = current_setting('app.workspace_id', true)
  AND EXISTS (
    SELECT 1
    FROM cloud_copyright_workspace_memberships AS membership
    WHERE membership.membership_id = current_setting('app.membership_id', true)
      AND membership.account_id = current_setting('app.account_id', true)
      AND membership.workspace_id = current_setting('app.workspace_id', true)
      AND membership.status = 'active'
  )
)
WITH CHECK (
  workspace_id = current_setting('app.workspace_id', true)
  AND current_setting('app.device_id', true) <> ''
);

CREATE POLICY cloud_copyright_changes_workspace_scope
ON cloud_copyright_changes
USING (
  workspace_id = current_setting('app.workspace_id', true)
  AND device_id = current_setting('app.device_id', true)
)
WITH CHECK (
  workspace_id = current_setting('app.workspace_id', true)
  AND device_id = current_setting('app.device_id', true)
);

CREATE POLICY cloud_copyright_events_workspace_scope
ON cloud_copyright_events
USING (workspace_id = current_setting('app.workspace_id', true));

CREATE POLICY cloud_copyright_audit_events_workspace_scope
ON cloud_copyright_audit_events
USING (
  workspace_id = current_setting('app.workspace_id', true)
  AND current_setting('app.actor_kind', true) <> ''
);

CREATE POLICY cloud_copyright_workspace_cursors_workspace_scope
ON cloud_copyright_workspace_cursors
USING (
  workspace_id = current_setting('app.workspace_id', true)
  AND device_id = current_setting('app.device_id', true)
)
WITH CHECK (
  workspace_id = current_setting('app.workspace_id', true)
  AND device_id = current_setting('app.device_id', true)
);

-- Runtime adapter requirement: SELECT set_config($1, $2, true) for every app.* key.
-- No PUBLIC grants, SET ROLE, global GUC scope, table-owner runtime, or bypass role is permitted.

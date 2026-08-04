#[cfg(feature = "postgres")]
use std::sync::Arc;

#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::{
    cloud_copyright::{
        CloudCopyrightActor, CloudCopyrightChangeCommand, CloudCopyrightDisposition,
        CloudCopyrightError, CloudCopyrightOperation, CloudCopyrightRepository,
        RevokeWorkspaceMembershipCommand,
    },
    database::{
        POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL,
        POSTGRES_P24_CLOUD_COPYRIGHT_MULTITENANT_CORE_DOWN_SQL,
        POSTGRES_P24_CLOUD_COPYRIGHT_MULTITENANT_CORE_UP_SQL,
    },
};
#[cfg(feature = "postgres")]
use serde::Serialize;
#[cfg(feature = "postgres")]
use serde_json::json;
#[cfg(feature = "postgres")]
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
#[cfg(feature = "postgres")]
use tokio::sync::Barrier;

#[cfg(feature = "postgres")]
const WORKSPACE_ID: &str = "ws_cloud_copyright_qa";
#[cfg(feature = "postgres")]
const RECORD_ID: &str = "ccr_cloud_copyright_qa";
#[cfg(feature = "postgres")]
const OWNER_ACCOUNT_ID: &str = "acct_cloud_copyright_owner";
#[cfg(feature = "postgres")]
const OWNER_DEVICE_ID: &str = "device_cloud_copyright_owner";
#[cfg(feature = "postgres")]
const EDITOR_ACCOUNT_ID: &str = "acct_cloud_copyright_editor";
#[cfg(feature = "postgres")]
const EDITOR_DEVICE_ID: &str = "device_cloud_copyright_editor";
#[cfg(feature = "postgres")]
const VIEWER_ACCOUNT_ID: &str = "acct_cloud_copyright_viewer";
#[cfg(feature = "postgres")]
const VIEWER_DEVICE_ID: &str = "device_cloud_copyright_viewer";
#[cfg(feature = "postgres")]
const OUTSIDER_ACCOUNT_ID: &str = "acct_cloud_copyright_outsider";
#[cfg(feature = "postgres")]
const OUTSIDER_DEVICE_ID: &str = "device_cloud_copyright_outsider";

#[cfg(feature = "postgres")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScenarioSnapshot {
    scenario_id: &'static str,
    result_a: String,
    result_b: String,
    record_version: i64,
    deleted: bool,
    change_count: i64,
    event_count: i64,
    audit_count: i64,
    cursor_count: i64,
    membership_status: String,
}

#[cfg(feature = "postgres")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| "missing HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or DATABASE_URL")?;
    if !safe_disposable_url(&database_url) {
        return Err(
            "cloud_copyright_postgres_concurrency_qa requires localhost/127.0.0.1 and hiddenshield_migrate_smoke in its PostgreSQL URL"
                .into(),
        );
    }

    let admin_pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url)
        .await?;
    let repository_a = Arc::new(CloudCopyrightRepository::new(
        PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await?,
    ));
    let repository_b = Arc::new(CloudCopyrightRepository::new(
        PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await?,
    ));

    let snapshots = vec![
        duplicate_idempotency(&admin_pool, &repository_a, &repository_b).await?,
        changed_duplicate(&admin_pool, &repository_a, &repository_b).await?,
        stale_version(&admin_pool, &repository_a, &repository_b).await?,
        revoke_vs_push(&admin_pool, &repository_a, &repository_b).await?,
        workspace_isolation(&admin_pool, &repository_a).await?,
        role_boundary(&admin_pool, &repository_a, &repository_b).await?,
        audit_failure_rollback(&admin_pool, &repository_a).await?,
        delete_vs_update(&admin_pool, &repository_a, &repository_b).await?,
    ];

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "databaseKind": "postgres",
            "harness": "cloud_copyright_postgres_concurrency_qa_v1",
            "connections": 2,
            "scenarios": snapshots
        })
    );
    Ok(())
}

#[cfg(feature = "postgres")]
async fn duplicate_idempotency(
    pool: &PgPool,
    repository_a: &Arc<CloudCopyrightRepository>,
    repository_b: &Arc<CloudCopyrightRepository>,
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    reset_and_seed(pool).await?;
    let barrier = Arc::new(Barrier::new(2));
    let command = change(
        "change-duplicate-a",
        "idempotency-duplicate",
        "digest-duplicate",
        6,
        CloudCopyrightOperation::UpsertRecord,
    );
    let a = run_change_with_barrier(
        Arc::clone(repository_a),
        owner_actor(),
        command.clone(),
        Arc::clone(&barrier),
    );
    let b = run_change_with_barrier(Arc::clone(repository_b), owner_actor(), command, barrier);
    let (result_a, result_b) = tokio::join!(a, b);
    assert_one_accepted_one_duplicate(&result_a, &result_b)?;
    snapshot(
        pool,
        "duplicate_idempotency",
        change_result_name(&result_a),
        change_result_name(&result_b),
    )
    .await
}

#[cfg(feature = "postgres")]
async fn changed_duplicate(
    pool: &PgPool,
    repository_a: &Arc<CloudCopyrightRepository>,
    repository_b: &Arc<CloudCopyrightRepository>,
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    reset_and_seed(pool).await?;
    let barrier = Arc::new(Barrier::new(2));
    let a = run_change_with_barrier(
        Arc::clone(repository_a),
        owner_actor(),
        change(
            "change-changed-a",
            "idempotency-changed",
            "digest-changed-a",
            6,
            CloudCopyrightOperation::UpsertRecord,
        ),
        Arc::clone(&barrier),
    );
    let b = run_change_with_barrier(
        Arc::clone(repository_b),
        owner_actor(),
        change(
            "change-changed-b",
            "idempotency-changed",
            "digest-changed-b",
            6,
            CloudCopyrightOperation::UpsertRecord,
        ),
        barrier,
    );
    let (result_a, result_b) = tokio::join!(a, b);
    assert_one_accepted_one_error(&result_a, &result_b, "conflict_payload_changed")?;
    snapshot(
        pool,
        "changed_duplicate",
        change_result_name(&result_a),
        change_result_name(&result_b),
    )
    .await
}

#[cfg(feature = "postgres")]
async fn stale_version(
    pool: &PgPool,
    repository_a: &Arc<CloudCopyrightRepository>,
    repository_b: &Arc<CloudCopyrightRepository>,
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    reset_and_seed(pool).await?;
    let barrier = Arc::new(Barrier::new(2));
    let a = run_change_with_barrier(
        Arc::clone(repository_a),
        owner_actor(),
        change(
            "change-stale-owner",
            "idempotency-stale-owner",
            "digest-stale-owner",
            6,
            CloudCopyrightOperation::UpsertRecord,
        ),
        Arc::clone(&barrier),
    );
    let b = run_change_with_barrier(
        Arc::clone(repository_b),
        editor_actor(),
        change(
            "change-stale-editor",
            "idempotency-stale-editor",
            "digest-stale-editor",
            6,
            CloudCopyrightOperation::UpsertRecord,
        ),
        barrier,
    );
    let (result_a, result_b) = tokio::join!(a, b);
    assert_one_accepted_one_error(&result_a, &result_b, "conflict_version_changed")?;
    snapshot(
        pool,
        "stale_version",
        change_result_name(&result_a),
        change_result_name(&result_b),
    )
    .await
}

#[cfg(feature = "postgres")]
async fn revoke_vs_push(
    pool: &PgPool,
    repository_a: &Arc<CloudCopyrightRepository>,
    repository_b: &Arc<CloudCopyrightRepository>,
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    reset_and_seed(pool).await?;
    let barrier = Arc::new(Barrier::new(2));
    let revoke_repository = Arc::clone(repository_a);
    let revoke_barrier = Arc::clone(&barrier);
    let revoke = async move {
        revoke_barrier.wait().await;
        revoke_repository
            .revoke_membership(
                &owner_actor(),
                &RevokeWorkspaceMembershipCommand {
                    workspace_id: WORKSPACE_ID.to_string(),
                    target_account_id: EDITOR_ACCOUNT_ID.to_string(),
                    request_digest: "digest-revoke-editor".to_string(),
                },
            )
            .await
    };
    let push = run_change_with_barrier(
        Arc::clone(repository_b),
        editor_actor(),
        change(
            "change-revoke-push",
            "idempotency-revoke-push",
            "digest-revoke-push",
            6,
            CloudCopyrightOperation::UpsertRecord,
        ),
        barrier,
    );
    let (revoke_result, push_result) = tokio::join!(revoke, push);
    if let Err(error) = &revoke_result {
        return Err(format!("revoke-vs-push revoke unexpectedly failed: {error:?}").into());
    }
    let replay = repository_b
        .execute_change(
            &editor_actor(),
            &change(
                "change-revoke-replay",
                "idempotency-revoke-replay",
                "digest-revoke-replay",
                6,
                CloudCopyrightOperation::UpsertRecord,
            ),
        )
        .await;
    if !matches!(replay, Err(CloudCopyrightError::MembershipRevoked)) {
        return Err("revoke-vs-push replay must fail closed after revocation".into());
    }
    if !matches!(
        push_result,
        Ok(CloudCopyrightDisposition::Accepted { .. })
            | Err(CloudCopyrightError::MembershipRevoked)
    ) {
        return Err(format!(
            "unexpected revoke-vs-push result: {}",
            change_result_name(&push_result)
        )
        .into());
    }
    snapshot(
        pool,
        "revoke_vs_push",
        generic_result_name(&revoke_result),
        change_result_name(&push_result),
    )
    .await
}

#[cfg(feature = "postgres")]
async fn workspace_isolation(
    pool: &PgPool,
    repository: &Arc<CloudCopyrightRepository>,
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    reset_and_seed(pool).await?;
    let read = repository
        .get_record(&outsider_actor(), WORKSPACE_ID, RECORD_ID)
        .await;
    let write = repository
        .execute_change(
            &outsider_actor(),
            &change(
                "change-isolation",
                "idempotency-isolation",
                "digest-isolation",
                6,
                CloudCopyrightOperation::UpsertRecord,
            ),
        )
        .await;
    if !matches!(read, Err(CloudCopyrightError::Forbidden))
        || !matches!(write, Err(CloudCopyrightError::Forbidden))
    {
        return Err("workspace isolation must reject outsider read and write".into());
    }
    snapshot(
        pool,
        "workspace_isolation",
        generic_result_name(&read),
        change_result_name(&write),
    )
    .await
}

#[cfg(feature = "postgres")]
async fn role_boundary(
    pool: &PgPool,
    repository_a: &Arc<CloudCopyrightRepository>,
    repository_b: &Arc<CloudCopyrightRepository>,
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    reset_and_seed(pool).await?;
    let viewer_write = repository_a
        .execute_change(
            &viewer_actor(),
            &change(
                "change-viewer",
                "idempotency-viewer",
                "digest-viewer",
                6,
                CloudCopyrightOperation::UpsertRecord,
            ),
        )
        .await;
    let editor_revoke = repository_b
        .revoke_membership(
            &editor_actor(),
            &RevokeWorkspaceMembershipCommand {
                workspace_id: WORKSPACE_ID.to_string(),
                target_account_id: VIEWER_ACCOUNT_ID.to_string(),
                request_digest: "digest-editor-revoke".to_string(),
            },
        )
        .await;
    if !matches!(viewer_write, Err(CloudCopyrightError::RoleDenied))
        || !matches!(editor_revoke, Err(CloudCopyrightError::RoleDenied))
    {
        return Err("role boundary must reject viewer write and editor revoke".into());
    }
    snapshot(
        pool,
        "role_boundary",
        change_result_name(&viewer_write),
        generic_result_name(&editor_revoke),
    )
    .await
}

#[cfg(feature = "postgres")]
async fn audit_failure_rollback(
    pool: &PgPool,
    repository: &Arc<CloudCopyrightRepository>,
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    reset_and_seed(pool).await?;
    sqlx::raw_sql(
        "CREATE OR REPLACE FUNCTION cloud_copyright_fail_audit_insert()
         RETURNS trigger LANGUAGE plpgsql AS $$
         BEGIN
             RAISE EXCEPTION 'injected cloud copyright audit failure';
         END;
         $$;
         CREATE TRIGGER cloud_copyright_fail_audit_insert_trigger
         BEFORE INSERT ON cloud_copyright_audit_events
         FOR EACH ROW EXECUTE FUNCTION cloud_copyright_fail_audit_insert();",
    )
    .execute(pool)
    .await?;
    let result = repository
        .execute_change(
            &owner_actor(),
            &change(
                "change-audit-failure",
                "idempotency-audit-failure",
                "digest-audit-failure",
                6,
                CloudCopyrightOperation::UpsertRecord,
            ),
        )
        .await;
    sqlx::raw_sql(
        "DROP TRIGGER IF EXISTS cloud_copyright_fail_audit_insert_trigger
         ON cloud_copyright_audit_events;
         DROP FUNCTION IF EXISTS cloud_copyright_fail_audit_insert();",
    )
    .execute(pool)
    .await?;
    if !matches!(result, Err(CloudCopyrightError::Database(_))) {
        return Err("audit failure must fail transaction".into());
    }
    let snapshot = snapshot(
        pool,
        "audit_failure_rollback",
        change_result_name(&result),
        "not_applicable".to_string(),
    )
    .await?;
    if snapshot.record_version != 6
        || snapshot.change_count != 0
        || snapshot.event_count != 0
        || snapshot.audit_count != 0
        || snapshot.cursor_count != 0
    {
        return Err(format!("audit failure leaked partial writes: {snapshot:?}").into());
    }
    Ok(snapshot)
}

#[cfg(feature = "postgres")]
async fn delete_vs_update(
    pool: &PgPool,
    repository_a: &Arc<CloudCopyrightRepository>,
    repository_b: &Arc<CloudCopyrightRepository>,
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    reset_and_seed(pool).await?;
    let barrier = Arc::new(Barrier::new(2));
    let delete = run_change_with_barrier(
        Arc::clone(repository_a),
        owner_actor(),
        change(
            "change-delete",
            "idempotency-delete",
            "digest-delete",
            6,
            CloudCopyrightOperation::TombstoneRecord,
        ),
        Arc::clone(&barrier),
    );
    let update = run_change_with_barrier(
        Arc::clone(repository_b),
        editor_actor(),
        change(
            "change-update",
            "idempotency-update",
            "digest-update",
            6,
            CloudCopyrightOperation::UpsertRecord,
        ),
        barrier,
    );
    let (delete_result, update_result) = tokio::join!(delete, update);
    assert_one_accepted_one_error(&delete_result, &update_result, "conflict_version_changed")?;
    let snapshot = snapshot(
        pool,
        "delete_vs_update",
        change_result_name(&delete_result),
        change_result_name(&update_result),
    )
    .await?;
    if snapshot.record_version != 7 || snapshot.event_count != 1 || snapshot.audit_count != 1 {
        return Err(format!("delete-vs-update produced invalid projection: {snapshot:?}").into());
    }
    Ok(snapshot)
}

#[cfg(feature = "postgres")]
async fn run_change_with_barrier(
    repository: Arc<CloudCopyrightRepository>,
    actor: CloudCopyrightActor,
    command: CloudCopyrightChangeCommand,
    barrier: Arc<Barrier>,
) -> Result<CloudCopyrightDisposition, CloudCopyrightError> {
    barrier.wait().await;
    repository.execute_change(&actor, &command).await
}

#[cfg(feature = "postgres")]
async fn reset_and_seed(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::raw_sql(POSTGRES_P24_CLOUD_COPYRIGHT_MULTITENANT_CORE_DOWN_SQL)
        .execute(pool)
        .await?;
    sqlx::raw_sql(POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL)
        .execute(pool)
        .await?;
    sqlx::raw_sql(POSTGRES_P24_CLOUD_COPYRIGHT_MULTITENANT_CORE_UP_SQL)
        .execute(pool)
        .await?;
    for (account_id, device_id) in [
        (OWNER_ACCOUNT_ID, OWNER_DEVICE_ID),
        (EDITOR_ACCOUNT_ID, EDITOR_DEVICE_ID),
        (VIEWER_ACCOUNT_ID, VIEWER_DEVICE_ID),
        (OUTSIDER_ACCOUNT_ID, OUTSIDER_DEVICE_ID),
    ] {
        seed_account(pool, account_id, device_id).await?;
    }
    sqlx::query(
        "INSERT INTO cloud_copyright_workspaces (
            workspace_id, owner_account_id, workspace_type, status,
            membership_version, created_at, updated_at
         ) VALUES ($1,$2,'team','active',1,NOW(),NOW())",
    )
    .bind(WORKSPACE_ID)
    .bind(OWNER_ACCOUNT_ID)
    .execute(pool)
    .await?;
    // Membership rows require the workspace foreign key, so insert them after workspace creation.
    for (account_id, role) in [
        (OWNER_ACCOUNT_ID, "owner"),
        (EDITOR_ACCOUNT_ID, "editor"),
        (VIEWER_ACCOUNT_ID, "viewer"),
    ] {
        sqlx::query(
            "INSERT INTO cloud_copyright_workspace_memberships (
                membership_id, workspace_id, account_id, role, status,
                membership_version, invited_by_account_id, joined_at,
                created_at, updated_at
             ) VALUES ($1,$2,$3,$4,'active',1,$5,NOW(),NOW(),NOW())",
        )
        .bind(format!("membership_{account_id}"))
        .bind(WORKSPACE_ID)
        .bind(account_id)
        .bind(role)
        .bind(OWNER_ACCOUNT_ID)
        .execute(pool)
        .await?;
    }
    sqlx::query(
        "INSERT INTO cloud_copyright_creator_profiles (
            creator_profile_id, account_id, display_name, seed_envelope_ref,
            seed_envelope_version, status, created_at, updated_at
         ) VALUES ('creator_cloud_copyright_qa',$1,'Owner','seed-envelope://qa',1,'active',NOW(),NOW())",
    )
    .bind(OWNER_ACCOUNT_ID)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO cloud_copyright_records (
            record_id, workspace_id, owner_account_id, creator_profile_id,
            origin_device_id, record_kind, watermark_uid, watermark_revision,
            parent_watermark_uid, original_hash, protected_copy_hash,
            evidence_digest, write_verification_status, rights_declaration_json,
            classification, visibility, record_version, etag, created_at, updated_at, deleted_at
         ) VALUES ($1,$2,$3,'creator_cloud_copyright_qa',$4,'image',
                   'HS-01234567-89ABCDEF-01234567-89ABCDEF',1,NULL,
                   'sha256:original','sha256:protected','sha256:evidence','verified',
                   $5,'private_metadata','workspace_members',6,'sha256:version-6',NOW(),NOW(),NULL)",
    )
    .bind(RECORD_ID)
    .bind(WORKSPACE_ID)
    .bind(OWNER_ACCOUNT_ID)
    .bind(OWNER_DEVICE_ID)
    .bind(json!({"workSource":"human_created","trainingPermission":"prohibited"}))
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn seed_account(
    pool: &PgPool,
    account_id: &str,
    device_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query(
        "INSERT INTO cloud_accounts (
            id, identifier, password_hash, password_salt, password_hash_algorithm,
            display_name, workspace_id, workspace_name, creator_profile_id,
            creator_display_name, creator_seed_ref, seed_envelope_version,
            entitlement_id, entitlement_plan_name, entitlement_plan_code,
            entitlement_status, entitlement_features_json, created_at, updated_at
         ) VALUES ($1,$2,'hash','salt','argon2id',$3,'legacy-ws','Legacy',
                   'legacy-creator','Legacy','seed-envelope://legacy',1,
                   'entitlement','Creator','creator','active',$4,NOW(),NOW())
         ON CONFLICT(id) DO NOTHING",
    )
    .bind(account_id)
    .bind(format!("{account_id}@example.test"))
    .bind(account_id)
    .bind(json!({"cloud_sync": true}))
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO cloud_devices (
            id, account_id, client_device_id, name, platform, app_version,
            public_key, registered, auto_sync_enabled, created_at, updated_at
         ) VALUES ($1,$2,$3,'QA device','qa','1.0',NULL,TRUE,TRUE,NOW(),NOW())
         ON CONFLICT(id) DO NOTHING",
    )
    .bind(device_id)
    .bind(account_id)
    .bind(format!("client-{device_id}"))
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
fn owner_actor() -> CloudCopyrightActor {
    CloudCopyrightActor {
        account_id: OWNER_ACCOUNT_ID.to_string(),
        device_id: OWNER_DEVICE_ID.to_string(),
    }
}

#[cfg(feature = "postgres")]
fn editor_actor() -> CloudCopyrightActor {
    CloudCopyrightActor {
        account_id: EDITOR_ACCOUNT_ID.to_string(),
        device_id: EDITOR_DEVICE_ID.to_string(),
    }
}

#[cfg(feature = "postgres")]
fn viewer_actor() -> CloudCopyrightActor {
    CloudCopyrightActor {
        account_id: VIEWER_ACCOUNT_ID.to_string(),
        device_id: VIEWER_DEVICE_ID.to_string(),
    }
}

#[cfg(feature = "postgres")]
fn outsider_actor() -> CloudCopyrightActor {
    CloudCopyrightActor {
        account_id: OUTSIDER_ACCOUNT_ID.to_string(),
        device_id: OUTSIDER_DEVICE_ID.to_string(),
    }
}

#[cfg(feature = "postgres")]
fn change(
    change_id: &str,
    idempotency_key: &str,
    request_digest: &str,
    base_record_version: i64,
    operation: CloudCopyrightOperation,
) -> CloudCopyrightChangeCommand {
    CloudCopyrightChangeCommand {
        change_id: change_id.to_string(),
        workspace_id: WORKSPACE_ID.to_string(),
        record_id: RECORD_ID.to_string(),
        idempotency_key: idempotency_key.to_string(),
        request_digest: request_digest.to_string(),
        base_record_version,
        operation,
        rights_declaration: json!({
            "workSource": "human_created",
            "trainingPermission": "prohibited"
        }),
    }
}

#[cfg(feature = "postgres")]
async fn snapshot(
    pool: &PgPool,
    scenario_id: &'static str,
    result_a: String,
    result_b: String,
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    let record = sqlx::query(
        "SELECT record_version, deleted_at
         FROM cloud_copyright_records
         WHERE workspace_id = $1 AND record_id = $2",
    )
    .bind(WORKSPACE_ID)
    .bind(RECORD_ID)
    .fetch_one(pool)
    .await?;
    let membership_status: String = sqlx::query_scalar(
        "SELECT status
         FROM cloud_copyright_workspace_memberships
         WHERE workspace_id = $1 AND account_id = $2",
    )
    .bind(WORKSPACE_ID)
    .bind(EDITOR_ACCOUNT_ID)
    .fetch_one(pool)
    .await?;
    Ok(ScenarioSnapshot {
        scenario_id,
        result_a,
        result_b,
        record_version: record.try_get("record_version")?,
        deleted: record
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("deleted_at")?
            .is_some(),
        change_count: count(pool, "cloud_copyright_changes").await?,
        event_count: count(pool, "cloud_copyright_events").await?,
        audit_count: count(pool, "cloud_copyright_audit_events").await?,
        cursor_count: count(pool, "cloud_copyright_workspace_cursors").await?,
        membership_status,
    })
}

#[cfg(feature = "postgres")]
async fn count(pool: &PgPool, table: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
}

#[cfg(feature = "postgres")]
fn change_result_name(result: &Result<CloudCopyrightDisposition, CloudCopyrightError>) -> String {
    match result {
        Ok(CloudCopyrightDisposition::Accepted { .. }) => "accepted".to_string(),
        Ok(CloudCopyrightDisposition::Duplicate { .. }) => "duplicate".to_string(),
        Err(error) => error_result_name(error),
    }
}

#[cfg(feature = "postgres")]
fn generic_result_name<T>(result: &Result<T, CloudCopyrightError>) -> String {
    match result {
        Ok(_) => "accepted".to_string(),
        Err(error) => error_result_name(error),
    }
}

#[cfg(feature = "postgres")]
fn error_result_name(error: &CloudCopyrightError) -> String {
    match error {
        CloudCopyrightError::ConflictPayloadChanged => "conflict_payload_changed".to_string(),
        CloudCopyrightError::ConflictVersionChanged => "conflict_version_changed".to_string(),
        CloudCopyrightError::MembershipRevoked => "blocked_by_membership_revoked".to_string(),
        CloudCopyrightError::Forbidden => "forbidden".to_string(),
        CloudCopyrightError::RoleDenied => "role_denied".to_string(),
        CloudCopyrightError::Database(_) => "database_error".to_string(),
        other => format!("{other:?}"),
    }
}

#[cfg(feature = "postgres")]
fn assert_one_accepted_one_duplicate(
    a: &Result<CloudCopyrightDisposition, CloudCopyrightError>,
    b: &Result<CloudCopyrightDisposition, CloudCopyrightError>,
) -> Result<(), Box<dyn std::error::Error>> {
    let results = [change_result_name(a), change_result_name(b)];
    if results
        .iter()
        .filter(|result| result.as_str() == "accepted")
        .count()
        != 1
        || results
            .iter()
            .filter(|result| result.as_str() == "duplicate")
            .count()
            != 1
    {
        return Err(format!("expected accepted + duplicate, got {results:?}").into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn assert_one_accepted_one_error(
    a: &Result<CloudCopyrightDisposition, CloudCopyrightError>,
    b: &Result<CloudCopyrightDisposition, CloudCopyrightError>,
    expected_error: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let results = [change_result_name(a), change_result_name(b)];
    if results
        .iter()
        .filter(|result| result.as_str() == "accepted")
        .count()
        != 1
        || results
            .iter()
            .filter(|result| result.as_str() == expected_error)
            .count()
            != 1
    {
        return Err(format!(
            "expected accepted + {expected_error}, got {results:?}; raw results: {a:?}, {b:?}"
        )
        .into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn safe_disposable_url(database_url: &str) -> bool {
    let lower = database_url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_migrate_smoke")
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("cloud_copyright_postgres_concurrency_qa requires --features postgres");
    std::process::exit(2);
}

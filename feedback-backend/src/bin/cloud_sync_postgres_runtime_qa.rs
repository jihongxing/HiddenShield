#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::{
    database::{POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL},
    postgres_auth::PostgresAuthRepository,
    postgres_sync::PostgresCloudSyncRepository,
    repository::{AuthRepository, CloudSyncRepository},
    schema::{
        AuthSessionRequest, CloudSyncBatchRequest, CloudSyncEvent, ContinueAccountCreatorProfile,
        ContinueAccountDevice,
    },
};

#[cfg(feature = "postgres")]
use sqlx::{Executor, PgPool};

#[cfg(feature = "postgres")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| {
            "missing HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or DATABASE_URL for cloud sync Postgres runtime QA"
        })?;
    if !is_safe_sync_qa_url(&database_url) {
        return Err(
            "refusing to run cloud sync Postgres runtime QA against non-disposable database URL; include localhost/127.0.0.1 and hiddenshield_sync_runtime_qa in the URL"
                .into(),
        );
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let pool = PgPool::connect(&database_url).await?;
        execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?;
        assert_sync_tables_absent(&pool).await?;
        execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL).await?;
        assert_sync_tables_present(&pool).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    drop(runtime);

    let qa_result = run_cloud_sync_repository_qa(&database_url);

    let cleanup_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    cleanup_runtime.block_on(async {
        let pool = PgPool::connect(&database_url).await?;
        execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?;
        assert_sync_tables_absent(&pool).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;

    let report = qa_result?;
    println!("{}", report);
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("cloud_sync_postgres_runtime_qa requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
fn run_cloud_sync_repository_qa(
    database_url: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let auth_repo = PostgresAuthRepository::connect(database_url, 5)?;
    let sync_repo = PostgresCloudSyncRepository::connect(database_url, 5)?;
    let run_id = std::env::var("HIDDENSHIELD_SYNC_POSTGRES_QA_RUN_ID")
        .unwrap_or_else(|_| format!("{}", chrono::Utc::now().timestamp_millis()));
    let identifier = format!("sync-postgres-{run_id}@example.test");
    let free_identifier = format!("sync-free-postgres-{run_id}@example.test");
    let profile = ContinueAccountCreatorProfile {
        display_name: "Postgres Sync QA".to_string(),
        creator_seed_ref: format!("seed-ref-{run_id}"),
        seed_envelope_version: 1,
    };
    let desktop_device = ContinueAccountDevice {
        client_device_id: format!("pg-sync-desktop-{run_id}"),
        name: "Desktop Sync Runtime QA".to_string(),
        platform: "desktop".to_string(),
        app_version: "0.1.0-qa".to_string(),
        public_key: None,
    };
    let mobile_device = ContinueAccountDevice {
        client_device_id: format!("pg-sync-mobile-{run_id}"),
        name: "Mobile Sync Runtime QA".to_string(),
        platform: "android".to_string(),
        app_version: "0.1.0-qa".to_string(),
        public_key: None,
    };

    let desktop_session = auth_repo.create_auth_session(&AuthSessionRequest {
        identifier: identifier.clone(),
        challenge_id: None,
        verification_code: String::new(),
        password: "sync qa password".to_string(),
        device: desktop_device.clone(),
        local_creator_profile: profile.clone(),
    })?;
    enable_cloud_sync(database_url, &desktop_session.account.id)?;
    let event_id = format!("event-{run_id}-001");
    let first_push = sync_repo.push_cloud_events_batch(
        &desktop_session.access_token,
        &CloudSyncBatchRequest {
            device_id: desktop_session.device.id.clone(),
            workspace_id: desktop_session.workspace.id.clone(),
            events: vec![CloudSyncEvent {
                client_event_id: event_id.clone(),
                operation: "upsert_watermark_record".to_string(),
                entity_type: "vault_record".to_string(),
                entity_id: format!("record-{run_id}-001"),
                payload: serde_json::json!({
                    "recordId": format!("record-{run_id}-001"),
                    "watermarkUid": "HS-11112222-33334444-55556666-77778888",
                    "syncQa": true
                }),
            }],
        },
    )?;
    assert_eq!(first_push.accepted, 1);
    assert_eq!(first_push.accepted_event_ids, vec![event_id.clone()]);
    assert_eq!(first_push.next_cursor.as_deref(), Some("cursor_1"));

    let mobile_session = auth_repo.create_auth_session(&AuthSessionRequest {
        identifier: identifier.clone(),
        challenge_id: None,
        verification_code: String::new(),
        password: "sync qa password".to_string(),
        device: mobile_device.clone(),
        local_creator_profile: profile.clone(),
    })?;
    assert_eq!(mobile_session.cloud_vault_cursor, None);

    let duplicate_push = sync_repo.push_cloud_events_batch(
        &desktop_session.access_token,
        &CloudSyncBatchRequest {
            device_id: desktop_session.device.id.clone(),
            workspace_id: desktop_session.workspace.id.clone(),
            events: vec![CloudSyncEvent {
                client_event_id: event_id.clone(),
                operation: "upsert_watermark_record".to_string(),
                entity_type: "vault_record".to_string(),
                entity_id: format!("record-{run_id}-001"),
                payload: serde_json::json!({
                    "recordId": format!("record-{run_id}-001"),
                    "watermarkUid": "HS-11112222-33334444-55556666-77778888",
                    "syncQa": true
                }),
            }],
        },
    )?;
    assert_eq!(duplicate_push.accepted, 1);
    assert_eq!(duplicate_push.next_cursor.as_deref(), Some("cursor_1"));
    assert_eq!(duplicate_push.event_results[0].disposition, "duplicate");

    let changed_duplicate_push = sync_repo.push_cloud_events_batch(
        &desktop_session.access_token,
        &CloudSyncBatchRequest {
            device_id: desktop_session.device.id.clone(),
            workspace_id: desktop_session.workspace.id.clone(),
            events: vec![CloudSyncEvent {
                client_event_id: event_id.clone(),
                operation: "upsert_watermark_record".to_string(),
                entity_type: "vault_record".to_string(),
                entity_id: format!("record-{run_id}-001"),
                payload: serde_json::json!({ "duplicate": true }),
            }],
        },
    )?;
    assert_eq!(changed_duplicate_push.accepted, 0);
    assert_eq!(
        changed_duplicate_push.event_results[0].disposition,
        "conflict_payload_changed"
    );
    assert_eq!(
        changed_duplicate_push.next_cursor.as_deref(),
        Some("cursor_1")
    );

    let mobile_initial = sync_repo.get_cloud_changes(
        &mobile_session.access_token,
        Some(&mobile_session.workspace.id),
        Some("cursor_999"),
    )?;
    assert_eq!(mobile_initial.next_cursor, "cursor_1");
    assert_eq!(mobile_initial.changes.len(), 1);
    assert_eq!(mobile_initial.changes[0].operation, "upsert");
    assert_eq!(
        mobile_initial.changes[0].source_device.as_deref(),
        Some(desktop_session.device.id.as_str())
    );

    let mobile_repeat = sync_repo.get_cloud_changes(
        &mobile_session.access_token,
        Some(&mobile_session.workspace.id),
        Some(&mobile_initial.next_cursor),
    )?;
    assert_eq!(mobile_repeat.next_cursor, "cursor_1");
    assert!(mobile_repeat.changes.is_empty());

    let second_event_id = format!("event-{run_id}-002");
    let second_push = sync_repo.push_cloud_events_batch(
        &desktop_session.access_token,
        &CloudSyncBatchRequest {
            device_id: desktop_session.device.id.clone(),
            workspace_id: desktop_session.workspace.id.clone(),
            events: vec![CloudSyncEvent {
                client_event_id: second_event_id.clone(),
                operation: "delete".to_string(),
                entity_type: "vault_record".to_string(),
                entity_id: format!("record-{run_id}-001"),
                payload: serde_json::json!({
                    "recordId": format!("record-{run_id}-001"),
                    "deleted": true
                }),
            }],
        },
    )?;
    assert_eq!(second_push.accepted, 1);
    assert_eq!(second_push.next_cursor.as_deref(), Some("cursor_2"));
    let mobile_resume = sync_repo.get_cloud_changes(
        &mobile_session.access_token,
        Some(&mobile_session.workspace.id),
        Some(&mobile_initial.next_cursor),
    )?;
    assert_eq!(mobile_resume.next_cursor, "cursor_2");
    assert_eq!(mobile_resume.changes.len(), 1);
    assert_eq!(mobile_resume.changes[0].operation, "delete");

    let wrong_device_rejected = sync_repo
        .push_cloud_events_batch(
            &desktop_session.access_token,
            &CloudSyncBatchRequest {
                device_id: mobile_session.device.id.clone(),
                workspace_id: desktop_session.workspace.id.clone(),
                events: vec![CloudSyncEvent {
                    client_event_id: format!("event-{run_id}-wrong-device"),
                    operation: "upsert".to_string(),
                    entity_type: "vault_record".to_string(),
                    entity_id: format!("record-{run_id}-wrong-device"),
                    payload: serde_json::json!({}),
                }],
            },
        )
        .is_err();
    assert!(wrong_device_rejected);

    let free_session = auth_repo.create_auth_session(&AuthSessionRequest {
        identifier: free_identifier,
        challenge_id: None,
        verification_code: String::new(),
        password: "free sync qa password".to_string(),
        device: ContinueAccountDevice {
            client_device_id: format!("pg-sync-free-{run_id}"),
            name: "Free Sync Runtime QA".to_string(),
            platform: "desktop".to_string(),
            app_version: "0.1.0-qa".to_string(),
            public_key: None,
        },
        local_creator_profile: profile,
    })?;
    let free_push_forbidden = sync_repo
        .push_cloud_events_batch(
            &free_session.access_token,
            &CloudSyncBatchRequest {
                device_id: free_session.device.id.clone(),
                workspace_id: free_session.workspace.id.clone(),
                events: vec![CloudSyncEvent {
                    client_event_id: format!("event-{run_id}-free"),
                    operation: "upsert".to_string(),
                    entity_type: "vault_record".to_string(),
                    entity_id: format!("record-{run_id}-free"),
                    payload: serde_json::json!({}),
                }],
            },
        )
        .is_err();
    assert!(free_push_forbidden);

    Ok(serde_json::json!({
        "ok": true,
        "qa": "cloud_sync_postgres_runtime_qa",
        "repository": "CloudSyncRepository",
        "adapter": "PostgresCloudSyncRepository",
        "runId": run_id,
        "checks": {
            "desktopPushAccepted": true,
            "duplicateClientEventIdIdempotent": true,
            "mobilePullInitialCount": mobile_initial.changes.len(),
            "mobileRepeatPullCount": mobile_repeat.changes.len(),
            "resumeAfterCursorCount": mobile_resume.changes.len(),
            "cursorAfterFirstPush": first_push.next_cursor,
            "cursorAfterSecondPush": second_push.next_cursor,
            "wrongDeviceRejected": wrong_device_rejected,
            "freePushForbidden": free_push_forbidden,
            "authRepositoryRequired": true,
            "registryRepositoryWritePath": "not_executed"
        },
        "productionDatabaseAllowed": false
    }))
}

#[cfg(feature = "postgres")]
fn enable_cloud_sync(
    database_url: &str,
    account_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let pool = PgPool::connect(database_url).await?;
        sqlx::query(
            "UPDATE cloud_accounts
             SET entitlement_plan_name = '创作者版',
                 entitlement_plan_code = 'creator',
                 entitlement_status = 'active',
                 entitlement_features_json = $2,
                 updated_at = now()
             WHERE id = $1",
        )
        .bind(account_id)
        .bind(serde_json::json!({
            "cloud_sync": true,
            "batch_processing": true,
            "report_export": true,
            "cloud_batch_processing": false,
            "cloud_video_processing": false,
            "priority_queue": false,
            "team_workspace": false,
            "api_access": false
        }))
        .execute(&pool)
        .await?;
        Ok::<(), sqlx::Error>(())
    })?;
    Ok(())
}

#[cfg(feature = "postgres")]
fn is_safe_sync_qa_url(database_url: &str) -> bool {
    let lower = database_url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_sync_runtime_qa")
}

#[cfg(feature = "postgres")]
async fn execute_sql_batch(pool: &PgPool, sql: &str) -> Result<(), sqlx::Error> {
    for statement in sql.split(';') {
        let statement = statement.trim();
        if statement.is_empty() {
            continue;
        }
        pool.execute(statement).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_sync_tables_present(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    for table in [
        "cloud_accounts",
        "cloud_devices",
        "cloud_sessions",
        "cloud_sync_events",
        "cloud_device_cursors",
    ] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = $1
            )",
        )
        .bind(table)
        .fetch_one(pool)
        .await?;
        if !exists {
            return Err(format!("expected sync table {table} to exist").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_sync_tables_absent(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    for table in [
        "cloud_accounts",
        "cloud_devices",
        "cloud_sessions",
        "cloud_sync_events",
        "cloud_device_cursors",
    ] {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.tables
                WHERE table_schema = 'public' AND table_name = $1
            )",
        )
        .bind(table)
        .fetch_one(pool)
        .await?;
        if exists {
            return Err(format!("expected sync table {table} to be absent").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::{
    database::{POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL},
    postgres_auth::PostgresAuthRepository,
    postgres_registry::PostgresWatermarkRegistryRepository,
    repository::{AuthRepository, WatermarkRegistryRepository},
    schema::{
        AuthSessionRequest, ContinueAccountCreatorProfile, ContinueAccountDevice,
        WatermarkIdConfirmRequest, WatermarkIdReconcileRequest, WatermarkIdReissueRequest,
        WatermarkIdReserveRequest,
    },
};

#[cfg(feature = "postgres")]
use sqlx::{Executor, PgPool};

#[cfg(feature = "postgres")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| {
            "missing HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or DATABASE_URL for watermark registry Postgres runtime QA"
        })?;
    if !is_safe_registry_qa_url(&database_url) {
        return Err(
            "refusing to run watermark registry Postgres runtime QA against non-disposable database URL; include localhost/127.0.0.1 and hiddenshield_registry_runtime_qa in the URL"
                .into(),
        );
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let pool = PgPool::connect(&database_url).await?;
        execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?;
        assert_registry_tables_absent(&pool).await?;
        execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL).await?;
        assert_registry_tables_present(&pool).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    drop(runtime);

    let qa_result = run_watermark_registry_repository_qa(&database_url);

    let cleanup_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    cleanup_runtime.block_on(async {
        let pool = PgPool::connect(&database_url).await?;
        execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?;
        assert_registry_tables_absent(&pool).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;

    let report = qa_result?;
    println!("{}", report);
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("watermark_registry_postgres_runtime_qa requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
fn run_watermark_registry_repository_qa(
    database_url: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let auth_repo = PostgresAuthRepository::connect(database_url, 5)?;
    let registry_repo = PostgresWatermarkRegistryRepository::connect(database_url, 5)?;
    let run_id = std::env::var("HIDDENSHIELD_REGISTRY_POSTGRES_QA_RUN_ID")
        .unwrap_or_else(|_| format!("{}", chrono::Utc::now().timestamp_millis()));
    let profile = ContinueAccountCreatorProfile {
        display_name: "Postgres Registry QA".to_string(),
        creator_seed_ref: format!("seed-ref-{run_id}"),
        seed_envelope_version: 1,
    };
    let session = auth_repo.create_auth_session(&AuthSessionRequest {
        identifier: format!("registry-postgres-{run_id}@example.test"),
        challenge_id: None,
        verification_code: String::new(),
        password: "registry qa password".to_string(),
        device: ContinueAccountDevice {
            client_device_id: format!("pg-registry-device-{run_id}"),
            name: "Registry Runtime QA".to_string(),
            platform: "desktop".to_string(),
            app_version: "0.1.0-qa".to_string(),
            public_key: None,
        },
        local_creator_profile: profile.clone(),
    })?;

    let reserve_request = WatermarkIdReserveRequest {
        request_id: format!("registry-request-{run_id}"),
        workspace_id: session.workspace.id.clone(),
        creator_profile_id: session.creator_profile.id.clone(),
        media_type: "image".to_string(),
        payload_protocol_version: 3,
        payload_bytes_length: 39,
        parent_watermark_uid: None,
        revision: 1,
        original_hash: Some(format!("sha256:original-{run_id}")),
    };
    let reserved = registry_repo.reserve_watermark_id(&session.access_token, &reserve_request)?;
    assert_eq!(reserved.registry_status, "reserved");
    assert_eq!(reserved.watermark_id_issue_mode, "server_reserved");
    assert!(reserved.watermark_uid.starts_with("HS-"));
    assert_eq!(reserved.watermark_uid.len(), 38);

    let duplicate_reserved =
        registry_repo.reserve_watermark_id(&session.access_token, &reserve_request)?;
    assert_eq!(duplicate_reserved.registry_id, reserved.registry_id);
    assert_eq!(duplicate_reserved.watermark_uid, reserved.watermark_uid);

    let confirmed = registry_repo.confirm_watermark_id(
        &session.access_token,
        &WatermarkIdConfirmRequest {
            workspace_id: session.workspace.id.clone(),
            creator_profile_id: session.creator_profile.id.clone(),
            watermark_uid: reserved.watermark_uid.clone(),
            payload_protocol_version: 3,
            payload_bytes_length: 39,
            original_hash: Some(format!("sha256:original-{run_id}")),
            protected_copy_hash: Some(format!("sha256:protected-{run_id}")),
            write_verification_status: "verified".to_string(),
        },
    )?;
    assert_eq!(confirmed.registry_status, "server_confirmed");
    assert_eq!(confirmed.watermark_id_issue_mode, "server_confirmed");

    let offline_uid = "HS-11112222-33334444-55556666-77778888".to_string();
    let reconciled = registry_repo.reconcile_watermark_id(
        &session.access_token,
        &WatermarkIdReconcileRequest {
            workspace_id: session.workspace.id.clone(),
            creator_profile_id: session.creator_profile.id.clone(),
            watermark_uid: offline_uid.clone(),
            media_type: "audio".to_string(),
            payload_protocol_version: 3,
            payload_bytes_length: 39,
            parent_watermark_uid: None,
            revision: 1,
            original_hash: Some(format!("sha256:offline-original-{run_id}")),
            protected_copy_hash: Some(format!("sha256:offline-protected-{run_id}")),
            write_verification_status: Some("verified".to_string()),
        },
    )?;
    assert_eq!(reconciled.watermark_uid, offline_uid);
    assert_eq!(reconciled.registry_status, "offline_confirmed");
    assert_eq!(reconciled.watermark_id_issue_mode, "offline_generated");

    let conflict_session = auth_repo.create_auth_session(&AuthSessionRequest {
        identifier: format!("registry-conflict-{run_id}@example.test"),
        challenge_id: None,
        verification_code: String::new(),
        password: "registry conflict qa password".to_string(),
        device: ContinueAccountDevice {
            client_device_id: format!("pg-registry-conflict-device-{run_id}"),
            name: "Registry Conflict Runtime QA".to_string(),
            platform: "desktop".to_string(),
            app_version: "0.1.0-qa".to_string(),
            public_key: None,
        },
        local_creator_profile: profile,
    })?;
    let conflict = registry_repo.reconcile_watermark_id(
        &conflict_session.access_token,
        &WatermarkIdReconcileRequest {
            workspace_id: conflict_session.workspace.id.clone(),
            creator_profile_id: conflict_session.creator_profile.id.clone(),
            watermark_uid: offline_uid.clone(),
            media_type: "audio".to_string(),
            payload_protocol_version: 3,
            payload_bytes_length: 39,
            parent_watermark_uid: None,
            revision: 1,
            original_hash: Some(format!("sha256:other-original-{run_id}")),
            protected_copy_hash: None,
            write_verification_status: Some("verified".to_string()),
        },
    )?;
    assert_eq!(conflict.registry_status, "conflict");

    let reissue = registry_repo.reissue_watermark_id(
        &session.access_token,
        &WatermarkIdReissueRequest {
            workspace_id: session.workspace.id.clone(),
            creator_profile_id: session.creator_profile.id.clone(),
            previous_watermark_uid: conflict.watermark_uid.clone(),
            media_type: "audio".to_string(),
            payload_protocol_version: 3,
            payload_bytes_length: 39,
            parent_watermark_uid: Some(conflict.watermark_uid.clone()),
            revision: 2,
            reason: "conflict_recovery".to_string(),
            original_hash: Some(format!("sha256:reissue-original-{run_id}")),
        },
    )?;
    assert_eq!(reissue.previous_watermark_uid, conflict.watermark_uid);
    assert_eq!(reissue.replacement.registry_status, "reserved");
    assert_eq!(
        reissue.replacement.watermark_id_issue_mode,
        "server_reissued"
    );
    assert_eq!(reissue.replacement.revision, 2);
    assert_eq!(
        reissue.replacement.parent_watermark_uid.as_deref(),
        Some(conflict.watermark_uid.as_str())
    );

    Ok(serde_json::json!({
        "ok": true,
        "qa": "watermark_registry_postgres_runtime_qa",
        "repository": "WatermarkRegistryRepository",
        "adapter": "PostgresWatermarkRegistryRepository",
        "runId": run_id,
        "checks": {
            "serverReserve": true,
            "idempotentReserveByRequestId": true,
            "serverConfirm": true,
            "offlineReconcile": true,
            "conflictDetection": true,
            "reissueCreated": true,
            "longWatermarkUidPreserved": true,
            "authRepositoryRequired": true,
            "syncRepositoryWritePath": "not_executed",
            "formalUiMockReleaseDefaultPath": "not_switched"
        },
        "productionDatabaseAllowed": false
    }))
}

#[cfg(feature = "postgres")]
fn is_safe_registry_qa_url(database_url: &str) -> bool {
    let lower = database_url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_registry_runtime_qa")
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
async fn assert_registry_tables_present(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    for table in [
        "cloud_accounts",
        "cloud_devices",
        "cloud_sessions",
        "watermark_id_registry",
        "watermark_id_reissue_jobs",
        "rights_manifests",
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
            return Err(format!("expected registry table {table} to exist").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_registry_tables_absent(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    for table in [
        "cloud_accounts",
        "cloud_devices",
        "cloud_sessions",
        "watermark_id_registry",
        "watermark_id_reissue_jobs",
        "rights_manifests",
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
            return Err(format!("expected registry table {table} to be absent").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::{
    database::{POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL},
    postgres_auth::PostgresAuthRepository,
    repository::AuthRepository,
    schema::{
        AuthChallengeRequest, AuthLogoutRequest, AuthRefreshRequest, AuthSessionRequest,
        ContinueAccountCreatorProfile, ContinueAccountDevice,
    },
};

#[cfg(feature = "postgres")]
use sqlx::{Executor, PgPool};

#[cfg(feature = "postgres")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| {
            "missing HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or DATABASE_URL for auth Postgres runtime QA"
        })?;
    if !is_safe_auth_qa_url(&database_url) {
        return Err(
            "refusing to run auth Postgres runtime QA against non-disposable database URL; include localhost/127.0.0.1 and hiddenshield_auth_runtime_qa in the URL"
                .into(),
        );
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async {
        let pool = PgPool::connect(&database_url).await?;
        execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?;
        assert_auth_tables_absent(&pool).await?;
        execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL).await?;
        assert_auth_tables_present(&pool).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;
    drop(runtime);

    let qa_result = run_auth_repository_qa(&database_url);

    let cleanup_runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    cleanup_runtime.block_on(async {
        let pool = PgPool::connect(&database_url).await?;
        execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?;
        assert_auth_tables_absent(&pool).await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    })?;

    let report = qa_result?;
    println!("{}", report);
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("auth_postgres_runtime_qa requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
fn run_auth_repository_qa(
    database_url: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let repo = PostgresAuthRepository::connect(database_url, 5)?;
    let run_id = std::env::var("HIDDENSHIELD_AUTH_POSTGRES_QA_RUN_ID")
        .unwrap_or_else(|_| format!("{}", chrono::Utc::now().timestamp_millis()));
    let identifier = format!("auth-postgres-{run_id}@example.test");
    let profile = ContinueAccountCreatorProfile {
        display_name: "Postgres Auth QA".to_string(),
        creator_seed_ref: format!("seed-ref-{run_id}"),
        seed_envelope_version: 1,
    };
    let desktop_device = ContinueAccountDevice {
        client_device_id: format!("pg-auth-desktop-{run_id}"),
        name: "Desktop Auth Runtime QA".to_string(),
        platform: "desktop".to_string(),
        app_version: "0.1.0-qa".to_string(),
        public_key: None,
    };
    let mobile_device = ContinueAccountDevice {
        client_device_id: format!("pg-auth-mobile-{run_id}"),
        name: "Mobile Auth Runtime QA".to_string(),
        platform: "android".to_string(),
        app_version: "0.1.0-qa".to_string(),
        public_key: Some(format!("qa-public-key-{run_id}")),
    };

    let challenge = repo.create_auth_challenge(&AuthChallengeRequest {
        identifier: identifier.clone(),
        purpose: "register_or_login".to_string(),
        client_device_id: desktop_device.client_device_id.clone(),
        captcha_token: None,
    })?;
    assert_eq!(challenge.delivery_channel, "fixture");
    assert_eq!(challenge.fixture_code.as_deref(), Some("000000"));

    let challenge_session = repo.create_auth_session(&AuthSessionRequest {
        identifier: identifier.clone(),
        challenge_id: Some(challenge.challenge_id.clone()),
        verification_code: "000000".to_string(),
        password: String::new(),
        device: desktop_device.clone(),
        local_creator_profile: profile.clone(),
    })?;
    assert_eq!(challenge_session.device.id, desktop_device.client_device_id);
    assert_eq!(challenge_session.entitlement.plan_code, "free");

    let password_session = repo.create_auth_session(&AuthSessionRequest {
        identifier: identifier.clone(),
        challenge_id: None,
        verification_code: String::new(),
        password: "correct horse battery staple".to_string(),
        device: mobile_device.clone(),
        local_creator_profile: profile.clone(),
    })?;
    assert_eq!(password_session.account.id, challenge_session.account.id);
    assert_eq!(password_session.device.id, mobile_device.client_device_id);

    let refreshed = repo.refresh_auth_session(&AuthRefreshRequest {
        refresh_token: password_session.refresh_token.clone(),
        device_id: password_session.device.id.clone(),
    })?;
    assert_ne!(refreshed.refresh_token, password_session.refresh_token);
    let old_refresh_rejected = repo
        .refresh_auth_session(&AuthRefreshRequest {
            refresh_token: password_session.refresh_token.clone(),
            device_id: password_session.device.id.clone(),
        })
        .is_err();
    assert!(old_refresh_rejected, "old refresh token must be revoked");

    let devices = repo.list_devices(&refreshed.access_token)?;
    assert_eq!(devices.devices.len(), 2);
    assert!(devices
        .devices
        .iter()
        .any(|device| device.id == refreshed.device.id && device.is_current));
    assert!(devices
        .devices
        .iter()
        .any(|device| device.id == challenge_session.device.id && !device.is_current));

    let revoked = repo.revoke_device(&refreshed.access_token, &challenge_session.device.id)?;
    assert!(revoked.ok);
    assert_eq!(revoked.device_id, challenge_session.device.id);
    assert!(revoked.revoked_session_count >= 1);
    let after_revoke = repo.list_devices(&refreshed.access_token)?;
    let revoked_device = after_revoke
        .devices
        .iter()
        .find(|device| device.id == challenge_session.device.id)
        .ok_or("revoked device missing from list")?;
    assert!(!revoked_device.registered);

    let logout = repo.logout_auth_session(&AuthLogoutRequest {
        refresh_token: refreshed.refresh_token.clone(),
        device_id: refreshed.device.id.clone(),
    })?;
    assert!(logout.ok);
    let logged_out_refresh_rejected = repo
        .refresh_auth_session(&AuthRefreshRequest {
            refresh_token: refreshed.refresh_token,
            device_id: refreshed.device.id.clone(),
        })
        .is_err();
    assert!(
        logged_out_refresh_rejected,
        "logged-out refresh token must be rejected"
    );

    Ok(serde_json::json!({
        "ok": true,
        "qa": "auth_postgres_runtime_qa",
        "repository": "AuthRepository",
        "adapter": "PostgresAuthRepository",
        "runId": run_id,
        "checks": {
            "challengeFixtureCode": true,
            "challengeSessionCreated": true,
            "passwordSessionCreated": true,
            "accountParityAcrossDevices": true,
            "refreshRotation": true,
            "oldRefreshRejected": old_refresh_rejected,
            "deviceListCount": 2,
            "deviceRevoke": true,
            "logoutRevokesRefresh": logged_out_refresh_rejected,
            "syncRepositoryWritePath": "not_executed",
            "registryRepositoryWritePath": "not_executed"
        },
        "productionDatabaseAllowed": false
    }))
}

#[cfg(feature = "postgres")]
fn is_safe_auth_qa_url(database_url: &str) -> bool {
    let lower = database_url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_auth_runtime_qa")
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
async fn assert_auth_tables_present(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    for table in [
        "cloud_accounts",
        "cloud_devices",
        "cloud_sessions",
        "auth_challenges",
        "auth_attempts",
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
            return Err(format!("expected auth table {table} to exist").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_auth_tables_absent(pool: &PgPool) -> Result<(), Box<dyn std::error::Error>> {
    for table in [
        "cloud_accounts",
        "cloud_devices",
        "cloud_sessions",
        "auth_challenges",
        "auth_attempts",
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
            return Err(format!("expected auth table {table} to be absent").into());
        }
    }
    Ok(())
}

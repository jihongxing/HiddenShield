#[cfg(feature = "postgres")]
use std::sync::{Arc, Barrier};
#[cfg(feature = "postgres")]
use std::thread;

#[cfg(feature = "postgres")]
use chrono::{Duration, Utc};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_credential_custody::{
    create_postgres_ready_marking_session, issue_postgres_production_credential,
    revoke_postgres_production_credential, rotate_postgres_production_credential,
    CreateReadyMarkingSessionCommand, CreateReadyMarkingSessionOutcome, CredentialCustodyConfig,
    CredentialCustodyError, CustodyAuthorizationDecision, CustodyAuthorizationInput,
    IssueProductionCredentialCommand, PepperMaterial, ProductionCredentialCustodyAuthorization,
    RevokeProductionCredentialCommand, RotateProductionCredentialCommand,
    REASON_CREDENTIAL_EXPIRED, REASON_CREDENTIAL_INACTIVE, REASON_CREDENTIAL_SCOPE_DENIED,
    REASON_CREDENTIAL_UNAUTHORIZED, REASON_IDEMPOTENCY_CONFLICT, REASON_LICENSE_INACTIVE,
    REASON_PROFILE_NOT_ENTITLED,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_production_provider::{
    ProductionCustodyOperation, ProductionProviderDeploymentError, ProductionProviderReadiness,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::database::{
    POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL, POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL,
    POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
    POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_UP_SQL,
    POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_UP_SQL,
    POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_UP_SQL,
};
#[cfg(feature = "postgres")]
use serde::Serialize;
#[cfg(feature = "postgres")]
use sqlx::{Connection, Row};

#[cfg(feature = "postgres")]
const PROFILE_IDS: [&str; 2] = [
    "hiddenshield_v3_image_anchor_v1",
    "cn_aigc_label_2025_image_export_v1",
];

#[cfg(feature = "postgres")]
struct AllowCustodyAuthorization;

#[cfg(feature = "postgres")]
impl ProductionCredentialCustodyAuthorization for AllowCustodyAuthorization {
    fn authorize(&self, input: &CustodyAuthorizationInput<'_>) -> CustodyAuthorizationDecision {
        CustodyAuthorizationDecision {
            authorized: input.actor_token_hash == "authorized-token-hash"
                && matches!(
                    input.operation,
                    "issue_production_credential"
                        | "rotate_production_credential"
                        | "revoke_production_credential"
                )
                && input.custody_key_id == "kms-key-qa",
            reason_code: None,
            receipt_id: Some(format!("receipt-{}", input.license_id)),
        }
    }
}

#[cfg(feature = "postgres")]
struct DenyCustodyAuthorization;

#[cfg(feature = "postgres")]
impl ProductionCredentialCustodyAuthorization for DenyCustodyAuthorization {
    fn authorize(&self, _input: &CustodyAuthorizationInput<'_>) -> CustodyAuthorizationDecision {
        CustodyAuthorizationDecision {
            authorized: false,
            reason_code: Some(REASON_CREDENTIAL_UNAUTHORIZED.to_string()),
            receipt_id: None,
        }
    }
}

#[cfg(feature = "postgres")]
struct UnavailableProvider;

#[cfg(feature = "postgres")]
impl ProductionProviderReadiness for UnavailableProvider {
    fn ensure_ready(
        &self,
        _operation: ProductionCustodyOperation,
    ) -> Result<(), ProductionProviderDeploymentError> {
        Err(ProductionProviderDeploymentError::Unavailable("kms_health"))
    }
}

#[cfg(feature = "postgres")]
struct QaReadyProvider;

#[cfg(feature = "postgres")]
impl ProductionProviderReadiness for QaReadyProvider {
    fn ensure_ready(
        &self,
        _operation: ProductionCustodyOperation,
    ) -> Result<(), ProductionProviderDeploymentError> {
        Ok(())
    }
}

#[cfg(feature = "postgres")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScenarioResult {
    scenario_id: &'static str,
    winner_count: usize,
    reason_codes: Vec<String>,
    credential_count: i64,
    session_count: i64,
    ready_session_count: i64,
    audit_count: i64,
    credential_last_used_count: i64,
}

#[cfg(feature = "postgres")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| "missing disposable PostgreSQL URL for credential custody QA")?;
    if !is_safe_smoke_url(&database_url) {
        return Err(
            "refusing credential QA against non-disposable URL; require localhost/127.0.0.1 and hiddenshield_migrate_smoke"
                .into(),
        );
    }
    let pool = sqlx::PgPool::connect(&database_url).await?;
    reset_schema(&pool).await?;
    let config = config();
    let authorization = AllowCustodyAuthorization;
    let mut results = Vec::new();

    results.push(run_unauthorized_issuance(&pool, &database_url, &config).await?);
    results.push(
        run_provider_unavailable_zero_write(&pool, &database_url, &config, &authorization).await?,
    );
    results.push(run_valid_session(&pool, &database_url, &config, &authorization).await?);
    results.push(
        run_rejection(
            &pool,
            &database_url,
            &config,
            &authorization,
            "suspended_credential",
            "UPDATE ai_sdk_credential_bindings SET status = 'suspended' WHERE credential_id = $1",
            REASON_CREDENTIAL_INACTIVE,
        )
        .await?,
    );
    results.push(
        run_rejection(
            &pool,
            &database_url,
            &config,
            &authorization,
            "expired_credential",
            "UPDATE ai_sdk_credential_bindings SET expires_at = NOW() - INTERVAL '1 minute' WHERE credential_id = $1",
            REASON_CREDENTIAL_EXPIRED,
        )
        .await?,
    );
    results.push(
        run_rejection(
            &pool,
            &database_url,
            &config,
            &authorization,
            "scope_denied",
            "UPDATE ai_sdk_credential_bindings SET scopes_json = '[\"verify:public\"]'::jsonb WHERE credential_id = $1",
            REASON_CREDENTIAL_SCOPE_DENIED,
        )
        .await?,
    );
    results.push(
        run_rejection(
            &pool,
            &database_url,
            &config,
            &authorization,
            "inactive_license",
            "UPDATE ai_transparency_licenses SET status = 'suspended' WHERE license_id = (
                SELECT license_id FROM ai_sdk_credential_bindings WHERE credential_id = $1
            )",
            REASON_LICENSE_INACTIVE,
        )
        .await?,
    );
    results.push(run_profile_rejection(&pool, &database_url, &config, &authorization).await?);
    results.push(run_concurrent_idempotency(&pool, &database_url, &config, &authorization).await?);
    results.push(run_rotate_revokes_old(&pool, &database_url, &config, &authorization).await?);
    results.push(run_revoke_blocks_session(&pool, &database_url, &config, &authorization).await?);

    println!("{}", serde_json::to_string_pretty(&results)?);
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_unauthorized_issuance(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "unauthorized_credential_issuance";
    seed_license_and_profiles(pool, scenario_id).await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let result = issue_postgres_production_credential(
        &mut connection,
        config,
        &DenyCustodyAuthorization,
        &IssueProductionCredentialCommand {
            credential_id: format!("credential-{scenario_id}"),
            api_key_id: format!("api-key-{scenario_id}"),
            license_id: format!("license-{scenario_id}"),
            scopes: vec!["mark:image".to_string()],
            issuer_modes: vec!["hiddenshield_managed".to_string()],
            expires_at: Utc::now() + Duration::hours(1),
            actor_token_hash: "denied-token-hash".to_string(),
            audit_event_id: format!("audit-issue-{scenario_id}"),
        },
    )
    .await;
    if result.is_ok() {
        return Err("unauthorized production credential issuance succeeded".into());
    }
    let scenario = snapshot(pool, scenario_id, &[]).await?;
    if scenario.credential_count != 0 || scenario.session_count != 0 || scenario.audit_count != 0 {
        return Err(format!("unauthorized issuance left side effects: {scenario:?}").into());
    }
    Ok(ScenarioResult {
        reason_codes: vec![REASON_CREDENTIAL_UNAUTHORIZED.to_string()],
        ..scenario
    })
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("ai_transparency_credential_custody_qa requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
fn config() -> CredentialCustodyConfig {
    CredentialCustodyConfig {
        custody_key_id: "kms-key-qa".to_string(),
        active_pepper: PepperMaterial {
            key_id: "kms-key-qa".to_string(),
            version: "qa-v1".to_string(),
            secret: "qa-only-secret-at-least-thirty-two-bytes-long".to_string(),
        },
        retained_peppers: Vec::new(),
        provider_readiness: std::sync::Arc::new(QaReadyProvider),
    }
}

#[cfg(feature = "postgres")]
fn is_safe_smoke_url(database_url: &str) -> bool {
    let lower = database_url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_migrate_smoke")
}

#[cfg(feature = "postgres")]
async fn reset_schema(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(pool)
        .await?;
    for migration in [
        POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL,
        POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL,
        POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
        POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_UP_SQL,
        POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_UP_SQL,
        POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_UP_SQL,
    ] {
        sqlx::raw_sql(migration).execute(pool).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_valid_session(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "valid_production_credential";
    let issued = seed_and_issue(pool, database_url, config, authorization, scenario_id).await?;
    assert_cleartext_not_persisted(pool, &issued.cleartext_api_key).await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let outcome = create_postgres_ready_marking_session(
        &mut connection,
        config,
        &session_command(
            scenario_id,
            &issued.cleartext_api_key,
            "idem-valid",
            "session-valid",
        ),
    )
    .await?;
    let result = snapshot(pool, scenario_id, &[outcome]).await?;
    assert_success(&result)?;
    Ok(result)
}

#[cfg(feature = "postgres")]
async fn run_provider_unavailable_zero_write(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "provider_unavailable";
    let unavailable_config = CredentialCustodyConfig {
        custody_key_id: config.custody_key_id.clone(),
        active_pepper: config.active_pepper.clone(),
        retained_peppers: config.retained_peppers.clone(),
        provider_readiness: Arc::new(UnavailableProvider),
    };
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let issue = issue_postgres_production_credential(
        &mut connection,
        &unavailable_config,
        authorization,
        &IssueProductionCredentialCommand {
            credential_id: format!("credential-{scenario_id}"),
            api_key_id: format!("api-key-{scenario_id}"),
            license_id: format!("license-{scenario_id}"),
            scopes: vec!["mark:image".to_string()],
            issuer_modes: vec!["hiddenshield_managed".to_string()],
            expires_at: Utc::now() + Duration::hours(1),
            actor_token_hash: "authorized-token-hash".to_string(),
            audit_event_id: format!("audit-issue-{scenario_id}"),
        },
    )
    .await;
    assert_provider_unavailable(issue)?;
    let session = create_postgres_ready_marking_session(
        &mut connection,
        &unavailable_config,
        &session_command(
            scenario_id,
            "hsai_live_missing",
            "idem-unavailable",
            "session",
        ),
    )
    .await;
    assert_provider_unavailable(session)?;

    let issued = seed_and_issue(pool, database_url, config, authorization, scenario_id).await?;
    let rotate = rotate_postgres_production_credential(
        &mut connection,
        &unavailable_config,
        authorization,
        &RotateProductionCredentialCommand {
            previous_credential_id: issued.credential_id.clone(),
            new_credential_id: format!("credential-replacement-{scenario_id}"),
            new_api_key_id: format!("api-key-replacement-{scenario_id}"),
            expires_at: Utc::now() + Duration::hours(1),
            actor_token_hash: "authorized-token-hash".to_string(),
            audit_event_id: format!("audit-rotate-{scenario_id}"),
        },
    )
    .await;
    assert_provider_unavailable(rotate)?;
    let revoke = revoke_postgres_production_credential(
        &mut connection,
        &unavailable_config,
        authorization,
        &RevokeProductionCredentialCommand {
            credential_id: issued.credential_id,
            revoked_reason: "qa-provider-unavailable".to_string(),
            actor_token_hash: "authorized-token-hash".to_string(),
            audit_event_id: format!("audit-revoke-{scenario_id}"),
        },
    )
    .await;
    assert_provider_unavailable(revoke)?;

    let result = snapshot(pool, scenario_id, &[]).await?;
    let active_credentials: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_sdk_credential_bindings
         WHERE license_id = $1 AND status = 'active'",
    )
    .bind(format!("license-{scenario_id}"))
    .fetch_one(pool)
    .await?;
    let lifecycle_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_credential_lifecycle_audit_events WHERE license_id = $1",
    )
    .bind(format!("license-{scenario_id}"))
    .fetch_one(pool)
    .await?;
    if result.credential_count != 1
        || result.session_count != 0
        || result.audit_count != 1
        || active_credentials != 1
        || lifecycle_audit_count != 0
    {
        return Err(format!("provider failure left custody side effects: {result:?}").into());
    }
    Ok(result)
}

#[cfg(feature = "postgres")]
fn assert_provider_unavailable<T>(
    result: Result<T, CredentialCustodyError>,
) -> Result<(), Box<dyn std::error::Error>> {
    match result {
        Err(CredentialCustodyError::ProviderUnavailable(reason))
            if reason.contains("kms_health") =>
        {
            Ok(())
        }
        _ => Err("provider-unavailable operation did not fail closed".into()),
    }
}

#[cfg(feature = "postgres")]
async fn run_rejection(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
    scenario_id: &'static str,
    mutation_sql: &str,
    expected_reason: &str,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let issued = seed_and_issue(pool, database_url, config, authorization, scenario_id).await?;
    sqlx::query(mutation_sql)
        .bind(format!("credential-{scenario_id}"))
        .execute(pool)
        .await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let outcome = create_postgres_ready_marking_session(
        &mut connection,
        config,
        &session_command(
            scenario_id,
            &issued.cleartext_api_key,
            "idem-reject",
            "session-reject",
        ),
    )
    .await?;
    let result = snapshot(pool, scenario_id, &[outcome]).await?;
    assert_rejected(&result, expected_reason)?;
    Ok(result)
}

#[cfg(feature = "postgres")]
async fn run_profile_rejection(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "profile_not_entitled";
    let issued = seed_and_issue(pool, database_url, config, authorization, scenario_id).await?;
    sqlx::query(
        "UPDATE ai_profile_entitlements SET status = 'suspended'
         WHERE license_id = $1 AND profile_id = $2",
    )
    .bind(format!("license-{scenario_id}"))
    .bind(PROFILE_IDS[1])
    .execute(pool)
    .await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let outcome = create_postgres_ready_marking_session(
        &mut connection,
        config,
        &session_command(
            scenario_id,
            &issued.cleartext_api_key,
            "idem-profile",
            "session-profile",
        ),
    )
    .await?;
    let result = snapshot(pool, scenario_id, &[outcome]).await?;
    assert_rejected(&result, REASON_PROFILE_NOT_ENTITLED)?;
    Ok(result)
}

#[cfg(feature = "postgres")]
async fn run_concurrent_idempotency(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "concurrent_idempotency";
    let issued = seed_and_issue(pool, database_url, config, authorization, scenario_id).await?;
    let commands = [
        session_command(
            scenario_id,
            &issued.cleartext_api_key,
            "same-idem",
            "session-a",
        ),
        session_command(
            scenario_id,
            &issued.cleartext_api_key,
            "same-idem",
            "session-b",
        ),
    ];
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for command in commands {
        let database_url = database_url.to_string();
        let config = config.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(
            move || -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
                let runtime = tokio::runtime::Runtime::new()?;
                runtime.block_on(async {
                    let mut connection = sqlx::PgConnection::connect(&database_url).await?;
                    barrier.wait();
                    Ok(
                        create_postgres_ready_marking_session(&mut connection, &config, &command)
                            .await?,
                    )
                })
            },
        ));
    }
    let outcomes = handles
        .into_iter()
        .map(|handle| {
            handle
                .join()
                .map_err(|_| "credential session thread panicked")?
                .map_err(|error| error.to_string().into())
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    let result = snapshot(pool, scenario_id, &outcomes).await?;
    if result.winner_count != 1
        || !result
            .reason_codes
            .iter()
            .any(|reason| reason == REASON_IDEMPOTENCY_CONFLICT)
        || result.session_count != 1
        || result.ready_session_count != 1
    {
        return Err(format!("concurrent idempotency failed: {result:?}").into());
    }
    Ok(result)
}

#[cfg(feature = "postgres")]
async fn run_rotate_revokes_old(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "rotate_revokes_old";
    let issued = seed_and_issue(pool, database_url, config, authorization, scenario_id).await?;
    let rotated_config = CredentialCustodyConfig {
        custody_key_id: config.custody_key_id.clone(),
        active_pepper: PepperMaterial {
            key_id: "kms-key-qa-v2".to_string(),
            version: "qa-v2".to_string(),
            secret: "qa-only-second-secret-at-least-thirty-two-bytes".to_string(),
        },
        retained_peppers: vec![config.active_pepper.clone()],
        provider_readiness: config.provider_readiness.clone(),
    };
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let replacement = rotate_postgres_production_credential(
        &mut connection,
        &rotated_config,
        authorization,
        &RotateProductionCredentialCommand {
            new_credential_id: format!("credential-{scenario_id}-v2"),
            new_api_key_id: format!("api-key-{scenario_id}-v2"),
            previous_credential_id: issued.credential_id,
            actor_token_hash: "authorized-token-hash".to_string(),
            expires_at: Utc::now() + Duration::hours(1),
            audit_event_id: format!("audit-rotate-{scenario_id}"),
        },
    )
    .await?;
    let old = create_postgres_ready_marking_session(
        &mut connection,
        &rotated_config,
        &session_command(
            scenario_id,
            &issued.cleartext_api_key,
            "old-idem",
            "old-session",
        ),
    )
    .await?;
    let new = create_postgres_ready_marking_session(
        &mut connection,
        &rotated_config,
        &session_command(
            scenario_id,
            &replacement.cleartext_api_key,
            "new-idem",
            "new-session",
        ),
    )
    .await?;
    let result = snapshot(pool, scenario_id, &[old, new]).await?;
    let version: String = sqlx::query_scalar(
        "SELECT hash_secret_version FROM ai_sdk_credential_bindings
         WHERE credential_id = $1",
    )
    .bind(format!("credential-{scenario_id}-v2"))
    .fetch_one(pool)
    .await?;
    if result.winner_count != 1
        || !result
            .reason_codes
            .iter()
            .any(|reason| reason == REASON_CREDENTIAL_INACTIVE)
        || result.session_count != 1
        || version != "qa-v2"
    {
        return Err(format!("rotation did not revoke old credential: {result:?}").into());
    }
    Ok(result)
}

#[cfg(feature = "postgres")]
async fn run_revoke_blocks_session(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "revoke_blocks_session";
    let issued = seed_and_issue(pool, database_url, config, authorization, scenario_id).await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    revoke_postgres_production_credential(
        &mut connection,
        config,
        authorization,
        &RevokeProductionCredentialCommand {
            credential_id: issued.credential_id,
            actor_token_hash: "authorized-token-hash".to_string(),
            revoked_reason: "security_test".to_string(),
            audit_event_id: format!("audit-revoke-{scenario_id}"),
        },
    )
    .await?;
    let outcome = create_postgres_ready_marking_session(
        &mut connection,
        config,
        &session_command(
            scenario_id,
            &issued.cleartext_api_key,
            "revoked-idem",
            "revoked-session",
        ),
    )
    .await?;
    let result = snapshot(pool, scenario_id, &[outcome]).await?;
    if result.winner_count != 0
        || !result
            .reason_codes
            .iter()
            .any(|reason| reason == REASON_CREDENTIAL_INACTIVE)
        || result.session_count != 0
    {
        return Err(format!("revoked credential created session: {result:?}").into());
    }
    Ok(result)
}

#[cfg(feature = "postgres")]
async fn seed_and_issue(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
    scenario_id: &str,
) -> Result<
    hiddenshield_feedback_backend::ai_transparency_credential_custody::IssuedProductionCredential,
    Box<dyn std::error::Error>,
> {
    seed_license_and_profiles(pool, scenario_id).await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    Ok(issue_postgres_production_credential(
        &mut connection,
        config,
        authorization,
        &IssueProductionCredentialCommand {
            credential_id: format!("credential-{scenario_id}"),
            api_key_id: format!("api-key-{scenario_id}"),
            license_id: format!("license-{scenario_id}"),
            scopes: vec!["mark:image".to_string()],
            issuer_modes: vec!["hiddenshield_managed".to_string()],
            expires_at: Utc::now() + Duration::hours(1),
            actor_token_hash: "authorized-token-hash".to_string(),
            audit_event_id: format!("audit-issue-{scenario_id}"),
        },
    )
    .await?)
}

#[cfg(feature = "postgres")]
async fn seed_license_and_profiles(
    pool: &sqlx::PgPool,
    scenario_id: &str,
) -> Result<(), sqlx::Error> {
    let license_id = format!("license-{scenario_id}");
    sqlx::query(
        "INSERT INTO ai_transparency_licenses (
            license_id, tenant_id, workspace_id, environment, status, issuer_mode,
            deployment_mode, public_verification_required, metering_plan_id,
            effective_at, expires_at, created_at, updated_at
         ) VALUES ($1,$2,$3,'production','active','hiddenshield_managed','hosted',
            TRUE,'metering-qa',NOW(),NOW() + INTERVAL '1 day',NOW(),NOW())",
    )
    .bind(&license_id)
    .bind(format!("tenant-{scenario_id}"))
    .bind(format!("workspace-{scenario_id}"))
    .execute(pool)
    .await?;
    for profile_id in PROFILE_IDS {
        sqlx::query(
            "INSERT INTO ai_profile_entitlements (
                license_id, profile_id, profile_kind, status, effective_at, expires_at,
                terms_version, approved_by, created_at, updated_at
             ) VALUES ($1,$2,'regulatory','active',NOW(),NOW() + INTERVAL '1 day',
                'terms-v1','owner-audit',NOW(),NOW())",
        )
        .bind(&license_id)
        .bind(profile_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn session_command(
    scenario_id: &str,
    cleartext_api_key: &str,
    idempotency_key: &str,
    session_id: &str,
) -> CreateReadyMarkingSessionCommand {
    CreateReadyMarkingSessionCommand {
        marking_session_id: format!("{session_id}-{scenario_id}"),
        cleartext_api_key: cleartext_api_key.to_string(),
        tenant_id: format!("tenant-{scenario_id}"),
        workspace_id: format!("workspace-{scenario_id}"),
        environment: "production".to_string(),
        idempotency_key: idempotency_key.to_string(),
        requested_profile_ids: PROFILE_IDS.iter().map(|value| value.to_string()).collect(),
        claim_type: "ai_generated".to_string(),
        provider_content_id: Some(format!("provider-{scenario_id}")),
        expires_at: Utc::now() + Duration::minutes(30),
        audit_event_id: format!("audit-session-{session_id}-{scenario_id}"),
    }
}

#[cfg(feature = "postgres")]
async fn snapshot(
    pool: &sqlx::PgPool,
    scenario_id: &'static str,
    outcomes: &[CreateReadyMarkingSessionOutcome],
) -> Result<ScenarioResult, sqlx::Error> {
    let counts = sqlx::query(
        "SELECT
            (SELECT COUNT(*) FROM ai_sdk_credential_bindings WHERE license_id = $1) credentials,
            (SELECT COUNT(*) FROM ai_marking_sessions WHERE license_id = $1) sessions,
            (SELECT COUNT(*) FROM ai_marking_sessions WHERE license_id = $1 AND status = 'ready_to_confirm') ready,
            (SELECT COUNT(*) FROM ai_runtime_credential_audit_events WHERE license_id = $1) audit,
            (SELECT COUNT(*) FROM ai_sdk_credential_bindings WHERE license_id = $1 AND last_used_at IS NOT NULL) used",
    )
    .bind(format!("license-{scenario_id}"))
    .fetch_one(pool)
    .await?;
    Ok(ScenarioResult {
        scenario_id,
        winner_count: outcomes.iter().filter(|outcome| outcome.succeeded).count(),
        reason_codes: outcomes
            .iter()
            .filter_map(|outcome| outcome.reason_code.clone())
            .collect(),
        credential_count: counts.get("credentials"),
        session_count: counts.get("sessions"),
        ready_session_count: counts.get("ready"),
        audit_count: counts.get("audit"),
        credential_last_used_count: counts.get("used"),
    })
}

#[cfg(feature = "postgres")]
fn assert_success(result: &ScenarioResult) -> Result<(), Box<dyn std::error::Error>> {
    if result.winner_count != 1
        || result.credential_count != 1
        || result.session_count != 1
        || result.ready_session_count != 1
        || result.audit_count != 2
        || result.credential_last_used_count != 1
    {
        return Err(format!("valid credential did not create ready session: {result:?}").into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn assert_rejected(
    result: &ScenarioResult,
    expected_reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if result.winner_count != 0
        || !result
            .reason_codes
            .iter()
            .any(|reason| reason == expected_reason)
        || result.session_count != 0
        || result.ready_session_count != 0
        || result.audit_count != 1
        || result.credential_last_used_count != 0
    {
        return Err(format!("invalid credential left session side effects: {result:?}").into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_cleartext_not_persisted(
    pool: &sqlx::PgPool,
    cleartext: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let found: bool = sqlx::query_scalar(
        "SELECT EXISTS (
            SELECT 1 FROM ai_sdk_credential_bindings
            WHERE row_to_json(ai_sdk_credential_bindings)::text LIKE '%' || $1 || '%'
        )",
    )
    .bind(cleartext)
    .fetch_one(pool)
    .await?;
    if found {
        return Err("cleartext production credential was persisted".into());
    }
    Ok(())
}

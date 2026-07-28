#[cfg(feature = "postgres")]
use std::{io::Cursor, path::PathBuf, process::Command, sync::Arc};

#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::{
    ai_transparency_credential_custody::{CredentialCustodyConfig, PepperMaterial},
    ai_transparency_platform_api::{
        build_ai_transparency_platform_router, AiTransparencyPlatformApiState,
    },
    ai_transparency_production_provider::{
        ProductionCustodyOperation, ProductionProviderDeploymentError, ProductionProviderReadiness,
    },
    ai_transparency_public_resolver::{
        build_ai_transparency_public_resolver_router, AiTransparencyPublicResolverState,
    },
    database::{
        POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL, POSTGRES_P20_AI_TRANSPARENCY_PLATFORM_API_UP_SQL,
        POSTGRES_P21_AI_TRANSPARENCY_PUBLIC_RESOLVER_UP_SQL,
        POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL,
        POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
        POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_UP_SQL,
        POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_UP_SQL,
        POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_UP_SQL,
    },
};
#[cfg(feature = "postgres")]
use hmac::{Hmac, Mac};
#[cfg(feature = "postgres")]
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
#[cfg(feature = "postgres")]
use serde_json::json;
#[cfg(feature = "postgres")]
use sha2::Sha256;

#[cfg(feature = "postgres")]
const LICENSE_ID: &str = "license-platform-api-e2e";
#[cfg(feature = "postgres")]
const TENANT_ID: &str = "tenant-platform-api-e2e";
#[cfg(feature = "postgres")]
const WORKSPACE_ID: &str = "workspace-platform-api-e2e";
#[cfg(feature = "postgres")]
const API_KEY: &str = "hsai_live_platform_api_e2e_credential_material_2026";
#[cfg(feature = "postgres")]
const PEPPER_SECRET: &str = "platform-api-e2e-pepper-secret-at-least-32-bytes";
#[cfg(feature = "postgres")]
const TOKEN_SECRET: &str = "platform-api-e2e-confirmation-token-secret-2026";
#[cfg(feature = "postgres")]
const REGULATORY_PROFILE: &str = "cn_aigc_label_2025_image_export_v1";
#[cfg(feature = "postgres")]
const TECHNICAL_PROFILE: &str = "hiddenshield_v3_image_anchor_v1";

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
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| "missing disposable PostgreSQL URL for platform API QA")?;
    if !safe_smoke_url(&database_url) {
        return Err(
            "refusing platform API QA against non-disposable URL; require localhost/127.0.0.1 and hiddenshield_migrate_smoke"
                .into(),
        );
    }
    let pool = sqlx::PgPool::connect(&database_url).await?;
    reset_schema(&pool).await?;
    seed_contract_state(&pool).await?;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    let app = build_ai_transparency_platform_router(AiTransparencyPlatformApiState {
        pool: pool.clone(),
        custody: custody_config(),
        confirmation_token_secret: Arc::from(TOKEN_SECRET),
        internal_verification_base_url: Arc::from(
            "https://internal.hiddenshield.local/v1/manifests",
        ),
    })
    .merge(build_ai_transparency_public_resolver_router(
        AiTransparencyPublicResolverState { pool: pool.clone() },
    ));
    let server = tokio::spawn(async move { axum::serve(listener, app).await });

    let image_path = write_image_fixture()?;
    let repository_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .ok_or("backend must have repository parent")?
        .to_path_buf();
    let node_status = tokio::task::spawn_blocking(move || {
        Command::new("node")
            .arg("scripts/qa-ai-transparency-platform-api-e2e.mjs")
            .current_dir(repository_root)
            .env(
                "HIDDENSHIELD_AI_PLATFORM_QA_BASE_URL",
                format!("http://{address}"),
            )
            .env("HIDDENSHIELD_AI_PLATFORM_QA_CREDENTIAL", API_KEY)
            .env("HIDDENSHIELD_AI_PLATFORM_QA_LICENSE_ID", LICENSE_ID)
            .env("HIDDENSHIELD_AI_PLATFORM_QA_TENANT_ID", TENANT_ID)
            .env("HIDDENSHIELD_AI_PLATFORM_QA_WORKSPACE_ID", WORKSPACE_ID)
            .env("HIDDENSHIELD_AI_PLATFORM_QA_IMAGE_PATH", image_path)
            .status()
    })
    .await??;
    if !node_status.success() {
        server.abort();
        return Err("SDK → API facade E2E process failed".into());
    }

    let manifest_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM ai_transparency_manifests")
        .fetch_one(&pool)
        .await?;
    let committed_ledger_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_marking_ledger
         WHERE metering_unit = 'confirmed_marked_image' AND ledger_status = 'committed'",
    )
    .fetch_one(&pool)
    .await?;
    let confirmed_session_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_marking_sessions WHERE status = 'confirmed'")
            .fetch_one(&pool)
            .await?;
    let replay_audit_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM ai_platform_api_audit_events
         WHERE operation = 'confirm_image' AND outcome = 'replayed'",
    )
    .fetch_one(&pool)
    .await?;
    let platform_audit_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM ai_platform_api_audit_events")
            .fetch_one(&pool)
            .await?;
    if manifest_count != 1
        || committed_ledger_count != 1
        || confirmed_session_count != 1
        || replay_audit_count != 1
        || platform_audit_count != 5
    {
        server.abort();
        return Err(format!(
            "unexpected E2E database state: manifests={manifest_count}, ledger={committed_ledger_count}, confirmed={confirmed_session_count}, replay={replay_audit_count}, platformAudit={platform_audit_count}"
        )
        .into());
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "scenarioId": "sdk_api_facade_postgres_e2e",
            "manifestCount": manifest_count,
            "committedConfirmedMarkedImageCount": committed_ledger_count,
            "confirmedSessionCount": confirmed_session_count,
            "replayAuditCount": replay_audit_count,
            "platformAuditCount": platform_audit_count,
            "status": "passed"
        }))?
    );
    server.abort();
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("ai_transparency_platform_api_qa requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
fn custody_config() -> CredentialCustodyConfig {
    CredentialCustodyConfig {
        custody_key_id: "platform-api-e2e-kms".to_string(),
        active_pepper: PepperMaterial {
            key_id: "platform-api-e2e-kms".to_string(),
            version: "platform-api-e2e-v1".to_string(),
            secret: PEPPER_SECRET.to_string(),
        },
        retained_peppers: Vec::new(),
        provider_readiness: Arc::new(QaReadyProvider),
    }
}

#[cfg(feature = "postgres")]
fn safe_smoke_url(database_url: &str) -> bool {
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
        POSTGRES_P20_AI_TRANSPARENCY_PLATFORM_API_UP_SQL,
        POSTGRES_P21_AI_TRANSPARENCY_PUBLIC_RESOLVER_UP_SQL,
    ] {
        sqlx::raw_sql(migration).execute(pool).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn seed_contract_state(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_transparency_licenses (
            license_id, tenant_id, workspace_id, environment, status, issuer_mode,
            deployment_mode, public_verification_required, metering_plan_id,
            effective_at, expires_at, created_at, updated_at
         ) VALUES ($1,$2,$3,'production','active','hiddenshield_managed','hosted',
            TRUE,'platform-api-e2e',NOW(),NOW() + INTERVAL '1 day',NOW(),NOW())",
    )
    .bind(LICENSE_ID)
    .bind(TENANT_ID)
    .bind(WORKSPACE_ID)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO ai_transparency_actor_role_snapshots (
            actor_role_snapshot_id, actor_id, actor_type, role, tenant_id, workspace_id,
            environment, role_binding_id, role_binding_version, source_identity_system,
            authentication_level, captured_at, source_expires_at, snapshot_sha256
         ) VALUES (
            'snapshot-platform-api-e2e', 'actor-platform-api-e2e', 'human',
            'ai_transparency_requester', $1, $2, 'production', 'binding-platform-api-e2e',
            1, 'hiddenshield_internal_iam', 'strong', NOW(), NOW() + INTERVAL '1 day',
            repeat('1', 64)
         )",
    )
    .bind(TENANT_ID)
    .bind(WORKSPACE_ID)
    .execute(pool)
    .await?;
    for (profile_id, profile_kind, review_column, suffix) in [
        (
            REGULATORY_PROFILE,
            "regulatory",
            "legal-ref-platform-api-e2e",
            "reg",
        ),
        (
            TECHNICAL_PROFILE,
            "technical",
            "security-ref-platform-api-e2e",
            "tech",
        ),
    ] {
        let change_request_id = format!("change-platform-api-e2e-{suffix}");
        let version_id = format!("entitlement-version-platform-api-e2e-{suffix}");
        sqlx::query(
            "INSERT INTO ai_transparency_change_requests (
                change_request_id, operation, target_type, target_id, target_scope_key,
                tenant_id, workspace_id, environment, expected_target_version,
                desired_next_version, desired_state_json, request_reason, contract_reference,
                legal_review_reference, security_review_reference, requester_snapshot_id,
                request_digest_version, request_digest, idempotency_key, status, expires_at,
                evidence_quality, production_eligibility, created_at, updated_at
             ) VALUES (
                $1, 'grant_profile_entitlement', 'profile_entitlement', $2, $3,
                $4, $5, 'production', NULL, 1, '{}'::jsonb, 'platform API E2E',
                'contract-platform-api-e2e', $6, $7, 'snapshot-platform-api-e2e',
                'hs-ai-change-request-digest-v1', $8, $9, 'succeeded',
                NOW() + INTERVAL '1 day', 'native_four_eyes', TRUE, NOW(), NOW()
             )",
        )
        .bind(&change_request_id)
        .bind(profile_id)
        .bind(format!("{LICENSE_ID}:{profile_id}"))
        .bind(TENANT_ID)
        .bind(WORKSPACE_ID)
        .bind((profile_kind == "regulatory").then_some(review_column))
        .bind((profile_kind == "technical").then_some(review_column))
        .bind(if suffix == "reg" {
            "2".repeat(64)
        } else {
            "3".repeat(64)
        })
        .bind(format!("idem-platform-api-e2e-{suffix}"))
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO ai_profile_entitlement_versions (
                profile_entitlement_version_id, license_id, profile_id, version,
                previous_version_id, profile_kind, status, effective_at, expires_at,
                terms_version, legal_review_reference, security_review_reference,
                source_change_request_id, created_at
             ) VALUES ($1,$2,$3,1,NULL,$4,'active',NOW(),NOW() + INTERVAL '1 day',
                'terms-v1',$5,$6,$7,NOW())",
        )
        .bind(&version_id)
        .bind(LICENSE_ID)
        .bind(profile_id)
        .bind(profile_kind)
        .bind((profile_kind == "regulatory").then_some(review_column))
        .bind((profile_kind == "technical").then_some(review_column))
        .bind(&change_request_id)
        .execute(pool)
        .await?;
        sqlx::query(
            "INSERT INTO ai_profile_entitlements (
                license_id, profile_id, profile_kind, status, effective_at, expires_at,
                terms_version, approved_by, created_at, updated_at, current_version_id,
                current_version, projection_updated_at
             ) VALUES ($1,$2,$3,'active',NOW(),NOW() + INTERVAL '1 day',
                'terms-v1','platform-api-e2e',NOW(),NOW(),$4,1,NOW())",
        )
        .bind(LICENSE_ID)
        .bind(profile_id)
        .bind(profile_kind)
        .bind(&version_id)
        .execute(pool)
        .await?;
    }
    let mut mac =
        Hmac::<Sha256>::new_from_slice(PEPPER_SECRET.as_bytes()).expect("QA pepper accepts HMAC");
    mac.update(API_KEY.as_bytes());
    let key_hash = format!(
        "hmac-sha256:v1:platform-api-e2e-v1:{}",
        hex_lower(&mac.finalize().into_bytes())
    );
    sqlx::query(
        "INSERT INTO ai_sdk_credential_bindings (
            credential_id, license_id, api_key_id, scopes_json, status, expires_at, created_at,
            key_prefix, key_hash, hash_secret_version, environment, issuer_modes_json,
            custody_key_id, issued_at
         ) VALUES (
            'credential-platform-api-e2e',$1,'api-key-platform-api-e2e',
            '[\"mark:image\"]'::jsonb,'active',NOW() + INTERVAL '1 day',NOW(),
            $2,$3,'platform-api-e2e-v1','production',
            '[\"hiddenshield_managed\"]'::jsonb,'platform-api-e2e-kms',NOW()
         )",
    )
    .bind(LICENSE_ID)
    .bind(API_KEY.chars().take(22).collect::<String>())
    .bind(key_hash)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
fn write_image_fixture() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let image = ImageBuffer::from_fn(512, 512, |x, y| {
        Rgba([
            ((x * 13 + y * 3) % 255) as u8,
            ((x * 5 + y * 17) % 255) as u8,
            ((x * 19 + y * 7) % 255) as u8,
            255,
        ])
    });
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image).write_to(&mut cursor, ImageFormat::Png)?;
    let path = std::env::temp_dir().join("hiddenshield-ai-platform-api-e2e.png");
    std::fs::write(&path, cursor.into_inner())?;
    Ok(path)
}

#[cfg(feature = "postgres")]
fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

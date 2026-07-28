#[cfg(feature = "postgres")]
use std::{fs, io::Cursor, path::Path, sync::Arc};

#[cfg(feature = "postgres")]
use chrono::{Duration, Utc};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_credential_custody::{
    create_postgres_ready_marking_session, issue_postgres_production_credential,
    CreateReadyMarkingSessionCommand, CredentialCustodyConfig, CustodyAuthorizationDecision,
    CustodyAuthorizationInput, IssueProductionCredentialCommand, PepperMaterial,
    ProductionCredentialCustodyAuthorization,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_image_marking_executor::{
    execute_postgres_internal_image_marking, InternalImageMarkingCommand,
    REASON_EXECUTOR_SESSION_INVALID,
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
use image::{DynamicImage, ImageBuffer, ImageFormat, Rgba};
#[cfg(feature = "postgres")]
use serde::Serialize;
#[cfg(feature = "postgres")]
use serde_json::json;
#[cfg(feature = "postgres")]
use sha2::{Digest, Sha256};
#[cfg(feature = "postgres")]
use sqlx::{Connection, Row};
#[cfg(feature = "postgres")]
use watermark_core::{MediaInput, WatermarkService};

#[cfg(feature = "postgres")]
const PROFILE_IDS: [&str; 2] = [
    "hiddenshield_v3_image_anchor_v1",
    "cn_aigc_label_2025_image_export_v1",
];

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
struct AllowCustodyAuthorization;

#[cfg(feature = "postgres")]
impl ProductionCredentialCustodyAuthorization for AllowCustodyAuthorization {
    fn authorize(&self, input: &CustodyAuthorizationInput<'_>) -> CustodyAuthorizationDecision {
        CustodyAuthorizationDecision {
            authorized: input.actor_token_hash == "executor-qa-token"
                && input.custody_key_id == "executor-qa-kms"
                && input.operation == "issue_production_credential",
            reason_code: None,
            receipt_id: Some(format!("receipt-{}", input.license_id)),
        }
    }
}

#[cfg(feature = "postgres")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScenarioResult {
    scenario_id: &'static str,
    succeeded: bool,
    session_status: String,
    manifest_count: i64,
    evidence_count: i64,
    marker_count: i64,
    receipt_count: i64,
    committed_ledger_count: i64,
    audit_count: i64,
    protected_image_sha256: Option<String>,
}

#[cfg(feature = "postgres")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| "missing disposable PostgreSQL URL for image marking executor QA")?;
    if !safe_smoke_url(&database_url) {
        return Err(
            "refusing image marking executor QA against non-disposable URL; require localhost/127.0.0.1 and hiddenshield_migrate_smoke"
                .into(),
        );
    }
    let pool = sqlx::PgPool::connect(&database_url).await?;
    reset_schema(&pool).await?;
    let config = custody_config();
    let authorization = AllowCustodyAuthorization;
    let success = run_success(&pool, &database_url, &config, &authorization).await?;
    let invalid = run_invalid_session(&pool, &database_url).await?;
    println!("{}", serde_json::to_string_pretty(&[success, invalid])?);
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("ai_transparency_image_marking_executor_qa requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
fn custody_config() -> CredentialCustodyConfig {
    CredentialCustodyConfig {
        custody_key_id: "executor-qa-kms".to_string(),
        active_pepper: PepperMaterial {
            key_id: "executor-qa-kms".to_string(),
            version: "executor-qa-v1".to_string(),
            secret: "executor-qa-secret-at-least-thirty-two-bytes".to_string(),
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
    ] {
        sqlx::raw_sql(migration).execute(pool).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_success(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "executor_success";
    let api_key =
        create_ready_session(pool, database_url, config, authorization, scenario_id).await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let watermark_uid = "HS-01234567-89ABCDEF-01234567-89ABCDEF";
    let outcome = execute_postgres_internal_image_marking(
        &mut connection,
        &executor_command(scenario_id, watermark_uid, image_fixture()),
    )
    .await?;
    if !outcome.succeeded || outcome.explicit_label_plans.len() != 1 {
        return Err(format!("executor did not succeed: {:?}", outcome.reason_code).into());
    }
    let protected = outcome
        .protected_image_bytes
        .ok_or("successful executor returned no protected image")?;
    let decoded = WatermarkService::extract(MediaInput::ImageBytes {
        bytes: protected.clone(),
    })
    .map_err(|error| {
        format!("executor protected image did not re-read through watermark-core: {error}")
    })?;
    if decoded.watermark_uid() != watermark_uid || !decoded.is_v3_minimal_anchor() {
        return Err("executor returned a mismatched watermark-core V3 image".into());
    }
    if let Ok(fixture_dir) = std::env::var("HIDDENSHIELD_EXECUTOR_FIXTURE_DIR") {
        write_cross_end_fixture(
            Path::new(&fixture_dir),
            &protected,
            watermark_uid,
            outcome
                .protected_image_sha256
                .as_deref()
                .ok_or("successful executor returned no protected image digest")?,
        )?;
    }
    if api_key.is_empty() {
        return Err("custody issued an empty QA key".into());
    }
    let result = snapshot(pool, scenario_id, true, outcome.protected_image_sha256).await?;
    if result.session_status != "confirmed"
        || result.manifest_count != 1
        || result.evidence_count != 1
        || result.marker_count != 1
        || result.receipt_count != 1
        || result.committed_ledger_count != 1
        || result.audit_count != 1
    {
        return Err(format!("executor success persistence mismatch: {result:?}").into());
    }
    Ok(result)
}

#[cfg(feature = "postgres")]
async fn run_invalid_session(
    pool: &sqlx::PgPool,
    database_url: &str,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "executor_invalid_session";
    seed_license(pool, scenario_id).await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let outcome = execute_postgres_internal_image_marking(
        &mut connection,
        &executor_command(
            scenario_id,
            "HS-01234567-89ABCDEF-01234567-89ABCDEF",
            image_fixture(),
        ),
    )
    .await?;
    if outcome.succeeded
        || outcome.reason_code.as_deref() != Some(REASON_EXECUTOR_SESSION_INVALID)
        || outcome.protected_image_bytes.is_some()
    {
        return Err("invalid session did not fail closed".into());
    }
    let result = snapshot(pool, scenario_id, false, None).await?;
    if result.session_status != "missing"
        || result.manifest_count != 0
        || result.evidence_count != 0
        || result.marker_count != 0
        || result.receipt_count != 0
        || result.committed_ledger_count != 0
        || result.audit_count != 0
    {
        return Err(format!("invalid session left confirm writes: {result:?}").into());
    }
    Ok(result)
}

#[cfg(feature = "postgres")]
async fn create_ready_session(
    pool: &sqlx::PgPool,
    database_url: &str,
    config: &CredentialCustodyConfig,
    authorization: &dyn ProductionCredentialCustodyAuthorization,
    scenario_id: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    seed_license(pool, scenario_id).await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let issued = issue_postgres_production_credential(
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
            actor_token_hash: "executor-qa-token".to_string(),
            audit_event_id: format!("audit-issue-{scenario_id}"),
        },
    )
    .await?;
    let created = create_postgres_ready_marking_session(
        &mut connection,
        config,
        &CreateReadyMarkingSessionCommand {
            marking_session_id: format!("session-{scenario_id}"),
            cleartext_api_key: issued.cleartext_api_key.clone(),
            tenant_id: format!("tenant-{scenario_id}"),
            workspace_id: format!("workspace-{scenario_id}"),
            environment: "production".to_string(),
            idempotency_key: format!("idem-{scenario_id}"),
            requested_profile_ids: PROFILE_IDS.iter().map(|value| value.to_string()).collect(),
            claim_type: "ai_generated".to_string(),
            provider_content_id: Some(format!("provider-content-{scenario_id}")),
            expires_at: Utc::now() + Duration::minutes(30),
            audit_event_id: format!("audit-session-{scenario_id}"),
        },
    )
    .await?;
    if !created.succeeded {
        return Err(format!("could not create ready session: {:?}", created.reason_code).into());
    }
    Ok(issued.cleartext_api_key)
}

#[cfg(feature = "postgres")]
async fn seed_license(pool: &sqlx::PgPool, scenario_id: &str) -> Result<(), sqlx::Error> {
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
                'terms-v1','executor-qa',NOW(),NOW())",
        )
        .bind(&license_id)
        .bind(profile_id)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn executor_command(
    scenario_id: &str,
    watermark_uid: &str,
    source_image_bytes: Vec<u8>,
) -> InternalImageMarkingCommand {
    InternalImageMarkingCommand {
        marking_session_id: format!("session-{scenario_id}"),
        execution_id: format!("executor-{scenario_id}"),
        watermark_uid: watermark_uid.to_string(),
        source_image_bytes,
        provider_id: "internal-qa-platform".to_string(),
        system_name: "Internal QA Image Platform".to_string(),
        system_version: "2026.07".to_string(),
        model_id: Some("internal-qa-model".to_string()),
        model_version: Some("1".to_string()),
        generation_mode: "text_to_image".to_string(),
        generated_at: Utc::now(),
        operations: json!([]),
        parent_subjects: json!([]),
    }
}

#[cfg(feature = "postgres")]
fn image_fixture() -> Vec<u8> {
    let image = ImageBuffer::from_fn(512, 512, |x, y| {
        Rgba([
            ((x * 13 + y * 3) % 255) as u8,
            ((x * 5 + y * 17) % 255) as u8,
            ((x * 19 + y * 7) % 255) as u8,
            255,
        ])
    });
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .expect("encode deterministic executor QA PNG");
    cursor.into_inner()
}

#[cfg(feature = "postgres")]
fn write_cross_end_fixture(
    fixture_dir: &Path,
    protected_image: &[u8],
    watermark_uid: &str,
    protected_image_sha256: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(fixture_dir)?;
    let with_metadata = append_png_text_chunk(
        protected_image,
        "ai_transparency_fixture",
        "internal_executor_v1",
    )?;
    let stripped = strip_png_metadata(&with_metadata)?;
    let with_external_metadata = append_png_text_chunk(
        &append_png_text_chunk(
            protected_image,
            "external_provenance_fixture",
            "untrusted_test_metadata_v1",
        )?,
        "external_metadata_namespace",
        "example.invalid/ai-provenance",
    )?;
    let external_metadata_stripped = strip_png_metadata(&with_external_metadata)?;
    for bytes in [
        protected_image,
        with_metadata.as_slice(),
        stripped.as_slice(),
        with_external_metadata.as_slice(),
        external_metadata_stripped.as_slice(),
    ] {
        let decoded = WatermarkService::extract(MediaInput::ImageBytes {
            bytes: bytes.to_vec(),
        })?;
        if decoded.watermark_uid() != watermark_uid
            || !decoded.is_v3_minimal_anchor()
            || decoded.payload_auth_status() != "verified"
        {
            return Err("cross-end fixture did not preserve the expected V3 anchor".into());
        }
    }
    fs::write(
        fixture_dir.join("platform-executor-v3.png"),
        protected_image,
    )?;
    fs::write(
        fixture_dir.join("platform-executor-v3-with-metadata.png"),
        &with_metadata,
    )?;
    fs::write(
        fixture_dir.join("platform-executor-v3-metadata-stripped.png"),
        &stripped,
    )?;
    fs::write(
        fixture_dir.join("platform-executor-v3-with-external-metadata.png"),
        &with_external_metadata,
    )?;
    fs::write(
        fixture_dir.join("platform-executor-v3-external-metadata-stripped.png"),
        &external_metadata_stripped,
    )?;
    let manifest = json!({
        "schemaVersion": "hs-ai-platform-executor-cross-end-fixture-v1",
        "mediaType": "image/png",
        "watermarkUid": watermark_uid,
        "payloadProtocolVersion": 3,
        "payloadBytesLength": 39,
        "payloadAuthStatus": "verified",
        "legalConclusion": false,
        "files": {
            "executorOutput": {
                "path": "platform-executor-v3.png",
                "sha256": protected_image_sha256
            },
            "withMetadata": {
                "path": "platform-executor-v3-with-metadata.png",
                "sha256": sha256_hex(&with_metadata)
            },
            "metadataStripped": {
                "path": "platform-executor-v3-metadata-stripped.png",
                "sha256": sha256_hex(&stripped)
            },
            "withExternalMetadata": {
                "path": "platform-executor-v3-with-external-metadata.png",
                "sha256": sha256_hex(&with_external_metadata),
                "metadataKeys": [
                    "external_provenance_fixture",
                    "external_metadata_namespace"
                ]
            },
            "externalMetadataStripped": {
                "path": "platform-executor-v3-external-metadata-stripped.png",
                "sha256": sha256_hex(&external_metadata_stripped)
            }
        }
    });
    fs::write(
        fixture_dir.join("manifest.json"),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok(())
}

#[cfg(feature = "postgres")]
fn append_png_text_chunk(
    png: &[u8],
    key: &str,
    value: &str,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
    if !png.starts_with(PNG_SIGNATURE) {
        return Err("executor output was not a PNG".into());
    }
    let mut offset = PNG_SIGNATURE.len();
    let mut iend_offset = None;
    while offset + 12 <= png.len() {
        let length = u32::from_be_bytes(png[offset..offset + 4].try_into()?) as usize;
        let chunk_end = offset + 12 + length;
        if chunk_end > png.len() {
            return Err("executor output PNG chunk was truncated".into());
        }
        if &png[offset + 4..offset + 8] == b"IEND" {
            iend_offset = Some(offset);
            break;
        }
        offset = chunk_end;
    }
    let iend_offset = iend_offset.ok_or("executor output PNG has no IEND")?;
    let mut data = Vec::new();
    data.extend_from_slice(key.as_bytes());
    data.push(0);
    data.extend_from_slice(value.as_bytes());
    let mut crc = crc32fast::Hasher::new();
    crc.update(b"tEXt");
    crc.update(&data);
    let mut output = Vec::with_capacity(png.len() + data.len() + 12);
    output.extend_from_slice(&png[..iend_offset]);
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(b"tEXt");
    output.extend_from_slice(&data);
    output.extend_from_slice(&crc.finalize().to_be_bytes());
    output.extend_from_slice(&png[iend_offset..]);
    Ok(output)
}

#[cfg(feature = "postgres")]
fn strip_png_metadata(png: &[u8]) -> Result<Vec<u8>, image::ImageError> {
    let image = image::load_from_memory(png)?;
    let mut cursor = Cursor::new(Vec::new());
    image.write_to(&mut cursor, ImageFormat::Png)?;
    Ok(cursor.into_inner())
}

#[cfg(feature = "postgres")]
fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(feature = "postgres")]
async fn snapshot(
    pool: &sqlx::PgPool,
    scenario_id: &'static str,
    succeeded: bool,
    protected_image_sha256: Option<String>,
) -> Result<ScenarioResult, sqlx::Error> {
    let session_id = format!("session-{scenario_id}");
    let session_status = sqlx::query_scalar::<_, String>(
        "SELECT status FROM ai_marking_sessions WHERE marking_session_id = $1",
    )
    .bind(&session_id)
    .fetch_optional(pool)
    .await?
    .unwrap_or_else(|| "missing".to_string());
    let counts = sqlx::query(
        "SELECT
            (SELECT COUNT(*) FROM ai_transparency_manifests WHERE marking_session_id = $1) manifests,
            (SELECT COUNT(*) FROM ai_claim_evidence WHERE transparency_manifest_id = (
                SELECT transparency_manifest_id FROM ai_transparency_manifests WHERE marking_session_id = $1
            )) evidence,
            (SELECT COUNT(*) FROM ai_marker_bindings WHERE transparency_manifest_id = (
                SELECT transparency_manifest_id FROM ai_transparency_manifests WHERE marking_session_id = $1
            )) markers,
            (SELECT COUNT(*) FROM ai_explicit_label_receipts WHERE transparency_manifest_id = (
                SELECT transparency_manifest_id FROM ai_transparency_manifests WHERE marking_session_id = $1
            )) receipts,
            (SELECT COUNT(*) FROM ai_marking_ledger WHERE marking_session_id = $1
                AND ledger_status = 'committed') committed_ledger,
            (SELECT COUNT(*) FROM ai_marking_confirm_audit_events WHERE marking_session_id = $1) audit",
    )
    .bind(&session_id)
    .fetch_one(pool)
    .await?;
    Ok(ScenarioResult {
        scenario_id,
        succeeded,
        session_status,
        manifest_count: counts.get("manifests"),
        evidence_count: counts.get("evidence"),
        marker_count: counts.get("markers"),
        receipt_count: counts.get("receipts"),
        committed_ledger_count: counts.get("committed_ledger"),
        audit_count: counts.get("audit"),
        protected_image_sha256,
    })
}

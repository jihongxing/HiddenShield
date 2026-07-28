#[cfg(feature = "postgres")]
use std::sync::{Arc, Barrier};
#[cfg(feature = "postgres")]
use std::thread;

#[cfg(feature = "postgres")]
use chrono::Utc;
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_confirm_command::{
    execute_postgres_confirm_marking_command, ConfirmEvidence, ConfirmExplicitLabelReceipt,
    ConfirmFailureInjection, ConfirmMarker, ConfirmMarkingCommand, ConfirmMarkingOutcome,
    REASON_AUDIT_WRITE_FAILED, REASON_CONFIRMATION_CONFLICT, REASON_LEDGER_WRITE_FAILED,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::database::{
    POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL, POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL,
    POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
    POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_UP_SQL,
};
#[cfg(feature = "postgres")]
use serde::Serialize;
#[cfg(feature = "postgres")]
use serde_json::json;
#[cfg(feature = "postgres")]
use sqlx::{Connection, Row};

#[cfg(feature = "postgres")]
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScenarioResult {
    scenario_id: &'static str,
    winner_count: usize,
    reason_codes: Vec<String>,
    session_status: String,
    manifest_count: i64,
    evidence_count: i64,
    marker_count: i64,
    receipt_count: i64,
    ledger_count: i64,
    committed_ledger_count: i64,
    audit_count: i64,
}

#[cfg(feature = "postgres")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| "missing disposable PostgreSQL URL for confirm concurrency QA")?;
    if !is_safe_smoke_url(&database_url) {
        return Err(
            "refusing confirm QA against non-disposable URL; require localhost/127.0.0.1 and hiddenshield_migrate_smoke"
                .into(),
        );
    }
    let pool = sqlx::PgPool::connect(&database_url).await?;
    reset_schema(&pool).await?;

    let mut results = Vec::new();
    results.push(run_concurrent_confirm(&pool, &database_url).await?);
    results.push(run_duplicate_confirm(&pool, &database_url).await?);
    results.push(
        run_failure(
            &pool,
            &database_url,
            "ledger_failure_rollback",
            ConfirmFailureInjection::Ledger,
            REASON_LEDGER_WRITE_FAILED,
        )
        .await?,
    );
    results.push(
        run_failure(
            &pool,
            &database_url,
            "audit_failure_rollback",
            ConfirmFailureInjection::Audit,
            REASON_AUDIT_WRITE_FAILED,
        )
        .await?,
    );

    println!("{}", serde_json::to_string_pretty(&results)?);
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("ai_transparency_confirm_concurrency_qa requires --features postgres");
    std::process::exit(2);
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
    ] {
        sqlx::raw_sql(migration).execute(pool).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn run_concurrent_confirm(
    pool: &sqlx::PgPool,
    database_url: &str,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "concurrent_confirm_one_winner";
    seed_session(pool, scenario_id).await?;
    let commands = [
        command(scenario_id, "a", ConfirmFailureInjection::None),
        command(scenario_id, "b", ConfirmFailureInjection::None),
    ];
    let outcomes = run_concurrently(database_url, commands)?;
    let result = snapshot(pool, scenario_id, &outcomes).await?;
    assert_result(&result, 1, Some(REASON_CONFIRMATION_CONFLICT), true)?;
    Ok(result)
}

#[cfg(feature = "postgres")]
async fn run_duplicate_confirm(
    pool: &sqlx::PgPool,
    database_url: &str,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    let scenario_id = "duplicate_confirm";
    seed_session(pool, scenario_id).await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let command = command(scenario_id, "same", ConfirmFailureInjection::None);
    let first = execute_postgres_confirm_marking_command(&mut connection, &command).await?;
    let second = execute_postgres_confirm_marking_command(&mut connection, &command).await?;
    let result = snapshot(pool, scenario_id, &[first, second]).await?;
    assert_result(&result, 1, Some(REASON_CONFIRMATION_CONFLICT), true)?;
    Ok(result)
}

#[cfg(feature = "postgres")]
async fn run_failure(
    pool: &sqlx::PgPool,
    database_url: &str,
    scenario_id: &'static str,
    injection: ConfirmFailureInjection,
    expected_reason: &str,
) -> Result<ScenarioResult, Box<dyn std::error::Error>> {
    seed_session(pool, scenario_id).await?;
    let mut connection = sqlx::PgConnection::connect(database_url).await?;
    let outcome = execute_postgres_confirm_marking_command(
        &mut connection,
        &command(scenario_id, "x", injection),
    )
    .await?;
    let result = snapshot(pool, scenario_id, &[outcome]).await?;
    assert_result(&result, 0, Some(expected_reason), false)?;
    Ok(result)
}

#[cfg(feature = "postgres")]
fn run_concurrently(
    database_url: &str,
    commands: [ConfirmMarkingCommand; 2],
) -> Result<Vec<ConfirmMarkingOutcome>, Box<dyn std::error::Error>> {
    let barrier = Arc::new(Barrier::new(2));
    let mut handles = Vec::new();
    for command in commands {
        let database_url = database_url.to_string();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(
            move || -> Result<_, Box<dyn std::error::Error + Send + Sync>> {
                let runtime = tokio::runtime::Runtime::new()?;
                runtime.block_on(async {
                    let mut connection = sqlx::PgConnection::connect(&database_url).await?;
                    barrier.wait();
                    Ok(execute_postgres_confirm_marking_command(&mut connection, &command).await?)
                })
            },
        ));
    }
    handles
        .into_iter()
        .map(|handle| {
            handle
                .join()
                .map_err(|_| "PostgreSQL confirm thread panicked")?
                .map_err(|error| error.to_string().into())
        })
        .collect()
}

#[cfg(feature = "postgres")]
async fn seed_session(pool: &sqlx::PgPool, scenario_id: &str) -> Result<(), sqlx::Error> {
    let license_id = format!("license-{scenario_id}");
    let session_id = format!("session-{scenario_id}");
    let tenant_id = format!("tenant-{scenario_id}");
    let workspace_id = format!("workspace-{scenario_id}");
    sqlx::query(
        "INSERT INTO ai_transparency_licenses (
            license_id, tenant_id, workspace_id, environment, status, issuer_mode,
            deployment_mode, public_verification_required, metering_plan_id,
            effective_at, expires_at, created_at, updated_at
         ) VALUES ($1,$2,$3,'production','active',
            'hiddenshield_managed','hosted',TRUE,'metering-qa',
            NOW(),NOW() + INTERVAL '1 day',NOW(),NOW())",
    )
    .bind(&license_id)
    .bind(&tenant_id)
    .bind(&workspace_id)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO ai_marking_sessions (
            marking_session_id, license_id, tenant_id, workspace_id, environment,
            idempotency_key, requested_profile_ids_json, claim_type, provider_content_id,
            status, expires_at, created_at, updated_at
         ) VALUES ($1,$2,$3,$4,'production',$5,$6,
            'ai_generated',$7,'ready_to_confirm',NOW() + INTERVAL '1 day',NOW(),NOW())",
    )
    .bind(session_id)
    .bind(license_id)
    .bind(tenant_id)
    .bind(workspace_id)
    .bind(format!("idem-{scenario_id}"))
    .bind(json!([
        "hiddenshield_v3_image_anchor_v1",
        "cn_aigc_label_2025_image_export_v1"
    ]))
    .bind(format!("provider-content-{scenario_id}"))
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
fn command(
    scenario_id: &str,
    suffix: &str,
    failure_injection: ConfirmFailureInjection,
) -> ConfirmMarkingCommand {
    ConfirmMarkingCommand {
        marking_session_id: format!("session-{scenario_id}"),
        transparency_manifest_id: format!("manifest-{scenario_id}-{suffix}"),
        ledger_entry_id: format!("ledger-{scenario_id}-{suffix}"),
        audit_event_id: format!("audit-{scenario_id}-{suffix}"),
        watermark_uid: format!("HS-{scenario_id}-{suffix}"),
        subject_digest: "a".repeat(64),
        generation_mode: "text_to_image".to_string(),
        provider_id: "platform.example".to_string(),
        system_name: "Platform Image".to_string(),
        system_version: "2026.07".to_string(),
        model_id: Some("model-example".to_string()),
        model_version: Some("1.0".to_string()),
        generated_at: Utc::now(),
        operations: json!([]),
        parent_subjects: json!([]),
        profile_statuses: json!([
            {"profileId":"hiddenshield_v3_image_anchor_v1","status":"applied"},
            {"profileId":"cn_aigc_label_2025_image_export_v1","status":"applied"}
        ]),
        evidence: ConfirmEvidence {
            evidence_id: format!("evidence-{scenario_id}-{suffix}"),
            evidence_level: "platform_signed".to_string(),
            evidence_source: "platform.example".to_string(),
            issuer_id: Some("platform.example".to_string()),
            key_id: Some("key-2026-07".to_string()),
            proof_type: "jws".to_string(),
            signature_algorithm: Some("EdDSA".to_string()),
            signature: Some("opaque-signature".to_string()),
        },
        markers: vec![ConfirmMarker {
            marker_binding_id: format!("marker-{scenario_id}-{suffix}"),
            marker_type: "blind_watermark".to_string(),
            marker_profile_id: "hiddenshield_v3_image_anchor_v1".to_string(),
            marker_version: "v1".to_string(),
            embed_status: "verified".to_string(),
            verify_status: "verified".to_string(),
            binding_digest: Some("b".repeat(64)),
        }],
        explicit_label_receipts: vec![ConfirmExplicitLabelReceipt {
            receipt_id: format!("receipt-{scenario_id}-{suffix}"),
            profile_id: "cn_aigc_label_2025_image_export_v1".to_string(),
            required_surface: "both".to_string(),
            render_mode: "file_overlay_and_platform_ui".to_string(),
            rendered_asset_digest: Some("c".repeat(64)),
            placement: json!({"position":"bottom_right"}),
            locale: "zh-CN".to_string(),
            label_text: "AI 生成".to_string(),
            applied_at: Utc::now(),
            applied_by: "platform.example".to_string(),
            verification_status: "verified".to_string(),
        }],
        write_after_read_verified: true,
        failure_injection,
    }
}

#[cfg(feature = "postgres")]
async fn snapshot(
    pool: &sqlx::PgPool,
    scenario_id: &'static str,
    outcomes: &[ConfirmMarkingOutcome],
) -> Result<ScenarioResult, sqlx::Error> {
    let session_id = format!("session-{scenario_id}");
    let session_status: String =
        sqlx::query_scalar("SELECT status FROM ai_marking_sessions WHERE marking_session_id = $1")
            .bind(&session_id)
            .fetch_one(pool)
            .await?;
    let counts = sqlx::query(
        "SELECT
            (SELECT COUNT(*) FROM ai_transparency_manifests WHERE marking_session_id = $1) manifests,
            (SELECT COUNT(*) FROM ai_claim_evidence WHERE transparency_manifest_id IN (
                SELECT transparency_manifest_id FROM ai_transparency_manifests WHERE marking_session_id = $1
            )) evidence,
            (SELECT COUNT(*) FROM ai_marker_bindings WHERE transparency_manifest_id IN (
                SELECT transparency_manifest_id FROM ai_transparency_manifests WHERE marking_session_id = $1
            )) markers,
            (SELECT COUNT(*) FROM ai_explicit_label_receipts WHERE transparency_manifest_id IN (
                SELECT transparency_manifest_id FROM ai_transparency_manifests WHERE marking_session_id = $1
            )) receipts,
            (SELECT COUNT(*) FROM ai_marking_ledger WHERE marking_session_id = $1) ledger,
            (SELECT COUNT(*) FROM ai_marking_ledger WHERE marking_session_id = $1 AND ledger_status = 'committed') committed,
            (SELECT COUNT(*) FROM ai_marking_confirm_audit_events WHERE marking_session_id = $1) audit",
    )
    .bind(&session_id)
    .fetch_one(pool)
    .await?;
    Ok(ScenarioResult {
        scenario_id,
        winner_count: outcomes.iter().filter(|outcome| outcome.succeeded).count(),
        reason_codes: outcomes
            .iter()
            .filter_map(|outcome| outcome.reason_code.clone())
            .collect(),
        session_status,
        manifest_count: counts.get("manifests"),
        evidence_count: counts.get("evidence"),
        marker_count: counts.get("markers"),
        receipt_count: counts.get("receipts"),
        ledger_count: counts.get("ledger"),
        committed_ledger_count: counts.get("committed"),
        audit_count: counts.get("audit"),
    })
}

#[cfg(feature = "postgres")]
fn assert_result(
    result: &ScenarioResult,
    expected_winners: usize,
    expected_reason: Option<&str>,
    expected_confirmed: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    if result.winner_count != expected_winners
        || expected_reason
            .is_some_and(|reason| !result.reason_codes.iter().any(|value| value == reason))
    {
        return Err(format!("unexpected outcomes: {result:?}").into());
    }
    if expected_confirmed {
        if result.session_status != "confirmed"
            || result.manifest_count != 1
            || result.evidence_count != 1
            || result.marker_count != 1
            || result.receipt_count != 1
            || result.ledger_count != 1
            || result.committed_ledger_count != 1
            || result.audit_count != 1
        {
            return Err(format!("confirmed scenario has inconsistent state: {result:?}").into());
        }
    } else if result.session_status != "ready_to_confirm"
        || result.manifest_count != 0
        || result.evidence_count != 0
        || result.marker_count != 0
        || result.receipt_count != 0
        || result.ledger_count != 0
        || result.committed_ledger_count != 0
        || result.audit_count != 0
    {
        return Err(format!("rollback scenario left partial state: {result:?}").into());
    }
    Ok(())
}

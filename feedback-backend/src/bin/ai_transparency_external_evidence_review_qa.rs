#[cfg(feature = "postgres")]
use chrono::{Duration, Utc};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::{
    ai_transparency_change_command::{
        ActorAuthorizationDecision, ActorAuthorizationInput, ApprovalReferenceAdapter,
        ApprovalReferenceDecision, ApprovalReferenceInput, ChangeCommandPreflight,
        InternalIamAuthorizationAdapter,
    },
    ai_transparency_external_evidence_intake::{
        review_external_evidence_intake, ExternalEvidenceIntakeError,
        ReviewExternalEvidenceIntakeCommand,
    },
    database::{
        POSTGRES_P22_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_INTAKE_UP_SQL,
        POSTGRES_P23_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_REVIEW_UP_SQL,
    },
};
#[cfg(feature = "postgres")]
use sqlx::PgPool;

#[cfg(feature = "postgres")]
struct QaIam;

#[cfg(feature = "postgres")]
impl InternalIamAuthorizationAdapter for QaIam {
    fn verify_actor_authorization(
        &self,
        input: &ActorAuthorizationInput<'_>,
    ) -> ActorAuthorizationDecision {
        ActorAuthorizationDecision {
            authorized: input.token_hash == "reviewer-token"
                && input.required_role == "ai_transparency_compliance_approver"
                && input.tenant_id == "tenant-qa"
                && input.workspace_id == "workspace-qa"
                && input.environment == "sandbox",
            reason_code: Some("iam_scope_denied".to_string()),
            verification_receipt_id: Some("iam-qa".to_string()),
        }
    }
}

#[cfg(feature = "postgres")]
struct QaReferences;

#[cfg(feature = "postgres")]
impl ApprovalReferenceAdapter for QaReferences {
    fn verify_approval_reference(
        &self,
        input: &ApprovalReferenceInput<'_>,
    ) -> ApprovalReferenceDecision {
        ApprovalReferenceDecision {
            verified: input.reference_id == "approval://security/review-qa"
                && input.tenant_id == "tenant-qa"
                && input.workspace_id == "workspace-qa"
                && input.environment == "sandbox",
            reason_code: Some("reference_unavailable".to_string()),
            verification_receipt_id: Some("reference-qa".to_string()),
        }
    }
}

#[cfg(feature = "postgres")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))?;
    if !database_url.contains("hiddenshield_migrate_smoke")
        || !(database_url.contains("localhost") || database_url.contains("127.0.0.1"))
    {
        return Err(
            "require a disposable localhost hiddenshield_migrate_smoke PostgreSQL URL".into(),
        );
    }
    let pool = PgPool::connect(&database_url).await?;
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await?;
    sqlx::raw_sql(
        "CREATE TABLE ai_transparency_actor_role_snapshots (
            actor_role_snapshot_id TEXT PRIMARY KEY
         )",
    )
    .execute(&pool)
    .await?;
    for snapshot in ["submitter", "reviewer-a", "reviewer-b"] {
        sqlx::query(
            "INSERT INTO ai_transparency_actor_role_snapshots (actor_role_snapshot_id) VALUES ($1)",
        )
        .bind(snapshot)
        .execute(&pool)
        .await?;
    }
    sqlx::raw_sql(POSTGRES_P22_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_INTAKE_UP_SQL)
        .execute(&pool)
        .await?;
    sqlx::raw_sql(POSTGRES_P23_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_REVIEW_UP_SQL)
        .execute(&pool)
        .await?;
    let now = Utc::now();
    seed_intake(
        &pool,
        "intake-success",
        "submitter",
        now + Duration::hours(1),
    )
    .await?;
    let iam = QaIam;
    let references = QaReferences;
    let preflight = ChangeCommandPreflight {
        iam: &iam,
        references: &references,
    };
    let command = review_command("intake-success", "reviewer-a", now);
    let outcome = review_external_evidence_intake(&pool, &preflight, &command).await?;
    assert_eq!(
        outcome.reason_code,
        "ai_external_evidence_accepted_for_gate"
    );
    assert_eq!(
        count(&pool, "ai_transparency_external_evidence_review_decisions").await?,
        1
    );
    assert_eq!(
        count(
            &pool,
            "ai_transparency_external_evidence_review_audit_events"
        )
        .await?,
        1
    );

    seed_intake(&pool, "intake-self", "reviewer-a", now + Duration::hours(1)).await?;
    assert!(matches!(
        review_external_evidence_intake(
            &pool,
            &preflight,
            &review_command("intake-self", "reviewer-a", now)
        )
        .await,
        Err(ExternalEvidenceIntakeError::Invalid(
            "reviewer_or_evidence_window"
        ))
    ));
    seed_intake(
        &pool,
        "intake-expired",
        "submitter",
        now - Duration::seconds(1),
    )
    .await?;
    assert!(matches!(
        review_external_evidence_intake(
            &pool,
            &preflight,
            &review_command("intake-expired", "reviewer-a", now)
        )
        .await,
        Err(ExternalEvidenceIntakeError::Invalid(
            "reviewer_or_evidence_window"
        ))
    ));
    seed_intake(
        &pool,
        "intake-reference",
        "submitter",
        now + Duration::hours(1),
    )
    .await?;
    let mut denied = review_command("intake-reference", "reviewer-a", now);
    denied.review_reference = "approval://security/denied".to_string();
    assert!(matches!(
        review_external_evidence_intake(&pool, &preflight, &denied).await,
        Err(ExternalEvidenceIntakeError::ReferenceDenied(_))
    ));
    assert_eq!(
        count(&pool, "ai_transparency_external_evidence_review_decisions").await?,
        1
    );

    seed_intake(
        &pool,
        "intake-concurrent",
        "submitter",
        now + Duration::hours(1),
    )
    .await?;
    let first_command = review_command("intake-concurrent", "reviewer-a", now);
    let second_command = review_command("intake-concurrent", "reviewer-b", now);
    let first = review_external_evidence_intake(&pool, &preflight, &first_command);
    let second = review_external_evidence_intake(&pool, &preflight, &second_command);
    let (first, second) = tokio::join!(first, second);
    assert!(first.is_ok() ^ second.is_ok());
    assert_eq!(
        count(&pool, "ai_transparency_external_evidence_review_decisions").await?,
        2
    );
    assert_eq!(
        count(
            &pool,
            "ai_transparency_external_evidence_review_audit_events"
        )
        .await?,
        2
    );
    sqlx::raw_sql(
        "CREATE FUNCTION fail_external_evidence_review_audit_qa()
         RETURNS TRIGGER AS $$ BEGIN RAISE EXCEPTION 'audit_failure_qa'; END; $$ LANGUAGE plpgsql;
         CREATE TRIGGER trg_fail_external_evidence_review_audit_qa
         BEFORE INSERT ON ai_transparency_external_evidence_review_audit_events
         FOR EACH ROW EXECUTE FUNCTION fail_external_evidence_review_audit_qa();",
    )
    .execute(&pool)
    .await?;
    seed_intake(
        &pool,
        "intake-audit-failure",
        "submitter",
        now + Duration::hours(1),
    )
    .await?;
    assert!(matches!(
        review_external_evidence_intake(
            &pool,
            &preflight,
            &review_command("intake-audit-failure", "reviewer-a", now)
        )
        .await,
        Err(ExternalEvidenceIntakeError::Database(_))
    ));
    assert_eq!(
        count(&pool, "ai_transparency_external_evidence_review_decisions").await?,
        2
    );
    assert_eq!(
        count(
            &pool,
            "ai_transparency_external_evidence_review_audit_events"
        )
        .await?,
        2
    );
    sqlx::raw_sql(
        "DROP TRIGGER trg_fail_external_evidence_review_audit_qa
         ON ai_transparency_external_evidence_review_audit_events;
         DROP FUNCTION fail_external_evidence_review_audit_qa();",
    )
    .execute(&pool)
    .await?;
    assert!(sqlx::query(
        "UPDATE ai_transparency_external_evidence_review_decisions
             SET decision = 'rejected' WHERE evidence_intake_id = 'intake-success'",
    )
    .execute(&pool)
    .await
    .is_err());
    assert!(sqlx::query(
        "DELETE FROM ai_transparency_external_evidence_intakes
             WHERE evidence_intake_id = 'intake-success'",
    )
    .execute(&pool)
    .await
    .is_err());
    println!(
        "{}",
        r#"{"scenarioId":"external_evidence_review_postgres_qa","status":"passed"}"#
    );
    pool.close().await;
    Ok(())
}

#[cfg(feature = "postgres")]
fn review_command(
    intake: &str,
    reviewer: &str,
    decided_at: chrono::DateTime<Utc>,
) -> ReviewExternalEvidenceIntakeCommand {
    ReviewExternalEvidenceIntakeCommand {
        evidence_intake_id: intake.to_string(),
        decision: "accepted_for_gate".to_string(),
        reviewer_snapshot_id: reviewer.to_string(),
        reviewer_token_hash: "reviewer-token".to_string(),
        review_reference: "approval://security/review-qa".to_string(),
        reason_digest: "a".repeat(64),
        decided_at,
    }
}

#[cfg(feature = "postgres")]
async fn seed_intake(
    pool: &PgPool,
    id: &str,
    submitter: &str,
    valid_until: chrono::DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ai_transparency_external_evidence_intakes (
         evidence_intake_id, source_kind, tenant_id, workspace_id, environment, source_reference,
         evidence_reference, evidence_sha256, signer_reference, contract_reference,
         security_review_reference, submitter_snapshot_id, valid_from, valid_until, received_at,
         status, created_at) VALUES ($1, 'provider_recovery', 'tenant-qa', 'workspace-qa', 'sandbox',
         $2, $3, $4, 'receipt://qa/signer', 'approval://contract/qa',
         'approval://security/qa', $5, NOW() - INTERVAL '1 hour', $6, NOW(), 'received_for_review', NOW())",
    )
    .bind(id)
    .bind(format!("provider://qa/{id}"))
    .bind(format!("evidence://sha256/{}", "a".repeat(64)))
    .bind("a".repeat(64))
    .bind(submitter)
    .bind(valid_until)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn count(pool: &PgPool, table: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("ai_transparency_external_evidence_review_qa requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
use std::sync::{Arc, Barrier};
#[cfg(feature = "postgres")]
use std::thread;

#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_change_command::{
    execute_postgres_change_command, ActorAuthorizationInput, ApprovalReferenceInput,
    ChangeCommandPreflight, InternalChangeCommand, InternalChangeCommandMode,
    InternalChangeCommandOutcome,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::ai_transparency_internal_provider::{
    iam_scope_digest, new_signed_receipt, reference_scope_digest, ControlledInternalProviderClient,
    InternalProviderClientConfig, InternalProviderTransport, ProviderHealth, ProviderReceiptKind,
    ProviderTransportError, SignedProviderReceipt,
};
#[cfg(feature = "postgres")]
use hiddenshield_feedback_backend::database::{
    POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL, POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL,
    POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
};
#[cfg(feature = "postgres")]
use serde::Serialize;
#[cfg(feature = "postgres")]
use sqlx::{Connection as _, Row};

#[cfg(feature = "postgres")]
const SCENARIOS: [(&str, usize, &str); 6] = [
    ("duplicate_idempotency_request", 1, "idempotency_replay"),
    ("concurrent_profile_renew", 1, "target_version_conflict"),
    ("duplicate_execution", 1, "target_state_conflict"),
    (
        "grant_vs_revoke_same_target",
        1,
        "conflicting_request_exists",
    ),
    ("audit_failure_rollback", 0, "audit_write_failed"),
    ("projection_version_conflict", 0, "target_version_conflict"),
];

#[cfg(feature = "postgres")]
const IAM_REJECTIONS: [(&str, &str, ProviderMutation); 5] = [
    (
        "iam_signature_invalid",
        "iam_token_invalid",
        ProviderMutation::InvalidSignature,
    ),
    (
        "iam_expired",
        "iam_token_expired",
        ProviderMutation::Expired,
    ),
    (
        "iam_scope_digest_mismatch",
        "iam_scope_denied",
        ProviderMutation::ScopeMismatch,
    ),
    (
        "iam_health_unavailable",
        "iam_unavailable",
        ProviderMutation::HealthUnavailable,
    ),
    (
        "iam_transport_unavailable",
        "iam_unavailable",
        ProviderMutation::TransportUnavailable,
    ),
];

#[cfg(feature = "postgres")]
const REFERENCE_REJECTIONS: [(&str, &str, ProviderMutation); 4] = [
    (
        "reference_signature_invalid",
        "reference_authority_untrusted",
        ProviderMutation::InvalidSignature,
    ),
    (
        "reference_expired",
        "reference_expired",
        ProviderMutation::Expired,
    ),
    (
        "reference_scope_digest_mismatch",
        "reference_scope_mismatch",
        ProviderMutation::ScopeMismatch,
    ),
    (
        "reference_transport_unavailable",
        "reference_unavailable",
        ProviderMutation::TransportUnavailable,
    ),
];

#[cfg(feature = "postgres")]
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ScenarioSnapshot {
    database_kind: &'static str,
    scenario_id: &'static str,
    winner_count: usize,
    request_statuses: Vec<String>,
    target_version: i32,
    active_version_count: i32,
    audit_sequence: Vec<i32>,
    reason_codes: Vec<String>,
    credential_count: i64,
    marking_session_count: i64,
    manifest_count: i64,
    ledger_count: i64,
}

#[cfg(feature = "postgres")]
#[derive(Clone, Copy)]
enum ProviderMutation {
    Valid,
    InvalidSignature,
    Expired,
    ScopeMismatch,
    HealthUnavailable,
    TransportUnavailable,
}

#[cfg(feature = "postgres")]
struct HarnessProviderTransport {
    config: InternalProviderClientConfig,
    iam_mutation: ProviderMutation,
    reference_mutation: ProviderMutation,
}

#[cfg(feature = "postgres")]
impl InternalProviderTransport for HarnessProviderTransport {
    fn health(&self) -> ProviderHealth {
        if matches!(self.iam_mutation, ProviderMutation::HealthUnavailable)
            || matches!(self.reference_mutation, ProviderMutation::HealthUnavailable)
        {
            ProviderHealth::Unavailable
        } else {
            ProviderHealth::Healthy
        }
    }

    fn fetch_iam_receipt(
        &self,
        input: &ActorAuthorizationInput<'_>,
    ) -> Result<SignedProviderReceipt, ProviderTransportError> {
        receipt_for_mutation(
            &self.config,
            "iam-receipt-qa",
            ProviderReceiptKind::Iam,
            iam_scope_digest(input),
            self.iam_mutation,
        )
    }

    fn fetch_reference_receipt(
        &self,
        input: &ApprovalReferenceInput<'_>,
    ) -> Result<SignedProviderReceipt, ProviderTransportError> {
        receipt_for_mutation(
            &self.config,
            "reference-receipt-qa",
            ProviderReceiptKind::Reference,
            reference_scope_digest(input),
            self.reference_mutation,
        )
    }
}

#[cfg(feature = "postgres")]
fn provider_config() -> InternalProviderClientConfig {
    InternalProviderClientConfig {
        provider_id: "hiddenshield-internal-qa".to_string(),
        key_id: "qa-hmac-v1".to_string(),
        hmac_secret: b"hiddenshield-internal-provider-qa-secret".to_vec(),
    }
}

#[cfg(feature = "postgres")]
fn provider_client(
    iam_mutation: ProviderMutation,
    reference_mutation: ProviderMutation,
) -> ControlledInternalProviderClient<HarnessProviderTransport> {
    let config = provider_config();
    ControlledInternalProviderClient::new(
        HarnessProviderTransport {
            config: config.clone(),
            iam_mutation,
            reference_mutation,
        },
        config,
    )
}

#[cfg(feature = "postgres")]
fn receipt_for_mutation(
    config: &InternalProviderClientConfig,
    receipt_id: &str,
    kind: ProviderReceiptKind,
    scope_digest: String,
    mutation: ProviderMutation,
) -> Result<SignedProviderReceipt, ProviderTransportError> {
    if matches!(mutation, ProviderMutation::TransportUnavailable) {
        return Err(ProviderTransportError::Unavailable);
    }
    let now = chrono::Utc::now();
    let mut receipt = new_signed_receipt(
        config,
        receipt_id,
        kind,
        scope_digest,
        now - chrono::Duration::seconds(1),
        now + chrono::Duration::minutes(5),
    );
    match mutation {
        ProviderMutation::Valid
        | ProviderMutation::HealthUnavailable
        | ProviderMutation::TransportUnavailable => {}
        ProviderMutation::InvalidSignature => receipt.signature = "invalid-signature".to_string(),
        ProviderMutation::Expired => {
            receipt.expires_at = now - chrono::Duration::seconds(1);
            receipt.signature =
                hiddenshield_feedback_backend::ai_transparency_internal_provider::sign_provider_receipt(
                    config, &receipt,
                );
        }
        ProviderMutation::ScopeMismatch => {
            receipt.scope_digest = "scope-digest-mismatch".to_string();
            receipt.signature =
                hiddenshield_feedback_backend::ai_transparency_internal_provider::sign_provider_receipt(
                    config, &receipt,
                );
        }
    }
    Ok(receipt)
}

#[cfg(feature = "postgres")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let postgres_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| "missing disposable PostgreSQL URL for concurrency QA")?;
    if !is_safe_smoke_url(&postgres_url) {
        return Err(
            "refusing concurrency QA against non-disposable URL; require localhost/127.0.0.1 and hiddenshield_migrate_smoke"
                .into(),
        );
    }

    let mut postgres = run_postgres_harness(&postgres_url).await?;
    postgres.extend(run_postgres_preflight_rejections(&postgres_url).await?);
    println!("{}", serde_json::to_string_pretty(&postgres)?);
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("ai_transparency_approval_concurrency_qa requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
fn is_safe_smoke_url(database_url: &str) -> bool {
    let lower = database_url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_migrate_smoke")
}

#[cfg(feature = "postgres")]
async fn run_postgres_harness(
    database_url: &str,
) -> Result<Vec<ScenarioSnapshot>, Box<dyn std::error::Error>> {
    let setup = sqlx::PgPool::connect(database_url).await?;
    let mut snapshots = Vec::new();
    for (scenario_id, expected_winners, expected_reason) in SCENARIOS {
        reset_postgres_schema(&setup).await?;
        seed_postgres_scenario(&setup, scenario_id).await?;
        let outcomes = run_postgres_commands(database_url, scenario_commands(scenario_id))?;
        let snapshot = postgres_snapshot(&setup, scenario_id, &outcomes).await?;
        assert_expected(&snapshot, expected_winners, expected_reason)?;
        snapshots.push(snapshot);
    }
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&setup)
        .await?;
    setup.close().await;
    Ok(snapshots)
}

#[cfg(feature = "postgres")]
async fn run_postgres_preflight_rejections(
    database_url: &str,
) -> Result<Vec<ScenarioSnapshot>, Box<dyn std::error::Error>> {
    let pool = sqlx::PgPool::connect(database_url).await?;
    let mut snapshots = Vec::new();
    for (scenario_id, reason_code, mutation) in IAM_REJECTIONS {
        snapshots.push(
            run_postgres_preflight_rejection(
                &pool,
                scenario_id,
                mutation,
                ProviderMutation::Valid,
                reason_code,
            )
            .await?,
        );
    }
    for (scenario_id, reason_code, mutation) in REFERENCE_REJECTIONS {
        snapshots.push(
            run_postgres_preflight_rejection(
                &pool,
                scenario_id,
                ProviderMutation::Valid,
                mutation,
                reason_code,
            )
            .await?,
        );
    }
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(&pool)
        .await?;
    pool.close().await;
    Ok(snapshots)
}

#[cfg(feature = "postgres")]
async fn run_postgres_preflight_rejection(
    pool: &sqlx::PgPool,
    scenario_id: &'static str,
    iam_mutation: ProviderMutation,
    reference_mutation: ProviderMutation,
    expected_reason: &str,
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    reset_postgres_schema(pool).await?;
    seed_postgres_scenario(pool, scenario_id).await?;
    let adapter = provider_client(iam_mutation, reference_mutation);
    let mut connection = sqlx::PgConnection::connect(
        &std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))?,
    )
    .await?;
    let outcome = execute_postgres_change_command(
        &mut connection,
        &command(
            scenario_id,
            InternalChangeCommandMode::ApplyProfileChange,
            "requester-a",
            1,
            "active",
        ),
        &ChangeCommandPreflight {
            iam: &adapter,
            references: &adapter,
        },
    )
    .await?;
    let snapshot = postgres_snapshot(pool, scenario_id, &[outcome]).await?;
    assert_preflight_rejection(&snapshot, expected_reason)?;
    Ok(snapshot)
}

#[cfg(feature = "postgres")]
fn run_postgres_commands(
    database_url: &str,
    commands: [InternalChangeCommand; 2],
) -> Result<Vec<InternalChangeCommandOutcome>, Box<dyn std::error::Error>> {
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
                    let adapter = provider_client(ProviderMutation::Valid, ProviderMutation::Valid);
                    Ok(execute_postgres_change_command(
                        &mut connection,
                        &command,
                        &ChangeCommandPreflight {
                            iam: &adapter,
                            references: &adapter,
                        },
                    )
                    .await?)
                })
            },
        ));
    }
    handles
        .into_iter()
        .map(|handle| {
            handle
                .join()
                .map_err(|_| "PostgreSQL change-command thread panicked")?
                .map_err(|error| error.to_string().into())
        })
        .collect()
}

#[cfg(feature = "postgres")]
fn scenario_commands(scenario_id: &str) -> [InternalChangeCommand; 2] {
    match scenario_id {
        "duplicate_idempotency_request" => {
            let command = command(
                "duplicate-idempotency",
                InternalChangeCommandMode::SubmitRequest,
                "requester-a",
                1,
                "active",
            );
            [command.clone(), command]
        }
        "concurrent_profile_renew" => [
            command(
                "concurrent-renew-a",
                InternalChangeCommandMode::ApplyProfileChange,
                "requester-a",
                1,
                "active",
            ),
            command(
                "concurrent-renew-b",
                InternalChangeCommandMode::ApplyProfileChange,
                "requester-b",
                1,
                "active",
            ),
        ],
        "duplicate_execution" => {
            let command = command(
                "duplicate-execution",
                InternalChangeCommandMode::ExecuteApprovedRequest,
                "requester-a",
                1,
                "active",
            );
            [command.clone(), command]
        }
        "grant_vs_revoke_same_target" => [
            command(
                "grant-request",
                InternalChangeCommandMode::SubmitRequest,
                "requester-a",
                1,
                "active",
            ),
            command(
                "revoke-request",
                InternalChangeCommandMode::SubmitRequest,
                "requester-b",
                1,
                "revoked",
            ),
        ],
        "audit_failure_rollback" => {
            let mut first = command(
                "audit-failure-a",
                InternalChangeCommandMode::ApplyProfileChange,
                "requester-a",
                1,
                "active",
            );
            first.inject_audit_failure = true;
            let mut second = command(
                "audit-failure-b",
                InternalChangeCommandMode::ApplyProfileChange,
                "requester-b",
                1,
                "active",
            );
            second.inject_audit_failure = true;
            [first, second]
        }
        "projection_version_conflict" => [
            command(
                "projection-conflict-a",
                InternalChangeCommandMode::ApplyProfileChange,
                "requester-a",
                2,
                "active",
            ),
            command(
                "projection-conflict-b",
                InternalChangeCommandMode::ApplyProfileChange,
                "requester-b",
                2,
                "active",
            ),
        ],
        _ => unreachable!("unknown scenario"),
    }
}

#[cfg(feature = "postgres")]
fn command(
    suffix: &str,
    mode: InternalChangeCommandMode,
    requester: &str,
    expected_target_version: i32,
    desired_status: &str,
) -> InternalChangeCommand {
    InternalChangeCommand {
        mode,
        change_request_id: format!("request-{suffix}"),
        approval_id: format!("approval-{suffix}"),
        execution_id: format!("execution-{suffix}"),
        entitlement_version_id: format!("version-{suffix}"),
        operation: if desired_status == "revoked" {
            "revoke_profile_entitlement".to_string()
        } else {
            "renew_profile_entitlement".to_string()
        },
        target_scope_key: "profile:license-qa:profile-qa".to_string(),
        tenant_id: "tenant-qa".to_string(),
        workspace_id: "workspace-qa".to_string(),
        environment: "production".to_string(),
        license_id: "license-qa".to_string(),
        profile_id: "profile-qa".to_string(),
        profile_kind: "regulatory".to_string(),
        expected_target_version,
        desired_next_version: expected_target_version + 1,
        desired_status: desired_status.to_string(),
        terms_version: format!("terms-v{}", expected_target_version + 1),
        contract_reference: None,
        legal_review_reference: Some("legal-review-qa".to_string()),
        security_review_reference: None,
        requester_snapshot_id: format!("snapshot-{requester}"),
        requester_actor_id: requester.to_string(),
        requester_token_hash: digest_for(&format!("{requester}-token")),
        approver_snapshot_id: "snapshot-approver".to_string(),
        approver_actor_id: "approver".to_string(),
        approver_role: "ai_transparency_compliance_approver".to_string(),
        approver_token_hash: digest_for("approver-token"),
        executor_snapshot_id: "snapshot-executor".to_string(),
        executor_token_hash: digest_for("executor-token"),
        request_digest: digest_for(suffix),
        idempotency_key: format!("idempotency-{suffix}"),
        inject_audit_failure: false,
    }
}

#[cfg(feature = "postgres")]
fn digest_for(value: &str) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(value.as_bytes()))
}

#[cfg(any())]
fn sqlite_scenario_path(scenario_id: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "hiddenshield-change-command-{scenario_id}-{}.sqlite",
        uuid::Uuid::new_v4()
    ))
}

#[cfg(any())]
fn seed_sqlite_scenario(
    conn: &Connection,
    scenario_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    conn.execute_batch(
        "INSERT INTO ai_transparency_actor_role_snapshots (
            actor_role_snapshot_id, actor_id, actor_type, role, tenant_id, workspace_id,
            environment, role_binding_id, role_binding_version, source_identity_system,
            authentication_level, captured_at, source_expires_at, snapshot_sha256
         ) VALUES
         ('snapshot-requester-a', 'requester-a', 'human', 'ai_transparency_requester',
            'tenant-qa', 'workspace-qa', 'production', 'binding-requester-a', 1,
            'hiddenshield_internal_iam', 'mfa', '2026-07-27T00:00:00Z',
            '2027-07-27T00:00:00Z', 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa'),
         ('snapshot-requester-b', 'requester-b', 'human', 'ai_transparency_requester',
            'tenant-qa', 'workspace-qa', 'production', 'binding-requester-b', 1,
            'hiddenshield_internal_iam', 'mfa', '2026-07-27T00:00:00Z',
            '2027-07-27T00:00:00Z', 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb'),
         ('snapshot-approver', 'approver', 'human', 'ai_transparency_compliance_approver',
            'tenant-qa', 'workspace-qa', 'production', 'binding-approver', 1,
            'hiddenshield_internal_iam', 'mfa', '2026-07-27T00:00:00Z',
            '2027-07-27T00:00:00Z', 'cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc'),
         ('snapshot-executor', 'executor', 'system', 'system_executor',
            'tenant-qa', 'workspace-qa', 'production', 'binding-executor', 1,
            'hiddenshield_internal_iam', 'system', '2026-07-27T00:00:00Z',
            '2027-07-27T00:00:00Z', 'dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd');
         INSERT INTO ai_transparency_licenses (
            license_id, tenant_id, workspace_id, environment, status, issuer_mode,
            deployment_mode, public_verification_required, metering_plan_id,
            effective_at, expires_at, created_at, updated_at
         ) VALUES (
            'license-qa', 'tenant-qa', 'workspace-qa', 'production', 'active',
            'hiddenshield_managed', 'hosted', 1, 'metering-qa',
            '2026-07-27T00:00:00Z', '2027-07-27T00:00:00Z',
            '2026-07-27T00:00:00Z', '2026-07-27T00:00:00Z'
         );
         INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_id, target_scope_key,
            tenant_id, workspace_id, environment, expected_target_version, desired_next_version,
            desired_state_json, request_reason, legal_review_reference, requester_snapshot_id,
            request_digest_version, request_digest, idempotency_key, status, expires_at,
            evidence_quality, production_eligibility, created_at, updated_at
         ) VALUES (
            'seed-version-request', 'grant_profile_entitlement', 'profile_entitlement',
            'profile-qa', 'seed:profile:license-qa:profile-qa', 'tenant-qa', 'workspace-qa',
            'production', 1, 1, '{}', 'seed version', 'legal-review-qa',
            'snapshot-requester-a', 'hs-ai-change-request-digest-v1',
            'eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee',
            'seed-version-idempotency', 'succeeded', '2027-07-27T00:00:00Z',
            'migrated_legacy_without_four_eyes', 0,
            '2026-07-27T00:00:00Z', '2026-07-27T00:00:00Z'
         );
         INSERT INTO ai_profile_entitlement_versions (
            profile_entitlement_version_id, license_id, profile_id, version, profile_kind,
            status, effective_at, expires_at, terms_version, legal_review_reference,
            source_change_request_id, created_at
         ) VALUES (
            'version-seed', 'license-qa', 'profile-qa', 1, 'regulatory', 'active',
            '2026-07-27T00:00:00Z', '2027-07-27T00:00:00Z', 'terms-v1',
            'legal-review-qa', 'seed-version-request', '2026-07-27T00:00:00Z'
         );
         INSERT INTO ai_profile_entitlements (
            license_id, profile_id, profile_kind, status, effective_at, expires_at,
            terms_version, approved_by, created_at, updated_at, current_version_id,
            current_version, projection_updated_at
         ) VALUES (
            'license-qa', 'profile-qa', 'regulatory', 'active',
            '2026-07-27T00:00:00Z', '2027-07-27T00:00:00Z', 'terms-v1',
            'seed', '2026-07-27T00:00:00Z', '2026-07-27T00:00:00Z',
            'version-seed', 1, '2026-07-27T00:00:00Z'
         );",
    )?;
    if scenario_id == "duplicate_execution" {
        seed_sqlite_approved_request(conn)?;
    }
    Ok(())
}

#[cfg(any())]
fn seed_sqlite_approved_request(conn: &Connection) -> Result<(), rusqlite::Error> {
    let command = command(
        "duplicate-execution",
        InternalChangeCommandMode::ExecuteApprovedRequest,
        "requester-a",
        1,
        "active",
    );
    conn.execute(
        "INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_id, target_scope_key,
            tenant_id, workspace_id, environment, expected_target_version, desired_next_version,
            desired_state_json, request_reason, legal_review_reference, requester_snapshot_id,
            request_digest_version, request_digest, idempotency_key, status, expires_at,
            created_at, updated_at
         ) VALUES (?1, ?2, 'profile_entitlement', ?3, ?4, ?5, ?6, ?7, ?8, ?9,
            '{}', 'seed approved request', ?10, ?11, 'hs-ai-change-request-digest-v1',
            ?12, ?13, 'approved', '2027-07-27T00:00:00Z',
            '2026-07-27T00:00:00Z', '2026-07-27T00:00:00Z')",
        params![
            command.change_request_id,
            command.operation,
            command.profile_id,
            command.target_scope_key,
            command.tenant_id,
            command.workspace_id,
            command.environment,
            command.expected_target_version,
            command.desired_next_version,
            command.legal_review_reference,
            command.requester_snapshot_id,
            command.request_digest,
            command.idempotency_key,
        ],
    )?;
    conn.execute(
        "INSERT INTO ai_transparency_change_approvals (
            approval_id, change_request_id, decision, approver_snapshot_id,
            requester_actor_id, approver_actor_id, approver_role, decision_reason,
            policy_version, request_digest, decided_at
         ) VALUES (?1, ?2, 'approved', ?3, ?4, ?5, ?6, 'seed approval', 'v1', ?7,
            '2026-07-27T00:00:00Z')",
        params![
            command.approval_id,
            command.change_request_id,
            command.approver_snapshot_id,
            command.requester_actor_id,
            command.approver_actor_id,
            command.approver_role,
            command.request_digest,
        ],
    )?;
    seed_sqlite_audit(
        conn,
        &command,
        1,
        "change_request_submitted",
        "pending_review",
    )?;
    seed_sqlite_audit(conn, &command, 2, "approval_granted", "approved")?;
    Ok(())
}

#[cfg(any())]
fn seed_sqlite_audit(
    conn: &Connection,
    command: &InternalChangeCommand,
    sequence: i32,
    event_type: &str,
    to_state: &str,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO ai_transparency_change_audit_events (
            audit_event_id, change_request_id, sequence, event_type, to_state,
            actor_snapshot_id, target_type, target_id, reason_code, request_digest,
            details_json, occurred_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'profile_entitlement', ?7,
            'seed', ?8, '{}', '2026-07-27T00:00:00Z')",
        params![
            format!("{}-audit-{sequence}", command.change_request_id),
            command.change_request_id,
            sequence,
            event_type,
            to_state,
            if sequence == 1 {
                &command.requester_snapshot_id
            } else {
                &command.approver_snapshot_id
            },
            command.profile_id,
            command.request_digest,
        ],
    )?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn reset_postgres_schema(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
        .execute(pool)
        .await?;
    for sql in [
        POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL,
        POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL,
        POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
    ] {
        sqlx::raw_sql(sql).execute(pool).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn seed_postgres_scenario(pool: &sqlx::PgPool, scenario_id: &str) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(
        "INSERT INTO ai_transparency_actor_role_snapshots (
            actor_role_snapshot_id, actor_id, actor_type, role, tenant_id, workspace_id,
            environment, role_binding_id, role_binding_version, source_identity_system,
            authentication_level, captured_at, source_expires_at, snapshot_sha256
         ) VALUES
         ('snapshot-requester-a', 'requester-a', 'human', 'ai_transparency_requester',
            'tenant-qa', 'workspace-qa', 'production', 'binding-requester-a', 1,
            'hiddenshield_internal_iam', 'mfa', NOW(), NOW() + INTERVAL '1 day', repeat('a', 64)),
         ('snapshot-requester-b', 'requester-b', 'human', 'ai_transparency_requester',
            'tenant-qa', 'workspace-qa', 'production', 'binding-requester-b', 1,
            'hiddenshield_internal_iam', 'mfa', NOW(), NOW() + INTERVAL '1 day', repeat('b', 64)),
         ('snapshot-approver', 'approver', 'human', 'ai_transparency_compliance_approver',
            'tenant-qa', 'workspace-qa', 'production', 'binding-approver', 1,
            'hiddenshield_internal_iam', 'mfa', NOW(), NOW() + INTERVAL '1 day', repeat('c', 64)),
         ('snapshot-executor', 'executor', 'system', 'system_executor',
            'tenant-qa', 'workspace-qa', 'production', 'binding-executor', 1,
            'hiddenshield_internal_iam', 'system', NOW(), NOW() + INTERVAL '1 day', repeat('d', 64));
         INSERT INTO ai_transparency_licenses (
            license_id, tenant_id, workspace_id, environment, status, issuer_mode,
            deployment_mode, public_verification_required, metering_plan_id,
            effective_at, expires_at, created_at, updated_at
         ) VALUES (
            'license-qa', 'tenant-qa', 'workspace-qa', 'production', 'active',
            'hiddenshield_managed', 'hosted', TRUE, 'metering-qa',
            NOW(), NOW() + INTERVAL '1 day', NOW(), NOW()
         );
         INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_id, target_scope_key,
            tenant_id, workspace_id, environment, expected_target_version, desired_next_version,
            desired_state_json, request_reason, legal_review_reference, requester_snapshot_id,
            request_digest_version, request_digest, idempotency_key, status, expires_at,
            evidence_quality, production_eligibility, created_at, updated_at
         ) VALUES (
            'seed-version-request', 'grant_profile_entitlement', 'profile_entitlement',
            'profile-qa', 'seed:profile:license-qa:profile-qa', 'tenant-qa', 'workspace-qa',
            'production', 1, 1, '{}'::jsonb, 'seed version', 'legal-review-qa',
            'snapshot-requester-a', 'hs-ai-change-request-digest-v1', repeat('e', 64),
            'seed-version-idempotency', 'succeeded', NOW() + INTERVAL '1 day',
            'migrated_legacy_without_four_eyes', FALSE, NOW(), NOW()
         );
         INSERT INTO ai_profile_entitlement_versions (
            profile_entitlement_version_id, license_id, profile_id, version, profile_kind,
            status, effective_at, expires_at, terms_version, legal_review_reference,
            source_change_request_id, created_at
         ) VALUES (
            'version-seed', 'license-qa', 'profile-qa', 1, 'regulatory', 'active',
            NOW(), NOW() + INTERVAL '1 day', 'terms-v1', 'legal-review-qa',
            'seed-version-request', NOW()
         );
         INSERT INTO ai_profile_entitlements (
            license_id, profile_id, profile_kind, status, effective_at, expires_at,
            terms_version, approved_by, created_at, updated_at, current_version_id,
            current_version, projection_updated_at
         ) VALUES (
            'license-qa', 'profile-qa', 'regulatory', 'active',
            NOW(), NOW() + INTERVAL '1 day', 'terms-v1', 'seed', NOW(), NOW(),
            'version-seed', 1, NOW()
         );",
    )
    .execute(pool)
    .await?;
    if scenario_id == "duplicate_execution" {
        seed_postgres_approved_request(pool).await?;
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn seed_postgres_approved_request(pool: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    let command = command(
        "duplicate-execution",
        InternalChangeCommandMode::ExecuteApprovedRequest,
        "requester-a",
        1,
        "active",
    );
    sqlx::query(
        "INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_id, target_scope_key,
            tenant_id, workspace_id, environment, expected_target_version, desired_next_version,
            desired_state_json, request_reason, legal_review_reference, requester_snapshot_id,
            request_digest_version, request_digest, idempotency_key, status, expires_at,
            created_at, updated_at
         ) VALUES ($1, $2, 'profile_entitlement', $3, $4, $5, $6, $7, $8, $9,
            '{}'::jsonb, 'seed approved request', $10, $11,
            'hs-ai-change-request-digest-v1', $12, $13, 'approved',
            NOW() + INTERVAL '1 day', NOW(), NOW())",
    )
    .bind(&command.change_request_id)
    .bind(&command.operation)
    .bind(&command.profile_id)
    .bind(&command.target_scope_key)
    .bind(&command.tenant_id)
    .bind(&command.workspace_id)
    .bind(&command.environment)
    .bind(command.expected_target_version)
    .bind(command.desired_next_version)
    .bind(&command.legal_review_reference)
    .bind(&command.requester_snapshot_id)
    .bind(&command.request_digest)
    .bind(&command.idempotency_key)
    .execute(pool)
    .await?;
    sqlx::query(
        "INSERT INTO ai_transparency_change_approvals (
            approval_id, change_request_id, decision, approver_snapshot_id,
            requester_actor_id, approver_actor_id, approver_role, decision_reason,
            policy_version, request_digest, decided_at
         ) VALUES ($1, $2, 'approved', $3, $4, $5, $6, 'seed approval', 'v1', $7, NOW())",
    )
    .bind(&command.approval_id)
    .bind(&command.change_request_id)
    .bind(&command.approver_snapshot_id)
    .bind(&command.requester_actor_id)
    .bind(&command.approver_actor_id)
    .bind(&command.approver_role)
    .bind(&command.request_digest)
    .execute(pool)
    .await?;
    for (sequence, event_type, to_state, actor) in [
        (
            1,
            "change_request_submitted",
            "pending_review",
            &command.requester_snapshot_id,
        ),
        (
            2,
            "approval_granted",
            "approved",
            &command.approver_snapshot_id,
        ),
    ] {
        sqlx::query(
            "INSERT INTO ai_transparency_change_audit_events (
                audit_event_id, change_request_id, sequence, event_type, to_state,
                actor_snapshot_id, target_type, target_id, reason_code, request_digest,
                details_json, occurred_at
             ) VALUES ($1, $2, $3, $4, $5, $6, 'profile_entitlement', $7,
                'seed', $8, '{}'::jsonb, NOW())",
        )
        .bind(format!("{}-audit-{sequence}", command.change_request_id))
        .bind(&command.change_request_id)
        .bind(sequence)
        .bind(event_type)
        .bind(to_state)
        .bind(actor)
        .bind(&command.profile_id)
        .bind(&command.request_digest)
        .execute(pool)
        .await?;
    }
    Ok(())
}

#[cfg(any())]
fn sqlite_snapshot(
    path: &Path,
    scenario_id: &'static str,
    outcomes: &[InternalChangeCommandOutcome],
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    let conn = Connection::open(path)?;
    let request_statuses = sqlite_strings(
        &conn,
        "SELECT status FROM ai_transparency_change_requests
         WHERE change_request_id NOT LIKE 'seed-%' ORDER BY change_request_id",
    )?;
    let audit_sequence = sqlite_i32s(
        &conn,
        "SELECT sequence FROM ai_transparency_change_audit_events
         WHERE change_request_id NOT LIKE 'seed-%'
         ORDER BY change_request_id, sequence",
    )?;
    Ok(ScenarioSnapshot {
        database_kind: "sqlite",
        scenario_id,
        winner_count: outcomes.iter().filter(|outcome| outcome.succeeded).count(),
        request_statuses,
        target_version: conn.query_row(
            "SELECT current_version FROM ai_profile_entitlements
             WHERE license_id = 'license-qa' AND profile_id = 'profile-qa'",
            [],
            |row| row.get(0),
        )?,
        active_version_count: conn.query_row(
            "SELECT COUNT(*) FROM ai_profile_entitlement_versions
             WHERE license_id = 'license-qa' AND profile_id = 'profile-qa' AND status = 'active'",
            [],
            |row| row.get(0),
        )?,
        audit_sequence,
        reason_codes: outcome_reasons(outcomes),
        credential_count: count_sqlite(&conn, "ai_sdk_credential_bindings")?,
        marking_session_count: count_sqlite(&conn, "ai_marking_sessions")?,
        manifest_count: count_sqlite(&conn, "ai_transparency_manifests")?,
        ledger_count: count_sqlite(&conn, "ai_marking_ledger")?,
    })
}

#[cfg(feature = "postgres")]
async fn postgres_snapshot(
    pool: &sqlx::PgPool,
    scenario_id: &'static str,
    outcomes: &[InternalChangeCommandOutcome],
) -> Result<ScenarioSnapshot, Box<dyn std::error::Error>> {
    let request_statuses = sqlx::query(
        "SELECT status FROM ai_transparency_change_requests
         WHERE change_request_id NOT LIKE 'seed-%' ORDER BY change_request_id",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.get("status"))
    .collect();
    let audit_sequence = sqlx::query(
        "SELECT sequence FROM ai_transparency_change_audit_events
         WHERE change_request_id NOT LIKE 'seed-%'
         ORDER BY change_request_id, sequence",
    )
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(|row| row.get("sequence"))
    .collect();
    Ok(ScenarioSnapshot {
        database_kind: "postgres",
        scenario_id,
        winner_count: outcomes.iter().filter(|outcome| outcome.succeeded).count(),
        request_statuses,
        target_version: sqlx::query_scalar(
            "SELECT current_version FROM ai_profile_entitlements
             WHERE license_id = 'license-qa' AND profile_id = 'profile-qa'",
        )
        .fetch_one(pool)
        .await?,
        active_version_count: sqlx::query_scalar(
            "SELECT COUNT(*)::INTEGER FROM ai_profile_entitlement_versions
             WHERE license_id = 'license-qa' AND profile_id = 'profile-qa' AND status = 'active'",
        )
        .fetch_one(pool)
        .await?,
        audit_sequence,
        reason_codes: outcome_reasons(outcomes),
        credential_count: count_postgres(pool, "ai_sdk_credential_bindings").await?,
        marking_session_count: count_postgres(pool, "ai_marking_sessions").await?,
        manifest_count: count_postgres(pool, "ai_transparency_manifests").await?,
        ledger_count: count_postgres(pool, "ai_marking_ledger").await?,
    })
}

#[cfg(feature = "postgres")]
fn outcome_reasons(outcomes: &[InternalChangeCommandOutcome]) -> Vec<String> {
    let mut reasons: Vec<_> = outcomes
        .iter()
        .filter_map(|outcome| outcome.reason_code.clone())
        .collect();
    reasons.sort();
    reasons
}

#[cfg(any())]
fn sqlite_strings(conn: &Connection, sql: &str) -> Result<Vec<String>, rusqlite::Error> {
    conn.prepare(sql)?
        .query_map([], |row| row.get(0))?
        .collect()
}

#[cfg(any())]
fn sqlite_i32s(conn: &Connection, sql: &str) -> Result<Vec<i32>, rusqlite::Error> {
    conn.prepare(sql)?
        .query_map([], |row| row.get(0))?
        .collect()
}

#[cfg(any())]
fn count_sqlite(conn: &Connection, table: &str) -> Result<i64, rusqlite::Error> {
    conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })
}

#[cfg(feature = "postgres")]
async fn count_postgres(pool: &sqlx::PgPool, table: &str) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
        .fetch_one(pool)
        .await
}

#[cfg(feature = "postgres")]
fn assert_expected(
    snapshot: &ScenarioSnapshot,
    expected_winners: usize,
    expected_reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if snapshot.winner_count != expected_winners {
        return Err(format!(
            "{} {} expected {expected_winners} winner(s), got {}",
            snapshot.database_kind, snapshot.scenario_id, snapshot.winner_count
        )
        .into());
    }
    let expected_reasons = 2usize.saturating_sub(expected_winners);
    if snapshot.reason_codes.len() != expected_reasons
        || snapshot
            .reason_codes
            .iter()
            .any(|reason| reason != expected_reason)
    {
        return Err(format!(
            "{} {} expected reason {}, got {:?}",
            snapshot.database_kind, snapshot.scenario_id, expected_reason, snapshot.reason_codes
        )
        .into());
    }
    if snapshot.credential_count != 0
        || snapshot.marking_session_count != 0
        || snapshot.manifest_count != 0
        || snapshot.ledger_count != 0
    {
        return Err(format!(
            "{} {} produced forbidden production side effects",
            snapshot.database_kind, snapshot.scenario_id
        )
        .into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
fn assert_preflight_rejection(
    snapshot: &ScenarioSnapshot,
    expected_reason: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if snapshot.winner_count != 0
        || snapshot.reason_codes != [expected_reason]
        || !snapshot.request_statuses.is_empty()
        || !snapshot.audit_sequence.is_empty()
        || snapshot.target_version != 1
        || snapshot.active_version_count != 1
        || snapshot.credential_count != 0
        || snapshot.marking_session_count != 0
        || snapshot.manifest_count != 0
        || snapshot.ledger_count != 0
    {
        return Err(format!(
            "{} {} did not fail closed with zero writes: {snapshot:?}",
            snapshot.database_kind, snapshot.scenario_id
        )
        .into());
    }
    Ok(())
}

#[cfg(any())]
fn remove_sqlite_files(path: &Path) {
    for candidate in [
        path.to_path_buf(),
        PathBuf::from(format!("{}-wal", path.display())),
        PathBuf::from(format!("{}-shm", path.display())),
    ] {
        let _ = std::fs::remove_file(candidate);
    }
}

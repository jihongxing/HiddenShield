#[cfg(feature = "postgres")]
use sqlx::{Executor, Row};

#[cfg(feature = "postgres")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    use hiddenshield_feedback_backend::database::{
        POSTGRES_P10_AI_TRANSPARENCY_ADAPTER_RECEIPTS_DOWN_SQL,
        POSTGRES_P10_AI_TRANSPARENCY_ADAPTER_RECEIPTS_UP_SQL,
        POSTGRES_P11_AI_TRANSPARENCY_RECOVERY_WORKER_DOWN_SQL,
        POSTGRES_P11_AI_TRANSPARENCY_RECOVERY_WORKER_UP_SQL,
        POSTGRES_P12_AI_TRANSPARENCY_DEAD_LETTER_REQUEUE_DOWN_SQL,
        POSTGRES_P12_AI_TRANSPARENCY_DEAD_LETTER_REQUEUE_UP_SQL,
        POSTGRES_P13_AI_TRANSPARENCY_CONFIRMED_DELIVERY_ENVELOPE_DOWN_SQL,
        POSTGRES_P13_AI_TRANSPARENCY_CONFIRMED_DELIVERY_ENVELOPE_UP_SQL,
        POSTGRES_P14_AI_TRANSPARENCY_DELIVERY_RETRIEVAL_DOWN_SQL,
        POSTGRES_P14_AI_TRANSPARENCY_DELIVERY_RETRIEVAL_UP_SQL,
        POSTGRES_P15_AI_TRANSPARENCY_DELIVERY_REVOKE_RESOURCE_BUDGET_DOWN_SQL,
        POSTGRES_P15_AI_TRANSPARENCY_DELIVERY_REVOKE_RESOURCE_BUDGET_UP_SQL,
        POSTGRES_P16_AI_TRANSPARENCY_DELIVERY_SECURITY_OBSERVABILITY_DOWN_SQL,
        POSTGRES_P16_AI_TRANSPARENCY_DELIVERY_SECURITY_OBSERVABILITY_UP_SQL,
        POSTGRES_P17_AI_TRANSPARENCY_DELIVERY_SECURITY_INCIDENT_RUNNER_DOWN_SQL,
        POSTGRES_P17_AI_TRANSPARENCY_DELIVERY_SECURITY_INCIDENT_RUNNER_UP_SQL,
        POSTGRES_P18_AI_TRANSPARENCY_DELIVERY_SECURITY_NOTIFICATION_OUTBOX_DOWN_SQL,
        POSTGRES_P18_AI_TRANSPARENCY_DELIVERY_SECURITY_NOTIFICATION_OUTBOX_UP_SQL,
        POSTGRES_P19_AI_TRANSPARENCY_NOTIFICATION_DELIVERY_GATE_DOWN_SQL,
        POSTGRES_P19_AI_TRANSPARENCY_NOTIFICATION_DELIVERY_GATE_UP_SQL,
        POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL,
        POSTGRES_P20_AI_TRANSPARENCY_PLATFORM_API_DOWN_SQL,
        POSTGRES_P20_AI_TRANSPARENCY_PLATFORM_API_UP_SQL,
        POSTGRES_P21_AI_TRANSPARENCY_PUBLIC_RESOLVER_DOWN_SQL,
        POSTGRES_P21_AI_TRANSPARENCY_PUBLIC_RESOLVER_UP_SQL,
        POSTGRES_P22_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_INTAKE_DOWN_SQL,
        POSTGRES_P22_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_INTAKE_UP_SQL,
        POSTGRES_P23_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_REVIEW_DOWN_SQL,
        POSTGRES_P23_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_REVIEW_UP_SQL,
        POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_DOWN_SQL, POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL,
        POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_DOWN_SQL,
        POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
        POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_DOWN_SQL,
        POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_UP_SQL,
        POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_DOWN_SQL,
        POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_UP_SQL,
        POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_DOWN_SQL,
        POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_UP_SQL,
        POSTGRES_P8_AI_TRANSPARENCY_POST_EMBED_SIGNING_DOWN_SQL,
        POSTGRES_P8_AI_TRANSPARENCY_POST_EMBED_SIGNING_UP_SQL,
        POSTGRES_P9_AI_TRANSPARENCY_SIGNING_RESERVATION_DOWN_SQL,
        POSTGRES_P9_AI_TRANSPARENCY_SIGNING_RESERVATION_UP_SQL,
    };
    use sqlx::PgPool;

    let database_url = std::env::var("HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL")
        .or_else(|_| std::env::var("DATABASE_URL"))
        .map_err(|_| {
            "missing HIDDENSHIELD_POSTGRES_TEST_DATABASE_URL or DATABASE_URL for disposable Postgres smoke"
        })?;

    if !is_safe_smoke_url(&database_url) {
        return Err(
            "refusing to run smoke against non-disposable database URL; include localhost/127.0.0.1 and hiddenshield_migrate_smoke in the URL"
                .into(),
        );
    }

    let pool = PgPool::connect(&database_url).await?;
    let required_tables = [
        "schema_migrations",
        "cloud_accounts",
        "cloud_devices",
        "cloud_sessions",
        "auth_challenges",
        "auth_attempts",
        "cloud_sync_events",
        "cloud_device_cursors",
        "watermark_id_registry",
        "watermark_id_reissue_jobs",
        "rights_manifests",
        "ai_transparency_licenses",
        "ai_profile_entitlements",
        "ai_sdk_credential_bindings",
        "ai_marking_sessions",
        "ai_transparency_manifests",
        "ai_claim_evidence",
        "ai_marker_bindings",
        "ai_explicit_label_receipts",
        "ai_marking_ledger",
        "ai_transparency_admin_audit_events",
        "ai_transparency_actor_role_snapshots",
        "ai_transparency_change_requests",
        "ai_profile_entitlement_versions",
        "ai_transparency_change_approvals",
        "ai_transparency_change_executions",
        "ai_transparency_change_audit_events",
        "ai_transparency_change_target_locks",
        "ai_marking_confirm_audit_events",
        "ai_runtime_credential_audit_events",
        "ai_credential_lifecycle_audit_events",
        "ai_post_embed_signing_executions",
        "ai_post_embed_signing_audit_events",
        "ai_post_embed_recovery_audit_events",
        "ai_post_embed_dead_letter_inspection_audit_events",
        "ai_post_embed_delivery_envelopes",
        "ai_delivery_retrieval_authorizations",
        "ai_delivery_download_audit_events",
        "ai_delivery_download_rate_limit_windows",
        "ai_delivery_security_observability_snapshots",
        "ai_delivery_security_operations_audit_events",
        "ai_delivery_security_incidents",
        "ai_delivery_security_incident_audit_events",
        "ai_delivery_security_cleanup_schedules",
        "ai_delivery_security_cleanup_runner_audit_events",
        "ai_delivery_security_incident_inspection_audit_events",
        "ai_delivery_security_notification_outbox",
        "ai_delivery_security_notification_outbox_audit_events",
        "ai_delivery_security_notification_provider_receipts",
        "ai_platform_profile_admissions",
        "ai_platform_marking_sessions",
        "ai_platform_marking_submissions",
        "ai_platform_api_audit_events",
        "ai_transparency_external_evidence_intakes",
        "ai_transparency_external_evidence_intake_audit_events",
        "ai_transparency_external_evidence_review_decisions",
        "ai_transparency_external_evidence_review_audit_events",
    ];
    let required_indexes = [
        "idx_auth_challenges_identifier_created",
        "idx_auth_attempts_identifier_created",
        "idx_cloud_sync_events_account_sequence",
        "idx_watermark_id_registry_account_workspace",
        "idx_watermark_id_registry_parent",
        "idx_watermark_id_reissue_jobs_account",
        "idx_rights_manifests_one_active",
        "idx_rights_manifests_watermark",
        "idx_rights_manifests_watermark_status",
        "idx_rights_manifests_watermark_version",
        "idx_rights_manifests_status_updated",
        "idx_ai_transparency_licenses_one_active",
        "idx_ai_profile_entitlements_license_status",
        "idx_ai_sdk_credential_bindings_license_status",
        "idx_ai_marking_sessions_license_status",
        "idx_ai_transparency_manifests_one_active",
        "idx_ai_transparency_manifests_watermark_status",
        "idx_ai_claim_evidence_manifest",
        "idx_ai_marker_bindings_manifest",
        "idx_ai_explicit_label_receipts_manifest",
        "idx_ai_marking_ledger_license_status",
        "idx_ai_transparency_admin_audit_events_license_time",
        "idx_ai_actor_role_snapshots_actor_scope",
        "idx_ai_change_requests_one_inflight_target",
        "idx_ai_change_requests_scope_status",
        "idx_ai_profile_entitlement_versions_one_active",
        "idx_ai_profile_entitlement_versions_history",
        "idx_ai_change_audit_events_request_sequence",
        "idx_ai_marking_confirm_audit_license_time",
        "idx_ai_sdk_credentials_key_prefix",
        "idx_ai_sdk_credentials_key_hash",
        "idx_ai_runtime_credential_audit_license_time",
        "idx_ai_sdk_credentials_rotated_from",
        "idx_ai_credential_lifecycle_audit_license_time",
        "idx_ai_post_embed_signing_license_time",
        "idx_ai_post_embed_signing_audit_execution_time",
        "idx_ai_post_embed_signing_invocation_key",
        "idx_ai_post_embed_signing_active_lease",
        "idx_ai_post_embed_signing_artifact_pending",
        "idx_ai_post_embed_signing_billable_invocation",
        "idx_ai_post_embed_signing_artifact_stage_receipt",
        "idx_ai_post_embed_signing_artifact_finalize_receipt",
        "idx_ai_post_embed_recovery_due",
        "idx_ai_post_embed_recovery_lease_expiry",
        "idx_ai_post_embed_recovery_audit_execution_time",
        "idx_ai_post_embed_dead_letter_inspection_execution_time",
        "idx_ai_post_embed_delivery_created",
        "idx_ai_delivery_retrieval_authorization_expiry",
        "idx_ai_delivery_download_audit_envelope_time",
        "idx_ai_delivery_download_rate_limit_updated",
        "idx_ai_delivery_security_summary_scope_time",
        "idx_ai_delivery_security_summary_retention",
        "idx_ai_delivery_security_operations_scope_time",
        "idx_ai_delivery_security_incidents_scope_status",
        "idx_ai_delivery_security_incidents_latest_summary",
        "idx_ai_delivery_security_incident_audit_incident_time",
        "idx_ai_delivery_security_cleanup_schedules_due",
        "idx_ai_delivery_security_cleanup_runner_schedule_time",
        "idx_ai_delivery_security_incident_inspection_scope_time",
        "idx_ai_delivery_security_notification_outbox_due",
        "idx_ai_delivery_security_notification_outbox_incident",
        "idx_ai_delivery_security_notification_outbox_audit_item_time",
        "idx_ai_delivery_security_notification_completion_idempotency",
        "idx_ai_delivery_security_notification_dead_letter",
        "idx_ai_delivery_security_notification_provider_receipt_id",
        "idx_ai_delivery_security_notification_provider_receipt_item",
        "idx_ai_platform_admissions_license_status",
        "idx_ai_platform_api_audit_license_time",
    ];
    let required_views = [
        "ai_public_confirmed_manifests",
        "ai_public_confirmed_markers",
        "ai_public_confirmed_evidence_summary",
    ];

    assert_tables_absent(&pool, &required_tables).await?;
    assert_views_absent(&pool, &required_views).await?;
    execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_UP_SQL).await?;
    execute_sql_batch(&pool, POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_UP_SQL).await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_UP_SQL,
    )
    .await?;
    execute_sql_batch(&pool, POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_UP_SQL).await?;
    execute_sql_batch(&pool, POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_UP_SQL).await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_UP_SQL,
    )
    .await?;
    execute_sql_batch(&pool, POSTGRES_P8_AI_TRANSPARENCY_POST_EMBED_SIGNING_UP_SQL).await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P9_AI_TRANSPARENCY_SIGNING_RESERVATION_UP_SQL,
    )
    .await?;
    execute_sql_batch(&pool, POSTGRES_P10_AI_TRANSPARENCY_ADAPTER_RECEIPTS_UP_SQL).await?;
    execute_sql_batch(&pool, POSTGRES_P11_AI_TRANSPARENCY_RECOVERY_WORKER_UP_SQL).await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P12_AI_TRANSPARENCY_DEAD_LETTER_REQUEUE_UP_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P13_AI_TRANSPARENCY_CONFIRMED_DELIVERY_ENVELOPE_UP_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P14_AI_TRANSPARENCY_DELIVERY_RETRIEVAL_UP_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P15_AI_TRANSPARENCY_DELIVERY_REVOKE_RESOURCE_BUDGET_UP_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P16_AI_TRANSPARENCY_DELIVERY_SECURITY_OBSERVABILITY_UP_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P17_AI_TRANSPARENCY_DELIVERY_SECURITY_INCIDENT_RUNNER_UP_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P18_AI_TRANSPARENCY_DELIVERY_SECURITY_NOTIFICATION_OUTBOX_UP_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P19_AI_TRANSPARENCY_NOTIFICATION_DELIVERY_GATE_UP_SQL,
    )
    .await?;
    execute_sql_batch(&pool, POSTGRES_P20_AI_TRANSPARENCY_PLATFORM_API_UP_SQL).await?;
    execute_sql_batch(&pool, POSTGRES_P21_AI_TRANSPARENCY_PUBLIC_RESOLVER_UP_SQL).await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P22_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_INTAKE_UP_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P23_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_REVIEW_UP_SQL,
    )
    .await?;
    assert_tables_present(&pool, &required_tables).await?;
    assert_indexes_present(&pool, &required_indexes).await?;
    assert_views_present(&pool, &required_views).await?;
    assert_column_type(&pool, "cloud_sync_events", "sequence", "bigint").await?;
    assert_column_type(&pool, "cloud_sync_events", "payload_json", "jsonb").await?;
    assert_column_type(&pool, "cloud_devices", "registered", "boolean").await?;
    assert_column_type(
        &pool,
        "cloud_accounts",
        "created_at",
        "timestamp with time zone",
    )
    .await?;
    assert_partial_index(
        &pool,
        "idx_rights_manifests_one_active",
        "WHERE (status = 'active'",
    )
    .await?;
    assert_partial_index(
        &pool,
        "idx_ai_transparency_licenses_one_active",
        "WHERE (status = 'active'",
    )
    .await?;
    assert_partial_index(
        &pool,
        "idx_ai_transparency_manifests_one_active",
        "WHERE (status = 'active'",
    )
    .await?;
    assert_ai_transparency_constraints(&pool).await?;
    assert_ai_transparency_approval_constraints(&pool).await?;

    execute_sql_batch(
        &pool,
        POSTGRES_P23_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_REVIEW_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P22_AI_TRANSPARENCY_EXTERNAL_EVIDENCE_INTAKE_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(&pool, POSTGRES_P21_AI_TRANSPARENCY_PUBLIC_RESOLVER_DOWN_SQL).await?;
    execute_sql_batch(&pool, POSTGRES_P20_AI_TRANSPARENCY_PLATFORM_API_DOWN_SQL).await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P19_AI_TRANSPARENCY_NOTIFICATION_DELIVERY_GATE_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P18_AI_TRANSPARENCY_DELIVERY_SECURITY_NOTIFICATION_OUTBOX_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P17_AI_TRANSPARENCY_DELIVERY_SECURITY_INCIDENT_RUNNER_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P16_AI_TRANSPARENCY_DELIVERY_SECURITY_OBSERVABILITY_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P15_AI_TRANSPARENCY_DELIVERY_REVOKE_RESOURCE_BUDGET_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P14_AI_TRANSPARENCY_DELIVERY_RETRIEVAL_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P13_AI_TRANSPARENCY_CONFIRMED_DELIVERY_ENVELOPE_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P12_AI_TRANSPARENCY_DEAD_LETTER_REQUEUE_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(&pool, POSTGRES_P11_AI_TRANSPARENCY_RECOVERY_WORKER_DOWN_SQL).await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P10_AI_TRANSPARENCY_ADAPTER_RECEIPTS_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P9_AI_TRANSPARENCY_SIGNING_RESERVATION_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P8_AI_TRANSPARENCY_POST_EMBED_SIGNING_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P7_AI_TRANSPARENCY_CREDENTIAL_LIFECYCLE_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P6_AI_TRANSPARENCY_CREDENTIAL_CUSTODY_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(&pool, POSTGRES_P5_AI_TRANSPARENCY_CONFIRM_AUDIT_DOWN_SQL).await?;
    execute_sql_batch(
        &pool,
        POSTGRES_P4_AI_TRANSPARENCY_APPROVAL_STATE_MACHINE_DOWN_SQL,
    )
    .await?;
    execute_sql_batch(&pool, POSTGRES_P3_AI_TRANSPARENCY_SCHEMA_DOWN_SQL).await?;
    execute_sql_batch(&pool, POSTGRES_P1_AUTH_SYNC_REGISTRY_DOWN_SQL).await?;
    assert_tables_absent(&pool, &required_tables).await?;
    assert_indexes_absent(&pool, &required_indexes).await?;
    assert_views_absent(&pool, &required_views).await?;

    println!(
        "{}",
        serde_json::json!({
            "ok": true,
            "migrations": ["0001_auth_sync_registry", "0002_ai_transparency_schema", "0003_ai_transparency_approval_state_machine", "0004_ai_transparency_confirm_audit", "0005_ai_transparency_credential_custody", "0006_ai_transparency_credential_lifecycle", "0007_ai_transparency_post_embed_signing", "0008_ai_transparency_signing_reservation_artifact_recovery", "0009_ai_transparency_adapter_receipts_crash_recovery", "0010_ai_transparency_post_embed_recovery_worker", "0011_ai_transparency_dead_letter_requeue_command", "0012_ai_transparency_confirmed_delivery_envelope", "0013_ai_transparency_delivery_authorization_retrieval", "0014_ai_transparency_delivery_revoke_resource_budget", "0015_ai_transparency_delivery_security_observability", "0016_ai_transparency_delivery_security_incident_runner", "0017_ai_transparency_delivery_security_notification_outbox", "0018_ai_transparency_notification_delivery_gate", "0019_ai_transparency_platform_api", "0020_ai_transparency_public_resolver", "0021_ai_transparency_external_evidence_intake", "0022_ai_transparency_external_evidence_review"],
            "upTablesChecked": required_tables.len(),
            "viewsChecked": required_views.len(),
            "indexesChecked": required_indexes.len(),
            "constraintRegressionsChecked": 14,
            "rollback": "empty_schema_verified"
        })
    );
    Ok(())
}

#[cfg(not(feature = "postgres"))]
fn main() {
    eprintln!("postgres_migrate_smoke requires --features postgres");
    std::process::exit(2);
}

#[cfg(feature = "postgres")]
fn is_safe_smoke_url(database_url: &str) -> bool {
    let lower = database_url.to_ascii_lowercase();
    (lower.contains("localhost") || lower.contains("127.0.0.1"))
        && lower.contains("hiddenshield_migrate_smoke")
}

#[cfg(feature = "postgres")]
async fn execute_sql_batch(pool: &sqlx::PgPool, sql: &str) -> Result<(), sqlx::Error> {
    sqlx::raw_sql(sql).execute(pool).await.map(|_| ())
}

#[cfg(feature = "postgres")]
async fn assert_ai_transparency_constraints(
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    pool.execute(
        "INSERT INTO ai_transparency_licenses (
            license_id, tenant_id, workspace_id, environment, status, issuer_mode,
            deployment_mode, public_verification_required, metering_plan_id,
            effective_at, expires_at, created_at, updated_at
        ) VALUES (
            'lic-smoke-primary', 'tenant-smoke', 'workspace-smoke', 'production', 'active',
            'hiddenshield_managed', 'hosted', TRUE, 'metering-smoke',
            NOW(), NOW() + INTERVAL '1 day', NOW(), NOW()
        )",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "active license uniqueness",
        "INSERT INTO ai_transparency_licenses (
            license_id, tenant_id, workspace_id, environment, status, issuer_mode,
            deployment_mode, public_verification_required, metering_plan_id,
            effective_at, expires_at, created_at, updated_at
        ) VALUES (
            'lic-smoke-duplicate', 'tenant-smoke', 'workspace-smoke', 'production', 'active',
            'hiddenshield_managed', 'hosted', TRUE, 'metering-smoke',
            NOW(), NOW() + INTERVAL '1 day', NOW(), NOW()
        )",
    )
    .await?;

    pool.execute(
        "INSERT INTO ai_profile_entitlements (
            license_id, profile_id, profile_kind, status, effective_at, expires_at,
            terms_version, approved_by, created_at, updated_at
        ) VALUES (
            'lic-smoke-primary', 'cn-image-v1', 'regulatory', 'active', NOW(),
            NOW() + INTERVAL '1 day', 'v1', 'smoke', NOW(), NOW()
        )",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "profile entitlement uniqueness",
        "INSERT INTO ai_profile_entitlements (
            license_id, profile_id, profile_kind, status, effective_at, expires_at,
            terms_version, approved_by, created_at, updated_at
        ) VALUES (
            'lic-smoke-primary', 'cn-image-v1', 'regulatory', 'active', NOW(),
            NOW() + INTERVAL '1 day', 'v1', 'smoke', NOW(), NOW()
        )",
    )
    .await?;

    insert_smoke_marking_session(pool, "session-smoke-primary", "idem-smoke-primary").await?;
    assert_statement_rejected(
        pool,
        "marking session idempotency",
        "INSERT INTO ai_marking_sessions (
            marking_session_id, license_id, tenant_id, workspace_id, environment,
            idempotency_key, requested_profile_ids_json, claim_type, status, expires_at,
            created_at, updated_at
        ) VALUES (
            'session-smoke-duplicate', 'lic-smoke-primary', 'tenant-smoke', 'workspace-smoke',
            'production', 'idem-smoke-primary', '[\"cn-image-v1\"]'::jsonb, 'ai_generated',
            'ready_to_confirm', NOW() + INTERVAL '1 day', NOW(), NOW()
        )",
    )
    .await?;

    insert_smoke_marking_session(pool, "session-smoke-secondary", "idem-smoke-secondary").await?;
    insert_smoke_manifest(
        pool,
        "manifest-smoke-primary",
        "session-smoke-primary",
        "HS-00000000-00000000-00000000-00000001",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "active manifest uniqueness",
        "INSERT INTO ai_transparency_manifests (
            transparency_manifest_id, marking_session_id, watermark_uid, manifest_version,
            status, claim_type, modality, generation_mode, provider_id, system_name,
            system_version, operations_json, generated_at, subject_digest_algorithm,
            subject_digest_scope, subject_digest, parent_subjects_json, manifest_sha256,
            created_at, updated_at
        ) VALUES (
            'manifest-smoke-duplicate', 'session-smoke-secondary',
            'HS-00000000-00000000-00000000-00000001', 1, 'active', 'ai_generated', 'image',
            'generated', 'smoke-provider', 'smoke-system', 'v1', '[]'::jsonb, NOW(), 'sha256',
            'protected_output', repeat('a', 64), '[]'::jsonb, repeat('b', 64), NOW(), NOW()
        )",
    )
    .await?;

    pool.execute(
        "INSERT INTO ai_marking_ledger (
            ledger_entry_id, license_id, marking_session_id, transparency_manifest_id,
            metering_unit, quantity, ledger_status, committed_at, created_at
        ) VALUES (
            'ledger-smoke-primary', 'lic-smoke-primary', 'session-smoke-primary',
            'manifest-smoke-primary', 'confirmed_marked_image', 1, 'committed', NOW(), NOW()
        )",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "marking ledger session and manifest uniqueness",
        "INSERT INTO ai_marking_ledger (
            ledger_entry_id, license_id, marking_session_id, transparency_manifest_id,
            metering_unit, quantity, ledger_status, committed_at, created_at
        ) VALUES (
            'ledger-smoke-duplicate', 'lic-smoke-primary', 'session-smoke-primary',
            'manifest-smoke-primary', 'confirmed_marked_image', 1, 'committed', NOW(), NOW()
        )",
    )
    .await?;

    insert_smoke_marking_session(pool, "session-smoke-metering", "idem-smoke-metering").await?;
    insert_smoke_manifest(
        pool,
        "manifest-smoke-metering",
        "session-smoke-metering",
        "HS-00000000-00000000-00000000-00000002",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "confirmed image metering quantity",
        "INSERT INTO ai_marking_ledger (
            ledger_entry_id, license_id, marking_session_id, transparency_manifest_id,
            metering_unit, quantity, ledger_status, created_at
        ) VALUES (
            'ledger-smoke-invalid-quantity', 'lic-smoke-primary', 'session-smoke-metering',
            'manifest-smoke-metering', 'confirmed_marked_image', 2, 'pending', NOW()
        )",
    )
    .await?;

    assert_statement_rejected(
        pool,
        "explicit exported-file label digest",
        "INSERT INTO ai_explicit_label_receipts (
            receipt_id, transparency_manifest_id, profile_id, required_surface, render_mode,
            placement_json, locale, label_text, applied_at, applied_by, verification_status,
            created_at
        ) VALUES (
            'receipt-smoke-invalid-digest', 'manifest-smoke-primary', 'cn-image-v1',
            'exported_file', 'overlay', '{}'::jsonb, 'zh-CN', 'AI generated', NOW(), 'smoke',
            'pending', NOW()
        )",
    )
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn insert_smoke_marking_session(
    pool: &sqlx::PgPool,
    marking_session_id: &str,
    idempotency_key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query(
        "INSERT INTO ai_marking_sessions (
            marking_session_id, license_id, tenant_id, workspace_id, environment,
            idempotency_key, requested_profile_ids_json, claim_type, status, expires_at,
            created_at, updated_at
        ) VALUES ($1, 'lic-smoke-primary', 'tenant-smoke', 'workspace-smoke', 'production',
            $2, '[\"cn-image-v1\"]'::jsonb, 'ai_generated', 'ready_to_confirm',
            NOW() + INTERVAL '1 day', NOW(), NOW())",
    )
    .bind(marking_session_id)
    .bind(idempotency_key)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_ai_transparency_approval_constraints(
    pool: &sqlx::PgPool,
) -> Result<(), Box<dyn std::error::Error>> {
    pool.execute(
        "INSERT INTO ai_transparency_actor_role_snapshots (
            actor_role_snapshot_id, actor_id, actor_type, role, tenant_id, workspace_id,
            environment, role_binding_id, role_binding_version, source_identity_system,
            authentication_level, captured_at, source_expires_at, snapshot_sha256
        ) VALUES
        ('approval-requester', 'approval-requester', 'human', 'ai_transparency_requester',
            'tenant-smoke', 'workspace-smoke', 'production', 'binding-requester', 1,
            'hiddenshield_internal_iam', 'mfa', NOW(), NOW() + INTERVAL '1 day', repeat('c', 64)),
        ('approval-approver', 'approval-approver', 'human', 'ai_transparency_compliance_approver',
            'tenant-smoke', 'workspace-smoke', 'production', 'binding-approver', 1,
            'hiddenshield_internal_iam', 'mfa', NOW(), NOW() + INTERVAL '1 day', repeat('d', 64)),
        ('approval-executor', 'approval-executor', 'system', 'system_executor',
            'tenant-smoke', 'workspace-smoke', 'production', 'binding-executor', 1,
            'hiddenshield_internal_iam', 'system', NOW(), NOW() + INTERVAL '1 day', repeat('e', 64))",
    )
    .await?;
    pool.execute(
        "INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_id, target_scope_key,
            tenant_id, workspace_id, environment, expected_target_version,
            desired_next_version, desired_state_json, request_reason,
            security_review_reference, requester_snapshot_id, request_digest_version,
            request_digest, idempotency_key, status, expires_at, created_at, updated_at
        ) VALUES (
            'dead-letter-request-smoke', 'requeue_post_embed_dead_letter',
            'post_embed_recovery', 'execution-dead-letter-smoke',
            'post_embed_recovery:execution-dead-letter-smoke',
            'tenant-smoke', 'workspace-smoke', 'production', 1, 2,
            '{\"recoveryState\":\"retry_scheduled\"}'::jsonb, 'dead-letter smoke',
            'security-smoke', 'approval-requester',
            'hs-ai-post-embed-dead-letter-requeue-digest-v1', repeat('b', 64),
            'idem-dead-letter-smoke', 'succeeded', NOW() + INTERVAL '1 day', NOW(), NOW()
        )",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "dead-letter request digest version",
        "INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_id, target_scope_key,
            tenant_id, workspace_id, environment, expected_target_version,
            desired_next_version, desired_state_json, request_reason,
            security_review_reference, requester_snapshot_id, request_digest_version,
            request_digest, idempotency_key, status, expires_at, created_at, updated_at
        ) VALUES (
            'dead-letter-request-invalid-digest-version', 'requeue_post_embed_dead_letter',
            'post_embed_recovery', 'execution-dead-letter-invalid',
            'post_embed_recovery:execution-dead-letter-invalid',
            'tenant-smoke', 'workspace-smoke', 'production', 1, 2,
            '{\"recoveryState\":\"retry_scheduled\"}'::jsonb, 'dead-letter invalid digest',
            'security-smoke', 'approval-requester', 'unsupported-digest-v1', repeat('b', 64),
            'idem-dead-letter-invalid', 'succeeded', NOW() + INTERVAL '1 day', NOW(), NOW()
        )",
    )
    .await?;
    pool.execute(
        "DELETE FROM ai_transparency_change_requests
         WHERE change_request_id = 'dead-letter-request-smoke'",
    )
    .await?;
    pool.execute(
        "INSERT INTO ai_profile_entitlements (
            license_id, profile_id, profile_kind, status, effective_at, expires_at,
            terms_version, approved_by, created_at, updated_at
        ) VALUES (
            'lic-smoke-primary', 'profile-smoke', 'regulatory', 'active', NOW(),
            NOW() + INTERVAL '1 day', 'v1', 'legacy-smoke', NOW(), NOW()
        )",
    )
    .await?;
    pool.execute(
        "INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_id, target_scope_key,
            tenant_id, workspace_id, environment, expected_target_version, desired_next_version,
            desired_state_json, request_reason, legal_review_reference, requester_snapshot_id,
            request_digest_version, request_digest, idempotency_key, status, expires_at,
            created_at, updated_at
        ) VALUES (
            'approval-request-primary', 'renew_profile_entitlement', 'profile_entitlement',
            'profile-smoke', 'profile:lic-smoke-primary:profile-smoke', 'tenant-smoke',
            'workspace-smoke', 'production', 1, 2, '{\"status\":\"active\"}'::jsonb,
            'smoke approval request', 'legal-smoke', 'approval-requester',
            'hs-ai-change-request-digest-v1', repeat('f', 64), 'idem-primary',
            'pending_review', NOW() + INTERVAL '1 day', NOW(), NOW()
        )",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "requester and approver separation",
        "INSERT INTO ai_transparency_change_approvals (
            approval_id, change_request_id, decision, approver_snapshot_id,
            requester_actor_id, approver_actor_id, approver_role, decision_reason,
            policy_version, request_digest, decided_at
        ) VALUES (
            'approval-self', 'approval-request-primary', 'approved', 'approval-requester',
            'approval-requester', 'approval-requester', 'ai_transparency_requester',
            'self approval', 'v1', repeat('f', 64), NOW()
        )",
    )
    .await?;
    pool.execute(
        "INSERT INTO ai_transparency_change_approvals (
            approval_id, change_request_id, decision, approver_snapshot_id,
            requester_actor_id, approver_actor_id, approver_role, decision_reason,
            policy_version, request_digest, decided_at
        ) VALUES (
            'approval-primary', 'approval-request-primary', 'approved', 'approval-approver',
            'approval-requester', 'approval-approver', 'ai_transparency_compliance_approver',
            'approved by smoke checker', 'v1', repeat('f', 64), NOW()
        )",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "one approval per request",
        "INSERT INTO ai_transparency_change_approvals (
            approval_id, change_request_id, decision, approver_snapshot_id,
            requester_actor_id, approver_actor_id, approver_role, decision_reason,
            policy_version, request_digest, decided_at
        ) VALUES (
            'approval-duplicate', 'approval-request-primary', 'rejected', 'approval-approver',
            'approval-requester', 'approval-approver', 'ai_transparency_compliance_approver',
            'duplicate', 'v1', repeat('f', 64), NOW()
        )",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "one in-flight request per target",
        "INSERT INTO ai_transparency_change_requests (
            change_request_id, operation, target_type, target_scope_key, tenant_id, workspace_id,
            environment, desired_state_json, request_reason, requester_snapshot_id,
            request_digest_version, request_digest, idempotency_key, status, expires_at, created_at, updated_at
        ) VALUES (
            'approval-request-conflict', 'renew_profile_entitlement', 'profile_entitlement',
            'profile:lic-smoke-primary:profile-smoke', 'tenant-smoke', 'workspace-smoke',
            'production', '{\"status\":\"active\"}'::jsonb, 'conflict', 'approval-requester',
            'hs-ai-change-request-digest-v1', repeat('a', 64), 'idem-conflict',
            'pending_review', NOW() + INTERVAL '1 day', NOW(), NOW()
        )",
    )
    .await?;
    pool.execute(
        "INSERT INTO ai_profile_entitlement_versions (
            profile_entitlement_version_id, license_id, profile_id, version, profile_kind, status,
            effective_at, expires_at, terms_version, legal_review_reference, source_change_request_id, created_at
        ) VALUES (
            'profile-version-primary', 'lic-smoke-primary', 'profile-smoke', 1, 'regulatory',
            'active', NOW(), NOW() + INTERVAL '1 day', 'v1', 'legal-smoke',
            'approval-request-primary', NOW()
        )",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "one active entitlement version",
        "INSERT INTO ai_profile_entitlement_versions (
            profile_entitlement_version_id, license_id, profile_id, version, profile_kind, status,
            effective_at, expires_at, terms_version, legal_review_reference, source_change_request_id, created_at
        ) VALUES (
            'profile-version-duplicate', 'lic-smoke-primary', 'profile-smoke', 2, 'regulatory',
            'active', NOW(), NOW() + INTERVAL '1 day', 'v2', 'legal-smoke',
            'approval-request-primary', NOW()
        )",
    )
    .await?;
    pool.execute(
        "INSERT INTO ai_transparency_change_executions (
            execution_id, change_request_id, executor_snapshot_id, status, target_version_before,
            target_version_after, resulting_entitlement_version_id, started_at, finished_at
        ) VALUES (
            'execution-primary', 'approval-request-primary', 'approval-executor', 'succeeded', 1, 2,
            'profile-version-primary', NOW(), NOW()
        )",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "one execution per request",
        "INSERT INTO ai_transparency_change_executions (
            execution_id, change_request_id, executor_snapshot_id, status, started_at
        ) VALUES (
            'execution-duplicate', 'approval-request-primary', 'approval-executor', 'executing', NOW()
        )",
    )
    .await?;
    pool.execute(
        "INSERT INTO ai_transparency_change_audit_events (
            audit_event_id, change_request_id, sequence, event_type, to_state, actor_snapshot_id,
            target_type, target_id, reason_code, request_digest, details_json, occurred_at
        ) VALUES (
            'audit-primary', 'approval-request-primary', 1, 'change_request_submitted', 'pending_review',
            'approval-requester', 'profile_entitlement', 'profile-smoke', 'submitted', repeat('f', 64),
            '{}'::jsonb, NOW()
        )",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "append-only audit update",
        "UPDATE ai_transparency_change_audit_events SET reason_code = 'changed' WHERE audit_event_id = 'audit-primary'",
    )
    .await?;
    assert_statement_rejected(
        pool,
        "append-only audit delete",
        "DELETE FROM ai_transparency_change_audit_events WHERE audit_event_id = 'audit-primary'",
    )
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn insert_smoke_manifest(
    pool: &sqlx::PgPool,
    transparency_manifest_id: &str,
    marking_session_id: &str,
    watermark_uid: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query(
        "INSERT INTO ai_transparency_manifests (
            transparency_manifest_id, marking_session_id, watermark_uid, manifest_version,
            status, claim_type, modality, generation_mode, provider_id, system_name,
            system_version, operations_json, generated_at, subject_digest_algorithm,
            subject_digest_scope, subject_digest, parent_subjects_json, manifest_sha256,
            created_at, updated_at
        ) VALUES ($1, $2, $3, 1, 'active', 'ai_generated', 'image', 'generated',
            'smoke-provider', 'smoke-system', 'v1', '[]'::jsonb, NOW(), 'sha256',
            'protected_output', repeat('a', 64), '[]'::jsonb, repeat('b', 64), NOW(), NOW())",
    )
    .bind(transparency_manifest_id)
    .bind(marking_session_id)
    .bind(watermark_uid)
    .execute(pool)
    .await?;
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_statement_rejected(
    pool: &sqlx::PgPool,
    constraint_name: &str,
    statement: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    if pool.execute(statement).await.is_ok() {
        return Err(format!("expected {constraint_name} constraint to reject invalid row").into());
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_tables_present(
    pool: &sqlx::PgPool,
    tables: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for table in tables {
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
            return Err(format!("expected table {table} to exist after migration up").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_tables_absent(
    pool: &sqlx::PgPool,
    tables: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for table in tables {
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
            return Err(format!("expected disposable schema to not contain table {table}").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_views_present(
    pool: &sqlx::PgPool,
    views: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for view in views {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.views
                WHERE table_schema = 'public' AND table_name = $1
            )",
        )
        .bind(view)
        .fetch_one(pool)
        .await?;
        if !exists {
            return Err(format!("expected view {view} to exist after migration up").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_views_absent(
    pool: &sqlx::PgPool,
    views: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for view in views {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM information_schema.views
                WHERE table_schema = 'public' AND table_name = $1
            )",
        )
        .bind(view)
        .fetch_one(pool)
        .await?;
        if exists {
            return Err(format!("expected disposable schema to not contain view {view}").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_indexes_present(
    pool: &sqlx::PgPool,
    indexes: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for index in indexes {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM pg_indexes
                WHERE schemaname = 'public' AND indexname = $1
            )",
        )
        .bind(index)
        .fetch_one(pool)
        .await?;
        if !exists {
            return Err(format!("expected index {index} to exist after migration up").into());
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_indexes_absent(
    pool: &sqlx::PgPool,
    indexes: &[&str],
) -> Result<(), Box<dyn std::error::Error>> {
    for index in indexes {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS (
                SELECT 1 FROM pg_indexes
                WHERE schemaname = 'public' AND indexname = $1
            )",
        )
        .bind(index)
        .fetch_one(pool)
        .await?;
        if exists {
            return Err(
                format!("expected index {index} to be dropped after migration down").into(),
            );
        }
    }
    Ok(())
}

#[cfg(feature = "postgres")]
async fn assert_column_type(
    pool: &sqlx::PgPool,
    table: &str,
    column: &str,
    expected_type: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let actual: Option<String> = sqlx::query_scalar(
        "SELECT data_type FROM information_schema.columns
         WHERE table_schema = 'public' AND table_name = $1 AND column_name = $2",
    )
    .bind(table)
    .bind(column)
    .fetch_optional(pool)
    .await?;
    match actual {
        Some(actual) if actual == expected_type => Ok(()),
        Some(actual) => {
            Err(format!("expected {table}.{column} to be {expected_type}, got {actual}").into())
        }
        None => Err(format!("missing column {table}.{column}").into()),
    }
}

#[cfg(feature = "postgres")]
async fn assert_partial_index(
    pool: &sqlx::PgPool,
    index: &str,
    expected_fragment: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let row = sqlx::query(
        "SELECT indexdef FROM pg_indexes WHERE schemaname = 'public' AND indexname = $1",
    )
    .bind(index)
    .fetch_one(pool)
    .await?;
    let indexdef: String = row.try_get("indexdef")?;
    if !indexdef.contains(expected_fragment) {
        return Err(format!(
            "partial index {index} missing fragment {expected_fragment}: {indexdef}"
        )
        .into());
    }
    Ok(())
}

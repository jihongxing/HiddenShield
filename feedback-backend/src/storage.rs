use std::collections::HashSet;
use std::path::Path;
use std::sync::Mutex;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use rusqlite::{params, params_from_iter, Connection, TransactionBehavior};
use sha2::{Digest, Sha256};

use crate::billing::{
    plan_amount_cents, BillingEvent, BillingEventType, BillingOrderStatus, BillingOrderStatusKind,
    BillingPaymentAction as ProviderPaymentAction, BillingPaymentSession,
    BillingPaymentSessionInput, BillingProvider, FixtureBillingProvider, ReportPurchaseEvent,
    ReportPurchaseEventType, ReportPurchaseOrderStatus, FIXTURE_PROVIDER, WECHAT_PAY_PROVIDER,
};
use crate::database::{
    apply_sqlite_ai_transparency_approval_state_machine, DatabaseBackendKind, DatabaseConfig,
    DatabaseConfigError,
};
use crate::schema::{
    AccountDevice, AccountDevicesResponse, AiTransparencyLicenseDecision,
    AiTransparencyLicenseDetailResponse, AiTransparencyLicenseRecord,
    AiTransparencyProfileDecision, AiTransparencyProfileEntitlementCheckRequest,
    AiTransparencyProfileEntitlementCheckResponse, AiTransparencyProfileEntitlementRecord,
    AnonymousEventOutcome, AnonymousFeedbackBatch, AnonymousFeedbackBatchAck,
    AnonymousFeedbackEvent, AnonymousFeedbackStatsQuery, AnonymousFeedbackStatsResponse,
    AuthChallengeRequest, AuthChallengeResponse, AuthLogoutRequest, AuthLogoutResponse,
    AuthRefreshRequest, AuthSessionRequest, BillingEventApplyResponse, BillingFixtureEventRequest,
    BillingPaymentAction, BillingPaymentSessionReconcileResponse, BillingPaymentSessionRequest,
    BillingPaymentSessionResponse, BillingPaymentSessionStatusResponse, CloudAccount,
    CloudAccountSession, CloudAccountSnapshot, CloudCreatorProfile, CloudDevice, CloudEntitlement,
    CloudSyncBatchRequest, CloudSyncBatchResult, CloudSyncChange, CloudSyncChangesResult,
    CloudSyncEventDisposition, CloudVideoTaskClaimRequest, CloudVideoTaskClaimResponse,
    CloudVideoTaskCompletionRequest, CloudVideoTaskFailureRequest, CloudVideoTaskListQuery,
    CloudVideoTaskListResponse, CloudVideoTaskRecord, CloudVideoTaskRequest,
    CloudVideoTaskStatusUpdateRequest, CloudWorkspace, CommercialAccountMetrics,
    CommercialAnonymousFailureRow, CommercialCloudSyncMetrics, CommercialEntitlementPlanRow,
    CommercialFeatureUsageMetrics, CommercialMetricsOverviewResponse,
    CommercialMetricsPrivacyBoundary, CommercialPaymentSessionMetrics,
    EnterpriseAdminAuditEventListResponse, EnterpriseAdminAuditEventQuery,
    EnterpriseAdminAuditEventRecord, EnterpriseApiAuditEventRequest, EnterpriseApiKeyCreateRequest,
    EnterpriseApiKeyListQuery, EnterpriseApiKeyListResponse, EnterpriseApiKeyRecord,
    EnterpriseGatewayAuditContract, EnterpriseGatewayClientFingerprint,
    EnterpriseGatewayDryRunDecision, EnterpriseGatewayDryRunRequest,
    EnterpriseGatewayQuotaChargePlan, EnterpriseGatewayRateLimitPolicy,
    EnterprisePublicRightsBatchRequest, EnterprisePublicRightsBatchResponse,
    EnterprisePublicRightsGateway, EnterpriseQuotaBalanceInitRequest, EnterpriseQuotaBalanceRecord,
    EnterpriseQuotaLedgerRecord, EnterpriseQuotaLedgerRequest, FeedbackStatRow, FeedbackTotals,
    PublicRightsBatchItem, PublicRightsBatchRequest, PublicRightsBatchResponse,
    PublicRightsMetadata, PublicRightsMetadataExport, PublicRightsQueryResponse,
    PublicRightsRegistrySnapshot, PublicRightsSignedManifestStore,
    PublicTrainingPermissionSnapshot, ReportPurchaseGrant, ReportPurchaseSessionReconcileResponse,
    ReportPurchaseSessionRequest, ReportPurchaseSessionResponse,
    ReportPurchaseSessionStatusResponse, RevokeDeviceResponse, RightsManifestBackfillItem,
    RightsManifestBackfillRequest, RightsManifestBackfillResponse, RightsManifestResponse,
    RightsManifestSummary, StatsDimension, SyncPreferencesRequest, SyncPreferencesResponse,
    TeamAuditListResponse, TeamAuditRecord, TeamMemberCreateRequest, TeamMemberListResponse,
    TeamMemberRecord, TeamMemberUpdateRequest, TeamSharedLibraryListResponse,
    TeamSharedLibraryRecord, TeamSharedLibraryShareRequest, TeamWorkspaceCreateRequest,
    TeamWorkspaceListResponse, TeamWorkspaceSummary, UpdateDeviceRequest,
    VideoFingerprintNotaryReceipt, VideoFingerprintNotaryRequest, VideoUploadManifest,
    WatermarkIdConfirmRequest, WatermarkIdReconcileRequest, WatermarkIdRegistryResponse,
    WatermarkIdReissueRequest, WatermarkIdReissueResponse, WatermarkIdReserveRequest,
    ENTERPRISE_GATEWAY_REQUIRED_STEPS, ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE,
    PUBLIC_RIGHTS_ANONYMOUS_BATCH_MAX_ITEMS,
};

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("database configuration error: {0}")]
    DatabaseConfig(#[from] DatabaseConfigError),
    #[cfg(feature = "postgres")]
    #[error("postgres database error: {0}")]
    PostgresDatabase(#[from] sqlx::Error),
    #[error("PostgreSQL storage adapter is not implemented yet")]
    PostgresAdapterNotImplemented,
    #[error("invalid retention days")]
    InvalidRetentionDays,
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("rate limited: {0}")]
    RateLimited(String),
    #[error("bad request: {0}")]
    BadRequest(String),
}

const CLOUD_VIDEO_TASK_STATUS_DRAFT: &str = "draft";
const CLOUD_VIDEO_TASK_STATUS_QUEUED: &str = "queued";
const CLOUD_VIDEO_TASK_STATUS_RUNNING: &str = "running";
const CLOUD_VIDEO_TASK_STATUS_WAITING_CLIENT_RENDER: &str = "waiting_client_render";
const CLOUD_VIDEO_TASK_STATUS_SELF_CHECKING: &str = "self_checking";
const CLOUD_VIDEO_TASK_STATUS_SUCCEEDED: &str = "succeeded";
const CLOUD_VIDEO_TASK_STATUS_FAILED: &str = "failed";
const CLOUD_VIDEO_TASK_STATUS_CANCELED: &str = "canceled";
const CLOUD_VIDEO_TASK_STATUS_EXPIRED: &str = "expired";
const CLOUD_VIDEO_TASK_CAPABILITY_HYBRID_VISUAL_WATERMARK: &str = "hybrid_visual_watermark";
const CLOUD_VIDEO_TASK_FAILURE_CODES: &[&str] = &[
    "manifest_invalid",
    "sandbox_transcode_failed",
    "strategy_invalid",
    "core_strategy_failed",
    "core_embed_failed",
    "self_check_failed",
    "registry_confirm_failed",
    "worker_receipt_invalid",
];
const L3_VIDEO_VISUAL_MAX_REGIONS: u32 = 96;
const L3_VIDEO_VISUAL_PAYLOAD_BYTES: u32 = 119;
const L3_VIDEO_VISUAL_SYNC_BITS: u32 = 16;
const L3_VIDEO_VISUAL_ECC_REPEAT: u32 = 3;
const L3_VIDEO_VISUAL_DCT_COEFF_PAIRS: u32 = 3;

pub struct Storage {
    conn: Mutex<Connection>,
    retention_days: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BillingPaymentReconcileSweep {
    pub checked: usize,
    pub succeeded: usize,
    pub pending: usize,
    pub failed: usize,
    pub skipped_unsupported_provider: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BillingPaymentSessionOrderQuery {
    pub payment_session_id: String,
    pub provider: String,
    pub provider_order_id: String,
}

#[derive(Debug, Clone)]
pub struct EnterpriseExpiredRotationCandidate {
    pub audit_event_id: String,
    pub old_api_key_id: String,
    pub new_api_key_id: Option<String>,
    pub account_id: Option<String>,
    pub workspace_id: Option<String>,
    pub rotation_deadline_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportPurchaseSessionOrderQuery {
    pub payment_session_id: String,
    pub provider: String,
    pub provider_order_id: String,
}

#[derive(Debug, Clone)]
pub struct ReportPurchaseEventApplyResponse {
    pub provider: String,
    pub provider_event_id: String,
    pub duplicate: bool,
    pub status: String,
    pub grant: Option<ReportPurchaseGrant>,
}

const REPORT_PRODUCT_COPYRIGHT_REPORT_SINGLE: &str = "copyright_report_single";
const REPORT_PRODUCT_RIGHTS_EVIDENCE_PACK_SINGLE: &str = "rights_evidence_pack_single";

pub fn dry_run_enterprise_gateway_readonly_scan(
    request: &EnterpriseGatewayDryRunRequest,
) -> EnterpriseGatewayDryRunDecision {
    let required_steps = ENTERPRISE_GATEWAY_REQUIRED_STEPS
        .iter()
        .map(|step| (*step).to_string())
        .collect::<Vec<_>>();
    let requested_units = i64::from(request.item_count);
    let idempotency_key = format!(
        "{}:{}:{}",
        request.request_id.trim(),
        request.quota_type.trim(),
        request.item_count
    );
    let client_fingerprint = normalize_enterprise_client_fingerprint(
        &request.client_fingerprint,
        request.auth.api_key_id.trim(),
    );

    let build_decision = |allowed: bool,
                          status_code: u16,
                          error_code: Option<&str>,
                          auth_decision: &str,
                          scope_decision: &str,
                          entitlement_decision: &str,
                          rate_limit_decision: &str,
                          quota_decision: &str|
     -> EnterpriseGatewayDryRunDecision {
        let chargeable_units = if allowed { requested_units } else { 0 };
        let quota = EnterpriseGatewayQuotaChargePlan {
            quota_type: request.quota_type.trim().to_string(),
            chargeable_units,
            idempotency_key: idempotency_key.clone(),
            ledger_status: if allowed { "committed" } else { "skipped" }.to_string(),
            charge_on_not_found: request.charge_on_not_found,
            charge_metadata_export: request.charge_metadata_export,
        };
        let audit = EnterpriseGatewayAuditContract {
            endpoint: request.endpoint.trim().to_string(),
            method: request.method.trim().to_ascii_uppercase(),
            request_id: request.request_id.trim().to_string(),
            request_count: 1,
            item_count: request.item_count,
            status_code,
            error_code: error_code.map(str::to_string),
            quota_units: chargeable_units,
            client_fingerprint: client_fingerprint.clone(),
            legal_conclusion: false,
        };

        EnterpriseGatewayDryRunDecision {
            allowed,
            status_code,
            error_code: error_code.map(str::to_string),
            auth_decision: auth_decision.to_string(),
            scope_decision: scope_decision.to_string(),
            entitlement_decision: entitlement_decision.to_string(),
            rate_limit_decision: rate_limit_decision.to_string(),
            quota_decision: quota_decision.to_string(),
            quota,
            audit,
            required_steps: required_steps.clone(),
            legal_conclusion: false,
        }
    };

    let api_key_id = request.auth.api_key_id.trim();
    let key_prefix = request.auth.key_prefix.trim();
    let account_id = request.auth.account_id.trim();
    let workspace_id = request.auth.workspace_id.trim();
    let status = request.auth.status.trim().to_ascii_lowercase();

    if api_key_id.is_empty() || key_prefix.is_empty() {
        return build_decision(
            false,
            401,
            Some("api_key_missing"),
            "failed:api_key_missing",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
        );
    } else if account_id.is_empty() || workspace_id.is_empty() || status.is_empty() {
        return build_decision(
            false,
            401,
            Some("api_key_invalid"),
            "failed:api_key_invalid",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
        );
    } else if status == "paused" {
        return build_decision(
            false,
            403,
            Some("api_key_paused"),
            "failed:api_key_paused",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
        );
    } else if status == "revoked" {
        return build_decision(
            false,
            403,
            Some("api_key_revoked"),
            "failed:api_key_revoked",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
        );
    } else if status == "expired" {
        return build_decision(
            false,
            403,
            Some("api_key_expired"),
            "failed:api_key_expired",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
        );
    } else if status != "active" {
        return build_decision(
            false,
            401,
            Some("api_key_invalid"),
            "failed:api_key_invalid",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
        );
    }

    if !request
        .auth
        .scopes
        .iter()
        .any(|scope| scope.trim() == request.required_scope.trim())
    {
        return build_decision(
            false,
            403,
            Some("scope_denied"),
            "passed",
            "failed:scope_denied",
            "not_evaluated",
            "not_evaluated",
            "not_evaluated",
        );
    }

    if !request.auth.api_access {
        return build_decision(
            false,
            403,
            Some("api_access_disabled"),
            "passed",
            "passed",
            "failed:api_access_disabled",
            "not_evaluated",
            "not_evaluated",
        );
    }

    let request_limit = request
        .rate_limit
        .requests_per_minute
        .saturating_add(request.rate_limit.burst_requests);
    let next_request_count = request.current_window_requests.saturating_add(1);
    let next_item_count = request
        .current_window_items
        .saturating_add(request.item_count);

    if next_request_count > request_limit || next_item_count > request.rate_limit.items_per_minute {
        return build_decision(
            false,
            429,
            Some("rate_limited"),
            "passed",
            "passed",
            "passed",
            "failed:rate_limited",
            "not_evaluated",
        );
    }

    if request.quota_type.trim() != ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE {
        return build_decision(
            false,
            403,
            Some("quota_contract_missing"),
            "passed",
            "passed",
            "passed",
            "passed",
            "failed:quota_contract_missing",
        );
    }

    let available_units = request
        .quota_included_units
        .saturating_sub(request.quota_used_units)
        .saturating_sub(request.quota_reserved_units);
    if requested_units > available_units && !request.quota_overage_allowed {
        return build_decision(
            false,
            402,
            Some("quota_exhausted"),
            "passed",
            "passed",
            "passed",
            "passed",
            "failed:quota_exhausted",
        );
    }

    build_decision(
        true, 200, None, "passed", "passed", "passed", "passed", "passed",
    )
}

impl Storage {
    pub fn open(path: impl AsRef<Path>, retention_days: i64) -> Result<Self, StorageError> {
        let config = DatabaseConfig::sqlite(path.as_ref().to_path_buf(), "local");
        Self::open_with_database_config(&config, retention_days)
    }

    pub fn open_with_database_config(
        config: &DatabaseConfig,
        retention_days: i64,
    ) -> Result<Self, StorageError> {
        if retention_days <= 0 {
            return Err(StorageError::InvalidRetentionDays);
        }
        config.validate()?;
        if config.backend == DatabaseBackendKind::Postgres {
            return Err(StorageError::PostgresAdapterNotImplemented);
        }
        let sqlite_path = config
            .sqlite_path
            .as_ref()
            .ok_or(DatabaseConfigError::MissingSqlitePath)?;
        let conn = Connection::open(sqlite_path)?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "foreign_keys", "ON")?;
        init_schema(&conn)?;
        let storage = Self {
            conn: Mutex::new(conn),
            retention_days,
        };
        storage.cleanup_old_events()?;
        Ok(storage)
    }

    pub fn ingest_batch(
        &self,
        batch: &AnonymousFeedbackBatch,
    ) -> Result<AnonymousFeedbackBatchAck, StorageError> {
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let request_id = uuid::Uuid::new_v4().to_string();
        let accepted_at = Utc::now();

        tx.execute(
            "INSERT INTO feedback_batches (
                request_id, install_id, session_id, app_version, sent_at, received_at,
                received_events, inserted_events, duplicate_events
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, 0)",
            params![
                request_id,
                batch.install_id,
                batch.session_id,
                batch.app_version,
                batch.sent_at.to_rfc3339(),
                accepted_at.to_rfc3339(),
            ],
        )?;

        let mut inserted_events = 0usize;
        let mut duplicate_events = 0usize;
        for event in &batch.events {
            if insert_anonymous_event(&tx, event)? {
                inserted_events += 1;
            } else {
                duplicate_events += 1;
            }
        }

        tx.execute(
            "UPDATE feedback_batches
             SET received_events = ?2, inserted_events = ?3, duplicate_events = ?4
             WHERE request_id = ?1",
            params![
                request_id,
                batch.events.len() as i64,
                inserted_events as i64,
                duplicate_events as i64
            ],
        )?;

        tx.commit()?;
        Ok(AnonymousFeedbackBatchAck {
            request_id,
            received_events: batch.events.len(),
            inserted_events,
            duplicate_events,
            accepted_at,
        })
    }

    pub fn cleanup_old_events(&self) -> Result<usize, StorageError> {
        let cutoff = (Utc::now() - Duration::days(self.retention_days)).to_rfc3339();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let deleted = conn.execute(
            "DELETE FROM feedback_events WHERE occurred_at < ?1",
            params![cutoff],
        )?;
        Ok(deleted)
    }

    pub fn query_stats(
        &self,
        query: &AnonymousFeedbackStatsQuery,
    ) -> Result<AnonymousFeedbackStatsResponse, StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let dimension = query.dimension.clone().unwrap_or_default();
        let (dimension_expr, label_expr) = dimension_sql(&dimension);
        let (where_sql, values) = build_filters(query);

        let totals_sql = format!(
            "SELECT
                COUNT(*) AS total_events,
                SUM(CASE WHEN outcome = 'success' THEN 1 ELSE 0 END) AS success_events,
                SUM(CASE WHEN outcome = 'failure' THEN 1 ELSE 0 END) AS failure_events,
                SUM(CASE WHEN outcome = 'crash' THEN 1 ELSE 0 END) AS crash_events,
                SUM(CASE WHEN outcome = 'diagnostic' THEN 1 ELSE 0 END) AS diagnostic_events,
                AVG(duration_ms) AS avg_duration_ms,
                MAX(occurred_at) AS last_event_at
             FROM feedback_events {}",
            where_sql
        );
        let mut totals_stmt = conn.prepare(&totals_sql)?;
        let totals = totals_stmt.query_row(params_from_iter(values.clone()), |row| {
            Ok(FeedbackTotals {
                total_events: row.get::<_, i64>(0).unwrap_or_default() as u64,
                success_events: row.get::<_, Option<i64>>(1)?.unwrap_or_default() as u64,
                failure_events: row.get::<_, Option<i64>>(2)?.unwrap_or_default() as u64,
                crash_events: row.get::<_, Option<i64>>(3)?.unwrap_or_default() as u64,
                diagnostic_events: row.get::<_, Option<i64>>(4)?.unwrap_or_default() as u64,
                avg_duration_ms: row.get::<_, Option<f64>>(5)?,
                last_event_at: row
                    .get::<_, Option<String>>(6)?
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                    .map(|dt| dt.with_timezone(&Utc)),
            })
        })?;

        let grouped_sql = format!(
            "SELECT
                {label_expr} AS label,
                COUNT(*) AS total_events,
                SUM(CASE WHEN outcome = 'success' THEN 1 ELSE 0 END) AS success_events,
                SUM(CASE WHEN outcome = 'failure' THEN 1 ELSE 0 END) AS failure_events,
                SUM(CASE WHEN outcome = 'crash' THEN 1 ELSE 0 END) AS crash_events,
                SUM(CASE WHEN outcome = 'diagnostic' THEN 1 ELSE 0 END) AS diagnostic_events,
                AVG(duration_ms) AS avg_duration_ms,
                MAX(occurred_at) AS last_event_at
             FROM feedback_events
             {}
             GROUP BY {dimension_expr}
             ORDER BY {dimension_expr} ASC",
            where_sql
        );

        let mut stmt = conn.prepare(&grouped_sql)?;
        let rows = stmt
            .query_map(params_from_iter(values), |row| {
                Ok(FeedbackStatRow {
                    label: row.get::<_, String>(0)?,
                    total_events: row.get::<_, i64>(1).unwrap_or_default() as u64,
                    success_events: row.get::<_, Option<i64>>(2)?.unwrap_or_default() as u64,
                    failure_events: row.get::<_, Option<i64>>(3)?.unwrap_or_default() as u64,
                    crash_events: row.get::<_, Option<i64>>(4)?.unwrap_or_default() as u64,
                    diagnostic_events: row.get::<_, Option<i64>>(5)?.unwrap_or_default() as u64,
                    avg_duration_ms: row.get::<_, Option<f64>>(6)?,
                    last_event_at: row
                        .get::<_, Option<String>>(7)?
                        .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                        .map(|dt| dt.with_timezone(&Utc)),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(AnonymousFeedbackStatsResponse {
            dimension: dimension.as_str().to_string(),
            totals,
            rows,
            generated_at: Utc::now(),
        })
    }

    pub fn commercial_metrics_overview(
        &self,
    ) -> Result<CommercialMetricsOverviewResponse, StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let accounts = conn.query_row(
            "SELECT
                COUNT(*) AS total_accounts,
                SUM(CASE WHEN date(created_at) = date('now') THEN 1 ELSE 0 END) AS new_accounts_today,
                SUM(CASE WHEN datetime(created_at) >= datetime('now', '-7 days') THEN 1 ELSE 0 END) AS new_accounts_7d
             FROM cloud_accounts",
            [],
            |row| {
                Ok(CommercialAccountMetrics {
                    total_accounts: row.get::<_, i64>(0).unwrap_or_default() as u64,
                    new_accounts_today: row.get::<_, Option<i64>>(1)?.unwrap_or_default() as u64,
                    new_accounts_7d: row.get::<_, Option<i64>>(2)?.unwrap_or_default() as u64,
                })
            },
        )?;

        let mut entitlement_stmt = conn.prepare(
            "SELECT entitlement_plan_code, entitlement_status, COUNT(*)
             FROM cloud_accounts
             GROUP BY entitlement_plan_code, entitlement_status
             ORDER BY entitlement_plan_code ASC, entitlement_status ASC",
        )?;
        let entitlement_distribution = entitlement_stmt
            .query_map([], |row| {
                Ok(CommercialEntitlementPlanRow {
                    plan_code: row.get(0)?,
                    status: row.get(1)?,
                    accounts: row.get::<_, i64>(2).unwrap_or_default() as u64,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let payment_sessions = conn.query_row(
            "SELECT
                COUNT(*) AS total,
                SUM(CASE WHEN status = 'created' THEN 1 ELSE 0 END) AS created,
                SUM(CASE WHEN status = 'pending' THEN 1 ELSE 0 END) AS pending,
                SUM(CASE WHEN status = 'succeeded' THEN 1 ELSE 0 END) AS succeeded,
                SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) AS failed,
                SUM(CASE WHEN status = 'expired' THEN 1 ELSE 0 END) AS expired,
                SUM(CASE WHEN status = 'closed' THEN 1 ELSE 0 END) AS closed
             FROM billing_payment_sessions",
            [],
            |row| {
                Ok(CommercialPaymentSessionMetrics {
                    total: row.get::<_, i64>(0).unwrap_or_default() as u64,
                    created: row.get::<_, Option<i64>>(1)?.unwrap_or_default() as u64,
                    pending: row.get::<_, Option<i64>>(2)?.unwrap_or_default() as u64,
                    succeeded: row.get::<_, Option<i64>>(3)?.unwrap_or_default() as u64,
                    failed: row.get::<_, Option<i64>>(4)?.unwrap_or_default() as u64,
                    expired: row.get::<_, Option<i64>>(5)?.unwrap_or_default() as u64,
                    closed: row.get::<_, Option<i64>>(6)?.unwrap_or_default() as u64,
                })
            },
        )?;

        let local_batch_units = usage_units_for_feature(&conn, "local_batch_processing")?;
        let report_export_units = usage_units_for_feature(&conn, "report_export")?;
        let l2_video_notary_count = conn.query_row(
            "SELECT COUNT(*) FROM video_fingerprint_notaries",
            [],
            |row| Ok(row.get::<_, i64>(0).unwrap_or_default() as u64),
        )?;
        let feature_usage = CommercialFeatureUsageMetrics {
            local_batch_units,
            report_export_units,
            l2_video_notary_count,
        };

        let accepted_events =
            conn.query_row("SELECT COUNT(*) FROM cloud_sync_events", [], |row| {
                Ok(row.get::<_, i64>(0).unwrap_or_default() as u64)
            })?;
        let failure_events = conn.query_row(
            "SELECT COUNT(*) FROM feedback_events
             WHERE outcome IN ('failure', 'crash', 'diagnostic')
               AND (feature_name LIKE '%sync%' OR feature_name LIKE '%cloud%')",
            [],
            |row| Ok(row.get::<_, i64>(0).unwrap_or_default() as u64),
        )?;
        let cloud_sync = CommercialCloudSyncMetrics {
            accepted_events,
            failure_events,
        };

        let mut failure_stmt = conn.prepare(
            "SELECT feature_name, COALESCE(NULLIF(error_code, ''), 'unknown') AS error_code, COUNT(*)
             FROM feedback_events
             WHERE outcome IN ('failure', 'crash', 'diagnostic')
             GROUP BY feature_name, COALESCE(NULLIF(error_code, ''), 'unknown')
             ORDER BY COUNT(*) DESC, feature_name ASC
             LIMIT 20",
        )?;
        let anonymous_failures = failure_stmt
            .query_map([], |row| {
                Ok(CommercialAnonymousFailureRow {
                    feature_name: row.get(0)?,
                    error_code: row.get(1)?,
                    events: row.get::<_, i64>(2).unwrap_or_default() as u64,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(CommercialMetricsOverviewResponse {
            generated_at: Utc::now(),
            privacy_boundary: CommercialMetricsPrivacyBoundary {
                excludes_original_media: true,
                excludes_watermarked_media: true,
                excludes_local_paths: true,
                excludes_file_names: true,
                excludes_full_media_hashes: true,
                note: "商业指标只聚合账户、权益、支付会话、功能次数、同步状态和匿名失败分类，不采集原始媒体、加水印媒体、本地路径、文件名或完整媒体哈希。"
                    .to_string(),
            },
            accounts,
            entitlement_distribution,
            payment_sessions,
            feature_usage,
            cloud_sync,
            anonymous_failures,
        })
    }

    pub fn record_admin_audit_event(
        &self,
        endpoint: &str,
        outcome: &str,
        reason: &str,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO admin_audit_events (
                audit_id, endpoint, outcome, reason, occurred_at
            ) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                uuid::Uuid::new_v4().to_string(),
                endpoint,
                outcome,
                reason,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(())
    }

    pub fn create_enterprise_api_key_internal(
        &self,
        request: &EnterpriseApiKeyCreateRequest,
    ) -> Result<EnterpriseApiKeyRecord, StorageError> {
        let account_id = request.account_id.trim();
        let workspace_id = request.workspace_id.trim();
        let name = request.name.trim();
        let key_prefix = request.key_prefix.trim();
        let key_hash = request.key_hash.trim();
        let created_by_account_id = request.created_by_account_id.trim();
        if account_id.is_empty()
            || workspace_id.is_empty()
            || name.is_empty()
            || key_prefix.is_empty()
            || key_hash.is_empty()
            || created_by_account_id.is_empty()
        {
            return Err(StorageError::BadRequest(
                "enterprise api key fields are required".to_string(),
            ));
        }
        for scope in &request.scopes {
            if !matches!(
                scope.as_str(),
                "public_rights:read" | "public_rights:batch_read" | "public_rights:metadata_export"
            ) {
                return Err(StorageError::BadRequest(
                    "enterprise api key scope is not allowed".to_string(),
                ));
            }
        }
        let now = Utc::now();
        let api_key_id = format!(
            "eak_{}",
            short_id(&format!("{account_id}:{workspace_id}:{key_prefix}:{now}"))
        );
        let scopes_json = serde_json::to_string(&request.scopes).map_err(|_| {
            StorageError::BadRequest("enterprise api key scopes are invalid".to_string())
        })?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO enterprise_api_keys (
                api_key_id, account_id, workspace_id, creator_profile_id, key_prefix, key_hash,
                name, status, scopes_json, rate_limit_policy_json, quota_policy_json,
                created_by_account_id, created_at, last_used_at, expires_at, revoked_at, revoked_reason
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'active', ?8, ?9, ?10, ?11, ?12, NULL, ?13, NULL, NULL)",
            params![
                api_key_id,
                account_id,
                workspace_id,
                request.creator_profile_id.as_deref(),
                key_prefix,
                key_hash,
                name,
                scopes_json,
                serde_json::json!({"source":"internal_command_required"}).to_string(),
                serde_json::json!({"quotaType": ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE, "source":"internal_command_required"}).to_string(),
                created_by_account_id,
                now.to_rfc3339(),
                request.expires_at.as_ref().map(|value| value.to_rfc3339()),
            ],
        )?;
        Ok(EnterpriseApiKeyRecord {
            api_key_id,
            account_id: account_id.to_string(),
            workspace_id: workspace_id.to_string(),
            creator_profile_id: request.creator_profile_id.clone(),
            key_prefix: key_prefix.to_string(),
            name: name.to_string(),
            status: "active".to_string(),
            scopes: request.scopes.clone(),
            created_by_account_id: created_by_account_id.to_string(),
            created_at: now,
            last_used_at: None,
            expires_at: request.expires_at,
            revoked_at: None,
            revoked_reason: None,
        })
    }

    pub fn list_enterprise_api_keys_internal(
        &self,
        query: &EnterpriseApiKeyListQuery,
    ) -> Result<EnterpriseApiKeyListResponse, StorageError> {
        if let Some(status) = query.status.as_deref() {
            if !matches!(status.trim(), "active" | "paused" | "revoked" | "expired") {
                return Err(StorageError::BadRequest(
                    "enterprise api key status is invalid".to_string(),
                ));
            }
        }
        let limit = query.limit.unwrap_or(100).clamp(1, 500) as i64;
        let account_id = query
            .account_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        let workspace_id = query
            .workspace_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        let status = query.status.as_deref().map(str::trim).unwrap_or_default();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT api_key_id, account_id, workspace_id, creator_profile_id, key_prefix,
                    name, status, scopes_json, created_by_account_id, created_at,
                    last_used_at, expires_at, revoked_at, revoked_reason
             FROM enterprise_api_keys
             WHERE (?1 = '' OR account_id = ?1)
               AND (?2 = '' OR workspace_id = ?2)
               AND (?3 = '' OR status = ?3)
             ORDER BY created_at DESC, api_key_id ASC
             LIMIT ?4",
        )?;
        let api_keys = stmt
            .query_map(params![account_id, workspace_id, status, limit], |row| {
                enterprise_api_key_record_from_sql(row)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(EnterpriseApiKeyListResponse {
            returned: api_keys.len() as u32,
            api_keys,
        })
    }

    pub fn get_enterprise_api_key_internal(
        &self,
        api_key_id: &str,
    ) -> Result<EnterpriseApiKeyRecord, StorageError> {
        let api_key_id = api_key_id.trim();
        if api_key_id.is_empty() {
            return Err(StorageError::BadRequest(
                "enterprise api key id is required".to_string(),
            ));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.query_row(
            "SELECT api_key_id, account_id, workspace_id, creator_profile_id, key_prefix,
                    name, status, scopes_json, created_by_account_id, created_at,
                    last_used_at, expires_at, revoked_at, revoked_reason
             FROM enterprise_api_keys
             WHERE api_key_id = ?1",
            params![api_key_id],
            enterprise_api_key_record_from_sql,
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                StorageError::BadRequest("enterprise api key not found".to_string())
            }
            other => StorageError::Database(other),
        })
    }

    pub fn pause_enterprise_api_key_internal(
        &self,
        api_key_id: &str,
        reason: &str,
    ) -> Result<EnterpriseApiKeyRecord, StorageError> {
        let api_key_id = api_key_id.trim();
        let reason = reason.trim();
        if api_key_id.is_empty() || reason.is_empty() {
            return Err(StorageError::BadRequest(
                "enterprise api key pause request is invalid".to_string(),
            ));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let current_status: String = conn
            .query_row(
                "SELECT status FROM enterprise_api_keys WHERE api_key_id = ?1",
                params![api_key_id],
                |row| row.get(0),
            )
            .map_err(|error| match error {
                rusqlite::Error::QueryReturnedNoRows => {
                    StorageError::BadRequest("enterprise api key not found".to_string())
                }
                other => StorageError::Database(other),
            })?;
        if current_status == "revoked" {
            return Err(StorageError::BadRequest(
                "enterprise api key is revoked".to_string(),
            ));
        }
        if current_status == "expired" {
            return Err(StorageError::BadRequest(
                "enterprise api key is expired".to_string(),
            ));
        }
        conn.execute(
            "UPDATE enterprise_api_keys
             SET status = 'paused'
             WHERE api_key_id = ?1",
            params![api_key_id],
        )?;
        drop(conn);
        self.get_enterprise_api_key_internal(api_key_id)
    }

    pub fn revoke_enterprise_api_key_internal(
        &self,
        api_key_id: &str,
        reason: &str,
    ) -> Result<EnterpriseApiKeyRecord, StorageError> {
        let api_key_id = api_key_id.trim();
        let reason = reason.trim();
        if api_key_id.is_empty() || reason.is_empty() {
            return Err(StorageError::BadRequest(
                "enterprise api key revoke request is invalid".to_string(),
            ));
        }
        let now = Utc::now();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let updated = conn.execute(
            "UPDATE enterprise_api_keys
             SET status = 'revoked', revoked_at = ?2, revoked_reason = ?3
             WHERE api_key_id = ?1 AND status != 'revoked'",
            params![api_key_id, now.to_rfc3339(), reason],
        )?;
        if updated == 0 {
            drop(conn);
            let current = self.get_enterprise_api_key_internal(api_key_id)?;
            if current.status == "revoked" {
                return Ok(current);
            }
            return Ok(current);
        }
        drop(conn);
        self.get_enterprise_api_key_internal(api_key_id)
    }

    pub fn initialize_enterprise_quota_balance_internal(
        &self,
        request: &EnterpriseQuotaBalanceInitRequest,
    ) -> Result<EnterpriseQuotaBalanceRecord, StorageError> {
        let account_id = request.account_id.trim();
        let workspace_id = request.workspace_id.trim();
        let quota_type = request.quota_type.trim();
        let currency = request.currency.trim().to_uppercase();
        if account_id.is_empty()
            || workspace_id.is_empty()
            || quota_type != ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE
            || request.period_end <= request.period_start
            || request.included_units < 0
            || request.overage_unit_price_cents.unwrap_or(0) < 0
            || currency.is_empty()
        {
            return Err(StorageError::BadRequest(
                "enterprise quota balance request is invalid".to_string(),
            ));
        }
        let now = Utc::now();
        let quota_balance_id = format!(
            "eqb_{}",
            short_id(&format!(
                "{}:{}:{}:{}:{}",
                account_id,
                workspace_id,
                quota_type,
                request.period_start.to_rfc3339(),
                request.period_end.to_rfc3339()
            ))
        );
        let period_start = request.period_start.to_rfc3339();
        let period_end = request.period_end.to_rfc3339();
        let overage_allowed = if request.overage_allowed { 1 } else { 0 };
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO enterprise_quota_balances (
                quota_balance_id, account_id, workspace_id, quota_type, period_start, period_end,
                included_units, used_units, reserved_units, overage_allowed,
                overage_unit_price_cents, currency, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, 0, ?8, ?9, ?10, ?11)
            ON CONFLICT(account_id, workspace_id, quota_type, period_start, period_end)
            DO UPDATE SET
                included_units = excluded.included_units,
                overage_allowed = excluded.overage_allowed,
                overage_unit_price_cents = excluded.overage_unit_price_cents,
                currency = excluded.currency,
                updated_at = excluded.updated_at",
            params![
                quota_balance_id,
                account_id,
                workspace_id,
                quota_type,
                period_start,
                period_end,
                request.included_units,
                overage_allowed,
                request.overage_unit_price_cents,
                currency,
                now.to_rfc3339(),
            ],
        )?;
        enterprise_quota_balance_by_key(
            &conn,
            account_id,
            workspace_id,
            quota_type,
            &period_start,
            &period_end,
        )
    }

    pub fn record_enterprise_quota_ledger_internal(
        &self,
        request: &EnterpriseQuotaLedgerRequest,
    ) -> Result<EnterpriseQuotaLedgerRecord, StorageError> {
        if request.account_id.trim().is_empty()
            || request.workspace_id.trim().is_empty()
            || request.quota_type.trim() != ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE
            || request.reference_id.trim().is_empty()
            || request.idempotency_key.trim().is_empty()
            || !matches!(request.direction.as_str(), "debit" | "credit")
            || !matches!(request.status.as_str(), "reserved" | "committed" | "voided")
        {
            return Err(StorageError::BadRequest(
                "enterprise quota ledger request is invalid".to_string(),
            ));
        }
        let now = Utc::now();
        let quota_ledger_id = format!(
            "eql_{}",
            short_id(&format!(
                "{}:{}:{}:{}",
                request.account_id,
                request.workspace_id,
                request.quota_type,
                request.idempotency_key
            ))
        );
        let committed_at = if request.status == "committed" {
            Some(now)
        } else {
            None
        };
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO enterprise_quota_ledger (
                quota_ledger_id, account_id, workspace_id, api_key_id, quota_type, units,
                direction, event_type, reference_id, idempotency_key, status, created_at, committed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                quota_ledger_id,
                request.account_id.trim(),
                request.workspace_id.trim(),
                request.api_key_id.as_deref(),
                request.quota_type.trim(),
                request.units,
                request.direction.trim(),
                request.event_type.trim(),
                request.reference_id.trim(),
                request.idempotency_key.trim(),
                request.status.trim(),
                now.to_rfc3339(),
                committed_at.as_ref().map(|value| value.to_rfc3339()),
            ],
        )?;
        Ok(EnterpriseQuotaLedgerRecord {
            quota_ledger_id,
            account_id: request.account_id.trim().to_string(),
            workspace_id: request.workspace_id.trim().to_string(),
            api_key_id: request.api_key_id.clone(),
            quota_type: request.quota_type.trim().to_string(),
            units: request.units,
            direction: request.direction.trim().to_string(),
            event_type: request.event_type.trim().to_string(),
            reference_id: request.reference_id.trim().to_string(),
            idempotency_key: request.idempotency_key.trim().to_string(),
            status: request.status.trim().to_string(),
            created_at: now,
            committed_at,
        })
    }

    pub fn record_enterprise_api_audit_event_internal(
        &self,
        request: &EnterpriseApiAuditEventRequest,
    ) -> Result<String, StorageError> {
        if request.account_id.trim().is_empty()
            || request.workspace_id.trim().is_empty()
            || request.endpoint.trim().is_empty()
            || request.method.trim().is_empty()
            || request.request_id.trim().is_empty()
        {
            return Err(StorageError::BadRequest(
                "enterprise api audit event request is invalid".to_string(),
            ));
        }
        let now = Utc::now();
        let audit_event_id = format!(
            "eae_{}",
            short_id(&format!(
                "{}:{}:{}",
                request.account_id, request.request_id, now
            ))
        );
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO enterprise_api_audit_events (
                audit_event_id, account_id, workspace_id, api_key_id, endpoint, method,
                request_count, item_count, status_code, error_code, quota_units, client_label,
                client_fingerprint_hash, trusted_proxy_status, request_id, occurred_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                audit_event_id,
                request.account_id.trim(),
                request.workspace_id.trim(),
                request.api_key_id.as_deref(),
                request.endpoint.trim(),
                request.method.trim(),
                request.request_count,
                request.item_count,
                request.status_code,
                request.error_code.as_deref(),
                request.quota_units,
                request.client_label.as_deref(),
                request.client_fingerprint_hash.as_deref(),
                request.trusted_proxy_status.as_deref(),
                request.request_id.trim(),
                now.to_rfc3339(),
            ],
        )?;
        Ok(audit_event_id)
    }

    pub fn record_enterprise_admin_audit_event_internal(
        &self,
        operation: &str,
        outcome: &str,
        endpoint: &str,
        account_id: Option<&str>,
        workspace_id: Option<&str>,
        api_key_id: Option<&str>,
        target_id: Option<&str>,
        reason: &str,
        details_json: serde_json::Value,
    ) -> Result<String, StorageError> {
        let operation = operation.trim();
        let outcome = outcome.trim();
        let endpoint = endpoint.trim();
        if !matches!(
            operation,
            "create_api_key"
                | "issue_api_key"
                | "rotate_api_key"
                | "revoke_expired_rotations"
                | "list_api_keys"
                | "get_api_key"
                | "pause_api_key"
                | "revoke_api_key"
                | "init_quota_balance"
                | "dry_run_gateway"
        ) || !matches!(outcome, "succeeded" | "failed")
            || endpoint.is_empty()
        {
            return Err(StorageError::BadRequest(
                "enterprise admin audit event is invalid".to_string(),
            ));
        }
        let audit_event_id = format!(
            "eaa_{}",
            short_id(&format!("{}:{}:{}", operation, endpoint, Utc::now()))
        );
        let details_json = serde_json::to_string(&details_json).map_err(|_| {
            StorageError::BadRequest("enterprise admin audit details are invalid".to_string())
        })?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO enterprise_admin_audit_events (
                audit_event_id, operation, outcome, endpoint, account_id, workspace_id,
                api_key_id, target_id, reason, details_json, occurred_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                audit_event_id,
                operation,
                outcome,
                endpoint,
                account_id.map(str::trim).filter(|value| !value.is_empty()),
                workspace_id
                    .map(str::trim)
                    .filter(|value| !value.is_empty()),
                api_key_id.map(str::trim).filter(|value| !value.is_empty()),
                target_id.map(str::trim).filter(|value| !value.is_empty()),
                reason.trim(),
                details_json,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(audit_event_id)
    }

    pub fn get_ai_transparency_license_internal(
        &self,
        license_id: &str,
    ) -> Result<Option<AiTransparencyLicenseDetailResponse>, StorageError> {
        let license_id = license_id.trim();
        if license_id.is_empty() {
            return Err(StorageError::BadRequest(
                "ai_license_id_required".to_string(),
            ));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let license = conn
            .query_row(
                "SELECT license_id, tenant_id, workspace_id, environment, status, issuer_mode,
                    deployment_mode, public_verification_required, metering_plan_id,
                    effective_at, expires_at, created_at, updated_at
                 FROM ai_transparency_licenses WHERE license_id = ?1",
                params![license_id],
                |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                        row.get::<_, String>(6)?,
                        row.get::<_, i64>(7)?,
                        row.get::<_, String>(8)?,
                        row.get::<_, String>(9)?,
                        row.get::<_, String>(10)?,
                        row.get::<_, String>(11)?,
                        row.get::<_, String>(12)?,
                    ))
                },
            )
            .optional()?;
        let Some(license) = license else {
            return Ok(None);
        };
        let mut statement = conn.prepare(
            "SELECT profile_id, profile_kind, status, effective_at, expires_at, terms_version,
                approved_by, created_at, updated_at
             FROM ai_profile_entitlements WHERE license_id = ?1 ORDER BY profile_id ASC",
        )?;
        let profiles = statement
            .query_map(params![license_id], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?,
                ))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(AiTransparencyLicenseDetailResponse {
            license: AiTransparencyLicenseRecord {
                license_id: license.0,
                tenant_id: license.1,
                workspace_id: license.2,
                environment: license.3,
                status: license.4,
                issuer_mode: license.5,
                deployment_mode: license.6,
                public_verification_required: license.7 != 0,
                metering_plan_id: license.8,
                effective_at: parse_utc_rfc3339(&license.9)?,
                expires_at: parse_utc_rfc3339(&license.10)?,
                created_at: parse_utc_rfc3339(&license.11)?,
                updated_at: parse_utc_rfc3339(&license.12)?,
            },
            profile_entitlements: profiles
                .into_iter()
                .map(|profile| {
                    Ok(AiTransparencyProfileEntitlementRecord {
                        profile_id: profile.0,
                        profile_kind: profile.1,
                        status: profile.2,
                        effective_at: parse_utc_rfc3339(&profile.3)?,
                        expires_at: parse_utc_rfc3339(&profile.4)?,
                        terms_version: profile.5,
                        approved_by: profile.6,
                        created_at: parse_utc_rfc3339(&profile.7)?,
                        updated_at: parse_utc_rfc3339(&profile.8)?,
                    })
                })
                .collect::<Result<Vec<_>, StorageError>>()?,
        }))
    }

    pub fn check_ai_transparency_profile_entitlements_internal(
        &self,
        request: &AiTransparencyProfileEntitlementCheckRequest,
    ) -> Result<AiTransparencyProfileEntitlementCheckResponse, StorageError> {
        let license_id = request.license_id.trim();
        let environment = request.environment.trim();
        if license_id.is_empty() || environment.is_empty() {
            return Err(StorageError::BadRequest(
                "ai_license_id_and_environment_required".to_string(),
            ));
        }
        if request.requested_profile_ids.is_empty() || request.requested_profile_ids.len() > 32 {
            return Err(StorageError::BadRequest(
                "ai_requested_profile_ids_invalid".to_string(),
            ));
        }
        let requested_profile_ids = request
            .requested_profile_ids
            .iter()
            .map(|profile_id| profile_id.trim().to_string())
            .collect::<Vec<_>>();
        if requested_profile_ids.iter().any(String::is_empty)
            || requested_profile_ids
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != requested_profile_ids.len()
        {
            return Err(StorageError::BadRequest(
                "ai_requested_profile_ids_invalid".to_string(),
            ));
        }
        let evaluated_at = Utc::now();
        let Some(detail) = self.get_ai_transparency_license_internal(license_id)? else {
            return Ok(AiTransparencyProfileEntitlementCheckResponse {
                license_id: license_id.to_string(),
                authorized: false,
                evaluated_at,
                license_decision: AiTransparencyLicenseDecision {
                    authorized: false,
                    reason_code: "ai_license_not_found".to_string(),
                },
                profile_decisions: requested_profile_ids
                    .into_iter()
                    .map(|profile_id| AiTransparencyProfileDecision {
                        profile_id,
                        authorized: false,
                        reason_code: "ai_license_not_found".to_string(),
                        profile_kind: None,
                        terms_version: None,
                        expires_at: None,
                    })
                    .collect(),
            });
        };
        let license_reason_code =
            ai_license_reason_code(&detail.license, environment, evaluated_at);
        let license_authorized = license_reason_code == "authorized";
        let profile_decisions = requested_profile_ids
            .into_iter()
            .map(|profile_id| {
                let entitlement = detail
                    .profile_entitlements
                    .iter()
                    .find(|entitlement| entitlement.profile_id == profile_id);
                let reason_code = if !license_authorized {
                    license_reason_code.to_string()
                } else {
                    ai_profile_reason_code(entitlement, evaluated_at).to_string()
                };
                AiTransparencyProfileDecision {
                    profile_id,
                    authorized: reason_code == "authorized",
                    reason_code,
                    profile_kind: entitlement.map(|item| item.profile_kind.clone()),
                    terms_version: entitlement.map(|item| item.terms_version.clone()),
                    expires_at: entitlement.map(|item| item.expires_at),
                }
            })
            .collect::<Vec<_>>();
        Ok(AiTransparencyProfileEntitlementCheckResponse {
            license_id: detail.license.license_id,
            authorized: license_authorized
                && profile_decisions.iter().all(|decision| decision.authorized),
            evaluated_at,
            license_decision: AiTransparencyLicenseDecision {
                authorized: license_authorized,
                reason_code: license_reason_code.to_string(),
            },
            profile_decisions,
        })
    }

    pub fn record_ai_transparency_admin_audit_event_internal(
        &self,
        operation: &str,
        outcome: &str,
        endpoint: &str,
        license: Option<&AiTransparencyLicenseRecord>,
        license_id: Option<&str>,
        requested_profile_ids: &[String],
        reason_code: &str,
        details_json: serde_json::Value,
    ) -> Result<String, StorageError> {
        if !matches!(operation, "get_license" | "check_profile_entitlements")
            || !matches!(outcome, "succeeded" | "denied" | "failed")
            || endpoint.trim().is_empty()
            || reason_code.trim().is_empty()
        {
            return Err(StorageError::BadRequest(
                "ai_transparency_admin_audit_event_invalid".to_string(),
            ));
        }
        let audit_event_id = format!(
            "ata_{}",
            short_id(&format!("{}:{}:{}", operation, endpoint, Utc::now()))
        );
        let requested_profile_ids_json =
            serde_json::to_string(requested_profile_ids).map_err(|_| {
                StorageError::BadRequest("ai_transparency_admin_audit_profiles_invalid".to_string())
            })?;
        let details_json = serde_json::to_string(&details_json).map_err(|_| {
            StorageError::BadRequest("ai_transparency_admin_audit_details_invalid".to_string())
        })?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        conn.execute(
            "INSERT INTO ai_transparency_admin_audit_events (
                audit_event_id, operation, outcome, endpoint, license_id, tenant_id, workspace_id,
                requested_profile_ids_json, reason_code, details_json, occurred_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                audit_event_id,
                operation,
                outcome,
                endpoint,
                license.map(|item| item.license_id.as_str()).or(license_id),
                license.map(|item| item.tenant_id.as_str()),
                license.map(|item| item.workspace_id.as_str()),
                requested_profile_ids_json,
                reason_code.trim(),
                details_json,
                Utc::now().to_rfc3339(),
            ],
        )?;
        Ok(audit_event_id)
    }

    pub fn list_enterprise_admin_audit_events_internal(
        &self,
        query: &EnterpriseAdminAuditEventQuery,
    ) -> Result<EnterpriseAdminAuditEventListResponse, StorageError> {
        if let Some(operation) = query.operation.as_deref() {
            if !matches!(
                operation.trim(),
                "create_api_key"
                    | "issue_api_key"
                    | "rotate_api_key"
                    | "revoke_expired_rotations"
                    | "list_api_keys"
                    | "get_api_key"
                    | "pause_api_key"
                    | "revoke_api_key"
                    | "init_quota_balance"
                    | "dry_run_gateway"
            ) {
                return Err(StorageError::BadRequest(
                    "enterprise admin audit operation is invalid".to_string(),
                ));
            }
        }
        if let Some(outcome) = query.outcome.as_deref() {
            if !matches!(outcome.trim(), "succeeded" | "failed") {
                return Err(StorageError::BadRequest(
                    "enterprise admin audit outcome is invalid".to_string(),
                ));
            }
        }
        if query
            .from_occurred_at
            .zip(query.to_occurred_at)
            .map(|(from, to)| from > to)
            .unwrap_or(false)
        {
            return Err(StorageError::BadRequest(
                "enterprise admin audit time range is invalid".to_string(),
            ));
        }
        let operation = query
            .operation
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        let outcome = query.outcome.as_deref().map(str::trim).unwrap_or_default();
        let account_id = query
            .account_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        let api_key_id = query
            .api_key_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        let from_occurred_at = query
            .from_occurred_at
            .as_ref()
            .map(|value| value.to_rfc3339())
            .unwrap_or_default();
        let to_occurred_at = query
            .to_occurred_at
            .as_ref()
            .map(|value| value.to_rfc3339())
            .unwrap_or_default();
        let limit = query.limit.unwrap_or(100).clamp(1, 500) as i64;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT audit_event_id, operation, outcome, endpoint, account_id, workspace_id,
                    api_key_id, target_id, reason, details_json, occurred_at
             FROM enterprise_admin_audit_events
             WHERE (?1 = '' OR operation = ?1)
               AND (?2 = '' OR outcome = ?2)
               AND (?3 = '' OR account_id = ?3)
               AND (?4 = '' OR api_key_id = ?4)
               AND (?5 = '' OR occurred_at >= ?5)
               AND (?6 = '' OR occurred_at <= ?6)
             ORDER BY occurred_at DESC, audit_event_id DESC
             LIMIT ?7",
        )?;
        let events = stmt
            .query_map(
                params![
                    operation,
                    outcome,
                    account_id,
                    api_key_id,
                    from_occurred_at,
                    to_occurred_at,
                    limit
                ],
                enterprise_admin_audit_event_record_from_sql,
            )?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(EnterpriseAdminAuditEventListResponse {
            returned: events.len() as u32,
            events,
        })
    }

    pub fn list_expired_enterprise_rotation_candidates_internal(
        &self,
        now: chrono::DateTime<Utc>,
        limit: u32,
    ) -> Result<Vec<EnterpriseExpiredRotationCandidate>, StorageError> {
        let limit = limit.clamp(1, 500) as i64;
        let now = now.to_rfc3339();
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT audit.audit_event_id,
                    audit.api_key_id,
                    audit.target_id,
                    audit.account_id,
                    audit.workspace_id,
                    json_extract(audit.details_json, '$.rotationDeadlineAt') AS rotation_deadline_at
             FROM enterprise_admin_audit_events audit
             JOIN enterprise_api_keys old_key ON old_key.api_key_id = audit.api_key_id
             WHERE audit.operation = 'rotate_api_key'
               AND audit.outcome = 'succeeded'
               AND old_key.status = 'paused'
               AND rotation_deadline_at IS NOT NULL
               AND rotation_deadline_at != ''
               AND rotation_deadline_at <= ?1
             ORDER BY rotation_deadline_at ASC, audit.occurred_at ASC
             LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(params![now, limit], |row| {
                let rotation_deadline_at: String = row.get(5)?;
                let rotation_deadline_at =
                    parse_rfc3339_utc(&rotation_deadline_at).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            5,
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(EnterpriseExpiredRotationCandidate {
                    audit_event_id: row.get(0)?,
                    old_api_key_id: row.get(1)?,
                    new_api_key_id: row.get(2)?,
                    account_id: row.get(3)?,
                    workspace_id: row.get(4)?,
                    rotation_deadline_at,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    #[cfg(test)]
    pub(crate) fn admin_audit_event_count_for_tests(
        &self,
        outcome: &str,
        reason: Option<&str>,
    ) -> Result<i64, StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let count = if let Some(reason) = reason {
            conn.query_row(
                "SELECT COUNT(*) FROM admin_audit_events WHERE outcome = ?1 AND reason = ?2",
                params![outcome, reason],
                |row| row.get(0),
            )?
        } else {
            conn.query_row(
                "SELECT COUNT(*) FROM admin_audit_events WHERE outcome = ?1",
                params![outcome],
                |row| row.get(0),
            )?
        };
        Ok(count)
    }

    #[cfg(test)]
    pub(crate) fn enterprise_admin_audit_event_count_for_tests(
        &self,
        operation: &str,
        outcome: &str,
    ) -> Result<i64, StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let count = conn.query_row(
            "SELECT COUNT(*) FROM enterprise_admin_audit_events
             WHERE operation = ?1 AND outcome = ?2",
            params![operation, outcome],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    #[cfg(test)]
    pub(crate) fn latest_enterprise_admin_audit_reason_for_tests(
        &self,
        operation: &str,
    ) -> Result<Option<String>, StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT reason FROM enterprise_admin_audit_events
             WHERE operation = ?1
             ORDER BY occurred_at DESC
             LIMIT 1",
        )?;
        let mut rows = stmt.query(params![operation])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn continue_account(
        &self,
        request: &crate::schema::ContinueAccountRequest,
    ) -> Result<CloudAccountSession, StorageError> {
        let password = if request.password.trim().is_empty() {
            request.verification_code.trim()
        } else {
            request.password.trim()
        };
        let request = AuthSessionRequest {
            identifier: request.identifier.clone(),
            challenge_id: None,
            verification_code: String::new(),
            password: password.to_string(),
            device: request.device.clone(),
            local_creator_profile: request.local_creator_profile.clone(),
        };
        self.create_auth_session(&request)
    }

    pub fn create_auth_challenge(
        &self,
        request: &AuthChallengeRequest,
    ) -> Result<AuthChallengeResponse, StorageError> {
        let identifier = normalize_identifier(&request.identifier)?;
        let purpose = request.purpose.trim();
        if !matches!(
            purpose,
            "register_or_login" | "login" | "bind_identifier" | "reset_password"
        ) {
            return Err(StorageError::BadRequest("purpose is invalid".to_string()));
        }
        let client_device_id = request.client_device_id.trim();
        if client_device_id.is_empty() {
            return Err(StorageError::BadRequest(
                "clientDeviceId is required".to_string(),
            ));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        ensure_auth_challenge_rate_limit(&conn, &identifier, client_device_id)?;
        let challenge_id = format!(
            "chal_{}",
            short_id(&format!(
                "{}:{}:{}",
                identifier,
                client_device_id,
                Utc::now()
            ))
        );
        let delivery_channel = auth_delivery_channel();
        let code = if delivery_channel == "fixture" {
            "000000".to_string()
        } else {
            generate_otp_code()
        };
        let code_salt = new_password_salt();
        let code_hash = auth_code_hash(&code, &code_salt);
        let now = Utc::now();
        let expires_at = now + Duration::minutes(10);
        conn.execute(
            "INSERT INTO auth_challenges (
                challenge_id, identifier, purpose, client_device_id, code_hash,
                code_salt, delivery_channel, expires_at, consumed_at, created_at, plain_code_for_delivery
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, ?9, ?10)",
            params![
                challenge_id,
                identifier,
                purpose,
                client_device_id,
                code_hash,
                code_salt,
                delivery_channel,
                expires_at.to_rfc3339(),
                now.to_rfc3339(),
                if delivery_channel == "fixture" {
                    None::<String>
                } else {
                    Some(code.clone())
                },
            ],
        )?;
        Ok(AuthChallengeResponse {
            challenge_id,
            delivery_channel: delivery_channel.clone(),
            expires_at,
            message: if delivery_channel == "fixture" {
                "本地研发环境未配置验证码投递服务，fixture 验证码为 000000。".to_string()
            } else {
                "如果账号可用，验证码会发送到对应联系方式。".to_string()
            },
            fixture_code: (delivery_channel == "fixture").then_some(code),
        })
    }

    pub fn take_auth_challenge_delivery_code(
        &self,
        challenge_id: &str,
    ) -> Result<Option<String>, StorageError> {
        let challenge_id = challenge_id.trim();
        if challenge_id.is_empty() {
            return Ok(None);
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let code = conn
            .query_row(
                "SELECT plain_code_for_delivery FROM auth_challenges WHERE challenge_id = ?1",
                params![challenge_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()?
            .flatten();
        if code.is_some() {
            conn.execute(
                "UPDATE auth_challenges SET plain_code_for_delivery = NULL WHERE challenge_id = ?1",
                params![challenge_id],
            )?;
        }
        Ok(code)
    }

    pub fn create_auth_session(
        &self,
        request: &AuthSessionRequest,
    ) -> Result<CloudAccountSession, StorageError> {
        let identifier = normalize_identifier(&request.identifier)?;
        let display_name = request
            .local_creator_profile
            .display_name
            .trim()
            .to_string();
        let creator_display_name = if display_name.is_empty() {
            identifier.clone()
        } else {
            display_name
        };
        let creator_seed_ref = request.local_creator_profile.creator_seed_ref.trim();
        let creator_seed_ref = if creator_seed_ref.is_empty() {
            "local-seed-ref".to_string()
        } else {
            creator_seed_ref.to_string()
        };
        let password = request.password.trim();
        let password = if password.is_empty() {
            None
        } else {
            Some(password)
        };
        let challenge_id = request
            .challenge_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if password.is_none() && challenge_id.is_none() {
            return Err(StorageError::Unauthorized);
        }

        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        ensure_auth_login_rate_limit(&conn, &identifier, &request.device.client_device_id)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;

        let now = Utc::now().to_rfc3339();
        let result = if let Some(challenge_id) = challenge_id {
            consume_auth_challenge_tx(
                &tx,
                challenge_id,
                &identifier,
                request.verification_code.trim(),
                &now,
            )
            .and_then(|_| {
                ensure_account(
                    &tx,
                    &identifier,
                    &creator_display_name,
                    &creator_seed_ref,
                    request.local_creator_profile.seed_envelope_version,
                    None,
                    &now,
                )
            })
        } else {
            ensure_account(
                &tx,
                &identifier,
                &creator_display_name,
                &creator_seed_ref,
                request.local_creator_profile.seed_envelope_version,
                password,
                &now,
            )
        };
        let account = match result {
            Ok(account) => account,
            Err(error) => {
                let reason = match &error {
                    StorageError::RateLimited(_) => "rate_limited",
                    StorageError::Unauthorized => "invalid_credential",
                    StorageError::BadRequest(_) => "bad_request",
                    _ => "storage_error",
                };
                let _ = record_auth_attempt_tx(
                    &tx,
                    &identifier,
                    Some(request.device.client_device_id.trim()),
                    "login",
                    "failure",
                    reason,
                    &now,
                );
                tx.commit()?;
                return Err(error);
            }
        };
        let device = ensure_device(&tx, &account.id, request, &now)?;
        let session = create_session(&tx, &account.id, &device.id, &now)?;
        record_auth_attempt_tx(
            &tx,
            &identifier,
            Some(request.device.client_device_id.trim()),
            "login",
            "success",
            if challenge_id.is_some() {
                "challenge"
            } else {
                "password"
            },
            &now,
        )?;
        tx.commit()?;
        let snapshot = account_snapshot_from_rows(&conn, account, device)?;
        Ok(CloudAccountSession {
            access_token: session.access_token,
            refresh_token: session.refresh_token,
            account: snapshot.account,
            workspace: snapshot.workspace,
            device: snapshot.device,
            creator_profile: snapshot.creator_profile,
            entitlement: snapshot.entitlement,
            sync_policy: snapshot.sync_policy,
            cloud_vault_cursor: snapshot.cloud_vault_cursor,
        })
    }

    pub fn refresh_auth_session(
        &self,
        request: &AuthRefreshRequest,
    ) -> Result<CloudAccountSession, StorageError> {
        let refresh_token = request.refresh_token.trim();
        let device_id = request.device_id.trim();
        if refresh_token.is_empty() || device_id.is_empty() {
            return Err(StorageError::Unauthorized);
        }
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let existing = tx
            .query_row(
                "SELECT account_id, device_id, revoked_at
                 FROM cloud_sessions
                 WHERE refresh_token = ?1 AND device_id = ?2
                 ORDER BY created_at DESC
                 LIMIT 1",
                params![refresh_token, device_id],
                |row| {
                    Ok(SessionRecord {
                        account_id: row.get(0)?,
                        device_id: row.get(1)?,
                        revoked_at: row.get(2)?,
                    })
                },
            )
            .optional()?
            .ok_or(StorageError::Unauthorized)?;
        if existing.revoked_at.is_some() {
            return Err(StorageError::Unauthorized);
        }
        let now = Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE cloud_sessions
             SET revoked_at = ?3, last_used_at = ?3
             WHERE refresh_token = ?1 AND device_id = ?2 AND revoked_at IS NULL",
            params![refresh_token, device_id, now],
        )?;
        let session = create_session(&tx, &existing.account_id, &existing.device_id, &now)?;
        tx.commit()?;
        let account = load_account_by_id_conn(&conn, &existing.account_id)?;
        let device = load_device_by_id_conn(&conn, &existing.account_id, &existing.device_id)?;
        let snapshot = account_snapshot_from_rows(&conn, account, device)?;
        Ok(CloudAccountSession {
            access_token: session.access_token,
            refresh_token: session.refresh_token,
            account: snapshot.account,
            workspace: snapshot.workspace,
            device: snapshot.device,
            creator_profile: snapshot.creator_profile,
            entitlement: snapshot.entitlement,
            sync_policy: snapshot.sync_policy,
            cloud_vault_cursor: snapshot.cloud_vault_cursor,
        })
    }

    pub fn logout_auth_session(
        &self,
        request: &AuthLogoutRequest,
    ) -> Result<AuthLogoutResponse, StorageError> {
        let refresh_token = request.refresh_token.trim();
        let device_id = request.device_id.trim();
        if refresh_token.is_empty() || device_id.is_empty() {
            return Err(StorageError::Unauthorized);
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let now = Utc::now().to_rfc3339();
        let changed = conn.execute(
            "UPDATE cloud_sessions
             SET revoked_at = COALESCE(revoked_at, ?3), last_used_at = ?3
             WHERE refresh_token = ?1 AND device_id = ?2",
            params![refresh_token, device_id, now],
        )?;
        if changed == 0 {
            return Err(StorageError::Unauthorized);
        }
        Ok(AuthLogoutResponse { ok: true })
    }

    pub fn current_account_snapshot(
        &self,
        access_token: &str,
    ) -> Result<CloudAccountSnapshot, StorageError> {
        let session = self.authenticate(access_token)?;
        self.account_snapshot_for_session_parts(&session.account_id, &session.device_id)
    }

    pub fn update_sync_preferences(
        &self,
        access_token: &str,
        request: &SyncPreferencesRequest,
    ) -> Result<SyncPreferencesResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let account = load_account_by_id_tx(&tx, &session.account_id)?;
        let entitlement_features = serde_json::from_str(&account.entitlement_features_json)
            .unwrap_or_else(|_| default_entitlement_features());
        let cloud_sync_entitled = entitlement_features
            .get("cloud_sync")
            .and_then(serde_json::Value::as_bool)
            == Some(true);
        if request.auto_sync_enabled && !cloud_sync_entitled {
            return Err(StorageError::Forbidden);
        }
        let now = Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE cloud_devices
             SET auto_sync_enabled = ?1, updated_at = ?2
             WHERE account_id = ?3 AND id = ?4",
            params![
                if request.auto_sync_enabled { 1 } else { 0 },
                now,
                session.account_id,
                session.device_id
            ],
        )?;
        tx.commit()?;

        let sync_policy = sync_policy_for_entitlement_and_preference(
            &entitlement_features,
            request.auto_sync_enabled,
        );
        let cloud_vault_cursor =
            device_cursor_with_conn(&conn, &session.account_id, &session.device_id)?;
        Ok(SyncPreferencesResponse {
            sync_policy,
            auto_sync_enabled: request.auto_sync_enabled,
            cloud_vault_cursor,
            entitlement: CloudEntitlement {
                id: account.entitlement_id,
                plan_name: Some(account.entitlement_plan_name),
                plan_code: account.entitlement_plan_code,
                status: account.entitlement_status,
                features: entitlement_features,
            },
        })
    }

    pub fn list_devices(&self, access_token: &str) -> Result<AccountDevicesResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        Ok(AccountDevicesResponse {
            devices: list_account_devices_with_conn(
                &conn,
                &session.account_id,
                &session.device_id,
            )?,
        })
    }

    pub fn update_device(
        &self,
        access_token: &str,
        device_id: &str,
        request: &UpdateDeviceRequest,
    ) -> Result<AccountDevice, StorageError> {
        let session = self.authenticate(access_token)?;
        let device_id = device_id.trim();
        let name = request.name.trim();
        if device_id.is_empty() || name.is_empty() {
            return Err(StorageError::BadRequest("device_name_required".to_string()));
        }
        if name.chars().count() > 60 {
            return Err(StorageError::BadRequest("device_name_too_long".to_string()));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let now = Utc::now().to_rfc3339();
        let changed = conn.execute(
            "UPDATE cloud_devices
             SET name = ?3, updated_at = ?4
             WHERE account_id = ?1 AND id = ?2 AND registered = 1",
            params![session.account_id, device_id, name, now],
        )?;
        if changed == 0 {
            return Err(StorageError::Unauthorized);
        }
        load_account_device_with_conn(&conn, &session.account_id, device_id, &session.device_id)?
            .ok_or(StorageError::Unauthorized)
    }

    pub fn revoke_device(
        &self,
        access_token: &str,
        device_id: &str,
    ) -> Result<RevokeDeviceResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let device_id = device_id.trim();
        if device_id.is_empty() {
            return Err(StorageError::BadRequest("device_id_required".to_string()));
        }
        if device_id == session.device_id {
            return Err(StorageError::BadRequest(
                "cannot_revoke_current_device".to_string(),
            ));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let exists = conn
            .query_row(
                "SELECT 1 FROM cloud_devices WHERE account_id = ?1 AND id = ?2",
                params![session.account_id, device_id],
                |_| Ok(()),
            )
            .optional()?
            .is_some();
        if !exists {
            return Err(StorageError::Unauthorized);
        }
        let now = Utc::now().to_rfc3339();
        let revoked = conn.execute(
            "UPDATE cloud_sessions
             SET revoked_at = COALESCE(revoked_at, ?3), last_used_at = ?3
             WHERE account_id = ?1 AND device_id = ?2 AND revoked_at IS NULL",
            params![session.account_id, device_id, now],
        )?;
        conn.execute(
            "UPDATE cloud_devices
             SET registered = 0, auto_sync_enabled = 0, updated_at = ?3
             WHERE account_id = ?1 AND id = ?2",
            params![session.account_id, device_id, now],
        )?;
        Ok(RevokeDeviceResponse {
            ok: true,
            device_id: device_id.to_string(),
            revoked_session_count: revoked as u32,
        })
    }

    pub fn push_cloud_events_batch(
        &self,
        access_token: &str,
        request: &CloudSyncBatchRequest,
    ) -> Result<CloudSyncBatchResult, StorageError> {
        let session = self.authenticate(access_token)?;
        let device_id = request.device_id.trim();
        if device_id.is_empty() {
            return Err(StorageError::BadRequest("deviceId is required".to_string()));
        }
        if session.device_id != device_id {
            return Err(StorageError::Unauthorized);
        }
        let workspace_id = request.workspace_id.trim();
        if workspace_id.is_empty() {
            return Err(StorageError::BadRequest(
                "workspaceId is required".to_string(),
            ));
        }
        if !self.session_workspace_matches(&session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        self.ensure_cloud_sync_entitled(&session.account_id)?;
        if request.events.is_empty() {
            return Err(StorageError::BadRequest(
                "events must not be empty".to_string(),
            ));
        }

        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut accepted_event_ids = Vec::new();
        let mut event_results = Vec::new();

        for event in &request.events {
            let client_event_id = event.client_event_id.trim();
            let entity_type = event.entity_type.trim();
            let entity_id = event.entity_id.trim();
            if client_event_id.is_empty() || entity_type.is_empty() || entity_id.is_empty() {
                event_results.push(CloudSyncEventDisposition {
                    client_event_id: client_event_id.to_string(),
                    disposition: "rejected_invalid_event".to_string(),
                    payload_hash: None,
                    entity_revision: None,
                    message: Some(
                        "clientEventId, entityType and entityId are required".to_string(),
                    ),
                });
                continue;
            }

            let payload_json =
                serde_json::to_string(&event.payload).unwrap_or_else(|_| "{}".to_string());
            let payload_hash = cloud_sync_payload_hash(&event.payload);
            let entity_revision = cloud_sync_entity_revision(&event.payload);
            let existing = tx
                .query_row(
                    "SELECT payload_hash, entity_revision FROM cloud_sync_events
                     WHERE account_id = ?1 AND device_id = ?2 AND client_event_id = ?3
                     LIMIT 1",
                    params![session.account_id, session.device_id, client_event_id],
                    |row| {
                        Ok((
                            row.get::<_, Option<String>>(0)?,
                            row.get::<_, Option<i64>>(1)?,
                        ))
                    },
                )
                .map(Some)
                .or_else(|err| match err {
                    rusqlite::Error::QueryReturnedNoRows => Ok(None),
                    other => Err(other),
                })?;
            if let Some((existing_hash, existing_revision)) = existing {
                if existing_hash
                    .as_deref()
                    .is_some_and(|hash| hash != payload_hash)
                {
                    event_results.push(CloudSyncEventDisposition {
                        client_event_id: client_event_id.to_string(),
                        disposition: "conflict_payload_changed".to_string(),
                        payload_hash: Some(payload_hash),
                        entity_revision,
                        message: Some(
                            "same clientEventId was received with a different payload hash"
                                .to_string(),
                        ),
                    });
                    continue;
                }
                accepted_event_ids.push(client_event_id.to_string());
                event_results.push(CloudSyncEventDisposition {
                    client_event_id: client_event_id.to_string(),
                    disposition: "duplicate".to_string(),
                    payload_hash: existing_hash.or(Some(payload_hash)),
                    entity_revision: existing_revision.or(entity_revision),
                    message: None,
                });
                continue;
            }

            tx.execute(
                "INSERT INTO cloud_sync_events (
                    account_id, device_id, client_event_id, operation, entity_type,
                    entity_id, payload_json, payload_hash, entity_revision, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    &session.account_id,
                    &session.device_id,
                    client_event_id,
                    event.operation.trim(),
                    entity_type,
                    entity_id,
                    &payload_json,
                    &payload_hash,
                    entity_revision,
                    Utc::now().to_rfc3339(),
                ],
            )?;
            let _ = upsert_rights_manifest_from_sync_payload_tx(
                &tx,
                &session.account_id,
                &session.device_id,
                entity_type,
                &payload_json,
            )?;
            accepted_event_ids.push(client_event_id.to_string());
            event_results.push(CloudSyncEventDisposition {
                client_event_id: client_event_id.to_string(),
                disposition: "accepted".to_string(),
                payload_hash: Some(payload_hash),
                entity_revision,
                message: None,
            });
        }

        tx.commit()?;
        let next_cursor = account_cursor_with_conn(&conn, &session.account_id)?;
        Ok(CloudSyncBatchResult {
            accepted: accepted_event_ids.len() as u32,
            accepted_event_ids,
            next_cursor,
            resolutions: serde_json::json!([]),
            event_results,
        })
    }

    pub fn public_rights_query(
        &self,
        watermark_uid: &str,
    ) -> Result<PublicRightsQueryResponse, StorageError> {
        let watermark_uid = normalize_watermark_uid(watermark_uid)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        public_rights_query_with_conn(&conn, &watermark_uid)
    }

    pub fn public_rights_batch(
        &self,
        request: &PublicRightsBatchRequest,
    ) -> Result<PublicRightsBatchResponse, StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        public_rights_batch_with_conn(&conn, request)
    }

    pub fn enterprise_public_rights_batch(
        &self,
        cleartext_api_key: &str,
        hash_secret: &str,
        hash_secret_version: &str,
        client_fingerprint: EnterpriseGatewayClientFingerprint,
        request: &EnterprisePublicRightsBatchRequest,
    ) -> Result<EnterprisePublicRightsBatchResponse, StorageError> {
        let watermark_uids = request
            .watermark_uids
            .iter()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect::<Vec<_>>();
        if watermark_uids.is_empty()
            || watermark_uids.len() > PUBLIC_RIGHTS_ANONYMOUS_BATCH_MAX_ITEMS
        {
            return Err(StorageError::BadRequest(
                "watermarkUids exceeds maximum batch size".to_string(),
            ));
        }
        let request_id = request
            .idempotency_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
            .unwrap_or_else(|| format!("ent_req_{}", uuid::Uuid::new_v4()));
        let key_prefix = enterprise_cleartext_key_prefix(cleartext_api_key)?;
        let key_hash =
            enterprise_api_key_hash_hex(cleartext_api_key, hash_secret, hash_secret_version)?;
        let now = Utc::now();
        let endpoint = "/v1/enterprise/public-rights/batch";
        let method = "POST";
        let mut rate_limit = EnterpriseGatewayRateLimitPolicy {
            policy_id: "enterprise_public_rights_default".to_string(),
            requests_per_minute: 60,
            items_per_minute: 600,
            burst_requests: 10,
            retry_after_seconds: 60,
        };
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let auth = match load_enterprise_api_key_auth_tx(&tx, &key_prefix, &key_hash, now) {
            Ok(auth) => auth,
            Err(error) => {
                tx.commit()?;
                return Err(error);
            }
        };
        let client_fingerprint =
            normalize_enterprise_client_fingerprint(&client_fingerprint, &auth.api_key_id);
        if !client_fingerprint.fingerprint_hash.is_empty() {
            rate_limit.policy_id = format!(
                "{}:{}",
                rate_limit.policy_id, client_fingerprint.fingerprint_hash
            );
        }
        let (current_window_requests, current_window_items) =
            enterprise_rate_limit_window_tx(&tx, &auth.api_key_id, &rate_limit, now)?;
        let quota_balance = load_active_enterprise_quota_balance_tx(
            &tx,
            &auth.account_id,
            &auth.workspace_id,
            ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE,
            now,
        )?;
        let decision = dry_run_enterprise_gateway_readonly_scan(&EnterpriseGatewayDryRunRequest {
            auth: auth.clone(),
            required_scope: "public_rights:batch_read".to_string(),
            endpoint: endpoint.to_string(),
            method: method.to_string(),
            request_id: request_id.clone(),
            item_count: watermark_uids.len() as u32,
            quota_type: ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE.to_string(),
            quota_included_units: quota_balance.included_units,
            quota_used_units: quota_balance.used_units,
            quota_reserved_units: quota_balance.reserved_units,
            quota_overage_allowed: quota_balance.overage_allowed,
            rate_limit: rate_limit.clone(),
            client_fingerprint: client_fingerprint.clone(),
            current_window_requests,
            current_window_items,
            charge_on_not_found: true,
            charge_metadata_export: false,
        });
        if !decision.allowed {
            record_enterprise_api_audit_event_tx(
                &tx,
                &EnterpriseApiAuditEventRequest {
                    account_id: auth.account_id,
                    workspace_id: auth.workspace_id,
                    api_key_id: Some(auth.api_key_id),
                    endpoint: endpoint.to_string(),
                    method: method.to_string(),
                    request_count: 1,
                    item_count: watermark_uids.len() as u32,
                    status_code: decision.status_code,
                    error_code: decision.error_code.clone(),
                    quota_units: 0,
                    client_label: request.client_label.clone(),
                    client_fingerprint_hash: Some(client_fingerprint.fingerprint_hash.clone())
                        .filter(|value| !value.is_empty()),
                    trusted_proxy_status: Some(client_fingerprint.source.clone()),
                    request_id,
                },
            )?;
            tx.commit()?;
            return Err(gateway_decision_error(decision.error_code.as_deref()));
        }
        increment_enterprise_rate_limit_window_tx(
            &tx,
            &auth.api_key_id,
            &rate_limit,
            watermark_uids.len() as i64,
            now,
        )?;
        let batch = public_rights_batch_with_conn(
            &tx,
            &PublicRightsBatchRequest {
                watermark_uids: watermark_uids.clone(),
            },
        )?;
        let quota_units = decision.quota.chargeable_units;
        record_enterprise_quota_ledger_tx(
            &tx,
            &EnterpriseQuotaLedgerRequest {
                account_id: auth.account_id.clone(),
                workspace_id: auth.workspace_id.clone(),
                api_key_id: Some(auth.api_key_id.clone()),
                quota_type: ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE.to_string(),
                units: quota_units,
                direction: "debit".to_string(),
                event_type: "public_rights_batch_scan".to_string(),
                reference_id: request_id.clone(),
                idempotency_key: format!(
                    "{}:{}:{}",
                    request_id,
                    ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE,
                    watermark_uids.len()
                ),
                status: "committed".to_string(),
            },
        )?;
        tx.execute(
            "UPDATE enterprise_quota_balances
             SET used_units = used_units + ?1, updated_at = ?2
             WHERE quota_balance_id = ?3",
            params![
                quota_units,
                now.to_rfc3339(),
                quota_balance.quota_balance_id
            ],
        )?;
        tx.execute(
            "UPDATE enterprise_api_keys SET last_used_at = ?1 WHERE api_key_id = ?2",
            params![now.to_rfc3339(), auth.api_key_id],
        )?;
        record_enterprise_api_audit_event_tx(
            &tx,
            &EnterpriseApiAuditEventRequest {
                account_id: auth.account_id.clone(),
                workspace_id: auth.workspace_id.clone(),
                api_key_id: Some(auth.api_key_id.clone()),
                endpoint: endpoint.to_string(),
                method: method.to_string(),
                request_count: 1,
                item_count: watermark_uids.len() as u32,
                status_code: 200,
                error_code: None,
                quota_units,
                client_label: request.client_label.clone(),
                client_fingerprint_hash: Some(client_fingerprint.fingerprint_hash.clone())
                    .filter(|value| !value.is_empty()),
                trusted_proxy_status: Some(client_fingerprint.source.clone()),
                request_id: request_id.clone(),
            },
        )?;
        tx.commit()?;
        Ok(EnterprisePublicRightsBatchResponse {
            gateway: EnterprisePublicRightsGateway {
                request_id,
                api_key_id: auth.api_key_id,
                account_id: auth.account_id,
                workspace_id: auth.workspace_id,
                quota_type: ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE.to_string(),
                quota_charged_units: quota_units,
                rate_limit_policy_id: rate_limit.policy_id,
                client_fingerprint_hash: Some(client_fingerprint.fingerprint_hash)
                    .filter(|value| !value.is_empty()),
                trusted_proxy_status: client_fingerprint.source,
                legal_conclusion: false,
            },
            batch,
        })
    }

    pub fn public_rights_metadata_export(
        &self,
        watermark_uid: &str,
    ) -> Result<PublicRightsMetadataExport, StorageError> {
        let watermark_uid = normalize_watermark_uid(watermark_uid)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let rights = public_rights_query_with_conn(&conn, &watermark_uid)?;
        Ok(public_rights_metadata_export_from_query(&rights))
    }

    pub fn backfill_rights_manifests(
        &self,
        request: &RightsManifestBackfillRequest,
    ) -> Result<RightsManifestBackfillResponse, StorageError> {
        let limit = request.limit.unwrap_or(50).clamp(1, 200);
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let watermark_uids = if request.watermark_uids.is_empty() {
            list_backfill_watermark_uids_tx(&tx, request.cursor.as_deref(), limit)?
        } else {
            request
                .watermark_uids
                .iter()
                .take(limit as usize)
                .map(|uid| normalize_watermark_uid(uid))
                .collect::<Result<Vec<_>, _>>()?
        };

        let mut processed = 0_u32;
        let mut succeeded = 0_u32;
        let mut needs_review = 0_u32;
        let mut retryable = 0_u32;
        let mut results = Vec::new();

        for watermark_uid in watermark_uids {
            processed += 1;
            match backfill_rights_manifest_for_uid_tx(&tx, &watermark_uid)? {
                BackfillOutcome::Succeeded {
                    rights_manifest_id,
                    manifest_version,
                    message,
                } => {
                    succeeded += 1;
                    results.push(RightsManifestBackfillItem {
                        watermark_uid,
                        status: "succeeded".to_string(),
                        error_code: None,
                        rights_manifest_id: Some(rights_manifest_id),
                        manifest_version: Some(manifest_version),
                        message,
                    });
                }
                BackfillOutcome::NeedsReview { code, message } => {
                    needs_review += 1;
                    results.push(RightsManifestBackfillItem {
                        watermark_uid,
                        status: "needs_review".to_string(),
                        error_code: Some(code),
                        rights_manifest_id: None,
                        manifest_version: None,
                        message,
                    });
                }
                BackfillOutcome::Retryable { code, message } => {
                    retryable += 1;
                    results.push(RightsManifestBackfillItem {
                        watermark_uid,
                        status: "retryable".to_string(),
                        error_code: Some(code),
                        rights_manifest_id: None,
                        manifest_version: None,
                        message,
                    });
                }
            }
        }
        tx.commit()?;
        let next_cursor = results.last().map(|item| item.watermark_uid.clone());
        Ok(RightsManifestBackfillResponse {
            processed,
            succeeded,
            needs_review,
            retryable,
            next_cursor,
            results,
            completed_at: Utc::now(),
        })
    }

    pub fn get_cloud_changes(
        &self,
        access_token: &str,
        workspace_id: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<CloudSyncChangesResult, StorageError> {
        let session = self.authenticate(access_token)?;
        let workspace_id = workspace_id.unwrap_or_default().trim();
        if workspace_id.is_empty() {
            return Err(StorageError::BadRequest(
                "workspaceId is required".to_string(),
            ));
        }
        if !self.session_workspace_matches(&session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        self.ensure_cloud_sync_entitled(&session.account_id)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let stored_device_cursor =
            device_cursor_with_conn(&conn, &session.account_id, &session.device_id)?;
        let client_since_sequence = sequence_from_cursor(cursor);
        let stored_since_sequence = sequence_from_cursor(stored_device_cursor.as_deref());
        let since_sequence = if stored_device_cursor.is_some() {
            client_since_sequence.min(stored_since_sequence)
        } else {
            0
        };
        let mut stmt = conn.prepare(
            "SELECT sequence, device_id, operation, entity_type, entity_id, payload_json
             FROM cloud_sync_events
             WHERE account_id = ?1 AND sequence > ?2
             ORDER BY sequence ASC",
        )?;
        let rows = stmt.query_map(params![session.account_id, since_sequence], |row| {
            let payload_json: String = row.get(5)?;
            Ok(CloudSyncChange {
                cursor: Some(cursor_from_sequence(row.get::<_, i64>(0)? as u64)),
                entity_type: row.get(3)?,
                operation: cloud_operation(&row.get::<_, String>(2)?),
                source_device: Some(row.get(1)?),
                entity: serde_json::from_str(&payload_json)
                    .unwrap_or_else(|_| serde_json::json!({})),
            })
        })?;
        let changes = rows.collect::<Result<Vec<_>, _>>()?;

        let next_cursor =
            account_cursor_with_conn(&conn, &session.account_id)?.unwrap_or_else(|| {
                cursor
                    .map(ToOwned::to_owned)
                    .unwrap_or_else(|| cursor_from_sequence(since_sequence as u64))
            });
        upsert_device_cursor(&conn, &session.account_id, &session.device_id, &next_cursor)?;

        Ok(CloudSyncChangesResult {
            next_cursor,
            changes,
        })
    }

    pub fn create_video_fingerprint_notary(
        &self,
        access_token: &str,
        request: &VideoFingerprintNotaryRequest,
    ) -> Result<VideoFingerprintNotaryReceipt, StorageError> {
        let session = self.authenticate(access_token)?;
        let workspace_id = request.workspace_id.trim();
        if workspace_id.is_empty() {
            return Err(StorageError::BadRequest(
                "workspaceId is required".to_string(),
            ));
        }
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !session_workspace_matches_with_conn(&conn, &session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(
            &conn,
            &session.account_id,
            request.creator_profile_id.trim(),
        )? {
            return Err(StorageError::BadRequest(
                "creator_profile_required".to_string(),
            ));
        }

        validate_l2_notary_request(request)?;

        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = Utc::now();
        let now_text = now.to_rfc3339();
        let notary_id = format!(
            "vfn_{}_{}",
            short_id(&session.account_id),
            short_id(&format!("{}{}", request.watermark_uid, now_text))
        );
        let usage_ledger_id = format!("usage_{}", short_id(&format!("{notary_id}{now_text}")));
        let server_receipt_signature = format!(
            "mock_server_signature_{}",
            short_id(&format!("{}{}", notary_id, request.fingerprint_root))
        );
        let global_frame_fingerprints_json =
            serde_json::to_string(&request.global_frame_fingerprints)
                .map_err(|error| StorageError::BadRequest(error.to_string()))?;
        let upload_manifest_json = serde_json::to_string(&request.upload_manifest)
            .map_err(|error| StorageError::BadRequest(error.to_string()))?;

        tx.execute(
            "INSERT INTO video_fingerprint_notaries (
                notary_id, account_id, workspace_id, creator_profile_id, watermark_uid,
                source_hash, duration_ms, frame_sample_policy, scene_count,
                fingerprint_schema_version, global_frame_fingerprints_json,
                local_block_fingerprint_root, local_block_count,
                crop_window_fingerprint_root, crop_window_count,
                fingerprint_root, client_signature, server_receipt_signature,
                upload_manifest_json, created_at, notarized_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21)",
            params![
                notary_id,
                session.account_id,
                workspace_id,
                request.creator_profile_id.trim(),
                request.watermark_uid.trim(),
                request.source_hash.trim(),
                request.duration_ms as i64,
                request.frame_sample_policy.trim(),
                request.scene_count as i64,
                request.fingerprint_schema_version.trim(),
                global_frame_fingerprints_json,
                request.local_block_fingerprint_root.trim(),
                request.local_block_count as i64,
                request.crop_window_fingerprint_root.trim(),
                request.crop_window_count as i64,
                request.fingerprint_root.trim(),
                request.client_signature.trim(),
                server_receipt_signature,
                upload_manifest_json,
                now_text,
                now_text,
            ],
        )?;

        tx.execute(
            "INSERT INTO cloud_usage_ledger (
                usage_ledger_id, account_id, workspace_id, feature_name,
                usage_type, quota_type, quota_units, occurred_at, reference_id
            ) VALUES (?1, ?2, ?3, 'video_fingerprint_notary', 'usage_ledger', NULL, 0, ?4, ?5)",
            params![
                usage_ledger_id,
                session.account_id,
                workspace_id,
                now_text,
                notary_id,
            ],
        )?;

        tx.commit()?;
        Ok(VideoFingerprintNotaryReceipt {
            schema_version: "video_fingerprint_notary_receipt_v1".to_string(),
            notary_id,
            watermark_uid: request.watermark_uid.trim().to_string(),
            source_hash: request.source_hash.trim().to_string(),
            fingerprint_root: request.fingerprint_root.trim().to_string(),
            notarized_at: now,
            server_receipt_signature,
            usage_ledger_id,
        })
    }

    pub fn create_cloud_video_task(
        &self,
        access_token: &str,
        request: &CloudVideoTaskRequest,
    ) -> Result<CloudVideoTaskRecord, StorageError> {
        let session = self.authenticate(access_token)?;
        let workspace_id = request.workspace_id.trim();
        if workspace_id.is_empty() {
            return Err(StorageError::BadRequest(
                "workspaceId is required".to_string(),
            ));
        }
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !session_workspace_matches_with_conn(&conn, &session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(
            &conn,
            &session.account_id,
            request.creator_profile_id.trim(),
        )? {
            return Err(StorageError::BadRequest(
                "creator_profile_required".to_string(),
            ));
        }

        validate_cloud_video_task_request(request)?;

        let entitlement = cloud_entitlement_for_account(&conn, &session.account_id)?;
        let cloud_video_processing = entitlement
            .features
            .get("cloud_video_processing")
            .and_then(serde_json::Value::as_bool)
            == Some(true);
        if !cloud_video_processing {
            return Err(StorageError::Forbidden);
        }

        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = Utc::now();
        let now_text = now.to_rfc3339();
        let task_id = format!(
            "l3task_{}_{}",
            short_id(&session.account_id),
            short_id(&format!("{}{}", request.watermark_uid, now_text))
        );
        let quota_units = quota_units_for_duration_ms(request.duration_ms);
        let target_profiles_json = serde_json::to_string(&request.target_profiles)
            .map_err(|error| StorageError::BadRequest(error.to_string()))?;
        let upload_manifest_json = serde_json::to_string(&request.upload_manifest)
            .map_err(|error| StorageError::BadRequest(error.to_string()))?;

        tx.execute(
            "INSERT INTO cloud_video_tasks (
                task_id, schema_version, account_id, workspace_id, creator_profile_id,
                capability_level, watermark_uid, source_hash, duration_ms, target_profiles_json,
                upload_manifest_json, status, quota_units, failure_code, strategy_digest,
                self_check_threshold, self_check_confidence, checked_frames,
                watermarked_media_hash, output_media_storage_ref, output_media_bytes,
                output_media_content_type, worker_receipt_hash, worker_receipt_json,
                server_receipt_signature, usage_ledger_id,
                created_at, updated_at, completed_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'draft', ?12, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?13, ?13, NULL)",
            params![
                task_id,
                request.schema_version.trim(),
                session.account_id,
                workspace_id,
                request.creator_profile_id.trim(),
                request.capability_level.trim(),
                request.watermark_uid.trim(),
                request.source_hash.trim(),
                request.duration_ms as i64,
                target_profiles_json,
                upload_manifest_json,
                quota_units as i64,
                now_text,
            ],
        )?;
        tx.commit()?;
        load_cloud_video_task_with_conn(&conn, &task_id)
    }

    pub fn authorize_cloud_video_object_upload(
        &self,
        access_token: &str,
        workspace_id: &str,
        creator_profile_id: &str,
    ) -> Result<String, StorageError> {
        let session = self.authenticate(access_token)?;
        let workspace_id = workspace_id.trim();
        let creator_profile_id = creator_profile_id.trim();
        if workspace_id.is_empty() {
            return Err(StorageError::BadRequest(
                "workspaceId is required".to_string(),
            ));
        }
        if creator_profile_id.is_empty() {
            return Err(StorageError::BadRequest(
                "creator_profile_required".to_string(),
            ));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !session_workspace_matches_with_conn(&conn, &session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(&conn, &session.account_id, creator_profile_id)? {
            return Err(StorageError::BadRequest(
                "creator_profile_required".to_string(),
            ));
        }
        let entitlement = cloud_entitlement_for_account(&conn, &session.account_id)?;
        let cloud_video_processing = entitlement
            .features
            .get("cloud_video_processing")
            .and_then(serde_json::Value::as_bool)
            == Some(true);
        if !cloud_video_processing {
            return Err(StorageError::Forbidden);
        }
        Ok(session.account_id)
    }

    pub fn list_cloud_video_tasks(
        &self,
        access_token: &str,
        query: &CloudVideoTaskListQuery,
    ) -> Result<CloudVideoTaskListResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let workspace_id = query
            .workspace_id
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        if !workspace_id.is_empty()
            && !session_workspace_matches_with_conn(&conn, &session.account_id, workspace_id)?
        {
            return Err(StorageError::Forbidden);
        }
        let limit = query.limit.unwrap_or(50).clamp(1, 200) as i64;
        let status = query.status.as_deref().map(str::trim).unwrap_or_default();
        if !status.is_empty() && !is_cloud_video_task_status(status) {
            return Err(StorageError::BadRequest(
                "cloud_video_task_status_invalid".to_string(),
            ));
        }
        let mut stmt = conn.prepare(
            "SELECT task_id FROM cloud_video_tasks
             WHERE account_id = ?1
               AND (?2 = '' OR workspace_id = ?2)
               AND (?3 = '' OR status = ?3)
             ORDER BY created_at DESC
             LIMIT ?4",
        )?;
        let task_ids = stmt
            .query_map(
                params![session.account_id, workspace_id, status, limit],
                |row| row.get::<_, String>(0),
            )?
            .collect::<Result<Vec<_>, _>>()?;
        let mut tasks = Vec::with_capacity(task_ids.len());
        for task_id in task_ids {
            tasks.push(load_cloud_video_task_with_conn(&conn, &task_id)?);
        }
        Ok(CloudVideoTaskListResponse {
            returned: tasks.len() as u32,
            tasks,
        })
    }

    pub fn get_cloud_video_task(
        &self,
        access_token: &str,
        task_id: &str,
    ) -> Result<CloudVideoTaskRecord, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let task = load_cloud_video_task_with_conn(&conn, task_id)?;
        if task.account_id != session.account_id {
            return Err(StorageError::Forbidden);
        }
        if !session_workspace_matches_with_conn(&conn, &session.account_id, &task.workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        Ok(task)
    }

    pub fn get_cloud_video_task_for_signed_download(
        &self,
        task_id: &str,
    ) -> Result<CloudVideoTaskRecord, StorageError> {
        let task_id = task_id.trim();
        if task_id.is_empty() {
            return Err(StorageError::BadRequest("task_id_required".to_string()));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        load_cloud_video_task_with_conn(&conn, task_id)
    }

    pub fn claim_cloud_video_task_for_worker(
        &self,
        request: &CloudVideoTaskClaimRequest,
    ) -> Result<CloudVideoTaskClaimResponse, StorageError> {
        validate_cloud_video_task_claim_request(request)?;
        let worker_id = request.worker_id.trim();
        let capability_level = request
            .capability_level
            .as_deref()
            .map(str::trim)
            .unwrap_or_default();
        let lease_seconds = request.lease_seconds.unwrap_or(900).clamp(60, 3_600);

        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = Utc::now();
        let now_text = now.to_rfc3339();
        let lease_expires_at = now + Duration::seconds(lease_seconds as i64);
        let lease_expires_at_text = lease_expires_at.to_rfc3339();
        let selected = tx.query_row(
            "SELECT task_id, attempt_count
             FROM cloud_video_tasks
             WHERE (
                 status IN ('draft', 'queued')
                 OR (status = 'running' AND (lease_expires_at IS NULL OR lease_expires_at <= ?1))
             )
               AND (?2 = '' OR capability_level = ?2)
             ORDER BY created_at ASC
             LIMIT 1",
            params![now_text, capability_level],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)),
        );
        let (task_id, previous_attempt_count) = match selected {
            Ok(value) => value,
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                return Err(StorageError::BadRequest(
                    "cloud_video_task_queue_empty".to_string(),
                ))
            }
            Err(error) => return Err(StorageError::Database(error)),
        };
        let attempt_count = previous_attempt_count.saturating_add(1);
        let attempt_id = format!(
            "l3attempt_{}_{}",
            short_id(&task_id),
            short_id(&format!("{worker_id}{now_text}{attempt_count}"))
        );
        let lease_token = generate_cloud_video_lease_token();
        let lease_token_hash = cloud_video_lease_token_hash(&lease_token);
        tx.execute(
            "UPDATE cloud_video_tasks
             SET status = 'running',
                 worker_id = ?2,
                 attempt_id = ?3,
                 lease_token_hash = ?4,
                 attempt_count = ?5,
                 lease_expires_at = ?6,
                 updated_at = ?7,
                 completed_at = NULL
             WHERE task_id = ?1",
            params![
                task_id,
                worker_id,
                attempt_id,
                lease_token_hash,
                attempt_count,
                lease_expires_at_text,
                now_text,
            ],
        )?;
        tx.commit()?;
        let task = load_cloud_video_task_with_conn(&conn, &task_id)?;
        Ok(CloudVideoTaskClaimResponse {
            task,
            worker_id: worker_id.to_string(),
            attempt_id,
            lease_token,
            lease_expires_at,
        })
    }

    pub fn update_cloud_video_task_status(
        &self,
        access_token: &str,
        task_id: &str,
        request: &CloudVideoTaskStatusUpdateRequest,
    ) -> Result<CloudVideoTaskRecord, StorageError> {
        if request.status.trim() == CLOUD_VIDEO_TASK_STATUS_SUCCEEDED {
            return Err(StorageError::BadRequest(
                "cloud_video_task_completion_requires_trusted_worker".to_string(),
            ));
        }
        let session = self.authenticate(access_token)?;
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let existing = load_cloud_video_task_with_conn(&conn, task_id)?;
        if existing.account_id != session.account_id {
            return Err(StorageError::Forbidden);
        }
        if !session_workspace_matches_with_conn(&conn, &session.account_id, &existing.workspace_id)?
        {
            return Err(StorageError::Forbidden);
        }
        validate_cloud_video_task_status_update(request)?;

        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = Utc::now();
        let now_text = now.to_rfc3339();
        let status = request.status.trim().to_string();
        let completed_at = if matches!(
            status.as_str(),
            CLOUD_VIDEO_TASK_STATUS_SUCCEEDED
                | CLOUD_VIDEO_TASK_STATUS_FAILED
                | CLOUD_VIDEO_TASK_STATUS_CANCELED
                | CLOUD_VIDEO_TASK_STATUS_EXPIRED
        ) {
            Some(now_text.clone())
        } else {
            None
        };
        let mut usage_ledger_id: Option<String> = existing.usage_ledger_id.clone();
        if status == CLOUD_VIDEO_TASK_STATUS_SUCCEEDED && usage_ledger_id.is_none() {
            let ledger_id = format!("usage_{}", short_id(&format!("{task_id}{now_text}")));
            tx.execute(
                "INSERT INTO cloud_usage_ledger (
                    usage_ledger_id, account_id, workspace_id, feature_name,
                    usage_type, quota_type, quota_units, occurred_at, reference_id
                ) VALUES (?1, ?2, ?3, 'cloud_video_processing', 'quota_ledger', 'video_minutes', ?4, ?5, ?6)",
                params![
                    ledger_id,
                    existing.account_id,
                    existing.workspace_id,
                    existing.quota_units as i64,
                    now_text,
                    task_id,
                ],
            )?;
            usage_ledger_id = Some(ledger_id);
        }
        tx.execute(
            "UPDATE cloud_video_tasks
             SET status = ?2,
                 failure_code = ?3,
                 strategy_digest = COALESCE(?4, strategy_digest),
                 self_check_threshold = COALESCE(?5, self_check_threshold),
                 self_check_confidence = COALESCE(?6, self_check_confidence),
                 checked_frames = COALESCE(?7, checked_frames),
                 watermarked_media_hash = COALESCE(?8, watermarked_media_hash),
                 server_receipt_signature = COALESCE(?9, server_receipt_signature),
                 usage_ledger_id = COALESCE(?10, usage_ledger_id),
                 updated_at = ?11,
                 completed_at = COALESCE(completed_at, ?12)
             WHERE task_id = ?1",
            params![
                task_id,
                status,
                request.failure_code.as_deref(),
                request.strategy_digest.as_deref(),
                request.self_check_threshold,
                request.self_check_confidence,
                request.checked_frames.map(|value| value as i64),
                request.watermarked_media_hash.as_deref(),
                request.server_receipt_signature.as_deref(),
                usage_ledger_id.as_deref(),
                now_text,
                completed_at,
            ],
        )?;
        tx.commit()?;
        load_cloud_video_task_with_conn(&conn, task_id)
    }

    pub fn fail_cloud_video_task_from_trusted_worker(
        &self,
        task_id: &str,
        request: &CloudVideoTaskFailureRequest,
    ) -> Result<CloudVideoTaskRecord, StorageError> {
        validate_cloud_video_task_failure_request(request)?;
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let existing = load_cloud_video_task_with_conn(&conn, task_id)?;
        validate_cloud_video_task_active_lease(
            &conn,
            &existing,
            request.worker_id.trim(),
            request.attempt_id.trim(),
            request.lease_token.trim(),
        )?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now_text = Utc::now().to_rfc3339();
        let status = if request.retryable {
            CLOUD_VIDEO_TASK_STATUS_QUEUED
        } else {
            CLOUD_VIDEO_TASK_STATUS_FAILED
        };
        let completed_at = if request.retryable {
            None
        } else {
            Some(now_text.clone())
        };
        tx.execute(
            "UPDATE cloud_video_tasks
             SET status = ?2,
                 failure_code = ?3,
                 lease_token_hash = NULL,
                 lease_expires_at = NULL,
                 last_failure_code = ?3,
                 last_failure_stage = ?4,
                 updated_at = ?5,
                 completed_at = COALESCE(completed_at, ?6)
             WHERE task_id = ?1",
            params![
                task_id,
                status,
                request.failure_code.trim(),
                request.failure_stage.as_deref().map(str::trim),
                now_text,
                completed_at,
            ],
        )?;
        tx.commit()?;
        load_cloud_video_task_with_conn(&conn, task_id)
    }

    pub fn complete_cloud_video_task_from_trusted_worker(
        &self,
        task_id: &str,
        request: &CloudVideoTaskCompletionRequest,
    ) -> Result<CloudVideoTaskRecord, StorageError> {
        validate_cloud_video_task_completion_request(request)?;
        let update = CloudVideoTaskStatusUpdateRequest {
            status: CLOUD_VIDEO_TASK_STATUS_SUCCEEDED.to_string(),
            failure_code: None,
            strategy_digest: Some(request.strategy_digest.trim().to_string()),
            self_check_threshold: Some(request.self_check_threshold),
            self_check_confidence: Some(request.self_check_confidence),
            checked_frames: Some(request.checked_frames),
            watermarked_media_hash: Some(request.watermarked_media_hash.trim().to_string()),
            server_receipt_signature: Some(request.server_receipt_signature.trim().to_string()),
        };
        validate_cloud_video_task_status_update(&update)?;

        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let existing = load_cloud_video_task_with_conn(&conn, task_id)?;
        if existing.status == CLOUD_VIDEO_TASK_STATUS_SUCCEEDED {
            return Err(StorageError::BadRequest(
                "cloud_video_task_already_succeeded".to_string(),
            ));
        }
        validate_cloud_video_task_active_lease(
            &conn,
            &existing,
            request.worker_id.trim(),
            request.attempt_id.trim(),
            request.lease_token.trim(),
        )?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = Utc::now();
        let now_text = now.to_rfc3339();
        let mut usage_ledger_id: Option<String> = existing.usage_ledger_id.clone();
        if usage_ledger_id.is_none() {
            let ledger_id = format!("usage_{}", short_id(&format!("{task_id}{now_text}")));
            tx.execute(
                "INSERT INTO cloud_usage_ledger (
                    usage_ledger_id, account_id, workspace_id, feature_name,
                    usage_type, quota_type, quota_units, occurred_at, reference_id
                ) VALUES (?1, ?2, ?3, 'cloud_video_processing', 'quota_ledger', 'video_minutes', ?4, ?5, ?6)",
                params![
                    ledger_id,
                    existing.account_id,
                    existing.workspace_id,
                    existing.quota_units as i64,
                    now_text,
                    task_id,
                ],
            )?;
            usage_ledger_id = Some(ledger_id);
        }
        tx.execute(
            "UPDATE cloud_video_tasks
             SET status = 'succeeded',
                 failure_code = NULL,
                 strategy_digest = ?2,
                 self_check_threshold = ?3,
                 self_check_confidence = ?4,
                 checked_frames = ?5,
                 watermarked_media_hash = ?6,
                 output_media_storage_ref = ?7,
                 output_media_bytes = ?8,
                 output_media_content_type = ?9,
                 worker_receipt_hash = ?10,
                 worker_receipt_json = ?11,
                 server_receipt_signature = ?12,
                 usage_ledger_id = COALESCE(?13, usage_ledger_id),
                 lease_token_hash = NULL,
                 lease_expires_at = NULL,
                 failure_code = NULL,
                 last_failure_code = NULL,
                 last_failure_stage = NULL,
                 updated_at = ?14,
                 completed_at = COALESCE(completed_at, ?14)
             WHERE task_id = ?1",
            params![
                task_id,
                update.strategy_digest.as_deref(),
                update.self_check_threshold,
                update.self_check_confidence,
                update.checked_frames.map(|value| value as i64),
                update.watermarked_media_hash.as_deref(),
                request.output_media_storage_ref.trim(),
                request.output_media_bytes as i64,
                request.output_media_content_type.trim(),
                request.worker_receipt_hash.trim(),
                serde_json::to_string(&request.worker_receipt)
                    .map_err(|error| StorageError::BadRequest(error.to_string()))?,
                update.server_receipt_signature.as_deref(),
                usage_ledger_id.as_deref(),
                now_text,
            ],
        )?;
        tx.commit()?;
        load_cloud_video_task_with_conn(&conn, task_id)
    }

    pub fn create_billing_payment_session(
        &self,
        request: &BillingPaymentSessionRequest,
    ) -> Result<BillingPaymentSessionResponse, StorageError> {
        let plan_code = normalize_self_service_plan_code(&request.plan_code)?;
        let billing_cycle = normalize_billing_cycle(&request.billing_cycle)?;
        let provider = request
            .preferred_provider
            .as_deref()
            .unwrap_or(FIXTURE_PROVIDER)
            .trim();
        if provider != FIXTURE_PROVIDER {
            return Err(StorageError::BadRequest(
                "billing_provider_not_available".to_string(),
            ));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !account_workspace_matches_with_conn(&conn, &request.account_id, &request.workspace_id)?
        {
            return Err(StorageError::Forbidden);
        }
        drop(conn);

        let input = BillingPaymentSessionInput {
            account_id: request.account_id.trim().to_string(),
            workspace_id: request.workspace_id.trim().to_string(),
            plan_code,
            billing_cycle,
        };
        let provider = FixtureBillingProvider;
        let session = provider.create_payment_session(&input);
        let payment_action = BillingPaymentAction {
            action_type: session.action.action_type.clone(),
            qr_code_url: session.action.qr_code_url.clone(),
            h5_url: session.action.h5_url.clone(),
        };
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        persist_billing_payment_session(
            &conn,
            &session,
            &input,
            &payment_action,
            plan_amount_cents(&input.plan_code, &input.billing_cycle),
            "CNY",
            "created",
        )?;
        Ok(BillingPaymentSessionResponse {
            payment_session_id: session.payment_session_id,
            provider: session.provider,
            provider_order_id: session.provider_order_id,
            payment_action,
            expires_at: session.expires_at,
        })
    }

    pub fn persist_provider_billing_payment_session(
        &self,
        request: &BillingPaymentSessionRequest,
        provider: &str,
        provider_order_id: &str,
        payment_action: BillingPaymentAction,
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<BillingPaymentSessionResponse, StorageError> {
        let plan_code = normalize_self_service_plan_code(&request.plan_code)?;
        let billing_cycle = normalize_billing_cycle(&request.billing_cycle)?;
        if !matches!(
            provider.trim(),
            FIXTURE_PROVIDER | WECHAT_PAY_PROVIDER | "stripe"
        ) {
            return Err(StorageError::BadRequest(
                "billing_provider_not_available".to_string(),
            ));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !account_workspace_matches_with_conn(&conn, &request.account_id, &request.workspace_id)?
        {
            return Err(StorageError::Forbidden);
        }
        let input = BillingPaymentSessionInput {
            account_id: request.account_id.trim().to_string(),
            workspace_id: request.workspace_id.trim().to_string(),
            plan_code,
            billing_cycle,
        };
        let provider = provider.trim().to_string();
        let provider_order_id = provider_order_id.trim().to_string();
        if provider_order_id.is_empty() {
            return Err(StorageError::BadRequest(
                "provider_order_id_required".to_string(),
            ));
        }
        let session = BillingPaymentSession {
            payment_session_id: format!(
                "pay_sess_{}",
                short_id(&format!("{provider}:{provider_order_id}"))
            ),
            provider,
            provider_order_id,
            action: ProviderPaymentAction {
                action_type: payment_action.action_type.clone(),
                qr_code_url: payment_action.qr_code_url.clone(),
                h5_url: payment_action.h5_url.clone(),
            },
            expires_at,
        };
        persist_billing_payment_session(
            &conn,
            &session,
            &input,
            &payment_action,
            plan_amount_cents(&input.plan_code, &input.billing_cycle),
            "CNY",
            "created",
        )?;
        Ok(BillingPaymentSessionResponse {
            payment_session_id: session.payment_session_id,
            provider: session.provider,
            provider_order_id: session.provider_order_id,
            payment_action,
            expires_at: session.expires_at,
        })
    }

    pub fn create_report_purchase_session(
        &self,
        request: &ReportPurchaseSessionRequest,
    ) -> Result<ReportPurchaseSessionResponse, StorageError> {
        let product_code = normalize_report_product_code(&request.product_code)?;
        let price_cents = report_product_price_cents(&product_code)?;
        let provider = request
            .preferred_provider
            .as_deref()
            .unwrap_or(FIXTURE_PROVIDER)
            .trim();
        if provider != FIXTURE_PROVIDER {
            return Err(StorageError::BadRequest(
                "billing_provider_not_available".to_string(),
            ));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !account_workspace_matches_with_conn(&conn, &request.account_id, &request.workspace_id)?
        {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(
            &conn,
            &request.account_id,
            &request.creator_profile_id,
        )? {
            return Err(StorageError::Forbidden);
        }
        drop(conn);

        let order_seed = format!(
            "report:{}:{}:{}:{}:{}",
            request.account_id.trim(),
            request.workspace_id.trim(),
            request.creator_profile_id.trim(),
            request.vault_record_id.trim(),
            product_code
        );
        let order_hash = short_id(&order_seed);
        let session = BillingPaymentSession {
            payment_session_id: format!("rpt_pay_sess_{order_hash}"),
            provider: FIXTURE_PROVIDER.to_string(),
            provider_order_id: format!("fixture_report_order_{order_hash}"),
            action: ProviderPaymentAction {
                action_type: "qr_code".to_string(),
                qr_code_url: Some(format!("fixture://pay/report/{product_code}/{order_hash}")),
                h5_url: None,
            },
            expires_at: Utc::now() + Duration::minutes(15),
        };
        let payment_action = BillingPaymentAction {
            action_type: session.action.action_type.clone(),
            qr_code_url: session.action.qr_code_url.clone(),
            h5_url: session.action.h5_url.clone(),
        };
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        persist_report_purchase_session(
            &conn,
            &session,
            request,
            &product_code,
            price_cents,
            &payment_action,
            "created",
        )?;
        Ok(ReportPurchaseSessionResponse {
            payment_session_id: session.payment_session_id,
            provider: session.provider,
            provider_order_id: session.provider_order_id,
            product_code,
            price_cents,
            currency: "CNY".to_string(),
            payment_action,
            expires_at: session.expires_at,
        })
    }

    pub fn persist_provider_report_purchase_session(
        &self,
        request: &ReportPurchaseSessionRequest,
        provider: &str,
        provider_order_id: &str,
        payment_action: BillingPaymentAction,
        expires_at: chrono::DateTime<Utc>,
    ) -> Result<ReportPurchaseSessionResponse, StorageError> {
        let provider = provider.trim().to_string();
        let provider_order_id = required_trimmed(provider_order_id, "provider_order_id_required")?;
        let product_code = normalize_report_product_code(&request.product_code)?;
        let price_cents = report_product_price_cents(&product_code)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !account_workspace_matches_with_conn(&conn, &request.account_id, &request.workspace_id)?
        {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(
            &conn,
            &request.account_id,
            &request.creator_profile_id,
        )? {
            return Err(StorageError::Forbidden);
        }
        let session = BillingPaymentSession {
            payment_session_id: format!(
                "rpt_pay_sess_{}",
                short_id(&format!("{provider}:{provider_order_id}"))
            ),
            provider,
            provider_order_id,
            action: ProviderPaymentAction {
                action_type: payment_action.action_type.clone(),
                qr_code_url: payment_action.qr_code_url.clone(),
                h5_url: payment_action.h5_url.clone(),
            },
            expires_at,
        };
        persist_report_purchase_session(
            &conn,
            &session,
            request,
            &product_code,
            price_cents,
            &payment_action,
            "created",
        )?;
        Ok(ReportPurchaseSessionResponse {
            payment_session_id: session.payment_session_id,
            provider: session.provider,
            provider_order_id: session.provider_order_id,
            product_code,
            price_cents,
            currency: "CNY".to_string(),
            payment_action,
            expires_at: session.expires_at,
        })
    }

    pub fn report_purchase_session_status(
        &self,
        access_token: &str,
        payment_session_id: &str,
    ) -> Result<ReportPurchaseSessionStatusResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let payment = load_report_purchase_session(&conn, payment_session_id)?;
        if payment.account_id != session.account_id {
            return Err(StorageError::Forbidden);
        }
        let grant = load_report_purchase_grant_for_session(&conn, &payment.payment_session_id)?;
        Ok(ReportPurchaseSessionStatusResponse {
            payment_session_id: payment.payment_session_id,
            provider: payment.provider,
            provider_order_id: payment.provider_order_id,
            status: payment.status,
            product_code: payment.product_code,
            price_cents: payment.price_cents,
            currency: payment.currency,
            vault_record_id: payment.vault_record_id,
            expires_at: parse_rfc3339_utc(&payment.expires_at)?,
            last_checked_at: parse_optional_rfc3339_utc(payment.last_checked_at.as_deref())?,
            next_check_after: parse_optional_rfc3339_utc(payment.next_check_after.as_deref())?,
            check_attempts: payment.check_attempts.max(0) as u32,
            grant,
        })
    }

    pub fn reconcile_report_purchase_session(
        &self,
        access_token: &str,
        payment_session_id: &str,
    ) -> Result<ReportPurchaseSessionReconcileResponse, StorageError> {
        let auth_session = self.authenticate(access_token)?;
        let payment = {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            let payment = load_report_purchase_session(&conn, payment_session_id)?;
            if payment.account_id != auth_session.account_id {
                return Err(StorageError::Forbidden);
            }
            payment
        };
        if payment.provider != FIXTURE_PROVIDER {
            return Err(StorageError::BadRequest(
                "report_purchase_reconcile_provider_not_available".to_string(),
            ));
        }
        if payment.status == "succeeded" {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            let grant = load_report_purchase_grant_for_session(&conn, &payment.payment_session_id)?;
            return Ok(ReportPurchaseSessionReconcileResponse {
                payment_session_id: payment.payment_session_id,
                status: "succeeded".to_string(),
                message: "支付已确认，报告授权已生效。".to_string(),
                grant,
            });
        }
        let grant = self.grant_report_purchase_from_payment(&payment)?;
        Ok(ReportPurchaseSessionReconcileResponse {
            payment_session_id: payment.payment_session_id,
            status: "succeeded".to_string(),
            message: "支付已确认，报告授权已生效。".to_string(),
            grant: Some(grant),
        })
    }

    pub fn billing_payment_session_status(
        &self,
        access_token: &str,
        payment_session_id: &str,
    ) -> Result<BillingPaymentSessionStatusResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let payment = load_billing_payment_session(&conn, payment_session_id)?;
        if payment.account_id != session.account_id {
            return Err(StorageError::Forbidden);
        }
        let entitlement = cloud_entitlement_for_account(&conn, &session.account_id)?;
        Ok(BillingPaymentSessionStatusResponse {
            payment_session_id: payment.payment_session_id,
            provider: payment.provider,
            provider_order_id: payment.provider_order_id,
            status: payment.status,
            plan_code: payment.plan_code,
            billing_cycle: payment.billing_cycle,
            expires_at: parse_rfc3339_utc(&payment.expires_at)?,
            last_checked_at: parse_optional_rfc3339_utc(payment.last_checked_at.as_deref())?,
            next_check_after: parse_optional_rfc3339_utc(payment.next_check_after.as_deref())?,
            check_attempts: payment.check_attempts.max(0) as u32,
            entitlement,
        })
    }

    pub fn reconcile_billing_payment_session(
        &self,
        access_token: &str,
        payment_session_id: &str,
    ) -> Result<BillingPaymentSessionReconcileResponse, StorageError> {
        let auth_session = self.authenticate(access_token)?;
        let payment = {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            let payment = load_billing_payment_session(&conn, payment_session_id)?;
            if payment.account_id != auth_session.account_id {
                return Err(StorageError::Forbidden);
            }
            payment
        };
        if payment.provider != FIXTURE_PROVIDER {
            return Err(StorageError::BadRequest(
                "billing_reconcile_provider_not_available".to_string(),
            ));
        }
        if payment.status == "succeeded" {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            let entitlement = cloud_entitlement_for_account(&conn, &auth_session.account_id)?;
            return Ok(BillingPaymentSessionReconcileResponse {
                payment_session_id: payment.payment_session_id,
                status: "succeeded".to_string(),
                message: "支付已确认，权益已生效。".to_string(),
                entitlement,
            });
        }
        self.reconcile_billing_payment_record(payment, &auth_session.account_id)
    }

    pub fn reconcile_pending_payment_sessions(
        &self,
        limit: usize,
    ) -> Result<BillingPaymentReconcileSweep, StorageError> {
        let due_sessions = {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            load_due_billing_payment_sessions(&conn, Utc::now(), limit)?
        };
        let mut sweep = BillingPaymentReconcileSweep::default();
        for payment in due_sessions {
            if payment.provider != FIXTURE_PROVIDER {
                let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
                defer_billing_payment_session_check(&conn, &payment)?;
                sweep.skipped_unsupported_provider += 1;
                continue;
            }
            match self.reconcile_billing_payment_record(payment, "") {
                Ok(result) => {
                    sweep.checked += 1;
                    match result.status.as_str() {
                        "succeeded" => sweep.succeeded += 1,
                        "failed" | "closed" | "expired" => sweep.failed += 1,
                        _ => sweep.pending += 1,
                    }
                }
                Err(error) => {
                    tracing::warn!(error = %error, "billing payment session background reconcile failed");
                    sweep.failed += 1;
                }
            }
        }
        Ok(sweep)
    }

    pub fn due_payment_sessions_for_provider(
        &self,
        provider: &str,
        limit: usize,
    ) -> Result<Vec<BillingPaymentSessionOrderQuery>, StorageError> {
        let due_sessions = {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            load_due_billing_payment_sessions(&conn, Utc::now(), limit)?
        };
        Ok(due_sessions
            .into_iter()
            .filter(|payment| payment.provider == provider)
            .map(|payment| BillingPaymentSessionOrderQuery {
                payment_session_id: payment.payment_session_id,
                provider: payment.provider,
                provider_order_id: payment.provider_order_id,
            })
            .collect())
    }

    pub fn reconcile_billing_order_status(
        &self,
        payment_session_id: &str,
        order_status: BillingOrderStatus,
    ) -> Result<BillingPaymentSessionReconcileResponse, StorageError> {
        let payment = {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            load_billing_payment_session(&conn, payment_session_id)?
        };
        if payment.provider != order_status.provider
            || payment.provider_order_id != order_status.provider_order_id
            || payment.account_id != order_status.account_id
            || payment.workspace_id != order_status.workspace_id
        {
            return Err(StorageError::Forbidden);
        }
        if payment.plan_code != order_status.plan_code
            || payment.billing_cycle != order_status.billing_cycle
            || plan_amount_cents(&payment.plan_code, &payment.billing_cycle)
                != order_status.amount_cents
            || payment.status == "succeeded"
        {
            return Err(StorageError::BadRequest(
                "billing_order_status_mismatch".to_string(),
            ));
        }
        let event = billing_event_for_order_status(&order_status);
        let Some(event) = event else {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            mark_billing_payment_session_checked(
                &conn,
                &payment.payment_session_id,
                order_status.status.as_str(),
                order_status.provider_transaction_id.as_deref(),
                None,
            )?;
            let entitlement = cloud_entitlement_for_account(&conn, &payment.account_id)?;
            return Ok(BillingPaymentSessionReconcileResponse {
                payment_session_id: payment.payment_session_id,
                status: order_status.status.as_str().to_string(),
                message: reconcile_message(order_status.status.as_str()).to_string(),
                entitlement,
            });
        };
        let applied = self.apply_billing_event(event.clone())?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        mark_billing_payment_session_checked(
            &conn,
            &payment.payment_session_id,
            "succeeded",
            event.provider_transaction_id.as_deref(),
            Some(&applied.provider_event_id),
        )?;
        Ok(BillingPaymentSessionReconcileResponse {
            payment_session_id: payment.payment_session_id,
            status: "succeeded".to_string(),
            message: "支付已确认，权益已生效。".to_string(),
            entitlement: applied.entitlement,
        })
    }

    pub fn due_report_purchase_sessions_for_provider(
        &self,
        provider: &str,
        limit: usize,
    ) -> Result<Vec<ReportPurchaseSessionOrderQuery>, StorageError> {
        let due_sessions = {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            load_due_report_purchase_sessions(&conn, Utc::now(), limit)?
        };
        Ok(due_sessions
            .into_iter()
            .filter(|payment| payment.provider == provider)
            .map(|payment| ReportPurchaseSessionOrderQuery {
                payment_session_id: payment.payment_session_id,
                provider: payment.provider,
                provider_order_id: payment.provider_order_id,
            })
            .collect())
    }

    pub fn reconcile_report_purchase_order_status(
        &self,
        payment_session_id: &str,
        order_status: ReportPurchaseOrderStatus,
    ) -> Result<ReportPurchaseSessionReconcileResponse, StorageError> {
        let payment = {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            load_report_purchase_session(&conn, payment_session_id)?
        };
        validate_report_purchase_order_status_matches(&payment, &order_status)?;
        match order_status.status {
            BillingOrderStatusKind::Succeeded | BillingOrderStatusKind::Refunded => {
                let event =
                    report_purchase_event_for_order_status(&order_status).ok_or_else(|| {
                        StorageError::BadRequest(
                            "report_purchase_order_status_mismatch".to_string(),
                        )
                    })?;
                let applied = self.apply_report_purchase_event(event)?;
                let message = if applied.status == "revoked" {
                    "支付已退款，报告授权已撤销。"
                } else {
                    "支付已确认，报告授权已生效。"
                };
                Ok(ReportPurchaseSessionReconcileResponse {
                    payment_session_id: payment.payment_session_id,
                    status: applied.status,
                    message: message.to_string(),
                    grant: applied.grant,
                })
            }
            _ => {
                let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
                mark_report_purchase_session_checked(
                    &conn,
                    &payment.payment_session_id,
                    order_status.status.as_str(),
                    order_status.provider_transaction_id.as_deref(),
                    None,
                )?;
                Ok(ReportPurchaseSessionReconcileResponse {
                    payment_session_id: payment.payment_session_id,
                    status: order_status.status.as_str().to_string(),
                    message: reconcile_message(order_status.status.as_str()).to_string(),
                    grant: None,
                })
            }
        }
    }

    fn reconcile_billing_payment_record(
        &self,
        payment: BillingPaymentSessionRecord,
        entitlement_account_id: &str,
    ) -> Result<BillingPaymentSessionReconcileResponse, StorageError> {
        if payment.status == "succeeded" {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            let entitlement = cloud_entitlement_for_account(&conn, &payment.account_id)?;
            return Ok(BillingPaymentSessionReconcileResponse {
                payment_session_id: payment.payment_session_id,
                status: "succeeded".to_string(),
                message: "支付已确认，权益已生效。".to_string(),
                entitlement,
            });
        }
        let input = BillingPaymentSessionInput {
            account_id: payment.account_id.clone(),
            workspace_id: payment.workspace_id.clone(),
            plan_code: payment.plan_code.clone(),
            billing_cycle: payment.billing_cycle.clone(),
        };
        let fixture_session = BillingPaymentSession {
            payment_session_id: payment.payment_session_id.clone(),
            provider: payment.provider.clone(),
            provider_order_id: payment.provider_order_id.clone(),
            action: serde_json::from_str(&payment.payment_action_json).unwrap_or_else(|_| {
                crate::billing::BillingPaymentAction {
                    action_type: "qr_code".to_string(),
                    qr_code_url: None,
                    h5_url: None,
                }
            }),
            expires_at: parse_rfc3339_utc(&payment.expires_at)?,
        };
        let provider = FixtureBillingProvider;
        let order_status = provider.query_order(&fixture_session, &input);
        let mut response =
            self.reconcile_billing_order_status(&payment.payment_session_id, order_status)?;
        if !entitlement_account_id.is_empty() {
            let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
            response.entitlement = cloud_entitlement_for_account(&conn, entitlement_account_id)?;
        }
        Ok(response)
    }

    fn grant_report_purchase_from_payment(
        &self,
        payment: &ReportPurchaseSessionRecord,
    ) -> Result<ReportPurchaseGrant, StorageError> {
        if payment.price_cents != report_product_price_cents(&payment.product_code)? {
            return Err(StorageError::BadRequest(
                "report_purchase_amount_mismatch".to_string(),
            ));
        }
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !account_workspace_matches_with_conn(&tx, &payment.account_id, &payment.workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(
            &tx,
            &payment.account_id,
            &payment.creator_profile_id,
        )? {
            return Err(StorageError::Forbidden);
        }
        let provider_event_id = format!(
            "report_purchase_{}_{}",
            payment.provider,
            short_id(&format!(
                "{}:{}:{}",
                payment.provider_order_id, payment.vault_record_id, payment.product_code
            ))
        );
        let provider_transaction_id = format!(
            "fixture_report_txn_{}",
            short_id(&payment.provider_order_id)
        );
        let grant = upsert_report_purchase_grant_tx(
            &tx,
            payment,
            &provider_transaction_id,
            &provider_event_id,
        )?;
        mark_report_purchase_session_checked_tx(
            &tx,
            &payment.provider,
            &payment.provider_order_id,
            "succeeded",
            Some(&provider_transaction_id),
            Some(&provider_event_id),
        )?;
        tx.commit()?;
        Ok(grant)
    }

    pub fn apply_fixture_billing_event(
        &self,
        request: &BillingFixtureEventRequest,
    ) -> Result<BillingEventApplyResponse, StorageError> {
        let event = billing_event_from_fixture_request(request)?;
        self.apply_billing_event(event)
    }

    pub fn apply_billing_event(
        &self,
        event: BillingEvent,
    ) -> Result<BillingEventApplyResponse, StorageError> {
        if !matches!(
            event.provider.as_str(),
            FIXTURE_PROVIDER | "wechat_pay" | "stripe"
        ) {
            return Err(StorageError::BadRequest(
                "billing_provider_not_available".to_string(),
            ));
        }
        let plan_code = normalize_plan_code(&event.plan_code)?;
        let billing_cycle = normalize_billing_cycle(&event.billing_cycle)?;
        if event.amount_cents < 0 || event.currency.trim().is_empty() {
            return Err(StorageError::BadRequest(
                "billing_amount_invalid".to_string(),
            ));
        }
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !account_workspace_matches_with_conn(&tx, &event.account_id, &event.workspace_id)? {
            return Err(StorageError::Forbidden);
        }

        let duplicate = !insert_subscription_event(&tx, &event)?;
        let entitlement = if duplicate {
            cloud_entitlement_for_account(&tx, &event.account_id)?
        } else {
            apply_billing_state_transition(&tx, &event, &plan_code, &billing_cycle)?
        };
        if !duplicate
            && matches!(
                event.event_type,
                BillingEventType::PaymentSucceeded | BillingEventType::SubscriptionRenewed
            )
        {
            mark_billing_payment_session_checked_tx(
                &tx,
                &event.provider,
                &event.provider_order_id,
                "succeeded",
                event.provider_transaction_id.as_deref(),
                Some(&event.provider_event_id),
            )?;
        }
        tx.commit()?;
        Ok(BillingEventApplyResponse {
            provider: event.provider,
            provider_event_id: event.provider_event_id,
            duplicate,
            entitlement,
        })
    }

    pub fn apply_report_purchase_event(
        &self,
        event: ReportPurchaseEvent,
    ) -> Result<ReportPurchaseEventApplyResponse, StorageError> {
        if !matches!(
            event.provider.as_str(),
            FIXTURE_PROVIDER | "wechat_pay" | "stripe"
        ) {
            return Err(StorageError::BadRequest(
                "billing_provider_not_available".to_string(),
            ));
        }
        let product_code = normalize_report_product_code(&event.product_code)?;
        if report_product_price_cents(&product_code)? != event.price_cents
            || event.currency.trim() != "CNY"
        {
            return Err(StorageError::BadRequest(
                "report_purchase_amount_mismatch".to_string(),
            ));
        }
        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if !account_workspace_matches_with_conn(&tx, &event.account_id, &event.workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(&tx, &event.account_id, &event.creator_profile_id)? {
            return Err(StorageError::Forbidden);
        }
        let payment = load_report_purchase_session_by_provider_tx(
            &tx,
            &event.provider,
            &event.provider_order_id,
        )?;
        if payment.account_id != event.account_id
            || payment.workspace_id != event.workspace_id
            || payment.creator_profile_id != event.creator_profile_id
            || payment.vault_record_id != event.vault_record_id
            || payment.product_code != product_code
            || payment.price_cents != event.price_cents
        {
            return Err(StorageError::BadRequest(
                "report_purchase_event_mismatch".to_string(),
            ));
        }
        let duplicate = report_purchase_provider_event_exists_tx(&tx, &event)?;
        let status = match event.event_type {
            ReportPurchaseEventType::PaymentSucceeded => "succeeded",
            ReportPurchaseEventType::RefundSucceeded => "revoked",
        };
        let grant = if duplicate {
            load_report_purchase_grant_for_session(&tx, &payment.payment_session_id)?
        } else {
            match event.event_type {
                ReportPurchaseEventType::PaymentSucceeded => Some(upsert_report_purchase_grant_tx(
                    &tx,
                    &payment,
                    event.provider_transaction_id.as_deref().unwrap_or_default(),
                    &event.provider_event_id,
                )?),
                ReportPurchaseEventType::RefundSucceeded => {
                    revoke_report_purchase_grant_tx(&tx, &payment, &event)?;
                    None
                }
            }
        };
        mark_report_purchase_session_checked_tx(
            &tx,
            &event.provider,
            &event.provider_order_id,
            status,
            event.provider_transaction_id.as_deref(),
            Some(&event.provider_event_id),
        )?;
        tx.commit()?;
        Ok(ReportPurchaseEventApplyResponse {
            provider: event.provider,
            provider_event_id: event.provider_event_id,
            duplicate,
            status: status.to_string(),
            grant,
        })
    }

    pub fn current_entitlement(
        &self,
        access_token: &str,
    ) -> Result<CloudEntitlement, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        cloud_entitlement_for_account(&conn, &session.account_id)
    }

    pub fn current_team_workspace(
        &self,
        access_token: &str,
    ) -> Result<TeamWorkspaceSummary, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        current_team_workspace_for_account(&conn, &session.account_id)
    }

    #[cfg(test)]
    pub(crate) fn set_entitlement_feature_for_tests(
        &self,
        account_id: &str,
        feature_name: &str,
        enabled: bool,
    ) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let features_json: String = conn.query_row(
            "SELECT entitlement_features_json FROM cloud_accounts WHERE id = ?1",
            params![account_id],
            |row| row.get(0),
        )?;
        let mut features: serde_json::Value =
            serde_json::from_str(&features_json).unwrap_or_else(|_| default_entitlement_features());
        features[feature_name] = serde_json::json!(enabled);
        conn.execute(
            "UPDATE cloud_accounts SET entitlement_features_json = ?1 WHERE id = ?2",
            params![features.to_string(), account_id],
        )?;
        Ok(())
    }

    pub fn list_team_workspaces(
        &self,
        access_token: &str,
    ) -> Result<TeamWorkspaceListResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !team_workspace_enabled_for_account(&conn, &session.account_id)? {
            return Err(StorageError::Forbidden);
        }
        let workspaces = list_team_workspaces_for_account(&conn, &session.account_id)?;
        let returned = workspaces.len() as u32;
        Ok(TeamWorkspaceListResponse {
            workspaces,
            returned,
        })
    }

    pub fn create_team_workspace(
        &self,
        access_token: &str,
        request: &TeamWorkspaceCreateRequest,
    ) -> Result<TeamWorkspaceSummary, StorageError> {
        let session = self.authenticate(access_token)?;
        if request.account_id.trim().is_empty() || request.name.trim().is_empty() {
            return Err(StorageError::BadRequest(
                "team workspace fields are required".to_string(),
            ));
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if session.account_id != request.account_id.trim() {
            return Err(StorageError::Forbidden);
        }
        if !team_workspace_enabled_for_account(&conn, &session.account_id)? {
            return Err(StorageError::Forbidden);
        }
        create_team_workspace_with_conn(&conn, &session.account_id, request)
    }

    pub fn list_team_members(
        &self,
        access_token: &str,
        workspace_id: &str,
    ) -> Result<TeamMemberListResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        ensure_team_workspace_access(&conn, &session.account_id, workspace_id)?;
        let members = list_team_members_for_workspace(&conn, workspace_id)?;
        let returned = members.len() as u32;
        Ok(TeamMemberListResponse { members, returned })
    }

    pub fn create_team_member(
        &self,
        access_token: &str,
        workspace_id: &str,
        request: &TeamMemberCreateRequest,
    ) -> Result<TeamMemberRecord, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        ensure_team_workspace_access(&conn, &session.account_id, workspace_id)?;
        if request.account_id.trim().is_empty() {
            return Err(StorageError::BadRequest(
                "accountId is required".to_string(),
            ));
        }
        create_team_member_with_conn(&conn, workspace_id, request)
    }

    pub fn update_team_member(
        &self,
        access_token: &str,
        member_id: &str,
        request: &TeamMemberUpdateRequest,
    ) -> Result<TeamMemberRecord, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        update_team_member_with_conn(&conn, &session.account_id, member_id, request)
    }

    pub fn list_team_shared_library_records(
        &self,
        access_token: &str,
        workspace_id: &str,
    ) -> Result<TeamSharedLibraryListResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        ensure_team_workspace_access(&conn, &session.account_id, workspace_id)?;
        let records = list_team_shared_library_records_for_workspace(&conn, workspace_id)?;
        let returned = records.len() as u32;
        Ok(TeamSharedLibraryListResponse { records, returned })
    }

    pub fn share_team_library_record(
        &self,
        access_token: &str,
        workspace_id: &str,
        request: &TeamSharedLibraryShareRequest,
    ) -> Result<TeamSharedLibraryRecord, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        ensure_team_workspace_access(&conn, &session.account_id, workspace_id)?;
        let record = share_team_library_record_with_conn(&conn, workspace_id, request)?;
        record_team_audit_event_with_conn(
            &conn,
            workspace_id,
            &session.account_id,
            None,
            "share_record",
            "shared_library",
            &record.shared_record_id,
            None,
            Some(serde_json::to_value(&record).unwrap_or_else(|_| serde_json::json!({}))),
            request.reason.trim(),
        )?;
        Ok(record)
    }

    pub fn list_team_audit_logs(
        &self,
        access_token: &str,
        workspace_id: &str,
    ) -> Result<TeamAuditListResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        ensure_team_workspace_access(&conn, &session.account_id, workspace_id)?;
        let events = list_team_audit_logs_for_workspace(&conn, workspace_id)?;
        let returned = events.len() as u32;
        Ok(TeamAuditListResponse { events, returned })
    }

    pub fn reserve_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReserveRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let workspace_id = require_non_empty(&request.workspace_id, "workspaceId")?;
        let creator_profile_id =
            require_non_empty(&request.creator_profile_id, "creatorProfileId")?;
        let request_id = require_non_empty(&request.request_id, "requestId")?;
        validate_payload_protocol(
            request.payload_protocol_version,
            request.payload_bytes_length,
        )?;
        validate_revision(request.parent_watermark_uid.as_deref(), request.revision)?;
        let media_type = normalize_media_type(&request.media_type)?;
        let parent_watermark_uid =
            normalize_optional_string(request.parent_watermark_uid.as_deref());
        let original_hash = normalize_optional_string(request.original_hash.as_deref());

        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !session_workspace_matches_with_conn(&conn, &session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(&conn, &session.account_id, creator_profile_id)? {
            return Err(StorageError::BadRequest(
                "creator_profile_required".to_string(),
            ));
        }

        if let Some(existing) =
            load_watermark_registry_by_request(&conn, &session.account_id, request_id)?
        {
            return registry_response_from_row(existing);
        }

        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = Utc::now().to_rfc3339();
        let watermark_uid = generate_watermark_uid();
        let registry_id = format!(
            "wmreg_{}",
            short_id(&format!("{request_id}{watermark_uid}"))
        );
        let registry_receipt = build_registry_receipt(&registry_id, &watermark_uid, "reserved");
        let registry_proof_hash = registry_proof_hash(&registry_receipt);
        tx.execute(
            "INSERT INTO watermark_id_registry (
                registry_id, request_id, account_id, workspace_id, creator_profile_id, device_id,
                watermark_uid, watermark_id_issue_mode, registry_status, registry_receipt,
                registry_proof_hash, media_type, payload_protocol_version, payload_bytes_length,
                parent_watermark_uid, revision, original_hash, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'server_reserved', 'reserved', ?8, ?9, ?10,
                ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            params![
                registry_id,
                request_id,
                session.account_id,
                workspace_id,
                creator_profile_id,
                session.device_id,
                watermark_uid,
                registry_receipt,
                registry_proof_hash,
                media_type,
                request.payload_protocol_version as i64,
                request.payload_bytes_length as i64,
                parent_watermark_uid,
                request.revision as i64,
                original_hash,
                now,
                now,
            ],
        )?;
        let row = load_watermark_registry_by_uid_tx(&tx, &watermark_uid)?
            .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
        tx.commit()?;
        registry_response_from_row(row)
    }

    pub fn confirm_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdConfirmRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let workspace_id = require_non_empty(&request.workspace_id, "workspaceId")?;
        let creator_profile_id =
            require_non_empty(&request.creator_profile_id, "creatorProfileId")?;
        let watermark_uid = normalize_watermark_uid(&request.watermark_uid)?;
        validate_payload_protocol(
            request.payload_protocol_version,
            request.payload_bytes_length,
        )?;
        let write_status = require_non_empty(
            &request.write_verification_status,
            "writeVerificationStatus",
        )?;
        let original_hash = normalize_optional_string(request.original_hash.as_deref());
        let protected_copy_hash = normalize_optional_string(request.protected_copy_hash.as_deref());

        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !session_workspace_matches_with_conn(&conn, &session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(&conn, &session.account_id, creator_profile_id)? {
            return Err(StorageError::BadRequest(
                "creator_profile_required".to_string(),
            ));
        }
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let Some(existing) = load_watermark_registry_by_uid_tx(&tx, &watermark_uid)? else {
            return Err(StorageError::BadRequest(
                "watermark_registry_missing".to_string(),
            ));
        };
        if existing.registry_status == "conflict" || existing.registry_status == "reissue_required"
        {
            return Err(StorageError::BadRequest(
                "watermark_registry_conflict".to_string(),
            ));
        }
        let now = Utc::now().to_rfc3339();
        let registry_receipt =
            build_registry_receipt(&existing.registry_id, &watermark_uid, "server_confirmed");
        let registry_proof_hash = registry_proof_hash(&registry_receipt);
        tx.execute(
            "UPDATE watermark_id_registry
             SET registry_status = 'server_confirmed',
                 watermark_id_issue_mode = 'server_confirmed',
                 registry_receipt = ?2,
                 registry_proof_hash = ?3,
                 payload_protocol_version = ?4,
                 payload_bytes_length = ?5,
                 original_hash = COALESCE(?6, original_hash),
                 protected_copy_hash = COALESCE(?7, protected_copy_hash),
                 write_verification_status = ?8,
                 confirmed_at = ?9,
                 updated_at = ?9
             WHERE watermark_uid = ?1 AND account_id = ?10 AND workspace_id = ?11",
            params![
                watermark_uid,
                registry_receipt,
                registry_proof_hash,
                request.payload_protocol_version as i64,
                request.payload_bytes_length as i64,
                original_hash,
                protected_copy_hash,
                write_status,
                now,
                session.account_id,
                workspace_id,
            ],
        )?;
        let row = load_watermark_registry_by_uid_tx(&tx, &watermark_uid)?
            .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
        tx.commit()?;
        registry_response_from_row(row)
    }

    pub fn reconcile_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReconcileRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let workspace_id = require_non_empty(&request.workspace_id, "workspaceId")?;
        let creator_profile_id =
            require_non_empty(&request.creator_profile_id, "creatorProfileId")?;
        let watermark_uid = normalize_watermark_uid(&request.watermark_uid)?;
        validate_payload_protocol(
            request.payload_protocol_version,
            request.payload_bytes_length,
        )?;
        validate_revision(request.parent_watermark_uid.as_deref(), request.revision)?;
        let media_type = normalize_media_type(&request.media_type)?;
        let parent_watermark_uid =
            normalize_optional_string(request.parent_watermark_uid.as_deref());
        let original_hash = normalize_optional_string(request.original_hash.as_deref());
        let protected_copy_hash = normalize_optional_string(request.protected_copy_hash.as_deref());
        let write_status = normalize_optional_string(request.write_verification_status.as_deref());

        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !session_workspace_matches_with_conn(&conn, &session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(&conn, &session.account_id, creator_profile_id)? {
            return Err(StorageError::BadRequest(
                "creator_profile_required".to_string(),
            ));
        }
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) = load_watermark_registry_by_uid_tx(&tx, &watermark_uid)? {
            let existing_original_hash = load_watermark_original_hash_tx(&tx, &watermark_uid)?;
            let same_owner = watermark_registry_owner_matches_tx(
                &tx,
                &watermark_uid,
                &session.account_id,
                workspace_id,
            )?;
            let same_original_hash = original_hash.is_none()
                || existing_original_hash.is_none()
                || existing_original_hash == original_hash;
            if same_owner && same_original_hash {
                let now = Utc::now().to_rfc3339();
                let status = if existing.registry_status == "server_confirmed" {
                    "server_confirmed"
                } else {
                    "offline_confirmed"
                };
                let issue_mode = if existing.watermark_id_issue_mode == "server_confirmed" {
                    "server_confirmed"
                } else {
                    "offline_generated"
                };
                let registry_receipt =
                    build_registry_receipt(&existing.registry_id, &watermark_uid, status);
                let registry_proof_hash = registry_proof_hash(&registry_receipt);
                tx.execute(
                    "UPDATE watermark_id_registry
                     SET registry_status = ?2,
                         watermark_id_issue_mode = ?3,
                         registry_receipt = ?4,
                         registry_proof_hash = ?5,
                         original_hash = COALESCE(?6, original_hash),
                         protected_copy_hash = COALESCE(?7, protected_copy_hash),
                         write_verification_status = COALESCE(?8, write_verification_status),
                         confirmed_at = COALESCE(confirmed_at, ?9),
                         updated_at = ?9
                     WHERE watermark_uid = ?1",
                    params![
                        watermark_uid,
                        status,
                        issue_mode,
                        registry_receipt,
                        registry_proof_hash,
                        original_hash,
                        protected_copy_hash,
                        write_status,
                        now,
                    ],
                )?;
            } else {
                let now = Utc::now().to_rfc3339();
                let registry_receipt =
                    build_registry_receipt(&existing.registry_id, &watermark_uid, "conflict");
                let registry_proof_hash = registry_proof_hash(&registry_receipt);
                tx.execute(
                    "UPDATE watermark_id_registry
                     SET registry_status = 'conflict',
                         registry_receipt = ?2,
                         registry_proof_hash = ?3,
                         updated_at = ?4
                     WHERE watermark_uid = ?1",
                    params![watermark_uid, registry_receipt, registry_proof_hash, now],
                )?;
            }
            let row = load_watermark_registry_by_uid_tx(&tx, &watermark_uid)?.ok_or_else(|| {
                StorageError::BadRequest("watermark_registry_missing".to_string())
            })?;
            tx.commit()?;
            return registry_response_from_row(row);
        }

        let now = Utc::now().to_rfc3339();
        let registry_id = format!(
            "wmreg_{}",
            short_id(&format!("{}{}{}", session.account_id, watermark_uid, now))
        );
        let registry_receipt =
            build_registry_receipt(&registry_id, &watermark_uid, "offline_confirmed");
        let registry_proof_hash = registry_proof_hash(&registry_receipt);
        tx.execute(
            "INSERT INTO watermark_id_registry (
                registry_id, request_id, account_id, workspace_id, creator_profile_id, device_id,
                watermark_uid, watermark_id_issue_mode, registry_status, registry_receipt,
                registry_proof_hash, media_type, payload_protocol_version, payload_bytes_length,
                parent_watermark_uid, revision, original_hash, protected_copy_hash,
                write_verification_status, confirmed_at, created_at, updated_at
            ) VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, 'offline_generated', 'offline_confirmed',
                ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?17, ?17)",
            params![
                registry_id,
                session.account_id,
                workspace_id,
                creator_profile_id,
                session.device_id,
                watermark_uid,
                registry_receipt,
                registry_proof_hash,
                media_type,
                request.payload_protocol_version as i64,
                request.payload_bytes_length as i64,
                parent_watermark_uid,
                request.revision as i64,
                original_hash,
                protected_copy_hash,
                write_status,
                now,
            ],
        )?;
        let row = load_watermark_registry_by_uid_tx(&tx, &watermark_uid)?
            .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
        tx.commit()?;
        registry_response_from_row(row)
    }

    pub fn reissue_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReissueRequest,
    ) -> Result<WatermarkIdReissueResponse, StorageError> {
        let session = self.authenticate(access_token)?;
        let workspace_id = require_non_empty(&request.workspace_id, "workspaceId")?;
        let creator_profile_id =
            require_non_empty(&request.creator_profile_id, "creatorProfileId")?;
        let previous_watermark_uid = normalize_watermark_uid(&request.previous_watermark_uid)?;
        validate_payload_protocol(
            request.payload_protocol_version,
            request.payload_bytes_length,
        )?;
        validate_revision(request.parent_watermark_uid.as_deref(), request.revision)?;
        let media_type = normalize_media_type(&request.media_type)?;
        let reason = require_non_empty(&request.reason, "reason")?;
        let parent_watermark_uid =
            normalize_optional_string(request.parent_watermark_uid.as_deref());
        let original_hash = normalize_optional_string(request.original_hash.as_deref());

        let mut conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        if !session_workspace_matches_with_conn(&conn, &session.account_id, workspace_id)? {
            return Err(StorageError::Forbidden);
        }
        if !creator_profile_matches_with_conn(&conn, &session.account_id, creator_profile_id)? {
            return Err(StorageError::BadRequest(
                "creator_profile_required".to_string(),
            ));
        }

        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let now = Utc::now().to_rfc3339();
        let watermark_uid = generate_watermark_uid();
        let registry_id = format!(
            "wmreg_{}",
            short_id(&format!("{}{}{}", session.account_id, watermark_uid, now))
        );
        let registry_receipt =
            build_registry_receipt(&registry_id, &watermark_uid, "server_reissued");
        let registry_proof_hash = registry_proof_hash(&registry_receipt);
        tx.execute(
            "INSERT INTO watermark_id_registry (
                registry_id, request_id, account_id, workspace_id, creator_profile_id, device_id,
                watermark_uid, watermark_id_issue_mode, registry_status, registry_receipt,
                registry_proof_hash, media_type, payload_protocol_version, payload_bytes_length,
                parent_watermark_uid, revision, original_hash, created_at, updated_at
            ) VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6, 'server_reissued', 'reserved', ?7, ?8, ?9,
                ?10, ?11, ?12, ?13, ?14, ?15, ?15)",
            params![
                registry_id,
                session.account_id,
                workspace_id,
                creator_profile_id,
                session.device_id,
                watermark_uid,
                registry_receipt,
                registry_proof_hash,
                media_type,
                request.payload_protocol_version as i64,
                request.payload_bytes_length as i64,
                parent_watermark_uid,
                request.revision as i64,
                original_hash,
                now,
            ],
        )?;
        let job_id = format!(
            "wmreissue_{}",
            short_id(&format!(
                "{}{}{}",
                previous_watermark_uid, watermark_uid, now
            ))
        );
        tx.execute(
            "INSERT INTO watermark_id_reissue_jobs (
                job_id, account_id, workspace_id, creator_profile_id, previous_watermark_uid,
                replacement_watermark_uid, reason, status, created_at, updated_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 'created', ?8, ?8)",
            params![
                job_id,
                session.account_id,
                workspace_id,
                creator_profile_id,
                previous_watermark_uid,
                watermark_uid,
                reason,
                now,
            ],
        )?;
        let row = load_watermark_registry_by_uid_tx(&tx, &watermark_uid)?
            .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
        tx.commit()?;
        Ok(WatermarkIdReissueResponse {
            job_id,
            previous_watermark_uid,
            replacement: registry_response_from_row(row)?,
        })
    }

    fn authenticate(&self, access_token: &str) -> Result<SessionRecord, StorageError> {
        let access_token = access_token.trim();
        if access_token.is_empty() {
            return Err(StorageError::Unauthorized);
        }
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let mut stmt = conn.prepare(
            "SELECT account_id, device_id, revoked_at
             FROM cloud_sessions
             WHERE access_token = ?1",
        )?;
        let session = stmt.query_row(params![access_token], |row| {
            Ok(SessionRecord {
                account_id: row.get(0)?,
                device_id: row.get(1)?,
                revoked_at: row.get(2)?,
            })
        });
        match session {
            Ok(session) if session.revoked_at.is_none() => Ok(session),
            _ => Err(StorageError::Unauthorized),
        }
    }

    fn account_snapshot_for_session_parts(
        &self,
        account_id: &str,
        device_id: &str,
    ) -> Result<CloudAccountSnapshot, StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        let account = load_account_by_id_conn(&conn, account_id)?;
        let device = load_device_by_id_conn(&conn, account_id, device_id)?;
        account_snapshot_from_rows(&conn, account, device)
    }

    fn session_workspace_matches(
        &self,
        account_id: &str,
        workspace_id: &str,
    ) -> Result<bool, StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        session_workspace_matches_with_conn(&conn, account_id, workspace_id)
    }

    fn ensure_cloud_sync_entitled(&self, account_id: &str) -> Result<(), StorageError> {
        let conn = self.conn.lock().unwrap_or_else(|e| e.into_inner());
        ensure_cloud_sync_entitled_with_conn(&conn, account_id)
    }
}

#[derive(Debug, Clone)]
struct CloudAccountRow {
    id: String,
    display_name: String,
    workspace_id: String,
    workspace_name: String,
    creator_profile_id: String,
    creator_display_name: String,
    entitlement_id: String,
    entitlement_plan_name: String,
    entitlement_plan_code: String,
    entitlement_status: String,
    entitlement_features_json: String,
}

#[derive(Debug, Clone)]
struct CloudDeviceRow {
    id: String,
    name: String,
    platform: String,
    auto_sync_enabled: bool,
}

#[derive(Debug, Clone)]
struct BillingPaymentSessionRecord {
    payment_session_id: String,
    provider: String,
    provider_order_id: String,
    account_id: String,
    workspace_id: String,
    plan_code: String,
    billing_cycle: String,
    status: String,
    payment_action_json: String,
    expires_at: String,
    last_checked_at: Option<String>,
    next_check_after: Option<String>,
    check_attempts: i64,
}

#[derive(Debug, Clone)]
struct ReportPurchaseSessionRecord {
    payment_session_id: String,
    provider: String,
    provider_order_id: String,
    account_id: String,
    workspace_id: String,
    creator_profile_id: String,
    vault_record_id: String,
    product_code: String,
    price_cents: i64,
    currency: String,
    status: String,
    expires_at: String,
    last_checked_at: Option<String>,
    next_check_after: Option<String>,
    check_attempts: i64,
}

#[derive(Debug, Clone)]
struct SessionRecord {
    account_id: String,
    device_id: String,
    revoked_at: Option<String>,
}

#[derive(Debug, Clone)]
struct WatermarkIdRegistryRow {
    registry_id: String,
    watermark_uid: String,
    watermark_id_issue_mode: String,
    registry_status: String,
    registry_receipt: String,
    registry_proof_hash: String,
    payload_protocol_version: i64,
    payload_bytes_length: i64,
    parent_watermark_uid: Option<String>,
    revision: i64,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct RightsManifestRow {
    rights_manifest_id: String,
    watermark_uid: String,
    manifest_version: i64,
    status: String,
    training_policy: String,
    work_source_declaration: String,
    creation_method_declaration: String,
    human_edit_level_declaration: String,
    authenticity_claim_declaration: String,
    custom_terms_url: Option<String>,
    custom_terms_hash: Option<String>,
    standard_mappings_json: String,
    manifest_sha256: String,
    signature: String,
    signed_by: String,
    effective_at: String,
    superseded_by_rights_manifest_id: Option<String>,
    revoked_at: Option<String>,
    created_at: String,
    updated_at: String,
}

fn init_schema(conn: &Connection) -> Result<(), rusqlite::Error> {
    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS feedback_events (
            event_id TEXT PRIMARY KEY,
            occurred_at TEXT NOT NULL,
            install_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            app_version TEXT NOT NULL,
            feature_name TEXT NOT NULL,
            outcome TEXT NOT NULL,
            media_type TEXT NOT NULL,
            file_size_bucket TEXT NOT NULL,
            duration_ms INTEGER,
            error_code TEXT,
            diagnostic_note TEXT,
            stack_summary TEXT,
            pipeline_id TEXT,
            ingested_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS feedback_batches (
            request_id TEXT PRIMARY KEY,
            install_id TEXT NOT NULL,
            session_id TEXT NOT NULL,
            app_version TEXT NOT NULL,
            sent_at TEXT NOT NULL,
            received_at TEXT NOT NULL,
            received_events INTEGER NOT NULL,
            inserted_events INTEGER NOT NULL,
            duplicate_events INTEGER NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_feedback_events_occurred_at ON feedback_events(occurred_at);
        CREATE INDEX IF NOT EXISTS idx_feedback_events_app_version ON feedback_events(app_version);
        CREATE INDEX IF NOT EXISTS idx_feedback_events_feature_name ON feedback_events(feature_name);
        CREATE INDEX IF NOT EXISTS idx_feedback_events_error_code ON feedback_events(error_code);
        CREATE INDEX IF NOT EXISTS idx_feedback_events_media_type ON feedback_events(media_type);
        CREATE INDEX IF NOT EXISTS idx_feedback_events_outcome ON feedback_events(outcome);

        CREATE TABLE IF NOT EXISTS cloud_accounts (
            id TEXT PRIMARY KEY,
            identifier TEXT NOT NULL UNIQUE,
            password_hash TEXT,
            password_salt TEXT,
            password_hash_algorithm TEXT NOT NULL DEFAULT 'sha256',
            display_name TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            workspace_name TEXT NOT NULL,
            creator_profile_id TEXT NOT NULL,
            creator_display_name TEXT NOT NULL,
            creator_seed_ref TEXT NOT NULL,
            seed_envelope_version INTEGER NOT NULL,
            entitlement_id TEXT NOT NULL,
            entitlement_plan_name TEXT NOT NULL,
            entitlement_plan_code TEXT NOT NULL,
            entitlement_status TEXT NOT NULL,
            entitlement_features_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS cloud_devices (
            id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            client_device_id TEXT NOT NULL,
            name TEXT NOT NULL,
            platform TEXT NOT NULL,
            app_version TEXT NOT NULL,
            public_key TEXT,
            registered INTEGER NOT NULL,
            auto_sync_enabled INTEGER NOT NULL DEFAULT 1,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(account_id, client_device_id)
        );

        CREATE TABLE IF NOT EXISTS cloud_sessions (
            access_token TEXT PRIMARY KEY,
            refresh_token TEXT NOT NULL,
            account_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            revoked_at TEXT,
            expires_at TEXT,
            refresh_expires_at TEXT,
            last_used_at TEXT,
            token_family_id TEXT
        );

        CREATE TABLE IF NOT EXISTS auth_challenges (
            challenge_id TEXT PRIMARY KEY,
            identifier TEXT NOT NULL,
            purpose TEXT NOT NULL,
            client_device_id TEXT NOT NULL,
            code_hash TEXT NOT NULL,
            code_salt TEXT NOT NULL,
            delivery_channel TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            consumed_at TEXT,
            created_at TEXT NOT NULL,
            plain_code_for_delivery TEXT
        );

        CREATE TABLE IF NOT EXISTS auth_attempts (
            attempt_id TEXT PRIMARY KEY,
            identifier TEXT NOT NULL,
            client_device_id TEXT,
            attempt_type TEXT NOT NULL,
            outcome TEXT NOT NULL,
            reason TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS cloud_sync_events (
            sequence INTEGER PRIMARY KEY AUTOINCREMENT,
            account_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            client_event_id TEXT NOT NULL,
            operation TEXT NOT NULL,
            entity_type TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            payload_json TEXT NOT NULL,
            payload_hash TEXT,
            entity_revision INTEGER,
            created_at TEXT NOT NULL,
            UNIQUE(account_id, device_id, client_event_id)
        );

        CREATE TABLE IF NOT EXISTS cloud_device_cursors (
            account_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            cursor TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY(account_id, device_id)
        );

        CREATE INDEX IF NOT EXISTS idx_cloud_sync_events_account_sequence
        ON cloud_sync_events(account_id, sequence ASC);

        CREATE TABLE IF NOT EXISTS watermark_id_registry (
            registry_id TEXT PRIMARY KEY,
            request_id TEXT,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            creator_profile_id TEXT NOT NULL,
            device_id TEXT NOT NULL,
            watermark_uid TEXT NOT NULL UNIQUE,
            watermark_id_issue_mode TEXT NOT NULL,
            registry_status TEXT NOT NULL,
            registry_receipt TEXT NOT NULL,
            registry_proof_hash TEXT NOT NULL,
            media_type TEXT NOT NULL,
            payload_protocol_version INTEGER NOT NULL,
            payload_bytes_length INTEGER NOT NULL,
            parent_watermark_uid TEXT,
            revision INTEGER NOT NULL,
            original_hash TEXT,
            protected_copy_hash TEXT,
            write_verification_status TEXT,
            confirmed_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(account_id, request_id)
        );

        CREATE INDEX IF NOT EXISTS idx_watermark_id_registry_account_workspace
        ON watermark_id_registry(account_id, workspace_id, updated_at DESC);

        CREATE INDEX IF NOT EXISTS idx_watermark_id_registry_parent
        ON watermark_id_registry(parent_watermark_uid, revision);

        CREATE TABLE IF NOT EXISTS rights_manifests (
            id TEXT PRIMARY KEY,
            rights_manifest_id TEXT NOT NULL UNIQUE,
            watermark_uid TEXT NOT NULL,
            manifest_version INTEGER NOT NULL,
            status TEXT NOT NULL,
            training_policy TEXT NOT NULL,
            work_source_declaration TEXT NOT NULL,
            creation_method_declaration TEXT NOT NULL,
            human_edit_level_declaration TEXT NOT NULL,
            authenticity_claim_declaration TEXT NOT NULL,
            tdm_reservation TEXT NOT NULL DEFAULT 'not_declared',
            search_indexing_policy TEXT NOT NULL DEFAULT 'not_declared',
            embedding_policy TEXT NOT NULL DEFAULT 'not_declared',
            commercial_training_policy TEXT NOT NULL DEFAULT 'not_declared',
            custom_terms_url TEXT,
            custom_terms_hash TEXT,
            standard_mappings_json TEXT NOT NULL,
            manifest_sha256 TEXT NOT NULL,
            signed_by TEXT NOT NULL,
            signature TEXT NOT NULL,
            effective_at TEXT NOT NULL,
            superseded_by_rights_manifest_id TEXT,
            revoked_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(watermark_uid, manifest_version),
            CHECK(status IN ('active', 'superseded', 'revoked', 'disputed')),
            CHECK(status != 'active' OR (manifest_sha256 != '' AND signature != '')),
            CHECK(custom_terms_hash IS NULL OR custom_terms_url IS NOT NULL)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_rights_manifests_one_active
        ON rights_manifests(watermark_uid)
        WHERE status = 'active';

        CREATE INDEX IF NOT EXISTS idx_rights_manifests_watermark
        ON rights_manifests(watermark_uid);

        CREATE INDEX IF NOT EXISTS idx_rights_manifests_watermark_status
        ON rights_manifests(watermark_uid, status);

        CREATE INDEX IF NOT EXISTS idx_rights_manifests_watermark_version
        ON rights_manifests(watermark_uid, manifest_version DESC);

        CREATE INDEX IF NOT EXISTS idx_rights_manifests_status_updated
        ON rights_manifests(status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS ai_transparency_licenses (
            license_id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            environment TEXT NOT NULL,
            status TEXT NOT NULL,
            issuer_mode TEXT NOT NULL,
            deployment_mode TEXT NOT NULL,
            public_verification_required INTEGER NOT NULL,
            metering_plan_id TEXT NOT NULL,
            effective_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK(environment IN ('sandbox', 'production')),
            CHECK(status IN ('active', 'suspended', 'expired', 'revoked')),
            CHECK(issuer_mode IN ('hiddenshield_managed', 'platform_managed', 'customer_byok')),
            CHECK(deployment_mode IN ('hosted', 'private')),
            CHECK(expires_at > effective_at),
            CHECK(environment <> 'production' OR public_verification_required = 1)
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_transparency_licenses_one_active
        ON ai_transparency_licenses(tenant_id, workspace_id, environment)
        WHERE status = 'active';

        CREATE TABLE IF NOT EXISTS ai_profile_entitlements (
            license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
            profile_id TEXT NOT NULL,
            profile_kind TEXT NOT NULL,
            status TEXT NOT NULL,
            effective_at TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            terms_version TEXT NOT NULL,
            approved_by TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            PRIMARY KEY(license_id, profile_id),
            CHECK(profile_kind IN ('regulatory', 'technical')),
            CHECK(status IN ('active', 'suspended', 'expired', 'revoked')),
            CHECK(expires_at > effective_at)
        );

        CREATE INDEX IF NOT EXISTS idx_ai_profile_entitlements_license_status
        ON ai_profile_entitlements(license_id, status, expires_at);

        CREATE TABLE IF NOT EXISTS ai_sdk_credential_bindings (
            credential_id TEXT PRIMARY KEY,
            license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
            api_key_id TEXT NOT NULL UNIQUE,
            scopes_json TEXT NOT NULL,
            status TEXT NOT NULL,
            expires_at TEXT,
            created_at TEXT NOT NULL,
            CHECK(status IN ('active', 'suspended', 'revoked'))
        );

        CREATE INDEX IF NOT EXISTS idx_ai_sdk_credential_bindings_license_status
        ON ai_sdk_credential_bindings(license_id, status);

        CREATE TABLE IF NOT EXISTS ai_marking_sessions (
            marking_session_id TEXT PRIMARY KEY,
            license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
            tenant_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            environment TEXT NOT NULL,
            idempotency_key TEXT NOT NULL,
            requested_profile_ids_json TEXT NOT NULL,
            claim_type TEXT NOT NULL,
            provider_content_id TEXT,
            status TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            confirmed_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(license_id, idempotency_key),
            CHECK(environment IN ('sandbox', 'production')),
            CHECK(claim_type IN ('ai_generated', 'ai_manipulated')),
            CHECK(status IN ('reserved', 'processing', 'ready_to_confirm', 'confirmed', 'failed', 'cancelled', 'expired'))
        );

        CREATE INDEX IF NOT EXISTS idx_ai_marking_sessions_license_status
        ON ai_marking_sessions(license_id, status, created_at DESC);

        CREATE TABLE IF NOT EXISTS ai_transparency_manifests (
            transparency_manifest_id TEXT PRIMARY KEY,
            marking_session_id TEXT NOT NULL UNIQUE REFERENCES ai_marking_sessions(marking_session_id),
            watermark_uid TEXT NOT NULL,
            manifest_version INTEGER NOT NULL,
            status TEXT NOT NULL,
            claim_type TEXT NOT NULL,
            modality TEXT NOT NULL,
            generation_mode TEXT NOT NULL,
            provider_id TEXT NOT NULL,
            system_name TEXT NOT NULL,
            system_version TEXT NOT NULL,
            model_id TEXT,
            model_version TEXT,
            operations_json TEXT NOT NULL,
            generated_at TEXT NOT NULL,
            provider_content_id TEXT,
            subject_digest_algorithm TEXT NOT NULL,
            subject_digest_scope TEXT NOT NULL,
            subject_digest TEXT NOT NULL,
            parent_subjects_json TEXT NOT NULL,
            profile_status_json TEXT NOT NULL DEFAULT '[]',
            manifest_sha256 TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(watermark_uid, manifest_version),
            CHECK(status IN ('active', 'superseded', 'revoked', 'disputed')),
            CHECK(claim_type IN ('ai_generated', 'ai_manipulated')),
            CHECK(modality = 'image'),
            CHECK(subject_digest_algorithm = 'sha256'),
            CHECK(subject_digest_scope = 'protected_output'),
            CHECK(length(subject_digest) = 64 AND subject_digest NOT GLOB '*[^0-9a-f]*'),
            CHECK(length(manifest_sha256) = 64 AND manifest_sha256 NOT GLOB '*[^0-9a-f]*')
        );

        CREATE UNIQUE INDEX IF NOT EXISTS idx_ai_transparency_manifests_one_active
        ON ai_transparency_manifests(watermark_uid)
        WHERE status = 'active';

        CREATE INDEX IF NOT EXISTS idx_ai_transparency_manifests_watermark_status
        ON ai_transparency_manifests(watermark_uid, status, manifest_version DESC);

        CREATE TABLE IF NOT EXISTS ai_claim_evidence (
            evidence_id TEXT PRIMARY KEY,
            transparency_manifest_id TEXT NOT NULL REFERENCES ai_transparency_manifests(transparency_manifest_id),
            evidence_level TEXT NOT NULL,
            evidence_source TEXT NOT NULL,
            issuer_id TEXT,
            key_id TEXT,
            proof_type TEXT NOT NULL,
            subject_digest TEXT NOT NULL,
            signature_algorithm TEXT,
            signature TEXT,
            verification_status TEXT NOT NULL,
            verified_at TEXT,
            failure_code TEXT,
            created_at TEXT NOT NULL,
            CHECK(evidence_level IN ('self_declared', 'device_signed', 'registry_signed', 'platform_signed', 'externally_verified', 'unsupported_proof', 'invalid_proof')),
            CHECK(length(subject_digest) = 64 AND subject_digest NOT GLOB '*[^0-9a-f]*'),
            CHECK(evidence_level NOT IN ('platform_signed', 'registry_signed', 'externally_verified') OR (issuer_id IS NOT NULL AND key_id IS NOT NULL AND signature_algorithm IS NOT NULL AND signature IS NOT NULL)),
            CHECK(evidence_level NOT IN ('unsupported_proof', 'invalid_proof') OR failure_code IS NOT NULL)
        );

        CREATE INDEX IF NOT EXISTS idx_ai_claim_evidence_manifest
        ON ai_claim_evidence(transparency_manifest_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS ai_marker_bindings (
            marker_binding_id TEXT PRIMARY KEY,
            transparency_manifest_id TEXT NOT NULL REFERENCES ai_transparency_manifests(transparency_manifest_id),
            marker_type TEXT NOT NULL,
            marker_profile_id TEXT NOT NULL,
            marker_version TEXT NOT NULL,
            detector_scheme TEXT,
            detector_endpoint TEXT,
            signpost TEXT,
            embed_status TEXT NOT NULL,
            verify_status TEXT NOT NULL,
            binding_digest TEXT,
            created_at TEXT NOT NULL,
            UNIQUE(transparency_manifest_id, marker_type, marker_profile_id),
            CHECK(marker_type IN ('c2pa', 'xmp', 'iptc', 'json_ld', 'blind_watermark', 'explicit_label'))
        );

        CREATE INDEX IF NOT EXISTS idx_ai_marker_bindings_manifest
        ON ai_marker_bindings(transparency_manifest_id, marker_type);

        CREATE TABLE IF NOT EXISTS ai_explicit_label_receipts (
            receipt_id TEXT PRIMARY KEY,
            transparency_manifest_id TEXT NOT NULL REFERENCES ai_transparency_manifests(transparency_manifest_id),
            profile_id TEXT NOT NULL,
            required_surface TEXT NOT NULL,
            render_mode TEXT NOT NULL,
            rendered_asset_digest TEXT,
            placement_json TEXT NOT NULL,
            locale TEXT NOT NULL,
            label_text TEXT NOT NULL,
            applied_at TEXT NOT NULL,
            applied_by TEXT NOT NULL,
            verification_status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE(transparency_manifest_id, profile_id, required_surface),
            CHECK(required_surface IN ('platform_ui', 'exported_file', 'both')),
            CHECK(
                required_surface = 'platform_ui'
                OR (
                    rendered_asset_digest IS NOT NULL
                    AND length(rendered_asset_digest) = 64
                    AND rendered_asset_digest NOT GLOB '*[^0-9a-f]*'
                )
            )
        );

        CREATE INDEX IF NOT EXISTS idx_ai_explicit_label_receipts_manifest
        ON ai_explicit_label_receipts(transparency_manifest_id, profile_id);

        CREATE TABLE IF NOT EXISTS ai_marking_ledger (
            ledger_entry_id TEXT PRIMARY KEY,
            license_id TEXT NOT NULL REFERENCES ai_transparency_licenses(license_id),
            marking_session_id TEXT NOT NULL UNIQUE REFERENCES ai_marking_sessions(marking_session_id),
            transparency_manifest_id TEXT NOT NULL UNIQUE REFERENCES ai_transparency_manifests(transparency_manifest_id),
            metering_unit TEXT NOT NULL,
            quantity INTEGER NOT NULL,
            ledger_status TEXT NOT NULL,
            committed_at TEXT,
            reversal_reason TEXT,
            created_at TEXT NOT NULL,
            CHECK(metering_unit = 'confirmed_marked_image'),
            CHECK(quantity = 1),
            CHECK(ledger_status IN ('pending', 'committed', 'reversed', 'no_charge'))
        );

        CREATE INDEX IF NOT EXISTS idx_ai_marking_ledger_license_status
        ON ai_marking_ledger(license_id, ledger_status, created_at DESC);

        CREATE TABLE IF NOT EXISTS ai_transparency_admin_audit_events (
            audit_event_id TEXT PRIMARY KEY,
            operation TEXT NOT NULL,
            outcome TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            license_id TEXT,
            tenant_id TEXT,
            workspace_id TEXT,
            requested_profile_ids_json TEXT NOT NULL,
            reason_code TEXT NOT NULL,
            details_json TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            CHECK(operation IN ('get_license', 'check_profile_entitlements')),
            CHECK(outcome IN ('succeeded', 'denied', 'failed'))
        );

        CREATE INDEX IF NOT EXISTS idx_ai_transparency_admin_audit_events_license_time
        ON ai_transparency_admin_audit_events(license_id, occurred_at DESC);

        CREATE TABLE IF NOT EXISTS watermark_id_reissue_jobs (
            job_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            creator_profile_id TEXT NOT NULL,
            previous_watermark_uid TEXT NOT NULL,
            replacement_watermark_uid TEXT NOT NULL,
            reason TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_watermark_id_reissue_jobs_account
        ON watermark_id_reissue_jobs(account_id, workspace_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS video_fingerprint_notaries (
            notary_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            creator_profile_id TEXT NOT NULL,
            watermark_uid TEXT NOT NULL,
            source_hash TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            frame_sample_policy TEXT NOT NULL,
            scene_count INTEGER NOT NULL,
            fingerprint_schema_version TEXT NOT NULL,
            global_frame_fingerprints_json TEXT NOT NULL,
            local_block_fingerprint_root TEXT NOT NULL,
            local_block_count INTEGER NOT NULL,
            crop_window_fingerprint_root TEXT NOT NULL,
            crop_window_count INTEGER NOT NULL,
            fingerprint_root TEXT NOT NULL,
            client_signature TEXT NOT NULL,
            server_receipt_signature TEXT NOT NULL,
            upload_manifest_json TEXT NOT NULL,
            created_at TEXT NOT NULL,
            notarized_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_video_fingerprint_notaries_account_workspace
        ON video_fingerprint_notaries(account_id, workspace_id, notarized_at DESC);

        CREATE TABLE IF NOT EXISTS cloud_usage_ledger (
            usage_ledger_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            feature_name TEXT NOT NULL,
            usage_type TEXT NOT NULL,
            quota_type TEXT,
            quota_units INTEGER NOT NULL DEFAULT 0,
            occurred_at TEXT NOT NULL,
            reference_id TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_cloud_usage_ledger_account_feature
        ON cloud_usage_ledger(account_id, feature_name, occurred_at DESC);

        CREATE TABLE IF NOT EXISTS cloud_video_tasks (
            task_id TEXT PRIMARY KEY,
            schema_version TEXT NOT NULL,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            creator_profile_id TEXT NOT NULL,
            capability_level TEXT NOT NULL,
            watermark_uid TEXT NOT NULL,
            source_hash TEXT NOT NULL,
            duration_ms INTEGER NOT NULL,
            target_profiles_json TEXT NOT NULL,
            upload_manifest_json TEXT NOT NULL,
            status TEXT NOT NULL,
            quota_units INTEGER NOT NULL,
            failure_code TEXT,
            strategy_digest TEXT,
            self_check_threshold REAL,
            self_check_confidence REAL,
            checked_frames INTEGER,
            watermarked_media_hash TEXT,
            output_media_storage_ref TEXT,
            output_media_bytes INTEGER,
            output_media_content_type TEXT,
            worker_receipt_hash TEXT,
            worker_receipt_json TEXT,
            server_receipt_signature TEXT,
            usage_ledger_id TEXT,
                worker_id TEXT,
                attempt_id TEXT,
                lease_token_hash TEXT,
                attempt_count INTEGER NOT NULL DEFAULT 0,
                lease_expires_at TEXT,
                last_failure_code TEXT,
                last_failure_stage TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                completed_at TEXT,
            CHECK(status IN ('draft', 'queued', 'running', 'waiting_client_render', 'self_checking', 'succeeded', 'failed', 'canceled', 'expired')),
            CHECK(capability_level IN ('audio_local', 'fingerprint_notary', 'hybrid_visual_watermark'))
        );

        CREATE INDEX IF NOT EXISTS idx_cloud_video_tasks_account_workspace_status
        ON cloud_video_tasks(account_id, workspace_id, status, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_cloud_video_tasks_watermark_uid
        ON cloud_video_tasks(watermark_uid, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_cloud_video_tasks_queue_claim
        ON cloud_video_tasks(status, lease_expires_at, created_at);

        CREATE TABLE IF NOT EXISTS team_workspaces (
            workspace_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            name TEXT NOT NULL,
            workspace_type TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK(workspace_type IN ('personal', 'team')),
            CHECK(status IN ('active', 'suspended', 'archived'))
        );

        CREATE INDEX IF NOT EXISTS idx_team_workspaces_account_status
        ON team_workspaces(account_id, status, updated_at DESC);

        CREATE TABLE IF NOT EXISTS team_members (
            member_id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            account_id TEXT NOT NULL,
            role TEXT NOT NULL,
            status TEXT NOT NULL,
            invited_by TEXT,
            joined_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            CHECK(role IN ('owner', 'admin', 'editor', 'viewer')),
            CHECK(status IN ('invited', 'active', 'removed'))
        );

        CREATE INDEX IF NOT EXISTS idx_team_members_workspace_status
        ON team_members(workspace_id, status, role, updated_at DESC);

        CREATE UNIQUE INDEX IF NOT EXISTS idx_team_members_workspace_account
        ON team_members(workspace_id, account_id);

        CREATE TABLE IF NOT EXISTS team_shared_library_records (
            shared_record_id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            source_record_id TEXT NOT NULL,
            watermark_uid TEXT NOT NULL,
            revision INTEGER NOT NULL,
            record_type TEXT NOT NULL,
            owner_creator_profile_id TEXT NOT NULL,
            visible_to_roles_json TEXT NOT NULL,
            sync_scope TEXT NOT NULL,
            created_by TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(workspace_id, source_record_id, revision),
            CHECK(sync_scope = 'metadata')
        );

        CREATE INDEX IF NOT EXISTS idx_team_shared_library_workspace_uid
        ON team_shared_library_records(workspace_id, watermark_uid, revision DESC);

        CREATE TABLE IF NOT EXISTS team_audit_logs (
            audit_id TEXT PRIMARY KEY,
            workspace_id TEXT NOT NULL,
            actor_account_id TEXT NOT NULL,
            actor_member_id TEXT,
            action TEXT NOT NULL,
            target_type TEXT NOT NULL,
            target_id TEXT NOT NULL,
            before_json TEXT,
            after_json TEXT,
            reason TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_team_audit_logs_workspace_time
        ON team_audit_logs(workspace_id, created_at DESC);

        CREATE TABLE IF NOT EXISTS enterprise_api_keys (
            api_key_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            creator_profile_id TEXT,
            key_prefix TEXT NOT NULL,
            key_hash TEXT NOT NULL,
            name TEXT NOT NULL,
            status TEXT NOT NULL,
            scopes_json TEXT NOT NULL,
            rate_limit_policy_json TEXT NOT NULL,
            quota_policy_json TEXT NOT NULL,
            created_by_account_id TEXT NOT NULL,
            created_at TEXT NOT NULL,
            last_used_at TEXT,
            expires_at TEXT,
            revoked_at TEXT,
            revoked_reason TEXT,
            CHECK(status IN ('active', 'paused', 'revoked', 'expired')),
            CHECK(key_hash != ''),
            CHECK(key_prefix != '')
        );

        CREATE INDEX IF NOT EXISTS idx_enterprise_api_keys_account
        ON enterprise_api_keys(account_id);

        CREATE INDEX IF NOT EXISTS idx_enterprise_api_keys_workspace_status
        ON enterprise_api_keys(workspace_id, status);

        CREATE INDEX IF NOT EXISTS idx_enterprise_api_keys_prefix
        ON enterprise_api_keys(key_prefix);

        CREATE INDEX IF NOT EXISTS idx_enterprise_api_keys_hash
        ON enterprise_api_keys(key_hash);

        CREATE TABLE IF NOT EXISTS enterprise_quota_balances (
            quota_balance_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            quota_type TEXT NOT NULL,
            period_start TEXT NOT NULL,
            period_end TEXT NOT NULL,
            included_units INTEGER NOT NULL,
            used_units INTEGER NOT NULL DEFAULT 0,
            reserved_units INTEGER NOT NULL DEFAULT 0,
            overage_allowed INTEGER NOT NULL DEFAULT 0,
            overage_unit_price_cents INTEGER,
            currency TEXT NOT NULL DEFAULT 'CNY',
            updated_at TEXT NOT NULL,
            UNIQUE(account_id, workspace_id, quota_type, period_start, period_end),
            CHECK(included_units >= 0),
            CHECK(used_units >= 0),
            CHECK(reserved_units >= 0)
        );

        CREATE INDEX IF NOT EXISTS idx_enterprise_quota_balances_account_period
        ON enterprise_quota_balances(account_id, period_start, period_end);

        CREATE INDEX IF NOT EXISTS idx_enterprise_quota_balances_workspace_type
        ON enterprise_quota_balances(workspace_id, quota_type);

        CREATE TABLE IF NOT EXISTS enterprise_quota_ledger (
            quota_ledger_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            api_key_id TEXT,
            quota_type TEXT NOT NULL,
            units INTEGER NOT NULL,
            direction TEXT NOT NULL,
            event_type TEXT NOT NULL,
            reference_id TEXT NOT NULL,
            idempotency_key TEXT NOT NULL,
            status TEXT NOT NULL,
            created_at TEXT NOT NULL,
            committed_at TEXT,
            UNIQUE(account_id, workspace_id, quota_type, idempotency_key),
            CHECK(direction IN ('debit', 'credit')),
            CHECK(status IN ('reserved', 'committed', 'voided'))
        );

        CREATE INDEX IF NOT EXISTS idx_enterprise_quota_ledger_account_type_time
        ON enterprise_quota_ledger(account_id, quota_type, created_at DESC);

        CREATE INDEX IF NOT EXISTS idx_enterprise_quota_ledger_reference
        ON enterprise_quota_ledger(reference_id);

        CREATE INDEX IF NOT EXISTS idx_enterprise_quota_ledger_idempotency
        ON enterprise_quota_ledger(idempotency_key);

        CREATE TABLE IF NOT EXISTS enterprise_api_audit_events (
            audit_event_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            api_key_id TEXT,
            endpoint TEXT NOT NULL,
            method TEXT NOT NULL,
            request_count INTEGER NOT NULL,
            item_count INTEGER NOT NULL,
            status_code INTEGER NOT NULL,
            error_code TEXT,
            quota_units INTEGER NOT NULL DEFAULT 0,
            client_label TEXT,
            client_fingerprint_hash TEXT,
            trusted_proxy_status TEXT,
            request_id TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            CHECK(request_count >= 0),
            CHECK(item_count >= 0)
        );

        CREATE INDEX IF NOT EXISTS idx_enterprise_api_audit_events_account_time
        ON enterprise_api_audit_events(account_id, occurred_at DESC);

        CREATE INDEX IF NOT EXISTS idx_enterprise_api_audit_events_key_time
        ON enterprise_api_audit_events(api_key_id, occurred_at DESC);

        CREATE INDEX IF NOT EXISTS idx_enterprise_api_audit_events_request
        ON enterprise_api_audit_events(request_id);

        CREATE TABLE IF NOT EXISTS enterprise_rate_limit_windows (
            api_key_id TEXT NOT NULL,
            policy_id TEXT NOT NULL,
            window_start TEXT NOT NULL,
            request_count INTEGER NOT NULL DEFAULT 0,
            item_count INTEGER NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL,
            PRIMARY KEY(api_key_id, policy_id, window_start),
            CHECK(request_count >= 0),
            CHECK(item_count >= 0)
        );

        CREATE INDEX IF NOT EXISTS idx_enterprise_rate_limit_windows_key_time
        ON enterprise_rate_limit_windows(api_key_id, window_start DESC);

        CREATE TABLE IF NOT EXISTS enterprise_admin_audit_events (
            audit_event_id TEXT PRIMARY KEY,
            operation TEXT NOT NULL,
            outcome TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            account_id TEXT,
            workspace_id TEXT,
            api_key_id TEXT,
            target_id TEXT,
            reason TEXT NOT NULL,
            details_json TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            CHECK(operation IN (
                'create_api_key',
                'issue_api_key',
                'rotate_api_key',
                'revoke_expired_rotations',
                'list_api_keys',
                'get_api_key',
                'pause_api_key',
                'revoke_api_key',
                'init_quota_balance',
                'dry_run_gateway'
            )),
            CHECK(outcome IN ('succeeded', 'failed'))
        );

        CREATE INDEX IF NOT EXISTS idx_enterprise_admin_audit_operation_time
        ON enterprise_admin_audit_events(operation, occurred_at DESC);

        CREATE INDEX IF NOT EXISTS idx_enterprise_admin_audit_account_time
        ON enterprise_admin_audit_events(account_id, occurred_at DESC);

        CREATE INDEX IF NOT EXISTS idx_enterprise_admin_audit_api_key_time
        ON enterprise_admin_audit_events(api_key_id, occurred_at DESC);

        CREATE TABLE IF NOT EXISTS billing_payment_sessions (
            payment_session_id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            provider_order_id TEXT NOT NULL,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            plan_code TEXT NOT NULL,
            billing_cycle TEXT NOT NULL,
            amount_cents INTEGER NOT NULL,
            currency TEXT NOT NULL,
            status TEXT NOT NULL,
            payment_action_json TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            last_provider_event_id TEXT,
            last_provider_transaction_id TEXT,
            last_checked_at TEXT,
            next_check_after TEXT,
            check_attempts INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(provider, provider_order_id)
        );

        CREATE INDEX IF NOT EXISTS idx_billing_payment_sessions_account
        ON billing_payment_sessions(account_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS report_purchase_sessions (
            payment_session_id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            provider_order_id TEXT NOT NULL,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            creator_profile_id TEXT NOT NULL,
            vault_record_id TEXT NOT NULL,
            product_code TEXT NOT NULL,
            price_cents INTEGER NOT NULL,
            currency TEXT NOT NULL,
            status TEXT NOT NULL,
            payment_action_json TEXT NOT NULL,
            expires_at TEXT NOT NULL,
            last_provider_event_id TEXT,
            last_provider_transaction_id TEXT,
            last_checked_at TEXT,
            next_check_after TEXT,
            check_attempts INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(provider, provider_order_id)
        );

        CREATE INDEX IF NOT EXISTS idx_report_purchase_sessions_account
        ON report_purchase_sessions(account_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS report_purchase_grants (
            grant_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            workspace_id TEXT NOT NULL,
            creator_profile_id TEXT NOT NULL,
            vault_record_id TEXT NOT NULL,
            product_code TEXT NOT NULL,
            price_cents INTEGER NOT NULL,
            currency TEXT NOT NULL,
            payment_session_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            provider_order_id TEXT NOT NULL,
            status TEXT NOT NULL,
            granted_at TEXT NOT NULL,
            revoked_at TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(account_id, workspace_id, vault_record_id, product_code)
        );

        CREATE INDEX IF NOT EXISTS idx_report_purchase_grants_record
        ON report_purchase_grants(account_id, workspace_id, vault_record_id, status);

        CREATE TABLE IF NOT EXISTS billing_customers (
            account_id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            provider_customer_id TEXT NOT NULL,
            email TEXT NOT NULL,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(provider, provider_customer_id)
        );

        CREATE TABLE IF NOT EXISTS subscriptions (
            subscription_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL,
            provider TEXT NOT NULL,
            provider_subscription_id TEXT NOT NULL,
            provider_price_id TEXT NOT NULL,
            provider_product_id TEXT,
            provider_order_id TEXT,
            provider_transaction_id TEXT,
            plan_code TEXT NOT NULL,
            billing_cycle TEXT NOT NULL,
            status TEXT NOT NULL,
            current_period_started_at TEXT,
            current_period_ends_at TEXT,
            trial_started_at TEXT,
            trial_ends_at TEXT,
            grace_ends_at TEXT,
            cancel_at_period_end INTEGER NOT NULL DEFAULT 0,
            canceled_at TEXT,
            latest_invoice_id TEXT,
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL,
            UNIQUE(provider, provider_subscription_id)
        );

        CREATE INDEX IF NOT EXISTS idx_subscriptions_account
        ON subscriptions(account_id, updated_at DESC);

        CREATE TABLE IF NOT EXISTS subscription_events (
            event_id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            provider_event_id TEXT NOT NULL,
            event_type TEXT NOT NULL,
            account_id TEXT,
            provider_customer_id TEXT,
            provider_subscription_id TEXT,
            provider_order_id TEXT,
            provider_transaction_id TEXT,
            payload_json TEXT NOT NULL,
            received_at TEXT NOT NULL,
            processed_at TEXT,
            processing_status TEXT NOT NULL,
            processing_error TEXT,
            UNIQUE(provider, provider_event_id)
        );

        CREATE INDEX IF NOT EXISTS idx_subscription_events_account
        ON subscription_events(account_id, received_at DESC);

        CREATE TABLE IF NOT EXISTS entitlements (
            entitlement_id TEXT PRIMARY KEY,
            account_id TEXT NOT NULL UNIQUE,
            plan_code TEXT NOT NULL,
            plan_name TEXT NOT NULL,
            status TEXT NOT NULL,
            features_json TEXT NOT NULL,
            billing_source TEXT,
            subscription_id TEXT,
            trial_started_at TEXT,
            trial_ends_at TEXT,
            current_period_started_at TEXT,
            current_period_ends_at TEXT,
            grace_ends_at TEXT,
            last_provider_event_id TEXT,
            updated_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS admin_audit_events (
            audit_id TEXT PRIMARY KEY,
            endpoint TEXT NOT NULL,
            outcome TEXT NOT NULL,
            reason TEXT NOT NULL,
            occurred_at TEXT NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_admin_audit_events_endpoint
        ON admin_audit_events(endpoint, occurred_at DESC);
        "#,
    )?;
    ensure_column(conn, "cloud_accounts", "password_hash", "TEXT")?;
    ensure_column(conn, "cloud_accounts", "password_salt", "TEXT")?;
    ensure_column(
        conn,
        "cloud_accounts",
        "password_hash_algorithm",
        "TEXT NOT NULL DEFAULT 'sha256'",
    )?;
    ensure_column(
        conn,
        "cloud_devices",
        "auto_sync_enabled",
        "INTEGER NOT NULL DEFAULT 1",
    )?;
    ensure_column(conn, "cloud_sessions", "expires_at", "TEXT")?;
    ensure_column(conn, "cloud_sessions", "refresh_expires_at", "TEXT")?;
    ensure_column(conn, "cloud_sessions", "last_used_at", "TEXT")?;
    ensure_column(conn, "cloud_sessions", "token_family_id", "TEXT")?;
    ensure_column(conn, "cloud_sync_events", "payload_hash", "TEXT")?;
    ensure_column(conn, "cloud_sync_events", "entity_revision", "INTEGER")?;
    ensure_column(conn, "cloud_video_tasks", "watermarked_media_hash", "TEXT")?;
    ensure_column(
        conn,
        "cloud_video_tasks",
        "output_media_storage_ref",
        "TEXT",
    )?;
    ensure_column(conn, "cloud_video_tasks", "output_media_bytes", "INTEGER")?;
    ensure_column(
        conn,
        "cloud_video_tasks",
        "output_media_content_type",
        "TEXT",
    )?;
    ensure_column(conn, "cloud_video_tasks", "worker_receipt_hash", "TEXT")?;
    ensure_column(conn, "cloud_video_tasks", "worker_receipt_json", "TEXT")?;
    ensure_column(conn, "cloud_video_tasks", "worker_id", "TEXT")?;
    ensure_column(conn, "cloud_video_tasks", "attempt_id", "TEXT")?;
    ensure_column(conn, "cloud_video_tasks", "lease_token_hash", "TEXT")?;
    ensure_column(
        conn,
        "cloud_video_tasks",
        "attempt_count",
        "INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(conn, "cloud_video_tasks", "lease_expires_at", "TEXT")?;
    ensure_column(conn, "cloud_video_tasks", "last_failure_code", "TEXT")?;
    ensure_column(conn, "cloud_video_tasks", "last_failure_stage", "TEXT")?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_cloud_video_tasks_queue_claim
         ON cloud_video_tasks(status, lease_expires_at, created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_auth_challenges_identifier_created
         ON auth_challenges(identifier, created_at)",
        [],
    )?;
    conn.execute(
        "CREATE INDEX IF NOT EXISTS idx_auth_attempts_identifier_created
         ON auth_attempts(identifier, created_at)",
        [],
    )?;
    ensure_column(conn, "auth_challenges", "plain_code_for_delivery", "TEXT")?;
    ensure_column(
        conn,
        "enterprise_api_audit_events",
        "client_fingerprint_hash",
        "TEXT",
    )?;
    ensure_column(
        conn,
        "enterprise_api_audit_events",
        "trusted_proxy_status",
        "TEXT",
    )?;
    ensure_enterprise_admin_audit_events_allow_current_operations(conn)?;
    apply_sqlite_ai_transparency_approval_state_machine(conn)?;
    Ok(())
}

fn ensure_enterprise_admin_audit_events_allow_current_operations(
    conn: &Connection,
) -> Result<(), rusqlite::Error> {
    let table_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'enterprise_admin_audit_events'",
            [],
            |row| row.get(0),
        )
        .unwrap_or_default();
    if table_sql.contains("'dry_run_gateway'")
        && table_sql.contains("'issue_api_key'")
        && table_sql.contains("'rotate_api_key'")
        && table_sql.contains("'revoke_expired_rotations'")
    {
        return Ok(());
    }

    conn.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS enterprise_admin_audit_events_next (
            audit_event_id TEXT PRIMARY KEY,
            operation TEXT NOT NULL,
            outcome TEXT NOT NULL,
            endpoint TEXT NOT NULL,
            account_id TEXT,
            workspace_id TEXT,
            api_key_id TEXT,
            target_id TEXT,
            reason TEXT NOT NULL,
            details_json TEXT NOT NULL,
            occurred_at TEXT NOT NULL,
            CHECK(operation IN (
                'create_api_key',
                'issue_api_key',
                'rotate_api_key',
                'revoke_expired_rotations',
                'list_api_keys',
                'get_api_key',
                'pause_api_key',
                'revoke_api_key',
                'init_quota_balance',
                'dry_run_gateway'
            )),
            CHECK(outcome IN ('succeeded', 'failed'))
        );

        INSERT OR IGNORE INTO enterprise_admin_audit_events_next (
            audit_event_id, operation, outcome, endpoint, account_id, workspace_id,
            api_key_id, target_id, reason, details_json, occurred_at
        )
        SELECT
            audit_event_id, operation, outcome, endpoint, account_id, workspace_id,
            api_key_id, target_id, reason, details_json, occurred_at
        FROM enterprise_admin_audit_events;

        DROP TABLE enterprise_admin_audit_events;
        ALTER TABLE enterprise_admin_audit_events_next RENAME TO enterprise_admin_audit_events;

        CREATE INDEX IF NOT EXISTS idx_enterprise_admin_audit_operation_time
        ON enterprise_admin_audit_events(operation, occurred_at DESC);

        CREATE INDEX IF NOT EXISTS idx_enterprise_admin_audit_account_time
        ON enterprise_admin_audit_events(account_id, occurred_at DESC);

        CREATE INDEX IF NOT EXISTS idx_enterprise_admin_audit_api_key_time
        ON enterprise_admin_audit_events(api_key_id, occurred_at DESC);
        "#,
    )?;
    Ok(())
}

fn ensure_column(
    conn: &Connection,
    table: &str,
    column: &str,
    column_type: &str,
) -> Result<(), rusqlite::Error> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(());
        }
    }
    conn.execute(
        &format!("ALTER TABLE {table} ADD COLUMN {column} {column_type}"),
        [],
    )?;
    Ok(())
}

fn legacy_sha256_password_hash(password: &str, salt_hex: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(salt_hex.as_bytes());
    hasher.update(b":");
    hasher.update(password.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn auth_code_hash(code: &str, salt_hex: &str) -> String {
    legacy_sha256_password_hash(code, salt_hex)
}

fn password_hash(password: &str, salt_hex: &str) -> String {
    let salt = SaltString::from_b64(salt_hex).unwrap_or_else(|_| SaltString::generate(&mut OsRng));
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .unwrap_or_else(|_| legacy_sha256_password_hash(password, salt_hex))
}

fn cloud_sync_payload_hash(payload: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(payload).unwrap_or_else(|_| b"{}".to_vec());
    format!("sha256:{}", hex_string(&Sha256::digest(bytes)))
}

fn cloud_sync_entity_revision(payload: &serde_json::Value) -> Option<i64> {
    payload
        .get("revision")
        .and_then(|value| value.as_i64().or_else(|| value.as_u64().map(|n| n as i64)))
        .filter(|revision| *revision > 0)
}

fn verify_password(password: &str, stored_hash: &str, stored_salt: &str, algorithm: &str) -> bool {
    if stored_hash.starts_with("$argon2") || algorithm == "argon2id" {
        return PasswordHash::new(stored_hash)
            .ok()
            .and_then(|parsed| {
                Argon2::default()
                    .verify_password(password.as_bytes(), &parsed)
                    .ok()
            })
            .is_some();
    }
    legacy_sha256_password_hash(password, stored_salt) == stored_hash
}

fn new_password_salt() -> String {
    let mut salt = [0_u8; 16];
    OsRng.fill_bytes(&mut salt);
    hex_string(&salt)
}

fn generate_otp_code() -> String {
    let mut bytes = [0_u8; 4];
    OsRng.fill_bytes(&mut bytes);
    let value = u32::from_le_bytes(bytes) % 1_000_000;
    format!("{value:06}")
}

fn auth_delivery_channel() -> String {
    if std::env::var("HIDDENSHIELD_AUTH_OTP_DELIVERY_ENDPOINT")
        .ok()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
    {
        std::env::var("HIDDENSHIELD_AUTH_OTP_DELIVERY_CHANNEL")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "email_or_sms".to_string())
    } else {
        "fixture".to_string()
    }
}

fn algorithm_normalized(value: Option<String>) -> String {
    match value.as_deref() {
        Some("argon2id") => "argon2id".to_string(),
        _ => "sha256".to_string(),
    }
}

fn ensure_auth_challenge_rate_limit(
    conn: &rusqlite::Connection,
    identifier: &str,
    client_device_id: &str,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let minute_ago = (now - Duration::minutes(1)).to_rfc3339();
    let hour_ago = (now - Duration::hours(1)).to_rfc3339();
    let recent_for_device: i64 = conn.query_row(
        "SELECT COUNT(*) FROM auth_challenges
         WHERE identifier = ?1 AND client_device_id = ?2 AND created_at >= ?3",
        params![identifier, client_device_id, minute_ago],
        |row| row.get(0),
    )?;
    if recent_for_device >= 1 {
        return Err(StorageError::RateLimited(
            "auth_challenge_too_frequent".to_string(),
        ));
    }
    let hourly_for_identifier: i64 = conn.query_row(
        "SELECT COUNT(*) FROM auth_challenges
         WHERE identifier = ?1 AND created_at >= ?2",
        params![identifier, hour_ago],
        |row| row.get(0),
    )?;
    if hourly_for_identifier >= 5 {
        return Err(StorageError::RateLimited(
            "auth_challenge_hourly_limit".to_string(),
        ));
    }
    Ok(())
}

fn ensure_auth_login_rate_limit(
    conn: &rusqlite::Connection,
    identifier: &str,
    client_device_id: &str,
) -> Result<(), StorageError> {
    let since = (Utc::now() - Duration::minutes(15)).to_rfc3339();
    let failed_for_identifier: i64 = conn.query_row(
        "SELECT COUNT(*) FROM auth_attempts
         WHERE identifier = ?1 AND outcome = 'failure' AND created_at >= ?2",
        params![identifier, since],
        |row| row.get(0),
    )?;
    let failed_for_device: i64 = conn.query_row(
        "SELECT COUNT(*) FROM auth_attempts
         WHERE identifier = ?1 AND client_device_id = ?2 AND outcome = 'failure' AND created_at >= ?3",
        params![identifier, client_device_id, since],
        |row| row.get(0),
    )?;
    if failed_for_identifier >= 10 || failed_for_device >= 5 {
        return Err(StorageError::RateLimited(
            "auth_login_temporarily_limited".to_string(),
        ));
    }
    Ok(())
}

fn record_auth_attempt_tx(
    tx: &rusqlite::Transaction<'_>,
    identifier: &str,
    client_device_id: Option<&str>,
    attempt_type: &str,
    outcome: &str,
    reason: &str,
    now: &str,
) -> Result<(), rusqlite::Error> {
    let attempt_id = format!(
        "auth_attempt_{}",
        short_id(&format!(
            "{}:{}:{}:{}:{}",
            identifier,
            client_device_id.unwrap_or_default(),
            attempt_type,
            outcome,
            now
        ))
    );
    tx.execute(
        "INSERT OR IGNORE INTO auth_attempts (
            attempt_id, identifier, client_device_id, attempt_type, outcome, reason, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            attempt_id,
            identifier,
            client_device_id,
            attempt_type,
            outcome,
            reason,
            now,
        ],
    )?;
    Ok(())
}

fn hex_string(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn insert_anonymous_event(
    tx: &rusqlite::Transaction<'_>,
    event: &AnonymousFeedbackEvent,
) -> Result<bool, rusqlite::Error> {
    let changed = tx.execute(
        "INSERT OR IGNORE INTO feedback_events (
            event_id, occurred_at, install_id, session_id, app_version, feature_name,
            outcome, media_type, file_size_bucket, duration_ms, error_code,
            diagnostic_note, stack_summary, pipeline_id, ingested_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            event.event_id,
            event.occurred_at.to_rfc3339(),
            event.install_id,
            event.session_id,
            event.app_version,
            event.feature_name,
            outcome_to_str(&event.outcome),
            event.media_type,
            event.file_size_bucket,
            event.duration_ms.map(|v| v as i64),
            event.error_code,
            event.diagnostic_note,
            event.stack_summary,
            event.pipeline_id,
            Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(changed > 0)
}

fn usage_units_for_feature(
    conn: &rusqlite::Connection,
    feature_name: &str,
) -> Result<u64, rusqlite::Error> {
    conn.query_row(
        "SELECT COALESCE(SUM(quota_units), 0) FROM cloud_usage_ledger WHERE feature_name = ?1",
        params![feature_name],
        |row| Ok(row.get::<_, Option<i64>>(0)?.unwrap_or_default() as u64),
    )
}

fn ensure_account(
    tx: &rusqlite::Transaction<'_>,
    identifier: &str,
    creator_display_name: &str,
    creator_seed_ref: &str,
    seed_envelope_version: u32,
    password: Option<&str>,
    now: &str,
) -> Result<CloudAccountRow, StorageError> {
    let existing = tx
        .query_row(
            "SELECT id, password_hash, password_salt, password_hash_algorithm
             FROM cloud_accounts WHERE identifier = ?1",
            params![identifier],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, Option<String>>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?;

    let (account_id, password_hash, password_salt, password_hash_algorithm) = match existing {
        Some((account_id, Some(stored_hash), Some(stored_salt), stored_algorithm)) => {
            if let Some(password) = password {
                let algorithm = stored_algorithm.as_deref().unwrap_or("sha256");
                if !verify_password(password, &stored_hash, &stored_salt, algorithm) {
                    return Err(StorageError::Unauthorized);
                }
                if algorithm != "argon2id" || !stored_hash.starts_with("$argon2") {
                    let salt = new_password_salt();
                    let hash = password_hash(password, &salt);
                    (account_id, hash, salt, "argon2id".to_string())
                } else {
                    (account_id, stored_hash, stored_salt, "argon2id".to_string())
                }
            } else {
                (
                    account_id,
                    stored_hash,
                    stored_salt,
                    algorithm_normalized(stored_algorithm),
                )
            }
        }
        Some((account_id, _, _, _)) => {
            if let Some(password) = password {
                let salt = new_password_salt();
                let hash = password_hash(password, &salt);
                (account_id, hash, salt, "argon2id".to_string())
            } else {
                (
                    account_id,
                    String::new(),
                    String::new(),
                    "argon2id".to_string(),
                )
            }
        }
        None => {
            if let Some(password) = password {
                let salt = new_password_salt();
                let hash = password_hash(password, &salt);
                (
                    format!("acct_{}", short_id(identifier)),
                    hash,
                    salt,
                    "argon2id".to_string(),
                )
            } else {
                (
                    format!("acct_{}", short_id(identifier)),
                    String::new(),
                    String::new(),
                    "argon2id".to_string(),
                )
            }
        }
    };
    let workspace_id = format!("ws_{}", short_id(&account_id));
    let creator_profile_id = format!("creator_{}", short_id(&account_id));
    let entitlement_id = format!("ent_{}", short_id(&account_id));
    let display_name = identifier.to_string();
    let workspace_name = "个人空间".to_string();
    let entitlement_plan_name = "免费版".to_string();
    let entitlement_plan_code = "free".to_string();
    let entitlement_status = "free".to_string();
    let entitlement_features_json = default_entitlement_features().to_string();

    tx.execute(
        "INSERT INTO cloud_accounts (
            id, identifier, password_hash, password_salt, password_hash_algorithm,
            display_name, workspace_id, workspace_name,
            creator_profile_id, creator_display_name, creator_seed_ref, seed_envelope_version,
            entitlement_id, entitlement_plan_name, entitlement_plan_code, entitlement_status,
            entitlement_features_json, created_at, updated_at
        ) VALUES (?1, ?2, NULLIF(?3, ''), NULLIF(?4, ''), ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
        ON CONFLICT(identifier) DO UPDATE SET
            password_hash = COALESCE(excluded.password_hash, cloud_accounts.password_hash),
            password_salt = COALESCE(excluded.password_salt, cloud_accounts.password_salt),
            password_hash_algorithm = CASE
                WHEN excluded.password_hash IS NULL THEN cloud_accounts.password_hash_algorithm
                ELSE excluded.password_hash_algorithm
            END,
            display_name = excluded.display_name,
            creator_display_name = excluded.creator_display_name,
            creator_seed_ref = excluded.creator_seed_ref,
            seed_envelope_version = excluded.seed_envelope_version,
            updated_at = excluded.updated_at",
        params![
            account_id,
            identifier,
            password_hash,
            password_salt,
            password_hash_algorithm,
            display_name,
            workspace_id,
            workspace_name,
            creator_profile_id,
            creator_display_name,
            creator_seed_ref,
            seed_envelope_version as i64,
            entitlement_id,
            entitlement_plan_name,
            entitlement_plan_code,
            entitlement_status,
            entitlement_features_json,
            now,
            now,
        ],
    )
    .map_err(StorageError::from)?;
    ensure_free_entitlement(tx, &account_id, &entitlement_id, now).map_err(StorageError::from)?;
    seed_team_workspace_records(tx, &account_id, &workspace_id, &display_name, &now)?;

    tx.query_row(
        "SELECT id, display_name, workspace_id, workspace_name, creator_profile_id,
                creator_display_name, entitlement_id, entitlement_plan_name,
                entitlement_plan_code, entitlement_status, entitlement_features_json
         FROM cloud_accounts WHERE identifier = ?1",
        params![identifier],
        |row| {
            Ok(CloudAccountRow {
                id: row.get(0)?,
                display_name: row.get(1)?,
                workspace_id: row.get(2)?,
                workspace_name: row.get(3)?,
                creator_profile_id: row.get(4)?,
                creator_display_name: row.get(5)?,
                entitlement_id: row.get(6)?,
                entitlement_plan_name: row.get(7)?,
                entitlement_plan_code: row.get(8)?,
                entitlement_status: row.get(9)?,
                entitlement_features_json: row.get(10)?,
            })
        },
    )
    .map_err(StorageError::from)
}

fn load_account_by_id_tx(
    tx: &rusqlite::Transaction<'_>,
    account_id: &str,
) -> Result<CloudAccountRow, StorageError> {
    tx.query_row(
        "SELECT id, display_name, workspace_id, workspace_name,
                creator_profile_id, creator_display_name,
                entitlement_id, entitlement_plan_name, entitlement_plan_code,
                entitlement_status, entitlement_features_json
         FROM cloud_accounts
         WHERE id = ?1",
        params![account_id],
        |row| {
            Ok(CloudAccountRow {
                id: row.get(0)?,
                display_name: row.get(1)?,
                workspace_id: row.get(2)?,
                workspace_name: row.get(3)?,
                creator_profile_id: row.get(4)?,
                creator_display_name: row.get(5)?,
                entitlement_id: row.get(6)?,
                entitlement_plan_name: row.get(7)?,
                entitlement_plan_code: row.get(8)?,
                entitlement_status: row.get(9)?,
                entitlement_features_json: row.get(10)?,
            })
        },
    )
    .optional()?
    .ok_or(StorageError::Unauthorized)
}

fn ensure_device(
    tx: &rusqlite::Transaction<'_>,
    account_id: &str,
    request: &AuthSessionRequest,
    now: &str,
) -> Result<CloudDeviceRow, rusqlite::Error> {
    let device_id = request.device.client_device_id.trim();
    let name = request.device.name.trim();
    let platform = request.device.platform.trim();
    let app_version = request.device.app_version.trim();
    let public_key = request.device.public_key.clone();
    let device_id = if device_id.is_empty() {
        format!("device_{}", short_id(account_id))
    } else {
        device_id.to_string()
    };
    let device_name = if name.is_empty() {
        "当前设备".to_string()
    } else {
        name.to_string()
    };
    let platform = if platform.is_empty() {
        "unknown".to_string()
    } else {
        platform.to_string()
    };
    let app_version = if app_version.is_empty() {
        "0.1.0".to_string()
    } else {
        app_version.to_string()
    };

    tx.execute(
        "INSERT INTO cloud_devices (
            id, account_id, client_device_id, name, platform, app_version,
            public_key, registered, auto_sync_enabled, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, 1, ?8, ?9)
        ON CONFLICT(id) DO UPDATE SET
            account_id = excluded.account_id,
            client_device_id = excluded.client_device_id,
            name = excluded.name,
            platform = excluded.platform,
            app_version = excluded.app_version,
            public_key = excluded.public_key,
            registered = excluded.registered,
            updated_at = excluded.updated_at",
        params![
            device_id,
            account_id,
            request.device.client_device_id.trim(),
            device_name,
            platform,
            app_version,
            public_key,
            now,
            now,
        ],
    )?;

    tx.query_row(
        "SELECT id, name, platform, auto_sync_enabled FROM cloud_devices
         WHERE account_id = ?1 AND client_device_id = ?2",
        params![account_id, request.device.client_device_id.trim()],
        |row| {
            Ok(CloudDeviceRow {
                id: row.get(0)?,
                name: row.get(1)?,
                platform: row.get(2)?,
                auto_sync_enabled: row.get::<_, i64>(3)? != 0,
            })
        },
    )
}

fn load_account_by_id_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
) -> Result<CloudAccountRow, StorageError> {
    conn.query_row(
        "SELECT id, display_name, workspace_id, workspace_name,
                creator_profile_id, creator_display_name,
                entitlement_id, entitlement_plan_name, entitlement_plan_code,
                entitlement_status, entitlement_features_json
         FROM cloud_accounts
         WHERE id = ?1",
        params![account_id],
        |row| {
            Ok(CloudAccountRow {
                id: row.get(0)?,
                display_name: row.get(1)?,
                workspace_id: row.get(2)?,
                workspace_name: row.get(3)?,
                creator_profile_id: row.get(4)?,
                creator_display_name: row.get(5)?,
                entitlement_id: row.get(6)?,
                entitlement_plan_name: row.get(7)?,
                entitlement_plan_code: row.get(8)?,
                entitlement_status: row.get(9)?,
                entitlement_features_json: row.get(10)?,
            })
        },
    )
    .optional()?
    .ok_or(StorageError::Unauthorized)
}

fn load_device_by_id_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
    device_id: &str,
) -> Result<CloudDeviceRow, StorageError> {
    conn.query_row(
        "SELECT id, name, platform, auto_sync_enabled FROM cloud_devices
         WHERE account_id = ?1 AND id = ?2",
        params![account_id, device_id],
        |row| {
            Ok(CloudDeviceRow {
                id: row.get(0)?,
                name: row.get(1)?,
                platform: row.get(2)?,
                auto_sync_enabled: row.get::<_, i64>(3)? != 0,
            })
        },
    )
    .optional()?
    .ok_or(StorageError::Unauthorized)
}

fn list_account_devices_with_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
    current_device_id: &str,
) -> Result<Vec<AccountDevice>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT
            d.id,
            d.client_device_id,
            d.name,
            d.platform,
            d.app_version,
            d.registered,
            d.auto_sync_enabled,
            d.created_at,
            d.updated_at,
            (
                SELECT COUNT(*)
                FROM cloud_sessions s
                WHERE s.account_id = d.account_id
                  AND s.device_id = d.id
                  AND s.revoked_at IS NULL
            ) AS active_session_count,
            (
                SELECT MAX(s.last_used_at)
                FROM cloud_sessions s
                WHERE s.account_id = d.account_id
                  AND s.device_id = d.id
            ) AS last_seen_at
         FROM cloud_devices d
         WHERE d.account_id = ?1
         ORDER BY CASE WHEN d.id = ?2 THEN 0 ELSE 1 END, d.updated_at DESC",
    )?;
    let rows = stmt.query_map(params![account_id, current_device_id], |row| {
        account_device_from_row(row, current_device_id)
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StorageError::from)
}

fn load_account_device_with_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
    device_id: &str,
    current_device_id: &str,
) -> Result<Option<AccountDevice>, StorageError> {
    conn.query_row(
        "SELECT
            d.id,
            d.client_device_id,
            d.name,
            d.platform,
            d.app_version,
            d.registered,
            d.auto_sync_enabled,
            d.created_at,
            d.updated_at,
            (
                SELECT COUNT(*)
                FROM cloud_sessions s
                WHERE s.account_id = d.account_id
                  AND s.device_id = d.id
                  AND s.revoked_at IS NULL
            ) AS active_session_count,
            (
                SELECT MAX(s.last_used_at)
                FROM cloud_sessions s
                WHERE s.account_id = d.account_id
                  AND s.device_id = d.id
            ) AS last_seen_at
         FROM cloud_devices d
         WHERE d.account_id = ?1 AND d.id = ?2",
        params![account_id, device_id],
        |row| account_device_from_row(row, current_device_id),
    )
    .optional()
    .map_err(StorageError::from)
}

fn account_device_from_row(
    row: &rusqlite::Row<'_>,
    current_device_id: &str,
) -> Result<AccountDevice, rusqlite::Error> {
    let id: String = row.get(0)?;
    let created_at: String = row.get(7)?;
    let updated_at: String = row.get(8)?;
    let last_seen_at: Option<String> = row.get(10)?;
    Ok(AccountDevice {
        is_current: id == current_device_id,
        id,
        client_device_id: row.get(1)?,
        name: row.get(2)?,
        platform: row.get(3)?,
        app_version: row.get(4)?,
        registered: row.get::<_, i64>(5)? != 0,
        auto_sync_enabled: row.get::<_, i64>(6)? != 0,
        created_at: parse_rfc3339_utc(&created_at).map_err(|_| rusqlite::Error::InvalidQuery)?,
        updated_at: parse_rfc3339_utc(&updated_at).map_err(|_| rusqlite::Error::InvalidQuery)?,
        active_session_count: row.get::<_, i64>(9)? as u32,
        last_seen_at: parse_optional_rfc3339_utc(last_seen_at.as_deref())
            .map_err(|_| rusqlite::Error::InvalidQuery)?,
    })
}

fn account_snapshot_from_rows(
    conn: &rusqlite::Connection,
    account: CloudAccountRow,
    device: CloudDeviceRow,
) -> Result<CloudAccountSnapshot, StorageError> {
    let entitlement_features = serde_json::from_str(&account.entitlement_features_json)
        .unwrap_or_else(|_| default_entitlement_features());
    let sync_policy =
        sync_policy_for_entitlement_and_preference(&entitlement_features, device.auto_sync_enabled);
    let cloud_vault_cursor = device_cursor_with_conn(conn, &account.id, &device.id)?;
    Ok(CloudAccountSnapshot {
        account: CloudAccount {
            id: account.id,
            display_name: account.display_name,
        },
        workspace: CloudWorkspace {
            id: account.workspace_id,
            name: account.workspace_name,
        },
        device: CloudDevice {
            id: device.id,
            name: Some(device.name),
            platform: Some(device.platform),
            registered: true,
        },
        creator_profile: CloudCreatorProfile {
            id: account.creator_profile_id,
            display_name: account.creator_display_name,
            is_default: true,
        },
        entitlement: CloudEntitlement {
            id: account.entitlement_id,
            plan_name: Some(account.entitlement_plan_name),
            plan_code: account.entitlement_plan_code,
            status: account.entitlement_status,
            features: entitlement_features,
        },
        sync_policy,
        cloud_vault_cursor,
    })
}

fn consume_auth_challenge_tx(
    tx: &rusqlite::Transaction<'_>,
    challenge_id: &str,
    identifier: &str,
    verification_code: &str,
    now: &str,
) -> Result<(), StorageError> {
    let challenge = tx
        .query_row(
            "SELECT code_hash, code_salt, expires_at, consumed_at
             FROM auth_challenges
             WHERE challenge_id = ?1 AND identifier = ?2",
            params![challenge_id, identifier],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                ))
            },
        )
        .optional()?
        .ok_or(StorageError::Unauthorized)?;
    let (code_hash, code_salt, expires_at, consumed_at) = challenge;
    if consumed_at.is_some() {
        return Err(StorageError::Unauthorized);
    }
    let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| StorageError::Unauthorized)?;
    if expires_at < Utc::now() {
        return Err(StorageError::Unauthorized);
    }
    if auth_code_hash(verification_code.trim(), &code_salt) != code_hash {
        return Err(StorageError::Unauthorized);
    }
    tx.execute(
        "UPDATE auth_challenges SET consumed_at = ?2 WHERE challenge_id = ?1",
        params![challenge_id, now],
    )?;
    Ok(())
}

fn ensure_free_entitlement(
    tx: &rusqlite::Transaction<'_>,
    account_id: &str,
    entitlement_id: &str,
    now: &str,
) -> Result<(), rusqlite::Error> {
    tx.execute(
        "INSERT INTO entitlements (
            entitlement_id, account_id, plan_code, plan_name, status, features_json,
            billing_source, subscription_id, updated_at
        ) VALUES (?1, ?2, 'free', '免费版', 'free', ?3, NULL, NULL, ?4)
        ON CONFLICT(account_id) DO NOTHING",
        params![
            account_id,
            entitlement_id,
            default_entitlement_features().to_string(),
            now
        ],
    )?;
    Ok(())
}

fn create_session(
    tx: &rusqlite::Transaction<'_>,
    account_id: &str,
    device_id: &str,
    now: &str,
) -> Result<SessionTokenRow, rusqlite::Error> {
    let token_nonce = new_password_salt();
    let access_token = format!(
        "hsat_{}_{}_{}",
        short_id(account_id),
        short_id(device_id),
        short_id(&format!("{now}:{token_nonce}:access"))
    );
    let refresh_token = format!(
        "hsrt_{}_{}_{}",
        short_id(account_id),
        short_id(device_id),
        short_id(&format!("{now}:{token_nonce}:refresh"))
    );
    let created_at = chrono::DateTime::parse_from_rfc3339(now)
        .map(|value| value.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now());
    let expires_at = (created_at + Duration::minutes(60)).to_rfc3339();
    let refresh_expires_at = (created_at + Duration::days(90)).to_rfc3339();
    let token_family_id = format!(
        "family_{}",
        short_id(&format!("{account_id}:{device_id}:{token_nonce}"))
    );
    tx.execute(
        "INSERT INTO cloud_sessions (
            access_token, refresh_token, account_id, device_id, created_at, revoked_at,
            expires_at, refresh_expires_at, last_used_at, token_family_id
        ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?5, ?8)",
        params![
            access_token,
            refresh_token,
            account_id,
            device_id,
            now,
            expires_at,
            refresh_expires_at,
            token_family_id,
        ],
    )?;
    Ok(SessionTokenRow {
        access_token,
        refresh_token,
    })
}

fn upsert_device_cursor(
    tx: &rusqlite::Connection,
    account_id: &str,
    device_id: &str,
    cursor: &str,
) -> Result<(), rusqlite::Error> {
    let now = Utc::now().to_rfc3339();
    tx.execute(
        "INSERT INTO cloud_device_cursors (
            account_id, device_id, cursor, updated_at
        ) VALUES (?1, ?2, ?3, ?4)
        ON CONFLICT(account_id, device_id) DO UPDATE SET
            cursor = excluded.cursor,
            updated_at = excluded.updated_at",
        params![account_id, device_id, cursor, now],
    )?;
    Ok(())
}

fn device_cursor_with_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
    device_id: &str,
) -> Result<Option<String>, StorageError> {
    let cursor = conn
        .query_row(
            "SELECT cursor
             FROM cloud_device_cursors
             WHERE account_id = ?1 AND device_id = ?2",
            params![account_id, device_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(cursor)
}

fn session_workspace_matches_with_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
    workspace_id: &str,
) -> Result<bool, StorageError> {
    let stored_workspace_id = conn
        .query_row(
            "SELECT workspace_id FROM cloud_accounts WHERE id = ?1",
            params![account_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(stored_workspace_id.as_deref() == Some(workspace_id))
}

fn account_workspace_matches_with_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
    workspace_id: &str,
) -> Result<bool, StorageError> {
    if account_id.trim().is_empty() || workspace_id.trim().is_empty() {
        return Ok(false);
    }
    session_workspace_matches_with_conn(conn, account_id.trim(), workspace_id.trim())
}

fn creator_profile_matches_with_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
    creator_profile_id: &str,
) -> Result<bool, StorageError> {
    if creator_profile_id.trim().is_empty() {
        return Ok(false);
    }
    let stored_creator_profile_id = conn
        .query_row(
            "SELECT creator_profile_id FROM cloud_accounts WHERE id = ?1",
            params![account_id],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(stored_creator_profile_id.as_deref() == Some(creator_profile_id))
}

fn account_cursor_with_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
) -> Result<Option<String>, StorageError> {
    let cursor = conn
        .query_row(
            "SELECT MAX(sequence) FROM cloud_sync_events WHERE account_id = ?1",
            params![account_id],
            |row| row.get::<_, Option<i64>>(0),
        )?
        .map(|value| cursor_from_sequence(value as u64));
    Ok(cursor)
}

fn load_watermark_registry_by_request(
    conn: &rusqlite::Connection,
    account_id: &str,
    request_id: &str,
) -> Result<Option<WatermarkIdRegistryRow>, StorageError> {
    conn.query_row(
        "SELECT registry_id, watermark_uid, watermark_id_issue_mode, registry_status,
                registry_receipt, registry_proof_hash, payload_protocol_version,
                payload_bytes_length, parent_watermark_uid, revision, created_at, updated_at
         FROM watermark_id_registry
         WHERE account_id = ?1 AND request_id = ?2",
        params![account_id, request_id],
        watermark_registry_row_from_sql,
    )
    .optional()
    .map_err(StorageError::from)
}

fn load_watermark_registry_by_uid_tx(
    conn: &rusqlite::Connection,
    watermark_uid: &str,
) -> Result<Option<WatermarkIdRegistryRow>, StorageError> {
    conn.query_row(
        "SELECT registry_id, watermark_uid, watermark_id_issue_mode, registry_status,
                registry_receipt, registry_proof_hash, payload_protocol_version,
                payload_bytes_length, parent_watermark_uid, revision, created_at, updated_at
         FROM watermark_id_registry
         WHERE watermark_uid = ?1",
        params![watermark_uid],
        watermark_registry_row_from_sql,
    )
    .optional()
    .map_err(StorageError::from)
}

fn watermark_registry_row_from_sql(
    row: &rusqlite::Row<'_>,
) -> Result<WatermarkIdRegistryRow, rusqlite::Error> {
    Ok(WatermarkIdRegistryRow {
        registry_id: row.get(0)?,
        watermark_uid: row.get(1)?,
        watermark_id_issue_mode: row.get(2)?,
        registry_status: row.get(3)?,
        registry_receipt: row.get(4)?,
        registry_proof_hash: row.get(5)?,
        payload_protocol_version: row.get(6)?,
        payload_bytes_length: row.get(7)?,
        parent_watermark_uid: row.get(8)?,
        revision: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
    })
}

fn rights_manifest_row_from_sql(
    row: &rusqlite::Row<'_>,
) -> Result<RightsManifestRow, rusqlite::Error> {
    Ok(RightsManifestRow {
        rights_manifest_id: row.get(0)?,
        watermark_uid: row.get(1)?,
        manifest_version: row.get(2)?,
        status: row.get(3)?,
        training_policy: row.get(4)?,
        work_source_declaration: row.get(5)?,
        creation_method_declaration: row.get(6)?,
        human_edit_level_declaration: row.get(7)?,
        authenticity_claim_declaration: row.get(8)?,
        custom_terms_url: row.get(9)?,
        custom_terms_hash: row.get(10)?,
        standard_mappings_json: row.get(11)?,
        manifest_sha256: row.get(12)?,
        signature: row.get(13)?,
        signed_by: row.get(14)?,
        effective_at: row.get(15)?,
        superseded_by_rights_manifest_id: row.get(16)?,
        revoked_at: row.get(17)?,
        created_at: row.get(18)?,
        updated_at: row.get(19)?,
    })
}

fn enterprise_api_key_record_from_sql(
    row: &rusqlite::Row<'_>,
) -> Result<EnterpriseApiKeyRecord, rusqlite::Error> {
    let scopes_json: String = row.get(7)?;
    let scopes = serde_json::from_str::<Vec<String>>(&scopes_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(7, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let created_at: String = row.get(9)?;
    let last_used_at: Option<String> = row.get(10)?;
    let expires_at: Option<String> = row.get(11)?;
    let revoked_at: Option<String> = row.get(12)?;
    Ok(EnterpriseApiKeyRecord {
        api_key_id: row.get(0)?,
        account_id: row.get(1)?,
        workspace_id: row.get(2)?,
        creator_profile_id: row.get(3)?,
        key_prefix: row.get(4)?,
        name: row.get(5)?,
        status: row.get(6)?,
        scopes,
        created_by_account_id: row.get(8)?,
        created_at: parse_utc_rfc3339_for_sql(&created_at, 9)?,
        last_used_at: parse_optional_utc_rfc3339_for_sql(last_used_at, 10)?,
        expires_at: parse_optional_utc_rfc3339_for_sql(expires_at, 11)?,
        revoked_at: parse_optional_utc_rfc3339_for_sql(revoked_at, 12)?,
        revoked_reason: row.get(13)?,
    })
}

fn enterprise_admin_audit_event_record_from_sql(
    row: &rusqlite::Row<'_>,
) -> Result<EnterpriseAdminAuditEventRecord, rusqlite::Error> {
    let details_json: String = row.get(9)?;
    let details = serde_json::from_str::<serde_json::Value>(&details_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(9, rusqlite::types::Type::Text, Box::new(error))
    })?;
    let occurred_at: String = row.get(10)?;
    Ok(EnterpriseAdminAuditEventRecord {
        audit_event_id: row.get(0)?,
        operation: row.get(1)?,
        outcome: row.get(2)?,
        endpoint: row.get(3)?,
        account_id: row.get(4)?,
        workspace_id: row.get(5)?,
        api_key_id: row.get(6)?,
        target_id: row.get(7)?,
        reason: row.get(8)?,
        details,
        occurred_at: parse_utc_rfc3339_for_sql(&occurred_at, 10)?,
    })
}

fn enterprise_cleartext_key_prefix(cleartext_api_key: &str) -> Result<String, StorageError> {
    let key = cleartext_api_key.trim();
    if key.is_empty() {
        return Err(StorageError::Unauthorized);
    }
    if key.len() < 22 || !key.starts_with("hsent_live_") {
        return Err(StorageError::Unauthorized);
    }
    Ok(key.chars().take(22).collect())
}

fn enterprise_api_key_hash_hex(
    cleartext_api_key: &str,
    hash_secret: &str,
    hash_secret_version: &str,
) -> Result<String, StorageError> {
    let secret = hash_secret.trim();
    if secret.is_empty() {
        return Err(StorageError::Forbidden);
    }
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes()).map_err(|_| {
        StorageError::BadRequest("enterprise api key hash secret is invalid".to_string())
    })?;
    mac.update(cleartext_api_key.trim().as_bytes());
    Ok(format!(
        "hmac-sha256:v1:{}:{}",
        hash_secret_version.trim(),
        hex_string(&mac.finalize().into_bytes())
    ))
}

fn load_enterprise_api_key_auth_tx(
    conn: &rusqlite::Connection,
    key_prefix: &str,
    key_hash: &str,
    now: chrono::DateTime<Utc>,
) -> Result<crate::schema::EnterpriseGatewayAuthContext, StorageError> {
    let (
        api_key_id,
        account_id,
        workspace_id,
        stored_prefix,
        status,
        scopes_json,
        expires_at,
    ): (String, String, String, String, String, String, Option<String>) = conn
        .query_row(
            "SELECT api_key_id, account_id, workspace_id, key_prefix, status, scopes_json, expires_at
             FROM enterprise_api_keys
             WHERE key_prefix = ?1 AND key_hash = ?2",
            params![key_prefix, key_hash],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            },
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => StorageError::Unauthorized,
            other => StorageError::Database(other),
        })?;
    let scopes: Vec<String> = serde_json::from_str(&scopes_json).unwrap_or_default();
    let effective_status = if let Some(expires_at) = expires_at.as_deref() {
        if parse_utc_rfc3339(expires_at)? <= now {
            "expired".to_string()
        } else {
            status
        }
    } else {
        status
    };
    Ok(crate::schema::EnterpriseGatewayAuthContext {
        api_key_id,
        account_id,
        workspace_id,
        key_prefix: stored_prefix,
        scopes,
        status: effective_status,
        api_access: true,
    })
}

fn enterprise_rate_limit_window_start(now: chrono::DateTime<Utc>) -> String {
    now.format("%Y-%m-%dT%H:%M:00Z").to_string()
}

fn normalize_enterprise_client_fingerprint(
    fingerprint: &EnterpriseGatewayClientFingerprint,
    api_key_id: &str,
) -> EnterpriseGatewayClientFingerprint {
    let fingerprint_hash = fingerprint.fingerprint_hash.trim();
    if fingerprint_hash.is_empty() {
        return EnterpriseGatewayClientFingerprint {
            fingerprint_hash: String::new(),
            source: "api_key_only".to_string(),
            trusted_proxy: false,
            rate_limit_subject: api_key_id.trim().to_string(),
        };
    }
    let source = fingerprint.source.trim();
    EnterpriseGatewayClientFingerprint {
        fingerprint_hash: fingerprint_hash.to_string(),
        source: if source.is_empty() {
            "trusted_proxy".to_string()
        } else {
            source.to_string()
        },
        trusted_proxy: fingerprint.trusted_proxy,
        rate_limit_subject: if fingerprint.rate_limit_subject.trim().is_empty() {
            format!("{}:{}", api_key_id.trim(), fingerprint_hash)
        } else {
            fingerprint.rate_limit_subject.trim().to_string()
        },
    }
}

fn enterprise_rate_limit_window_tx(
    conn: &rusqlite::Connection,
    api_key_id: &str,
    policy: &EnterpriseGatewayRateLimitPolicy,
    now: chrono::DateTime<Utc>,
) -> Result<(u32, u32), StorageError> {
    let window_start = enterprise_rate_limit_window_start(now);
    let counts = conn
        .query_row(
            "SELECT request_count, item_count
             FROM enterprise_rate_limit_windows
             WHERE api_key_id = ?1 AND policy_id = ?2 AND window_start = ?3",
            params![api_key_id, policy.policy_id, window_start],
            |row| Ok((row.get::<_, i64>(0)? as u32, row.get::<_, i64>(1)? as u32)),
        )
        .optional()?
        .unwrap_or((0, 0));
    Ok(counts)
}

fn increment_enterprise_rate_limit_window_tx(
    conn: &rusqlite::Connection,
    api_key_id: &str,
    policy: &EnterpriseGatewayRateLimitPolicy,
    item_count: i64,
    now: chrono::DateTime<Utc>,
) -> Result<(), StorageError> {
    let window_start = enterprise_rate_limit_window_start(now);
    conn.execute(
        "INSERT INTO enterprise_rate_limit_windows (
            api_key_id, policy_id, window_start, request_count, item_count, updated_at
         ) VALUES (?1, ?2, ?3, 1, ?4, ?5)
         ON CONFLICT(api_key_id, policy_id, window_start)
         DO UPDATE SET
            request_count = request_count + 1,
            item_count = item_count + excluded.item_count,
            updated_at = excluded.updated_at",
        params![
            api_key_id,
            policy.policy_id,
            window_start,
            item_count,
            now.to_rfc3339(),
        ],
    )?;
    Ok(())
}

fn load_active_enterprise_quota_balance_tx(
    conn: &rusqlite::Connection,
    account_id: &str,
    workspace_id: &str,
    quota_type: &str,
    now: chrono::DateTime<Utc>,
) -> Result<EnterpriseQuotaBalanceRecord, StorageError> {
    let now_text = now.to_rfc3339();
    let (
        quota_balance_id,
        account_id,
        workspace_id,
        quota_type,
        period_start,
        period_end,
        included_units,
        used_units,
        reserved_units,
        overage_allowed,
        overage_unit_price_cents,
        currency,
        updated_at,
    ): (
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        i64,
        i64,
        Option<i64>,
        String,
        String,
    ) = conn
        .query_row(
            "SELECT quota_balance_id, account_id, workspace_id, quota_type, period_start, period_end,
                    included_units, used_units, reserved_units, overage_allowed,
                    overage_unit_price_cents, currency, updated_at
             FROM enterprise_quota_balances
             WHERE account_id = ?1 AND workspace_id = ?2 AND quota_type = ?3
               AND period_start <= ?4 AND period_end > ?4
             ORDER BY period_start DESC
             LIMIT 1",
            params![account_id, workspace_id, quota_type, now_text],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                    row.get(8)?,
                    row.get(9)?,
                    row.get(10)?,
                    row.get(11)?,
                    row.get(12)?,
                ))
            },
        )
        .map_err(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => {
                StorageError::BadRequest("quota_contract_missing".to_string())
            }
            other => StorageError::Database(other),
        })?;
    Ok(EnterpriseQuotaBalanceRecord {
        quota_balance_id,
        account_id,
        workspace_id,
        quota_type,
        period_start: parse_utc_rfc3339(&period_start)?,
        period_end: parse_utc_rfc3339(&period_end)?,
        included_units,
        used_units,
        reserved_units,
        overage_allowed: overage_allowed != 0,
        overage_unit_price_cents,
        currency,
        updated_at: parse_utc_rfc3339(&updated_at)?,
    })
}

fn record_enterprise_quota_ledger_tx(
    conn: &rusqlite::Connection,
    request: &EnterpriseQuotaLedgerRequest,
) -> Result<EnterpriseQuotaLedgerRecord, StorageError> {
    if request.account_id.trim().is_empty()
        || request.workspace_id.trim().is_empty()
        || request.quota_type.trim() != ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE
        || request.reference_id.trim().is_empty()
        || request.idempotency_key.trim().is_empty()
        || !matches!(request.direction.as_str(), "debit" | "credit")
        || !matches!(request.status.as_str(), "reserved" | "committed" | "voided")
    {
        return Err(StorageError::BadRequest(
            "enterprise quota ledger request is invalid".to_string(),
        ));
    }
    let now = Utc::now();
    let quota_ledger_id = format!(
        "eql_{}",
        short_id(&format!(
            "{}:{}:{}:{}",
            request.account_id, request.workspace_id, request.quota_type, request.idempotency_key
        ))
    );
    let committed_at = if request.status == "committed" {
        Some(now)
    } else {
        None
    };
    conn.execute(
        "INSERT INTO enterprise_quota_ledger (
            quota_ledger_id, account_id, workspace_id, api_key_id, quota_type, units,
            direction, event_type, reference_id, idempotency_key, status, created_at, committed_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            quota_ledger_id,
            request.account_id.trim(),
            request.workspace_id.trim(),
            request.api_key_id.as_deref(),
            request.quota_type.trim(),
            request.units,
            request.direction.trim(),
            request.event_type.trim(),
            request.reference_id.trim(),
            request.idempotency_key.trim(),
            request.status.trim(),
            now.to_rfc3339(),
            committed_at.as_ref().map(|value| value.to_rfc3339()),
        ],
    )?;
    Ok(EnterpriseQuotaLedgerRecord {
        quota_ledger_id,
        account_id: request.account_id.trim().to_string(),
        workspace_id: request.workspace_id.trim().to_string(),
        api_key_id: request.api_key_id.clone(),
        quota_type: request.quota_type.trim().to_string(),
        units: request.units,
        direction: request.direction.trim().to_string(),
        event_type: request.event_type.trim().to_string(),
        reference_id: request.reference_id.trim().to_string(),
        idempotency_key: request.idempotency_key.trim().to_string(),
        status: request.status.trim().to_string(),
        created_at: now,
        committed_at,
    })
}

fn record_enterprise_api_audit_event_tx(
    conn: &rusqlite::Connection,
    request: &EnterpriseApiAuditEventRequest,
) -> Result<String, StorageError> {
    if request.account_id.trim().is_empty()
        || request.workspace_id.trim().is_empty()
        || request.endpoint.trim().is_empty()
        || request.method.trim().is_empty()
        || request.request_id.trim().is_empty()
    {
        return Err(StorageError::BadRequest(
            "enterprise api audit event request is invalid".to_string(),
        ));
    }
    let now = Utc::now();
    let audit_event_id = format!(
        "eae_{}",
        short_id(&format!(
            "{}:{}:{}",
            request.account_id, request.request_id, now
        ))
    );
    conn.execute(
        "INSERT INTO enterprise_api_audit_events (
            audit_event_id, account_id, workspace_id, api_key_id, endpoint, method,
            request_count, item_count, status_code, error_code, quota_units, client_label,
            client_fingerprint_hash, trusted_proxy_status, request_id, occurred_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            audit_event_id,
            request.account_id.trim(),
            request.workspace_id.trim(),
            request.api_key_id.as_deref(),
            request.endpoint.trim(),
            request.method.trim(),
            request.request_count,
            request.item_count,
            request.status_code,
            request.error_code.as_deref(),
            request.quota_units,
            request.client_label.as_deref(),
            request.client_fingerprint_hash.as_deref(),
            request.trusted_proxy_status.as_deref(),
            request.request_id.trim(),
            now.to_rfc3339(),
        ],
    )?;
    Ok(audit_event_id)
}

fn enterprise_quota_balance_by_key(
    conn: &Connection,
    account_id: &str,
    workspace_id: &str,
    quota_type: &str,
    period_start: &str,
    period_end: &str,
) -> Result<EnterpriseQuotaBalanceRecord, StorageError> {
    let (
        quota_balance_id,
        account_id,
        workspace_id,
        quota_type,
        period_start,
        period_end,
        included_units,
        used_units,
        reserved_units,
        overage_allowed,
        overage_unit_price_cents,
        currency,
        updated_at,
    ): (
        String,
        String,
        String,
        String,
        String,
        String,
        i64,
        i64,
        i64,
        i64,
        Option<i64>,
        String,
        String,
    ) = conn.query_row(
        "SELECT quota_balance_id, account_id, workspace_id, quota_type, period_start, period_end,
                included_units, used_units, reserved_units, overage_allowed,
                overage_unit_price_cents, currency, updated_at
         FROM enterprise_quota_balances
         WHERE account_id = ?1 AND workspace_id = ?2 AND quota_type = ?3
           AND period_start = ?4 AND period_end = ?5",
        params![
            account_id,
            workspace_id,
            quota_type,
            period_start,
            period_end
        ],
        |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
                row.get(11)?,
                row.get(12)?,
            ))
        },
    )?;
    Ok(EnterpriseQuotaBalanceRecord {
        quota_balance_id,
        account_id,
        workspace_id,
        quota_type,
        period_start: parse_utc_rfc3339(&period_start)?,
        period_end: parse_utc_rfc3339(&period_end)?,
        included_units,
        used_units,
        reserved_units,
        overage_allowed: overage_allowed != 0,
        overage_unit_price_cents,
        currency,
        updated_at: parse_utc_rfc3339(&updated_at)?,
    })
}

fn parse_utc_rfc3339(value: &str) -> Result<chrono::DateTime<Utc>, StorageError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| StorageError::BadRequest("timestamp is invalid".to_string()))
}

fn ai_license_reason_code(
    license: &AiTransparencyLicenseRecord,
    environment: &str,
    evaluated_at: chrono::DateTime<Utc>,
) -> &'static str {
    if license.environment != environment {
        "ai_license_environment_mismatch"
    } else if license.status != "active" {
        "ai_license_inactive"
    } else if evaluated_at < license.effective_at {
        "ai_license_not_effective"
    } else if evaluated_at >= license.expires_at {
        "ai_license_expired"
    } else {
        "authorized"
    }
}

fn ai_profile_reason_code(
    entitlement: Option<&AiTransparencyProfileEntitlementRecord>,
    evaluated_at: chrono::DateTime<Utc>,
) -> &'static str {
    let Some(entitlement) = entitlement else {
        return "ai_profile_not_entitled";
    };
    if entitlement.status != "active" {
        "ai_profile_inactive"
    } else if evaluated_at < entitlement.effective_at {
        "ai_profile_not_effective"
    } else if evaluated_at >= entitlement.expires_at {
        "ai_profile_expired"
    } else {
        "authorized"
    }
}

fn parse_utc_rfc3339_for_sql(
    value: &str,
    index: usize,
) -> Result<chrono::DateTime<Utc>, rusqlite::Error> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                index,
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })
}

fn parse_optional_utc_rfc3339_for_sql(
    value: Option<String>,
    index: usize,
) -> Result<Option<chrono::DateTime<Utc>>, rusqlite::Error> {
    value
        .as_deref()
        .map(|value| parse_utc_rfc3339_for_sql(value, index))
        .transpose()
}

fn load_active_rights_manifest_tx(
    conn: &rusqlite::Connection,
    watermark_uid: &str,
) -> Result<Option<RightsManifestRow>, StorageError> {
    conn.query_row(
        "SELECT rights_manifest_id, watermark_uid, manifest_version, status,
                training_policy, work_source_declaration, creation_method_declaration,
                human_edit_level_declaration, authenticity_claim_declaration,
                custom_terms_url, custom_terms_hash, standard_mappings_json,
                manifest_sha256, signature, signed_by, effective_at,
                superseded_by_rights_manifest_id, revoked_at, created_at, updated_at
         FROM rights_manifests
         WHERE watermark_uid = ?1 AND status = 'active'
         ORDER BY manifest_version DESC
         LIMIT 1",
        params![watermark_uid],
        rights_manifest_row_from_sql,
    )
    .optional()
    .map_err(StorageError::from)
}

fn list_rights_manifest_history_tx(
    conn: &rusqlite::Connection,
    watermark_uid: &str,
) -> Result<Vec<RightsManifestSummary>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT rights_manifest_id, watermark_uid, manifest_version, status,
                training_policy, updated_at
         FROM rights_manifests
         WHERE watermark_uid = ?1
         ORDER BY manifest_version DESC
         LIMIT 3",
    )?;
    let rows = stmt.query_map(params![watermark_uid], |row| {
        let updated_at: String = row.get(5)?;
        let updated_at = chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map(|value| value.with_timezone(&Utc))
            .map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    5,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })?;
        Ok(RightsManifestSummary {
            rights_manifest_id: row.get(0)?,
            watermark_uid: row.get(1)?,
            manifest_version: row.get::<_, i64>(2)? as u32,
            status: row.get(3)?,
            training_policy: row.get(4)?,
            updated_at,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StorageError::from)
}

fn public_rights_query_with_conn(
    conn: &rusqlite::Connection,
    watermark_uid: &str,
) -> Result<PublicRightsQueryResponse, StorageError> {
    let registry = load_watermark_registry_by_uid_tx(conn, watermark_uid)?
        .ok_or_else(|| StorageError::BadRequest("watermark_registry_missing".to_string()))?;
    let manifest = load_active_rights_manifest_tx(conn, watermark_uid)?;
    let history = list_rights_manifest_history_tx(conn, watermark_uid)?;
    let scan_status = public_scan_status(&registry, manifest.as_ref());
    let warnings = public_rights_warnings(&registry, manifest.as_ref());
    let training_policy = manifest
        .as_ref()
        .map(|row| row.training_policy.clone())
        .unwrap_or_else(|| "not_declared".to_string());
    let standard_mappings = manifest
        .as_ref()
        .and_then(|row| serde_json::from_str(&row.standard_mappings_json).ok())
        .unwrap_or_else(default_standard_mappings);
    Ok(PublicRightsQueryResponse {
        watermark_uid: watermark_uid.to_string(),
        scan_status,
        registry: public_registry_snapshot_from_row(registry)?,
        rights_manifest: manifest
            .map(rights_manifest_response_from_row)
            .transpose()?,
        history,
        public_metadata: PublicRightsMetadata {
            c2pa: "not_present".to_string(),
            iptc: "not_present".to_string(),
            xmp: "not_present".to_string(),
            consistency: "registry_only".to_string(),
            standard_mappings,
        },
        training_permission: PublicTrainingPermissionSnapshot {
            policy: training_policy.clone(),
            label: training_policy_label(&training_policy).to_string(),
            source: if training_policy == "not_declared" {
                "registry_manifest_missing".to_string()
            } else {
                "creator_declaration_registry".to_string()
            },
            effective_source: if training_policy == "not_declared" {
                "watermark_anchor_only".to_string()
            } else {
                "registry".to_string()
            },
            legal_conclusion: false,
        },
        warnings,
        resolved_at: Utc::now(),
    })
}

fn public_rights_batch_with_conn(
    conn: &rusqlite::Connection,
    request: &PublicRightsBatchRequest,
) -> Result<PublicRightsBatchResponse, StorageError> {
    let mut seen = HashSet::new();
    let mut results = Vec::new();
    let resolved_at = Utc::now();
    for uid in &request.watermark_uids {
        let normalized = match normalize_watermark_uid(uid) {
            Ok(uid) => uid,
            Err(_) => {
                results.push(PublicRightsBatchItem {
                    watermark_uid: uid.trim().to_string(),
                    status: "error".to_string(),
                    error_code: Some("watermark_uid_invalid".to_string()),
                    result: None,
                    resolved_at,
                });
                continue;
            }
        };
        if !seen.insert(normalized.clone()) {
            continue;
        }
        match public_rights_query_with_conn(conn, &normalized) {
            Ok(result) => results.push(PublicRightsBatchItem {
                watermark_uid: normalized,
                status: "ok".to_string(),
                error_code: None,
                result: Some(result),
                resolved_at,
            }),
            Err(StorageError::BadRequest(code)) if code == "watermark_registry_missing" => {
                results.push(PublicRightsBatchItem {
                    watermark_uid: normalized,
                    status: "error".to_string(),
                    error_code: Some("not_found".to_string()),
                    result: None,
                    resolved_at,
                });
            }
            Err(error) => {
                results.push(PublicRightsBatchItem {
                    watermark_uid: normalized,
                    status: "error".to_string(),
                    error_code: Some(error_code_for_storage_error(&error).to_string()),
                    result: None,
                    resolved_at,
                });
            }
        }
    }
    Ok(PublicRightsBatchResponse {
        results,
        resolved_at,
    })
}

fn public_rights_metadata_export_from_query(
    rights: &PublicRightsQueryResponse,
) -> PublicRightsMetadataExport {
    let generated_at = Utc::now();
    let manifest_hash = rights
        .rights_manifest
        .as_ref()
        .map(|manifest| manifest.manifest_sha256.clone())
        .unwrap_or_else(|| {
            sha256_hex(
                serde_json::json!({
                    "watermarkUid": rights.watermark_uid,
                    "registryProofHash": rights.registry.registry_proof_hash,
                    "trainingPolicy": rights.training_permission.policy,
                    "scanStatus": rights.scan_status,
                })
                .to_string()
                .as_bytes(),
            )
        });
    let work_source = rights
        .rights_manifest
        .as_ref()
        .map(|manifest| manifest.work_source_declaration.as_str())
        .unwrap_or("unspecified");
    let creation_method = rights
        .rights_manifest
        .as_ref()
        .map(|manifest| manifest.creation_method_declaration.as_str())
        .unwrap_or("unspecified");
    let human_edit_level = rights
        .rights_manifest
        .as_ref()
        .map(|manifest| manifest.human_edit_level_declaration.as_str())
        .unwrap_or("unspecified");
    let authenticity_claim = rights
        .rights_manifest
        .as_ref()
        .map(|manifest| manifest.authenticity_claim_declaration.as_str())
        .unwrap_or("unspecified");
    let custom_terms_url = rights
        .rights_manifest
        .as_ref()
        .and_then(|manifest| manifest.custom_terms_url.clone());
    let custom_terms_hash = rights
        .rights_manifest
        .as_ref()
        .and_then(|manifest| manifest.custom_terms_hash.clone());
    let policy = &rights.training_permission.policy;
    let manifest_id = rights
        .rights_manifest
        .as_ref()
        .map(|manifest| manifest.rights_manifest_id.clone());
    let assertion = serde_json::json!({
        "label": "cawg.training-and-data-mining",
        "version": "1.0-hidden-shield-draft",
        "data": {
            "watermarkUid": rights.watermark_uid,
            "trainingPolicy": policy,
            "trainingPolicyLabel": rights.training_permission.label,
            "policySource": rights.training_permission.source,
            "effectiveSource": rights.training_permission.effective_source,
            "legalConclusion": false,
            "customTermsUrl": custom_terms_url,
            "customTermsHash": custom_terms_hash
        }
    });
    let actions_assertion = serde_json::json!({
        "label": "org.contentauthenticity.actions",
        "data": {
            "workSourceDeclaration": work_source,
            "creationMethodDeclaration": creation_method,
            "humanEditLevelDeclaration": human_edit_level,
            "authenticityClaimDeclaration": authenticity_claim
        }
    });
    let c2pa_assertions = vec![assertion.clone(), actions_assertion.clone()];
    let content_credentials = serde_json::json!({
        "format": "c2pa-manifest-store-json-sidecar",
        "embeddedInMedia": false,
        "claimGenerator": "HiddenShield feedback-backend",
        "claimVersion": 1,
        "watermarkUid": rights.watermark_uid,
        "rightsManifestId": manifest_id,
        "registry": {
            "anchorProtocol": rights.registry.anchor_protocol,
            "mediaPayloadRole": rights.registry.media_payload_role,
            "registryProofHash": rights.registry.registry_proof_hash,
            "payloadAuthStatus": rights.registry.payload_auth_status
        }
    });
    let signed_manifest_store = build_signed_public_rights_manifest_store(
        rights,
        generated_at,
        &manifest_hash,
        &content_credentials,
        &c2pa_assertions,
    );
    PublicRightsMetadataExport {
        watermark_uid: rights.watermark_uid.clone(),
        export_version: 1,
        generated_at,
        legal_conclusion: false,
        boundary: "creator_declaration_registry_snapshot_not_legal_advice".to_string(),
        manifest_hash: manifest_hash.clone(),
        content_credentials,
        signed_manifest_store,
        c2pa_assertions,
        iptc: serde_json::json!({
            "schema": "IPTC Photo Metadata / PLUS Data Mining sidecar draft",
            "digitalSourceType": work_source,
            "dataMining": iptc_data_mining_value(policy),
            "dataMiningConstraint": policy,
            "licensorStatement": rights.training_permission.label,
            "webStatementOfRights": custom_terms_url,
            "customTermsHash": custom_terms_hash
        }),
        xmp: serde_json::json!({
            "xmpRights:Marked": true,
            "xmpRights:WebStatement": custom_terms_url,
            "hiddenShield:WatermarkUid": rights.watermark_uid,
            "hiddenShield:TrainingPolicy": policy,
            "hiddenShield:TrainingPolicyLabel": rights.training_permission.label,
            "hiddenShield:RightsManifestHash": manifest_hash,
            "hiddenShield:LegalConclusion": false
        }),
        json_ld: serde_json::json!({
            "@context": {
                "schema": "https://schema.org/",
                "hs": "https://hiddenshield.local/ns#",
                "tdm": "https://www.w3.org/ns/tdmrep#"
            },
            "@type": "schema:CreativeWork",
            "schema:identifier": rights.watermark_uid,
            "hs:rightsManifestId": manifest_id,
            "hs:trainingPolicy": policy,
            "hs:trainingPolicyLabel": rights.training_permission.label,
            "hs:legalConclusion": false,
            "hs:scanStatus": rights.scan_status,
            "hs:anchorProtocol": rights.registry.anchor_protocol,
            "hs:workSourceDeclaration": work_source,
            "hs:creationMethodDeclaration": creation_method,
            "hs:humanEditLevelDeclaration": human_edit_level,
            "hs:authenticityClaimDeclaration": authenticity_claim,
            "schema:license": custom_terms_url,
            "hs:customTermsHash": custom_terms_hash
        }),
    }
}

fn build_signed_public_rights_manifest_store(
    rights: &PublicRightsQueryResponse,
    generated_at: chrono::DateTime<Utc>,
    manifest_hash: &str,
    content_credentials: &serde_json::Value,
    c2pa_assertions: &[serde_json::Value],
) -> PublicRightsSignedManifestStore {
    let manifest_store = serde_json::json!({
        "version": "1.0-hidden-shield-signed-manifest-store",
        "profile": "c2pa-compatible-public-rights-metadata",
        "generatedAt": generated_at,
        "watermarkUid": rights.watermark_uid,
        "manifestHash": manifest_hash,
        "legalConclusion": false,
        "contentCredentials": content_credentials,
        "assertions": c2pa_assertions,
        "registry": rights.registry,
        "trainingPermission": rights.training_permission,
        "warnings": rights.warnings,
    });
    let canonical = serde_json::to_string(&manifest_store).unwrap_or_else(|_| "{}".to_string());
    let manifest_store_hash = sha256_hex(canonical.as_bytes());
    let signing_secret = std::env::var("HIDDENSHIELD_C2PA_MANIFEST_SIGNING_SECRET")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "hidden-shield-local-c2pa-manifest-dev-secret".to_string());
    let mut mac =
        Hmac::<Sha256>::new_from_slice(signing_secret.as_bytes()).expect("HMAC accepts any key");
    mac.update(canonical.as_bytes());
    let signature = hex_string(&mac.finalize().into_bytes());
    let signed_by = std::env::var("HIDDENSHIELD_C2PA_MANIFEST_SIGNER")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "HiddenShield public-rights registry".to_string());
    PublicRightsSignedManifestStore {
        format: "hidden-shield-signed-c2pa-manifest-store-json".to_string(),
        profile: "c2pa-compatible-public-rights-metadata".to_string(),
        manifest_store,
        manifest_store_hash,
        signature_algorithm: "HMAC-SHA256".to_string(),
        signature,
        signed_by,
        verification_status: "signed_by_hiddenshield_registry_key".to_string(),
        legal_conclusion: false,
    }
}

fn load_watermark_original_hash_tx(
    conn: &rusqlite::Connection,
    watermark_uid: &str,
) -> Result<Option<String>, StorageError> {
    conn.query_row(
        "SELECT original_hash FROM watermark_id_registry WHERE watermark_uid = ?1",
        params![watermark_uid],
        |row| row.get::<_, Option<String>>(0),
    )
    .optional()
    .map(|value| value.flatten())
    .map_err(StorageError::from)
}

fn watermark_registry_owner_matches_tx(
    conn: &rusqlite::Connection,
    watermark_uid: &str,
    account_id: &str,
    workspace_id: &str,
) -> Result<bool, StorageError> {
    let owner = conn
        .query_row(
            "SELECT account_id, workspace_id FROM watermark_id_registry WHERE watermark_uid = ?1",
            params![watermark_uid],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()?;
    Ok(owner
        .map(|(stored_account, stored_workspace)| {
            stored_account == account_id && stored_workspace == workspace_id
        })
        .unwrap_or(false))
}

fn registry_response_from_row(
    row: WatermarkIdRegistryRow,
) -> Result<WatermarkIdRegistryResponse, StorageError> {
    let issued_at = parse_registry_time(&row.created_at)?;
    let updated_at = parse_registry_time(&row.updated_at)?;
    Ok(WatermarkIdRegistryResponse {
        registry_id: row.registry_id,
        watermark_uid: row.watermark_uid,
        watermark_id_issue_mode: row.watermark_id_issue_mode,
        registry_status: row.registry_status,
        registry_receipt: row.registry_receipt,
        registry_proof_hash: row.registry_proof_hash,
        payload_protocol_version: row.payload_protocol_version as u32,
        payload_bytes_length: row.payload_bytes_length as u32,
        parent_watermark_uid: row.parent_watermark_uid,
        revision: row.revision as u32,
        issued_at,
        updated_at,
    })
}

fn public_registry_snapshot_from_row(
    row: WatermarkIdRegistryRow,
) -> Result<PublicRightsRegistrySnapshot, StorageError> {
    Ok(PublicRightsRegistrySnapshot {
        registry_id: row.registry_id,
        watermark_uid: row.watermark_uid,
        registry_status: row.registry_status,
        registry_proof_hash: row.registry_proof_hash,
        registry_receipt: row.registry_receipt,
        payload_auth_status: "verified".to_string(),
        watermark_id_issue_mode: row.watermark_id_issue_mode,
        payload_protocol_version: row.payload_protocol_version as u32,
        payload_bytes_length: row.payload_bytes_length as u32,
        parent_watermark_uid: row.parent_watermark_uid,
        revision: row.revision as u32,
        anchor_protocol: if row.payload_protocol_version >= 3 {
            "v3_minimal_anchor".to_string()
        } else {
            "v2_migration_anchor".to_string()
        },
        media_payload_role: if row.payload_protocol_version >= 3 {
            "minimal_media_anchor".to_string()
        } else {
            "legacy_bridge_anchor".to_string()
        },
        rights_source: "rights_registry".to_string(),
        issued_at: parse_registry_time(&row.created_at)?,
        updated_at: parse_registry_time(&row.updated_at)?,
    })
}

fn rights_manifest_response_from_row(
    row: RightsManifestRow,
) -> Result<RightsManifestResponse, StorageError> {
    Ok(RightsManifestResponse {
        rights_manifest_id: row.rights_manifest_id,
        watermark_uid: row.watermark_uid,
        manifest_version: row.manifest_version as u32,
        status: row.status,
        training_policy: row.training_policy,
        work_source_declaration: row.work_source_declaration,
        creation_method_declaration: row.creation_method_declaration,
        human_edit_level_declaration: row.human_edit_level_declaration,
        authenticity_claim_declaration: row.authenticity_claim_declaration,
        custom_terms_url: row.custom_terms_url,
        custom_terms_hash: row.custom_terms_hash,
        standard_mappings: serde_json::from_str(&row.standard_mappings_json)
            .unwrap_or_else(|_| default_standard_mappings()),
        manifest_sha256: row.manifest_sha256,
        signature: row.signature,
        signed_by: row.signed_by,
        effective_at: parse_registry_time(&row.effective_at)?,
        superseded_by: row.superseded_by_rights_manifest_id,
        revoked_at: row
            .revoked_at
            .as_deref()
            .map(parse_registry_time)
            .transpose()?,
        created_at: parse_registry_time(&row.created_at)?,
        updated_at: parse_registry_time(&row.updated_at)?,
    })
}

fn upsert_rights_manifest_from_sync_payload_tx(
    tx: &rusqlite::Transaction<'_>,
    account_id: &str,
    device_id: &str,
    entity_type: &str,
    payload_json: &str,
) -> Result<Option<String>, StorageError> {
    if entity_type != "vaultRecord" && entity_type != "evidenceRecord" {
        return Ok(None);
    }
    let payload: serde_json::Value =
        serde_json::from_str(payload_json).unwrap_or_else(|_| serde_json::json!({}));
    let Some(payload) = payload.as_object() else {
        return Ok(None);
    };
    let Some(watermark_uid) = payload_string(payload, "watermark_uid")
        .or_else(|| payload_string(payload, "watermarkUid"))
        .and_then(|uid| normalize_watermark_uid(&uid).ok())
    else {
        return Ok(None);
    };

    let Some(registry) = load_watermark_registry_by_uid_tx(tx, &watermark_uid)? else {
        return Ok(None);
    };
    if matches!(
        registry.registry_status.as_str(),
        "conflict" | "reissue_required" | "pending_registry_reconcile"
    ) {
        return create_disputed_rights_manifest_tx(
            tx,
            &registry,
            account_id,
            "registry_status_requires_review",
        )
        .map(Some);
    }
    let declaration = ManifestDeclaration::from_payload(payload);
    create_or_replace_active_rights_manifest_tx(tx, &registry, account_id, device_id, &declaration)
        .map(Some)
}

fn create_or_replace_active_rights_manifest_tx(
    tx: &rusqlite::Connection,
    registry: &WatermarkIdRegistryRow,
    account_id: &str,
    device_id: &str,
    declaration: &ManifestDeclaration,
) -> Result<String, StorageError> {
    let existing = load_active_rights_manifest_tx(tx, &registry.watermark_uid)?;
    if let Some(existing) = existing {
        if existing.training_policy == declaration.training_policy
            && existing.work_source_declaration == declaration.work_source_declaration
            && existing.creation_method_declaration == declaration.creation_method_declaration
            && existing.human_edit_level_declaration == declaration.human_edit_level_declaration
            && existing.authenticity_claim_declaration == declaration.authenticity_claim_declaration
        {
            return Ok(existing.rights_manifest_id);
        }
        let now = Utc::now().to_rfc3339();
        tx.execute(
            "UPDATE rights_manifests
             SET status = 'superseded',
                 superseded_by_rights_manifest_id = ?2,
                 updated_at = ?3
             WHERE rights_manifest_id = ?1",
            params![
                existing.rights_manifest_id,
                next_rights_manifest_id(&registry.watermark_uid, existing.manifest_version + 1),
                now
            ],
        )?;
    }

    let manifest_version = next_rights_manifest_version_tx(tx, &registry.watermark_uid)?;
    let rights_manifest_id = next_rights_manifest_id(&registry.watermark_uid, manifest_version);
    insert_rights_manifest_tx(
        tx,
        registry,
        &rights_manifest_id,
        manifest_version,
        "active",
        account_id,
        device_id,
        declaration,
        None,
    )?;
    Ok(rights_manifest_id)
}

fn create_disputed_rights_manifest_tx(
    tx: &rusqlite::Connection,
    registry: &WatermarkIdRegistryRow,
    account_id: &str,
    reason: &str,
) -> Result<String, StorageError> {
    if let Some(existing) = load_active_rights_manifest_tx(tx, &registry.watermark_uid)? {
        return Ok(existing.rights_manifest_id);
    }
    let existing_disputed = tx
        .query_row(
            "SELECT rights_manifest_id, watermark_uid, manifest_version, status,
                    training_policy, work_source_declaration, creation_method_declaration,
                    human_edit_level_declaration, authenticity_claim_declaration,
                    custom_terms_url, custom_terms_hash, standard_mappings_json,
                    manifest_sha256, signature, signed_by, effective_at,
                    superseded_by_rights_manifest_id, revoked_at, created_at, updated_at
             FROM rights_manifests
             WHERE watermark_uid = ?1 AND status = 'disputed'
             ORDER BY manifest_version DESC LIMIT 1",
            params![registry.watermark_uid],
            rights_manifest_row_from_sql,
        )
        .optional()?;
    if let Some(existing) = existing_disputed {
        return Ok(existing.rights_manifest_id);
    }
    let declaration = ManifestDeclaration {
        training_policy: "not_declared".to_string(),
        work_source_declaration: "unspecified".to_string(),
        creation_method_declaration: "unspecified".to_string(),
        human_edit_level_declaration: "unspecified".to_string(),
        authenticity_claim_declaration: "unspecified".to_string(),
        custom_terms_url: None,
        custom_terms_hash: None,
        standard_mappings: serde_json::json!({
            "c2pa": {"status": "not_declared"},
            "iptc": {"status": "not_declared"},
            "xmp": {"status": "not_declared"},
            "backfill": {"status": "disputed", "reason": reason}
        }),
    };
    let manifest_version = next_rights_manifest_version_tx(tx, &registry.watermark_uid)?;
    let rights_manifest_id = next_rights_manifest_id(&registry.watermark_uid, manifest_version);
    insert_rights_manifest_tx(
        tx,
        registry,
        &rights_manifest_id,
        manifest_version,
        "disputed",
        account_id,
        "backfill",
        &declaration,
        None,
    )?;
    Ok(rights_manifest_id)
}

fn insert_rights_manifest_tx(
    tx: &rusqlite::Connection,
    registry: &WatermarkIdRegistryRow,
    rights_manifest_id: &str,
    manifest_version: i64,
    status: &str,
    account_id: &str,
    device_id: &str,
    declaration: &ManifestDeclaration,
    revoked_at: Option<&str>,
) -> Result<(), StorageError> {
    let now = Utc::now().to_rfc3339();
    let canonical = canonical_rights_manifest_json(
        rights_manifest_id,
        &registry.watermark_uid,
        manifest_version,
        status,
        declaration,
    );
    let manifest_sha256 = sha256_hex(canonical.as_bytes());
    let signature = format!(
        "hs_registry_mock_sig:{}",
        short_id(&format!(
            "{rights_manifest_id}{manifest_sha256}{account_id}"
        ))
    );
    let standard_mappings_json =
        serde_json::to_string(&declaration.standard_mappings).unwrap_or_else(|_| "{}".to_string());
    tx.execute(
        "INSERT INTO rights_manifests (
            id, rights_manifest_id, watermark_uid, manifest_version, status,
            training_policy, work_source_declaration, creation_method_declaration,
            human_edit_level_declaration, authenticity_claim_declaration,
            custom_terms_url, custom_terms_hash, standard_mappings_json,
            manifest_sha256, signed_by, signature, effective_at,
            superseded_by_rights_manifest_id, revoked_at, created_at, updated_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17,
                   NULL, ?18, ?19, ?20)",
        params![
            format!("rights_manifest_{}", short_id(rights_manifest_id)),
            rights_manifest_id,
            registry.watermark_uid,
            manifest_version,
            status,
            declaration.training_policy,
            declaration.work_source_declaration,
            declaration.creation_method_declaration,
            declaration.human_edit_level_declaration,
            declaration.authenticity_claim_declaration,
            declaration.custom_terms_url,
            declaration.custom_terms_hash,
            standard_mappings_json,
            manifest_sha256,
            format!("account:{account_id}:device:{device_id}"),
            signature,
            now,
            revoked_at,
            now,
            now,
        ],
    )?;
    Ok(())
}

fn parse_registry_time(value: &str) -> Result<chrono::DateTime<Utc>, StorageError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| StorageError::BadRequest("registry_time_invalid".to_string()))
}

#[derive(Debug, Clone)]
struct ManifestDeclaration {
    training_policy: String,
    work_source_declaration: String,
    creation_method_declaration: String,
    human_edit_level_declaration: String,
    authenticity_claim_declaration: String,
    custom_terms_url: Option<String>,
    custom_terms_hash: Option<String>,
    standard_mappings: serde_json::Value,
}

impl ManifestDeclaration {
    fn from_payload(payload: &serde_json::Map<String, serde_json::Value>) -> Self {
        let training_declaration = payload_string(payload, "training_permission_declaration")
            .unwrap_or_else(|| "prohibited".to_string());
        let training_policy = map_training_policy(&training_declaration);
        let custom_terms_url = payload_string(payload, "custom_terms_url");
        let custom_terms_hash = payload_string(payload, "custom_terms_hash");
        Self {
            training_policy: training_policy.to_string(),
            work_source_declaration: payload_string(payload, "work_source_declaration")
                .unwrap_or_else(|| "unspecified".to_string()),
            creation_method_declaration: payload_string(payload, "creation_method_declaration")
                .unwrap_or_else(|| "unspecified".to_string()),
            human_edit_level_declaration: payload_string(payload, "human_edit_level_declaration")
                .unwrap_or_else(|| "unspecified".to_string()),
            authenticity_claim_declaration: payload_string(
                payload,
                "authenticity_claim_declaration",
            )
            .unwrap_or_else(|| "unspecified".to_string()),
            custom_terms_url,
            custom_terms_hash,
            standard_mappings: standard_mappings_for_policy(training_policy),
        }
    }

    fn from_registry_only() -> Self {
        Self {
            training_policy: "not_declared".to_string(),
            work_source_declaration: "unspecified".to_string(),
            creation_method_declaration: "unspecified".to_string(),
            human_edit_level_declaration: "unspecified".to_string(),
            authenticity_claim_declaration: "unspecified".to_string(),
            custom_terms_url: None,
            custom_terms_hash: None,
            standard_mappings: default_standard_mappings(),
        }
    }
}

enum BackfillOutcome {
    Succeeded {
        rights_manifest_id: String,
        manifest_version: u32,
        message: String,
    },
    NeedsReview {
        code: String,
        message: String,
    },
    Retryable {
        code: String,
        message: String,
    },
}

fn list_backfill_watermark_uids_tx(
    tx: &rusqlite::Transaction<'_>,
    cursor: Option<&str>,
    limit: u32,
) -> Result<Vec<String>, StorageError> {
    let cursor = cursor.unwrap_or_default().trim();
    let mut stmt = tx.prepare(
        "SELECT r.watermark_uid
         FROM watermark_id_registry r
         LEFT JOIN rights_manifests m ON m.watermark_uid = r.watermark_uid
         WHERE m.watermark_uid IS NULL
           AND (?1 = '' OR r.watermark_uid > ?1)
         ORDER BY r.watermark_uid ASC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![cursor, limit as i64], |row| row.get::<_, String>(0))?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StorageError::from)
}

fn backfill_rights_manifest_for_uid_tx(
    tx: &rusqlite::Transaction<'_>,
    watermark_uid: &str,
) -> Result<BackfillOutcome, StorageError> {
    let Some(registry) = load_watermark_registry_by_uid_tx(tx, watermark_uid)? else {
        return Ok(BackfillOutcome::Retryable {
            code: "watermark_registry_missing".to_string(),
            message: "registry row missing".to_string(),
        });
    };
    if matches!(
        registry.registry_status.as_str(),
        "conflict" | "reissue_required" | "pending_registry_reconcile"
    ) {
        let rights_manifest_id = create_disputed_rights_manifest_tx(
            tx,
            &registry,
            "system",
            "registry_status_requires_review",
        )?;
        return Ok(BackfillOutcome::NeedsReview {
            code: "backfill_disputed".to_string(),
            message: format!("created disputed manifest {rights_manifest_id}"),
        });
    }
    if let Some(existing) = load_active_rights_manifest_tx(tx, watermark_uid)? {
        return Ok(BackfillOutcome::Succeeded {
            rights_manifest_id: existing.rights_manifest_id,
            manifest_version: existing.manifest_version as u32,
            message: "active manifest already exists".to_string(),
        });
    }
    let declaration = latest_declaration_from_cloud_events_tx(tx, watermark_uid)?
        .unwrap_or_else(ManifestDeclaration::from_registry_only);
    let rights_manifest_id = create_or_replace_active_rights_manifest_tx(
        tx,
        &registry,
        "system",
        "backfill",
        &declaration,
    )?;
    let manifest_version = load_active_rights_manifest_tx(tx, watermark_uid)?
        .map(|row| row.manifest_version as u32)
        .unwrap_or(1);
    Ok(BackfillOutcome::Succeeded {
        rights_manifest_id,
        manifest_version,
        message: "active manifest backfilled".to_string(),
    })
}

fn latest_declaration_from_cloud_events_tx(
    tx: &rusqlite::Transaction<'_>,
    watermark_uid: &str,
) -> Result<Option<ManifestDeclaration>, StorageError> {
    let mut stmt = tx.prepare(
        "SELECT payload_json
         FROM cloud_sync_events
         WHERE entity_type IN ('vaultRecord', 'evidenceRecord')
           AND payload_json LIKE ?1
         ORDER BY sequence DESC
         LIMIT 20",
    )?;
    let like = format!("%{watermark_uid}%");
    let rows = stmt.query_map(params![like], |row| row.get::<_, String>(0))?;
    for row in rows {
        let payload_json = row?;
        let payload: serde_json::Value =
            serde_json::from_str(&payload_json).unwrap_or_else(|_| serde_json::json!({}));
        let Some(payload) = payload.as_object() else {
            continue;
        };
        let uid = payload_string(payload, "watermark_uid")
            .or_else(|| payload_string(payload, "watermarkUid"));
        if uid
            .and_then(|uid| normalize_watermark_uid(&uid).ok())
            .as_deref()
            == Some(watermark_uid)
        {
            return Ok(Some(ManifestDeclaration::from_payload(payload)));
        }
    }
    Ok(None)
}

fn next_rights_manifest_version_tx(
    conn: &rusqlite::Connection,
    watermark_uid: &str,
) -> Result<i64, StorageError> {
    let max_version = conn
        .query_row(
            "SELECT MAX(manifest_version) FROM rights_manifests WHERE watermark_uid = ?1",
            params![watermark_uid],
            |row| row.get::<_, Option<i64>>(0),
        )?
        .unwrap_or(0);
    Ok(max_version + 1)
}

fn next_rights_manifest_id(watermark_uid: &str, manifest_version: i64) -> String {
    format!(
        "rmf_{}_v{}",
        short_id(&format!("{watermark_uid}:{manifest_version}")),
        manifest_version
    )
}

fn canonical_rights_manifest_json(
    rights_manifest_id: &str,
    watermark_uid: &str,
    manifest_version: i64,
    status: &str,
    declaration: &ManifestDeclaration,
) -> String {
    serde_json::json!({
        "rightsManifestId": rights_manifest_id,
        "watermarkUid": watermark_uid,
        "manifestVersion": manifest_version,
        "status": status,
        "trainingPolicy": declaration.training_policy,
        "workSourceDeclaration": declaration.work_source_declaration,
        "creationMethodDeclaration": declaration.creation_method_declaration,
        "humanEditLevelDeclaration": declaration.human_edit_level_declaration,
        "authenticityClaimDeclaration": declaration.authenticity_claim_declaration,
        "customTermsUrl": declaration.custom_terms_url,
        "customTermsHash": declaration.custom_terms_hash,
    })
    .to_string()
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn payload_string(
    payload: &serde_json::Map<String, serde_json::Value>,
    key: &str,
) -> Option<String> {
    payload
        .get(key)
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn map_training_policy(value: &str) -> &'static str {
    match value.trim() {
        "unspecified" | "not_declared" => "not_declared",
        "separate_authorization_required" | "separate_license_required" => {
            "separate_license_required"
        }
        "non_commercial_allowed" | "non_commercial_research_allowed" => {
            "non_commercial_research_allowed"
        }
        "commercial_allowed" | "commercial_training_allowed" => "commercial_training_allowed",
        "public_domain" | "public_domain_or_unrestricted" => "public_domain_or_unrestricted",
        "custom_terms" => "custom_terms",
        _ => "no_ai_training",
    }
}

fn training_policy_label(value: &str) -> &'static str {
    match value {
        "not_declared" => "未声明训练许可",
        "separate_license_required" => "需单独授权",
        "non_commercial_research_allowed" => "允许非商业研究训练",
        "commercial_training_allowed" => "允许商业训练",
        "public_domain_or_unrestricted" => "公共领域或不限制",
        "custom_terms" => "自定义条款",
        _ => "禁止 AI / ML 训练",
    }
}

fn iptc_data_mining_value(policy: &str) -> &'static str {
    match policy {
        "commercial_training_allowed" => "allowed-commercial",
        "non_commercial_research_allowed" => "allowed-non-commercial",
        "separate_license_required" => "separate-license-required",
        "public_domain_or_unrestricted" => "allowed-unrestricted",
        "custom_terms" => "custom-terms",
        "not_declared" => "not-declared",
        _ => "prohibited",
    }
}

fn default_standard_mappings() -> serde_json::Value {
    serde_json::json!({
        "c2pa": {"status": "not_present"},
        "iptc": {"status": "not_present"},
        "xmp": {"status": "not_present"}
    })
}

fn standard_mappings_for_policy(policy: &str) -> serde_json::Value {
    serde_json::json!({
        "c2pa": {
            "assertion": "cawg.training-mining",
            "trainingPolicy": policy
        },
        "iptc": {
            "dataMining": policy
        },
        "xmp": {
            "hiddenShield:trainingPolicy": policy
        }
    })
}

fn public_scan_status(
    registry: &WatermarkIdRegistryRow,
    manifest: Option<&RightsManifestRow>,
) -> String {
    if matches!(
        registry.registry_status.as_str(),
        "conflict" | "reissue_required"
    ) {
        return "backfill_disputed".to_string();
    }
    match manifest.map(|row| row.status.as_str()) {
        Some("active") => "registry_active".to_string(),
        Some("revoked") => "registry_revoked".to_string(),
        Some("superseded") => "registry_superseded".to_string(),
        Some("disputed") => "backfill_disputed".to_string(),
        _ => "watermark_only".to_string(),
    }
}

fn public_rights_warnings(
    registry: &WatermarkIdRegistryRow,
    manifest: Option<&RightsManifestRow>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if manifest.is_none() {
        warnings.push("backfill_pending".to_string());
    }
    if matches!(
        registry.registry_status.as_str(),
        "conflict" | "reissue_required"
    ) {
        warnings.push("registry_requires_human_review".to_string());
    }
    warnings
}

fn error_code_for_storage_error(error: &StorageError) -> &'static str {
    match error {
        StorageError::Unauthorized => "unauthorized",
        StorageError::Forbidden => "forbidden",
        StorageError::BadRequest(_) => "bad_request",
        StorageError::RateLimited(_) => "rate_limited",
        StorageError::Database(_) => "internal_error",
        StorageError::DatabaseConfig(_) => "internal_error",
        #[cfg(feature = "postgres")]
        StorageError::PostgresDatabase(_) => "internal_error",
        StorageError::PostgresAdapterNotImplemented => "internal_error",
        StorageError::InvalidRetentionDays => "internal_error",
    }
}

fn gateway_decision_error(error_code: Option<&str>) -> StorageError {
    match error_code.unwrap_or("internal_error") {
        "api_key_missing" | "api_key_invalid" => StorageError::Unauthorized,
        "api_key_paused"
        | "api_key_revoked"
        | "api_key_expired"
        | "scope_denied"
        | "api_access_disabled"
        | "quota_contract_missing" => StorageError::Forbidden,
        "rate_limited" => StorageError::RateLimited("rate_limited".to_string()),
        "quota_exhausted" => StorageError::BadRequest("quota_exhausted".to_string()),
        other => StorageError::BadRequest(other.to_string()),
    }
}

fn validate_payload_protocol(version: u32, bytes: u32) -> Result<(), StorageError> {
    if version == 2 && bytes == 119 {
        return Ok(());
    }
    if version == 3 && (33..=64).contains(&bytes) {
        return Ok(());
    }
    if version == 3 {
        return Err(StorageError::BadRequest(
            "payload_protocol_v3_anchor_size_invalid".to_string(),
        ));
    }
    Err(StorageError::BadRequest(
        "payload_protocol_v2_or_v3_required".to_string(),
    ))
}

fn validate_revision(
    parent_watermark_uid: Option<&str>,
    revision: u32,
) -> Result<(), StorageError> {
    if revision == 0 {
        return Err(StorageError::BadRequest("revision_invalid".to_string()));
    }
    if normalize_optional_string(parent_watermark_uid).is_some() && revision == 1 {
        return Err(StorageError::BadRequest(
            "parent_watermark_requires_revision_gt_1".to_string(),
        ));
    }
    Ok(())
}

fn normalize_media_type(value: &str) -> Result<String, StorageError> {
    let value = value.trim().to_lowercase();
    match value.as_str() {
        "image" | "audio" | "video_audio_track" | "video_visual" => Ok(value),
        _ => Err(StorageError::BadRequest("media_type_invalid".to_string())),
    }
}

fn require_non_empty<'a>(value: &'a str, field: &str) -> Result<&'a str, StorageError> {
    let value = value.trim();
    if value.is_empty() {
        return Err(StorageError::BadRequest(format!("{field} is required")));
    }
    Ok(value)
}

fn normalize_optional_string(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_watermark_uid(value: &str) -> Result<String, StorageError> {
    let value = value.trim().to_uppercase();
    let parts = value.split('-').collect::<Vec<_>>();
    let valid = parts.len() == 5
        && parts[0] == "HS"
        && parts[1..]
            .iter()
            .all(|part| part.len() == 8 && part.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if !valid {
        return Err(StorageError::BadRequest(
            "watermark_uid_invalid".to_string(),
        ));
    }
    Ok(value)
}

fn generate_watermark_uid() -> String {
    let mut bytes = [0_u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    if bytes == [0_u8; 16] {
        bytes[15] = 1;
    }
    format!(
        "HS-{}-{}-{}-{}",
        hex_upper(&bytes[0..4]),
        hex_upper(&bytes[4..8]),
        hex_upper(&bytes[8..12]),
        hex_upper(&bytes[12..16])
    )
}

fn build_registry_receipt(registry_id: &str, watermark_uid: &str, status: &str) -> String {
    let now = Utc::now().to_rfc3339();
    let proof = registry_proof_hash(&format!("{registry_id}:{watermark_uid}:{status}:{now}"));
    format!("hsreg:v1:{registry_id}:{watermark_uid}:{status}:{proof}")
}

fn registry_proof_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    hex_string(&digest[..16])
}

fn hex_upper(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02X}"));
    }
    out
}

fn validate_l2_notary_request(request: &VideoFingerprintNotaryRequest) -> Result<(), StorageError> {
    if request.crop_window_fingerprint_root.trim().is_empty() || request.crop_window_count == 0 {
        return Err(StorageError::BadRequest(
            "crop_windows_required".to_string(),
        ));
    }
    if request.fingerprint_root.trim().is_empty()
        || request.local_block_fingerprint_root.trim().is_empty()
    {
        return Err(StorageError::BadRequest(
            "fingerprint_root_invalid".to_string(),
        ));
    }
    if request.client_signature.trim().is_empty() {
        return Err(StorageError::BadRequest(
            "client_signature_invalid".to_string(),
        ));
    }
    if request.upload_manifest.contains_original_video {
        return Err(StorageError::BadRequest(
            "original_video_forbidden".to_string(),
        ));
    }
    if request.upload_manifest.contains_watermarked_video {
        return Err(StorageError::BadRequest(
            "watermarked_video_forbidden".to_string(),
        ));
    }
    if request.upload_manifest.contains_local_paths {
        return Err(StorageError::BadRequest("local_path_forbidden".to_string()));
    }
    if request.upload_manifest.schema_version.trim().is_empty()
        || request.upload_manifest.items.is_empty()
    {
        return Err(StorageError::BadRequest(
            "invalid_upload_manifest".to_string(),
        ));
    }
    Ok(())
}

fn is_cloud_video_task_status(status: &str) -> bool {
    matches!(
        status,
        CLOUD_VIDEO_TASK_STATUS_DRAFT
            | CLOUD_VIDEO_TASK_STATUS_QUEUED
            | CLOUD_VIDEO_TASK_STATUS_RUNNING
            | CLOUD_VIDEO_TASK_STATUS_WAITING_CLIENT_RENDER
            | CLOUD_VIDEO_TASK_STATUS_SELF_CHECKING
            | CLOUD_VIDEO_TASK_STATUS_SUCCEEDED
            | CLOUD_VIDEO_TASK_STATUS_FAILED
            | CLOUD_VIDEO_TASK_STATUS_CANCELED
            | CLOUD_VIDEO_TASK_STATUS_EXPIRED
    )
}

fn quota_units_for_duration_ms(duration_ms: u64) -> u64 {
    (duration_ms.saturating_add(59_999) / 60_000).max(1)
}

fn validate_cloud_video_task_request(request: &CloudVideoTaskRequest) -> Result<(), StorageError> {
    if request.schema_version.trim() != "cloud_video_task_v1" {
        return Err(StorageError::BadRequest(
            "cloud_video_task_schema_invalid".to_string(),
        ));
    }
    if request.duration_ms == 0 {
        return Err(StorageError::BadRequest("duration_ms_required".to_string()));
    }
    if request.target_profiles.is_empty() {
        return Err(StorageError::BadRequest(
            "target_profiles_required".to_string(),
        ));
    }
    if request.capability_level.trim() != CLOUD_VIDEO_TASK_CAPABILITY_HYBRID_VISUAL_WATERMARK {
        return Err(StorageError::BadRequest(
            "cloud_video_task_capability_invalid".to_string(),
        ));
    }
    if request.upload_manifest.contains_original_video {
        return Err(StorageError::BadRequest(
            "original_video_forbidden".to_string(),
        ));
    }
    if request.upload_manifest.contains_watermarked_video {
        return Err(StorageError::BadRequest(
            "watermarked_video_forbidden".to_string(),
        ));
    }
    if request.upload_manifest.contains_local_paths {
        return Err(StorageError::BadRequest("local_path_forbidden".to_string()));
    }
    if request.upload_manifest.schema_version.trim() != "video_upload_manifest_v1"
        || request.upload_manifest.items.is_empty()
    {
        return Err(StorageError::BadRequest(
            "invalid_upload_manifest".to_string(),
        ));
    }
    validate_l3_video_visual_upload_manifest_capacity(&request.upload_manifest)?;
    Ok(())
}

fn validate_l3_video_visual_upload_manifest_capacity(
    manifest: &VideoUploadManifest,
) -> Result<(), StorageError> {
    for item in &manifest.items {
        let kind = item.kind.trim();
        if kind != "l3_user_object_upload_proxy" && kind != "l3_controlled_upload_proxy" {
            continue;
        }
        let (Some(width), Some(height), Some(frame_count)) =
            (item.width, item.height, item.frame_count)
        else {
            continue;
        };
        if !l3_video_visual_declared_capacity_is_supported(width, height, frame_count) {
            return Err(StorageError::BadRequest(
                "l3_strategy_capacity_insufficient".to_string(),
            ));
        }
    }
    Ok(())
}

fn l3_video_visual_declared_capacity_is_supported(
    width: u32,
    height: u32,
    frame_count: u32,
) -> bool {
    if width < 512 || height < 512 || width % 8 != 0 || height % 8 != 0 || frame_count == 0 {
        return false;
    }
    let region_width = (width / 8).max(1);
    let region_height = (height / 8).max(1);
    let blocks_per_region = (region_width / 8).max(1) * (region_height / 8).max(1);
    let min_regions_per_strategy_frame = (L3_VIDEO_VISUAL_MAX_REGIONS / frame_count).max(1);
    let estimated_bits =
        blocks_per_region * min_regions_per_strategy_frame * L3_VIDEO_VISUAL_DCT_COEFF_PAIRS;
    let required_bits =
        L3_VIDEO_VISUAL_SYNC_BITS + L3_VIDEO_VISUAL_PAYLOAD_BYTES * 8 * L3_VIDEO_VISUAL_ECC_REPEAT;
    estimated_bits >= required_bits
}

fn validate_cloud_video_task_claim_request(
    request: &CloudVideoTaskClaimRequest,
) -> Result<(), StorageError> {
    if request.worker_id.trim().is_empty() {
        return Err(StorageError::BadRequest(
            "cloud_video_task_worker_id_required".to_string(),
        ));
    }
    if let Some(capability_level) = request.capability_level.as_deref() {
        let capability_level = capability_level.trim();
        if !capability_level.is_empty()
            && capability_level != CLOUD_VIDEO_TASK_CAPABILITY_HYBRID_VISUAL_WATERMARK
        {
            return Err(StorageError::BadRequest(
                "cloud_video_task_capability_invalid".to_string(),
            ));
        }
    }
    if request.lease_seconds == Some(0) {
        return Err(StorageError::BadRequest(
            "cloud_video_task_lease_seconds_invalid".to_string(),
        ));
    }
    Ok(())
}

fn validate_cloud_video_task_failure_request(
    request: &CloudVideoTaskFailureRequest,
) -> Result<(), StorageError> {
    if request.worker_id.trim().is_empty() {
        return Err(StorageError::BadRequest(
            "cloud_video_task_worker_id_required".to_string(),
        ));
    }
    if request.attempt_id.trim().is_empty() {
        return Err(StorageError::BadRequest(
            "cloud_video_task_attempt_id_required".to_string(),
        ));
    }
    if request.lease_token.trim().is_empty() {
        return Err(StorageError::BadRequest(
            "cloud_video_task_lease_token_required".to_string(),
        ));
    }
    let failure_code = request.failure_code.trim();
    if !CLOUD_VIDEO_TASK_FAILURE_CODES.contains(&failure_code) {
        return Err(StorageError::BadRequest(
            "cloud_video_task_failure_code_invalid".to_string(),
        ));
    }
    if request
        .failure_stage
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .len()
        > 96
    {
        return Err(StorageError::BadRequest(
            "cloud_video_task_failure_stage_invalid".to_string(),
        ));
    }
    if request
        .failure_message
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .len()
        > 512
    {
        return Err(StorageError::BadRequest(
            "cloud_video_task_failure_message_invalid".to_string(),
        ));
    }
    Ok(())
}

fn validate_cloud_video_task_completion_request(
    request: &CloudVideoTaskCompletionRequest,
) -> Result<(), StorageError> {
    if request.worker_id.trim().is_empty() {
        return Err(StorageError::BadRequest(
            "cloud_video_task_worker_id_required".to_string(),
        ));
    }
    if request.attempt_id.trim().is_empty() {
        return Err(StorageError::BadRequest(
            "cloud_video_task_attempt_id_required".to_string(),
        ));
    }
    if request.lease_token.trim().is_empty() {
        return Err(StorageError::BadRequest(
            "cloud_video_task_lease_token_required".to_string(),
        ));
    }
    let output_ref = request.output_media_storage_ref.trim();
    if !output_ref.starts_with("object://l3-output/") || looks_like_local_path(output_ref) {
        return Err(StorageError::BadRequest(
            "output_media_storage_ref_invalid".to_string(),
        ));
    }
    if output_ref.contains('\\') || output_ref.contains("..") {
        return Err(StorageError::BadRequest(
            "output_media_storage_ref_invalid".to_string(),
        ));
    }
    if request.output_media_bytes == 0 {
        return Err(StorageError::BadRequest(
            "output_media_bytes_required".to_string(),
        ));
    }
    if request.output_media_content_type.trim() != "video/mp4" {
        return Err(StorageError::BadRequest(
            "output_media_content_type_invalid".to_string(),
        ));
    }
    if !looks_like_sha256(request.worker_receipt_hash.trim()) {
        return Err(StorageError::BadRequest(
            "worker_receipt_hash_invalid".to_string(),
        ));
    }
    let receipt = request
        .worker_receipt
        .as_object()
        .ok_or_else(|| StorageError::BadRequest("worker_receipt_required".to_string()))?;
    if receipt.is_empty() {
        return Err(StorageError::BadRequest(
            "worker_receipt_required".to_string(),
        ));
    }
    let receipt_text = serde_json::to_string(&request.worker_receipt)
        .map_err(|error| StorageError::BadRequest(error.to_string()))?;
    let expected_hash = format!(
        "sha256:{}",
        hex_lower_storage(&Sha256::digest(receipt_text.as_bytes()))
    );
    if expected_hash != request.worker_receipt_hash.trim() {
        return Err(StorageError::BadRequest(
            "worker_receipt_hash_mismatch".to_string(),
        ));
    }
    Ok(())
}

fn validate_cloud_video_task_status_update(
    request: &CloudVideoTaskStatusUpdateRequest,
) -> Result<(), StorageError> {
    let status = request.status.trim();
    if !is_cloud_video_task_status(status) {
        return Err(StorageError::BadRequest(
            "cloud_video_task_status_invalid".to_string(),
        ));
    }
    if matches!(
        status,
        CLOUD_VIDEO_TASK_STATUS_FAILED
            | CLOUD_VIDEO_TASK_STATUS_CANCELED
            | CLOUD_VIDEO_TASK_STATUS_EXPIRED
    ) && request
        .failure_code
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        return Err(StorageError::BadRequest(
            "cloud_video_task_failure_code_required".to_string(),
        ));
    }
    if status == CLOUD_VIDEO_TASK_STATUS_SUCCEEDED {
        if request
            .strategy_digest
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            return Err(StorageError::BadRequest(
                "strategy_digest_required".to_string(),
            ));
        }
        let threshold = request
            .self_check_threshold
            .ok_or_else(|| StorageError::BadRequest("self_check_threshold_required".to_string()))?;
        if !(0.0..=1.0).contains(&threshold) {
            return Err(StorageError::BadRequest(
                "self_check_threshold_invalid".to_string(),
            ));
        }
        let confidence = request.self_check_confidence.ok_or_else(|| {
            StorageError::BadRequest("self_check_confidence_required".to_string())
        })?;
        if !(0.0..=1.0).contains(&confidence) {
            return Err(StorageError::BadRequest(
                "self_check_confidence_invalid".to_string(),
            ));
        }
        if confidence < threshold {
            return Err(StorageError::BadRequest(
                "self_check_confidence_below_threshold".to_string(),
            ));
        }
        if request.checked_frames.unwrap_or_default() == 0 {
            return Err(StorageError::BadRequest(
                "checked_frames_required".to_string(),
            ));
        }
        if request
            .watermarked_media_hash
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            return Err(StorageError::BadRequest(
                "watermarked_media_hash_required".to_string(),
            ));
        }
        if request
            .server_receipt_signature
            .as_deref()
            .map(str::trim)
            .unwrap_or_default()
            .is_empty()
        {
            return Err(StorageError::BadRequest(
                "server_receipt_signature_required".to_string(),
            ));
        }
    }
    Ok(())
}

fn billing_event_from_fixture_request(
    request: &BillingFixtureEventRequest,
) -> Result<BillingEvent, StorageError> {
    let event_type = match request.event_type.trim() {
        "payment.succeeded" => BillingEventType::PaymentSucceeded,
        "subscription.renewed" => BillingEventType::SubscriptionRenewed,
        "payment.failed" => BillingEventType::PaymentFailed,
        "subscription.canceled" => BillingEventType::SubscriptionCanceled,
        "subscription.expired" => BillingEventType::SubscriptionExpired,
        "refund.succeeded" => BillingEventType::RefundSucceeded,
        _ => {
            return Err(StorageError::BadRequest(
                "billing_event_type_invalid".to_string(),
            ))
        }
    };
    Ok(BillingEvent {
        provider: FIXTURE_PROVIDER.to_string(),
        provider_event_id: required_trimmed(
            &request.provider_event_id,
            "provider_event_id_required",
        )?,
        provider_order_id: required_trimmed(
            &request.provider_order_id,
            "provider_order_id_required",
        )?,
        provider_transaction_id: request
            .provider_transaction_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned),
        account_id: required_trimmed(&request.account_id, "account_id_required")?,
        workspace_id: required_trimmed(&request.workspace_id, "workspace_id_required")?,
        plan_code: normalize_plan_code(&request.plan_code)?,
        billing_cycle: normalize_billing_cycle(&request.billing_cycle)?,
        amount_cents: request.amount_cents,
        currency: required_trimmed(&request.currency, "currency_required")?,
        event_type,
        occurred_at: request.occurred_at,
        raw_payload_json: request.raw_payload_json.to_string(),
    })
}

fn billing_event_for_order_status(status: &BillingOrderStatus) -> Option<BillingEvent> {
    let event_type = match status.status {
        BillingOrderStatusKind::Succeeded => BillingEventType::PaymentSucceeded,
        BillingOrderStatusKind::Refunded => BillingEventType::RefundSucceeded,
        _ => return None,
    };
    Some(BillingEvent {
        provider: status.provider.clone(),
        provider_event_id: format!(
            "order_query_{}_{}",
            status.provider,
            short_id(&format!(
                "{}:{}:{}",
                status.provider_order_id,
                status
                    .provider_transaction_id
                    .as_deref()
                    .unwrap_or_default(),
                event_type.as_str()
            ))
        ),
        provider_order_id: status.provider_order_id.clone(),
        provider_transaction_id: status.provider_transaction_id.clone(),
        account_id: status.account_id.clone(),
        workspace_id: status.workspace_id.clone(),
        plan_code: status.plan_code.clone(),
        billing_cycle: status.billing_cycle.clone(),
        amount_cents: status.amount_cents,
        currency: status.currency.clone(),
        event_type,
        occurred_at: status.paid_at.unwrap_or_else(Utc::now),
        raw_payload_json: status.raw_payload_json.clone(),
    })
}

fn report_purchase_event_for_order_status(
    status: &ReportPurchaseOrderStatus,
) -> Option<ReportPurchaseEvent> {
    let event_type = match status.status {
        BillingOrderStatusKind::Succeeded => ReportPurchaseEventType::PaymentSucceeded,
        BillingOrderStatusKind::Refunded => ReportPurchaseEventType::RefundSucceeded,
        _ => return None,
    };
    Some(ReportPurchaseEvent {
        provider: status.provider.clone(),
        provider_event_id: format!(
            "order_query_{}_{}",
            status.provider,
            short_id(&format!(
                "{}:{}:{}",
                status.provider_order_id,
                status
                    .provider_transaction_id
                    .as_deref()
                    .unwrap_or_default(),
                event_type.as_str()
            ))
        ),
        provider_order_id: status.provider_order_id.clone(),
        provider_transaction_id: status.provider_transaction_id.clone(),
        account_id: status.account_id.clone(),
        workspace_id: status.workspace_id.clone(),
        creator_profile_id: status.creator_profile_id.clone(),
        vault_record_id: status.vault_record_id.clone(),
        product_code: status.product_code.clone(),
        price_cents: status.price_cents,
        currency: status.currency.clone(),
        event_type,
        occurred_at: status.paid_at.unwrap_or_else(Utc::now),
        raw_payload_json: status.raw_payload_json.clone(),
    })
}

fn validate_report_purchase_order_status_matches(
    payment: &ReportPurchaseSessionRecord,
    status: &ReportPurchaseOrderStatus,
) -> Result<(), StorageError> {
    if payment.provider != status.provider
        || payment.provider_order_id != status.provider_order_id
        || payment.account_id != status.account_id
        || payment.workspace_id != status.workspace_id
        || payment.creator_profile_id != status.creator_profile_id
        || payment.vault_record_id != status.vault_record_id
        || payment.product_code != status.product_code
        || payment.price_cents != status.price_cents
        || payment.currency != status.currency
    {
        return Err(StorageError::BadRequest(
            "report_purchase_order_status_mismatch".to_string(),
        ));
    }
    Ok(())
}

fn persist_billing_payment_session(
    conn: &rusqlite::Connection,
    session: &BillingPaymentSession,
    input: &BillingPaymentSessionInput,
    action: &BillingPaymentAction,
    amount_cents: i64,
    currency: &str,
    status: &str,
) -> Result<(), StorageError> {
    let now = Utc::now().to_rfc3339();
    let action_json = serde_json::to_string(action)
        .map_err(|error| StorageError::BadRequest(error.to_string()))?;
    conn.execute(
        "INSERT INTO billing_payment_sessions (
            payment_session_id, provider, provider_order_id, account_id, workspace_id,
            plan_code, billing_cycle, amount_cents, currency, status, payment_action_json,
            expires_at, last_provider_event_id, last_provider_transaction_id,
            last_checked_at, next_check_after, check_attempts, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, NULL, NULL, ?13, 0, ?14, ?15)
        ON CONFLICT(provider, provider_order_id) DO UPDATE SET
            payment_session_id = excluded.payment_session_id,
            account_id = excluded.account_id,
            workspace_id = excluded.workspace_id,
            plan_code = excluded.plan_code,
            billing_cycle = excluded.billing_cycle,
            amount_cents = excluded.amount_cents,
            currency = excluded.currency,
            status = excluded.status,
            payment_action_json = excluded.payment_action_json,
            expires_at = excluded.expires_at,
            updated_at = excluded.updated_at",
        params![
            session.payment_session_id,
            session.provider,
            session.provider_order_id,
            input.account_id,
            input.workspace_id,
            input.plan_code,
            input.billing_cycle,
            amount_cents,
            currency,
            status,
            action_json,
            session.expires_at.to_rfc3339(),
            now,
            now,
            now,
        ],
    )?;
    Ok(())
}

fn load_billing_payment_session(
    conn: &rusqlite::Connection,
    payment_session_id: &str,
) -> Result<BillingPaymentSessionRecord, StorageError> {
    conn.query_row(
        "SELECT payment_session_id, provider, provider_order_id, account_id, workspace_id,
                plan_code, billing_cycle, status, payment_action_json, expires_at,
                last_checked_at, next_check_after, check_attempts
         FROM billing_payment_sessions
         WHERE payment_session_id = ?1",
        params![payment_session_id.trim()],
        |row| {
            Ok(BillingPaymentSessionRecord {
                payment_session_id: row.get(0)?,
                provider: row.get(1)?,
                provider_order_id: row.get(2)?,
                account_id: row.get(3)?,
                workspace_id: row.get(4)?,
                plan_code: row.get(5)?,
                billing_cycle: row.get(6)?,
                status: row.get(7)?,
                payment_action_json: row.get(8)?,
                expires_at: row.get(9)?,
                last_checked_at: row.get(10)?,
                next_check_after: row.get(11)?,
                check_attempts: row.get(12)?,
            })
        },
    )
    .map_err(StorageError::from)
}

fn persist_report_purchase_session(
    conn: &rusqlite::Connection,
    session: &BillingPaymentSession,
    request: &ReportPurchaseSessionRequest,
    product_code: &str,
    price_cents: i64,
    action: &BillingPaymentAction,
    status: &str,
) -> Result<(), StorageError> {
    let now = Utc::now().to_rfc3339();
    let action_json = serde_json::to_string(action)
        .map_err(|error| StorageError::BadRequest(error.to_string()))?;
    conn.execute(
        "INSERT INTO report_purchase_sessions (
            payment_session_id, provider, provider_order_id, account_id, workspace_id,
            creator_profile_id, vault_record_id, product_code, price_cents, currency,
            status, payment_action_json, expires_at, last_provider_event_id,
            last_provider_transaction_id, last_checked_at, next_check_after,
            check_attempts, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, 'CNY', ?10, ?11, ?12, NULL, NULL, NULL, ?13, 0, ?14, ?15)
        ON CONFLICT(provider, provider_order_id) DO UPDATE SET
            payment_session_id = excluded.payment_session_id,
            account_id = excluded.account_id,
            workspace_id = excluded.workspace_id,
            creator_profile_id = excluded.creator_profile_id,
            vault_record_id = excluded.vault_record_id,
            product_code = excluded.product_code,
            price_cents = excluded.price_cents,
            currency = excluded.currency,
            status = excluded.status,
            payment_action_json = excluded.payment_action_json,
            expires_at = excluded.expires_at,
            updated_at = excluded.updated_at",
        params![
            session.payment_session_id,
            session.provider,
            session.provider_order_id,
            request.account_id.trim(),
            request.workspace_id.trim(),
            request.creator_profile_id.trim(),
            request.vault_record_id.trim(),
            product_code,
            price_cents,
            status,
            action_json,
            session.expires_at.to_rfc3339(),
            now,
            now,
            now,
        ],
    )?;
    Ok(())
}

fn load_report_purchase_session(
    conn: &rusqlite::Connection,
    payment_session_id: &str,
) -> Result<ReportPurchaseSessionRecord, StorageError> {
    conn.query_row(
        "SELECT payment_session_id, provider, provider_order_id, account_id, workspace_id,
                creator_profile_id, vault_record_id, product_code, price_cents, currency,
                status, payment_action_json, expires_at, last_checked_at, next_check_after,
                check_attempts
         FROM report_purchase_sessions
         WHERE payment_session_id = ?1",
        params![payment_session_id.trim()],
        |row| {
            Ok(ReportPurchaseSessionRecord {
                payment_session_id: row.get(0)?,
                provider: row.get(1)?,
                provider_order_id: row.get(2)?,
                account_id: row.get(3)?,
                workspace_id: row.get(4)?,
                creator_profile_id: row.get(5)?,
                vault_record_id: row.get(6)?,
                product_code: row.get(7)?,
                price_cents: row.get(8)?,
                currency: row.get(9)?,
                status: row.get(10)?,
                expires_at: row.get(12)?,
                last_checked_at: row.get(13)?,
                next_check_after: row.get(14)?,
                check_attempts: row.get(15)?,
            })
        },
    )
    .map_err(StorageError::from)
}

fn load_report_purchase_session_by_provider_tx(
    tx: &rusqlite::Transaction<'_>,
    provider: &str,
    provider_order_id: &str,
) -> Result<ReportPurchaseSessionRecord, StorageError> {
    tx.query_row(
        "SELECT payment_session_id, provider, provider_order_id, account_id, workspace_id,
                creator_profile_id, vault_record_id, product_code, price_cents, currency,
                status, payment_action_json, expires_at, last_checked_at, next_check_after,
                check_attempts
         FROM report_purchase_sessions
         WHERE provider = ?1 AND provider_order_id = ?2",
        params![provider.trim(), provider_order_id.trim()],
        |row| {
            Ok(ReportPurchaseSessionRecord {
                payment_session_id: row.get(0)?,
                provider: row.get(1)?,
                provider_order_id: row.get(2)?,
                account_id: row.get(3)?,
                workspace_id: row.get(4)?,
                creator_profile_id: row.get(5)?,
                vault_record_id: row.get(6)?,
                product_code: row.get(7)?,
                price_cents: row.get(8)?,
                currency: row.get(9)?,
                status: row.get(10)?,
                expires_at: row.get(12)?,
                last_checked_at: row.get(13)?,
                next_check_after: row.get(14)?,
                check_attempts: row.get(15)?,
            })
        },
    )
    .map_err(StorageError::from)
}

fn load_report_purchase_grant_for_session(
    conn: &rusqlite::Connection,
    payment_session_id: &str,
) -> Result<Option<ReportPurchaseGrant>, StorageError> {
    conn.query_row(
        "SELECT grant_id, account_id, workspace_id, creator_profile_id, vault_record_id,
                product_code, price_cents, currency, status, granted_at, revoked_at
         FROM report_purchase_grants
         WHERE payment_session_id = ?1 AND status = 'active'",
        params![payment_session_id.trim()],
        report_purchase_grant_from_row,
    )
    .optional()
    .map_err(StorageError::from)
}

fn report_purchase_provider_event_exists_tx(
    tx: &rusqlite::Transaction<'_>,
    event: &ReportPurchaseEvent,
) -> Result<bool, StorageError> {
    let existing: Option<String> = tx
        .query_row(
            "SELECT payment_session_id
             FROM report_purchase_sessions
             WHERE provider = ?1
               AND provider_order_id = ?2
               AND last_provider_event_id = ?3",
            params![
                event.provider.trim(),
                event.provider_order_id.trim(),
                event.provider_event_id.trim()
            ],
            |row| row.get(0),
        )
        .optional()?;
    Ok(existing.is_some())
}

fn upsert_report_purchase_grant_tx(
    tx: &rusqlite::Transaction<'_>,
    payment: &ReportPurchaseSessionRecord,
    provider_transaction_id: &str,
    provider_event_id: &str,
) -> Result<ReportPurchaseGrant, StorageError> {
    let now = Utc::now().to_rfc3339();
    let grant_id = format!(
        "rpt_grant_{}",
        short_id(&format!(
            "{}:{}:{}:{}",
            payment.account_id, payment.workspace_id, payment.vault_record_id, payment.product_code
        ))
    );
    tx.execute(
        "INSERT INTO report_purchase_grants (
            grant_id, account_id, workspace_id, creator_profile_id, vault_record_id,
            product_code, price_cents, currency, payment_session_id, provider,
            provider_order_id, status, granted_at, revoked_at, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'active', ?12, NULL, ?13, ?14)
        ON CONFLICT(account_id, workspace_id, vault_record_id, product_code) DO UPDATE SET
            creator_profile_id = excluded.creator_profile_id,
            price_cents = excluded.price_cents,
            currency = excluded.currency,
            payment_session_id = excluded.payment_session_id,
            provider = excluded.provider,
            provider_order_id = excluded.provider_order_id,
            status = 'active',
            granted_at = excluded.granted_at,
            revoked_at = NULL,
            updated_at = excluded.updated_at",
        params![
            grant_id,
            payment.account_id,
            payment.workspace_id,
            payment.creator_profile_id,
            payment.vault_record_id,
            payment.product_code,
            payment.price_cents,
            payment.currency,
            payment.payment_session_id,
            payment.provider,
            payment.provider_order_id,
            now,
            now,
            now,
        ],
    )?;
    let _ = (provider_transaction_id, provider_event_id);
    load_report_purchase_grant_for_session(tx, &payment.payment_session_id)?
        .ok_or_else(|| StorageError::BadRequest("report_purchase_grant_missing".to_string()))
}

fn revoke_report_purchase_grant_tx(
    tx: &rusqlite::Transaction<'_>,
    payment: &ReportPurchaseSessionRecord,
    event: &ReportPurchaseEvent,
) -> Result<(), StorageError> {
    let now = event.occurred_at.to_rfc3339();
    tx.execute(
        "UPDATE report_purchase_grants
         SET status = 'revoked',
             revoked_at = ?5,
             updated_at = ?5
         WHERE account_id = ?1
           AND workspace_id = ?2
           AND vault_record_id = ?3
           AND product_code = ?4",
        params![
            payment.account_id,
            payment.workspace_id,
            payment.vault_record_id,
            payment.product_code,
            now,
        ],
    )?;
    Ok(())
}

fn report_purchase_grant_from_row(
    row: &rusqlite::Row<'_>,
) -> rusqlite::Result<ReportPurchaseGrant> {
    let granted_at: String = row.get(9)?;
    let revoked_at: Option<String> = row.get(10)?;
    Ok(ReportPurchaseGrant {
        grant_id: row.get(0)?,
        account_id: row.get(1)?,
        workspace_id: row.get(2)?,
        creator_profile_id: row.get(3)?,
        vault_record_id: row.get(4)?,
        product_code: row.get(5)?,
        price_cents: row.get(6)?,
        currency: row.get(7)?,
        status: row.get(8)?,
        granted_at: chrono::DateTime::parse_from_rfc3339(&granted_at)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        revoked_at: revoked_at.and_then(|value| {
            chrono::DateTime::parse_from_rfc3339(&value)
                .map(|value| value.with_timezone(&Utc))
                .ok()
        }),
    })
}

fn load_due_billing_payment_sessions(
    conn: &rusqlite::Connection,
    now: chrono::DateTime<Utc>,
    limit: usize,
) -> Result<Vec<BillingPaymentSessionRecord>, StorageError> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT payment_session_id, provider, provider_order_id, account_id, workspace_id,
                plan_code, billing_cycle, status, payment_action_json, expires_at,
                last_checked_at, next_check_after, check_attempts
         FROM billing_payment_sessions
         WHERE status IN ('created', 'pending')
           AND expires_at > ?1
           AND (next_check_after IS NULL OR next_check_after <= ?1)
         ORDER BY COALESCE(next_check_after, updated_at) ASC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![now.to_rfc3339(), limit as i64], |row| {
        Ok(BillingPaymentSessionRecord {
            payment_session_id: row.get(0)?,
            provider: row.get(1)?,
            provider_order_id: row.get(2)?,
            account_id: row.get(3)?,
            workspace_id: row.get(4)?,
            plan_code: row.get(5)?,
            billing_cycle: row.get(6)?,
            status: row.get(7)?,
            payment_action_json: row.get(8)?,
            expires_at: row.get(9)?,
            last_checked_at: row.get(10)?,
            next_check_after: row.get(11)?,
            check_attempts: row.get(12)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StorageError::from)
}

fn load_due_report_purchase_sessions(
    conn: &rusqlite::Connection,
    now: chrono::DateTime<Utc>,
    limit: usize,
) -> Result<Vec<ReportPurchaseSessionRecord>, StorageError> {
    if limit == 0 {
        return Ok(Vec::new());
    }
    let mut stmt = conn.prepare(
        "SELECT payment_session_id, provider, provider_order_id, account_id, workspace_id,
                creator_profile_id, vault_record_id, product_code, price_cents, currency,
                status, payment_action_json, expires_at, last_checked_at, next_check_after,
                check_attempts
         FROM report_purchase_sessions
         WHERE status IN ('created', 'pending')
           AND expires_at > ?1
           AND (next_check_after IS NULL OR next_check_after <= ?1)
         ORDER BY COALESCE(next_check_after, updated_at) ASC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(params![now.to_rfc3339(), limit as i64], |row| {
        Ok(ReportPurchaseSessionRecord {
            payment_session_id: row.get(0)?,
            provider: row.get(1)?,
            provider_order_id: row.get(2)?,
            account_id: row.get(3)?,
            workspace_id: row.get(4)?,
            creator_profile_id: row.get(5)?,
            vault_record_id: row.get(6)?,
            product_code: row.get(7)?,
            price_cents: row.get(8)?,
            currency: row.get(9)?,
            status: row.get(10)?,
            expires_at: row.get(12)?,
            last_checked_at: row.get(13)?,
            next_check_after: row.get(14)?,
            check_attempts: row.get(15)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(StorageError::from)
}

fn mark_billing_payment_session_checked(
    conn: &rusqlite::Connection,
    payment_session_id: &str,
    status: &str,
    provider_transaction_id: Option<&str>,
    provider_event_id: Option<&str>,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let next_check_after =
        next_billing_payment_check_after(status, 1).map(|value| value.to_rfc3339());
    conn.execute(
        "UPDATE billing_payment_sessions
         SET status = ?2,
             last_provider_transaction_id = COALESCE(?3, last_provider_transaction_id),
             last_provider_event_id = COALESCE(?4, last_provider_event_id),
             last_checked_at = ?5,
             next_check_after = ?6,
             check_attempts = check_attempts + 1,
             updated_at = ?5
         WHERE payment_session_id = ?1",
        params![
            payment_session_id,
            status,
            provider_transaction_id,
            provider_event_id,
            now.to_rfc3339(),
            next_check_after,
        ],
    )?;
    Ok(())
}

fn mark_report_purchase_session_checked(
    conn: &rusqlite::Connection,
    payment_session_id: &str,
    status: &str,
    provider_transaction_id: Option<&str>,
    provider_event_id: Option<&str>,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let next_check_after =
        next_billing_payment_check_after(status, 1).map(|value| value.to_rfc3339());
    conn.execute(
        "UPDATE report_purchase_sessions
         SET status = ?2,
             last_provider_transaction_id = COALESCE(?3, last_provider_transaction_id),
             last_provider_event_id = COALESCE(?4, last_provider_event_id),
             last_checked_at = ?5,
             next_check_after = ?6,
             check_attempts = check_attempts + 1,
             updated_at = ?5
         WHERE payment_session_id = ?1",
        params![
            payment_session_id,
            status,
            provider_transaction_id,
            provider_event_id,
            now.to_rfc3339(),
            next_check_after,
        ],
    )?;
    Ok(())
}

fn defer_billing_payment_session_check(
    conn: &rusqlite::Connection,
    payment: &BillingPaymentSessionRecord,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let next_check_after =
        next_billing_payment_check_after(&payment.status, payment.check_attempts + 1)
            .map(|value| value.to_rfc3339());
    conn.execute(
        "UPDATE billing_payment_sessions
         SET last_checked_at = ?2,
             next_check_after = ?3,
             check_attempts = check_attempts + 1,
             updated_at = ?2
         WHERE payment_session_id = ?1",
        params![
            payment.payment_session_id,
            now.to_rfc3339(),
            next_check_after,
        ],
    )?;
    Ok(())
}

fn mark_billing_payment_session_checked_tx(
    tx: &rusqlite::Transaction<'_>,
    provider: &str,
    provider_order_id: &str,
    status: &str,
    provider_transaction_id: Option<&str>,
    provider_event_id: Option<&str>,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let next_check_after =
        next_billing_payment_check_after(status, 0).map(|value| value.to_rfc3339());
    tx.execute(
        "UPDATE billing_payment_sessions
         SET status = ?3,
             last_provider_transaction_id = COALESCE(?4, last_provider_transaction_id),
             last_provider_event_id = COALESCE(?5, last_provider_event_id),
             last_checked_at = ?6,
             next_check_after = ?7,
             updated_at = ?6
         WHERE provider = ?1 AND provider_order_id = ?2",
        params![
            provider,
            provider_order_id,
            status,
            provider_transaction_id,
            provider_event_id,
            now.to_rfc3339(),
            next_check_after,
        ],
    )?;
    Ok(())
}

fn mark_report_purchase_session_checked_tx(
    tx: &rusqlite::Transaction<'_>,
    provider: &str,
    provider_order_id: &str,
    status: &str,
    provider_transaction_id: Option<&str>,
    provider_event_id: Option<&str>,
) -> Result<(), StorageError> {
    let now = Utc::now();
    let next_check_after =
        next_billing_payment_check_after(status, 0).map(|value| value.to_rfc3339());
    tx.execute(
        "UPDATE report_purchase_sessions
         SET status = ?3,
             last_provider_transaction_id = COALESCE(?4, last_provider_transaction_id),
             last_provider_event_id = COALESCE(?5, last_provider_event_id),
             last_checked_at = ?6,
             next_check_after = ?7,
             check_attempts = check_attempts + 1,
             updated_at = ?6
         WHERE provider = ?1 AND provider_order_id = ?2",
        params![
            provider,
            provider_order_id,
            status,
            provider_transaction_id,
            provider_event_id,
            now.to_rfc3339(),
            next_check_after,
        ],
    )?;
    Ok(())
}

fn next_billing_payment_check_after(
    status: &str,
    completed_attempts: i64,
) -> Option<chrono::DateTime<Utc>> {
    if !matches!(status, "created" | "pending") {
        return None;
    }
    let delay_seconds = match completed_attempts {
        value if value <= 1 => 10,
        2 => 20,
        3 => 40,
        4 => 60,
        _ => 120,
    };
    Some(Utc::now() + Duration::seconds(delay_seconds))
}

fn insert_subscription_event(
    tx: &rusqlite::Transaction<'_>,
    event: &BillingEvent,
) -> Result<bool, StorageError> {
    let event_id = format!(
        "subevt_{}",
        short_id(&format!("{}:{}", event.provider, event.provider_event_id))
    );
    let received_at = Utc::now().to_rfc3339();
    let changed = tx.execute(
        "INSERT OR IGNORE INTO subscription_events (
            event_id, provider, provider_event_id, event_type, account_id,
            provider_customer_id, provider_subscription_id, provider_order_id,
            provider_transaction_id, payload_json, received_at, processed_at,
            processing_status, processing_error
        ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, ?9, ?10, ?11, 'processed', NULL)",
        params![
            event_id,
            event.provider,
            event.provider_event_id,
            event.event_type.as_str(),
            event.account_id,
            provider_subscription_id(event),
            event.provider_order_id,
            event.provider_transaction_id,
            event.raw_payload_json,
            received_at,
            received_at,
        ],
    )?;
    Ok(changed > 0)
}

fn apply_billing_state_transition(
    tx: &rusqlite::Transaction<'_>,
    event: &BillingEvent,
    plan_code: &str,
    billing_cycle: &str,
) -> Result<CloudEntitlement, StorageError> {
    let now = Utc::now();
    let now_text = now.to_rfc3339();
    let subscription_id = format!("sub_{}", short_id(&provider_subscription_id(event)));
    let provider_subscription_id = provider_subscription_id(event);
    let period_ends_at = match billing_cycle {
        "yearly" => now + Duration::days(365),
        _ => now + Duration::days(30),
    };
    let (subscription_status, entitlement_status, effective_plan_code, features, grace_ends_at) =
        match event.event_type {
            BillingEventType::PaymentSucceeded | BillingEventType::SubscriptionRenewed => (
                "active",
                "active",
                plan_code,
                entitlement_features_for_plan(plan_code),
                None,
            ),
            BillingEventType::PaymentFailed => (
                "grace",
                "grace",
                plan_code,
                entitlement_features_for_plan(plan_code),
                Some((now + Duration::days(3)).to_rfc3339()),
            ),
            BillingEventType::SubscriptionCanceled
            | BillingEventType::SubscriptionExpired
            | BillingEventType::RefundSucceeded => (
                "expired",
                "expired",
                "free",
                default_entitlement_features(),
                None,
            ),
        };
    let effective_plan_name = plan_name_for_code(effective_plan_code);
    let features_json = features.to_string();

    tx.execute(
        "INSERT INTO billing_customers (
            account_id, provider, provider_customer_id, email, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        ON CONFLICT(account_id) DO UPDATE SET
            provider = excluded.provider,
            provider_customer_id = excluded.provider_customer_id,
            email = excluded.email,
            updated_at = excluded.updated_at",
        params![
            event.account_id,
            event.provider,
            format!(
                "{}_customer_{}",
                event.provider,
                short_id(&event.account_id)
            ),
            event.account_id,
            now_text,
            now_text,
        ],
    )?;

    tx.execute(
        "INSERT INTO subscriptions (
            subscription_id, account_id, provider, provider_subscription_id,
            provider_price_id, provider_product_id, provider_order_id,
            provider_transaction_id, plan_code, billing_cycle, status,
            current_period_started_at, current_period_ends_at, trial_started_at,
            trial_ends_at, grace_ends_at, cancel_at_period_end, canceled_at,
            latest_invoice_id, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, NULL, NULL, ?14, 0, ?15, ?16, ?17, ?18)
        ON CONFLICT(provider, provider_subscription_id) DO UPDATE SET
            provider_price_id = excluded.provider_price_id,
            provider_product_id = excluded.provider_product_id,
            provider_order_id = excluded.provider_order_id,
            provider_transaction_id = excluded.provider_transaction_id,
            plan_code = excluded.plan_code,
            billing_cycle = excluded.billing_cycle,
            status = excluded.status,
            current_period_started_at = excluded.current_period_started_at,
            current_period_ends_at = excluded.current_period_ends_at,
            grace_ends_at = excluded.grace_ends_at,
            canceled_at = excluded.canceled_at,
            latest_invoice_id = excluded.latest_invoice_id,
            updated_at = excluded.updated_at",
        params![
            subscription_id,
            event.account_id,
            event.provider,
            provider_subscription_id,
            provider_price_id(plan_code, billing_cycle),
            provider_product_id(plan_code),
            event.provider_order_id,
            event.provider_transaction_id,
            plan_code,
            billing_cycle,
            subscription_status,
            now_text,
            period_ends_at.to_rfc3339(),
            grace_ends_at,
            if subscription_status == "expired" {
                Some(now_text.clone())
            } else {
                None
            },
            event.provider_transaction_id,
            now_text,
            now_text,
        ],
    )?;

    let entitlement_id = tx.query_row(
        "SELECT entitlement_id FROM cloud_accounts WHERE id = ?1",
        params![event.account_id],
        |row| row.get::<_, String>(0),
    )?;
    tx.execute(
        "INSERT INTO entitlements (
            entitlement_id, account_id, plan_code, plan_name, status, features_json,
            billing_source, subscription_id, trial_started_at, trial_ends_at,
            current_period_started_at, current_period_ends_at, grace_ends_at,
            last_provider_event_id, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, NULL, NULL, ?9, ?10, ?11, ?12, ?13)
        ON CONFLICT(account_id) DO UPDATE SET
            plan_code = excluded.plan_code,
            plan_name = excluded.plan_name,
            status = excluded.status,
            features_json = excluded.features_json,
            billing_source = excluded.billing_source,
            subscription_id = excluded.subscription_id,
            current_period_started_at = excluded.current_period_started_at,
            current_period_ends_at = excluded.current_period_ends_at,
            grace_ends_at = excluded.grace_ends_at,
            last_provider_event_id = excluded.last_provider_event_id,
            updated_at = excluded.updated_at",
        params![
            entitlement_id,
            event.account_id,
            effective_plan_code,
            effective_plan_name,
            entitlement_status,
            features_json,
            event.provider,
            subscription_id,
            now_text,
            period_ends_at.to_rfc3339(),
            grace_ends_at,
            event.provider_event_id,
            now_text,
        ],
    )?;

    tx.execute(
        "UPDATE cloud_accounts
         SET entitlement_plan_name = ?2,
             entitlement_plan_code = ?3,
             entitlement_status = ?4,
             entitlement_features_json = ?5,
             updated_at = ?6
         WHERE id = ?1",
        params![
            event.account_id,
            effective_plan_name,
            effective_plan_code,
            entitlement_status,
            features_json,
            now_text,
        ],
    )?;
    cloud_entitlement_for_account(tx, &event.account_id)
}

fn cloud_entitlement_for_account(
    conn: &rusqlite::Connection,
    account_id: &str,
) -> Result<CloudEntitlement, StorageError> {
    conn.query_row(
        "SELECT entitlement_id, entitlement_plan_name, entitlement_plan_code,
                entitlement_status, entitlement_features_json
         FROM cloud_accounts WHERE id = ?1",
        params![account_id],
        |row| {
            let features_json: String = row.get(4)?;
            Ok(CloudEntitlement {
                id: row.get(0)?,
                plan_name: Some(row.get(1)?),
                plan_code: row.get(2)?,
                status: row.get(3)?,
                features: serde_json::from_str(&features_json)
                    .unwrap_or_else(|_| default_entitlement_features()),
            })
        },
    )
    .map_err(StorageError::from)
}

fn seed_team_workspace_records(
    tx: &rusqlite::Transaction<'_>,
    account_id: &str,
    workspace_id: &str,
    display_name: &str,
    now: &str,
) -> Result<(), StorageError> {
    let workspace_name = if display_name.trim().is_empty() {
        "个人空间"
    } else {
        display_name.trim()
    };
    tx.execute(
        "INSERT INTO team_workspaces (
            workspace_id, account_id, name, workspace_type, status, created_at, updated_at
        ) VALUES (?1, ?2, ?3, 'personal', 'active', ?4, ?4)
        ON CONFLICT(workspace_id) DO UPDATE SET
            name = excluded.name,
            updated_at = excluded.updated_at",
        params![workspace_id, account_id, workspace_name, now],
    )?;
    tx.execute(
        "INSERT INTO team_members (
            member_id, workspace_id, account_id, role, status, invited_by, joined_at,
            created_at, updated_at
        ) VALUES (?1, ?2, ?3, 'owner', 'active', NULL, ?4, ?4, ?4)
        ON CONFLICT(workspace_id, account_id) DO UPDATE SET
            role = excluded.role,
            status = excluded.status,
            joined_at = excluded.joined_at,
            updated_at = excluded.updated_at",
        params![
            format!("tm_{}", short_id(&format!("{workspace_id}:{account_id}"))),
            workspace_id,
            account_id,
            now
        ],
    )?;
    record_team_audit_event_with_conn(
        tx,
        workspace_id,
        account_id,
        None,
        "seed_workspace",
        "workspace",
        workspace_id,
        None,
        Some(serde_json::json!({
            "workspaceId": workspace_id,
            "workspaceType": "personal",
            "name": workspace_name,
        })),
        "seed_personal_workspace",
    )?;
    Ok(())
}

fn team_workspace_enabled_for_account(
    conn: &rusqlite::Connection,
    account_id: &str,
) -> Result<bool, StorageError> {
    let features_json: String = conn.query_row(
        "SELECT entitlement_features_json FROM cloud_accounts WHERE id = ?1",
        params![account_id.trim()],
        |row| row.get(0),
    )?;
    let features =
        serde_json::from_str(&features_json).unwrap_or_else(|_| default_entitlement_features());
    Ok(features
        .get("team_workspace")
        .and_then(serde_json::Value::as_bool)
        == Some(true))
}

fn current_team_workspace_for_account(
    conn: &rusqlite::Connection,
    account_id: &str,
) -> Result<TeamWorkspaceSummary, StorageError> {
    conn.query_row(
        "SELECT w.workspace_id, w.account_id, w.name, w.workspace_type, w.status,
                COUNT(DISTINCT m.member_id) AS member_count,
                COUNT(DISTINCT s.shared_record_id) AS shared_record_count,
                COUNT(DISTINCT a.audit_id) AS audit_event_count,
                w.created_at, w.updated_at
         FROM team_workspaces w
         LEFT JOIN team_members m ON m.workspace_id = w.workspace_id AND m.status != 'removed'
         LEFT JOIN team_shared_library_records s ON s.workspace_id = w.workspace_id
         LEFT JOIN team_audit_logs a ON a.workspace_id = w.workspace_id
         WHERE w.account_id = ?1
         GROUP BY w.workspace_id, w.account_id, w.name, w.workspace_type, w.status, w.created_at, w.updated_at
         ORDER BY CASE WHEN w.workspace_type = 'team' THEN 0 ELSE 1 END, w.created_at ASC
         LIMIT 1",
        params![account_id],
        team_workspace_summary_from_row,
    )
    .map_err(StorageError::from)
}

fn list_team_workspaces_for_account(
    conn: &rusqlite::Connection,
    account_id: &str,
) -> Result<Vec<TeamWorkspaceSummary>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT w.workspace_id, w.account_id, w.name, w.workspace_type, w.status,
                COUNT(DISTINCT m.member_id) AS member_count,
                COUNT(DISTINCT s.shared_record_id) AS shared_record_count,
                COUNT(DISTINCT a.audit_id) AS audit_event_count,
                w.created_at, w.updated_at
         FROM team_workspaces w
         LEFT JOIN team_members m ON m.workspace_id = w.workspace_id AND m.status != 'removed'
         LEFT JOIN team_shared_library_records s ON s.workspace_id = w.workspace_id
         LEFT JOIN team_audit_logs a ON a.workspace_id = w.workspace_id
         WHERE w.account_id = ?1
         GROUP BY w.workspace_id, w.account_id, w.name, w.workspace_type, w.status, w.created_at, w.updated_at
         ORDER BY CASE WHEN w.workspace_type = 'team' THEN 0 ELSE 1 END, w.created_at ASC",
    )?;
    let rows = stmt.query_map(params![account_id], team_workspace_summary_from_row)?;
    let workspaces = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(workspaces)
}

fn create_team_workspace_with_conn(
    conn: &rusqlite::Connection,
    account_id: &str,
    request: &TeamWorkspaceCreateRequest,
) -> Result<TeamWorkspaceSummary, StorageError> {
    let now = Utc::now().to_rfc3339();
    let workspace_id = format!(
        "tw_{}",
        short_id(&format!("{}:{}:{}", account_id, request.name.trim(), now))
    );
    conn.execute(
        "INSERT INTO team_workspaces (
            workspace_id, account_id, name, workspace_type, status, created_at, updated_at
        ) VALUES (?1, ?2, ?3, 'team', 'active', ?4, ?4)",
        params![workspace_id, account_id, request.name.trim(), now],
    )?;
    conn.execute(
        "INSERT INTO team_members (
            member_id, workspace_id, account_id, role, status, invited_by, joined_at,
            created_at, updated_at
        ) VALUES (?1, ?2, ?3, 'owner', 'active', NULL, ?4, ?4, ?4)",
        params![
            format!(
                "tm_{}",
                short_id(&format!("{workspace_id}:{account_id}:owner"))
            ),
            workspace_id,
            account_id,
            now
        ],
    )?;
    record_team_audit_event_with_conn(
        conn,
        &workspace_id,
        account_id,
        None,
        "create_workspace",
        "workspace",
        &workspace_id,
        None,
        Some(serde_json::json!({
            "workspaceId": workspace_id,
            "workspaceType": "team",
            "name": request.name.trim(),
        })),
        "create_team_workspace",
    )?;
    current_team_workspace_by_id(conn, &workspace_id)
}

fn current_team_workspace_by_id(
    conn: &rusqlite::Connection,
    workspace_id: &str,
) -> Result<TeamWorkspaceSummary, StorageError> {
    conn.query_row(
        "SELECT w.workspace_id, w.account_id, w.name, w.workspace_type, w.status,
                COUNT(DISTINCT m.member_id) AS member_count,
                COUNT(DISTINCT s.shared_record_id) AS shared_record_count,
                COUNT(DISTINCT a.audit_id) AS audit_event_count,
                w.created_at, w.updated_at
         FROM team_workspaces w
         LEFT JOIN team_members m ON m.workspace_id = w.workspace_id AND m.status != 'removed'
         LEFT JOIN team_shared_library_records s ON s.workspace_id = w.workspace_id
         LEFT JOIN team_audit_logs a ON a.workspace_id = w.workspace_id
         WHERE w.workspace_id = ?1
         GROUP BY w.workspace_id, w.account_id, w.name, w.workspace_type, w.status, w.created_at, w.updated_at",
        params![workspace_id],
        team_workspace_summary_from_row,
    )
    .map_err(StorageError::from)
}

fn ensure_team_workspace_access(
    conn: &rusqlite::Connection,
    account_id: &str,
    workspace_id: &str,
) -> Result<(), StorageError> {
    if !team_workspace_enabled_for_account(conn, account_id)? {
        return Err(StorageError::Forbidden);
    }
    let allowed = conn
        .query_row(
            "SELECT 1 FROM team_members WHERE workspace_id = ?1 AND account_id = ?2 AND status = 'active'",
            params![workspace_id, account_id],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if allowed {
        Ok(())
    } else {
        Err(StorageError::Forbidden)
    }
}

fn list_team_members_for_workspace(
    conn: &rusqlite::Connection,
    workspace_id: &str,
) -> Result<Vec<TeamMemberRecord>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT member_id, workspace_id, account_id, role, status, invited_by, joined_at, created_at, updated_at
         FROM team_members
         WHERE workspace_id = ?1
         ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![workspace_id], team_member_from_row)?;
    let members = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(members)
}

fn create_team_member_with_conn(
    conn: &rusqlite::Connection,
    workspace_id: &str,
    request: &TeamMemberCreateRequest,
) -> Result<TeamMemberRecord, StorageError> {
    let now = Utc::now().to_rfc3339();
    let member_id = format!(
        "tm_{}",
        short_id(&format!(
            "{}:{}:{}",
            workspace_id,
            request.account_id.trim(),
            now
        ))
    );
    conn.execute(
        "INSERT INTO team_members (
            member_id, workspace_id, account_id, role, status, invited_by, joined_at, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, 'active', ?5, ?6, ?6, ?6)
        ON CONFLICT(workspace_id, account_id) DO UPDATE SET
            role = excluded.role,
            status = excluded.status,
            invited_by = excluded.invited_by,
            joined_at = excluded.joined_at,
            updated_at = excluded.updated_at",
        params![
            member_id,
            workspace_id,
            request.account_id.trim(),
            request.role.trim(),
            request.invited_by.as_deref(),
            now
        ],
    )?;
    let record = team_member_by_account(conn, workspace_id, request.account_id.trim())?;
    record_team_audit_event_with_conn(
        conn,
        workspace_id,
        request.account_id.trim(),
        Some(&record.member_id),
        "create_member",
        "member",
        request.account_id.trim(),
        None,
        Some(serde_json::json!({
            "role": request.role.trim(),
            "status": "active",
            "invitedBy": request.invited_by,
        })),
        "create_team_member",
    )?;
    Ok(record)
}

fn update_team_member_with_conn(
    conn: &rusqlite::Connection,
    actor_account_id: &str,
    member_id: &str,
    request: &TeamMemberUpdateRequest,
) -> Result<TeamMemberRecord, StorageError> {
    let before = team_member_by_id(conn, member_id)?;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE team_members
         SET role = ?1, status = ?2, updated_at = ?3,
             joined_at = CASE WHEN ?2 = 'active' AND joined_at IS NULL THEN ?3 ELSE joined_at END
         WHERE member_id = ?4",
        params![request.role.trim(), request.status.trim(), now, member_id],
    )?;
    let after = team_member_by_id(conn, member_id)?;
    record_team_audit_event_with_conn(
        conn,
        &after.workspace_id,
        actor_account_id,
        Some(member_id),
        "update_member",
        "member",
        member_id,
        Some(serde_json::to_value(before).unwrap_or_else(|_| serde_json::json!({}))),
        Some(serde_json::to_value(&after).unwrap_or_else(|_| serde_json::json!({}))),
        request.reason.trim(),
    )?;
    Ok(after)
}

fn share_team_library_record_with_conn(
    conn: &rusqlite::Connection,
    workspace_id: &str,
    request: &TeamSharedLibraryShareRequest,
) -> Result<TeamSharedLibraryRecord, StorageError> {
    if request.sync_scope.trim() != "metadata" {
        return Err(StorageError::BadRequest(
            "team shared library sync scope must be metadata".to_string(),
        ));
    }
    let now = Utc::now().to_rfc3339();
    let visible_to_roles_json = serde_json::to_string(&request.visible_to_roles)
        .map_err(|_| StorageError::BadRequest("visible roles invalid".to_string()))?;
    let shared_record_id = format!(
        "ts_{}",
        short_id(&format!(
            "{}:{}:{}:{}",
            workspace_id,
            request.source_record_id.trim(),
            request.watermark_uid.trim(),
            now
        ))
    );
    conn.execute(
        "INSERT INTO team_shared_library_records (
            shared_record_id, workspace_id, source_record_id, watermark_uid, revision, record_type,
            owner_creator_profile_id, visible_to_roles_json, sync_scope, created_by, created_at, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
        ON CONFLICT(workspace_id, source_record_id, revision) DO UPDATE SET
            watermark_uid = excluded.watermark_uid,
            record_type = excluded.record_type,
            owner_creator_profile_id = excluded.owner_creator_profile_id,
            visible_to_roles_json = excluded.visible_to_roles_json,
            sync_scope = excluded.sync_scope,
            created_by = excluded.created_by,
            updated_at = excluded.updated_at",
        params![
            shared_record_id,
            workspace_id,
            request.source_record_id.trim(),
            request.watermark_uid.trim(),
            request.revision,
            request.record_type.trim(),
            request.owner_creator_profile_id.trim(),
            visible_to_roles_json,
            request.sync_scope.trim(),
            request.created_by.trim(),
            now
        ],
    )?;
    team_shared_library_record_by_source(
        conn,
        workspace_id,
        request.source_record_id.trim(),
        request.revision,
    )
}

fn list_team_shared_library_records_for_workspace(
    conn: &rusqlite::Connection,
    workspace_id: &str,
) -> Result<Vec<TeamSharedLibraryRecord>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT shared_record_id, workspace_id, source_record_id, watermark_uid, revision,
                record_type, owner_creator_profile_id, visible_to_roles_json, sync_scope,
                created_by, created_at, updated_at
         FROM team_shared_library_records
         WHERE workspace_id = ?1
         ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map(params![workspace_id], team_shared_library_record_from_row)?;
    let records = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(records)
}

fn team_shared_library_record_by_source(
    conn: &rusqlite::Connection,
    workspace_id: &str,
    source_record_id: &str,
    revision: i64,
) -> Result<TeamSharedLibraryRecord, StorageError> {
    conn.query_row(
        "SELECT shared_record_id, workspace_id, source_record_id, watermark_uid, revision,
                record_type, owner_creator_profile_id, visible_to_roles_json, sync_scope,
                created_by, created_at, updated_at
         FROM team_shared_library_records
         WHERE workspace_id = ?1 AND source_record_id = ?2 AND revision = ?3",
        params![workspace_id, source_record_id, revision],
        team_shared_library_record_from_row,
    )
    .map_err(StorageError::from)
}

fn list_team_audit_logs_for_workspace(
    conn: &rusqlite::Connection,
    workspace_id: &str,
) -> Result<Vec<TeamAuditRecord>, StorageError> {
    let mut stmt = conn.prepare(
        "SELECT audit_id, workspace_id, actor_account_id, actor_member_id, action, target_type,
                target_id, before_json, after_json, reason, created_at
         FROM team_audit_logs
         WHERE workspace_id = ?1
         ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![workspace_id], team_audit_record_from_row)?;
    let events = rows.collect::<Result<Vec<_>, _>>()?;
    Ok(events)
}

fn record_team_audit_event_with_conn(
    conn: &rusqlite::Connection,
    workspace_id: &str,
    actor_account_id: &str,
    actor_member_id: Option<&str>,
    action: &str,
    target_type: &str,
    target_id: &str,
    before_json: Option<serde_json::Value>,
    after_json: Option<serde_json::Value>,
    reason: &str,
) -> Result<(), StorageError> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO team_audit_logs (
            audit_id, workspace_id, actor_account_id, actor_member_id, action, target_type,
            target_id, before_json, after_json, reason, created_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
        params![
            format!(
                "ta_{}",
                short_id(&format!(
                    "{workspace_id}:{actor_account_id}:{action}:{target_id}:{now}"
                ))
            ),
            workspace_id,
            actor_account_id,
            actor_member_id,
            action,
            target_type,
            target_id,
            before_json.map(|value| value.to_string()),
            after_json.map(|value| value.to_string()),
            if reason.trim().is_empty() {
                action
            } else {
                reason.trim()
            },
            now,
        ],
    )?;
    Ok(())
}

fn team_workspace_summary_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<TeamWorkspaceSummary, rusqlite::Error> {
    let created_at: String = row.get(8)?;
    let updated_at: String = row.get(9)?;
    Ok(TeamWorkspaceSummary {
        workspace_id: row.get(0)?,
        account_id: row.get(1)?,
        name: row.get(2)?,
        workspace_type: row.get(3)?,
        status: row.get(4)?,
        member_count: row.get::<_, i64>(5).unwrap_or_default() as u32,
        shared_record_count: row.get::<_, i64>(6).unwrap_or_default() as u32,
        audit_event_count: row.get::<_, i64>(7).unwrap_or_default() as u32,
        created_at: chrono::DateTime::parse_from_rfc3339(&created_at)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&updated_at)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

fn team_member_from_row(row: &rusqlite::Row<'_>) -> Result<TeamMemberRecord, rusqlite::Error> {
    Ok(TeamMemberRecord {
        member_id: row.get(0)?,
        workspace_id: row.get(1)?,
        account_id: row.get(2)?,
        role: row.get(3)?,
        status: row.get(4)?,
        invited_by: row.get(5)?,
        joined_at: row
            .get::<_, Option<String>>(6)?
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
            .map(|value| value.with_timezone(&Utc)),
        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(7)?)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(8)?)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

fn team_member_by_id(
    conn: &rusqlite::Connection,
    member_id: &str,
) -> Result<TeamMemberRecord, StorageError> {
    conn.query_row(
        "SELECT member_id, workspace_id, account_id, role, status, invited_by, joined_at, created_at, updated_at
         FROM team_members
         WHERE member_id = ?1",
        params![member_id],
        team_member_from_row,
    )
    .map_err(StorageError::from)
}

fn team_member_by_account(
    conn: &rusqlite::Connection,
    workspace_id: &str,
    account_id: &str,
) -> Result<TeamMemberRecord, StorageError> {
    conn.query_row(
        "SELECT member_id, workspace_id, account_id, role, status, invited_by, joined_at, created_at, updated_at
         FROM team_members
         WHERE workspace_id = ?1 AND account_id = ?2",
        params![workspace_id, account_id],
        team_member_from_row,
    )
    .map_err(StorageError::from)
}

fn team_shared_library_record_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<TeamSharedLibraryRecord, rusqlite::Error> {
    let visible_to_roles_json: String = row.get(7)?;
    let visible_to_roles = serde_json::from_str(&visible_to_roles_json).unwrap_or_default();
    Ok(TeamSharedLibraryRecord {
        shared_record_id: row.get(0)?,
        workspace_id: row.get(1)?,
        source_record_id: row.get(2)?,
        watermark_uid: row.get(3)?,
        revision: row.get(4)?,
        record_type: row.get(5)?,
        owner_creator_profile_id: row.get(6)?,
        visible_to_roles,
        sync_scope: row.get(8)?,
        created_by: row.get(9)?,
        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(11)?)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

fn team_audit_record_from_row(row: &rusqlite::Row<'_>) -> Result<TeamAuditRecord, rusqlite::Error> {
    let before_json = row
        .get::<_, Option<String>>(7)?
        .and_then(|value| serde_json::from_str(&value).ok());
    let after_json = row
        .get::<_, Option<String>>(8)?
        .and_then(|value| serde_json::from_str(&value).ok());
    Ok(TeamAuditRecord {
        audit_id: row.get(0)?,
        workspace_id: row.get(1)?,
        actor_account_id: row.get(2)?,
        actor_member_id: row.get(3)?,
        action: row.get(4)?,
        target_type: row.get(5)?,
        target_id: row.get(6)?,
        before_json,
        after_json,
        reason: row.get(9)?,
        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(10)?)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
    })
}

fn cloud_video_task_record_from_row(
    row: &rusqlite::Row<'_>,
) -> Result<CloudVideoTaskRecord, rusqlite::Error> {
    let target_profiles_json: String = row.get(9)?;
    let upload_manifest_json: String = row.get(10)?;
    Ok(CloudVideoTaskRecord {
        task_id: row.get(0)?,
        schema_version: row.get(1)?,
        account_id: row.get(2)?,
        workspace_id: row.get(3)?,
        creator_profile_id: row.get(4)?,
        capability_level: row.get(5)?,
        watermark_uid: row.get(6)?,
        source_hash: row.get(7)?,
        duration_ms: row.get::<_, i64>(8)? as u64,
        target_profiles: serde_json::from_str(&target_profiles_json).unwrap_or_default(),
        upload_manifest: serde_json::from_str(&upload_manifest_json).unwrap_or(
            VideoUploadManifest {
                schema_version: "video_upload_manifest_v1".to_string(),
                contains_original_video: false,
                contains_watermarked_video: false,
                contains_local_paths: false,
                contains_proxy: false,
                items: Vec::new(),
            },
        ),
        status: row.get(11)?,
        quota_units: row.get::<_, i64>(12)? as u64,
        failure_code: row.get(13)?,
        strategy_digest: row.get(14)?,
        self_check_threshold: row.get(15)?,
        self_check_confidence: row.get(16)?,
        checked_frames: row.get::<_, Option<i64>>(17)?.map(|value| value as u32),
        watermarked_media_hash: row.get(18)?,
        output_media_storage_ref: row.get(19)?,
        output_media_bytes: row.get::<_, Option<i64>>(20)?.map(|value| value as u64),
        output_media_content_type: row.get(21)?,
        worker_receipt_hash: row.get(22)?,
        worker_receipt: row
            .get::<_, Option<String>>(23)?
            .and_then(|value| serde_json::from_str(&value).ok()),
        server_receipt_signature: row.get(24)?,
        usage_ledger_id: row.get(25)?,
        worker_id: row.get(26)?,
        attempt_id: row.get(27)?,
        attempt_count: row.get::<_, i64>(29)? as u32,
        lease_expires_at: row
            .get::<_, Option<String>>(30)?
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
            .map(|value| value.with_timezone(&Utc)),
        last_failure_code: row.get(31)?,
        last_failure_stage: row.get(32)?,
        created_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(33)?)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        updated_at: chrono::DateTime::parse_from_rfc3339(&row.get::<_, String>(34)?)
            .map(|value| value.with_timezone(&Utc))
            .unwrap_or_else(|_| Utc::now()),
        completed_at: row
            .get::<_, Option<String>>(35)?
            .and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok())
            .map(|value| value.with_timezone(&Utc)),
    })
}

fn load_cloud_video_task_with_conn(
    conn: &rusqlite::Connection,
    task_id: &str,
) -> Result<CloudVideoTaskRecord, StorageError> {
    conn.query_row(
        "SELECT task_id, schema_version, account_id, workspace_id, creator_profile_id,
                capability_level, watermark_uid, source_hash, duration_ms,
                target_profiles_json, upload_manifest_json, status, quota_units,
                failure_code, strategy_digest, self_check_threshold, self_check_confidence,
                checked_frames, watermarked_media_hash, output_media_storage_ref,
                output_media_bytes, output_media_content_type, worker_receipt_hash,
                worker_receipt_json, server_receipt_signature,
                usage_ledger_id, worker_id, attempt_id, lease_token_hash,
                attempt_count, lease_expires_at, last_failure_code, last_failure_stage,
                created_at, updated_at, completed_at
         FROM cloud_video_tasks
         WHERE task_id = ?1",
        params![task_id.trim()],
        cloud_video_task_record_from_row,
    )
    .map_err(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => {
            StorageError::BadRequest("cloud_video_task_not_found".to_string())
        }
        other => StorageError::Database(other),
    })
}

fn parse_rfc3339_utc(value: &str) -> Result<chrono::DateTime<Utc>, StorageError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| StorageError::BadRequest("datetime_invalid".to_string()))
}

fn parse_optional_rfc3339_utc(
    value: Option<&str>,
) -> Result<Option<chrono::DateTime<Utc>>, StorageError> {
    value.map(parse_rfc3339_utc).transpose()
}

fn validate_cloud_video_task_active_lease(
    conn: &Connection,
    existing: &CloudVideoTaskRecord,
    worker_id: &str,
    attempt_id: &str,
    lease_token: &str,
) -> Result<(), StorageError> {
    if existing.status != CLOUD_VIDEO_TASK_STATUS_RUNNING {
        return Err(StorageError::BadRequest(
            "cloud_video_task_completion_stale_attempt".to_string(),
        ));
    }
    if existing.worker_id.as_deref().map(str::trim) != Some(worker_id)
        || existing.attempt_id.as_deref().map(str::trim) != Some(attempt_id)
    {
        return Err(StorageError::BadRequest(
            "cloud_video_task_completion_stale_attempt".to_string(),
        ));
    }
    let (lease_token_hash, lease_expires_at): (Option<String>, Option<String>) = conn.query_row(
        "SELECT lease_token_hash, lease_expires_at FROM cloud_video_tasks WHERE task_id = ?1",
        params![existing.task_id],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;
    let expected_hash = cloud_video_lease_token_hash(lease_token);
    if lease_token_hash.as_deref() != Some(expected_hash.as_str()) {
        return Err(StorageError::BadRequest(
            "cloud_video_task_completion_stale_attempt".to_string(),
        ));
    }
    let lease_expires_at = lease_expires_at
        .as_deref()
        .ok_or_else(|| {
            StorageError::BadRequest("cloud_video_task_completion_stale_attempt".to_string())
        })
        .and_then(parse_rfc3339_utc)?;
    if lease_expires_at <= Utc::now() {
        return Err(StorageError::BadRequest(
            "cloud_video_task_completion_stale_attempt".to_string(),
        ));
    }
    Ok(())
}

fn reconcile_message(status: &str) -> &'static str {
    match status {
        "succeeded" => "支付已确认，权益已生效。",
        "pending" => "尚未确认支付完成，请完成支付或稍后刷新。",
        "expired" => "支付会话已过期，请重新创建支付。",
        "failed" | "closed" => "支付未完成，请重新创建支付。",
        _ => "暂未检测到支付完成，请稍后再试。",
    }
}

fn normalize_plan_code(plan_code: &str) -> Result<String, StorageError> {
    let value = plan_code.trim().to_lowercase();
    match value.as_str() {
        "creator" | "studio" | "enterprise" => Ok(value),
        _ => Err(StorageError::BadRequest(
            "plan_code_not_allowed".to_string(),
        )),
    }
}

fn normalize_self_service_plan_code(plan_code: &str) -> Result<String, StorageError> {
    let value = normalize_plan_code(plan_code)?;
    match value.as_str() {
        "creator" | "studio" => Ok(value),
        _ => Err(StorageError::BadRequest(
            "plan_code_not_allowed".to_string(),
        )),
    }
}

fn normalize_billing_cycle(billing_cycle: &str) -> Result<String, StorageError> {
    let value = billing_cycle.trim().to_lowercase();
    match value.as_str() {
        "monthly" | "yearly" => Ok(value),
        _ => Err(StorageError::BadRequest(
            "billing_cycle_not_allowed".to_string(),
        )),
    }
}

fn normalize_report_product_code(product_code: &str) -> Result<String, StorageError> {
    let value = product_code.trim().to_lowercase();
    match value.as_str() {
        REPORT_PRODUCT_COPYRIGHT_REPORT_SINGLE | REPORT_PRODUCT_RIGHTS_EVIDENCE_PACK_SINGLE => {
            Ok(value)
        }
        _ => Err(StorageError::BadRequest(
            "report_product_not_allowed".to_string(),
        )),
    }
}

fn report_product_price_cents(product_code: &str) -> Result<i64, StorageError> {
    match product_code {
        REPORT_PRODUCT_COPYRIGHT_REPORT_SINGLE => Ok(1990),
        REPORT_PRODUCT_RIGHTS_EVIDENCE_PACK_SINGLE => Ok(4990),
        _ => Err(StorageError::BadRequest(
            "report_product_not_allowed".to_string(),
        )),
    }
}

fn required_trimmed(value: &str, error_code: &str) -> Result<String, StorageError> {
    let value = value.trim();
    if value.is_empty() {
        Err(StorageError::BadRequest(error_code.to_string()))
    } else {
        Ok(value.to_string())
    }
}

fn provider_subscription_id(event: &BillingEvent) -> String {
    format!("{}_{}", event.provider, event.provider_order_id)
}

fn provider_product_id(plan_code: &str) -> String {
    format!("WECHAT_PRODUCT_{}", plan_code.to_uppercase())
}

fn provider_price_id(plan_code: &str, billing_cycle: &str) -> String {
    format!(
        "WECHAT_PRODUCT_{}_{}",
        plan_code.to_uppercase(),
        billing_cycle.to_uppercase()
    )
}

fn plan_name_for_code(plan_code: &str) -> &'static str {
    match plan_code {
        "creator" => "Creator",
        "studio" => "Studio",
        "enterprise" => "Enterprise",
        _ => "免费版",
    }
}

fn entitlement_features_for_plan(plan_code: &str) -> serde_json::Value {
    match plan_code {
        "creator" => serde_json::json!({
            "cloud_sync": true,
            "batch_processing": true,
            "report_export": false,
            "cloud_batch_processing": false,
            "cloud_video_processing": false,
            "priority_queue": false,
            "team_workspace": false,
            "api_access": false
        }),
        "studio" => serde_json::json!({
            "cloud_sync": true,
            "batch_processing": true,
            "report_export": false,
            "cloud_batch_processing": true,
            "cloud_video_processing": true,
            "priority_queue": true,
            "team_workspace": true,
            "api_access": false
        }),
        "enterprise" => serde_json::json!({
            "cloud_sync": true,
            "batch_processing": true,
            "report_export": false,
            "cloud_batch_processing": true,
            "cloud_video_processing": true,
            "priority_queue": true,
            "team_workspace": true,
            "api_access": true
        }),
        _ => default_entitlement_features(),
    }
}

fn sync_policy_for_entitlement_and_preference(
    features: &serde_json::Value,
    auto_sync_enabled: bool,
) -> String {
    if features
        .get("cloud_sync")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return "blocked_by_entitlement".to_string();
    }
    if auto_sync_enabled {
        "auto_cloud_vault".to_string()
    } else {
        "manual_local_only".to_string()
    }
}

fn ensure_cloud_sync_entitled_with_conn(
    conn: &Connection,
    account_id: &str,
) -> Result<(), StorageError> {
    let features_json: String = conn.query_row(
        "SELECT entitlement_features_json FROM cloud_accounts WHERE id = ?1",
        params![account_id.trim()],
        |row| row.get(0),
    )?;
    let features =
        serde_json::from_str(&features_json).unwrap_or_else(|_| default_entitlement_features());
    if features
        .get("cloud_sync")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
    {
        Ok(())
    } else {
        Err(StorageError::Forbidden)
    }
}

fn outcome_to_str(outcome: &AnonymousEventOutcome) -> &'static str {
    match outcome {
        AnonymousEventOutcome::Success => "success",
        AnonymousEventOutcome::Failure => "failure",
        AnonymousEventOutcome::Crash => "crash",
        AnonymousEventOutcome::Diagnostic => "diagnostic",
    }
}

fn cloud_operation(operation: &str) -> String {
    if operation.starts_with("upsert") {
        "upsert".to_string()
    } else {
        operation.to_string()
    }
}

fn sequence_from_cursor(cursor: Option<&str>) -> i64 {
    let Some(cursor) = cursor.map(str::trim).filter(|value| !value.is_empty()) else {
        return 0;
    };
    let Some(value) = cursor.strip_prefix("cursor_") else {
        return 0;
    };
    value.parse::<i64>().unwrap_or(0).max(0)
}

fn cursor_from_sequence(value: u64) -> String {
    format!("cursor_{value}")
}

fn short_id(input: &str) -> String {
    let mut hash = 2166136261u32;
    for byte in input.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")
}

fn generate_cloud_video_lease_token() -> String {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    format!("l3lease_{}", hex_lower_storage(&bytes))
}

fn cloud_video_lease_token_hash(token: &str) -> String {
    let digest = Sha256::digest(token.trim().as_bytes());
    format!("sha256:{}", hex_lower_storage(&digest))
}

fn looks_like_sha256(value: &str) -> bool {
    let Some(hex) = value.trim().strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn looks_like_local_path(value: &str) -> bool {
    value.starts_with("file:")
        || value.starts_with('/')
        || value.starts_with('\\')
        || value.as_bytes().get(1) == Some(&b':')
}

fn hex_lower_storage(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn normalize_identifier(input: &str) -> Result<String, StorageError> {
    let identifier = input.trim().to_lowercase();
    if identifier.is_empty() {
        return Err(StorageError::BadRequest(
            "identifier is required".to_string(),
        ));
    }
    Ok(identifier)
}

fn default_entitlement_features() -> serde_json::Value {
    serde_json::json!({
        "cloud_sync": false,
        "batch_processing": false,
        "report_export": false,
        "cloud_batch_processing": false,
        "cloud_video_processing": false,
        "priority_queue": false,
        "team_workspace": false,
        "api_access": false
    })
}

trait OptionalRowExt<T> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error>;
}

impl<T> OptionalRowExt<T> for Result<T, rusqlite::Error> {
    fn optional(self) -> Result<Option<T>, rusqlite::Error> {
        match self {
            Ok(value) => Ok(Some(value)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

#[derive(Debug, Clone)]
struct SessionTokenRow {
    access_token: String,
    refresh_token: String,
}

fn dimension_sql(dimension: &StatsDimension) -> (&'static str, &'static str) {
    match dimension {
        StatsDimension::Day => ("substr(occurred_at, 1, 10)", "substr(occurred_at, 1, 10)"),
        StatsDimension::Version => ("app_version", "app_version"),
        StatsDimension::Feature => ("feature_name", "feature_name"),
        StatsDimension::ErrorCode => (
            "coalesce(nullif(error_code, ''), 'none')",
            "coalesce(nullif(error_code, ''), 'none')",
        ),
        StatsDimension::MediaType => ("media_type", "media_type"),
        StatsDimension::Outcome => ("outcome", "outcome"),
    }
}

fn build_filters(query: &AnonymousFeedbackStatsQuery) -> (String, Vec<rusqlite::types::Value>) {
    let mut clauses = Vec::new();
    let mut values = Vec::new();

    if let Some(from) = query.from {
        clauses.push("occurred_at >= ?".to_string());
        values.push(rusqlite::types::Value::Text(from.to_rfc3339()));
    }
    if let Some(to) = query.to {
        clauses.push("occurred_at <= ?".to_string());
        values.push(rusqlite::types::Value::Text(to.to_rfc3339()));
    }
    if let Some(ref app_version) = query.app_version {
        clauses.push("app_version = ?".to_string());
        values.push(rusqlite::types::Value::Text(app_version.clone()));
    }
    if let Some(ref feature_name) = query.feature_name {
        clauses.push("feature_name = ?".to_string());
        values.push(rusqlite::types::Value::Text(feature_name.clone()));
    }
    if let Some(ref media_type) = query.media_type {
        clauses.push("media_type = ?".to_string());
        values.push(rusqlite::types::Value::Text(media_type.clone()));
    }
    if let Some(ref error_code) = query.error_code {
        clauses.push("error_code = ?".to_string());
        values.push(rusqlite::types::Value::Text(error_code.clone()));
    }
    if let Some(ref outcome) = query.outcome {
        clauses.push("outcome = ?".to_string());
        values.push(rusqlite::types::Value::Text(
            outcome_to_str(outcome).to_string(),
        ));
    }

    if clauses.is_empty() {
        (String::new(), values)
    } else {
        (format!("WHERE {}", clauses.join(" AND ")), values)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    fn sample_event(event_id: &str, outcome: AnonymousEventOutcome) -> AnonymousFeedbackEvent {
        AnonymousFeedbackEvent {
            event_id: event_id.to_string(),
            occurred_at: Utc::now(),
            install_id: "inst-1".to_string(),
            session_id: "sess-1".to_string(),
            app_version: "0.1.0".to_string(),
            feature_name: "watermark_video".to_string(),
            outcome,
            media_type: "video".to_string(),
            file_size_bucket: "50-200mb".to_string(),
            duration_ms: Some(1234),
            error_code: None,
            diagnostic_note: None,
            stack_summary: None,
            pipeline_id: Some("pipe-1".to_string()),
        }
    }

    fn sample_continue_request(
        identifier: &str,
        device_id: &str,
    ) -> crate::schema::ContinueAccountRequest {
        crate::schema::ContinueAccountRequest {
            identifier: identifier.to_string(),
            password: "correct-password".to_string(),
            verification_code: "000000".to_string(),
            device: crate::schema::ContinueAccountDevice {
                client_device_id: device_id.to_string(),
                name: "Device".to_string(),
                platform: "contract".to_string(),
                app_version: "0.1.0".to_string(),
                public_key: None,
            },
            local_creator_profile: crate::schema::ContinueAccountCreatorProfile {
                display_name: "Creator".to_string(),
                creator_seed_ref: "seed-ref".to_string(),
                seed_envelope_version: 1,
            },
        }
    }

    fn sample_cloud_video_task_request(
        session: &crate::schema::CloudAccountSession,
    ) -> crate::schema::CloudVideoTaskRequest {
        crate::schema::CloudVideoTaskRequest {
            schema_version: "cloud_video_task_v1".to_string(),
            workspace_id: session.workspace.id.clone(),
            creator_profile_id: session.creator_profile.id.clone(),
            capability_level: "hybrid_visual_watermark".to_string(),
            watermark_uid: "wm_cloud_video_l3".to_string(),
            source_hash: "sha256:cloud-video-source".to_string(),
            duration_ms: 125_000,
            target_profiles: vec!["douyin_9_16_h264_high_crf18_720p".to_string()],
            upload_manifest: crate::schema::VideoUploadManifest {
                schema_version: "video_upload_manifest_v1".to_string(),
                contains_original_video: false,
                contains_watermarked_video: false,
                contains_local_paths: false,
                contains_proxy: false,
                items: vec![crate::schema::VideoUploadManifestItem {
                    kind: "video_fingerprint_bundle".to_string(),
                    sha256: "sha256:cloud-video-bundle".to_string(),
                    bytes: 48_212,
                    storage_ref: None,
                    sandbox_profile: None,
                    transcode_profile: None,
                    width: None,
                    height: None,
                    frame_count: None,
                }],
            },
        }
    }

    fn sample_l3_worker_receipt_fields(
        task_id: &str,
        worker_id: &str,
        media_hash: &str,
    ) -> (serde_json::Value, String, String, u64, String) {
        let storage_ref = format!("object://l3-output/{task_id}/storage-test.mp4");
        let receipt = serde_json::json!({
            "algorithmSource": "watermark-core",
            "output": {
                "bytes": 4096,
                "contentType": "video/mp4",
                "sha256": media_hash,
                "storageRef": storage_ref,
            },
            "schemaVersion": "l3_worker_receipt_v1",
            "taskId": task_id,
            "workerId": worker_id,
        });
        let receipt_text = serde_json::to_string(&receipt).unwrap();
        let receipt_hash = format!(
            "sha256:{}",
            hex_lower_storage(&Sha256::digest(receipt_text.as_bytes()))
        );
        (
            receipt,
            receipt_hash,
            storage_ref,
            4096,
            "video/mp4".to_string(),
        )
    }

    fn sample_team_workspace_create_request(
        account_id: &str,
        name: &str,
    ) -> crate::schema::TeamWorkspaceCreateRequest {
        crate::schema::TeamWorkspaceCreateRequest {
            account_id: account_id.to_string(),
            name: name.to_string(),
        }
    }

    fn sample_team_member_create_request(
        account_id: &str,
        role: &str,
    ) -> crate::schema::TeamMemberCreateRequest {
        crate::schema::TeamMemberCreateRequest {
            account_id: account_id.to_string(),
            role: role.to_string(),
            invited_by: Some("owner_account".to_string()),
        }
    }

    fn sample_team_member_update_request(
        role: &str,
        status: &str,
        reason: &str,
    ) -> crate::schema::TeamMemberUpdateRequest {
        crate::schema::TeamMemberUpdateRequest {
            role: role.to_string(),
            status: status.to_string(),
            reason: reason.to_string(),
        }
    }

    fn sample_team_shared_library_share_request(
        source_record_id: &str,
        watermark_uid: &str,
    ) -> crate::schema::TeamSharedLibraryShareRequest {
        crate::schema::TeamSharedLibraryShareRequest {
            source_record_id: source_record_id.to_string(),
            watermark_uid: watermark_uid.to_string(),
            revision: 1,
            record_type: "vault_record".to_string(),
            owner_creator_profile_id: "creator_owner".to_string(),
            visible_to_roles: vec!["owner".to_string(), "admin".to_string()],
            sync_scope: "metadata".to_string(),
            created_by: "owner_account".to_string(),
            reason: "team share".to_string(),
        }
    }

    #[test]
    fn team_workspace_seed_create_member_share_and_audit_roundtrip() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("studio@example.com", "dev-1"))
            .unwrap();
        storage
            .set_entitlement_feature_for_tests(&session.account.id, "team_workspace", true)
            .unwrap();

        let current = storage
            .current_team_workspace(&session.access_token)
            .unwrap();
        assert_eq!(current.workspace_type, "personal");
        assert_eq!(current.member_count, 1);

        let created = storage
            .create_team_workspace(
                &session.access_token,
                &sample_team_workspace_create_request(&session.account.id, "Studio Alpha"),
            )
            .unwrap();
        assert_eq!(created.workspace_type, "team");
        assert_eq!(created.account_id, session.account.id);

        let workspaces = storage.list_team_workspaces(&session.access_token).unwrap();
        assert_eq!(workspaces.returned, 2);
        assert_eq!(workspaces.workspaces[0].workspace_type, "team");

        let current = storage
            .current_team_workspace(&session.access_token)
            .unwrap();
        assert_eq!(current.workspace_id, created.workspace_id);
        assert_eq!(current.workspace_type, "team");

        let team_members = storage
            .create_team_member(
                &session.access_token,
                &created.workspace_id,
                &sample_team_member_create_request("studio-member@example.com", "editor"),
            )
            .unwrap();
        assert_eq!(team_members.role, "editor");
        assert_eq!(team_members.status, "active");

        let updated = storage
            .update_team_member(
                &session.access_token,
                &team_members.member_id,
                &sample_team_member_update_request("viewer", "active", "downgrade"),
            )
            .unwrap();
        assert_eq!(updated.role, "viewer");
        assert_eq!(updated.status, "active");

        let shared = storage
            .share_team_library_record(
                &session.access_token,
                &created.workspace_id,
                &sample_team_shared_library_share_request(
                    "source-1",
                    "HS-11112222-33334444-55556666-77778888",
                ),
            )
            .unwrap();
        assert_eq!(shared.sync_scope, "metadata");
        assert_eq!(shared.record_type, "vault_record");

        let members = storage
            .list_team_members(&session.access_token, &created.workspace_id)
            .unwrap();
        assert_eq!(members.returned, 2);

        let records = storage
            .list_team_shared_library_records(&session.access_token, &created.workspace_id)
            .unwrap();
        assert_eq!(records.returned, 1);
        assert_eq!(records.records[0].shared_record_id, shared.shared_record_id);

        let audit_logs = storage
            .list_team_audit_logs(&session.access_token, &created.workspace_id)
            .unwrap();
        assert!(audit_logs.returned >= 3);
        assert_eq!(audit_logs.events[0].action, "share_record");
    }

    fn sample_l2_notary_request(
        session: &crate::schema::CloudAccountSession,
    ) -> VideoFingerprintNotaryRequest {
        VideoFingerprintNotaryRequest {
            schema_version: "video_fingerprint_notary_request_v1".to_string(),
            workspace_id: session.workspace.id.clone(),
            creator_profile_id: session.creator_profile.id.clone(),
            watermark_uid: "wm_video_l2".to_string(),
            source_hash: "sha256:source".to_string(),
            duration_ms: 125_000,
            frame_sample_policy: "uniform_8_frames_v1".to_string(),
            scene_count: 8,
            fingerprint_schema_version: "video_fingerprint_v1".to_string(),
            global_frame_fingerprints: vec![crate::schema::VideoGlobalFrameFingerprint {
                scene_index: 0,
                timestamp_ms: 1000,
                phash: "0000000000000001".to_string(),
                color_hash: "0000000000000002".to_string(),
                edge_hash: "0000000000000003".to_string(),
                motion_summary: "static-frame-v1".to_string(),
            }],
            local_block_fingerprint_root: "sha256:local-block-root".to_string(),
            local_block_count: 912,
            crop_window_fingerprint_root: "sha256:crop-window-root".to_string(),
            crop_window_count: 56,
            fingerprint_root: "sha256:fingerprint-root".to_string(),
            client_signature: "ed25519:client-signature".to_string(),
            upload_manifest: crate::schema::VideoUploadManifest {
                schema_version: "video_upload_manifest_v1".to_string(),
                contains_original_video: false,
                contains_watermarked_video: false,
                contains_local_paths: false,
                contains_proxy: false,
                items: vec![crate::schema::VideoUploadManifestItem {
                    kind: "video_fingerprint_bundle".to_string(),
                    sha256: "sha256:bundle".to_string(),
                    bytes: 48212,
                    storage_ref: None,
                    sandbox_profile: None,
                    transcode_profile: None,
                    width: None,
                    height: None,
                    frame_count: None,
                }],
            },
        }
    }

    fn sample_payment_session_request(
        session: &crate::schema::CloudAccountSession,
        plan_code: &str,
    ) -> BillingPaymentSessionRequest {
        BillingPaymentSessionRequest {
            account_id: session.account.id.clone(),
            workspace_id: session.workspace.id.clone(),
            plan_code: plan_code.to_string(),
            billing_cycle: "monthly".to_string(),
            preferred_provider: Some(FIXTURE_PROVIDER.to_string()),
        }
    }

    fn sample_report_purchase_request(
        session: &crate::schema::CloudAccountSession,
        product_code: &str,
    ) -> ReportPurchaseSessionRequest {
        ReportPurchaseSessionRequest {
            account_id: session.account.id.clone(),
            workspace_id: session.workspace.id.clone(),
            creator_profile_id: session.creator_profile.id.clone(),
            vault_record_id: "vault-report-1".to_string(),
            product_code: product_code.to_string(),
            preferred_provider: Some(FIXTURE_PROVIDER.to_string()),
        }
    }

    fn sample_watermark_reserve_request(
        session: &crate::schema::CloudAccountSession,
        request_id: &str,
    ) -> WatermarkIdReserveRequest {
        WatermarkIdReserveRequest {
            request_id: request_id.to_string(),
            workspace_id: session.workspace.id.clone(),
            creator_profile_id: session.creator_profile.id.clone(),
            media_type: "image".to_string(),
            payload_protocol_version: 2,
            payload_bytes_length: 119,
            parent_watermark_uid: None,
            revision: 1,
            original_hash: Some("sha256:original-a".to_string()),
        }
    }

    fn sample_watermark_reconcile_request(
        session: &crate::schema::CloudAccountSession,
        watermark_uid: &str,
        original_hash: &str,
    ) -> WatermarkIdReconcileRequest {
        WatermarkIdReconcileRequest {
            workspace_id: session.workspace.id.clone(),
            creator_profile_id: session.creator_profile.id.clone(),
            watermark_uid: watermark_uid.to_string(),
            media_type: "image".to_string(),
            payload_protocol_version: 2,
            payload_bytes_length: 119,
            parent_watermark_uid: None,
            revision: 1,
            original_hash: Some(original_hash.to_string()),
            protected_copy_hash: Some("sha256:protected-a".to_string()),
            write_verification_status: Some("verified".to_string()),
        }
    }

    fn sample_fixture_event(
        session: &crate::schema::CloudAccountSession,
        payment_session: &BillingPaymentSessionResponse,
        event_type: &str,
        event_id: &str,
    ) -> BillingFixtureEventRequest {
        BillingFixtureEventRequest {
            provider_event_id: event_id.to_string(),
            provider_order_id: payment_session.provider_order_id.clone(),
            provider_transaction_id: Some(format!("fixture_txn_{event_id}")),
            account_id: session.account.id.clone(),
            workspace_id: session.workspace.id.clone(),
            plan_code: "creator".to_string(),
            billing_cycle: "monthly".to_string(),
            amount_cents: 1900,
            currency: "CNY".to_string(),
            event_type: event_type.to_string(),
            occurred_at: Utc::now(),
            raw_payload_json: serde_json::json!({
                "provider": "fixture",
                "eventType": event_type,
                "providerOrderId": payment_session.provider_order_id,
            }),
        }
    }

    #[test]
    fn ingests_events_and_dedupes() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let batch = AnonymousFeedbackBatch {
            install_id: "inst-1".to_string(),
            session_id: "sess-1".to_string(),
            app_version: "0.1.0".to_string(),
            sent_at: Utc::now(),
            events: vec![
                sample_event("evt-1", AnonymousEventOutcome::Success),
                sample_event("evt-1", AnonymousEventOutcome::Success),
            ],
        };

        let ack = storage.ingest_batch(&batch).unwrap();
        assert_eq!(ack.received_events, 2);
        assert_eq!(ack.inserted_events, 1);
        assert_eq!(ack.duplicate_events, 1);
    }

    #[test]
    fn stats_group_by_day() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let batch = AnonymousFeedbackBatch {
            install_id: "inst-1".to_string(),
            session_id: "sess-1".to_string(),
            app_version: "0.1.0".to_string(),
            sent_at: Utc::now(),
            events: vec![sample_event("evt-2", AnonymousEventOutcome::Failure)],
        };
        storage.ingest_batch(&batch).unwrap();

        let stats = storage
            .query_stats(&AnonymousFeedbackStatsQuery::default())
            .unwrap();
        assert_eq!(stats.totals.total_events, 1);
        assert_eq!(stats.totals.failure_events, 1);
        assert_eq!(stats.rows.len(), 1);
    }

    #[test]
    fn commercial_metrics_overview_aggregates_without_media_identifiers() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&session, "creator"))
            .unwrap();
        let event =
            sample_fixture_event(&session, &payment, "payment.succeeded", "evt-pay-metrics");
        storage.apply_fixture_billing_event(&event).unwrap();

        let sync_event = CloudSyncBatchRequest {
            device_id: session.device.id.clone(),
            workspace_id: session.workspace.id.clone(),
            events: vec![crate::schema::CloudSyncEvent {
                client_event_id: "sync-metrics-1".to_string(),
                operation: "upsert".to_string(),
                entity_type: "vault_record".to_string(),
                entity_id: "record-1".to_string(),
                payload: serde_json::json!({
                    "watermarkUid": "wm-1",
                    "syncStatus": "pending"
                }),
            }],
        };
        storage
            .push_cloud_events_batch(&session.access_token, &sync_event)
            .unwrap();
        storage
            .create_video_fingerprint_notary(
                &session.access_token,
                &sample_l2_notary_request(&session),
            )
            .unwrap();

        let mut failed = sample_event("evt-metrics-failed", AnonymousEventOutcome::Failure);
        failed.feature_name = "cloud_sync_push".to_string();
        failed.error_code = Some("http_500".to_string());
        storage
            .ingest_batch(&AnonymousFeedbackBatch {
                install_id: "inst-1".to_string(),
                session_id: "sess-1".to_string(),
                app_version: "0.1.0".to_string(),
                sent_at: Utc::now(),
                events: vec![failed],
            })
            .unwrap();

        let metrics = storage.commercial_metrics_overview().unwrap();
        assert_eq!(metrics.accounts.total_accounts, 1);
        assert_eq!(metrics.payment_sessions.total, 1);
        assert_eq!(metrics.payment_sessions.succeeded, 1);
        assert_eq!(metrics.cloud_sync.accepted_events, 1);
        assert_eq!(metrics.cloud_sync.failure_events, 1);
        assert_eq!(metrics.feature_usage.l2_video_notary_count, 1);
        assert_eq!(
            metrics.anonymous_failures[0].feature_name,
            "cloud_sync_push"
        );
        assert_eq!(metrics.anonymous_failures[0].error_code, "http_500");
        assert!(metrics.privacy_boundary.excludes_original_media);
        assert!(metrics.privacy_boundary.excludes_watermarked_media);
        assert!(metrics.privacy_boundary.excludes_local_paths);
        assert!(metrics.privacy_boundary.excludes_file_names);
        assert!(metrics.privacy_boundary.excludes_full_media_hashes);
    }

    #[test]
    fn admin_audit_event_persists_without_secret_or_media_fields() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        storage
            .record_admin_audit_event(
                "/v1/commercial/metrics/overview",
                "denied",
                "admin_token_invalid",
            )
            .unwrap();

        let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
        let (endpoint, outcome, reason): (String, String, String) = conn
            .query_row(
                "SELECT endpoint, outcome, reason FROM admin_audit_events LIMIT 1",
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(endpoint, "/v1/commercial/metrics/overview");
        assert_eq!(outcome, "denied");
        assert_eq!(reason, "admin_token_invalid");
    }

    #[test]
    fn continue_account_returns_session_and_persists() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        assert_eq!(session.account.display_name, "alice@example.com");
        assert_eq!(session.workspace.name, "个人空间");
        assert_eq!(session.device.id, "dev-1");
        assert_eq!(session.creator_profile.is_default, true);
        assert_eq!(session.entitlement.plan_code, "free");
        assert_eq!(session.entitlement.features["cloud_sync"], false);
        assert_eq!(session.sync_policy, "blocked_by_entitlement");
        assert_eq!(session.cloud_vault_cursor, None);
    }

    #[test]
    fn continue_account_requires_matching_password_for_existing_account() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();

        let same_password = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-2"))
            .unwrap();
        assert_eq!(same_password.account.display_name, "alice@example.com");
        assert_eq!(same_password.device.id, "dev-2");

        let mut wrong_password = sample_continue_request("alice@example.com", "dev-3");
        wrong_password.password = "wrong-password".to_string();
        assert!(matches!(
            storage.continue_account(&wrong_password),
            Err(StorageError::Unauthorized)
        ));
    }

    #[test]
    fn password_sessions_store_argon2id_and_migrate_legacy_sha256() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let request = sample_continue_request("argon@example.com", "dev-1");
        storage.continue_account(&request).unwrap();
        {
            let conn = storage.conn.lock().unwrap();
            let (hash, algorithm): (String, String) = conn
                .query_row(
                    "SELECT password_hash, password_hash_algorithm FROM cloud_accounts WHERE identifier = ?1",
                    params!["argon@example.com"],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .unwrap();
            assert!(hash.starts_with("$argon2"));
            assert_eq!(algorithm, "argon2id");
        }

        let legacy_salt = new_password_salt();
        let legacy_hash = legacy_sha256_password_hash("legacy-password", &legacy_salt);
        {
            let conn = storage.conn.lock().unwrap();
            conn.execute(
                "UPDATE cloud_accounts
                 SET password_hash = ?2, password_salt = ?3, password_hash_algorithm = 'sha256'
                 WHERE identifier = ?1",
                params!["argon@example.com", legacy_hash, legacy_salt],
            )
            .unwrap();
        }
        let mut legacy_login = sample_continue_request("argon@example.com", "dev-2");
        legacy_login.password = "legacy-password".to_string();
        storage.continue_account(&legacy_login).unwrap();
        let conn = storage.conn.lock().unwrap();
        let (hash, algorithm): (String, String) = conn
            .query_row(
                "SELECT password_hash, password_hash_algorithm FROM cloud_accounts WHERE identifier = ?1",
                params!["argon@example.com"],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(hash.starts_with("$argon2"));
        assert_eq!(algorithm, "argon2id");
    }

    #[test]
    fn auth_challenge_and_login_rate_limits_are_enforced() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let request = AuthChallengeRequest {
            identifier: "limit@example.com".to_string(),
            purpose: "register_or_login".to_string(),
            client_device_id: "dev-limit".to_string(),
            captcha_token: None,
        };
        let first = storage.create_auth_challenge(&request).unwrap();
        assert_eq!(first.delivery_channel, "fixture");
        assert_eq!(first.fixture_code.as_deref(), Some("000000"));
        assert!(matches!(
            storage.create_auth_challenge(&request),
            Err(StorageError::RateLimited(_))
        ));

        let base = sample_continue_request("failed-login@example.com", "dev-login-limit");
        storage.continue_account(&base).unwrap();
        for _ in 0..5 {
            let mut wrong = sample_continue_request("failed-login@example.com", "dev-login-limit");
            wrong.password = "wrong-password".to_string();
            assert!(matches!(
                storage.continue_account(&wrong),
                Err(StorageError::Unauthorized) | Err(StorageError::RateLimited(_))
            ));
        }
        let mut correct = sample_continue_request("failed-login@example.com", "dev-login-limit");
        correct.password = "correct-password".to_string();
        assert!(matches!(
            storage.continue_account(&correct),
            Err(StorageError::RateLimited(_))
        ));
    }

    #[test]
    fn auth_challenge_session_me_refresh_and_logout_flow() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let challenge = storage
            .create_auth_challenge(&AuthChallengeRequest {
                identifier: "formal-auth@example.com".to_string(),
                purpose: "register_or_login".to_string(),
                client_device_id: "formal-dev-1".to_string(),
                captcha_token: None,
            })
            .unwrap();
        assert!(challenge.challenge_id.starts_with("chal_"));
        assert_eq!(challenge.delivery_channel, "fixture");

        let request = AuthSessionRequest {
            identifier: "formal-auth@example.com".to_string(),
            challenge_id: Some(challenge.challenge_id.clone()),
            verification_code: "000000".to_string(),
            password: String::new(),
            device: crate::schema::ContinueAccountDevice {
                client_device_id: "formal-dev-1".to_string(),
                name: "Formal Device".to_string(),
                platform: "contract".to_string(),
                app_version: "0.1.0".to_string(),
                public_key: None,
            },
            local_creator_profile: crate::schema::ContinueAccountCreatorProfile {
                display_name: "Formal Creator".to_string(),
                creator_seed_ref: "formal-seed-ref".to_string(),
                seed_envelope_version: 1,
            },
        };
        let session = storage.create_auth_session(&request).unwrap();
        assert!(session.access_token.starts_with("hsat_"));
        assert!(session.refresh_token.starts_with("hsrt_"));
        assert_eq!(session.sync_policy, "blocked_by_entitlement");

        let me = storage
            .current_account_snapshot(&session.access_token)
            .unwrap();
        assert_eq!(me.account.id, session.account.id);
        assert_eq!(me.device.id, session.device.id);
        assert_eq!(me.creator_profile.display_name, "Formal Creator");

        assert!(matches!(
            storage.create_auth_session(&request),
            Err(StorageError::Unauthorized)
        ));

        let refreshed = storage
            .refresh_auth_session(&AuthRefreshRequest {
                refresh_token: session.refresh_token.clone(),
                device_id: session.device.id.clone(),
            })
            .unwrap();
        assert_ne!(refreshed.access_token, session.access_token);
        assert_ne!(refreshed.refresh_token, session.refresh_token);
        assert!(matches!(
            storage.refresh_auth_session(&AuthRefreshRequest {
                refresh_token: session.refresh_token,
                device_id: session.device.id.clone(),
            }),
            Err(StorageError::Unauthorized)
        ));
        assert!(matches!(
            storage.current_account_snapshot(&session.access_token),
            Err(StorageError::Unauthorized)
        ));

        let logout = storage
            .logout_auth_session(&AuthLogoutRequest {
                refresh_token: refreshed.refresh_token.clone(),
                device_id: refreshed.device.id.clone(),
            })
            .unwrap();
        assert!(logout.ok);
        assert!(matches!(
            storage.current_account_snapshot(&refreshed.access_token),
            Err(StorageError::Unauthorized)
        ));
        assert!(matches!(
            storage.refresh_auth_session(&AuthRefreshRequest {
                refresh_token: refreshed.refresh_token,
                device_id: refreshed.device.id,
            }),
            Err(StorageError::Unauthorized)
        ));
    }

    #[test]
    fn device_management_lists_renames_and_revokes_other_device_sessions() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let first = storage
            .continue_account(&sample_continue_request("devices@example.com", "dev-1"))
            .unwrap();
        let second = storage
            .continue_account(&sample_continue_request("devices@example.com", "dev-2"))
            .unwrap();

        let devices = storage.list_devices(&first.access_token).unwrap();
        assert_eq!(devices.devices.len(), 2);
        assert!(devices
            .devices
            .iter()
            .any(|device| device.id == first.device.id && device.is_current));
        assert!(devices
            .devices
            .iter()
            .any(|device| device.id == second.device.id && !device.is_current));

        let renamed = storage
            .update_device(
                &first.access_token,
                &second.device.id,
                &UpdateDeviceRequest {
                    name: "Revocable Phone".to_string(),
                },
            )
            .unwrap();
        assert_eq!(renamed.name, "Revocable Phone");

        let revoked = storage
            .revoke_device(&first.access_token, &second.device.id)
            .unwrap();
        assert!(revoked.ok);
        assert_eq!(revoked.device_id, second.device.id);
        assert!(revoked.revoked_session_count >= 1);
        assert!(matches!(
            storage.current_account_snapshot(&second.access_token),
            Err(StorageError::Unauthorized)
        ));
        assert!(matches!(
            storage.revoke_device(&first.access_token, &first.device.id),
            Err(StorageError::BadRequest(_))
        ));

        let after = storage.list_devices(&first.access_token).unwrap();
        let revoked_device = after
            .devices
            .iter()
            .find(|device| device.id == second.device.id)
            .unwrap();
        assert!(!revoked_device.registered);
        assert_eq!(revoked_device.active_session_count, 0);
    }

    #[test]
    fn formal_auth_preserves_creator_manual_sync_policy_across_me_and_refresh() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let request = AuthSessionRequest {
            identifier: "formal-creator@example.com".to_string(),
            challenge_id: None,
            verification_code: String::new(),
            password: "correct-password".to_string(),
            device: crate::schema::ContinueAccountDevice {
                client_device_id: "creator-dev-1".to_string(),
                name: "Creator Device".to_string(),
                platform: "contract".to_string(),
                app_version: "0.1.0".to_string(),
                public_key: None,
            },
            local_creator_profile: crate::schema::ContinueAccountCreatorProfile {
                display_name: "Creator".to_string(),
                creator_seed_ref: "creator-seed-ref".to_string(),
                seed_envelope_version: 1,
            },
        };
        let session = storage.create_auth_session(&request).unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&session, "creator"))
            .unwrap();
        let paid =
            sample_fixture_event(&session, &payment, "payment.succeeded", "evt-auth-creator");
        storage.apply_fixture_billing_event(&paid).unwrap();

        let creator_session = storage.create_auth_session(&request).unwrap();
        assert_eq!(creator_session.sync_policy, "auto_cloud_vault");
        let paused = storage
            .update_sync_preferences(
                &creator_session.access_token,
                &SyncPreferencesRequest {
                    auto_sync_enabled: false,
                    reason: "user_paused".to_string(),
                },
            )
            .unwrap();
        assert_eq!(paused.sync_policy, "manual_local_only");

        let me = storage
            .current_account_snapshot(&creator_session.access_token)
            .unwrap();
        assert_eq!(me.sync_policy, "manual_local_only");

        let refreshed = storage
            .refresh_auth_session(&AuthRefreshRequest {
                refresh_token: creator_session.refresh_token,
                device_id: creator_session.device.id,
            })
            .unwrap();
        assert_eq!(refreshed.sync_policy, "manual_local_only");
        assert_eq!(refreshed.entitlement.plan_code, "creator");
    }

    #[test]
    fn push_and_pull_cloud_events_round_trip() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&session, "creator"))
            .unwrap();
        let event =
            sample_fixture_event(&session, &payment, "payment.succeeded", "evt-sync-creator");
        storage.apply_fixture_billing_event(&event).unwrap();
        let batch = CloudSyncBatchRequest {
            device_id: "dev-1".to_string(),
            workspace_id: session.workspace.id.clone(),
            events: vec![crate::schema::CloudSyncEvent {
                client_event_id: "evt-1".to_string(),
                operation: "upsertVaultRecord".to_string(),
                entity_type: "vaultRecord".to_string(),
                entity_id: "record-1".to_string(),
                payload: serde_json::json!({
                    "id": "record-1",
                    "kind": "image",
                    "title": "demo.png",
                    "watermark_uid": "uid-1",
                    "revision": 1,
                    "sha256": "hash-1",
                    "created_at": "2026-06-16T12:00:00Z"
                }),
            }],
        };
        let result = storage
            .push_cloud_events_batch(&session.access_token, &batch)
            .unwrap();
        assert_eq!(result.accepted, 1);
        assert_eq!(result.accepted_event_ids, vec!["evt-1".to_string()]);
        assert_eq!(result.event_results.len(), 1);
        assert_eq!(result.event_results[0].disposition, "accepted");
        assert!(result.event_results[0]
            .payload_hash
            .as_deref()
            .unwrap_or_default()
            .starts_with("sha256:"));
        assert_eq!(result.event_results[0].entity_revision, Some(1));

        let duplicate = storage
            .push_cloud_events_batch(&session.access_token, &batch)
            .unwrap();
        assert_eq!(duplicate.accepted, 1);
        assert_eq!(duplicate.event_results[0].disposition, "duplicate");

        let mut changed_batch = batch.clone();
        changed_batch.events[0].payload["revision"] = serde_json::json!(2);
        let conflict = storage
            .push_cloud_events_batch(&session.access_token, &changed_batch)
            .unwrap();
        assert_eq!(conflict.accepted, 0);
        assert!(conflict.accepted_event_ids.is_empty());
        assert_eq!(
            conflict.event_results[0].disposition,
            "conflict_payload_changed"
        );

        let changes = storage
            .get_cloud_changes(&session.access_token, Some(&session.workspace.id), None)
            .unwrap();
        assert_eq!(changes.changes.len(), 1);
        assert_eq!(changes.changes[0].entity_type, "vaultRecord");
        assert_eq!(changes.changes[0].operation, "upsert");
        assert_eq!(changes.changes[0].entity["watermark_uid"], "uid-1");
    }

    #[test]
    fn new_device_session_uses_device_cursor_before_first_pull() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let desktop = storage
            .continue_account(&sample_continue_request(
                "device-cursor@example.com",
                "desktop-dev",
            ))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&desktop, "creator"))
            .unwrap();
        let event =
            sample_fixture_event(&desktop, &payment, "payment.succeeded", "evt-device-cursor");
        storage.apply_fixture_billing_event(&event).unwrap();

        let batch = CloudSyncBatchRequest {
            device_id: desktop.device.id.clone(),
            workspace_id: desktop.workspace.id.clone(),
            events: vec![crate::schema::CloudSyncEvent {
                client_event_id: "evt-device-cursor-record".to_string(),
                operation: "upsertVaultRecord".to_string(),
                entity_type: "vaultRecord".to_string(),
                entity_id: "record-device-cursor".to_string(),
                payload: serde_json::json!({
                    "id": "record-device-cursor",
                    "kind": "image",
                    "title": "cursor-device.png",
                    "watermark_uid": "uid-device-cursor",
                    "revision": 1,
                    "created_at": "2026-06-16T12:00:00Z"
                }),
            }],
        };
        let pushed = storage
            .push_cloud_events_batch(&desktop.access_token, &batch)
            .unwrap();
        assert_eq!(pushed.next_cursor.as_deref(), Some("cursor_1"));

        let mobile = storage
            .continue_account(&sample_continue_request(
                "device-cursor@example.com",
                "mobile-dev",
            ))
            .unwrap();
        assert_eq!(mobile.entitlement.plan_code, "creator");
        assert_eq!(mobile.cloud_vault_cursor, None);

        let mobile_me = storage
            .current_account_snapshot(&mobile.access_token)
            .unwrap();
        assert_eq!(mobile_me.cloud_vault_cursor, None);

        let first_pull = storage
            .get_cloud_changes(
                &mobile.access_token,
                Some(&mobile.workspace.id),
                Some("cursor_999"),
            )
            .unwrap();
        assert_eq!(first_pull.next_cursor, "cursor_1");
        assert_eq!(first_pull.changes.len(), 1);
        assert_eq!(
            first_pull.changes[0].entity["watermark_uid"],
            "uid-device-cursor"
        );

        let mobile_after_pull = storage
            .current_account_snapshot(&mobile.access_token)
            .unwrap();
        assert_eq!(
            mobile_after_pull.cloud_vault_cursor.as_deref(),
            Some("cursor_1")
        );
    }

    #[test]
    fn free_cloud_sync_is_blocked_by_entitlement() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("free-sync@example.com", "dev-1"))
            .unwrap();
        let batch = CloudSyncBatchRequest {
            device_id: "dev-1".to_string(),
            workspace_id: session.workspace.id.clone(),
            events: vec![crate::schema::CloudSyncEvent {
                client_event_id: "evt-free-blocked".to_string(),
                operation: "upsertVaultRecord".to_string(),
                entity_type: "vaultRecord".to_string(),
                entity_id: "record-free-blocked".to_string(),
                payload: serde_json::json!({
                    "id": "record-free-blocked",
                    "watermark_uid": "uid-free-blocked"
                }),
            }],
        };

        assert!(matches!(
            storage.push_cloud_events_batch(&session.access_token, &batch),
            Err(StorageError::Forbidden)
        ));
        assert!(matches!(
            storage.get_cloud_changes(&session.access_token, Some(&session.workspace.id), None),
            Err(StorageError::Forbidden)
        ));
    }

    #[test]
    fn creator_continue_account_returns_auto_cloud_vault_policy() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let first = storage
            .continue_account(&sample_continue_request(
                "creator-sync@example.com",
                "dev-1",
            ))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&first, "creator"))
            .unwrap();
        let event = sample_fixture_event(&first, &payment, "payment.succeeded", "evt-sync-policy");
        storage.apply_fixture_billing_event(&event).unwrap();

        let second = storage
            .continue_account(&sample_continue_request(
                "creator-sync@example.com",
                "dev-2",
            ))
            .unwrap();
        assert_eq!(second.entitlement.plan_code, "creator");
        assert_eq!(second.entitlement.features["cloud_sync"], true);
        assert_eq!(second.sync_policy, "auto_cloud_vault");
    }

    #[test]
    fn creator_can_pause_and_resume_device_auto_cloud_sync() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let first = storage
            .continue_account(&sample_continue_request(
                "creator-sync-prefs@example.com",
                "dev-1",
            ))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&first, "creator"))
            .unwrap();
        let event = sample_fixture_event(&first, &payment, "payment.succeeded", "evt-sync-prefs");
        storage.apply_fixture_billing_event(&event).unwrap();

        let session = storage
            .continue_account(&sample_continue_request(
                "creator-sync-prefs@example.com",
                "dev-1",
            ))
            .unwrap();
        assert_eq!(session.sync_policy, "auto_cloud_vault");

        let paused = storage
            .update_sync_preferences(
                &session.access_token,
                &SyncPreferencesRequest {
                    auto_sync_enabled: false,
                    reason: "user_paused".to_string(),
                },
            )
            .unwrap();
        assert!(!paused.auto_sync_enabled);
        assert_eq!(paused.sync_policy, "manual_local_only");
        assert_eq!(paused.entitlement.features["cloud_sync"], true);

        let continued = storage
            .continue_account(&sample_continue_request(
                "creator-sync-prefs@example.com",
                "dev-1",
            ))
            .unwrap();
        assert_eq!(continued.sync_policy, "manual_local_only");

        let resumed = storage
            .update_sync_preferences(
                &continued.access_token,
                &SyncPreferencesRequest {
                    auto_sync_enabled: true,
                    reason: "user_resumed".to_string(),
                },
            )
            .unwrap();
        assert!(resumed.auto_sync_enabled);
        assert_eq!(resumed.sync_policy, "auto_cloud_vault");
    }

    #[test]
    fn free_cannot_resume_auto_cloud_sync() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request(
                "free-sync-prefs@example.com",
                "dev-1",
            ))
            .unwrap();

        assert!(matches!(
            storage.update_sync_preferences(
                &session.access_token,
                &SyncPreferencesRequest {
                    auto_sync_enabled: true,
                    reason: "user_resumed".to_string(),
                },
            ),
            Err(StorageError::Forbidden)
        ));
    }

    #[test]
    fn push_cloud_events_rejects_device_and_workspace_mismatch() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let event = crate::schema::CloudSyncEvent {
            client_event_id: "evt-guard".to_string(),
            operation: "upsertVaultRecord".to_string(),
            entity_type: "vaultRecord".to_string(),
            entity_id: "record-guard".to_string(),
            payload: serde_json::json!({
                "id": "record-guard",
                "kind": "image",
                "title": "guard.png",
                "watermark_uid": "uid-guard",
                "revision": 1,
                "created_at": "2026-06-16T12:00:00Z"
            }),
        };
        let wrong_device = CloudSyncBatchRequest {
            device_id: "dev-other".to_string(),
            workspace_id: session.workspace.id.clone(),
            events: vec![event.clone()],
        };
        assert!(matches!(
            storage.push_cloud_events_batch(&session.access_token, &wrong_device),
            Err(StorageError::Unauthorized)
        ));

        let wrong_workspace = CloudSyncBatchRequest {
            device_id: "dev-1".to_string(),
            workspace_id: "ws-other".to_string(),
            events: vec![event],
        };
        assert!(matches!(
            storage.push_cloud_events_batch(&session.access_token, &wrong_workspace),
            Err(StorageError::Forbidden)
        ));

        assert!(matches!(
            storage.get_cloud_changes(&session.access_token, Some("ws-other"), None),
            Err(StorageError::Forbidden)
        ));
    }

    #[test]
    fn watermark_id_reserve_is_idempotent_and_confirmable() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("wm-reg@example.com", "dev-1"))
            .unwrap();
        let request = sample_watermark_reserve_request(&session, "wm-request-1");

        let reserved = storage
            .reserve_watermark_id(&session.access_token, &request)
            .unwrap();
        assert_eq!(reserved.watermark_id_issue_mode, "server_reserved");
        assert_eq!(reserved.registry_status, "reserved");
        assert_eq!(reserved.payload_protocol_version, 2);
        assert_eq!(reserved.payload_bytes_length, 119);

        let duplicate = storage
            .reserve_watermark_id(&session.access_token, &request)
            .unwrap();
        assert_eq!(duplicate.watermark_uid, reserved.watermark_uid);
        assert_eq!(duplicate.registry_id, reserved.registry_id);

        let confirmed = storage
            .confirm_watermark_id(
                &session.access_token,
                &WatermarkIdConfirmRequest {
                    workspace_id: session.workspace.id.clone(),
                    creator_profile_id: session.creator_profile.id.clone(),
                    watermark_uid: reserved.watermark_uid.clone(),
                    payload_protocol_version: 2,
                    payload_bytes_length: 119,
                    original_hash: Some("sha256:original-a".to_string()),
                    protected_copy_hash: Some("sha256:protected-a".to_string()),
                    write_verification_status: "verified".to_string(),
                },
            )
            .unwrap();
        assert_eq!(confirmed.watermark_id_issue_mode, "server_confirmed");
        assert_eq!(confirmed.registry_status, "server_confirmed");
        assert!(confirmed
            .registry_receipt
            .contains(&confirmed.watermark_uid));
    }

    #[test]
    fn public_rights_query_uses_active_manifest_from_cloud_vault_payload() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("rights@example.com", "dev-1"))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&session, "creator"))
            .unwrap();
        let event = sample_fixture_event(&session, &payment, "payment.succeeded", "evt-rights");
        storage.apply_fixture_billing_event(&event).unwrap();
        let reserved = storage
            .reserve_watermark_id(
                &session.access_token,
                &sample_watermark_reserve_request(&session, "rights-wm-request"),
            )
            .unwrap();
        storage
            .confirm_watermark_id(
                &session.access_token,
                &WatermarkIdConfirmRequest {
                    workspace_id: session.workspace.id.clone(),
                    creator_profile_id: session.creator_profile.id.clone(),
                    watermark_uid: reserved.watermark_uid.clone(),
                    payload_protocol_version: 2,
                    payload_bytes_length: 119,
                    original_hash: Some("sha256:rights-original".to_string()),
                    protected_copy_hash: Some("sha256:rights-protected".to_string()),
                    write_verification_status: "verified".to_string(),
                },
            )
            .unwrap();

        let batch = CloudSyncBatchRequest {
            device_id: session.device.id.clone(),
            workspace_id: session.workspace.id.clone(),
            events: vec![crate::schema::CloudSyncEvent {
                client_event_id: "evt-rights-vault".to_string(),
                operation: "upsertVaultRecord".to_string(),
                entity_type: "vaultRecord".to_string(),
                entity_id: "record-rights".to_string(),
                payload: serde_json::json!({
                    "id": "record-rights",
                    "kind": "image",
                    "title": "rights.png",
                    "watermark_uid": reserved.watermark_uid,
                    "revision": 1,
                    "sha256": "rights-original",
                    "work_source_declaration": "ai_assisted",
                    "training_permission_declaration": "commercial_allowed",
                    "creation_method_declaration": "text_to_image",
                    "human_edit_level_declaration": "light",
                    "authenticity_claim_declaration": "synthetic",
                    "created_at": "2026-06-29T12:00:00Z"
                }),
            }],
        };
        let pushed = storage
            .push_cloud_events_batch(&session.access_token, &batch)
            .unwrap();
        assert_eq!(pushed.accepted, 1);

        let rights = storage
            .public_rights_query(&reserved.watermark_uid)
            .unwrap();
        assert_eq!(rights.scan_status, "registry_active");
        assert_eq!(
            rights.training_permission.policy,
            "commercial_training_allowed"
        );
        assert!(!rights.training_permission.legal_conclusion);
        let manifest = rights.rights_manifest.unwrap();
        assert_eq!(manifest.status, "active");
        assert_eq!(manifest.work_source_declaration, "ai_assisted");
        assert!(!manifest.manifest_sha256.is_empty());
        assert!(!manifest.signature.is_empty());
    }

    #[test]
    fn public_rights_metadata_export_maps_registry_manifest_to_c2pa_iptc_xmp_jsonld() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request(
                "rights-metadata@example.com",
                "dev-1",
            ))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&session, "creator"))
            .unwrap();
        let event = sample_fixture_event(&session, &payment, "payment.succeeded", "evt-rights-md");
        storage.apply_fixture_billing_event(&event).unwrap();
        let reserved = storage
            .reserve_watermark_id(
                &session.access_token,
                &sample_watermark_reserve_request(&session, "rights-md-request"),
            )
            .unwrap();
        storage
            .confirm_watermark_id(
                &session.access_token,
                &WatermarkIdConfirmRequest {
                    workspace_id: session.workspace.id.clone(),
                    creator_profile_id: session.creator_profile.id.clone(),
                    watermark_uid: reserved.watermark_uid.clone(),
                    payload_protocol_version: 2,
                    payload_bytes_length: 119,
                    original_hash: Some("sha256:rights-md-original".to_string()),
                    protected_copy_hash: Some("sha256:rights-md-protected".to_string()),
                    write_verification_status: "verified".to_string(),
                },
            )
            .unwrap();
        storage
            .push_cloud_events_batch(
                &session.access_token,
                &CloudSyncBatchRequest {
                    device_id: session.device.id.clone(),
                    workspace_id: session.workspace.id.clone(),
                    events: vec![crate::schema::CloudSyncEvent {
                        client_event_id: "evt-rights-md-vault".to_string(),
                        operation: "upsertVaultRecord".to_string(),
                        entity_type: "vaultRecord".to_string(),
                        entity_id: "record-rights-md".to_string(),
                        payload: serde_json::json!({
                            "id": "record-rights-md",
                            "kind": "image",
                            "title": "rights-md.png",
                            "watermark_uid": reserved.watermark_uid,
                            "revision": 1,
                            "sha256": "rights-md-original",
                            "work_source_declaration": "ai_assisted",
                            "training_permission_declaration": "commercial_allowed",
                            "creation_method_declaration": "text_to_image",
                            "human_edit_level_declaration": "light",
                            "authenticity_claim_declaration": "synthetic",
                            "custom_terms_url": "https://example.test/rights",
                            "custom_terms_hash": "sha256:terms",
                            "created_at": "2026-06-29T12:00:00Z"
                        }),
                    }],
                },
            )
            .unwrap();

        let exported = storage
            .public_rights_metadata_export(&reserved.watermark_uid)
            .unwrap();
        assert_eq!(exported.watermark_uid, reserved.watermark_uid);
        assert!(!exported.legal_conclusion);
        assert_eq!(exported.content_credentials["embeddedInMedia"], false);
        assert_eq!(
            exported.c2pa_assertions[0]["data"]["trainingPolicy"],
            "commercial_training_allowed"
        );
        assert_eq!(exported.iptc["dataMining"], "allowed-commercial");
        assert_eq!(
            exported.xmp["hiddenShield:TrainingPolicy"],
            "commercial_training_allowed"
        );
        assert_eq!(
            exported.json_ld["hs:trainingPolicy"],
            "commercial_training_allowed"
        );
        assert_eq!(
            exported.json_ld["schema:license"],
            "https://example.test/rights"
        );
    }

    #[test]
    fn public_rights_batch_returns_record_level_errors() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request(
                "rights-batch@example.com",
                "dev-1",
            ))
            .unwrap();
        let reserved = storage
            .reserve_watermark_id(
                &session.access_token,
                &sample_watermark_reserve_request(&session, "rights-batch-request"),
            )
            .unwrap();

        let result = storage
            .public_rights_batch(&PublicRightsBatchRequest {
                watermark_uids: vec![
                    reserved.watermark_uid.clone(),
                    "not-a-watermark".to_string(),
                ],
            })
            .unwrap();
        assert_eq!(result.results.len(), 2);
        assert_eq!(result.results[0].status, "ok");
        assert_eq!(
            result.results[0].result.as_ref().unwrap().scan_status,
            "watermark_only"
        );
        assert_eq!(result.results[1].status, "error");
        assert_eq!(
            result.results[1].error_code.as_deref(),
            Some("watermark_uid_invalid")
        );
    }

    #[test]
    fn public_rights_sdk_contract_freezes_batch_limit_and_error_codes() {
        assert_eq!(crate::schema::PUBLIC_RIGHTS_ANONYMOUS_BATCH_MAX_ITEMS, 100);
        for code in [
            "not_found",
            "registry_unavailable",
            "payload_invalid",
            "manifest_conflict",
            "backfill_pending",
            "backfill_disputed",
            "rate_limited",
            "watermark_uid_invalid",
            "internal_error",
        ] {
            assert!(
                crate::schema::PUBLIC_RIGHTS_STABLE_ERROR_CODES.contains(&code),
                "{code} must stay in the public rights SDK stable error set"
            );
        }
    }

    #[test]
    fn enterprise_public_rights_internal_models_persist_without_external_api() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let key = storage
            .create_enterprise_api_key_internal(&EnterpriseApiKeyCreateRequest {
                account_id: "acct_enterprise".to_string(),
                workspace_id: "ws_enterprise".to_string(),
                creator_profile_id: None,
                name: "Rights scanner key".to_string(),
                key_prefix: "hsent_live_1234".to_string(),
                key_hash: "hash_only_no_plain_secret".to_string(),
                scopes: vec![
                    "public_rights:read".to_string(),
                    "public_rights:batch_read".to_string(),
                ],
                created_by_account_id: "acct_admin".to_string(),
                expires_at: None,
            })
            .unwrap();
        assert_eq!(key.status, "active");
        assert_eq!(key.key_prefix, "hsent_live_1234");

        let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
        let stored_hash: String = conn
            .query_row(
                "SELECT key_hash FROM enterprise_api_keys WHERE api_key_id = ?1",
                params![key.api_key_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_hash, "hash_only_no_plain_secret");
        drop(conn);

        let ledger = storage
            .record_enterprise_quota_ledger_internal(&EnterpriseQuotaLedgerRequest {
                account_id: "acct_enterprise".to_string(),
                workspace_id: "ws_enterprise".to_string(),
                api_key_id: Some(key.api_key_id.clone()),
                quota_type: "public_rights_scan_units".to_string(),
                units: 25,
                direction: "debit".to_string(),
                event_type: "scan_batch".to_string(),
                reference_id: "req_1".to_string(),
                idempotency_key: "idem_1".to_string(),
                status: "committed".to_string(),
            })
            .unwrap();
        assert_eq!(ledger.quota_type, "public_rights_scan_units");
        assert_eq!(ledger.status, "committed");
        assert!(ledger.committed_at.is_some());

        let audit_id = storage
            .record_enterprise_api_audit_event_internal(&EnterpriseApiAuditEventRequest {
                account_id: "acct_enterprise".to_string(),
                workspace_id: "ws_enterprise".to_string(),
                api_key_id: Some(key.api_key_id),
                endpoint: "/v1/enterprise/public-rights/batch".to_string(),
                method: "POST".to_string(),
                request_count: 1,
                item_count: 25,
                status_code: 200,
                error_code: None,
                quota_units: 25,
                client_label: Some("contract-test".to_string()),
                client_fingerprint_hash: None,
                trusted_proxy_status: None,
                request_id: "req_1".to_string(),
            })
            .unwrap();
        assert!(audit_id.starts_with("eae_"));
    }

    #[test]
    fn enterprise_public_rights_internal_models_reject_write_scopes_and_duplicate_quota() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let rejected = storage.create_enterprise_api_key_internal(&EnterpriseApiKeyCreateRequest {
            account_id: "acct_enterprise".to_string(),
            workspace_id: "ws_enterprise".to_string(),
            creator_profile_id: None,
            name: "Bad key".to_string(),
            key_prefix: "hsent_live_bad".to_string(),
            key_hash: "hash_only".to_string(),
            scopes: vec!["rights_manifest:write".to_string()],
            created_by_account_id: "acct_admin".to_string(),
            expires_at: None,
        });
        assert!(matches!(rejected, Err(StorageError::BadRequest(_))));

        let request = EnterpriseQuotaLedgerRequest {
            account_id: "acct_enterprise".to_string(),
            workspace_id: "ws_enterprise".to_string(),
            api_key_id: None,
            quota_type: "public_rights_scan_units".to_string(),
            units: 1,
            direction: "debit".to_string(),
            event_type: "scan_batch".to_string(),
            reference_id: "req_duplicate".to_string(),
            idempotency_key: "idem_duplicate".to_string(),
            status: "committed".to_string(),
        };
        storage
            .record_enterprise_quota_ledger_internal(&request)
            .unwrap();
        assert!(matches!(
            storage.record_enterprise_quota_ledger_internal(&request),
            Err(StorageError::Database(_))
        ));
    }

    #[test]
    fn enterprise_public_rights_external_batch_charges_quota_and_audits() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request(
                "enterprise-scan@example.com",
                "dev-1",
            ))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&session, "creator"))
            .unwrap();
        let paid = sample_fixture_event(
            &session,
            &payment,
            "payment.succeeded",
            "evt-enterprise-scan",
        );
        storage.apply_fixture_billing_event(&paid).unwrap();
        let reserved = storage
            .reserve_watermark_id(
                &session.access_token,
                &sample_watermark_reserve_request(&session, "enterprise-scan-reserve"),
            )
            .unwrap();
        let _ = storage
            .confirm_watermark_id(
                &session.access_token,
                &WatermarkIdConfirmRequest {
                    workspace_id: session.workspace.id.clone(),
                    creator_profile_id: session.creator_profile.id.clone(),
                    watermark_uid: reserved.watermark_uid.clone(),
                    payload_protocol_version: reserved.payload_protocol_version,
                    payload_bytes_length: reserved.payload_bytes_length,
                    original_hash: Some("sha256:original-a".to_string()),
                    protected_copy_hash: Some("sha256:protected-a".to_string()),
                    write_verification_status: "verified".to_string(),
                },
            )
            .unwrap();
        let batch = CloudSyncBatchRequest {
            device_id: session.device.id.clone(),
            workspace_id: session.workspace.id.clone(),
            events: vec![crate::schema::CloudSyncEvent {
                client_event_id: "enterprise-scan-sync".to_string(),
                operation: "upsertVaultRecord".to_string(),
                entity_type: "vaultRecord".to_string(),
                entity_id: "enterprise-scan-record".to_string(),
                payload: serde_json::json!({
                    "id": "enterprise-scan-record",
                    "kind": "image",
                    "title": "enterprise-scan.png",
                    "watermark_uid": reserved.watermark_uid,
                    "revision": 1,
                    "sha256": "sha256:original-a",
                    "protected_copy_hash": "sha256:protected-a",
                    "payload_protocol_version": reserved.payload_protocol_version,
                    "payload_bytes_length": reserved.payload_bytes_length,
                    "payload_auth_status": "verified",
                    "work_source_declaration": "ai_assisted",
                    "training_permission_declaration": "commercial_allowed",
                    "creation_method_declaration": "text_to_image",
                    "human_edit_level_declaration": "light",
                    "authenticity_claim_declaration": "synthetic",
                    "created_at": Utc::now().to_rfc3339()
                }),
            }],
        };
        storage
            .push_cloud_events_batch(&session.access_token, &batch)
            .unwrap();
        let cleartext_key = "hsent_live_test_secret_extra";
        let hash_secret = "enterprise-test-hash-secret";
        let hash_secret_version = "storage-test-v1";
        let key_hash =
            enterprise_api_key_hash_hex(cleartext_key, hash_secret, hash_secret_version).unwrap();
        let key = storage
            .create_enterprise_api_key_internal(&EnterpriseApiKeyCreateRequest {
                account_id: "acct_enterprise".to_string(),
                workspace_id: "ws_enterprise".to_string(),
                creator_profile_id: None,
                name: "External batch scanner".to_string(),
                key_prefix: cleartext_key.chars().take(22).collect::<String>(),
                key_hash,
                scopes: vec!["public_rights:batch_read".to_string()],
                created_by_account_id: "acct_admin".to_string(),
                expires_at: None,
            })
            .unwrap();
        let now = Utc::now();
        storage
            .initialize_enterprise_quota_balance_internal(&EnterpriseQuotaBalanceInitRequest {
                account_id: "acct_enterprise".to_string(),
                workspace_id: "ws_enterprise".to_string(),
                quota_type: ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE.to_string(),
                period_start: now - Duration::hours(1),
                period_end: now + Duration::hours(1),
                included_units: 10,
                overage_allowed: false,
                overage_unit_price_cents: None,
                currency: "USD".to_string(),
            })
            .unwrap();

        let response = storage
            .enterprise_public_rights_batch(
                cleartext_key,
                hash_secret,
                hash_secret_version,
                EnterpriseGatewayClientFingerprint::default(),
                &EnterprisePublicRightsBatchRequest {
                    watermark_uids: vec![
                        reserved.watermark_uid.clone(),
                        "wm_0000000000000000000000000000000000000000".to_string(),
                    ],
                    idempotency_key: Some("enterprise-scan-request".to_string()),
                    client_label: Some("contract-test".to_string()),
                },
            )
            .unwrap();
        assert_eq!(response.gateway.api_key_id, key.api_key_id);
        assert_eq!(response.gateway.quota_charged_units, 2);
        assert_eq!(response.gateway.legal_conclusion, false);
        assert_eq!(response.batch.results.len(), 2);
        assert_eq!(response.batch.results[0].status, "ok");
        assert_eq!(
            response.batch.results[1].error_code.as_deref(),
            Some("watermark_uid_invalid")
        );

        let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
        let used_units: i64 = conn
            .query_row(
                "SELECT used_units FROM enterprise_quota_balances
                 WHERE account_id = 'acct_enterprise' AND workspace_id = 'ws_enterprise'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM enterprise_api_audit_events
                 WHERE api_key_id = ?1 AND request_id = 'enterprise-scan-request'",
                params![key.api_key_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(used_units, 2);
        assert_eq!(audit_count, 1);
    }

    #[test]
    fn enterprise_gateway_readonly_contract_freezes_auth_rate_limit_quota_and_audit() {
        use crate::schema::{
            EnterpriseGatewayAuditContract, EnterpriseGatewayAuthContext,
            EnterpriseGatewayQuotaChargePlan, EnterpriseGatewayRateLimitPolicy,
            EnterpriseGatewayReadOnlyScanContract, ENTERPRISE_GATEWAY_REQUIRED_STEPS,
            ENTERPRISE_GATEWAY_STABLE_ERROR_CODES,
        };

        for step in [
            "authenticate_api_key",
            "authorize_scope",
            "check_entitlement_api_access",
            "apply_rate_limit",
            "resolve_readonly_public_rights",
            "record_quota_ledger",
            "record_api_audit_event",
        ] {
            assert!(
                ENTERPRISE_GATEWAY_REQUIRED_STEPS.contains(&step),
                "{step} must remain part of the Enterprise gateway contract"
            );
        }

        for code in [
            "enterprise_api_closed",
            "api_key_invalid",
            "api_access_disabled",
            "rate_limited",
            "quota_exhausted",
            "watermark_uid_invalid",
            "not_found",
            "registry_unavailable",
        ] {
            assert!(
                ENTERPRISE_GATEWAY_STABLE_ERROR_CODES.contains(&code),
                "{code} must remain stable for future Enterprise API clients"
            );
        }

        let contract = EnterpriseGatewayReadOnlyScanContract {
            auth: EnterpriseGatewayAuthContext {
                api_key_id: "eak_contract".to_string(),
                account_id: "acct_enterprise".to_string(),
                workspace_id: "ws_enterprise".to_string(),
                key_prefix: "hsent_live".to_string(),
                scopes: vec!["public_rights:batch_read".to_string()],
                status: "active".to_string(),
                api_access: false,
            },
            rate_limit: EnterpriseGatewayRateLimitPolicy {
                policy_id: "enterprise_public_rights_default".to_string(),
                requests_per_minute: 60,
                items_per_minute: 600,
                burst_requests: 10,
                retry_after_seconds: 60,
            },
            quota: EnterpriseGatewayQuotaChargePlan {
                quota_type: ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE.to_string(),
                chargeable_units: 2,
                idempotency_key: "req_contract:public_rights_scan_units".to_string(),
                ledger_status: "committed".to_string(),
                charge_on_not_found: false,
                charge_metadata_export: false,
            },
            audit: EnterpriseGatewayAuditContract {
                endpoint: "/v1/enterprise/public-rights/batch".to_string(),
                method: "POST".to_string(),
                request_id: "req_contract".to_string(),
                request_count: 1,
                item_count: 2,
                status_code: 403,
                error_code: Some("enterprise_api_closed".to_string()),
                quota_units: 0,
                client_fingerprint: EnterpriseGatewayClientFingerprint {
                    fingerprint_hash: "sha256:client-fingerprint".to_string(),
                    source: "trusted_proxy_x_forwarded_for".to_string(),
                    trusted_proxy: true,
                    rate_limit_subject: "eak_contract:sha256:client-fingerprint".to_string(),
                },
                legal_conclusion: false,
            },
            required_steps: ENTERPRISE_GATEWAY_REQUIRED_STEPS
                .iter()
                .map(|step| (*step).to_string())
                .collect(),
        };

        assert!(!contract.auth.api_access);
        assert_eq!(
            contract.quota.quota_type,
            ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE
        );
        assert_eq!(contract.audit.legal_conclusion, false);
        assert!(contract.audit.endpoint.starts_with("/v1/enterprise/"));
        assert_eq!(
            contract.audit.error_code.as_deref(),
            Some("enterprise_api_closed")
        );
    }

    fn sample_enterprise_gateway_dry_run_request() -> EnterpriseGatewayDryRunRequest {
        use crate::schema::{EnterpriseGatewayAuthContext, EnterpriseGatewayRateLimitPolicy};

        EnterpriseGatewayDryRunRequest {
            auth: EnterpriseGatewayAuthContext {
                api_key_id: "eak_dry_run".to_string(),
                account_id: "acct_enterprise".to_string(),
                workspace_id: "ws_enterprise".to_string(),
                key_prefix: "hsent_live".to_string(),
                scopes: vec!["public_rights:batch_read".to_string()],
                status: "active".to_string(),
                api_access: true,
            },
            required_scope: "public_rights:batch_read".to_string(),
            endpoint: "/v1/enterprise/public-rights/batch".to_string(),
            method: "post".to_string(),
            request_id: "req_dry_run".to_string(),
            item_count: 3,
            quota_type: ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE.to_string(),
            quota_included_units: 100,
            quota_used_units: 10,
            quota_reserved_units: 5,
            quota_overage_allowed: false,
            rate_limit: EnterpriseGatewayRateLimitPolicy {
                policy_id: "enterprise_public_rights_default".to_string(),
                requests_per_minute: 60,
                items_per_minute: 600,
                burst_requests: 10,
                retry_after_seconds: 60,
            },
            client_fingerprint: EnterpriseGatewayClientFingerprint {
                fingerprint_hash: "sha256:dry-run-client".to_string(),
                source: "trusted_proxy_x_hiddenshield_client_fingerprint".to_string(),
                trusted_proxy: true,
                rate_limit_subject: "eak_dry_run:sha256:dry-run-client".to_string(),
            },
            current_window_requests: 5,
            current_window_items: 20,
            charge_on_not_found: false,
            charge_metadata_export: false,
        }
    }

    #[test]
    fn enterprise_gateway_dry_run_helper_outputs_auth_rate_limit_quota_and_audit_decisions() {
        let request = sample_enterprise_gateway_dry_run_request();
        let decision = dry_run_enterprise_gateway_readonly_scan(&request);

        assert!(decision.allowed);
        assert_eq!(decision.status_code, 200);
        assert_eq!(decision.error_code, None);
        assert_eq!(decision.auth_decision, "passed");
        assert_eq!(decision.scope_decision, "passed");
        assert_eq!(decision.entitlement_decision, "passed");
        assert_eq!(decision.rate_limit_decision, "passed");
        assert_eq!(decision.quota_decision, "passed");
        assert_eq!(decision.quota.chargeable_units, 3);
        assert_eq!(decision.quota.ledger_status, "committed");
        assert_eq!(decision.audit.status_code, 200);
        assert_eq!(decision.audit.quota_units, 3);
        assert_eq!(decision.audit.method, "POST");
        assert_eq!(
            decision.audit.client_fingerprint.fingerprint_hash,
            "sha256:dry-run-client"
        );
        assert!(decision.audit.client_fingerprint.trusted_proxy);
        assert_eq!(decision.audit.legal_conclusion, false);
        assert_eq!(decision.legal_conclusion, false);
        assert_eq!(
            decision.required_steps,
            ENTERPRISE_GATEWAY_REQUIRED_STEPS
                .iter()
                .map(|step| (*step).to_string())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn enterprise_gateway_dry_run_helper_denies_without_charging_or_legal_conclusion() {
        let cases = vec![
            ("scope_denied", 403, {
                let mut request = sample_enterprise_gateway_dry_run_request();
                request.required_scope = "public_rights:metadata_export".to_string();
                request
            }),
            ("api_access_disabled", 403, {
                let mut request = sample_enterprise_gateway_dry_run_request();
                request.auth.api_access = false;
                request
            }),
            ("rate_limited", 429, {
                let mut request = sample_enterprise_gateway_dry_run_request();
                request.current_window_requests = 70;
                request
            }),
            ("quota_exhausted", 402, {
                let mut request = sample_enterprise_gateway_dry_run_request();
                request.quota_included_units = 12;
                request.quota_used_units = 10;
                request.quota_reserved_units = 0;
                request
            }),
            ("api_key_revoked", 403, {
                let mut request = sample_enterprise_gateway_dry_run_request();
                request.auth.status = "revoked".to_string();
                request
            }),
            ("quota_contract_missing", 403, {
                let mut request = sample_enterprise_gateway_dry_run_request();
                request.quota_type = "video_minutes".to_string();
                request
            }),
        ];

        for (error_code, status_code, request) in cases {
            let decision = dry_run_enterprise_gateway_readonly_scan(&request);
            assert!(!decision.allowed, "{error_code} must deny the dry run");
            assert_eq!(decision.status_code, status_code);
            assert_eq!(decision.error_code.as_deref(), Some(error_code));
            assert_eq!(decision.quota.chargeable_units, 0);
            assert_eq!(decision.quota.ledger_status, "skipped");
            assert_eq!(decision.audit.quota_units, 0);
            assert_eq!(decision.audit.error_code.as_deref(), Some(error_code));
            assert_eq!(decision.audit.legal_conclusion, false);
            assert_eq!(decision.legal_conclusion, false);
        }
    }

    #[test]
    fn enterprise_quota_balance_initialization_is_idempotent_without_resetting_usage() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let period_start = Utc::now();
        let period_end = period_start + Duration::days(30);
        let request = EnterpriseQuotaBalanceInitRequest {
            account_id: "acct_enterprise".to_string(),
            workspace_id: "ws_enterprise".to_string(),
            quota_type: "public_rights_scan_units".to_string(),
            period_start,
            period_end,
            included_units: 10_000,
            overage_allowed: false,
            overage_unit_price_cents: None,
            currency: "cny".to_string(),
        };
        let first = storage
            .initialize_enterprise_quota_balance_internal(&request)
            .unwrap();
        assert_eq!(first.included_units, 10_000);
        assert_eq!(first.used_units, 0);
        assert_eq!(first.reserved_units, 0);
        assert_eq!(first.currency, "CNY");

        {
            let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute(
                "UPDATE enterprise_quota_balances
                 SET used_units = 7, reserved_units = 3
                 WHERE quota_balance_id = ?1",
                params![first.quota_balance_id],
            )
            .unwrap();
        }

        let second = storage
            .initialize_enterprise_quota_balance_internal(&EnterpriseQuotaBalanceInitRequest {
                included_units: 20_000,
                overage_allowed: true,
                overage_unit_price_cents: Some(2),
                currency: "usd".to_string(),
                ..request
            })
            .unwrap();
        assert_eq!(second.quota_balance_id, first.quota_balance_id);
        assert_eq!(second.included_units, 20_000);
        assert_eq!(second.used_units, 7);
        assert_eq!(second.reserved_units, 3);
        assert_eq!(second.overage_allowed, true);
        assert_eq!(second.overage_unit_price_cents, Some(2));
        assert_eq!(second.currency, "USD");
    }

    #[test]
    fn enterprise_quota_balance_initialization_rejects_invalid_contract_shape() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let period_start = Utc::now();
        let invalid = storage.initialize_enterprise_quota_balance_internal(
            &EnterpriseQuotaBalanceInitRequest {
                account_id: "acct_enterprise".to_string(),
                workspace_id: "ws_enterprise".to_string(),
                quota_type: "video_minutes".to_string(),
                period_start,
                period_end: period_start + Duration::days(30),
                included_units: 10_000,
                overage_allowed: false,
                overage_unit_price_cents: None,
                currency: "CNY".to_string(),
            },
        );
        assert!(matches!(
            invalid,
            Err(StorageError::BadRequest(message))
                if message == "enterprise quota balance request is invalid"
        ));
    }

    #[test]
    fn enterprise_api_key_internal_list_get_pause_and_revoke_work_without_hash_exposure() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let first = storage
            .create_enterprise_api_key_internal(&EnterpriseApiKeyCreateRequest {
                account_id: "acct_enterprise".to_string(),
                workspace_id: "ws_enterprise".to_string(),
                creator_profile_id: Some("creator_enterprise".to_string()),
                name: "Primary scanner".to_string(),
                key_prefix: "hs_live_abcd".to_string(),
                key_hash: "sha256:key-hash-primary".to_string(),
                scopes: vec![
                    "public_rights:read".to_string(),
                    "public_rights:batch_read".to_string(),
                ],
                created_by_account_id: "acct_admin".to_string(),
                expires_at: None,
            })
            .unwrap();
        let second = storage
            .create_enterprise_api_key_internal(&EnterpriseApiKeyCreateRequest {
                account_id: "acct_other".to_string(),
                workspace_id: "ws_other".to_string(),
                creator_profile_id: None,
                name: "Other scanner".to_string(),
                key_prefix: "hs_live_efgh".to_string(),
                key_hash: "sha256:key-hash-other".to_string(),
                scopes: vec!["public_rights:read".to_string()],
                created_by_account_id: "acct_admin".to_string(),
                expires_at: None,
            })
            .unwrap();

        let listed = storage
            .list_enterprise_api_keys_internal(&EnterpriseApiKeyListQuery {
                account_id: Some("acct_enterprise".to_string()),
                workspace_id: None,
                status: Some("active".to_string()),
                limit: Some(10),
            })
            .unwrap();
        assert_eq!(listed.returned, 1);
        assert_eq!(listed.api_keys[0].api_key_id, first.api_key_id);
        assert_eq!(listed.api_keys[0].key_prefix, "hs_live_abcd");
        assert!(!listed
            .api_keys
            .iter()
            .any(|record| record.api_key_id == second.api_key_id));

        let fetched = storage
            .get_enterprise_api_key_internal(&first.api_key_id)
            .unwrap();
        assert_eq!(fetched.name, "Primary scanner");
        assert_eq!(fetched.scopes.len(), 2);

        let paused = storage
            .pause_enterprise_api_key_internal(&first.api_key_id, "contract review")
            .unwrap();
        assert_eq!(paused.status, "paused");
        assert_eq!(paused.revoked_at, None);
        assert_eq!(paused.revoked_reason, None);

        let revoked = storage
            .revoke_enterprise_api_key_internal(&first.api_key_id, "customer offboarded")
            .unwrap();
        assert_eq!(revoked.status, "revoked");
        assert!(revoked.revoked_at.is_some());
        assert_eq!(
            revoked.revoked_reason.as_deref(),
            Some("customer offboarded")
        );

        let revoked_again = storage
            .revoke_enterprise_api_key_internal(&first.api_key_id, "duplicate revoke")
            .unwrap();
        assert_eq!(revoked_again.status, "revoked");
        assert_eq!(
            revoked_again.revoked_reason.as_deref(),
            Some("customer offboarded")
        );

        assert!(matches!(
            storage.pause_enterprise_api_key_internal(&first.api_key_id, "should fail"),
            Err(StorageError::BadRequest(message)) if message == "enterprise api key is revoked"
        ));

        let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
        let stored_hash: String = conn
            .query_row(
                "SELECT key_hash FROM enterprise_api_keys WHERE api_key_id = ?1",
                params![first.api_key_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_hash, "sha256:key-hash-primary");
    }

    #[test]
    fn enterprise_admin_audit_events_record_specific_internal_operations() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let audit_id = storage
            .record_enterprise_admin_audit_event_internal(
                "issue_api_key",
                "succeeded",
                "/internal/enterprise/api-key-issuances",
                Some("acct_enterprise"),
                Some("ws_enterprise"),
                Some("eak_test"),
                Some("eak_test"),
                "customer onboarding",
                serde_json::json!({"keyPrefix":"hsent_live_abc", "shownOnce":true}),
            )
            .unwrap();
        assert!(audit_id.starts_with("eaa_"));
        assert_eq!(
            storage
                .enterprise_admin_audit_event_count_for_tests("issue_api_key", "succeeded")
                .unwrap(),
            1
        );
        assert_eq!(
            storage
                .latest_enterprise_admin_audit_reason_for_tests("issue_api_key")
                .unwrap()
                .as_deref(),
            Some("customer onboarding")
        );
        assert!(matches!(
            storage.record_enterprise_admin_audit_event_internal(
                "scan_public_rights",
                "succeeded",
                "/internal/enterprise/unknown",
                None,
                None,
                None,
                None,
                "not allowed",
                serde_json::json!({})
            ),
            Err(StorageError::BadRequest(message))
                if message == "enterprise admin audit event is invalid"
        ));
    }

    #[test]
    fn enterprise_admin_audit_events_can_be_filtered_read_only() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let before = Utc::now() - Duration::seconds(1);
        storage
            .record_enterprise_admin_audit_event_internal(
                "create_api_key",
                "succeeded",
                "/internal/enterprise/api-keys",
                Some("acct_enterprise"),
                Some("ws_enterprise"),
                Some("eak_create"),
                Some("eak_create"),
                "created",
                serde_json::json!({"keyPrefix":"hs_live_abcd"}),
            )
            .unwrap();
        storage
            .record_enterprise_admin_audit_event_internal(
                "pause_api_key",
                "failed",
                "/internal/enterprise/api-keys/:api_key_id/pause",
                Some("acct_enterprise"),
                Some("ws_enterprise"),
                Some("eak_pause"),
                Some("eak_pause"),
                "enterprise api key is revoked",
                serde_json::json!({"requestReason":"should fail"}),
            )
            .unwrap();
        storage
            .record_enterprise_admin_audit_event_internal(
                "init_quota_balance",
                "succeeded",
                "/internal/enterprise/quota-balances",
                Some("acct_other"),
                Some("ws_other"),
                None,
                Some("eqb_test"),
                "initialized",
                serde_json::json!({"quotaType":"public_rights_scan_units"}),
            )
            .unwrap();
        let after = Utc::now() + Duration::seconds(1);

        let filtered = storage
            .list_enterprise_admin_audit_events_internal(&EnterpriseAdminAuditEventQuery {
                operation: Some("pause_api_key".to_string()),
                outcome: Some("failed".to_string()),
                account_id: Some("acct_enterprise".to_string()),
                api_key_id: Some("eak_pause".to_string()),
                from_occurred_at: Some(before),
                to_occurred_at: Some(after),
                limit: Some(10),
            })
            .unwrap();
        assert_eq!(filtered.returned, 1);
        assert_eq!(filtered.events[0].operation, "pause_api_key");
        assert_eq!(filtered.events[0].outcome, "failed");
        assert_eq!(filtered.events[0].api_key_id.as_deref(), Some("eak_pause"));
        assert_eq!(
            filtered.events[0].details["requestReason"],
            serde_json::json!("should fail")
        );

        let limited = storage
            .list_enterprise_admin_audit_events_internal(&EnterpriseAdminAuditEventQuery {
                operation: None,
                outcome: None,
                account_id: None,
                api_key_id: None,
                from_occurred_at: None,
                to_occurred_at: None,
                limit: Some(2),
            })
            .unwrap();
        assert_eq!(limited.returned, 2);

        assert!(matches!(
            storage.list_enterprise_admin_audit_events_internal(&EnterpriseAdminAuditEventQuery {
                operation: Some("scan_public_rights".to_string()),
                outcome: None,
                account_id: None,
                api_key_id: None,
                from_occurred_at: None,
                to_occurred_at: None,
                limit: Some(10),
            }),
            Err(StorageError::BadRequest(message))
                if message == "enterprise admin audit operation is invalid"
        ));

        assert!(matches!(
            storage.list_enterprise_admin_audit_events_internal(&EnterpriseAdminAuditEventQuery {
                operation: None,
                outcome: Some("pending".to_string()),
                account_id: None,
                api_key_id: None,
                from_occurred_at: None,
                to_occurred_at: None,
                limit: Some(10),
            }),
            Err(StorageError::BadRequest(message))
                if message == "enterprise admin audit outcome is invalid"
        ));
    }

    #[test]
    fn registry_accepts_v3_minimal_anchor_without_requiring_v2_payload_size() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("v3-anchor@example.com", "dev-1"))
            .unwrap();
        let reserved = storage
            .reserve_watermark_id(
                &session.access_token,
                &WatermarkIdReserveRequest {
                    request_id: "v3-anchor-request".to_string(),
                    workspace_id: session.workspace.id.clone(),
                    creator_profile_id: session.creator_profile.id.clone(),
                    media_type: "image".to_string(),
                    payload_protocol_version: 3,
                    payload_bytes_length: 33,
                    parent_watermark_uid: None,
                    revision: 1,
                    original_hash: Some("sha256:v3-original".to_string()),
                },
            )
            .unwrap();
        assert_eq!(reserved.payload_protocol_version, 3);
        assert_eq!(reserved.payload_bytes_length, 33);

        let rights = storage
            .public_rights_query(&reserved.watermark_uid)
            .unwrap();
        assert_eq!(rights.registry.anchor_protocol, "v3_minimal_anchor");
        assert_eq!(rights.registry.media_payload_role, "minimal_media_anchor");
        assert_eq!(rights.registry.rights_source, "rights_registry");
    }

    #[test]
    fn rights_manifest_backfill_creates_registry_only_manifest_idempotently() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request(
                "rights-backfill@example.com",
                "dev-1",
            ))
            .unwrap();
        let reserved = storage
            .reserve_watermark_id(
                &session.access_token,
                &sample_watermark_reserve_request(&session, "rights-backfill-request"),
            )
            .unwrap();

        let before = storage
            .public_rights_query(&reserved.watermark_uid)
            .unwrap();
        assert_eq!(before.scan_status, "watermark_only");
        assert!(before.rights_manifest.is_none());
        assert!(before.warnings.contains(&"backfill_pending".to_string()));

        let first = storage
            .backfill_rights_manifests(&RightsManifestBackfillRequest {
                watermark_uids: vec![reserved.watermark_uid.clone()],
                cursor: None,
                limit: None,
            })
            .unwrap();
        assert_eq!(first.processed, 1);
        assert_eq!(first.succeeded, 1);
        assert_eq!(first.results[0].status, "succeeded");
        assert_eq!(first.results[0].manifest_version, Some(1));

        let after = storage
            .public_rights_query(&reserved.watermark_uid)
            .unwrap();
        assert_eq!(after.scan_status, "registry_active");
        assert_eq!(after.training_permission.policy, "not_declared");
        assert!(!after.warnings.contains(&"backfill_pending".to_string()));
        let manifest_id = after
            .rights_manifest
            .as_ref()
            .map(|manifest| manifest.rights_manifest_id.clone())
            .unwrap();

        let second = storage
            .backfill_rights_manifests(&RightsManifestBackfillRequest {
                watermark_uids: vec![reserved.watermark_uid.clone()],
                cursor: None,
                limit: None,
            })
            .unwrap();
        assert_eq!(second.processed, 1);
        assert_eq!(second.succeeded, 1);
        assert_eq!(
            second.results[0].rights_manifest_id.as_deref(),
            Some(manifest_id.as_str())
        );
        assert_eq!(second.results[0].manifest_version, Some(1));
    }

    #[test]
    fn watermark_id_reconcile_registers_offline_id_and_detects_conflict() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let first = storage
            .continue_account(&sample_continue_request("offline-a@example.com", "dev-1"))
            .unwrap();
        let second = storage
            .continue_account(&sample_continue_request("offline-b@example.com", "dev-2"))
            .unwrap();
        let offline_uid = "HS-11111111-22222222-33333333-44444444";

        let reconciled = storage
            .reconcile_watermark_id(
                &first.access_token,
                &sample_watermark_reconcile_request(
                    &first,
                    offline_uid,
                    "sha256:offline-original-a",
                ),
            )
            .unwrap();
        assert_eq!(reconciled.watermark_uid, offline_uid);
        assert_eq!(reconciled.watermark_id_issue_mode, "offline_generated");
        assert_eq!(reconciled.registry_status, "offline_confirmed");

        let conflict = storage
            .reconcile_watermark_id(
                &second.access_token,
                &sample_watermark_reconcile_request(
                    &second,
                    offline_uid,
                    "sha256:offline-original-b",
                ),
            )
            .unwrap();
        assert_eq!(conflict.registry_status, "conflict");
    }

    #[test]
    fn watermark_id_reissue_creates_repair_job_and_replacement_id() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("reissue@example.com", "dev-1"))
            .unwrap();

        let response = storage
            .reissue_watermark_id(
                &session.access_token,
                &WatermarkIdReissueRequest {
                    workspace_id: session.workspace.id.clone(),
                    creator_profile_id: session.creator_profile.id.clone(),
                    previous_watermark_uid: "HS-AAAAAAAA-BBBBBBBB-CCCCCCCC-DDDDDDDD".to_string(),
                    media_type: "image".to_string(),
                    payload_protocol_version: 2,
                    payload_bytes_length: 119,
                    parent_watermark_uid: None,
                    revision: 1,
                    reason: "legacy duplicate repair".to_string(),
                    original_hash: Some("sha256:original-reissue".to_string()),
                },
            )
            .unwrap();
        assert_eq!(
            response.previous_watermark_uid,
            "HS-AAAAAAAA-BBBBBBBB-CCCCCCCC-DDDDDDDD"
        );
        assert_eq!(
            response.replacement.watermark_id_issue_mode,
            "server_reissued"
        );
        assert_eq!(response.replacement.registry_status, "reserved");
        assert_ne!(
            response.replacement.watermark_uid,
            response.previous_watermark_uid
        );
    }

    #[test]
    fn fixture_billing_payment_success_updates_entitlement_and_is_idempotent() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&session, "creator"))
            .unwrap();
        assert_eq!(payment.provider, FIXTURE_PROVIDER);
        assert_eq!(payment.payment_action.action_type, "qr_code");

        let event = sample_fixture_event(&session, &payment, "payment.succeeded", "evt-pay-1");
        let applied = storage.apply_fixture_billing_event(&event).unwrap();
        assert_eq!(applied.duplicate, false);
        assert_eq!(applied.entitlement.plan_code, "creator");
        assert_eq!(applied.entitlement.status, "active");
        assert_eq!(applied.entitlement.features["cloud_sync"], true);
        assert_eq!(applied.entitlement.features["batch_processing"], true);
        assert_eq!(applied.entitlement.features["report_export"], false);

        let duplicated = storage.apply_fixture_billing_event(&event).unwrap();
        assert_eq!(duplicated.duplicate, true);
        assert_eq!(duplicated.entitlement.plan_code, "creator");

        let current = storage.current_entitlement(&session.access_token).unwrap();
        assert_eq!(current.plan_code, "creator");
        assert_eq!(current.status, "active");
        assert_eq!(current.features["cloud_sync"], true);

        let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
        let event_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM subscription_events WHERE provider_event_id = ?1",
                params!["evt-pay-1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1);
    }

    #[test]
    fn fixture_billing_reconcile_recovers_payment_without_webhook() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&session, "creator"))
            .unwrap();

        let before = storage
            .billing_payment_session_status(&session.access_token, &payment.payment_session_id)
            .unwrap();
        assert_eq!(before.status, "created");
        assert_eq!(before.entitlement.plan_code, "free");

        let reconciled = storage
            .reconcile_billing_payment_session(&session.access_token, &payment.payment_session_id)
            .unwrap();
        assert_eq!(reconciled.status, "succeeded");
        assert_eq!(reconciled.entitlement.plan_code, "creator");
        assert_eq!(reconciled.entitlement.status, "active");
        assert_eq!(reconciled.entitlement.features["cloud_sync"], true);

        let after = storage
            .billing_payment_session_status(&session.access_token, &payment.payment_session_id)
            .unwrap();
        assert_eq!(after.status, "succeeded");
        assert_eq!(after.check_attempts, 1);
        assert_eq!(after.entitlement.plan_code, "creator");

        let duplicate = storage
            .reconcile_billing_payment_session(&session.access_token, &payment.payment_session_id)
            .unwrap();
        assert_eq!(duplicate.status, "succeeded");
        assert_eq!(duplicate.entitlement.plan_code, "creator");

        let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
        let event_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM subscription_events WHERE provider_order_id = ?1",
                params![payment.provider_order_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1);
    }

    #[test]
    fn free_report_purchase_grants_single_record_without_upgrading_entitlement() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("report@example.com", "dev-1"))
            .unwrap();

        let purchase = storage
            .create_report_purchase_session(&sample_report_purchase_request(
                &session,
                REPORT_PRODUCT_COPYRIGHT_REPORT_SINGLE,
            ))
            .unwrap();
        assert_eq!(purchase.provider, FIXTURE_PROVIDER);
        assert_eq!(
            purchase.product_code,
            REPORT_PRODUCT_COPYRIGHT_REPORT_SINGLE
        );
        assert_eq!(purchase.price_cents, 1990);
        assert_eq!(purchase.currency, "CNY");

        let before = storage
            .report_purchase_session_status(&session.access_token, &purchase.payment_session_id)
            .unwrap();
        assert_eq!(before.status, "created");
        assert!(before.grant.is_none());

        let reconciled = storage
            .reconcile_report_purchase_session(&session.access_token, &purchase.payment_session_id)
            .unwrap();
        assert_eq!(reconciled.status, "succeeded");
        let grant = reconciled.grant.unwrap();
        assert_eq!(grant.product_code, REPORT_PRODUCT_COPYRIGHT_REPORT_SINGLE);
        assert_eq!(grant.price_cents, 1990);
        assert_eq!(grant.vault_record_id, "vault-report-1");
        assert_eq!(grant.status, "active");

        let entitlement = storage.current_entitlement(&session.access_token).unwrap();
        assert_eq!(entitlement.plan_code, "free");
        assert_eq!(entitlement.features["report_export"], false);

        let after = storage
            .report_purchase_session_status(&session.access_token, &purchase.payment_session_id)
            .unwrap();
        assert_eq!(after.status, "succeeded");
        assert_eq!(after.check_attempts, 1);
        assert!(after.grant.is_some());

        let duplicate = storage
            .reconcile_report_purchase_session(&session.access_token, &purchase.payment_session_id)
            .unwrap();
        assert_eq!(duplicate.status, "succeeded");
        assert!(duplicate.grant.is_some());
    }

    #[test]
    fn report_purchase_supports_evidence_pack_price_and_rejects_unknown_product() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("evidence@example.com", "dev-1"))
            .unwrap();

        let evidence = storage
            .create_report_purchase_session(&sample_report_purchase_request(
                &session,
                REPORT_PRODUCT_RIGHTS_EVIDENCE_PACK_SINGLE,
            ))
            .unwrap();
        assert_eq!(
            evidence.product_code,
            REPORT_PRODUCT_RIGHTS_EVIDENCE_PACK_SINGLE
        );
        assert_eq!(evidence.price_cents, 4990);

        let invalid = storage.create_report_purchase_session(&sample_report_purchase_request(
            &session,
            "unknown_report",
        ));
        assert!(matches!(
            invalid,
            Err(StorageError::BadRequest(message)) if message == "report_product_not_allowed"
        ));
    }

    #[test]
    fn wechat_report_purchase_order_status_grants_then_refund_revokes_without_entitlement_change() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request(
                "wechat-report@example.com",
                "dev-1",
            ))
            .unwrap();
        let request = ReportPurchaseSessionRequest {
            preferred_provider: Some(WECHAT_PAY_PROVIDER.to_string()),
            ..sample_report_purchase_request(&session, REPORT_PRODUCT_RIGHTS_EVIDENCE_PACK_SINGLE)
        };
        let purchase = storage
            .persist_provider_report_purchase_session(
                &request,
                WECHAT_PAY_PROVIDER,
                "wechat_report_order_1",
                BillingPaymentAction {
                    action_type: "qr_code".to_string(),
                    qr_code_url: Some("weixin://wxpay/bizpayurl?pr=report".to_string()),
                    h5_url: None,
                },
                Utc::now() + Duration::minutes(15),
            )
            .unwrap();
        assert_eq!(purchase.provider, WECHAT_PAY_PROVIDER);
        assert_eq!(purchase.price_cents, 4990);

        let status = ReportPurchaseOrderStatus {
            provider: WECHAT_PAY_PROVIDER.to_string(),
            provider_order_id: "wechat_report_order_1".to_string(),
            provider_transaction_id: Some("wx_report_txn_1".to_string()),
            account_id: session.account.id.clone(),
            workspace_id: session.workspace.id.clone(),
            creator_profile_id: session.creator_profile.id.clone(),
            vault_record_id: "vault-report-1".to_string(),
            product_code: REPORT_PRODUCT_RIGHTS_EVIDENCE_PACK_SINGLE.to_string(),
            price_cents: 4990,
            currency: "CNY".to_string(),
            status: BillingOrderStatusKind::Succeeded,
            paid_at: Some(Utc::now()),
            raw_payload_json: r#"{"trade_state":"SUCCESS"}"#.to_string(),
        };
        let reconciled = storage
            .reconcile_report_purchase_order_status(&purchase.payment_session_id, status)
            .unwrap();
        assert_eq!(reconciled.status, "succeeded");
        assert_eq!(
            reconciled.grant.as_ref().map(|grant| grant.status.as_str()),
            Some("active")
        );

        let entitlement = storage.current_entitlement(&session.access_token).unwrap();
        assert_eq!(entitlement.plan_code, "free");
        assert_eq!(entitlement.features["report_export"], false);

        let refund = ReportPurchaseEvent {
            provider: WECHAT_PAY_PROVIDER.to_string(),
            provider_event_id: "wechat_report_refund_1".to_string(),
            provider_order_id: "wechat_report_order_1".to_string(),
            provider_transaction_id: Some("wx_report_txn_1".to_string()),
            account_id: session.account.id.clone(),
            workspace_id: session.workspace.id.clone(),
            creator_profile_id: session.creator_profile.id.clone(),
            vault_record_id: "vault-report-1".to_string(),
            product_code: REPORT_PRODUCT_RIGHTS_EVIDENCE_PACK_SINGLE.to_string(),
            price_cents: 4990,
            currency: "CNY".to_string(),
            event_type: ReportPurchaseEventType::RefundSucceeded,
            occurred_at: Utc::now(),
            raw_payload_json: r#"{"event_type":"REFUND.SUCCESS"}"#.to_string(),
        };
        let revoked = storage.apply_report_purchase_event(refund).unwrap();
        assert_eq!(revoked.status, "revoked");
        assert!(revoked.grant.is_none());

        let after = storage
            .report_purchase_session_status(&session.access_token, &purchase.payment_session_id)
            .unwrap();
        assert_eq!(after.status, "revoked");
        assert!(after.grant.is_none());
    }

    #[test]
    fn fixture_billing_background_reconcile_recovers_due_payment_without_webhook() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&session, "creator"))
            .unwrap();

        let sweep = storage.reconcile_pending_payment_sessions(10).unwrap();
        assert_eq!(sweep.checked, 1);
        assert_eq!(sweep.succeeded, 1);
        assert_eq!(sweep.pending, 0);
        assert_eq!(sweep.skipped_unsupported_provider, 0);

        let after = storage
            .billing_payment_session_status(&session.access_token, &payment.payment_session_id)
            .unwrap();
        assert_eq!(after.status, "succeeded");
        assert_eq!(after.next_check_after, None);
        assert_eq!(after.entitlement.plan_code, "creator");
        assert_eq!(after.entitlement.features["cloud_sync"], true);

        let second_sweep = storage.reconcile_pending_payment_sessions(10).unwrap();
        assert_eq!(second_sweep.checked, 0);
        assert_eq!(second_sweep.succeeded, 0);
    }

    #[test]
    fn background_reconcile_skips_wechat_pay_sessions_until_webhook_or_real_query_exists() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let request = BillingPaymentSessionRequest {
            preferred_provider: Some(WECHAT_PAY_PROVIDER.to_string()),
            ..sample_payment_session_request(&session, "creator")
        };
        let payment = storage
            .persist_provider_billing_payment_session(
                &request,
                WECHAT_PAY_PROVIDER,
                "wechat_order_1",
                BillingPaymentAction {
                    action_type: "qr_code".to_string(),
                    qr_code_url: Some("weixin://wxpay/bizpayurl?pr=test".to_string()),
                    h5_url: None,
                },
                Utc::now() + Duration::minutes(15),
            )
            .unwrap();
        assert_eq!(payment.provider, WECHAT_PAY_PROVIDER);

        let sweep = storage.reconcile_pending_payment_sessions(10).unwrap();
        assert_eq!(sweep.checked, 0);
        assert_eq!(sweep.skipped_unsupported_provider, 1);

        let after = storage
            .billing_payment_session_status(&session.access_token, &payment.payment_session_id)
            .unwrap();
        assert_eq!(after.status, "created");
        assert_eq!(after.check_attempts, 1);
        assert!(after.next_check_after.is_some());
        assert_eq!(after.entitlement.plan_code, "free");

        assert!(matches!(
            storage.reconcile_billing_payment_session(&session.access_token, &payment.payment_session_id),
            Err(StorageError::BadRequest(message)) if message == "billing_reconcile_provider_not_available"
        ));
    }

    #[test]
    fn wechat_order_status_reconcile_uses_standard_billing_event_path() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let request = BillingPaymentSessionRequest {
            preferred_provider: Some(WECHAT_PAY_PROVIDER.to_string()),
            ..sample_payment_session_request(&session, "creator")
        };
        let payment = storage
            .persist_provider_billing_payment_session(
                &request,
                WECHAT_PAY_PROVIDER,
                "wechat_order_success_1",
                BillingPaymentAction {
                    action_type: "qr_code".to_string(),
                    qr_code_url: Some("weixin://wxpay/bizpayurl?pr=success".to_string()),
                    h5_url: None,
                },
                Utc::now() + Duration::minutes(15),
            )
            .unwrap();

        let status = BillingOrderStatus {
            provider: WECHAT_PAY_PROVIDER.to_string(),
            provider_order_id: "wechat_order_success_1".to_string(),
            provider_transaction_id: Some("wx_txn_success_1".to_string()),
            account_id: session.account.id.clone(),
            workspace_id: session.workspace.id.clone(),
            plan_code: "creator".to_string(),
            billing_cycle: "monthly".to_string(),
            amount_cents: 1900,
            currency: "CNY".to_string(),
            status: BillingOrderStatusKind::Succeeded,
            paid_at: Some(Utc::now()),
            raw_payload_json: r#"{"trade_state":"SUCCESS"}"#.to_string(),
        };

        let reconciled = storage
            .reconcile_billing_order_status(&payment.payment_session_id, status)
            .unwrap();
        assert_eq!(reconciled.status, "succeeded");
        assert_eq!(reconciled.entitlement.plan_code, "creator");
        assert_eq!(reconciled.entitlement.features["batch_processing"], true);

        let after = storage
            .billing_payment_session_status(&session.access_token, &payment.payment_session_id)
            .unwrap();
        assert_eq!(after.status, "succeeded");
        assert_eq!(after.next_check_after, None);

        let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
        let event_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM subscription_events WHERE provider = ?1 AND provider_order_id = ?2",
                params![WECHAT_PAY_PROVIDER, "wechat_order_success_1"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(event_count, 1);
    }

    #[test]
    fn fixture_billing_failed_payment_enters_grace_then_refund_downgrades_to_free() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let payment = storage
            .create_billing_payment_session(&sample_payment_session_request(&session, "creator"))
            .unwrap();

        let paid = sample_fixture_event(&session, &payment, "payment.succeeded", "evt-pay-2");
        storage.apply_fixture_billing_event(&paid).unwrap();

        let failed = sample_fixture_event(&session, &payment, "payment.failed", "evt-fail-1");
        let grace = storage.apply_fixture_billing_event(&failed).unwrap();
        assert_eq!(grace.entitlement.plan_code, "creator");
        assert_eq!(grace.entitlement.status, "grace");
        assert_eq!(grace.entitlement.features["cloud_sync"], true);

        let refunded = sample_fixture_event(&session, &payment, "refund.succeeded", "evt-refund-1");
        let expired = storage.apply_fixture_billing_event(&refunded).unwrap();
        assert_eq!(expired.entitlement.plan_code, "free");
        assert_eq!(expired.entitlement.status, "expired");
        assert_eq!(expired.entitlement.features["cloud_sync"], false);
        assert_eq!(expired.entitlement.features["batch_processing"], false);
        assert_eq!(expired.entitlement.features["report_export"], false);
    }

    #[test]
    fn fixture_billing_rejects_enterprise_self_service_and_workspace_mismatch() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();

        let enterprise_self_service_plan = sample_payment_session_request(&session, "enterprise");
        assert!(matches!(
            storage.create_billing_payment_session(&enterprise_self_service_plan),
            Err(StorageError::BadRequest(message)) if message == "plan_code_not_allowed"
        ));

        let mut wrong_workspace = sample_payment_session_request(&session, "creator");
        wrong_workspace.workspace_id = "ws-other".to_string();
        assert!(matches!(
            storage.create_billing_payment_session(&wrong_workspace),
            Err(StorageError::Forbidden)
        ));
    }

    #[test]
    fn l2_video_fingerprint_notary_persists_without_video_minutes_quota() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let request = sample_l2_notary_request(&session);

        let receipt = storage
            .create_video_fingerprint_notary(&session.access_token, &request)
            .unwrap();
        assert_eq!(
            receipt.schema_version,
            "video_fingerprint_notary_receipt_v1"
        );
        assert_eq!(receipt.watermark_uid, "wm_video_l2");
        assert_eq!(receipt.fingerprint_root, "sha256:fingerprint-root");

        let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
        let stored_crop_root: String = conn
            .query_row(
                "SELECT crop_window_fingerprint_root FROM video_fingerprint_notaries WHERE notary_id = ?1",
                params![receipt.notary_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(stored_crop_root, "sha256:crop-window-root");

        let (feature_name, quota_type, quota_units): (String, Option<String>, i64) = conn
            .query_row(
                "SELECT feature_name, quota_type, quota_units FROM cloud_usage_ledger WHERE usage_ledger_id = ?1",
                params![receipt.usage_ledger_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(feature_name, "video_fingerprint_notary");
        assert_eq!(quota_type, None);
        assert_eq!(quota_units, 0);
    }

    #[test]
    fn l2_video_fingerprint_notary_rejects_manifest_media_and_local_paths() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();

        let mut original_video = sample_l2_notary_request(&session);
        original_video.upload_manifest.contains_original_video = true;
        assert!(matches!(
            storage.create_video_fingerprint_notary(&session.access_token, &original_video),
            Err(StorageError::BadRequest(message)) if message == "original_video_forbidden"
        ));

        let mut watermarked_video = sample_l2_notary_request(&session);
        watermarked_video.upload_manifest.contains_watermarked_video = true;
        assert!(matches!(
            storage.create_video_fingerprint_notary(&session.access_token, &watermarked_video),
            Err(StorageError::BadRequest(message)) if message == "watermarked_video_forbidden"
        ));

        let mut local_path = sample_l2_notary_request(&session);
        local_path.upload_manifest.contains_local_paths = true;
        assert!(matches!(
            storage.create_video_fingerprint_notary(&session.access_token, &local_path),
            Err(StorageError::BadRequest(message)) if message == "local_path_forbidden"
        ));
    }

    #[test]
    fn l2_video_fingerprint_notary_requires_crop_windows() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();
        let mut request = sample_l2_notary_request(&session);
        request.crop_window_fingerprint_root.clear();
        request.crop_window_count = 0;

        assert!(matches!(
            storage.create_video_fingerprint_notary(&session.access_token, &request),
            Err(StorageError::BadRequest(message)) if message == "crop_windows_required"
        ));
    }

    #[test]
    fn l2_video_fingerprint_notary_rejects_workspace_and_creator_mismatch() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("alice@example.com", "dev-1"))
            .unwrap();

        let mut wrong_workspace = sample_l2_notary_request(&session);
        wrong_workspace.workspace_id = "ws_other".to_string();
        assert!(matches!(
            storage.create_video_fingerprint_notary(&session.access_token, &wrong_workspace),
            Err(StorageError::Forbidden)
        ));

        let mut wrong_creator = sample_l2_notary_request(&session);
        wrong_creator.creator_profile_id = "creator_other".to_string();
        assert!(matches!(
            storage.create_video_fingerprint_notary(&session.access_token, &wrong_creator),
            Err(StorageError::BadRequest(message)) if message == "creator_profile_required"
        ));
    }

    #[test]
    fn cloud_video_task_requires_cloud_video_processing_entitlement_and_routes_status_flow() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("l3@example.com", "dev-1"))
            .unwrap();
        let request = sample_cloud_video_task_request(&session);

        assert!(matches!(
            storage.create_cloud_video_task(&session.access_token, &request),
            Err(StorageError::Forbidden)
        ));

        storage
            .set_entitlement_feature_for_tests(&session.account.id, "cloud_video_processing", true)
            .unwrap();

        let created = storage
            .create_cloud_video_task(&session.access_token, &request)
            .unwrap();
        assert_eq!(created.schema_version, "cloud_video_task_v1");
        assert_eq!(created.capability_level, "hybrid_visual_watermark");
        assert_eq!(created.status, "draft");
        assert_eq!(created.quota_units, 3);
        assert!(created.usage_ledger_id.is_none());

        let listed = storage
            .list_cloud_video_tasks(
                &session.access_token,
                &crate::schema::CloudVideoTaskListQuery {
                    workspace_id: Some(session.workspace.id.clone()),
                    status: Some("draft".to_string()),
                    limit: Some(20),
                },
            )
            .unwrap();
        assert_eq!(listed.returned, 1);
        assert_eq!(listed.tasks[0].task_id, created.task_id);

        let fetched = storage
            .get_cloud_video_task(&session.access_token, &created.task_id)
            .unwrap();
        assert_eq!(fetched.task_id, created.task_id);

        assert!(matches!(
            storage.update_cloud_video_task_status(
                &session.access_token,
                &created.task_id,
                &crate::schema::CloudVideoTaskStatusUpdateRequest {
                    status: "succeeded".to_string(),
                    failure_code: None,
                    strategy_digest: Some("sha256:strategy".to_string()),
                    self_check_threshold: Some(0.9),
                    self_check_confidence: Some(0.95),
                    checked_frames: Some(8),
                    watermarked_media_hash: Some("sha256:watermarked-video".to_string()),
                    server_receipt_signature: Some("sig:server-receipt".to_string()),
                },
            ),
            Err(StorageError::BadRequest(message))
                if message == "cloud_video_task_completion_requires_trusted_worker"
        ));

        let claim = storage
            .claim_cloud_video_task_for_worker(&crate::schema::CloudVideoTaskClaimRequest {
                worker_id: "worker-l3-release".to_string(),
                capability_level: Some("hybrid_visual_watermark".to_string()),
                lease_seconds: Some(900),
            })
            .unwrap();
        assert_eq!(claim.task.task_id, created.task_id);
        assert_eq!(claim.task.status, "running");
        assert_eq!(claim.task.worker_id.as_deref(), Some("worker-l3-release"));
        assert_eq!(claim.task.attempt_count, 1);
        assert!(!claim.lease_token.is_empty());

        let (worker_receipt, worker_receipt_hash, output_ref, output_bytes, output_content_type) =
            sample_l3_worker_receipt_fields(
                &created.task_id,
                &claim.worker_id,
                "sha256:watermarked-video",
            );
        let succeeded = storage
            .complete_cloud_video_task_from_trusted_worker(
                &created.task_id,
                &crate::schema::CloudVideoTaskCompletionRequest {
                    strategy_digest: "sha256:strategy".to_string(),
                    self_check_threshold: 0.9,
                    self_check_confidence: 0.95,
                    checked_frames: 8,
                    watermarked_media_hash: "sha256:watermarked-video".to_string(),
                    output_media_storage_ref: output_ref.clone(),
                    output_media_bytes: output_bytes,
                    output_media_content_type: output_content_type,
                    worker_receipt_hash: worker_receipt_hash.clone(),
                    worker_receipt,
                    server_receipt_signature: "sig:server-receipt".to_string(),
                    worker_id: claim.worker_id.clone(),
                    attempt_id: claim.attempt_id.clone(),
                    lease_token: claim.lease_token.clone(),
                },
            )
            .unwrap();
        assert_eq!(succeeded.status, "succeeded");
        assert_eq!(succeeded.self_check_threshold, Some(0.9));
        assert_eq!(succeeded.self_check_confidence, Some(0.95));
        assert_eq!(succeeded.checked_frames, Some(8));
        assert_eq!(
            succeeded.watermarked_media_hash.as_deref(),
            Some("sha256:watermarked-video")
        );
        assert_eq!(
            succeeded.server_receipt_signature.as_deref(),
            Some("sig:server-receipt")
        );
        assert_eq!(
            succeeded.output_media_storage_ref.as_deref(),
            Some(output_ref.as_str())
        );
        assert_eq!(succeeded.output_media_bytes, Some(output_bytes));
        assert_eq!(
            succeeded.worker_receipt_hash.as_deref(),
            Some(worker_receipt_hash.as_str())
        );
        assert_eq!(
            succeeded
                .worker_receipt
                .as_ref()
                .and_then(|value| value.get("schemaVersion"))
                .and_then(serde_json::Value::as_str),
            Some("l3_worker_receipt_v1")
        );
        assert!(succeeded.completed_at.is_some());
        assert!(succeeded.usage_ledger_id.is_some());
        assert_eq!(succeeded.worker_id.as_deref(), Some("worker-l3-release"));
        assert_eq!(
            succeeded.attempt_id.as_deref(),
            Some(claim.attempt_id.as_str())
        );

        let (feature_name, quota_type, quota_units, reference_id): (String, String, i64, String) = {
            let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
            conn.query_row(
                "SELECT feature_name, quota_type, quota_units, reference_id FROM cloud_usage_ledger WHERE usage_ledger_id = ?1",
                params![succeeded.usage_ledger_id.clone().unwrap()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .unwrap()
        };
        assert_eq!(feature_name, "cloud_video_processing");
        assert_eq!(quota_type, "video_minutes");
        assert_eq!(quota_units, 3);
        assert_eq!(reference_id, created.task_id);

        let failed = storage
            .create_cloud_video_task(&session.access_token, &request)
            .unwrap();
        let canceled = storage
            .update_cloud_video_task_status(
                &session.access_token,
                &failed.task_id,
                &crate::schema::CloudVideoTaskStatusUpdateRequest {
                    status: "canceled".to_string(),
                    failure_code: Some("user_canceled".to_string()),
                    strategy_digest: None,
                    self_check_threshold: None,
                    self_check_confidence: None,
                    checked_frames: None,
                    watermarked_media_hash: None,
                    server_receipt_signature: None,
                },
            )
            .unwrap();
        assert_eq!(canceled.status, "canceled");
        let ledger_count: i64 = {
            let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
            conn.query_row(
                "SELECT COUNT(*) FROM cloud_usage_ledger WHERE reference_id = ?1",
                params![failed.task_id],
                |row| row.get(0),
            )
            .unwrap()
        };
        assert_eq!(ledger_count, 0);
    }

    #[test]
    fn cloud_video_task_rejects_privacy_fields_and_invalid_completion_updates() {
        let file = NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let session = storage
            .continue_account(&sample_continue_request("l3-privacy@example.com", "dev-1"))
            .unwrap();
        storage
            .set_entitlement_feature_for_tests(&session.account.id, "cloud_video_processing", true)
            .unwrap();

        let mut request = sample_cloud_video_task_request(&session);
        request.upload_manifest.contains_original_video = true;
        assert!(matches!(
            storage.create_cloud_video_task(&session.access_token, &request),
            Err(StorageError::BadRequest(message)) if message == "original_video_forbidden"
        ));

        let mut request = sample_cloud_video_task_request(&session);
        request.upload_manifest.contains_watermarked_video = true;
        assert!(matches!(
            storage.create_cloud_video_task(&session.access_token, &request),
            Err(StorageError::BadRequest(message)) if message == "watermarked_video_forbidden"
        ));

        let mut request = sample_cloud_video_task_request(&session);
        request.upload_manifest.contains_local_paths = true;
        assert!(matches!(
            storage.create_cloud_video_task(&session.access_token, &request),
            Err(StorageError::BadRequest(message)) if message == "local_path_forbidden"
        ));

        let created = storage
            .create_cloud_video_task(
                &session.access_token,
                &sample_cloud_video_task_request(&session),
            )
            .unwrap();
        assert!(matches!(
            storage.update_cloud_video_task_status(
                &session.access_token,
                &created.task_id,
                &crate::schema::CloudVideoTaskStatusUpdateRequest {
                    status: "failed".to_string(),
                    failure_code: None,
                    strategy_digest: None,
                    self_check_threshold: None,
                    self_check_confidence: None,
                    checked_frames: None,
                    watermarked_media_hash: None,
                    server_receipt_signature: None,
                },
            ),
            Err(StorageError::BadRequest(message)) if message == "cloud_video_task_failure_code_required"
        ));
        assert!(matches!(
            storage.update_cloud_video_task_status(
                &session.access_token,
                &created.task_id,
                &crate::schema::CloudVideoTaskStatusUpdateRequest {
                    status: "succeeded".to_string(),
                    failure_code: None,
                    strategy_digest: None,
                    self_check_threshold: None,
                    self_check_confidence: None,
                    checked_frames: None,
                    watermarked_media_hash: None,
                    server_receipt_signature: None,
                },
            ),
            Err(StorageError::BadRequest(message))
                if message == "cloud_video_task_completion_requires_trusted_worker"
        ));
        let (
            invalid_worker_receipt,
            invalid_worker_receipt_hash,
            invalid_output_ref,
            invalid_output_bytes,
            invalid_output_content_type,
        ) = sample_l3_worker_receipt_fields(
            &created.task_id,
            "worker-invalid",
            "sha256:watermarked-video",
        );
        assert!(matches!(
            storage.complete_cloud_video_task_from_trusted_worker(
                &created.task_id,
                &crate::schema::CloudVideoTaskCompletionRequest {
                    strategy_digest: "".to_string(),
                    self_check_threshold: 0.95,
                    self_check_confidence: 0.95,
                    checked_frames: 8,
                    watermarked_media_hash: "sha256:watermarked-video".to_string(),
                    output_media_storage_ref: invalid_output_ref.clone(),
                    output_media_bytes: invalid_output_bytes,
                    output_media_content_type: invalid_output_content_type.clone(),
                    worker_receipt_hash: invalid_worker_receipt_hash.clone(),
                    worker_receipt: invalid_worker_receipt.clone(),
                    server_receipt_signature: "sig:server-receipt".to_string(),
                    worker_id: "worker-invalid".to_string(),
                    attempt_id: "attempt-invalid".to_string(),
                    lease_token: "lease-invalid".to_string(),
                },
            ),
            Err(StorageError::BadRequest(message)) if message == "strategy_digest_required"
        ));
        assert!(matches!(
            storage.complete_cloud_video_task_from_trusted_worker(
                &created.task_id,
                &crate::schema::CloudVideoTaskCompletionRequest {
                    strategy_digest: "sha256:strategy".to_string(),
                    self_check_threshold: 0.95,
                    self_check_confidence: 0.90,
                    checked_frames: 8,
                    watermarked_media_hash: "sha256:watermarked-video".to_string(),
                    output_media_storage_ref: invalid_output_ref,
                    output_media_bytes: invalid_output_bytes,
                    output_media_content_type: invalid_output_content_type,
                    worker_receipt_hash: invalid_worker_receipt_hash,
                    worker_receipt: invalid_worker_receipt,
                    server_receipt_signature: "sig:server-receipt".to_string(),
                    worker_id: "worker-invalid".to_string(),
                    attempt_id: "attempt-invalid".to_string(),
                    lease_token: "lease-invalid".to_string(),
                },
            ),
            Err(StorageError::BadRequest(message)) if message == "self_check_confidence_below_threshold"
        ));
        let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
        let ledger_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM cloud_usage_ledger WHERE reference_id = ?1",
                params![created.task_id],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ledger_count, 0);
    }

    #[test]
    fn ai_transparency_internal_license_query_and_profile_check_are_read_only_and_audited() {
        let file = tempfile::NamedTempFile::new().unwrap();
        let storage = Storage::open(file.path(), 30).unwrap();
        let now = Utc::now();
        let effective_at = (now - Duration::days(1)).to_rfc3339();
        let expires_at = (now + Duration::days(1)).to_rfc3339();
        {
            let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
            conn.execute(
                "INSERT INTO ai_transparency_licenses (
                    license_id, tenant_id, workspace_id, environment, status, issuer_mode,
                    deployment_mode, public_verification_required, metering_plan_id,
                    effective_at, expires_at, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?10, ?10)",
                params![
                    "atl-test",
                    "tenant-test",
                    "workspace-test",
                    "production",
                    "active",
                    "hiddenshield_managed",
                    "hosted",
                    1,
                    "metering-test",
                    effective_at,
                    expires_at
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO ai_profile_entitlements (
                    license_id, profile_id, profile_kind, status, effective_at, expires_at,
                    terms_version, approved_by, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?5, ?5)",
                params![
                    "atl-test",
                    "cn-image-v1",
                    "regulatory",
                    "active",
                    effective_at,
                    expires_at,
                    "v1",
                    "test"
                ],
            )
            .unwrap();
        }
        let detail = storage
            .get_ai_transparency_license_internal("atl-test")
            .unwrap()
            .unwrap();
        assert_eq!(detail.profile_entitlements.len(), 1);
        let authorized = storage
            .check_ai_transparency_profile_entitlements_internal(
                &AiTransparencyProfileEntitlementCheckRequest {
                    license_id: "atl-test".to_string(),
                    environment: "production".to_string(),
                    requested_profile_ids: vec!["cn-image-v1".to_string()],
                },
            )
            .unwrap();
        assert!(authorized.authorized);
        let denied = storage
            .check_ai_transparency_profile_entitlements_internal(
                &AiTransparencyProfileEntitlementCheckRequest {
                    license_id: "atl-test".to_string(),
                    environment: "sandbox".to_string(),
                    requested_profile_ids: vec!["cn-image-v1".to_string()],
                },
            )
            .unwrap();
        assert_eq!(
            denied.license_decision.reason_code,
            "ai_license_environment_mismatch"
        );
        storage
            .record_ai_transparency_admin_audit_event_internal(
                "check_profile_entitlements",
                "succeeded",
                "/internal/ai-transparency/profile-entitlements/check",
                Some(&detail.license),
                Some("atl-test"),
                &["cn-image-v1".to_string()],
                "authorized",
                serde_json::json!({ "requestedProfileCount": 1 }),
            )
            .unwrap();
        let conn = storage.conn.lock().unwrap_or_else(|e| e.into_inner());
        let ledger_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ai_marking_ledger WHERE license_id = ?1",
                params!["atl-test"],
                |row| row.get(0),
            )
            .unwrap();
        let audit_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM ai_transparency_admin_audit_events WHERE license_id = ?1",
                params!["atl-test"],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(ledger_count, 0);
        assert_eq!(audit_count, 1);
    }
}

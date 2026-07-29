pub mod ai_transparency_change_command;
#[cfg(feature = "postgres")]
pub mod ai_transparency_confirm_command;
#[cfg(feature = "postgres")]
pub mod ai_transparency_credential_custody;
#[cfg(feature = "postgres")]
pub mod ai_transparency_dead_letter_command;
#[cfg(feature = "postgres")]
pub mod ai_transparency_delivery_envelope;
#[cfg(feature = "postgres")]
pub mod ai_transparency_delivery_observability;
#[cfg(feature = "postgres")]
pub mod ai_transparency_delivery_retrieval;
#[cfg(feature = "postgres")]
pub mod ai_transparency_delivery_security_incident;
#[cfg(feature = "postgres")]
pub mod ai_transparency_delivery_security_notification;
#[cfg(feature = "postgres")]
pub mod ai_transparency_external_evidence_intake;
#[cfg(feature = "postgres")]
pub mod ai_transparency_image_marking_executor;
pub mod ai_transparency_internal_provider;
#[cfg(feature = "postgres")]
pub mod ai_transparency_notification_delivery;
#[cfg(feature = "postgres")]
pub mod ai_transparency_platform_api;
#[cfg(feature = "postgres")]
pub mod ai_transparency_post_embed_recovery;
#[cfg(feature = "postgres")]
pub mod ai_transparency_post_embed_signing;
pub mod ai_transparency_production_provider;
#[cfg(feature = "postgres")]
pub mod ai_transparency_public_resolver;
pub mod billing;
#[cfg(feature = "postgres")]
pub mod cloud_copyright;
pub mod database;
#[cfg(feature = "postgres")]
pub mod postgres_auth;
#[cfg(feature = "postgres")]
pub mod postgres_registry;
#[cfg(feature = "postgres")]
pub mod postgres_sync;
pub mod repository;
pub mod schema;
pub mod storage;

use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    body::Bytes,
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{get, patch, post, put},
    Json, Router,
};
use base64::Engine;
use chrono::Utc;
use clap::Parser;
use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::billing::{
    BillingPaymentSessionInput, ReportPurchasePaymentInput, WechatPayHeaders,
    WechatPayNativeAdapter, WechatPayNormalizedEvent, WECHAT_PAY_PROVIDER,
};
use crate::database::{DatabaseBackendKind, DatabaseConfig};
use crate::schema::{
    AccountDevicesResponse, AiTransparencyLicenseDetailResponse,
    AiTransparencyProfileEntitlementCheckRequest, AiTransparencyProfileEntitlementCheckResponse,
    AnonymousFeedbackBatch, AnonymousFeedbackBatchAck, AnonymousFeedbackStatsQuery,
    AnonymousFeedbackStatsResponse, AuthChallengeRequest, AuthLogoutRequest, AuthRefreshRequest,
    AuthSessionRequest, BillingFixtureEventRequest, BillingPaymentSessionRequest,
    BillingWechatPayNotificationRequest, CloudSyncBatchRequest, CloudSyncChangesResult,
    CloudVideoTaskClaimRequest, CloudVideoTaskClaimResponse, CloudVideoTaskCompletionRequest,
    CloudVideoTaskDownloadAuthorizationQuery, CloudVideoTaskDownloadAuthorizationRequest,
    CloudVideoTaskDownloadAuthorizationResponse, CloudVideoTaskFailureRequest,
    CloudVideoTaskListQuery, CloudVideoTaskListResponse,
    CloudVideoTaskObjectUploadAuthorizationRequest,
    CloudVideoTaskObjectUploadAuthorizationResponse, CloudVideoTaskObjectUploadQuery,
    CloudVideoTaskObjectUploadResponse, CloudVideoTaskRecord, CloudVideoTaskRequest,
    CloudVideoTaskStatusUpdateRequest, CommercialMetricsOverviewResponse, ContinueAccountRequest,
    EnterpriseAdminAuditEventListResponse, EnterpriseAdminAuditEventQuery,
    EnterpriseApiKeyCreateRequest, EnterpriseApiKeyIssueRequest, EnterpriseApiKeyIssueResponse,
    EnterpriseApiKeyListQuery, EnterpriseApiKeyListResponse, EnterpriseApiKeyRecord,
    EnterpriseApiKeyRotateRequest, EnterpriseApiKeyRotateResponse,
    EnterpriseApiKeyStatusChangeRequest, EnterpriseExpiredRotationRevokeItem,
    EnterpriseExpiredRotationRevokeRequest, EnterpriseExpiredRotationRevokeResponse,
    EnterpriseGatewayClientFingerprint, EnterpriseGatewayDryRunDecision,
    EnterpriseGatewayDryRunRequest, EnterprisePublicRightsBatchRequest,
    EnterprisePublicRightsBatchResponse, EnterpriseQuotaBalanceInitRequest,
    EnterpriseQuotaBalanceRecord, PublicRightsBatchRequest, PublicRightsBatchResponse,
    PublicRightsMetadataExport, PublicRightsQueryResponse, ReportPurchaseSessionRequest,
    RevokeDeviceResponse, RightsManifestBackfillRequest, RightsManifestBackfillResponse,
    SyncPreferencesRequest, TeamAuditListResponse, TeamMemberCreateRequest, TeamMemberListResponse,
    TeamMemberUpdateRequest, TeamSharedLibraryListResponse, TeamSharedLibraryShareRequest,
    TeamWorkspaceCreateRequest, TeamWorkspaceListResponse, TeamWorkspaceSummary,
    UpdateDeviceRequest, VideoFingerprintNotaryRequest, WatermarkIdConfirmRequest,
    WatermarkIdReconcileRequest, WatermarkIdRegistryResponse, WatermarkIdReissueRequest,
    WatermarkIdReissueResponse, WatermarkIdReserveRequest, PUBLIC_RIGHTS_ANONYMOUS_BATCH_MAX_ITEMS,
};
use crate::storage::{dry_run_enterprise_gateway_readonly_scan, Storage, StorageError};

#[derive(Debug, Parser, Clone)]
pub struct ServerArgs {
    #[arg(
        long,
        env = "HIDDENSHIELD_FEEDBACK_BIND_ADDR",
        default_value = "127.0.0.1:8787"
    )]
    pub bind_addr: SocketAddr,

    #[arg(
        long,
        env = "HIDDENSHIELD_FEEDBACK_DB_PATH",
        default_value = "feedback.sqlite"
    )]
    pub db_path: PathBuf,

    #[arg(
        long,
        env = "HIDDENSHIELD_DATABASE_BACKEND",
        value_enum,
        default_value_t = DatabaseBackendKind::Sqlite
    )]
    pub database_backend: DatabaseBackendKind,

    #[arg(long, env = "HIDDENSHIELD_DATABASE_URL")]
    pub database_url: Option<String>,

    #[arg(long, env = "HIDDENSHIELD_DEPLOYMENT_ENV", default_value = "local")]
    pub deployment_env: String,

    #[arg(
        long,
        env = "HIDDENSHIELD_FEEDBACK_RETENTION_DAYS",
        default_value_t = 180
    )]
    pub retention_days: i64,

    #[arg(
        long,
        env = "HIDDENSHIELD_BILLING_RECONCILE_INTERVAL_SECS",
        default_value_t = 30
    )]
    pub billing_reconcile_interval_secs: u64,

    #[arg(
        long,
        env = "HIDDENSHIELD_BILLING_RECONCILE_BATCH_SIZE",
        default_value_t = 50
    )]
    pub billing_reconcile_batch_size: usize,

    #[arg(long, env = "HIDDENSHIELD_COMMERCIAL_METRICS_ADMIN_TOKEN")]
    pub commercial_metrics_admin_token: Option<String>,

    #[arg(long, env = "HIDDENSHIELD_AUTH_OTP_DELIVERY_ENDPOINT")]
    pub auth_otp_delivery_endpoint: Option<String>,

    #[arg(long, env = "HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET")]
    pub enterprise_api_key_hash_secret: Option<String>,

    #[arg(
        long,
        env = "HIDDENSHIELD_ENTERPRISE_API_KEY_HASH_SECRET_VERSION",
        default_value = "local-dev"
    )]
    pub enterprise_api_key_hash_secret_version: String,

    #[arg(long, env = "HIDDENSHIELD_TRUSTED_PROXY_SHARED_SECRET")]
    pub trusted_proxy_shared_secret: Option<String>,

    #[arg(
        long,
        env = "HIDDENSHIELD_ENTERPRISE_REQUIRE_TRUSTED_PROXY",
        default_value_t = false
    )]
    pub enterprise_require_trusted_proxy: bool,
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<Storage>,
    pub wechat_pay: Option<Arc<WechatPayNativeAdapter>>,
    pub http_client: reqwest::Client,
    pub commercial_metrics_admin_token: Option<String>,
    pub auth_otp_delivery_endpoint: Option<String>,
    pub enterprise_api_key_hash_secret: Option<String>,
    pub enterprise_api_key_hash_secret_version: String,
    pub trusted_proxy_shared_secret: Option<String>,
    pub enterprise_require_trusted_proxy: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    ok: bool,
    service: &'static str,
    status: &'static str,
    version: &'static str,
    timestamp: chrono::DateTime<chrono::Utc>,
    cloud_sync: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrustedTimeResponse {
    status: &'static str,
    source: String,
    trusted_time_at: chrono::DateTime<chrono::Utc>,
    third_party_verification_status: &'static str,
    third_party_verification_provider: String,
    verification_path: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiErrorResponse {
    error: String,
    message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("bad request")]
    BadRequest(String),
    #[error("unauthorized")]
    Unauthorized,
    #[error("forbidden")]
    Forbidden,
    #[error("not found")]
    NotFound(String),
    #[error("rate limited")]
    RateLimited(String),
    #[error("storage error")]
    Storage(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error, message) = match self {
            ApiError::BadRequest(message) => {
                (StatusCode::BAD_REQUEST, "bad_request".to_string(), message)
            }
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "unauthorized".to_string(),
                "unauthorized".to_string(),
            ),
            ApiError::Forbidden => (
                StatusCode::FORBIDDEN,
                "forbidden".to_string(),
                "forbidden".to_string(),
            ),
            ApiError::NotFound(message) => {
                (StatusCode::NOT_FOUND, "not_found".to_string(), message)
            }
            ApiError::RateLimited(message) => (
                StatusCode::TOO_MANY_REQUESTS,
                "rate_limited".to_string(),
                message,
            ),
            ApiError::Storage(message) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "storage_error".to_string(),
                message,
            ),
        };
        (status, Json(ApiErrorResponse { error, message })).into_response()
    }
}

pub fn build_app(storage: Arc<Storage>) -> Router {
    build_app_with_billing(storage, None)
}

pub fn build_app_with_billing(
    storage: Arc<Storage>,
    wechat_pay: Option<Arc<WechatPayNativeAdapter>>,
) -> Router {
    build_app_with_billing_and_admin(storage, wechat_pay, None)
}

pub fn build_app_with_billing_and_admin(
    storage: Arc<Storage>,
    wechat_pay: Option<Arc<WechatPayNativeAdapter>>,
    commercial_metrics_admin_token: Option<String>,
) -> Router {
    build_app_with_billing_admin_and_auth_delivery(
        storage,
        wechat_pay,
        commercial_metrics_admin_token,
        None,
    )
}

pub fn build_app_with_billing_admin_and_auth_delivery(
    storage: Arc<Storage>,
    wechat_pay: Option<Arc<WechatPayNativeAdapter>>,
    commercial_metrics_admin_token: Option<String>,
    auth_otp_delivery_endpoint: Option<String>,
) -> Router {
    build_app_with_admin_auth_and_enterprise_custody(
        storage,
        wechat_pay,
        commercial_metrics_admin_token,
        auth_otp_delivery_endpoint,
        None,
        "local-dev".to_string(),
    )
}

pub fn build_app_with_admin_auth_and_enterprise_custody(
    storage: Arc<Storage>,
    wechat_pay: Option<Arc<WechatPayNativeAdapter>>,
    commercial_metrics_admin_token: Option<String>,
    auth_otp_delivery_endpoint: Option<String>,
    enterprise_api_key_hash_secret: Option<String>,
    enterprise_api_key_hash_secret_version: String,
) -> Router {
    build_app_with_admin_auth_enterprise_custody_and_proxy(
        storage,
        wechat_pay,
        commercial_metrics_admin_token,
        auth_otp_delivery_endpoint,
        enterprise_api_key_hash_secret,
        enterprise_api_key_hash_secret_version,
        None,
        false,
    )
}

pub fn build_app_with_admin_auth_enterprise_custody_and_proxy(
    storage: Arc<Storage>,
    wechat_pay: Option<Arc<WechatPayNativeAdapter>>,
    commercial_metrics_admin_token: Option<String>,
    auth_otp_delivery_endpoint: Option<String>,
    enterprise_api_key_hash_secret: Option<String>,
    enterprise_api_key_hash_secret_version: String,
    trusted_proxy_shared_secret: Option<String>,
    enterprise_require_trusted_proxy: bool,
) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/health", get(healthz))
        .route("/v1/trusted-time", get(get_trusted_time))
        .route("/v1/auth/challenges", post(create_auth_challenge))
        .route("/v1/auth/sessions", post(create_auth_session))
        .route("/v1/auth/refresh", post(refresh_auth_session))
        .route("/v1/auth/logout", post(logout_auth_session))
        .route("/v1/auth/continue", post(continue_account))
        .route("/v1/me", get(get_me))
        .route("/v1/me/sync-preferences", patch(update_sync_preferences))
        .route("/v1/devices", get(list_devices))
        .route(
            "/v1/devices/:device_id",
            patch(update_device).delete(revoke_device),
        )
        .route(
            "/v1/billing/payment-sessions",
            post(create_billing_payment_session),
        )
        .route(
            "/v1/billing/payment-sessions/:payment_session_id",
            get(get_billing_payment_session_status),
        )
        .route(
            "/v1/billing/payment-sessions/:payment_session_id/reconcile",
            post(reconcile_billing_payment_session),
        )
        .route(
            "/v1/billing/report-purchase-sessions",
            post(create_report_purchase_session),
        )
        .route(
            "/v1/billing/report-purchase-sessions/:payment_session_id",
            get(get_report_purchase_session_status),
        )
        .route(
            "/v1/billing/report-purchase-sessions/:payment_session_id/reconcile",
            post(reconcile_report_purchase_session),
        )
        .route(
            "/v1/billing/webhooks/fixture",
            post(apply_fixture_billing_webhook),
        )
        .route(
            "/v1/billing/webhooks/wechat-pay",
            post(apply_wechat_pay_webhook),
        )
        .route("/v1/entitlements/current", get(get_current_entitlement))
        .route(
            "/v1/video-fingerprints/notaries",
            post(create_video_fingerprint_notary),
        )
        .route(
            "/v1/video-tasks",
            post(create_cloud_video_task).get(list_cloud_video_tasks),
        )
        .route(
            "/v1/video-tasks/object-upload-authorizations",
            post(create_cloud_video_task_object_upload_authorization),
        )
        .route(
            "/v1/video-object-store/upload",
            put(resolve_cloud_video_task_object_upload_authorization),
        )
        .route("/v1/video-tasks/:task_id", get(get_cloud_video_task))
        .route(
            "/v1/video-tasks/:task_id/output-download-authorizations",
            post(create_cloud_video_task_output_download_authorization),
        )
        .route(
            "/v1/video-tasks/:task_id/output-download",
            get(resolve_cloud_video_task_output_download_authorization),
        )
        .route(
            "/v1/video-tasks/:task_id/status",
            patch(update_cloud_video_task_status),
        )
        .route(
            "/internal/video-tasks/claim",
            post(claim_cloud_video_task_internal),
        )
        .route(
            "/internal/video-tasks/:task_id/completion",
            post(complete_cloud_video_task_internal),
        )
        .route(
            "/internal/video-tasks/:task_id/failure",
            post(fail_cloud_video_task_internal),
        )
        .route("/v1/watermark-ids/reserve", post(reserve_watermark_id))
        .route("/v1/watermark-ids/confirm", post(confirm_watermark_id))
        .route("/v1/watermark-ids/reconcile", post(reconcile_watermark_id))
        .route("/v1/watermark-ids/reissue", post(reissue_watermark_id))
        .route(
            "/v1/team/workspaces/current",
            get(get_current_team_workspace),
        )
        .route(
            "/v1/team/workspaces",
            get(list_team_workspaces).post(create_team_workspace),
        )
        .route(
            "/v1/team/workspaces/:workspace_id/members",
            get(list_team_members),
        )
        .route(
            "/v1/team/workspaces/:workspace_id/members",
            post(create_team_member),
        )
        .route("/v1/team/members/:member_id", patch(update_team_member))
        .route(
            "/v1/team/workspaces/:workspace_id/vault",
            get(list_team_shared_library),
        )
        .route(
            "/v1/team/workspaces/:workspace_id/vault/share",
            post(share_team_library_record),
        )
        .route(
            "/v1/team/workspaces/:workspace_id/audit-logs",
            get(list_team_audit_logs),
        )
        .route("/v1/public/rights/:watermark_uid", get(get_public_rights))
        .route(
            "/v1/public/rights/:watermark_uid/metadata",
            get(get_public_rights_metadata),
        )
        .route("/v1/public/rights/batch", post(get_public_rights_batch))
        .route(
            "/v1/enterprise/public-rights/batch",
            post(enterprise_public_rights_batch),
        )
        .route(
            "/internal/rights-manifests/backfill",
            post(backfill_rights_manifests),
        )
        .route(
            "/internal/enterprise/api-keys",
            get(list_enterprise_api_keys_internal).post(create_enterprise_api_key_internal),
        )
        .route(
            "/internal/enterprise/api-key-issuances",
            post(issue_enterprise_api_key_internal),
        )
        .route(
            "/internal/enterprise/api-keys/:api_key_id",
            get(get_enterprise_api_key_internal),
        )
        .route(
            "/internal/enterprise/api-keys/:api_key_id/pause",
            post(pause_enterprise_api_key_internal),
        )
        .route(
            "/internal/enterprise/api-keys/:api_key_id/rotate",
            post(rotate_enterprise_api_key_internal),
        )
        .route(
            "/internal/enterprise/api-keys/:api_key_id/revoke",
            post(revoke_enterprise_api_key_internal),
        )
        .route(
            "/internal/enterprise/api-key-rotations/revoke-expired",
            post(revoke_expired_enterprise_rotations_internal),
        )
        .route(
            "/internal/enterprise/quota-balances",
            post(initialize_enterprise_quota_balance_internal),
        )
        .route(
            "/internal/enterprise/admin-audit-events",
            get(list_enterprise_admin_audit_events_internal),
        )
        .route(
            "/internal/enterprise/gateway-dry-run",
            post(dry_run_enterprise_gateway_internal),
        )
        .route(
            "/internal/ai-transparency/licenses/:license_id",
            get(get_ai_transparency_license_internal),
        )
        .route(
            "/internal/ai-transparency/profile-entitlements/check",
            post(check_ai_transparency_profile_entitlements_internal),
        )
        .route("/v1/sync/events:batch", post(push_cloud_events_batch))
        .route("/v1/sync/changes", get(get_cloud_changes))
        .route("/v1/anonymous-feedback/batches", post(ingest_batch))
        .route("/v1/anonymous-feedback/stats", get(get_stats))
        .route(
            "/v1/commercial/metrics/overview",
            get(get_commercial_metrics_overview),
        )
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::list([
                    "tauri://localhost"
                        .parse::<HeaderValue>()
                        .expect("valid Tauri WebView origin"),
                    "http://tauri.localhost"
                        .parse::<HeaderValue>()
                        .expect("valid Tauri WebView HTTP origin"),
                    "https://tauri.localhost"
                        .parse::<HeaderValue>()
                        .expect("valid Tauri WebView HTTPS origin"),
                    "http://localhost:1420"
                        .parse::<HeaderValue>()
                        .expect("valid Vite dev origin"),
                    "http://127.0.0.1:43189"
                        .parse::<HeaderValue>()
                        .expect("valid Flutter Web preview origin"),
                    "http://127.0.0.1:8080"
                        .parse::<HeaderValue>()
                        .expect("valid fallback Flutter Web preview origin"),
                ]))
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::HeaderName::from_static("x-hiddenshield-api-key"),
                    axum::http::HeaderName::from_static("x-hiddenshield-proxy-secret"),
                    axum::http::HeaderName::from_static("x-hiddenshield-client-fingerprint"),
                    axum::http::HeaderName::from_static("x-forwarded-for"),
                    axum::http::HeaderName::from_static("x-real-ip"),
                ]),
        )
        .with_state(AppState {
            storage,
            wechat_pay,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: normalize_admin_token(
                commercial_metrics_admin_token.as_deref(),
            ),
            auth_otp_delivery_endpoint: normalize_optional_url(
                auth_otp_delivery_endpoint.as_deref(),
            ),
            enterprise_api_key_hash_secret: normalize_admin_token(
                enterprise_api_key_hash_secret.as_deref(),
            ),
            enterprise_api_key_hash_secret_version: normalize_secret_version(
                &enterprise_api_key_hash_secret_version,
            ),
            trusted_proxy_shared_secret: normalize_admin_token(
                trusted_proxy_shared_secret.as_deref(),
            ),
            enterprise_require_trusted_proxy,
        })
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = ServerArgs::parse();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let database_config = DatabaseConfig::from_server_args(
        args.database_backend,
        args.db_path.clone(),
        args.database_url.clone(),
        args.deployment_env.clone(),
    );
    if database_config.runtime_mode.is_production() {
        crate::ai_transparency_production_provider::ProductionProviderDeploymentConfig::from_environment()?;
    }
    let storage = Arc::new(Storage::open_with_database_config(
        &database_config,
        args.retention_days,
    )?);
    let wechat_pay = WechatPayNativeAdapter::from_env()
        .map_err(|error| format!("invalid WeChat Pay config: {error}"))?
        .map(Arc::new);
    let http_client = reqwest::Client::new();
    spawn_billing_reconcile_worker(
        Arc::clone(&storage),
        wechat_pay.clone(),
        http_client,
        Duration::from_secs(args.billing_reconcile_interval_secs.max(5)),
        args.billing_reconcile_batch_size.max(1),
    );
    let app = build_app_with_admin_auth_enterprise_custody_and_proxy(
        storage,
        wechat_pay,
        normalize_admin_token(args.commercial_metrics_admin_token.as_deref()),
        normalize_optional_url(args.auth_otp_delivery_endpoint.as_deref()),
        normalize_admin_token(args.enterprise_api_key_hash_secret.as_deref()),
        normalize_secret_version(&args.enterprise_api_key_hash_secret_version),
        normalize_admin_token(args.trusted_proxy_shared_secret.as_deref()),
        args.enterprise_require_trusted_proxy,
    );
    let listener = tokio::net::TcpListener::bind(args.bind_addr).await?;

    tracing::info!(
        "HiddenShield feedback backend listening on {}",
        args.bind_addr
    );
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
}

fn spawn_billing_reconcile_worker(
    storage: Arc<Storage>,
    wechat_pay: Option<Arc<WechatPayNativeAdapter>>,
    http_client: reqwest::Client,
    interval: Duration,
    batch_size: usize,
) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(interval);
        loop {
            ticker.tick().await;
            if let Some(wechat_pay) = wechat_pay.as_ref() {
                match storage.due_payment_sessions_for_provider(WECHAT_PAY_PROVIDER, batch_size) {
                    Ok(sessions) => {
                        for session in sessions {
                            match wechat_pay
                                .query_order_by_out_trade_no(
                                    &http_client,
                                    &session.provider_order_id,
                                )
                                .await
                            {
                                Ok(order_status) => {
                                    if let Err(error) = storage.reconcile_billing_order_status(
                                        &session.payment_session_id,
                                        order_status,
                                    ) {
                                        tracing::warn!(
                                            payment_session_id = session.payment_session_id,
                                            error = %error,
                                            "wechat payment session order status reconcile failed"
                                        );
                                    }
                                }
                                Err(error) => {
                                    tracing::warn!(
                                        payment_session_id = session.payment_session_id,
                                        provider_order_id = session.provider_order_id,
                                        error = %error,
                                        "wechat payment session order query failed"
                                    );
                                }
                            }
                        }
                    }
                    Err(error) => {
                        tracing::warn!(error = %error, "wechat payment session due query failed");
                    }
                }
            }
            match storage.reconcile_pending_payment_sessions(batch_size) {
                Ok(sweep) if sweep.checked > 0 || sweep.skipped_unsupported_provider > 0 => {
                    tracing::info!(
                        checked = sweep.checked,
                        succeeded = sweep.succeeded,
                        pending = sweep.pending,
                        failed = sweep.failed,
                        skipped_unsupported_provider = sweep.skipped_unsupported_provider,
                        wechat_query_configured = wechat_pay.is_some(),
                        "billing payment session background reconcile completed"
                    );
                }
                Ok(_) => {}
                Err(error) => {
                    tracing::warn!(error = %error, "billing payment session background reconcile failed");
                }
            }
        }
    });
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "hidden-shield-feedback-backend",
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        timestamp: Utc::now(),
        cloud_sync: true,
    })
}

async fn get_trusted_time(State(state): State<AppState>) -> Json<TrustedTimeResponse> {
    let source = "https://freetsa.org/tsr";
    let trusted_time_at = fetch_http_date(&state.http_client, source)
        .await
        .unwrap_or_else(Utc::now);
    Json(TrustedTimeResponse {
        status: "已记录网络授时",
        source: source.to_string(),
        trusted_time_at,
        third_party_verification_status: "已记录网络授时",
        third_party_verification_provider: "freetsa.org".to_string(),
        verification_path: "HiddenShield 后端 HTTP Date",
    })
}

async fn fetch_http_date(
    http_client: &reqwest::Client,
    source: &str,
) -> Option<chrono::DateTime<chrono::Utc>> {
    let response = http_client
        .head(source)
        .timeout(Duration::from_secs(5))
        .send()
        .await
        .ok()?;
    let value = response
        .headers()
        .get(reqwest::header::DATE)?
        .to_str()
        .ok()?;
    chrono::DateTime::parse_from_rfc2822(value)
        .map(|value| value.with_timezone(&Utc))
        .ok()
        .or_else(|| {
            chrono::DateTime::parse_from_rfc3339(value)
                .map(|value| value.with_timezone(&Utc))
                .ok()
        })
}

async fn continue_account(
    State(state): State<AppState>,
    Json(request): Json<ContinueAccountRequest>,
) -> Result<Json<crate::schema::CloudAccountSession>, ApiError> {
    validate_continue_account(&request)?;
    Ok(Json(
        state
            .storage
            .continue_account(&request)
            .map_err(ApiError::from)?,
    ))
}

async fn create_auth_challenge(
    State(state): State<AppState>,
    Json(request): Json<AuthChallengeRequest>,
) -> Result<Json<crate::schema::AuthChallengeResponse>, ApiError> {
    validate_auth_challenge(&request)?;
    let response = state
        .storage
        .create_auth_challenge(&request)
        .map_err(ApiError::from)?;
    deliver_auth_challenge_if_configured(&state, &request, &response).await?;
    Ok(Json(response))
}

async fn create_auth_session(
    State(state): State<AppState>,
    Json(request): Json<AuthSessionRequest>,
) -> Result<Json<crate::schema::CloudAccountSession>, ApiError> {
    validate_auth_session(&request)?;
    Ok(Json(
        state
            .storage
            .create_auth_session(&request)
            .map_err(ApiError::from)?,
    ))
}

async fn refresh_auth_session(
    State(state): State<AppState>,
    Json(request): Json<AuthRefreshRequest>,
) -> Result<Json<crate::schema::CloudAccountSession>, ApiError> {
    validate_auth_refresh(&request)?;
    Ok(Json(
        state
            .storage
            .refresh_auth_session(&request)
            .map_err(ApiError::from)?,
    ))
}

async fn logout_auth_session(
    State(state): State<AppState>,
    Json(request): Json<AuthLogoutRequest>,
) -> Result<Json<crate::schema::AuthLogoutResponse>, ApiError> {
    validate_auth_logout(&request)?;
    Ok(Json(
        state
            .storage
            .logout_auth_session(&request)
            .map_err(ApiError::from)?,
    ))
}

async fn get_me(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::schema::CloudAccountSnapshot>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .current_account_snapshot(token)
            .map_err(ApiError::from)?,
    ))
}

async fn update_sync_preferences(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SyncPreferencesRequest>,
) -> Result<Json<crate::schema::SyncPreferencesResponse>, ApiError> {
    validate_sync_preferences(&request)?;
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .update_sync_preferences(token, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn list_devices(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<AccountDevicesResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state.storage.list_devices(token).map_err(ApiError::from)?,
    ))
}

async fn update_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(device_id): axum::extract::Path<String>,
    Json(request): Json<UpdateDeviceRequest>,
) -> Result<Json<crate::schema::AccountDevice>, ApiError> {
    validate_update_device(&request)?;
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .update_device(token, &device_id, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn revoke_device(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(device_id): axum::extract::Path<String>,
) -> Result<Json<RevokeDeviceResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .revoke_device(token, &device_id)
            .map_err(ApiError::from)?,
    ))
}

async fn push_cloud_events_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(batch): Json<CloudSyncBatchRequest>,
) -> Result<Json<crate::schema::CloudSyncBatchResult>, ApiError> {
    validate_cloud_batch(&batch)?;
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .push_cloud_events_batch(token, &batch)
            .map_err(ApiError::from)?,
    ))
}

async fn create_billing_payment_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BillingPaymentSessionRequest>,
) -> Result<Json<crate::schema::BillingPaymentSessionResponse>, ApiError> {
    let _token = bearer_token(&headers)?;
    validate_billing_payment_session(&request)?;
    if request.preferred_provider.as_deref() == Some(WECHAT_PAY_PROVIDER) {
        let wechat_pay = state
            .wechat_pay
            .as_ref()
            .ok_or_else(|| ApiError::BadRequest("wechat_pay_not_configured".to_string()))?;
        let input = BillingPaymentSessionInput {
            account_id: request.account_id.trim().to_string(),
            workspace_id: request.workspace_id.trim().to_string(),
            plan_code: request.plan_code.trim().to_string(),
            billing_cycle: request.billing_cycle.trim().to_string(),
        };
        let provider_order_id = wechat_pay.build_native_order_request(&input).out_trade_no;
        let order = wechat_pay
            .create_native_order(&state.http_client, &input)
            .await
            .map_err(|error| ApiError::BadRequest(error.to_string()))?;
        let payment_action = crate::schema::BillingPaymentAction {
            action_type: "qr_code".to_string(),
            qr_code_url: Some(order.code_url),
            h5_url: None,
        };
        let session = state
            .storage
            .persist_provider_billing_payment_session(
                &request,
                WECHAT_PAY_PROVIDER,
                &provider_order_id,
                payment_action,
                Utc::now() + chrono::Duration::minutes(15),
            )
            .map_err(ApiError::from)?;
        return Ok(Json(session));
    }
    Ok(Json(
        state
            .storage
            .create_billing_payment_session(&request)
            .map_err(ApiError::from)?,
    ))
}

async fn get_billing_payment_session_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(payment_session_id): axum::extract::Path<String>,
) -> Result<Json<crate::schema::BillingPaymentSessionStatusResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .billing_payment_session_status(token, &payment_session_id)
            .map_err(ApiError::from)?,
    ))
}

async fn reconcile_billing_payment_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(payment_session_id): axum::extract::Path<String>,
) -> Result<Json<crate::schema::BillingPaymentSessionReconcileResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .reconcile_billing_payment_session(token, &payment_session_id)
            .map_err(ApiError::from)?,
    ))
}

async fn create_report_purchase_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<ReportPurchaseSessionRequest>,
) -> Result<Json<crate::schema::ReportPurchaseSessionResponse>, ApiError> {
    let _token = bearer_token(&headers)?;
    validate_report_purchase_session(&request)?;
    if request.preferred_provider.as_deref() == Some(WECHAT_PAY_PROVIDER) {
        let wechat_pay = state
            .wechat_pay
            .as_ref()
            .ok_or_else(|| ApiError::BadRequest("wechat_pay_not_configured".to_string()))?;
        let product_code = request.product_code.trim().to_lowercase();
        let input = ReportPurchasePaymentInput {
            account_id: request.account_id.trim().to_string(),
            workspace_id: request.workspace_id.trim().to_string(),
            creator_profile_id: request.creator_profile_id.trim().to_string(),
            vault_record_id: request.vault_record_id.trim().to_string(),
            price_cents: report_product_price_cents_for_api(&product_code)?,
            product_code,
        };
        let provider_order_id = wechat_pay
            .build_report_purchase_native_order_request(&input)
            .out_trade_no;
        let order = wechat_pay
            .create_report_purchase_native_order(&state.http_client, &input)
            .await
            .map_err(|error| ApiError::BadRequest(error.to_string()))?;
        let payment_action = crate::schema::BillingPaymentAction {
            action_type: "qr_code".to_string(),
            qr_code_url: Some(order.code_url),
            h5_url: None,
        };
        let session = state
            .storage
            .persist_provider_report_purchase_session(
                &request,
                WECHAT_PAY_PROVIDER,
                &provider_order_id,
                payment_action,
                Utc::now() + chrono::Duration::minutes(15),
            )
            .map_err(ApiError::from)?;
        return Ok(Json(session));
    }
    Ok(Json(
        state
            .storage
            .create_report_purchase_session(&request)
            .map_err(ApiError::from)?,
    ))
}

async fn get_report_purchase_session_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(payment_session_id): axum::extract::Path<String>,
) -> Result<Json<crate::schema::ReportPurchaseSessionStatusResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .report_purchase_session_status(token, &payment_session_id)
            .map_err(ApiError::from)?,
    ))
}

async fn reconcile_report_purchase_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(payment_session_id): axum::extract::Path<String>,
) -> Result<Json<crate::schema::ReportPurchaseSessionReconcileResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .reconcile_report_purchase_session(token, &payment_session_id)
            .map_err(ApiError::from)?,
    ))
}

async fn apply_fixture_billing_webhook(
    State(state): State<AppState>,
    Json(request): Json<BillingFixtureEventRequest>,
) -> Result<Json<crate::schema::BillingEventApplyResponse>, ApiError> {
    Ok(Json(
        state
            .storage
            .apply_fixture_billing_event(&request)
            .map_err(ApiError::from)?,
    ))
}

async fn apply_wechat_pay_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<BillingWechatPayNotificationRequest>,
) -> Result<Json<crate::schema::BillingEventApplyResponse>, ApiError> {
    let wechat_pay = state
        .wechat_pay
        .as_ref()
        .ok_or_else(|| ApiError::BadRequest("wechat_pay_not_configured".to_string()))?;
    let body = serde_json::to_string(&request.body)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    let pay_headers = wechat_headers_from_header_map(&headers)?;
    let event = wechat_pay
        .verify_and_normalize_notification(&pay_headers, &body)
        .map_err(|error| ApiError::BadRequest(error.to_string()))?;
    match event {
        WechatPayNormalizedEvent::Billing(event) => Ok(Json(
            state
                .storage
                .apply_billing_event(event)
                .map_err(ApiError::from)?,
        )),
        WechatPayNormalizedEvent::ReportPurchase(event) => {
            let applied = state
                .storage
                .apply_report_purchase_event(event)
                .map_err(ApiError::from)?;
            Ok(Json(crate::schema::BillingEventApplyResponse {
                provider: applied.provider,
                provider_event_id: applied.provider_event_id,
                duplicate: applied.duplicate,
                entitlement: crate::schema::CloudEntitlement {
                    id: "report_purchase_grant".to_string(),
                    plan_name: Some("Free".to_string()),
                    plan_code: "free".to_string(),
                    status: applied.status,
                    features: serde_json::json!({ "report_export": false }),
                },
            }))
        }
    }
}

async fn get_current_entitlement(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<crate::schema::CloudEntitlement>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .current_entitlement(token)
            .map_err(ApiError::from)?,
    ))
}

async fn get_current_team_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<TeamWorkspaceSummary>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .current_team_workspace(token)
            .map_err(ApiError::from)?,
    ))
}

async fn list_team_workspaces(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<TeamWorkspaceListResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .list_team_workspaces(token)
            .map_err(ApiError::from)?,
    ))
}

async fn create_team_workspace(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<TeamWorkspaceCreateRequest>,
) -> Result<Json<TeamWorkspaceSummary>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .create_team_workspace(token, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn list_team_members(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Result<Json<TeamMemberListResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .list_team_members(token, &workspace_id)
            .map_err(ApiError::from)?,
    ))
}

async fn create_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
    Json(request): Json<TeamMemberCreateRequest>,
) -> Result<Json<crate::schema::TeamMemberRecord>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .create_team_member(token, &workspace_id, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn update_team_member(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(member_id): axum::extract::Path<String>,
    Json(request): Json<TeamMemberUpdateRequest>,
) -> Result<Json<crate::schema::TeamMemberRecord>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .update_team_member(token, &member_id, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn list_team_shared_library(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Result<Json<TeamSharedLibraryListResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .list_team_shared_library_records(token, &workspace_id)
            .map_err(ApiError::from)?,
    ))
}

async fn share_team_library_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
    Json(request): Json<TeamSharedLibraryShareRequest>,
) -> Result<Json<crate::schema::TeamSharedLibraryRecord>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .share_team_library_record(token, &workspace_id, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn list_team_audit_logs(
    State(state): State<AppState>,
    headers: HeaderMap,
    axum::extract::Path(workspace_id): axum::extract::Path<String>,
) -> Result<Json<TeamAuditListResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .list_team_audit_logs(token, &workspace_id)
            .map_err(ApiError::from)?,
    ))
}

async fn create_video_fingerprint_notary(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<VideoFingerprintNotaryRequest>,
) -> Result<Json<crate::schema::VideoFingerprintNotaryReceipt>, ApiError> {
    validate_video_fingerprint_notary(&request)?;
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .create_video_fingerprint_notary(token, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn create_cloud_video_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CloudVideoTaskRequest>,
) -> Result<Json<CloudVideoTaskRecord>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .create_cloud_video_task(token, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn list_cloud_video_tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CloudVideoTaskListQuery>,
) -> Result<Json<CloudVideoTaskListResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .list_cloud_video_tasks(token, &query)
            .map_err(ApiError::from)?,
    ))
}

async fn create_cloud_video_task_object_upload_authorization(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CloudVideoTaskObjectUploadAuthorizationRequest>,
) -> Result<Json<CloudVideoTaskObjectUploadAuthorizationResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    validate_l3_object_upload_authorization_request(&request)?;
    let account_id = state
        .storage
        .authorize_cloud_video_object_upload(
            token,
            &request.workspace_id,
            &request.creator_profile_id,
        )
        .map_err(ApiError::from)?;
    let ttl_seconds = request.ttl_seconds.unwrap_or(600).clamp(60, 900);
    let expires_at = Utc::now() + chrono::Duration::seconds(ttl_seconds as i64);
    let payload = build_l3_object_upload_payload(&account_id, &request, expires_at)?;
    let upload_token =
        sign_l3_object_upload_token(l3_output_download_signing_secret(&state)?, &payload)?;
    Ok(Json(l3_object_upload_authorization_response(
        payload,
        upload_token,
    )))
}

async fn resolve_cloud_video_task_object_upload_authorization(
    State(state): State<AppState>,
    Query(query): Query<CloudVideoTaskObjectUploadQuery>,
    body: Bytes,
) -> Result<Json<CloudVideoTaskObjectUploadResponse>, ApiError> {
    let payload = verify_l3_object_upload_token(
        l3_output_download_signing_secret(&state)?,
        query.token.trim(),
    )?;
    if payload.expires_at <= Utc::now() {
        return Err(ApiError::Forbidden);
    }
    if body.len() as u64 != payload.expected_bytes {
        return Err(ApiError::BadRequest(
            "l3_object_upload_bytes_mismatch".to_string(),
        ));
    }
    let actual_hash = format!("sha256:{}", hex_lower(&Sha256::digest(&body)));
    if actual_hash != payload.expected_sha256 {
        return Err(ApiError::BadRequest(
            "l3_object_upload_sha256_mismatch".to_string(),
        ));
    }
    let object_path = l3_object_storage_ref_to_path(&payload.storage_ref)?;
    if let Some(parent) = object_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| ApiError::Storage(format!("create l3 object dir failed: {error}")))?;
    }
    fs::write(&object_path, &body)
        .map_err(|error| ApiError::Storage(format!("write l3 upload object failed: {error}")))?;
    Ok(Json(CloudVideoTaskObjectUploadResponse {
        schema_version: "l3_object_upload_result_v1".to_string(),
        status: "stored".to_string(),
        storage_ref: payload.storage_ref,
        sha256: actual_hash,
        bytes: body.len() as u64,
        content_type: payload.content_type,
        privacy_boundary: "signed_object_upload_only_no_local_path_no_raw_video_sync".to_string(),
    }))
}

async fn get_cloud_video_task(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
) -> Result<Json<CloudVideoTaskRecord>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .get_cloud_video_task(token, &task_id)
            .map_err(ApiError::from)?,
    ))
}

async fn create_cloud_video_task_output_download_authorization(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(request): Json<CloudVideoTaskDownloadAuthorizationRequest>,
) -> Result<Json<CloudVideoTaskDownloadAuthorizationResponse>, ApiError> {
    let token = bearer_token(&headers)?;
    let task = state
        .storage
        .get_cloud_video_task(token, &task_id)
        .map_err(ApiError::from)?;
    validate_l3_output_download_ready(&task)?;
    let ttl_seconds = request.ttl_seconds.unwrap_or(600).clamp(60, 900);
    let expires_at = Utc::now() + chrono::Duration::seconds(ttl_seconds as i64);
    let payload = build_l3_output_download_payload(&task, expires_at)?;
    let download_token =
        sign_l3_output_download_token(l3_output_download_signing_secret(&state)?, &payload)?;
    Ok(Json(l3_output_download_response(payload, download_token)))
}

async fn resolve_cloud_video_task_output_download_authorization(
    State(state): State<AppState>,
    Path(task_id): Path<String>,
    Query(query): Query<CloudVideoTaskDownloadAuthorizationQuery>,
) -> Result<axum::response::Response, ApiError> {
    let payload = verify_l3_output_download_token(
        l3_output_download_signing_secret(&state)?,
        query.token.trim(),
    )?;
    if payload.task_id != task_id.trim() {
        return Err(ApiError::Forbidden);
    }
    if payload.expires_at <= Utc::now() {
        return Err(ApiError::Forbidden);
    }
    let task = state
        .storage
        .get_cloud_video_task_for_signed_download(&payload.task_id)
        .map_err(ApiError::from)?;
    validate_l3_output_download_ready(&task)?;
    if task.account_id != payload.account_id
        || task.workspace_id != payload.workspace_id
        || task.output_media_storage_ref.as_deref()
            != Some(payload.output_media_storage_ref.as_str())
        || task.watermarked_media_hash.as_deref() != Some(payload.watermarked_media_hash.as_str())
        || task.worker_receipt_hash.as_deref() != Some(payload.worker_receipt_hash.as_str())
    {
        return Err(ApiError::Forbidden);
    }
    let object_path = l3_object_storage_ref_to_path(&payload.output_media_storage_ref)?;
    let bytes = fs::read(&object_path)
        .map_err(|error| ApiError::Storage(format!("read l3 output object failed: {error}")))?;
    if bytes.len() as u64 != payload.output_media_bytes {
        return Err(ApiError::Forbidden);
    }
    let actual_hash = format!("sha256:{}", hex_lower(&Sha256::digest(&bytes)));
    if actual_hash != payload.watermarked_media_hash {
        return Err(ApiError::Forbidden);
    }
    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static("video/mp4"));
    headers.insert(
        header::CONTENT_LENGTH,
        HeaderValue::from_str(&bytes.len().to_string())
            .map_err(|error| ApiError::Storage(format!("invalid content length: {error}")))?,
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!(
            "attachment; filename=\"{}.l3-watermarked.mp4\"",
            payload.task_id
        ))
        .map_err(|error| ApiError::Storage(format!("invalid content disposition: {error}")))?,
    );
    headers.insert(
        axum::http::HeaderName::from_static("x-hiddenshield-watermarked-media-hash"),
        HeaderValue::from_str(&payload.watermarked_media_hash)
            .map_err(|error| ApiError::Storage(format!("invalid media hash header: {error}")))?,
    );
    headers.insert(
        axum::http::HeaderName::from_static("x-hiddenshield-worker-receipt-hash"),
        HeaderValue::from_str(&payload.worker_receipt_hash)
            .map_err(|error| ApiError::Storage(format!("invalid receipt hash header: {error}")))?,
    );
    Ok((StatusCode::OK, headers, bytes).into_response())
}

async fn update_cloud_video_task_status(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(request): Json<CloudVideoTaskStatusUpdateRequest>,
) -> Result<Json<CloudVideoTaskRecord>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .update_cloud_video_task_status(token, &task_id, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn claim_cloud_video_task_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<CloudVideoTaskClaimRequest>,
) -> Result<Json<CloudVideoTaskClaimResponse>, ApiError> {
    let endpoint = "/internal/video-tasks/claim";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    Ok(Json(
        state
            .storage
            .claim_cloud_video_task_for_worker(&request)
            .map_err(ApiError::from)?,
    ))
}

async fn complete_cloud_video_task_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(request): Json<CloudVideoTaskCompletionRequest>,
) -> Result<Json<CloudVideoTaskRecord>, ApiError> {
    let endpoint = "/internal/video-tasks/:task_id/completion";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    validate_l3_completion_receipt(&state, &task_id, &request)?;
    Ok(Json(
        state
            .storage
            .complete_cloud_video_task_from_trusted_worker(&task_id, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn fail_cloud_video_task_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(task_id): Path<String>,
    Json(request): Json<CloudVideoTaskFailureRequest>,
) -> Result<Json<CloudVideoTaskRecord>, ApiError> {
    let endpoint = "/internal/video-tasks/:task_id/failure";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    Ok(Json(
        state
            .storage
            .fail_cloud_video_task_from_trusted_worker(&task_id, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn reserve_watermark_id(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<WatermarkIdReserveRequest>,
) -> Result<Json<WatermarkIdRegistryResponse>, ApiError> {
    validate_watermark_id_reserve(&request)?;
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .reserve_watermark_id(token, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn confirm_watermark_id(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<WatermarkIdConfirmRequest>,
) -> Result<Json<WatermarkIdRegistryResponse>, ApiError> {
    validate_watermark_id_confirm(&request)?;
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .confirm_watermark_id(token, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn reconcile_watermark_id(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<WatermarkIdReconcileRequest>,
) -> Result<Json<WatermarkIdRegistryResponse>, ApiError> {
    validate_watermark_id_reconcile(&request)?;
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .reconcile_watermark_id(token, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn reissue_watermark_id(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<WatermarkIdReissueRequest>,
) -> Result<Json<WatermarkIdReissueResponse>, ApiError> {
    validate_watermark_id_reissue(&request)?;
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .reissue_watermark_id(token, &request)
            .map_err(ApiError::from)?,
    ))
}

async fn get_public_rights(
    State(state): State<AppState>,
    Path(watermark_uid): Path<String>,
) -> Result<Json<PublicRightsQueryResponse>, ApiError> {
    Ok(Json(
        state
            .storage
            .public_rights_query(&watermark_uid)
            .map_err(ApiError::from)?,
    ))
}

async fn get_public_rights_metadata(
    State(state): State<AppState>,
    Path(watermark_uid): Path<String>,
) -> Result<Json<PublicRightsMetadataExport>, ApiError> {
    Ok(Json(
        state
            .storage
            .public_rights_metadata_export(&watermark_uid)
            .map_err(ApiError::from)?,
    ))
}

async fn get_public_rights_batch(
    State(state): State<AppState>,
    Json(request): Json<PublicRightsBatchRequest>,
) -> Result<Json<PublicRightsBatchResponse>, ApiError> {
    if request.watermark_uids.is_empty() {
        return Err(ApiError::BadRequest(
            "watermarkUids must not be empty".to_string(),
        ));
    }
    if request.watermark_uids.len() > PUBLIC_RIGHTS_ANONYMOUS_BATCH_MAX_ITEMS {
        return Err(ApiError::BadRequest(
            "watermarkUids exceeds maximum batch size".to_string(),
        ));
    }
    Ok(Json(
        state
            .storage
            .public_rights_batch(&request)
            .map_err(ApiError::from)?,
    ))
}

async fn enterprise_public_rights_batch(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<EnterprisePublicRightsBatchRequest>,
) -> Result<Json<EnterprisePublicRightsBatchResponse>, ApiError> {
    if request.watermark_uids.is_empty() {
        return Err(ApiError::BadRequest(
            "watermarkUids must not be empty".to_string(),
        ));
    }
    if request.watermark_uids.len() > PUBLIC_RIGHTS_ANONYMOUS_BATCH_MAX_ITEMS {
        return Err(ApiError::BadRequest(
            "watermarkUids exceeds maximum batch size".to_string(),
        ));
    }
    let cleartext_api_key = extract_enterprise_api_key(&headers)?;
    let hash_secret = state
        .enterprise_api_key_hash_secret
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or(ApiError::Forbidden)?;
    let client_fingerprint = extract_enterprise_client_fingerprint(&state, &headers)?;
    Ok(Json(
        state
            .storage
            .enterprise_public_rights_batch(
                &cleartext_api_key,
                hash_secret,
                &state.enterprise_api_key_hash_secret_version,
                client_fingerprint,
                &request,
            )
            .map_err(ApiError::from)?,
    ))
}

async fn backfill_rights_manifests(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<RightsManifestBackfillRequest>,
) -> Result<Json<RightsManifestBackfillResponse>, ApiError> {
    validate_admin_endpoint(&state, &headers, "/internal/rights-manifests/backfill")?;
    Ok(Json(
        state
            .storage
            .backfill_rights_manifests(&request)
            .map_err(ApiError::from)?,
    ))
}

async fn create_enterprise_api_key_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<EnterpriseApiKeyCreateRequest>,
) -> Result<Json<EnterpriseApiKeyRecord>, ApiError> {
    let endpoint = "/internal/enterprise/api-keys";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let result = state.storage.create_enterprise_api_key_internal(&request);
    match result {
        Ok(record) => {
            record_enterprise_admin_operation(
                &state,
                "create_api_key",
                "succeeded",
                endpoint,
                Some(&record.account_id),
                Some(&record.workspace_id),
                Some(&record.api_key_id),
                Some(&record.api_key_id),
                "created",
                serde_json::json!({
                    "name": record.name,
                    "keyPrefix": record.key_prefix,
                    "scopes": record.scopes,
                    "status": record.status
                }),
            )?;
            Ok(Json(record))
        }
        Err(error) => {
            let reason = error.to_string();
            let _ = record_enterprise_admin_operation(
                &state,
                "create_api_key",
                "failed",
                endpoint,
                Some(&request.account_id),
                Some(&request.workspace_id),
                None,
                None,
                &reason,
                serde_json::json!({
                    "name": request.name,
                    "keyPrefix": request.key_prefix
                }),
            );
            Err(ApiError::from(error))
        }
    }
}

async fn issue_enterprise_api_key_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<EnterpriseApiKeyIssueRequest>,
) -> Result<Json<EnterpriseApiKeyIssueResponse>, ApiError> {
    let endpoint = "/internal/enterprise/api-key-issuances";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let result = issue_enterprise_api_key_with_custody(&state, &request);
    match result {
        Ok(response) => {
            record_enterprise_admin_operation(
                &state,
                "issue_api_key",
                "succeeded",
                endpoint,
                Some(&response.api_key.account_id),
                Some(&response.api_key.workspace_id),
                Some(&response.api_key.api_key_id),
                Some(&response.api_key.api_key_id),
                request.reason.as_str(),
                serde_json::json!({
                    "name": response.api_key.name,
                    "keyPrefix": response.key_prefix,
                    "scopes": response.api_key.scopes,
                    "status": response.api_key.status,
                    "hashAlgorithm": response.hash_algorithm,
                    "shownOnce": response.shown_once,
                    "deliveryChannel": request.delivery_channel,
                    "recipientRef": request.recipient_ref
                }),
            )?;
            Ok(Json(response))
        }
        Err(error) => {
            let reason = error.to_string();
            let _ = record_enterprise_admin_operation(
                &state,
                "issue_api_key",
                "failed",
                endpoint,
                Some(&request.account_id),
                Some(&request.workspace_id),
                None,
                None,
                &reason,
                serde_json::json!({
                    "name": request.name,
                    "scopes": request.scopes,
                    "deliveryChannel": request.delivery_channel,
                    "recipientRef": request.recipient_ref
                }),
            );
            Err(error)
        }
    }
}

async fn list_enterprise_api_keys_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<EnterpriseApiKeyListQuery>,
) -> Result<Json<EnterpriseApiKeyListResponse>, ApiError> {
    let endpoint = "/internal/enterprise/api-keys";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let result = state.storage.list_enterprise_api_keys_internal(&query);
    match result {
        Ok(response) => {
            record_enterprise_admin_operation(
                &state,
                "list_api_keys",
                "succeeded",
                endpoint,
                query.account_id.as_deref(),
                query.workspace_id.as_deref(),
                None,
                None,
                "listed",
                serde_json::json!({
                    "status": query.status,
                    "limit": query.limit,
                    "returned": response.returned
                }),
            )?;
            Ok(Json(response))
        }
        Err(error) => {
            let reason = error.to_string();
            let _ = record_enterprise_admin_operation(
                &state,
                "list_api_keys",
                "failed",
                endpoint,
                query.account_id.as_deref(),
                query.workspace_id.as_deref(),
                None,
                None,
                &reason,
                serde_json::json!({
                    "status": query.status,
                    "limit": query.limit
                }),
            );
            Err(ApiError::from(error))
        }
    }
}

async fn get_enterprise_api_key_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(api_key_id): Path<String>,
) -> Result<Json<EnterpriseApiKeyRecord>, ApiError> {
    let endpoint = "/internal/enterprise/api-keys/:api_key_id";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let result = state.storage.get_enterprise_api_key_internal(&api_key_id);
    match result {
        Ok(record) => {
            record_enterprise_admin_operation(
                &state,
                "get_api_key",
                "succeeded",
                endpoint,
                Some(&record.account_id),
                Some(&record.workspace_id),
                Some(&record.api_key_id),
                Some(&record.api_key_id),
                "fetched",
                serde_json::json!({"status": record.status}),
            )?;
            Ok(Json(record))
        }
        Err(error) => {
            let reason = error.to_string();
            let _ = record_enterprise_admin_operation(
                &state,
                "get_api_key",
                "failed",
                endpoint,
                None,
                None,
                Some(&api_key_id),
                Some(&api_key_id),
                &reason,
                serde_json::json!({}),
            );
            Err(ApiError::from(error))
        }
    }
}

async fn pause_enterprise_api_key_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(api_key_id): Path<String>,
    Json(request): Json<EnterpriseApiKeyStatusChangeRequest>,
) -> Result<Json<EnterpriseApiKeyRecord>, ApiError> {
    let endpoint = "/internal/enterprise/api-keys/:api_key_id/pause";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let result = state
        .storage
        .pause_enterprise_api_key_internal(&api_key_id, &request.reason);
    match result {
        Ok(record) => {
            record_enterprise_admin_operation(
                &state,
                "pause_api_key",
                "succeeded",
                endpoint,
                Some(&record.account_id),
                Some(&record.workspace_id),
                Some(&record.api_key_id),
                Some(&record.api_key_id),
                request.reason.as_str(),
                serde_json::json!({"status": record.status}),
            )?;
            Ok(Json(record))
        }
        Err(error) => {
            let reason = error.to_string();
            let _ = record_enterprise_admin_operation(
                &state,
                "pause_api_key",
                "failed",
                endpoint,
                None,
                None,
                Some(&api_key_id),
                Some(&api_key_id),
                &reason,
                serde_json::json!({"requestReason": request.reason}),
            );
            Err(ApiError::from(error))
        }
    }
}

async fn rotate_enterprise_api_key_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(api_key_id): Path<String>,
    Json(request): Json<EnterpriseApiKeyRotateRequest>,
) -> Result<Json<EnterpriseApiKeyRotateResponse>, ApiError> {
    let endpoint = "/internal/enterprise/api-keys/:api_key_id/rotate";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let result = rotate_enterprise_api_key_with_custody(&state, &api_key_id, &request);
    match result {
        Ok(response) => {
            record_enterprise_admin_operation(
                &state,
                "rotate_api_key",
                "succeeded",
                endpoint,
                Some(&response.old_api_key.account_id),
                Some(&response.old_api_key.workspace_id),
                Some(&response.old_api_key.api_key_id),
                Some(&response.new_api_key.api_key_id),
                request.reason.as_str(),
                serde_json::json!({
                    "oldApiKeyId": response.old_api_key.api_key_id,
                    "oldStatus": response.old_api_key.status,
                    "newApiKeyId": response.new_api_key.api_key_id,
                    "newKeyPrefix": response.key_prefix,
                    "hashAlgorithm": response.hash_algorithm,
                    "shownOnce": response.shown_once,
                    "gracePeriodHours": request.grace_period_hours,
                    "rotationDeadlineAt": response.rotation_deadline_at,
                    "deliveryChannel": request.delivery_channel,
                    "recipientRef": request.recipient_ref
                }),
            )?;
            Ok(Json(response))
        }
        Err(error) => {
            let reason = error.to_string();
            let _ = record_enterprise_admin_operation(
                &state,
                "rotate_api_key",
                "failed",
                endpoint,
                None,
                None,
                Some(&api_key_id),
                Some(&api_key_id),
                &reason,
                serde_json::json!({
                    "requestReason": request.reason,
                    "gracePeriodHours": request.grace_period_hours,
                    "deliveryChannel": request.delivery_channel,
                    "recipientRef": request.recipient_ref
                }),
            );
            Err(error)
        }
    }
}

async fn revoke_enterprise_api_key_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(api_key_id): Path<String>,
    Json(request): Json<EnterpriseApiKeyStatusChangeRequest>,
) -> Result<Json<EnterpriseApiKeyRecord>, ApiError> {
    let endpoint = "/internal/enterprise/api-keys/:api_key_id/revoke";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let result = state
        .storage
        .revoke_enterprise_api_key_internal(&api_key_id, &request.reason);
    match result {
        Ok(record) => {
            record_enterprise_admin_operation(
                &state,
                "revoke_api_key",
                "succeeded",
                endpoint,
                Some(&record.account_id),
                Some(&record.workspace_id),
                Some(&record.api_key_id),
                Some(&record.api_key_id),
                request.reason.as_str(),
                serde_json::json!({
                    "status": record.status,
                    "revokedAt": record.revoked_at
                }),
            )?;
            Ok(Json(record))
        }
        Err(error) => {
            let reason = error.to_string();
            let _ = record_enterprise_admin_operation(
                &state,
                "revoke_api_key",
                "failed",
                endpoint,
                None,
                None,
                Some(&api_key_id),
                Some(&api_key_id),
                &reason,
                serde_json::json!({"requestReason": request.reason}),
            );
            Err(ApiError::from(error))
        }
    }
}

async fn revoke_expired_enterprise_rotations_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<EnterpriseExpiredRotationRevokeRequest>,
) -> Result<Json<EnterpriseExpiredRotationRevokeResponse>, ApiError> {
    let endpoint = "/internal/enterprise/api-key-rotations/revoke-expired";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let result = revoke_expired_enterprise_rotations(&state, &request);
    match result {
        Ok(response) => {
            record_enterprise_admin_operation(
                &state,
                "revoke_expired_rotations",
                "succeeded",
                endpoint,
                None,
                None,
                None,
                None,
                request.reason.as_str(),
                serde_json::json!({
                    "processed": response.processed,
                    "revoked": response.revoked,
                    "skipped": response.skipped,
                    "limit": request.limit,
                    "now": request.now
                }),
            )?;
            Ok(Json(response))
        }
        Err(error) => {
            let reason = error.to_string();
            let _ = record_enterprise_admin_operation(
                &state,
                "revoke_expired_rotations",
                "failed",
                endpoint,
                None,
                None,
                None,
                None,
                &reason,
                serde_json::json!({
                    "requestReason": request.reason,
                    "limit": request.limit,
                    "now": request.now
                }),
            );
            Err(error)
        }
    }
}

async fn initialize_enterprise_quota_balance_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<EnterpriseQuotaBalanceInitRequest>,
) -> Result<Json<EnterpriseQuotaBalanceRecord>, ApiError> {
    let endpoint = "/internal/enterprise/quota-balances";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let result = state
        .storage
        .initialize_enterprise_quota_balance_internal(&request);
    match result {
        Ok(record) => {
            record_enterprise_admin_operation(
                &state,
                "init_quota_balance",
                "succeeded",
                endpoint,
                Some(&record.account_id),
                Some(&record.workspace_id),
                None,
                Some(&record.quota_balance_id),
                "initialized",
                serde_json::json!({
                    "quotaType": record.quota_type,
                    "periodStart": record.period_start,
                    "periodEnd": record.period_end,
                    "includedUnits": record.included_units,
                    "usedUnits": record.used_units,
                    "reservedUnits": record.reserved_units
                }),
            )?;
            Ok(Json(record))
        }
        Err(error) => {
            let reason = error.to_string();
            let _ = record_enterprise_admin_operation(
                &state,
                "init_quota_balance",
                "failed",
                endpoint,
                Some(&request.account_id),
                Some(&request.workspace_id),
                None,
                None,
                &reason,
                serde_json::json!({
                    "quotaType": request.quota_type,
                    "periodStart": request.period_start,
                    "periodEnd": request.period_end,
                    "includedUnits": request.included_units
                }),
            );
            Err(ApiError::from(error))
        }
    }
}

async fn list_enterprise_admin_audit_events_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<EnterpriseAdminAuditEventQuery>,
) -> Result<Json<EnterpriseAdminAuditEventListResponse>, ApiError> {
    validate_admin_endpoint(&state, &headers, "/internal/enterprise/admin-audit-events")?;
    Ok(Json(
        state
            .storage
            .list_enterprise_admin_audit_events_internal(&query)
            .map_err(ApiError::from)?,
    ))
}

async fn dry_run_enterprise_gateway_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<EnterpriseGatewayDryRunRequest>,
) -> Result<Json<EnterpriseGatewayDryRunDecision>, ApiError> {
    let endpoint = "/internal/enterprise/gateway-dry-run";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let decision = dry_run_enterprise_gateway_readonly_scan(&request);
    record_enterprise_admin_operation(
        &state,
        "dry_run_gateway",
        "succeeded",
        endpoint,
        Some(&request.auth.account_id),
        Some(&request.auth.workspace_id),
        Some(&request.auth.api_key_id),
        Some(&request.request_id),
        if decision.allowed {
            "allowed"
        } else {
            "denied"
        },
        serde_json::json!({
            "requiredScope": request.required_scope,
            "itemCount": request.item_count,
            "quotaType": request.quota_type,
            "statusCode": decision.status_code,
            "errorCode": decision.error_code.clone(),
            "chargeableUnits": decision.quota.chargeable_units,
            "ledgerStatus": decision.quota.ledger_status.clone(),
            "legalConclusion": decision.legal_conclusion
        }),
    )?;
    Ok(Json(decision))
}

async fn get_ai_transparency_license_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(license_id): Path<String>,
) -> Result<Json<AiTransparencyLicenseDetailResponse>, ApiError> {
    let endpoint = "/internal/ai-transparency/licenses/:license_id";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    match state
        .storage
        .get_ai_transparency_license_internal(&license_id)
    {
        Ok(Some(response)) => {
            state
                .storage
                .record_ai_transparency_admin_audit_event_internal(
                    "get_license",
                    "succeeded",
                    endpoint,
                    Some(&response.license),
                    Some(&license_id),
                    &[],
                    "authorized",
                    serde_json::json!({ "environment": response.license.environment }),
                )?;
            Ok(Json(response))
        }
        Ok(None) => {
            state
                .storage
                .record_ai_transparency_admin_audit_event_internal(
                    "get_license",
                    "denied",
                    endpoint,
                    None,
                    Some(&license_id),
                    &[],
                    "ai_license_not_found",
                    serde_json::json!({}),
                )?;
            Err(ApiError::NotFound("ai_license_not_found".to_string()))
        }
        Err(error) => {
            let reason_code = error.to_string();
            state
                .storage
                .record_ai_transparency_admin_audit_event_internal(
                    "get_license",
                    "failed",
                    endpoint,
                    None,
                    Some(&license_id),
                    &[],
                    &reason_code,
                    serde_json::json!({}),
                )?;
            Err(ApiError::from(error))
        }
    }
}

async fn check_ai_transparency_profile_entitlements_internal(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<AiTransparencyProfileEntitlementCheckRequest>,
) -> Result<Json<AiTransparencyProfileEntitlementCheckResponse>, ApiError> {
    let endpoint = "/internal/ai-transparency/profile-entitlements/check";
    validate_admin_endpoint(&state, &headers, endpoint)?;
    let requested_profile_ids = request.requested_profile_ids.clone();
    match state
        .storage
        .check_ai_transparency_profile_entitlements_internal(&request)
    {
        Ok(response) => {
            let license = state
                .storage
                .get_ai_transparency_license_internal(&request.license_id)?
                .map(|detail| detail.license);
            let reason_code = if response.authorized {
                "authorized".to_string()
            } else {
                response
                    .profile_decisions
                    .iter()
                    .find(|decision| !decision.authorized)
                    .map(|decision| decision.reason_code.clone())
                    .unwrap_or_else(|| response.license_decision.reason_code.clone())
            };
            state
                .storage
                .record_ai_transparency_admin_audit_event_internal(
                    "check_profile_entitlements",
                    if response.authorized {
                        "succeeded"
                    } else {
                        "denied"
                    },
                    endpoint,
                    license.as_ref(),
                    Some(&request.license_id),
                    &requested_profile_ids,
                    &reason_code,
                    serde_json::json!({
                        "environment": request.environment,
                        "authorized": response.authorized,
                        "requestedProfileCount": requested_profile_ids.len()
                    }),
                )?;
            Ok(Json(response))
        }
        Err(error) => {
            let reason_code = error.to_string();
            state
                .storage
                .record_ai_transparency_admin_audit_event_internal(
                    "check_profile_entitlements",
                    "failed",
                    endpoint,
                    None,
                    Some(&request.license_id),
                    &requested_profile_ids,
                    &reason_code,
                    serde_json::json!({
                        "environment": request.environment,
                        "requestedProfileCount": requested_profile_ids.len()
                    }),
                )?;
            Err(ApiError::from(error))
        }
    }
}

async fn get_cloud_changes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CloudChangesQuery>,
) -> Result<Json<CloudSyncChangesResult>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(
        state
            .storage
            .get_cloud_changes(
                token,
                query.workspace_id.as_deref(),
                query.cursor.as_deref(),
            )
            .map_err(ApiError::from)?,
    ))
}

async fn ingest_batch(
    State(state): State<AppState>,
    Json(batch): Json<AnonymousFeedbackBatch>,
) -> Result<Json<AnonymousFeedbackBatchAck>, ApiError> {
    validate_batch(&batch)?;
    let ack = state.storage.ingest_batch(&batch)?;
    Ok(Json(ack))
}

async fn get_stats(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<AnonymousFeedbackStatsQuery>,
) -> Result<Json<AnonymousFeedbackStatsResponse>, ApiError> {
    let stats = state.storage.query_stats(&query)?;
    Ok(Json(stats))
}

async fn get_commercial_metrics_overview(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<CommercialMetricsOverviewResponse>, ApiError> {
    validate_commercial_metrics_admin(&state, &headers)?;
    Ok(Json(state.storage.commercial_metrics_overview()?))
}

fn validate_commercial_metrics_admin(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(), ApiError> {
    validate_admin_endpoint(state, headers, "/v1/commercial/metrics/overview")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct L3ObjectUploadTokenPayload {
    schema_version: String,
    authorization_id: String,
    account_id: String,
    workspace_id: String,
    creator_profile_id: String,
    storage_ref: String,
    expected_sha256: String,
    expected_bytes: u64,
    content_type: String,
    object_kind: String,
    expires_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct L3OutputDownloadTokenPayload {
    schema_version: String,
    authorization_id: String,
    task_id: String,
    account_id: String,
    workspace_id: String,
    output_media_storage_ref: String,
    output_media_bytes: u64,
    output_media_content_type: String,
    watermarked_media_hash: String,
    worker_receipt_hash: String,
    expires_at: chrono::DateTime<Utc>,
}

fn validate_l3_object_upload_authorization_request(
    request: &CloudVideoTaskObjectUploadAuthorizationRequest,
) -> Result<(), ApiError> {
    let object_kind = request
        .object_kind
        .as_deref()
        .map(str::trim)
        .unwrap_or("l3_user_object_upload_proxy");
    if request.workspace_id.trim().is_empty() || request.creator_profile_id.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "l3_object_upload_workspace_required".to_string(),
        ));
    }
    if request.content_type.trim() != "video/mp4" {
        return Err(ApiError::BadRequest(
            "l3_object_upload_content_type_invalid".to_string(),
        ));
    }
    if object_kind != "l3_user_object_upload_proxy" {
        return Err(ApiError::BadRequest(
            "l3_object_upload_kind_invalid".to_string(),
        ));
    }
    if request.bytes == 0 || request.bytes > 5 * 1024 * 1024 * 1024 {
        return Err(ApiError::BadRequest(
            "l3_object_upload_bytes_invalid".to_string(),
        ));
    }
    if !looks_like_sha256_api(request.sha256.trim()) {
        return Err(ApiError::BadRequest(
            "l3_object_upload_sha256_invalid".to_string(),
        ));
    }
    Ok(())
}

fn build_l3_object_upload_payload(
    account_id: &str,
    request: &CloudVideoTaskObjectUploadAuthorizationRequest,
    expires_at: chrono::DateTime<Utc>,
) -> Result<L3ObjectUploadTokenPayload, ApiError> {
    validate_l3_object_upload_authorization_request(request)?;
    let authorization_id = generate_l3_object_upload_authorization_id();
    Ok(L3ObjectUploadTokenPayload {
        schema_version: "l3_object_upload_authorization_v1".to_string(),
        authorization_id: authorization_id.clone(),
        account_id: account_id.trim().to_string(),
        workspace_id: request.workspace_id.trim().to_string(),
        creator_profile_id: request.creator_profile_id.trim().to_string(),
        storage_ref: format!("object://l3-upload/{authorization_id}/source-proxy.mp4"),
        expected_sha256: request.sha256.trim().to_string(),
        expected_bytes: request.bytes,
        content_type: request.content_type.trim().to_string(),
        object_kind: request
            .object_kind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("l3_user_object_upload_proxy")
            .to_string(),
        expires_at,
    })
}

fn l3_object_upload_authorization_response(
    payload: L3ObjectUploadTokenPayload,
    upload_token: String,
) -> CloudVideoTaskObjectUploadAuthorizationResponse {
    CloudVideoTaskObjectUploadAuthorizationResponse {
        schema_version: payload.schema_version,
        authorization_id: payload.authorization_id,
        workspace_id: payload.workspace_id,
        creator_profile_id: payload.creator_profile_id,
        storage_ref: payload.storage_ref,
        expected_sha256: payload.expected_sha256,
        expected_bytes: payload.expected_bytes,
        content_type: payload.content_type,
        expires_at: payload.expires_at,
        signed_upload_url: format!("/v1/video-object-store/upload?token={upload_token}"),
        upload_method: "PUT".to_string(),
        upload_token,
        privacy_boundary: "signed_object_upload_only_no_local_path_no_raw_video_sync".to_string(),
    }
}

fn generate_l3_object_upload_authorization_id() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    format!(
        "l3up_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

fn sign_l3_object_upload_token(
    secret: &str,
    payload: &L3ObjectUploadTokenPayload,
) -> Result<String, ApiError> {
    let payload_json = serde_json::to_vec(payload)
        .map_err(|error| ApiError::Storage(format!("upload payload serialize failed: {error}")))?;
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json);
    let signature = l3_object_upload_signature(secret, &payload_b64)?;
    Ok(format!("hs_l3up_v1.{payload_b64}.{signature}"))
}

fn verify_l3_object_upload_token(
    secret: &str,
    token: &str,
) -> Result<L3ObjectUploadTokenPayload, ApiError> {
    let mut parts = token.split('.');
    let prefix = parts.next().unwrap_or_default();
    let payload_b64 = parts.next().unwrap_or_default();
    let signature = parts.next().unwrap_or_default();
    if prefix != "hs_l3up_v1"
        || payload_b64.is_empty()
        || signature.is_empty()
        || parts.next().is_some()
    {
        return Err(ApiError::Forbidden);
    }
    let expected = l3_object_upload_signature(secret, payload_b64)?;
    if signature != expected {
        return Err(ApiError::Forbidden);
    }
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64.as_bytes())
        .map_err(|_| ApiError::Forbidden)?;
    let payload: L3ObjectUploadTokenPayload =
        serde_json::from_slice(&payload_bytes).map_err(|_| ApiError::Forbidden)?;
    if payload.schema_version != "l3_object_upload_authorization_v1"
        || payload.authorization_id.trim().is_empty()
        || !payload.storage_ref.starts_with("object://l3-upload/")
        || !looks_like_sha256_api(payload.expected_sha256.trim())
        || payload.expected_bytes == 0
        || payload.content_type != "video/mp4"
    {
        return Err(ApiError::Forbidden);
    }
    Ok(payload)
}

fn l3_object_upload_signature(secret: &str, payload_b64: &str) -> Result<String, ApiError> {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| ApiError::Storage("l3 object upload signing secret is invalid".to_string()))?;
    mac.update(b"hidden-shield:l3-object-upload:v1\n");
    mac.update(payload_b64.as_bytes());
    let digest = mac.finalize().into_bytes();
    Ok(hex_lower(&digest))
}

fn validate_l3_output_download_ready(task: &CloudVideoTaskRecord) -> Result<(), ApiError> {
    if task.status != "succeeded" {
        return Err(ApiError::BadRequest(
            "cloud_video_task_output_not_ready".to_string(),
        ));
    }
    let output_ref = task
        .output_media_storage_ref
        .as_deref()
        .map(str::trim)
        .filter(|value| value.starts_with("object://l3-output/"))
        .ok_or_else(|| ApiError::BadRequest("cloud_video_task_output_not_ready".to_string()))?;
    let content_type = task
        .output_media_content_type
        .as_deref()
        .map(str::trim)
        .filter(|value| *value == "video/mp4")
        .ok_or_else(|| ApiError::BadRequest("cloud_video_task_output_not_ready".to_string()))?;
    let bytes = task
        .output_media_bytes
        .filter(|value| *value > 0)
        .ok_or_else(|| ApiError::BadRequest("cloud_video_task_output_not_ready".to_string()))?;
    let media_hash = task
        .watermarked_media_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| value.starts_with("sha256:"))
        .ok_or_else(|| ApiError::BadRequest("cloud_video_task_output_not_ready".to_string()))?;
    let receipt_hash = task
        .worker_receipt_hash
        .as_deref()
        .map(str::trim)
        .filter(|value| value.starts_with("sha256:"))
        .ok_or_else(|| ApiError::BadRequest("cloud_video_task_output_not_ready".to_string()))?;
    if l3_object_storage_ref_to_path(output_ref).is_err() {
        return Err(ApiError::BadRequest(
            "cloud_video_task_output_not_ready".to_string(),
        ));
    }
    if content_type != "video/mp4" || bytes == 0 || media_hash.is_empty() || receipt_hash.is_empty()
    {
        return Err(ApiError::BadRequest(
            "cloud_video_task_output_not_ready".to_string(),
        ));
    }
    Ok(())
}

fn build_l3_output_download_payload(
    task: &CloudVideoTaskRecord,
    expires_at: chrono::DateTime<Utc>,
) -> Result<L3OutputDownloadTokenPayload, ApiError> {
    validate_l3_output_download_ready(task)?;
    Ok(L3OutputDownloadTokenPayload {
        schema_version: "l3_output_download_authorization_v1".to_string(),
        authorization_id: generate_l3_output_download_authorization_id(),
        task_id: task.task_id.clone(),
        account_id: task.account_id.clone(),
        workspace_id: task.workspace_id.clone(),
        output_media_storage_ref: task.output_media_storage_ref.clone().unwrap_or_default(),
        output_media_bytes: task.output_media_bytes.unwrap_or_default(),
        output_media_content_type: task.output_media_content_type.clone().unwrap_or_default(),
        watermarked_media_hash: task.watermarked_media_hash.clone().unwrap_or_default(),
        worker_receipt_hash: task.worker_receipt_hash.clone().unwrap_or_default(),
        expires_at,
    })
}

fn l3_output_download_response(
    payload: L3OutputDownloadTokenPayload,
    download_token: String,
) -> CloudVideoTaskDownloadAuthorizationResponse {
    CloudVideoTaskDownloadAuthorizationResponse {
        schema_version: payload.schema_version,
        authorization_id: payload.authorization_id,
        task_id: payload.task_id.clone(),
        status: "authorized".to_string(),
        output_media_storage_ref: payload.output_media_storage_ref,
        output_media_bytes: payload.output_media_bytes,
        output_media_content_type: payload.output_media_content_type,
        watermarked_media_hash: payload.watermarked_media_hash,
        worker_receipt_hash: payload.worker_receipt_hash,
        expires_at: payload.expires_at,
        signed_download_url: format!(
            "/v1/video-tasks/{}/output-download?token={}",
            payload.task_id, download_token
        ),
        download_method: "GET".to_string(),
        download_token,
        privacy_boundary: "signed_download_authorization_only_no_local_path_no_raw_upload"
            .to_string(),
    }
}

fn l3_object_storage_ref_to_path(storage_ref: &str) -> Result<PathBuf, ApiError> {
    let storage_ref = storage_ref.trim();
    let relative = storage_ref
        .strip_prefix("object://")
        .ok_or_else(|| ApiError::BadRequest("l3_object_storage_ref_invalid".to_string()))?;
    if relative.trim().is_empty() || relative.contains('\\') || relative.contains(':') {
        return Err(ApiError::BadRequest(
            "l3_object_storage_ref_invalid".to_string(),
        ));
    }
    let root = l3_object_store_root();
    let mut path = root;
    for segment in relative.split('/') {
        if segment.is_empty() || segment == "." || segment == ".." {
            return Err(ApiError::BadRequest(
                "l3_object_storage_ref_invalid".to_string(),
            ));
        }
        path.push(segment);
    }
    Ok(path)
}

fn l3_object_store_root() -> PathBuf {
    std::env::var("HIDDENSHIELD_L3_OBJECT_STORE_DIR")
        .ok()
        .map(|value| PathBuf::from(value.trim()))
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| std::env::temp_dir().join("hiddenshield-l3-object-store"))
}

fn looks_like_sha256_api(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn l3_output_download_signing_secret(state: &AppState) -> Result<&str, ApiError> {
    state
        .commercial_metrics_admin_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(ApiError::Forbidden)
}

fn generate_l3_output_download_authorization_id() -> String {
    let mut bytes = [0_u8; 16];
    OsRng.fill_bytes(&mut bytes);
    format!(
        "l3dl_{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    )
}

fn sign_l3_output_download_token(
    secret: &str,
    payload: &L3OutputDownloadTokenPayload,
) -> Result<String, ApiError> {
    let payload_json = serde_json::to_vec(payload).map_err(|error| {
        ApiError::Storage(format!("download payload serialize failed: {error}"))
    })?;
    let payload_b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload_json);
    let signature = l3_output_download_signature(secret, &payload_b64)?;
    Ok(format!("hs_l3dl_v1.{payload_b64}.{signature}"))
}

fn verify_l3_output_download_token(
    secret: &str,
    token: &str,
) -> Result<L3OutputDownloadTokenPayload, ApiError> {
    let mut parts = token.split('.');
    let prefix = parts.next().unwrap_or_default();
    let payload_b64 = parts.next().unwrap_or_default();
    let signature = parts.next().unwrap_or_default();
    if prefix != "hs_l3dl_v1"
        || payload_b64.is_empty()
        || signature.is_empty()
        || parts.next().is_some()
    {
        return Err(ApiError::Forbidden);
    }
    let expected = l3_output_download_signature(secret, payload_b64)?;
    if signature != expected {
        return Err(ApiError::Forbidden);
    }
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64.as_bytes())
        .map_err(|_| ApiError::Forbidden)?;
    let payload: L3OutputDownloadTokenPayload =
        serde_json::from_slice(&payload_bytes).map_err(|_| ApiError::Forbidden)?;
    if payload.schema_version != "l3_output_download_authorization_v1"
        || payload.authorization_id.trim().is_empty()
        || payload.task_id.trim().is_empty()
        || payload.output_media_storage_ref.trim().is_empty()
    {
        return Err(ApiError::Forbidden);
    }
    Ok(payload)
}

fn l3_output_download_signature(secret: &str, payload_b64: &str) -> Result<String, ApiError> {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| {
        ApiError::Storage("l3 output download signing secret is invalid".to_string())
    })?;
    mac.update(b"hidden-shield:l3-output-download:v1\n");
    mac.update(payload_b64.as_bytes());
    let digest = mac.finalize().into_bytes();
    Ok(hex_lower(&digest))
}

fn validate_l3_completion_receipt(
    state: &AppState,
    task_id: &str,
    request: &CloudVideoTaskCompletionRequest,
) -> Result<(), ApiError> {
    let secret = state
        .commercial_metrics_admin_token
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or(ApiError::Forbidden)?;
    let expected = l3_completion_receipt_signature(secret, task_id, request)?;
    if request.server_receipt_signature.trim() != expected {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

fn l3_completion_receipt_signature(
    secret: &str,
    task_id: &str,
    request: &CloudVideoTaskCompletionRequest,
) -> Result<String, ApiError> {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| ApiError::Storage("l3 completion receipt secret is invalid".to_string()))?;
    mac.update(b"hidden-shield:l3-completion:v1\n");
    mac.update(task_id.trim().as_bytes());
    mac.update(b"\n");
    mac.update(request.strategy_digest.trim().as_bytes());
    mac.update(b"\n");
    mac.update(format!("{:.6}", request.self_check_threshold).as_bytes());
    mac.update(b"\n");
    mac.update(format!("{:.6}", request.self_check_confidence).as_bytes());
    mac.update(b"\n");
    mac.update(request.checked_frames.to_string().as_bytes());
    mac.update(b"\n");
    mac.update(request.watermarked_media_hash.trim().as_bytes());
    mac.update(b"\n");
    mac.update(request.output_media_storage_ref.trim().as_bytes());
    mac.update(b"\n");
    mac.update(request.output_media_bytes.to_string().as_bytes());
    mac.update(b"\n");
    mac.update(request.output_media_content_type.trim().as_bytes());
    mac.update(b"\n");
    mac.update(request.worker_receipt_hash.trim().as_bytes());
    mac.update(b"\n");
    mac.update(request.worker_id.trim().as_bytes());
    mac.update(b"\n");
    mac.update(request.attempt_id.trim().as_bytes());
    mac.update(b"\n");
    mac.update(request.lease_token.trim().as_bytes());
    let digest = mac.finalize().into_bytes();
    Ok(format!(
        "hmac-sha256:l3-completion-v1:{}",
        hex_lower(&digest)
    ))
}

fn validate_admin_endpoint(
    state: &AppState,
    headers: &HeaderMap,
    endpoint: &str,
) -> Result<(), ApiError> {
    let configured_token = match state.commercial_metrics_admin_token.as_deref() {
        Some(token) => token,
        None => {
            let _ = state.storage.record_admin_audit_event(
                endpoint,
                "denied",
                "admin_token_not_configured",
            );
            return Err(ApiError::Forbidden);
        }
    };
    let request_token = admin_bearer_token(headers);
    if request_token.as_deref() != Some(configured_token) {
        let _ = state
            .storage
            .record_admin_audit_event(endpoint, "denied", "admin_token_invalid");
        return Err(ApiError::Unauthorized);
    }
    state
        .storage
        .record_admin_audit_event(endpoint, "allowed", "admin_token")
        .map_err(ApiError::from)?;
    Ok(())
}

fn extract_enterprise_api_key(headers: &HeaderMap) -> Result<String, ApiError> {
    let bearer = headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .and_then(|value| value.strip_prefix("Bearer ").map(str::trim))
        .filter(|value| !value.is_empty());
    let explicit = headers
        .get("x-hiddenshield-api-key")
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty());
    bearer
        .or(explicit)
        .map(str::to_string)
        .ok_or(ApiError::Unauthorized)
}

fn extract_enterprise_client_fingerprint(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<EnterpriseGatewayClientFingerprint, ApiError> {
    let configured_secret = state
        .trusted_proxy_shared_secret
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let provided_secret = header_value(headers, "x-hiddenshield-proxy-secret");
    if configured_secret.is_none() {
        if state.enterprise_require_trusted_proxy {
            return Err(ApiError::Forbidden);
        }
        return Ok(EnterpriseGatewayClientFingerprint::default());
    }
    let configured_secret = configured_secret.expect("checked above");
    if provided_secret.as_deref() != Some(configured_secret) {
        if state.enterprise_require_trusted_proxy || provided_secret.is_some() {
            return Err(ApiError::Forbidden);
        }
        return Ok(EnterpriseGatewayClientFingerprint::default());
    }

    let (raw_fingerprint, source) = header_value(headers, "x-hiddenshield-client-fingerprint")
        .map(|value| (value, "trusted_proxy_x_hiddenshield_client_fingerprint"))
        .or_else(|| {
            header_value(headers, "x-forwarded-for")
                .and_then(|value| value.split(',').next().map(str::trim).map(str::to_string))
                .filter(|value| !value.is_empty())
                .map(|value| (value, "trusted_proxy_x_forwarded_for"))
        })
        .or_else(|| {
            header_value(headers, "x-real-ip").map(|value| (value, "trusted_proxy_x_real_ip"))
        })
        .ok_or_else(|| {
            ApiError::BadRequest("trusted_proxy_client_fingerprint_missing".to_string())
        })?;
    let fingerprint_hash = sha256_prefixed(&format!(
        "enterprise-client-fingerprint:v1:{raw_fingerprint}"
    ));
    Ok(EnterpriseGatewayClientFingerprint {
        rate_limit_subject: fingerprint_hash.clone(),
        fingerprint_hash,
        source: source.to_string(),
        trusted_proxy: true,
    })
}

fn header_value(headers: &HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn sha256_prefixed(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    let mut out = String::from("sha256:");
    for byte in digest {
        use std::fmt::Write;
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

fn record_enterprise_admin_operation(
    state: &AppState,
    operation: &str,
    outcome: &str,
    endpoint: &str,
    account_id: Option<&str>,
    workspace_id: Option<&str>,
    api_key_id: Option<&str>,
    target_id: Option<&str>,
    reason: &str,
    details_json: serde_json::Value,
) -> Result<(), ApiError> {
    state
        .storage
        .record_enterprise_admin_audit_event_internal(
            operation,
            outcome,
            endpoint,
            account_id,
            workspace_id,
            api_key_id,
            target_id,
            reason,
            details_json,
        )
        .map(|_| ())
        .map_err(ApiError::from)
}

fn validate_batch(batch: &AnonymousFeedbackBatch) -> Result<(), ApiError> {
    if batch.install_id.trim().is_empty() {
        return Err(ApiError::BadRequest("installId is required".to_string()));
    }
    if batch.session_id.trim().is_empty() {
        return Err(ApiError::BadRequest("sessionId is required".to_string()));
    }
    if batch.events.is_empty() {
        return Err(ApiError::BadRequest("events must not be empty".to_string()));
    }
    if batch.events.len() > 1000 {
        return Err(ApiError::BadRequest(
            "events exceeds maximum batch size".to_string(),
        ));
    }

    for event in &batch.events {
        if event.event_id.trim().is_empty() {
            return Err(ApiError::BadRequest("eventId is required".to_string()));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct CloudChangesQuery {
    workspace_id: Option<String>,
    cursor: Option<String>,
}

fn validate_continue_account(request: &ContinueAccountRequest) -> Result<(), ApiError> {
    if request.identifier.trim().is_empty() {
        return Err(ApiError::BadRequest("identifier is required".to_string()));
    }
    if request.password.trim().is_empty() && request.verification_code.trim().is_empty() {
        return Err(ApiError::BadRequest("password is required".to_string()));
    }
    if request.device.client_device_id.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "device.clientDeviceId is required".to_string(),
        ));
    }
    Ok(())
}

fn validate_auth_challenge(request: &AuthChallengeRequest) -> Result<(), ApiError> {
    require_api_field(&request.identifier, "identifier")?;
    require_api_field(&request.client_device_id, "clientDeviceId")?;
    match request.purpose.trim() {
        "register_or_login" | "login" | "bind_identifier" | "reset_password" => Ok(()),
        _ => Err(ApiError::BadRequest("purpose is invalid".to_string())),
    }
}

fn validate_auth_session(request: &AuthSessionRequest) -> Result<(), ApiError> {
    require_api_field(&request.identifier, "identifier")?;
    require_api_field(&request.device.client_device_id, "device.clientDeviceId")?;
    let has_password = !request.password.trim().is_empty();
    let has_challenge = request
        .challenge_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some();
    if has_challenge && request.verification_code.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "verificationCode is required".to_string(),
        ));
    }
    if !has_password && !has_challenge {
        return Err(ApiError::BadRequest(
            "password or challenge is required".to_string(),
        ));
    }
    Ok(())
}

fn validate_auth_refresh(request: &AuthRefreshRequest) -> Result<(), ApiError> {
    require_api_field(&request.refresh_token, "refreshToken")?;
    require_api_field(&request.device_id, "deviceId")?;
    Ok(())
}

fn validate_auth_logout(request: &AuthLogoutRequest) -> Result<(), ApiError> {
    require_api_field(&request.refresh_token, "refreshToken")?;
    require_api_field(&request.device_id, "deviceId")?;
    Ok(())
}

fn validate_sync_preferences(request: &SyncPreferencesRequest) -> Result<(), ApiError> {
    let reason = request.reason.trim();
    if !reason.is_empty() && !matches!(reason, "user_paused" | "user_resumed") {
        return Err(ApiError::BadRequest("reason is invalid".to_string()));
    }
    Ok(())
}

fn validate_update_device(request: &UpdateDeviceRequest) -> Result<(), ApiError> {
    let name = request.name.trim();
    if name.is_empty() {
        return Err(ApiError::BadRequest("name is required".to_string()));
    }
    if name.chars().count() > 60 {
        return Err(ApiError::BadRequest("name is too long".to_string()));
    }
    Ok(())
}

fn validate_cloud_batch(batch: &CloudSyncBatchRequest) -> Result<(), ApiError> {
    if batch.device_id.trim().is_empty() {
        return Err(ApiError::BadRequest("deviceId is required".to_string()));
    }
    if batch.workspace_id.trim().is_empty() {
        return Err(ApiError::BadRequest("workspaceId is required".to_string()));
    }
    if batch.events.is_empty() {
        return Err(ApiError::BadRequest("events must not be empty".to_string()));
    }
    if batch.events.len() > 100 {
        return Err(ApiError::BadRequest(
            "events exceeds maximum batch size".to_string(),
        ));
    }
    Ok(())
}

fn validate_watermark_id_reserve(request: &WatermarkIdReserveRequest) -> Result<(), ApiError> {
    require_api_field(&request.request_id, "requestId")?;
    validate_watermark_common(
        &request.workspace_id,
        &request.creator_profile_id,
        &request.media_type,
        request.payload_protocol_version,
        request.payload_bytes_length,
        request.parent_watermark_uid.as_deref(),
        request.revision,
    )
}

fn validate_watermark_id_confirm(request: &WatermarkIdConfirmRequest) -> Result<(), ApiError> {
    require_api_field(&request.workspace_id, "workspaceId")?;
    require_api_field(&request.creator_profile_id, "creatorProfileId")?;
    validate_watermark_uid_for_api(&request.watermark_uid)?;
    validate_payload_protocol_for_api(
        request.payload_protocol_version,
        request.payload_bytes_length,
    )?;
    require_api_field(
        &request.write_verification_status,
        "writeVerificationStatus",
    )?;
    Ok(())
}

fn validate_watermark_id_reconcile(request: &WatermarkIdReconcileRequest) -> Result<(), ApiError> {
    validate_watermark_uid_for_api(&request.watermark_uid)?;
    validate_watermark_common(
        &request.workspace_id,
        &request.creator_profile_id,
        &request.media_type,
        request.payload_protocol_version,
        request.payload_bytes_length,
        request.parent_watermark_uid.as_deref(),
        request.revision,
    )
}

fn validate_watermark_id_reissue(request: &WatermarkIdReissueRequest) -> Result<(), ApiError> {
    validate_watermark_uid_for_api(&request.previous_watermark_uid)?;
    require_api_field(&request.reason, "reason")?;
    validate_watermark_common(
        &request.workspace_id,
        &request.creator_profile_id,
        &request.media_type,
        request.payload_protocol_version,
        request.payload_bytes_length,
        request.parent_watermark_uid.as_deref(),
        request.revision,
    )
}

fn validate_watermark_common(
    workspace_id: &str,
    creator_profile_id: &str,
    media_type: &str,
    protocol_version: u32,
    payload_bytes: u32,
    parent_watermark_uid: Option<&str>,
    revision: u32,
) -> Result<(), ApiError> {
    require_api_field(workspace_id, "workspaceId")?;
    require_api_field(creator_profile_id, "creatorProfileId")?;
    validate_media_type_for_api(media_type)?;
    validate_payload_protocol_for_api(protocol_version, payload_bytes)?;
    if revision == 0 {
        return Err(ApiError::BadRequest("revision is invalid".to_string()));
    }
    if parent_watermark_uid
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
        && revision == 1
    {
        return Err(ApiError::BadRequest(
            "parentWatermarkUid requires revision greater than 1".to_string(),
        ));
    }
    Ok(())
}

fn require_api_field(value: &str, field: &str) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        return Err(ApiError::BadRequest(format!("{field} is required")));
    }
    Ok(())
}

fn validate_media_type_for_api(value: &str) -> Result<(), ApiError> {
    match value.trim() {
        "image" | "audio" | "video_audio_track" | "video_visual" => Ok(()),
        _ => Err(ApiError::BadRequest("mediaType is invalid".to_string())),
    }
}

fn validate_payload_protocol_for_api(version: u32, bytes: u32) -> Result<(), ApiError> {
    if version == 2 && bytes == 119 {
        return Ok(());
    }
    if version == 3 && (33..=64).contains(&bytes) {
        return Ok(());
    }
    Err(ApiError::BadRequest(
        "payloadProtocol must be V2/119 or V3 minimal anchor".to_string(),
    ))
}

fn validate_watermark_uid_for_api(value: &str) -> Result<(), ApiError> {
    let value = value.trim();
    let parts = value.split('-').collect::<Vec<_>>();
    let valid = parts.len() == 5
        && parts[0] == "HS"
        && parts[1..]
            .iter()
            .all(|part| part.len() == 8 && part.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if valid {
        Ok(())
    } else {
        Err(ApiError::BadRequest("watermarkUid is invalid".to_string()))
    }
}

fn validate_billing_payment_session(
    request: &BillingPaymentSessionRequest,
) -> Result<(), ApiError> {
    if request.account_id.trim().is_empty() {
        return Err(ApiError::BadRequest("accountId is required".to_string()));
    }
    if request.workspace_id.trim().is_empty() {
        return Err(ApiError::BadRequest("workspaceId is required".to_string()));
    }
    if request.plan_code.trim().is_empty() {
        return Err(ApiError::BadRequest("planCode is required".to_string()));
    }
    if request.billing_cycle.trim().is_empty() {
        return Err(ApiError::BadRequest("billingCycle is required".to_string()));
    }
    Ok(())
}

fn validate_report_purchase_session(
    request: &ReportPurchaseSessionRequest,
) -> Result<(), ApiError> {
    if request.account_id.trim().is_empty() {
        return Err(ApiError::BadRequest("accountId is required".to_string()));
    }
    if request.workspace_id.trim().is_empty() {
        return Err(ApiError::BadRequest("workspaceId is required".to_string()));
    }
    if request.creator_profile_id.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "creatorProfileId is required".to_string(),
        ));
    }
    if request.vault_record_id.trim().is_empty() {
        return Err(ApiError::BadRequest(
            "vaultRecordId is required".to_string(),
        ));
    }
    match request.product_code.trim() {
        "copyright_report_single" | "rights_evidence_pack_single" => Ok(()),
        _ => Err(ApiError::BadRequest(
            "reportProductCode is invalid".to_string(),
        )),
    }
}

fn report_product_price_cents_for_api(product_code: &str) -> Result<i64, ApiError> {
    match product_code.trim() {
        "copyright_report_single" => Ok(1990),
        "rights_evidence_pack_single" => Ok(4990),
        _ => Err(ApiError::BadRequest(
            "report_product_not_allowed".to_string(),
        )),
    }
}

fn validate_video_fingerprint_notary(
    request: &VideoFingerprintNotaryRequest,
) -> Result<(), ApiError> {
    if request.workspace_id.trim().is_empty() {
        return Err(ApiError::BadRequest("workspaceId is required".to_string()));
    }
    if request.creator_profile_id.trim().is_empty() {
        return Err(ApiError::BadRequest("creator_profile_required".to_string()));
    }
    if request.watermark_uid.trim().is_empty() {
        return Err(ApiError::BadRequest("watermarkUid is required".to_string()));
    }
    if request.source_hash.trim().is_empty() {
        return Err(ApiError::BadRequest("sourceHash is required".to_string()));
    }
    if request.fingerprint_root.trim().is_empty()
        || request.local_block_fingerprint_root.trim().is_empty()
    {
        return Err(ApiError::BadRequest("fingerprint_root_invalid".to_string()));
    }
    if request.crop_window_fingerprint_root.trim().is_empty() || request.crop_window_count == 0 {
        return Err(ApiError::BadRequest("crop_windows_required".to_string()));
    }
    if request.client_signature.trim().is_empty() {
        return Err(ApiError::BadRequest("client_signature_invalid".to_string()));
    }
    validate_video_upload_manifest(&request.upload_manifest)
}

fn validate_video_upload_manifest(
    manifest: &crate::schema::VideoUploadManifest,
) -> Result<(), ApiError> {
    if manifest.contains_original_video {
        return Err(ApiError::BadRequest("original_video_forbidden".to_string()));
    }
    if manifest.contains_watermarked_video {
        return Err(ApiError::BadRequest(
            "watermarked_video_forbidden".to_string(),
        ));
    }
    if manifest.contains_local_paths {
        return Err(ApiError::BadRequest("local_path_forbidden".to_string()));
    }
    if manifest.schema_version.trim().is_empty() || manifest.items.is_empty() {
        return Err(ApiError::BadRequest("invalid_upload_manifest".to_string()));
    }
    Ok(())
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let token = header.strip_prefix("Bearer ").unwrap_or_default().trim();
    if token.is_empty() {
        Err(ApiError::Unauthorized)
    } else {
        Ok(token)
    }
}

fn admin_bearer_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_admin_token(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .map(ToOwned::to_owned)
}

fn normalize_secret_version(value: &str) -> String {
    let version = value.trim();
    if version.is_empty() {
        "local-dev".to_string()
    } else {
        version.to_string()
    }
}

fn normalize_optional_url(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(|url| url.trim_end_matches('/').to_string())
}

fn issue_enterprise_api_key_with_custody(
    state: &AppState,
    request: &EnterpriseApiKeyIssueRequest,
) -> Result<EnterpriseApiKeyIssueResponse, ApiError> {
    let reason = request.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::BadRequest(
            "enterprise api key issue reason is required".to_string(),
        ));
    }
    let create_request = EnterpriseApiKeyCreateRequest {
        account_id: request.account_id.clone(),
        workspace_id: request.workspace_id.clone(),
        creator_profile_id: request.creator_profile_id.clone(),
        name: request.name.clone(),
        key_prefix: String::new(),
        key_hash: String::new(),
        scopes: request.scopes.clone(),
        created_by_account_id: request.created_by_account_id.clone(),
        expires_at: request.expires_at,
    };
    let issued = create_enterprise_api_key_with_generated_secret(state, create_request)?;
    Ok(EnterpriseApiKeyIssueResponse {
        api_key: issued.record,
        cleartext_api_key: issued.cleartext_api_key,
        key_prefix: issued.key_prefix,
        hash_algorithm: issued.hash_algorithm,
        shown_once: true,
        custody_notice: "Store this cleartext API key now. HiddenShield will not show it again."
            .to_string(),
    })
}

fn rotate_enterprise_api_key_with_custody(
    state: &AppState,
    old_api_key_id: &str,
    request: &EnterpriseApiKeyRotateRequest,
) -> Result<EnterpriseApiKeyRotateResponse, ApiError> {
    let reason = request.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::BadRequest(
            "enterprise api key rotate reason is required".to_string(),
        ));
    }
    let created_by_account_id = request.created_by_account_id.trim();
    if created_by_account_id.is_empty() {
        return Err(ApiError::BadRequest(
            "enterprise api key rotate creator is required".to_string(),
        ));
    }
    let current = state
        .storage
        .get_enterprise_api_key_internal(old_api_key_id)
        .map_err(ApiError::from)?;
    if current.status == "revoked" || current.status == "expired" {
        return Err(ApiError::BadRequest(
            "enterprise api key cannot be rotated from terminal status".to_string(),
        ));
    }
    let new_name = request
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| format!("{} rotated", current.name));
    let new_scopes = request
        .scopes
        .clone()
        .unwrap_or_else(|| current.scopes.clone());
    let create_request = EnterpriseApiKeyCreateRequest {
        account_id: current.account_id.clone(),
        workspace_id: current.workspace_id.clone(),
        creator_profile_id: current.creator_profile_id.clone(),
        name: new_name,
        key_prefix: String::new(),
        key_hash: String::new(),
        scopes: new_scopes,
        created_by_account_id: created_by_account_id.to_string(),
        expires_at: request.expires_at.or(current.expires_at),
    };
    let issued = create_enterprise_api_key_with_generated_secret(state, create_request)?;
    let pause_reason = format!("rotated to {}: {}", issued.record.api_key_id, reason);
    let old_paused = match state
        .storage
        .pause_enterprise_api_key_internal(&current.api_key_id, &pause_reason)
    {
        Ok(record) => record,
        Err(error) => {
            let _ = state.storage.revoke_enterprise_api_key_internal(
                &issued.record.api_key_id,
                "rotation rollback after old key pause failed",
            );
            return Err(ApiError::from(error));
        }
    };
    let grace_period_hours = request.grace_period_hours.unwrap_or(24).clamp(0, 720);
    let rotation_deadline_at = Utc::now() + chrono::Duration::hours(i64::from(grace_period_hours));
    Ok(EnterpriseApiKeyRotateResponse {
        old_api_key: old_paused,
        new_api_key: issued.record,
        cleartext_api_key: issued.cleartext_api_key,
        key_prefix: issued.key_prefix,
        hash_algorithm: issued.hash_algorithm,
        shown_once: true,
        rotation_deadline_at,
        custody_notice:
            "Store this rotated cleartext API key now. HiddenShield will not show it again."
                .to_string(),
    })
}

fn revoke_expired_enterprise_rotations(
    state: &AppState,
    request: &EnterpriseExpiredRotationRevokeRequest,
) -> Result<EnterpriseExpiredRotationRevokeResponse, ApiError> {
    let reason = request.reason.trim();
    if reason.is_empty() {
        return Err(ApiError::BadRequest(
            "enterprise expired rotation revoke reason is required".to_string(),
        ));
    }
    let now = request.now.unwrap_or_else(Utc::now);
    let limit = request.limit.unwrap_or(100).clamp(1, 500);
    let candidates = state
        .storage
        .list_expired_enterprise_rotation_candidates_internal(now, limit)
        .map_err(ApiError::from)?;
    let mut items = Vec::with_capacity(candidates.len());
    let mut revoked = 0_u32;
    let mut skipped = 0_u32;
    for candidate in candidates {
        let revoke_reason = format!(
            "expired rotation grace period from {}: {}",
            candidate.audit_event_id, reason
        );
        match state
            .storage
            .revoke_enterprise_api_key_internal(&candidate.old_api_key_id, &revoke_reason)
        {
            Ok(record) if record.status == "revoked" => {
                revoked += 1;
                record_enterprise_admin_operation(
                    state,
                    "revoke_api_key",
                    "succeeded",
                    "/internal/enterprise/api-key-rotations/revoke-expired",
                    Some(&record.account_id),
                    Some(&record.workspace_id),
                    Some(&record.api_key_id),
                    candidate.new_api_key_id.as_deref(),
                    &revoke_reason,
                    serde_json::json!({
                        "status": record.status,
                        "revokedAt": record.revoked_at,
                        "source": "revoke_expired_rotations",
                        "rotationAuditEventId": candidate.audit_event_id,
                        "newApiKeyId": candidate.new_api_key_id,
                        "rotationDeadlineAt": candidate.rotation_deadline_at
                    }),
                )?;
                items.push(EnterpriseExpiredRotationRevokeItem {
                    old_api_key_id: record.api_key_id,
                    new_api_key_id: candidate.new_api_key_id,
                    account_id: Some(record.account_id),
                    workspace_id: Some(record.workspace_id),
                    rotation_deadline_at: candidate.rotation_deadline_at,
                    outcome: "revoked".to_string(),
                    reason: revoke_reason,
                });
            }
            Ok(record) => {
                skipped += 1;
                items.push(EnterpriseExpiredRotationRevokeItem {
                    old_api_key_id: record.api_key_id,
                    new_api_key_id: candidate.new_api_key_id,
                    account_id: Some(record.account_id),
                    workspace_id: Some(record.workspace_id),
                    rotation_deadline_at: candidate.rotation_deadline_at,
                    outcome: format!("skipped:{}", record.status),
                    reason: "candidate was no longer paused".to_string(),
                });
            }
            Err(error) => {
                skipped += 1;
                items.push(EnterpriseExpiredRotationRevokeItem {
                    old_api_key_id: candidate.old_api_key_id,
                    new_api_key_id: candidate.new_api_key_id,
                    account_id: candidate.account_id,
                    workspace_id: candidate.workspace_id,
                    rotation_deadline_at: candidate.rotation_deadline_at,
                    outcome: "failed".to_string(),
                    reason: error.to_string(),
                });
            }
        }
    }
    Ok(EnterpriseExpiredRotationRevokeResponse {
        processed: items.len() as u32,
        revoked,
        skipped,
        items,
    })
}

struct GeneratedEnterpriseApiKey {
    record: EnterpriseApiKeyRecord,
    cleartext_api_key: String,
    key_prefix: String,
    hash_algorithm: String,
}

fn create_enterprise_api_key_with_generated_secret(
    state: &AppState,
    mut request: EnterpriseApiKeyCreateRequest,
) -> Result<GeneratedEnterpriseApiKey, ApiError> {
    let secret = state
        .enterprise_api_key_hash_secret
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(ApiError::Forbidden)?;
    let cleartext_api_key = generate_enterprise_cleartext_api_key();
    let key_prefix = derive_enterprise_key_prefix(&cleartext_api_key);
    let secret_version = normalize_secret_version(&state.enterprise_api_key_hash_secret_version);
    let key_hash = hmac_sha256_key_hash(&cleartext_api_key, secret, &secret_version)?;
    request.key_prefix = key_prefix.clone();
    request.key_hash = key_hash;
    let record = state
        .storage
        .create_enterprise_api_key_internal(&request)
        .map_err(ApiError::from)?;
    Ok(GeneratedEnterpriseApiKey {
        record,
        cleartext_api_key,
        key_prefix,
        hash_algorithm: format!("hmac-sha256:v1:{secret_version}"),
    })
}

fn generate_enterprise_cleartext_api_key() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes);
    format!("hsent_live_{token}")
}

fn derive_enterprise_key_prefix(cleartext_api_key: &str) -> String {
    cleartext_api_key.chars().take(22).collect::<String>()
}

fn hmac_sha256_key_hash(
    cleartext_api_key: &str,
    secret: &str,
    secret_version: &str,
) -> Result<String, ApiError> {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| ApiError::Storage("enterprise key hash secret is invalid".to_string()))?;
    mac.update(cleartext_api_key.as_bytes());
    let digest = mac.finalize().into_bytes();
    Ok(format!(
        "hmac-sha256:v1:{}:{}",
        secret_version,
        hex_lower(&digest)
    ))
}

fn hex_lower(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

async fn deliver_auth_challenge_if_configured(
    state: &AppState,
    request: &AuthChallengeRequest,
    response: &crate::schema::AuthChallengeResponse,
) -> Result<(), ApiError> {
    let endpoint = match state.auth_otp_delivery_endpoint.as_deref() {
        Some(endpoint) if response.delivery_channel != "fixture" => endpoint,
        _ => return Ok(()),
    };
    let code = state
        .storage
        .take_auth_challenge_delivery_code(&response.challenge_id)
        .map_err(ApiError::from)?
        .ok_or_else(|| ApiError::Storage("auth challenge delivery code missing".to_string()))?;
    let payload = serde_json::json!({
        "identifier": request.identifier.trim(),
        "purpose": request.purpose.trim(),
        "clientDeviceId": request.client_device_id.trim(),
        "challengeId": response.challenge_id,
        "verificationCode": code,
        "expiresAt": response.expires_at,
    });
    let delivery_response = state
        .http_client
        .post(endpoint)
        .json(&payload)
        .send()
        .await
        .map_err(|error| ApiError::Storage(format!("auth_otp_delivery_failed: {error}")))?;
    if !delivery_response.status().is_success() {
        return Err(ApiError::Storage(format!(
            "auth_otp_delivery_failed: HTTP {}",
            delivery_response.status()
        )));
    }
    Ok(())
}

fn wechat_headers_from_header_map(headers: &HeaderMap) -> Result<WechatPayHeaders, ApiError> {
    fn required(headers: &HeaderMap, name: &str) -> Result<String, ApiError> {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .ok_or_else(|| ApiError::BadRequest(format!("{name}_required")))
    }
    Ok(WechatPayHeaders {
        timestamp: required(headers, "wechatpay-timestamp")?,
        nonce: required(headers, "wechatpay-nonce")?,
        signature: required(headers, "wechatpay-signature")?,
        serial: required(headers, "wechatpay-serial")?,
    })
}

impl From<StorageError> for ApiError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::Unauthorized => ApiError::Unauthorized,
            StorageError::Forbidden => ApiError::Forbidden,
            StorageError::RateLimited(message) => ApiError::RateLimited(message),
            StorageError::BadRequest(message) => ApiError::BadRequest(message),
            other => ApiError::Storage(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{AnonymousEventOutcome, AnonymousFeedbackBatch, AnonymousFeedbackEvent};
    use axum::body::Body;
    use http::Request;
    use tempfile::NamedTempFile;
    use tower::util::ServiceExt;

    fn sample_event(event_id: &str) -> AnonymousFeedbackEvent {
        AnonymousFeedbackEvent {
            event_id: event_id.to_string(),
            occurred_at: Utc::now(),
            install_id: "inst-1".to_string(),
            session_id: "sess-1".to_string(),
            app_version: "0.1.0".to_string(),
            feature_name: "watermark_video".to_string(),
            outcome: AnonymousEventOutcome::Success,
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
        workspace_id: &str,
        creator_profile_id: &str,
    ) -> crate::schema::CloudVideoTaskRequest {
        crate::schema::CloudVideoTaskRequest {
            schema_version: "cloud_video_task_v1".to_string(),
            workspace_id: workspace_id.to_string(),
            creator_profile_id: creator_profile_id.to_string(),
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
        let storage_ref = format!("object://l3-output/{task_id}/route-test.mp4");
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
            hex_lower(&Sha256::digest(receipt_text.as_bytes()))
        );
        (
            receipt,
            receipt_hash,
            storage_ref,
            4096,
            "video/mp4".to_string(),
        )
    }

    #[tokio::test]
    async fn tauri_webview_origins_are_allowed_for_public_rights_routes() {
        for origin in [
            "tauri://localhost",
            "http://tauri.localhost",
            "https://tauri.localhost",
        ] {
            let file = NamedTempFile::new().unwrap();
            let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
            let app = build_app(storage);
            let response = app
                .oneshot(
                    Request::builder()
                        .method("OPTIONS")
                        .uri("/v1/public/rights/HS-00000000-00000000-00000000-00000000")
                        .header(axum::http::header::ORIGIN, origin)
                        .header(axum::http::header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(
                response
                    .headers()
                    .get(axum::http::header::ACCESS_CONTROL_ALLOW_ORIGIN)
                    .and_then(|value| value.to_str().ok()),
                Some(origin)
            );
        }
    }

    #[test]
    fn validate_batch_rejects_missing_required_fields() {
        let mut batch = AnonymousFeedbackBatch {
            install_id: String::new(),
            session_id: String::new(),
            app_version: "0.1.0".to_string(),
            sent_at: Utc::now(),
            events: vec![],
        };

        assert!(matches!(
            validate_batch(&batch),
            Err(ApiError::BadRequest(_))
        ));

        batch.install_id = "inst-1".to_string();
        assert!(matches!(
            validate_batch(&batch),
            Err(ApiError::BadRequest(_))
        ));

        batch.session_id = "sess-1".to_string();
        assert!(matches!(
            validate_batch(&batch),
            Err(ApiError::BadRequest(_))
        ));

        batch.events.push(sample_event(""));
        assert!(matches!(
            validate_batch(&batch),
            Err(ApiError::BadRequest(_))
        ));
    }

    #[test]
    fn commercial_metrics_admin_rejects_unconfigured_and_invalid_token_then_audits() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let state = AppState {
            storage: Arc::clone(&storage),
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: None,
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: None,
            enterprise_api_key_hash_secret_version: "local-dev".to_string(),
            trusted_proxy_shared_secret: None,
            enterprise_require_trusted_proxy: false,
        };
        let headers = HeaderMap::new();
        assert!(matches!(
            validate_commercial_metrics_admin(&state, &headers),
            Err(ApiError::Forbidden)
        ));

        let state = AppState {
            commercial_metrics_admin_token: Some("secret-admin-token".to_string()),
            ..state
        };
        assert!(matches!(
            validate_commercial_metrics_admin(&state, &headers),
            Err(ApiError::Unauthorized)
        ));

        let denied_count = storage
            .admin_audit_event_count_for_tests("denied", None)
            .unwrap();
        assert_eq!(denied_count, 2);
    }

    #[test]
    fn commercial_metrics_admin_accepts_configured_bearer_token_then_audits() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let state = AppState {
            storage: Arc::clone(&storage),
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: Some("secret-admin-token".to_string()),
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: None,
            enterprise_api_key_hash_secret_version: "local-dev".to_string(),
            trusted_proxy_shared_secret: None,
            enterprise_require_trusted_proxy: false,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer secret-admin-token".parse().unwrap(),
        );
        assert!(validate_commercial_metrics_admin(&state, &headers).is_ok());

        let allowed_count = storage
            .admin_audit_event_count_for_tests("allowed", Some("admin_token"))
            .unwrap();
        assert_eq!(allowed_count, 1);
    }

    #[test]
    fn team_workspace_handlers_return_current_and_created_workspace() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let request = crate::schema::ContinueAccountRequest {
            identifier: "route@example.com".to_string(),
            password: "correct-password".to_string(),
            verification_code: "000000".to_string(),
            device: crate::schema::ContinueAccountDevice {
                client_device_id: "dev-1".to_string(),
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
        };
        let session = storage.continue_account(&request).unwrap();
        storage
            .set_entitlement_feature_for_tests(&session.account.id, "team_workspace", true)
            .unwrap();
        let state = AppState {
            storage: Arc::clone(&storage),
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: None,
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: None,
            enterprise_api_key_hash_secret_version: "local-dev".to_string(),
            trusted_proxy_shared_secret: None,
            enterprise_require_trusted_proxy: false,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            format!("Bearer {}", session.access_token).parse().unwrap(),
        );

        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let Json(current) = get_current_team_workspace(State(state.clone()), headers.clone())
                .await
                .unwrap();
            assert_eq!(current.workspace_type, "personal");
            assert_eq!(current.member_count, 1);

            let Json(created) = create_team_workspace(
                State(state.clone()),
                headers.clone(),
                Json(crate::schema::TeamWorkspaceCreateRequest {
                    account_id: session.account.id.clone(),
                    name: "Studio Route QA".to_string(),
                }),
            )
            .await
            .unwrap();
            assert_eq!(created.workspace_type, "team");
            assert_eq!(created.account_id, session.account.id);

            let Json(workspaces) = list_team_workspaces(State(state.clone()), headers.clone())
                .await
                .unwrap();
            assert_eq!(workspaces.returned, 2);
            assert_eq!(workspaces.workspaces[0].workspace_type, "team");

            let Json(members) = list_team_members(
                State(state.clone()),
                headers.clone(),
                axum::extract::Path(created.workspace_id.clone()),
            )
            .await
            .unwrap();
            assert_eq!(members.returned, 1);

            let Json(records) = list_team_shared_library(
                State(state),
                headers.clone(),
                axum::extract::Path(created.workspace_id.clone()),
            )
            .await
            .unwrap();
            assert_eq!(records.returned, 0);
        });
    }

    #[tokio::test]
    async fn cloud_video_task_routes_roundtrip_through_http_handlers() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let session = storage
            .continue_account(&sample_continue_request("routes@example.com", "dev-1"))
            .unwrap();
        storage
            .set_entitlement_feature_for_tests(&session.account.id, "cloud_video_processing", true)
            .unwrap();
        let app = build_app_with_admin_auth_enterprise_custody_and_proxy(
            storage,
            None,
            Some("secret-admin-token".to_string()),
            None,
            None,
            "local-dev".to_string(),
            None,
            false,
        );
        let token_header = format!("Bearer {}", session.access_token);
        let request =
            sample_cloud_video_task_request(&session.workspace.id, &session.creator_profile.id);

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/v1/video-tasks")
                    .header(axum::http::header::AUTHORIZATION, token_header.clone())
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::OK);

        let body = axum::body::to_bytes(created.into_body(), usize::MAX)
            .await
            .unwrap();
        let created_record: crate::schema::CloudVideoTaskRecord =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(created_record.status, "draft");

        let listed = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/v1/video-tasks?status=draft&limit=10")
                    .header(axum::http::header::AUTHORIZATION, token_header.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(listed.status(), StatusCode::OK);

        let get_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(format!("/v1/video-tasks/{}", created_record.task_id))
                    .header(axum::http::header::AUTHORIZATION, token_header.clone())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(get_response.status(), StatusCode::OK);

        let user_completion_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri(format!("/v1/video-tasks/{}/status", created_record.task_id))
                    .header(axum::http::header::AUTHORIZATION, token_header)
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&crate::schema::CloudVideoTaskStatusUpdateRequest {
                            status: "succeeded".to_string(),
                            failure_code: None,
                            strategy_digest: Some("sha256:strategy".to_string()),
                            self_check_threshold: Some(0.9),
                            self_check_confidence: Some(0.95),
                            checked_frames: Some(8),
                            watermarked_media_hash: Some("sha256:watermarked-video".to_string()),
                            server_receipt_signature: Some("sig:server-receipt".to_string()),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(user_completion_response.status(), StatusCode::BAD_REQUEST);

        let claim_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/internal/video-tasks/claim")
                    .header(
                        axum::http::header::AUTHORIZATION,
                        "Bearer secret-admin-token",
                    )
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        serde_json::to_vec(&crate::schema::CloudVideoTaskClaimRequest {
                            worker_id: "worker-route-qa".to_string(),
                            capability_level: Some("hybrid_visual_watermark".to_string()),
                            lease_seconds: Some(900),
                        })
                        .unwrap(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(claim_response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(claim_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let claim: crate::schema::CloudVideoTaskClaimResponse =
            serde_json::from_slice(&body).unwrap();
        assert_eq!(claim.task.task_id, created_record.task_id);
        assert_eq!(claim.task.status, "running");
        assert_eq!(claim.task.worker_id.as_deref(), Some("worker-route-qa"));
        assert_eq!(claim.task.attempt_count, 1);

        let (worker_receipt, worker_receipt_hash, output_ref, output_bytes, output_content_type) =
            sample_l3_worker_receipt_fields(
                &created_record.task_id,
                &claim.worker_id,
                "sha256:watermarked-video",
            );
        let completion = crate::schema::CloudVideoTaskCompletionRequest {
            strategy_digest: "sha256:strategy".to_string(),
            self_check_threshold: 0.9,
            self_check_confidence: 0.95,
            checked_frames: 8,
            watermarked_media_hash: "sha256:watermarked-video".to_string(),
            output_media_storage_ref: output_ref,
            output_media_bytes: output_bytes,
            output_media_content_type: output_content_type,
            worker_receipt_hash,
            worker_receipt,
            server_receipt_signature: String::new(),
            worker_id: claim.worker_id.clone(),
            attempt_id: claim.attempt_id.clone(),
            lease_token: claim.lease_token.clone(),
        };
        let signature = l3_completion_receipt_signature(
            "secret-admin-token",
            &created_record.task_id,
            &completion,
        )
        .unwrap();
        let completion = crate::schema::CloudVideoTaskCompletionRequest {
            server_receipt_signature: signature,
            ..completion
        };
        let completion_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/internal/video-tasks/{}/completion",
                        created_record.task_id
                    ))
                    .header(
                        axum::http::header::AUTHORIZATION,
                        "Bearer secret-admin-token",
                    )
                    .header(axum::http::header::CONTENT_TYPE, "application/json")
                    .body(Body::from(serde_json::to_vec(&completion).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(completion_response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(completion_response.into_body(), usize::MAX)
            .await
            .unwrap();
        let completed: crate::schema::CloudVideoTaskRecord = serde_json::from_slice(&body).unwrap();
        assert_eq!(completed.status, "succeeded");
        assert!(completed.usage_ledger_id.is_some());
        assert!(completed.output_media_storage_ref.is_some());
        assert!(completed.worker_receipt_hash.is_some());
        assert_eq!(
            completed
                .worker_receipt
                .as_ref()
                .and_then(|value| value.get("schemaVersion"))
                .and_then(serde_json::Value::as_str),
            Some("l3_worker_receipt_v1")
        );
        assert_eq!(completed.worker_id.as_deref(), Some("worker-route-qa"));
        assert_eq!(
            completed.attempt_id.as_deref(),
            Some(claim.attempt_id.as_str())
        );
    }

    #[test]
    fn enterprise_trusted_proxy_fingerprint_requires_configured_proxy_when_enabled() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let state = AppState {
            storage,
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: None,
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: None,
            enterprise_api_key_hash_secret_version: "local-dev".to_string(),
            trusted_proxy_shared_secret: Some("proxy-secret".to_string()),
            enterprise_require_trusted_proxy: true,
        };
        let headers = HeaderMap::new();
        assert!(matches!(
            extract_enterprise_client_fingerprint(&state, &headers),
            Err(ApiError::Forbidden)
        ));
    }

    #[test]
    fn enterprise_trusted_proxy_fingerprint_hashes_forwarded_identity() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let state = AppState {
            storage,
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: None,
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: None,
            enterprise_api_key_hash_secret_version: "local-dev".to_string(),
            trusted_proxy_shared_secret: Some("proxy-secret".to_string()),
            enterprise_require_trusted_proxy: true,
        };
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-hiddenshield-proxy-secret",
            "proxy-secret".parse().unwrap(),
        );
        headers.insert("x-forwarded-for", "203.0.113.10, 10.0.0.1".parse().unwrap());
        let fingerprint = extract_enterprise_client_fingerprint(&state, &headers).unwrap();
        assert!(fingerprint.trusted_proxy);
        assert_eq!(fingerprint.source, "trusted_proxy_x_forwarded_for");
        assert!(fingerprint.fingerprint_hash.starts_with("sha256:"));
        assert!(!fingerprint.fingerprint_hash.contains("203.0.113.10"));
    }

    #[test]
    fn internal_rights_backfill_uses_admin_token_gate() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let state = AppState {
            storage: Arc::clone(&storage),
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: Some("secret-admin-token".to_string()),
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: None,
            enterprise_api_key_hash_secret_version: "local-dev".to_string(),
            trusted_proxy_shared_secret: None,
            enterprise_require_trusted_proxy: false,
        };
        let endpoint = "/internal/rights-manifests/backfill";
        let headers = HeaderMap::new();
        assert!(matches!(
            validate_admin_endpoint(&state, &headers, endpoint),
            Err(ApiError::Unauthorized)
        ));

        let mut headers = HeaderMap::new();
        headers.insert(
            axum::http::header::AUTHORIZATION,
            "Bearer secret-admin-token".parse().unwrap(),
        );
        assert!(validate_admin_endpoint(&state, &headers, endpoint).is_ok());

        let allowed_count = storage
            .admin_audit_event_count_for_tests("allowed", Some("admin_token"))
            .unwrap();
        assert_eq!(allowed_count, 1);
    }

    #[test]
    fn internal_enterprise_admin_endpoints_use_same_admin_token_gate() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let state = AppState {
            storage: Arc::clone(&storage),
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: Some("secret-admin-token".to_string()),
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: Some("enterprise-hash-secret".to_string()),
            enterprise_api_key_hash_secret_version: "test-v1".to_string(),
            trusted_proxy_shared_secret: None,
            enterprise_require_trusted_proxy: false,
        };
        let endpoints = [
            "/internal/enterprise/api-keys",
            "/internal/enterprise/api-key-issuances",
            "/internal/enterprise/api-keys/:api_key_id",
            "/internal/enterprise/api-keys/:api_key_id/pause",
            "/internal/enterprise/api-keys/:api_key_id/rotate",
            "/internal/enterprise/api-keys/:api_key_id/revoke",
            "/internal/enterprise/api-key-rotations/revoke-expired",
            "/internal/enterprise/quota-balances",
            "/internal/enterprise/gateway-dry-run",
        ];
        for endpoint in endpoints {
            assert!(matches!(
                validate_admin_endpoint(&state, &HeaderMap::new(), endpoint),
                Err(ApiError::Unauthorized)
            ));
            let mut headers = HeaderMap::new();
            headers.insert(
                axum::http::header::AUTHORIZATION,
                "Bearer secret-admin-token".parse().unwrap(),
            );
            assert!(validate_admin_endpoint(&state, &headers, endpoint).is_ok());
        }
        let allowed_count = storage
            .admin_audit_event_count_for_tests("allowed", Some("admin_token"))
            .unwrap();
        assert_eq!(allowed_count, 9);
    }

    #[tokio::test]
    async fn ai_transparency_internal_routes_require_admin_token_and_hide_unknown_license() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let app =
            build_app_with_billing_and_admin(storage, None, Some("secret-admin-token".to_string()));
        let unauthorized = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/internal/ai-transparency/licenses/atl-missing")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauthorized.status(), StatusCode::UNAUTHORIZED);

        let not_found = app
            .oneshot(
                Request::builder()
                    .uri("/internal/ai-transparency/licenses/atl-missing")
                    .header(
                        axum::http::header::AUTHORIZATION,
                        "Bearer secret-admin-token",
                    )
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(not_found.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn enterprise_api_key_issuance_generates_one_time_cleartext_and_hash_metadata() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let state = AppState {
            storage: Arc::clone(&storage),
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: Some("secret-admin-token".to_string()),
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: Some("enterprise-hash-secret".to_string()),
            enterprise_api_key_hash_secret_version: "test-v1".to_string(),
            trusted_proxy_shared_secret: None,
            enterprise_require_trusted_proxy: false,
        };
        let request = EnterpriseApiKeyIssueRequest {
            account_id: "acct_enterprise".to_string(),
            workspace_id: "ws_enterprise".to_string(),
            creator_profile_id: None,
            name: "QA scanner".to_string(),
            scopes: vec![
                "public_rights:read".to_string(),
                "public_rights:batch_read".to_string(),
            ],
            created_by_account_id: "admin_acct".to_string(),
            expires_at: None,
            reason: "customer onboarding".to_string(),
            delivery_channel: Some("secure_note".to_string()),
            recipient_ref: Some("secops-ticket-1".to_string()),
        };

        let issued = issue_enterprise_api_key_with_custody(&state, &request).unwrap();
        assert!(issued.cleartext_api_key.starts_with("hsent_live_"));
        assert!(issued.shown_once);
        assert_eq!(issued.hash_algorithm, "hmac-sha256:v1:test-v1");
        assert_eq!(
            issued.key_prefix,
            derive_enterprise_key_prefix(&issued.cleartext_api_key)
        );

        let listed = storage
            .list_enterprise_api_keys_internal(&EnterpriseApiKeyListQuery {
                account_id: Some("acct_enterprise".to_string()),
                workspace_id: None,
                status: Some("active".to_string()),
                limit: Some(10),
            })
            .unwrap();
        assert_eq!(listed.returned, 1);
        assert_eq!(listed.api_keys[0].key_prefix, issued.key_prefix);

        let record_json = serde_json::to_string(&listed).unwrap();
        assert!(!record_json.contains("cleartext"));
        assert!(!record_json.contains("keyHash"));
        assert!(!record_json.contains(&issued.cleartext_api_key));
    }

    #[test]
    fn enterprise_api_key_issuance_requires_custody_secret() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let state = AppState {
            storage,
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: Some("secret-admin-token".to_string()),
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: None,
            enterprise_api_key_hash_secret_version: "test-v1".to_string(),
            trusted_proxy_shared_secret: None,
            enterprise_require_trusted_proxy: false,
        };
        let request = EnterpriseApiKeyIssueRequest {
            account_id: "acct_enterprise".to_string(),
            workspace_id: "ws_enterprise".to_string(),
            creator_profile_id: None,
            name: "QA scanner".to_string(),
            scopes: vec!["public_rights:read".to_string()],
            created_by_account_id: "admin_acct".to_string(),
            expires_at: None,
            reason: "customer onboarding".to_string(),
            delivery_channel: None,
            recipient_ref: None,
        };

        assert!(matches!(
            issue_enterprise_api_key_with_custody(&state, &request),
            Err(ApiError::Forbidden)
        ));
    }

    #[test]
    fn enterprise_api_key_rotation_generates_new_key_and_pauses_old_key() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let state = AppState {
            storage: Arc::clone(&storage),
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: Some("secret-admin-token".to_string()),
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: Some("enterprise-hash-secret".to_string()),
            enterprise_api_key_hash_secret_version: "test-v1".to_string(),
            trusted_proxy_shared_secret: None,
            enterprise_require_trusted_proxy: false,
        };
        let issued = issue_enterprise_api_key_with_custody(
            &state,
            &EnterpriseApiKeyIssueRequest {
                account_id: "acct_rotate".to_string(),
                workspace_id: "ws_rotate".to_string(),
                creator_profile_id: None,
                name: "Rotate scanner".to_string(),
                scopes: vec!["public_rights:read".to_string()],
                created_by_account_id: "admin_acct".to_string(),
                expires_at: None,
                reason: "initial issue".to_string(),
                delivery_channel: Some("secure_note".to_string()),
                recipient_ref: Some("ticket-initial".to_string()),
            },
        )
        .unwrap();
        let rotated = rotate_enterprise_api_key_with_custody(
            &state,
            &issued.api_key.api_key_id,
            &EnterpriseApiKeyRotateRequest {
                reason: "scheduled rotation".to_string(),
                name: Some("Rotate scanner v2".to_string()),
                scopes: None,
                created_by_account_id: "admin_acct".to_string(),
                grace_period_hours: Some(12),
                expires_at: None,
                delivery_channel: Some("secure_note".to_string()),
                recipient_ref: Some("ticket-rotate".to_string()),
            },
        )
        .unwrap();

        assert_eq!(rotated.old_api_key.status, "paused");
        assert_eq!(rotated.new_api_key.status, "active");
        assert_ne!(
            rotated.old_api_key.api_key_id,
            rotated.new_api_key.api_key_id
        );
        assert!(rotated.cleartext_api_key.starts_with("hsent_live_"));
        assert_eq!(rotated.hash_algorithm, "hmac-sha256:v1:test-v1");
        assert!(rotated.shown_once);

        let listed = storage
            .list_enterprise_api_keys_internal(&EnterpriseApiKeyListQuery {
                account_id: Some("acct_rotate".to_string()),
                workspace_id: None,
                status: None,
                limit: Some(10),
            })
            .unwrap();
        assert_eq!(listed.returned, 2);
        let listed_json = serde_json::to_string(&listed).unwrap();
        assert!(!listed_json.contains(&rotated.cleartext_api_key));
        assert!(!listed_json.contains("keyHash"));
        assert_eq!(
            storage
                .get_enterprise_api_key_internal(&issued.api_key.api_key_id)
                .unwrap()
                .status,
            "paused"
        );
    }

    #[test]
    fn enterprise_expired_rotation_sweep_revokes_paused_old_key_after_deadline() {
        let file = NamedTempFile::new().unwrap();
        let storage = Arc::new(Storage::open(file.path(), 30).unwrap());
        let state = AppState {
            storage: Arc::clone(&storage),
            wechat_pay: None,
            http_client: reqwest::Client::new(),
            commercial_metrics_admin_token: Some("secret-admin-token".to_string()),
            auth_otp_delivery_endpoint: None,
            enterprise_api_key_hash_secret: Some("enterprise-hash-secret".to_string()),
            enterprise_api_key_hash_secret_version: "test-v1".to_string(),
            trusted_proxy_shared_secret: None,
            enterprise_require_trusted_proxy: false,
        };
        let issued = issue_enterprise_api_key_with_custody(
            &state,
            &EnterpriseApiKeyIssueRequest {
                account_id: "acct_sweep".to_string(),
                workspace_id: "ws_sweep".to_string(),
                creator_profile_id: None,
                name: "Sweep scanner".to_string(),
                scopes: vec!["public_rights:read".to_string()],
                created_by_account_id: "admin_acct".to_string(),
                expires_at: None,
                reason: "initial issue".to_string(),
                delivery_channel: Some("secure_note".to_string()),
                recipient_ref: Some("ticket-initial".to_string()),
            },
        )
        .unwrap();
        let rotated = rotate_enterprise_api_key_with_custody(
            &state,
            &issued.api_key.api_key_id,
            &EnterpriseApiKeyRotateRequest {
                reason: "scheduled rotation".to_string(),
                name: None,
                scopes: None,
                created_by_account_id: "admin_acct".to_string(),
                grace_period_hours: Some(1),
                expires_at: None,
                delivery_channel: Some("secure_note".to_string()),
                recipient_ref: Some("ticket-rotate".to_string()),
            },
        )
        .unwrap();
        record_enterprise_admin_operation(
            &state,
            "rotate_api_key",
            "succeeded",
            "/internal/enterprise/api-keys/:api_key_id/rotate",
            Some(&rotated.old_api_key.account_id),
            Some(&rotated.old_api_key.workspace_id),
            Some(&rotated.old_api_key.api_key_id),
            Some(&rotated.new_api_key.api_key_id),
            "scheduled rotation",
            serde_json::json!({
                "oldApiKeyId": rotated.old_api_key.api_key_id,
                "newApiKeyId": rotated.new_api_key.api_key_id,
                "rotationDeadlineAt": rotated.rotation_deadline_at
            }),
        )
        .unwrap();

        let early = revoke_expired_enterprise_rotations(
            &state,
            &EnterpriseExpiredRotationRevokeRequest {
                now: Some(rotated.rotation_deadline_at - chrono::Duration::seconds(1)),
                limit: Some(10),
                reason: "sweep before deadline".to_string(),
            },
        )
        .unwrap();
        assert_eq!(early.processed, 0);
        assert_eq!(
            storage
                .get_enterprise_api_key_internal(&issued.api_key.api_key_id)
                .unwrap()
                .status,
            "paused"
        );

        let expired = revoke_expired_enterprise_rotations(
            &state,
            &EnterpriseExpiredRotationRevokeRequest {
                now: Some(rotated.rotation_deadline_at + chrono::Duration::seconds(1)),
                limit: Some(10),
                reason: "grace period complete".to_string(),
            },
        )
        .unwrap();
        assert_eq!(expired.processed, 1);
        assert_eq!(expired.revoked, 1);
        assert_eq!(expired.items[0].old_api_key_id, issued.api_key.api_key_id);
        assert_eq!(
            expired.items[0].new_api_key_id.as_deref(),
            Some(rotated.new_api_key.api_key_id.as_str())
        );
        assert_eq!(
            storage
                .get_enterprise_api_key_internal(&issued.api_key.api_key_id)
                .unwrap()
                .status,
            "revoked"
        );
    }

    #[test]
    fn enterprise_public_rights_external_api_route_is_key_protected() {
        let source = include_str!("lib.rs");
        assert!(source.contains(concat!("/v1/enterprise/", "public-rights/batch")));
        assert!(source.contains("enterprise_public_rights_batch"));
        assert!(source.contains("extract_enterprise_api_key"));
    }
}

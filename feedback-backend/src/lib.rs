pub mod schema;
pub mod storage;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use clap::Parser;
use serde::Serialize;
use tracing_subscriber::EnvFilter;

use crate::schema::{
    AnonymousFeedbackBatch, AnonymousFeedbackBatchAck, AnonymousFeedbackStatsQuery,
    AnonymousFeedbackStatsResponse, CloudSyncBatchRequest, CloudSyncChangesResult,
    ContinueAccountRequest,
};
use crate::storage::{Storage, StorageError};

#[derive(Debug, Parser, Clone)]
pub struct ServerArgs {
    #[arg(long, env = "HIDDENSHIELD_FEEDBACK_BIND_ADDR", default_value = "127.0.0.1:8787")]
    pub bind_addr: SocketAddr,

    #[arg(long, env = "HIDDENSHIELD_FEEDBACK_DB_PATH", default_value = "feedback.sqlite")]
    pub db_path: PathBuf,

    #[arg(long, env = "HIDDENSHIELD_FEEDBACK_RETENTION_DAYS", default_value_t = 180)]
    pub retention_days: i64,
}

#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<Storage>,
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
    #[error("storage error")]
    Storage(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error, message) = match self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request".to_string(), message),
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized".to_string(), "unauthorized".to_string()),
            ApiError::Storage(message) => (StatusCode::INTERNAL_SERVER_ERROR, "storage_error".to_string(), message),
        };
        (status, Json(ApiErrorResponse { error, message })).into_response()
    }
}

pub fn build_app(storage: Arc<Storage>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/health", get(healthz))
        .route("/v1/auth/continue", post(continue_account))
        .route("/v1/sync/events:batch", post(push_cloud_events_batch))
        .route("/v1/sync/changes", get(get_cloud_changes))
        .route("/v1/anonymous-feedback/batches", post(ingest_batch))
        .route("/v1/anonymous-feedback/stats", get(get_stats))
        .with_state(AppState { storage })
}

pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args = ServerArgs::parse();

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(filter).init();

    let storage = Arc::new(Storage::open(&args.db_path, args.retention_days)?);
    let app = build_app(storage);
    let listener = tokio::net::TcpListener::bind(args.bind_addr).await?;

    tracing::info!("HiddenShield feedback backend listening on {}", args.bind_addr);
    axum::serve(listener, app.into_make_service()).await?;
    Ok(())
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

async fn continue_account(
    State(state): State<AppState>,
    Json(request): Json<ContinueAccountRequest>,
) -> Result<Json<crate::schema::CloudAccountSession>, ApiError> {
    validate_continue_account(&request)?;
    Ok(Json(state.storage.continue_account(&request).map_err(ApiError::from)?))
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

async fn get_cloud_changes(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<CloudChangesQuery>,
) -> Result<Json<CloudSyncChangesResult>, ApiError> {
    let token = bearer_token(&headers)?;
    Ok(Json(state.storage.get_cloud_changes(token, query.cursor.as_deref()).map_err(ApiError::from)?))
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
        return Err(ApiError::BadRequest("events exceeds maximum batch size".to_string()));
    }

    for event in &batch.events {
        if event.event_id.trim().is_empty() {
            return Err(ApiError::BadRequest("eventId is required".to_string()));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, serde::Deserialize)]
struct CloudChangesQuery {
    cursor: Option<String>,
}

fn validate_continue_account(request: &ContinueAccountRequest) -> Result<(), ApiError> {
    if request.identifier.trim().is_empty() {
        return Err(ApiError::BadRequest("identifier is required".to_string()));
    }
    if request.device.client_device_id.trim().is_empty() {
        return Err(ApiError::BadRequest("device.clientDeviceId is required".to_string()));
    }
    Ok(())
}

fn validate_cloud_batch(batch: &CloudSyncBatchRequest) -> Result<(), ApiError> {
    if batch.device_id.trim().is_empty() {
        return Err(ApiError::BadRequest("deviceId is required".to_string()));
    }
    if batch.events.is_empty() {
        return Err(ApiError::BadRequest("events must not be empty".to_string()));
    }
    if batch.events.len() > 100 {
        return Err(ApiError::BadRequest("events exceeds maximum batch size".to_string()));
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

impl From<StorageError> for ApiError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::Unauthorized => ApiError::Unauthorized,
            StorageError::BadRequest(message) => ApiError::BadRequest(message),
            other => ApiError::Storage(other.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{AnonymousEventOutcome, AnonymousFeedbackBatch, AnonymousFeedbackEvent};

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

    #[test]
    fn validate_batch_rejects_missing_required_fields() {
        let mut batch = AnonymousFeedbackBatch {
            install_id: String::new(),
            session_id: String::new(),
            app_version: "0.1.0".to_string(),
            sent_at: Utc::now(),
            events: vec![],
        };

        assert!(matches!(validate_batch(&batch), Err(ApiError::BadRequest(_))));

        batch.install_id = "inst-1".to_string();
        assert!(matches!(validate_batch(&batch), Err(ApiError::BadRequest(_))));

        batch.session_id = "sess-1".to_string();
        assert!(matches!(validate_batch(&batch), Err(ApiError::BadRequest(_))));

        batch.events.push(sample_event(""));
        assert!(matches!(validate_batch(&batch), Err(ApiError::BadRequest(_))));
    }
}

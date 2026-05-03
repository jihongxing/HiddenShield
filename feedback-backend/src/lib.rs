pub mod schema;
pub mod storage;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
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
    AnonymousFeedbackStatsResponse,
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
    status: &'static str,
    version: &'static str,
    timestamp: chrono::DateTime<chrono::Utc>,
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
    #[error("storage error")]
    Storage(#[from] StorageError),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, error, message) = match self {
            ApiError::BadRequest(message) => (StatusCode::BAD_REQUEST, "bad_request".to_string(), message),
            ApiError::Storage(err) => (StatusCode::INTERNAL_SERVER_ERROR, "storage_error".to_string(), err.to_string()),
        };
        (status, Json(ApiErrorResponse { error, message })).into_response()
    }
}

pub fn build_app(storage: Arc<Storage>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
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
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        timestamp: Utc::now(),
    })
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

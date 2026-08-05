use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    routing::{get, patch, post},
    Json, Router,
};
use hmac::{Hmac, Mac};
use serde::Deserialize;
use sha2::Sha256;

use crate::{
    postgres_auth::PostgresAuthRepository,
    postgres_registry::PostgresWatermarkRegistryRepository,
    postgres_sync::PostgresCloudSyncRepository,
    repository::{AuthRepository, CloudSyncRepository, WatermarkRegistryRepository},
    schema::{
        AuthChallengeRequest, AuthLogoutRequest, AuthRefreshRequest, AuthSessionRequest,
        CloudSyncBatchRequest, ContinueAccountRequest, SyncPreferencesRequest, UpdateDeviceRequest,
        WatermarkIdConfirmRequest, WatermarkIdReconcileRequest, WatermarkIdReissueRequest,
        WatermarkIdReserveRequest,
    },
    ApiError, HealthResponse,
};

#[derive(Clone)]
pub struct PostgresHttpState {
    auth: Arc<PostgresAuthRepository>,
    sync: Arc<PostgresCloudSyncRepository>,
    registry: Arc<PostgresWatermarkRegistryRepository>,
    qa_entitlement_grant_enabled: bool,
    qa_internal_token: Option<String>,
}

impl PostgresHttpState {
    pub fn connect(
        database_url: String,
        max_connections: u32,
        qa_entitlement_grant_enabled: bool,
        qa_internal_token: Option<String>,
    ) -> Result<Self, crate::storage::StorageError> {
        Ok(Self {
            auth: Arc::new(PostgresAuthRepository::connect(
                &database_url,
                max_connections,
            )?),
            sync: Arc::new(PostgresCloudSyncRepository::connect(
                &database_url,
                max_connections,
            )?),
            registry: Arc::new(PostgresWatermarkRegistryRepository::connect(
                &database_url,
                max_connections,
            )?),
            qa_entitlement_grant_enabled,
            qa_internal_token,
        })
    }
}

pub fn build_postgres_app(state: PostgresHttpState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/health", get(healthz))
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
        .route("/v1/sync/events:batch", post(push_cloud_events_batch))
        .route("/v1/sync/changes", get(get_cloud_changes))
        .route("/v1/watermark-ids/reserve", post(reserve_watermark_id))
        .route("/v1/watermark-ids/confirm", post(confirm_watermark_id))
        .route("/v1/watermark-ids/reconcile", post(reconcile_watermark_id))
        .route("/v1/watermark-ids/reissue", post(reissue_watermark_id))
        .route(
            "/internal/qa/entitlements/cloud-sync",
            post(grant_cloud_sync_for_qa),
        )
        .with_state(state)
}

async fn healthz() -> Json<HealthResponse> {
    Json(HealthResponse::postgres())
}

async fn continue_account(
    State(state): State<PostgresHttpState>,
    Json(request): Json<ContinueAccountRequest>,
) -> Result<Json<crate::schema::CloudAccountSession>, ApiError> {
    call_auth(state.auth, move |repo| repo.continue_account(&request))
        .await
        .map(Json)
}

async fn create_auth_challenge(
    State(state): State<PostgresHttpState>,
    Json(request): Json<AuthChallengeRequest>,
) -> Result<Json<crate::schema::AuthChallengeResponse>, ApiError> {
    super::validate_auth_challenge(&request)?;
    call_auth(state.auth, move |repo| repo.create_auth_challenge(&request))
        .await
        .map(Json)
}

async fn create_auth_session(
    State(state): State<PostgresHttpState>,
    Json(request): Json<AuthSessionRequest>,
) -> Result<Json<crate::schema::CloudAccountSession>, ApiError> {
    super::validate_auth_session(&request)?;
    call_auth(state.auth, move |repo| repo.create_auth_session(&request))
        .await
        .map(Json)
}

async fn refresh_auth_session(
    State(state): State<PostgresHttpState>,
    Json(request): Json<AuthRefreshRequest>,
) -> Result<Json<crate::schema::CloudAccountSession>, ApiError> {
    super::validate_auth_refresh(&request)?;
    call_auth(state.auth, move |repo| repo.refresh_auth_session(&request))
        .await
        .map(Json)
}

async fn logout_auth_session(
    State(state): State<PostgresHttpState>,
    Json(request): Json<AuthLogoutRequest>,
) -> Result<Json<crate::schema::AuthLogoutResponse>, ApiError> {
    super::validate_auth_logout(&request)?;
    call_auth(state.auth, move |repo| repo.logout_auth_session(&request))
        .await
        .map(Json)
}

async fn get_me(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
) -> Result<Json<crate::schema::CloudAccountSnapshot>, ApiError> {
    let token = bearer_token_owned(&headers)?;
    call_auth(state.auth, move |repo| {
        repo.current_account_snapshot(&token)
    })
    .await
    .map(Json)
}

async fn update_sync_preferences(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
    Json(request): Json<SyncPreferencesRequest>,
) -> Result<Json<crate::schema::SyncPreferencesResponse>, ApiError> {
    super::validate_sync_preferences(&request)?;
    let token = bearer_token_owned(&headers)?;
    call_auth(state.auth, move |repo| {
        repo.update_sync_preferences(&token, &request)
    })
    .await
    .map(Json)
}

async fn list_devices(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
) -> Result<Json<crate::schema::AccountDevicesResponse>, ApiError> {
    let token = bearer_token_owned(&headers)?;
    call_auth(state.auth, move |repo| repo.list_devices(&token))
        .await
        .map(Json)
}

async fn update_device(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
    Path(device_id): Path<String>,
    Json(request): Json<UpdateDeviceRequest>,
) -> Result<Json<crate::schema::AccountDevice>, ApiError> {
    super::validate_update_device(&request)?;
    let token = bearer_token_owned(&headers)?;
    call_auth(state.auth, move |repo| {
        repo.update_device(&token, &device_id, &request)
    })
    .await
    .map(Json)
}

async fn revoke_device(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
    Path(device_id): Path<String>,
) -> Result<Json<crate::schema::RevokeDeviceResponse>, ApiError> {
    let token = bearer_token_owned(&headers)?;
    call_auth(state.auth, move |repo| {
        repo.revoke_device(&token, &device_id)
    })
    .await
    .map(Json)
}

async fn push_cloud_events_batch(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
    Json(batch): Json<CloudSyncBatchRequest>,
) -> Result<Json<crate::schema::CloudSyncBatchResult>, ApiError> {
    super::validate_cloud_batch(&batch)?;
    let token = bearer_token_owned(&headers)?;
    call_sync(state.sync, move |repo| {
        repo.push_cloud_events_batch(&token, &batch)
    })
    .await
    .map(Json)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SyncChangesQuery {
    workspace_id: Option<String>,
    cursor: Option<String>,
}

async fn get_cloud_changes(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
    Query(query): Query<SyncChangesQuery>,
) -> Result<Json<crate::schema::CloudSyncChangesResult>, ApiError> {
    let token = bearer_token_owned(&headers)?;
    call_sync(state.sync, move |repo| {
        repo.get_cloud_changes(
            &token,
            query.workspace_id.as_deref(),
            query.cursor.as_deref(),
        )
    })
    .await
    .map(Json)
}

async fn reserve_watermark_id(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
    Json(request): Json<WatermarkIdReserveRequest>,
) -> Result<Json<crate::schema::WatermarkIdRegistryResponse>, ApiError> {
    super::validate_watermark_id_reserve(&request)?;
    let token = bearer_token_owned(&headers)?;
    call_registry(state.registry, move |repo| {
        repo.reserve_watermark_id(&token, &request)
    })
    .await
    .map(Json)
}

async fn confirm_watermark_id(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
    Json(request): Json<WatermarkIdConfirmRequest>,
) -> Result<Json<crate::schema::WatermarkIdRegistryResponse>, ApiError> {
    super::validate_watermark_id_confirm(&request)?;
    let token = bearer_token_owned(&headers)?;
    call_registry(state.registry, move |repo| {
        repo.confirm_watermark_id(&token, &request)
    })
    .await
    .map(Json)
}

async fn reconcile_watermark_id(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
    Json(request): Json<WatermarkIdReconcileRequest>,
) -> Result<Json<crate::schema::WatermarkIdRegistryResponse>, ApiError> {
    super::validate_watermark_id_reconcile(&request)?;
    let token = bearer_token_owned(&headers)?;
    call_registry(state.registry, move |repo| {
        repo.reconcile_watermark_id(&token, &request)
    })
    .await
    .map(Json)
}

async fn reissue_watermark_id(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
    Json(request): Json<WatermarkIdReissueRequest>,
) -> Result<Json<crate::schema::WatermarkIdReissueResponse>, ApiError> {
    super::validate_watermark_id_reissue(&request)?;
    let token = bearer_token_owned(&headers)?;
    call_registry(state.registry, move |repo| {
        repo.reissue_watermark_id(&token, &request)
    })
    .await
    .map(Json)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct QaEntitlementGrantRequest {
    account_id: String,
    workspace_id: String,
}

async fn grant_cloud_sync_for_qa(
    State(state): State<PostgresHttpState>,
    headers: HeaderMap,
    Json(request): Json<QaEntitlementGrantRequest>,
) -> Result<Json<crate::schema::CloudEntitlement>, ApiError> {
    if !state.qa_entitlement_grant_enabled {
        return Err(ApiError::NotFound(
            "qa_entitlement_grant_not_enabled".to_string(),
        ));
    }
    let token = headers
        .get("x-hiddenshield-internal-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let expected = state
        .qa_internal_token
        .as_deref()
        .ok_or(ApiError::Forbidden)?;
    if !constant_time_token_matches(token, expected) {
        return Err(ApiError::Forbidden);
    }
    call_auth(state.auth, move |repo| {
        repo.grant_cloud_sync_for_qa(&request.account_id, &request.workspace_id)
    })
    .await
    .map(Json)
}

async fn call_auth<T, F>(
    repository: Arc<PostgresAuthRepository>,
    operation: F,
) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce(&PostgresAuthRepository) -> Result<T, crate::storage::StorageError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || operation(&repository))
        .await
        .map_err(|error| ApiError::Storage(format!("postgres_auth_task_failed:{error}")))?
        .map_err(ApiError::from)
}

async fn call_sync<T, F>(
    repository: Arc<PostgresCloudSyncRepository>,
    operation: F,
) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce(&PostgresCloudSyncRepository) -> Result<T, crate::storage::StorageError>
        + Send
        + 'static,
{
    tokio::task::spawn_blocking(move || operation(&repository))
        .await
        .map_err(|error| ApiError::Storage(format!("postgres_sync_task_failed:{error}")))?
        .map_err(ApiError::from)
}

async fn call_registry<T, F>(
    repository: Arc<PostgresWatermarkRegistryRepository>,
    operation: F,
) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce(&PostgresWatermarkRegistryRepository) -> Result<T, crate::storage::StorageError>
        + Send
        + 'static,
{
    tokio::task::spawn_blocking(move || operation(&repository))
        .await
        .map_err(|error| ApiError::Storage(format!("postgres_registry_task_failed:{error}")))?
        .map_err(ApiError::from)
}

fn bearer_token_owned(headers: &HeaderMap) -> Result<String, ApiError> {
    super::bearer_token(headers).map(ToOwned::to_owned)
}

fn constant_time_token_matches(actual: &str, expected: &str) -> bool {
    type TokenMac = Hmac<Sha256>;
    let key = [0_u8; 32];
    let mut expected_mac = TokenMac::new_from_slice(&key).expect("fixed HMAC key is valid");
    expected_mac.update(expected.as_bytes());
    let expected_tag = expected_mac.finalize().into_bytes();
    let mut actual_mac = TokenMac::new_from_slice(&key).expect("fixed HMAC key is valid");
    actual_mac.update(actual.as_bytes());
    actual_mac.verify_slice(&expected_tag).is_ok()
}

#[cfg(test)]
mod tests {
    use super::constant_time_token_matches;

    #[test]
    fn qa_internal_token_requires_exact_match() {
        assert!(constant_time_token_matches(
            "local-http-gate-internal-token",
            "local-http-gate-internal-token"
        ));
        assert!(!constant_time_token_matches(
            "local-http-gate-internal-token-wrong",
            "local-http-gate-internal-token"
        ));
        assert!(!constant_time_token_matches(
            "",
            "local-http-gate-internal-token"
        ));
    }
}

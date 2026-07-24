use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;
use tauri::{AppHandle, Manager, State};

use crate::config;
use crate::db::billing::{self, EntitlementState, EntitlementStatus};
use crate::db::queries;
use crate::identity;
use crate::pipeline::ffmpeg;
use crate::sync::cloud::{
    clear_desktop_cloud_sync_profile, load_desktop_cloud_sync_profile,
    save_desktop_cloud_sync_profile, vault_record_to_cloud_event,
    video_fingerprint_bundle_to_notary_request, AccountDevice, AuthChallengeRequest,
    AuthChallengeResponse, BillingPaymentSessionReconcileResponse, BillingPaymentSessionRequest,
    BillingPaymentSessionResponse, BillingPaymentSessionStatusResponse, CloudPullResult,
    CloudQueueFlushResult, CloudQueueStatus, CloudSyncBatchResult, CloudSyncChangesResult,
    CloudSyncClient, CloudSyncEvent, CloudVideoTaskObjectUploadAuthorizationRequest,
    CloudVideoTaskRecord, CloudVideoTaskRequest, ContinueAccountCreatorProfile,
    ContinueAccountDevice, ContinueAccountRequest, DesktopCloudSyncProfile, ReportPurchaseGrant,
    ReportPurchaseSessionReconcileResponse, ReportPurchaseSessionRequest,
    ReportPurchaseSessionResponse, ReportPurchaseSessionStatusResponse, RevokeDeviceResponse,
    VideoFingerprintBundleForNotary, VideoFingerprintNotaryReceipt, VideoFingerprintNotaryRequest,
    VideoUploadManifest, VideoUploadManifestItem, WatermarkIdReserveRequest,
};
use crate::sync::storage::{self, MobileSyncQueueItem};
use crate::video_fingerprint::VideoFingerprintBundleGeneration;
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MobileSyncStatus {
    pub enabled: bool,
    pub listen_port: u16,
    pub listen_address: String,
    pub pairing_code: String,
    pub received_events: u64,
    pub latest_event_at: Option<String>,
    pub resolution_count: u64,
    pub latest_resolution: Option<storage::SyncResolutionSummary>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueCloudAccountInput {
    pub identifier: String,
    pub password: Option<String>,
    pub challenge_id: Option<String>,
    pub verification_code: Option<String>,
    pub creator_display_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDesktopAuthChallengeInput {
    pub identifier: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushDesktopVaultRecordInput {
    pub base_url: String,
    pub access_token: String,
    pub device_id: String,
    pub workspace_id: String,
    pub record_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchCloudChangesInput {
    pub base_url: String,
    pub access_token: String,
    pub workspace_id: String,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushSavedDesktopVaultRecordInput {
    pub record_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlushCloudSyncQueueInput {
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetDesktopCloudAutoSyncInput {
    pub enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDesktopCloudDeviceInput {
    pub device_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeDesktopCloudDeviceInput {
    pub device_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVideoFingerprintNotaryInput {
    pub request: Option<VideoFingerprintNotaryRequest>,
    pub bundle: Option<VideoFingerprintBundleForNotary>,
    pub bundle_sha256: Option<String>,
    pub bundle_bytes: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateVideoFingerprintNotaryFromBundleFileInput {
    pub bundle_path: String,
    pub title: Option<String>,
    pub bundle_elapsed_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateVideoFingerprintBundleInput {
    pub input_path: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveL3VideoVisualTaskInput {
    pub task_id: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateL3VideoVisualUploadTaskInput {
    pub input_path: String,
    pub title: Option<String>,
    pub duration_secs: Option<f64>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub frame_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateL3VideoVisualUploadTaskResult {
    pub task: CloudVideoTaskRecord,
    pub watermark_uid: String,
    pub source_sha256: String,
    pub uploaded_bytes: u64,
    pub privacy_boundary: String,
    pub next_action: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveL3VideoVisualTaskResult {
    pub task: CloudVideoTaskRecord,
    pub vault_record: crate::commands::vault::VaultRecord,
    pub output_path: String,
    pub output_sha256: String,
    pub cloud_sync: Option<CloudQueueFlushResult>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateBillingPaymentSessionInput {
    pub plan_code: String,
    pub billing_cycle: Option<String>,
    pub preferred_provider: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingPaymentSessionIdInput {
    pub payment_session_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReportPurchaseSessionInput {
    pub vault_record_id: u32,
    pub product_code: String,
    pub preferred_provider: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPurchaseSessionIdInput {
    pub payment_session_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFingerprintNotaryResult {
    pub receipt: VideoFingerprintNotaryReceipt,
    pub vault_record: crate::commands::vault::VaultRecord,
}

#[tauri::command]
pub fn get_mobile_sync_status(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<MobileSyncStatus, String> {
    let listen_port = config::load_system_config().lan_debug_port;
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let pairing_code = storage::get_or_create_pairing_code(&app_data_dir)?;

    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    let received_events =
        storage::count_sync_events(&conn).map_err(|e| format!("sync count failed: {e}"))?;
    let latest_event_at = storage::latest_sync_event_at(&conn)
        .map_err(|e| format!("sync latest event query failed: {e}"))?;
    let resolution_count = storage::count_sync_resolutions(&conn)
        .map_err(|e| format!("sync resolution count failed: {e}"))?;
    let latest_resolution = storage::latest_sync_resolution(&conn)
        .map_err(|e| format!("sync latest resolution query failed: {e}"))?;

    Ok(MobileSyncStatus {
        enabled: true,
        listen_port,
        listen_address: format!("http://0.0.0.0:{listen_port}"),
        pairing_code,
        received_events,
        latest_event_at,
        resolution_count,
        latest_resolution,
    })
}

#[tauri::command]
pub fn regenerate_mobile_pairing_code(app_handle: AppHandle) -> Result<String, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let code = storage::new_pairing_code();
    storage::save_pairing_code(&app_data_dir, &code)?;
    Ok(code)
}

#[tauri::command]
pub fn get_desktop_cloud_sync_profile(
    app_handle: AppHandle,
) -> Result<Option<DesktopCloudSyncProfile>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    Ok(load_desktop_cloud_sync_profile(&app_data_dir))
}

#[tauri::command]
pub fn get_desktop_cloud_queue_status(
    state: State<'_, AppState>,
) -> Result<CloudQueueStatus, String> {
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    let stale_recovered = storage::recover_stale_cloud_syncing_queue(
        &conn,
        chrono::Utc::now() - chrono::Duration::minutes(10),
    )
    .map_err(|e| format!("恢复中断云同步队列失败: {e}"))?;
    Ok(CloudQueueStatus {
        pending: storage::count_cloud_sync_queue_by_status(&conn, "pending")
            .map_err(|e| format!("读取云同步队列失败: {e}"))?,
        syncing: storage::count_cloud_sync_queue_by_status(&conn, "syncing")
            .map_err(|e| format!("读取云同步队列失败: {e}"))?,
        failed: storage::count_cloud_sync_queue_by_status(&conn, "failed")
            .map_err(|e| format!("读取云同步队列失败: {e}"))?,
        blocked: storage::count_cloud_sync_queue_by_status(&conn, "blocked")
            .map_err(|e| format!("读取云同步队列失败: {e}"))?,
        synced: storage::count_cloud_sync_queue_by_status(&conn, "synced")
            .map_err(|e| format!("读取云同步队列失败: {e}"))?,
        retry_exhausted: storage::count_cloud_sync_queue_retry_exhausted(&conn)
            .map_err(|e| format!("读取云同步重试上限失败: {e}"))?,
        stale_recovered,
        last_attempt_at: storage::latest_cloud_sync_queue_update_by_status(
            &conn,
            &["syncing", "synced", "failed", "blocked"],
        )
        .map_err(|e| format!("读取云同步最近尝试时间失败: {e}"))?,
        last_success_at: storage::latest_cloud_sync_queue_update_by_status(&conn, &["synced"])
            .map_err(|e| format!("读取云同步最近成功时间失败: {e}"))?,
        last_failure_at: storage::latest_cloud_sync_queue_update_by_status(
            &conn,
            &["failed", "blocked"],
        )
        .map_err(|e| format!("读取云同步最近失败时间失败: {e}"))?,
        next_retry_at: storage::earliest_cloud_sync_queue_retry_at(&conn)
            .map_err(|e| format!("读取云同步下次重试时间失败: {e}"))?,
        last_error: storage::latest_cloud_sync_queue_error(&conn)
            .map_err(|e| format!("读取云同步最近错误失败: {e}"))?,
        last_error_code: storage::latest_cloud_sync_queue_error_code(&conn)
            .map_err(|e| format!("读取云同步最近错误码失败: {e}"))?,
        last_http_status: storage::latest_cloud_sync_queue_http_status(&conn)
            .map_err(|e| format!("读取云同步最近 HTTP 状态失败: {e}"))?,
        blocked_reason: storage::latest_cloud_sync_queue_blocked_reason(&conn)
            .map_err(|e| format!("读取云同步阻断原因失败: {e}"))?,
    })
}

#[tauri::command]
pub fn sign_out_desktop_cloud(app_handle: AppHandle) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    if let Some(profile) = load_desktop_cloud_sync_profile(&app_data_dir) {
        if !profile.refresh_token.trim().is_empty() && !profile.device_id.trim().is_empty() {
            if let Ok(client) = CloudSyncClient::new(&profile.cloud_base_url) {
                let _ = client.logout_auth_session(&profile.refresh_token, &profile.device_id);
            }
        }
    }
    clear_desktop_cloud_sync_profile(&app_data_dir)
}

#[tauri::command]
pub async fn create_desktop_auth_challenge(
    app_handle: AppHandle,
    input: CreateDesktopAuthChallengeInput,
) -> Result<AuthChallengeResponse, String> {
    if input.identifier.trim().is_empty() {
        return Err("请输入账户".to_string());
    }
    let cloud_base_url = config::load_system_config().cloud_base_url;
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let device_id = identity::load_identity(&app_data_dir)
        .map(|value| format!("desktop-{}", stable_short_id(&value.device_identity)))
        .unwrap_or_else(|| {
            format!(
                "desktop-{}",
                stable_short_id(&identity::current_device_identity())
            )
        });
    let request = AuthChallengeRequest {
        identifier: input.identifier.trim().to_string(),
        purpose: "register_or_login".to_string(),
        client_device_id: device_id,
        captcha_token: None,
    };
    let request_base_url = cloud_base_url.clone();
    tauri::async_runtime::spawn_blocking(move || {
        CloudSyncClient::new(&request_base_url)?.create_auth_challenge(&request)
    })
    .await
    .map_err(|e| format!("发送验证码任务失败: {e}"))?
}

#[tauri::command]
pub async fn continue_cloud_account(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: ContinueCloudAccountInput,
) -> Result<DesktopCloudSyncProfile, String> {
    let cloud_base_url = config::load_system_config().cloud_base_url;
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let local_identity = identity::load_identity(&app_data_dir);
    let creator_seed_ref = local_identity
        .as_ref()
        .map(|value| {
            format!(
                "desktop-creator-{}",
                stable_short_id(&value.creator_display_name)
            )
        })
        .unwrap_or_else(|| "desktop-seed-uninitialized".to_string());
    let device_id = local_identity
        .as_ref()
        .map(|value| format!("desktop-{}", stable_short_id(&value.device_identity)))
        .unwrap_or_else(|| {
            format!(
                "desktop-{}",
                stable_short_id(&identity::current_device_identity())
            )
        });
    let device_name = hostname::get()
        .ok()
        .and_then(|name| name.into_string().ok())
        .unwrap_or_else(|| "HiddenShield Desktop".to_string());

    let request = ContinueAccountRequest {
        identifier: input.identifier.trim().to_string(),
        password: input
            .password
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string(),
        verification_code: input
            .verification_code
            .as_deref()
            .unwrap_or_default()
            .trim()
            .to_string(),
        challenge_id: input.challenge_id,
        device: ContinueAccountDevice {
            client_device_id: device_id,
            name: device_name,
            platform: std::env::consts::OS.to_string(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            public_key: None,
        },
        local_creator_profile: ContinueAccountCreatorProfile {
            display_name: input.creator_display_name.trim().to_string(),
            creator_seed_ref,
            seed_envelope_version: 1,
        },
    };

    let request_base_url = cloud_base_url.clone();
    let session = tauri::async_runtime::spawn_blocking(move || {
        CloudSyncClient::new(&request_base_url)?.create_auth_session(&request)
    })
    .await
    .map_err(|e| format!("登录账户任务失败: {e}"))??;
    let profile = DesktopCloudSyncProfile::from_session(&cloud_base_url, session);
    save_creator_identity_if_needed(&app_data_dir, &input.creator_display_name)?;
    save_desktop_cloud_sync_profile(&app_data_dir, &profile)?;
    if can_auto_cloud_sync(&profile) {
        run_auto_cloud_sync_sequence(&app_data_dir, &state, profile.clone());
    }
    Ok(profile)
}

#[tauri::command]
pub async fn set_desktop_cloud_auto_sync_enabled(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: SetDesktopCloudAutoSyncInput,
) -> Result<DesktopCloudSyncProfile, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再调整云同步偏好".to_string())?;
    let mut profile =
        refresh_cloud_profile_snapshot_with_reauth(&app_data_dir, state.inner(), profile)?;
    ensure_cloud_sync_entitled(&profile)?;
    let base_url = profile.cloud_base_url.clone();
    let access_token = profile.access_token.clone();
    let enabled = input.enabled;
    let response = tauri::async_runtime::spawn_blocking(move || {
        CloudSyncClient::new(&base_url)?.update_sync_preferences(&access_token, enabled)
    })
    .await
    .map_err(|e| format!("调整自动云同步任务失败: {e}"))??;
    profile.apply_sync_preferences(response);
    save_desktop_cloud_sync_profile(&app_data_dir, &profile)?;
    if can_auto_cloud_sync(&profile) {
        run_auto_cloud_sync_sequence(&app_data_dir, &state, profile.clone());
    }
    Ok(profile)
}

#[tauri::command]
pub async fn list_desktop_cloud_devices(
    app_handle: AppHandle,
) -> Result<Vec<AccountDevice>, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再管理设备".to_string())?;
    let base_url = profile.cloud_base_url.clone();
    let access_token = profile.access_token.clone();
    let devices = tauri::async_runtime::spawn_blocking(move || {
        CloudSyncClient::new(&base_url)?.list_devices(&access_token)
    })
    .await
    .map_err(|e| format!("读取设备列表任务失败: {e}"))??
    .devices;
    Ok(devices)
}

#[tauri::command]
pub async fn update_desktop_cloud_device_name(
    app_handle: AppHandle,
    input: UpdateDesktopCloudDeviceInput,
) -> Result<AccountDevice, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再管理设备".to_string())?;
    let base_url = profile.cloud_base_url.clone();
    let access_token = profile.access_token.clone();
    let device_id = input.device_id;
    let name = input.name;
    tauri::async_runtime::spawn_blocking(move || {
        CloudSyncClient::new(&base_url)?.update_device_name(&access_token, &device_id, &name)
    })
    .await
    .map_err(|e| format!("更新设备名称任务失败: {e}"))?
}

#[tauri::command]
pub async fn revoke_desktop_cloud_device(
    app_handle: AppHandle,
    input: RevokeDesktopCloudDeviceInput,
) -> Result<RevokeDeviceResponse, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再管理设备".to_string())?;
    let base_url = profile.cloud_base_url.clone();
    let access_token = profile.access_token.clone();
    let device_id = input.device_id;
    tauri::async_runtime::spawn_blocking(move || {
        CloudSyncClient::new(&base_url)?.revoke_device(&access_token, &device_id)
    })
    .await
    .map_err(|e| format!("撤销设备任务失败: {e}"))?
}

#[tauri::command]
pub async fn refresh_desktop_auth_session(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<DesktopCloudSyncProfile, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let mut profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先登录 HiddenShield 账户，再刷新会话".to_string())?;
    let base_url = profile.cloud_base_url.clone();
    let refresh_token = profile.refresh_token.clone();
    let device_id = profile.device_id.clone();
    let session = tauri::async_runtime::spawn_blocking(move || {
        CloudSyncClient::new(&base_url)?.refresh_auth_session(&refresh_token, &device_id)
    })
    .await
    .map_err(|e| format!("刷新登录会话任务失败: {e}"))??;
    let cloud_base_url = profile.cloud_base_url.clone();
    profile.apply_session(&cloud_base_url, session);
    save_desktop_cloud_sync_profile(&app_data_dir, &profile)?;
    save_profile_entitlement_to_local_state(&state, &profile)?;
    if can_auto_cloud_sync(&profile) {
        run_auto_cloud_sync_sequence(&app_data_dir, &state, profile.clone());
    }
    Ok(profile)
}

#[tauri::command]
pub async fn refresh_desktop_cloud_account_snapshot(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<DesktopCloudSyncProfile, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let mut profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先登录 HiddenShield 账户，再刷新账户状态".to_string())?;
    profile = refresh_cloud_profile_snapshot_with_reauth(&app_data_dir, state.inner(), profile)?;
    if can_auto_cloud_sync(&profile) {
        run_auto_cloud_sync_sequence(&app_data_dir, &state, profile.clone());
    }
    Ok(profile)
}

fn save_creator_identity_if_needed(
    app_data_dir: &std::path::Path,
    creator_input: &str,
) -> Result<(), String> {
    let creator = creator_input.trim();
    if creator.is_empty() {
        return Ok(());
    }
    match identity::load_identity(app_data_dir) {
        Some(local_identity) => {
            let current = local_identity.creator_display_name.trim();
            if current != creator {
                identity::initialize_identity(app_data_dir, creator)?;
            }
        }
        None => {
            identity::initialize_identity(app_data_dir, creator)?;
        }
    }
    Ok(())
}

fn stable_short_id(value: &str) -> String {
    let digest = Sha256::digest(value.trim().as_bytes());
    hex::encode(&digest[..8])
}

#[tauri::command]
pub async fn create_billing_payment_session(
    app_handle: AppHandle,
    input: CreateBillingPaymentSessionInput,
) -> Result<BillingPaymentSessionResponse, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再开通订阅".to_string())?;
    let plan_code = input.plan_code.trim().to_lowercase();
    if !matches!(plan_code.as_str(), "creator" | "studio") {
        return Err("当前仅支持开通 Creator / Studio".to_string());
    }
    let billing_cycle = input
        .billing_cycle
        .as_deref()
        .unwrap_or("monthly")
        .trim()
        .to_lowercase();
    if !matches!(billing_cycle.as_str(), "monthly" | "yearly") {
        return Err("支付周期仅支持 monthly / yearly".to_string());
    }
    let request = BillingPaymentSessionRequest {
        account_id: profile.account_id.clone(),
        workspace_id: profile.workspace_id.clone(),
        plan_code,
        billing_cycle,
        preferred_provider: input
            .preferred_provider
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_string()),
    };
    CloudSyncClient::new(&profile.cloud_base_url)?
        .create_billing_payment_session(&profile.access_token, &request)
}

#[tauri::command]
pub async fn refresh_billing_entitlement(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<DesktopCloudSyncProfile, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let mut profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再刷新权益".to_string())?;
    let entitlement = CloudSyncClient::new(&profile.cloud_base_url)?
        .get_current_entitlement(&profile.access_token)?;
    profile.apply_entitlement(entitlement);
    save_desktop_cloud_sync_profile(&app_data_dir, &profile)?;
    save_profile_entitlement_to_local_state(&state, &profile)?;
    Ok(profile)
}

#[tauri::command]
pub async fn get_billing_payment_session_status(
    app_handle: AppHandle,
    input: BillingPaymentSessionIdInput,
) -> Result<BillingPaymentSessionStatusResponse, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再查看支付状态".to_string())?;
    CloudSyncClient::new(&profile.cloud_base_url)?
        .get_billing_payment_session_status(&profile.access_token, &input.payment_session_id)
}

#[tauri::command]
pub async fn reconcile_billing_payment_session(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: BillingPaymentSessionIdInput,
) -> Result<BillingPaymentSessionReconcileResponse, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let mut profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再确认支付状态".to_string())?;
    let result = CloudSyncClient::new(&profile.cloud_base_url)?
        .reconcile_billing_payment_session(&profile.access_token, &input.payment_session_id)?;
    profile.apply_entitlement(result.entitlement.clone());
    save_desktop_cloud_sync_profile(&app_data_dir, &profile)?;

    let local_state = cloud_profile_to_local_entitlement(&profile);
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    billing::save_entitlement_state(&conn, &local_state)
        .map_err(|e| format!("保存本地权益快照失败: {e}"))?;
    Ok(result)
}

#[tauri::command]
pub async fn create_report_purchase_session(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: CreateReportPurchaseSessionInput,
) -> Result<ReportPurchaseSessionResponse, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再购买报告".to_string())?;
    let record_exists = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        queries::list_records(&conn)
            .into_iter()
            .any(|record| record.id == input.vault_record_id)
    };
    if !record_exists {
        return Err(format!("未找到版权记录: {}", input.vault_record_id));
    }
    let product_code = normalize_report_product_code(&input.product_code)?;
    let request = ReportPurchaseSessionRequest {
        account_id: profile.account_id.clone(),
        workspace_id: profile.workspace_id.clone(),
        creator_profile_id: profile.creator_profile_id.clone(),
        vault_record_id: input.vault_record_id.to_string(),
        product_code,
        preferred_provider: input
            .preferred_provider
            .filter(|value| !value.trim().is_empty())
            .map(|value| value.trim().to_string()),
    };
    let cloud_base_url = profile.cloud_base_url.clone();
    let access_token = profile.access_token.clone();
    tokio::task::spawn_blocking(move || {
        CloudSyncClient::new(&cloud_base_url)?
            .create_report_purchase_session(&access_token, &request)
    })
    .await
    .map_err(|error| format!("创建报告购买会话任务异常: {error}"))?
}

#[tauri::command]
pub async fn get_report_purchase_session_status(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: ReportPurchaseSessionIdInput,
) -> Result<ReportPurchaseSessionStatusResponse, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再查看报告购买状态".to_string())?;
    let cloud_base_url = profile.cloud_base_url.clone();
    let access_token = profile.access_token.clone();
    let payment_session_id = input.payment_session_id;
    let result = tokio::task::spawn_blocking(move || {
        CloudSyncClient::new(&cloud_base_url)?
            .get_report_purchase_session_status(&access_token, &payment_session_id)
    })
    .await
    .map_err(|error| format!("查询报告购买会话任务异常: {error}"))??;
    if let Some(grant) = &result.grant {
        persist_report_purchase_grant(&state, grant)?;
    }
    Ok(result)
}

#[tauri::command]
pub async fn reconcile_report_purchase_session(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: ReportPurchaseSessionIdInput,
) -> Result<ReportPurchaseSessionReconcileResponse, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中登录账户，再确认报告购买状态".to_string())?;
    let cloud_base_url = profile.cloud_base_url.clone();
    let access_token = profile.access_token.clone();
    let payment_session_id = input.payment_session_id;
    let result = tokio::task::spawn_blocking(move || {
        CloudSyncClient::new(&cloud_base_url)?
            .reconcile_report_purchase_session(&access_token, &payment_session_id)
    })
    .await
    .map_err(|error| format!("确认报告购买会话任务异常: {error}"))??;
    if let Some(grant) = &result.grant {
        persist_report_purchase_grant(&state, grant)?;
    }
    Ok(result)
}

#[tauri::command]
pub async fn push_desktop_vault_record_to_cloud(
    state: State<'_, AppState>,
    input: PushDesktopVaultRecordInput,
) -> Result<CloudSyncBatchResult, String> {
    let record = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        queries::list_records(&conn)
            .into_iter()
            .find(|record| record.id == input.record_id)
            .ok_or_else(|| format!("未找到版权记录: {}", input.record_id))?
    };
    let event = vault_record_to_cloud_event(&record);
    CloudSyncClient::new(input.base_url)?.send_events_batch(
        &input.access_token,
        &input.device_id,
        &input.workspace_id,
        vec![event],
    )
}

#[tauri::command]
pub async fn push_saved_desktop_vault_record_to_cloud(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: PushSavedDesktopVaultRecordInput,
) -> Result<CloudSyncBatchResult, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "尚未登录 HiddenShield 账户".to_string())?;
    ensure_cloud_sync_entitled(&profile)?;
    enqueue_desktop_record_for_cloud(&state, input.record_id)?;
    let flush = flush_cloud_queue_with_profile(&state, &profile, 50)?;
    Ok(CloudSyncBatchResult {
        accepted: flush.synced,
        accepted_event_ids: Vec::new(),
        next_cursor: None,
        resolutions: serde_json::json!([]),
        event_results: None,
    })
}

#[tauri::command]
pub async fn fetch_cloud_changes(
    input: FetchCloudChangesInput,
) -> Result<CloudSyncChangesResult, String> {
    CloudSyncClient::new(input.base_url)?.fetch_changes(
        &input.access_token,
        &input.workspace_id,
        input.cursor.as_deref(),
    )
}

#[tauri::command]
pub async fn create_video_fingerprint_notary(
    app_handle: AppHandle,
    input: CreateVideoFingerprintNotaryInput,
) -> Result<VideoFingerprintNotaryReceipt, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "尚未登录 HiddenShield 账户".to_string())?;
    let mut request = match input.request {
        Some(request) => request,
        None => {
            let bundle = input
                .bundle
                .as_ref()
                .ok_or_else(|| "视频指纹存证缺少 bundle".to_string())?;
            video_fingerprint_bundle_to_notary_request(
                &profile.workspace_id,
                &profile.creator_profile_id,
                input.bundle_sha256.as_deref().unwrap_or_default(),
                input.bundle_bytes.unwrap_or_default(),
                bundle,
            )?
        }
    };
    request.workspace_id = profile.workspace_id.clone();
    request.creator_profile_id = profile.creator_profile_id.clone();
    CloudSyncClient::new(&profile.cloud_base_url)?
        .create_video_fingerprint_notary(&profile.access_token, &request)
}

#[tauri::command]
pub async fn create_video_fingerprint_notary_from_bundle_file(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: CreateVideoFingerprintNotaryFromBundleFileInput,
) -> Result<VideoFingerprintNotaryResult, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "尚未登录 HiddenShield 账户".to_string())?;
    let bundle_path = input.bundle_path.trim();
    if bundle_path.is_empty() {
        return Err("请选择视频指纹 bundle.json".to_string());
    }
    if !bundle_path.ends_with("bundle.json") {
        return Err("请选择 video_fingerprint_spike 生成的 bundle.json".to_string());
    }

    let bundle_bytes =
        std::fs::read(bundle_path).map_err(|e| format!("读取视频指纹 bundle 失败: {e}"))?;
    let bundle: VideoFingerprintBundleForNotary = serde_json::from_slice(&bundle_bytes)
        .map_err(|e| format!("解析视频指纹 bundle 失败: {e}"))?;
    let bundle_sha256 = format!("sha256:{:x}", Sha256::digest(&bundle_bytes));
    let request = video_fingerprint_bundle_to_notary_request(
        &profile.workspace_id,
        &profile.creator_profile_id,
        &bundle_sha256,
        bundle_bytes.len() as u64,
        &bundle,
    )?;

    let receipt = {
        let base_url = profile.cloud_base_url.clone();
        let access_token = profile.access_token.clone();
        let request = request.clone();
        tauri::async_runtime::spawn_blocking(move || {
            CloudSyncClient::new(&base_url)?
                .create_video_fingerprint_notary(&access_token, &request)
        })
        .await
        .map_err(|e| format!("提交视频指纹存证任务失败: {e}"))??
    };
    let record = persist_video_fingerprint_notary_record(
        &state,
        input
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("视频指纹存证"),
        &bundle_sha256,
        bundle_bytes.len() as u64,
        input.bundle_elapsed_ms,
        &bundle,
        &receipt,
        Some(profile.creator_display_name.clone()),
    )?;
    Ok(VideoFingerprintNotaryResult {
        receipt,
        vault_record: record,
    })
}

#[tauri::command]
pub async fn generate_video_fingerprint_bundle(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: GenerateVideoFingerprintBundleInput,
) -> Result<VideoFingerprintBundleGeneration, String> {
    let input_path = input.input_path.trim();
    if input_path.is_empty() {
        return Err("请选择视频文件".to_string());
    }
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let paths = if let Some(paths) = state.get_ffmpeg_paths() {
        paths
    } else {
        let paths = ffmpeg::detect_ffmpeg()
            .await
            .map_err(|e| format!("未找到 FFmpeg，无法生成视频指纹包: {e}"))?;
        state.set_ffmpeg_paths(paths.clone());
        paths
    };
    let output_root = app_data_dir.join("video_fingerprint_bundles");
    crate::video_fingerprint::generate_bundle(
        std::path::Path::new(input_path),
        &output_root,
        &paths.ffmpeg,
        &paths.ffprobe,
        8,
    )
}

#[tauri::command]
pub async fn create_l3_video_visual_upload_task(
    app_handle: AppHandle,
    input: CreateL3VideoVisualUploadTaskInput,
) -> Result<CreateL3VideoVisualUploadTaskResult, String> {
    let input_path = input.input_path.trim();
    if input_path.is_empty() {
        return Err("请选择要上传的 MP4 视频".to_string());
    }
    let source_path = Path::new(input_path);
    if !source_path.exists() || !source_path.is_file() {
        return Err("L3 上传源文件不存在或不是文件".to_string());
    }
    let content_type = l3_formal_upload_content_type(source_path)?;
    let duration_ms = input
        .duration_secs
        .filter(|value| value.is_finite() && *value > 0.0)
        .map(|value| (value * 1000.0).round() as u64)
        .filter(|value| *value > 0)
        .ok_or_else(|| "L3 创建任务需要可读取的视频时长，请先完成素材探测".to_string())?;
    if let (Some(width), Some(height), Some(frame_count)) =
        (input.width, input.height, input.frame_count)
    {
        if !l3_declared_capacity_is_supported(width, height, frame_count) {
            return Err(
                "L3 当前 release gate 不接收该尺寸 / 帧率组合：strategy_invalid 容量不足，请换用 1080p / 1024x576 以上主战场样本或降低短视频帧抽样密度"
                    .to_string(),
            );
        }
    }
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "尚未登录 HiddenShield 账户，无法创建 L3 云端任务".to_string())?;
    ensure_cloud_video_processing_entitled(&profile)?;

    let source_bytes =
        std::fs::read(source_path).map_err(|e| format!("读取 L3 上传视频失败: {e}"))?;
    if source_bytes.is_empty() {
        return Err("L3 上传视频为空文件".to_string());
    }
    let source_sha256 = format!("sha256:{:x}", Sha256::digest(&source_bytes));
    let uploaded_bytes = source_bytes.len() as u64;
    let request_id = format!(
        "desktop:{}:l3-video-visual:{}",
        profile.device_id,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|value| value.as_micros())
            .unwrap_or_default()
    );
    let cloud_base_url = profile.cloud_base_url.clone();
    let access_token = profile.access_token.clone();
    let workspace_id = profile.workspace_id.clone();
    let creator_profile_id = profile.creator_profile_id.clone();
    let width = input.width;
    let height = input.height;
    let frame_count = input.frame_count;

    let task = tauri::async_runtime::spawn_blocking(move || {
        let client = CloudSyncClient::new(&cloud_base_url)?;
        let upload_auth = client.create_cloud_video_task_object_upload_authorization(
            &access_token,
            &CloudVideoTaskObjectUploadAuthorizationRequest {
                workspace_id: workspace_id.clone(),
                creator_profile_id: creator_profile_id.clone(),
                sha256: source_sha256.clone(),
                bytes: uploaded_bytes,
                content_type,
                object_kind: Some("l3_user_object_upload_proxy".to_string()),
                ttl_seconds: Some(900),
            },
        )?;
        let upload_result = client
            .upload_cloud_video_task_object_bytes(&upload_auth.upload_token, &source_bytes)?;
        if upload_result.status != "uploaded" {
            return Err("L3 对象上传未返回 uploaded 状态".to_string());
        }
        if upload_result.sha256 != source_sha256 || upload_result.bytes != uploaded_bytes {
            return Err("L3 对象上传回读哈希或字节数不一致，已停止创建任务".to_string());
        }
        if upload_result.storage_ref != upload_auth.storage_ref
            || !upload_result.storage_ref.starts_with("object://l3-upload/")
        {
            return Err("L3 对象上传 storageRef 不在正式 l3-upload 对象边界内".to_string());
        }
        let reserved = client.reserve_watermark_id(
            &access_token,
            &WatermarkIdReserveRequest {
                request_id,
                workspace_id: workspace_id.clone(),
                creator_profile_id: creator_profile_id.clone(),
                media_type: "video_visual".to_string(),
                payload_protocol_version: 2,
                payload_bytes_length: 119,
                parent_watermark_uid: None,
                revision: 1,
                original_hash: Some(source_sha256.clone()),
            },
        )?;
        let task = client.create_cloud_video_task(
            &access_token,
            &CloudVideoTaskRequest {
                schema_version: "cloud_video_task_v1".to_string(),
                workspace_id,
                creator_profile_id,
                capability_level: "hybrid_visual_watermark".to_string(),
                watermark_uid: reserved.watermark_uid,
                source_hash: source_sha256,
                duration_ms,
                target_profiles: vec!["studio_enterprise_l3_formal_upload_h264".to_string()],
                upload_manifest: VideoUploadManifest {
                    schema_version: "video_upload_manifest_v1".to_string(),
                    contains_original_video: false,
                    contains_watermarked_video: false,
                    contains_local_paths: false,
                    contains_proxy: true,
                    items: vec![VideoUploadManifestItem {
                        kind: "l3_user_object_upload_proxy".to_string(),
                        sha256: upload_result.sha256,
                        bytes: upload_result.bytes,
                        storage_ref: Some(upload_result.storage_ref),
                        sandbox_profile: Some("l3_ffmpeg_transcode_sandbox_v1".to_string()),
                        transcode_profile: Some("h264_controlled_proxy_v1".to_string()),
                        width,
                        height,
                        frame_count,
                    }],
                },
            },
        )?;
        Ok::<_, String>(task)
    })
    .await
    .map_err(|e| format!("创建 L3 上传任务失败: {e}"))??;

    Ok(CreateL3VideoVisualUploadTaskResult {
        watermark_uid: task.watermark_uid.clone(),
        source_sha256: task.source_hash.clone(),
        uploaded_bytes,
        task,
        privacy_boundary: l3_formal_upload_privacy_boundary().to_string(),
        next_action: "等待 trusted worker 完成自检和收据固化；任务 succeeded 后再下载并保存版权库"
            .to_string(),
    })
}

#[tauri::command]
pub async fn save_l3_video_visual_task_to_vault(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: SaveL3VideoVisualTaskInput,
) -> Result<SaveL3VideoVisualTaskResult, String> {
    let task_id = input.task_id.trim().to_string();
    if task_id.is_empty() {
        return Err("请输入已成功的 L3 taskId".to_string());
    }
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "尚未登录 HiddenShield 账户".to_string())?;
    ensure_cloud_video_processing_entitled(&profile)?;

    let title = input
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("L3 视频画面盲水印成品")
        .to_string();
    let cloud_base_url = profile.cloud_base_url.clone();
    let access_token = profile.access_token.clone();
    let task_id_for_worker = task_id.clone();
    let (task, output_bytes) = tauri::async_runtime::spawn_blocking(move || {
        let client = CloudSyncClient::new(&cloud_base_url)?;
        let task = client.get_cloud_video_task(&access_token, &task_id_for_worker)?;
        validate_l3_video_visual_task_for_vault(&task)?;
        let auth = client.create_cloud_video_task_download_authorization(
            &access_token,
            &task_id_for_worker,
            Some(900),
        )?;
        if auth.status != "succeeded" || auth.output_media_content_type != "video/mp4" {
            return Err("L3 下载授权不是 succeeded/video/mp4".to_string());
        }
        let bytes = client.download_cloud_video_task_output(
            &access_token,
            &task_id_for_worker,
            &auth.download_token,
        )?;
        Ok::<_, String>((task, bytes))
    })
    .await
    .map_err(|e| format!("领取 L3 视频成品失败: {e}"))??;

    let output_sha256 = format!("sha256:{:x}", Sha256::digest(&output_bytes));
    let expected_hash = task
        .watermarked_media_hash
        .as_deref()
        .ok_or_else(|| "L3 succeeded task 缺少 watermarkedMediaHash".to_string())?;
    if output_sha256 != expected_hash {
        return Err("L3 下载成品哈希与后端完成态不一致，已拒绝入库".to_string());
    }
    let expected_bytes = task
        .output_media_bytes
        .ok_or_else(|| "L3 succeeded task 缺少 outputMediaBytes".to_string())?;
    if output_bytes.len() as u64 != expected_bytes {
        return Err("L3 下载成品字节数与后端完成态不一致，已拒绝入库".to_string());
    }

    let output_dir = app_data_dir.join("l3_video_visual_outputs");
    std::fs::create_dir_all(&output_dir).map_err(|e| format!("创建 L3 输出目录失败: {e}"))?;
    let output_name = format!("{}.l3-watermarked.mp4", safe_file_stem(&task.task_id));
    let output_path = output_dir.join(&output_name);
    std::fs::write(&output_path, &output_bytes)
        .map_err(|e| format!("保存 L3 MP4 成品失败: {e}"))?;

    let mut record =
        build_l3_video_visual_vault_record(&title, &output_name, &task, &profile, &output_path)?;
    {
        let mut conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("创建 L3 版权记录事务失败: {e}"))?;
        let record_id = queries::insert_record_tx(&tx, &record)
            .map_err(|e| format!("保存 L3 版权记录失败: {e}"))?;
        tx.commit()
            .map_err(|e| format!("提交 L3 版权记录失败: {e}"))?;
        record.id = record_id as u32;
    }
    enqueue_desktop_record_for_cloud(&state, record.id)?;
    let cloud_sync = if can_auto_cloud_sync(&profile) {
        Some(flush_cloud_queue_with_profile(&state, &profile, 50)?)
    } else {
        None
    };

    Ok(SaveL3VideoVisualTaskResult {
        task,
        vault_record: record,
        output_path: output_path.to_string_lossy().to_string(),
        output_sha256,
        cloud_sync,
    })
}

#[tauri::command]
pub async fn pull_saved_cloud_changes_into_desktop(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<CloudPullResult, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "尚未登录 HiddenShield 账户".to_string())?;
    pull_saved_cloud_changes_with_profile(&app_data_dir, &state, profile)
}

fn pull_saved_cloud_changes_with_profile(
    app_data_dir: &std::path::Path,
    state: &State<'_, AppState>,
    mut profile: DesktopCloudSyncProfile,
) -> Result<CloudPullResult, String> {
    ensure_cloud_sync_entitled(&profile)?;
    let changes = CloudSyncClient::new(&profile.cloud_base_url)?.fetch_changes(
        &profile.access_token,
        &profile.workspace_id,
        profile.last_remote_cursor.as_deref(),
    )?;

    let mut applied = 0u32;
    let mut skipped = 0u32;
    let mut imported_queue_ids = Vec::new();
    {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        for change in &changes.changes {
            let Some(item) = cloud_change_to_mobile_sync_item(change) else {
                skipped += 1;
                continue;
            };
            imported_queue_ids.push(item.queue_id.clone());
            match storage::record_sync_event(&conn, &item) {
                Ok(_) => applied += 1,
                Err(_) => skipped += 1,
            }
        }
    }

    profile.last_remote_cursor = Some(changes.next_cursor.clone());
    save_desktop_cloud_sync_profile(&app_data_dir, &profile)?;

    Ok(CloudPullResult {
        next_cursor: changes.next_cursor,
        total_changes: changes.changes.len() as u32,
        applied,
        skipped,
        imported_queue_ids,
    })
}

#[tauri::command]
pub async fn flush_desktop_cloud_sync_queue(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: FlushCloudSyncQueueInput,
) -> Result<CloudQueueFlushResult, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "尚未登录 HiddenShield 账户".to_string())?;
    let profile =
        refresh_cloud_profile_snapshot_with_reauth(&app_data_dir, state.inner(), profile)?;
    if !has_cloud_sync_entitlement(&profile) {
        let message =
            "正式云同步从 Creator 开放，当前账户以后端权益快照为准，已阻断本次上传".to_string();
        let blocked = {
            let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
            storage::mark_uploadable_cloud_sync_queue_blocked_by_entitlement(&conn, &message)
                .map_err(|e| format!("写入云同步权益阻断诊断失败: {e}"))?
        };
        return Ok(CloudQueueFlushResult {
            attempted: 0,
            synced: 0,
            failed: blocked as u32,
            message,
        });
    }
    {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        storage::recover_stale_cloud_syncing_queue(
            &conn,
            chrono::Utc::now() - chrono::Duration::minutes(10),
        )
        .map_err(|e| format!("恢复中断云同步队列失败: {e}"))?;
        storage::reset_cloud_sync_queue_backoff(&conn)
            .map_err(|e| format!("重置云同步重试退避失败: {e}"))?;
    }
    flush_cloud_queue_with_profile(&state, &profile, input.limit.unwrap_or(50))
}

fn enqueue_desktop_record_for_cloud(
    state: &State<'_, AppState>,
    record_id: u32,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    let record = queries::list_records(&conn)
        .into_iter()
        .find(|record| record.id == record_id)
        .ok_or_else(|| format!("未找到版权记录: {record_id}"))?;
    let event = vault_record_to_cloud_event(&record);
    let event_json =
        serde_json::to_string(&event).map_err(|e| format!("序列化云同步事件失败: {e}"))?;
    storage::enqueue_cloud_sync_event(&conn, &event.client_event_id, record.id, &event_json)
        .map_err(|e| format!("写入云同步队列失败: {e}"))
}

fn persist_video_fingerprint_notary_record(
    state: &State<'_, AppState>,
    title: &str,
    bundle_sha256: &str,
    bundle_bytes: u64,
    elapsed_ms: Option<u64>,
    bundle: &VideoFingerprintBundleForNotary,
    receipt: &VideoFingerprintNotaryReceipt,
    creator_display_name: Option<String>,
) -> Result<crate::commands::vault::VaultRecord, String> {
    let source_hash = strip_sha256_prefix(&receipt.source_hash)
        .or_else(|| strip_sha256_prefix(&bundle.source_hash))
        .unwrap_or_else(|| receipt.source_hash.clone());
    let duration_secs = bundle.duration_ms as f64 / 1000.0;
    let record = crate::commands::vault::VaultRecord {
        id: 0,
        original_hash: source_hash,
        file_name: title.to_string(),
        created_at: receipt.notarized_at.clone(),
        duration_secs,
        resolution: "视频指纹存证".to_string(),
        watermark_uid: receipt.watermark_uid.clone(),
        creator_display_name: creator_display_name.filter(|value| !value.trim().is_empty()),
        thumbnail_path: None,
        output_douyin: None,
        output_bilibili: None,
        output_xhs: None,
        is_hdr_source: false,
        hw_encoder_used: None,
        process_time_ms: None,
        tsa_token_path: None,
        network_time: None,
        tsa_source: None,
        tsa_request_nonce: None,
        is_ai_generated: false,
        ai_training_permission: None,
        ai_generation_method: None,
        human_modification_level: None,
        authenticity_claim: None,
        custom_metadata: Some("L2 不可逆视频指纹存证".to_string()),
        output_douyin_hash: None,
        output_bilibili_hash: None,
        output_xhs_hash: None,
        protected_copy_name: None,
        protected_copy_path: None,
        protected_copy_hash: None,
        output_strategy: "minimal_required_change".to_string(),
        work_source_declaration: "unspecified".to_string(),
        training_permission_declaration: "prohibited".to_string(),
        creation_method_declaration: "unspecified".to_string(),
        human_edit_level_declaration: "unspecified".to_string(),
        authenticity_claim_declaration: "unspecified".to_string(),
        custom_rights_statement: None,
        parent_watermark_uid: None,
        revision: 1,
        rewrite_reason: None,
        write_verification_status: Some("verified".to_string()),
        write_verification_message: Some("云端视频指纹存证已完成".to_string()),
        write_verification_at: Some(receipt.notarized_at.clone()),
        payload_protocol_version: 2,
        payload_bytes_length: 119,
        watermark_id_issue_mode: "offline_generated".to_string(),
        watermark_id_registry_status: "pending_registration".to_string(),
        watermark_id_registry_receipt: None,
        payload_auth_status: "verified".to_string(),
        video_notary_id: Some(receipt.notary_id.clone()),
        video_notary_at: Some(receipt.notarized_at.clone()),
        video_notary_receipt_signature: Some(receipt.server_receipt_signature.clone()),
        video_notary_usage_ledger_id: Some(receipt.usage_ledger_id.clone()),
        video_fingerprint_root: Some(receipt.fingerprint_root.clone()),
        video_bundle_sha256: Some(bundle_sha256.to_string()),
        video_bundle_bytes: Some(bundle_bytes),
        video_bundle_scene_count: Some(bundle.scene_count as u32),
        video_bundle_elapsed_ms: elapsed_ms,
        video_frame_sample_policy: Some(bundle.frame_sample_policy.clone()),
        video_visual_task_id: None,
        video_visual_completed_at: None,
        video_visual_strategy_digest: None,
        video_visual_self_check_confidence: None,
        video_visual_self_check_threshold: None,
        video_visual_checked_frames: None,
        video_visual_media_hash: None,
        video_visual_receipt_hash: None,
        video_visual_output_bytes: None,
        video_visual_output_content_type: None,
    };
    let mut conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    let tx = conn
        .transaction()
        .map_err(|e| format!("创建视频存证记录事务失败: {e}"))?;
    let record_id = queries::insert_record_tx(&tx, &record)
        .map_err(|e| format!("保存视频存证记录失败: {e}"))?;
    tx.commit()
        .map_err(|e| format!("提交视频存证记录失败: {e}"))?;
    let mut saved = record;
    saved.id = record_id as u32;
    Ok(saved)
}

fn validate_l3_video_visual_task_for_vault(task: &CloudVideoTaskRecord) -> Result<(), String> {
    if task.status != "succeeded" {
        return Err("只能领取已 succeeded 的 L3 视频画面盲水印任务".to_string());
    }
    if !is_l3_video_visual_task_capability(&task.capability_level) {
        return Err("该任务不是 L3 视频画面盲水印任务".to_string());
    }
    let confidence = task
        .self_check_confidence
        .ok_or_else(|| "L3 task 缺少 selfCheckConfidence".to_string())?;
    let threshold = task
        .self_check_threshold
        .ok_or_else(|| "L3 task 缺少 selfCheckThreshold".to_string())?;
    if confidence < threshold {
        return Err("L3 task 自检置信度低于阈值，拒绝入库".to_string());
    }
    if task.checked_frames.unwrap_or_default() == 0 {
        return Err("L3 task 缺少 checkedFrames".to_string());
    }
    let output_ref = task
        .output_media_storage_ref
        .as_deref()
        .ok_or_else(|| "L3 task 缺少 outputMediaStorageRef".to_string())?;
    if !output_ref.starts_with("object://l3-output/") {
        return Err("L3 task 输出不是正式对象存储产物".to_string());
    }
    if task.output_media_content_type.as_deref() != Some("video/mp4") {
        return Err("L3 task 输出不是 video/mp4".to_string());
    }
    if task.output_media_bytes.unwrap_or_default() == 0 {
        return Err("L3 task 输出字节数为空".to_string());
    }
    if task
        .watermarked_media_hash
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        return Err("L3 task 缺少 watermarkedMediaHash".to_string());
    }
    if task
        .worker_receipt_hash
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        return Err("L3 task 缺少 workerReceiptHash".to_string());
    }
    if task
        .server_receipt_signature
        .as_deref()
        .unwrap_or_default()
        .is_empty()
    {
        return Err("L3 task 缺少 serverReceiptSignature".to_string());
    }
    Ok(())
}

fn build_l3_video_visual_vault_record(
    title: &str,
    output_name: &str,
    task: &CloudVideoTaskRecord,
    profile: &DesktopCloudSyncProfile,
    output_path: &Path,
) -> Result<crate::commands::vault::VaultRecord, String> {
    validate_l3_video_visual_task_for_vault(task)?;
    let original_hash =
        strip_sha256_prefix(&task.source_hash).unwrap_or_else(|| task.source_hash.clone());
    let protected_hash = task
        .watermarked_media_hash
        .as_deref()
        .and_then(strip_sha256_prefix)
        .or_else(|| task.watermarked_media_hash.clone())
        .ok_or_else(|| "L3 task 缺少 watermarkedMediaHash".to_string())?;
    Ok(crate::commands::vault::VaultRecord {
        id: 0,
        original_hash,
        file_name: title.to_string(),
        created_at: task
            .completed_at
            .clone()
            .unwrap_or_else(|| task.updated_at.clone()),
        duration_secs: task.duration_ms as f64 / 1000.0,
        resolution: "L3 视频画面盲水印".to_string(),
        watermark_uid: task.watermark_uid.clone(),
        creator_display_name: Some(profile.creator_display_name.clone())
            .filter(|value| !value.trim().is_empty()),
        thumbnail_path: None,
        output_douyin: None,
        output_bilibili: None,
        output_xhs: None,
        is_hdr_source: false,
        hw_encoder_used: None,
        process_time_ms: None,
        tsa_token_path: None,
        network_time: None,
        tsa_source: None,
        tsa_request_nonce: None,
        is_ai_generated: false,
        ai_training_permission: None,
        ai_generation_method: None,
        human_modification_level: None,
        authenticity_claim: None,
        custom_metadata: Some("L3 视频画面盲水印成品收据".to_string()),
        output_douyin_hash: None,
        output_bilibili_hash: None,
        output_xhs_hash: None,
        protected_copy_name: Some(output_name.to_string()),
        protected_copy_path: Some(output_path.to_string_lossy().to_string()),
        protected_copy_hash: Some(protected_hash),
        output_strategy: "cloud_l3_video_visual_watermark".to_string(),
        work_source_declaration: "unspecified".to_string(),
        training_permission_declaration: "prohibited".to_string(),
        creation_method_declaration: "unspecified".to_string(),
        human_edit_level_declaration: "unspecified".to_string(),
        authenticity_claim_declaration: "unspecified".to_string(),
        custom_rights_statement: None,
        parent_watermark_uid: None,
        revision: 1,
        rewrite_reason: None,
        write_verification_status: Some("verified".to_string()),
        write_verification_message: Some(
            "L3 云端视频画面盲水印自检和签名下载哈希校验已通过".to_string(),
        ),
        write_verification_at: task
            .completed_at
            .clone()
            .or_else(|| Some(task.updated_at.clone())),
        payload_protocol_version: 2,
        payload_bytes_length: 119,
        watermark_id_issue_mode: "server_reserved".to_string(),
        watermark_id_registry_status: "server_confirmed".to_string(),
        watermark_id_registry_receipt: task.server_receipt_signature.clone(),
        payload_auth_status: "verified".to_string(),
        video_notary_id: None,
        video_notary_at: None,
        video_notary_receipt_signature: None,
        video_notary_usage_ledger_id: None,
        video_fingerprint_root: None,
        video_bundle_sha256: None,
        video_bundle_bytes: None,
        video_bundle_scene_count: None,
        video_bundle_elapsed_ms: None,
        video_frame_sample_policy: None,
        video_visual_task_id: Some(task.task_id.clone()),
        video_visual_completed_at: task.completed_at.clone(),
        video_visual_strategy_digest: task.strategy_digest.clone(),
        video_visual_self_check_confidence: task.self_check_confidence,
        video_visual_self_check_threshold: task.self_check_threshold,
        video_visual_checked_frames: task.checked_frames,
        video_visual_media_hash: task.watermarked_media_hash.clone(),
        video_visual_receipt_hash: task.worker_receipt_hash.clone(),
        video_visual_output_bytes: task.output_media_bytes,
        video_visual_output_content_type: task.output_media_content_type.clone(),
    })
}

fn safe_file_stem(value: &str) -> String {
    let stem = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let stem = stem.trim_matches('_');
    if stem.is_empty() {
        "l3-video-visual".to_string()
    } else {
        stem.to_string()
    }
}

fn l3_formal_upload_content_type(path: &Path) -> Result<String, String> {
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    match extension.as_str() {
        "mp4" => Ok("video/mp4".to_string()),
        _ => Err("L3 正式创建上传入口当前只接收 MP4；MOV / WebM / MKV 需要等 worker 转码入口放开后再承诺".to_string()),
    }
}

fn l3_declared_capacity_is_supported(width: u32, height: u32, frame_count: u32) -> bool {
    const MAX_REGIONS: u32 = 96;
    const PAYLOAD_BYTES: u32 = 119;
    const SYNC_BITS: u32 = 16;
    const ECC_REPEAT: u32 = 3;
    const DCT_COEFF_PAIRS: u32 = 3;
    if width < 512 || height < 512 || width % 8 != 0 || height % 8 != 0 || frame_count == 0 {
        return false;
    }
    let region_width = (width / 8).max(1);
    let region_height = (height / 8).max(1);
    let blocks_per_region = (region_width / 8).max(1) * (region_height / 8).max(1);
    let min_regions_per_strategy_frame = (MAX_REGIONS / frame_count).max(1);
    let estimated_bits = blocks_per_region * min_regions_per_strategy_frame * DCT_COEFF_PAIRS;
    let required_bits = SYNC_BITS + PAYLOAD_BYTES * 8 * ECC_REPEAT;
    estimated_bits >= required_bits
}

fn is_l3_video_visual_task_capability(value: &str) -> bool {
    matches!(value.trim(), "video_visual" | "hybrid_visual_watermark")
}

fn l3_formal_upload_privacy_boundary() -> &'static str {
    "signed_object_upload_only_no_local_path_no_raw_video_sync"
}

fn strip_sha256_prefix(value: &str) -> Option<String> {
    value
        .trim()
        .strip_prefix("sha256:")
        .map(|rest| rest.to_string())
}

#[derive(Debug, Clone)]
struct ParsedCloudQueueEvent {
    queue_id: String,
    client_event_id: String,
}

#[derive(Debug, Clone)]
struct CloudQueueEventFailure {
    queue_id: String,
    error: String,
    error_code: String,
}

#[derive(Debug, Clone)]
struct CloudQueueBatchOutcome {
    synced_queue_ids: Vec<String>,
    failed_events: Vec<CloudQueueEventFailure>,
    parse_failed_count: usize,
}

fn cloud_queue_batch_outcome(
    parsed_events: &[ParsedCloudQueueEvent],
    parse_failed_ids: &[String],
    batch: &CloudSyncBatchResult,
) -> CloudQueueBatchOutcome {
    let mut synced_queue_ids = Vec::new();
    let mut failed_events = Vec::new();

    if let Some(event_results) = &batch.event_results {
        for event in parsed_events {
            let result = event_results
                .iter()
                .find(|result| result.client_event_id == event.client_event_id);
            let Some(result) = result else {
                failed_events.push(CloudQueueEventFailure {
                    queue_id: event.queue_id.clone(),
                    error: "云端未返回该事件的同步结果".to_string(),
                    error_code: "missing_event_result".to_string(),
                });
                continue;
            };

            match result.disposition.as_str() {
                "accepted" | "duplicate" => synced_queue_ids.push(event.queue_id.clone()),
                "conflict_payload_changed" | "rejected_invalid_event" => {
                    failed_events.push(CloudQueueEventFailure {
                        queue_id: event.queue_id.clone(),
                        error: cloud_event_disposition_error(
                            &result.disposition,
                            result.message.as_deref(),
                        ),
                        error_code: result.disposition.clone(),
                    });
                }
                other => failed_events.push(CloudQueueEventFailure {
                    queue_id: event.queue_id.clone(),
                    error: cloud_event_disposition_error(other, result.message.as_deref()),
                    error_code: "unexpected_event_result".to_string(),
                }),
            }
        }
    } else {
        for event in parsed_events {
            if batch.accepted_event_ids.is_empty()
                || batch
                    .accepted_event_ids
                    .iter()
                    .any(|id| id == &event.client_event_id || id == &event.queue_id)
            {
                synced_queue_ids.push(event.queue_id.clone());
            } else {
                failed_events.push(CloudQueueEventFailure {
                    queue_id: event.queue_id.clone(),
                    error: "云端未接收该事件".to_string(),
                    error_code: "cloud_event_not_accepted".to_string(),
                });
            }
        }
    }

    CloudQueueBatchOutcome {
        synced_queue_ids,
        failed_events,
        parse_failed_count: parse_failed_ids.len(),
    }
}

fn cloud_event_disposition_error(disposition: &str, message: Option<&str>) -> String {
    let message = message.map(str::trim).filter(|value| !value.is_empty());
    match message {
        Some(message) => format!("云端未接收该事件: {disposition} ({message})"),
        None => format!("云端未接收该事件: {disposition}"),
    }
}

fn flush_cloud_queue_with_profile(
    state: &State<'_, AppState>,
    profile: &DesktopCloudSyncProfile,
    limit: u32,
) -> Result<CloudQueueFlushResult, String> {
    ensure_cloud_sync_entitled(profile)?;
    let limit = limit.clamp(1, 100) as usize;
    let queued = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        storage::recover_stale_cloud_syncing_queue(
            &conn,
            chrono::Utc::now() - chrono::Duration::minutes(10),
        )
        .map_err(|e| format!("恢复中断云同步队列失败: {e}"))?;
        storage::list_pending_cloud_sync_queue(&conn, limit)
            .map_err(|e| format!("读取云同步队列失败: {e}"))?
    };
    if queued.is_empty() {
        return Ok(CloudQueueFlushResult {
            attempted: 0,
            synced: 0,
            failed: 0,
            message: "没有待同步的云队列".to_string(),
        });
    }
    let queue_ids = queued
        .iter()
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        storage::mark_cloud_sync_queue_syncing(&conn, &queue_ids)
            .map_err(|e| format!("更新云同步队列失败: {e}"))?;
    }

    let client = CloudSyncClient::new(&profile.cloud_base_url)?;
    let mut events = Vec::new();
    let mut parsed_events = Vec::new();
    let mut parse_failed_ids = Vec::new();
    for item in &queued {
        match serde_json::from_str::<CloudSyncEvent>(&item.event_json) {
            Ok(event) => {
                match reconcile_cloud_event_before_send(state, profile, &client, item, event) {
                    Ok(event) => {
                        parsed_events.push(ParsedCloudQueueEvent {
                            queue_id: item.id.clone(),
                            client_event_id: event.client_event_id.clone(),
                        });
                        events.push(event);
                    }
                    Err(error) => {
                        parse_failed_ids.push(item.id.clone());
                        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
                        storage::mark_cloud_sync_queue_failed(&conn, &[item.id.clone()], &error)
                            .map_err(|e| format!("更新云同步队列失败: {e}"))?;
                    }
                }
            }
            Err(error) => {
                parse_failed_ids.push(item.id.clone());
                let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
                storage::mark_cloud_sync_queue_failed(
                    &conn,
                    &[item.id.clone()],
                    &format!("解析队列事件失败: {error}"),
                )
                .map_err(|e| format!("更新云同步队列失败: {e}"))?;
            }
        }
    }
    if events.is_empty() {
        return Ok(CloudQueueFlushResult {
            attempted: queued.len() as u32,
            synced: 0,
            failed: queued.len() as u32,
            message: "云同步队列事件解析失败".to_string(),
        });
    }

    let result = client.send_events_batch(
        &profile.access_token,
        &profile.device_id,
        &profile.workspace_id,
        events,
    );
    match result {
        Ok(batch) => {
            let outcome = cloud_queue_batch_outcome(&parsed_events, &parse_failed_ids, &batch);
            let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
            storage::mark_cloud_sync_queue_synced(&conn, &outcome.synced_queue_ids)
                .map_err(|e| format!("更新云同步队列失败: {e}"))?;
            for failure in &outcome.failed_events {
                storage::mark_cloud_sync_queue_failed_structured(
                    &conn,
                    &[failure.queue_id.clone()],
                    &failure.error,
                    &failure.error_code,
                    None,
                )
                .map_err(|e| format!("更新云同步队列失败: {e}"))?;
            }
            let failed_count = outcome.failed_events.len() + outcome.parse_failed_count;
            Ok(CloudQueueFlushResult {
                attempted: queue_ids.len() as u32,
                synced: outcome.synced_queue_ids.len() as u32,
                failed: failed_count as u32,
                message: format!(
                    "已同步 {} 条，失败 {} 条",
                    outcome.synced_queue_ids.len(),
                    failed_count
                ),
            })
        }
        Err(error) => {
            let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
            if cloud_error_has_http_status(&error, 403) {
                storage::mark_cloud_sync_queue_blocked_by_entitlement(&conn, &queue_ids, &error)
                    .map_err(|e| format!("更新云同步权益阻断失败: {e}"))?;
            } else if cloud_error_has_http_status(&error, 401) {
                storage::mark_cloud_sync_queue_auth_required(&conn, &queue_ids, &error)
                    .map_err(|e| format!("更新云同步会话诊断失败: {e}"))?;
            } else {
                storage::mark_cloud_sync_queue_failed_structured(
                    &conn,
                    &queue_ids,
                    &error,
                    cloud_error_code(&error),
                    cloud_error_http_status(&error),
                )
                .map_err(|e| format!("更新云同步队列失败: {e}"))?;
            }
            Ok(CloudQueueFlushResult {
                attempted: queue_ids.len() as u32,
                synced: 0,
                failed: queue_ids.len() as u32,
                message: error,
            })
        }
    }
}

fn refresh_cloud_profile_snapshot_with_reauth(
    app_data_dir: &Path,
    state: &AppState,
    mut profile: DesktopCloudSyncProfile,
) -> Result<DesktopCloudSyncProfile, String> {
    let client = CloudSyncClient::new(&profile.cloud_base_url)?;
    let snapshot = match client.get_me(&profile.access_token) {
        Ok(snapshot) => snapshot,
        Err(error) if cloud_error_has_http_status(&error, 401) => {
            let session =
                client.refresh_auth_session(&profile.refresh_token, &profile.device_id)?;
            let cloud_base_url = profile.cloud_base_url.clone();
            profile.apply_session(&cloud_base_url, session);
            save_desktop_cloud_sync_profile(app_data_dir, &profile)?;
            client.get_me(&profile.access_token)?
        }
        Err(error) => return Err(error),
    };
    profile.apply_snapshot(snapshot);
    save_desktop_cloud_sync_profile(app_data_dir, &profile)?;
    save_profile_entitlement_to_local_state_from_app_state(state, &profile)?;
    Ok(profile)
}

fn save_profile_entitlement_to_local_state_from_app_state(
    state: &AppState,
    profile: &DesktopCloudSyncProfile,
) -> Result<(), String> {
    let local_state = cloud_profile_to_local_entitlement(profile);
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    billing::save_entitlement_state(&conn, &local_state)
        .map_err(|e| format!("保存本地权益快照失败: {e}"))
}

fn cloud_error_has_http_status(error: &str, status: u16) -> bool {
    error.contains(&format!("HTTP {status}"))
}

fn cloud_error_http_status(error: &str) -> Option<u16> {
    for status in [
        400u16, 401, 403, 404, 408, 409, 422, 429, 500, 502, 503, 504,
    ] {
        if cloud_error_has_http_status(error, status) {
            return Some(status);
        }
    }
    None
}

fn cloud_error_code(error: &str) -> &'static str {
    if cloud_error_has_http_status(error, 401) {
        "auth_required"
    } else if cloud_error_has_http_status(error, 403) {
        "blocked_by_entitlement"
    } else if cloud_error_has_http_status(error, 429) {
        "rate_limited"
    } else if cloud_error_http_status(error).is_some() {
        "http_error"
    } else {
        "network_or_unknown_error"
    }
}

fn reconcile_cloud_event_before_send(
    state: &State<'_, AppState>,
    profile: &DesktopCloudSyncProfile,
    client: &CloudSyncClient,
    item: &storage::CloudSyncQueueItem,
    event: CloudSyncEvent,
) -> Result<CloudSyncEvent, String> {
    if event.entity_type != "vaultRecord" {
        return Ok(event);
    }
    let status = event
        .payload
        .get("watermark_id_registry_status")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    if !matches!(status, "pending_registration" | "reserved") {
        return Ok(event);
    }

    let mut record = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        queries::list_records(&conn)
            .into_iter()
            .find(|record| record.id == item.record_id)
            .ok_or_else(|| format!("未找到待同步版权记录: {}", item.record_id))?
    };
    let media_type = media_type_for_cloud_event(&event);
    let write_status = record
        .write_verification_status
        .clone()
        .unwrap_or_else(|| "verified".to_string());
    let original_hash = Some(prefixed_sha256_for_registry(&record.original_hash));
    let protected_copy_hash = record
        .protected_copy_hash
        .as_deref()
        .map(prefixed_sha256_for_registry);
    let response = if record.watermark_id_issue_mode == "server_reserved"
        || record.watermark_id_registry_status == "reserved"
    {
        client.confirm_watermark_id(
            &profile.access_token,
            &crate::sync::cloud::WatermarkIdConfirmRequest {
                workspace_id: profile.workspace_id.clone(),
                creator_profile_id: profile.creator_profile_id.clone(),
                watermark_uid: record.watermark_uid.clone(),
                payload_protocol_version: record.payload_protocol_version,
                payload_bytes_length: record.payload_bytes_length,
                original_hash,
                protected_copy_hash,
                write_verification_status: write_status,
            },
        )?
    } else {
        client.reconcile_watermark_id(
            &profile.access_token,
            &crate::sync::cloud::WatermarkIdReconcileRequest {
                workspace_id: profile.workspace_id.clone(),
                creator_profile_id: profile.creator_profile_id.clone(),
                watermark_uid: record.watermark_uid.clone(),
                media_type,
                payload_protocol_version: record.payload_protocol_version,
                payload_bytes_length: record.payload_bytes_length,
                parent_watermark_uid: record.parent_watermark_uid.clone(),
                revision: record.revision,
                original_hash,
                protected_copy_hash,
                write_verification_status: Some(write_status),
            },
        )?
    };

    {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        queries::update_watermark_registry_fields(
            &conn,
            item.record_id,
            &response.watermark_id_issue_mode,
            &response.registry_status,
            Some(&response.registry_receipt),
            response.payload_protocol_version,
            response.payload_bytes_length,
            response.parent_watermark_uid.as_deref(),
            response.revision,
        )
        .map_err(|e| format!("回写版权编号登记状态失败: {e}"))?;
    }

    record.watermark_id_issue_mode = response.watermark_id_issue_mode;
    record.watermark_id_registry_status = response.registry_status;
    record.watermark_id_registry_receipt = Some(response.registry_receipt);
    record.payload_protocol_version = response.payload_protocol_version;
    record.payload_bytes_length = response.payload_bytes_length;
    record.parent_watermark_uid = response.parent_watermark_uid;
    record.revision = response.revision;
    Ok(vault_record_to_cloud_event(&record))
}

fn media_type_for_cloud_event(event: &CloudSyncEvent) -> String {
    match event
        .payload
        .get("kind")
        .and_then(|value| value.as_str())
        .unwrap_or_default()
    {
        "image" => "image",
        "audio" => "audio",
        "video" | "video_audio_track" => "video_audio_track",
        _ => "image",
    }
    .to_string()
}

fn prefixed_sha256_for_registry(value: &str) -> String {
    if value.trim().starts_with("sha256:") {
        value.trim().to_string()
    } else {
        format!("sha256:{}", value.trim())
    }
}

fn ensure_cloud_sync_entitled(profile: &DesktopCloudSyncProfile) -> Result<(), String> {
    if has_cloud_sync_entitlement(profile) {
        return Ok(());
    }
    Err("正式云同步从 Creator 开放，当前账户可继续本地使用".to_string())
}

fn ensure_cloud_video_processing_entitled(profile: &DesktopCloudSyncProfile) -> Result<(), String> {
    if profile
        .entitlement_features
        .get("cloud_video_processing")
        .and_then(|value| value.as_bool())
        == Some(true)
    {
        return Ok(());
    }
    Err("L3 视频画面盲水印对象领取需要 Studio / Enterprise 权益".to_string())
}

fn can_auto_cloud_sync(profile: &DesktopCloudSyncProfile) -> bool {
    has_cloud_sync_entitlement(profile) && profile.sync_policy == "auto_cloud_vault"
}

fn run_auto_cloud_sync_sequence(
    app_data_dir: &Path,
    state: &State<'_, AppState>,
    profile: DesktopCloudSyncProfile,
) {
    tokio::task::block_in_place(|| {
        let _ = pull_saved_cloud_changes_with_profile(app_data_dir, state, profile.clone());
        let _ = flush_cloud_queue_with_profile(state, &profile, 50);
        let _ = pull_saved_cloud_changes_with_profile(app_data_dir, state, profile);
    });
}

pub(crate) fn trigger_desktop_cloud_sync_after_local_enqueue(app_handle: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let Ok(app_data_dir) = app_handle.path().app_data_dir() else {
            return;
        };
        let state = app_handle.state::<AppState>();
        let Some(profile) = load_desktop_cloud_sync_profile(&app_data_dir) else {
            return;
        };
        let profile =
            match refresh_cloud_profile_snapshot_with_reauth(&app_data_dir, state.inner(), profile)
            {
                Ok(profile) => profile,
                Err(error) => {
                    log::warn!(
                        "cloud sync auto trigger skipped after snapshot refresh failed: {error}"
                    );
                    return;
                }
            };
        if can_auto_cloud_sync(&profile) {
            run_auto_cloud_sync_sequence(&app_data_dir, &state, profile);
        } else if !has_cloud_sync_entitlement(&profile) {
            let message =
                "正式云同步从 Creator 开放，当前账户以后端权益快照为准，已阻断后台自动上传";
            if let Ok(conn) = state.db.lock() {
                let _ = storage::mark_uploadable_cloud_sync_queue_blocked_by_entitlement(
                    &conn, message,
                );
            }
        }
    });
}

fn has_cloud_sync_entitlement(profile: &DesktopCloudSyncProfile) -> bool {
    profile
        .entitlement_features
        .get("cloud_sync")
        .and_then(|value| value.as_bool())
        == Some(true)
}

fn save_profile_entitlement_to_local_state(
    state: &State<'_, AppState>,
    profile: &DesktopCloudSyncProfile,
) -> Result<(), String> {
    let local_state = cloud_profile_to_local_entitlement(profile);
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    billing::save_entitlement_state(&conn, &local_state)
        .map_err(|e| format!("保存本地权益快照失败: {e}"))
}

fn cloud_profile_to_local_entitlement(profile: &DesktopCloudSyncProfile) -> EntitlementState {
    let now = chrono::Utc::now().to_rfc3339();
    EntitlementState {
        status: EntitlementStatus::from_db(&profile.entitlement_status),
        plan_name: Some(profile.entitlement_label.clone()),
        plan_code: profile.entitlement_plan_code.clone(),
        features: cloud_features_to_bool_map(&profile.entitlement_features),
        billing_source: None,
        subscription_id: None,
        trial_started_at: None,
        trial_ends_at: None,
        current_period_started_at: None,
        current_period_ends_at: None,
        grace_ends_at: None,
        last_checked_at: Some(now.clone()),
        updated_at: now,
    }
}

fn persist_report_purchase_grant(
    state: &State<'_, AppState>,
    grant: &ReportPurchaseGrant,
) -> Result<(), String> {
    let local_grant = billing::ReportPurchaseGrant {
        grant_id: grant.grant_id.clone(),
        account_id: grant.account_id.clone(),
        workspace_id: grant.workspace_id.clone(),
        creator_profile_id: grant.creator_profile_id.clone(),
        vault_record_id: grant.vault_record_id.clone(),
        product_code: grant.product_code.clone(),
        price_cents: grant.price_cents,
        currency: grant.currency.clone(),
        status: grant.status.clone(),
        granted_at: grant.granted_at.clone(),
        revoked_at: grant.revoked_at.clone(),
    };
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    billing::save_report_purchase_grant(&conn, &local_grant)
        .map_err(|e| format!("保存报告授权失败: {e}"))
}

fn normalize_report_product_code(product_code: &str) -> Result<String, String> {
    let value = product_code.trim().to_lowercase();
    if matches!(
        value.as_str(),
        "copyright_report_single" | "rights_evidence_pack_single"
    ) {
        return Ok(value);
    }
    Err("报告商品不在可购买范围内".to_string())
}

fn cloud_features_to_bool_map(features: &serde_json::Value) -> BTreeMap<String, bool> {
    let mut merged = billing::default_entitlement_features();
    if let Some(object) = features.as_object() {
        for (key, value) in object {
            merged.insert(key.clone(), value.as_bool().unwrap_or(false));
        }
    }
    merged
}

fn cloud_change_to_mobile_sync_item(
    change: &crate::sync::cloud::CloudSyncChange,
) -> Option<MobileSyncQueueItem> {
    let (operation, payload_type) = match change.entity_type.as_str() {
        "vaultRecord" => ("upsertVaultRecord", "vault_record"),
        "evidenceRecord" => ("upsertEvidenceRecord", "evidence_record"),
        _ => return None,
    };
    let entity = change.entity.as_object()?;
    let entity_id = entity
        .get("id")
        .and_then(|value| value.as_str())
        .map(str::to_owned)
        .filter(|value| !value.is_empty())?;
    let payload = serde_json::Value::Object(entity.clone());
    Some(MobileSyncQueueItem {
        queue_id: change
            .cursor
            .clone()
            .unwrap_or_else(|| format!("cloud-{entity_id}")),
        record_id: entity_id,
        operation: operation.to_string(),
        payload_type: payload_type.to_string(),
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::cloud::CloudSyncEventDisposition;

    fn profile_with_cloud_sync(enabled: bool) -> DesktopCloudSyncProfile {
        DesktopCloudSyncProfile {
            sync_policy: if enabled {
                "auto_cloud_vault".to_string()
            } else {
                "blocked_by_entitlement".to_string()
            },
            entitlement_features: serde_json::json!({
                "cloud_sync": enabled,
                "batch_processing": false,
                "cloud_video_processing": false
            }),
            ..DesktopCloudSyncProfile::default()
        }
    }

    #[test]
    fn cloud_sync_entitlement_allows_creator_feature() {
        assert!(ensure_cloud_sync_entitled(&profile_with_cloud_sync(true)).is_ok());
    }

    #[test]
    fn cloud_sync_entitlement_allows_manual_local_only_creator() {
        let mut profile = profile_with_cloud_sync(true);
        profile.sync_policy = "manual_local_only".to_string();
        assert!(ensure_cloud_sync_entitled(&profile).is_ok());
        assert!(!can_auto_cloud_sync(&profile));
    }

    #[test]
    fn cloud_sync_entitlement_rejects_free_feature() {
        let err = ensure_cloud_sync_entitled(&profile_with_cloud_sync(false)).unwrap_err();
        assert_eq!(err, "正式云同步从 Creator 开放，当前账户可继续本地使用");
    }

    #[test]
    fn cloud_sync_entitlement_rejects_missing_feature() {
        let profile = DesktopCloudSyncProfile::default();
        let err = ensure_cloud_sync_entitled(&profile).unwrap_err();
        assert_eq!(err, "正式云同步从 Creator 开放，当前账户可继续本地使用");
    }

    #[test]
    fn desktop_flush_event_results_keep_conflicts_failed() {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        storage::init_sync_storage(&conn).unwrap();
        let queue_ids = vec![
            "queue-accepted".to_string(),
            "queue-duplicate".to_string(),
            "queue-conflict".to_string(),
            "queue-rejected".to_string(),
        ];
        for queue_id in &queue_ids {
            storage::enqueue_cloud_sync_event(&conn, queue_id, 1, "{}").unwrap();
        }
        storage::mark_cloud_sync_queue_syncing(&conn, &queue_ids).unwrap();

        let parsed_events = vec![
            ParsedCloudQueueEvent {
                queue_id: "queue-accepted".to_string(),
                client_event_id: "evt-accepted".to_string(),
            },
            ParsedCloudQueueEvent {
                queue_id: "queue-duplicate".to_string(),
                client_event_id: "evt-duplicate".to_string(),
            },
            ParsedCloudQueueEvent {
                queue_id: "queue-conflict".to_string(),
                client_event_id: "evt-conflict".to_string(),
            },
            ParsedCloudQueueEvent {
                queue_id: "queue-rejected".to_string(),
                client_event_id: "evt-rejected".to_string(),
            },
        ];
        let batch = CloudSyncBatchResult {
            accepted: 2,
            accepted_event_ids: vec!["evt-accepted".to_string(), "evt-duplicate".to_string()],
            next_cursor: Some("cursor_2".to_string()),
            resolutions: serde_json::json!([]),
            event_results: Some(vec![
                CloudSyncEventDisposition {
                    client_event_id: "evt-accepted".to_string(),
                    disposition: "accepted".to_string(),
                    payload_hash: Some("sha256:accepted".to_string()),
                    entity_revision: Some(1),
                    message: None,
                },
                CloudSyncEventDisposition {
                    client_event_id: "evt-duplicate".to_string(),
                    disposition: "duplicate".to_string(),
                    payload_hash: Some("sha256:duplicate".to_string()),
                    entity_revision: Some(1),
                    message: None,
                },
                CloudSyncEventDisposition {
                    client_event_id: "evt-conflict".to_string(),
                    disposition: "conflict_payload_changed".to_string(),
                    payload_hash: Some("sha256:conflict".to_string()),
                    entity_revision: Some(2),
                    message: Some(
                        "same clientEventId was received with a different payload hash".to_string(),
                    ),
                },
                CloudSyncEventDisposition {
                    client_event_id: "evt-rejected".to_string(),
                    disposition: "rejected_invalid_event".to_string(),
                    payload_hash: None,
                    entity_revision: None,
                    message: Some(
                        "clientEventId, entityType and entityId are required".to_string(),
                    ),
                },
            ]),
        };

        let outcome = cloud_queue_batch_outcome(&parsed_events, &[], &batch);
        storage::mark_cloud_sync_queue_synced(&conn, &outcome.synced_queue_ids).unwrap();
        for failure in &outcome.failed_events {
            storage::mark_cloud_sync_queue_failed_structured(
                &conn,
                &[failure.queue_id.clone()],
                &failure.error,
                &failure.error_code,
                None,
            )
            .unwrap();
        }

        assert_eq!(
            storage::count_cloud_sync_queue_by_status(&conn, "synced").unwrap(),
            2
        );
        assert_eq!(
            storage::count_cloud_sync_queue_by_status(&conn, "failed").unwrap(),
            2
        );
        assert_queue_status_and_code(&conn, "queue-accepted", "synced", None);
        assert_queue_status_and_code(&conn, "queue-duplicate", "synced", None);
        assert_queue_status_and_code(
            &conn,
            "queue-conflict",
            "failed",
            Some("conflict_payload_changed"),
        );
        assert_queue_status_and_code(
            &conn,
            "queue-rejected",
            "failed",
            Some("rejected_invalid_event"),
        );
    }

    fn assert_queue_status_and_code(
        conn: &rusqlite::Connection,
        queue_id: &str,
        expected_status: &str,
        expected_error_code: Option<&str>,
    ) {
        let (status, error_code): (String, Option<String>) = conn
            .query_row(
                "SELECT status, last_error_code FROM cloud_sync_queue WHERE id = ?1",
                [queue_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(status, expected_status);
        assert_eq!(error_code.as_deref(), expected_error_code);
    }
}

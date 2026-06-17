use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::config;
use crate::db::queries;
use crate::identity;
use crate::sync::cloud::{
    clear_desktop_cloud_sync_profile, load_desktop_cloud_sync_profile,
    save_desktop_cloud_sync_profile, vault_record_to_cloud_event, CloudPullResult,
    CloudQueueFlushResult, CloudQueueStatus, CloudSyncBatchResult, CloudSyncChangesResult,
    CloudSyncClient, CloudSyncEvent, ContinueAccountCreatorProfile, ContinueAccountDevice,
    ContinueAccountRequest, DesktopCloudSyncProfile,
};
use crate::sync::storage::{self, MobileSyncQueueItem};
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
    pub verification_code: Option<String>,
    pub creator_display_name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushDesktopVaultRecordInput {
    pub base_url: String,
    pub access_token: String,
    pub device_id: String,
    pub record_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FetchCloudChangesInput {
    pub base_url: String,
    pub access_token: String,
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
    Ok(CloudQueueStatus {
        pending: storage::count_cloud_sync_queue_by_status(&conn, "pending")
            .map_err(|e| format!("读取云同步队列失败: {e}"))?,
        failed: storage::count_cloud_sync_queue_by_status(&conn, "failed")
            .map_err(|e| format!("读取云同步队列失败: {e}"))?,
        synced: storage::count_cloud_sync_queue_by_status(&conn, "synced")
            .map_err(|e| format!("读取云同步队列失败: {e}"))?,
        last_attempt_at: storage::latest_cloud_sync_queue_update_by_status(
            &conn,
            &["syncing", "synced", "failed"],
        )
        .map_err(|e| format!("读取云同步最近尝试时间失败: {e}"))?,
        last_success_at: storage::latest_cloud_sync_queue_update_by_status(&conn, &["synced"])
            .map_err(|e| format!("读取云同步最近成功时间失败: {e}"))?,
        last_failure_at: storage::latest_cloud_sync_queue_update_by_status(&conn, &["failed"])
            .map_err(|e| format!("读取云同步最近失败时间失败: {e}"))?,
        last_error: storage::latest_cloud_sync_queue_error(&conn)
            .map_err(|e| format!("读取云同步最近错误失败: {e}"))?,
    })
}

#[tauri::command]
pub fn sign_out_desktop_cloud(app_handle: AppHandle) -> Result<(), String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    clear_desktop_cloud_sync_profile(&app_data_dir)
}

#[tauri::command]
pub async fn continue_cloud_account(
    app_handle: AppHandle,
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
        .map(|value| format!("desktop-seed-{}", value.user_seed_hex))
        .unwrap_or_else(|| "desktop-seed-uninitialized".to_string());
    let device_id = local_identity
        .as_ref()
        .map(|value| format!("desktop-{}", value.device_id_hex))
        .unwrap_or_else(|| format!("desktop-{}", hex::encode(identity::compute_device_id())));
    let device_name = hostname::get()
        .ok()
        .and_then(|name| name.into_string().ok())
        .unwrap_or_else(|| "HiddenShield Desktop".to_string());

    let request = ContinueAccountRequest {
        identifier: input.identifier.trim().to_string(),
        verification_code: input.verification_code.unwrap_or_default(),
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

    let session = CloudSyncClient::new(&cloud_base_url)?.continue_account(&request)?;
    let profile = DesktopCloudSyncProfile::from_session(&cloud_base_url, session);
    save_desktop_cloud_sync_profile(&app_data_dir, &profile)?;
    Ok(profile)
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
        .ok_or_else(|| "尚未继续使用 HiddenShield 账户".to_string())?;
    enqueue_desktop_record_for_cloud(&state, input.record_id)?;
    let flush = flush_cloud_queue_with_profile(&state, &profile, 50)?;
    Ok(CloudSyncBatchResult {
        accepted: flush.synced,
        accepted_event_ids: Vec::new(),
        next_cursor: None,
        resolutions: serde_json::json!([]),
    })
}

#[tauri::command]
pub async fn fetch_cloud_changes(
    input: FetchCloudChangesInput,
) -> Result<CloudSyncChangesResult, String> {
    CloudSyncClient::new(input.base_url)?
        .fetch_changes(&input.access_token, input.cursor.as_deref())
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
    let mut profile = load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "尚未继续使用 HiddenShield 账户".to_string())?;
    let changes = CloudSyncClient::new(&profile.cloud_base_url)?
        .fetch_changes(&profile.access_token, profile.last_remote_cursor.as_deref())?;

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
        .ok_or_else(|| "尚未继续使用 HiddenShield 账户".to_string())?;
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

fn flush_cloud_queue_with_profile(
    state: &State<'_, AppState>,
    profile: &DesktopCloudSyncProfile,
    limit: u32,
) -> Result<CloudQueueFlushResult, String> {
    let limit = limit.clamp(1, 100) as usize;
    let queued = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
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

    let mut events = Vec::new();
    let mut parsed_queue_ids = Vec::new();
    let mut parse_failed_ids = Vec::new();
    for item in &queued {
        match serde_json::from_str::<CloudSyncEvent>(&item.event_json) {
            Ok(event) => {
                parsed_queue_ids.push(item.id.clone());
                events.push(event);
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

    let result = CloudSyncClient::new(&profile.cloud_base_url)?.send_events_batch(
        &profile.access_token,
        &profile.device_id,
        events,
    );
    match result {
        Ok(batch) => {
            let accepted = batch.accepted_event_ids;
            let accepted_ids = if accepted.is_empty() {
                parsed_queue_ids.clone()
            } else {
                accepted
            };
            let mut failed_ids = parsed_queue_ids
                .iter()
                .filter(|id| !accepted_ids.contains(id))
                .cloned()
                .collect::<Vec<_>>();
            failed_ids.extend(parse_failed_ids);
            let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
            storage::mark_cloud_sync_queue_synced(&conn, &accepted_ids)
                .map_err(|e| format!("更新云同步队列失败: {e}"))?;
            if !failed_ids.is_empty() {
                storage::mark_cloud_sync_queue_failed(&conn, &failed_ids, "云端未接收该事件")
                    .map_err(|e| format!("更新云同步队列失败: {e}"))?;
            }
            Ok(CloudQueueFlushResult {
                attempted: queue_ids.len() as u32,
                synced: accepted_ids.len() as u32,
                failed: failed_ids.len() as u32,
                message: format!(
                    "已同步 {} 条，失败 {} 条",
                    accepted_ids.len(),
                    failed_ids.len()
                ),
            })
        }
        Err(error) => {
            let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
            storage::mark_cloud_sync_queue_failed(&conn, &queue_ids, &error)
                .map_err(|e| format!("更新云同步队列失败: {e}"))?;
            Ok(CloudQueueFlushResult {
                attempted: queue_ids.len() as u32,
                synced: 0,
                failed: queue_ids.len() as u32,
                message: error,
            })
        }
    }
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

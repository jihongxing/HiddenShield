use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::sync::storage;
use crate::AppState;

const MOBILE_SYNC_PORT: u16 = 47219;

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

#[tauri::command]
pub fn get_mobile_sync_status(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<MobileSyncStatus, String> {
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
        listen_port: MOBILE_SYNC_PORT,
        listen_address: format!("http://0.0.0.0:{MOBILE_SYNC_PORT}"),
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

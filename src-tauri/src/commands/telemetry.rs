use tauri::{AppHandle, Manager};

use crate::telemetry::{self, DataUsageInfo};

/// Get whether telemetry is currently enabled.
#[tauri::command]
pub fn get_telemetry_enabled() -> bool {
    telemetry::is_enabled()
}

/// Set telemetry enabled/disabled state.
#[tauri::command]
pub fn set_telemetry_enabled(app_handle: AppHandle, enabled: bool) {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");
    telemetry::set_enabled(&app_data_dir, enabled);
}

/// Check if the user has acknowledged the telemetry notice.
#[tauri::command]
pub fn get_telemetry_acknowledged(app_handle: AppHandle) -> bool {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");
    telemetry::is_acknowledged(&app_data_dir)
}

/// Mark the telemetry notice as acknowledged.
#[tauri::command]
pub fn acknowledge_telemetry(app_handle: AppHandle) {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");
    telemetry::set_acknowledged(&app_data_dir);
}

/// Export crash log contents.
#[tauri::command]
pub fn export_crash_log(app_handle: AppHandle) -> String {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");
    telemetry::read_crash_log(&app_data_dir)
}

/// Get data usage breakdown.
#[tauri::command]
pub fn get_data_usage(app_handle: AppHandle) -> DataUsageInfo {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");
    telemetry::calculate_data_usage(&app_data_dir)
}

/// Clear all application data (FFmpeg cache, database, logs).
#[tauri::command]
pub fn clear_all_data(app_handle: AppHandle) -> Result<String, String> {
    let state = app_handle.state::<crate::AppState>();
    let active = state.active_pipelines.lock().unwrap_or_else(|e| e.into_inner());
    if !active.is_empty() {
        return Err("有任务正在处理中，请等待完成后再清理数据".into());
    }
    drop(active);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");

    // Remove FFmpeg binaries
    let _ = std::fs::remove_file(app_data_dir.join("ffmpeg"));
    let _ = std::fs::remove_file(app_data_dir.join("ffmpeg.exe"));
    let _ = std::fs::remove_file(app_data_dir.join("ffprobe"));
    let _ = std::fs::remove_file(app_data_dir.join("ffprobe.exe"));

    // Remove database
    let _ = std::fs::remove_file(app_data_dir.join("vault.db"));

    // Remove logs directory
    let _ = std::fs::remove_dir_all(app_data_dir.join("logs"));

    // Remove telemetry config
    let _ = std::fs::remove_file(app_data_dir.join("telemetry_config.json"));

    Ok("所有数据已清除，可安全卸载".into())
}

/// Clear only cache data (FFmpeg binaries + logs), preserve vault database.
#[tauri::command]
pub fn clear_cache_only(app_handle: AppHandle) -> Result<String, String> {
    let state = app_handle.state::<crate::AppState>();
    let active = state.active_pipelines.lock().unwrap_or_else(|e| e.into_inner());
    if !active.is_empty() {
        return Err("有任务正在处理中，请等待完成后再清理数据".into());
    }
    drop(active);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");

    // Remove FFmpeg binaries
    let _ = std::fs::remove_file(app_data_dir.join("ffmpeg"));
    let _ = std::fs::remove_file(app_data_dir.join("ffmpeg.exe"));
    let _ = std::fs::remove_file(app_data_dir.join("ffprobe"));
    let _ = std::fs::remove_file(app_data_dir.join("ffprobe.exe"));

    // Remove logs directory
    let _ = std::fs::remove_dir_all(app_data_dir.join("logs"));

    Ok("缓存已清除，版权库数据已保留".into())
}

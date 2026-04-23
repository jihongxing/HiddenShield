use tauri::{AppHandle, Manager};

use rusqlite::Connection;

use crate::db::queries;
use crate::telemetry::{self, DataUsageInfo};

fn remove_file_if_exists(path: &std::path::Path) -> Result<(), String> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("删除文件失败 {}: {e}", path.display())),
    }
}

fn remove_dir_if_exists(path: &std::path::Path) -> Result<(), String> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("删除目录失败 {}: {e}", path.display())),
    }
}

fn detach_database_connection(state: &crate::AppState) -> Result<(), String> {
    let mut conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let placeholder =
        Connection::open_in_memory().map_err(|e| format!("创建临时数据库连接失败: {e}"))?;
    let old_conn = std::mem::replace(&mut *conn, placeholder);
    drop(old_conn);
    Ok(())
}

fn reopen_database_connection(
    state: &crate::AppState,
    db_path: &std::path::Path,
) -> Result<(), String> {
    let mut conn = state.db.lock().unwrap_or_else(|e| e.into_inner());
    let fresh_conn = Connection::open(db_path).map_err(|e| format!("重建版权库失败: {e}"))?;
    queries::init_db(&fresh_conn).map_err(|e| format!("初始化版权库失败: {e}"))?;
    *conn = fresh_conn;
    Ok(())
}

struct DetachedDatabaseSession<'a> {
    state: &'a crate::AppState,
    db_path: std::path::PathBuf,
    reopened: bool,
}

impl<'a> DetachedDatabaseSession<'a> {
    fn begin(state: &'a crate::AppState, db_path: &std::path::Path) -> Result<Self, String> {
        detach_database_connection(state)?;
        Ok(Self {
            state,
            db_path: db_path.to_path_buf(),
            reopened: false,
        })
    }

    fn reopen(mut self) -> Result<(), String> {
        let result = reopen_database_connection(self.state, &self.db_path);
        if result.is_ok() {
            self.reopened = true;
        }
        result
    }
}

impl Drop for DetachedDatabaseSession<'_> {
    fn drop(&mut self) {
        if self.reopened {
            return;
        }

        if let Err(err) = reopen_database_connection(self.state, &self.db_path) {
            log::error!(
                "Failed to restore vault database connection after clear_all_data path: {}",
                err
            );
        } else {
            self.reopened = true;
        }
    }
}

fn clear_persisted_app_data(
    app_data_dir: &std::path::Path,
    db_path: &std::path::Path,
) -> Result<(), String> {
    remove_file_if_exists(db_path)?;
    remove_file_if_exists(&app_data_dir.join("vault.db-wal"))?;
    remove_file_if_exists(&app_data_dir.join("vault.db-shm"))?;
    remove_dir_if_exists(&app_data_dir.join("logs"))?;
    remove_dir_if_exists(&app_data_dir.join("tsa_tokens"))?;
    remove_file_if_exists(&app_data_dir.join("identity.json"))?;
    remove_file_if_exists(&app_data_dir.join("telemetry_config.json"))?;
    remove_file_if_exists(&app_data_dir.join("ffmpeg"))?;
    remove_file_if_exists(&app_data_dir.join("ffmpeg.exe"))?;
    remove_file_if_exists(&app_data_dir.join("ffprobe"))?;
    remove_file_if_exists(&app_data_dir.join("ffprobe.exe"))?;
    Ok(())
}

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

/// Get whether optional network features are enabled.
#[tauri::command]
pub fn get_network_enabled(app_handle: AppHandle) -> bool {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");
    telemetry::is_network_enabled(&app_data_dir)
}

/// Set whether optional network features are enabled.
#[tauri::command]
pub fn set_network_enabled(app_handle: AppHandle, enabled: bool) {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");
    telemetry::set_network_enabled(&app_data_dir, enabled);
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
    let active = state
        .active_pipelines
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if !active.is_empty() {
        return Err("有任务正在处理中，请等待完成后再清理数据".into());
    }
    drop(active);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");
    let db_path = app_data_dir.join("vault.db");

    state.clear_runtime_caches();
    let detached_db = DetachedDatabaseSession::begin(&state, &db_path)?;
    let clear_result = clear_persisted_app_data(&app_data_dir, &db_path);
    let reopen_result = detached_db.reopen();

    if reopen_result.is_ok() {
        telemetry::init(&app_data_dir);
    }

    match (clear_result, reopen_result) {
        (Ok(()), Ok(())) => Ok("所有数据已清除，应用已重置为首次启动状态。".into()),
        (Err(clear_err), Ok(())) => Err(clear_err),
        (Ok(()), Err(reopen_err)) => Err(format!(
            "{reopen_err}。应用已尝试恢复数据库连接但未成功，请重启应用后重试。"
        )),
        (Err(clear_err), Err(reopen_err)) => Err(format!(
            "{clear_err}；同时恢复数据库连接失败: {reopen_err}。请重启应用后重试。"
        )),
    }
}

/// Clear only cache data (FFmpeg binaries + logs), preserve vault database.
#[tauri::command]
pub fn clear_cache_only(app_handle: AppHandle) -> Result<String, String> {
    let state = app_handle.state::<crate::AppState>();
    let active = state
        .active_pipelines
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if !active.is_empty() {
        return Err("有任务正在处理中，请等待完成后再清理数据".into());
    }
    drop(active);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .expect("failed to resolve app data directory");

    state.clear_runtime_caches();
    remove_file_if_exists(&app_data_dir.join("ffmpeg"))?;
    remove_file_if_exists(&app_data_dir.join("ffmpeg.exe"))?;
    remove_file_if_exists(&app_data_dir.join("ffprobe"))?;
    remove_file_if_exists(&app_data_dir.join("ffprobe.exe"))?;
    remove_dir_if_exists(&app_data_dir.join("logs"))?;
    remove_dir_if_exists(&app_data_dir.join("tsa_tokens"))?;

    Ok("缓存已清除，运行时探测结果已重置。".into())
}

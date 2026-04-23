mod commands;
mod db;
mod encoder;
pub mod identity;
mod pipeline;
mod telemetry;
pub mod tsa;
mod utils;

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use rusqlite::Connection;
use tauri::Manager;
use tokio::sync::Semaphore;

use crate::encoder::hw_detect::DetectedHardware;
use crate::pipeline::FfmpegPaths;

/// Maximum concurrent FFmpeg processes. Limits resource usage to prevent OOM.
/// Set to half the available CPU cores (minimum 2).
fn max_concurrent_ffmpeg() -> usize {
    let cpus = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    (cpus / 2).max(2)
}

/// Maximum concurrent GPU encode sessions.
/// Consumer GPUs (NVENC) typically support 3-5 concurrent sessions,
/// but running more than 1-2 causes severe contention. Keep it serial.
const MAX_HW_ENCODE_CONCURRENT: usize = 1;

pub struct AppState {
    pub active_pipelines: Mutex<HashSet<String>>,
    pub db: Mutex<Connection>,
    pub ffmpeg_paths: Mutex<Option<FfmpegPaths>>,
    pub hw_info: Mutex<Option<DetectedHardware>>,
    /// Global semaphore limiting concurrent FFmpeg processes (CPU) to prevent OOM.
    pub ffmpeg_semaphore: Semaphore,
    /// Dedicated semaphore for GPU hardware encoding sessions.
    /// Consumer GPUs can't handle multiple concurrent encode sessions well.
    pub hw_encode_semaphore: Semaphore,
    /// Global sleep inhibitor: held as long as any task is active.
    /// Uses reference counting: acquired when count goes 0→1, released when 1→0.
    pub sleep_lock: Mutex<Option<pipeline::system_guard::SleepInhibitor>>,
    pub active_task_count: AtomicUsize,
}

impl AppState {
    pub fn get_ffmpeg_paths(&self) -> Option<FfmpegPaths> {
        self.ffmpeg_paths
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_ffmpeg_paths(&self, paths: FfmpegPaths) {
        let mut lock = self.ffmpeg_paths.lock().unwrap_or_else(|e| e.into_inner());
        *lock = Some(paths);
    }

    pub fn get_hw_info(&self) -> Option<DetectedHardware> {
        self.hw_info
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_hw_info(&self, hw: DetectedHardware) {
        let mut lock = self.hw_info.lock().unwrap_or_else(|e| e.into_inner());
        *lock = Some(hw);
    }

    pub fn clear_runtime_caches(&self) {
        let mut ffmpeg_paths = self.ffmpeg_paths.lock().unwrap_or_else(|e| e.into_inner());
        *ffmpeg_paths = None;

        let mut hw_info = self.hw_info.lock().unwrap_or_else(|e| e.into_inner());
        *hw_info = None;
    }

    /// Increment active task count. Acquires sleep lock on first task.
    pub fn acquire_sleep_lock(&self) {
        let prev = self.active_task_count.fetch_add(1, Ordering::SeqCst);
        if prev == 0 {
            // First task — acquire system sleep inhibitor
            match pipeline::system_guard::inhibit_sleep("HiddenShield 任务处理中") {
                Ok(guard) => {
                    let mut lock = self.sleep_lock.lock().unwrap_or_else(|e| e.into_inner());
                    *lock = Some(guard);
                }
                Err(e) => {
                    log::warn!("Failed to acquire sleep lock: {e}");
                }
            }
        }
    }

    /// Decrement active task count. Releases sleep lock when all tasks complete.
    pub fn release_sleep_lock(&self) {
        let prev = self.active_task_count.fetch_sub(1, Ordering::SeqCst);
        if prev == 1 {
            // Last task finished — release system sleep inhibitor
            let mut lock = self.sleep_lock.lock().unwrap_or_else(|e| e.into_inner());
            *lock = None; // Drop the SleepInhibitor → releases the OS lock
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            commands::identity::get_identity_status,
            commands::identity::setup_identity,
            commands::probe::probe_source,
            commands::probe::system_check,
            commands::transcode::start_pipeline,
            commands::transcode::cancel_pipeline,
            commands::transcode::check_active_pipelines,
            commands::transcode::get_hw_info,
            commands::transcode::open_output_dir,
            commands::vault::list_vault_records,
            commands::vault::check_files_exist,
            commands::verify::verify_suspect,
            commands::telemetry::get_telemetry_enabled,
            commands::telemetry::set_telemetry_enabled,
            commands::telemetry::get_telemetry_acknowledged,
            commands::telemetry::acknowledge_telemetry,
            commands::telemetry::get_network_enabled,
            commands::telemetry::set_network_enabled,
            commands::telemetry::export_crash_log,
            commands::telemetry::get_data_usage,
            commands::telemetry::clear_all_data,
            commands::telemetry::clear_cache_only
        ])
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }

            // Initialize SQLite database in the app data directory.
            let app_data_dir = app
                .path()
                .app_data_dir()
                .expect("failed to resolve app data directory");
            std::fs::create_dir_all(&app_data_dir)?;

            // Initialize telemetry and install panic hook
            telemetry::init(&app_data_dir);
            telemetry::install_panic_hook(app_data_dir.clone());

            let db_path = app_data_dir.join("vault.db");
            let conn = Connection::open(&db_path)
                .map_err(|e| format!("failed to open SQLite database: {e}"))?;

            db::queries::init_db(&conn)
                .map_err(|e| format!("failed to initialize database: {e}"))?;

            app.manage(AppState {
                active_pipelines: Mutex::new(HashSet::new()),
                db: Mutex::new(conn),
                ffmpeg_paths: Mutex::new(None),
                hw_info: Mutex::new(None),
                ffmpeg_semaphore: Semaphore::new(max_concurrent_ffmpeg()),
                hw_encode_semaphore: Semaphore::new(MAX_HW_ENCODE_CONCURRENT),
                sleep_lock: Mutex::new(None),
                active_task_count: AtomicUsize::new(0),
            });

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

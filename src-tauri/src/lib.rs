pub mod cloud_sync_runtime_qa;
pub mod commands;
mod config;
mod db;
pub mod desktop_offline_release_gate;
mod encoder;
pub mod entitlements;
pub mod identity;
pub mod offline_license;
mod pipeline;
mod report_pdf;
mod sync;
mod telemetry;
pub mod tsa;
mod utils;
mod video_fingerprint;

use std::collections::HashSet;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use rusqlite::Connection;
use tauri::Manager;
use tokio::sync::Semaphore;

use crate::db::offline_license::{InstallationSecretStore, OsKeyringInstallationSecretStore};
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
    pub installation_secret_store: Arc<dyn InstallationSecretStore>,
    pub ffmpeg_paths: Mutex<Option<FfmpegPaths>>,
    pub ffmpeg_version: Mutex<Option<String>>,
    pub hw_info: Mutex<Option<DetectedHardware>>,
    pub report_export_lock: Mutex<()>,
    pub report_pdf_worker: Mutex<report_pdf::ReportPdfWorkerManager>,
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
    pub fn new(conn: Connection) -> Self {
        Self::new_with_installation_secret_store(conn, Arc::new(OsKeyringInstallationSecretStore))
    }

    pub fn new_with_installation_secret_store(
        conn: Connection,
        installation_secret_store: Arc<dyn InstallationSecretStore>,
    ) -> Self {
        Self {
            active_pipelines: Mutex::new(HashSet::new()),
            db: Mutex::new(conn),
            installation_secret_store,
            ffmpeg_paths: Mutex::new(None),
            ffmpeg_version: Mutex::new(None),
            hw_info: Mutex::new(None),
            report_export_lock: Mutex::new(()),
            report_pdf_worker: Mutex::new(report_pdf::ReportPdfWorkerManager::default()),
            ffmpeg_semaphore: Semaphore::new(max_concurrent_ffmpeg()),
            hw_encode_semaphore: Semaphore::new(MAX_HW_ENCODE_CONCURRENT),
            sleep_lock: Mutex::new(None),
            active_task_count: AtomicUsize::new(0),
        }
    }

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

    pub fn get_ffmpeg_version(&self) -> Option<String> {
        self.ffmpeg_version
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone()
    }

    pub fn set_ffmpeg_version(&self, version: String) {
        let mut lock = self
            .ffmpeg_version
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *lock = Some(version);
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

        let mut ffmpeg_version = self
            .ffmpeg_version
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        *ffmpeg_version = None;

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
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::CloseRequested { .. }) {
                let app_handle = window.app_handle();
                if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
                    if telemetry::is_enabled()
                        && telemetry::is_acknowledged(&app_data_dir)
                        && telemetry::is_network_enabled(&app_data_dir)
                    {
                        let _ = telemetry::anonymous::flush_queue(&app_data_dir);
                    }
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::identity::get_identity_status,
            commands::identity::setup_identity,
            commands::probe::probe_source,
            commands::probe::system_check,
            commands::preferences::get_preferences,
            commands::preferences::save_preferences,
            commands::transcode::start_pipeline,
            commands::transcode::cancel_pipeline,
            commands::transcode::check_active_pipelines,
            commands::transcode::get_hw_info,
            commands::transcode::open_output_dir,
            commands::vault::list_vault_records,
            commands::vault::check_files_exist,
            commands::vault::supplement_vault_trusted_time,
            commands::vault::repair_watermark_record_reissue,
            commands::verify::inspect_rewrite_target,
            commands::verify::verify_suspect,
            commands::verify::verify_suspect_readonly_candidate,
            commands::billing::get_entitlement_state,
            commands::billing::get_usage_ledger_summary,
            commands::billing::record_usage_event,
            commands::offline_license::get_offline_license_status,
            commands::offline_license::export_offline_activation_request,
            commands::offline_license::import_offline_license,
            commands::offline_license::clear_offline_license,
            commands::offline_license::import_offline_revocation_list,
            commands::report::export_vault_batch_summary_report,
            commands::report::export_vault_formal_report,
            commands::report::import_mobile_report_handoff,
            commands::report::verify_formal_report_bundle,
            commands::report::verify_rights_evidence_pack,
            commands::public_metadata::export_public_rights_embedded_image,
            commands::local_batch::list_local_batch_jobs,
            commands::local_batch::save_local_batch_job,
            commands::telemetry::get_telemetry_enabled,
            commands::telemetry::set_telemetry_enabled,
            commands::telemetry::get_telemetry_acknowledged,
            commands::telemetry::acknowledge_telemetry,
            commands::telemetry::get_network_enabled,
            commands::telemetry::set_network_enabled,
            commands::telemetry::export_crash_log,
            commands::telemetry::get_data_usage,
            commands::telemetry::get_anonymous_feedback_status,
            commands::telemetry::flush_anonymous_feedback_queue,
            commands::telemetry::clear_all_data,
            commands::telemetry::clear_cache_only,
            commands::sync::get_mobile_sync_status,
            commands::sync::regenerate_mobile_pairing_code,
            commands::sync::get_desktop_cloud_sync_profile,
            commands::sync::get_desktop_cloud_queue_status,
            commands::sync::sign_out_desktop_cloud,
            commands::sync::create_desktop_auth_challenge,
            commands::sync::continue_cloud_account,
            commands::sync::set_desktop_cloud_auto_sync_enabled,
            commands::sync::list_desktop_cloud_devices,
            commands::sync::update_desktop_cloud_device_name,
            commands::sync::revoke_desktop_cloud_device,
            commands::sync::refresh_desktop_auth_session,
            commands::sync::refresh_desktop_cloud_account_snapshot,
            commands::sync::create_billing_payment_session,
            commands::sync::create_report_purchase_session,
            commands::sync::refresh_billing_entitlement,
            commands::sync::get_billing_payment_session_status,
            commands::sync::get_report_purchase_session_status,
            commands::sync::reconcile_billing_payment_session,
            commands::sync::reconcile_report_purchase_session,
            commands::sync::push_desktop_vault_record_to_cloud,
            commands::sync::push_saved_desktop_vault_record_to_cloud,
            commands::sync::flush_desktop_cloud_sync_queue,
            commands::sync::generate_video_fingerprint_bundle,
            commands::sync::create_video_fingerprint_notary,
            commands::sync::create_video_fingerprint_notary_from_bundle_file,
            commands::sync::create_l3_video_visual_upload_task,
            commands::sync::save_l3_video_visual_task_to_vault,
            commands::sync::fetch_cloud_changes,
            commands::sync::pull_saved_cloud_changes_into_desktop
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

            sync::storage::init_sync_storage(&conn)
                .map_err(|e| format!("failed to initialize sync database: {e}"))?;

            app.manage(AppState::new(conn));

            let report_worker_app = app.handle().clone();
            std::thread::spawn(move || {
                let state = report_worker_app.state::<AppState>();
                let result = state
                    .report_pdf_worker
                    .lock()
                    .map_err(|error| format!("Chromium report worker lock failed: {error}"))
                    .and_then(|mut worker| worker.prewarm(&report_worker_app));
                if let Err(error) = result {
                    log::warn!("Failed to prewarm Chromium report worker: {error}");
                }
            });

            sync::start_sync_server(app.handle().clone());

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

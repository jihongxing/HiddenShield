use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::encoder::hw_detect;
use crate::entitlements;
use crate::pipeline::ffmpeg;
use crate::pipeline::progress;
use crate::pipeline::scheduler::{self, PipelineParams};
use crate::telemetry;
use crate::AppState;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Eq, PartialEq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum Platform {
    Douyin,
    Bilibili,
    Xiaohongshu,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AspectStrategy {
    Letterbox,
    SmartCrop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EncodingMode {
    FastGpu,
    HighQualityCpu,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AIContentOptions {
    pub work_source_declaration: String,
    pub training_permission_declaration: String,
    pub creation_method_declaration: String,
    pub human_edit_level_declaration: String,
    pub authenticity_claim_declaration: String,
    pub custom_rights_statement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TranscodeOptions {
    pub aspect_strategy: AspectStrategy,
    pub encoding_mode: EncodingMode,
    pub ai_content: Option<AIContentOptions>,
    pub allow_rewrite: bool,
    pub rewrite_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HardwareInfo {
    pub preferred_encoder: String,
    pub available_encoders: Vec<String>,
    pub tone_mapping_supported: bool,
    pub ffmpeg_status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStartResult {
    pub pipeline_id: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineProgressPayload {
    pub pipeline_id: String,
    pub stage: String,
    pub percent: u8,
    pub platform_percents: progress::PlatformPercents,
}

// ---------------------------------------------------------------------------
// Helper: ensure FFmpeg paths are available from the system PATH
// ---------------------------------------------------------------------------

async fn ensure_ffmpeg_paths(app_handle: &AppHandle) -> Result<ffmpeg::FfmpegPaths, String> {
    let state = app_handle.state::<AppState>();

    // Fast path: already cached
    if let Some(paths) = state.get_ffmpeg_paths() {
        return Ok(paths.clone());
    }

    let _ = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;

    let paths = ffmpeg::detect_ffmpeg()
        .await
        .map_err(|e| format!("FFmpeg 不可用，请先手动安装并加入 PATH：{e}"))?;
    state.set_ffmpeg_paths(paths.clone());
    Ok(paths)
}

// ---------------------------------------------------------------------------
// Helper: ensure hardware info is available
// ---------------------------------------------------------------------------

async fn ensure_hw_info(
    app_handle: &AppHandle,
    ffmpeg_paths: &ffmpeg::FfmpegPaths,
) -> hw_detect::DetectedHardware {
    let state = app_handle.state::<AppState>();

    if let Some(info) = state.get_hw_info() {
        return info.clone();
    }

    let detected = hw_detect::detect_hardware(&ffmpeg_paths.ffmpeg).await;
    state.set_hw_info(detected.clone());
    detected
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn get_hw_info(app_handle: AppHandle) -> Result<HardwareInfo, String> {
    let ffmpeg_paths = ensure_ffmpeg_paths(&app_handle).await;

    let (hw, ffmpeg_status) = match ffmpeg_paths {
        Ok(paths) => {
            let hw = ensure_hw_info(&app_handle, &paths).await;
            (hw, "detected".to_string())
        }
        Err(e) => {
            // FFmpeg not available — return software fallback info
            let fallback = hw_detect::DetectedHardware {
                preferred_encoder: "libx264".to_string(),
                available_encoders: vec!["libx264".to_string(), "libx265".to_string()],
                hw_type: hw_detect::HwEncoderType::Software,
            };
            (fallback, format!("unavailable: {e}"))
        }
    };

    Ok(HardwareInfo {
        preferred_encoder: hw.preferred_encoder,
        available_encoders: hw.available_encoders,
        tone_mapping_supported: true,
        ffmpeg_status,
    })
}

#[tauri::command]
pub async fn start_pipeline(
    input_path: String,
    platforms: Vec<Platform>,
    options: TranscodeOptions,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<PipelineStartResult, String> {
    if input_path.trim().is_empty() {
        return Err("input path cannot be empty".to_string());
    }
    ensure_local_batch_execution_entitled(&state, &input_path)?;

    // Platform-specific transcode presets are no longer part of the formal
    // HiddenShield write flow. Video now creates one L1 audio-track protected copy.
    let file_type = scheduler::classify_file(std::path::Path::new(&input_path));
    let is_media_only = file_type != scheduler::FileType::Video;

    let input_size_bytes = std::fs::metadata(&input_path)
        .map(|meta| meta.len())
        .unwrap_or(0);

    let pipeline_id = format!(
        "pipe-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|err| err.to_string())?
            .as_millis()
    );

    let ffmpeg_paths = if file_type == scheduler::FileType::Image {
        None
    } else {
        Some(ensure_ffmpeg_paths(&app_handle).await?)
    };
    let hw_info = if let Some(ref paths) = ffmpeg_paths {
        Some(ensure_hw_info(&app_handle, paths).await)
    } else {
        None
    };

    {
        let mut active = state
            .active_pipelines
            .lock()
            .map_err(|err| err.to_string())?;
        active.insert(pipeline_id.clone());
    }

    let summary = if is_media_only {
        let type_label = if file_type == scheduler::FileType::Image {
            "图片"
        } else {
            "音频"
        };
        format!("已创建{}水印嵌入任务", type_label)
    } else {
        format!(
            "已创建视频音轨保护副本任务（{}）",
            hw_info
                .as_ref()
                .map(|info| info.preferred_encoder.as_str())
                .unwrap_or("software"),
        )
    };

    let params = PipelineParams {
        input_path: std::path::PathBuf::from(&input_path),
        platforms: platforms.clone(),
        options,
        ffmpeg_paths,
        hw_info,
        pipeline_id: pipeline_id.clone(),
    };

    let app_handle_clone = app_handle.clone();
    let pipeline_id_clone = pipeline_id.clone();

    tauri::async_runtime::spawn(async move {
        let state = app_handle_clone.state::<AppState>();
        let result = scheduler::run_pipeline(params, app_handle_clone.clone(), &state.db).await;

        // Remove from active set on completion
        if let Ok(mut active) = state.active_pipelines.lock() {
            active.remove(&pipeline_id_clone);
        }

        if let Err(e) = result {
            // Don't emit failure for user-initiated cancellation
            if !matches!(e, crate::pipeline::error::PipelineError::Cancelled) {
                log::error!("Pipeline {pipeline_id_clone} failed: {e}");
                if let Ok(app_data_dir) = app_handle_clone.path().app_data_dir() {
                    telemetry::anonymous::record_failure_event(
                        &app_data_dir,
                        if is_media_only {
                            if file_type == scheduler::FileType::Image {
                                "watermark_image"
                            } else {
                                "watermark_audio"
                            }
                        } else {
                            "watermark_video"
                        },
                        if file_type == scheduler::FileType::Image {
                            "image"
                        } else if file_type == scheduler::FileType::Audio {
                            "audio"
                        } else {
                            "video"
                        },
                        input_size_bytes,
                        None,
                        e.watermark_code()
                            .map(ToString::to_string)
                            .unwrap_or_else(|| e.to_string()),
                        Some(pipeline_id_clone.clone()),
                    );
                }
                let _ = app_handle_clone.emit(
                    "pipeline-progress",
                    PipelineProgressPayload {
                        pipeline_id: pipeline_id_clone,
                        stage: pipeline_failure_stage(&e),
                        percent: 0,
                        platform_percents: progress::PlatformPercents::new(),
                    },
                );
            }
        }
    });

    Ok(PipelineStartResult {
        pipeline_id,
        summary,
    })
}

fn ensure_local_batch_execution_entitled(state: &AppState, input_path: &str) -> Result<(), String> {
    let conn = state
        .db
        .lock()
        .map_err(|error| format!("db lock error: {error}"))?;
    let is_running_batch_item: bool = conn
        .query_row(
            "SELECT EXISTS(
                SELECT 1 FROM local_batch_items
                WHERE input_ref = ?1 AND status IN ('queued', 'running')
            )",
            [input_path],
            |row| row.get(0),
        )
        .map_err(|error| format!("读取本地批量队列失败: {error}"))?;
    if !is_running_batch_item {
        return Ok(());
    }
    let entitlement = entitlements::resolve_effective_entitlement(
        &conn,
        state.installation_secret_store.as_ref(),
    )
    .map_err(|error| format!("读取权益状态失败: {error}"))?;
    if entitlement.features.get("batch_processing") == Some(&true) {
        return Ok(());
    }
    Err("本地批量处理从 Creator 开放".to_string())
}

#[cfg(test)]
mod entitlement_tests {
    use rusqlite::Connection;

    use super::*;
    use crate::db::billing::{self, EntitlementState, EntitlementStatus};
    use crate::db::offline_license::MemoryInstallationSecretStore;
    use crate::db::schema;

    #[test]
    fn queued_local_batch_execution_is_denied_without_effective_entitlement() {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();
        conn.execute(
            "INSERT INTO local_batch_jobs (
                id, status, created_at, updated_at, entitlement_plan_code, entitlement_status
             ) VALUES ('batch-k2', 'queued', '2026-07-15T00:00:00Z',
                       '2026-07-15T00:00:00Z', 'free', 'free')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO local_batch_items (
                id, job_id, input_ref, file_name, media_kind, status, attempts,
                created_at, updated_at
             ) VALUES ('item-k2', 'batch-k2', 'C:/media/k2.png', 'k2.png',
                       'image', 'queued', 0, '2026-07-15T00:00:00Z',
                       '2026-07-15T00:00:00Z')",
            [],
        )
        .unwrap();
        let store = MemoryInstallationSecretStore::with_secret(vec![7u8; 32]);
        let state = AppState::new_with_installation_secret_store(conn, store);

        assert_eq!(
            ensure_local_batch_execution_entitled(&state, "C:/media/k2.png").unwrap_err(),
            "本地批量处理从 Creator 开放"
        );

        let mut entitlement = EntitlementState::default();
        entitlement.status = EntitlementStatus::Active;
        entitlement.plan_code = "creator".to_string();
        entitlement
            .features
            .insert("batch_processing".to_string(), true);
        let conn = state.db.lock().unwrap();
        billing::save_entitlement_state(&conn, &entitlement).unwrap();
        drop(conn);

        ensure_local_batch_execution_entitled(&state, "C:/media/k2.png").unwrap();
    }
}

fn pipeline_failure_stage(error: &crate::pipeline::error::PipelineError) -> String {
    if error.watermark_code() == Some("audio_duration_unknown")
        || error
            .to_string()
            .contains("audio_protection_duration_unknown")
    {
        return "失败：无法确认音频时长，未生成保护副本。请更换可识别时长的完整音频文件后重试"
            .to_string();
    }
    if error.watermark_code() == Some("audio_too_short")
        || error.to_string().contains("audio_protection_min_duration")
    {
        return "失败：音频时长不足 30 秒，未生成保护副本。请选择 30 秒以上的完整音频作品后重试"
            .to_string();
    }
    if error.watermark_code() == Some("audio_sample_rate_too_low")
        || error.watermark_code() == Some("audio_sample_rate_too_high")
    {
        return "失败：当前仅支持 8–48 kHz 音频采样率，未生成保护副本。请保持原始规格并更换常见采样率音频后重试"
            .to_string();
    }
    if error.watermark_code() == Some("audio_channels_unsupported") {
        return "失败：当前仅支持 mono 或 stereo 音频，未生成保护副本。请保持原始规格并选择单声道或立体声音频后重试"
            .to_string();
    }
    if error.watermark_code() == Some("audio_spec_unknown") {
        return "失败：无法确认音频采样率或声道，未生成保护副本。请选择可识别规格的完整音频文件后重试"
            .to_string();
    }
    if error.watermark_code() == Some("image_capacity_insufficient") {
        return "失败：当前图片可用水印容量不足，未生成保护副本。请选择像素更多或裁剪更少的原图后重试"
            .to_string();
    }
    if error.watermark_code() == Some("image_pixel_limit_exceeded") {
        return "失败：图片超过 100 MP 上限，未生成保护副本。请选择像素不超过 100 MP 的静态图片后重试"
            .to_string();
    }
    if error.watermark_code() == Some("image_file_size_limit_exceeded") {
        return "失败：图片超过 512 MiB 上限，未生成保护副本。请选择文件不超过 512 MiB 的静态图片后重试"
            .to_string();
    }
    if error.watermark_code() == Some("image_format_unsupported") {
        return "失败：当前仅支持静态 PNG、JPEG、WebP，未生成保护副本。请转换为正式支持格式后重试"
            .to_string();
    }
    if error.watermark_code() == Some("already_watermarked") {
        if let Some(uid) = error.existing_watermark_uid() {
            return format!("失败：检测到已有版权记录 {uid}。如需生成新版，请开启“作为新版写入”。");
        }
        return "失败：检测到已有版权记录。如需生成新版，请开启“作为新版写入”。".to_string();
    }
    if error.watermark_code() == Some("missing_creator_identity") {
        return "失败：请先完成创作者身份设置，再生成保护副本。".to_string();
    }
    if error.watermark_code() == Some("embed_failed") {
        return "失败：保护副本未生成。请确认文件可读取后重试；如果持续失败，请复制诊断信息反馈。"
            .to_string();
    }
    let error = error.to_string();
    format!("失败：{error}")
}

#[tauri::command]
pub async fn cancel_pipeline(
    pipeline_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut active = state
        .active_pipelines
        .lock()
        .map_err(|err| err.to_string())?;
    active.remove(&pipeline_id);
    Ok(())
}

/// Returns the set of currently active pipeline IDs.
/// Used by the frontend to reconcile state after window regains focus.
#[tauri::command]
pub async fn check_active_pipelines(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let active = state
        .active_pipelines
        .lock()
        .map_err(|err| err.to_string())?;
    Ok(active.iter().cloned().collect())
}

#[tauri::command]
pub async fn open_output_dir(dir_path: String) -> Result<(), String> {
    let path = std::path::Path::new(&dir_path);
    if !path.exists() {
        return Err(format!("目录不存在: {dir_path}"));
    }

    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer.exe")
            .arg(&dir_path)
            .spawn()
            .map_err(|e| format!("打开文件管理器失败: {e}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg(&dir_path)
            .spawn()
            .map_err(|e| format!("打开文件管理器失败: {e}"))?;
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(&dir_path)
            .spawn()
            .map_err(|e| format!("打开文件管理器失败: {e}"))?;
    }

    Ok(())
}

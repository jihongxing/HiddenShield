use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

use crate::encoder::hw_detect;
use crate::pipeline::ffmpeg;
use crate::pipeline::progress;
use crate::pipeline::scheduler::{self, PipelineParams};
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
pub struct TranscodeOptions {
    pub aspect_strategy: AspectStrategy,
    pub encoding_mode: EncodingMode,
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

    // For video files, at least one platform is required.
    // For image/audio files, platforms can be empty (watermark-only workflow).
    let file_type = scheduler::classify_file(std::path::Path::new(&input_path));
    let is_media_only = file_type != scheduler::FileType::Video;

    if platforms.is_empty() && !is_media_only {
        return Err("at least one platform must be selected".to_string());
    }

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
            "已为 {} 个平台创建压制任务（{}）",
            platforms.len(),
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
                let _ = app_handle_clone.emit(
                    "pipeline-progress",
                    PipelineProgressPayload {
                        pipeline_id: pipeline_id_clone,
                        stage: format!("失败：{e}"),
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

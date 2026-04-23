use std::path::{Path, PathBuf};
use std::time::Instant;

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::commands::transcode::{
    EncodingMode, PipelineProgressPayload, Platform, TranscodeOptions,
};
use crate::commands::vault::VaultRecord;
use crate::db::queries;
use crate::encoder::hw_detect::DetectedHardware;
use crate::encoder::presets;
use crate::encoder::tonemap;
use crate::identity;
use crate::pipeline::error::PipelineError;
use crate::pipeline::ffmpeg::{self, FfmpegPaths};
use crate::pipeline::image_watermark;
use crate::pipeline::progress::PlatformPercents;
use crate::pipeline::system_guard;
use crate::pipeline::watermark::{self, WatermarkPayload};
use crate::tsa;
use crate::utils::fs as ufs;
use crate::utils::hash;
use crate::AppState;

// ---------------------------------------------------------------------------
// Pipeline Complete Payload
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputFileInfo {
    pub platform: String,
    pub path: String,
    pub size_mb: f64,
    pub resolution: String,
    pub fps: f64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineCompletePayload {
    pub pipeline_id: String,
    pub watermark_uid: String,
    pub process_time_ms: u64,
    pub encoder_used: String,
    pub outputs: Vec<OutputFileInfo>,
    pub vault_record: VaultRecord,
}

// ---------------------------------------------------------------------------
// File type classification
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Video,
    Image,
    Audio,
}

pub fn classify_file(path: &Path) -> FileType {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "bmp" | "tiff" | "webp" | "gif" => FileType::Image,
        "wav" | "mp3" | "flac" | "aac" | "ogg" | "m4a" => FileType::Audio,
        _ => FileType::Video,
    }
}

// ---------------------------------------------------------------------------
// Pipeline parameters
// ---------------------------------------------------------------------------

pub struct PipelineParams {
    pub input_path: PathBuf,
    pub platforms: Vec<Platform>,
    pub options: TranscodeOptions,
    pub ffmpeg_paths: Option<FfmpegPaths>,
    pub hw_info: Option<DetectedHardware>,
    pub pipeline_id: String,
}

// ---------------------------------------------------------------------------
// Main entry point
// ---------------------------------------------------------------------------

/// Run the full pipeline: classify file type and route to the appropriate chain.
/// Manages global sleep lock lifecycle (acquire on start, release on completion/error).
pub async fn run_pipeline(
    params: PipelineParams,
    app_handle: AppHandle,
    _db: &std::sync::Mutex<rusqlite::Connection>,
) -> Result<(), PipelineError> {
    // Acquire global sleep lock (ref-counted)
    let state = app_handle.state::<AppState>();
    state.acquire_sleep_lock();

    let file_type = classify_file(&params.input_path);
    let result = match file_type {
        FileType::Video => run_video_pipeline(params, app_handle.clone()).await,
        FileType::Image => run_image_pipeline(params, app_handle.clone()).await,
        FileType::Audio => run_audio_pipeline(params, app_handle.clone()).await,
    };

    // Release global sleep lock (ref-counted)
    let state = app_handle.state::<AppState>();
    state.release_sleep_lock();

    result
}

/// Insert a vault record on a blocking thread to avoid stalling the Tokio runtime.
/// Uses `spawn_blocking` to isolate potential I/O latency from async tasks.
async fn insert_record_async(
    app_handle: &AppHandle,
    record: VaultRecord,
) -> Result<(), PipelineError> {
    let handle = app_handle.clone();
    tokio::task::spawn_blocking(move || -> Result<(), PipelineError> {
        let state = handle.state::<AppState>();
        let conn = match state.db.lock() {
            Ok(c) => c,
            Err(e) => {
                return Err(PipelineError::DatabaseError(format!("DB lock failed: {e}")));
            }
        };
        if let Err(e) = queries::insert_record(&conn, &record) {
            return Err(PipelineError::DatabaseError(format!(
                "Failed to insert vault record: {e}"
            )));
        }
        Ok(())
    })
    .await
    .map_err(|e| PipelineError::DatabaseError(format!("join blocking DB write task: {e}")))?
}

// ---------------------------------------------------------------------------
// Video pipeline
// ---------------------------------------------------------------------------

async fn run_video_pipeline(
    params: PipelineParams,
    app_handle: AppHandle,
) -> Result<(), PipelineError> {
    let ffmpeg_paths = params
        .ffmpeg_paths
        .as_ref()
        .ok_or(PipelineError::FfmpegNotFound)?;
    let hw_info = params
        .hw_info
        .as_ref()
        .ok_or_else(|| PipelineError::FfmpegFailed("missing hardware info".into()))?;
    let start = Instant::now();
    let input_str = params.input_path.to_string_lossy().to_string();
    let output_dir = ufs::safe_output_dir(&input_str);

    // 1. Disk space pre-check
    let file_size = std::fs::metadata(&params.input_path)
        .map(|m| m.len())
        .unwrap_or(0);
    system_guard::check_disk_space(&output_dir, file_size, params.platforms.len())?;

    emit_progress(&app_handle, &params.pipeline_id, "正在分析视频信息...", 5);
    check_cancelled(&app_handle, &params.pipeline_id)?;

    // 3. Probe source via ffprobe
    let probe = ffmpeg::ffprobe_source(&ffmpeg_paths.ffprobe, &input_str).await?;
    let video_stream = probe
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"));
    let duration_secs = probe
        .format
        .as_ref()
        .and_then(|f| f.duration)
        .unwrap_or(0.0);
    let (width, height) = video_stream
        .map(|s| (s.width.unwrap_or(0), s.height.unwrap_or(0)))
        .unwrap_or((0, 0));
    let fps = video_stream.and_then(|s| s.r_frame_rate).unwrap_or(30.0);
    let is_hdr = video_stream
        .map(|s| tonemap::is_hdr(s.color_transfer.as_deref(), s.color_primaries.as_deref()))
        .unwrap_or(false);

    if is_hdr {
        emit_progress(
            &app_handle,
            &params.pipeline_id,
            "检测到 iPhone HDR 视频，正在优化色彩...",
            8,
        );
    }

    // 4. Extract audio to temp WAV
    emit_progress(&app_handle, &params.pipeline_id, "正在注入版权基因...", 12);
    check_cancelled(&app_handle, &params.pipeline_id)?;

    let temp_dir = ufs::create_temp_dir(&params.pipeline_id)
        .map_err(|e| PipelineError::FfmpegFailed(format!("create temp dir: {e}")))?;
    let temp_wav = temp_dir.join("audio.wav");
    let watermarked_wav = temp_dir.join("watermarked.wav");

    extract_audio(&ffmpeg_paths.ffmpeg, &params.input_path, &temp_wav).await?;

    // 5. Read WAV, embed watermark, write back
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| PipelineError::FfmpegFailed(format!("resolve app data dir: {e}")))?;
    let id_bytes =
        identity::get_identity_bytes(&app_data_dir).unwrap_or_else(|| identity::IdentityBytes {
            user_seed: [0; 8],
            device_id: identity::compute_device_id(),
        });
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let file_hash = compute_file_hash_prefix(&input_str);
    let payload =
        WatermarkPayload::new(id_bytes.user_seed, timestamp, id_bytes.device_id, file_hash);

    embed_watermark_wav(&temp_wav, &watermarked_wav, &payload)?;

    emit_progress(&app_handle, &params.pipeline_id, "版权保护已激活", 20);
    check_cancelled(&app_handle, &params.pipeline_id)?;

    // 6. Build source meta for presets
    let source_meta = crate::commands::probe::SourceMeta {
        file_name: params
            .input_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        path: input_str.clone(),
        width,
        height,
        fps,
        duration_secs,
        file_size_mb: file_size as f64 / 1024.0 / 1024.0,
        is_hdr,
        color_profile: if is_hdr {
            "BT.2020 / PQ".to_string()
        } else {
            "BT.709 / SDR".to_string()
        },
        sha256: String::new(), // computed later
        file_type: "video".to_string(),
    };

    let tonemap_filter = tonemap::build_tonemap_filter(is_hdr);

    // 7. Multi-platform parallel transcode
    let source_stem = params
        .input_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    let mut output_douyin: Option<String> = None;
    let mut output_bilibili: Option<String> = None;
    let mut output_xhs: Option<String> = None;

    let total_platforms = params.platforms.len();
    for (idx, platform) in params.platforms.iter().enumerate() {
        check_cancelled(&app_handle, &params.pipeline_id)?;

        let config = presets::build_transcode_config(
            *platform,
            &source_meta,
            hw_info,
            &params.options,
            tonemap_filter.as_deref(),
        );

        let out_name = ufs::output_file_name(&source_stem, *platform);
        let out_path = output_dir.join(&out_name);

        let stage_label = format!(
            "正在压制 {} ({}/{})",
            platform_label(*platform),
            idx + 1,
            total_platforms
        );
        let base_percent = 20 + (idx as u8) * (70 / total_platforms as u8);
        emit_progress(&app_handle, &params.pipeline_id, &stage_label, base_percent);

        // Try hardware encoding first, fall back to CPU on failure
        let result = transcode_one(
            &ffmpeg_paths.ffmpeg,
            &params.input_path,
            &watermarked_wav,
            &out_path,
            &config,
            duration_secs,
            &app_handle,
            &params.pipeline_id,
            *platform,
        )
        .await;

        let final_result = match result {
            Ok(()) => Ok(()),
            Err(e) => {
                // Retry with CPU software encoding if we were using hardware
                if is_hw_encoder(&config.video_codec) {
                    log::warn!(
                        "Hardware encode failed for {:?}, retrying with CPU: {e}",
                        platform
                    );
                    let cpu_options = TranscodeOptions {
                        aspect_strategy: params.options.aspect_strategy.clone(),
                        encoding_mode: EncodingMode::HighQualityCpu,
                    };
                    let cpu_config = presets::build_transcode_config(
                        *platform,
                        &source_meta,
                        hw_info,
                        &cpu_options,
                        tonemap_filter.as_deref(),
                    );
                    // Emit degradation warning so frontend can notify the user
                    let _ = app_handle.emit("hw-degradation", serde_json::json!({
                        "pipelineId": params.pipeline_id,
                        "failedEncoder": config.video_codec,
                        "fallbackEncoder": cpu_config.video_codec,
                        "message": "检测到显卡编码器异常，已自动切换为 CPU 兼容模式（耗时将增加）。建议更新显卡驱动。"
                    }));
                    emit_progress(
                        &app_handle,
                        &params.pipeline_id,
                        &format!(
                            "硬件编码失败，已自动切换 CPU 编码重试 {}",
                            platform_label(*platform)
                        ),
                        base_percent,
                    );
                    transcode_one(
                        &ffmpeg_paths.ffmpeg,
                        &params.input_path,
                        &watermarked_wav,
                        &out_path,
                        &cpu_config,
                        duration_secs,
                        &app_handle,
                        &params.pipeline_id,
                        *platform,
                    )
                    .await
                } else {
                    Err(e)
                }
            }
        };

        final_result?;

        // Record output path
        let out_str = out_path.to_string_lossy().to_string();
        match platform {
            Platform::Douyin => output_douyin = Some(out_str),
            Platform::Bilibili => output_bilibili = Some(out_str),
            Platform::Xiaohongshu => output_xhs = Some(out_str),
        }
    }

    // 8. Compute file hash and insert vault record
    let sha256 = hash::sha256_of_file(&input_str).unwrap_or_default();
    let process_time_ms = start.elapsed().as_millis() as u64;

    let record = VaultRecord {
        id: 0,
        original_hash: sha256.clone(),
        file_name: source_meta.file_name.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_secs,
        resolution: format!("{}x{}", width, height),
        watermark_uid: payload.watermark_uid(),
        thumbnail_path: None,
        output_douyin,
        output_bilibili,
        output_xhs,
        is_hdr_source: is_hdr,
        hw_encoder_used: Some(hw_info.preferred_encoder.clone()),
        process_time_ms: Some(process_time_ms),
        tsa_token_path: None,
        network_time: None,
        tsa_source: None,
        tsa_request_nonce: None,
    };

    // Request trusted timestamp (non-blocking, best-effort)
    let tsa_dir = app_handle
        .path()
        .app_data_dir()
        .map(|d| d.join("tsa_tokens"))
        .unwrap_or_default();
    let attestation = if crate::telemetry::is_network_enabled(&app_data_dir)
        && crate::telemetry::is_acknowledged(&app_data_dir)
    {
        tsa::request_attestation(&sha256, &record.watermark_uid, &tsa_dir).await
    } else {
        tsa::TimestampAttestation::offline()
    };
    let mut record = record;
    record.tsa_token_path = attestation.tsa_token_path;
    record.network_time = attestation.network_time;
    record.tsa_source = attestation.tsa_source;
    record.tsa_request_nonce = attestation.tsa_request_nonce;

    insert_record_async(&app_handle, record.clone()).await?;

    // 9. Cleanup temp files
    let _ = ufs::cleanup_temp_dir(&params.pipeline_id);

    // 10. Build output file info and emit pipeline-complete event
    let mut outputs: Vec<OutputFileInfo> = Vec::new();
    let output_entries: Vec<(&Option<String>, &str)> = vec![
        (&record.output_douyin, "douyin"),
        (&record.output_bilibili, "bilibili"),
        (&record.output_xhs, "xiaohongshu"),
    ];
    for (opt_path, platform_name) in &output_entries {
        let path_str = match opt_path {
            Some(p) => p.clone(),
            None => continue,
        };
        if path_str.is_empty() {
            continue;
        }
        let size_mb = std::fs::metadata(&path_str)
            .map(|m| (m.len() as f64 / 1024.0 / 1024.0 * 10.0).round() / 10.0)
            .unwrap_or(0.0);
        // Use the target resolution from presets for this platform
        let (out_w, out_h, out_fps) = get_output_specs(
            params
                .platforms
                .iter()
                .find(|p| platform_key(**p) == *platform_name)
                .copied(),
            width,
            height,
            fps,
        );
        outputs.push(OutputFileInfo {
            platform: platform_name.to_string(),
            path: path_str.clone(),
            size_mb,
            resolution: format!("{}x{}", out_w, out_h),
            fps: out_fps,
        });
    }

    let complete_payload = PipelineCompletePayload {
        pipeline_id: params.pipeline_id.clone(),
        watermark_uid: record.watermark_uid.clone(),
        process_time_ms,
        encoder_used: hw_info.preferred_encoder.clone(),
        outputs,
        vault_record: record,
    };
    let _ = app_handle.emit("pipeline-complete", &complete_payload);

    emit_progress(&app_handle, &params.pipeline_id, "全部文件已就绪", 100);
    Ok(())
}

// ---------------------------------------------------------------------------
// Image pipeline
// ---------------------------------------------------------------------------

async fn run_image_pipeline(
    params: PipelineParams,
    app_handle: AppHandle,
) -> Result<(), PipelineError> {
    let start = Instant::now();
    let input_str = params.input_path.to_string_lossy().to_string();

    check_cancelled(&app_handle, &params.pipeline_id)?;
    emit_progress(&app_handle, &params.pipeline_id, "图片水印嵌入中", 10);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| PipelineError::FfmpegFailed(format!("resolve app data dir: {e}")))?;
    let id_bytes =
        identity::get_identity_bytes(&app_data_dir).unwrap_or_else(|| identity::IdentityBytes {
            user_seed: [0; 8],
            device_id: identity::compute_device_id(),
        });
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let file_hash = compute_file_hash_prefix(&input_str);
    let payload =
        WatermarkPayload::new(id_bytes.user_seed, timestamp, id_bytes.device_id, file_hash);

    // Output path: same directory, with _watermarked suffix
    let stem = params
        .input_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let ext = params
        .input_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    // DWT-DCT-SVD watermark can survive JPEG, but PNG preserves full fidelity.
    // Always output as PNG for maximum watermark extraction reliability.
    let out_ext = if ext == "jpg" || ext == "jpeg" {
        "png"
    } else {
        &ext
    };
    let output_path = params
        .input_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("{stem}_watermarked.{out_ext}"));

    check_cancelled(&app_handle, &params.pipeline_id)?;
    image_watermark::embed_image_watermark(&params.input_path, &payload, &output_path)?;

    check_cancelled(&app_handle, &params.pipeline_id)?;
    emit_progress(&app_handle, &params.pipeline_id, "水印嵌入完成", 80);

    // Read image dimensions for the record
    let (width, height) = image::image_dimensions(&params.input_path).unwrap_or((0, 0));
    let sha256 = hash::sha256_of_file(&input_str).unwrap_or_default();
    let process_time_ms = start.elapsed().as_millis() as u64;

    let record = VaultRecord {
        id: 0,
        original_hash: sha256,
        file_name: params
            .input_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_secs: 0.0,
        resolution: format!("{}x{}", width, height),
        watermark_uid: payload.watermark_uid(),
        thumbnail_path: None,
        output_douyin: None,
        output_bilibili: None,
        output_xhs: None,
        is_hdr_source: false,
        hw_encoder_used: None,
        process_time_ms: Some(process_time_ms),
        tsa_token_path: None,
        network_time: None,
        tsa_source: None,
        tsa_request_nonce: None,
    };

    // Request trusted timestamp (non-blocking, best-effort)
    let tsa_dir = app_handle
        .path()
        .app_data_dir()
        .map(|d| d.join("tsa_tokens"))
        .unwrap_or_default();
    let attestation = if crate::telemetry::is_network_enabled(&app_data_dir)
        && crate::telemetry::is_acknowledged(&app_data_dir)
    {
        tsa::request_attestation(&record.original_hash, &record.watermark_uid, &tsa_dir).await
    } else {
        tsa::TimestampAttestation::offline()
    };
    let mut record = record;
    record.tsa_token_path = attestation.tsa_token_path;
    record.network_time = attestation.network_time;
    record.tsa_source = attestation.tsa_source;
    record.tsa_request_nonce = attestation.tsa_request_nonce;

    check_cancelled(&app_handle, &params.pipeline_id)?;
    insert_record_async(&app_handle, record.clone()).await?;

    check_cancelled(&app_handle, &params.pipeline_id)?;

    let complete_payload = PipelineCompletePayload {
        pipeline_id: params.pipeline_id.clone(),
        watermark_uid: record.watermark_uid.clone(),
        process_time_ms,
        encoder_used: "DWT-DCT-SVD Blind Watermark".to_string(),
        outputs: vec![OutputFileInfo {
            platform: "image".to_string(),
            path: output_path.to_string_lossy().to_string(),
            size_mb: std::fs::metadata(&output_path)
                .map(|m| (m.len() as f64 / 1024.0 / 1024.0 * 10.0).round() / 10.0)
                .unwrap_or(0.0),
            resolution: format!("{}x{}", width, height),
            fps: 0.0,
        }],
        vault_record: record,
    };
    let _ = app_handle.emit("pipeline-complete", &complete_payload);

    emit_progress(&app_handle, &params.pipeline_id, "图片处理完成", 100);

    Ok(())
}

// ---------------------------------------------------------------------------
// Audio pipeline
// ---------------------------------------------------------------------------

async fn run_audio_pipeline(
    params: PipelineParams,
    app_handle: AppHandle,
) -> Result<(), PipelineError> {
    let ffmpeg_paths = params
        .ffmpeg_paths
        .as_ref()
        .ok_or(PipelineError::FfmpegNotFound)?;
    let start = Instant::now();
    let input_str = params.input_path.to_string_lossy().to_string();

    check_cancelled(&app_handle, &params.pipeline_id)?;
    emit_progress(&app_handle, &params.pipeline_id, "音频水印嵌入中", 10);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| PipelineError::FfmpegFailed(format!("resolve app data dir: {e}")))?;
    let id_bytes =
        identity::get_identity_bytes(&app_data_dir).unwrap_or_else(|| identity::IdentityBytes {
            user_seed: [0; 8],
            device_id: identity::compute_device_id(),
        });
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let file_hash = compute_file_hash_prefix(&input_str);
    let payload =
        WatermarkPayload::new(id_bytes.user_seed, timestamp, id_bytes.device_id, file_hash);

    // Convert input to PCM WAV if needed, then embed watermark
    let temp_dir = ufs::create_temp_dir(&params.pipeline_id)
        .map_err(|e| PipelineError::FfmpegFailed(format!("create temp dir: {e}")))?;
    let temp_wav = temp_dir.join("input.wav");

    check_cancelled(&app_handle, &params.pipeline_id)?;
    // Use FFmpeg to convert any audio format to standard WAV
    extract_audio(&ffmpeg_paths.ffmpeg, &params.input_path, &temp_wav).await?;

    check_cancelled(&app_handle, &params.pipeline_id)?;
    emit_progress(&app_handle, &params.pipeline_id, "频域水印写入中", 40);

    let stem = params
        .input_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let output_path = params
        .input_path
        .parent()
        .unwrap_or(Path::new("."))
        .join(format!("{stem}_watermarked.wav"));

    check_cancelled(&app_handle, &params.pipeline_id)?;
    embed_watermark_wav(&temp_wav, &output_path, &payload)?;

    check_cancelled(&app_handle, &params.pipeline_id)?;
    emit_progress(&app_handle, &params.pipeline_id, "水印嵌入完成", 80);

    let sha256 = hash::sha256_of_file(&input_str).unwrap_or_default();
    let process_time_ms = start.elapsed().as_millis() as u64;

    let record = VaultRecord {
        id: 0,
        original_hash: sha256,
        file_name: params
            .input_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_secs: 0.0,
        resolution: String::new(),
        watermark_uid: payload.watermark_uid(),
        thumbnail_path: None,
        output_douyin: None,
        output_bilibili: None,
        output_xhs: None,
        is_hdr_source: false,
        hw_encoder_used: None,
        process_time_ms: Some(process_time_ms),
        tsa_token_path: None,
        network_time: None,
        tsa_source: None,
        tsa_request_nonce: None,
    };

    // Request trusted timestamp (non-blocking, best-effort)
    let tsa_dir = app_handle
        .path()
        .app_data_dir()
        .map(|d| d.join("tsa_tokens"))
        .unwrap_or_default();
    let attestation = if crate::telemetry::is_network_enabled(&app_data_dir)
        && crate::telemetry::is_acknowledged(&app_data_dir)
    {
        tsa::request_attestation(&record.original_hash, &record.watermark_uid, &tsa_dir).await
    } else {
        tsa::TimestampAttestation::offline()
    };
    let mut record = record;
    record.tsa_token_path = attestation.tsa_token_path;
    record.network_time = attestation.network_time;
    record.tsa_source = attestation.tsa_source;
    record.tsa_request_nonce = attestation.tsa_request_nonce;

    check_cancelled(&app_handle, &params.pipeline_id)?;
    insert_record_async(&app_handle, record.clone()).await?;

    check_cancelled(&app_handle, &params.pipeline_id)?;

    let complete_payload = PipelineCompletePayload {
        pipeline_id: params.pipeline_id.clone(),
        watermark_uid: record.watermark_uid.clone(),
        process_time_ms,
        encoder_used: "Frequency Domain Watermark".to_string(),
        outputs: vec![OutputFileInfo {
            platform: "audio".to_string(),
            path: output_path.to_string_lossy().to_string(),
            size_mb: std::fs::metadata(&output_path)
                .map(|m| (m.len() as f64 / 1024.0 / 1024.0 * 10.0).round() / 10.0)
                .unwrap_or(0.0),
            resolution: String::new(),
            fps: 0.0,
        }],
        vault_record: record,
    };
    let _ = app_handle.emit("pipeline-complete", &complete_payload);

    emit_progress(&app_handle, &params.pipeline_id, "音频处理完成", 100);

    let _ = ufs::cleanup_temp_dir(&params.pipeline_id);
    Ok(())
}

// ---------------------------------------------------------------------------
// FFmpeg helpers
// ---------------------------------------------------------------------------

/// Extract audio from a media file to a 16-bit 44.1kHz stereo WAV.
/// Uses `-async 1` to align audio timestamps and fill gaps from VFR sources.
async fn extract_audio(
    ffmpeg: &Path,
    input: &Path,
    output_wav: &Path,
) -> Result<(), PipelineError> {
    let args: Vec<String> = vec![
        "-y".into(),
        "-i".into(),
        input.to_string_lossy().to_string(),
        "-vn".into(),
        "-acodec".into(),
        "pcm_s16le".into(),
        "-ar".into(),
        "44100".into(),
        "-ac".into(),
        "2".into(),
        "-af".into(),
        "aresample=async=1".into(), // Fix A/V sync: align timestamps for VFR sources
        output_wav.to_string_lossy().to_string(),
    ];

    let mut child = ffmpeg::spawn_ffmpeg(ffmpeg, &args).await?;
    let status = child
        .child
        .wait()
        .await
        .map_err(|e| PipelineError::FfmpegFailed(format!("wait audio extract: {e}")))?;

    if !status.success() {
        return Err(PipelineError::FfmpegFailed(
            "audio extraction failed".into(),
        ));
    }
    Ok(())
}

/// Read a WAV file, embed watermark into PCM samples, write output WAV.
/// If audio is too short for watermark embedding (< 4096 samples), the file
/// is copied as-is without watermark to avoid a panic/crash.
fn embed_watermark_wav(
    input_wav: &Path,
    output_wav: &Path,
    payload: &WatermarkPayload,
) -> Result<(), PipelineError> {
    let mut reader = hound::WavReader::open(input_wav)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("open WAV: {e}")))?;
    let spec = reader.spec();

    // Read all samples as f32
    let mut samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Float {
        reader.samples::<f32>().filter_map(|s| s.ok()).collect()
    } else {
        let bits = spec.bits_per_sample;
        let max_val = (1i32 << (bits - 1)) as f32;
        reader
            .samples::<i32>()
            .filter_map(|s| s.ok())
            .map(|s| s as f32 / max_val)
            .collect()
    };

    // Guard: skip watermark if audio is too short (< FFT window size)
    if samples.len() >= 4096 {
        watermark::embed_watermark(&mut samples, payload)?;
    } else {
        log::warn!(
            "Audio too short ({} samples < 4096), skipping watermark embedding",
            samples.len()
        );
    }

    // Write output WAV as 16-bit PCM
    let out_spec = hound::WavSpec {
        channels: spec.channels,
        sample_rate: spec.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut writer = hound::WavWriter::create(output_wav, out_spec)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("create WAV: {e}")))?;

    for &sample in &samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let int_val = (clamped * 32767.0) as i16;
        writer
            .write_sample(int_val)
            .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("write sample: {e}")))?;
    }
    writer
        .finalize()
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("finalize WAV: {e}")))?;

    Ok(())
}

/// Run a single-platform FFmpeg transcode with progress parsing.
/// Acquires the appropriate semaphore (HW or SW) to limit concurrent FFmpeg processes.
/// Uses a dual-stage watchdog:
///   - Init stage (cold start): 90s timeout before first `time=` output
///   - Stall stage (active encoding): 30s timeout between `time=` outputs
#[allow(clippy::too_many_arguments)]
async fn transcode_one(
    ffmpeg: &Path,
    video_input: &Path,
    audio_input: &Path,
    output: &Path,
    config: &presets::TranscodeConfig,
    total_duration: f64,
    app_handle: &AppHandle,
    pipeline_id: &str,
    platform: Platform,
) -> Result<(), PipelineError> {
    // Acquire the appropriate semaphore based on encoder type
    let state = app_handle.state::<AppState>();
    let is_hw = is_hw_encoder(&config.video_codec);
    let _permit = if is_hw {
        state
            .hw_encode_semaphore
            .acquire()
            .await
            .map_err(|_| PipelineError::FfmpegFailed("hw semaphore closed".into()))?
    } else {
        state
            .ffmpeg_semaphore
            .acquire()
            .await
            .map_err(|_| PipelineError::FfmpegFailed("semaphore closed".into()))?
    };

    let mut args: Vec<String> = vec![
        "-y".into(),
        "-i".into(),
        video_input.to_string_lossy().to_string(),
        "-i".into(),
        audio_input.to_string_lossy().to_string(),
        "-map".into(),
        "0:v:0".into(),
        "-map".into(),
        "1:a:0".into(),
        "-c:v".into(),
        config.video_codec.clone(),
        "-vf".into(),
        config.video_filter.clone(),
    ];
    args.extend(config.video_params.clone());
    args.extend(config.audio_params.clone());
    args.extend(config.container_params.clone());
    // Force FFmpeg to output stats every second for reliable heartbeat
    args.extend_from_slice(&["-stats_period".into(), "1".into()]);
    // Prevent trailing audio from causing black frames at end
    args.push("-shortest".into());
    args.push(output.to_string_lossy().to_string());

    let mut child = ffmpeg::spawn_ffmpeg(ffmpeg, &args).await?;

    // Dual-stage watchdog: Init (90s) → Stall (30s)
    let init_timeout = std::time::Duration::from_secs(90);
    let stall_timeout = std::time::Duration::from_secs(30);

    if let Some(stderr) = child.child.stderr.take() {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut last_emit = Instant::now() - std::time::Duration::from_millis(200);
        let mut last_progress_time = Instant::now();
        let mut is_initialized = false;
        let throttle_interval = std::time::Duration::from_millis(100);

        loop {
            let timeout_duration = if is_initialized {
                stall_timeout
            } else {
                init_timeout
            };

            let line_result =
                tokio::time::timeout(std::time::Duration::from_secs(5), lines.next_line()).await;

            match line_result {
                Ok(Ok(Some(line))) => {
                    if let Some(progress) = ffmpeg::parse_progress_line(&line, total_duration) {
                        is_initialized = true;
                        last_progress_time = Instant::now();
                        let now = Instant::now();
                        if now.duration_since(last_emit) >= throttle_interval {
                            last_emit = now;
                            let percent = (20.0 + progress * 70.0) as u8;
                            let mut platform_percents = PlatformPercents::new();
                            let pct = (progress * 100.0) as u8;
                            platform_percents.insert(platform_key(platform).to_string(), pct);
                            let _ = app_handle.emit(
                                "pipeline-progress",
                                PipelineProgressPayload {
                                    pipeline_id: pipeline_id.to_string(),
                                    stage: format!("正在压制 {}", platform_label(platform)),
                                    percent,
                                    platform_percents,
                                },
                            );
                        }
                    }
                }
                Ok(Ok(None)) => break, // EOF
                Ok(Err(_)) => break,   // Read error
                Err(_) => {
                    // Timeout reading — check dual-stage watchdog
                    if last_progress_time.elapsed() > timeout_duration {
                        let stage_label = if is_initialized {
                            "压制中失速"
                        } else {
                            "冷启动"
                        };
                        let timeout_secs = timeout_duration.as_secs();
                        log::error!(
                            "FFmpeg watchdog triggered ({}): no progress for {}s, killing process",
                            stage_label,
                            timeout_secs
                        );
                        let _ = child.child.kill().await;
                        let _ = std::fs::remove_file(output);
                        return Err(PipelineError::FfmpegFailed(format!(
                            "FFmpeg {}阶段无响应超过 {} 秒，已强制终止",
                            stage_label, timeout_secs
                        )));
                    }
                }
            }

            // Check cancellation during transcode
            if is_cancelled(app_handle, pipeline_id) {
                let _ = child.child.kill().await;
                let _ = std::fs::remove_file(output);
                let _ = ufs::cleanup_temp_dir(pipeline_id);
                return Err(PipelineError::Cancelled);
            }
        }
    }

    let status = child.child.wait().await.map_err(|e| {
        let _ = std::fs::remove_file(output); // Clean up partial output
        PipelineError::FfmpegFailed(format!("wait transcode: {e}"))
    })?;

    if !status.success() {
        let _ = std::fs::remove_file(output); // Clean up failed output

        // Report FFmpeg crash to telemetry
        if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
            let report = crate::telemetry::FfmpegCrashReport {
                exit_code: status.code().unwrap_or(-1),
                stderr_tail: format!("(stderr consumed by progress parser) exit={}", status),
                encoder: config.video_codec.clone(),
                input_format: format!("{}x{}", "video", total_duration),
                app_version: env!("CARGO_PKG_VERSION").to_string(),
                anonymous_device_id: crate::telemetry::anonymous_device_id(),
            };
            crate::telemetry::report_ffmpeg_crash(&app_data_dir, &report);
        }

        return Err(PipelineError::FfmpegFailed(format!(
            "FFmpeg exited with {}",
            status
        )));
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Utility helpers
// ---------------------------------------------------------------------------

fn emit_progress(app_handle: &AppHandle, pipeline_id: &str, stage: &str, percent: u8) {
    let _ = app_handle.emit(
        "pipeline-progress",
        PipelineProgressPayload {
            pipeline_id: pipeline_id.to_string(),
            stage: stage.to_string(),
            percent,
            platform_percents: PlatformPercents::new(),
        },
    );
}

fn is_cancelled(app_handle: &AppHandle, pipeline_id: &str) -> bool {
    app_handle
        .try_state::<AppState>()
        .and_then(|state| {
            state.active_pipelines.lock().ok().map(
                |active: std::sync::MutexGuard<'_, std::collections::HashSet<String>>| {
                    !active.contains(pipeline_id)
                },
            )
        })
        .unwrap_or(false)
}

fn check_cancelled(app_handle: &AppHandle, pipeline_id: &str) -> Result<(), PipelineError> {
    if is_cancelled(app_handle, pipeline_id) {
        Err(PipelineError::Cancelled)
    } else {
        Ok(())
    }
}

fn is_hw_encoder(codec: &str) -> bool {
    codec.contains("nvenc")
        || codec.contains("videotoolbox")
        || codec.contains("qsv")
        || codec.contains("amf")
}

fn platform_key(platform: Platform) -> &'static str {
    match platform {
        Platform::Douyin => "douyin",
        Platform::Bilibili => "bilibili",
        Platform::Xiaohongshu => "xiaohongshu",
    }
}

fn platform_label(platform: Platform) -> &'static str {
    match platform {
        Platform::Douyin => "抖音",
        Platform::Bilibili => "B站",
        Platform::Xiaohongshu => "小红书",
    }
}

/// Get approximate output specs for a platform based on source dimensions.
fn get_output_specs(
    platform: Option<Platform>,
    src_w: u32,
    src_h: u32,
    src_fps: f64,
) -> (u32, u32, f64) {
    match platform {
        Some(Platform::Douyin) => (1080, 1920, 30.0),
        Some(Platform::Xiaohongshu) => (1080, 1440, 30.0),
        Some(Platform::Bilibili) => {
            let fps = if src_fps > 30.0 { 60.0 } else { 30.0 };
            (1920, 1080, fps)
        }
        None => (src_w, src_h, src_fps),
    }
}

/// Compute the first 4 bytes of a file's SHA-256 hash for asset binding.
fn compute_file_hash_prefix(file_path: &str) -> [u8; 4] {
    let hex_str = hash::sha256_of_file(file_path).unwrap_or_default();
    let mut prefix = [0u8; 4];
    if let Ok(bytes) = hex::decode(&hex_str) {
        if bytes.len() >= 4 {
            prefix.copy_from_slice(&bytes[..4]);
        }
    }
    prefix
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_video_extensions() {
        assert_eq!(classify_file(Path::new("test.mp4")), FileType::Video);
        assert_eq!(classify_file(Path::new("test.mov")), FileType::Video);
        assert_eq!(classify_file(Path::new("test.avi")), FileType::Video);
        assert_eq!(classify_file(Path::new("test.mkv")), FileType::Video);
    }

    #[test]
    fn classify_image_extensions() {
        assert_eq!(classify_file(Path::new("photo.jpg")), FileType::Image);
        assert_eq!(classify_file(Path::new("photo.JPEG")), FileType::Image);
        assert_eq!(classify_file(Path::new("photo.png")), FileType::Image);
        assert_eq!(classify_file(Path::new("photo.webp")), FileType::Image);
    }

    #[test]
    fn classify_audio_extensions() {
        assert_eq!(classify_file(Path::new("song.wav")), FileType::Audio);
        assert_eq!(classify_file(Path::new("song.mp3")), FileType::Audio);
        assert_eq!(classify_file(Path::new("song.flac")), FileType::Audio);
        assert_eq!(classify_file(Path::new("song.m4a")), FileType::Audio);
    }

    #[test]
    fn classify_unknown_defaults_to_video() {
        assert_eq!(classify_file(Path::new("file.xyz")), FileType::Video);
        assert_eq!(classify_file(Path::new("noext")), FileType::Video);
    }

    #[test]
    fn is_hw_encoder_detection() {
        assert!(is_hw_encoder("h264_nvenc"));
        assert!(is_hw_encoder("hevc_videotoolbox"));
        assert!(is_hw_encoder("h264_qsv"));
        assert!(is_hw_encoder("h264_amf"));
        assert!(!is_hw_encoder("libx264"));
        assert!(!is_hw_encoder("libx265"));
    }
}

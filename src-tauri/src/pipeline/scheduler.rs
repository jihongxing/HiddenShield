use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Emitter, Manager};
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::commands::transcode::{PipelineProgressPayload, Platform, TranscodeOptions};
use crate::commands::vault::VaultRecord;
use crate::db::billing::{self, UsageLedgerEntry};
use crate::db::queries;
use crate::encoder::hw_detect::DetectedHardware;
use crate::encoder::tonemap;
use crate::identity;
use crate::pipeline::error::PipelineError;
use crate::pipeline::ffmpeg::{self, FfmpegPaths};
use crate::pipeline::progress::PlatformPercents;
use crate::pipeline::system_guard;
use crate::pipeline::watermark::{self, WatermarkPayload};
use crate::sync::{cloud, storage as sync_storage};
use crate::telemetry;
use crate::tsa;
use crate::utils::fs as ufs;
use crate::utils::hash;
use crate::AppState;
#[cfg(test)]
use sha2::{Digest, Sha256};
#[cfg(test)]
use watermark_core::PayloadDigestBuildInput;
use watermark_core::{
    validate_audio_protection_file_size, validate_audio_protection_input, AudioProtectionMode,
    EmbedOptions, ImageOutputFormat, MediaInput, MediaOutput, PayloadV2BuildInput,
    WatermarkIssueMode, WatermarkMediaType, WatermarkService, MIN_AUDIO_PROTECTION_SECONDS,
};

fn creator_display_name_for_record(app_data_dir: &Path) -> Option<String> {
    identity::load_identity(app_data_dir)
        .map(|value| value.creator_display_name)
        .or_else(|| {
            cloud::load_desktop_cloud_sync_profile(app_data_dir)
                .map(|profile| profile.creator_display_name)
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
fn build_watermark_payload(
    creator_identity: &str,
    device_identity: &str,
    timestamp: u64,
    media_sha256: [u8; 32],
    ai_flags: watermark::AIContentFlags,
) -> Result<WatermarkPayload, PipelineError> {
    WatermarkPayload::from_identity_and_media_sha256(PayloadDigestBuildInput {
        creator_identity,
        device_identity,
        media_sha256,
        timestamp,
        ai_flags,
    })
    .map_err(core_watermark_error_to_pipeline)
}

#[derive(Debug, Clone)]
struct ReservedWatermarkId {
    response: cloud::WatermarkIdRegistryResponse,
    watermark_id: [u8; 16],
    registry_proof_hash: [u8; 16],
}

fn build_v2_watermark_payload(
    creator_identity: &str,
    timestamp: u64,
    media_sha256: [u8; 32],
    ai_flags: watermark::AIContentFlags,
    media_type: WatermarkMediaType,
    parent_watermark_uid: Option<&str>,
    revision: u32,
    reserved: Option<&ReservedWatermarkId>,
) -> Result<WatermarkPayload, PipelineError> {
    let parent_watermark_id = parent_watermark_uid
        .map(parse_watermark_uid_to_id)
        .transpose()?;
    let (watermark_id, issue_mode, registry_proof_hash) = if let Some(reserved) = reserved {
        (
            reserved.watermark_id,
            WatermarkIssueMode::ServerReserved,
            Some(reserved.registry_proof_hash),
        )
    } else {
        (
            watermark_core::generate_offline_watermark_id()
                .map_err(core_watermark_error_to_pipeline)?,
            WatermarkIssueMode::OfflineGenerated,
            None,
        )
    };

    WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id,
        parent_watermark_id,
        revision,
        issued_at: timestamp,
        original_sha256: media_sha256,
        ai_flags,
        issue_mode,
        media_type,
        registry_proof_hash,
        creator_binding: Some(creator_identity),
    })
    .map_err(core_watermark_error_to_pipeline)
}

fn parse_watermark_uid_to_id(uid: &str) -> Result<[u8; 16], PipelineError> {
    let compact = uid
        .trim()
        .strip_prefix("HS-")
        .unwrap_or(uid.trim())
        .replace('-', "");
    let bytes = hex::decode(compact).map_err(|error| {
        PipelineError::WatermarkEmbedFailed(format!("invalid watermark uid: {error}"))
    })?;
    if bytes.len() != 16 {
        return Err(PipelineError::WatermarkEmbedFailed(
            "invalid watermark uid length".to_string(),
        ));
    }
    let mut output = [0u8; 16];
    output.copy_from_slice(&bytes);
    Ok(output)
}

fn parse_hex_16(value: &str) -> Result<[u8; 16], PipelineError> {
    let bytes = hex::decode(value.trim()).map_err(|error| {
        PipelineError::WatermarkEmbedFailed(format!("invalid registry proof hash: {error}"))
    })?;
    if bytes.len() != 16 {
        return Err(PipelineError::WatermarkEmbedFailed(
            "invalid registry proof hash length".to_string(),
        ));
    }
    let mut output = [0u8; 16];
    output.copy_from_slice(&bytes);
    Ok(output)
}

fn media_sha256_hex(media_sha256: [u8; 32]) -> String {
    hex::encode(media_sha256)
}

fn prefixed_sha256(value: &str) -> String {
    if value.trim().starts_with("sha256:") {
        value.trim().to_string()
    } else {
        format!("sha256:{}", value.trim())
    }
}

fn try_reserve_watermark_id_blocking(
    app_data_dir: &Path,
    pipeline_id: &str,
    media_type: &str,
    original_hash: &str,
    parent_watermark_uid: Option<&str>,
    revision: u32,
) -> Option<ReservedWatermarkId> {
    let profile = cloud::load_desktop_cloud_sync_profile(app_data_dir)?;
    if profile.cloud_base_url.trim().is_empty()
        || profile.access_token.trim().is_empty()
        || profile.workspace_id.trim().is_empty()
        || profile.creator_profile_id.trim().is_empty()
    {
        return None;
    }
    let request = cloud::WatermarkIdReserveRequest {
        request_id: format!(
            "desktop:{pipeline_id}:{media_type}:{revision}:{}",
            original_hash.trim()
        ),
        workspace_id: profile.workspace_id.clone(),
        creator_profile_id: profile.creator_profile_id.clone(),
        media_type: media_type.to_string(),
        payload_protocol_version: 3,
        payload_bytes_length: watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32,
        parent_watermark_uid: parent_watermark_uid.map(ToOwned::to_owned),
        revision,
        original_hash: Some(prefixed_sha256(original_hash)),
    };
    let response = cloud::CloudSyncClient::new(&profile.cloud_base_url)
        .and_then(|client| client.reserve_watermark_id(&profile.access_token, &request))
        .ok()?;
    let watermark_id = parse_watermark_uid_to_id(&response.watermark_uid).ok()?;
    let registry_proof_hash = parse_hex_16(&response.registry_proof_hash).ok()?;
    Some(ReservedWatermarkId {
        response,
        watermark_id,
        registry_proof_hash,
    })
}

async fn try_reserve_watermark_id(
    app_data_dir: PathBuf,
    pipeline_id: String,
    media_type: String,
    original_hash: String,
    parent_watermark_uid: Option<String>,
    revision: u32,
) -> Option<ReservedWatermarkId> {
    tauri::async_runtime::spawn_blocking(move || {
        try_reserve_watermark_id_blocking(
            &app_data_dir,
            &pipeline_id,
            &media_type,
            &original_hash,
            parent_watermark_uid.as_deref(),
            revision,
        )
    })
    .await
    .ok()
    .flatten()
}

fn try_confirm_watermark_id_blocking(
    app_data_dir: &Path,
    watermark_uid: &str,
    original_hash: &str,
    protected_copy_hash: Option<&str>,
    write_verification_status: &str,
) -> Option<cloud::WatermarkIdRegistryResponse> {
    let profile = cloud::load_desktop_cloud_sync_profile(app_data_dir)?;
    if profile.cloud_base_url.trim().is_empty()
        || profile.access_token.trim().is_empty()
        || profile.workspace_id.trim().is_empty()
        || profile.creator_profile_id.trim().is_empty()
    {
        return None;
    }
    let request = cloud::WatermarkIdConfirmRequest {
        workspace_id: profile.workspace_id.clone(),
        creator_profile_id: profile.creator_profile_id.clone(),
        watermark_uid: watermark_uid.to_string(),
        payload_protocol_version: 3,
        payload_bytes_length: watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32,
        original_hash: Some(prefixed_sha256(original_hash)),
        protected_copy_hash: protected_copy_hash.map(prefixed_sha256),
        write_verification_status: write_verification_status.to_string(),
    };
    cloud::CloudSyncClient::new(&profile.cloud_base_url)
        .and_then(|client| client.confirm_watermark_id(&profile.access_token, &request))
        .ok()
}

async fn try_confirm_watermark_id(
    app_data_dir: PathBuf,
    watermark_uid: String,
    original_hash: String,
    protected_copy_hash: Option<String>,
    write_verification_status: String,
) -> Option<cloud::WatermarkIdRegistryResponse> {
    tauri::async_runtime::spawn_blocking(move || {
        try_confirm_watermark_id_blocking(
            &app_data_dir,
            &watermark_uid,
            &original_hash,
            protected_copy_hash.as_deref(),
            &write_verification_status,
        )
    })
    .await
    .ok()
    .flatten()
}

async fn request_attestation_quick(
    file_hash_hex: &str,
    watermark_uid: &str,
    tsa_dir: &Path,
) -> tsa::TimestampAttestation {
    match tokio::time::timeout(
        Duration::from_secs(3),
        tsa::request_attestation(file_hash_hex, watermark_uid, tsa_dir),
    )
    .await
    {
        Ok(attestation) => attestation,
        Err(_) => {
            log::warn!(
                "trusted timestamp request timed out after 3s; continuing with offline attestation"
            );
            tsa::TimestampAttestation::offline()
        }
    }
}

fn core_watermark_error_to_pipeline(error: watermark_core::WatermarkError) -> PipelineError {
    let code = error.code_str();
    PipelineError::watermark_failure(
        code,
        error.to_string(),
        error.existing_uid().map(ToString::to_string),
    )
}

fn require_identity(app_data_dir: &Path) -> Result<identity::Identity, PipelineError> {
    identity::load_identity(app_data_dir).ok_or_else(|| {
        PipelineError::WatermarkEmbedFailed(
            "[missing_creator_identity] 请先完成创作者身份设置，再生成保护副本。".to_string(),
        )
    })
}

fn parse_sha256_hex_32(value: &str) -> Option<[u8; 32]> {
    let bytes = hex::decode(value).ok()?;
    if bytes.len() != 32 {
        return None;
    }
    let mut output = [0u8; 32];
    output.copy_from_slice(&bytes);
    Some(output)
}

#[cfg(test)]
fn sha256_32_of_bytes(bytes: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(bytes);
    let mut output = [0u8; 32];
    output.copy_from_slice(&digest);
    output
}

fn sha256_of_file_for_payload(file_path: &str) -> Result<[u8; 32], PipelineError> {
    let value = hash::sha256_of_file(file_path).map_err(|error| {
        PipelineError::WatermarkEmbedFailed(format!("hash source file: {error}"))
    })?;
    parse_sha256_hex_32(&value).ok_or_else(|| {
        PipelineError::WatermarkEmbedFailed("invalid source file sha256".to_string())
    })
}

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
pub struct WriteVerificationInfo {
    pub verified: bool,
    pub watermark_uid: String,
    pub revision: u32,
    pub message: String,
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
    pub write_verification: Option<WriteVerificationInfo>,
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
    #[allow(dead_code)]
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

/// Persist a successful pipeline result and its usage ledger entry on a blocking thread.
/// Uses `spawn_blocking` to isolate potential I/O latency from async tasks.
async fn persist_record_and_usage_async(
    app_handle: &AppHandle,
    record: VaultRecord,
    usage_entry: UsageLedgerEntry,
) -> Result<VaultRecord, PipelineError> {
    let handle = app_handle.clone();
    let saved_record =
        tokio::task::spawn_blocking(move || -> Result<VaultRecord, PipelineError> {
            let state = handle.state::<AppState>();
            let mut conn = match state.db.lock() {
                Ok(c) => c,
                Err(e) => {
                    return Err(PipelineError::DatabaseError(format!("DB lock failed: {e}")));
                }
            };
            let record_id = billing::insert_record_and_usage(&mut conn, &record, usage_entry)
                .map_err(|e| {
                    PipelineError::DatabaseError(format!(
                        "Failed to persist record and usage ledger: {e}"
                    ))
                })?;
            let mut record = record;
            record.id = record_id as u32;
            let event = cloud::vault_record_to_cloud_event(&record);
            let event_json = serde_json::to_string(&event).map_err(|e| {
                PipelineError::DatabaseError(format!("Failed to serialize cloud sync event: {e}"))
            })?;
            sync_storage::enqueue_cloud_sync_event(
                &conn,
                &event.client_event_id,
                record.id,
                &event_json,
            )
            .map_err(|e| {
                PipelineError::DatabaseError(format!("Failed to enqueue cloud sync event: {e}"))
            })?;
            Ok(record)
        })
        .await
        .map_err(|e| PipelineError::DatabaseError(format!("join blocking DB write task: {e}")))??;
    crate::commands::sync::trigger_desktop_cloud_sync_after_local_enqueue(app_handle.clone());
    Ok(saved_record)
}

fn build_usage_entry(
    feature_name: &str,
    media_type: &str,
    file_size_bytes: u64,
    entitlement_state: &billing::EntitlementState,
    pipeline_id: &str,
) -> UsageLedgerEntry {
    UsageLedgerEntry::success(
        feature_name,
        media_type,
        file_size_bytes,
        entitlement_state,
        Some(pipeline_id.to_string()),
    )
}

async fn load_entitlement_state(
    app_handle: &AppHandle,
) -> Result<billing::EntitlementState, PipelineError> {
    let handle = app_handle.clone();
    tokio::task::spawn_blocking(
        move || -> Result<billing::EntitlementState, PipelineError> {
            let state = handle.state::<AppState>();
            let conn = state
                .db
                .lock()
                .map_err(|e| PipelineError::DatabaseError(format!("db lock failed: {e}")))?;
            billing::get_entitlement_state(&conn).map_err(|e| {
                PipelineError::DatabaseError(format!("Failed to load entitlement: {e}"))
            })
        },
    )
    .await
    .map_err(|e| PipelineError::DatabaseError(format!("join blocking DB read task: {e}")))?
}

async fn load_latest_record_by_uid(
    app_handle: &AppHandle,
    uid: String,
) -> Result<Option<VaultRecord>, PipelineError> {
    let handle = app_handle.clone();
    tokio::task::spawn_blocking(move || -> Result<Option<VaultRecord>, PipelineError> {
        let state = handle.state::<AppState>();
        let conn = state
            .db
            .lock()
            .map_err(|e| PipelineError::DatabaseError(format!("db lock failed: {e}")))?;
        Ok(queries::find_by_watermark_uid(&conn, &uid))
    })
    .await
    .map_err(|e| PipelineError::DatabaseError(format!("join blocking DB read task: {e}")))?
}

fn rewrite_reason_from_options(options: &TranscodeOptions) -> Option<String> {
    options
        .rewrite_reason
        .as_deref()
        .map(str::trim)
        .filter(|reason| !reason.is_empty())
        .map(ToOwned::to_owned)
}

fn verify_embedded_bytes(
    media_input: MediaInput,
    expected_payload: &WatermarkPayload,
    revision: u32,
) -> Result<WriteVerificationInfo, PipelineError> {
    let extracted =
        WatermarkService::extract(media_input).map_err(core_watermark_error_to_pipeline)?;
    let expected_uid = expected_payload.watermark_uid();
    let actual_uid = extracted.watermark_uid();
    if actual_uid != expected_uid {
        return Err(PipelineError::WatermarkExtractFailed(format!(
            "写入后回读的版权编号不一致，期望 {expected_uid}，实际 {actual_uid}"
        )));
    }
    Ok(WriteVerificationInfo {
        verified: true,
        watermark_uid: actual_uid,
        revision,
        message: format!("已回读验证版权编号，保护副本可取证。写入次数：第 {revision} 次"),
    })
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
    let app_data_dir_for_output = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| PipelineError::FfmpegFailed(format!("resolve app data dir: {e}")))?;
    let output_dir =
        crate::config::resolve_output_dir(&app_data_dir_for_output, &params.input_path);
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| PipelineError::FfmpegFailed(format!("create output dir: {e}")))?;

    // 1. Disk space pre-check. L1 video creates a single protected copy.
    let file_size = std::fs::metadata(&params.input_path)
        .map(|m| m.len())
        .unwrap_or(0);
    system_guard::check_disk_space(&output_dir, file_size, 1)?;

    emit_progress(&app_handle, &params.pipeline_id, "正在分析视频信息...", 5);
    check_cancelled(&app_handle, &params.pipeline_id)?;

    // 3. Probe source via ffprobe
    let probe = ffmpeg::ffprobe_source(&ffmpeg_paths.ffprobe, &input_str).await?;
    let video_stream = probe
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("video"));
    let audio_spec = probe
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("audio"))
        .map(AudioExtractionSpec::from_probe_stream);
    let video_track_audio_spec = Some(AudioExtractionSpec {
        sample_rate: Some(44_100),
        channels: Some(1),
        encoding: AudioExtractionEncoding::PcmS16Le,
    });
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

    extract_audio(
        &ffmpeg_paths.ffmpeg,
        &params.input_path,
        &temp_wav,
        video_track_audio_spec.or(audio_spec),
    )
    .await?;

    // 5. Read WAV, embed watermark, write back
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| PipelineError::FfmpegFailed(format!("resolve app data dir: {e}")))?;
    let identity = require_identity(&app_data_dir)?;
    let creator_display_name = creator_display_name_for_record(&app_data_dir);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let media_sha256 = sha256_of_file_for_payload(&input_str)?;

    // Parse AI content flags from options
    let ai_flags = parse_ai_flags(&params.options.ai_content);

    let revision = 1;
    let original_hash_hex = media_sha256_hex(media_sha256);
    let reserved_watermark = try_reserve_watermark_id(
        app_data_dir.clone(),
        params.pipeline_id.clone(),
        "video_audio_track".to_string(),
        original_hash_hex.clone(),
        None,
        revision,
    )
    .await;
    let payload = build_v2_watermark_payload(
        &identity.creator_display_name,
        timestamp,
        media_sha256,
        ai_flags,
        WatermarkMediaType::VideoAudioTrack,
        None,
        revision,
        reserved_watermark.as_ref(),
    )?;

    let wav_bytes = std::fs::read(&temp_wav)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("read wav bytes: {e}")))?;
    let embedded = WatermarkService::embed(
        MediaInput::AudioWavBytes { bytes: wav_bytes },
        &payload,
        EmbedOptions {
            audio_protection_mode: AudioProtectionMode::VideoTrack,
            ..EmbedOptions::default()
        },
    )
    .map_err(core_watermark_error_to_pipeline)?;

    let MediaOutput::AudioWavBytes { bytes } = embedded else {
        return Err(PipelineError::WatermarkEmbedFailed(
            "unexpected non-audio output from watermark service".into(),
        ));
    };
    std::fs::write(&watermarked_wav, bytes)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("write wav bytes: {e}")))?;

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
        duration_confirmed: duration_secs > 0.0,
        sample_rate: None,
        channels: None,
        watermark_eligible: None,
        file_size_bytes: file_size,
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

    // 7. Generate one L1 video-audio-track protected copy. This avoids
    // platform-specific aspect presets and keeps the formal workflow focused
    // on blind watermarking.
    let source_stem = params
        .input_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let out_extension = output_extension_for_video_input(&params.input_path);
    let out_path = output_dir.join(format!("{source_stem}_保护副本.{out_extension}"));
    emit_progress(
        &app_handle,
        &params.pipeline_id,
        "正在生成视频音轨保护副本",
        35,
    );
    remux_video_with_protected_audio(
        &ffmpeg_paths.ffmpeg,
        &params.input_path,
        &watermarked_wav,
        &out_path,
        duration_secs,
        &app_handle,
        &params.pipeline_id,
    )
    .await?;

    // 8. Compute file hash and insert vault record
    let sha256 = hash::sha256_of_file(&input_str).unwrap_or_default();
    let protected_copy_path = out_path.to_string_lossy().to_string();
    let protected_copy_hash = hash::sha256_of_file(&protected_copy_path).unwrap_or_default();
    let protected_copy_name = out_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string());
    emit_progress(&app_handle, &params.pipeline_id, "正在回读验证版权编号", 82);
    let write_verification = extract_from_video_audio_for_verification(
        &ffmpeg_paths.ffmpeg,
        &ffmpeg_paths.ffprobe,
        &out_path,
        duration_secs,
        &app_handle,
        &params.pipeline_id,
        audio_spec,
    )
    .await
    .and_then(|bytes| {
        verify_embedded_bytes(MediaInput::AudioWavBytes { bytes }, &payload, revision)
    })?;
    let write_verification_status = if write_verification.verified {
        "verified"
    } else {
        "failed"
    };
    let confirmed_registry = if reserved_watermark.is_some() {
        try_confirm_watermark_id(
            app_data_dir.clone(),
            payload.watermark_uid(),
            sha256.clone(),
            Some(protected_copy_hash.clone()),
            write_verification_status.to_string(),
        )
        .await
    } else {
        None
    };
    let registry_response = confirmed_registry.as_ref().or_else(|| {
        reserved_watermark
            .as_ref()
            .map(|reserved| &reserved.response)
    });
    let process_time_ms = start.elapsed().as_millis() as u64;

    let record = VaultRecord {
        id: 0,
        original_hash: sha256.clone(),
        file_name: source_meta.file_name.clone(),
        created_at: chrono::Utc::now().to_rfc3339(),
        duration_secs,
        resolution: format!("{}x{}", width, height),
        watermark_uid: payload.watermark_uid(),
        creator_display_name,
        thumbnail_path: None,
        output_douyin: None,
        output_bilibili: None,
        output_xhs: None,
        is_hdr_source: is_hdr,
        hw_encoder_used: Some(hw_info.preferred_encoder.clone()),
        process_time_ms: Some(process_time_ms),
        tsa_token_path: None,
        network_time: None,
        tsa_source: None,
        tsa_request_nonce: None,
        is_ai_generated: ai_flags.is_ai_generated,
        ai_training_permission: Some(format!("{:?}", ai_flags.training_permission).to_lowercase()),
        ai_generation_method: Some(format!("{:?}", ai_flags.generation_method).to_lowercase()),
        human_modification_level: Some(
            format!("{:?}", ai_flags.human_modification_level).to_lowercase(),
        ),
        authenticity_claim: Some(format!("{:?}", ai_flags.authenticity_claim).to_lowercase()),
        custom_metadata: params
            .options
            .ai_content
            .as_ref()
            .and_then(|ai| ai.custom_rights_statement.clone()),
        output_douyin_hash: None,
        output_bilibili_hash: None,
        output_xhs_hash: None,
        protected_copy_name,
        protected_copy_path: Some(protected_copy_path),
        protected_copy_hash: Some(protected_copy_hash),
        output_strategy: "minimal_required_change".to_string(),
        work_source_declaration: declaration_work_source(&params.options),
        training_permission_declaration: declaration_training_permission(&params.options),
        creation_method_declaration: declaration_creation_method(&params.options),
        human_edit_level_declaration: declaration_human_edit_level(&params.options),
        authenticity_claim_declaration: declaration_authenticity_claim(&params.options),
        custom_rights_statement: declaration_custom_rights_statement(&params.options),
        parent_watermark_uid: None,
        revision,
        rewrite_reason: None,
        write_verification_status: Some(write_verification_status.to_string()),
        write_verification_message: Some(write_verification.message.clone()),
        write_verification_at: Some(chrono::Utc::now().to_rfc3339()),
        payload_protocol_version: 3,
        payload_bytes_length: watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32,
        watermark_id_issue_mode: registry_response
            .map(|response| response.watermark_id_issue_mode.clone())
            .unwrap_or_else(|| "offline_generated".to_string()),
        watermark_id_registry_status: registry_response
            .map(|response| response.registry_status.clone())
            .unwrap_or_else(|| "pending_registration".to_string()),
        watermark_id_registry_receipt: registry_response
            .map(|response| response.registry_receipt.clone()),
        payload_auth_status: if write_verification.verified {
            "verified".to_string()
        } else {
            "failed".to_string()
        },
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

    // Request trusted timestamp (non-blocking, best-effort)
    let tsa_dir = app_handle
        .path()
        .app_data_dir()
        .map(|d| d.join("tsa_tokens"))
        .unwrap_or_default();
    let attestation = if crate::telemetry::is_network_enabled(&app_data_dir)
        && crate::telemetry::is_acknowledged(&app_data_dir)
    {
        request_attestation_quick(&sha256, &record.watermark_uid, &tsa_dir).await
    } else {
        tsa::TimestampAttestation::offline()
    };
    let mut record = record;
    record.tsa_token_path = attestation.tsa_token_path;
    record.network_time = attestation.network_time;
    record.tsa_source = attestation.tsa_source.or(attestation.network_time_source);
    record.tsa_request_nonce = attestation.tsa_request_nonce;

    let entitlement_state = load_entitlement_state(&app_handle).await?;
    let usage_entry = build_usage_entry(
        "watermark_video",
        "video",
        file_size,
        &entitlement_state,
        &params.pipeline_id,
    );
    record = persist_record_and_usage_async(&app_handle, record, usage_entry).await?;

    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        telemetry::anonymous::record_success_event(
            &app_data_dir,
            "watermark_video",
            "video",
            file_size,
            Some(process_time_ms),
            Some(params.pipeline_id.clone()),
        );
    }

    // 9. Cleanup temp files
    let _ = ufs::cleanup_temp_dir(&params.pipeline_id);

    let complete_payload = PipelineCompletePayload {
        pipeline_id: params.pipeline_id.clone(),
        watermark_uid: record.watermark_uid.clone(),
        process_time_ms,
        encoder_used: hw_info.preferred_encoder.clone(),
        outputs: vec![OutputFileInfo {
            platform: "video_audio_track".to_string(),
            path: out_path.to_string_lossy().to_string(),
            size_mb: std::fs::metadata(&out_path)
                .map(|m| (m.len() as f64 / 1024.0 / 1024.0 * 10.0).round() / 10.0)
                .unwrap_or(0.0),
            resolution: format!("{}x{}", width, height),
            fps,
        }],
        vault_record: record,
        write_verification: Some(write_verification),
    };
    let _ = app_handle.emit("pipeline-complete", &complete_payload);

    emit_progress(&app_handle, &params.pipeline_id, "全部文件已就绪", 100);
    Ok(())
}

// ---------------------------------------------------------------------------
// Image pipeline
// ---------------------------------------------------------------------------

async fn run_image_embed_with_progress(
    app_handle: &AppHandle,
    pipeline_id: String,
    input_bytes: Vec<u8>,
    payload: WatermarkPayload,
    options: EmbedOptions,
) -> Result<MediaOutput, PipelineError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        WatermarkService::embed(
            MediaInput::ImageBytes { bytes: input_bytes },
            &payload,
            options,
        )
        .map_err(core_watermark_error_to_pipeline)
    });

    tokio::pin!(task);

    let heartbeat = [
        (44, "正在写入图片盲水印，大图处理可能需要十几秒"),
        (50, "正在增强保护副本可验证性"),
        (56, "正在完成图片频域写入"),
        (62, "正在收尾图片盲水印写入"),
    ];
    let mut heartbeat_index = 0usize;
    let mut interval = tokio::time::interval(Duration::from_secs(3));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        tokio::select! {
            result = &mut task => {
                return result
                    .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("image embed task failed: {e}")))?;
            }
            _ = interval.tick(), if heartbeat_index < heartbeat.len() => {
                let (percent, stage) = heartbeat[heartbeat_index];
                heartbeat_index += 1;
                emit_progress(app_handle, &pipeline_id, stage, percent);
            }
        }
    }
}

fn log_image_stage_timing(stage: &str, start: Instant, last_mark: &mut Instant) {
    let now = Instant::now();
    log::info!(
        "image_pipeline_perf stage={} stage_ms={} total_ms={}",
        stage,
        now.duration_since(*last_mark).as_millis(),
        now.duration_since(start).as_millis()
    );
    *last_mark = now;
}

async fn run_image_pipeline(
    params: PipelineParams,
    app_handle: AppHandle,
) -> Result<(), PipelineError> {
    let start = Instant::now();
    let mut last_perf_mark = start;
    let input_str = params.input_path.to_string_lossy().to_string();
    let input_extension = params
        .input_path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(input_extension.as_str(), "png" | "jpg" | "jpeg" | "webp") {
        return Err(PipelineError::WatermarkEmbedFailed(
            "image format unsupported: only PNG, JPEG, and WebP are supported".into(),
        ));
    }
    let input_size_bytes = std::fs::metadata(&params.input_path)
        .map(|m| m.len())
        .unwrap_or(0);
    if watermark_core::validate_image_protection_file_size(input_size_bytes as usize).is_err() {
        return Err(PipelineError::WatermarkEmbedFailed(
            "image file size limit exceeded: maximum 512 MiB".into(),
        ));
    }
    let (input_width, input_height) =
        image::image_dimensions(&params.input_path).map_err(|error| {
            PipelineError::WatermarkEmbedFailed(format!("read image dimensions: {error}"))
        })?;
    if matches!(
        watermark_core::validate_image_protection_input(input_width, input_height),
        Err("image_pixel_limit_exceeded")
    ) {
        return Err(PipelineError::WatermarkEmbedFailed(
            "image pixel limit exceeded: maximum 100 MP".into(),
        ));
    }

    check_cancelled(&app_handle, &params.pipeline_id)?;
    emit_progress(&app_handle, &params.pipeline_id, "正在准备图片版权载荷", 8);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| PipelineError::FfmpegFailed(format!("resolve app data dir: {e}")))?;
    let identity = require_identity(&app_data_dir)?;
    let creator_display_name = creator_display_name_for_record(&app_data_dir);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let media_sha256 = sha256_of_file_for_payload(&input_str)?;

    // Parse AI content flags from options
    let ai_flags = parse_ai_flags(&params.options.ai_content);
    log_image_stage_timing("prepare_payload", start, &mut last_perf_mark);
    emit_progress(&app_handle, &params.pipeline_id, "正在读取原图", 18);

    // Output path: same directory, with _watermarked suffix
    let stem = params
        .input_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    // 图片保护采用取证优先策略：统一输出 PNG，减少有损压缩对后续取证的影响。
    let out_ext = "png";
    let output_dir = crate::config::resolve_output_dir(&app_data_dir, &params.input_path);
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("create output dir: {e}")))?;
    let output_path = output_dir.join(format!("{stem}_watermarked.{out_ext}"));

    check_cancelled(&app_handle, &params.pipeline_id)?;

    let input_bytes = std::fs::read(&params.input_path)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("read image bytes: {e}")))?;
    log_image_stage_timing("read_source_bytes", start, &mut last_perf_mark);
    let image_format = image_output_format_for_ext(out_ext);
    emit_progress(&app_handle, &params.pipeline_id, "正在检查重写状态", 28);
    let parent_watermark_uid = if params.options.allow_rewrite {
        WatermarkService::extract(MediaInput::ImageBytes {
            bytes: input_bytes.clone(),
        })
        .ok()
        .map(|payload| payload.watermark_uid())
    } else {
        None
    };
    let parent_record = if let Some(uid) = parent_watermark_uid.clone() {
        load_latest_record_by_uid(&app_handle, uid).await?
    } else {
        None
    };
    let revision = parent_record
        .as_ref()
        .map(|record| record.revision.saturating_add(1))
        .unwrap_or(1);
    let original_hash_hex = media_sha256_hex(media_sha256);
    let reserved_watermark = try_reserve_watermark_id(
        app_data_dir.clone(),
        params.pipeline_id.clone(),
        "image".to_string(),
        original_hash_hex.clone(),
        parent_watermark_uid.clone(),
        revision,
    )
    .await;
    let payload = build_v2_watermark_payload(
        &identity.creator_display_name,
        timestamp,
        media_sha256,
        ai_flags,
        WatermarkMediaType::Image,
        parent_watermark_uid.as_deref(),
        revision,
        reserved_watermark.as_ref(),
    )?;
    log_image_stage_timing("inspect_rewrite", start, &mut last_perf_mark);
    emit_progress(&app_handle, &params.pipeline_id, "正在写入图片盲水印", 38);
    let embedded = run_image_embed_with_progress(
        &app_handle,
        params.pipeline_id.clone(),
        input_bytes,
        payload.clone(),
        EmbedOptions {
            image_output_format: image_format,
            allow_rewrite: params.options.allow_rewrite,
            ..EmbedOptions::default()
        },
    )
    .await?;
    log_image_stage_timing("embed_watermark", start, &mut last_perf_mark);

    let MediaOutput::ImageBytes { bytes, .. } = embedded else {
        return Err(PipelineError::WatermarkEmbedFailed(
            "unexpected non-image output from watermark service".into(),
        ));
    };
    emit_progress(&app_handle, &params.pipeline_id, "正在保存保护副本", 68);
    std::fs::write(&output_path, &bytes)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("write image bytes: {e}")))?;
    log_image_stage_timing("write_output_bytes", start, &mut last_perf_mark);

    check_cancelled(&app_handle, &params.pipeline_id)?;
    emit_progress(&app_handle, &params.pipeline_id, "正在回读验证版权编号", 78);

    // Read image dimensions for the record
    let (width, height) = image::image_dimensions(&params.input_path).unwrap_or((0, 0));
    let sha256 = hash::sha256_of_file(&input_str).unwrap_or_default();

    // Compute output file hash
    let output_hash =
        hash::sha256_of_file(output_path.to_string_lossy().as_ref()).unwrap_or_default();
    log_image_stage_timing("metadata_and_hash", start, &mut last_perf_mark);
    let write_verification = verify_embedded_bytes(
        MediaInput::ImageBytes {
            bytes: bytes.clone(),
        },
        &payload,
        revision,
    )?;
    let write_verification_status = if write_verification.verified {
        "verified"
    } else {
        "failed"
    };
    let confirmed_registry = if reserved_watermark.is_some() {
        try_confirm_watermark_id(
            app_data_dir.clone(),
            payload.watermark_uid(),
            sha256.clone(),
            Some(output_hash.clone()),
            write_verification_status.to_string(),
        )
        .await
    } else {
        None
    };
    let registry_response = confirmed_registry.as_ref().or_else(|| {
        reserved_watermark
            .as_ref()
            .map(|reserved| &reserved.response)
    });
    let process_time_ms = start.elapsed().as_millis() as u64;
    log_image_stage_timing("verify_embedded_bytes", start, &mut last_perf_mark);
    emit_progress(&app_handle, &params.pipeline_id, "正在保存版权记录", 88);

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
        creator_display_name,
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
        is_ai_generated: ai_flags.is_ai_generated,
        ai_training_permission: Some(format!("{:?}", ai_flags.training_permission).to_lowercase()),
        ai_generation_method: Some(format!("{:?}", ai_flags.generation_method).to_lowercase()),
        human_modification_level: Some(
            format!("{:?}", ai_flags.human_modification_level).to_lowercase(),
        ),
        authenticity_claim: Some(format!("{:?}", ai_flags.authenticity_claim).to_lowercase()),
        custom_metadata: params
            .options
            .ai_content
            .as_ref()
            .and_then(|ai| ai.custom_rights_statement.clone()),
        output_douyin_hash: None,
        output_bilibili_hash: None,
        output_xhs_hash: None,
        protected_copy_name: output_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string()),
        protected_copy_path: Some(output_path.to_string_lossy().to_string()),
        protected_copy_hash: Some(output_hash),
        output_strategy: "minimal_required_change".to_string(),
        work_source_declaration: declaration_work_source(&params.options),
        training_permission_declaration: declaration_training_permission(&params.options),
        creation_method_declaration: declaration_creation_method(&params.options),
        human_edit_level_declaration: declaration_human_edit_level(&params.options),
        authenticity_claim_declaration: declaration_authenticity_claim(&params.options),
        custom_rights_statement: declaration_custom_rights_statement(&params.options),
        parent_watermark_uid,
        revision,
        rewrite_reason: rewrite_reason_from_options(&params.options),
        write_verification_status: Some(write_verification_status.to_string()),
        write_verification_message: Some(write_verification.message.clone()),
        write_verification_at: Some(chrono::Utc::now().to_rfc3339()),
        payload_protocol_version: 3,
        payload_bytes_length: watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32,
        watermark_id_issue_mode: registry_response
            .map(|response| response.watermark_id_issue_mode.clone())
            .unwrap_or_else(|| "offline_generated".to_string()),
        watermark_id_registry_status: registry_response
            .map(|response| response.registry_status.clone())
            .unwrap_or_else(|| "pending_registration".to_string()),
        watermark_id_registry_receipt: registry_response
            .map(|response| response.registry_receipt.clone()),
        payload_auth_status: if write_verification.verified {
            "verified".to_string()
        } else {
            "failed".to_string()
        },
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

    // Request trusted timestamp (non-blocking, best-effort)
    let tsa_dir = app_handle
        .path()
        .app_data_dir()
        .map(|d| d.join("tsa_tokens"))
        .unwrap_or_default();
    let attestation = if crate::telemetry::is_network_enabled(&app_data_dir)
        && crate::telemetry::is_acknowledged(&app_data_dir)
    {
        request_attestation_quick(&record.original_hash, &record.watermark_uid, &tsa_dir).await
    } else {
        tsa::TimestampAttestation::offline()
    };
    let mut record = record;
    record.tsa_token_path = attestation.tsa_token_path;
    record.network_time = attestation.network_time;
    record.tsa_source = attestation.tsa_source.or(attestation.network_time_source);
    record.tsa_request_nonce = attestation.tsa_request_nonce;

    check_cancelled(&app_handle, &params.pipeline_id)?;
    let entitlement_state = load_entitlement_state(&app_handle).await?;
    let usage_entry = build_usage_entry(
        "watermark_image",
        "image",
        input_size_bytes,
        &entitlement_state,
        &params.pipeline_id,
    );
    record = persist_record_and_usage_async(&app_handle, record, usage_entry).await?;
    log_image_stage_timing("persist_record_and_usage", start, &mut last_perf_mark);
    emit_progress(&app_handle, &params.pipeline_id, "正在生成存证摘要", 94);

    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        telemetry::anonymous::record_success_event(
            &app_data_dir,
            "watermark_image",
            "image",
            input_size_bytes,
            Some(process_time_ms),
            Some(params.pipeline_id.clone()),
        );
    }

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
        write_verification: Some(write_verification),
    };
    let _ = app_handle.emit("pipeline-complete", &complete_payload);

    emit_progress(&app_handle, &params.pipeline_id, "图片处理完成", 100);
    log::info!(
        "image_pipeline_perf stage=complete total_ms={}",
        start.elapsed().as_millis()
    );

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
    let input_size_bytes = std::fs::metadata(&params.input_path)
        .map(|m| m.len())
        .unwrap_or(0);
    validate_audio_protection_file_size(input_size_bytes)
        .map_err(PipelineError::WatermarkEmbedFailed)?;
    let probe = ffmpeg::ffprobe_source(&ffmpeg_paths.ffprobe, &input_str).await?;
    let audio_stream = probe
        .streams
        .iter()
        .find(|s| s.codec_type.as_deref() == Some("audio"))
        .ok_or_else(|| {
            PipelineError::WatermarkEmbedFailed("audio_protection_spec_unknown".to_string())
        })?;
    let audio_spec = Some(AudioExtractionSpec::from_probe_stream(audio_stream));
    let duration = probe
        .format
        .as_ref()
        .and_then(|format| format.duration)
        .filter(|duration| duration.is_finite() && *duration > 0.0);
    let Some(duration_secs) = duration else {
        return Err(PipelineError::WatermarkEmbedFailed(
            "audio_protection_duration_unknown".to_string(),
        ));
    };
    let sample_rate = audio_stream.sample_rate.ok_or_else(|| {
        PipelineError::WatermarkEmbedFailed("audio_protection_spec_unknown".to_string())
    })?;
    let channels = audio_stream.channels.ok_or_else(|| {
        PipelineError::WatermarkEmbedFailed("audio_protection_spec_unknown".to_string())
    })?;
    validate_audio_protection_input(
        sample_rate,
        channels,
        duration_secs,
        MIN_AUDIO_PROTECTION_SECONDS,
    )
    .map_err(|code| PipelineError::WatermarkEmbedFailed(code.to_string()))?;

    check_cancelled(&app_handle, &params.pipeline_id)?;
    emit_progress(&app_handle, &params.pipeline_id, "音频水印嵌入中", 10);

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| PipelineError::FfmpegFailed(format!("resolve app data dir: {e}")))?;
    let identity = require_identity(&app_data_dir)?;
    let creator_display_name = creator_display_name_for_record(&app_data_dir);
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let media_sha256 = sha256_of_file_for_payload(&input_str)?;

    // Parse AI content flags from options
    let ai_flags = parse_ai_flags(&params.options.ai_content);

    // Convert input to PCM WAV if needed, then embed watermark
    let temp_dir = ufs::create_temp_dir(&params.pipeline_id)
        .map_err(|e| PipelineError::FfmpegFailed(format!("create temp dir: {e}")))?;
    let temp_wav = temp_dir.join("input.wav");

    check_cancelled(&app_handle, &params.pipeline_id)?;
    // Convert to PCM WAV while preserving source sample rate and channel count when available.
    extract_audio(
        &ffmpeg_paths.ffmpeg,
        &params.input_path,
        &temp_wav,
        audio_spec,
    )
    .await?;

    check_cancelled(&app_handle, &params.pipeline_id)?;
    emit_progress(&app_handle, &params.pipeline_id, "频域水印写入中", 40);

    let stem = params
        .input_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    let output_dir = crate::config::resolve_output_dir(&app_data_dir, &params.input_path);
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("create output dir: {e}")))?;
    let output_path = output_dir.join(format!("{stem}_watermarked.wav"));

    check_cancelled(&app_handle, &params.pipeline_id)?;

    let wav_bytes = std::fs::read(&temp_wav)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("read wav bytes: {e}")))?;
    let parent_watermark_uid = if params.options.allow_rewrite {
        WatermarkService::extract(MediaInput::AudioWavBytes {
            bytes: wav_bytes.clone(),
        })
        .ok()
        .map(|payload| payload.watermark_uid())
    } else {
        None
    };
    let parent_record = if let Some(uid) = parent_watermark_uid.clone() {
        load_latest_record_by_uid(&app_handle, uid).await?
    } else {
        None
    };
    let revision = parent_record
        .as_ref()
        .map(|record| record.revision.saturating_add(1))
        .unwrap_or(1);
    let original_hash_hex = media_sha256_hex(media_sha256);
    let reserved_watermark = try_reserve_watermark_id(
        app_data_dir.clone(),
        params.pipeline_id.clone(),
        "audio".to_string(),
        original_hash_hex.clone(),
        parent_watermark_uid.clone(),
        revision,
    )
    .await;
    let payload = build_v2_watermark_payload(
        &identity.creator_display_name,
        timestamp,
        media_sha256,
        ai_flags,
        WatermarkMediaType::Audio,
        parent_watermark_uid.as_deref(),
        revision,
        reserved_watermark.as_ref(),
    )?;
    let embedded = WatermarkService::embed(
        MediaInput::AudioWavBytes { bytes: wav_bytes },
        &payload,
        EmbedOptions {
            allow_rewrite: params.options.allow_rewrite,
            ..EmbedOptions::default()
        },
    )
    .map_err(core_watermark_error_to_pipeline)?;

    let MediaOutput::AudioWavBytes { bytes } = embedded else {
        return Err(PipelineError::WatermarkEmbedFailed(
            "unexpected non-audio output from watermark service".into(),
        ));
    };
    std::fs::write(&output_path, &bytes)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("write wav bytes: {e}")))?;

    check_cancelled(&app_handle, &params.pipeline_id)?;
    emit_progress(&app_handle, &params.pipeline_id, "写入验收中", 80);

    let sha256 = hash::sha256_of_file(&input_str).unwrap_or_default();

    // Compute output file hash
    let output_hash =
        hash::sha256_of_file(output_path.to_string_lossy().as_ref()).unwrap_or_default();
    let write_verification = verify_embedded_bytes(
        MediaInput::AudioWavBytes {
            bytes: bytes.clone(),
        },
        &payload,
        revision,
    )?;
    let write_verification_status = if write_verification.verified {
        "verified"
    } else {
        "failed"
    };
    let confirmed_registry = if reserved_watermark.is_some() {
        try_confirm_watermark_id(
            app_data_dir.clone(),
            payload.watermark_uid(),
            sha256.clone(),
            Some(output_hash.clone()),
            write_verification_status.to_string(),
        )
        .await
    } else {
        None
    };
    let registry_response = confirmed_registry.as_ref().or_else(|| {
        reserved_watermark
            .as_ref()
            .map(|reserved| &reserved.response)
    });
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
        duration_secs,
        resolution: String::new(),
        watermark_uid: payload.watermark_uid(),
        creator_display_name,
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
        is_ai_generated: ai_flags.is_ai_generated,
        ai_training_permission: Some(format!("{:?}", ai_flags.training_permission).to_lowercase()),
        ai_generation_method: Some(format!("{:?}", ai_flags.generation_method).to_lowercase()),
        human_modification_level: Some(
            format!("{:?}", ai_flags.human_modification_level).to_lowercase(),
        ),
        authenticity_claim: Some(format!("{:?}", ai_flags.authenticity_claim).to_lowercase()),
        custom_metadata: params
            .options
            .ai_content
            .as_ref()
            .and_then(|ai| ai.custom_rights_statement.clone()),
        output_douyin_hash: None,
        output_bilibili_hash: None,
        output_xhs_hash: None,
        protected_copy_name: output_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string()),
        protected_copy_path: Some(output_path.to_string_lossy().to_string()),
        protected_copy_hash: Some(output_hash),
        output_strategy: "minimal_required_change".to_string(),
        work_source_declaration: declaration_work_source(&params.options),
        training_permission_declaration: declaration_training_permission(&params.options),
        creation_method_declaration: declaration_creation_method(&params.options),
        human_edit_level_declaration: declaration_human_edit_level(&params.options),
        authenticity_claim_declaration: declaration_authenticity_claim(&params.options),
        custom_rights_statement: declaration_custom_rights_statement(&params.options),
        parent_watermark_uid,
        revision,
        rewrite_reason: rewrite_reason_from_options(&params.options),
        write_verification_status: Some(write_verification_status.to_string()),
        write_verification_message: Some(write_verification.message.clone()),
        write_verification_at: Some(chrono::Utc::now().to_rfc3339()),
        payload_protocol_version: 3,
        payload_bytes_length: watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32,
        watermark_id_issue_mode: registry_response
            .map(|response| response.watermark_id_issue_mode.clone())
            .unwrap_or_else(|| "offline_generated".to_string()),
        watermark_id_registry_status: registry_response
            .map(|response| response.registry_status.clone())
            .unwrap_or_else(|| "pending_registration".to_string()),
        watermark_id_registry_receipt: registry_response
            .map(|response| response.registry_receipt.clone()),
        payload_auth_status: if write_verification.verified {
            "verified".to_string()
        } else {
            "failed".to_string()
        },
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

    // Request trusted timestamp (non-blocking, best-effort)
    let tsa_dir = app_handle
        .path()
        .app_data_dir()
        .map(|d| d.join("tsa_tokens"))
        .unwrap_or_default();
    let attestation = if crate::telemetry::is_network_enabled(&app_data_dir)
        && crate::telemetry::is_acknowledged(&app_data_dir)
    {
        request_attestation_quick(&record.original_hash, &record.watermark_uid, &tsa_dir).await
    } else {
        tsa::TimestampAttestation::offline()
    };
    let mut record = record;
    record.tsa_token_path = attestation.tsa_token_path;
    record.network_time = attestation.network_time;
    record.tsa_source = attestation.tsa_source.or(attestation.network_time_source);
    record.tsa_request_nonce = attestation.tsa_request_nonce;

    check_cancelled(&app_handle, &params.pipeline_id)?;
    let entitlement_state = load_entitlement_state(&app_handle).await?;
    let usage_entry = build_usage_entry(
        "watermark_audio",
        "audio",
        input_size_bytes,
        &entitlement_state,
        &params.pipeline_id,
    );
    record = persist_record_and_usage_async(&app_handle, record, usage_entry).await?;

    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        telemetry::anonymous::record_success_event(
            &app_data_dir,
            "watermark_audio",
            "audio",
            input_size_bytes,
            Some(process_time_ms),
            Some(params.pipeline_id.clone()),
        );
    }

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
        write_verification: Some(write_verification),
    };
    let _ = app_handle.emit("pipeline-complete", &complete_payload);

    emit_progress(&app_handle, &params.pipeline_id, "音频处理完成", 100);

    let _ = ufs::cleanup_temp_dir(&params.pipeline_id);
    Ok(())
}

// ---------------------------------------------------------------------------
// FFmpeg helpers
// ---------------------------------------------------------------------------

/// Extract audio to a PCM WAV while preserving source sample rate, channel count and supported bit depth.
/// Uses `-async 1` to align audio timestamps and fill gaps from VFR sources.
async fn extract_audio(
    ffmpeg: &Path,
    input: &Path,
    output_wav: &Path,
    spec: Option<AudioExtractionSpec>,
) -> Result<(), PipelineError> {
    let mut args: Vec<String> = vec![
        "-y".into(),
        "-i".into(),
        input.to_string_lossy().to_string(),
        "-vn".into(),
    ];
    if let Some(spec) = spec {
        spec.apply_ffmpeg_args(&mut args);
        let codec = spec.ffmpeg_codec().ok_or_else(|| {
            PipelineError::FfmpegFailed(
                "unsupported floating-point source precision; only 32-bit float WAV output is supported"
                    .to_string(),
            )
        })?;
        args.extend(["-acodec".into(), codec.into()]);
    } else {
        args.extend(["-acodec".into(), "pcm_s16le".into()]);
    }
    args.extend([
        "-af".into(),
        "aresample=async=1".into(), // Fix A/V sync: align timestamps for VFR sources
        output_wav.to_string_lossy().to_string(),
    ]);

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

fn output_extension_for_video_input(input: &Path) -> &'static str {
    match input
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase()
        .as_str()
    {
        "webm" => "webm",
        _ => "mp4",
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AudioExtractionSpec {
    sample_rate: Option<u32>,
    channels: Option<u16>,
    encoding: AudioExtractionEncoding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AudioExtractionEncoding {
    PcmS16Le,
    PcmS24Le,
    PcmS32Le,
    PcmF32Le,
    Unsupported,
}

impl AudioExtractionSpec {
    fn from_probe_stream(stream: &ffmpeg::FfprobeStream) -> Self {
        let sample_format = stream
            .sample_fmt
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase();
        let bits_per_sample = stream
            .bits_per_raw_sample
            .or(stream.bits_per_sample)
            .unwrap_or(16);
        let encoding = if sample_format.starts_with("dbl") {
            AudioExtractionEncoding::Unsupported
        } else if sample_format.starts_with("flt") {
            AudioExtractionEncoding::PcmF32Le
        } else if (24..32).contains(&bits_per_sample) {
            AudioExtractionEncoding::PcmS24Le
        } else if bits_per_sample >= 32 || sample_format.starts_with("s32") {
            AudioExtractionEncoding::PcmS32Le
        } else {
            AudioExtractionEncoding::PcmS16Le
        };
        Self {
            sample_rate: stream.sample_rate.filter(|value| *value > 0),
            channels: stream.channels.filter(|value| *value > 0),
            encoding,
        }
    }

    fn apply_ffmpeg_args(&self, args: &mut Vec<String>) {
        if let Some(sample_rate) = self.sample_rate {
            args.extend(["-ar".into(), sample_rate.to_string()]);
        }
        if let Some(channels) = self.channels {
            args.extend(["-ac".into(), channels.to_string()]);
        }
    }

    fn ffmpeg_codec(&self) -> Option<&'static str> {
        Some(match self.encoding {
            AudioExtractionEncoding::PcmS16Le => "pcm_s16le",
            AudioExtractionEncoding::PcmS24Le => "pcm_s24le",
            AudioExtractionEncoding::PcmS32Le => "pcm_s32le",
            AudioExtractionEncoding::PcmF32Le => "pcm_f32le",
            AudioExtractionEncoding::Unsupported => return None,
        })
    }
}

fn image_output_format_for_ext(ext: &str) -> ImageOutputFormat {
    match ext {
        "jpg" | "jpeg" => ImageOutputFormat::Png,
        "png" => ImageOutputFormat::Png,
        "webp" => ImageOutputFormat::WebP,
        "bmp" => ImageOutputFormat::Bmp,
        "tif" | "tiff" => ImageOutputFormat::Tiff,
        _ => ImageOutputFormat::Png,
    }
}

async fn remux_video_with_protected_audio(
    ffmpeg: &Path,
    video_input: &Path,
    audio_input: &Path,
    output: &Path,
    total_duration: f64,
    app_handle: &AppHandle,
    pipeline_id: &str,
) -> Result<(), PipelineError> {
    let state = app_handle.state::<AppState>();
    let _permit = state
        .ffmpeg_semaphore
        .acquire()
        .await
        .map_err(|_| PipelineError::FfmpegFailed("semaphore closed".into()))?;
    let output_extension = output
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
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
        "copy".into(),
        "-stats_period".into(),
        "1".into(),
        "-shortest".into(),
    ];
    if output_extension == "webm" {
        args.extend([
            "-c:a".into(),
            "libopus".into(),
            "-ar".into(),
            "48000".into(),
            "-ac".into(),
            "2".into(),
            "-application".into(),
            "audio".into(),
            "-vbr".into(),
            "on".into(),
            "-compression_level".into(),
            "10".into(),
            "-b:a".into(),
            "160k".into(),
        ]);
    } else {
        args.extend([
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            "192k".into(),
            "-movflags".into(),
            "+faststart".into(),
        ]);
    }
    args.push(output.to_string_lossy().to_string());
    let mut child = ffmpeg::spawn_ffmpeg(ffmpeg, &args).await?;
    if let Some(stderr) = child.child.stderr.take() {
        let reader = BufReader::new(stderr);
        let mut lines = reader.lines();
        let mut last_emit = Instant::now() - Duration::from_millis(200);
        while let Ok(Some(line)) = lines.next_line().await {
            if let Some(progress) = ffmpeg::parse_progress_line(&line, total_duration) {
                let now = Instant::now();
                if now.duration_since(last_emit) >= Duration::from_millis(250) {
                    last_emit = now;
                    emit_progress(
                        app_handle,
                        pipeline_id,
                        "正在生成视频音轨保护副本",
                        (35.0 + progress * 42.0) as u8,
                    );
                }
            }
            if is_cancelled(app_handle, pipeline_id) {
                let _ = child.child.kill().await;
                let _ = std::fs::remove_file(output);
                return Err(PipelineError::Cancelled);
            }
        }
    }
    let status = child.child.wait().await.map_err(|e| {
        let _ = std::fs::remove_file(output);
        PipelineError::FfmpegFailed(format!("wait video protected copy: {e}"))
    })?;
    if !status.success() {
        let _ = std::fs::remove_file(output);
        return Err(PipelineError::FfmpegFailed(format!(
            "video protected copy generation exited with {status}"
        )));
    }
    Ok(())
}

async fn extract_from_video_audio_for_verification(
    ffmpeg: &Path,
    ffprobe: &Path,
    input: &Path,
    total_duration: f64,
    app_handle: &AppHandle,
    pipeline_id: &str,
    source_audio_spec: Option<AudioExtractionSpec>,
) -> Result<Vec<u8>, PipelineError> {
    let temp_dir = ufs::create_temp_dir(&format!("{pipeline_id}-verify"))
        .map_err(|e| PipelineError::FfmpegFailed(format!("create verify temp dir: {e}")))?;
    let input_str = input.to_string_lossy().to_string();
    let input_probe = ffmpeg::ffprobe_source(ffprobe, &input_str).await?;
    let output_audio_spec = input_probe
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"))
        .map(AudioExtractionSpec::from_probe_stream);
    let candidate_specs = [
        Some(AudioExtractionSpec {
            sample_rate: Some(44_100),
            channels: Some(1),
            encoding: AudioExtractionEncoding::PcmS16Le,
        }),
        output_audio_spec,
        source_audio_spec,
    ];

    for (index, spec) in candidate_specs.into_iter().enumerate() {
        let Some(spec) = spec else {
            continue;
        };
        let verify_wav = temp_dir.join(format!("verify-{index}.wav"));
        if extract_audio(ffmpeg, input, &verify_wav, Some(spec))
            .await
            .is_err()
        {
            continue;
        }
        if total_duration > 0.0 {
            emit_progress(app_handle, pipeline_id, "正在回读验证版权编号", 84);
        }
        let bytes = std::fs::read(&verify_wav)
            .map_err(|e| PipelineError::WatermarkExtractFailed(format!("read verify wav: {e}")))?;
        if watermark_core::WatermarkService::extract(MediaInput::AudioWavBytes {
            bytes: bytes.clone(),
        })
        .is_ok()
        {
            let _ = std::fs::remove_dir_all(temp_dir);
            return Ok(bytes);
        }
    }
    let _ = std::fs::remove_dir_all(temp_dir);
    Err(PipelineError::WatermarkExtractFailed(
        "video audio verification failed for all extraction strategies".to_string(),
    ))
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

/// Parse AI content flags from frontend options.
fn parse_ai_flags(
    ai_content: &Option<crate::commands::transcode::AIContentOptions>,
) -> watermark::AIContentFlags {
    use watermark::{
        AIContentFlags, AuthenticityClaim, GenerationMethod, ModificationLevel, TrainingPermission,
    };

    let Some(ai) = ai_content else {
        return AIContentFlags::default();
    };

    let training_permission = match ai.training_permission_declaration.as_str() {
        "non_commercial_allowed" => TrainingPermission::NonCommercial,
        "commercial_allowed" => TrainingPermission::Commercial,
        _ => TrainingPermission::Prohibited,
    };

    let generation_method = match ai.creation_method_declaration.as_str() {
        "text_to_image" => GenerationMethod::TextToImage,
        "image_to_image" => GenerationMethod::ImageToImage,
        "text_to_video" => GenerationMethod::TextToVideo,
        "video_to_video" => GenerationMethod::VideoToVideo,
        "audio_generation" => GenerationMethod::AudioGeneration,
        "multimodal" => GenerationMethod::Multimodal,
        "other_ai" => GenerationMethod::OtherAI,
        _ => GenerationMethod::HumanCreated,
    };

    let modification_level = match ai.human_edit_level_declaration.as_str() {
        "light" => ModificationLevel::LightEdit,
        "moderate" => ModificationLevel::ModerateEdit,
        "heavy" => ModificationLevel::HeavyEdit,
        _ => ModificationLevel::PureAI,
    };

    let authenticity_claim = match ai.authenticity_claim_declaration.as_str() {
        "synthetic" => AuthenticityClaim::Synthetic,
        "based_on_reality" => AuthenticityClaim::BasedOnReality,
        "creator_claimed_authentic" | "authentic" => AuthenticityClaim::AuthenticRecord,
        _ => AuthenticityClaim::Unspecified,
    };

    AIContentFlags {
        is_ai_generated: ai.work_source_declaration == "ai_generated",
        training_permission,
        generation_method,
        human_modification_level: modification_level,
        authenticity_claim,
        reserved: 0,
    }
}

fn declaration_work_source(options: &TranscodeOptions) -> String {
    options
        .ai_content
        .as_ref()
        .map(|value| value.work_source_declaration.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("unspecified")
        .to_string()
}

fn declaration_training_permission(options: &TranscodeOptions) -> String {
    options
        .ai_content
        .as_ref()
        .map(|value| value.training_permission_declaration.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("prohibited")
        .to_string()
}

fn declaration_creation_method(options: &TranscodeOptions) -> String {
    options
        .ai_content
        .as_ref()
        .map(|value| value.creation_method_declaration.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("unspecified")
        .to_string()
}

fn declaration_human_edit_level(options: &TranscodeOptions) -> String {
    options
        .ai_content
        .as_ref()
        .map(|value| value.human_edit_level_declaration.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("unspecified")
        .to_string()
}

fn declaration_authenticity_claim(options: &TranscodeOptions) -> String {
    options
        .ai_content
        .as_ref()
        .map(|value| value.authenticity_claim_declaration.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("unspecified")
        .to_string()
}

fn declaration_custom_rights_statement(options: &TranscodeOptions) -> Option<String> {
    options
        .ai_content
        .as_ref()
        .and_then(|value| value.custom_rights_statement.as_ref())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use watermark_core::{
        build_video_feature_bundle, build_video_visual_payload, derive_video_visual_strategy,
        derive_video_visual_strategy_with_region_selection, embed_video_visual_dct_frames,
        extract_video_visual_dct_from_frames, self_check_video_visual_dct_frames,
        video_frame_plane_from_decoded_luma, DecodedVideoLumaPlane, VideoFeatureBundleBuildInput,
        VideoLumaBitDepth, VideoLumaColorRange, VideoVisualPayloadBuildInput, VideoVisualProfile,
        VideoVisualRegionSelectionMode, VideoVisualSelfCheckFramesInput,
        VideoVisualStrategyBuildInput, WatermarkErrorCode,
    };

    #[derive(Clone, Copy)]
    struct L3VideoEncodeProfile<'a> {
        codec: &'a str,
        preset: &'a str,
        crf: u8,
        profile: Option<&'a str>,
        level: Option<&'a str>,
        maxrate: Option<&'a str>,
        bufsize: Option<&'a str>,
        video_bitrate: Option<&'a str>,
        input_rate: u32,
        output_rate: Option<u32>,
        gop: Option<u32>,
        keyint_min: Option<u32>,
        video_filter: Option<&'a str>,
        extra_video_args: &'a [&'a str],
    }

    impl<'a> L3VideoEncodeProfile<'a> {
        fn h264_crf(crf: u8, preset: &'a str, video_filter: Option<&'a str>) -> Self {
            Self {
                codec: "libx264",
                preset,
                crf,
                profile: None,
                level: None,
                maxrate: None,
                bufsize: None,
                video_bitrate: None,
                input_rate: 4,
                output_rate: None,
                gop: None,
                keyint_min: None,
                video_filter,
                extra_video_args: &[],
            }
        }
    }

    #[derive(Clone, Copy)]
    struct L3CommercialSamplingCase<'a> {
        case_name: &'a str,
        width: usize,
        height: usize,
        bitrate: &'a str,
        maxrate: &'a str,
        bufsize: &'a str,
        codec: &'a str,
        codec_profile: Option<&'a str>,
        crf: u8,
        extra_video_args: &'a [&'a str],
        max_regions: u32,
        sampled_frames: usize,
        region_selection_mode: Option<VideoVisualRegionSelectionMode>,
        video_filter: Option<&'a str>,
    }

    struct L3CommercialSamplingMetrics {
        ffmpeg_source_and_sample_ms: u128,
        core_embed_ms: u128,
        ffmpeg_sample_roundtrip_ms: u128,
        core_self_check_ms: u128,
        total_ms: u128,
        self_check_status: String,
        self_check_passed: bool,
    }

    #[derive(Clone, Copy)]
    struct L3PlatformSecondPassCase<'a> {
        risk_profile: &'a str,
        source_lavfi: &'a str,
        first_pass: L3CommercialSamplingCase<'a>,
        second_pass_bitrate: &'a str,
        second_pass_maxrate: &'a str,
        second_pass_bufsize: &'a str,
        second_pass_crf: u8,
        expect_pass: bool,
    }

    struct L3PlatformSecondPassMetrics {
        ffmpeg_source_and_sample_ms: u128,
        core_embed_ms: u128,
        ffmpeg_first_pass_ms: u128,
        ffmpeg_second_pass_ms: u128,
        ffmpeg_decode_second_pass_ms: u128,
        core_self_check_ms: u128,
        total_ms: u128,
        self_check_status: String,
        self_check_passed: bool,
        checked_frames: u32,
        confidence: f32,
    }

    #[derive(Clone, Copy)]
    struct L3HighBitrateReleaseSampleCase<'a> {
        group: &'a str,
        failure_attribution: &'a str,
        min_confidence: f32,
        case: L3PlatformSecondPassCase<'a>,
    }

    struct L3HighBitrateReleaseSampleOutcome {
        group: String,
        case_name: String,
        failure_attribution: String,
        confidence: f32,
        self_check_status: String,
        passed_threshold: bool,
    }

    #[derive(Clone, Copy)]
    struct L1VideoContainerSpec<'a> {
        extension: &'a str,
        video_codec: &'a str,
        audio_codec: &'a str,
        audio_bitrate: Option<&'a str>,
        protected_extension: &'a str,
        protected_video_codec: &'a str,
        protected_audio_codec: &'a str,
        protected_audio_bitrate: Option<&'a str>,
    }

    impl<'a> L1VideoContainerSpec<'a> {
        fn mp4(extension: &'a str) -> Self {
            Self {
                extension,
                video_codec: "libx264",
                audio_codec: "aac",
                audio_bitrate: Some("160k"),
                protected_extension: "mp4",
                protected_video_codec: "copy",
                protected_audio_codec: "aac",
                protected_audio_bitrate: Some("160k"),
            }
        }

        fn webm() -> Self {
            Self {
                extension: "webm",
                video_codec: "libvpx",
                audio_codec: "libopus",
                audio_bitrate: Some("160k"),
                protected_extension: "webm",
                protected_video_codec: "copy",
                protected_audio_codec: "libopus",
                protected_audio_bitrate: Some("160k"),
            }
        }

        fn avi() -> Self {
            Self {
                extension: "avi",
                video_codec: "mpeg4",
                audio_codec: "aac",
                audio_bitrate: Some("160k"),
                protected_extension: "mp4",
                protected_video_codec: "libx264",
                protected_audio_codec: "aac",
                protected_audio_bitrate: Some("160k"),
            }
        }
    }

    #[test]
    fn classify_video_extensions() {
        assert_eq!(classify_file(Path::new("test.mp4")), FileType::Video);
        assert_eq!(classify_file(Path::new("test.mov")), FileType::Video);
        assert_eq!(classify_file(Path::new("test.webm")), FileType::Video);
        assert_eq!(classify_file(Path::new("test.avi")), FileType::Video);
        assert_eq!(classify_file(Path::new("test.mkv")), FileType::Video);
        assert_eq!(classify_file(Path::new("test.m4v")), FileType::Video);
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
    fn audio_extraction_spec_preserves_source_sample_rate_and_channels() {
        let spec = AudioExtractionSpec {
            sample_rate: Some(48_000),
            channels: Some(1),
            encoding: AudioExtractionEncoding::PcmS24Le,
        };
        let mut args = Vec::new();
        spec.apply_ffmpeg_args(&mut args);
        assert_eq!(args, vec!["-ar", "48000", "-ac", "1"]);
        assert_eq!(spec.ffmpeg_codec(), Some("pcm_s24le"));
    }

    #[test]
    fn audio_extraction_spec_omits_unknown_values() {
        let spec = AudioExtractionSpec {
            sample_rate: None,
            channels: Some(2),
            encoding: AudioExtractionEncoding::PcmF32Le,
        };
        let mut args = Vec::new();
        spec.apply_ffmpeg_args(&mut args);
        assert_eq!(args, vec!["-ac", "2"]);
        assert_eq!(spec.ffmpeg_codec(), Some("pcm_f32le"));
    }

    #[test]
    fn audio_extraction_spec_keeps_24_bit_streams_reported_as_s32() {
        let stream = serde_json::from_value::<ffmpeg::FfprobeStream>(serde_json::json!({
            "codec_type": "audio",
            "codec_name": "pcm_s24le",
            "sample_rate": "48000",
            "channels": 1,
            "sample_fmt": "s32",
            "bits_per_sample": 24,
            "bits_per_raw_sample": 24,
        }))
        .unwrap();

        let spec = AudioExtractionSpec::from_probe_stream(&stream);

        assert_eq!(spec.encoding, AudioExtractionEncoding::PcmS24Le);
        assert_eq!(spec.ffmpeg_codec(), Some("pcm_s24le"));
    }

    #[tokio::test]
    async fn desktop_transcode_audio_fixtures_extract_to_core_wav() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("desktop_transcode_fixture_skip: {error}");
                return;
            }
        };

        let fixture_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("mobile_app")
            .join("rust")
            .join("testdata")
            .join("audio");
        for fixture in [
            "sine_31s.flac",
            "sine_31s.mp3",
            "sine_31s.ogg",
            "sine_31s.m4a",
            "sine_31s.aac",
        ] {
            let input = fixture_dir.join(fixture);
            assert!(
                input.exists(),
                "missing desktop transcode fixture {fixture}"
            );
            let input_str = input.to_string_lossy().to_string();
            let probe = ffmpeg::ffprobe_source(&paths.ffprobe, &input_str)
                .await
                .unwrap();
            let spec = probe
                .streams
                .iter()
                .find(|stream| stream.codec_type.as_deref() == Some("audio"))
                .map(AudioExtractionSpec::from_probe_stream);
            let temp_dir = tempfile::tempdir().unwrap();
            let output_wav = temp_dir.path().join(format!("{fixture}.wav"));

            extract_audio(&paths.ffmpeg, &input, &output_wav, spec)
                .await
                .unwrap();

            let wav_bytes = std::fs::read(&output_wav).unwrap();
            assert!(
                wav_bytes.len() > 1024,
                "desktop transcode output too small for {fixture}"
            );
            assert_eq!(&wav_bytes[0..4], b"RIFF");
            assert_eq!(&wav_bytes[8..12], b"WAVE");

            let payload = build_watermark_payload(
                "desktop-transcode-fixture",
                "desktop-transcode-device",
                1_700_000_456,
                [0x33; 32],
                watermark::AIContentFlags::default(),
            )
            .unwrap();
            let output = WatermarkService::embed(
                MediaInput::AudioWavBytes { bytes: wav_bytes },
                &payload,
                EmbedOptions::default(),
            )
            .unwrap();
            let MediaOutput::AudioWavBytes { bytes } = output else {
                panic!("unexpected output");
            };
            let extracted = WatermarkService::extract(MediaInput::AudioWavBytes { bytes }).unwrap();
            assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
        }
    }

    #[tokio::test]
    async fn l1_video_audio_track_roundtrip_extracts_core_watermark() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l1_video_audio_track_fixture_skip: {error}");
                return;
            }
        };

        let fixture_audio = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("mobile_app")
            .join("rust")
            .join("testdata")
            .join("audio")
            .join("sine_31s.m4a");
        assert!(
            fixture_audio.exists(),
            "missing L1 video audio fixture {}",
            fixture_audio.display()
        );

        let temp_dir = tempfile::tempdir().unwrap();
        let source_video = temp_dir.path().join("l1-source.mp4");
        let extracted_wav = temp_dir.path().join("l1-audio.wav");
        let watermarked_wav = temp_dir.path().join("l1-watermarked.wav");
        let protected_video = temp_dir.path().join("l1-protected.mp4");
        let verify_wav = temp_dir.path().join("l1-verify.wav");

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=320x180:rate=24:duration=31",
                "-i",
                &fixture_audio.to_string_lossy(),
                "-map",
                "0:v:0",
                "-map",
                "1:a:0",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-b:a",
                "160k",
                "-shortest",
                &source_video.to_string_lossy(),
            ],
            "create L1 source video fixture",
        )
        .await;

        let input_str = source_video.to_string_lossy().to_string();
        let probe = ffmpeg::ffprobe_source(&paths.ffprobe, &input_str)
            .await
            .unwrap();
        let spec = probe
            .streams
            .iter()
            .find(|stream| stream.codec_type.as_deref() == Some("audio"))
            .map(AudioExtractionSpec::from_probe_stream);
        extract_audio(&paths.ffmpeg, &source_video, &extracted_wav, spec)
            .await
            .unwrap();

        let payload = build_watermark_payload(
            "l1-video-audio-track",
            "l1-video-device",
            1_700_000_789,
            [0x44; 32],
            watermark::AIContentFlags::default(),
        )
        .unwrap();
        let wav_bytes = std::fs::read(&extracted_wav).unwrap();
        let embedded = WatermarkService::embed(
            MediaInput::AudioWavBytes { bytes: wav_bytes },
            &payload,
            EmbedOptions {
                audio_protection_mode: AudioProtectionMode::VideoTrack,
                ..EmbedOptions::default()
            },
        )
        .unwrap();
        let MediaOutput::AudioWavBytes { bytes } = embedded else {
            panic!("unexpected non-audio output");
        };
        std::fs::write(&watermarked_wav, bytes).unwrap();

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-i",
                &source_video.to_string_lossy(),
                "-i",
                &watermarked_wav.to_string_lossy(),
                "-map",
                "0:v:0",
                "-map",
                "1:a:0",
                "-c:v",
                "copy",
                "-c:a",
                "aac",
                "-b:a",
                "160k",
                "-shortest",
                &protected_video.to_string_lossy(),
            ],
            "mux L1 protected video fixture",
        )
        .await;

        let protected_str = protected_video.to_string_lossy().to_string();
        let protected_probe = ffmpeg::ffprobe_source(&paths.ffprobe, &protected_str)
            .await
            .unwrap();
        let protected_spec = protected_probe
            .streams
            .iter()
            .find(|stream| stream.codec_type.as_deref() == Some("audio"))
            .map(AudioExtractionSpec::from_probe_stream);
        extract_audio(&paths.ffmpeg, &protected_video, &verify_wav, protected_spec)
            .await
            .unwrap();

        let verify_bytes = std::fs::read(&verify_wav).unwrap();
        let extracted = WatermarkService::extract(MediaInput::AudioWavBytes {
            bytes: verify_bytes,
        })
        .unwrap();
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
    }

    #[tokio::test]
    async fn l1_video_audio_track_accepts_release_input_containers() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l1_video_audio_track_container_matrix_skip: {error}");
                return;
            }
        };

        if !ffmpeg_encoder_available(&paths.ffmpeg, "libx264").await
            || !ffmpeg_encoder_available(&paths.ffmpeg, "libvpx").await
            || !ffmpeg_encoder_available(&paths.ffmpeg, "libopus").await
            || !ffmpeg_encoder_available(&paths.ffmpeg, "aac").await
            || !ffmpeg_encoder_available(&paths.ffmpeg, "mpeg4").await
        {
            println!("l1_video_audio_track_container_matrix_skip: missing ffmpeg encoder");
            return;
        }

        let temp_dir = tempfile::tempdir().unwrap();
        let fixture_audio = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("mobile_app")
            .join("rust")
            .join("testdata")
            .join("audio")
            .join("sine_31s.m4a");
        assert!(
            fixture_audio.exists(),
            "missing L1 video audio fixture {}",
            fixture_audio.display()
        );
        let fixture_audio_str = fixture_audio.to_string_lossy().to_string();
        let payload = build_watermark_payload(
            "l1-video-container-matrix",
            "l1-video-container-device",
            1_700_001_234,
            [0x55; 32],
            watermark::AIContentFlags::default(),
        )
        .unwrap();

        for spec in [
            L1VideoContainerSpec::mp4("mp4"),
            L1VideoContainerSpec::mp4("mov"),
            L1VideoContainerSpec::avi(),
            L1VideoContainerSpec::mp4("mkv"),
            L1VideoContainerSpec::mp4("m4v"),
        ] {
            let source_video = temp_dir
                .path()
                .join(format!("l1-container-source.{}", spec.extension));
            let extracted_wav = temp_dir
                .path()
                .join(format!("l1-container-{}-audio.wav", spec.extension));
            let watermarked_wav = temp_dir
                .path()
                .join(format!("l1-container-{}-watermarked.wav", spec.extension));
            let protected_video = temp_dir.path().join(format!(
                "l1-container-{}-protected.{}",
                spec.extension, spec.protected_extension
            ));
            let verify_wav = temp_dir
                .path()
                .join(format!("l1-container-{}-verify.wav", spec.extension));

            let mut create_args = vec![
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=320x180:rate=24:duration=31",
                "-i",
                &fixture_audio_str,
                "-map",
                "0:v:0",
                "-map",
                "1:a:0",
                "-c:v",
                spec.video_codec,
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                spec.audio_codec,
                "-shortest",
            ];
            if let Some(audio_bitrate) = spec.audio_bitrate {
                create_args.extend(["-b:a", audio_bitrate]);
            }
            create_args.push(source_video.to_str().unwrap());
            run_ffmpeg_test_command(
                &paths.ffmpeg,
                &create_args,
                &format!("create L1 {} source video fixture", spec.extension),
            )
            .await;

            let input_str = source_video.to_string_lossy().to_string();
            let probe = ffmpeg::ffprobe_source(&paths.ffprobe, &input_str)
                .await
                .unwrap();
            let spec_from_probe = probe
                .streams
                .iter()
                .find(|stream| stream.codec_type.as_deref() == Some("audio"))
                .map(AudioExtractionSpec::from_probe_stream);
            extract_audio(
                &paths.ffmpeg,
                &source_video,
                &extracted_wav,
                spec_from_probe,
            )
            .await
            .unwrap();

            let wav_bytes = std::fs::read(&extracted_wav).unwrap();
            let embedded = WatermarkService::embed(
                MediaInput::AudioWavBytes { bytes: wav_bytes },
                &payload,
                EmbedOptions {
                    audio_protection_mode: AudioProtectionMode::VideoTrack,
                    ..EmbedOptions::default()
                },
            )
            .unwrap();
            let MediaOutput::AudioWavBytes { bytes } = embedded else {
                panic!("unexpected non-audio output");
            };
            std::fs::write(&watermarked_wav, bytes).unwrap();

            let mut mux_args = vec![
                "-y".to_string(),
                "-i".to_string(),
                source_video.to_string_lossy().to_string(),
                "-i".to_string(),
                watermarked_wav.to_string_lossy().to_string(),
                "-map".to_string(),
                "0:v:0".to_string(),
                "-map".to_string(),
                "1:a:0".to_string(),
                "-c:v".to_string(),
                spec.protected_video_codec.to_string(),
                "-c:a".to_string(),
                spec.protected_audio_codec.to_string(),
            ];
            if let Some(audio_bitrate) = spec.protected_audio_bitrate {
                mux_args.extend(["-b:a".to_string(), audio_bitrate.to_string()]);
            }
            mux_args.extend([
                "-shortest".to_string(),
                protected_video.to_string_lossy().to_string(),
            ]);
            let mux_arg_refs = mux_args.iter().map(String::as_str).collect::<Vec<_>>();

            run_ffmpeg_test_command(
                &paths.ffmpeg,
                &mux_arg_refs,
                &format!("mux L1 {} protected video fixture", spec.extension),
            )
            .await;

            let protected_str = protected_video.to_string_lossy().to_string();
            let protected_probe = ffmpeg::ffprobe_source(&paths.ffprobe, &protected_str)
                .await
                .unwrap_or_else(|error| {
                    panic!(
                        "probe protected video for {} failed: {error}",
                        spec.extension
                    )
                });
            let protected_audio_spec = protected_probe
                .streams
                .iter()
                .find(|stream| stream.codec_type.as_deref() == Some("audio"))
                .map(AudioExtractionSpec::from_probe_stream);
            let candidate_specs = [
                Some(AudioExtractionSpec {
                    sample_rate: Some(44_100),
                    channels: Some(1),
                    encoding: AudioExtractionEncoding::PcmS16Le,
                }),
                protected_audio_spec,
                spec_from_probe,
            ];
            let mut extracted_audio = false;
            let mut extracted_payload = false;
            for candidate_spec in candidate_specs.into_iter().flatten() {
                if extract_audio(
                    &paths.ffmpeg,
                    &protected_video,
                    &verify_wav,
                    Some(candidate_spec),
                )
                .await
                .is_err()
                {
                    continue;
                }
                extracted_audio = true;
                let verify_bytes = std::fs::read(&verify_wav).unwrap();
                if WatermarkService::extract(MediaInput::AudioWavBytes {
                    bytes: verify_bytes,
                })
                .is_ok()
                {
                    extracted_payload = true;
                    break;
                }
            }
            assert!(
                extracted_audio,
                "extract verify audio for {} failed: all extraction strategies failed",
                spec.extension
            );
            assert!(
                extracted_payload,
                "core extract for {} failed: all extraction strategies failed",
                spec.extension
            );
        }
    }

    #[tokio::test]
    async fn l3_decoded_video_y_plane_fixture_enters_watermark_core() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_decoded_y_plane_fixture_skip: {error}");
                return;
            }
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let source_video = temp_dir.path().join("l3-y-plane-source.mp4");
        let raw_y_plane = temp_dir.path().join("l3-y-plane.gray10le");

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=128x72:rate=1:duration=1",
                "-frames:v",
                "1",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p10le",
                &source_video.to_string_lossy(),
            ],
            "create L3 decoded Y-plane source fixture",
        )
        .await;

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-i",
                &source_video.to_string_lossy(),
                "-frames:v",
                "1",
                "-map",
                "0:v:0",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray10le",
                &raw_y_plane.to_string_lossy(),
            ],
            "extract L3 decoded Y-plane fixture",
        )
        .await;

        let raw = std::fs::read(&raw_y_plane).unwrap();
        assert_eq!(raw.len(), 128 * 72 * 2);
        let samples = raw
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let frame = video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
            width: 128,
            height: 72,
            stride_samples: 128,
            samples: &samples,
            bit_depth: VideoLumaBitDepth::Ten,
            color_range: VideoLumaColorRange::Limited,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
        })
        .unwrap();

        assert_eq!(frame.profile, VideoVisualProfile::LumaDctMidBandV1);
        assert_eq!(frame.width, 128);
        assert_eq!(frame.height, 72);
        assert_eq!(frame.stride, 128);
        assert_eq!(frame.visible_rows().count(), 72);
    }

    #[tokio::test]
    async fn l3_decoded_video_y_plane_fixture_roundtrips_dct_in_watermark_core() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_decoded_y_plane_dct_roundtrip_fixture_skip: {error}");
                return;
            }
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let source_video = temp_dir.path().join("l3-y-plane-dct-source.mp4");
        let raw_y_plane = temp_dir.path().join("l3-y-plane-dct.gray10le");

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=512x512:rate=4:duration=1",
                "-frames:v",
                "4",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p10le",
                &source_video.to_string_lossy(),
            ],
            "create L3 decoded Y-plane DCT source fixture",
        )
        .await;

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-i",
                &source_video.to_string_lossy(),
                "-frames:v",
                "4",
                "-map",
                "0:v:0",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray10le",
                &raw_y_plane.to_string_lossy(),
            ],
            "extract L3 decoded Y-plane DCT fixture",
        )
        .await;

        const WIDTH: usize = 512;
        const HEIGHT: usize = 512;
        const FRAME_COUNT: usize = 4;
        let raw = std::fs::read(&raw_y_plane).unwrap();
        assert_eq!(raw.len(), WIDTH * HEIGHT * FRAME_COUNT * 2);
        let samples = raw
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let mut frames = samples
            .chunks_exact(WIDTH * HEIGHT)
            .take(FRAME_COUNT)
            .map(|frame_samples| {
                video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                    width: WIDTH as u32,
                    height: HEIGHT as u32,
                    stride_samples: WIDTH,
                    samples: frame_samples,
                    bit_depth: VideoLumaBitDepth::Ten,
                    color_range: VideoLumaColorRange::Limited,
                    target_profile: VideoVisualProfile::LumaDctMidBandV1,
                })
                .unwrap()
            })
            .collect::<Vec<_>>();

        let source_sha = parse_sha256_hex_32(
            &hash::sha256_of_file(source_video.to_string_lossy().as_ref()).unwrap(),
        )
        .unwrap();
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: 1_000,
        })
        .unwrap();
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "l3-core-fixture",
            device_identity: "desktop-ffmpeg-decoder",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: watermark::AIContentFlags::default(),
        })
        .unwrap();
        let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
            task_id: "l3-decoded-y-plane-dct-roundtrip",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: 32,
        })
        .unwrap();

        let embedded = embed_video_visual_dct_frames(&mut frames, &strategy, &payload).unwrap();
        let extracted = extract_video_visual_dct_from_frames(&frames, &strategy).unwrap();
        let self_check = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();

        assert_eq!(embedded, 4);
        assert_eq!(extracted, payload);
        assert!(self_check.passed);
        assert_eq!(self_check.checked_frames, 4);
        assert!(
            self_check.confidence >= strategy.self_check_threshold,
            "expected encoded DCT self-check confidence {} to meet threshold {}",
            self_check.confidence,
            strategy.self_check_threshold
        );
    }

    #[tokio::test]
    async fn l3_encoded_video_y_plane_fixture_self_checks_after_ffmpeg_roundtrip() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_encoded_y_plane_dct_self_check_fixture_skip: {error}");
                return;
            }
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let source_video = temp_dir.path().join("l3-y-plane-encode-source.mp4");
        let raw_y_plane = temp_dir.path().join("l3-y-plane-encode-source.gray10le");
        let written_y_plane = temp_dir.path().join("l3-y-plane-written.gray");
        let protected_video = temp_dir.path().join("l3-y-plane-protected.mp4");
        let decoded_written_y_plane = temp_dir.path().join("l3-y-plane-written-decoded.gray10le");

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=512x512:rate=4:duration=1",
                "-frames:v",
                "4",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p10le",
                &source_video.to_string_lossy(),
            ],
            "create L3 encoded Y-plane source fixture",
        )
        .await;

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-i",
                &source_video.to_string_lossy(),
                "-frames:v",
                "4",
                "-map",
                "0:v:0",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray10le",
                &raw_y_plane.to_string_lossy(),
            ],
            "extract L3 encoded Y-plane source fixture",
        )
        .await;

        const WIDTH: usize = 512;
        const HEIGHT: usize = 512;
        const FRAME_COUNT: usize = 4;
        let raw = std::fs::read(&raw_y_plane).unwrap();
        assert_eq!(raw.len(), WIDTH * HEIGHT * FRAME_COUNT * 2);
        let samples = raw
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let mut frames = samples
            .chunks_exact(WIDTH * HEIGHT)
            .take(FRAME_COUNT)
            .map(|frame_samples| {
                video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                    width: WIDTH as u32,
                    height: HEIGHT as u32,
                    stride_samples: WIDTH,
                    samples: frame_samples,
                    bit_depth: VideoLumaBitDepth::Ten,
                    color_range: VideoLumaColorRange::Limited,
                    target_profile: VideoVisualProfile::LumaDctMidBandV1,
                })
                .unwrap()
            })
            .collect::<Vec<_>>();

        let source_sha = parse_sha256_hex_32(
            &hash::sha256_of_file(source_video.to_string_lossy().as_ref()).unwrap(),
        )
        .unwrap();
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: 1_000,
        })
        .unwrap();
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "l3-core-fixture",
            device_identity: "desktop-ffmpeg-encoder",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: watermark::AIContentFlags::default(),
        })
        .unwrap();
        let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
            task_id: "l3-encoded-y-plane-dct-self-check",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: 32,
        })
        .unwrap();

        embed_video_visual_dct_frames(&mut frames, &strategy, &payload).unwrap();
        let written = frames
            .iter()
            .flat_map(|frame| frame.luma_pixels())
            .collect::<Vec<_>>();
        assert_eq!(written.len(), WIDTH * HEIGHT * FRAME_COUNT);
        std::fs::write(&written_y_plane, written).unwrap();

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray",
                "-s:v",
                "512x512",
                "-r",
                "4",
                "-i",
                &written_y_plane.to_string_lossy(),
                "-frames:v",
                "4",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-crf",
                "0",
                "-pix_fmt",
                "yuv420p",
                &protected_video.to_string_lossy(),
            ],
            "encode L3 written Y-plane fixture",
        )
        .await;

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-i",
                &protected_video.to_string_lossy(),
                "-frames:v",
                "4",
                "-map",
                "0:v:0",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray10le",
                &decoded_written_y_plane.to_string_lossy(),
            ],
            "decode L3 written Y-plane fixture",
        )
        .await;

        let decoded = std::fs::read(&decoded_written_y_plane).unwrap();
        assert_eq!(decoded.len(), WIDTH * HEIGHT * FRAME_COUNT * 2);
        let decoded_samples = decoded
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let decoded_frames = decoded_samples
            .chunks_exact(WIDTH * HEIGHT)
            .take(FRAME_COUNT)
            .map(|frame_samples| {
                video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                    width: WIDTH as u32,
                    height: HEIGHT as u32,
                    stride_samples: WIDTH,
                    samples: frame_samples,
                    bit_depth: VideoLumaBitDepth::Ten,
                    color_range: VideoLumaColorRange::Limited,
                    target_profile: VideoVisualProfile::LumaDctMidBandV1,
                })
                .unwrap()
            })
            .collect::<Vec<_>>();

        let extracted = extract_video_visual_dct_from_frames(&decoded_frames, &strategy).unwrap();
        let self_check = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &decoded_frames,
            expected_payload: &payload,
        })
        .unwrap();

        assert_eq!(extracted, payload);
        assert!(self_check.passed);
        assert_eq!(self_check.checked_frames, 4);
        assert!(
            self_check.confidence >= strategy.self_check_threshold,
            "expected encoded DCT self-check confidence {} to meet threshold {}",
            self_check.confidence,
            strategy.self_check_threshold
        );
    }

    #[tokio::test]
    async fn l3_lossy_video_y_plane_fixture_classifies_dct_self_check_boundary() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_lossy_y_plane_dct_matrix_fixture_skip: {error}");
                return;
            }
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let source_video = temp_dir.path().join("l3-y-plane-lossy-source.mp4");
        let raw_y_plane = temp_dir.path().join("l3-y-plane-lossy-source.gray10le");
        let written_y_plane = temp_dir.path().join("l3-y-plane-lossy-written.gray");

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=512x512:rate=4:duration=1",
                "-frames:v",
                "4",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p10le",
                &source_video.to_string_lossy(),
            ],
            "create L3 lossy Y-plane source fixture",
        )
        .await;

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-i",
                &source_video.to_string_lossy(),
                "-frames:v",
                "4",
                "-map",
                "0:v:0",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray10le",
                &raw_y_plane.to_string_lossy(),
            ],
            "extract L3 lossy Y-plane source fixture",
        )
        .await;

        const WIDTH: usize = 512;
        const HEIGHT: usize = 512;
        const FRAME_COUNT: usize = 4;
        let raw = std::fs::read(&raw_y_plane).unwrap();
        assert_eq!(raw.len(), WIDTH * HEIGHT * FRAME_COUNT * 2);
        let samples = raw
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let mut frames = samples
            .chunks_exact(WIDTH * HEIGHT)
            .take(FRAME_COUNT)
            .map(|frame_samples| {
                video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                    width: WIDTH as u32,
                    height: HEIGHT as u32,
                    stride_samples: WIDTH,
                    samples: frame_samples,
                    bit_depth: VideoLumaBitDepth::Ten,
                    color_range: VideoLumaColorRange::Limited,
                    target_profile: VideoVisualProfile::LumaDctMidBandV1,
                })
                .unwrap()
            })
            .collect::<Vec<_>>();

        let source_sha = parse_sha256_hex_32(
            &hash::sha256_of_file(source_video.to_string_lossy().as_ref()).unwrap(),
        )
        .unwrap();
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: 1_000,
        })
        .unwrap();
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "l3-core-fixture",
            device_identity: "desktop-ffmpeg-lossy-encoder",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: watermark::AIContentFlags::default(),
        })
        .unwrap();
        let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
            task_id: "l3-lossy-y-plane-dct-matrix",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: 32,
        })
        .unwrap();

        embed_video_visual_dct_frames(&mut frames, &strategy, &payload).unwrap();
        let written = frames
            .iter()
            .flat_map(|frame| frame.luma_pixels())
            .collect::<Vec<_>>();
        std::fs::write(&written_y_plane, written).unwrap();

        let medium_loss_frames = encode_and_decode_l3_y_plane_fixture(
            &paths.ffmpeg,
            temp_dir.path(),
            &written_y_plane,
            12,
        )
        .await;
        let medium_loss_check =
            self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
                strategy: &strategy,
                observed_strategy_digest: &strategy.strategy_digest,
                frames: &medium_loss_frames,
                expected_payload: &payload,
            })
            .unwrap();

        assert!(medium_loss_check.passed);
        assert!(
            medium_loss_check.confidence >= strategy.self_check_threshold,
            "expected CRF 12 DCT self-check confidence {} to meet threshold {}",
            medium_loss_check.confidence,
            strategy.self_check_threshold
        );

        let high_loss_frames = encode_and_decode_l3_y_plane_fixture(
            &paths.ffmpeg,
            temp_dir.path(),
            &written_y_plane,
            38,
        )
        .await;
        let high_loss_error = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &high_loss_frames,
            expected_payload: &payload,
        })
        .unwrap_err();

        assert_eq!(high_loss_error.code(), WatermarkErrorCode::SelfCheckFailed);
    }

    #[tokio::test]
    async fn l3_target_platform_transcode_matrix_classifies_dct_survival() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_target_platform_transcode_matrix_fixture_skip: {error}");
                return;
            }
        };

        let temp_dir = tempfile::tempdir().unwrap();
        let source_video = temp_dir.path().join("l3-platform-matrix-source.mp4");
        let raw_y_plane = temp_dir.path().join("l3-platform-matrix-source.gray10le");
        let written_y_plane = temp_dir.path().join("l3-platform-matrix-written.gray");

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-f",
                "lavfi",
                "-i",
                "testsrc2=size=512x512:rate=4:duration=1",
                "-frames:v",
                "4",
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p10le",
                &source_video.to_string_lossy(),
            ],
            "create L3 target platform matrix source fixture",
        )
        .await;

        run_ffmpeg_test_command(
            &paths.ffmpeg,
            &[
                "-y",
                "-i",
                &source_video.to_string_lossy(),
                "-frames:v",
                "4",
                "-map",
                "0:v:0",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray10le",
                &raw_y_plane.to_string_lossy(),
            ],
            "extract L3 target platform matrix source fixture",
        )
        .await;

        const WIDTH: usize = 512;
        const HEIGHT: usize = 512;
        const FRAME_COUNT: usize = 4;
        let raw = std::fs::read(&raw_y_plane).unwrap();
        assert_eq!(raw.len(), WIDTH * HEIGHT * FRAME_COUNT * 2);
        let samples = raw
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let mut frames = samples
            .chunks_exact(WIDTH * HEIGHT)
            .take(FRAME_COUNT)
            .map(|frame_samples| {
                video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                    width: WIDTH as u32,
                    height: HEIGHT as u32,
                    stride_samples: WIDTH,
                    samples: frame_samples,
                    bit_depth: VideoLumaBitDepth::Ten,
                    color_range: VideoLumaColorRange::Limited,
                    target_profile: VideoVisualProfile::LumaDctMidBandV1,
                })
                .unwrap()
            })
            .collect::<Vec<_>>();

        let source_sha = parse_sha256_hex_32(
            &hash::sha256_of_file(source_video.to_string_lossy().as_ref()).unwrap(),
        )
        .unwrap();
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: 1_000,
        })
        .unwrap();
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "l3-core-fixture",
            device_identity: "desktop-ffmpeg-platform-matrix",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: watermark::AIContentFlags::default(),
        })
        .unwrap();
        let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
            task_id: "l3-target-platform-transcode-matrix",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: 32,
        })
        .unwrap();

        embed_video_visual_dct_frames(&mut frames, &strategy, &payload).unwrap();
        let written = frames
            .iter()
            .flat_map(|frame| frame.luma_pixels())
            .collect::<Vec<_>>();
        std::fs::write(&written_y_plane, written).unwrap();

        for (case_name, crf, preset) in [
            ("douyin_high_quality_h264_crf18", 18u8, "medium"),
            ("bilibili_standard_h264_crf23", 23u8, "medium"),
        ] {
            let frames = encode_and_decode_l3_y_plane_fixture_with_options(
                &paths.ffmpeg,
                temp_dir.path(),
                &written_y_plane,
                case_name,
                WIDTH,
                HEIGHT,
                FRAME_COUNT,
                crf,
                preset,
                None,
            )
            .await;
            let result = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
                strategy: &strategy,
                observed_strategy_digest: &strategy.strategy_digest,
                frames: &frames,
                expected_payload: &payload,
            })
            .unwrap();
            assert!(
                result.confidence >= strategy.self_check_threshold,
                "{case_name} confidence {} should meet threshold {}",
                result.confidence,
                strategy.self_check_threshold
            );
        }

        for (case_name, crf, filter) in [(
            "scaled_down_up_h264_crf18",
            18u8,
            Some("scale=384:384:flags=bilinear,scale=512:512:flags=bilinear"),
        )] {
            let frames = encode_and_decode_l3_y_plane_fixture_with_options(
                &paths.ffmpeg,
                temp_dir.path(),
                &written_y_plane,
                case_name,
                WIDTH,
                HEIGHT,
                FRAME_COUNT,
                crf,
                "medium",
                filter,
            )
            .await;
            let result = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
                strategy: &strategy,
                observed_strategy_digest: &strategy.strategy_digest,
                frames: &frames,
                expected_payload: &payload,
            })
            .unwrap();
            assert!(
                result.confidence >= strategy.self_check_threshold,
                "{case_name} confidence {} should meet threshold {}",
                result.confidence,
                strategy.self_check_threshold
            );
        }

        let high_loss_frames = encode_and_decode_l3_y_plane_fixture_with_options(
            &paths.ffmpeg,
            temp_dir.path(),
            &written_y_plane,
            "aggressive_h264_crf38",
            WIDTH,
            HEIGHT,
            FRAME_COUNT,
            38,
            "medium",
            None,
        )
        .await;
        let high_loss_error = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &high_loss_frames,
            expected_payload: &payload,
        })
        .unwrap_err();
        assert_eq!(high_loss_error.code(), WatermarkErrorCode::SelfCheckFailed);
    }

    #[tokio::test]
    async fn l3_main_resolution_transcode_matrix_covers_720p_1080p_2k() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_main_resolution_transcode_matrix_fixture_skip: {error}");
                return;
            }
        };

        for (case_name, width, height, crf, filter) in [
            ("main_720p_h264_crf23", 1280usize, 720usize, 23u8, None),
            ("main_720p_h264_crf28", 1280usize, 720usize, 28u8, None),
            ("main_1080p_h264_crf23", 1920usize, 1080usize, 23u8, None),
            ("main_1080p_h264_crf28", 1920usize, 1080usize, 28u8, None),
            ("main_2k_h264_crf23", 2560usize, 1440usize, 23u8, None),
            ("main_2k_h264_crf28", 2560usize, 1440usize, 28u8, None),
            (
                "main_720p_center_crop_pad_crf23",
                1280usize,
                720usize,
                23u8,
                Some("crop=1152:648:64:36,pad=1280:720:64:36:black"),
            ),
            (
                "main_1080p_center_crop_pad_crf23",
                1920usize,
                1080usize,
                23u8,
                Some("crop=1728:972:96:54,pad=1920:1080:96:54:black"),
            ),
            (
                "main_2k_center_crop_pad_crf23",
                2560usize,
                1440usize,
                23u8,
                Some("crop=2304:1296:128:72,pad=2560:1440:128:72:black"),
            ),
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let source_video = temp_dir.path().join(format!("{case_name}-source.mp4"));
            let raw_y_plane = temp_dir.path().join(format!("{case_name}-source.gray10le"));
            let written_y_plane = temp_dir.path().join(format!("{case_name}-written.gray"));
            const FRAME_COUNT: usize = 2;

            run_ffmpeg_test_command(
                &paths.ffmpeg,
                &[
                    "-y",
                    "-f",
                    "lavfi",
                    "-i",
                    &format!("testsrc2=size={width}x{height}:rate=2:duration=1"),
                    "-frames:v",
                    &FRAME_COUNT.to_string(),
                    "-c:v",
                    "libx264",
                    "-preset",
                    "ultrafast",
                    "-pix_fmt",
                    "yuv420p10le",
                    &source_video.to_string_lossy(),
                ],
                "create L3 main resolution source fixture",
            )
            .await;

            run_ffmpeg_test_command(
                &paths.ffmpeg,
                &[
                    "-y",
                    "-i",
                    &source_video.to_string_lossy(),
                    "-frames:v",
                    &FRAME_COUNT.to_string(),
                    "-map",
                    "0:v:0",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "gray10le",
                    &raw_y_plane.to_string_lossy(),
                ],
                "extract L3 main resolution source fixture",
            )
            .await;

            let raw = std::fs::read(&raw_y_plane).unwrap();
            assert_eq!(raw.len(), width * height * FRAME_COUNT * 2);
            let samples = raw
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect::<Vec<_>>();
            let mut frames = samples
                .chunks_exact(width * height)
                .take(FRAME_COUNT)
                .map(|frame_samples| {
                    video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                        width: width as u32,
                        height: height as u32,
                        stride_samples: width,
                        samples: frame_samples,
                        bit_depth: VideoLumaBitDepth::Ten,
                        color_range: VideoLumaColorRange::Limited,
                        target_profile: VideoVisualProfile::LumaDctMidBandV1,
                    })
                    .unwrap()
                })
                .collect::<Vec<_>>();

            let source_sha = parse_sha256_hex_32(
                &hash::sha256_of_file(source_video.to_string_lossy().as_ref()).unwrap(),
            )
            .unwrap();
            let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
                frames: &frames,
                source_video_sha256: source_sha,
                duration_ms: 1_000,
            })
            .unwrap();
            let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
                creator_identity: "l3-core-fixture",
                device_identity: "desktop-ffmpeg-main-resolution",
                source_video_sha256: source_sha,
                timestamp: 1_786_147_200,
                ai_flags: watermark::AIContentFlags::default(),
            })
            .unwrap();
            let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
                task_id: case_name,
                payload: &payload,
                feature_bundle: &feature_bundle,
                target_profile: VideoVisualProfile::LumaDctMidBandV1,
                expires_at: 1_786_150_000,
                self_check_threshold: 0.75,
                max_regions: 32,
            })
            .unwrap();

            embed_video_visual_dct_frames(&mut frames, &strategy, &payload).unwrap();
            let written = frames
                .iter()
                .flat_map(|frame| frame.luma_pixels())
                .collect::<Vec<_>>();
            std::fs::write(&written_y_plane, written).unwrap();

            let decoded_frames = encode_and_decode_l3_y_plane_fixture_with_options(
                &paths.ffmpeg,
                temp_dir.path(),
                &written_y_plane,
                case_name,
                width,
                height,
                FRAME_COUNT,
                crf,
                "medium",
                filter,
            )
            .await;
            let self_check = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
                strategy: &strategy,
                observed_strategy_digest: &strategy.strategy_digest,
                frames: &decoded_frames,
                expected_payload: &payload,
            })
            .unwrap();

            assert!(
                self_check.confidence >= strategy.self_check_threshold,
                "{case_name} confidence {} should meet threshold {}",
                self_check.confidence,
                strategy.self_check_threshold
            );
        }
    }

    #[tokio::test]
    async fn l3_main_resolution_platform_profiles_cover_720p_1080p_2k() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_main_resolution_platform_profiles_fixture_skip: {error}");
                return;
            }
        };

        for (case_name, width, height, profile) in [
            (
                "douyin_720p_vertical_h264_high_crf18",
                720usize,
                1280usize,
                L3VideoEncodeProfile {
                    codec: "libx264",
                    preset: "medium",
                    crf: 18,
                    profile: Some("high"),
                    level: Some("4.1"),
                    maxrate: Some("12000k"),
                    bufsize: Some("24000k"),
                    video_bitrate: None,
                    input_rate: 30,
                    output_rate: Some(30),
                    gop: Some(60),
                    keyint_min: None,
                    video_filter: None,
                    extra_video_args: &[],
                },
            ),
            (
                "douyin_1080p_vertical_h264_high_crf18",
                1080usize,
                1920usize,
                L3VideoEncodeProfile {
                    codec: "libx264",
                    preset: "medium",
                    crf: 18,
                    profile: Some("high"),
                    level: Some("4.1"),
                    maxrate: Some("12000k"),
                    bufsize: Some("24000k"),
                    video_bitrate: None,
                    input_rate: 30,
                    output_rate: Some(30),
                    gop: Some(60),
                    keyint_min: None,
                    video_filter: None,
                    extra_video_args: &[],
                },
            ),
            (
                "bilibili_720p_landscape_h264_high_crf18",
                1280usize,
                720usize,
                L3VideoEncodeProfile {
                    codec: "libx264",
                    preset: "medium",
                    crf: 18,
                    profile: Some("high"),
                    level: Some("4.2"),
                    maxrate: Some("8000k"),
                    bufsize: Some("16000k"),
                    video_bitrate: None,
                    input_rate: 30,
                    output_rate: Some(30),
                    gop: Some(300),
                    keyint_min: Some(30),
                    video_filter: None,
                    extra_video_args: &["-refs", "4"],
                },
            ),
            (
                "bilibili_1080p_landscape_h264_high_crf18",
                1920usize,
                1080usize,
                L3VideoEncodeProfile {
                    codec: "libx264",
                    preset: "medium",
                    crf: 18,
                    profile: Some("high"),
                    level: Some("4.2"),
                    maxrate: Some("8000k"),
                    bufsize: Some("16000k"),
                    video_bitrate: None,
                    input_rate: 30,
                    output_rate: Some(30),
                    gop: Some(300),
                    keyint_min: Some(30),
                    video_filter: None,
                    extra_video_args: &["-refs", "4"],
                },
            ),
            (
                "bilibili_2k_landscape_h264_high_crf18",
                2560usize,
                1440usize,
                L3VideoEncodeProfile {
                    codec: "libx264",
                    preset: "medium",
                    crf: 18,
                    profile: Some("high"),
                    level: Some("5.1"),
                    maxrate: Some("12000k"),
                    bufsize: Some("24000k"),
                    video_bitrate: None,
                    input_rate: 30,
                    output_rate: Some(30),
                    gop: Some(300),
                    keyint_min: Some(30),
                    video_filter: None,
                    extra_video_args: &["-refs", "4"],
                },
            ),
            (
                "xiaohongshu_720p_vertical_h264_high_crf17",
                720usize,
                960usize,
                L3VideoEncodeProfile {
                    codec: "libx264",
                    preset: "medium",
                    crf: 17,
                    profile: Some("high"),
                    level: Some("4.1"),
                    maxrate: Some("15000k"),
                    bufsize: Some("30000k"),
                    video_bitrate: None,
                    input_rate: 30,
                    output_rate: Some(30),
                    gop: Some(60),
                    keyint_min: None,
                    video_filter: None,
                    extra_video_args: &[],
                },
            ),
            (
                "xiaohongshu_1080p_vertical_h264_high_crf17",
                1080usize,
                1440usize,
                L3VideoEncodeProfile {
                    codec: "libx264",
                    preset: "medium",
                    crf: 17,
                    profile: Some("high"),
                    level: Some("4.1"),
                    maxrate: Some("15000k"),
                    bufsize: Some("30000k"),
                    video_bitrate: None,
                    input_rate: 30,
                    output_rate: Some(30),
                    gop: Some(60),
                    keyint_min: None,
                    video_filter: None,
                    extra_video_args: &[],
                },
            ),
        ] {
            let started_at = std::time::Instant::now();
            let temp_dir = tempfile::tempdir().unwrap();
            let source_video = temp_dir.path().join(format!("{case_name}-source.mp4"));
            let raw_y_plane = temp_dir.path().join(format!("{case_name}-source.gray10le"));
            let written_y_plane = temp_dir.path().join(format!("{case_name}-written.gray"));
            const FRAME_COUNT: usize = 4;

            run_ffmpeg_test_command(
                &paths.ffmpeg,
                &[
                    "-y",
                    "-f",
                    "lavfi",
                    "-i",
                    &format!(
                        "testsrc2=size={width}x{height}:rate={}:duration=1",
                        profile.input_rate
                    ),
                    "-frames:v",
                    &FRAME_COUNT.to_string(),
                    "-c:v",
                    "libx264",
                    "-preset",
                    "ultrafast",
                    "-pix_fmt",
                    "yuv420p10le",
                    &source_video.to_string_lossy(),
                ],
                "create L3 platform profile source fixture",
            )
            .await;

            run_ffmpeg_test_command(
                &paths.ffmpeg,
                &[
                    "-y",
                    "-i",
                    &source_video.to_string_lossy(),
                    "-frames:v",
                    &FRAME_COUNT.to_string(),
                    "-map",
                    "0:v:0",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "gray10le",
                    &raw_y_plane.to_string_lossy(),
                ],
                "extract L3 platform profile source fixture",
            )
            .await;

            let raw = std::fs::read(&raw_y_plane).unwrap();
            assert_eq!(raw.len(), width * height * FRAME_COUNT * 2);
            let samples = raw
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect::<Vec<_>>();
            let mut frames = samples
                .chunks_exact(width * height)
                .take(FRAME_COUNT)
                .map(|frame_samples| {
                    video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                        width: width as u32,
                        height: height as u32,
                        stride_samples: width,
                        samples: frame_samples,
                        bit_depth: VideoLumaBitDepth::Ten,
                        color_range: VideoLumaColorRange::Limited,
                        target_profile: VideoVisualProfile::LumaDctMidBandV1,
                    })
                    .unwrap()
                })
                .collect::<Vec<_>>();

            let source_sha = parse_sha256_hex_32(
                &hash::sha256_of_file(source_video.to_string_lossy().as_ref()).unwrap(),
            )
            .unwrap();
            let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
                frames: &frames,
                source_video_sha256: source_sha,
                duration_ms: 1_000,
            })
            .unwrap();
            let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
                creator_identity: "l3-core-fixture",
                device_identity: "desktop-ffmpeg-platform-profile",
                source_video_sha256: source_sha,
                timestamp: 1_786_147_200,
                ai_flags: watermark::AIContentFlags::default(),
            })
            .unwrap();
            let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
                task_id: case_name,
                payload: &payload,
                feature_bundle: &feature_bundle,
                target_profile: VideoVisualProfile::LumaDctMidBandV1,
                expires_at: 1_786_150_000,
                self_check_threshold: 0.75,
                max_regions: 32,
            })
            .unwrap();

            embed_video_visual_dct_frames(&mut frames, &strategy, &payload).unwrap();
            let written = frames
                .iter()
                .flat_map(|frame| frame.luma_pixels())
                .collect::<Vec<_>>();
            std::fs::write(&written_y_plane, written).unwrap();

            let decoded_frames = encode_and_decode_l3_y_plane_fixture_with_profile(
                &paths.ffmpeg,
                temp_dir.path(),
                &written_y_plane,
                case_name,
                width,
                height,
                FRAME_COUNT,
                profile,
            )
            .await;
            let self_check = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
                strategy: &strategy,
                observed_strategy_digest: &strategy.strategy_digest,
                frames: &decoded_frames,
                expected_payload: &payload,
            })
            .unwrap_or_else(|error| panic!("{case_name} self-check failed: {error}"));

            println!(
                "l3_platform_profile_matrix_case={case_name} elapsed_ms={}",
                started_at.elapsed().as_millis()
            );
            assert!(
                self_check.confidence >= strategy.self_check_threshold,
                "{case_name} confidence {} should meet threshold {}",
                self_check.confidence,
                strategy.self_check_threshold
            );
        }
    }

    #[tokio::test]
    async fn l3_mainstream_bitrate_floor_matrix_covers_720p_1080p_2k() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_mainstream_bitrate_floor_fixture_skip: {error}");
                return;
            }
        };

        for (case_name, width, height, bitrate, maxrate, bufsize) in [
            (
                "mainstream_floor_720p_h264_2500k",
                1280usize,
                720usize,
                "2500k",
                "3000k",
                "6000k",
            ),
            (
                "mainstream_floor_1080p_h264_4500k",
                1920usize,
                1080usize,
                "4500k",
                "5500k",
                "11000k",
            ),
            (
                "mainstream_floor_2k_h264_8000k",
                2560usize,
                1440usize,
                "8000k",
                "10000k",
                "20000k",
            ),
        ] {
            let started_at = std::time::Instant::now();
            let temp_dir = tempfile::tempdir().unwrap();
            let source_video = temp_dir.path().join(format!("{case_name}-source.mp4"));
            let raw_y_plane = temp_dir.path().join(format!("{case_name}-source.gray10le"));
            let written_y_plane = temp_dir.path().join(format!("{case_name}-written.gray"));
            const FRAME_COUNT: usize = 4;

            run_ffmpeg_test_command(
                &paths.ffmpeg,
                &[
                    "-y",
                    "-f",
                    "lavfi",
                    "-i",
                    &format!("testsrc2=size={width}x{height}:rate=30:duration=1"),
                    "-frames:v",
                    &FRAME_COUNT.to_string(),
                    "-c:v",
                    "libx264",
                    "-preset",
                    "ultrafast",
                    "-pix_fmt",
                    "yuv420p10le",
                    &source_video.to_string_lossy(),
                ],
                "create L3 mainstream bitrate source fixture",
            )
            .await;

            run_ffmpeg_test_command(
                &paths.ffmpeg,
                &[
                    "-y",
                    "-i",
                    &source_video.to_string_lossy(),
                    "-frames:v",
                    &FRAME_COUNT.to_string(),
                    "-map",
                    "0:v:0",
                    "-f",
                    "rawvideo",
                    "-pix_fmt",
                    "gray10le",
                    &raw_y_plane.to_string_lossy(),
                ],
                "extract L3 mainstream bitrate source fixture",
            )
            .await;

            let raw = std::fs::read(&raw_y_plane).unwrap();
            assert_eq!(raw.len(), width * height * FRAME_COUNT * 2);
            let samples = raw
                .chunks_exact(2)
                .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
                .collect::<Vec<_>>();
            let mut frames = samples
                .chunks_exact(width * height)
                .take(FRAME_COUNT)
                .map(|frame_samples| {
                    video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                        width: width as u32,
                        height: height as u32,
                        stride_samples: width,
                        samples: frame_samples,
                        bit_depth: VideoLumaBitDepth::Ten,
                        color_range: VideoLumaColorRange::Limited,
                        target_profile: VideoVisualProfile::LumaDctMidBandV1,
                    })
                    .unwrap()
                })
                .collect::<Vec<_>>();

            let source_sha = parse_sha256_hex_32(
                &hash::sha256_of_file(source_video.to_string_lossy().as_ref()).unwrap(),
            )
            .unwrap();
            let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
                frames: &frames,
                source_video_sha256: source_sha,
                duration_ms: 1_000,
            })
            .unwrap();
            let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
                creator_identity: "l3-core-fixture",
                device_identity: "desktop-ffmpeg-mainstream-bitrate",
                source_video_sha256: source_sha,
                timestamp: 1_786_147_200,
                ai_flags: watermark::AIContentFlags::default(),
            })
            .unwrap();
            let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
                task_id: case_name,
                payload: &payload,
                feature_bundle: &feature_bundle,
                target_profile: VideoVisualProfile::LumaDctMidBandV1,
                expires_at: 1_786_150_000,
                self_check_threshold: 0.75,
                max_regions: 32,
            })
            .unwrap();

            embed_video_visual_dct_frames(&mut frames, &strategy, &payload).unwrap();
            let written = frames
                .iter()
                .flat_map(|frame| frame.luma_pixels())
                .collect::<Vec<_>>();
            std::fs::write(&written_y_plane, written).unwrap();

            let decoded_frames = encode_and_decode_l3_y_plane_fixture_with_profile(
                &paths.ffmpeg,
                temp_dir.path(),
                &written_y_plane,
                case_name,
                width,
                height,
                FRAME_COUNT,
                L3VideoEncodeProfile {
                    codec: "libx264",
                    preset: "medium",
                    crf: 23,
                    profile: Some("high"),
                    level: Some("5.1"),
                    maxrate: Some(maxrate),
                    bufsize: Some(bufsize),
                    video_bitrate: Some(bitrate),
                    input_rate: 30,
                    output_rate: Some(30),
                    gop: Some(60),
                    keyint_min: Some(30),
                    video_filter: None,
                    extra_video_args: &[],
                },
            )
            .await;
            let self_check = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
                strategy: &strategy,
                observed_strategy_digest: &strategy.strategy_digest,
                frames: &decoded_frames,
                expected_payload: &payload,
            })
            .unwrap_or_else(|error| panic!("{case_name} self-check failed: {error}"));

            println!(
                "l3_mainstream_bitrate_floor_case={case_name} elapsed_ms={}",
                started_at.elapsed().as_millis()
            );
            assert!(
                self_check.confidence >= strategy.self_check_threshold,
                "{case_name} confidence {} should meet threshold {}",
                self_check.confidence,
                strategy.self_check_threshold
            );
        }
    }

    #[tokio::test]
    async fn l3_30s_commercial_sampling_performance_records_cost_breakdown() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_30s_commercial_sampling_performance_fixture_skip: {error}");
                return;
            }
        };

        for case in [
            L3CommercialSamplingCase {
                case_name: "commercial_30s_720p_12frames_h264_2500k",
                width: 1280,
                height: 720,
                bitrate: "2500k",
                maxrate: "3000k",
                bufsize: "6000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 12,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "commercial_30s_1080p_12frames_h264_4500k",
                width: 1920,
                height: 1080,
                bitrate: "4500k",
                maxrate: "5500k",
                bufsize: "11000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 12,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "commercial_30s_2k_12frames_h264_8000k",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 12,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_30s_commercial_sampling_case(&paths.ffmpeg, temp_dir.path(), &case).await;
            println!(
                "l3_30s_commercial_sampling_case={} source_duration_s=30 sampled_frames={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                case.sampled_frames,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            assert!(
                metrics.self_check_passed,
                "{} should pass with 12 sampled frames and 96 strategy regions; status={}",
                case.case_name, metrics.self_check_status
            );
        }
    }

    #[tokio::test]
    async fn l3_bilibili_hevc_mainstream_floor_records_cost_breakdown() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_bilibili_hevc_mainstream_floor_fixture_skip: {error}");
                return;
            }
        };
        if !ffmpeg_encoder_available(&paths.ffmpeg, "libx265").await {
            println!("l3_bilibili_hevc_mainstream_floor_fixture_skip: libx265 unavailable");
            return;
        }

        for case in [
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_1080p_12frames_hevc_4000k",
                width: 1920,
                height: 1080,
                bitrate: "4000k",
                maxrate: "5000k",
                bufsize: "10000k",
                codec: "libx265",
                codec_profile: Some("main"),
                crf: 20,
                extra_video_args: &["-x265-params", "rc-lookahead=20:ref=3:bframes=4:aq-mode=2"],
                max_regions: 96,
                sampled_frames: 12,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_12frames_hevc_6500k",
                width: 2560,
                height: 1440,
                bitrate: "6500k",
                maxrate: "8000k",
                bufsize: "16000k",
                codec: "libx265",
                codec_profile: Some("main"),
                crf: 20,
                extra_video_args: &["-x265-params", "rc-lookahead=20:ref=3:bframes=4:aq-mode=2"],
                max_regions: 96,
                sampled_frames: 12,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_30s_commercial_sampling_case(&paths.ffmpeg, temp_dir.path(), &case).await;
            println!(
                "l3_bilibili_hevc_mainstream_floor_case={} source_duration_s=30 sampled_frames={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                case.sampled_frames,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            assert!(
                metrics.self_check_passed,
                "{} should pass with 12 sampled frames and 96 strategy regions; status={}",
                case.case_name, metrics.self_check_status
            );
        }
    }

    #[tokio::test]
    async fn l3_bilibili_h264_hevc_cost_comparison_records_budget() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_bilibili_h264_hevc_cost_comparison_fixture_skip: {error}");
                return;
            }
        };
        if !ffmpeg_encoder_available(&paths.ffmpeg, "libx265").await {
            println!("l3_bilibili_h264_hevc_cost_comparison_fixture_skip: libx265 unavailable");
            return;
        }

        for (codec_family, case) in [
            (
                "h264",
                L3CommercialSamplingCase {
                    case_name: "bilibili_30s_1080p_12frames_h264_4500k_cost",
                    width: 1920,
                    height: 1080,
                    bitrate: "4500k",
                    maxrate: "5500k",
                    bufsize: "11000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 12,
                    region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                    video_filter: None,
                },
            ),
            (
                "hevc",
                L3CommercialSamplingCase {
                    case_name: "bilibili_30s_1080p_12frames_hevc_4000k_cost",
                    width: 1920,
                    height: 1080,
                    bitrate: "4000k",
                    maxrate: "5000k",
                    bufsize: "10000k",
                    codec: "libx265",
                    codec_profile: Some("main"),
                    crf: 20,
                    extra_video_args: &[
                        "-x265-params",
                        "rc-lookahead=20:ref=3:bframes=4:aq-mode=2",
                    ],
                    max_regions: 96,
                    sampled_frames: 12,
                    region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                    video_filter: None,
                },
            ),
            (
                "h264",
                L3CommercialSamplingCase {
                    case_name: "bilibili_30s_2k_12frames_h264_8000k_cost",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 12,
                    region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                    video_filter: None,
                },
            ),
            (
                "hevc",
                L3CommercialSamplingCase {
                    case_name: "bilibili_30s_2k_12frames_hevc_6500k_cost",
                    width: 2560,
                    height: 1440,
                    bitrate: "6500k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx265",
                    codec_profile: Some("main"),
                    crf: 20,
                    extra_video_args: &[
                        "-x265-params",
                        "rc-lookahead=20:ref=3:bframes=4:aq-mode=2",
                    ],
                    max_regions: 96,
                    sampled_frames: 12,
                    region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                    video_filter: None,
                },
            ),
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_30s_commercial_sampling_case(&paths.ffmpeg, temp_dir.path(), &case).await;
            println!(
                "l3_bilibili_h264_hevc_cost_comparison_case={} codec_family={} source_duration_s=30 sampled_frames={} max_regions={} resolution={}x{} bitrate={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                codec_family,
                case.sampled_frames,
                case.max_regions,
                case.width,
                case.height,
                case.bitrate,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            assert!(
                metrics.self_check_passed,
                "{} should pass in the Bilibili H.264/HEVC 30s cost comparison matrix; status={}",
                case.case_name, metrics.self_check_status
            );
        }
    }

    #[tokio::test]
    async fn l3_bilibili_hevc_texture_aware_records_cost_budget() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_bilibili_hevc_texture_aware_fixture_skip: {error}");
                return;
            }
        };
        if !ffmpeg_encoder_available(&paths.ffmpeg, "libx265").await {
            println!("l3_bilibili_hevc_texture_aware_fixture_skip: libx265 unavailable");
            return;
        }

        for case in [
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_1080p_hevc_4000k_16frames_texture_aware",
                width: 1920,
                height: 1080,
                bitrate: "4000k",
                maxrate: "5000k",
                bufsize: "10000k",
                codec: "libx265",
                codec_profile: Some("main"),
                crf: 20,
                extra_video_args: &["-x265-params", "rc-lookahead=20:ref=3:bframes=4:aq-mode=2"],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::TextureAware),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_hevc_6500k_16frames_texture_aware",
                width: 2560,
                height: 1440,
                bitrate: "6500k",
                maxrate: "8000k",
                bufsize: "16000k",
                codec: "libx265",
                codec_profile: Some("main"),
                crf: 20,
                extra_video_args: &["-x265-params", "rc-lookahead=20:ref=3:bframes=4:aq-mode=2"],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::TextureAware),
                video_filter: None,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_30s_commercial_sampling_case(&paths.ffmpeg, temp_dir.path(), &case).await;
            println!(
                "l3_bilibili_hevc_texture_aware_case={} source_duration_s=30 sampled_frames={} max_regions={} region_selection=texture_aware resolution={}x{} bitrate={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                case.sampled_frames,
                case.max_regions,
                case.width,
                case.height,
                case.bitrate,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            assert!(
                metrics.self_check_passed,
                "{} should pass in the Bilibili HEVC TextureAware 30s cost matrix; status={}",
                case.case_name, metrics.self_check_status
            );
        }
    }

    #[tokio::test]
    async fn l3_default_transcode_stable_h264_hevc_regression_records_cost_budget() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_default_transcode_stable_regression_fixture_skip: {error}");
                return;
            }
        };
        if !ffmpeg_encoder_available(&paths.ffmpeg, "libx265").await {
            println!("l3_default_transcode_stable_regression_fixture_skip: libx265 unavailable");
            return;
        }

        for case in [
            L3CommercialSamplingCase {
                case_name: "default_30s_720p_h264_2500k_12frames_core_default",
                width: 1280,
                height: 720,
                bitrate: "2500k",
                maxrate: "3000k",
                bufsize: "6000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 12,
                region_selection_mode: None,
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "default_30s_1080p_h264_6000k_16frames_core_default",
                width: 1920,
                height: 1080,
                bitrate: "6000k",
                maxrate: "8000k",
                bufsize: "16000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 20,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: None,
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "default_30s_2k_h264_8000k_16frames_core_default",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: None,
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "default_30s_1080p_hevc_4000k_16frames_core_default",
                width: 1920,
                height: 1080,
                bitrate: "4000k",
                maxrate: "5000k",
                bufsize: "10000k",
                codec: "libx265",
                codec_profile: Some("main"),
                crf: 20,
                extra_video_args: &["-x265-params", "rc-lookahead=20:ref=3:bframes=4:aq-mode=2"],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: None,
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "default_30s_2k_hevc_6500k_16frames_core_default",
                width: 2560,
                height: 1440,
                bitrate: "6500k",
                maxrate: "8000k",
                bufsize: "16000k",
                codec: "libx265",
                codec_profile: Some("main"),
                crf: 20,
                extra_video_args: &["-x265-params", "rc-lookahead=20:ref=3:bframes=4:aq-mode=2"],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: None,
                video_filter: None,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_30s_commercial_sampling_case(&paths.ffmpeg, temp_dir.path(), &case).await;
            println!(
                "l3_default_transcode_stable_regression_case={} source_duration_s=30 sampled_frames={} max_regions={} region_selection=core_default resolution={}x{} codec={} bitrate={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                case.sampled_frames,
                case.max_regions,
                case.width,
                case.height,
                case.codec,
                case.bitrate,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            assert!(
                metrics.self_check_passed,
                "{} should pass after the core default strategy applies the main battlefield budget; status={}",
                case.case_name, metrics.self_check_status
            );
        }
    }

    #[tokio::test]
    async fn l3_default_strategy_texture_diversity_records_cost_budget() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_default_strategy_texture_diversity_fixture_skip: {error}");
                return;
            }
        };

        for (texture_profile, source_lavfi, case) in [
            (
                "low_texture_grid_1080p_landscape",
                "color=c=gray:s=1920x1080:r=30:d=30,drawgrid=w=96:h=96:t=2:c=white@0.35",
                L3CommercialSamplingCase {
                    case_name: "default_30s_1080p_h264_6000k_low_texture_grid",
                    width: 1920,
                    height: 1080,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
            ),
            (
                "high_texture_1080p_landscape",
                "testsrc2=size=1920x1080:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                L3CommercialSamplingCase {
                    case_name: "default_30s_1080p_h264_6000k_high_texture",
                    width: 1920,
                    height: 1080,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
            ),
            (
                "high_texture_1080p_vertical",
                "testsrc2=size=1080x1920:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                L3CommercialSamplingCase {
                    case_name: "default_30s_1080p_vertical_h264_6000k_high_texture",
                    width: 1080,
                    height: 1920,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
            ),
            (
                "low_texture_grid_2k_landscape",
                "color=c=gray:s=2560x1440:r=30:d=30,drawgrid=w=128:h=128:t=2:c=white@0.35",
                L3CommercialSamplingCase {
                    case_name: "default_30s_2k_h264_8000k_low_texture_grid",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
            ),
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics = run_l3_30s_commercial_sampling_case_with_source(
                &paths.ffmpeg,
                temp_dir.path(),
                &case,
                Some(source_lavfi),
            )
            .await;
            println!(
                "l3_default_strategy_texture_diversity_case={} texture_profile={} source_duration_s=30 sampled_frames={} max_regions={} region_selection=core_default resolution={}x{} codec={} bitrate={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                texture_profile,
                case.sampled_frames,
                case.max_regions,
                case.width,
                case.height,
                case.codec,
                case.bitrate,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            assert!(
                metrics.self_check_passed,
                "{} should pass the default-strategy texture diversity matrix; status={}",
                case.case_name, metrics.self_check_status
            );
        }
    }

    #[tokio::test]
    async fn l3_default_strategy_real_content_risk_boundary_records_outcomes() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_default_strategy_risk_boundary_fixture_skip: {error}");
                return;
            }
        };

        for (risk_profile, source_lavfi, expect_pass, case) in [
            (
                "vertical_high_detail_lower_bitrate",
                "testsrc2=size=1080x1920:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                true,
                L3CommercialSamplingCase {
                    case_name: "risk_30s_1080p_vertical_h264_4500k_high_detail",
                    width: 1080,
                    height: 1920,
                    bitrate: "4500k",
                    maxrate: "5500k",
                    bufsize: "11000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
            ),
            (
                "extreme_programmatic_high_frequency",
                "nullsrc=s=1920x1080:r=30:d=30,geq=lum='mod(X*17+Y*31,256)':cb=128:cr=128",
                false,
                L3CommercialSamplingCase {
                    case_name: "risk_30s_1080p_h264_6000k_extreme_high_frequency",
                    width: 1920,
                    height: 1080,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
            ),
            (
                "temporal_random_noise",
                "nullsrc=s=1920x1080:r=30:d=30,geq=lum='mod(X*17+Y*31+N*47,256)':cb=128:cr=128",
                false,
                L3CommercialSamplingCase {
                    case_name: "risk_30s_1080p_h264_6000k_temporal_noise",
                    width: 1920,
                    height: 1080,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
            ),
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics = run_l3_30s_commercial_sampling_case_with_source(
                &paths.ffmpeg,
                temp_dir.path(),
                &case,
                Some(source_lavfi),
            )
            .await;
            println!(
                "l3_default_strategy_risk_boundary_case={} risk_profile={} expected={} source_duration_s=30 sampled_frames={} max_regions={} region_selection=core_default resolution={}x{} codec={} bitrate={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                risk_profile,
                if expect_pass { "pass" } else { "self_check_failed" },
                case.sampled_frames,
                case.max_regions,
                case.width,
                case.height,
                case.codec,
                case.bitrate,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            if expect_pass {
                assert!(
                    metrics.self_check_passed,
                    "{} should pass the real-content risk boundary matrix; status={}",
                    case.case_name, metrics.self_check_status
                );
            } else {
                assert!(
                    !metrics.self_check_passed
                        && metrics.self_check_status == "failed:self_check_failed",
                    "{} should stay classified as self_check_failed risk boundary; status={}",
                    case.case_name,
                    metrics.self_check_status
                );
            }
        }
    }

    #[tokio::test]
    async fn l3_platform_second_pass_transcode_risk_records_outcomes() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_platform_second_pass_transcode_risk_fixture_skip: {error}");
                return;
            }
        };

        for case in [
            L3PlatformSecondPassCase {
                risk_profile: "vertical_high_detail_6mbps_to_45mbps",
                source_lavfi: "testsrc2=size=1080x1920:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name: "second_pass_30s_1080p_vertical_high_detail_6000k_to_4500k",
                    width: 1080,
                    height: 1920,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "4500k",
                second_pass_maxrate: "5500k",
                second_pass_bufsize: "11000k",
                second_pass_crf: 23,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "bilibili_2k_8mbps_to_65mbps",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30",
                first_pass: L3CommercialSamplingCase {
                    case_name: "second_pass_30s_2k_landscape_8000k_to_6500k",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "6500k",
                second_pass_maxrate: "8000k",
                second_pass_bufsize: "16000k",
                second_pass_crf: 24,
                expect_pass: false,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_platform_second_pass_transcode_case(&paths.ffmpeg, temp_dir.path(), &case)
                    .await;
            println!(
                "l3_platform_second_pass_transcode_case={} risk_profile={} expected={} source_duration_s=30 sampled_frames={} max_regions={} region_selection=core_default resolution={}x{} codec={} first_pass_bitrate={} first_pass_crf={} second_pass_bitrate={} second_pass_crf={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_first_pass_ms={} ffmpeg_second_pass_ms={} ffmpeg_decode_second_pass_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.first_pass.case_name,
                case.risk_profile,
                if case.expect_pass { "pass" } else { "self_check_failed" },
                case.first_pass.sampled_frames,
                case.first_pass.max_regions,
                case.first_pass.width,
                case.first_pass.height,
                case.first_pass.codec,
                case.first_pass.bitrate,
                case.first_pass.crf,
                case.second_pass_bitrate,
                case.second_pass_crf,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_first_pass_ms,
                metrics.ffmpeg_second_pass_ms,
                metrics.ffmpeg_decode_second_pass_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            if case.expect_pass {
                assert!(
                    metrics.self_check_passed,
                    "{} should pass the platform second-pass transcode risk matrix; status={}",
                    case.first_pass.case_name, metrics.self_check_status
                );
            } else {
                assert!(
                    !metrics.self_check_passed
                        && metrics.self_check_status == "failed:self_check_failed",
                    "{} should stay classified as self_check_failed second-pass risk boundary; status={}",
                    case.first_pass.case_name,
                    metrics.self_check_status
                );
            }
        }
    }

    #[tokio::test]
    async fn l3_platform_second_pass_stability_diagnostics_records_budget_curve() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_platform_second_pass_stability_diagnostics_fixture_skip: {error}");
                return;
            }
        };

        for case in [
            L3PlatformSecondPassCase {
                risk_profile: "vertical_high_detail_20frames_96regions",
                source_lavfi: "testsrc2=size=1080x1920:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name: "second_pass_diag_1080p_vertical_6000k_to_4500k_20frames_96regions",
                    width: 1080,
                    height: 1920,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 20,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "4500k",
                second_pass_maxrate: "5500k",
                second_pass_bufsize: "11000k",
                second_pass_crf: 23,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "vertical_high_detail_16frames_128regions",
                source_lavfi: "testsrc2=size=1080x1920:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name: "second_pass_diag_1080p_vertical_6000k_to_4500k_16frames_128regions",
                    width: 1080,
                    height: 1920,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 128,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "4500k",
                second_pass_maxrate: "5500k",
                second_pass_bufsize: "11000k",
                second_pass_crf: 23,
                expect_pass: false,
            },
            L3PlatformSecondPassCase {
                risk_profile: "vertical_high_detail_transcode_stable_16frames_96regions",
                source_lavfi: "testsrc2=size=1080x1920:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name: "second_pass_diag_1080p_vertical_6000k_to_4500k_16frames_96regions_transcode_stable",
                    width: 1080,
                    height: 1920,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: Some(VideoVisualRegionSelectionMode::TranscodeStable),
                    video_filter: None,
                },
                second_pass_bitrate: "4500k",
                second_pass_maxrate: "5500k",
                second_pass_bufsize: "11000k",
                second_pass_crf: 23,
                expect_pass: false,
            },
            L3PlatformSecondPassCase {
                risk_profile: "bilibili_2k_20frames_96regions",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30",
                first_pass: L3CommercialSamplingCase {
                    case_name: "second_pass_diag_2k_8000k_to_6500k_20frames_96regions",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 20,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "6500k",
                second_pass_maxrate: "8000k",
                second_pass_bufsize: "16000k",
                second_pass_crf: 24,
                expect_pass: true,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_platform_second_pass_transcode_case(&paths.ffmpeg, temp_dir.path(), &case)
                    .await;
            println!(
                "l3_platform_second_pass_stability_diagnostic_case={} risk_profile={} expected={} source_duration_s=30 sampled_frames={} max_regions={} region_selection={} resolution={}x{} codec={} first_pass_bitrate={} second_pass_bitrate={} checked_frames={} confidence={:.3} self_check_status={} total_ms={}",
                case.first_pass.case_name,
                case.risk_profile,
                if case.expect_pass { "pass" } else { "observation" },
                case.first_pass.sampled_frames,
                case.first_pass.max_regions,
                case.first_pass
                    .region_selection_mode
                    .map(|mode| mode.as_str())
                    .unwrap_or("core_default"),
                case.first_pass.width,
                case.first_pass.height,
                case.first_pass.codec,
                case.first_pass.bitrate,
                case.second_pass_bitrate,
                metrics.checked_frames,
                metrics.confidence,
                metrics.self_check_status,
                metrics.total_ms
            );
            if case.expect_pass {
                assert!(
                    metrics.self_check_passed,
                    "{} should pass the platform second-pass stability diagnostic baseline; status={}",
                    case.first_pass.case_name,
                    metrics.self_check_status
                );
            }
        }
    }

    #[tokio::test]
    async fn l3_transcode_stable_second_pass_platform_matrix_records_generalization() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_transcode_stable_second_pass_platform_matrix_fixture_skip: {error}");
                return;
            }
        };
        let hevc_available = ffmpeg_encoder_available(&paths.ffmpeg, "libx265").await;

        for case in [
            L3PlatformSecondPassCase {
                risk_profile: "main_720p_h264_core_default_second_pass_regression",
                source_lavfi: "testsrc2=size=1280x720:rate=30:duration=30",
                first_pass: L3CommercialSamplingCase {
                    case_name: "core_default_30s_720p_h264_4000k_to_3000k_16frames",
                    width: 1280,
                    height: 720,
                    bitrate: "4000k",
                    maxrate: "5000k",
                    bufsize: "10000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 21,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "3000k",
                second_pass_maxrate: "4000k",
                second_pass_bufsize: "8000k",
                second_pass_crf: 23,
                expect_pass: false,
            },
            L3PlatformSecondPassCase {
                risk_profile: "bilibili_1080p_h264_transcode_stable",
                source_lavfi: "testsrc2=size=1920x1080:rate=30:duration=30,unsharp=5:5:0.6:3:3:0.3",
                first_pass: L3CommercialSamplingCase {
                    case_name: "transcode_stable_30s_1080p_h264_6000k_to_4500k_16frames",
                    width: 1920,
                    height: 1080,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: Some(VideoVisualRegionSelectionMode::TranscodeStable),
                    video_filter: None,
                },
                second_pass_bitrate: "4500k",
                second_pass_maxrate: "6000k",
                second_pass_bufsize: "12000k",
                second_pass_crf: 23,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "bilibili_2k_h264_transcode_stable",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30",
                first_pass: L3CommercialSamplingCase {
                    case_name: "transcode_stable_30s_2k_h264_8000k_to_6500k_16frames",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: Some(VideoVisualRegionSelectionMode::TranscodeStable),
                    video_filter: None,
                },
                second_pass_bitrate: "6500k",
                second_pass_maxrate: "8000k",
                second_pass_bufsize: "16000k",
                second_pass_crf: 24,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "bilibili_1080p_hevc_transcode_stable",
                source_lavfi: "testsrc2=size=1920x1080:rate=30:duration=30,unsharp=5:5:0.6:3:3:0.3",
                first_pass: L3CommercialSamplingCase {
                    case_name: "transcode_stable_30s_1080p_hevc_4000k_to_3200k_16frames",
                    width: 1920,
                    height: 1080,
                    bitrate: "4000k",
                    maxrate: "5500k",
                    bufsize: "11000k",
                    codec: "libx265",
                    codec_profile: None,
                    crf: 20,
                    extra_video_args: &["-tag:v", "hvc1"],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: Some(VideoVisualRegionSelectionMode::TranscodeStable),
                    video_filter: None,
                },
                second_pass_bitrate: "3200k",
                second_pass_maxrate: "4500k",
                second_pass_bufsize: "9000k",
                second_pass_crf: 24,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "bilibili_2k_hevc_transcode_stable",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30",
                first_pass: L3CommercialSamplingCase {
                    case_name: "transcode_stable_30s_2k_hevc_6500k_to_5200k_16frames",
                    width: 2560,
                    height: 1440,
                    bitrate: "6500k",
                    maxrate: "8500k",
                    bufsize: "17000k",
                    codec: "libx265",
                    codec_profile: None,
                    crf: 20,
                    extra_video_args: &["-tag:v", "hvc1"],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: Some(VideoVisualRegionSelectionMode::TranscodeStable),
                    video_filter: None,
                },
                second_pass_bitrate: "5200k",
                second_pass_maxrate: "7000k",
                second_pass_bufsize: "14000k",
                second_pass_crf: 24,
                expect_pass: true,
            },
        ] {
            if case.first_pass.codec == "libx265" && !hevc_available {
                println!(
                    "l3_transcode_stable_second_pass_platform_matrix_case={} skipped=libx265_unavailable",
                    case.first_pass.case_name
                );
                continue;
            }
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_platform_second_pass_transcode_case(&paths.ffmpeg, temp_dir.path(), &case)
                    .await;
            let region_selection = case
                .first_pass
                .region_selection_mode
                .map(|mode| mode.as_str())
                .unwrap_or("core_default");
            println!(
                "l3_transcode_stable_second_pass_platform_matrix_case={} risk_profile={} source_duration_s=30 sampled_frames={} max_regions={} region_selection={} resolution={}x{} codec={} first_pass_bitrate={} second_pass_bitrate={} checked_frames={} confidence={:.3} self_check_status={} total_ms={}",
                case.first_pass.case_name,
                case.risk_profile,
                case.first_pass.sampled_frames,
                case.first_pass.max_regions,
                region_selection,
                case.first_pass.width,
                case.first_pass.height,
                case.first_pass.codec,
                case.first_pass.bitrate,
                case.second_pass_bitrate,
                metrics.checked_frames,
                metrics.confidence,
                metrics.self_check_status,
                metrics.total_ms
            );
            if case.expect_pass {
                assert!(
                    metrics.self_check_passed,
                    "{} should pass the TranscodeStable second-pass generalization matrix; status={}",
                    case.first_pass.case_name, metrics.self_check_status
                );
            } else {
                assert!(
                    !metrics.self_check_passed,
                    "{} should remain a recorded second-pass risk boundary; status={}",
                    case.first_pass.case_name, metrics.self_check_status
                );
            }
        }
    }

    #[tokio::test]
    async fn l3_default_transcode_stable_second_pass_platform_matrix_records_cost_weight() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!(
                    "l3_default_transcode_stable_second_pass_platform_matrix_fixture_skip: {error}"
                );
                return;
            }
        };
        let hevc_available = ffmpeg_encoder_available(&paths.ffmpeg, "libx265").await;

        for case in [
            L3PlatformSecondPassCase {
                risk_profile: "main_720p_h264_core_default_second_pass_regression",
                source_lavfi: "testsrc2=size=1280x720:rate=30:duration=30",
                first_pass: L3CommercialSamplingCase {
                    case_name: "default_30s_720p_h264_4000k_to_3000k_16frames",
                    width: 1280,
                    height: 720,
                    bitrate: "4000k",
                    maxrate: "5000k",
                    bufsize: "10000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 21,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "3000k",
                second_pass_maxrate: "4000k",
                second_pass_bufsize: "8000k",
                second_pass_crf: 23,
                expect_pass: false,
            },
            L3PlatformSecondPassCase {
                risk_profile: "bilibili_1080p_h264_core_default_transcode_stable",
                source_lavfi: "testsrc2=size=1920x1080:rate=30:duration=30,unsharp=5:5:0.6:3:3:0.3",
                first_pass: L3CommercialSamplingCase {
                    case_name: "default_30s_1080p_h264_6000k_to_4500k_16frames",
                    width: 1920,
                    height: 1080,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "4500k",
                second_pass_maxrate: "6000k",
                second_pass_bufsize: "12000k",
                second_pass_crf: 23,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "bilibili_2k_h264_core_default_transcode_stable",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30",
                first_pass: L3CommercialSamplingCase {
                    case_name: "default_30s_2k_h264_8000k_to_6500k_16frames",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "6500k",
                second_pass_maxrate: "8000k",
                second_pass_bufsize: "16000k",
                second_pass_crf: 24,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "bilibili_1080p_hevc_core_default_transcode_stable",
                source_lavfi: "testsrc2=size=1920x1080:rate=30:duration=30,unsharp=5:5:0.6:3:3:0.3",
                first_pass: L3CommercialSamplingCase {
                    case_name: "default_30s_1080p_hevc_4000k_to_3200k_16frames",
                    width: 1920,
                    height: 1080,
                    bitrate: "4000k",
                    maxrate: "5500k",
                    bufsize: "11000k",
                    codec: "libx265",
                    codec_profile: None,
                    crf: 20,
                    extra_video_args: &["-tag:v", "hvc1"],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "3200k",
                second_pass_maxrate: "4500k",
                second_pass_bufsize: "9000k",
                second_pass_crf: 24,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "bilibili_2k_hevc_core_default_transcode_stable",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30",
                first_pass: L3CommercialSamplingCase {
                    case_name: "default_30s_2k_hevc_6500k_to_5200k_16frames",
                    width: 2560,
                    height: 1440,
                    bitrate: "6500k",
                    maxrate: "8500k",
                    bufsize: "17000k",
                    codec: "libx265",
                    codec_profile: None,
                    crf: 20,
                    extra_video_args: &["-tag:v", "hvc1"],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "5200k",
                second_pass_maxrate: "7000k",
                second_pass_bufsize: "14000k",
                second_pass_crf: 24,
                expect_pass: true,
            },
        ] {
            if case.first_pass.codec == "libx265" && !hevc_available {
                println!(
                    "l3_default_transcode_stable_second_pass_platform_matrix_case={} skipped=libx265_unavailable",
                    case.first_pass.case_name
                );
                continue;
            }
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_platform_second_pass_transcode_case(&paths.ffmpeg, temp_dir.path(), &case)
                    .await;
            println!(
                "l3_default_transcode_stable_second_pass_platform_matrix_case={} risk_profile={} source_duration_s=30 sampled_frames={} max_regions={} region_selection=core_default resolution={}x{} codec={} first_pass_bitrate={} second_pass_bitrate={} checked_frames={} confidence={:.3} self_check_status={} total_ms={}",
                case.first_pass.case_name,
                case.risk_profile,
                case.first_pass.sampled_frames,
                case.first_pass.max_regions,
                case.first_pass.width,
                case.first_pass.height,
                case.first_pass.codec,
                case.first_pass.bitrate,
                case.second_pass_bitrate,
                metrics.checked_frames,
                metrics.confidence,
                metrics.self_check_status,
                metrics.total_ms
            );
            if case.expect_pass {
                assert!(
                    metrics.self_check_passed,
                    "{} should pass through the core default TranscodeStable second-pass matrix; status={}",
                    case.first_pass.case_name, metrics.self_check_status
                );
            } else {
                assert!(
                    !metrics.self_check_passed
                        && metrics.self_check_status == "failed:self_check_failed",
                    "{} should remain a recorded core default second-pass risk boundary; status={}",
                    case.first_pass.case_name,
                    metrics.self_check_status
                );
            }
        }
    }

    #[tokio::test]
    async fn l3_default_transcode_stable_real_content_second_pass_matrix_records_outcomes() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!(
                    "l3_default_transcode_stable_real_content_second_pass_fixture_skip: {error}"
                );
                return;
            }
        };

        for case in [
            L3PlatformSecondPassCase {
                risk_profile: "1080p_landscape_high_detail_h264",
                source_lavfi: "testsrc2=size=1920x1080:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name:
                        "real_content_default_30s_1080p_landscape_h264_6000k_to_4500k_16frames",
                    width: 1920,
                    height: 1080,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "4500k",
                second_pass_maxrate: "6000k",
                second_pass_bufsize: "12000k",
                second_pass_crf: 23,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "1080p_vertical_high_detail_h264",
                source_lavfi: "testsrc2=size=1080x1920:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name:
                        "real_content_default_30s_1080p_vertical_h264_6000k_to_4500k_16frames",
                    width: 1080,
                    height: 1920,
                    bitrate: "6000k",
                    maxrate: "8000k",
                    bufsize: "16000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 20,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "4500k",
                second_pass_maxrate: "6000k",
                second_pass_bufsize: "12000k",
                second_pass_crf: 23,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "2k_landscape_regular_texture_h264",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30",
                first_pass: L3CommercialSamplingCase {
                    case_name: "real_content_default_30s_2k_regular_h264_8000k_to_6500k_16frames",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "6500k",
                second_pass_maxrate: "8000k",
                second_pass_bufsize: "16000k",
                second_pass_crf: 24,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "2k_landscape_high_detail_h264",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name:
                        "real_content_default_30s_2k_high_detail_h264_8000k_to_6500k_16frames",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "6500k",
                second_pass_maxrate: "8000k",
                second_pass_bufsize: "16000k",
                second_pass_crf: 24,
                expect_pass: false,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_platform_second_pass_transcode_case(&paths.ffmpeg, temp_dir.path(), &case)
                    .await;
            println!(
                "l3_default_transcode_stable_real_content_second_pass_case={} risk_profile={} source_duration_s=30 sampled_frames={} max_regions={} region_selection=core_default resolution={}x{} codec={} first_pass_bitrate={} second_pass_bitrate={} checked_frames={} confidence={:.3} self_check_status={} total_ms={}",
                case.first_pass.case_name,
                case.risk_profile,
                case.first_pass.sampled_frames,
                case.first_pass.max_regions,
                case.first_pass.width,
                case.first_pass.height,
                case.first_pass.codec,
                case.first_pass.bitrate,
                case.second_pass_bitrate,
                metrics.checked_frames,
                metrics.confidence,
                metrics.self_check_status,
                metrics.total_ms
            );
            if case.expect_pass {
                assert!(
                    metrics.self_check_passed,
                    "{} should pass the core default TranscodeStable real-content second-pass matrix; status={}",
                    case.first_pass.case_name, metrics.self_check_status
                );
            } else {
                assert!(
                    !metrics.self_check_passed
                        && metrics.self_check_status == "failed:self_check_failed",
                    "{} should stay classified as a real-content second-pass risk boundary; status={}",
                    case.first_pass.case_name,
                    metrics.self_check_status
                );
            }
        }
    }

    #[tokio::test]
    async fn l3_2k_high_detail_h264_second_pass_budget_strategy_records_outcomes() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_2k_high_detail_h264_second_pass_budget_fixture_skip: {error}");
                return;
            }
        };

        for case in [
            L3PlatformSecondPassCase {
                risk_profile: "2k_high_detail_20frames_96regions_h264",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name: "budget_2k_high_detail_h264_8000k_to_6500k_20frames_96regions",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 20,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "6500k",
                second_pass_maxrate: "8000k",
                second_pass_bufsize: "16000k",
                second_pass_crf: 24,
                expect_pass: false,
            },
            L3PlatformSecondPassCase {
                risk_profile: "2k_high_detail_16frames_128regions_h264",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name: "budget_2k_high_detail_h264_8000k_to_6500k_16frames_128regions",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 23,
                    extra_video_args: &[],
                    max_regions: 128,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "6500k",
                second_pass_maxrate: "8000k",
                second_pass_bufsize: "16000k",
                second_pass_crf: 24,
                expect_pass: false,
            },
            L3PlatformSecondPassCase {
                risk_profile: "2k_high_detail_higher_bitrate_16frames_96regions_h264",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name: "budget_2k_high_detail_h264_10000k_to_8000k_16frames_96regions",
                    width: 2560,
                    height: 1440,
                    bitrate: "10000k",
                    maxrate: "12000k",
                    bufsize: "24000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 21,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "8000k",
                second_pass_maxrate: "10000k",
                second_pass_bufsize: "20000k",
                second_pass_crf: 23,
                expect_pass: true,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_platform_second_pass_transcode_case(&paths.ffmpeg, temp_dir.path(), &case)
                    .await;
            println!(
                "l3_2k_high_detail_h264_second_pass_budget_case={} risk_profile={} expected={} source_duration_s=30 sampled_frames={} max_regions={} region_selection=core_default resolution={}x{} codec={} first_pass_bitrate={} second_pass_bitrate={} checked_frames={} confidence={:.3} self_check_status={} total_ms={}",
                case.first_pass.case_name,
                case.risk_profile,
                if case.expect_pass { "pass" } else { "self_check_failed" },
                case.first_pass.sampled_frames,
                case.first_pass.max_regions,
                case.first_pass.width,
                case.first_pass.height,
                case.first_pass.codec,
                case.first_pass.bitrate,
                case.second_pass_bitrate,
                metrics.checked_frames,
                metrics.confidence,
                metrics.self_check_status,
                metrics.total_ms
            );
            if case.expect_pass {
                assert!(
                    metrics.self_check_passed,
                    "{} should pass the 2K high-detail H.264 second-pass budget strategy matrix; status={}",
                    case.first_pass.case_name, metrics.self_check_status
                );
            } else {
                assert!(
                    !metrics.self_check_passed
                        && metrics.self_check_status == "failed:self_check_failed",
                    "{} should stay classified as a 2K high-detail H.264 budget risk boundary; status={}",
                    case.first_pass.case_name,
                    metrics.self_check_status
                );
            }
        }
    }

    #[tokio::test]
    async fn l3_2k_high_bitrate_content_candidate_matrix_records_outcomes() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_2k_high_bitrate_content_candidate_fixture_skip: {error}");
                return;
            }
        };
        let hevc_available = ffmpeg_encoder_available(&paths.ffmpeg, "libx265").await;

        for case in [
            L3PlatformSecondPassCase {
                risk_profile: "2k_high_bitrate_high_detail_h264",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name: "candidate_2k_high_detail_h264_10000k_to_8000k_16frames_96regions",
                    width: 2560,
                    height: 1440,
                    bitrate: "10000k",
                    maxrate: "12000k",
                    bufsize: "24000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 21,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "8000k",
                second_pass_maxrate: "10000k",
                second_pass_bufsize: "20000k",
                second_pass_crf: 23,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "2k_high_bitrate_low_texture_h264",
                source_lavfi: "testsrc=size=2560x1440:rate=30:duration=30,boxblur=2:1",
                first_pass: L3CommercialSamplingCase {
                    case_name: "candidate_2k_low_texture_h264_10000k_to_8000k_16frames_96regions",
                    width: 2560,
                    height: 1440,
                    bitrate: "10000k",
                    maxrate: "12000k",
                    bufsize: "24000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 21,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "8000k",
                second_pass_maxrate: "10000k",
                second_pass_bufsize: "20000k",
                second_pass_crf: 23,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "2k_high_bitrate_motion_texture_h264",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30,tmix=frames=3:weights='1 1 1',unsharp=5:5:0.5:3:3:0.2",
                first_pass: L3CommercialSamplingCase {
                    case_name: "candidate_2k_motion_texture_h264_10000k_to_8000k_16frames_96regions",
                    width: 2560,
                    height: 1440,
                    bitrate: "10000k",
                    maxrate: "12000k",
                    bufsize: "24000k",
                    codec: "libx264",
                    codec_profile: Some("high"),
                    crf: 21,
                    extra_video_args: &[],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "8000k",
                second_pass_maxrate: "10000k",
                second_pass_bufsize: "20000k",
                second_pass_crf: 23,
                expect_pass: true,
            },
            L3PlatformSecondPassCase {
                risk_profile: "2k_high_bitrate_high_detail_hevc",
                source_lavfi: "testsrc2=size=2560x1440:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
                first_pass: L3CommercialSamplingCase {
                    case_name: "candidate_2k_high_detail_hevc_8000k_to_6500k_16frames_96regions",
                    width: 2560,
                    height: 1440,
                    bitrate: "8000k",
                    maxrate: "10000k",
                    bufsize: "20000k",
                    codec: "libx265",
                    codec_profile: None,
                    crf: 20,
                    extra_video_args: &["-tag:v", "hvc1"],
                    max_regions: 96,
                    sampled_frames: 16,
                    region_selection_mode: None,
                    video_filter: None,
                },
                second_pass_bitrate: "6500k",
                second_pass_maxrate: "8000k",
                second_pass_bufsize: "16000k",
                second_pass_crf: 24,
                expect_pass: true,
            },
        ] {
            if case.first_pass.codec == "libx265" && !hevc_available {
                println!(
                    "l3_2k_high_bitrate_content_candidate_case={} skipped=libx265_unavailable",
                    case.first_pass.case_name
                );
                continue;
            }

            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_platform_second_pass_transcode_case(&paths.ffmpeg, temp_dir.path(), &case)
                    .await;
            println!(
                "l3_2k_high_bitrate_content_candidate_case={} risk_profile={} expected={} source_duration_s=30 sampled_frames={} max_regions={} region_selection=core_default resolution={}x{} codec={} first_pass_bitrate={} second_pass_bitrate={} checked_frames={} confidence={:.3} self_check_status={} total_ms={}",
                case.first_pass.case_name,
                case.risk_profile,
                if case.expect_pass { "pass" } else { "self_check_failed" },
                case.first_pass.sampled_frames,
                case.first_pass.max_regions,
                case.first_pass.width,
                case.first_pass.height,
                case.first_pass.codec,
                case.first_pass.bitrate,
                case.second_pass_bitrate,
                metrics.checked_frames,
                metrics.confidence,
                metrics.self_check_status,
                metrics.total_ms
            );
            if case.expect_pass {
                assert!(
                    metrics.self_check_passed,
                    "{} should pass the 2K high-bitrate content candidate matrix; status={}",
                    case.first_pass.case_name, metrics.self_check_status
                );
            } else {
                assert!(
                    !metrics.self_check_passed
                        && metrics.self_check_status == "failed:self_check_failed",
                    "{} should stay classified as a 2K high-bitrate content risk boundary; status={}",
                    case.first_pass.case_name,
                    metrics.self_check_status
                );
            }
        }
    }

    #[tokio::test]
    async fn l3_2k_high_bitrate_release_sample_pool_records_thresholds() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_2k_high_bitrate_release_sample_pool_fixture_skip: {error}");
                return;
            }
        };
        let hevc_available = ffmpeg_encoder_available(&paths.ffmpeg, "libx265").await;
        let samples = l3_2k_high_bitrate_release_sample_pool_cases();
        assert_eq!(
            samples.len(),
            24,
            "2K high-bitrate release sample pool must keep 24 samples"
        );
        assert_sample_definition_count(&samples, "H264-HD", 6);
        assert_sample_definition_count(&samples, "H264-LT", 4);
        assert_sample_definition_count(&samples, "H264-MT", 4);
        assert_sample_definition_count(&samples, "H264-RISK", 2);
        assert_sample_definition_count(&samples, "HEVC-HD", 4);
        assert_sample_definition_count(&samples, "HEVC-MIX", 4);
        let full_release_pool =
            std::env::var("HIDDENSHIELD_L3_FULL_RELEASE_POOL").as_deref() == Ok("1");
        let samples_to_run = if full_release_pool {
            samples.to_vec()
        } else {
            first_sample_per_group(&samples)
        };

        let mut outcomes = Vec::new();
        let mut hevc_skipped = 0usize;
        for sample in samples_to_run {
            if sample.case.first_pass.codec == "libx265" && !hevc_available {
                hevc_skipped += 1;
                println!(
                    "l3_2k_high_bitrate_release_sample_pool_case={} group={} failure_attribution=encoder_unavailable skipped=libx265_unavailable",
                    sample.case.first_pass.case_name,
                    sample.group
                );
                continue;
            }

            let temp_dir = tempfile::tempdir().unwrap();
            let metrics = run_l3_platform_second_pass_transcode_case(
                &paths.ffmpeg,
                temp_dir.path(),
                &sample.case,
            )
            .await;
            let attribution = classify_l3_high_bitrate_release_sample(
                sample.min_confidence,
                sample.failure_attribution,
                &metrics,
            );
            let passed_threshold = attribution == "pass" || attribution == "risk_boundary_expected";

            println!(
                "l3_2k_high_bitrate_release_sample_pool_case={} group={} risk_profile={} expected_attribution={} observed_attribution={} source_duration_s=30 sampled_frames={} max_regions={} region_selection=core_default resolution={}x{} codec={} first_pass_bitrate={} second_pass_bitrate={} min_confidence={:.3} checked_frames={} confidence={:.3} self_check_status={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_first_pass_ms={} ffmpeg_second_pass_ms={} ffmpeg_decode_second_pass_ms={} core_self_check_ms={} total_ms={}",
                sample.case.first_pass.case_name,
                sample.group,
                sample.case.risk_profile,
                sample.failure_attribution,
                attribution,
                sample.case.first_pass.sampled_frames,
                sample.case.first_pass.max_regions,
                sample.case.first_pass.width,
                sample.case.first_pass.height,
                sample.case.first_pass.codec,
                sample.case.first_pass.bitrate,
                sample.case.second_pass_bitrate,
                sample.min_confidence,
                metrics.checked_frames,
                metrics.confidence,
                metrics.self_check_status,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_first_pass_ms,
                metrics.ffmpeg_second_pass_ms,
                metrics.ffmpeg_decode_second_pass_ms,
                metrics.core_self_check_ms,
                metrics.total_ms
            );

            outcomes.push(L3HighBitrateReleaseSampleOutcome {
                group: sample.group.to_string(),
                case_name: sample.case.first_pass.case_name.to_string(),
                failure_attribution: attribution.to_string(),
                confidence: metrics.confidence,
                self_check_status: metrics.self_check_status,
                passed_threshold,
            });
        }

        if full_release_pool {
            assert_group_count(&outcomes, "H264-HD", 6);
            assert_group_count(&outcomes, "H264-LT", 4);
            assert_group_count(&outcomes, "H264-MT", 4);
            assert_group_count(&outcomes, "H264-RISK", 2);
            if hevc_available {
                assert_eq!(hevc_skipped, 0);
                assert_group_count(&outcomes, "HEVC-HD", 4);
                assert_group_count(&outcomes, "HEVC-MIX", 4);
            } else {
                assert_eq!(hevc_skipped, 8);
                assert_group_count(&outcomes, "HEVC-HD", 0);
                assert_group_count(&outcomes, "HEVC-MIX", 0);
            }
        } else {
            assert_group_count(&outcomes, "H264-HD", 1);
            assert_group_count(&outcomes, "H264-LT", 1);
            assert_group_count(&outcomes, "H264-MT", 1);
            assert_group_count(&outcomes, "H264-RISK", 1);
            if hevc_available {
                assert_eq!(hevc_skipped, 0);
                assert_group_count(&outcomes, "HEVC-HD", 1);
                assert_group_count(&outcomes, "HEVC-MIX", 1);
            } else {
                assert_eq!(hevc_skipped, 2);
                assert_group_count(&outcomes, "HEVC-HD", 0);
                assert_group_count(&outcomes, "HEVC-MIX", 0);
            }
        }

        assert_group_all_at_least(&outcomes, "H264-HD", 0.950);
        assert_group_all_at_least(&outcomes, "H264-LT", 0.950);
        assert_group_all_at_least(&outcomes, "H264-MT", 0.950);
        if full_release_pool {
            assert_group_average_at_least(&outcomes, "H264-HD", 0.970);
            assert_group_average_at_least(&outcomes, "H264-LT", 0.980);
            assert_group_average_at_least(&outcomes, "H264-MT", 0.980);
        }
        assert!(
            outcomes
                .iter()
                .filter(|outcome| outcome.group == "H264-RISK")
                .all(|outcome| outcome.failure_attribution == "risk_boundary_expected"),
            "H264-RISK samples must be recorded only as risk_boundary_expected"
        );

        if hevc_available {
            assert_group_all_at_least(&outcomes, "HEVC-HD", 0.970);
            assert_group_all_at_least(&outcomes, "HEVC-MIX", 0.970);
            if full_release_pool {
                assert_group_average_at_least(&outcomes, "HEVC-HD", 0.990);
                assert_group_average_at_least(&outcomes, "HEVC-MIX", 0.990);
            }
        }

        let h264_hd_min = group_min_confidence(&outcomes, "H264-HD");
        let h264_hd_avg = group_average_confidence(&outcomes, "H264-HD");
        let release_blocked_by_h264_hd = h264_hd_min < 0.950 || h264_hd_avg < 0.970;
        println!(
            "l3_2k_high_bitrate_release_sample_pool_summary h264_hd_min_confidence={:.3} h264_hd_avg_confidence={:.3} release_status={}",
            h264_hd_min,
            h264_hd_avg,
            if release_blocked_by_h264_hd {
                "release_blocked_h264_hd_confidence_below_threshold"
            } else {
                "release_thresholds_met"
            }
        );
        assert!(
            !release_blocked_by_h264_hd,
            "H264-HD must meet 0.950 per-sample and 0.970 group-average thresholds before L3 can enter the formal release gate"
        );
    }

    #[tokio::test]
    async fn l3_2k_h264_strategy_density_budget_records_confidence_curve() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_2k_h264_strategy_density_budget_fixture_skip: {error}");
                return;
            }
        };

        for case in [
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_12frames_h264_8000k_regions_96",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 12,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_12frames_h264_8000k_regions_128",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 128,
                sampled_frames: 12,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_12frames_h264_8000k_regions_160",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 160,
                sampled_frames: 12,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_30s_commercial_sampling_case(&paths.ffmpeg, temp_dir.path(), &case).await;
            println!(
                "l3_2k_h264_strategy_density_budget_case={} source_duration_s=30 sampled_frames={} max_regions={} resolution={}x{} bitrate={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                case.sampled_frames,
                case.max_regions,
                case.width,
                case.height,
                case.bitrate,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            assert!(
                metrics.self_check_passed,
                "{} should pass while recording the 2K H.264 strategy-density confidence curve; status={}",
                case.case_name, metrics.self_check_status
            );
        }
    }

    #[tokio::test]
    async fn l3_2k_h264_sample_count_budget_records_confidence_curve() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_2k_h264_sample_count_budget_fixture_skip: {error}");
                return;
            }
        };

        for case in [
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_h264_8000k_12frames_regions_96",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 12,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_h264_8000k_16frames_regions_96",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_h264_8000k_20frames_regions_96",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 20,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_30s_commercial_sampling_case(&paths.ffmpeg, temp_dir.path(), &case).await;
            println!(
                "l3_2k_h264_sample_count_budget_case={} source_duration_s=30 sampled_frames={} max_regions={} resolution={}x{} bitrate={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                case.sampled_frames,
                case.max_regions,
                case.width,
                case.height,
                case.bitrate,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            assert!(
                metrics.self_check_passed,
                "{} should pass while recording the 2K H.264 sample-count confidence curve; status={}",
                case.case_name, metrics.self_check_status
            );
        }
    }

    #[tokio::test]
    async fn l3_2k_h264_region_quality_budget_records_confidence_curve() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_2k_h264_region_quality_budget_fixture_skip: {error}");
                return;
            }
        };

        for case in [
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_h264_8000k_16frames_seeded_random_regions_96",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_h264_8000k_16frames_center_safe_regions_96",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::CenterSafeGrid),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_h264_8000k_16frames_distributed_regions_96",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::DistributedGrid),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_h264_8000k_16frames_texture_aware_regions_96",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::TextureAware),
                video_filter: None,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_30s_commercial_sampling_case(&paths.ffmpeg, temp_dir.path(), &case).await;
            println!(
                "l3_2k_h264_region_quality_budget_case={} source_duration_s=30 sampled_frames={} max_regions={} region_selection={} resolution={}x{} bitrate={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                case.sampled_frames,
                case.max_regions,
                case.region_selection_mode
                    .map(|mode| mode.as_str())
                    .unwrap_or("core_default"),
                case.width,
                case.height,
                case.bitrate,
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            if case.region_selection_mode == Some(VideoVisualRegionSelectionMode::SeededRandom) {
                assert!(
                    metrics.self_check_passed,
                    "{} should preserve the current default region-selection baseline; status={}",
                    case.case_name, metrics.self_check_status
                );
            }
        }
    }

    #[tokio::test]
    async fn l3_platform_timing_budget_records_16frame_seeded_costs() {
        let paths = match ffmpeg::detect_ffmpeg().await {
            Ok(paths) => paths,
            Err(error) => {
                println!("l3_platform_timing_budget_fixture_skip: {error}");
                return;
            }
        };

        for case in [
            L3CommercialSamplingCase {
                case_name: "douyin_30s_1080p_vertical_h264_4500k_16frames_seeded",
                width: 1080,
                height: 1920,
                bitrate: "4500k",
                maxrate: "5500k",
                bufsize: "11000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "xiaohongshu_30s_1080p_vertical_h264_6000k_16frames_seeded",
                width: 1080,
                height: 1440,
                bitrate: "6000k",
                maxrate: "8000k",
                bufsize: "16000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 20,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_1080p_landscape_h264_6000k_16frames_seeded",
                width: 1920,
                height: 1080,
                bitrate: "6000k",
                maxrate: "8000k",
                bufsize: "16000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 20,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_landscape_h264_8000k_16frames_seeded",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::SeededRandom),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "douyin_30s_1080p_vertical_h264_4500k_16frames_texture_aware",
                width: 1080,
                height: 1920,
                bitrate: "4500k",
                maxrate: "5500k",
                bufsize: "11000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::TextureAware),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "xiaohongshu_30s_1080p_vertical_h264_6000k_16frames_texture_aware",
                width: 1080,
                height: 1440,
                bitrate: "6000k",
                maxrate: "8000k",
                bufsize: "16000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 20,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::TextureAware),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_1080p_landscape_h264_6000k_16frames_texture_aware",
                width: 1920,
                height: 1080,
                bitrate: "6000k",
                maxrate: "8000k",
                bufsize: "16000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 20,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::TextureAware),
                video_filter: None,
            },
            L3CommercialSamplingCase {
                case_name: "bilibili_30s_2k_landscape_h264_8000k_16frames_texture_aware",
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 23,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: Some(VideoVisualRegionSelectionMode::TextureAware),
                video_filter: None,
            },
        ] {
            let temp_dir = tempfile::tempdir().unwrap();
            let metrics =
                run_l3_30s_commercial_sampling_case(&paths.ffmpeg, temp_dir.path(), &case).await;
            println!(
                "l3_platform_timing_budget_case={} source_duration_s=30 sampled_frames={} max_regions={} region_selection={} resolution={}x{} bitrate={} video_filter={} ffmpeg_source_and_sample_ms={} core_embed_ms={} ffmpeg_sample_roundtrip_ms={} core_self_check_ms={} self_check_status={} total_ms={}",
                case.case_name,
                case.sampled_frames,
                case.max_regions,
                case.region_selection_mode
                    .map(|mode| mode.as_str())
                    .unwrap_or("core_default"),
                case.width,
                case.height,
                case.bitrate,
                case.video_filter.unwrap_or("none"),
                metrics.ffmpeg_source_and_sample_ms,
                metrics.core_embed_ms,
                metrics.ffmpeg_sample_roundtrip_ms,
                metrics.core_self_check_ms,
                metrics.self_check_status,
                metrics.total_ms
            );
            assert!(
                metrics.self_check_passed,
                "{} should pass while recording the L3 platform timing budget; status={}",
                case.case_name, metrics.self_check_status
            );
        }
    }

    async fn encode_and_decode_l3_y_plane_fixture(
        ffmpeg: &Path,
        temp_dir: &Path,
        written_y_plane: &Path,
        crf: u8,
    ) -> Vec<watermark_core::VideoFramePlane> {
        encode_and_decode_l3_y_plane_fixture_with_options(
            ffmpeg,
            temp_dir,
            written_y_plane,
            &format!("crf-{crf}"),
            512,
            512,
            4,
            crf,
            "ultrafast",
            None,
        )
        .await
    }

    async fn run_l3_30s_commercial_sampling_case(
        ffmpeg: &Path,
        temp_dir: &Path,
        case: &L3CommercialSamplingCase<'_>,
    ) -> L3CommercialSamplingMetrics {
        run_l3_30s_commercial_sampling_case_with_source(ffmpeg, temp_dir, case, None).await
    }

    async fn run_l3_30s_commercial_sampling_case_with_source(
        ffmpeg: &Path,
        temp_dir: &Path,
        case: &L3CommercialSamplingCase<'_>,
        source_lavfi: Option<&str>,
    ) -> L3CommercialSamplingMetrics {
        let total_started_at = std::time::Instant::now();
        let source_video = temp_dir.join(format!("{}-source.mp4", case.case_name));
        let raw_y_plane = temp_dir.join(format!("{}-sampled.gray10le", case.case_name));
        let written_y_plane = temp_dir.join(format!("{}-written.gray", case.case_name));
        const SOURCE_DURATION_SECONDS: usize = 30;
        let source_lavfi = source_lavfi.map(ToString::to_string).unwrap_or_else(|| {
            format!(
                "testsrc2=size={}x{}:rate=30:duration={SOURCE_DURATION_SECONDS}",
                case.width, case.height
            )
        });

        let ffmpeg_source_started_at = std::time::Instant::now();
        run_ffmpeg_test_command(
            ffmpeg,
            &[
                "-y",
                "-f",
                "lavfi",
                "-i",
                &source_lavfi,
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p10le",
                &source_video.to_string_lossy(),
            ],
            "create L3 30s commercial sampling source fixture",
        )
        .await;

        run_ffmpeg_test_command(
            ffmpeg,
            &[
                "-y",
                "-i",
                &source_video.to_string_lossy(),
                "-vf",
                &format!("fps={}/{SOURCE_DURATION_SECONDS}", case.sampled_frames),
                "-frames:v",
                &case.sampled_frames.to_string(),
                "-map",
                "0:v:0",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray10le",
                &raw_y_plane.to_string_lossy(),
            ],
            "extract L3 30s commercial sampling Y-plane fixture",
        )
        .await;
        let ffmpeg_source_and_sample_ms = ffmpeg_source_started_at.elapsed().as_millis();

        let raw = std::fs::read(&raw_y_plane).unwrap();
        assert_eq!(
            raw.len(),
            case.width * case.height * case.sampled_frames * 2
        );
        let samples = raw
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let mut frames = samples
            .chunks_exact(case.width * case.height)
            .take(case.sampled_frames)
            .map(|frame_samples| {
                video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                    width: case.width as u32,
                    height: case.height as u32,
                    stride_samples: case.width,
                    samples: frame_samples,
                    bit_depth: VideoLumaBitDepth::Ten,
                    color_range: VideoLumaColorRange::Limited,
                    target_profile: VideoVisualProfile::LumaDctMidBandV1,
                })
                .unwrap()
            })
            .collect::<Vec<_>>();

        let core_started_at = std::time::Instant::now();
        let source_sha = parse_sha256_hex_32(
            &hash::sha256_of_file(source_video.to_string_lossy().as_ref()).unwrap(),
        )
        .unwrap();
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: (SOURCE_DURATION_SECONDS * 1_000) as u64,
        })
        .unwrap();
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "l3-core-fixture",
            device_identity: "desktop-ffmpeg-commercial-sampling",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: watermark::AIContentFlags::default(),
        })
        .unwrap();
        let strategy_input = VideoVisualStrategyBuildInput {
            task_id: case.case_name,
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: case.max_regions,
        };
        let strategy = match case.region_selection_mode {
            Some(mode) => derive_video_visual_strategy_with_region_selection(strategy_input, mode),
            None => derive_video_visual_strategy(strategy_input),
        }
        .unwrap();

        embed_video_visual_dct_frames(&mut frames, &strategy, &payload).unwrap();
        let core_embed_ms = core_started_at.elapsed().as_millis();

        let written = frames
            .iter()
            .flat_map(|frame| frame.luma_pixels())
            .collect::<Vec<_>>();
        std::fs::write(&written_y_plane, written).unwrap();

        let ffmpeg_roundtrip_started_at = std::time::Instant::now();
        let decoded_frames = encode_and_decode_l3_y_plane_fixture_with_profile(
            ffmpeg,
            temp_dir,
            &written_y_plane,
            case.case_name,
            case.width,
            case.height,
            case.sampled_frames,
            L3VideoEncodeProfile {
                codec: case.codec,
                preset: "medium",
                crf: case.crf,
                profile: case.codec_profile,
                level: Some("5.1"),
                maxrate: Some(case.maxrate),
                bufsize: Some(case.bufsize),
                video_bitrate: Some(case.bitrate),
                input_rate: 30,
                output_rate: Some(30),
                gop: Some(60),
                keyint_min: Some(30),
                video_filter: case.video_filter,
                extra_video_args: case.extra_video_args,
            },
        )
        .await;
        let ffmpeg_sample_roundtrip_ms = ffmpeg_roundtrip_started_at.elapsed().as_millis();

        let core_self_check_started_at = std::time::Instant::now();
        let self_check_result =
            self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
                strategy: &strategy,
                observed_strategy_digest: &strategy.strategy_digest,
                frames: &decoded_frames,
                expected_payload: &payload,
            });
        let self_check_passed = self_check_result
            .as_ref()
            .map(|self_check| self_check.confidence >= strategy.self_check_threshold)
            .unwrap_or(false);
        let self_check_status = self_check_result
            .as_ref()
            .map(|self_check| format!("passed:{:.3}", self_check.confidence))
            .unwrap_or_else(|error| format!("failed:{}", error.code_str()));
        let core_self_check_ms = core_self_check_started_at.elapsed().as_millis();

        L3CommercialSamplingMetrics {
            ffmpeg_source_and_sample_ms,
            core_embed_ms,
            ffmpeg_sample_roundtrip_ms,
            core_self_check_ms,
            total_ms: total_started_at.elapsed().as_millis(),
            self_check_status,
            self_check_passed,
        }
    }

    async fn run_l3_platform_second_pass_transcode_case(
        ffmpeg: &Path,
        temp_dir: &Path,
        case: &L3PlatformSecondPassCase<'_>,
    ) -> L3PlatformSecondPassMetrics {
        let total_started_at = std::time::Instant::now();
        let first_pass = &case.first_pass;
        let source_video = temp_dir.join(format!("{}-source.mp4", first_pass.case_name));
        let raw_y_plane = temp_dir.join(format!("{}-sampled.gray10le", first_pass.case_name));
        let written_y_plane = temp_dir.join(format!("{}-written.gray", first_pass.case_name));
        let protected_video = temp_dir.join(format!("{}-first-pass.mp4", first_pass.case_name));
        let second_pass_video = temp_dir.join(format!("{}-second-pass.mp4", first_pass.case_name));
        const SOURCE_DURATION_SECONDS: usize = 30;

        let ffmpeg_source_started_at = std::time::Instant::now();
        run_ffmpeg_test_command(
            ffmpeg,
            &[
                "-y",
                "-f",
                "lavfi",
                "-i",
                case.source_lavfi,
                "-c:v",
                "libx264",
                "-preset",
                "ultrafast",
                "-pix_fmt",
                "yuv420p10le",
                &source_video.to_string_lossy(),
            ],
            "create L3 platform second-pass source fixture",
        )
        .await;

        run_ffmpeg_test_command(
            ffmpeg,
            &[
                "-y",
                "-i",
                &source_video.to_string_lossy(),
                "-vf",
                &format!(
                    "fps={}/{SOURCE_DURATION_SECONDS}",
                    first_pass.sampled_frames
                ),
                "-frames:v",
                &first_pass.sampled_frames.to_string(),
                "-map",
                "0:v:0",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray10le",
                &raw_y_plane.to_string_lossy(),
            ],
            "extract L3 platform second-pass Y-plane fixture",
        )
        .await;
        let ffmpeg_source_and_sample_ms = ffmpeg_source_started_at.elapsed().as_millis();

        let raw = std::fs::read(&raw_y_plane).unwrap();
        assert_eq!(
            raw.len(),
            first_pass.width * first_pass.height * first_pass.sampled_frames * 2
        );
        let samples = raw
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        let mut frames = samples
            .chunks_exact(first_pass.width * first_pass.height)
            .take(first_pass.sampled_frames)
            .map(|frame_samples| {
                video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                    width: first_pass.width as u32,
                    height: first_pass.height as u32,
                    stride_samples: first_pass.width,
                    samples: frame_samples,
                    bit_depth: VideoLumaBitDepth::Ten,
                    color_range: VideoLumaColorRange::Limited,
                    target_profile: VideoVisualProfile::LumaDctMidBandV1,
                })
                .unwrap()
            })
            .collect::<Vec<_>>();

        let core_started_at = std::time::Instant::now();
        let source_sha = sha256_32_of_bytes(
            format!(
                "hidden-shield-l3-release-fixture-v1|case={}|lavfi={}|duration_s={SOURCE_DURATION_SECONDS}|sampled_frames={}",
                first_pass.case_name, case.source_lavfi, first_pass.sampled_frames
            )
            .as_bytes(),
        );
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: (SOURCE_DURATION_SECONDS * 1_000) as u64,
        })
        .unwrap();
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "l3-core-fixture",
            device_identity: "desktop-ffmpeg-platform-second-pass",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: watermark::AIContentFlags::default(),
        })
        .unwrap();
        let strategy_input = VideoVisualStrategyBuildInput {
            task_id: first_pass.case_name,
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: first_pass.max_regions,
        };
        let strategy = match first_pass.region_selection_mode {
            Some(mode) => derive_video_visual_strategy_with_region_selection(strategy_input, mode),
            None => derive_video_visual_strategy(strategy_input),
        }
        .unwrap();

        embed_video_visual_dct_frames(&mut frames, &strategy, &payload).unwrap();
        let core_embed_ms = core_started_at.elapsed().as_millis();

        let written = frames
            .iter()
            .flat_map(|frame| frame.luma_pixels())
            .collect::<Vec<_>>();
        std::fs::write(&written_y_plane, written).unwrap();

        let ffmpeg_first_pass_started_at = std::time::Instant::now();
        encode_l3_y_plane_fixture_to_video(
            ffmpeg,
            &written_y_plane,
            &protected_video,
            first_pass.width,
            first_pass.height,
            first_pass.sampled_frames,
            L3VideoEncodeProfile {
                codec: first_pass.codec,
                preset: "medium",
                crf: first_pass.crf,
                profile: first_pass.codec_profile,
                level: Some("5.1"),
                maxrate: Some(first_pass.maxrate),
                bufsize: Some(first_pass.bufsize),
                video_bitrate: Some(first_pass.bitrate),
                input_rate: 30,
                output_rate: Some(30),
                gop: Some(60),
                keyint_min: Some(30),
                video_filter: first_pass.video_filter,
                extra_video_args: first_pass.extra_video_args,
            },
        )
        .await;
        let ffmpeg_first_pass_ms = ffmpeg_first_pass_started_at.elapsed().as_millis();

        let ffmpeg_second_pass_started_at = std::time::Instant::now();
        transcode_l3_video_fixture(
            ffmpeg,
            &protected_video,
            &second_pass_video,
            L3VideoEncodeProfile {
                codec: first_pass.codec,
                preset: "medium",
                crf: case.second_pass_crf,
                profile: first_pass.codec_profile,
                level: Some("5.1"),
                maxrate: Some(case.second_pass_maxrate),
                bufsize: Some(case.second_pass_bufsize),
                video_bitrate: Some(case.second_pass_bitrate),
                input_rate: 30,
                output_rate: Some(30),
                gop: Some(60),
                keyint_min: Some(30),
                video_filter: None,
                extra_video_args: first_pass.extra_video_args,
            },
        )
        .await;
        let ffmpeg_second_pass_ms = ffmpeg_second_pass_started_at.elapsed().as_millis();

        let ffmpeg_decode_second_pass_started_at = std::time::Instant::now();
        let decoded_frames = decode_l3_video_fixture_to_y_planes(
            ffmpeg,
            temp_dir,
            &second_pass_video,
            first_pass.case_name,
            first_pass.width,
            first_pass.height,
            first_pass.sampled_frames,
        )
        .await;
        let ffmpeg_decode_second_pass_ms =
            ffmpeg_decode_second_pass_started_at.elapsed().as_millis();

        let core_self_check_started_at = std::time::Instant::now();
        let self_check_result =
            self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
                strategy: &strategy,
                observed_strategy_digest: &strategy.strategy_digest,
                frames: &decoded_frames,
                expected_payload: &payload,
            });
        let self_check_passed = self_check_result
            .as_ref()
            .map(|self_check| self_check.confidence >= strategy.self_check_threshold)
            .unwrap_or(false);
        let checked_frames = self_check_result
            .as_ref()
            .map(|self_check| self_check.checked_frames)
            .unwrap_or(0);
        let confidence = self_check_result
            .as_ref()
            .map(|self_check| self_check.confidence)
            .unwrap_or(0.0);
        let self_check_status = self_check_result
            .as_ref()
            .map(|self_check| format!("passed:{:.3}", self_check.confidence))
            .unwrap_or_else(|error| format!("failed:{}", error.code_str()));
        let core_self_check_ms = core_self_check_started_at.elapsed().as_millis();

        L3PlatformSecondPassMetrics {
            ffmpeg_source_and_sample_ms,
            core_embed_ms,
            ffmpeg_first_pass_ms,
            ffmpeg_second_pass_ms,
            ffmpeg_decode_second_pass_ms,
            core_self_check_ms,
            total_ms: total_started_at.elapsed().as_millis(),
            self_check_status,
            self_check_passed,
            checked_frames,
            confidence,
        }
    }

    fn l3_2k_high_bitrate_release_sample_pool_cases(
    ) -> [L3HighBitrateReleaseSampleCase<'static>; 24] {
        [
            h264_hd_release_sample(
                "release_2k_h264_hd_testsrc2_unsharp_10000k_to_8000k",
                "testsrc2=size=2560x1440:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
            ),
            h264_hd_release_sample(
                "release_2k_h264_hd_grid_unsharp_10000k_to_8000k",
                "color=c=gray:s=2560x1440:r=30:d=30,drawgrid=w=96:h=96:t=2:c=white@0.35,unsharp=5:5:0.7:3:3:0.3",
            ),
            h264_hd_release_sample(
                "release_2k_h264_hd_frequency_10000k_to_8000k",
                "nullsrc=s=2560x1440:r=30:d=30,geq=lum='mod(X*11+Y*19,256)':cb=128:cr=128",
            ),
            h264_hd_release_sample(
                "release_2k_h264_hd_fine_grid_10000k_to_8000k",
                "color=c=gray:s=2560x1440:r=30:d=30,drawgrid=w=64:h=64:t=1:c=white@0.45,unsharp=5:5:0.9:3:3:0.4",
            ),
            h264_hd_release_sample(
                "release_2k_h264_hd_zoneplate_10000k_to_8000k",
                "testsrc=size=2560x1440:rate=30:duration=30,edgedetect=mode=colormix:high=0.18:low=0.08,format=yuv420p",
            ),
            h264_hd_release_sample(
                "release_2k_h264_hd_blended_detail_10000k_to_8000k",
                "testsrc2=size=2560x1440:rate=30:duration=30,tmix=frames=2:weights='1 1',unsharp=7:7:0.6:3:3:0.3",
            ),
            h264_lt_release_sample(
                "release_2k_h264_lt_testsrc_blur_10000k_to_8000k",
                "testsrc=size=2560x1440:rate=30:duration=30,boxblur=2:1",
            ),
            h264_lt_release_sample(
                "release_2k_h264_lt_gray_grid_10000k_to_8000k",
                "color=c=gray:s=2560x1440:r=30:d=30,drawgrid=w=160:h=160:t=2:c=white@0.2",
            ),
            h264_lt_release_sample(
                "release_2k_h264_lt_smpte_blur_10000k_to_8000k",
                "smptebars=size=2560x1440:rate=30:duration=30,boxblur=1:1",
            ),
            h264_lt_release_sample(
                "release_2k_h264_lt_soft_gradient_10000k_to_8000k",
                "nullsrc=s=2560x1440:r=30:d=30,geq=lum='128+30*sin(X/160)+20*sin(Y/120)':cb=128:cr=128,boxblur=1:1",
            ),
            h264_mt_release_sample(
                "release_2k_h264_mt_testsrc2_tmix_10000k_to_8000k",
                "testsrc2=size=2560x1440:rate=30:duration=30,tmix=frames=3:weights='1 1 1',unsharp=5:5:0.5:3:3:0.2",
            ),
            h264_mt_release_sample(
                "release_2k_h264_mt_scroll_grid_10000k_to_8000k",
                "testsrc2=size=2560x1440:rate=30:duration=30,scroll=horizontal=0.01:vertical=0.004,unsharp=5:5:0.4:3:3:0.2",
            ),
            h264_mt_release_sample(
                "release_2k_h264_mt_blended_motion_10000k_to_8000k",
                "testsrc2=size=2560x1440:rate=30:duration=30,tmix=frames=2:weights='1 1',unsharp=5:5:0.45:3:3:0.2",
            ),
            h264_mt_release_sample(
                "release_2k_h264_mt_fast_pattern_10000k_to_8000k",
                "testsrc2=size=2560x1440:rate=30:duration=30,scroll=horizontal=0.006:vertical=0.006,tmix=frames=2:weights='1 1'",
            ),
            h264_risk_release_sample(
                "release_2k_h264_risk_extreme_frequency_10000k_to_8000k",
                "nullsrc=s=2560x1440:r=30:d=30,geq=lum='255*mod(X+Y,2)':cb=128:cr=128",
            ),
            h264_risk_release_sample(
                "release_2k_h264_risk_temporal_noise_10000k_to_8000k",
                "nullsrc=s=2560x1440:r=30:d=30,geq=lum='255*mod(X+Y+N,2)':cb=128:cr=128",
            ),
            hevc_hd_release_sample(
                "release_2k_hevc_hd_testsrc2_unsharp_8000k_to_6500k",
                "testsrc2=size=2560x1440:rate=30:duration=30,unsharp=5:5:0.8:3:3:0.4",
            ),
            hevc_hd_release_sample(
                "release_2k_hevc_hd_grid_unsharp_8000k_to_6500k",
                "color=c=gray:s=2560x1440:r=30:d=30,drawgrid=w=96:h=96:t=2:c=white@0.35,unsharp=5:5:0.7:3:3:0.3",
            ),
            hevc_hd_release_sample(
                "release_2k_hevc_hd_frequency_8000k_to_6500k",
                "nullsrc=s=2560x1440:r=30:d=30,geq=lum='mod(X*11+Y*19,256)':cb=128:cr=128",
            ),
            hevc_hd_release_sample(
                "release_2k_hevc_hd_blended_detail_8000k_to_6500k",
                "testsrc2=size=2560x1440:rate=30:duration=30,tmix=frames=2:weights='1 1',unsharp=7:7:0.6:3:3:0.3",
            ),
            hevc_mix_release_sample(
                "release_2k_hevc_mix_motion_texture_8000k_to_6500k",
                "testsrc2=size=2560x1440:rate=30:duration=30,tmix=frames=3:weights='1 1 1',unsharp=5:5:0.5:3:3:0.2",
            ),
            hevc_mix_release_sample(
                "release_2k_hevc_mix_low_texture_8000k_to_6500k",
                "testsrc=size=2560x1440:rate=30:duration=30,boxblur=2:1",
            ),
            hevc_mix_release_sample(
                "release_2k_hevc_mix_soft_gradient_8000k_to_6500k",
                "nullsrc=s=2560x1440:r=30:d=30,geq=lum='128+30*sin(X/160)+20*sin(Y/120)':cb=128:cr=128,boxblur=1:1",
            ),
            hevc_mix_release_sample(
                "release_2k_hevc_mix_scroll_pattern_8000k_to_6500k",
                "testsrc2=size=2560x1440:rate=30:duration=30,scroll=horizontal=0.01:vertical=0.004,unsharp=5:5:0.4:3:3:0.2",
            ),
        ]
    }

    fn h264_hd_release_sample(
        case_name: &'static str,
        source_lavfi: &'static str,
    ) -> L3HighBitrateReleaseSampleCase<'static> {
        L3HighBitrateReleaseSampleCase {
            group: "H264-HD",
            failure_attribution: "pass",
            min_confidence: 0.950,
            case: h264_release_second_pass_case(
                "2k_release_h264_high_detail",
                case_name,
                source_lavfi,
            ),
        }
    }

    fn h264_lt_release_sample(
        case_name: &'static str,
        source_lavfi: &'static str,
    ) -> L3HighBitrateReleaseSampleCase<'static> {
        L3HighBitrateReleaseSampleCase {
            group: "H264-LT",
            failure_attribution: "pass",
            min_confidence: 0.950,
            case: h264_release_second_pass_case(
                "2k_release_h264_low_texture",
                case_name,
                source_lavfi,
            ),
        }
    }

    fn h264_mt_release_sample(
        case_name: &'static str,
        source_lavfi: &'static str,
    ) -> L3HighBitrateReleaseSampleCase<'static> {
        L3HighBitrateReleaseSampleCase {
            group: "H264-MT",
            failure_attribution: "pass",
            min_confidence: 0.950,
            case: h264_release_second_pass_case(
                "2k_release_h264_motion_texture",
                case_name,
                source_lavfi,
            ),
        }
    }

    fn h264_risk_release_sample(
        case_name: &'static str,
        source_lavfi: &'static str,
    ) -> L3HighBitrateReleaseSampleCase<'static> {
        let mut case =
            h264_release_second_pass_case("2k_release_h264_risk_boundary", case_name, source_lavfi);
        case.first_pass.region_selection_mode =
            Some(VideoVisualRegionSelectionMode::DistributedGrid);
        L3HighBitrateReleaseSampleCase {
            group: "H264-RISK",
            failure_attribution: "risk_boundary_expected",
            min_confidence: 0.950,
            case,
        }
    }

    fn hevc_hd_release_sample(
        case_name: &'static str,
        source_lavfi: &'static str,
    ) -> L3HighBitrateReleaseSampleCase<'static> {
        L3HighBitrateReleaseSampleCase {
            group: "HEVC-HD",
            failure_attribution: "pass",
            min_confidence: 0.970,
            case: hevc_release_second_pass_case(
                "2k_release_hevc_high_detail",
                case_name,
                source_lavfi,
            ),
        }
    }

    fn hevc_mix_release_sample(
        case_name: &'static str,
        source_lavfi: &'static str,
    ) -> L3HighBitrateReleaseSampleCase<'static> {
        L3HighBitrateReleaseSampleCase {
            group: "HEVC-MIX",
            failure_attribution: "pass",
            min_confidence: 0.970,
            case: hevc_release_second_pass_case("2k_release_hevc_mixed", case_name, source_lavfi),
        }
    }

    fn h264_release_second_pass_case(
        risk_profile: &'static str,
        case_name: &'static str,
        source_lavfi: &'static str,
    ) -> L3PlatformSecondPassCase<'static> {
        L3PlatformSecondPassCase {
            risk_profile,
            source_lavfi,
            first_pass: L3CommercialSamplingCase {
                case_name,
                width: 2560,
                height: 1440,
                bitrate: "10000k",
                maxrate: "12000k",
                bufsize: "24000k",
                codec: "libx264",
                codec_profile: Some("high"),
                crf: 21,
                extra_video_args: &[],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: None,
                video_filter: None,
            },
            second_pass_bitrate: "8000k",
            second_pass_maxrate: "10000k",
            second_pass_bufsize: "20000k",
            second_pass_crf: 23,
            expect_pass: true,
        }
    }

    fn hevc_release_second_pass_case(
        risk_profile: &'static str,
        case_name: &'static str,
        source_lavfi: &'static str,
    ) -> L3PlatformSecondPassCase<'static> {
        L3PlatformSecondPassCase {
            risk_profile,
            source_lavfi,
            first_pass: L3CommercialSamplingCase {
                case_name,
                width: 2560,
                height: 1440,
                bitrate: "8000k",
                maxrate: "10000k",
                bufsize: "20000k",
                codec: "libx265",
                codec_profile: None,
                crf: 20,
                extra_video_args: &["-tag:v", "hvc1"],
                max_regions: 96,
                sampled_frames: 16,
                region_selection_mode: None,
                video_filter: None,
            },
            second_pass_bitrate: "6500k",
            second_pass_maxrate: "8000k",
            second_pass_bufsize: "16000k",
            second_pass_crf: 24,
            expect_pass: true,
        }
    }

    fn classify_l3_high_bitrate_release_sample(
        min_confidence: f32,
        expected_attribution: &str,
        metrics: &L3PlatformSecondPassMetrics,
    ) -> &'static str {
        if expected_attribution == "risk_boundary_expected" && !metrics.self_check_passed {
            return "risk_boundary_expected";
        }
        if !metrics.self_check_passed {
            if metrics.self_check_status == "failed:self_check_failed" {
                return "self_check_failed";
            }
            if metrics.self_check_status == "failed:visual_extract_failed" {
                return "visual_extract_failed";
            }
            return "decode_or_transcode_failed";
        }
        if metrics.confidence < min_confidence {
            return "confidence_below_threshold";
        }
        "pass"
    }

    fn assert_group_count(
        outcomes: &[L3HighBitrateReleaseSampleOutcome],
        group: &str,
        expected: usize,
    ) {
        assert_eq!(
            outcomes
                .iter()
                .filter(|outcome| outcome.group == group)
                .count(),
            expected,
            "{group} sample count drifted"
        );
    }

    fn assert_sample_definition_count(
        samples: &[L3HighBitrateReleaseSampleCase<'_>],
        group: &str,
        expected: usize,
    ) {
        assert_eq!(
            samples
                .iter()
                .filter(|sample| sample.group == group)
                .count(),
            expected,
            "{group} sample definition count drifted"
        );
    }

    fn first_sample_per_group(
        samples: &[L3HighBitrateReleaseSampleCase<'static>],
    ) -> Vec<L3HighBitrateReleaseSampleCase<'static>> {
        let mut selected = Vec::new();
        for group in [
            "H264-HD",
            "H264-LT",
            "H264-MT",
            "H264-RISK",
            "HEVC-HD",
            "HEVC-MIX",
        ] {
            if let Some(sample) = samples.iter().find(|sample| sample.group == group) {
                selected.push(*sample);
            }
        }
        selected
    }

    fn assert_group_all_at_least(
        outcomes: &[L3HighBitrateReleaseSampleOutcome],
        group: &str,
        min_confidence: f32,
    ) {
        for outcome in outcomes.iter().filter(|outcome| outcome.group == group) {
            assert!(
                outcome.passed_threshold && outcome.confidence >= min_confidence,
                "{} in {group} must meet confidence {:.3}; confidence={:.3}, attribution={}, status={}",
                outcome.case_name,
                min_confidence,
                outcome.confidence,
                outcome.failure_attribution,
                outcome.self_check_status
            );
        }
    }

    fn assert_group_average_at_least(
        outcomes: &[L3HighBitrateReleaseSampleOutcome],
        group: &str,
        min_average: f32,
    ) {
        let average = group_average_confidence(outcomes, group);
        assert!(
            average >= min_average,
            "{group} average confidence {:.3} must meet {:.3}",
            average,
            min_average
        );
    }

    fn group_min_confidence(outcomes: &[L3HighBitrateReleaseSampleOutcome], group: &str) -> f32 {
        outcomes
            .iter()
            .filter(|outcome| outcome.group == group)
            .map(|outcome| outcome.confidence)
            .fold(f32::INFINITY, f32::min)
    }

    fn group_average_confidence(
        outcomes: &[L3HighBitrateReleaseSampleOutcome],
        group: &str,
    ) -> f32 {
        let values = outcomes
            .iter()
            .filter(|outcome| outcome.group == group)
            .map(|outcome| outcome.confidence)
            .collect::<Vec<_>>();
        assert!(!values.is_empty(), "{group} sample group must not be empty");
        values.iter().sum::<f32>() / values.len() as f32
    }

    async fn encode_and_decode_l3_y_plane_fixture_with_options(
        ffmpeg: &Path,
        temp_dir: &Path,
        written_y_plane: &Path,
        case_name: &str,
        width: usize,
        height: usize,
        frame_count: usize,
        crf: u8,
        preset: &str,
        video_filter: Option<&str>,
    ) -> Vec<watermark_core::VideoFramePlane> {
        encode_and_decode_l3_y_plane_fixture_with_profile(
            ffmpeg,
            temp_dir,
            written_y_plane,
            case_name,
            width,
            height,
            frame_count,
            L3VideoEncodeProfile::h264_crf(crf, preset, video_filter),
        )
        .await
    }

    async fn encode_and_decode_l3_y_plane_fixture_with_profile(
        ffmpeg: &Path,
        temp_dir: &Path,
        written_y_plane: &Path,
        case_name: &str,
        width: usize,
        height: usize,
        frame_count: usize,
        profile: L3VideoEncodeProfile<'_>,
    ) -> Vec<watermark_core::VideoFramePlane> {
        let protected_video = temp_dir.join(format!("l3-y-plane-{case_name}.mp4"));
        encode_l3_y_plane_fixture_to_video(
            ffmpeg,
            written_y_plane,
            &protected_video,
            width,
            height,
            frame_count,
            profile,
        )
        .await;

        decode_l3_video_fixture_to_y_planes(
            ffmpeg,
            temp_dir,
            &protected_video,
            case_name,
            width,
            height,
            frame_count,
        )
        .await
    }

    async fn encode_l3_y_plane_fixture_to_video(
        ffmpeg: &Path,
        written_y_plane: &Path,
        output_video: &Path,
        width: usize,
        height: usize,
        frame_count: usize,
        profile: L3VideoEncodeProfile<'_>,
    ) {
        let mut encode_args = vec![
            "-y".to_string(),
            "-f".to_string(),
            "rawvideo".to_string(),
            "-pix_fmt".to_string(),
            "gray".to_string(),
            "-s:v".to_string(),
            format!("{width}x{height}"),
            "-r".to_string(),
            profile.input_rate.to_string(),
            "-i".to_string(),
            written_y_plane.to_string_lossy().to_string(),
            "-frames:v".to_string(),
            frame_count.to_string(),
        ];
        if let Some(filter) = profile.video_filter {
            encode_args.push("-vf".to_string());
            encode_args.push(filter.to_string());
        }
        encode_args.extend([
            "-c:v".to_string(),
            profile.codec.to_string(),
            "-preset".to_string(),
            profile.preset.to_string(),
            "-crf".to_string(),
            profile.crf.to_string(),
        ]);
        if let Some(video_profile) = profile.profile {
            encode_args.push("-profile:v".to_string());
            encode_args.push(video_profile.to_string());
        }
        if let Some(level) = profile.level {
            encode_args.push("-level".to_string());
            encode_args.push(level.to_string());
        }
        if let Some(maxrate) = profile.maxrate {
            encode_args.push("-maxrate".to_string());
            encode_args.push(maxrate.to_string());
        }
        if let Some(bufsize) = profile.bufsize {
            encode_args.push("-bufsize".to_string());
            encode_args.push(bufsize.to_string());
        }
        if let Some(video_bitrate) = profile.video_bitrate {
            encode_args.push("-b:v".to_string());
            encode_args.push(video_bitrate.to_string());
        }
        if let Some(output_rate) = profile.output_rate {
            encode_args.push("-r".to_string());
            encode_args.push(output_rate.to_string());
        }
        if let Some(gop) = profile.gop {
            encode_args.push("-g".to_string());
            encode_args.push(gop.to_string());
        }
        if let Some(keyint_min) = profile.keyint_min {
            encode_args.push("-keyint_min".to_string());
            encode_args.push(keyint_min.to_string());
        }
        encode_args.extend(
            profile
                .extra_video_args
                .iter()
                .map(|arg| (*arg).to_string()),
        );
        encode_args.extend([
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
            "-movflags".to_string(),
            "+faststart".to_string(),
            output_video.to_string_lossy().to_string(),
        ]);
        let encode_args = encode_args.iter().map(String::as_str).collect::<Vec<_>>();

        run_ffmpeg_test_command(ffmpeg, &encode_args, "encode L3 lossy Y-plane fixture").await;
    }

    async fn transcode_l3_video_fixture(
        ffmpeg: &Path,
        input_video: &Path,
        output_video: &Path,
        profile: L3VideoEncodeProfile<'_>,
    ) {
        let mut transcode_args = vec![
            "-y".to_string(),
            "-i".to_string(),
            input_video.to_string_lossy().to_string(),
            "-c:v".to_string(),
            profile.codec.to_string(),
            "-preset".to_string(),
            profile.preset.to_string(),
            "-crf".to_string(),
            profile.crf.to_string(),
        ];
        if let Some(video_profile) = profile.profile {
            transcode_args.push("-profile:v".to_string());
            transcode_args.push(video_profile.to_string());
        }
        if let Some(level) = profile.level {
            transcode_args.push("-level".to_string());
            transcode_args.push(level.to_string());
        }
        if let Some(maxrate) = profile.maxrate {
            transcode_args.push("-maxrate".to_string());
            transcode_args.push(maxrate.to_string());
        }
        if let Some(bufsize) = profile.bufsize {
            transcode_args.push("-bufsize".to_string());
            transcode_args.push(bufsize.to_string());
        }
        if let Some(video_bitrate) = profile.video_bitrate {
            transcode_args.push("-b:v".to_string());
            transcode_args.push(video_bitrate.to_string());
        }
        if let Some(output_rate) = profile.output_rate {
            transcode_args.push("-r".to_string());
            transcode_args.push(output_rate.to_string());
        }
        if let Some(gop) = profile.gop {
            transcode_args.push("-g".to_string());
            transcode_args.push(gop.to_string());
        }
        if let Some(keyint_min) = profile.keyint_min {
            transcode_args.push("-keyint_min".to_string());
            transcode_args.push(keyint_min.to_string());
        }
        transcode_args.extend(
            profile
                .extra_video_args
                .iter()
                .map(|arg| (*arg).to_string()),
        );
        transcode_args.extend([
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
            "-movflags".to_string(),
            "+faststart".to_string(),
            output_video.to_string_lossy().to_string(),
        ]);
        let transcode_args = transcode_args
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();

        run_ffmpeg_test_command(ffmpeg, &transcode_args, "transcode L3 protected fixture").await;
    }

    async fn decode_l3_video_fixture_to_y_planes(
        ffmpeg: &Path,
        temp_dir: &Path,
        input_video: &Path,
        case_name: &str,
        width: usize,
        height: usize,
        frame_count: usize,
    ) -> Vec<watermark_core::VideoFramePlane> {
        let decoded_y_plane = temp_dir.join(format!("l3-y-plane-{case_name}.gray10le"));

        run_ffmpeg_test_command(
            ffmpeg,
            &[
                "-y",
                "-i",
                &input_video.to_string_lossy(),
                "-frames:v",
                &frame_count.to_string(),
                "-map",
                "0:v:0",
                "-f",
                "rawvideo",
                "-pix_fmt",
                "gray10le",
                &decoded_y_plane.to_string_lossy(),
            ],
            "decode L3 lossy Y-plane fixture",
        )
        .await;

        let decoded = std::fs::read(&decoded_y_plane).unwrap();
        assert_eq!(decoded.len(), width * height * frame_count * 2);
        let decoded_samples = decoded
            .chunks_exact(2)
            .map(|chunk| u16::from_le_bytes([chunk[0], chunk[1]]))
            .collect::<Vec<_>>();
        decoded_samples
            .chunks_exact(width * height)
            .take(frame_count)
            .map(|frame_samples| {
                video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
                    width: width as u32,
                    height: height as u32,
                    stride_samples: width,
                    samples: frame_samples,
                    bit_depth: VideoLumaBitDepth::Ten,
                    color_range: VideoLumaColorRange::Limited,
                    target_profile: VideoVisualProfile::LumaDctMidBandV1,
                })
                .unwrap()
            })
            .collect::<Vec<_>>()
    }

    async fn run_ffmpeg_test_command(ffmpeg: &Path, args: &[&str], label: &str) {
        let args: Vec<String> = args.iter().map(|value| (*value).to_string()).collect();
        let mut child = ffmpeg::spawn_ffmpeg(ffmpeg, &args)
            .await
            .unwrap_or_else(|error| panic!("{label}: spawn failed: {error}"));
        let status = child
            .child
            .wait()
            .await
            .unwrap_or_else(|error| panic!("{label}: wait failed: {error}"));
        assert!(status.success(), "{label}: ffmpeg exited with {status}");
    }

    async fn ffmpeg_encoder_available(ffmpeg: &Path, encoder: &str) -> bool {
        let mut command = tokio::process::Command::new(ffmpeg);
        crate::utils::process::hide_tokio_window(&mut command);
        let output = match command
            .args(["-hide_banner", "-encoders"])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
        {
            Ok(output) => output,
            Err(_) => return false,
        };
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        output.status.success() && (stdout.contains(encoder) || stderr.contains(encoder))
    }
}

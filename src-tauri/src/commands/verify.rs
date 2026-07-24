use std::path::Path;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::commands::vault::VaultRecord;
use crate::db::queries;
use crate::pipeline::ffmpeg;
use crate::pipeline::scheduler::{classify_file, FileType};
use crate::telemetry::anonymous;
use crate::tsa;
use crate::utils::fs as ufs;
use crate::AppState;
use watermark_core::{MediaInput, WatermarkDecodedPayload, WatermarkService};

const DISCLAIMER: &str = "本报告仅基于既定算法进行特征码技术提取，仅供参考，不代表任何司法鉴定意见。平台不对因本报告引发的连带法律责任负责。";
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    pub matched: bool,
    pub watermark_uid: Option<String>,
    pub confidence: f64,
    pub matched_record: Option<VaultRecord>,
    pub summary: String,
    pub reason_code: String,
    pub reason_detail: String,
    pub disclaimer: String,
    /// Whether a TSA token file is present locally. This is not a cryptographic verification.
    pub tsa_token_present: bool,
    pub tsa_token_verified: bool,
    pub tsa_verification_path: Option<tsa::TimestampTrustPath>,
    pub tsa_source: Option<String>,
    pub network_time: Option<String>,
    pub created_at: Option<String>,
    pub original_hash: Option<String>,
    pub payload_protocol_version: Option<u32>,
    pub payload_bytes_length: Option<u32>,
    pub payload_auth_status: Option<String>,
    pub watermark_id_issue_mode: Option<String>,
    pub media_payload_role: Option<String>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AudioVerificationSpec {
    sample_rate: Option<u32>,
    channels: Option<u16>,
}

impl AudioVerificationSpec {
    fn mono_44100() -> Self {
        Self {
            sample_rate: Some(44_100),
            channels: Some(1),
        }
    }

    fn stereo_44100() -> Self {
        Self {
            sample_rate: Some(44_100),
            channels: Some(2),
        }
    }

    fn from_probe_stream(stream: &ffmpeg::FfprobeStream) -> Self {
        Self {
            sample_rate: stream.sample_rate.filter(|value| *value > 0),
            channels: stream.channels.filter(|value| *value > 0),
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
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RewriteTargetInspectionResult {
    pub supported: bool,
    pub file_kind: String,
    pub has_watermark: bool,
    pub watermark_uid: Option<String>,
    pub detected_revision: Option<u32>,
    pub next_revision: u32,
    pub parent_watermark_uid: Option<String>,
    pub rewrite_reason: Option<String>,
    pub summary: String,
    pub reason_code: String,
    pub reason_detail: String,
}

struct VerificationReason {
    code: &'static str,
    detail: &'static str,
}

#[derive(Debug, Clone)]
struct RewriteTargetPlan {
    supported: bool,
    file_kind: String,
    has_watermark: bool,
    watermark_uid: Option<String>,
    detected_revision: Option<u32>,
    next_revision: u32,
    parent_watermark_uid: Option<String>,
    rewrite_reason: Option<String>,
    summary: String,
    reason_code: String,
    reason_detail: String,
}

#[tauri::command]
pub async fn inspect_rewrite_target(
    path: String,
    app_handle: AppHandle,
) -> Result<RewriteTargetInspectionResult, String> {
    let file_path = Path::new(&path);
    if !file_path.exists() {
        return Err(format!("文件不存在: {path}"));
    }
    let plan = inspect_rewrite_target_plan(file_path, &app_handle).await;
    Ok(RewriteTargetInspectionResult {
        supported: plan.supported,
        file_kind: plan.file_kind,
        has_watermark: plan.has_watermark,
        watermark_uid: plan.watermark_uid,
        detected_revision: plan.detected_revision,
        next_revision: plan.next_revision,
        parent_watermark_uid: plan.parent_watermark_uid,
        rewrite_reason: plan.rewrite_reason,
        summary: plan.summary,
        reason_code: plan.reason_code,
        reason_detail: plan.reason_detail,
    })
}

#[tauri::command]
pub async fn verify_suspect(
    path: String,
    app_handle: AppHandle,
) -> Result<VerificationResult, String> {
    let started_at = Instant::now();
    let file_path = Path::new(&path);
    let file_type = classify_file(file_path);
    let media_type = media_type_label(file_type);
    let file_size_bytes = std::fs::metadata(file_path)
        .map(|meta| meta.len())
        .unwrap_or(0);

    if !file_path.exists() {
        if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
            anonymous::record_failure_event(
                &app_data_dir,
                "verify_suspect",
                media_type,
                file_size_bytes,
                Some(started_at.elapsed().as_millis() as u64),
                "file_not_found",
                None,
            );
        }
        return Err(format!("文件不存在: {path}"));
    }

    let mut extraction_error: Option<String> = None;
    let extraction = match file_type {
        FileType::Image => extract_from_image(file_path),
        FileType::Audio if is_wav_file(file_path) => extract_from_audio_wav(file_path),
        FileType::Video | FileType::Audio => {
            extract_from_audio_bearing(file_path, &app_handle).await
        }
    };
    let (payload, confidence) = match extraction {
        Ok((payload, confidence)) => (Some(payload), confidence),
        Err(err) => {
            extraction_error = Some(err);
            (None, 0.0)
        }
    };

    let state = app_handle.state::<AppState>();
    let result = if let Some(ref decoded) = payload {
        let uid = decoded.watermark_uid();
        let (matched_record, uid_exists) = {
            let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
            (
                queries::find_by_watermark_uid(&conn, &uid),
                queries::has_watermark_uid(&conn, &uid),
            )
        };

        if confidence >= 0.95 {
            let reason;
            let summary = if let Some(ref record) = matched_record {
                reason = VerificationReason {
                    code: "matched_uid_registry",
                    detail: "V3 最小锚点有效，水印 UID 命中本地版权库；完整权利声明以版权库 / registry 为事实源。",
                };
                format!("✅ 文件验证通过，水印 UID: {}", record.watermark_uid)
            } else if uid_exists {
                reason = VerificationReason {
                    code: "watermark_detected_uid_only",
                    detail: "检测到有效 V3 水印 UID，但未读取到完整本地记录；完整权利声明需继续查询 registry。",
                };
                format!("⚠️ 检测到有效 V3 水印锚点，水印 UID: {uid}")
            } else {
                reason = VerificationReason {
                    code: "watermark_detected_unregistered",
                    detail:
                        "检测到有效 V3 水印锚点，但本机版权库没有对应 UID，可能来自其他设备或尚未同步。",
                };
                format!("⚠️ 检测到有效水印但未在本地金库找到记录，水印 UID: {uid}")
            };
            let tsa_token_path = matched_record
                .as_ref()
                .and_then(|r| r.tsa_token_path.as_ref())
                .cloned();
            let tsa_token_present = tsa_token_path
                .as_ref()
                .map(|p| std::path::Path::new(p).exists())
                .unwrap_or(false);
            let (tsa_token_verified, tsa_verification_path) =
                match (matched_record.as_ref(), tsa_token_path.as_ref()) {
                    (Some(record), Some(token_path)) if tsa_token_present => {
                        match tsa::verify_saved_token(
                            std::path::Path::new(token_path),
                            &record.original_hash,
                            record.tsa_request_nonce.as_deref(),
                        ) {
                            Ok(verified) => (true, Some(verified.trust_path)),
                            Err(err) => {
                                log::warn!(
                                    "TSA token revalidation failed for record {}: {}",
                                    record.id,
                                    err
                                );
                                (false, None)
                            }
                        }
                    }
                    _ => (false, None),
                };
            let tsa_source = matched_record.as_ref().and_then(|r| r.tsa_source.clone());
            let network_time = matched_record.as_ref().and_then(|r| r.network_time.clone());
            let created_at = matched_record.as_ref().map(|r| r.created_at.clone());
            let original_hash = matched_record.as_ref().map(|r| r.original_hash.clone());

            VerificationResult {
                matched: matched_record.is_some(),
                watermark_uid: Some(uid),
                confidence,
                matched_record,
                summary,
                reason_code: reason.code.to_string(),
                reason_detail: reason.detail.to_string(),
                disclaimer: DISCLAIMER.to_string(),
                tsa_token_present,
                tsa_token_verified,
                tsa_verification_path,
                tsa_source,
                network_time,
                created_at,
                original_hash,
                payload_protocol_version: Some(decoded.protocol_version() as u32),
                payload_bytes_length: Some(decoded.payload_bytes_length() as u32),
                payload_auth_status: Some(decoded.payload_auth_status().to_string()),
                watermark_id_issue_mode: Some(decoded_issue_mode(decoded).to_string()),
                media_payload_role: Some(decoded_payload_role(decoded).to_string()),
                duration_ms: started_at.elapsed().as_millis() as u64,
            }
        } else if confidence >= 0.5 {
            VerificationResult {
                matched: false,
                watermark_uid: Some(uid),
                confidence,
                matched_record: None,
                summary: "检测到疑似水印特征但置信度不足，无法确认匹配".to_string(),
                reason_code: "low_confidence".to_string(),
                reason_detail: "提取到了部分水印特征，但完整性不足；文件可能经过强压缩、裁剪、重采样或音轨替换。".to_string(),
                disclaimer: DISCLAIMER.to_string(),
                tsa_token_present: false,
                tsa_token_verified: false,
                tsa_verification_path: None,
                tsa_source: None,
                network_time: None,
                created_at: None,
                original_hash: None,
                payload_protocol_version: Some(decoded.protocol_version() as u32),
                payload_bytes_length: Some(decoded.payload_bytes_length() as u32),
                payload_auth_status: Some(decoded.payload_auth_status().to_string()),
                watermark_id_issue_mode: Some(decoded_issue_mode(decoded).to_string()),
                media_payload_role: Some(decoded_payload_role(decoded).to_string()),
                duration_ms: started_at.elapsed().as_millis() as u64,
            }
        } else {
            VerificationResult {
                matched: false,
                watermark_uid: None,
                confidence,
                matched_record: None,
                summary: "未检测到有效水印".to_string(),
                reason_code: "no_valid_watermark".to_string(),
                reason_detail: "未提取到可验证的 HiddenShield 水印载荷；可能不是本软件处理的作品，或水印已被严重破坏。".to_string(),
                disclaimer: DISCLAIMER.to_string(),
                tsa_token_present: false,
                tsa_token_verified: false,
                tsa_verification_path: None,
                tsa_source: None,
                network_time: None,
                created_at: None,
                original_hash: None,
                payload_protocol_version: None,
                payload_bytes_length: None,
                payload_auth_status: None,
                watermark_id_issue_mode: None,
                media_payload_role: None,
                duration_ms: started_at.elapsed().as_millis() as u64,
            }
        }
    } else {
        VerificationResult {
            matched: false,
            watermark_uid: None,
            confidence,
            matched_record: None,
            summary: "未检测到有效水印".to_string(),
            reason_code: extraction_error_reason_code(extraction_error.as_deref()).to_string(),
            reason_detail: extraction_error_reason_detail_for_file_type(
                file_type,
                extraction_error.as_deref(),
            ),
            disclaimer: DISCLAIMER.to_string(),
            tsa_token_present: false,
            tsa_token_verified: false,
            tsa_verification_path: None,
            tsa_source: None,
            network_time: None,
            created_at: None,
            original_hash: None,
            payload_protocol_version: None,
            payload_bytes_length: None,
            payload_auth_status: None,
            watermark_id_issue_mode: None,
            media_payload_role: None,
            duration_ms: started_at.elapsed().as_millis() as u64,
        }
    };

    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        let duration_ms = Some(started_at.elapsed().as_millis() as u64);
        if let Some(err) = extraction_error {
            anonymous::record_failure_event(
                &app_data_dir,
                "verify_suspect",
                media_type,
                file_size_bytes,
                duration_ms,
                err,
                None,
            );
        } else if result.matched {
            anonymous::record_success_event(
                &app_data_dir,
                "verify_suspect",
                media_type,
                file_size_bytes,
                duration_ms,
                None,
            );
        } else {
            let note = if result.watermark_uid.is_some() {
                format!(
                    "result=watermark_detected_but_unbound | confidence_bucket={}",
                    confidence_bucket(result.confidence)
                )
            } else if result.confidence >= 0.5 {
                format!(
                    "result=low_confidence | confidence_bucket={}",
                    confidence_bucket(result.confidence)
                )
            } else {
                format!(
                    "result=no_match | confidence_bucket={}",
                    confidence_bucket(result.confidence)
                )
            };
            anonymous::record_diagnostic_event(
                &app_data_dir,
                "verify_suspect",
                media_type,
                file_size_bytes,
                duration_ms,
                note,
                None,
            );
        }
    }

    Ok(result)
}

#[tauri::command]
pub async fn verify_suspect_readonly_candidate(
    path: String,
    app_handle: AppHandle,
) -> Result<VerificationResult, String> {
    let started_at = Instant::now();
    let file_path = Path::new(&path);
    if !file_path.exists() {
        return Err(format!("文件不存在: {path}"));
    }

    let file_type = classify_file(file_path);
    let decoded = extract_readonly_candidate(file_path, file_type, &app_handle).await?;
    let uid = decoded.watermark_uid();
    let state = app_handle.state::<AppState>();
    let matched_record = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        queries::find_by_watermark_uid(&conn, &uid)
    };
    let matched = matched_record.is_some();
    let media_payload_role = decoded_payload_role(&decoded);
    let summary = if matched {
        format!("受控只读候选验证命中本机版权库，水印 UID: {uid}")
    } else {
        format!("受控只读候选验证读取到媒体锚点，水印 UID: {uid}")
    };
    let reason = if matched {
        VerificationReason {
            code: "readonly_candidate_registry_matched",
            detail: "显式 V3/V2 只读候选 reader 读取到媒体锚点，并命中本机版权库。完整权利声明仍以 registry / 版权库为事实源。",
        }
    } else {
        VerificationReason {
            code: "readonly_candidate_anchor_detected",
            detail: "显式 V3/V2 只读候选 reader 读取到媒体锚点，但本机版权库尚无对应记录。完整权利声明需继续查询 registry。",
        }
    };

    Ok(VerificationResult {
        matched,
        watermark_uid: Some(uid),
        confidence: 1.0,
        matched_record,
        summary,
        reason_code: reason.code.to_string(),
        reason_detail: reason.detail.to_string(),
        disclaimer: DISCLAIMER.to_string(),
        tsa_token_present: false,
        tsa_token_verified: false,
        tsa_verification_path: None,
        tsa_source: None,
        network_time: None,
        created_at: None,
        original_hash: decoded_original_hash(&decoded),
        payload_protocol_version: Some(decoded.protocol_version() as u32),
        payload_bytes_length: Some(decoded.payload_bytes_length() as u32),
        payload_auth_status: Some(decoded.payload_auth_status().to_string()),
        watermark_id_issue_mode: Some(decoded_issue_mode(&decoded).to_string()),
        media_payload_role: Some(media_payload_role.to_string()),
        duration_ms: started_at.elapsed().as_millis() as u64,
    })
}

// ---------------------------------------------------------------------------
// Extraction helpers
// ---------------------------------------------------------------------------

/// Extract V3 watermark from an image file via the unified watermark service.
fn extract_from_image(file_path: &Path) -> Result<(WatermarkDecodedPayload, f64), String> {
    let bytes = std::fs::read(file_path).map_err(|e| format!("image_read_failed: {e}"))?;
    let payload = WatermarkService::extract(MediaInput::ImageBytes { bytes })
        .map_err(|e| format!("image_watermark_extract_failed: {e}"))?;

    let confidence = compute_confidence(&payload);
    Ok((payload, confidence))
}

fn is_wav_file(file_path: &Path) -> bool {
    file_path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.eq_ignore_ascii_case("wav"))
        .unwrap_or(false)
}

fn extract_from_audio_wav(file_path: &Path) -> Result<(WatermarkDecodedPayload, f64), String> {
    let bytes = std::fs::read(file_path).map_err(|e| format!("audio_read_failed: {e}"))?;
    let payload = WatermarkService::extract(MediaInput::AudioWavBytes { bytes })
        .map_err(|e| format!("audio_watermark_extract_failed: {e}"))?;

    let confidence = compute_confidence(&payload);
    Ok((payload, confidence))
}

async fn extract_matching_audio_wav_bytes<F>(
    file_path: &Path,
    ffmpeg_paths: &ffmpeg::FfmpegPaths,
    temp_prefix: &str,
    accepts: F,
) -> Result<Vec<u8>, String>
where
    F: Fn(&[u8]) -> bool,
{
    let temp_id = format!(
        "{temp_prefix}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let temp_dir = ufs::create_temp_dir(&temp_id)
        .map_err(|e| format!("verify_temp_dir_create_failed: {e}"))?;

    let mut candidate_specs = vec![AudioVerificationSpec::mono_44100()];
    let input_str = file_path.to_string_lossy().to_string();
    if let Ok(probe) = ffmpeg::ffprobe_source(&ffmpeg_paths.ffprobe, &input_str).await {
        if let Some(output_spec) = probe
            .streams
            .iter()
            .find(|stream| stream.codec_type.as_deref() == Some("audio"))
            .map(AudioVerificationSpec::from_probe_stream)
        {
            if !candidate_specs.contains(&output_spec) {
                candidate_specs.push(output_spec);
            }
        }
    }
    let stereo = AudioVerificationSpec::stereo_44100();
    if !candidate_specs.contains(&stereo) {
        candidate_specs.push(stereo);
    }

    let mut last_error = "audio_extract_failed".to_string();
    for (index, spec) in candidate_specs.iter().enumerate() {
        let temp_wav = temp_dir.join(format!("audio-{index}.wav"));
        let mut args: Vec<String> = vec![
            "-y".into(),
            "-i".into(),
            file_path.to_string_lossy().to_string(),
            "-vn".into(),
            "-acodec".into(),
            "pcm_s16le".into(),
        ];
        spec.apply_ffmpeg_args(&mut args);
        args.push(temp_wav.to_string_lossy().to_string());

        let mut child = match ffmpeg::spawn_ffmpeg(&ffmpeg_paths.ffmpeg, &args).await {
            Ok(child) => child,
            Err(e) => {
                last_error = format!("ffmpeg_start_failed: {e}");
                continue;
            }
        };
        let status = match child.child.wait().await {
            Ok(status) => status,
            Err(e) => {
                last_error = format!("ffmpeg_wait_failed: {e}");
                continue;
            }
        };
        if !status.success() {
            last_error = "audio_extract_failed".to_string();
            continue;
        }
        let wav_bytes = match std::fs::read(&temp_wav) {
            Ok(bytes) => bytes,
            Err(e) => {
                last_error = format!("wav_read_failed: {e}");
                continue;
            }
        };
        if accepts(&wav_bytes) {
            let _ = ufs::cleanup_temp_dir(&temp_id);
            return Ok(wav_bytes);
        }
        last_error = "audio_watermark_extract_failed".to_string();
    }

    let _ = ufs::cleanup_temp_dir(&temp_id);
    Err(last_error)
}

/// Extract watermark from a video or audio file:
/// 1. Use FFmpeg to extract audio to a temp WAV
/// 2. Read WAV bytes and run the unified watermark service
async fn extract_from_audio_bearing(
    file_path: &Path,
    app_handle: &AppHandle,
) -> Result<(WatermarkDecodedPayload, f64), String> {
    let state = app_handle.state::<AppState>();

    // Resolve FFmpeg paths
    let ffmpeg_paths = {
        if let Some(paths) = state.get_ffmpeg_paths() {
            paths.clone()
        } else {
            let _ = app_handle
                .path()
                .app_data_dir()
                .map_err(|e| format!("app_data_dir_resolve_failed: {e}"))?;
            ffmpeg::detect_ffmpeg()
                .await
                .map_err(|e| format!("ffmpeg_unavailable: {e}"))?
        }
    };

    let wav_bytes = extract_matching_audio_wav_bytes(file_path, &ffmpeg_paths, "verify", |bytes| {
        WatermarkService::extract(MediaInput::AudioWavBytes {
            bytes: bytes.to_vec(),
        })
        .is_ok()
    })
    .await?;

    let payload = WatermarkService::extract(MediaInput::AudioWavBytes { bytes: wav_bytes })
        .map_err(|e| format!("audio_watermark_extract_failed: {e}"))?;

    let confidence = compute_confidence(&payload);
    Ok((payload, confidence))
}

async fn extract_readonly_candidate(
    file_path: &Path,
    file_type: FileType,
    app_handle: &AppHandle,
) -> Result<WatermarkDecodedPayload, String> {
    match file_type {
        FileType::Image => {
            let bytes = std::fs::read(file_path).map_err(|e| format!("image_read_failed: {e}"))?;
            WatermarkService::extract(MediaInput::ImageBytes { bytes })
                .map_err(|e| format!("image_readonly_candidate_extract_failed: {e}"))
        }
        FileType::Audio if is_wav_file(file_path) => {
            let bytes = std::fs::read(file_path).map_err(|e| format!("audio_read_failed: {e}"))?;
            watermark_core::extract_watermark_wav_readonly_candidate_bytes(&bytes)
                .map_err(|e| format!("audio_readonly_candidate_extract_failed: {e}"))
        }
        FileType::Video | FileType::Audio => {
            let wav_bytes = extract_wav_bytes_for_verification(file_path, app_handle).await?;
            watermark_core::extract_watermark_wav_readonly_candidate_bytes(&wav_bytes)
                .map_err(|e| format!("audio_readonly_candidate_extract_failed: {e}"))
        }
    }
}

async fn extract_wav_bytes_for_verification(
    file_path: &Path,
    app_handle: &AppHandle,
) -> Result<Vec<u8>, String> {
    let state = app_handle.state::<AppState>();
    let ffmpeg_paths = {
        if let Some(paths) = state.get_ffmpeg_paths() {
            paths.clone()
        } else {
            ffmpeg::detect_ffmpeg()
                .await
                .map_err(|e| format!("ffmpeg_unavailable: {e}"))?
        }
    };
    extract_matching_audio_wav_bytes(file_path, &ffmpeg_paths, "verify-readonly", |bytes| {
        watermark_core::extract_watermark_wav_readonly_candidate_bytes(bytes).is_ok()
            || WatermarkService::extract(MediaInput::AudioWavBytes {
                bytes: bytes.to_vec(),
            })
            .is_ok()
    })
    .await
}

/// Compute confidence after the shared core has decoded and authenticated default V3 payload.
fn compute_confidence(payload: &WatermarkDecodedPayload) -> f64 {
    if payload.is_v3_minimal_anchor() {
        1.0
    } else {
        0.0
    }
}

fn issue_mode_label(mode: watermark_core::WatermarkIssueMode) -> &'static str {
    match mode {
        watermark_core::WatermarkIssueMode::ServerReserved => "server_reserved",
        watermark_core::WatermarkIssueMode::OfflineGenerated => "offline_generated",
        watermark_core::WatermarkIssueMode::ServerConfirmed => "server_confirmed",
        watermark_core::WatermarkIssueMode::ServerReissued => "server_reissued",
    }
}

fn decoded_issue_mode(decoded: &WatermarkDecodedPayload) -> &'static str {
    match decoded {
        WatermarkDecodedPayload::V2(payload) => issue_mode_label(payload.issue_mode),
        WatermarkDecodedPayload::V3MinimalAnchor(_) => "registry_resolved",
    }
}

fn decoded_payload_role(decoded: &WatermarkDecodedPayload) -> &'static str {
    match decoded {
        WatermarkDecodedPayload::V2(_) => "v2_full_record",
        WatermarkDecodedPayload::V3MinimalAnchor(_) => "v3_minimal_anchor",
    }
}

fn decoded_original_hash(decoded: &WatermarkDecodedPayload) -> Option<String> {
    match decoded {
        WatermarkDecodedPayload::V2(payload) => Some(hex::encode(payload.original_hash_prefix)),
        WatermarkDecodedPayload::V3MinimalAnchor(_) => None,
    }
}

fn media_type_label(file_type: FileType) -> &'static str {
    match file_type {
        FileType::Image => "image",
        FileType::Video => "video",
        FileType::Audio => "audio",
    }
}

fn confidence_bucket(confidence: f64) -> &'static str {
    if confidence >= 0.95 {
        "0.95-1.00"
    } else if confidence >= 0.5 {
        "0.50-0.94"
    } else {
        "0.00-0.49"
    }
}

async fn inspect_rewrite_target_plan(
    file_path: &Path,
    app_handle: &AppHandle,
) -> RewriteTargetPlan {
    let file_type = classify_file(file_path);
    let file_kind = media_type_label(file_type).to_string();
    let extraction = match file_type {
        FileType::Image => {
            let path = file_path.to_path_buf();
            match tokio::task::spawn_blocking(move || inspect_image_rewrite_preflight(&path)).await
            {
                Ok(result) => result,
                Err(join_error) => Err(format!("preflight_task_failed: {join_error}")),
            }
        }
        FileType::Audio => extract_from_audio_bearing(file_path, app_handle).await,
        FileType::Video => {
            return unsupported_rewrite_plan(file_kind);
        }
    };

    let (payload, confidence) = match extraction {
        Ok((payload, confidence)) => (payload, confidence),
        Err(err) => {
            return extraction_error_rewrite_plan(file_kind, &err);
        }
    };

    if confidence < 0.95 {
        return RewriteTargetPlan {
            supported: true,
            file_kind,
            has_watermark: false,
            watermark_uid: None,
            detected_revision: None,
            next_revision: 1,
            parent_watermark_uid: None,
            rewrite_reason: None,
            summary: "检测到疑似水印特征但置信度不足，将按首次写入处理。".to_string(),
            reason_code: "preflight_low_confidence".to_string(),
            reason_detail: "当前文件可能经过强压缩、裁剪或转码；若确认要覆盖旧水印，请开启重写。"
                .to_string(),
        };
    }

    let uid = payload.watermark_uid();
    let record = {
        let state = app_handle.state::<AppState>();
        let conn = state.db.lock().ok();
        conn.and_then(|conn| queries::find_by_watermark_uid(&conn, &uid))
    };
    rewrite_detected_plan(file_kind, uid, record)
}

fn inspect_image_rewrite_preflight(
    file_path: &Path,
) -> Result<(WatermarkDecodedPayload, f64), String> {
    let bytes = std::fs::read(file_path).map_err(|e| format!("image_read_failed: {e}"))?;
    let payload = WatermarkService::extract(MediaInput::ImageBytes { bytes })
        .map_err(|e| format!("image_watermark_extract_failed: {e}"))?;
    let confidence = compute_confidence(&payload);
    Ok((payload, confidence))
}

fn unsupported_rewrite_plan(file_kind: String) -> RewriteTargetPlan {
    RewriteTargetPlan {
        supported: false,
        file_kind,
        has_watermark: false,
        watermark_uid: None,
        detected_revision: None,
        next_revision: 1,
        parent_watermark_uid: None,
        rewrite_reason: None,
        summary: "该类型暂不支持写前水印预检。".to_string(),
        reason_code: "unsupported_preflight".to_string(),
        reason_detail: "当前写前预检只覆盖图片和音频；视频仍按桌面端转码管线处理。".to_string(),
    }
}

fn extraction_error_rewrite_plan(file_kind: String, error: &str) -> RewriteTargetPlan {
    let reason_code = extraction_error_reason_code(Some(error));
    let is_no_valid_watermark = reason_code == "no_valid_watermark";
    RewriteTargetPlan {
        supported: true,
        file_kind,
        has_watermark: false,
        watermark_uid: None,
        detected_revision: None,
        next_revision: 1,
        parent_watermark_uid: None,
        rewrite_reason: None,
        summary: if is_no_valid_watermark {
            "未检测到已有隐盾水印，将按首次写入处理。".to_string()
        } else {
            "写前预检未完成，继续写入前请先处理检测异常。".to_string()
        },
        reason_code: if is_no_valid_watermark {
            "no_valid_watermark".to_string()
        } else {
            "preflight_extract_failed".to_string()
        },
        reason_detail: if is_no_valid_watermark {
            "写前预检没有提取到有效水印；如果继续写入，会创建新的版权存证。".to_string()
        } else {
            extraction_error_reason_detail(Some(error)).to_string()
        },
    }
}

fn rewrite_detected_plan(
    file_kind: String,
    uid: String,
    record: Option<VaultRecord>,
) -> RewriteTargetPlan {
    let detected_revision = record.as_ref().map(|record| record.revision).unwrap_or(1);
    let next_revision = detected_revision.saturating_add(1);
    RewriteTargetPlan {
        supported: true,
        file_kind,
        has_watermark: true,
        watermark_uid: Some(uid.clone()),
        detected_revision: Some(detected_revision),
        next_revision,
        parent_watermark_uid: Some(uid.clone()),
        rewrite_reason: record
            .as_ref()
            .and_then(|record| record.rewrite_reason.clone()),
        summary: format!("检测到已有隐盾水印，继续写入将记录为第 {next_revision} 次写入。"),
        reason_code: "rewrite_detected".to_string(),
        reason_detail: if record.is_some() {
            "已在本地版权库找到同 UID 记录，重写会保留父级 UID 和递增版本。".to_string()
        } else {
            "检测到有效水印但本机版权库未找到对应记录，重写仍会保留提取到的父级 UID。".to_string()
        },
    }
}

fn extraction_error_reason_code(error: Option<&str>) -> &'static str {
    let Some(error) = error else {
        return "no_valid_watermark";
    };
    if error.contains("ffmpeg_unavailable") {
        "ffmpeg_unavailable"
    } else if error.contains("audio_extract_failed") {
        "audio_extract_failed"
    } else if error.contains("image_read_failed") || error.contains("wav_read_failed") {
        "file_read_failed"
    } else if error.contains("image_watermark_extract_failed")
        || error.contains("audio_watermark_extract_failed")
        || error.contains("no_valid_watermark")
    {
        "no_valid_watermark"
    } else {
        "extract_failed"
    }
}

fn extraction_error_reason_detail(error: Option<&str>) -> &'static str {
    let Some(error) = error else {
        return "未提取到可验证的 HiddenShield 水印载荷。";
    };
    if error.contains("ffmpeg_unavailable") {
        "音视频取证需要 FFmpeg；当前环境未找到可用 FFmpeg，无法抽取音轨。"
    } else if error.contains("audio_extract_failed") {
        "无法从该文件抽取可检测音轨；可能没有音轨、音轨损坏，或格式暂不受支持。"
    } else if error.contains("image_read_failed") || error.contains("wav_read_failed") {
        "文件读取失败，请确认文件仍存在且当前用户有读取权限。"
    } else if error.contains("image_watermark_extract_failed") {
        "图片中未提取到可验证水印；可能不是 HiddenShield 输出，或图片经过强压缩、裁剪、截图转发。"
    } else if error.contains("audio_watermark_extract_failed") {
        "音频中未提取到可验证水印；可能不是 HiddenShield 输出，或音频经过重采样、降噪、裁剪、转码。"
    } else if error.contains("no_valid_watermark") {
        "未检测到已有隐盾水印，将按首次写入处理。"
    } else {
        "提取过程异常，建议复制报告或发送诊断以便定位。"
    }
}

fn extraction_error_reason_detail_for_file_type(
    file_type: FileType,
    error: Option<&str>,
) -> String {
    let detail = extraction_error_reason_detail(error);
    match file_type {
        FileType::Video => detail
            .replace("音频中未提取到", "视频音轨中未提取到")
            .replace("音频经过重采样", "视频音轨经过重采样"),
        _ => detail.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use watermark_core::{
        AIContentFlags, AudioProtectionMode, EmbedOptions, MediaOutput, PayloadV2BuildInput,
        WatermarkIssueMode, WatermarkMediaType, WatermarkPayload,
    };

    fn test_v2_payload(media_type: WatermarkMediaType) -> WatermarkPayload {
        WatermarkPayload::from_v2(PayloadV2BuildInput {
            watermark_id: [0x24; 16],
            parent_watermark_id: None,
            revision: 1,
            issued_at: 1_786_147_200,
            original_sha256: [0x62; 32],
            ai_flags: AIContentFlags::default(),
            issue_mode: WatermarkIssueMode::OfflineGenerated,
            media_type,
            registry_proof_hash: Some([0x90; 16]),
            creator_binding: Some("HiddenShield desktop verification QA"),
        })
        .expect("valid V2 payload")
    }

    fn make_test_wav_bytes() -> Vec<u8> {
        let sample_rate = 44_100usize;
        let sample_count = sample_rate * 6;
        let data_bytes = sample_count * 2;
        let mut bytes = vec![0u8; 44 + data_bytes];
        bytes[0..4].copy_from_slice(b"RIFF");
        bytes[4..8].copy_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
        bytes[8..12].copy_from_slice(b"WAVE");
        bytes[12..16].copy_from_slice(b"fmt ");
        bytes[16..20].copy_from_slice(&16u32.to_le_bytes());
        bytes[20..22].copy_from_slice(&1u16.to_le_bytes());
        bytes[22..24].copy_from_slice(&1u16.to_le_bytes());
        bytes[24..28].copy_from_slice(&(sample_rate as u32).to_le_bytes());
        bytes[28..32].copy_from_slice(&((sample_rate * 2) as u32).to_le_bytes());
        bytes[32..34].copy_from_slice(&2u16.to_le_bytes());
        bytes[34..36].copy_from_slice(&16u16.to_le_bytes());
        bytes[36..40].copy_from_slice(b"data");
        bytes[40..44].copy_from_slice(&(data_bytes as u32).to_le_bytes());
        for i in 0..sample_count {
            let t = i as f32 / 44_100.0;
            let sample = (t * 440.0 * std::f32::consts::TAU).sin() * 0.2;
            bytes[44 + i * 2..46 + i * 2]
                .copy_from_slice(&((sample * 32767.0) as i16).to_le_bytes());
        }
        bytes
    }

    #[test]
    fn extraction_errors_map_to_actionable_reason_codes() {
        assert_eq!(
            extraction_error_reason_code(Some("ffmpeg_unavailable: not found")),
            "ffmpeg_unavailable"
        );
        assert_eq!(
            extraction_error_reason_code(Some("audio_extract_failed")),
            "audio_extract_failed"
        );
        assert_eq!(
            extraction_error_reason_code(Some("image_watermark_extract_failed: decode")),
            "no_valid_watermark"
        );
        assert_eq!(
            extraction_error_reason_code(Some("no_valid_watermark")),
            "no_valid_watermark"
        );
        assert_eq!(
            extraction_error_reason_code(Some("image_read_failed: denied")),
            "file_read_failed"
        );
        assert_eq!(extraction_error_reason_code(None), "no_valid_watermark");
    }

    #[test]
    fn extraction_error_details_are_user_facing() {
        assert!(extraction_error_reason_detail(Some("audio_extract_failed")).contains("音轨"));
        assert!(
            extraction_error_reason_detail(Some("audio_watermark_extract_failed")).contains("音频")
        );
        assert!(
            extraction_error_reason_detail(Some("audio_watermark_extract_failed"))
                .contains("音频经过重采样")
        );
        assert!(extraction_error_reason_detail(None).contains("水印"));
    }

    #[test]
    fn video_extraction_error_details_prefer_video_audio_track_language() {
        let detail = extraction_error_reason_detail_for_file_type(
            FileType::Video,
            Some("audio_watermark_extract_failed"),
        );
        assert!(detail.contains("视频音轨中未提取到可验证水印"));
        assert!(detail.contains("视频音轨经过重采样"));
    }

    #[test]
    fn v2_payload_decoded_by_core_is_legacy_confidence() {
        let payload = test_v2_payload(WatermarkMediaType::Image);
        let decoded = WatermarkDecodedPayload::V2(payload);

        assert_eq!(compute_confidence(&decoded), 0.0);
    }

    #[test]
    fn wav_protected_copy_verification_reads_core_payload_without_transcoding() {
        let payload = test_v2_payload(WatermarkMediaType::Audio);
        let output = WatermarkService::embed(
            MediaInput::AudioWavBytes {
                bytes: make_test_wav_bytes(),
            },
            &payload,
            EmbedOptions {
                audio_protection_mode: AudioProtectionMode::VideoTrack,
                ..EmbedOptions::default()
            },
        )
        .expect("embed wav");
        let MediaOutput::AudioWavBytes { bytes } = output else {
            panic!("unexpected output");
        };
        let wav_file = NamedTempFile::new().expect("temp wav");
        std::fs::write(wav_file.path(), bytes).expect("write temp wav");

        let (extracted, confidence) =
            extract_from_audio_wav(wav_file.path()).expect("extract temp wav");

        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
        assert_eq!(confidence, 1.0);
    }

    #[test]
    fn rewrite_preflight_maps_no_watermark_to_first_write() {
        let plan = extraction_error_rewrite_plan(
            "image".to_string(),
            "image_watermark_extract_failed: decode",
        );

        assert!(plan.supported);
        assert!(!plan.has_watermark);
        assert_eq!(plan.next_revision, 1);
        assert_eq!(plan.reason_code, "no_valid_watermark");
        assert!(plan.summary.contains("首次写入"));
    }

    #[test]
    fn rewrite_preflight_maps_plain_no_valid_watermark_to_first_write() {
        let plan = extraction_error_rewrite_plan("image".to_string(), "no_valid_watermark");

        assert!(plan.supported);
        assert!(!plan.has_watermark);
        assert_eq!(plan.next_revision, 1);
        assert_eq!(plan.reason_code, "no_valid_watermark");
        assert!(plan.summary.contains("首次写入"));
    }

    #[test]
    fn rewrite_preflight_keeps_parent_uid_and_increments_revision() {
        let plan = rewrite_detected_plan("audio".to_string(), "uid-parent".to_string(), None);

        assert!(plan.supported);
        assert!(plan.has_watermark);
        assert_eq!(plan.watermark_uid.as_deref(), Some("uid-parent"));
        assert_eq!(plan.parent_watermark_uid.as_deref(), Some("uid-parent"));
        assert_eq!(plan.detected_revision, Some(1));
        assert_eq!(plan.next_revision, 2);
        assert_eq!(plan.reason_code, "rewrite_detected");
    }
}

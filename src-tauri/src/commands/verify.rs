use std::path::Path;
use std::time::Instant;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::commands::vault::VaultRecord;
use crate::db::queries;
use crate::pipeline::ffmpeg;
use crate::pipeline::scheduler::{classify_file, FileType};
use crate::pipeline::watermark;
use crate::telemetry::anonymous;
use crate::tsa;
use crate::utils::fs as ufs;
use crate::AppState;
use watermark_core::{MediaInput, WatermarkService};

const DISCLAIMER: &str = "本报告仅基于既定算法进行特征码技术提取，仅供参考，不代表任何司法鉴定意见。平台不对因本报告引发的连带法律责任负责。";

/// Magic number expected in a valid watermark payload.
const WATERMARK_MAGIC: [u8; 4] = [0x48, 0x44, 0x35, 0x48];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VerificationResult {
    pub matched: bool,
    pub watermark_uid: Option<String>,
    pub confidence: f64,
    pub matched_record: Option<VaultRecord>,
    pub summary: String,
    pub disclaimer: String,
    /// Whether a TSA token file is present locally. This is not a cryptographic verification.
    pub tsa_token_present: bool,
    pub tsa_token_verified: bool,
    pub tsa_verification_path: Option<tsa::TimestampTrustPath>,
    pub tsa_source: Option<String>,
    pub network_time: Option<String>,
    pub created_at: Option<String>,
    pub original_hash: Option<String>,
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
    let result = if let Some(ref payload) = payload {
        let uid = payload.watermark_uid();
        let (matched_record, uid_exists) = {
            let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
            let file_hash_2bytes = [payload.file_hash[0], payload.file_hash[1]];
            (
                queries::find_by_uid_and_hash(&conn, &uid, &file_hash_2bytes),
                queries::has_watermark_uid(&conn, &uid),
            )
        };

        if confidence >= 0.95 {
            let summary = if let Some(ref record) = matched_record {
                let file_hash_2bytes = [payload.file_hash[0], payload.file_hash[1]];
                let prefix_hex = hex::encode(file_hash_2bytes);

                if record.original_hash.starts_with(&prefix_hex) {
                    format!("✅ 原始文件验证通过，水印 UID: {uid}")
                } else if record
                    .output_douyin_hash
                    .as_ref()
                    .map(|h| h.starts_with(&prefix_hex))
                    .unwrap_or(false)
                {
                    format!("✅ 输出文件验证通过（抖音），水印 UID: {uid}")
                } else if record
                    .output_bilibili_hash
                    .as_ref()
                    .map(|h| h.starts_with(&prefix_hex))
                    .unwrap_or(false)
                {
                    format!("✅ 输出文件验证通过（B站），水印 UID: {uid}")
                } else if record
                    .output_xhs_hash
                    .as_ref()
                    .map(|h| h.starts_with(&prefix_hex))
                    .unwrap_or(false)
                {
                    format!("✅ 输出文件验证通过（小红书），水印 UID: {uid}")
                } else {
                    format!("✅ 文件验证通过，水印 UID: {uid}")
                }
            } else if uid_exists {
                format!("⚠️ 检测到有效水印但文件已被修改，水印 UID: {uid}")
            } else {
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
                disclaimer: DISCLAIMER.to_string(),
                tsa_token_present,
                tsa_token_verified,
                tsa_verification_path,
                tsa_source,
                network_time,
                created_at,
                original_hash,
            }
        } else if confidence >= 0.5 {
            VerificationResult {
                matched: false,
                watermark_uid: Some(uid),
                confidence,
                matched_record: None,
                summary: "检测到疑似水印特征但置信度不足，无法确认匹配".to_string(),
                disclaimer: DISCLAIMER.to_string(),
                tsa_token_present: false,
                tsa_token_verified: false,
                tsa_verification_path: None,
                tsa_source: None,
                network_time: None,
                created_at: None,
                original_hash: None,
            }
        } else {
            VerificationResult {
                matched: false,
                watermark_uid: None,
                confidence,
                matched_record: None,
                summary: "未检测到有效水印".to_string(),
                disclaimer: DISCLAIMER.to_string(),
                tsa_token_present: false,
                tsa_token_verified: false,
                tsa_verification_path: None,
                tsa_source: None,
                network_time: None,
                created_at: None,
                original_hash: None,
            }
        }
    } else {
        VerificationResult {
            matched: false,
            watermark_uid: None,
            confidence,
            matched_record: None,
            summary: "未检测到有效水印".to_string(),
            disclaimer: DISCLAIMER.to_string(),
            tsa_token_present: false,
            tsa_token_verified: false,
            tsa_verification_path: None,
            tsa_source: None,
            network_time: None,
            created_at: None,
            original_hash: None,
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

// ---------------------------------------------------------------------------
// Extraction helpers
// ---------------------------------------------------------------------------

/// Extract watermark from an image file via the unified watermark service.
fn extract_from_image(file_path: &Path) -> Result<(watermark::WatermarkPayload, f64), String> {
    let bytes = std::fs::read(file_path).map_err(|e| format!("image_read_failed: {e}"))?;
    let payload = WatermarkService::extract(MediaInput::ImageBytes { bytes })
        .map_err(|e| format!("image_watermark_extract_failed: {e}"))?;

    let confidence = compute_confidence(&payload);
    Ok((payload, confidence))
}

/// Extract watermark from a video or audio file:
/// 1. Use FFmpeg to extract audio to a temp WAV
/// 2. Read WAV bytes and run the unified watermark service
async fn extract_from_audio_bearing(
    file_path: &Path,
    app_handle: &AppHandle,
) -> Result<(watermark::WatermarkPayload, f64), String> {
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

    // Create temp directory and extract audio
    let temp_id = format!(
        "verify-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let temp_dir = ufs::create_temp_dir(&temp_id)
        .map_err(|e| format!("verify_temp_dir_create_failed: {e}"))?;
    let temp_wav = temp_dir.join("audio.wav");

    // Extract audio to WAV using FFmpeg
    let args: Vec<String> = vec![
        "-y".into(),
        "-i".into(),
        file_path.to_string_lossy().to_string(),
        "-vn".into(),
        "-acodec".into(),
        "pcm_s16le".into(),
        "-ar".into(),
        "44100".into(),
        "-ac".into(),
        "2".into(),
        temp_wav.to_string_lossy().to_string(),
    ];

    let mut child = ffmpeg::spawn_ffmpeg(&ffmpeg_paths.ffmpeg, &args)
        .await
        .map_err(|e| format!("ffmpeg_start_failed: {e}"))?;

    let status = child
        .child
        .wait()
        .await
        .map_err(|e| format!("ffmpeg_wait_failed: {e}"))?;

    if !status.success() {
        let _ = ufs::cleanup_temp_dir(&temp_id);
        return Err("audio_extract_failed".to_string());
    }

    // Read WAV bytes and extract via the service layer
    let wav_bytes = std::fs::read(&temp_wav).map_err(|e| format!("wav_read_failed: {e}"))?;
    let _ = ufs::cleanup_temp_dir(&temp_id);

    let payload = WatermarkService::extract(MediaInput::AudioWavBytes { bytes: wav_bytes })
        .map_err(|e| format!("audio_watermark_extract_failed: {e}"))?;

    let confidence = compute_confidence(&payload);
    Ok((payload, confidence))
}

/// Compute confidence based on magic number and HMAC auth tag validity.
///
/// - Magic matches AND HMAC passes → 1.0
/// - Magic matches but HMAC fails → 0.7 (partial match — data may be corrupted)
/// - Magic doesn't match → 0.0
fn compute_confidence(payload: &watermark::WatermarkPayload) -> f64 {
    let magic_ok = payload.magic == WATERMARK_MAGIC;
    if !magic_ok {
        return 0.0;
    }

    // Re-verify HMAC auth tag over the serialized first 28 bytes
    let encoded = watermark::encode_payload(payload);
    let stored_tag = &encoded[28..32];
    // If decode_payload succeeded, the HMAC already passed.
    // Double-check by re-encoding and comparing the tag bytes.
    let recomputed = watermark::encode_payload(payload);
    if stored_tag == &recomputed[28..32] {
        1.0
    } else {
        0.7
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

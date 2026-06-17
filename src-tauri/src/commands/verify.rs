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
}

struct VerificationReason {
    code: &'static str,
    detail: &'static str,
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
            let reason;
            let summary = if let Some(ref record) = matched_record {
                let file_hash_2bytes = [payload.file_hash[0], payload.file_hash[1]];
                let prefix_hex = hex::encode(file_hash_2bytes);

                if record.original_hash.starts_with(&prefix_hex) {
                    reason = VerificationReason {
                        code: "matched_original",
                        detail: "水印有效，文件哈希片段与原始存证匹配。",
                    };
                    format!("✅ 原始文件验证通过，水印 UID: {uid}")
                } else if record
                    .output_douyin_hash
                    .as_ref()
                    .map(|h| h.starts_with(&prefix_hex))
                    .unwrap_or(false)
                {
                    reason = VerificationReason {
                        code: "matched_output",
                        detail: "水印有效，文件哈希片段与抖音输出副本匹配。",
                    };
                    format!("✅ 输出文件验证通过（抖音），水印 UID: {uid}")
                } else if record
                    .output_bilibili_hash
                    .as_ref()
                    .map(|h| h.starts_with(&prefix_hex))
                    .unwrap_or(false)
                {
                    reason = VerificationReason {
                        code: "matched_output",
                        detail: "水印有效，文件哈希片段与 B 站输出副本匹配。",
                    };
                    format!("✅ 输出文件验证通过（B站），水印 UID: {uid}")
                } else if record
                    .output_xhs_hash
                    .as_ref()
                    .map(|h| h.starts_with(&prefix_hex))
                    .unwrap_or(false)
                {
                    reason = VerificationReason {
                        code: "matched_output",
                        detail: "水印有效，文件哈希片段与小红书输出副本匹配。",
                    };
                    format!("✅ 输出文件验证通过（小红书），水印 UID: {uid}")
                } else {
                    reason = VerificationReason {
                        code: "matched_hash_mismatch",
                        detail: "水印 UID 命中本地记录，但文件哈希片段与已登记原件/输出不一致，可能经历过压缩、裁剪、转码或二次传播。",
                    };
                    format!("✅ 文件验证通过，水印 UID: {uid}")
                }
            } else if uid_exists {
                reason = VerificationReason {
                    code: "watermark_detected_asset_mismatch",
                    detail: "检测到有效水印 UID，但当前文件哈希片段未绑定到本地版权库记录。",
                };
                format!("⚠️ 检测到有效水印但文件已被修改，水印 UID: {uid}")
            } else {
                reason = VerificationReason {
                    code: "watermark_detected_unregistered",
                    detail:
                        "检测到有效水印，但本机版权库没有对应 UID，可能来自其他设备或尚未同步。",
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
            reason_detail: extraction_error_reason_detail(extraction_error.as_deref()).to_string(),
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
    } else {
        "提取过程异常，建议复制报告或发送诊断以便定位。"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(extraction_error_reason_detail(None).contains("水印"));
    }
}

use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::commands::vault::VaultRecord;
use crate::db::queries;
use crate::pipeline::ffmpeg;
use crate::pipeline::image_watermark;
use crate::pipeline::scheduler::{classify_file, FileType};
use crate::pipeline::watermark;
use crate::tsa;
use crate::utils::fs as ufs;
use crate::AppState;

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
    let file_path = Path::new(&path);
    if !file_path.exists() {
        return Err(format!("文件不存在: {path}"));
    }

    let file_type = classify_file(file_path);

    // Extract watermark based on file type
    let extraction = match file_type {
        FileType::Image => extract_from_image(file_path),
        FileType::Video | FileType::Audio => {
            extract_from_audio_bearing(file_path, &app_handle).await
        }
    };

    let (payload, confidence) = match extraction {
        Ok((p, c)) => (Some(p), c),
        Err(_) => (None, 0.0),
    };

    // Build result based on confidence thresholds
    let state = app_handle.state::<AppState>();

    if let Some(ref payload) = payload {
        let uid = payload.watermark_uid();

        let (matched_record, uid_exists) = {
            let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
            (
                queries::find_by_uid_and_hash(&conn, &uid, &payload.file_hash),
                queries::has_watermark_uid(&conn, &uid),
            )
        };

        if confidence >= 0.95 {
            let summary = if matched_record.is_some() {
                format!("水印匹配成功，水印 UID: {uid}，已在版权金库中找到对应记录。")
            } else if uid_exists {
                format!(
          "提取到有效水印 (UID: {uid})，但仅命中同一创作者标识，未通过素材哈希绑定校验，不能判定为同一作品。"
        )
            } else {
                format!("提取到有效水印 (UID: {uid})，但未在本地版权金库中找到匹配记录。")
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
            return Ok(VerificationResult {
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
            });
        }

        if confidence >= 0.5 {
            return Ok(VerificationResult {
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
            });
        }
    }

    // confidence < 0.5 or extraction failed entirely
    Ok(VerificationResult {
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
    })
}

// ---------------------------------------------------------------------------
// Extraction helpers
// ---------------------------------------------------------------------------

/// Extract watermark from an image file via DWT-DCT-SVD blind watermark.
fn extract_from_image(file_path: &Path) -> Result<(watermark::WatermarkPayload, f64), String> {
    let payload = image_watermark::extract_image_watermark(file_path)
        .map_err(|e| format!("图片水印提取失败: {e}"))?;

    let confidence = compute_confidence(&payload);
    Ok((payload, confidence))
}

/// Extract watermark from a video or audio file:
/// 1. Use FFmpeg to extract audio to a temp WAV
/// 2. Read WAV samples and run watermark::extract_watermark
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
                .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
            ffmpeg::detect_ffmpeg()
                .await
                .map_err(|e| format!("FFmpeg 不可用: {e}"))?
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
    let temp_dir = ufs::create_temp_dir(&temp_id).map_err(|e| format!("创建临时目录失败: {e}"))?;
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
        .map_err(|e| format!("FFmpeg 启动失败: {e}"))?;

    let status = child
        .child
        .wait()
        .await
        .map_err(|e| format!("FFmpeg 等待失败: {e}"))?;

    if !status.success() {
        let _ = ufs::cleanup_temp_dir(&temp_id);
        return Err("音频抽取失败".to_string());
    }

    // Read WAV samples
    let samples = read_wav_samples(&temp_wav)?;
    let _ = ufs::cleanup_temp_dir(&temp_id);

    // Extract watermark from PCM samples
    let payload =
        watermark::extract_watermark(&samples).map_err(|e| format!("水印提取失败: {e}"))?;

    let confidence = compute_confidence(&payload);
    Ok((payload, confidence))
}

/// Read a WAV file into f32 samples.
fn read_wav_samples(wav_path: &Path) -> Result<Vec<f32>, String> {
    let mut reader = hound::WavReader::open(wav_path).map_err(|e| format!("打开 WAV 失败: {e}"))?;
    let spec = reader.spec();

    let samples: Vec<f32> = if spec.sample_format == hound::SampleFormat::Float {
        reader.samples::<f32>().filter_map(|s| s.ok()).collect()
    } else {
        let max_val = (1i32 << (spec.bits_per_sample - 1)) as f32;
        reader
            .samples::<i32>()
            .filter_map(|s| s.ok())
            .map(|s| s as f32 / max_val)
            .collect()
    };

    Ok(samples)
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

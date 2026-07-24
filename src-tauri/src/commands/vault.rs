use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::db::queries;
use crate::AppState;
use crate::{identity, sync::cloud};
use watermark_core::{
    EmbedOptions, ImageOutputFormat, MediaInput, MediaOutput, PayloadV2BuildInput,
    WatermarkIssueMode, WatermarkMediaType, WatermarkPayload, WatermarkService,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultRecord {
    pub id: u32,
    pub original_hash: String,
    pub file_name: String,
    pub created_at: String,
    pub duration_secs: f64,
    pub resolution: String,
    pub watermark_uid: String,
    pub creator_display_name: Option<String>,
    pub thumbnail_path: Option<String>,
    pub output_douyin: Option<String>,
    pub output_bilibili: Option<String>,
    pub output_xhs: Option<String>,
    pub is_hdr_source: bool,
    pub hw_encoder_used: Option<String>,
    pub process_time_ms: Option<u64>,
    pub tsa_token_path: Option<String>,
    pub network_time: Option<String>,
    pub tsa_source: Option<String>,
    pub tsa_request_nonce: Option<String>,
    // AI content identification fields
    pub is_ai_generated: bool,
    pub ai_training_permission: Option<String>,
    pub ai_generation_method: Option<String>,
    pub human_modification_level: Option<String>,
    pub authenticity_claim: Option<String>,
    pub custom_metadata: Option<String>,
    // Output file hashes for asset binding verification
    pub output_douyin_hash: Option<String>,
    pub output_bilibili_hash: Option<String>,
    pub output_xhs_hash: Option<String>,
    pub protected_copy_name: Option<String>,
    pub protected_copy_path: Option<String>,
    pub protected_copy_hash: Option<String>,
    pub output_strategy: String,
    pub work_source_declaration: String,
    pub training_permission_declaration: String,
    pub creation_method_declaration: String,
    pub human_edit_level_declaration: String,
    pub authenticity_claim_declaration: String,
    pub custom_rights_statement: Option<String>,
    // Rewrite lineage. Defaults to revision 1 for first-party original writes.
    pub parent_watermark_uid: Option<String>,
    pub revision: u32,
    pub rewrite_reason: Option<String>,
    // Completion-time verification status for generated protected copies.
    pub write_verification_status: Option<String>,
    pub write_verification_message: Option<String>,
    pub write_verification_at: Option<String>,
    // Payload V2 and backend registry metadata. These fields are synced and
    // reported, but never include original media or local file paths.
    pub payload_protocol_version: u32,
    pub payload_bytes_length: u32,
    pub watermark_id_issue_mode: String,
    pub watermark_id_registry_status: String,
    pub watermark_id_registry_receipt: Option<String>,
    pub payload_auth_status: String,
    // L2 video fingerprint notary. Contains only irreversible receipt and
    // bundle metadata; never stores original video paths or media bytes.
    pub video_notary_id: Option<String>,
    pub video_notary_at: Option<String>,
    pub video_notary_receipt_signature: Option<String>,
    pub video_notary_usage_ledger_id: Option<String>,
    pub video_fingerprint_root: Option<String>,
    pub video_bundle_sha256: Option<String>,
    pub video_bundle_bytes: Option<u64>,
    pub video_bundle_scene_count: Option<u32>,
    pub video_bundle_elapsed_ms: Option<u64>,
    pub video_frame_sample_policy: Option<String>,
    // L3 video visual watermark task receipt. Contains only audit metadata;
    // never stores signed download URLs, object storage refs, local paths, or media bytes.
    pub video_visual_task_id: Option<String>,
    pub video_visual_completed_at: Option<String>,
    pub video_visual_strategy_digest: Option<String>,
    pub video_visual_self_check_confidence: Option<f64>,
    pub video_visual_self_check_threshold: Option<f64>,
    pub video_visual_checked_frames: Option<u32>,
    pub video_visual_media_hash: Option<String>,
    pub video_visual_receipt_hash: Option<String>,
    pub video_visual_output_bytes: Option<u64>,
    pub video_visual_output_content_type: Option<String>,
}

#[tauri::command]
pub async fn list_vault_records(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<Vec<VaultRecord>, String> {
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    let mut records = queries::list_records(&conn);
    if records.iter().any(|record| {
        record
            .creator_display_name
            .as_deref()
            .unwrap_or_default()
            .trim()
            .is_empty()
    }) {
        let creator_display_name = app_handle
            .path()
            .app_data_dir()
            .ok()
            .and_then(|app_data_dir| creator_display_name_for_display(&app_data_dir));
        if let Some(creator_display_name) = creator_display_name {
            for record in &mut records {
                if record
                    .creator_display_name
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                {
                    record.creator_display_name = Some(creator_display_name.clone());
                }
            }
        }
    }
    Ok(records)
}

pub(crate) fn creator_display_name_for_display(app_data_dir: &Path) -> Option<String> {
    identity::load_identity(app_data_dir)
        .map(|value| value.creator_display_name)
        .or_else(|| {
            cloud::load_desktop_cloud_sync_profile(app_data_dir)
                .map(|profile| profile.creator_display_name)
        })
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

/// Check which file paths still exist on disk.
/// Returns a list of paths that are missing/offline.
#[tauri::command]
pub async fn check_files_exist(paths: Vec<String>) -> Result<Vec<String>, String> {
    let missing: Vec<String> = paths
        .into_iter()
        .filter(|p| !Path::new(p).exists())
        .collect();
    Ok(missing)
}

#[tauri::command]
pub async fn supplement_vault_trusted_time(
    record_id: u32,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<VaultRecord, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法读取应用数据目录: {e}"))?;
    if !crate::telemetry::is_acknowledged(&app_data_dir)
        || !crate::telemetry::is_network_enabled(&app_data_dir)
    {
        return Err("请先在设置中确认隐私选项并允许联网，再补充可信时间。".to_string());
    }

    let record = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        queries::find_by_id(&conn, record_id).ok_or_else(|| "版权记录不存在。".to_string())?
    };
    let tsa_dir = app_data_dir.join("tsa_tokens");
    let attestation =
        crate::tsa::request_attestation(&record.original_hash, &record.watermark_uid, &tsa_dir)
            .await;
    if attestation.tsa_token_path.is_none() && attestation.network_time.is_none() {
        return Err("未能连接第三方时间服务，请检查网络后重试。".to_string());
    }
    let source = attestation
        .tsa_source
        .as_deref()
        .or(attestation.network_time_source.as_deref());

    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    queries::update_timestamp_attestation(
        &conn,
        record_id,
        attestation.tsa_token_path.as_deref(),
        attestation.network_time.as_deref(),
        source,
        attestation.tsa_request_nonce.as_deref(),
    )
    .map_err(|e| format!("保存可信时间失败: {e}"))?;
    queries::find_by_id(&conn, record_id)
        .ok_or_else(|| "可信时间已保存，但记录读取失败。".to_string())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairWatermarkRecordRequest {
    pub record_id: u32,
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RepairWatermarkRecordResult {
    pub record_id: u32,
    pub previous_watermark_uid: String,
    pub replacement_watermark_uid: Option<String>,
    pub job_id: Option<String>,
    pub status: String,
    pub message: String,
    pub protected_copy_path: Option<String>,
    pub protected_copy_hash: Option<String>,
}

#[tauri::command]
pub async fn repair_watermark_record_reissue(
    input: RepairWatermarkRecordRequest,
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<RepairWatermarkRecordResult, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;
    let profile = cloud::load_desktop_cloud_sync_profile(&app_data_dir)
        .ok_or_else(|| "请先在设置中继续 HiddenShield 账户，再执行编号重新签发。".to_string())?;
    let record = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        queries::list_records(&conn)
            .into_iter()
            .find(|record| record.id == input.record_id)
            .ok_or_else(|| format!("未找到版权记录: {}", input.record_id))?
    };
    if record.video_notary_id.is_some() {
        return Err("视频指纹存证没有保护副本 payload，不能执行盲水印重签修复。".to_string());
    }

    let media_type = media_type_for_record(&record)?;
    let revision = record.revision.saturating_add(1).max(2);
    let reissue = cloud::CloudSyncClient::new(&profile.cloud_base_url)?.reissue_watermark_id(
        &profile.access_token,
        &cloud::WatermarkIdReissueRequest {
            workspace_id: profile.workspace_id.clone(),
            creator_profile_id: profile.creator_profile_id.clone(),
            previous_watermark_uid: record.watermark_uid.clone(),
            media_type: media_type.clone(),
            payload_protocol_version: 2,
            payload_bytes_length: watermark_core::PAYLOAD_BYTES as u32,
            parent_watermark_uid: Some(record.watermark_uid.clone()),
            revision,
            reason: input
                .reason
                .as_deref()
                .unwrap_or("historical_duplicate_watermark_uid_repair")
                .to_string(),
            original_hash: Some(prefixed_sha256_for_record(&record.original_hash)),
        },
    )?;

    let protected_copy_path = record
        .protected_copy_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let Some(protected_copy_path) = protected_copy_path else {
        mark_reissue_required(
            &state,
            record.id,
            Some(&reissue.replacement.registry_receipt),
        )?;
        return Ok(RepairWatermarkRecordResult {
            record_id: record.id,
            previous_watermark_uid: record.watermark_uid,
            replacement_watermark_uid: Some(reissue.replacement.watermark_uid),
            job_id: Some(reissue.job_id),
            status: "reissue_required".to_string(),
            message: "已创建重新签发任务，但本机没有保护副本路径。请重新选择原作品或保护副本后生成新的 V2 保护副本。".to_string(),
            protected_copy_path: None,
            protected_copy_hash: None,
        });
    };
    if !protected_copy_path.exists() {
        mark_reissue_required(
            &state,
            record.id,
            Some(&reissue.replacement.registry_receipt),
        )?;
        return Ok(RepairWatermarkRecordResult {
            record_id: record.id,
            previous_watermark_uid: record.watermark_uid,
            replacement_watermark_uid: Some(reissue.replacement.watermark_uid),
            job_id: Some(reissue.job_id),
            status: "reissue_required".to_string(),
            message: "已创建重新签发任务，但本机无法访问原保护副本。请重新选择原作品或保护副本后生成新的 V2 保护副本。".to_string(),
            protected_copy_path: Some(protected_copy_path.to_string_lossy().to_string()),
            protected_copy_hash: None,
        });
    }

    let repaired = repair_protected_copy_file(
        &record,
        &protected_copy_path,
        &reissue.replacement,
        &media_type,
        revision,
    )?;
    {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        queries::update_record_after_reissue_repair(
            &conn,
            record.id,
            &record.watermark_uid,
            &reissue.replacement.watermark_uid,
            &repaired.name,
            &repaired.path,
            &repaired.sha256,
            &reissue.replacement.watermark_id_issue_mode,
            &reissue.replacement.registry_status,
            &reissue.replacement.registry_receipt,
            reissue.replacement.payload_protocol_version,
            reissue.replacement.payload_bytes_length,
            revision,
        )
        .map_err(|e| format!("更新修复后的版权记录失败: {e}"))?;
    }

    Ok(RepairWatermarkRecordResult {
        record_id: record.id,
        previous_watermark_uid: record.watermark_uid,
        replacement_watermark_uid: Some(reissue.replacement.watermark_uid),
        job_id: Some(reissue.job_id),
        status: "repaired".to_string(),
        message: "已重新签发版权编号，并生成可回读验证的 V2 保护副本。".to_string(),
        protected_copy_path: Some(repaired.path),
        protected_copy_hash: Some(repaired.sha256),
    })
}

struct RepairedProtectedCopy {
    name: String,
    path: String,
    sha256: String,
}

fn repair_protected_copy_file(
    record: &VaultRecord,
    protected_copy_path: &Path,
    replacement: &cloud::WatermarkIdRegistryResponse,
    media_type: &str,
    revision: u32,
) -> Result<RepairedProtectedCopy, String> {
    let bytes = std::fs::read(protected_copy_path)
        .map_err(|e| format!("读取保护副本失败，无法修复 payload: {e}"))?;
    let payload = build_reissue_payload(record, replacement, media_type, revision)?;
    let output = match media_type {
        "image" => WatermarkService::embed(
            MediaInput::ImageBytes { bytes },
            &payload,
            EmbedOptions {
                image_output_format: ImageOutputFormat::Png,
                allow_rewrite: true,
                ..EmbedOptions::default()
            },
        )
        .map_err(|e| format!("重写图片保护副本失败: {e}"))?,
        "audio" => WatermarkService::embed(
            MediaInput::AudioWavBytes { bytes },
            &payload,
            EmbedOptions {
                allow_rewrite: true,
                ..EmbedOptions::default()
            },
        )
        .map_err(|e| format!("重写音频保护副本失败: {e}"))?,
        _ => return Err("当前仅支持图片和音频保护副本修复。".to_string()),
    };
    let (output_bytes, extension) = match output {
        MediaOutput::ImageBytes { bytes, .. } => (bytes, "png"),
        MediaOutput::AudioWavBytes { bytes } => (bytes, "wav"),
        _ => return Err("保护副本修复输出类型异常。".to_string()),
    };
    verify_repaired_payload(
        &output_bytes,
        media_type,
        &replacement.watermark_uid,
        Some(&record.watermark_uid),
    )?;
    let repaired_path = repaired_output_path(protected_copy_path, extension);
    std::fs::write(&repaired_path, &output_bytes)
        .map_err(|e| format!("保存修复后的保护副本失败: {e}"))?;
    let sha256 = crate::utils::hash::sha256_of_file(repaired_path.to_string_lossy().as_ref())
        .map_err(|e| format!("计算修复保护副本摘要失败: {e}"))?;
    Ok(RepairedProtectedCopy {
        name: repaired_path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("reissued_protected_copy.{extension}")),
        path: repaired_path.to_string_lossy().to_string(),
        sha256,
    })
}

fn verify_repaired_payload(
    bytes: &[u8],
    media_type: &str,
    expected_uid: &str,
    expected_parent_uid: Option<&str>,
) -> Result<(), String> {
    let input = match media_type {
        "image" => MediaInput::ImageBytes {
            bytes: bytes.to_vec(),
        },
        "audio" => MediaInput::AudioWavBytes {
            bytes: bytes.to_vec(),
        },
        _ => return Err("当前仅支持图片和音频保护副本修复。".to_string()),
    };
    let extracted =
        WatermarkService::extract(input).map_err(|e| format!("修复后回读验证失败: {e}"))?;
    if extracted.watermark_uid() != expected_uid {
        return Err(format!(
            "修复后回读版权编号不一致，期望 {expected_uid}，实际 {}。",
            extracted.watermark_uid()
        ));
    }
    if expected_parent_uid.is_some()
        && extracted.payload_bytes_length() != watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES
    {
        return Err(
            "修复后回读 payload 不是 V3 最小锚点，无法按默认算法确认迁移结果。".to_string(),
        );
    }
    Ok(())
}

fn build_reissue_payload(
    record: &VaultRecord,
    replacement: &cloud::WatermarkIdRegistryResponse,
    media_type: &str,
    revision: u32,
) -> Result<WatermarkPayload, String> {
    let watermark_id = parse_watermark_uid_to_id(&replacement.watermark_uid)?;
    let parent_watermark_id = parse_watermark_uid_to_id(&record.watermark_uid)?;
    let original_sha256 = parse_sha256_hex_32(&record.original_hash)?;
    let registry_proof_hash = parse_hex_16(&replacement.registry_proof_hash)?;
    let media_type = match media_type {
        "image" => WatermarkMediaType::Image,
        "audio" => WatermarkMediaType::Audio,
        _ => return Err("当前仅支持图片和音频保护副本修复。".to_string()),
    };
    WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id,
        parent_watermark_id: Some(parent_watermark_id),
        revision,
        issued_at: chrono::Utc::now().timestamp_millis() as u64,
        original_sha256,
        ai_flags: Default::default(),
        issue_mode: WatermarkIssueMode::ServerReissued,
        media_type,
        registry_proof_hash: Some(registry_proof_hash),
        creator_binding: record.creator_display_name.as_deref(),
    })
    .map_err(|e| format!("构造重签 V2 payload 失败: {e}"))
}

fn repaired_output_path(source: &Path, extension: &str) -> PathBuf {
    let stem = source
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "protected_copy".to_string());
    source.with_file_name(format!("{stem}_reissued.{extension}"))
}

fn media_type_for_record(record: &VaultRecord) -> Result<String, String> {
    let lower = record.file_name.to_lowercase();
    if lower.ends_with(".wav")
        || lower.ends_with(".mp3")
        || lower.ends_with(".flac")
        || lower.ends_with(".ogg")
        || lower.ends_with(".m4a")
        || lower.ends_with(".aac")
    {
        return Ok("audio".to_string());
    }
    if lower.ends_with(".png")
        || lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".webp")
        || lower.ends_with(".bmp")
        || lower.ends_with(".tif")
        || lower.ends_with(".tiff")
    {
        return Ok("image".to_string());
    }
    let protected = record
        .protected_copy_name
        .as_deref()
        .unwrap_or_default()
        .to_lowercase();
    if protected.ends_with(".wav") {
        Ok("audio".to_string())
    } else if protected.ends_with(".png") {
        Ok("image".to_string())
    } else {
        Err("当前仅支持图片和音频保护副本修复。".to_string())
    }
}

fn mark_reissue_required(
    state: &State<'_, AppState>,
    record_id: u32,
    registry_receipt: Option<&str>,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    queries::mark_record_reissue_required(&conn, record_id, registry_receipt)
        .map_err(|e| format!("标记待修复记录失败: {e}"))
}

fn prefixed_sha256_for_record(value: &str) -> String {
    if value.trim().starts_with("sha256:") {
        value.trim().to_string()
    } else {
        format!("sha256:{}", value.trim())
    }
}

fn parse_watermark_uid_to_id(uid: &str) -> Result<[u8; 16], String> {
    let compact = uid
        .trim()
        .strip_prefix("HS-")
        .unwrap_or(uid.trim())
        .replace('-', "");
    let bytes = hex::decode(compact).map_err(|e| format!("版权编号格式无效: {e}"))?;
    if bytes.len() != 16 {
        return Err("版权编号不是 128-bit V2 编号。".to_string());
    }
    let mut output = [0u8; 16];
    output.copy_from_slice(&bytes);
    Ok(output)
}

fn parse_hex_16(value: &str) -> Result<[u8; 16], String> {
    let bytes = hex::decode(value.trim()).map_err(|e| format!("registry proof 无效: {e}"))?;
    if bytes.len() != 16 {
        return Err("registry proof 长度无效。".to_string());
    }
    let mut output = [0u8; 16];
    output.copy_from_slice(&bytes);
    Ok(output)
}

fn parse_sha256_hex_32(value: &str) -> Result<[u8; 32], String> {
    let trimmed = value.trim().strip_prefix("sha256:").unwrap_or(value.trim());
    let bytes = hex::decode(trimmed).map_err(|e| format!("作品指纹不是有效 SHA-256: {e}"))?;
    if bytes.len() != 32 {
        return Err("作品指纹长度不是 SHA-256。".to_string());
    }
    let mut output = [0u8; 32];
    output.copy_from_slice(&bytes);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_record() -> VaultRecord {
        VaultRecord {
            id: 7,
            original_hash: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                .to_string(),
            file_name: "sample.png".to_string(),
            created_at: "2026-06-27T00:00:00Z".to_string(),
            duration_secs: 0.0,
            resolution: "1024x1024".to_string(),
            watermark_uid: "HS-11111111-22222222-33333333-44444444".to_string(),
            creator_display_name: Some("测试创作者".to_string()),
            thumbnail_path: None,
            output_douyin: None,
            output_bilibili: None,
            output_xhs: None,
            is_hdr_source: false,
            hw_encoder_used: None,
            process_time_ms: None,
            tsa_token_path: None,
            network_time: None,
            tsa_source: None,
            tsa_request_nonce: None,
            is_ai_generated: false,
            ai_training_permission: None,
            ai_generation_method: None,
            human_modification_level: None,
            authenticity_claim: None,
            custom_metadata: None,
            output_douyin_hash: None,
            output_bilibili_hash: None,
            output_xhs_hash: None,
            protected_copy_name: Some("sample.protected.png".to_string()),
            protected_copy_path: None,
            protected_copy_hash: None,
            output_strategy: "minimal_required_change".to_string(),
            work_source_declaration: "unspecified".to_string(),
            training_permission_declaration: "prohibited".to_string(),
            creation_method_declaration: "human_created".to_string(),
            human_edit_level_declaration: "unspecified".to_string(),
            authenticity_claim_declaration: "unspecified".to_string(),
            custom_rights_statement: None,
            parent_watermark_uid: None,
            revision: 1,
            rewrite_reason: None,
            write_verification_status: Some("failed".to_string()),
            write_verification_message: None,
            write_verification_at: None,
            payload_protocol_version: 2,
            payload_bytes_length: watermark_core::PAYLOAD_BYTES as u32,
            watermark_id_issue_mode: "offline_generated".to_string(),
            watermark_id_registry_status: "pending_registry_reconcile".to_string(),
            watermark_id_registry_receipt: None,
            payload_auth_status: "pending_repair".to_string(),
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
        }
    }

    fn sample_replacement() -> cloud::WatermarkIdRegistryResponse {
        cloud::WatermarkIdRegistryResponse {
            registry_id: "reg-reissue-1".to_string(),
            watermark_uid: "HS-55555555-66666666-77777777-88888888".to_string(),
            watermark_id_issue_mode: "server_reissued".to_string(),
            registry_status: "server_confirmed".to_string(),
            registry_receipt: "receipt-reissue-1".to_string(),
            registry_proof_hash: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_string(),
            payload_protocol_version: 2,
            payload_bytes_length: watermark_core::PAYLOAD_BYTES as u32,
            parent_watermark_uid: Some("HS-11111111-22222222-33333333-44444444".to_string()),
            revision: 2,
            issued_at: "2026-06-27T00:00:01Z".to_string(),
            updated_at: "2026-06-27T00:00:01Z".to_string(),
        }
    }

    #[test]
    fn reissue_payload_keeps_previous_uid_as_parent() {
        let record = sample_record();
        let replacement = sample_replacement();

        let payload = build_reissue_payload(&record, &replacement, "image", 2).unwrap();

        assert_eq!(payload.watermark_uid(), replacement.watermark_uid);
        assert_eq!(
            payload.parent_watermark_uid().as_deref(),
            Some(record.watermark_uid.as_str())
        );
        assert_eq!(payload.revision, 2);
        assert_eq!(payload.issue_mode, WatermarkIssueMode::ServerReissued);
        assert_eq!(payload.media_type, WatermarkMediaType::Image);
    }
}

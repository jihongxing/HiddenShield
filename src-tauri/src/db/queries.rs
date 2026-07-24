use rusqlite::{params, Connection, Transaction};

use crate::commands::vault::VaultRecord;
use crate::db::schema;

const VAULT_COLUMNS: &str = "id, original_hash, file_name, created_at, duration_secs,
    resolution, watermark_uid, creator_display_name, thumbnail_path, output_douyin,
    output_bilibili, output_xhs, is_hdr_source, hw_encoder_used,
    process_time_ms, tsa_token_path, network_time, tsa_source, tsa_request_nonce,
    is_ai_generated, ai_training_permission, ai_generation_method,
    human_modification_level, authenticity_claim, custom_metadata,
    output_douyin_hash, output_bilibili_hash, output_xhs_hash,
    protected_copy_name, protected_copy_path, protected_copy_hash,
    output_strategy, work_source_declaration, training_permission_declaration,
    creation_method_declaration, human_edit_level_declaration,
    authenticity_claim_declaration, custom_rights_statement,
    parent_watermark_uid, revision, rewrite_reason,
    write_verification_status, write_verification_message, write_verification_at,
    payload_protocol_version, payload_bytes_length, watermark_id_issue_mode,
    watermark_id_registry_status, watermark_id_registry_receipt, payload_auth_status,
    video_notary_id, video_notary_at, video_notary_receipt_signature,
    video_notary_usage_ledger_id, video_fingerprint_root, video_bundle_sha256,
    video_bundle_bytes, video_bundle_scene_count, video_bundle_elapsed_ms,
    video_frame_sample_policy, video_visual_task_id, video_visual_completed_at,
    video_visual_strategy_digest, video_visual_self_check_confidence,
    video_visual_self_check_threshold, video_visual_checked_frames,
    video_visual_media_hash, video_visual_receipt_hash, video_visual_output_bytes,
    video_visual_output_content_type";

fn row_to_vault_record(row: &rusqlite::Row<'_>) -> Result<VaultRecord, rusqlite::Error> {
    Ok(VaultRecord {
        id: row.get::<_, u32>(0)?,
        original_hash: row.get(1)?,
        file_name: row.get(2)?,
        created_at: row.get(3)?,
        duration_secs: row.get(4)?,
        resolution: row.get(5)?,
        watermark_uid: row.get(6)?,
        creator_display_name: row.get(7)?,
        thumbnail_path: row.get(8)?,
        output_douyin: row.get(9)?,
        output_bilibili: row.get(10)?,
        output_xhs: row.get(11)?,
        is_hdr_source: row.get::<_, i32>(12)? != 0,
        hw_encoder_used: row.get(13)?,
        process_time_ms: row.get::<_, Option<i64>>(14)?.map(|v| v as u64),
        tsa_token_path: row.get(15)?,
        network_time: row.get(16)?,
        tsa_source: row.get(17)?,
        tsa_request_nonce: row.get(18)?,
        is_ai_generated: row.get::<_, i32>(19)? != 0,
        ai_training_permission: row.get(20)?,
        ai_generation_method: row.get(21)?,
        human_modification_level: row.get(22)?,
        authenticity_claim: row.get(23)?,
        custom_metadata: row.get(24)?,
        output_douyin_hash: row.get(25)?,
        output_bilibili_hash: row.get(26)?,
        output_xhs_hash: row.get(27)?,
        protected_copy_name: row.get(28)?,
        protected_copy_path: row.get(29)?,
        protected_copy_hash: row.get(30)?,
        output_strategy: row
            .get::<_, Option<String>>(31)?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "minimal_required_change".to_string()),
        work_source_declaration: row
            .get::<_, Option<String>>(32)?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "unspecified".to_string()),
        training_permission_declaration: row
            .get::<_, Option<String>>(33)?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "prohibited".to_string()),
        creation_method_declaration: row
            .get::<_, Option<String>>(34)?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "unspecified".to_string()),
        human_edit_level_declaration: row
            .get::<_, Option<String>>(35)?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "unspecified".to_string()),
        authenticity_claim_declaration: row
            .get::<_, Option<String>>(36)?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "unspecified".to_string()),
        custom_rights_statement: row.get(37)?,
        parent_watermark_uid: row.get(38)?,
        revision: row.get::<_, i64>(39)? as u32,
        rewrite_reason: row.get(40)?,
        write_verification_status: row.get(41)?,
        write_verification_message: row.get(42)?,
        write_verification_at: row.get(43)?,
        payload_protocol_version: row.get::<_, i64>(44)? as u32,
        payload_bytes_length: row.get::<_, i64>(45)? as u32,
        watermark_id_issue_mode: row
            .get::<_, Option<String>>(46)?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "offline_generated".to_string()),
        watermark_id_registry_status: row
            .get::<_, Option<String>>(47)?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "pending_registration".to_string()),
        watermark_id_registry_receipt: row.get(48)?,
        payload_auth_status: row
            .get::<_, Option<String>>(49)?
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "verified".to_string()),
        video_notary_id: row.get(50)?,
        video_notary_at: row.get(51)?,
        video_notary_receipt_signature: row.get(52)?,
        video_notary_usage_ledger_id: row.get(53)?,
        video_fingerprint_root: row.get(54)?,
        video_bundle_sha256: row.get(55)?,
        video_bundle_bytes: row.get::<_, Option<i64>>(56)?.map(|value| value as u64),
        video_bundle_scene_count: row.get::<_, Option<i64>>(57)?.map(|value| value as u32),
        video_bundle_elapsed_ms: row.get::<_, Option<i64>>(58)?.map(|value| value as u64),
        video_frame_sample_policy: row.get(59)?,
        video_visual_task_id: row.get(60)?,
        video_visual_completed_at: row.get(61)?,
        video_visual_strategy_digest: row.get(62)?,
        video_visual_self_check_confidence: row.get(63)?,
        video_visual_self_check_threshold: row.get(64)?,
        video_visual_checked_frames: row.get::<_, Option<i64>>(65)?.map(|value| value as u32),
        video_visual_media_hash: row.get(66)?,
        video_visual_receipt_hash: row.get(67)?,
        video_visual_output_bytes: row.get::<_, Option<i64>>(68)?.map(|value| value as u64),
        video_visual_output_content_type: row.get(69)?,
    })
}

pub fn infer_vault_record_file_type(record: &VaultRecord) -> &'static str {
    if record.video_notary_id.is_some()
        || record.video_fingerprint_root.is_some()
        || record.video_visual_task_id.is_some()
        || record.video_visual_media_hash.is_some()
    {
        return "video";
    }

    match record.resolution.trim().to_ascii_lowercase().as_str() {
        "image" => return "image",
        "audio" => return "audio",
        _ => {}
    }

    if let Some(file_type) = media_type_from_extension(&record.file_name) {
        return file_type;
    }

    let candidates = [
        record.protected_copy_name.as_deref(),
        record.protected_copy_path.as_deref(),
        record.output_douyin.as_deref(),
        record.output_bilibili.as_deref(),
        record.output_xhs.as_deref(),
    ];

    if candidates
        .iter()
        .flatten()
        .any(|value| has_media_extension(value, IMAGE_EXTENSIONS))
    {
        return "image";
    }
    if candidates
        .iter()
        .flatten()
        .any(|value| has_media_extension(value, AUDIO_EXTENSIONS))
    {
        return "audio";
    }
    "video"
}

const IMAGE_EXTENSIONS: &[&str] = &["jpg", "jpeg", "png", "bmp", "tiff", "webp", "gif"];
const AUDIO_EXTENSIONS: &[&str] = &["wav", "mp3", "flac", "aac", "ogg", "m4a"];
const VIDEO_EXTENSIONS: &[&str] = &["mp4", "mov", "webm", "avi", "mkv", "m4v"];

fn media_type_from_extension(value: &str) -> Option<&'static str> {
    if has_media_extension(value, IMAGE_EXTENSIONS) {
        Some("image")
    } else if has_media_extension(value, AUDIO_EXTENSIONS) {
        Some("audio")
    } else if has_media_extension(value, VIDEO_EXTENSIONS) {
        Some("video")
    } else {
        None
    }
}

fn has_media_extension(value: &str, extensions: &[&str]) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    extensions
        .iter()
        .any(|extension| lower.ends_with(&format!(".{extension}")))
}

/// Initialize the database by running all pending migrations.
pub fn init_db(conn: &Connection) -> Result<(), rusqlite::Error> {
    schema::run_migrations(conn)
}

/// Insert a single VaultRecord into the database.
#[allow(dead_code)]
pub fn insert_record(conn: &Connection, record: &VaultRecord) -> Result<(), rusqlite::Error> {
    conn.execute(
        "INSERT INTO vault_records (
      original_hash, file_name, created_at, duration_secs, resolution,
      watermark_uid, creator_display_name, thumbnail_path, output_douyin, output_bilibili,
      output_xhs, is_hdr_source, hw_encoder_used, process_time_ms,
      tsa_token_path, network_time, tsa_source, tsa_request_nonce,
      is_ai_generated, ai_training_permission, ai_generation_method,
      human_modification_level, authenticity_claim, custom_metadata,
      output_douyin_hash, output_bilibili_hash, output_xhs_hash,
      protected_copy_name, protected_copy_path, protected_copy_hash,
      output_strategy, work_source_declaration, training_permission_declaration,
      creation_method_declaration, human_edit_level_declaration,
      authenticity_claim_declaration, custom_rights_statement,
      parent_watermark_uid, revision, rewrite_reason,
      write_verification_status, write_verification_message, write_verification_at,
      payload_protocol_version, payload_bytes_length, watermark_id_issue_mode,
      watermark_id_registry_status, watermark_id_registry_receipt, payload_auth_status,
      video_notary_id, video_notary_at, video_notary_receipt_signature,
      video_notary_usage_ledger_id, video_fingerprint_root, video_bundle_sha256,
      video_bundle_bytes, video_bundle_scene_count, video_bundle_elapsed_ms,
      video_frame_sample_policy, video_visual_task_id, video_visual_completed_at,
      video_visual_strategy_digest, video_visual_self_check_confidence,
      video_visual_self_check_threshold, video_visual_checked_frames,
      video_visual_media_hash, video_visual_receipt_hash, video_visual_output_bytes,
      video_visual_output_content_type, file_type
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70)",
        params![
            record.original_hash,
            record.file_name,
            record.created_at,
            record.duration_secs,
            record.resolution,
            record.watermark_uid,
            record.creator_display_name,
            record.thumbnail_path,
            record.output_douyin,
            record.output_bilibili,
            record.output_xhs,
            record.is_hdr_source as i32,
            record.hw_encoder_used,
            record.process_time_ms.map(|v| v as i64),
            record.tsa_token_path,
            record.network_time,
            record.tsa_source,
            record.tsa_request_nonce,
            record.is_ai_generated as i32,
            record.ai_training_permission,
            record.ai_generation_method,
            record.human_modification_level,
            record.authenticity_claim,
            record.custom_metadata,
            record.output_douyin_hash,
            record.output_bilibili_hash,
            record.output_xhs_hash,
            record.protected_copy_name,
            record.protected_copy_path,
            record.protected_copy_hash,
            record.output_strategy,
            record.work_source_declaration,
            record.training_permission_declaration,
            record.creation_method_declaration,
            record.human_edit_level_declaration,
            record.authenticity_claim_declaration,
            record.custom_rights_statement,
            record.parent_watermark_uid,
            record.revision as i64,
            record.rewrite_reason,
            record.write_verification_status,
            record.write_verification_message,
            record.write_verification_at,
            record.payload_protocol_version as i64,
            record.payload_bytes_length as i64,
            record.watermark_id_issue_mode,
            record.watermark_id_registry_status,
            record.watermark_id_registry_receipt,
            record.payload_auth_status,
            record.video_notary_id,
            record.video_notary_at,
            record.video_notary_receipt_signature,
            record.video_notary_usage_ledger_id,
            record.video_fingerprint_root,
            record.video_bundle_sha256,
            record.video_bundle_bytes.map(|value| value as i64),
            record.video_bundle_scene_count.map(|value| value as i64),
            record.video_bundle_elapsed_ms.map(|value| value as i64),
            record.video_frame_sample_policy,
            record.video_visual_task_id,
            record.video_visual_completed_at,
            record.video_visual_strategy_digest,
            record.video_visual_self_check_confidence,
            record.video_visual_self_check_threshold,
            record.video_visual_checked_frames.map(|value| value as i64),
            record.video_visual_media_hash,
            record.video_visual_receipt_hash,
            record.video_visual_output_bytes.map(|value| value as i64),
            record.video_visual_output_content_type,
            infer_vault_record_file_type(record),
        ],
    )?;
    Ok(())
}

/// Insert a VaultRecord inside an existing transaction and return the new row id.
pub fn insert_record_tx(
    tx: &Transaction<'_>,
    record: &VaultRecord,
) -> Result<i64, rusqlite::Error> {
    tx.execute(
        "INSERT INTO vault_records (
      original_hash, file_name, created_at, duration_secs, resolution,
      watermark_uid, creator_display_name, thumbnail_path, output_douyin, output_bilibili,
      output_xhs, is_hdr_source, hw_encoder_used, process_time_ms,
      tsa_token_path, network_time, tsa_source, tsa_request_nonce,
      is_ai_generated, ai_training_permission, ai_generation_method,
      human_modification_level, authenticity_claim, custom_metadata,
      output_douyin_hash, output_bilibili_hash, output_xhs_hash,
      protected_copy_name, protected_copy_path, protected_copy_hash,
      output_strategy, work_source_declaration, training_permission_declaration,
      creation_method_declaration, human_edit_level_declaration,
      authenticity_claim_declaration, custom_rights_statement,
      parent_watermark_uid, revision, rewrite_reason,
      write_verification_status, write_verification_message, write_verification_at,
      payload_protocol_version, payload_bytes_length, watermark_id_issue_mode,
      watermark_id_registry_status, watermark_id_registry_receipt, payload_auth_status,
      video_notary_id, video_notary_at, video_notary_receipt_signature,
      video_notary_usage_ledger_id, video_fingerprint_root, video_bundle_sha256,
      video_bundle_bytes, video_bundle_scene_count, video_bundle_elapsed_ms,
      video_frame_sample_policy, video_visual_task_id, video_visual_completed_at,
      video_visual_strategy_digest, video_visual_self_check_confidence,
      video_visual_self_check_threshold, video_visual_checked_frames,
      video_visual_media_hash, video_visual_receipt_hash, video_visual_output_bytes,
      video_visual_output_content_type, file_type
    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35, ?36, ?37, ?38, ?39, ?40, ?41, ?42, ?43, ?44, ?45, ?46, ?47, ?48, ?49, ?50, ?51, ?52, ?53, ?54, ?55, ?56, ?57, ?58, ?59, ?60, ?61, ?62, ?63, ?64, ?65, ?66, ?67, ?68, ?69, ?70)",
        params![
            record.original_hash,
            record.file_name,
            record.created_at,
            record.duration_secs,
            record.resolution,
            record.watermark_uid,
            record.creator_display_name,
            record.thumbnail_path,
            record.output_douyin,
            record.output_bilibili,
            record.output_xhs,
            record.is_hdr_source as i32,
            record.hw_encoder_used,
            record.process_time_ms.map(|v| v as i64),
            record.tsa_token_path,
            record.network_time,
            record.tsa_source,
            record.tsa_request_nonce,
            record.is_ai_generated as i32,
            record.ai_training_permission,
            record.ai_generation_method,
            record.human_modification_level,
            record.authenticity_claim,
            record.custom_metadata,
            record.output_douyin_hash,
            record.output_bilibili_hash,
            record.output_xhs_hash,
            record.protected_copy_name,
            record.protected_copy_path,
            record.protected_copy_hash,
            record.output_strategy,
            record.work_source_declaration,
            record.training_permission_declaration,
            record.creation_method_declaration,
            record.human_edit_level_declaration,
            record.authenticity_claim_declaration,
            record.custom_rights_statement,
            record.parent_watermark_uid,
            record.revision as i64,
            record.rewrite_reason,
            record.write_verification_status,
            record.write_verification_message,
            record.write_verification_at,
            record.payload_protocol_version as i64,
            record.payload_bytes_length as i64,
            record.watermark_id_issue_mode,
            record.watermark_id_registry_status,
            record.watermark_id_registry_receipt,
            record.payload_auth_status,
            record.video_notary_id,
            record.video_notary_at,
            record.video_notary_receipt_signature,
            record.video_notary_usage_ledger_id,
            record.video_fingerprint_root,
            record.video_bundle_sha256,
            record.video_bundle_bytes.map(|value| value as i64),
            record.video_bundle_scene_count.map(|value| value as i64),
            record.video_bundle_elapsed_ms.map(|value| value as i64),
            record.video_frame_sample_policy,
            record.video_visual_task_id,
            record.video_visual_completed_at,
            record.video_visual_strategy_digest,
            record.video_visual_self_check_confidence,
            record.video_visual_self_check_threshold,
            record.video_visual_checked_frames.map(|value| value as i64),
            record.video_visual_media_hash,
            record.video_visual_receipt_hash,
            record.video_visual_output_bytes.map(|value| value as i64),
            record.video_visual_output_content_type,
            infer_vault_record_file_type(record),
        ],
    )?;
    Ok(tx.last_insert_rowid())
}

/// Query all vault records ordered by created_at descending.
pub fn list_records(conn: &Connection) -> Vec<VaultRecord> {
    let mut stmt = match conn.prepare(&format!(
        "SELECT {VAULT_COLUMNS} FROM vault_records ORDER BY created_at DESC"
    )) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let rows = stmt.query_map([], row_to_vault_record);

    match rows {
        Ok(mapped) => mapped.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

pub fn find_by_id(conn: &Connection, record_id: u32) -> Option<VaultRecord> {
    conn.query_row(
        &format!("SELECT {VAULT_COLUMNS} FROM vault_records WHERE id = ?1 LIMIT 1"),
        params![record_id],
        row_to_vault_record,
    )
    .ok()
}

pub fn update_timestamp_attestation(
    conn: &Connection,
    record_id: u32,
    tsa_token_path: Option<&str>,
    network_time: Option<&str>,
    tsa_source: Option<&str>,
    tsa_request_nonce: Option<&str>,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE vault_records
         SET tsa_token_path = ?2,
             network_time = ?3,
             tsa_source = ?4,
             tsa_request_nonce = ?5
         WHERE id = ?1",
        params![
            record_id,
            tsa_token_path,
            network_time,
            tsa_source,
            tsa_request_nonce
        ],
    )?;
    Ok(())
}

/// Find the latest record by watermark_uid.
#[allow(dead_code)]
pub fn find_by_watermark_uid(conn: &Connection, uid: &str) -> Option<VaultRecord> {
    conn.query_row(
        &format!(
            "SELECT {VAULT_COLUMNS} FROM vault_records WHERE watermark_uid = ?1 ORDER BY created_at DESC, id DESC LIMIT 1"
        ),
        params![uid],
        row_to_vault_record,
    )
    .ok()
}

/// Find a record by watermark_uid and exact file hash prefix.
/// Returns `None` if the asset-binding hash prefix does not match.
pub fn find_by_uid_and_hash(
    conn: &Connection,
    uid: &str,
    file_hash_prefix: &[u8; 2],
) -> Option<VaultRecord> {
    let mut stmt = conn
        .prepare(
            &format!(
                "SELECT {VAULT_COLUMNS} FROM vault_records WHERE watermark_uid = ?1 ORDER BY created_at DESC"
            ),
        )
        .ok()?;

    let records: Vec<VaultRecord> = stmt
        .query_map(params![uid], row_to_vault_record)
        .ok()?
        .filter_map(|r| r.ok())
        .collect();

    if records.is_empty() {
        return None;
    }

    // Try to match by file hash prefix (first 2 bytes of SHA-256)
    let prefix_hex = hex::encode(file_hash_prefix);
    for record in &records {
        // Check original file hash
        if record.original_hash.starts_with(&prefix_hex) {
            return Some(record.clone());
        }
        // Check output file hashes
        if let Some(ref hash) = record.output_douyin_hash {
            if hash.starts_with(&prefix_hex) {
                return Some(record.clone());
            }
        }
        if let Some(ref hash) = record.output_bilibili_hash {
            if hash.starts_with(&prefix_hex) {
                return Some(record.clone());
            }
        }
        if let Some(ref hash) = record.output_xhs_hash {
            if hash.starts_with(&prefix_hex) {
                return Some(record.clone());
            }
        }
    }

    None
}

/// Whether any vault record exists for the given watermark UID.
pub fn has_watermark_uid(conn: &Connection, uid: &str) -> bool {
    conn.query_row(
        "SELECT 1 FROM vault_records WHERE watermark_uid = ?1 LIMIT 1",
        params![uid],
        |_| Ok(()),
    )
    .is_ok()
}

pub fn update_watermark_registry_fields(
    conn: &Connection,
    record_id: u32,
    issue_mode: &str,
    registry_status: &str,
    registry_receipt: Option<&str>,
    payload_protocol_version: u32,
    payload_bytes_length: u32,
    parent_watermark_uid: Option<&str>,
    revision: u32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE vault_records
         SET watermark_id_issue_mode = ?2,
             watermark_id_registry_status = ?3,
             watermark_id_registry_receipt = ?4,
             payload_protocol_version = ?5,
             payload_bytes_length = ?6,
             parent_watermark_uid = ?7,
             revision = ?8
         WHERE id = ?1",
        params![
            record_id,
            issue_mode,
            registry_status,
            registry_receipt,
            payload_protocol_version as i64,
            payload_bytes_length as i64,
            parent_watermark_uid,
            revision as i64,
        ],
    )?;
    Ok(())
}

pub fn mark_record_reissue_required(
    conn: &Connection,
    record_id: u32,
    registry_receipt: Option<&str>,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE vault_records
         SET watermark_id_registry_status = 'reissue_required',
             watermark_id_registry_receipt = COALESCE(?2, watermark_id_registry_receipt),
             write_verification_status = 'failed',
             write_verification_message = '已创建重新签发任务，但本机暂时无法访问保护副本。请重新选择原作品或保护副本后生成新的 V2 保护副本。',
             payload_auth_status = 'pending_repair'
         WHERE id = ?1",
        params![record_id, registry_receipt],
    )?;
    Ok(())
}

pub fn update_record_after_reissue_repair(
    conn: &Connection,
    record_id: u32,
    previous_watermark_uid: &str,
    replacement_watermark_uid: &str,
    protected_copy_name: &str,
    protected_copy_path: &str,
    protected_copy_hash: &str,
    issue_mode: &str,
    registry_status: &str,
    registry_receipt: &str,
    payload_protocol_version: u32,
    payload_bytes_length: u32,
    revision: u32,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "UPDATE vault_records
         SET watermark_uid = ?3,
             protected_copy_name = ?4,
             protected_copy_path = ?5,
             protected_copy_hash = ?6,
             watermark_id_issue_mode = ?7,
             watermark_id_registry_status = ?8,
             watermark_id_registry_receipt = ?9,
             payload_protocol_version = ?10,
             payload_bytes_length = ?11,
             parent_watermark_uid = ?2,
             revision = ?12,
             rewrite_reason = '历史重复编号重新签发并修复保护副本',
             write_verification_status = 'verified',
             write_verification_message = '已重新签发版权编号，并回读验证修复后的 V2 保护副本。',
             write_verification_at = ?13,
             payload_auth_status = 'verified'
         WHERE id = ?1",
        params![
            record_id,
            previous_watermark_uid,
            replacement_watermark_uid,
            protected_copy_name,
            protected_copy_path,
            protected_copy_hash,
            issue_mode,
            registry_status,
            registry_receipt,
            payload_protocol_version as i64,
            payload_bytes_length as i64,
            revision as i64,
            chrono::Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    fn sample_record(hash: &str, uid: &str, created_at: &str) -> VaultRecord {
        VaultRecord {
            id: 0,
            original_hash: hash.to_string(),
            file_name: "sample.mp4".to_string(),
            created_at: created_at.to_string(),
            duration_secs: 1.0,
            resolution: "1920x1080".to_string(),
            watermark_uid: uid.to_string(),
            creator_display_name: Some("测试创作者".to_string()),
            thumbnail_path: None,
            output_douyin: None,
            output_bilibili: None,
            output_xhs: None,
            output_douyin_hash: None,
            output_bilibili_hash: None,
            output_xhs_hash: None,
            protected_copy_name: None,
            protected_copy_path: None,
            protected_copy_hash: None,
            output_strategy: "minimal_required_change".to_string(),
            work_source_declaration: "unspecified".to_string(),
            training_permission_declaration: "prohibited".to_string(),
            creation_method_declaration: "unspecified".to_string(),
            human_edit_level_declaration: "unspecified".to_string(),
            authenticity_claim_declaration: "unspecified".to_string(),
            custom_rights_statement: None,
            parent_watermark_uid: None,
            revision: 1,
            rewrite_reason: None,
            write_verification_status: None,
            write_verification_message: None,
            write_verification_at: None,
            payload_protocol_version: 2,
            payload_bytes_length: 119,
            watermark_id_issue_mode: "offline_generated".to_string(),
            watermark_id_registry_status: "pending_registration".to_string(),
            watermark_id_registry_receipt: None,
            payload_auth_status: "verified".to_string(),
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
        }
    }

    #[test]
    fn exact_hash_prefix_is_required_for_match() {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();

        let uid = "HS-ABCDEF01-23456789-AABBCCDD-EEFF0011";
        insert_record(
            &conn,
            &sample_record("11223344deadbeef", uid, "2026-04-20T10:00:00Z"),
        )
        .unwrap();
        insert_record(
            &conn,
            &sample_record("55667788cafebabe", uid, "2026-04-21T10:00:00Z"),
        )
        .unwrap();

        assert!(has_watermark_uid(&conn, uid));
        assert!(find_by_uid_and_hash(&conn, uid, &[0x11, 0x22]).is_some());
        assert!(find_by_uid_and_hash(&conn, uid, &[0xaa, 0xbb]).is_none());
    }

    #[test]
    fn insert_record_persists_inferred_file_type() {
        let conn = Connection::open_in_memory().unwrap();
        schema::run_migrations(&conn).unwrap();

        let mut image = sample_record("image-hash", "HS-IMAGE-FILETYPE", "2026-07-04T02:00:00Z");
        image.file_name = "creator-cover.png".to_string();
        image.protected_copy_name = Some("creator-cover_watermarked.png".to_string());
        insert_record(&conn, &image).unwrap();

        let mut audio = sample_record("audio-hash", "HS-AUDIO-FILETYPE", "2026-07-04T02:01:00Z");
        audio.file_name = "field-recording.m4a".to_string();
        audio.protected_copy_name = Some("field-recording_watermarked.wav".to_string());
        insert_record(&conn, &audio).unwrap();

        let mut l2 = sample_record("l2-hash", "HS-L2-FILETYPE", "2026-07-04T02:02:00Z");
        l2.file_name = "operator-renamed.png".to_string();
        l2.video_notary_id = Some("vfn_filetype".to_string());
        l2.video_fingerprint_root = Some("sha256:filetype-root".to_string());
        insert_record(&conn, &l2).unwrap();

        let mut stmt = conn
            .prepare("SELECT watermark_uid, file_type FROM vault_records ORDER BY watermark_uid")
            .unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        assert_eq!(
            rows,
            vec![
                ("HS-AUDIO-FILETYPE".to_string(), "audio".to_string()),
                ("HS-IMAGE-FILETYPE".to_string(), "image".to_string()),
                ("HS-L2-FILETYPE".to_string(), "video".to_string()),
            ]
        );
    }
}

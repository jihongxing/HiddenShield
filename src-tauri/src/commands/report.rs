use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, Runtime, State};

use crate::commands::sync::get_desktop_cloud_sync_profile;
use crate::commands::vault::{creator_display_name_for_display, VaultRecord};
use crate::db::{billing, queries};
use crate::entitlements;
use crate::report_pdf::{ReportPdfRenderResult, REPORT_PDF_GENERATION_BUDGET_MS};
use crate::AppState;

const FORMAL_REPORT_DISCLAIMER: &str = "本报告由 HiddenShield 根据本机版权库记录生成，仅作为技术验证与版权管理辅助材料，不构成法律意见、司法鉴定意见或诉讼结果承诺。载荷认证标签与 Manifest 摘要匹配只表示技术规则或文件完整性匹配，不等于发行方数字签名、实名认证或法定权属确认。";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportVaultFormalReportInput {
    pub record_id: u32,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyFormalReportBundleInput {
    pub report_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyRightsEvidencePackInput {
    pub case_dir: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMobileReportHandoffInput {
    pub report_dir: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormalReportExportResult {
    pub report_id: String,
    pub report_type: String,
    pub report_dir: String,
    pub pdf_path: String,
    pub json_path: String,
    pub manifest_path: String,
    pub exported_at: String,
    pub record_count: usize,
    pub pdf_generation_ms: f64,
    pub pdf_page_count: usize,
    pub bundle_version: u32,
    pub supersedes_report_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportManifest {
    schema_version: u32,
    report_id: String,
    report_type: String,
    generated_at: String,
    source_schema_version: u32,
    bundle: FormalReportBundleLineage,
    renderer: FormalReportRenderer,
    files: Vec<FormalReportManifestFile>,
    integrity: FormalReportIntegrityChain,
    signature: FormalReportSignatureStatus,
    trusted_time: FormalReportTrustedTimeStatus,
    verification: FormalReportVerificationContract,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportBundleLineage {
    source_key: String,
    bundle_version: u32,
    supersedes_report_id: Option<String>,
    source_handoff_report_id: Option<String>,
    source_handoff_source_key: Option<String>,
    source_handoff_root_digest: Option<String>,
    source_handoff_platform: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportRenderer {
    engine: String,
    worker_mode: String,
    template_version: String,
    controlled_fonts: Vec<String>,
    generation_ms: f64,
    generation_budget_ms: u64,
    page_count: usize,
    pagination_stable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportManifestFile {
    path: String,
    media_type: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportIntegrityChain {
    algorithm: String,
    genesis: String,
    entries: Vec<FormalReportIntegrityEntry>,
    root_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportIntegrityEntry {
    sequence: usize,
    path: String,
    file_sha256: String,
    file_bytes: u64,
    previous_chain_digest: String,
    chain_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportSignatureStatus {
    status: String,
    profile: Option<String>,
    signer_key_id: Option<String>,
    certificate_chain_status: String,
    revocation_status: String,
    signed_at: Option<String>,
    note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportTrustedTimeStatus {
    status: String,
    package_timestamp_present: bool,
    record_material_token_present: bool,
    note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportVerificationContract {
    offline_mode: String,
    online_status: String,
    qr_status: String,
    online_verification_url: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormalReportBundleVerificationResult {
    pub report_id: Option<String>,
    pub report_type: Option<String>,
    pub report_dir: String,
    pub verified_at: String,
    pub manifest_schema_version: Option<u32>,
    pub bundle_version: Option<u32>,
    pub supersedes_report_id: Option<String>,
    pub integrity_status: String,
    pub manifest_chain_status: String,
    pub document_contract_status: String,
    pub signature_status: String,
    pub trusted_time_status: String,
    pub files: Vec<FormalReportVerifiedFile>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormalReportVerifiedFile {
    pub path: String,
    pub expected_bytes: u64,
    pub actual_bytes: Option<u64>,
    pub expected_sha256: String,
    pub actual_sha256: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RightsEvidencePackVerificationResult {
    pub pack_id: Option<String>,
    pub case_id: Option<String>,
    pub case_dir: String,
    pub verified_at: String,
    pub manifest_schema_version: Option<u32>,
    pub directory_contract_status: String,
    pub attachment_integrity_status: String,
    pub event_chain_status: String,
    pub attachment_chain_status: String,
    pub signature_status: String,
    pub trusted_time_status: String,
    pub declared_root_digest: Option<String>,
    pub computed_root_digest: Option<String>,
    pub attachments: Vec<RightsEvidencePackVerifiedAttachment>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RightsEvidencePackVerifiedAttachment {
    pub attachment_id: String,
    pub path: String,
    pub role: String,
    pub expected_bytes: u64,
    pub actual_bytes: Option<u64>,
    pub expected_sha256: String,
    pub actual_sha256: Option<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackCaseDocument {
    schema_version: u32,
    document_type: String,
    pack_id: String,
    case: RightsEvidencePackCaseIdentity,
    collection_events: Vec<serde_json::Value>,
    attachments: Vec<RightsEvidencePackCaseAttachment>,
    automated_findings: Vec<RightsEvidencePackAutomatedFinding>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackCaseIdentity {
    case_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackCaseAttachment {
    attachment_id: String,
    sequence: usize,
    role: String,
    relative_path: String,
    derived_from_attachment_id: Option<String>,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackAutomatedFinding {
    input_attachment_ids: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackManifest {
    schema_version: u32,
    manifest_type: String,
    pack_id: String,
    case_id: String,
    directory_contract: RightsEvidencePackDirectoryContract,
    case_file: RightsEvidencePackCaseFile,
    files: Vec<RightsEvidencePackManifestAttachment>,
    event_chain: RightsEvidencePackEventChain,
    attachment_chain: RightsEvidencePackAttachmentChain,
    integrity: RightsEvidencePackRootIntegrity,
    signature: RightsEvidencePackTrustStatus,
    trusted_time: RightsEvidencePackTrustStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackDirectoryContract {
    case_document: String,
    manifest: String,
    attachment_root: String,
    allowed_top_level_entries: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct RightsEvidencePackCaseFile {
    path: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackManifestAttachment {
    attachment_id: String,
    path: String,
    role: String,
    bytes: u64,
    sha256: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackEventChain {
    algorithm: String,
    genesis: String,
    entries: Vec<RightsEvidencePackEventChainEntry>,
    root_digest: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackEventChainEntry {
    sequence: usize,
    event_id: String,
    event_digest: String,
    previous_chain_digest: String,
    chain_digest: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackAttachmentChain {
    algorithm: String,
    genesis: String,
    entries: Vec<RightsEvidencePackAttachmentChainEntry>,
    root_digest: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackAttachmentChainEntry {
    sequence: usize,
    attachment_id: String,
    path: String,
    role: String,
    file_bytes: u64,
    file_sha256: String,
    previous_chain_digest: String,
    chain_digest: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RightsEvidencePackRootIntegrity {
    algorithm: String,
    root_digest: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RightsEvidencePackTrustStatus {
    status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportDocument {
    schema_version: u32,
    report_id: String,
    report_type: String,
    exported_at: String,
    app_version: String,
    records: Vec<FormalReportRecord>,
    privacy: FormalReportPrivacy,
    disclaimer: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportRecord {
    record_id: u32,
    file_name: String,
    watermark_uid: String,
    creator_display_name: Option<String>,
    original_hash: String,
    resolution: String,
    duration_secs: f64,
    created_at: String,
    revision: u32,
    parent_watermark_uid: Option<String>,
    rewrite_reason: Option<String>,
    write_verification_status: Option<String>,
    write_verification_message: Option<String>,
    write_verification_at: Option<String>,
    payload_registry: FormalReportPayloadRegistry,
    protected_copy: FormalReportProtectedCopy,
    trusted_time: FormalReportTrustedTime,
    rights_declaration: FormalReportRightsDeclaration,
    video_notary: FormalReportVideoNotary,
    video_visual_watermark: FormalReportVideoVisualWatermark,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportProtectedCopy {
    name: Option<String>,
    hash: Option<String>,
    output_strategy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportPayloadRegistry {
    payload_protocol_version: u32,
    payload_bytes_length: u32,
    media_payload_role: String,
    watermark_id_issue_mode: String,
    watermark_id_registry_status: String,
    watermark_id_registry_receipt: Option<String>,
    payload_auth_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportTrustedTime {
    network_time: Option<String>,
    tsa_source: Option<String>,
    tsa_token_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportRightsDeclaration {
    work_source_declaration: String,
    training_permission_declaration: String,
    creation_method_declaration: String,
    human_edit_level_declaration: String,
    authenticity_claim_declaration: String,
    custom_rights_statement: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportVideoNotary {
    notary_id: Option<String>,
    notary_at: Option<String>,
    receipt_signature: Option<String>,
    usage_ledger_id: Option<String>,
    fingerprint_root: Option<String>,
    bundle_sha256: Option<String>,
    bundle_bytes: Option<u64>,
    bundle_scene_count: Option<u32>,
    bundle_elapsed_ms: Option<u64>,
    frame_sample_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportVideoVisualWatermark {
    task_id: Option<String>,
    completed_at: Option<String>,
    strategy_digest: Option<String>,
    self_check_confidence: Option<f64>,
    self_check_threshold: Option<f64>,
    checked_frames: Option<u32>,
    media_hash: Option<String>,
    receipt_hash: Option<String>,
    output_bytes: Option<u64>,
    output_content_type: Option<String>,
}

#[cfg(test)]
impl FormalReportVideoNotary {
    fn has_notary(&self) -> bool {
        self.notary_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
    }
}

#[cfg(test)]
impl FormalReportVideoVisualWatermark {
    fn has_receipt(&self) -> bool {
        self.task_id
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
            || self
                .media_hash
                .as_deref()
                .is_some_and(|value| !value.trim().is_empty())
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FormalReportPrivacy {
    excludes_original_media: bool,
    excludes_watermarked_media: bool,
    excludes_local_media_paths: bool,
    included_fields: Vec<&'static str>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MobileReportHandoffDocument {
    schema_version: u32,
    report_id: String,
    report_type: String,
    exported_at: String,
    source_platform: String,
    records: Vec<MobileReportHandoffRecord>,
    handoff: MobileReportHandoffStatus,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MobileReportHandoffRecord {
    source_record_id: String,
    file_name: String,
    watermark_uid: String,
    creator_display_name: Option<String>,
    original_hash: String,
    created_at: String,
    revision: u32,
    parent_watermark_uid: Option<String>,
    rewrite_reason: Option<String>,
    write_verification_status: Option<String>,
    write_verification_message: Option<String>,
    write_verification_at: Option<String>,
    payload_registry: FormalReportPayloadRegistry,
    protected_copy: FormalReportProtectedCopy,
    trusted_time: FormalReportTrustedTime,
    rights_declaration: FormalReportRightsDeclaration,
    video_notary: FormalReportVideoNotary,
    video_visual_watermark: FormalReportVideoVisualWatermark,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MobileReportHandoffStatus {
    status: String,
    requested_output: Vec<String>,
}

#[derive(Debug, Clone)]
struct FormalReportImportSource {
    source_key: String,
    report_id: String,
    root_digest: String,
    platform: String,
}

#[tauri::command]
pub async fn export_vault_formal_report(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: ExportVaultFormalReportInput,
) -> Result<FormalReportExportResult, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let record = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        let entitlement = entitlements::resolve_effective_entitlement(
            &conn,
            state.installation_secret_store.as_ref(),
        )
        .map_err(|e| format!("读取权益状态失败: {e}"))?;
        let record = queries::list_records(&conn)
            .into_iter()
            .find(|record| record.id == input.record_id)
            .ok_or_else(|| format!("未找到版权记录: {}", input.record_id))?;
        ensure_single_report_export_entitled(&app_handle, &conn, &entitlement, &record)?;
        record
    };
    let record = backfill_report_creator(record, &app_data_dir);

    let exported = export_report_files(
        &app_handle,
        &state,
        build_report_document("formal_report", vec![record.clone()]),
    )?;

    record_report_usage(&state, Some(record.id as i64), exported_size(&exported)?)?;
    Ok(exported)
}

#[tauri::command]
pub async fn export_vault_batch_summary_report(
    app_handle: AppHandle,
    state: State<'_, AppState>,
) -> Result<FormalReportExportResult, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let records = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        let entitlement = entitlements::resolve_effective_entitlement(
            &conn,
            state.installation_secret_store.as_ref(),
        )
        .map_err(|e| format!("读取权益状态失败: {e}"))?;
        ensure_report_export_entitled(&entitlement)?;
        queries::list_records(&conn)
    };
    if records.is_empty() {
        return Err("版权库暂无可导出的记录".to_string());
    }
    let records = records
        .into_iter()
        .map(|record| backfill_report_creator(record, &app_data_dir))
        .collect();

    let exported = export_report_files(
        &app_handle,
        &state,
        build_report_document("batch_summary", records),
    )?;

    record_report_usage(&state, None, exported_size(&exported)?)?;
    Ok(exported)
}

#[tauri::command]
pub async fn verify_formal_report_bundle(
    app_handle: AppHandle,
    input: VerifyFormalReportBundleInput,
) -> Result<FormalReportBundleVerificationResult, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let reports_root = std::fs::canonicalize(app_data_dir.join("reports")).ok();
    let report_dir = std::fs::canonicalize(&input.report_dir)
        .map_err(|e| format!("报告包目录不存在或不可访问: {e}"))?;
    let verified = verify_report_bundle_at(&report_dir)?;
    let is_internal_report = reports_root
        .as_ref()
        .is_some_and(|reports_root| report_dir.starts_with(reports_root));
    if !is_internal_report && verified.report_type.as_deref() != Some("formal_report_handoff") {
        return Err("外部目录只允许校验 HiddenShield 移动报告交接包".to_string());
    }
    Ok(verified)
}

#[tauri::command]
pub async fn verify_rights_evidence_pack(
    input: VerifyRightsEvidencePackInput,
) -> Result<RightsEvidencePackVerificationResult, String> {
    let case_dir = std::fs::canonicalize(&input.case_dir)
        .map_err(|error| format!("案件包目录不存在或不可访问: {error}"))?;
    verify_rights_evidence_pack_at(&case_dir)
}

#[cfg(feature = "runtime-qa")]
pub fn build_rights_evidence_pack_runtime_qa_app(
) -> Result<tauri::App<tauri::test::MockRuntime>, String> {
    tauri::test::mock_builder()
        .invoke_handler(tauri::generate_handler![verify_rights_evidence_pack])
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .map_err(|error| format!("构建 Tauri MockRuntime 失败: {error}"))
}

#[tauri::command]
pub async fn import_mobile_report_handoff(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: ImportMobileReportHandoffInput,
) -> Result<FormalReportExportResult, String> {
    import_mobile_report_handoff_with_state(&app_handle, &state, &input.report_dir)
}

pub fn import_mobile_report_handoff_with_state<R: Runtime>(
    app_handle: &AppHandle<R>,
    state: &AppState,
    report_dir: &str,
) -> Result<FormalReportExportResult, String> {
    {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        let entitlement = entitlements::resolve_effective_entitlement(
            &conn,
            state.installation_secret_store.as_ref(),
        )
        .map_err(|e| format!("读取权益状态失败: {e}"))?;
        ensure_report_export_entitled(&entitlement)?;
    }
    let report_dir = std::fs::canonicalize(report_dir)
        .map_err(|e| format!("移动交接包目录不存在或不可访问: {e}"))?;
    let (document, import_source) = prepare_mobile_report_handoff_import(&report_dir)?;
    let exported =
        export_report_files_with_source(app_handle, state, document, Some(import_source))?;
    record_report_usage(state, None, exported_size(&exported)?)?;
    Ok(exported)
}

pub fn run_mobile_report_handoff_runtime_qa<R: Runtime>(
    app_handle: &AppHandle<R>,
    report_dir: &str,
) -> Result<FormalReportExportResult, String> {
    let conn = rusqlite::Connection::open_in_memory()
        .map_err(|error| format!("创建运行态 QA 数据库失败: {error}"))?;
    queries::init_db(&conn).map_err(|error| format!("初始化运行态 QA 数据库失败: {error}"))?;
    let mut entitlement = billing::EntitlementState::default();
    entitlement.status = billing::EntitlementStatus::Active;
    entitlement.plan_name = Some("Creator Runtime QA".to_string());
    entitlement.plan_code = "creator_runtime_qa".to_string();
    entitlement
        .features
        .insert("report_export".to_string(), true);
    entitlement.billing_source = Some("runtime_qa".to_string());
    billing::save_entitlement_state(&conn, &entitlement)
        .map_err(|error| format!("写入运行态 QA 权益失败: {error}"))?;
    let state = AppState::new(conn);
    import_mobile_report_handoff_with_state(app_handle, &state, report_dir)
}

fn backfill_report_creator(mut record: VaultRecord, app_data_dir: &std::path::Path) -> VaultRecord {
    if record
        .creator_display_name
        .as_deref()
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        record.creator_display_name = creator_display_name_for_display(app_data_dir);
    }
    record
}

fn ensure_report_export_entitled(entitlement: &billing::EntitlementState) -> Result<(), String> {
    if entitlement.features.get("report_export") == Some(&true) {
        return Ok(());
    }
    Err("正式报告需要按记录单独购买，当前可复制基础摘要".to_string())
}

fn ensure_single_report_export_entitled(
    app_handle: &AppHandle,
    conn: &rusqlite::Connection,
    entitlement: &billing::EntitlementState,
    record: &VaultRecord,
) -> Result<(), String> {
    if ensure_report_export_entitled(entitlement).is_ok() {
        return Ok(());
    }
    let profile = get_desktop_cloud_sync_profile(app_handle.clone())?
        .ok_or_else(|| "正式报告需要按记录单独购买，当前可复制基础摘要".to_string())?;
    let record_id = record.id.to_string();
    let has_grant = ["copyright_report_single", "rights_evidence_pack_single"]
        .iter()
        .try_fold(false, |allowed, product_code| {
            if allowed {
                return Ok(true);
            }
            billing::has_active_report_purchase_grant(
                conn,
                &profile.account_id,
                &profile.workspace_id,
                &record_id,
                product_code,
            )
        })
        .map_err(|e| format!("读取报告授权失败: {e}"))?;
    if has_grant {
        return Ok(());
    }
    Err("正式报告需要按记录单独购买，当前可复制基础摘要".to_string())
}

fn build_report_document(report_type: &str, records: Vec<VaultRecord>) -> FormalReportDocument {
    let exported_at = Utc::now().to_rfc3339();
    let seed = records
        .iter()
        .map(|record| format!("{}:{}:{}", record.id, record.watermark_uid, record.revision))
        .collect::<Vec<_>>()
        .join("|");
    let report_id = stable_report_id(report_type, &exported_at, &seed);
    FormalReportDocument {
        schema_version: 2,
        report_id,
        report_type: report_type.to_string(),
        exported_at,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        records: records.into_iter().map(report_record_from_vault).collect(),
        privacy: FormalReportPrivacy {
            excludes_original_media: true,
            excludes_watermarked_media: true,
            excludes_local_media_paths: true,
            included_fields: vec![
                "file_name",
                "watermark_uid",
                "creator_display_name",
                "revision",
                "hashes",
                "verification_status",
                "payload_registry",
                "protected_copy_metadata",
                "rights_declaration",
                "trusted_time_status",
                "video_notary_receipt",
                "video_fingerprint_bundle_metadata",
            ],
        },
        disclaimer: FORMAL_REPORT_DISCLAIMER.to_string(),
    }
}

fn report_record_from_vault(record: VaultRecord) -> FormalReportRecord {
    FormalReportRecord {
        record_id: record.id,
        file_name: record.file_name,
        watermark_uid: record.watermark_uid,
        creator_display_name: record.creator_display_name,
        original_hash: record.original_hash,
        resolution: record.resolution,
        duration_secs: record.duration_secs,
        created_at: record.created_at,
        revision: record.revision,
        parent_watermark_uid: record.parent_watermark_uid,
        rewrite_reason: record.rewrite_reason,
        write_verification_status: record.write_verification_status,
        write_verification_message: record.write_verification_message,
        write_verification_at: record.write_verification_at,
        payload_registry: FormalReportPayloadRegistry {
            payload_protocol_version: record.payload_protocol_version,
            payload_bytes_length: record.payload_bytes_length,
            media_payload_role: media_payload_role_for_protocol(record.payload_protocol_version)
                .to_string(),
            watermark_id_issue_mode: record.watermark_id_issue_mode,
            watermark_id_registry_status: record.watermark_id_registry_status,
            watermark_id_registry_receipt: record.watermark_id_registry_receipt,
            payload_auth_status: record.payload_auth_status,
        },
        protected_copy: FormalReportProtectedCopy {
            name: record.protected_copy_name,
            hash: record.protected_copy_hash,
            output_strategy: record.output_strategy,
        },
        trusted_time: FormalReportTrustedTime {
            network_time: record.network_time,
            tsa_source: record.tsa_source,
            tsa_token_present: record.tsa_token_path.is_some(),
        },
        rights_declaration: FormalReportRightsDeclaration {
            work_source_declaration: record.work_source_declaration,
            training_permission_declaration: record.training_permission_declaration,
            creation_method_declaration: record.creation_method_declaration,
            human_edit_level_declaration: record.human_edit_level_declaration,
            authenticity_claim_declaration: record.authenticity_claim_declaration,
            custom_rights_statement: record.custom_rights_statement,
        },
        video_notary: FormalReportVideoNotary {
            notary_id: record.video_notary_id,
            notary_at: record.video_notary_at,
            receipt_signature: record.video_notary_receipt_signature,
            usage_ledger_id: record.video_notary_usage_ledger_id,
            fingerprint_root: record.video_fingerprint_root,
            bundle_sha256: record.video_bundle_sha256,
            bundle_bytes: record.video_bundle_bytes,
            bundle_scene_count: record.video_bundle_scene_count,
            bundle_elapsed_ms: record.video_bundle_elapsed_ms,
            frame_sample_policy: record.video_frame_sample_policy,
        },
        video_visual_watermark: FormalReportVideoVisualWatermark {
            task_id: record.video_visual_task_id,
            completed_at: record.video_visual_completed_at,
            strategy_digest: record.video_visual_strategy_digest,
            self_check_confidence: record.video_visual_self_check_confidence,
            self_check_threshold: record.video_visual_self_check_threshold,
            checked_frames: record.video_visual_checked_frames,
            media_hash: record.video_visual_media_hash,
            receipt_hash: record.video_visual_receipt_hash,
            output_bytes: record.video_visual_output_bytes,
            output_content_type: record.video_visual_output_content_type,
        },
    }
}

fn export_report_files<R: Runtime>(
    app_handle: &AppHandle<R>,
    state: &AppState,
    document: FormalReportDocument,
) -> Result<FormalReportExportResult, String> {
    export_report_files_with_source(app_handle, state, document, None)
}

fn export_report_files_with_source<R: Runtime>(
    app_handle: &AppHandle<R>,
    state: &AppState,
    document: FormalReportDocument,
    import_source: Option<FormalReportImportSource>,
) -> Result<FormalReportExportResult, String> {
    let _export_guard = state
        .report_export_lock
        .lock()
        .map_err(|e| format!("正式报告导出锁失败: {e}"))?;
    let reports_dir = match std::env::var_os("HIDDENSHIELD_REPORT_OUTPUT_DIR") {
        Some(path) => std::path::PathBuf::from(path),
        None => app_handle
            .path()
            .app_data_dir()
            .map_err(|e| format!("failed to resolve app data directory: {e}"))?
            .join("reports"),
    };
    std::fs::create_dir_all(&reports_dir).map_err(|e| format!("创建报告目录失败: {e}"))?;

    let source_key = import_source
        .as_ref()
        .map(|source| {
            sha256_hex(format!("import_mobile_report_handoff|{}", source.source_key).as_bytes())
        })
        .unwrap_or_else(|| report_source_key(&document));
    let previous = find_previous_report_manifest(&reports_dir, &source_key)?;
    let bundle = FormalReportBundleLineage {
        source_key,
        bundle_version: previous
            .as_ref()
            .map(|manifest| manifest.bundle.bundle_version.saturating_add(1))
            .unwrap_or(1),
        supersedes_report_id: previous.map(|manifest| manifest.report_id),
        source_handoff_report_id: import_source
            .as_ref()
            .map(|source| source.report_id.clone()),
        source_handoff_source_key: import_source
            .as_ref()
            .map(|source| source.source_key.clone()),
        source_handoff_root_digest: import_source
            .as_ref()
            .map(|source| source.root_digest.clone()),
        source_handoff_platform: import_source.map(|source| source.platform),
    };
    let base_name = sanitize_file_name(&format!("{}-{}", document.report_type, document.report_id));
    let report_dir = reports_dir.join(base_name);
    std::fs::create_dir(&report_dir).map_err(|e| format!("创建报告包目录失败: {e}"))?;

    let result = export_report_bundle(state, app_handle, &report_dir, &document, &bundle);
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&report_dir);
    }
    result
}

fn prepare_mobile_report_handoff_import(
    report_dir: &std::path::Path,
) -> Result<(FormalReportDocument, FormalReportImportSource), String> {
    let verified = verify_report_bundle_at(report_dir)?;
    if verified.report_type.as_deref() != Some("formal_report_handoff")
        || verified.integrity_status != "matched"
        || verified.manifest_chain_status != "matched"
        || verified.document_contract_status != "matched"
    {
        return Err("移动报告交接包未通过完整性或文档合同校验".to_string());
    }
    let manifest: FormalReportManifest = serde_json::from_slice(
        &std::fs::read(report_dir.join("manifest.json"))
            .map_err(|e| format!("读取移动交接包 manifest.json 失败: {e}"))?,
    )
    .map_err(|e| format!("解析移动交接包 manifest.json 失败: {e}"))?;
    let handoff: MobileReportHandoffDocument = serde_json::from_slice(
        &std::fs::read(report_dir.join("report.json"))
            .map_err(|e| format!("读取移动交接包 report.json 失败: {e}"))?,
    )
    .map_err(|e| format!("解析移动交接包 report.json 失败: {e}"))?;
    if handoff.schema_version != 2
        || handoff.report_type != "formal_report_handoff"
        || handoff.report_id != manifest.report_id
        || handoff.exported_at != manifest.generated_at
        || handoff.handoff.status != "awaiting_desktop_render"
        || handoff.handoff.requested_output != ["report.pdf", "report.json", "manifest.json"]
    {
        return Err("移动报告交接包生成请求合同不匹配".to_string());
    }
    if handoff.records.is_empty() {
        return Err("移动报告交接包不包含版权记录".to_string());
    }
    let exported_at = Utc::now().to_rfc3339();
    let seed = format!(
        "{}|{}|{}",
        handoff.report_id, manifest.bundle.source_key, manifest.integrity.root_digest
    );
    let document = FormalReportDocument {
        schema_version: 2,
        report_id: stable_report_id("formal_report", &exported_at, &seed),
        report_type: "formal_report".to_string(),
        exported_at,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        records: handoff
            .records
            .into_iter()
            .map(formal_report_record_from_mobile_handoff)
            .collect(),
        privacy: FormalReportPrivacy {
            excludes_original_media: true,
            excludes_watermarked_media: true,
            excludes_local_media_paths: true,
            included_fields: vec![
                "file_name",
                "watermark_uid",
                "creator_display_name",
                "revision",
                "hashes",
                "verification_status",
                "payload_registry",
                "protected_copy_metadata",
                "rights_declaration",
                "trusted_time_status",
                "video_notary_receipt",
                "video_fingerprint_bundle_metadata",
            ],
        },
        disclaimer: FORMAL_REPORT_DISCLAIMER.to_string(),
    };
    Ok((
        document,
        FormalReportImportSource {
            source_key: manifest.bundle.source_key,
            report_id: manifest.report_id,
            root_digest: manifest.integrity.root_digest,
            platform: handoff.source_platform,
        },
    ))
}

fn formal_report_record_from_mobile_handoff(
    record: MobileReportHandoffRecord,
) -> FormalReportRecord {
    FormalReportRecord {
        record_id: stable_mobile_record_id(&record.source_record_id),
        file_name: record.file_name,
        watermark_uid: record.watermark_uid,
        creator_display_name: record.creator_display_name,
        original_hash: record.original_hash,
        resolution: "未记录（移动交接）".to_string(),
        duration_secs: 0.0,
        created_at: record.created_at,
        revision: record.revision,
        parent_watermark_uid: record.parent_watermark_uid,
        rewrite_reason: record.rewrite_reason,
        write_verification_status: record.write_verification_status,
        write_verification_message: record.write_verification_message,
        write_verification_at: record.write_verification_at,
        payload_registry: record.payload_registry,
        protected_copy: record.protected_copy,
        trusted_time: record.trusted_time,
        rights_declaration: record.rights_declaration,
        video_notary: record.video_notary,
        video_visual_watermark: record.video_visual_watermark,
    }
}

fn stable_mobile_record_id(source_record_id: &str) -> u32 {
    let digest = Sha256::digest(source_record_id.as_bytes());
    u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]).max(1)
}

fn export_report_bundle<R: Runtime>(
    state: &AppState,
    app_handle: &AppHandle<R>,
    report_dir: &std::path::Path,
    document: &FormalReportDocument,
    bundle: &FormalReportBundleLineage,
) -> Result<FormalReportExportResult, String> {
    let pdf_path = report_dir.join("report.pdf");
    let json_path = report_dir.join("report.json");
    let manifest_path = report_dir.join("manifest.json");
    let pdf_temp_path = report_dir.join(".report.pdf.tmp");
    let json_temp_path = report_dir.join(".report.json.tmp");
    let manifest_temp_path = report_dir.join(".manifest.json.tmp");

    let json = serde_json::to_vec_pretty(document)
        .map_err(|e| format!("生成 FormalReportDocument JSON 失败: {e}"))?;
    std::fs::write(&json_temp_path, &json)
        .map_err(|e| format!("写入报告 JSON 临时文件失败: {e}"))?;
    let document_value = serde_json::to_value(document)
        .map_err(|e| format!("序列化 Chromium 报告事实模型失败: {e}"))?;

    let pdf_result = state
        .report_pdf_worker
        .lock()
        .map_err(|e| format!("Chromium 报告 worker 锁失败: {e}"))?
        .render(app_handle, &document_value, &pdf_temp_path)?;
    verify_pdf_result(&pdf_temp_path, &pdf_result)?;

    let manifest = build_report_manifest(document, bundle, &json, &pdf_result);
    let manifest_json =
        serde_json::to_vec_pretty(&manifest).map_err(|e| format!("生成报告 Manifest 失败: {e}"))?;
    std::fs::write(&manifest_temp_path, manifest_json)
        .map_err(|e| format!("写入报告 Manifest 临时文件失败: {e}"))?;

    std::fs::rename(&json_temp_path, &json_path)
        .map_err(|e| format!("提交 report.json 失败: {e}"))?;
    std::fs::rename(&pdf_temp_path, &pdf_path).map_err(|e| format!("提交 report.pdf 失败: {e}"))?;
    std::fs::rename(&manifest_temp_path, &manifest_path)
        .map_err(|e| format!("提交 manifest.json 失败: {e}"))?;

    Ok(FormalReportExportResult {
        report_id: document.report_id.clone(),
        report_type: document.report_type.clone(),
        report_dir: report_dir.to_string_lossy().to_string(),
        pdf_path: pdf_path.to_string_lossy().to_string(),
        json_path: json_path.to_string_lossy().to_string(),
        manifest_path: manifest_path.to_string_lossy().to_string(),
        exported_at: document.exported_at.clone(),
        record_count: document.records.len(),
        pdf_generation_ms: pdf_result.generation_ms,
        pdf_page_count: pdf_result.page_count,
        bundle_version: bundle.bundle_version,
        supersedes_report_id: bundle.supersedes_report_id.clone(),
    })
}

fn build_report_manifest(
    document: &FormalReportDocument,
    bundle: &FormalReportBundleLineage,
    report_json: &[u8],
    pdf_result: &ReportPdfRenderResult,
) -> FormalReportManifest {
    let files = vec![
        FormalReportManifestFile {
            path: "report.pdf".to_string(),
            media_type: "application/pdf".to_string(),
            bytes: pdf_result.bytes,
            sha256: pdf_result.sha256.clone(),
        },
        FormalReportManifestFile {
            path: "report.json".to_string(),
            media_type: "application/json".to_string(),
            bytes: report_json.len() as u64,
            sha256: sha256_hex(report_json),
        },
    ];
    FormalReportManifest {
        schema_version: 2,
        report_id: document.report_id.clone(),
        report_type: document.report_type.clone(),
        generated_at: document.exported_at.clone(),
        source_schema_version: document.schema_version,
        bundle: bundle.clone(),
        renderer: FormalReportRenderer {
            engine: "chromium".to_string(),
            worker_mode: "persistent_warm_worker".to_string(),
            template_version: "R1.1".to_string(),
            controlled_fonts: vec![
                "NotoSansSC-Controlled.ttf".to_string(),
                "NotoSerifSC-Controlled.ttf".to_string(),
            ],
            generation_ms: pdf_result.generation_ms,
            generation_budget_ms: REPORT_PDF_GENERATION_BUDGET_MS,
            page_count: pdf_result.page_count,
            pagination_stable: !pdf_result.page_overflow.iter().any(|page| page.overflow),
        },
        integrity: build_integrity_chain(&files),
        files,
        signature: FormalReportSignatureStatus {
            status: "not_signed".to_string(),
            profile: None,
            signer_key_id: None,
            certificate_chain_status: "not_evaluated".to_string(),
            revocation_status: "not_applicable".to_string(),
            signed_at: None,
            note: "当前仅支持离线完整性校验；PDF/CMS/PAdES 数字签名尚未接入。".to_string(),
        },
        trusted_time: FormalReportTrustedTimeStatus {
            status: "not_verified".to_string(),
            package_timestamp_present: false,
            record_material_token_present: document
                .records
                .iter()
                .any(|record| record.trusted_time.tsa_token_present),
            note: "记录级 TSA 材料状态不等于报告包已获得可信时间戳。".to_string(),
        },
        verification: FormalReportVerificationContract {
            offline_mode: "sha256_chain_v1".to_string(),
            online_status: "not_deployed".to_string(),
            qr_status: "not_issued".to_string(),
            online_verification_url: None,
        },
    }
}

fn verify_pdf_result(
    pdf_path: &std::path::Path,
    result: &ReportPdfRenderResult,
) -> Result<(), String> {
    let bytes = std::fs::read(pdf_path).map_err(|e| format!("读取 Chromium PDF 结果失败: {e}"))?;
    if bytes.len() as u64 != result.bytes {
        return Err(format!(
            "Chromium PDF 大小校验失败: worker={}, actual={}",
            result.bytes,
            bytes.len()
        ));
    }
    let actual_sha256 = sha256_hex(&bytes);
    if actual_sha256 != result.sha256 {
        return Err("Chromium PDF SHA-256 校验失败".to_string());
    }
    Ok(())
}

fn verify_report_bundle_at(
    report_dir: &std::path::Path,
) -> Result<FormalReportBundleVerificationResult, String> {
    let report_dir = std::fs::canonicalize(report_dir)
        .map_err(|e| format!("报告包目录不存在或不可访问: {e}"))?;
    let manifest_path = report_dir.join("manifest.json");
    let manifest_bytes =
        std::fs::read(&manifest_path).map_err(|e| format!("读取 manifest.json 失败: {e}"))?;
    let manifest: FormalReportManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("解析 manifest.json 失败: {e}"))?;
    if manifest.schema_version != 2 {
        return Err(format!(
            "不支持的 Manifest schema: {}，当前要求 schema v2",
            manifest.schema_version
        ));
    }
    verify_manifest_file_contract(&manifest)?;

    let manifest_chain_valid = verify_integrity_chain(&manifest.files, &manifest.integrity);
    let mut files = Vec::with_capacity(manifest.files.len());
    for expected in &manifest.files {
        files.push(verify_manifest_file(&report_dir, expected)?);
    }
    let files_match = files.iter().all(|file| file.status == "matched");
    let document_contract_valid = verify_report_document_contract(&report_dir, &manifest)?;
    let integrity_status = if files_match && manifest_chain_valid {
        "matched"
    } else {
        "mismatch"
    };
    let signature_status = match manifest.signature.status.as_str() {
        "not_signed" => "not_signed",
        _ => "present_unverified",
    };
    let trusted_time_status = if manifest.trusted_time.package_timestamp_present {
        "present_unverified"
    } else {
        "not_timestamped"
    };
    let message = if integrity_status == "matched"
        && document_contract_valid
        && manifest.report_type == "formal_report_handoff"
    {
        "移动报告交接包文件、Manifest 摘要链与 report.json 合同匹配；该目录尚未生成 PDF，也未完成包级数字签名或包级时间戳。"
    } else if integrity_status == "matched" && document_contract_valid {
        "报告包文件、Manifest 摘要链与 report.json 合同匹配；当前报告包未签名、未获得包级时间戳，不能据此判断签名主体或时间可信性。"
    } else if integrity_status == "matched" {
        "报告包文件摘要匹配，但 report.json 文档合同不匹配。"
    } else {
        "报告包完整性校验失败；至少一个文件摘要、大小或 Manifest 摘要链不匹配。"
    };

    Ok(FormalReportBundleVerificationResult {
        report_id: Some(manifest.report_id),
        report_type: Some(manifest.report_type),
        report_dir: report_dir.to_string_lossy().to_string(),
        verified_at: Utc::now().to_rfc3339(),
        manifest_schema_version: Some(manifest.schema_version),
        bundle_version: Some(manifest.bundle.bundle_version),
        supersedes_report_id: manifest.bundle.supersedes_report_id,
        integrity_status: integrity_status.to_string(),
        manifest_chain_status: if manifest_chain_valid {
            "matched".to_string()
        } else {
            "mismatch".to_string()
        },
        document_contract_status: if document_contract_valid {
            "matched".to_string()
        } else {
            "mismatch".to_string()
        },
        signature_status: signature_status.to_string(),
        trusted_time_status: trusted_time_status.to_string(),
        files,
        message: message.to_string(),
    })
}

fn verify_rights_evidence_pack_at(
    case_dir: &std::path::Path,
) -> Result<RightsEvidencePackVerificationResult, String> {
    if !case_dir.is_dir() {
        return Err("案件包路径不是目录".to_string());
    }
    let case_dir =
        std::fs::canonicalize(case_dir).map_err(|error| format!("解析案件包目录失败: {error}"))?;
    let case_json_path = case_dir.join("case.json");
    let manifest_path = case_dir.join("case-manifest.json");
    let case_bytes =
        std::fs::read(&case_json_path).map_err(|error| format!("读取 case.json 失败: {error}"))?;
    let manifest_bytes = std::fs::read(&manifest_path)
        .map_err(|error| format!("读取 case-manifest.json 失败: {error}"))?;
    let case_document: RightsEvidencePackCaseDocument = serde_json::from_slice(&case_bytes)
        .map_err(|error| format!("解析 case.json 失败: {error}"))?;
    let manifest: RightsEvidencePackManifest = serde_json::from_slice(&manifest_bytes)
        .map_err(|error| format!("解析 case-manifest.json 失败: {error}"))?;

    let expected_top_level = vec![
        "attachments".to_string(),
        "case-manifest.json".to_string(),
        "case.json".to_string(),
    ];
    let mut actual_top_level = std::fs::read_dir(&case_dir)
        .map_err(|error| format!("读取案件包顶层目录失败: {error}"))?
        .map(|entry| {
            entry
                .map_err(|error| format!("读取案件包顶层入口失败: {error}"))
                .and_then(|entry| {
                    entry
                        .file_name()
                        .into_string()
                        .map_err(|_| "案件包包含非 UTF-8 顶层入口".to_string())
                })
        })
        .collect::<Result<Vec<_>, _>>()?;
    actual_top_level.sort();

    let case_path_safe = is_regular_non_symlink_file(&case_json_path);
    let manifest_path_safe = is_regular_non_symlink_file(&manifest_path);
    let attachments_root = case_dir.join("attachments");
    let (mut physical_attachment_paths, attachment_tree_safe) =
        collect_rights_evidence_attachment_files(&attachments_root, &case_dir)?;
    physical_attachment_paths.sort();

    let mut expected_attachment_paths = case_document
        .attachments
        .iter()
        .map(|attachment| attachment.relative_path.clone())
        .collect::<Vec<_>>();
    expected_attachment_paths.sort();
    let mut manifest_attachment_paths = manifest
        .files
        .iter()
        .map(|attachment| attachment.path.clone())
        .collect::<Vec<_>>();
    manifest_attachment_paths.sort();

    let attachment_metadata_valid =
        verify_rights_evidence_attachment_metadata(&case_document, &manifest);
    let attachments = manifest
        .files
        .iter()
        .map(|attachment| verify_rights_evidence_attachment(&case_dir, attachment))
        .collect::<Result<Vec<_>, _>>()?;
    let attachment_files_match = attachments
        .iter()
        .all(|attachment| attachment.status == "matched");
    let attachment_integrity_valid = attachment_metadata_valid
        && attachment_files_match
        && attachment_tree_safe
        && physical_attachment_paths == expected_attachment_paths
        && physical_attachment_paths == manifest_attachment_paths;

    let expected_event_chain = build_rights_evidence_event_chain(&case_document.collection_events)?;
    let event_chain_valid = expected_event_chain == manifest.event_chain;
    let expected_attachment_chain =
        build_rights_evidence_attachment_chain(&case_document.attachments)?;
    let attachment_chain_valid = expected_attachment_chain == manifest.attachment_chain;
    let computed_root_digest = sha256_hex(
        format!(
            "HiddenShield-Rights-Evidence-Pack-Root-v1\n{}\n{}\n{}",
            sha256_hex(&case_bytes),
            expected_event_chain.root_digest,
            expected_attachment_chain.root_digest
        )
        .as_bytes(),
    );

    let mut allowed_top_level = manifest
        .directory_contract
        .allowed_top_level_entries
        .clone();
    allowed_top_level.sort();
    let directory_contract_valid = manifest.schema_version == 1
        && manifest.manifest_type == "rights_evidence_pack_manifest"
        && case_document.schema_version == 1
        && case_document.document_type == "rights_evidence_pack"
        && manifest.pack_id == case_document.pack_id
        && manifest.case_id == case_document.case.case_id
        && manifest.directory_contract.case_document == "case.json"
        && manifest.directory_contract.manifest == "case-manifest.json"
        && manifest.directory_contract.attachment_root == "attachments/"
        && allowed_top_level == expected_top_level
        && actual_top_level == expected_top_level
        && case_path_safe
        && manifest_path_safe
        && manifest.case_file.path == "case.json"
        && manifest.case_file.bytes == case_bytes.len() as u64
        && manifest.case_file.sha256 == sha256_hex(&case_bytes)
        && manifest.integrity.algorithm == "sha256_case_event_attachment_roots_v1"
        && manifest.integrity.root_digest == computed_root_digest
        && attachment_tree_safe
        && physical_attachment_paths == manifest_attachment_paths;

    let signature_status = if manifest.signature.status == "not_signed" {
        "not_signed"
    } else {
        "present_unverified"
    };
    let trusted_time_status = if manifest.trusted_time.status == "not_timestamped" {
        "not_timestamped"
    } else {
        "present_unverified"
    };
    let all_integrity_matched = directory_contract_valid
        && attachment_integrity_valid
        && event_chain_valid
        && attachment_chain_valid;
    let message = if all_integrity_matched {
        "案件包目录、附件、采集事件链和附件链匹配；当前案件包未签名，也未获得包级时间戳。"
    } else {
        "案件包至少一项目录或完整性校验不匹配；请保留原目录并核对逐项状态。"
    };

    Ok(RightsEvidencePackVerificationResult {
        pack_id: Some(case_document.pack_id),
        case_id: Some(case_document.case.case_id),
        case_dir: case_dir.to_string_lossy().to_string(),
        verified_at: Utc::now().to_rfc3339(),
        manifest_schema_version: Some(manifest.schema_version),
        directory_contract_status: verification_status(directory_contract_valid),
        attachment_integrity_status: verification_status(attachment_integrity_valid),
        event_chain_status: verification_status(event_chain_valid),
        attachment_chain_status: verification_status(attachment_chain_valid),
        signature_status: signature_status.to_string(),
        trusted_time_status: trusted_time_status.to_string(),
        declared_root_digest: Some(manifest.integrity.root_digest),
        computed_root_digest: Some(computed_root_digest),
        attachments,
        message: message.to_string(),
    })
}

fn verify_rights_evidence_attachment_metadata(
    case_document: &RightsEvidencePackCaseDocument,
    manifest: &RightsEvidencePackManifest,
) -> bool {
    if case_document.attachments.len() != manifest.files.len() {
        return false;
    }
    let allowed_roles = ["original", "working_copy", "capture", "external_receipt"];
    let attachment_ids = case_document
        .attachments
        .iter()
        .map(|attachment| attachment.attachment_id.as_str())
        .collect::<std::collections::HashSet<_>>();
    if attachment_ids.len() != case_document.attachments.len() {
        return false;
    }
    let attachment_paths = case_document
        .attachments
        .iter()
        .map(|attachment| attachment.relative_path.as_str())
        .collect::<std::collections::HashSet<_>>();
    if attachment_paths.len() != case_document.attachments.len() {
        return false;
    }
    if case_document
        .automated_findings
        .iter()
        .flat_map(|finding| &finding.input_attachment_ids)
        .any(|attachment_id| !attachment_ids.contains(attachment_id.as_str()))
    {
        return false;
    }
    for (index, (attachment, manifest_file)) in case_document
        .attachments
        .iter()
        .zip(&manifest.files)
        .enumerate()
    {
        if attachment.sequence != index + 1
            || !allowed_roles.contains(&attachment.role.as_str())
            || !is_safe_rights_evidence_attachment_path(&attachment.relative_path)
            || attachment.attachment_id != manifest_file.attachment_id
            || attachment.relative_path != manifest_file.path
            || attachment.role != manifest_file.role
            || attachment.bytes != manifest_file.bytes
            || attachment.sha256 != manifest_file.sha256
        {
            return false;
        }
        if attachment.role == "working_copy" {
            let Some(source_id) = attachment.derived_from_attachment_id.as_deref() else {
                return false;
            };
            let Some(source) = case_document
                .attachments
                .iter()
                .find(|candidate| candidate.attachment_id == source_id)
            else {
                return false;
            };
            if source.role != "original" || source.sequence >= attachment.sequence {
                return false;
            }
        } else if attachment.derived_from_attachment_id.is_some() {
            return false;
        }
    }
    true
}

fn verify_rights_evidence_attachment(
    case_dir: &std::path::Path,
    expected: &RightsEvidencePackManifestAttachment,
) -> Result<RightsEvidencePackVerifiedAttachment, String> {
    if !is_safe_rights_evidence_attachment_path(&expected.path) {
        return Ok(rights_evidence_attachment_result(
            expected,
            None,
            None,
            "unsafe_path",
        ));
    }
    let relative_path = std::path::Path::new(&expected.path);
    let file_path = case_dir.join(relative_path);
    if !file_path.exists() {
        return Ok(rights_evidence_attachment_result(
            expected, None, None, "missing",
        ));
    }
    if path_contains_symlink(case_dir, relative_path)? || !file_path.is_file() {
        return Ok(rights_evidence_attachment_result(
            expected,
            None,
            None,
            "unsafe_path",
        ));
    }
    let canonical_file = std::fs::canonicalize(&file_path)
        .map_err(|error| format!("解析案件附件路径失败 {}: {error}", expected.path))?;
    if !canonical_file.starts_with(case_dir) {
        return Ok(rights_evidence_attachment_result(
            expected,
            None,
            None,
            "unsafe_path",
        ));
    }
    let bytes = std::fs::read(&canonical_file)
        .map_err(|error| format!("读取案件附件失败 {}: {error}", expected.path))?;
    let actual_bytes = bytes.len() as u64;
    let actual_sha256 = sha256_hex(&bytes);
    let status = if actual_bytes == expected.bytes && actual_sha256 == expected.sha256 {
        "matched"
    } else {
        "mismatch"
    };
    Ok(rights_evidence_attachment_result(
        expected,
        Some(actual_bytes),
        Some(actual_sha256),
        status,
    ))
}

fn rights_evidence_attachment_result(
    expected: &RightsEvidencePackManifestAttachment,
    actual_bytes: Option<u64>,
    actual_sha256: Option<String>,
    status: &str,
) -> RightsEvidencePackVerifiedAttachment {
    RightsEvidencePackVerifiedAttachment {
        attachment_id: expected.attachment_id.clone(),
        path: expected.path.clone(),
        role: expected.role.clone(),
        expected_bytes: expected.bytes,
        actual_bytes,
        expected_sha256: expected.sha256.clone(),
        actual_sha256,
        status: status.to_string(),
    }
}

fn build_rights_evidence_event_chain(
    events: &[serde_json::Value],
) -> Result<RightsEvidencePackEventChain, String> {
    let genesis = "HiddenShield-Rights-Evidence-Pack-Event-Chain-v1".to_string();
    let mut previous_chain_digest = sha256_hex(genesis.as_bytes());
    let mut entries = Vec::with_capacity(events.len());
    for (index, event) in events.iter().enumerate() {
        let sequence = index + 1;
        let event_sequence = event
            .get("sequence")
            .and_then(serde_json::Value::as_u64)
            .and_then(|value| usize::try_from(value).ok());
        let event_id = event
            .get("eventId")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("采集事件缺少 eventId: sequence={sequence}"))?;
        if event_sequence != Some(sequence) {
            return Err(format!("采集事件 sequence 不连续: eventId={event_id}"));
        }
        let event_digest = sha256_hex(stable_json_string(event)?.as_bytes());
        let chain_digest = sha256_hex(
            format!("{sequence}\n{event_id}\n{event_digest}\n{previous_chain_digest}").as_bytes(),
        );
        entries.push(RightsEvidencePackEventChainEntry {
            sequence,
            event_id: event_id.to_string(),
            event_digest,
            previous_chain_digest: previous_chain_digest.clone(),
            chain_digest: chain_digest.clone(),
        });
        previous_chain_digest = chain_digest;
    }
    Ok(RightsEvidencePackEventChain {
        algorithm: "sha256_append_chain_v1".to_string(),
        genesis,
        entries,
        root_digest: previous_chain_digest,
    })
}

fn build_rights_evidence_attachment_chain(
    attachments: &[RightsEvidencePackCaseAttachment],
) -> Result<RightsEvidencePackAttachmentChain, String> {
    let genesis = "HiddenShield-Rights-Evidence-Pack-Attachment-Chain-v1".to_string();
    let mut previous_chain_digest = sha256_hex(genesis.as_bytes());
    let mut entries = Vec::with_capacity(attachments.len());
    for (index, attachment) in attachments.iter().enumerate() {
        let sequence = index + 1;
        if attachment.sequence != sequence {
            return Err(format!(
                "附件 sequence 不连续: attachmentId={}",
                attachment.attachment_id
            ));
        }
        let chain_digest = sha256_hex(
            format!(
                "{sequence}\n{}\n{}\n{}\n{}\n{}\n{previous_chain_digest}",
                attachment.attachment_id,
                attachment.relative_path,
                attachment.role,
                attachment.bytes,
                attachment.sha256
            )
            .as_bytes(),
        );
        entries.push(RightsEvidencePackAttachmentChainEntry {
            sequence,
            attachment_id: attachment.attachment_id.clone(),
            path: attachment.relative_path.clone(),
            role: attachment.role.clone(),
            file_bytes: attachment.bytes,
            file_sha256: attachment.sha256.clone(),
            previous_chain_digest: previous_chain_digest.clone(),
            chain_digest: chain_digest.clone(),
        });
        previous_chain_digest = chain_digest;
    }
    Ok(RightsEvidencePackAttachmentChain {
        algorithm: "sha256_append_chain_v1".to_string(),
        genesis,
        entries,
        root_digest: previous_chain_digest,
    })
}

fn stable_json_string(value: &serde_json::Value) -> Result<String, String> {
    match value {
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {
            serde_json::to_string(value).map_err(|error| format!("序列化事件字段失败: {error}"))
        }
        serde_json::Value::Array(items) => {
            let items = items
                .iter()
                .map(stable_json_string)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(format!("[{}]", items.join(",")))
        }
        serde_json::Value::Object(object) => {
            let mut keys = object.keys().collect::<Vec<_>>();
            keys.sort();
            let entries = keys
                .into_iter()
                .map(|key| {
                    let serialized_key = serde_json::to_string(key)
                        .map_err(|error| format!("序列化事件键失败: {error}"))?;
                    let serialized_value = stable_json_string(&object[key])?;
                    Ok(format!("{serialized_key}:{serialized_value}"))
                })
                .collect::<Result<Vec<_>, String>>()?;
            Ok(format!("{{{}}}", entries.join(",")))
        }
    }
}

fn collect_rights_evidence_attachment_files(
    attachments_root: &std::path::Path,
    case_dir: &std::path::Path,
) -> Result<(Vec<String>, bool), String> {
    let Ok(root_metadata) = std::fs::symlink_metadata(attachments_root) else {
        return Ok((Vec::new(), false));
    };
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Ok((Vec::new(), false));
    }
    let mut files = Vec::new();
    let mut safe = true;
    collect_rights_evidence_attachment_files_recursive(
        attachments_root,
        case_dir,
        &mut files,
        &mut safe,
    )?;
    Ok((files, safe))
}

fn collect_rights_evidence_attachment_files_recursive(
    directory: &std::path::Path,
    case_dir: &std::path::Path,
    files: &mut Vec<String>,
    safe: &mut bool,
) -> Result<(), String> {
    for entry in std::fs::read_dir(directory)
        .map_err(|error| format!("读取附件目录失败 {}: {error}", directory.display()))?
    {
        let entry = entry.map_err(|error| format!("读取附件目录入口失败: {error}"))?;
        let path = entry.path();
        let metadata = std::fs::symlink_metadata(&path)
            .map_err(|error| format!("读取附件元数据失败 {}: {error}", path.display()))?;
        if metadata.file_type().is_symlink() {
            *safe = false;
            continue;
        }
        if metadata.is_dir() {
            collect_rights_evidence_attachment_files_recursive(&path, case_dir, files, safe)?;
        } else if metadata.is_file() {
            let relative_path = path
                .strip_prefix(case_dir)
                .map_err(|_| format!("附件路径不在案件包内: {}", path.display()))?;
            files.push(relative_path.to_string_lossy().replace('\\', "/"));
        } else {
            *safe = false;
        }
    }
    Ok(())
}

fn is_safe_rights_evidence_attachment_path(value: &str) -> bool {
    if value.contains('\\') {
        return false;
    }
    let components = std::path::Path::new(value).components().collect::<Vec<_>>();
    components.len() >= 3
        && matches!(
            components.first(),
            Some(std::path::Component::Normal(component)) if *component == std::ffi::OsStr::new("attachments")
        )
        && components
            .iter()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

fn path_contains_symlink(
    case_dir: &std::path::Path,
    relative_path: &std::path::Path,
) -> Result<bool, String> {
    let mut current = case_dir.to_path_buf();
    for component in relative_path.components() {
        let std::path::Component::Normal(component) = component else {
            return Ok(true);
        };
        current.push(component);
        let metadata = match std::fs::symlink_metadata(&current) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
            Err(error) => {
                return Err(format!(
                    "读取案件包路径元数据失败 {}: {error}",
                    current.display()
                ))
            }
        };
        if metadata.file_type().is_symlink() {
            return Ok(true);
        }
    }
    Ok(false)
}

fn is_regular_non_symlink_file(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && !metadata.file_type().is_symlink())
}

fn verification_status(matched: bool) -> String {
    if matched { "matched" } else { "mismatch" }.to_string()
}

fn verify_manifest_file_contract(manifest: &FormalReportManifest) -> Result<(), String> {
    let expected_paths: &[&str] = if manifest.report_type == "formal_report_handoff" {
        &["report.json"]
    } else {
        &["report.pdf", "report.json"]
    };
    let mut actual_paths = manifest
        .files
        .iter()
        .map(|file| file.path.as_str())
        .collect::<Vec<_>>();
    actual_paths.sort_unstable();
    let mut expected_paths = expected_paths.to_vec();
    expected_paths.sort_unstable();
    if actual_paths != expected_paths {
        return Err(format!(
            "Manifest 文件合同不匹配: reportType={} expected={expected_paths:?} actual={actual_paths:?}",
            manifest.report_type
        ));
    }
    if manifest.report_type == "formal_report_handoff"
        && (manifest.renderer.engine != "mobile_handoff"
            || manifest.renderer.worker_mode != "not_rendered"
            || manifest.renderer.page_count != 0)
    {
        return Err("移动交接包 renderer 合同不匹配".to_string());
    }
    Ok(())
}

fn verify_report_document_contract(
    report_dir: &std::path::Path,
    manifest: &FormalReportManifest,
) -> Result<bool, String> {
    let report_json_path = report_dir.join("report.json");
    let report_json =
        std::fs::read(&report_json_path).map_err(|e| format!("读取 report.json 失败: {e}"))?;
    let document: serde_json::Value =
        serde_json::from_slice(&report_json).map_err(|e| format!("解析 report.json 失败: {e}"))?;
    Ok(document
        .get("schemaVersion")
        .and_then(|value| value.as_u64())
        == Some(2)
        && document.get("reportId").and_then(|value| value.as_str())
            == Some(manifest.report_id.as_str())
        && document.get("reportType").and_then(|value| value.as_str())
            == Some(manifest.report_type.as_str()))
}

fn verify_manifest_file(
    report_dir: &std::path::Path,
    expected: &FormalReportManifestFile,
) -> Result<FormalReportVerifiedFile, String> {
    let relative_path = std::path::Path::new(&expected.path);
    let is_safe_relative_path = relative_path.components().count() == 1
        && matches!(
            relative_path.components().next(),
            Some(std::path::Component::Normal(_))
        );
    if !is_safe_relative_path {
        return Ok(FormalReportVerifiedFile {
            path: expected.path.clone(),
            expected_bytes: expected.bytes,
            actual_bytes: None,
            expected_sha256: expected.sha256.clone(),
            actual_sha256: None,
            status: "unsafe_path".to_string(),
        });
    }

    let file_path = report_dir.join(relative_path);
    if !file_path.is_file() {
        return Ok(FormalReportVerifiedFile {
            path: expected.path.clone(),
            expected_bytes: expected.bytes,
            actual_bytes: None,
            expected_sha256: expected.sha256.clone(),
            actual_sha256: None,
            status: "missing".to_string(),
        });
    }
    let canonical_file = std::fs::canonicalize(&file_path)
        .map_err(|e| format!("解析报告文件路径失败 {}: {e}", expected.path))?;
    if !canonical_file.starts_with(report_dir) {
        return Ok(FormalReportVerifiedFile {
            path: expected.path.clone(),
            expected_bytes: expected.bytes,
            actual_bytes: None,
            expected_sha256: expected.sha256.clone(),
            actual_sha256: None,
            status: "unsafe_path".to_string(),
        });
    }

    let bytes = std::fs::read(&canonical_file)
        .map_err(|e| format!("读取报告文件失败 {}: {e}", expected.path))?;
    let actual_sha256 = sha256_hex(&bytes);
    let actual_bytes = bytes.len() as u64;
    let status = if actual_bytes == expected.bytes && actual_sha256 == expected.sha256 {
        "matched"
    } else {
        "mismatch"
    };
    Ok(FormalReportVerifiedFile {
        path: expected.path.clone(),
        expected_bytes: expected.bytes,
        actual_bytes: Some(actual_bytes),
        expected_sha256: expected.sha256.clone(),
        actual_sha256: Some(actual_sha256),
        status: status.to_string(),
    })
}

fn build_integrity_chain(files: &[FormalReportManifestFile]) -> FormalReportIntegrityChain {
    let genesis = "HiddenShield-Report-Manifest-v2".to_string();
    let mut previous_chain_digest = sha256_hex(genesis.as_bytes());
    let mut entries = Vec::with_capacity(files.len());
    for (index, file) in files.iter().enumerate() {
        let sequence = index + 1;
        let chain_digest = integrity_entry_digest(sequence, file, &previous_chain_digest);
        entries.push(FormalReportIntegrityEntry {
            sequence,
            path: file.path.clone(),
            file_sha256: file.sha256.clone(),
            file_bytes: file.bytes,
            previous_chain_digest: previous_chain_digest.clone(),
            chain_digest: chain_digest.clone(),
        });
        previous_chain_digest = chain_digest;
    }
    FormalReportIntegrityChain {
        algorithm: "sha256_chain_v1".to_string(),
        genesis,
        entries,
        root_digest: previous_chain_digest,
    }
}

fn verify_integrity_chain(
    files: &[FormalReportManifestFile],
    integrity: &FormalReportIntegrityChain,
) -> bool {
    if integrity.algorithm != "sha256_chain_v1" || integrity.entries.len() != files.len() {
        return false;
    }
    let mut previous_chain_digest = sha256_hex(integrity.genesis.as_bytes());
    for (index, (file, entry)) in files.iter().zip(&integrity.entries).enumerate() {
        let sequence = index + 1;
        if entry.sequence != sequence
            || entry.path != file.path
            || entry.file_sha256 != file.sha256
            || entry.file_bytes != file.bytes
            || entry.previous_chain_digest != previous_chain_digest
        {
            return false;
        }
        let expected_digest = integrity_entry_digest(sequence, file, &previous_chain_digest);
        if entry.chain_digest != expected_digest {
            return false;
        }
        previous_chain_digest = expected_digest;
    }
    integrity.root_digest == previous_chain_digest
}

fn integrity_entry_digest(
    sequence: usize,
    file: &FormalReportManifestFile,
    previous_chain_digest: &str,
) -> String {
    sha256_hex(
        format!(
            "{sequence}\n{}\n{}\n{}\n{previous_chain_digest}",
            file.path, file.bytes, file.sha256
        )
        .as_bytes(),
    )
}

fn report_source_key(document: &FormalReportDocument) -> String {
    let mut record_ids = document
        .records
        .iter()
        .map(|record| record.record_id)
        .collect::<Vec<_>>();
    record_ids.sort_unstable();
    let ids = record_ids
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    sha256_hex(format!("{}|{ids}", document.report_type).as_bytes())
}

fn find_previous_report_manifest(
    reports_dir: &std::path::Path,
    source_key: &str,
) -> Result<Option<FormalReportManifest>, String> {
    let entries =
        std::fs::read_dir(reports_dir).map_err(|e| format!("读取历史报告目录失败: {e}"))?;
    let mut latest: Option<FormalReportManifest> = None;
    for entry in entries {
        let Ok(entry) = entry else {
            continue;
        };
        let manifest_path = entry.path().join("manifest.json");
        let Ok(bytes) = std::fs::read(manifest_path) else {
            continue;
        };
        let Ok(manifest) = serde_json::from_slice::<FormalReportManifest>(&bytes) else {
            continue;
        };
        if manifest.schema_version != 2 || manifest.bundle.source_key != source_key {
            continue;
        }
        let should_replace = latest
            .as_ref()
            .map(|current| manifest.bundle.bundle_version > current.bundle.bundle_version)
            .unwrap_or(true);
        if should_replace {
            latest = Some(manifest);
        }
    }
    Ok(latest)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

#[cfg(test)]
fn render_markdown_report(document: &FormalReportDocument) -> String {
    let mut lines = vec![
        "# HiddenShield 正式版权报告".to_string(),
        String::new(),
        format!("- 报告编号: {}", document.report_id),
        format!("- 报告类型: {}", report_type_label(&document.report_type)),
        format!("- 导出时间: {}", document.exported_at),
        format!("- 记录数量: {}", document.records.len()),
        String::new(),
        "## 隐私边界".to_string(),
        String::new(),
        "- 不包含原始媒体文件".to_string(),
        "- 不包含加水印后的媒体文件".to_string(),
        "- 不包含本地媒体文件路径".to_string(),
        String::new(),
    ];

    for (index, record) in document.records.iter().enumerate() {
        lines.extend([
            format!("## 记录 {}", index + 1),
            String::new(),
            format!("- 文件名: {}", record.file_name),
            format!("- 版权编号: {}", record.watermark_uid),
            optional_line("- 创作者身份", record.creator_display_name.as_deref()),
            format!("- 版本次数: 第 {} 次", record.revision),
            optional_line("- 上一版编号", record.parent_watermark_uid.as_deref()),
            optional_line("- 更新说明", record.rewrite_reason.as_deref()),
            format!("- 作品指纹: {}", record.original_hash),
            format!("- 分辨率: {}", record.resolution),
            format!("- 时长: {:.2} 秒", record.duration_secs),
            format!("- 入库时间: {}", record.created_at),
            optional_line("- 完成后验证", record.write_verification_status.as_deref()),
            optional_line("- 验证说明", record.write_verification_message.as_deref()),
            optional_line("- 验证时间", record.write_verification_at.as_deref()),
            format!(
                "- Payload 协议: V{} / {} bytes",
                record.payload_registry.payload_protocol_version,
                record.payload_registry.payload_bytes_length
            ),
            format!(
                "- 媒体载荷角色: {}",
                media_payload_role_label(&record.payload_registry.media_payload_role)
            ),
            format!(
                "- 编号签发模式: {}",
                watermark_issue_mode_label(&record.payload_registry.watermark_id_issue_mode)
            ),
            format!(
                "- 登记状态: {}",
                registry_status_label(&record.payload_registry.watermark_id_registry_status)
            ),
            optional_line(
                "- 登记收据",
                record
                    .payload_registry
                    .watermark_id_registry_receipt
                    .as_deref(),
            ),
            format!(
                "- Payload 认证状态: {}",
                payload_auth_status_label(&record.payload_registry.payload_auth_status)
            ),
            optional_line("- 保护副本名称", record.protected_copy.name.as_deref()),
            optional_line("- 保护副本摘要", record.protected_copy.hash.as_deref()),
            format!(
                "- 输出策略: {}",
                output_strategy_label(&record.protected_copy.output_strategy)
            ),
            format!(
                "- 作品来源声明: {}",
                work_source_declaration_label(&record.rights_declaration.work_source_declaration)
            ),
            format!(
                "- 训练许可声明: {}",
                training_permission_label(
                    &record.rights_declaration.training_permission_declaration
                )
            ),
            optional_line(
                "- 创作方式声明",
                Some(&record.rights_declaration.creation_method_declaration),
            ),
            optional_line(
                "- 人工编辑声明",
                Some(&record.rights_declaration.human_edit_level_declaration),
            ),
            format!(
                "- 真实性声明: {}",
                authenticity_claim_label(&record.rights_declaration.authenticity_claim_declaration)
            ),
            optional_line(
                "- 自定义版权声明",
                record.rights_declaration.custom_rights_statement.as_deref(),
            ),
            optional_line("- 网络授时", record.trusted_time.network_time.as_deref()),
            optional_line("- 时间回执来源", record.trusted_time.tsa_source.as_deref()),
            format!(
                "- 时间回执文件: {}",
                if record.trusted_time.tsa_token_present {
                    "已记录"
                } else {
                    "未记录"
                }
            ),
            String::new(),
        ]);
        if record.video_notary.has_notary() {
            lines.extend([
                "### 视频指纹存证".to_string(),
                String::new(),
                optional_line("- 存证编号", record.video_notary.notary_id.as_deref()),
                optional_line("- 存证时间", record.video_notary.notary_at.as_deref()),
                optional_line(
                    "- 收据签名",
                    record.video_notary.receipt_signature.as_deref(),
                ),
                optional_line("- 用量流水", record.video_notary.usage_ledger_id.as_deref()),
                optional_line("- 指纹根", record.video_notary.fingerprint_root.as_deref()),
                optional_line("- 指纹包摘要", record.video_notary.bundle_sha256.as_deref()),
                optional_u64_line("- 指纹包大小", record.video_notary.bundle_bytes),
                optional_u32_line("- 采样帧", record.video_notary.bundle_scene_count),
                optional_elapsed_line("- 生成耗时", record.video_notary.bundle_elapsed_ms),
                optional_line(
                    "- 采样策略",
                    record.video_notary.frame_sample_policy.as_deref(),
                ),
                String::new(),
            ]);
        }
        if record.video_visual_watermark.has_receipt() {
            lines.extend([
                "### L3 视频画面盲水印".to_string(),
                String::new(),
                optional_line(
                    "- 任务编号",
                    record.video_visual_watermark.task_id.as_deref(),
                ),
                optional_line(
                    "- 完成时间",
                    record.video_visual_watermark.completed_at.as_deref(),
                ),
                optional_line(
                    "- 策略摘要",
                    record.video_visual_watermark.strategy_digest.as_deref(),
                ),
                optional_f64_line(
                    "- 自检置信度",
                    record.video_visual_watermark.self_check_confidence,
                ),
                optional_f64_line(
                    "- 自检阈值",
                    record.video_visual_watermark.self_check_threshold,
                ),
                optional_u32_line("- 检查帧数", record.video_visual_watermark.checked_frames),
                optional_line(
                    "- 成品媒体摘要",
                    record.video_visual_watermark.media_hash.as_deref(),
                ),
                optional_line(
                    "- Worker 收据摘要",
                    record.video_visual_watermark.receipt_hash.as_deref(),
                ),
                optional_u64_line("- 成品字节数", record.video_visual_watermark.output_bytes),
                optional_line(
                    "- 成品内容类型",
                    record.video_visual_watermark.output_content_type.as_deref(),
                ),
                String::new(),
            ]);
        }
    }

    lines.extend([
        "## 免责声明".to_string(),
        String::new(),
        document.disclaimer.clone(),
        String::new(),
    ]);
    lines.join("\n")
}

#[cfg(test)]
fn optional_line(label: &str, value: Option<&str>) -> String {
    value
        .filter(|value| !value.trim().is_empty())
        .map(|value| format!("{label}: {value}"))
        .unwrap_or_else(|| format!("{label}: 无"))
}

#[cfg(test)]
fn output_strategy_label(value: &str) -> &'static str {
    match value {
        "minimal_required_change" | "" => "最小必要变更",
        _ => "自定义策略",
    }
}

#[cfg(test)]
fn work_source_declaration_label(value: &str) -> &'static str {
    match value {
        "human_created" => "人工创作",
        "ai_assisted" => "AI 辅助",
        "ai_generated" => "AI 生成",
        _ => "未声明",
    }
}

#[cfg(test)]
fn training_permission_label(value: &str) -> &'static str {
    match value {
        "separate_authorization_required" => "需单独授权",
        "non_commercial_allowed" => "允许非商业训练",
        "commercial_allowed" => "允许商业训练",
        "unspecified" => "未声明",
        _ => "禁止模型训练",
    }
}

#[cfg(test)]
fn watermark_issue_mode_label(value: &str) -> &'static str {
    match value {
        "server_reserved" => "后端预签发",
        "server_confirmed" => "后端已确认",
        "server_reissued" => "后端重新签发",
        "registry_resolved" => "registry 解析",
        _ => "本地离线生成",
    }
}

#[cfg(test)]
fn registry_status_label(value: &str) -> &'static str {
    match value {
        "reserved" => "已预留，等待写入确认",
        "server_confirmed" => "后端已确认",
        "offline_confirmed" => "离线编号已补登记",
        "conflict" => "编号冲突",
        "reissue_required" => "需要重新签发",
        _ => "等待联网登记",
    }
}

#[cfg(test)]
fn payload_auth_status_label(value: &str) -> &'static str {
    match value {
        "verified" => "已验证",
        "failed" => "验证失败",
        _ => "未验证",
    }
}

fn media_payload_role_for_protocol(protocol_version: u32) -> &'static str {
    if protocol_version >= 3 {
        "v3_minimal_anchor"
    } else {
        "v2_full_record"
    }
}

#[cfg(test)]
fn media_payload_role_label(value: &str) -> &'static str {
    match value {
        "v3_minimal_anchor" => "V3 最小锚点",
        "v2_full_record" => "V2 完整载荷",
        _ => "未记录",
    }
}

#[cfg(test)]
fn authenticity_claim_label(value: &str) -> &'static str {
    match value {
        "synthetic" => "虚构或合成",
        "based_on_reality" => "基于真实",
        "creator_claimed_authentic" | "authentic" => "创作者声明真实",
        _ => "未声明",
    }
}

#[cfg(test)]
fn optional_u64_line(label: &str, value: Option<u64>) -> String {
    value
        .map(|value| format!("{label}: {value}"))
        .unwrap_or_else(|| format!("{label}: 无"))
}

#[cfg(test)]
fn optional_u32_line(label: &str, value: Option<u32>) -> String {
    value
        .map(|value| format!("{label}: {value}"))
        .unwrap_or_else(|| format!("{label}: 无"))
}

#[cfg(test)]
fn optional_f64_line(label: &str, value: Option<f64>) -> String {
    value
        .map(|value| format!("{label}: {:.6}", value))
        .unwrap_or_else(|| format!("{label}: 未记录"))
}

#[cfg(test)]
fn optional_elapsed_line(label: &str, value: Option<u64>) -> String {
    value
        .map(|value| format!("{label}: {:.1} 秒", value as f64 / 1000.0))
        .unwrap_or_else(|| format!("{label}: 无"))
}

fn record_report_usage(
    state: &AppState,
    vault_record_id: Option<i64>,
    file_size_bytes: u64,
) -> Result<(), String> {
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    let entitlement = entitlements::resolve_effective_entitlement(
        &conn,
        state.installation_secret_store.as_ref(),
    )
    .map_err(|e| format!("读取权益状态失败: {e}"))?;
    let mut entry = billing::UsageLedgerEntry::success(
        "report_export",
        "report",
        file_size_bytes,
        &entitlement,
        None,
    );
    entry.vault_record_id = vault_record_id;
    billing::append_usage_entry(&conn, &entry).map_err(|e| format!("写入报告用量失败: {e}"))
}

fn exported_size(result: &FormalReportExportResult) -> Result<u64, String> {
    let pdf = std::fs::metadata(&result.pdf_path)
        .map_err(|e| format!("读取 PDF 报告大小失败: {e}"))?
        .len();
    let json = std::fs::metadata(&result.json_path)
        .map_err(|e| format!("读取 JSON 报告大小失败: {e}"))?
        .len();
    let manifest = std::fs::metadata(&result.manifest_path)
        .map_err(|e| format!("读取报告 Manifest 大小失败: {e}"))?
        .len();
    Ok(pdf + json + manifest)
}

fn stable_report_id(report_type: &str, exported_at: &str, seed: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(report_type.as_bytes());
    hasher.update(exported_at.as_bytes());
    hasher.update(seed.as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("hsr-{}", &digest[..16])
}

#[cfg(test)]
fn report_type_label(report_type: &str) -> &'static str {
    match report_type {
        "batch_summary" => "批量摘要",
        _ => "单条正式报告",
    }
}

fn sanitize_file_name(value: &str) -> String {
    value
        .chars()
        .map(|ch| match ch {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => ch,
            _ => '-',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn sample_record() -> VaultRecord {
        VaultRecord {
            id: 7,
            original_hash: "original-hash".to_string(),
            file_name: "demo.png".to_string(),
            created_at: "2026-06-18T12:00:00Z".to_string(),
            duration_secs: 0.0,
            resolution: "1024x768".to_string(),
            watermark_uid: "uid-demo".to_string(),
            creator_display_name: Some("测试创作者".to_string()),
            thumbnail_path: Some("D:\\media\\thumb.png".to_string()),
            output_douyin: Some("D:\\media\\douyin.png".to_string()),
            output_bilibili: Some("D:\\media\\bilibili.png".to_string()),
            output_xhs: None,
            is_hdr_source: false,
            hw_encoder_used: None,
            process_time_ms: Some(1200),
            tsa_token_path: Some("D:\\tokens\\uid-demo.tsr".to_string()),
            network_time: Some("2026-06-18T12:00:01Z".to_string()),
            tsa_source: Some("tsa.example".to_string()),
            tsa_request_nonce: Some("nonce".to_string()),
            is_ai_generated: false,
            ai_training_permission: None,
            ai_generation_method: None,
            human_modification_level: None,
            authenticity_claim: None,
            custom_metadata: None,
            output_douyin_hash: Some("hash-douyin".to_string()),
            output_bilibili_hash: None,
            output_xhs_hash: None,
            protected_copy_name: Some("demo_protected.png".to_string()),
            protected_copy_path: Some("D:\\media\\demo_protected.png".to_string()),
            protected_copy_hash: Some("hash-protected".to_string()),
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
            write_verification_status: Some("verified".to_string()),
            write_verification_message: Some("完成后验证已通过".to_string()),
            write_verification_at: Some("2026-06-18T12:01:00Z".to_string()),
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
        }
    }

    #[test]
    fn formal_report_omits_local_media_paths() {
        let document = build_report_document("formal_report", vec![sample_record()]);
        let json = serde_json::to_string(&document).unwrap();
        let markdown = render_markdown_report(&document);
        assert!(!json.contains("D:\\media"));
        assert!(!json.contains("D:\\tokens"));
        assert!(!markdown.contains("D:\\media"));
        assert!(!markdown.contains("D:\\tokens"));
        assert!(json.contains("\"excludesLocalMediaPaths\":true"));
        assert!(markdown.contains("不包含本地媒体文件路径"));
    }

    #[test]
    fn formal_report_marks_v2_and_v3_payload_roles() {
        let mut v2 = sample_record();
        v2.watermark_uid = "uid-v2".to_string();
        let mut v3 = sample_record();
        v3.watermark_uid = "uid-v3".to_string();
        v3.payload_protocol_version = 3;
        v3.payload_bytes_length = 39;
        v3.watermark_id_issue_mode = "registry_resolved".to_string();

        let document = build_report_document("batch_summary", vec![v2, v3]);
        let json = serde_json::to_string(&document).unwrap();
        let markdown = render_markdown_report(&document);

        assert!(json.contains("\"mediaPayloadRole\":\"v2_full_record\""));
        assert!(json.contains("\"mediaPayloadRole\":\"v3_minimal_anchor\""));
        assert!(markdown.contains("媒体载荷角色: V2 完整载荷"));
        assert!(markdown.contains("媒体载荷角色: V3 最小锚点"));
        assert!(markdown.contains("Payload 协议: V3 / 39 bytes"));
    }

    #[test]
    fn formal_report_includes_video_notary_without_media_paths() {
        let mut record = sample_record();
        record.file_name = "demo-video.mp4".to_string();
        record.video_notary_id = Some("vfn_123".to_string());
        record.video_notary_at = Some("2026-06-19T08:00:00Z".to_string());
        record.video_notary_receipt_signature = Some("sig_abc".to_string());
        record.video_notary_usage_ledger_id = Some("usage_123".to_string());
        record.video_fingerprint_root = Some("sha256:fingerprint-root".to_string());
        record.video_bundle_sha256 = Some("sha256:bundle".to_string());
        record.video_bundle_bytes = Some(4096);
        record.video_bundle_scene_count = Some(8);
        record.video_bundle_elapsed_ms = Some(1234);
        record.video_frame_sample_policy = Some("8 evenly spaced frames".to_string());

        let document = build_report_document("formal_report", vec![record]);
        let json = serde_json::to_string(&document).unwrap();
        let markdown = render_markdown_report(&document);

        assert!(json.contains("\"videoNotary\""));
        assert!(json.contains("\"notaryId\":\"vfn_123\""));
        assert!(json.contains("\"fingerprintRoot\":\"sha256:fingerprint-root\""));
        assert!(json.contains("\"bundleSha256\":\"sha256:bundle\""));
        assert!(markdown.contains("### 视频指纹存证"));
        assert!(markdown.contains("- 存证编号: vfn_123"));
        assert!(markdown.contains("- 指纹根: sha256:fingerprint-root"));
        assert!(markdown.contains("- 指纹包摘要: sha256:bundle"));
        assert!(!json.contains("D:\\media"));
        assert!(!json.contains("D:\\tokens"));
        assert!(!markdown.contains("D:\\media"));
        assert!(!markdown.contains("D:\\tokens"));
    }

    #[test]
    fn formal_report_includes_l3_video_visual_receipt_without_paths_or_urls() {
        let mut record = sample_record();
        record.file_name = "demo-l3.mp4".to_string();
        record.resolution = "1024x1024".to_string();
        record.video_visual_task_id = Some("l3task_abc".to_string());
        record.video_visual_completed_at = Some("2026-07-01T10:00:00Z".to_string());
        record.video_visual_strategy_digest = Some("sha256:strategy".to_string());
        record.video_visual_self_check_confidence = Some(1.0);
        record.video_visual_self_check_threshold = Some(0.9);
        record.video_visual_checked_frames = Some(4);
        record.video_visual_media_hash = Some("sha256:media".to_string());
        record.video_visual_receipt_hash = Some("sha256:receipt".to_string());
        record.video_visual_output_bytes = Some(123456);
        record.video_visual_output_content_type = Some("video/mp4".to_string());

        let document = build_report_document("formal_report", vec![record]);
        let json = serde_json::to_string(&document).unwrap();
        let markdown = render_markdown_report(&document);

        assert!(json.contains("\"videoVisualWatermark\""));
        assert!(json.contains("\"taskId\":\"l3task_abc\""));
        assert!(json.contains("\"mediaHash\":\"sha256:media\""));
        assert!(json.contains("\"receiptHash\":\"sha256:receipt\""));
        assert!(markdown.contains("### L3 视频画面盲水印"));
        assert!(markdown.contains("- 任务编号: l3task_abc"));
        assert!(markdown.contains("- 成品媒体摘要: sha256:media"));
        assert!(!json.contains("object://"));
        assert!(!json.contains("output-download"));
        assert!(!json.contains("D:\\media"));
        assert!(!markdown.contains("object://"));
        assert!(!markdown.contains("output-download"));
        assert!(!markdown.contains("D:\\media"));
    }

    #[test]
    fn report_export_requires_entitlement_feature() {
        let mut entitlement = billing::EntitlementState::default();
        assert!(ensure_report_export_entitled(&entitlement).is_err());
        entitlement.features = BTreeMap::from([("report_export".to_string(), true)]);
        assert!(ensure_report_export_entitled(&entitlement).is_ok());
    }

    #[test]
    fn manifest_hashes_pdf_and_report_json_from_same_document() {
        let document = build_report_document("formal_report", vec![sample_record()]);
        let report_json = serde_json::to_vec_pretty(&document).unwrap();
        let pdf_result = ReportPdfRenderResult {
            generation_ms: 742.5,
            page_count: 4,
            bytes: 7,
            sha256: sha256_hex(b"%PDF-R1"),
            page_overflow: vec![],
            font_state: crate::report_pdf::ReportPdfFontState {
                sans_loaded: true,
                serif_loaded: true,
            },
        };
        let bundle = FormalReportBundleLineage {
            source_key: report_source_key(&document),
            bundle_version: 1,
            supersedes_report_id: None,
            source_handoff_report_id: None,
            source_handoff_source_key: None,
            source_handoff_root_digest: None,
            source_handoff_platform: None,
        };

        let manifest = build_report_manifest(&document, &bundle, &report_json, &pdf_result);

        assert_eq!(manifest.schema_version, 2);
        assert_eq!(manifest.source_schema_version, 2);
        assert_eq!(manifest.bundle.bundle_version, 1);
        assert_eq!(manifest.renderer.worker_mode, "persistent_warm_worker");
        assert_eq!(manifest.renderer.generation_budget_ms, 3_000);
        assert_eq!(manifest.files[0].path, "report.pdf");
        assert_eq!(manifest.files[0].sha256, pdf_result.sha256);
        assert_eq!(manifest.files[1].path, "report.json");
        assert_eq!(manifest.files[1].sha256, sha256_hex(&report_json));
        assert!(verify_integrity_chain(&manifest.files, &manifest.integrity));
        assert_eq!(manifest.signature.status, "not_signed");
        assert_eq!(manifest.signature.revocation_status, "not_applicable");
        assert_eq!(manifest.trusted_time.status, "not_verified");
        assert_eq!(manifest.verification.qr_status, "not_issued");
    }

    #[test]
    fn report_bundle_verification_detects_file_tampering() {
        let document = build_report_document("formal_report", vec![sample_record()]);
        let report_json = serde_json::to_vec_pretty(&document).unwrap();
        let pdf_bytes = b"%PDF-R2";
        let pdf_result = ReportPdfRenderResult {
            generation_ms: 520.0,
            page_count: 4,
            bytes: pdf_bytes.len() as u64,
            sha256: sha256_hex(pdf_bytes),
            page_overflow: vec![],
            font_state: crate::report_pdf::ReportPdfFontState {
                sans_loaded: true,
                serif_loaded: true,
            },
        };
        let bundle = FormalReportBundleLineage {
            source_key: report_source_key(&document),
            bundle_version: 2,
            supersedes_report_id: Some("hsr-previous".to_string()),
            source_handoff_report_id: None,
            source_handoff_source_key: None,
            source_handoff_root_digest: None,
            source_handoff_platform: None,
        };
        let manifest = build_report_manifest(&document, &bundle, &report_json, &pdf_result);
        let report_dir = std::env::temp_dir().join(format!(
            "hidden-shield-report-r2-{}",
            stable_report_id("test", &Utc::now().to_rfc3339(), "tamper")
        ));
        std::fs::create_dir(&report_dir).unwrap();
        std::fs::write(report_dir.join("report.pdf"), pdf_bytes).unwrap();
        std::fs::write(report_dir.join("report.json"), &report_json).unwrap();
        std::fs::write(
            report_dir.join("manifest.json"),
            serde_json::to_vec_pretty(&manifest).unwrap(),
        )
        .unwrap();

        let matched = verify_report_bundle_at(&report_dir).unwrap();
        assert_eq!(matched.integrity_status, "matched");
        assert_eq!(matched.document_contract_status, "matched");
        assert_eq!(matched.signature_status, "not_signed");
        assert_eq!(matched.trusted_time_status, "not_timestamped");
        assert_eq!(matched.bundle_version, Some(2));
        assert_eq!(
            matched.supersedes_report_id.as_deref(),
            Some("hsr-previous")
        );

        std::fs::write(report_dir.join("report.pdf"), b"%PDF-TAMPERED").unwrap();
        let tampered = verify_report_bundle_at(&report_dir).unwrap();
        assert_eq!(tampered.integrity_status, "mismatch");
        assert_eq!(tampered.files[0].status, "mismatch");

        std::fs::remove_dir_all(report_dir).unwrap();
    }

    #[test]
    fn desktop_verifies_mobile_generated_report_handoff_fixture() {
        let report_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../mobile_app/test/fixtures/report_handoff_r3/mobile-image");

        let verified = verify_report_bundle_at(&report_dir).unwrap();

        assert_eq!(
            verified.report_type.as_deref(),
            Some("formal_report_handoff")
        );
        assert_eq!(verified.integrity_status, "matched");
        assert_eq!(verified.manifest_chain_status, "matched");
        assert_eq!(verified.document_contract_status, "matched");
        assert_eq!(verified.signature_status, "not_signed");
        assert_eq!(verified.trusted_time_status, "not_timestamped");
        assert_eq!(verified.files.len(), 1);
        assert_eq!(verified.files[0].path, "report.json");
        assert!(verified.message.contains("尚未生成 PDF"));
    }

    #[test]
    fn mobile_report_handoff_rejects_pdf_placeholders() {
        let manifest_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../mobile_app/test/fixtures/report_handoff_r3/mobile-image/manifest.json");
        let mut manifest: FormalReportManifest =
            serde_json::from_slice(&std::fs::read(manifest_path).unwrap()).unwrap();
        manifest.files.push(FormalReportManifestFile {
            path: "report.pdf".to_string(),
            media_type: "application/pdf".to_string(),
            bytes: 0,
            sha256: sha256_hex(&[]),
        });

        let error = verify_manifest_file_contract(&manifest).unwrap_err();

        assert!(error.contains("Manifest 文件合同不匹配"));
    }

    #[test]
    fn prepares_mobile_handoff_for_chromium_render() {
        let report_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../mobile_app/test/fixtures/report_handoff_r3/mobile-image");
        let source_manifest: FormalReportManifest =
            serde_json::from_slice(&std::fs::read(report_dir.join("manifest.json")).unwrap())
                .unwrap();

        let (document, source) = prepare_mobile_report_handoff_import(&report_dir).unwrap();

        assert_eq!(document.schema_version, 2);
        assert_eq!(document.report_type, "formal_report");
        assert_ne!(document.report_id, source_manifest.report_id);
        assert_eq!(document.records.len(), 1);
        assert_eq!(
            document.records[0].watermark_uid,
            "HS-MOBILE-R3-IMAGE-000401"
        );
        assert_eq!(document.records[0].resolution, "未记录（移动交接）");
        assert_eq!(source.report_id, source_manifest.report_id);
        assert_eq!(source.source_key, source_manifest.bundle.source_key);
        assert_eq!(source.root_digest, source_manifest.integrity.root_digest);
        assert_eq!(source.platform, "flutter_mobile");
    }

    #[test]
    fn imported_manifest_records_mobile_handoff_root_digest() {
        let report_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../mobile_app/test/fixtures/report_handoff_r3/mobile-image");
        let (document, source) = prepare_mobile_report_handoff_import(&report_dir).unwrap();
        let report_json = serde_json::to_vec_pretty(&document).unwrap();
        let pdf_result = ReportPdfRenderResult {
            generation_ms: 680.0,
            page_count: 4,
            bytes: 11,
            sha256: sha256_hex(b"%PDF-IMPORT"),
            page_overflow: vec![],
            font_state: crate::report_pdf::ReportPdfFontState {
                sans_loaded: true,
                serif_loaded: true,
            },
        };
        let bundle = FormalReportBundleLineage {
            source_key: sha256_hex(
                format!("import_mobile_report_handoff|{}", source.source_key).as_bytes(),
            ),
            bundle_version: 1,
            supersedes_report_id: None,
            source_handoff_report_id: Some(source.report_id.clone()),
            source_handoff_source_key: Some(source.source_key.clone()),
            source_handoff_root_digest: Some(source.root_digest.clone()),
            source_handoff_platform: Some(source.platform.clone()),
        };

        let manifest = build_report_manifest(&document, &bundle, &report_json, &pdf_result);

        assert_eq!(
            manifest.bundle.source_handoff_root_digest.as_deref(),
            Some(source.root_digest.as_str())
        );
        assert_eq!(
            manifest.bundle.source_handoff_report_id.as_deref(),
            Some(source.report_id.as_str())
        );
        assert_eq!(
            manifest.bundle.source_handoff_platform.as_deref(),
            Some("flutter_mobile")
        );
        assert_eq!(manifest.report_type, "formal_report");
        assert_eq!(manifest.renderer.worker_mode, "persistent_warm_worker");
    }

    #[test]
    fn mobile_handoff_import_rejects_tampered_report_json() {
        let fixture_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../mobile_app/test/fixtures/report_handoff_r3/mobile-image");
        let report_dir = std::env::temp_dir().join(format!(
            "hidden-shield-mobile-handoff-tamper-{}",
            stable_mobile_record_id(&Utc::now().to_rfc3339())
        ));
        std::fs::create_dir(&report_dir).unwrap();
        std::fs::copy(
            fixture_dir.join("manifest.json"),
            report_dir.join("manifest.json"),
        )
        .unwrap();
        std::fs::write(
            report_dir.join("report.json"),
            br#"{"schemaVersion":2,"reportId":"tampered","reportType":"formal_report_handoff"}"#,
        )
        .unwrap();

        let error = prepare_mobile_report_handoff_import(&report_dir).unwrap_err();

        assert!(error.contains("未通过完整性或文档合同校验"));
        std::fs::remove_dir_all(report_dir).unwrap();
    }

    #[test]
    fn verifies_rights_evidence_pack_fixture_with_six_independent_statuses() {
        let case_dir = rights_evidence_pack_fixture_dir();

        let verified = verify_rights_evidence_pack_at(&case_dir).unwrap();

        assert_eq!(verified.pack_id.as_deref(), Some("hsep-fixture-r4-0001"));
        assert_eq!(verified.case_id.as_deref(), Some("case-fixture-r4-0001"));
        assert_eq!(verified.directory_contract_status, "matched");
        assert_eq!(verified.attachment_integrity_status, "matched");
        assert_eq!(verified.event_chain_status, "matched");
        assert_eq!(verified.attachment_chain_status, "matched");
        assert_eq!(verified.signature_status, "not_signed");
        assert_eq!(verified.trusted_time_status, "not_timestamped");
        assert_eq!(verified.declared_root_digest, verified.computed_root_digest);
        assert_eq!(verified.attachments.len(), 4);
        assert!(verified
            .attachments
            .iter()
            .all(|attachment| attachment.status == "matched"));
    }

    #[test]
    fn rights_evidence_pack_attachment_tamper_only_breaks_attachment_integrity() {
        let temp_dir = tempfile::tempdir().unwrap();
        let case_dir = temp_dir.path().join("case");
        copy_test_directory(&rights_evidence_pack_fixture_dir(), &case_dir);
        let capture_path = case_dir
            .join("attachments")
            .join("capture")
            .join("ATT-03-disputed-page-capture.txt");
        std::fs::write(capture_path, b"tampered capture bytes").unwrap();

        let verified = verify_rights_evidence_pack_at(&case_dir).unwrap();

        assert_eq!(verified.directory_contract_status, "matched");
        assert_eq!(verified.attachment_integrity_status, "mismatch");
        assert_eq!(verified.event_chain_status, "matched");
        assert_eq!(verified.attachment_chain_status, "matched");
        assert_eq!(
            verified
                .attachments
                .iter()
                .find(|attachment| attachment.role == "capture")
                .map(|attachment| attachment.status.as_str()),
            Some("mismatch")
        );
    }

    #[test]
    fn rights_evidence_pack_event_tamper_breaks_directory_and_event_chain() {
        let temp_dir = tempfile::tempdir().unwrap();
        let case_dir = temp_dir.path().join("case");
        copy_test_directory(&rights_evidence_pack_fixture_dir(), &case_dir);
        let case_path = case_dir.join("case.json");
        let mut document: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&case_path).unwrap()).unwrap();
        document["collectionEvents"][0]["description"] =
            serde_json::Value::String("tampered event".to_string());
        let mut bytes = serde_json::to_vec_pretty(&document).unwrap();
        bytes.push(b'\n');
        std::fs::write(case_path, bytes).unwrap();

        let verified = verify_rights_evidence_pack_at(&case_dir).unwrap();

        assert_eq!(verified.directory_contract_status, "mismatch");
        assert_eq!(verified.attachment_integrity_status, "matched");
        assert_eq!(verified.event_chain_status, "mismatch");
        assert_eq!(verified.attachment_chain_status, "matched");
        assert_ne!(verified.declared_root_digest, verified.computed_root_digest);
    }

    #[test]
    fn rights_evidence_pack_rejects_unregistered_attachment_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let case_dir = temp_dir.path().join("case");
        copy_test_directory(&rights_evidence_pack_fixture_dir(), &case_dir);
        std::fs::write(
            case_dir
                .join("attachments")
                .join("capture")
                .join("UNREGISTERED.txt"),
            b"unregistered",
        )
        .unwrap();

        let verified = verify_rights_evidence_pack_at(&case_dir).unwrap();

        assert_eq!(verified.directory_contract_status, "mismatch");
        assert_eq!(verified.attachment_integrity_status, "mismatch");
        assert_eq!(verified.event_chain_status, "matched");
        assert_eq!(verified.attachment_chain_status, "matched");
    }

    fn rights_evidence_pack_fixture_dir() -> std::path::PathBuf {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../docs/fixtures/rights-evidence-pack-r4/case-fixture-r4-0001")
    }

    fn copy_test_directory(source: &std::path::Path, target: &std::path::Path) {
        std::fs::create_dir_all(target).unwrap();
        for entry in std::fs::read_dir(source).unwrap() {
            let entry = entry.unwrap();
            let source_path = entry.path();
            let target_path = target.join(entry.file_name());
            if entry.file_type().unwrap().is_dir() {
                copy_test_directory(&source_path, &target_path);
            } else {
                std::fs::copy(source_path, target_path).unwrap();
            }
        }
    }
}

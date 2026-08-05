use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};

use crate::commands::vault::VaultRecord;
use crate::db::queries;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueAccountRequest {
    pub identifier: String,
    #[serde(default)]
    pub challenge_id: Option<String>,
    pub password: String,
    pub verification_code: String,
    pub device: ContinueAccountDevice,
    pub local_creator_profile: ContinueAccountCreatorProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueAccountDevice {
    pub client_device_id: String,
    pub name: String,
    pub platform: String,
    pub app_version: String,
    pub public_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueAccountCreatorProfile {
    pub display_name: String,
    pub creator_seed_ref: String,
    pub seed_envelope_version: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthChallengeRequest {
    pub identifier: String,
    pub purpose: String,
    pub client_device_id: String,
    #[serde(default)]
    pub captcha_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthChallengeResponse {
    pub challenge_id: String,
    pub delivery_channel: String,
    pub expires_at: String,
    pub message: String,
    #[serde(default)]
    pub fixture_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAccountSession {
    pub access_token: String,
    pub refresh_token: String,
    pub account: CloudAccount,
    pub workspace: CloudWorkspace,
    pub device: CloudDevice,
    pub creator_profile: CloudCreatorProfile,
    pub entitlement: CloudEntitlement,
    #[serde(default)]
    pub sync_policy: String,
    #[serde(default)]
    pub cloud_vault_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAccountSnapshot {
    pub account: CloudAccount,
    pub workspace: CloudWorkspace,
    pub device: CloudDevice,
    pub creator_profile: CloudCreatorProfile,
    pub entitlement: CloudEntitlement,
    #[serde(default)]
    pub sync_policy: String,
    #[serde(default)]
    pub cloud_vault_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudAccount {
    pub id: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudWorkspace {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudDevice {
    pub id: String,
    pub name: Option<String>,
    pub platform: Option<String>,
    pub registered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudCreatorProfile {
    pub id: String,
    pub display_name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudEntitlement {
    pub id: String,
    pub plan_name: Option<String>,
    pub plan_code: String,
    #[serde(default)]
    pub plan_key: String,
    #[serde(default)]
    pub plan_label: String,
    pub status: String,
    pub features: serde_json::Value,
}

impl CloudEntitlement {
    fn normalized_plan_key(&self) -> String {
        if !self.plan_key.trim().is_empty() {
            return self.plan_key.clone();
        }
        entitlement_plan_presentation(&self.plan_code, &self.features)
            .0
            .to_string()
    }

    fn normalized_plan_label(&self) -> String {
        if !self.plan_label.trim().is_empty() {
            return self.plan_label.clone();
        }
        entitlement_plan_presentation(&self.plan_code, &self.features)
            .1
            .to_string()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncBatchResult {
    pub accepted: u32,
    pub accepted_event_ids: Vec<String>,
    pub next_cursor: Option<String>,
    pub resolutions: serde_json::Value,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_results: Option<Vec<CloudSyncEventDisposition>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncEventDisposition {
    pub client_event_id: String,
    pub disposition: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload_hash: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entity_revision: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncChange {
    pub cursor: Option<String>,
    pub entity_type: String,
    pub operation: String,
    pub source_device: Option<String>,
    pub entity: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncChangesResult {
    pub next_cursor: String,
    pub changes: Vec<CloudSyncChange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPreferencesResponse {
    pub sync_policy: String,
    pub auto_sync_enabled: bool,
    pub cloud_vault_cursor: Option<String>,
    pub entitlement: CloudEntitlement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDevice {
    pub id: String,
    pub client_device_id: String,
    pub name: String,
    pub platform: String,
    pub app_version: String,
    pub registered: bool,
    pub auto_sync_enabled: bool,
    pub is_current: bool,
    pub active_session_count: u32,
    pub last_seen_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDevicesResponse {
    pub devices: Vec<AccountDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RevokeDeviceResponse {
    pub ok: bool,
    pub device_id: String,
    pub revoked_session_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFingerprintNotaryRequest {
    pub schema_version: String,
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub watermark_uid: String,
    pub source_hash: String,
    pub duration_ms: u64,
    pub frame_sample_policy: String,
    pub scene_count: u32,
    pub fingerprint_schema_version: String,
    pub global_frame_fingerprints: Vec<VideoGlobalFrameFingerprint>,
    pub local_block_fingerprint_root: String,
    pub local_block_count: u32,
    pub crop_window_fingerprint_root: String,
    pub crop_window_count: u32,
    pub fingerprint_root: String,
    pub client_signature: String,
    pub upload_manifest: VideoUploadManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoGlobalFrameFingerprint {
    pub scene_index: u32,
    pub timestamp_ms: u64,
    pub phash: String,
    pub color_hash: String,
    pub edge_hash: String,
    pub motion_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoUploadManifest {
    pub schema_version: String,
    pub contains_original_video: bool,
    pub contains_watermarked_video: bool,
    pub contains_local_paths: bool,
    pub contains_proxy: bool,
    pub items: Vec<VideoUploadManifestItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoUploadManifestItem {
    pub kind: String,
    pub sha256: String,
    pub bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sandbox_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transcode_profile: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_count: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFingerprintNotaryReceipt {
    pub schema_version: String,
    pub notary_id: String,
    pub watermark_uid: String,
    pub source_hash: String,
    pub fingerprint_root: String,
    pub notarized_at: String,
    pub server_receipt_signature: String,
    pub usage_ledger_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskRecord {
    pub task_id: String,
    pub schema_version: String,
    pub account_id: String,
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub capability_level: String,
    pub watermark_uid: String,
    pub source_hash: String,
    pub duration_ms: u64,
    pub target_profiles: Vec<String>,
    pub upload_manifest: VideoUploadManifest,
    pub status: String,
    pub quota_units: u64,
    pub failure_code: Option<String>,
    pub strategy_digest: Option<String>,
    pub self_check_threshold: Option<f64>,
    pub self_check_confidence: Option<f64>,
    pub checked_frames: Option<u32>,
    pub watermarked_media_hash: Option<String>,
    pub output_media_storage_ref: Option<String>,
    pub output_media_bytes: Option<u64>,
    pub output_media_content_type: Option<String>,
    pub worker_receipt_hash: Option<String>,
    pub worker_receipt: Option<serde_json::Value>,
    pub server_receipt_signature: Option<String>,
    pub usage_ledger_id: Option<String>,
    pub worker_id: Option<String>,
    pub attempt_id: Option<String>,
    pub attempt_count: u32,
    pub lease_expires_at: Option<String>,
    pub last_failure_code: Option<String>,
    pub last_failure_stage: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskRequest {
    pub schema_version: String,
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub capability_level: String,
    pub watermark_uid: String,
    pub source_hash: String,
    pub duration_ms: u64,
    pub target_profiles: Vec<String>,
    pub upload_manifest: VideoUploadManifest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskObjectUploadAuthorizationRequest {
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub sha256: String,
    pub bytes: u64,
    pub content_type: String,
    #[serde(default)]
    pub object_kind: Option<String>,
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskObjectUploadAuthorizationResponse {
    pub schema_version: String,
    pub authorization_id: String,
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub storage_ref: String,
    pub expected_sha256: String,
    pub expected_bytes: u64,
    pub content_type: String,
    pub expires_at: String,
    pub signed_upload_url: String,
    pub upload_method: String,
    pub upload_token: String,
    pub privacy_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskObjectUploadResponse {
    pub schema_version: String,
    pub status: String,
    pub storage_ref: String,
    pub sha256: String,
    pub bytes: u64,
    pub content_type: String,
    pub privacy_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskDownloadAuthorizationRequest {
    #[serde(default)]
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskDownloadAuthorizationResponse {
    pub schema_version: String,
    pub authorization_id: String,
    pub task_id: String,
    pub status: String,
    pub output_media_storage_ref: String,
    pub output_media_bytes: u64,
    pub output_media_content_type: String,
    pub watermarked_media_hash: String,
    pub worker_receipt_hash: String,
    pub expires_at: String,
    pub signed_download_url: String,
    pub download_method: String,
    pub download_token: String,
    pub privacy_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFingerprintBundleForNotary {
    pub schema_version: String,
    pub watermark_uid: String,
    pub source_hash: String,
    pub duration_ms: u64,
    pub frame_sample_policy: String,
    pub scene_count: usize,
    pub fingerprints: Vec<VideoFingerprintFrameForNotary>,
    pub client_signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFingerprintFrameForNotary {
    pub scene_index: usize,
    pub timestamp_ms: u64,
    pub phash: String,
    pub color_hash: String,
    pub edge_hash: String,
    pub local_blocks: Vec<VideoFingerprintLocalBlockForNotary>,
    pub crop_windows: Vec<VideoFingerprintCropWindowForNotary>,
    pub motion_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFingerprintLocalBlockForNotary {
    pub grid: String,
    pub row: u8,
    pub col: u8,
    pub phash: String,
    pub edge_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFingerprintCropWindowForNotary {
    pub region: String,
    pub phash: String,
    pub edge_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudPullResult {
    pub next_cursor: String,
    pub total_changes: u32,
    pub applied: u32,
    pub skipped: u32,
    pub imported_queue_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudQueueStatus {
    pub pending: u64,
    pub syncing: u64,
    pub failed: u64,
    pub blocked: u64,
    pub synced: u64,
    pub retry_exhausted: u64,
    pub stale_recovered: u64,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub last_failure_at: Option<String>,
    pub next_retry_at: Option<String>,
    pub last_error: Option<String>,
    pub last_error_code: Option<String>,
    pub last_http_status: Option<u16>,
    pub blocked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudQueueFlushResult {
    pub attempted: u32,
    pub synced: u32,
    pub failed: u32,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkIdReserveRequest {
    pub request_id: String,
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub media_type: String,
    pub payload_protocol_version: u32,
    pub payload_bytes_length: u32,
    pub parent_watermark_uid: Option<String>,
    pub revision: u32,
    pub original_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkIdConfirmRequest {
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub watermark_uid: String,
    pub payload_protocol_version: u32,
    pub payload_bytes_length: u32,
    pub original_hash: Option<String>,
    pub protected_copy_hash: Option<String>,
    pub write_verification_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkIdReconcileRequest {
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub watermark_uid: String,
    pub media_type: String,
    pub payload_protocol_version: u32,
    pub payload_bytes_length: u32,
    pub parent_watermark_uid: Option<String>,
    pub revision: u32,
    pub original_hash: Option<String>,
    pub protected_copy_hash: Option<String>,
    pub write_verification_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkIdReissueRequest {
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub previous_watermark_uid: String,
    pub media_type: String,
    pub payload_protocol_version: u32,
    pub payload_bytes_length: u32,
    pub parent_watermark_uid: Option<String>,
    pub revision: u32,
    pub reason: String,
    pub original_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkIdRegistryResponse {
    pub registry_id: String,
    pub watermark_uid: String,
    pub watermark_id_issue_mode: String,
    pub registry_status: String,
    pub registry_receipt: String,
    pub registry_proof_hash: String,
    pub payload_protocol_version: u32,
    pub payload_bytes_length: u32,
    pub parent_watermark_uid: Option<String>,
    pub revision: u32,
    pub issued_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WatermarkIdReissueResponse {
    pub job_id: String,
    pub previous_watermark_uid: String,
    pub replacement: WatermarkIdRegistryResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingPaymentSessionRequest {
    pub account_id: String,
    pub workspace_id: String,
    pub plan_code: String,
    pub billing_cycle: String,
    pub preferred_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingPaymentSessionResponse {
    pub payment_session_id: String,
    pub provider: String,
    pub provider_order_id: String,
    pub payment_action: BillingPaymentAction,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingPaymentAction {
    #[serde(rename = "type")]
    pub action_type: String,
    pub qr_code_url: Option<String>,
    pub h5_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingPaymentSessionStatusResponse {
    pub payment_session_id: String,
    pub provider: String,
    pub provider_order_id: String,
    pub status: String,
    pub plan_code: String,
    pub billing_cycle: String,
    pub expires_at: String,
    pub last_checked_at: Option<String>,
    pub next_check_after: Option<String>,
    pub check_attempts: u32,
    pub entitlement: CloudEntitlement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingPaymentSessionReconcileResponse {
    pub payment_session_id: String,
    pub status: String,
    pub message: String,
    pub entitlement: CloudEntitlement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPurchaseSessionRequest {
    pub account_id: String,
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub vault_record_id: String,
    pub product_code: String,
    pub preferred_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPurchaseSessionResponse {
    pub payment_session_id: String,
    pub provider: String,
    pub provider_order_id: String,
    pub product_code: String,
    pub price_cents: i64,
    pub currency: String,
    pub payment_action: BillingPaymentAction,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPurchaseGrant {
    pub grant_id: String,
    pub account_id: String,
    pub workspace_id: String,
    pub creator_profile_id: String,
    pub vault_record_id: String,
    pub product_code: String,
    pub price_cents: i64,
    pub currency: String,
    pub status: String,
    pub granted_at: String,
    pub revoked_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPurchaseSessionStatusResponse {
    pub payment_session_id: String,
    pub provider: String,
    pub provider_order_id: String,
    pub status: String,
    pub product_code: String,
    pub price_cents: i64,
    pub currency: String,
    pub vault_record_id: String,
    pub expires_at: String,
    pub last_checked_at: Option<String>,
    pub next_check_after: Option<String>,
    pub check_attempts: u32,
    pub grant: Option<ReportPurchaseGrant>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportPurchaseSessionReconcileResponse {
    pub payment_session_id: String,
    pub status: String,
    pub message: String,
    pub grant: Option<ReportPurchaseGrant>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DesktopCloudSyncProfile {
    pub cloud_base_url: String,
    pub account_id: String,
    pub account_label: String,
    pub access_token: String,
    pub refresh_token: String,
    pub workspace_id: String,
    pub workspace_name: String,
    pub device_id: String,
    pub device_name: Option<String>,
    pub device_platform: Option<String>,
    pub creator_profile_id: String,
    pub creator_display_name: String,
    pub entitlement_id: String,
    pub entitlement_label: String,
    pub entitlement_status: String,
    pub entitlement_plan_code: String,
    #[serde(default)]
    pub entitlement_plan_key: String,
    pub entitlement_features: serde_json::Value,
    #[serde(default)]
    pub sync_policy: String,
    pub last_remote_cursor: Option<String>,
    pub updated_at: String,
}

impl DesktopCloudSyncProfile {
    pub fn from_session(base_url: &str, session: CloudAccountSession) -> Self {
        let entitlement_label = session.entitlement.normalized_plan_label();
        let entitlement_plan_key = session.entitlement.normalized_plan_key();
        Self {
            cloud_base_url: base_url.trim().trim_end_matches('/').to_string(),
            account_id: session.account.id,
            account_label: session.account.display_name,
            access_token: session.access_token,
            refresh_token: session.refresh_token,
            workspace_id: session.workspace.id,
            workspace_name: session.workspace.name,
            device_id: session.device.id,
            device_name: session.device.name,
            device_platform: session.device.platform,
            creator_profile_id: session.creator_profile.id,
            creator_display_name: session.creator_profile.display_name,
            entitlement_id: session.entitlement.id,
            entitlement_label,
            entitlement_status: session.entitlement.status,
            entitlement_plan_code: session.entitlement.plan_code,
            entitlement_plan_key,
            sync_policy: normalized_sync_policy(
                &session.sync_policy,
                &session.entitlement.features,
            ),
            entitlement_features: session.entitlement.features,
            last_remote_cursor: session.cloud_vault_cursor,
            updated_at: Utc::now().to_rfc3339(),
        }
    }

    pub fn apply_snapshot(&mut self, snapshot: CloudAccountSnapshot) {
        let entitlement_label = snapshot.entitlement.normalized_plan_label();
        let entitlement_plan_key = snapshot.entitlement.normalized_plan_key();
        self.account_id = snapshot.account.id;
        self.account_label = snapshot.account.display_name;
        self.workspace_id = snapshot.workspace.id;
        self.workspace_name = snapshot.workspace.name;
        self.device_id = snapshot.device.id;
        self.device_name = snapshot.device.name;
        self.device_platform = snapshot.device.platform;
        self.creator_profile_id = snapshot.creator_profile.id;
        self.creator_display_name = snapshot.creator_profile.display_name;
        self.entitlement_id = snapshot.entitlement.id;
        self.entitlement_label = entitlement_label;
        self.entitlement_plan_key = entitlement_plan_key;
        self.entitlement_status = snapshot.entitlement.status;
        self.entitlement_plan_code = snapshot.entitlement.plan_code;
        self.sync_policy =
            normalized_sync_policy(&snapshot.sync_policy, &snapshot.entitlement.features);
        self.entitlement_features = snapshot.entitlement.features;
        self.last_remote_cursor = snapshot.cloud_vault_cursor;
        self.updated_at = Utc::now().to_rfc3339();
    }

    pub fn apply_session(&mut self, base_url: &str, session: CloudAccountSession) {
        *self = Self::from_session(base_url, session);
    }

    pub fn apply_entitlement(&mut self, entitlement: CloudEntitlement) {
        let entitlement_label = entitlement.normalized_plan_label();
        let entitlement_plan_key = entitlement.normalized_plan_key();
        self.entitlement_id = entitlement.id;
        self.entitlement_label = entitlement_label;
        self.entitlement_plan_key = entitlement_plan_key;
        self.entitlement_status = entitlement.status;
        self.entitlement_plan_code = entitlement.plan_code;
        self.entitlement_features = entitlement.features;
        self.sync_policy = sync_policy_for_features_and_preference(
            &self.entitlement_features,
            self.sync_policy != "manual_local_only",
        );
        self.updated_at = Utc::now().to_rfc3339();
    }

    pub fn apply_sync_preferences(&mut self, response: SyncPreferencesResponse) {
        self.apply_entitlement(response.entitlement);
        self.sync_policy =
            normalized_sync_policy(&response.sync_policy, &self.entitlement_features);
        self.last_remote_cursor = response.cloud_vault_cursor;
        self.updated_at = Utc::now().to_rfc3339();
    }
}

fn normalized_sync_policy(value: &str, features: &serde_json::Value) -> String {
    let value = value.trim();
    if value.is_empty() {
        sync_policy_for_features(features)
    } else {
        value.to_string()
    }
}

fn sync_policy_for_features(features: &serde_json::Value) -> String {
    sync_policy_for_features_and_preference(features, true)
}

fn sync_policy_for_features_and_preference(
    features: &serde_json::Value,
    auto_sync_enabled: bool,
) -> String {
    if features
        .get("cloud_sync")
        .and_then(serde_json::Value::as_bool)
        != Some(true)
    {
        return "blocked_by_entitlement".to_string();
    }
    if auto_sync_enabled {
        "auto_cloud_vault".to_string()
    } else {
        "manual_local_only".to_string()
    }
}

pub struct CloudSyncClient {
    base_url: String,
    http: reqwest::blocking::Client,
}

impl CloudSyncClient {
    pub fn new(base_url: impl Into<String>) -> Result<Self, String> {
        let base_url = normalize_base_url(base_url.into())?;
        let http = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(|e| format!("创建云同步 HTTP client 失败: {e}"))?;
        Ok(Self { base_url, http })
    }

    pub fn create_auth_session(
        &self,
        request: &ContinueAccountRequest,
    ) -> Result<CloudAccountSession, String> {
        self.post_json("/v1/auth/sessions", None, request)
    }

    pub fn create_auth_challenge(
        &self,
        request: &AuthChallengeRequest,
    ) -> Result<AuthChallengeResponse, String> {
        self.post_json("/v1/auth/challenges", None, request)
    }

    pub fn refresh_auth_session(
        &self,
        refresh_token: &str,
        device_id: &str,
    ) -> Result<CloudAccountSession, String> {
        if refresh_token.trim().is_empty() {
            return Err("云同步 refresh token 为空".to_string());
        }
        if device_id.trim().is_empty() {
            return Err("云同步 deviceId 为空".to_string());
        }
        let body = json!({
            "refreshToken": refresh_token.trim(),
            "deviceId": device_id.trim(),
        });
        self.post_json("/v1/auth/refresh", None, &body)
    }

    pub fn logout_auth_session(&self, refresh_token: &str, device_id: &str) -> Result<(), String> {
        if refresh_token.trim().is_empty() || device_id.trim().is_empty() {
            return Ok(());
        }
        let body = json!({
            "refreshToken": refresh_token.trim(),
            "deviceId": device_id.trim(),
        });
        let _: serde_json::Value = self.post_json("/v1/auth/logout", None, &body)?;
        Ok(())
    }

    pub fn get_me(&self, access_token: &str) -> Result<CloudAccountSnapshot, String> {
        if access_token.trim().is_empty() {
            return Err("云同步 access token 为空".to_string());
        }
        self.get_json("/v1/me", access_token)
    }

    pub fn update_sync_preferences(
        &self,
        access_token: &str,
        auto_sync_enabled: bool,
    ) -> Result<SyncPreferencesResponse, String> {
        if access_token.trim().is_empty() {
            return Err("云同步 access token 为空".to_string());
        }
        let body = json!({
            "autoSyncEnabled": auto_sync_enabled,
            "reason": if auto_sync_enabled { "user_resumed" } else { "user_paused" },
        });
        self.patch_json("/v1/me/sync-preferences", Some(access_token), &body)
    }

    pub fn list_devices(&self, access_token: &str) -> Result<AccountDevicesResponse, String> {
        if access_token.trim().is_empty() {
            return Err("云同步 access token 为空".to_string());
        }
        self.get_json("/v1/devices", access_token)
    }

    pub fn update_device_name(
        &self,
        access_token: &str,
        device_id: &str,
        name: &str,
    ) -> Result<AccountDevice, String> {
        if access_token.trim().is_empty() {
            return Err("云同步 access token 为空".to_string());
        }
        if device_id.trim().is_empty() {
            return Err("云同步 deviceId 为空".to_string());
        }
        if name.trim().is_empty() {
            return Err("设备名称为空".to_string());
        }
        let body = json!({ "name": name.trim() });
        self.patch_json(
            &format!("/v1/devices/{}", device_id.trim()),
            Some(access_token),
            &body,
        )
    }

    pub fn revoke_device(
        &self,
        access_token: &str,
        device_id: &str,
    ) -> Result<RevokeDeviceResponse, String> {
        if access_token.trim().is_empty() {
            return Err("云同步 access token 为空".to_string());
        }
        if device_id.trim().is_empty() {
            return Err("云同步 deviceId 为空".to_string());
        }
        self.delete_json(&format!("/v1/devices/{}", device_id.trim()), access_token)
    }

    pub fn send_events_batch(
        &self,
        access_token: &str,
        device_id: &str,
        workspace_id: &str,
        events: Vec<CloudSyncEvent>,
    ) -> Result<CloudSyncBatchResult, String> {
        if access_token.trim().is_empty() {
            return Err("云同步 access token 为空".to_string());
        }
        if device_id.trim().is_empty() {
            return Err("云同步 deviceId 为空".to_string());
        }
        if workspace_id.trim().is_empty() {
            return Err("云同步 workspaceId 为空".to_string());
        }
        if events.is_empty() {
            return Err("云同步事件为空".to_string());
        }
        let body = json!({
            "deviceId": device_id,
            "workspaceId": workspace_id,
            "events": events,
        });
        self.post_json("/v1/sync/events:batch", Some(access_token), &body)
    }

    pub fn fetch_changes(
        &self,
        access_token: &str,
        workspace_id: &str,
        cursor: Option<&str>,
    ) -> Result<CloudSyncChangesResult, String> {
        if access_token.trim().is_empty() {
            return Err("云同步 access token 为空".to_string());
        }
        if workspace_id.trim().is_empty() {
            return Err("云同步 workspaceId 为空".to_string());
        }
        let mut path = format!("/v1/sync/changes?workspaceId={}", workspace_id.trim());
        if let Some(cursor) = cursor.filter(|value| !value.trim().is_empty()) {
            path.push_str("&cursor=");
            path.push_str(cursor);
        }
        self.get_json(&path, access_token)
    }

    pub fn create_video_fingerprint_notary(
        &self,
        access_token: &str,
        request: &VideoFingerprintNotaryRequest,
    ) -> Result<VideoFingerprintNotaryReceipt, String> {
        validate_video_fingerprint_notary_request(request)?;
        if access_token.trim().is_empty() {
            return Err("视频指纹存证 access token 为空".to_string());
        }
        self.post_json(
            "/v1/video-fingerprints/notaries",
            Some(access_token),
            request,
        )
    }

    pub fn get_cloud_video_task(
        &self,
        access_token: &str,
        task_id: &str,
    ) -> Result<CloudVideoTaskRecord, String> {
        if access_token.trim().is_empty() {
            return Err("L3 视频任务 access token 为空".to_string());
        }
        if task_id.trim().is_empty() {
            return Err("L3 视频任务 taskId 为空".to_string());
        }
        self.get_json(&format!("/v1/video-tasks/{}", task_id.trim()), access_token)
    }

    pub fn create_cloud_video_task(
        &self,
        access_token: &str,
        request: &CloudVideoTaskRequest,
    ) -> Result<CloudVideoTaskRecord, String> {
        if access_token.trim().is_empty() {
            return Err("L3 视频任务 access token 为空".to_string());
        }
        self.post_json("/v1/video-tasks", Some(access_token), request)
    }

    pub fn create_cloud_video_task_object_upload_authorization(
        &self,
        access_token: &str,
        request: &CloudVideoTaskObjectUploadAuthorizationRequest,
    ) -> Result<CloudVideoTaskObjectUploadAuthorizationResponse, String> {
        if access_token.trim().is_empty() {
            return Err("L3 上传授权 access token 为空".to_string());
        }
        self.post_json(
            "/v1/video-tasks/object-upload-authorizations",
            Some(access_token),
            request,
        )
    }

    pub fn upload_cloud_video_task_object_bytes(
        &self,
        upload_token: &str,
        bytes: &[u8],
    ) -> Result<CloudVideoTaskObjectUploadResponse, String> {
        if upload_token.trim().is_empty() {
            return Err("L3 上传缺少签名 token".to_string());
        }
        let response = self
            .http
            .put(format!(
                "{}/v1/video-object-store/upload?token={}",
                self.base_url,
                upload_token.trim()
            ))
            .body(bytes.to_vec())
            .send()
            .map_err(|e| cloud_network_error_message("上传 L3 视频对象", &e.to_string()))?;
        parse_response(response)
    }

    pub fn create_cloud_video_task_download_authorization(
        &self,
        access_token: &str,
        task_id: &str,
        ttl_seconds: Option<u64>,
    ) -> Result<CloudVideoTaskDownloadAuthorizationResponse, String> {
        if access_token.trim().is_empty() {
            return Err("L3 下载授权 access token 为空".to_string());
        }
        if task_id.trim().is_empty() {
            return Err("L3 下载授权 taskId 为空".to_string());
        }
        let request = CloudVideoTaskDownloadAuthorizationRequest { ttl_seconds };
        self.post_json(
            &format!(
                "/v1/video-tasks/{}/output-download-authorizations",
                task_id.trim()
            ),
            Some(access_token),
            &request,
        )
    }

    pub fn download_cloud_video_task_output(
        &self,
        access_token: &str,
        task_id: &str,
        download_token: &str,
    ) -> Result<Vec<u8>, String> {
        if access_token.trim().is_empty() {
            return Err("L3 下载 access token 为空".to_string());
        }
        if task_id.trim().is_empty() || download_token.trim().is_empty() {
            return Err("L3 下载缺少 taskId 或 token".to_string());
        }
        let response = self
            .http
            .get(format!(
                "{}{}",
                self.base_url,
                format!(
                    "/v1/video-tasks/{}/output-download?token={}",
                    task_id.trim(),
                    download_token.trim()
                )
            ))
            .bearer_auth(access_token.trim())
            .send()
            .map_err(|e| cloud_network_error_message("下载 L3 视频产物", &e.to_string()))?;
        let response = parse_response_bytes(response)?;
        Ok(response)
    }

    pub fn reserve_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReserveRequest,
    ) -> Result<WatermarkIdRegistryResponse, String> {
        validate_watermark_id_common(
            access_token,
            &request.workspace_id,
            &request.creator_profile_id,
            request.payload_protocol_version,
            request.payload_bytes_length,
        )?;
        if request.request_id.trim().is_empty() {
            return Err("版权编号签发 requestId 为空".to_string());
        }
        if request.media_type.trim().is_empty() {
            return Err("版权编号签发 mediaType 为空".to_string());
        }
        self.post_json("/v1/watermark-ids/reserve", Some(access_token), request)
    }

    pub fn confirm_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdConfirmRequest,
    ) -> Result<WatermarkIdRegistryResponse, String> {
        validate_watermark_id_common(
            access_token,
            &request.workspace_id,
            &request.creator_profile_id,
            request.payload_protocol_version,
            request.payload_bytes_length,
        )?;
        if request.watermark_uid.trim().is_empty() {
            return Err("版权编号确认 watermarkUid 为空".to_string());
        }
        if request.write_verification_status.trim().is_empty() {
            return Err("版权编号确认 writeVerificationStatus 为空".to_string());
        }
        self.post_json("/v1/watermark-ids/confirm", Some(access_token), request)
    }

    pub fn reconcile_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReconcileRequest,
    ) -> Result<WatermarkIdRegistryResponse, String> {
        validate_watermark_id_common(
            access_token,
            &request.workspace_id,
            &request.creator_profile_id,
            request.payload_protocol_version,
            request.payload_bytes_length,
        )?;
        if request.watermark_uid.trim().is_empty() {
            return Err("版权编号补登记 watermarkUid 为空".to_string());
        }
        if request.media_type.trim().is_empty() {
            return Err("版权编号补登记 mediaType 为空".to_string());
        }
        self.post_json("/v1/watermark-ids/reconcile", Some(access_token), request)
    }

    pub fn reissue_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReissueRequest,
    ) -> Result<WatermarkIdReissueResponse, String> {
        validate_watermark_id_common(
            access_token,
            &request.workspace_id,
            &request.creator_profile_id,
            request.payload_protocol_version,
            request.payload_bytes_length,
        )?;
        if request.previous_watermark_uid.trim().is_empty() {
            return Err("版权编号重签 previousWatermarkUid 为空".to_string());
        }
        if request.media_type.trim().is_empty() {
            return Err("版权编号重签 mediaType 为空".to_string());
        }
        if request.reason.trim().is_empty() {
            return Err("版权编号重签 reason 为空".to_string());
        }
        self.post_json("/v1/watermark-ids/reissue", Some(access_token), request)
    }

    pub fn create_billing_payment_session(
        &self,
        access_token: &str,
        request: &BillingPaymentSessionRequest,
    ) -> Result<BillingPaymentSessionResponse, String> {
        if access_token.trim().is_empty() {
            return Err("支付会话 access token 为空".to_string());
        }
        if request.account_id.trim().is_empty() || request.workspace_id.trim().is_empty() {
            return Err("支付会话缺少账户或工作区".to_string());
        }
        if !matches!(request.plan_code.as_str(), "creator" | "studio") {
            return Err("支付会话仅支持 Creator / Studio".to_string());
        }
        if !matches!(request.billing_cycle.as_str(), "monthly" | "yearly") {
            return Err("支付周期仅支持 monthly / yearly".to_string());
        }
        self.post_json("/v1/billing/payment-sessions", Some(access_token), request)
    }

    pub fn get_current_entitlement(&self, access_token: &str) -> Result<CloudEntitlement, String> {
        if access_token.trim().is_empty() {
            return Err("权益刷新 access token 为空".to_string());
        }
        self.get_json("/v1/entitlements/current", access_token)
    }

    pub fn get_billing_payment_session_status(
        &self,
        access_token: &str,
        payment_session_id: &str,
    ) -> Result<BillingPaymentSessionStatusResponse, String> {
        if access_token.trim().is_empty() {
            return Err("支付会话 access token 为空".to_string());
        }
        if payment_session_id.trim().is_empty() {
            return Err("支付会话 ID 为空".to_string());
        }
        self.get_json(
            &format!("/v1/billing/payment-sessions/{}", payment_session_id.trim()),
            access_token,
        )
    }

    pub fn reconcile_billing_payment_session(
        &self,
        access_token: &str,
        payment_session_id: &str,
    ) -> Result<BillingPaymentSessionReconcileResponse, String> {
        if access_token.trim().is_empty() {
            return Err("支付会话 access token 为空".to_string());
        }
        if payment_session_id.trim().is_empty() {
            return Err("支付会话 ID 为空".to_string());
        }
        self.post_json(
            &format!(
                "/v1/billing/payment-sessions/{}/reconcile",
                payment_session_id.trim()
            ),
            Some(access_token),
            &serde_json::json!({}),
        )
    }

    pub fn create_report_purchase_session(
        &self,
        access_token: &str,
        request: &ReportPurchaseSessionRequest,
    ) -> Result<ReportPurchaseSessionResponse, String> {
        if access_token.trim().is_empty() {
            return Err("报告购买会话 access token 为空".to_string());
        }
        if request.account_id.trim().is_empty()
            || request.workspace_id.trim().is_empty()
            || request.creator_profile_id.trim().is_empty()
            || request.vault_record_id.trim().is_empty()
        {
            return Err("报告购买会话缺少账户、工作区、创作者或版权记录".to_string());
        }
        if !matches!(
            request.product_code.as_str(),
            "copyright_report_single" | "rights_evidence_pack_single"
        ) {
            return Err("报告商品不在可购买范围内".to_string());
        }
        self.post_json(
            "/v1/billing/report-purchase-sessions",
            Some(access_token),
            request,
        )
    }

    pub fn get_report_purchase_session_status(
        &self,
        access_token: &str,
        payment_session_id: &str,
    ) -> Result<ReportPurchaseSessionStatusResponse, String> {
        if access_token.trim().is_empty() {
            return Err("报告购买会话 access token 为空".to_string());
        }
        if payment_session_id.trim().is_empty() {
            return Err("报告购买会话 ID 为空".to_string());
        }
        self.get_json(
            &format!(
                "/v1/billing/report-purchase-sessions/{}",
                payment_session_id.trim()
            ),
            access_token,
        )
    }

    pub fn reconcile_report_purchase_session(
        &self,
        access_token: &str,
        payment_session_id: &str,
    ) -> Result<ReportPurchaseSessionReconcileResponse, String> {
        if access_token.trim().is_empty() {
            return Err("报告购买会话 access token 为空".to_string());
        }
        if payment_session_id.trim().is_empty() {
            return Err("报告购买会话 ID 为空".to_string());
        }
        self.post_json(
            &format!(
                "/v1/billing/report-purchase-sessions/{}/reconcile",
                payment_session_id.trim()
            ),
            Some(access_token),
            &serde_json::json!({}),
        )
    }

    fn post_json<T, R>(&self, path: &str, token: Option<&str>, body: &T) -> Result<R, String>
    where
        T: Serialize + ?Sized,
        R: for<'de> Deserialize<'de>,
    {
        let payload =
            serde_json::to_string(body).map_err(|e| format!("序列化云同步请求失败: {e}"))?;
        let mut request = self
            .http
            .post(format!("{}{}", self.base_url, path))
            .header(CONTENT_TYPE, "application/json")
            .body(payload);
        if let Some(token) = token {
            request = request.bearer_auth(token.trim());
        }
        let response = request
            .send()
            .map_err(|e| cloud_network_error_message("发送云同步请求", &e.to_string()))?;
        parse_response(response)
    }

    fn patch_json<T, R>(&self, path: &str, token: Option<&str>, body: &T) -> Result<R, String>
    where
        T: Serialize + ?Sized,
        R: for<'de> Deserialize<'de>,
    {
        let payload =
            serde_json::to_string(body).map_err(|e| format!("序列化云同步请求失败: {e}"))?;
        let mut request = self
            .http
            .patch(format!("{}{}", self.base_url, path))
            .header(CONTENT_TYPE, "application/json")
            .body(payload);
        if let Some(token) = token {
            request = request.bearer_auth(token.trim());
        }
        let response = request
            .send()
            .map_err(|e| cloud_network_error_message("发送云同步请求", &e.to_string()))?;
        parse_response(response)
    }

    fn get_json<R>(&self, path: &str, token: &str) -> Result<R, String>
    where
        R: for<'de> Deserialize<'de>,
    {
        let response = self
            .http
            .get(format!("{}{}", self.base_url, path))
            .bearer_auth(token.trim())
            .send()
            .map_err(|e| cloud_network_error_message("拉取云端变更", &e.to_string()))?;
        parse_response(response)
    }

    fn delete_json<R>(&self, path: &str, token: &str) -> Result<R, String>
    where
        R: for<'de> Deserialize<'de>,
    {
        let response = self
            .http
            .delete(format!("{}{}", self.base_url, path))
            .bearer_auth(token.trim())
            .send()
            .map_err(|e| cloud_network_error_message("发送云同步请求", &e.to_string()))?;
        parse_response(response)
    }
}

fn validate_watermark_id_common(
    access_token: &str,
    workspace_id: &str,
    creator_profile_id: &str,
    payload_protocol_version: u32,
    payload_bytes_length: u32,
) -> Result<(), String> {
    if access_token.trim().is_empty() {
        return Err("版权编号登记 access token 为空".to_string());
    }
    if workspace_id.trim().is_empty() {
        return Err("版权编号登记 workspaceId 为空".to_string());
    }
    if creator_profile_id.trim().is_empty() {
        return Err("版权编号登记 creatorProfileId 为空".to_string());
    }
    let accepts_v2_rollback = payload_protocol_version == 2
        && payload_bytes_length == watermark_core::PAYLOAD_BYTES as u32;
    let accepts_v3_default = payload_protocol_version == 3
        && payload_bytes_length == watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32;
    if !accepts_v2_rollback && !accepts_v3_default {
        return Err(
            "版权编号登记只接受 V3/39 默认 payload 或显式 V2/119 迁移回滚 payload".to_string(),
        );
    }
    Ok(())
}

pub fn validate_video_fingerprint_notary_request(
    request: &VideoFingerprintNotaryRequest,
) -> Result<(), String> {
    if request.workspace_id.trim().is_empty() {
        return Err("视频指纹存证 workspaceId 为空".to_string());
    }
    if request.creator_profile_id.trim().is_empty() {
        return Err("视频指纹存证 creatorProfileId 为空".to_string());
    }
    if request.crop_window_fingerprint_root.trim().is_empty() || request.crop_window_count == 0 {
        return Err("视频指纹存证缺少裁剪候选窗口摘要".to_string());
    }
    if request.upload_manifest.contains_original_video
        || request.upload_manifest.contains_watermarked_video
        || request.upload_manifest.contains_local_paths
    {
        return Err("视频指纹存证不得上传原始视频、加水印视频或本地路径".to_string());
    }
    if request.upload_manifest.items.is_empty() {
        return Err("视频指纹存证 uploadManifest 为空".to_string());
    }
    Ok(())
}

pub fn video_fingerprint_bundle_to_notary_request(
    workspace_id: &str,
    creator_profile_id: &str,
    bundle_sha256: &str,
    bundle_bytes: u64,
    bundle: &VideoFingerprintBundleForNotary,
) -> Result<VideoFingerprintNotaryRequest, String> {
    if workspace_id.trim().is_empty() {
        return Err("视频指纹存证 workspaceId 为空".to_string());
    }
    if creator_profile_id.trim().is_empty() {
        return Err("视频指纹存证 creatorProfileId 为空".to_string());
    }
    if bundle.scene_count == 0 || bundle.fingerprints.is_empty() {
        return Err("视频指纹存证缺少整帧摘要".to_string());
    }
    if bundle.scene_count > u32::MAX as usize || bundle.fingerprints.len() > u32::MAX as usize {
        return Err("视频指纹存证 sceneCount 超出支持范围".to_string());
    }

    let local_block_count = bundle
        .fingerprints
        .iter()
        .map(|frame| frame.local_blocks.len())
        .sum::<usize>();
    if local_block_count == 0 || local_block_count > u32::MAX as usize {
        return Err("视频指纹存证缺少局部块摘要".to_string());
    }

    let crop_window_count = bundle
        .fingerprints
        .iter()
        .map(|frame| frame.crop_windows.len())
        .sum::<usize>();
    if crop_window_count == 0 || crop_window_count > u32::MAX as usize {
        return Err("视频指纹存证缺少裁剪候选窗口摘要".to_string());
    }

    let global_frame_fingerprints = bundle
        .fingerprints
        .iter()
        .map(|frame| {
            if frame.scene_index > u32::MAX as usize {
                return Err("视频指纹存证 sceneIndex 超出支持范围".to_string());
            }
            Ok(VideoGlobalFrameFingerprint {
                scene_index: frame.scene_index as u32,
                timestamp_ms: frame.timestamp_ms,
                phash: frame.phash.clone(),
                color_hash: frame.color_hash.clone(),
                edge_hash: frame.edge_hash.clone(),
                motion_summary: frame.motion_summary.clone(),
            })
        })
        .collect::<Result<Vec<_>, String>>()?;

    let local_block_fingerprint_root = local_block_fingerprint_root(bundle);
    let crop_window_fingerprint_root = crop_window_fingerprint_root(bundle);
    let fingerprint_root = video_fingerprint_root(
        bundle,
        &global_frame_fingerprints,
        &local_block_fingerprint_root,
        &crop_window_fingerprint_root,
    );

    let request = VideoFingerprintNotaryRequest {
        schema_version: "video_fingerprint_notary_request_v1".to_string(),
        workspace_id: workspace_id.trim().to_string(),
        creator_profile_id: creator_profile_id.trim().to_string(),
        watermark_uid: bundle.watermark_uid.clone(),
        source_hash: bundle.source_hash.clone(),
        duration_ms: bundle.duration_ms,
        frame_sample_policy: bundle.frame_sample_policy.clone(),
        scene_count: bundle.scene_count as u32,
        fingerprint_schema_version: bundle.schema_version.clone(),
        global_frame_fingerprints,
        local_block_fingerprint_root,
        local_block_count: local_block_count as u32,
        crop_window_fingerprint_root,
        crop_window_count: crop_window_count as u32,
        fingerprint_root,
        client_signature: bundle.client_signature.clone(),
        upload_manifest: VideoUploadManifest {
            schema_version: "video_upload_manifest_v1".to_string(),
            contains_original_video: false,
            contains_watermarked_video: false,
            contains_local_paths: false,
            contains_proxy: false,
            items: vec![VideoUploadManifestItem {
                kind: "video_fingerprint_bundle".to_string(),
                sha256: bundle_sha256.trim().to_string(),
                bytes: bundle_bytes,
                storage_ref: None,
                sandbox_profile: None,
                transcode_profile: None,
                width: None,
                height: None,
                frame_count: None,
            }],
        },
    };

    validate_video_fingerprint_notary_request(&request)?;
    Ok(request)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncEvent {
    pub client_event_id: String,
    pub operation: String,
    pub entity_type: String,
    pub entity_id: String,
    pub payload: serde_json::Value,
}

pub fn vault_record_to_cloud_event(record: &VaultRecord) -> CloudSyncEvent {
    let mut payload = Map::new();
    payload.insert(
        "id".to_string(),
        json!(format!("desktop-vault-{}", record.id)),
    );
    payload.insert("kind".to_string(), json!(desktop_record_kind(record)));
    payload.insert("title".to_string(), json!(record.file_name.clone()));
    payload.insert(
        "watermark_uid".to_string(),
        json!(record.watermark_uid.clone()),
    );
    payload.insert("revision".to_string(), json!(record.revision));
    payload.insert(
        "creator_display_name".to_string(),
        json!(record.creator_display_name.clone()),
    );
    payload.insert(
        "trusted_time_status".to_string(),
        json!(trusted_time_status(record)),
    );
    payload.insert(
        "trusted_time_source".to_string(),
        json!(record.tsa_source.clone()),
    );
    payload.insert(
        "trusted_time_at".to_string(),
        json!(record.network_time.clone()),
    );
    payload.insert(
        "third_party_verification_status".to_string(),
        json!(third_party_verification_status(record)),
    );
    payload.insert(
        "third_party_verification_provider".to_string(),
        json!(record.tsa_source.clone()),
    );
    payload.insert(
        "third_party_verification_path".to_string(),
        json!(if record.tsa_token_path.is_some() {
            "TSA 回执"
        } else {
            "未记录"
        }),
    );
    payload.insert("sha256".to_string(), json!(record.original_hash.clone()));
    payload.insert(
        "parent_watermark_uid".to_string(),
        json!(record.parent_watermark_uid.clone()),
    );
    payload.insert(
        "rewrite_reason".to_string(),
        json!(record.rewrite_reason.clone()),
    );
    payload.insert(
        "write_verification_status".to_string(),
        json!(record.write_verification_status.clone()),
    );
    payload.insert(
        "write_verification_message".to_string(),
        json!(record.write_verification_message.clone()),
    );
    payload.insert(
        "write_verification_at".to_string(),
        json!(record.write_verification_at.clone()),
    );
    payload.insert(
        "protected_copy_name".to_string(),
        json!(record.protected_copy_name.clone()),
    );
    payload.insert(
        "protected_copy_hash".to_string(),
        json!(record.protected_copy_hash.clone()),
    );
    payload.insert(
        "payload_protocol_version".to_string(),
        json!(record.payload_protocol_version),
    );
    payload.insert(
        "payload_bytes_length".to_string(),
        json!(record.payload_bytes_length),
    );
    payload.insert(
        "media_payload_role".to_string(),
        json!(media_payload_role_for_protocol(
            record.payload_protocol_version
        )),
    );
    payload.insert(
        "watermark_id_issue_mode".to_string(),
        json!(record.watermark_id_issue_mode.clone()),
    );
    payload.insert(
        "watermark_id_registry_status".to_string(),
        json!(record.watermark_id_registry_status.clone()),
    );
    payload.insert(
        "watermark_id_registry_receipt".to_string(),
        json!(record.watermark_id_registry_receipt.clone()),
    );
    payload.insert(
        "payload_auth_status".to_string(),
        json!(record.payload_auth_status.clone()),
    );
    payload.insert(
        "output_strategy".to_string(),
        json!(record.output_strategy.clone()),
    );
    payload.insert(
        "work_source_declaration".to_string(),
        json!(record.work_source_declaration.clone()),
    );
    payload.insert(
        "training_permission_declaration".to_string(),
        json!(record.training_permission_declaration.clone()),
    );
    payload.insert(
        "creation_method_declaration".to_string(),
        json!(record.creation_method_declaration.clone()),
    );
    payload.insert(
        "human_edit_level_declaration".to_string(),
        json!(record.human_edit_level_declaration.clone()),
    );
    payload.insert(
        "authenticity_claim_declaration".to_string(),
        json!(record.authenticity_claim_declaration.clone()),
    );
    payload.insert(
        "custom_rights_statement".to_string(),
        json!(record.custom_rights_statement.clone()),
    );
    payload.insert(
        "video_notary_id".to_string(),
        json!(record.video_notary_id.clone()),
    );
    payload.insert(
        "video_notary_at".to_string(),
        json!(record.video_notary_at.clone()),
    );
    payload.insert(
        "video_notary_receipt_signature".to_string(),
        json!(record.video_notary_receipt_signature.clone()),
    );
    payload.insert(
        "video_notary_usage_ledger_id".to_string(),
        json!(record.video_notary_usage_ledger_id.clone()),
    );
    payload.insert(
        "video_fingerprint_root".to_string(),
        json!(record.video_fingerprint_root.clone()),
    );
    payload.insert(
        "video_bundle_sha256".to_string(),
        json!(record.video_bundle_sha256.clone()),
    );
    payload.insert(
        "video_bundle_bytes".to_string(),
        json!(record.video_bundle_bytes),
    );
    payload.insert(
        "video_bundle_scene_count".to_string(),
        json!(record.video_bundle_scene_count),
    );
    payload.insert(
        "video_bundle_elapsed_ms".to_string(),
        json!(record.video_bundle_elapsed_ms),
    );
    payload.insert(
        "video_frame_sample_policy".to_string(),
        json!(record.video_frame_sample_policy.clone()),
    );
    payload.insert(
        "video_visual_task_id".to_string(),
        json!(record.video_visual_task_id.clone()),
    );
    payload.insert(
        "video_visual_completed_at".to_string(),
        json!(record.video_visual_completed_at.clone()),
    );
    payload.insert(
        "video_visual_strategy_digest".to_string(),
        json!(record.video_visual_strategy_digest.clone()),
    );
    payload.insert(
        "video_visual_self_check_confidence".to_string(),
        json!(record.video_visual_self_check_confidence),
    );
    payload.insert(
        "video_visual_self_check_threshold".to_string(),
        json!(record.video_visual_self_check_threshold),
    );
    payload.insert(
        "video_visual_checked_frames".to_string(),
        json!(record.video_visual_checked_frames),
    );
    payload.insert(
        "video_visual_media_hash".to_string(),
        json!(record.video_visual_media_hash.clone()),
    );
    payload.insert(
        "video_visual_receipt_hash".to_string(),
        json!(record.video_visual_receipt_hash.clone()),
    );
    payload.insert(
        "video_visual_output_bytes".to_string(),
        json!(record.video_visual_output_bytes),
    );
    payload.insert(
        "video_visual_output_content_type".to_string(),
        json!(record.video_visual_output_content_type.clone()),
    );
    payload.insert("source".to_string(), json!("write"));
    payload.insert("sync_status".to_string(), json!("pending"));
    payload.insert("created_at".to_string(), json!(record.created_at.clone()));

    CloudSyncEvent {
        client_event_id: format!("desktop-vault-{}-{}", record.id, record.revision),
        operation: "upsertVaultRecord".to_string(),
        entity_type: "vaultRecord".to_string(),
        entity_id: format!("desktop-vault-{}", record.id),
        payload: Value::Object(payload),
    }
}

fn media_payload_role_for_protocol(protocol_version: u32) -> &'static str {
    if protocol_version >= 3 {
        "v3_minimal_anchor"
    } else {
        "v2_full_record"
    }
}

fn parse_response<R>(response: reqwest::blocking::Response) -> Result<R, String>
where
    R: for<'de> Deserialize<'de>,
{
    let status = response.status();
    let body = response
        .text()
        .map_err(|e| format!("读取云同步响应失败: {e}"))?;
    if !status.is_success() {
        return Err(cloud_http_error_message(status.as_u16(), &body));
    }
    serde_json::from_str(&body)
        .map_err(|e| format!("解析云同步响应失败: {e}; body={}", short_body(&body)))
}

fn parse_response_bytes(response: reqwest::blocking::Response) -> Result<Vec<u8>, String> {
    let status = response.status();
    if !status.is_success() {
        let body = response
            .text()
            .map_err(|e| format!("读取云同步响应失败: {e}"))?;
        return Err(cloud_http_error_message(status.as_u16(), &body));
    }
    response
        .bytes()
        .map(|bytes| bytes.to_vec())
        .map_err(|e| format!("读取 L3 视频下载字节失败: {e}"))
}

fn trusted_time_status(record: &VaultRecord) -> &'static str {
    if record.tsa_token_path.is_some() || record.network_time.is_some() {
        "已记录"
    } else {
        "未记录"
    }
}

fn third_party_verification_status(record: &VaultRecord) -> &'static str {
    if record.tsa_token_path.is_some() {
        "已获取时间戳回执"
    } else if record.network_time.is_some() {
        "已记录网络授时"
    } else {
        "未记录"
    }
}

fn cloud_http_error_message(status_code: u16, body: &str) -> String {
    let detail = short_body(body);
    let suffix = if detail.is_empty() {
        format!("HTTP {status_code}")
    } else {
        format!("HTTP {status_code} {detail}")
    };
    match status_code {
        401 => format!(
            "云同步失败：登录状态已失效或设备未被当前账户授权，请重新登录账户后再同步。({suffix})"
        ),
        403 => format!(
            "云同步失败：后端权益快照未开放正式云同步，或当前工作区/设备与云端账户不匹配，请刷新账户状态后重试。({suffix})"
        ),
        408 | 429 | 500..=599 => {
            format!("云同步失败：云服务暂时不可用或网络超时，请稍后重试。({suffix})")
        }
        _ => format!("云同步失败：云端返回异常，请复制同步诊断并反馈。({suffix})"),
    }
}

fn cloud_network_error_message(action: &str, error: &str) -> String {
    format!(
        "{action}失败：无法连接云服务，请检查网络或系统配置中的云服务地址后重试。({})",
        short_body(error)
    )
}

fn normalize_base_url(value: String) -> Result<String, String> {
    let trimmed = value.trim().trim_end_matches('/').to_string();
    if trimmed.is_empty() {
        return Err("云同步地址为空".to_string());
    }
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err("云同步地址必须以 http:// 或 https:// 开头".to_string());
    }
    Ok(trimmed)
}

fn short_body(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.len() > 160 {
        format!("{}...", &trimmed[..160])
    } else {
        trimmed.to_string()
    }
}

fn local_block_fingerprint_root(bundle: &VideoFingerprintBundleForNotary) -> String {
    let mut hasher = Sha256::new();
    update_hash_str(&mut hasher, "video-local-block-root-v1");
    for frame in &bundle.fingerprints {
        update_hash_u64(&mut hasher, frame.scene_index as u64);
        update_hash_u64(&mut hasher, frame.timestamp_ms);
        for block in &frame.local_blocks {
            update_hash_str(&mut hasher, &block.grid);
            update_hash_u64(&mut hasher, block.row as u64);
            update_hash_u64(&mut hasher, block.col as u64);
            update_hash_str(&mut hasher, &block.phash);
            update_hash_str(&mut hasher, &block.edge_hash);
        }
    }
    finish_sha256(hasher)
}

fn crop_window_fingerprint_root(bundle: &VideoFingerprintBundleForNotary) -> String {
    let mut hasher = Sha256::new();
    update_hash_str(&mut hasher, "video-crop-window-root-v1");
    for frame in &bundle.fingerprints {
        update_hash_u64(&mut hasher, frame.scene_index as u64);
        update_hash_u64(&mut hasher, frame.timestamp_ms);
        for window in &frame.crop_windows {
            update_hash_str(&mut hasher, &window.region);
            update_hash_str(&mut hasher, &window.phash);
            update_hash_str(&mut hasher, &window.edge_hash);
        }
    }
    finish_sha256(hasher)
}

fn video_fingerprint_root(
    bundle: &VideoFingerprintBundleForNotary,
    global_frames: &[VideoGlobalFrameFingerprint],
    local_block_root: &str,
    crop_window_root: &str,
) -> String {
    let mut hasher = Sha256::new();
    update_hash_str(&mut hasher, "video-fingerprint-notary-root-v1");
    update_hash_str(&mut hasher, &bundle.schema_version);
    update_hash_str(&mut hasher, &bundle.watermark_uid);
    update_hash_str(&mut hasher, &bundle.source_hash);
    update_hash_u64(&mut hasher, bundle.duration_ms);
    update_hash_str(&mut hasher, &bundle.frame_sample_policy);
    update_hash_u64(&mut hasher, bundle.scene_count as u64);
    for frame in global_frames {
        update_hash_u64(&mut hasher, frame.scene_index as u64);
        update_hash_u64(&mut hasher, frame.timestamp_ms);
        update_hash_str(&mut hasher, &frame.phash);
        update_hash_str(&mut hasher, &frame.color_hash);
        update_hash_str(&mut hasher, &frame.edge_hash);
        update_hash_str(&mut hasher, &frame.motion_summary);
    }
    update_hash_str(&mut hasher, local_block_root);
    update_hash_str(&mut hasher, crop_window_root);
    update_hash_str(&mut hasher, &bundle.client_signature);
    finish_sha256(hasher)
}

fn update_hash_str(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn update_hash_u64(hasher: &mut Sha256, value: u64) {
    hasher.update(value.to_le_bytes());
}

fn finish_sha256(hasher: Sha256) -> String {
    format!("sha256:{:x}", hasher.finalize())
}

fn desktop_record_kind(record: &VaultRecord) -> &'static str {
    queries::infer_vault_record_file_type(record)
}

fn entitlement_plan_presentation(
    plan_code: &str,
    features: &serde_json::Value,
) -> (&'static str, &'static str) {
    let has_annual_features = features
        .get("batch_processing")
        .and_then(serde_json::Value::as_bool)
        == Some(true)
        && features
            .get("cloud_sync")
            .and_then(serde_json::Value::as_bool)
            == Some(true);
    if has_annual_features
        || matches!(
            plan_code.trim().to_ascii_lowercase().as_str(),
            "creator" | "studio" | "enterprise"
        )
    {
        ("image_audio_annual", "图片 / 音频年费")
    } else {
        ("base_unpaid", "未付费")
    }
}

pub fn load_desktop_cloud_sync_profile(app_data_dir: &Path) -> Option<DesktopCloudSyncProfile> {
    let path = profile_path(app_data_dir);
    std::fs::read_to_string(path)
        .ok()
        .and_then(|body| serde_json::from_str(&body).ok())
}

pub fn save_desktop_cloud_sync_profile(
    app_data_dir: &Path,
    profile: &DesktopCloudSyncProfile,
) -> Result<(), String> {
    std::fs::create_dir_all(app_data_dir).map_err(|e| format!("创建应用数据目录失败: {e}"))?;
    let body =
        serde_json::to_string_pretty(profile).map_err(|e| format!("序列化云同步档案失败: {e}"))?;
    std::fs::write(profile_path(app_data_dir), body).map_err(|e| format!("保存云同步档案失败: {e}"))
}

pub fn clear_desktop_cloud_sync_profile(app_data_dir: &Path) -> Result<(), String> {
    let path = profile_path(app_data_dir);
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("清除云同步档案失败: {err}")),
    }
}

fn profile_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("cloud_sync_profile.json")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_video_notary_request() -> VideoFingerprintNotaryRequest {
        VideoFingerprintNotaryRequest {
            schema_version: "video_fingerprint_notary_request_v1".to_string(),
            workspace_id: "ws-1".to_string(),
            creator_profile_id: "creator-1".to_string(),
            watermark_uid: "wm-video".to_string(),
            source_hash: "sha256:source".to_string(),
            duration_ms: 125_000,
            frame_sample_policy: "uniform_8_frames_v1".to_string(),
            scene_count: 8,
            fingerprint_schema_version: "video_fingerprint_v1".to_string(),
            global_frame_fingerprints: vec![VideoGlobalFrameFingerprint {
                scene_index: 0,
                timestamp_ms: 1000,
                phash: "0000000000000001".to_string(),
                color_hash: "0000000000000002".to_string(),
                edge_hash: "0000000000000003".to_string(),
                motion_summary: "static-frame-v1".to_string(),
            }],
            local_block_fingerprint_root: "sha256:local-block-root".to_string(),
            local_block_count: 912,
            crop_window_fingerprint_root: "sha256:crop-window-root".to_string(),
            crop_window_count: 56,
            fingerprint_root: "sha256:fingerprint-root".to_string(),
            client_signature: "ed25519:client-signature".to_string(),
            upload_manifest: VideoUploadManifest {
                schema_version: "video_upload_manifest_v1".to_string(),
                contains_original_video: false,
                contains_watermarked_video: false,
                contains_local_paths: false,
                contains_proxy: false,
                items: vec![VideoUploadManifestItem {
                    kind: "video_fingerprint_bundle".to_string(),
                    sha256: "sha256:bundle".to_string(),
                    bytes: 48_212,
                    storage_ref: None,
                    sandbox_profile: None,
                    transcode_profile: None,
                    width: None,
                    height: None,
                    frame_count: None,
                }],
            },
        }
    }

    fn sample_video_fingerprint_bundle() -> VideoFingerprintBundleForNotary {
        VideoFingerprintBundleForNotary {
            schema_version: "video_fingerprint_v1".to_string(),
            watermark_uid: "wm-video".to_string(),
            source_hash: "sha256:source".to_string(),
            duration_ms: 125_000,
            frame_sample_policy: "uniform_8_frames_v1".to_string(),
            scene_count: 2,
            fingerprints: vec![
                VideoFingerprintFrameForNotary {
                    scene_index: 0,
                    timestamp_ms: 1000,
                    phash: "0000000000000001".to_string(),
                    color_hash: "0000000000000002".to_string(),
                    edge_hash: "0000000000000003".to_string(),
                    local_blocks: vec![
                        VideoFingerprintLocalBlockForNotary {
                            grid: "4x4".to_string(),
                            row: 0,
                            col: 0,
                            phash: "block-phash-0".to_string(),
                            edge_hash: "block-edge-0".to_string(),
                        },
                        VideoFingerprintLocalBlockForNotary {
                            grid: "4x4".to_string(),
                            row: 0,
                            col: 1,
                            phash: "block-phash-1".to_string(),
                            edge_hash: "block-edge-1".to_string(),
                        },
                    ],
                    crop_windows: vec![VideoFingerprintCropWindowForNotary {
                        region: "center_80".to_string(),
                        phash: "crop-phash-0".to_string(),
                        edge_hash: "crop-edge-0".to_string(),
                    }],
                    motion_summary: "static-frame-v1".to_string(),
                },
                VideoFingerprintFrameForNotary {
                    scene_index: 1,
                    timestamp_ms: 9000,
                    phash: "0000000000000011".to_string(),
                    color_hash: "0000000000000012".to_string(),
                    edge_hash: "0000000000000013".to_string(),
                    local_blocks: vec![VideoFingerprintLocalBlockForNotary {
                        grid: "4x4".to_string(),
                        row: 1,
                        col: 0,
                        phash: "block-phash-2".to_string(),
                        edge_hash: "block-edge-2".to_string(),
                    }],
                    crop_windows: vec![
                        VideoFingerprintCropWindowForNotary {
                            region: "center_80".to_string(),
                            phash: "crop-phash-1".to_string(),
                            edge_hash: "crop-edge-1".to_string(),
                        },
                        VideoFingerprintCropWindowForNotary {
                            region: "left_80".to_string(),
                            phash: "crop-phash-2".to_string(),
                            edge_hash: "crop-edge-2".to_string(),
                        },
                    ],
                    motion_summary: "motion-low-v1".to_string(),
                },
            ],
            client_signature: "ed25519:client-signature".to_string(),
        }
    }

    fn sample_video_fingerprint_bundle_json() -> &'static str {
        r#"{
  "schemaVersion": "video_fingerprint_v1",
  "watermarkUid": "wm-video",
  "sourceHash": "sha256:source",
  "durationMs": 125000,
  "frameSamplePolicy": "uniform_2_frames_v1",
  "sceneCount": 2,
  "fingerprints": [
    {
      "sceneIndex": 0,
      "timestampMs": 1000,
      "phash": "0000000000000001",
      "colorHash": "0000000000000002",
      "edgeHash": "0000000000000003",
      "localBlocks": [
        {
          "grid": "4x4",
          "row": 0,
          "col": 0,
          "phash": "block-phash-0",
          "edgeHash": "block-edge-0"
        }
      ],
      "cropWindows": [
        {
          "region": "center_80",
          "phash": "crop-phash-0",
          "edgeHash": "crop-edge-0"
        }
      ],
      "motionSummary": "static-frame-v1"
    },
    {
      "sceneIndex": 1,
      "timestampMs": 9000,
      "phash": "0000000000000011",
      "colorHash": "0000000000000012",
      "edgeHash": "0000000000000013",
      "localBlocks": [
        {
          "grid": "dense_64x36",
          "row": 1,
          "col": 2,
          "phash": "block-phash-1",
          "edgeHash": "block-edge-1"
        }
      ],
      "cropWindows": [
        {
          "region": "right_80",
          "phash": "crop-phash-1",
          "edgeHash": "crop-edge-1"
        }
      ],
      "motionSummary": "motion-low-v1"
    }
  ],
  "clientSignature": "sha256:client-signature"
}"#
    }

    fn sample_record(file_name: &str) -> VaultRecord {
        VaultRecord {
            id: 7,
            original_hash: "hash-7".to_string(),
            file_name: file_name.to_string(),
            created_at: "2026-06-16T12:00:00.000Z".to_string(),
            duration_secs: 1.0,
            resolution: "1920x1080".to_string(),
            watermark_uid: "uid-7".to_string(),
            creator_display_name: Some("测试创作者".to_string()),
            thumbnail_path: None,
            output_douyin: None,
            output_bilibili: None,
            output_xhs: None,
            is_hdr_source: false,
            hw_encoder_used: None,
            process_time_ms: None,
            tsa_token_path: Some("D:\\tokens\\uid-7.tsr".to_string()),
            network_time: Some("2026-06-16T12:00:02.000Z".to_string()),
            tsa_source: Some("https://freetsa.org/tsr".to_string()),
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
            protected_copy_name: Some("cover_protected.png".to_string()),
            protected_copy_path: Some("D:\\media\\cover_protected.png".to_string()),
            protected_copy_hash: Some("hash-protected".to_string()),
            output_strategy: "minimal_required_change".to_string(),
            work_source_declaration: "unspecified".to_string(),
            training_permission_declaration: "prohibited".to_string(),
            creation_method_declaration: "unspecified".to_string(),
            human_edit_level_declaration: "unspecified".to_string(),
            authenticity_claim_declaration: "unspecified".to_string(),
            custom_rights_statement: None,
            parent_watermark_uid: Some("uid-parent".to_string()),
            revision: 2,
            rewrite_reason: Some("owner rewrite".to_string()),
            write_verification_status: Some("verified".to_string()),
            write_verification_message: Some("完成后验证已通过".to_string()),
            write_verification_at: Some("2026-06-16T12:00:01.000Z".to_string()),
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
    fn vault_record_to_cloud_event_matches_mobile_protocol() {
        let event = vault_record_to_cloud_event(&sample_record("cover.png"));

        assert_eq!(event.operation, "upsertVaultRecord");
        assert_eq!(event.entity_type, "vaultRecord");
        assert_eq!(event.entity_id, "desktop-vault-7");
        assert_eq!(event.payload["kind"], "image");
        assert_eq!(event.payload["title"], "cover.png");
        assert_eq!(event.payload["watermark_uid"], "uid-7");
        assert_eq!(event.payload["revision"], 2);
        assert_eq!(event.payload["creator_display_name"], "测试创作者");
        assert_eq!(event.payload["trusted_time_status"], "已记录");
        assert_eq!(
            event.payload["trusted_time_source"],
            "https://freetsa.org/tsr"
        );
        assert_eq!(event.payload["trusted_time_at"], "2026-06-16T12:00:02.000Z");
        assert_eq!(
            event.payload["third_party_verification_status"],
            "已获取时间戳回执"
        );
        assert_eq!(
            event.payload["third_party_verification_provider"],
            "https://freetsa.org/tsr"
        );
        assert_eq!(event.payload["third_party_verification_path"], "TSA 回执");
        assert_eq!(event.payload["sha256"], "hash-7");
        assert_eq!(event.payload["parent_watermark_uid"], "uid-parent");
        assert_eq!(event.payload["write_verification_status"], "verified");
        assert_eq!(
            event.payload["write_verification_message"],
            "完成后验证已通过"
        );
        assert_eq!(
            event.payload["write_verification_at"],
            "2026-06-16T12:00:01.000Z"
        );
        assert_eq!(event.payload["payload_protocol_version"], 2);
        assert_eq!(event.payload["payload_bytes_length"], 119);
        assert_eq!(event.payload["media_payload_role"], "v2_full_record");
        assert_eq!(
            event.payload["watermark_id_issue_mode"],
            "offline_generated"
        );
        assert_eq!(
            event.payload["watermark_id_registry_status"],
            "pending_registration"
        );
        assert_eq!(event.payload["payload_auth_status"], "verified");
        assert!(event.payload.get("output_douyin").is_none());
        assert!(event.payload.get("output_bilibili").is_none());
        assert!(event.payload.get("output_xhs").is_none());
        assert!(event.payload.get("local_path").is_none());
        assert!(event.payload.get("bundle_path").is_none());
    }

    #[test]
    fn vault_record_to_cloud_event_marks_v3_minimal_anchor_role() {
        let mut record = sample_record("v3-anchor.png");
        record.payload_protocol_version = 3;
        record.payload_bytes_length = 39;
        record.watermark_id_issue_mode = "registry_resolved".to_string();

        let event = vault_record_to_cloud_event(&record);

        assert_eq!(event.payload["payload_protocol_version"], 3);
        assert_eq!(event.payload["payload_bytes_length"], 39);
        assert_eq!(event.payload["media_payload_role"], "v3_minimal_anchor");
        assert_eq!(
            event.payload["watermark_id_issue_mode"],
            "registry_resolved"
        );
        assert!(event.payload.get("output_douyin").is_none());
        assert!(event.payload.get("local_path").is_none());
    }

    #[test]
    fn desktop_record_kind_detects_audio_and_video() {
        assert_eq!(desktop_record_kind(&sample_record("song.wav")), "audio");
        assert_eq!(desktop_record_kind(&sample_record("movie.mp4")), "video");
    }

    #[test]
    fn cloud_http_errors_map_to_actionable_guidance() {
        let unauthorized = cloud_http_error_message(401, r#"{"error":"unauthorized"}"#);
        assert!(unauthorized.contains("登录状态已失效"));
        assert!(unauthorized.contains("HTTP 401"));

        let forbidden = cloud_http_error_message(403, r#"{"error":"forbidden"}"#);
        assert!(forbidden.contains("后端权益快照未开放正式云同步"));
        assert!(forbidden.contains("工作区/设备与云端账户不匹配"));
        assert!(forbidden.contains("HTTP 403"));

        let unavailable = cloud_http_error_message(503, r#"{"error":"unavailable"}"#);
        assert!(unavailable.contains("稍后重试"));
        assert!(unavailable.contains("HTTP 503"));
    }

    #[test]
    fn video_fingerprint_notary_request_uses_safe_manifest_contract() {
        let request = sample_video_notary_request();
        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["cropWindowFingerprintRoot"], "sha256:crop-window-root");
        assert_eq!(json["cropWindowCount"], 56);
        assert_eq!(json["uploadManifest"]["containsOriginalVideo"], false);
        assert_eq!(json["uploadManifest"]["containsWatermarkedVideo"], false);
        assert_eq!(json["uploadManifest"]["containsLocalPaths"], false);
        assert!(json.get("localPath").is_none());
        assert!(validate_video_fingerprint_notary_request(&request).is_ok());
    }

    #[test]
    fn video_fingerprint_bundle_maps_to_notary_request_with_three_layers() {
        let bundle = sample_video_fingerprint_bundle();
        let request = video_fingerprint_bundle_to_notary_request(
            " ws-1 ",
            " creator-1 ",
            " sha256:bundle ",
            48_212,
            &bundle,
        )
        .unwrap();

        assert_eq!(
            request.schema_version,
            "video_fingerprint_notary_request_v1"
        );
        assert_eq!(request.workspace_id, "ws-1");
        assert_eq!(request.creator_profile_id, "creator-1");
        assert_eq!(request.fingerprint_schema_version, "video_fingerprint_v1");
        assert_eq!(request.global_frame_fingerprints.len(), 2);
        assert_eq!(request.local_block_count, 3);
        assert_eq!(request.crop_window_count, 3);
        assert!(request.local_block_fingerprint_root.starts_with("sha256:"));
        assert!(request.crop_window_fingerprint_root.starts_with("sha256:"));
        assert!(request.fingerprint_root.starts_with("sha256:"));
        assert_eq!(
            request.upload_manifest.schema_version,
            "video_upload_manifest_v1"
        );
        assert_eq!(
            request.upload_manifest.items[0].kind,
            "video_fingerprint_bundle"
        );
        assert_eq!(request.upload_manifest.items[0].sha256, "sha256:bundle");
        assert!(validate_video_fingerprint_notary_request(&request).is_ok());
    }

    #[test]
    fn video_fingerprint_spike_bundle_json_parses_into_notary_request() {
        let bundle: VideoFingerprintBundleForNotary =
            serde_json::from_str(sample_video_fingerprint_bundle_json()).unwrap();
        let request = video_fingerprint_bundle_to_notary_request(
            "ws-1",
            "creator-1",
            "sha256:bundle-json",
            sample_video_fingerprint_bundle_json().len() as u64,
            &bundle,
        )
        .unwrap();

        assert_eq!(bundle.schema_version, "video_fingerprint_v1");
        assert_eq!(
            bundle.fingerprints[0].local_blocks[0].edge_hash,
            "block-edge-0"
        );
        assert_eq!(bundle.fingerprints[1].crop_windows[0].region, "right_80");
        assert_eq!(request.global_frame_fingerprints.len(), 2);
        assert_eq!(request.local_block_count, 2);
        assert_eq!(request.crop_window_count, 2);
        assert_eq!(
            request.upload_manifest.items[0].sha256,
            "sha256:bundle-json"
        );
        assert!(validate_video_fingerprint_notary_request(&request).is_ok());
    }

    #[test]
    fn video_fingerprint_bundle_file_smoke_binds_manifest_hash_and_size() {
        let temp_dir = tempfile::tempdir().unwrap();
        let bundle_path = temp_dir.path().join("bundle.json");
        std::fs::write(&bundle_path, sample_video_fingerprint_bundle_json()).unwrap();

        let bundle_bytes = std::fs::read(&bundle_path).unwrap();
        let bundle_sha256 = format!("sha256:{:x}", Sha256::digest(&bundle_bytes));
        let bundle: VideoFingerprintBundleForNotary =
            serde_json::from_slice(&bundle_bytes).unwrap();
        let request = video_fingerprint_bundle_to_notary_request(
            "ws-1",
            "creator-1",
            &bundle_sha256,
            bundle_bytes.len() as u64,
            &bundle,
        )
        .unwrap();

        let manifest_item = &request.upload_manifest.items[0];
        assert_eq!(manifest_item.kind, "video_fingerprint_bundle");
        assert_eq!(manifest_item.sha256, bundle_sha256);
        assert_eq!(manifest_item.bytes, bundle_bytes.len() as u64);
        assert_eq!(request.upload_manifest.contains_original_video, false);
        assert_eq!(request.upload_manifest.contains_watermarked_video, false);
        assert_eq!(request.upload_manifest.contains_local_paths, false);
        assert!(validate_video_fingerprint_notary_request(&request).is_ok());
    }

    #[test]
    fn video_fingerprint_bundle_notary_manifest_does_not_leak_media_or_paths() {
        let bundle = sample_video_fingerprint_bundle();
        let request = video_fingerprint_bundle_to_notary_request(
            "ws-1",
            "creator-1",
            "sha256:bundle",
            48_212,
            &bundle,
        )
        .unwrap();
        let json = serde_json::to_value(&request).unwrap();

        assert_eq!(json["uploadManifest"]["containsOriginalVideo"], false);
        assert_eq!(json["uploadManifest"]["containsWatermarkedVideo"], false);
        assert_eq!(json["uploadManifest"]["containsLocalPaths"], false);
        assert_eq!(json["uploadManifest"]["containsProxy"], false);
        assert!(json.get("localPath").is_none());
        assert!(json.get("originalVideo").is_none());
        assert!(json.get("watermarkedVideo").is_none());
    }

    #[test]
    fn video_fingerprint_bundle_rejects_missing_local_blocks_or_crop_windows() {
        let mut bundle = sample_video_fingerprint_bundle();
        for frame in &mut bundle.fingerprints {
            frame.local_blocks.clear();
        }
        let err = video_fingerprint_bundle_to_notary_request(
            "ws-1",
            "creator-1",
            "sha256:bundle",
            48_212,
            &bundle,
        )
        .unwrap_err();
        assert_eq!(err, "视频指纹存证缺少局部块摘要");

        let mut bundle = sample_video_fingerprint_bundle();
        for frame in &mut bundle.fingerprints {
            frame.crop_windows.clear();
        }
        let err = video_fingerprint_bundle_to_notary_request(
            "ws-1",
            "creator-1",
            "sha256:bundle",
            48_212,
            &bundle,
        )
        .unwrap_err();
        assert_eq!(err, "视频指纹存证缺少裁剪候选窗口摘要");
    }

    #[test]
    fn video_fingerprint_bundle_roots_are_deterministic_and_crop_sensitive() {
        let bundle = sample_video_fingerprint_bundle();
        let request_a = video_fingerprint_bundle_to_notary_request(
            "ws-1",
            "creator-1",
            "sha256:bundle",
            48_212,
            &bundle,
        )
        .unwrap();
        let request_b = video_fingerprint_bundle_to_notary_request(
            "ws-1",
            "creator-1",
            "sha256:bundle",
            48_212,
            &bundle,
        )
        .unwrap();
        assert_eq!(
            request_a.local_block_fingerprint_root,
            request_b.local_block_fingerprint_root
        );
        assert_eq!(
            request_a.crop_window_fingerprint_root,
            request_b.crop_window_fingerprint_root
        );
        assert_eq!(request_a.fingerprint_root, request_b.fingerprint_root);

        let mut changed = sample_video_fingerprint_bundle();
        changed.fingerprints[1].crop_windows[1].phash = "crop-phash-mutated".to_string();
        let changed_request = video_fingerprint_bundle_to_notary_request(
            "ws-1",
            "creator-1",
            "sha256:bundle",
            48_212,
            &changed,
        )
        .unwrap();
        assert_eq!(
            request_a.local_block_fingerprint_root,
            changed_request.local_block_fingerprint_root
        );
        assert_ne!(
            request_a.crop_window_fingerprint_root,
            changed_request.crop_window_fingerprint_root
        );
        assert_ne!(request_a.fingerprint_root, changed_request.fingerprint_root);
    }

    #[test]
    fn video_fingerprint_notary_request_rejects_missing_crop_windows() {
        let mut request = sample_video_notary_request();
        request.crop_window_fingerprint_root.clear();
        request.crop_window_count = 0;

        let err = validate_video_fingerprint_notary_request(&request).unwrap_err();
        assert_eq!(err, "视频指纹存证缺少裁剪候选窗口摘要");
    }

    #[test]
    fn video_fingerprint_notary_request_rejects_media_and_local_paths() {
        let mut request = sample_video_notary_request();
        request.upload_manifest.contains_original_video = true;
        let err = validate_video_fingerprint_notary_request(&request).unwrap_err();
        assert_eq!(err, "视频指纹存证不得上传原始视频、加水印视频或本地路径");

        let mut request = sample_video_notary_request();
        request.upload_manifest.contains_local_paths = true;
        let err = validate_video_fingerprint_notary_request(&request).unwrap_err();
        assert_eq!(err, "视频指纹存证不得上传原始视频、加水印视频或本地路径");
    }
}

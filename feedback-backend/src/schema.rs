use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const PUBLIC_RIGHTS_ANONYMOUS_BATCH_MAX_ITEMS: usize = 100;
pub const PUBLIC_RIGHTS_STABLE_ERROR_CODES: &[&str] = &[
    "not_found",
    "registry_unavailable",
    "payload_invalid",
    "manifest_conflict",
    "backfill_pending",
    "backfill_disputed",
    "rate_limited",
    "watermark_uid_invalid",
    "internal_error",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueAccountRequest {
    pub identifier: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
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
    pub captcha_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthChallengeResponse {
    pub challenge_id: String,
    pub delivery_channel: String,
    pub expires_at: DateTime<Utc>,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixture_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSessionRequest {
    pub identifier: String,
    #[serde(default)]
    pub challenge_id: Option<String>,
    #[serde(default)]
    pub verification_code: String,
    #[serde(default)]
    pub password: String,
    pub device: ContinueAccountDevice,
    pub local_creator_profile: ContinueAccountCreatorProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthRefreshRequest {
    pub refresh_token: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthLogoutRequest {
    pub refresh_token: String,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthLogoutResponse {
    pub ok: bool,
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
    pub sync_policy: String,
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
    pub sync_policy: String,
    pub cloud_vault_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncPreferencesRequest {
    pub auto_sync_enabled: bool,
    #[serde(default)]
    pub reason: String,
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
    pub last_seen_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountDevicesResponse {
    pub devices: Vec<AccountDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateDeviceRequest {
    pub name: String,
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
pub struct TeamWorkspaceSummary {
    pub workspace_id: String,
    pub account_id: String,
    pub name: String,
    pub workspace_type: String,
    pub status: String,
    pub member_count: u32,
    pub shared_record_count: u32,
    pub audit_event_count: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberRecord {
    pub member_id: String,
    pub workspace_id: String,
    pub account_id: String,
    pub role: String,
    pub status: String,
    pub invited_by: Option<String>,
    pub joined_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSharedLibraryRecord {
    pub shared_record_id: String,
    pub workspace_id: String,
    pub source_record_id: String,
    pub watermark_uid: String,
    pub revision: i64,
    pub record_type: String,
    pub owner_creator_profile_id: String,
    pub visible_to_roles: Vec<String>,
    pub sync_scope: String,
    pub created_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamAuditRecord {
    pub audit_id: String,
    pub workspace_id: String,
    pub actor_account_id: String,
    pub actor_member_id: Option<String>,
    pub action: String,
    pub target_type: String,
    pub target_id: String,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub reason: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamWorkspaceListResponse {
    pub workspaces: Vec<TeamWorkspaceSummary>,
    pub returned: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberListResponse {
    pub members: Vec<TeamMemberRecord>,
    pub returned: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSharedLibraryListResponse {
    pub records: Vec<TeamSharedLibraryRecord>,
    pub returned: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamAuditListResponse {
    pub events: Vec<TeamAuditRecord>,
    pub returned: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamWorkspaceCreateRequest {
    pub account_id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberCreateRequest {
    pub account_id: String,
    pub role: String,
    #[serde(default)]
    pub invited_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamMemberUpdateRequest {
    pub role: String,
    pub status: String,
    #[serde(default)]
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TeamSharedLibraryShareRequest {
    pub source_record_id: String,
    pub watermark_uid: String,
    pub revision: i64,
    pub record_type: String,
    pub owner_creator_profile_id: String,
    pub visible_to_roles: Vec<String>,
    pub sync_scope: String,
    pub created_by: String,
    #[serde(default)]
    pub reason: String,
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
    pub status: String,
    pub features: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudVideoTaskStatusKind {
    Draft,
    Queued,
    Running,
    WaitingClientRender,
    SelfChecking,
    Succeeded,
    Failed,
    Canceled,
    Expired,
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
    pub expires_at: DateTime<Utc>,
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
    pub expires_at: DateTime<Utc>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub next_check_after: Option<DateTime<Utc>>,
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
    pub expires_at: DateTime<Utc>,
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
    pub expires_at: DateTime<Utc>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub next_check_after: Option<DateTime<Utc>>,
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
    pub granted_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingWechatPayNotificationRequest {
    pub body: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingFixtureEventRequest {
    pub provider_event_id: String,
    pub provider_order_id: String,
    pub provider_transaction_id: Option<String>,
    pub account_id: String,
    pub workspace_id: String,
    pub plan_code: String,
    pub billing_cycle: String,
    pub amount_cents: i64,
    pub currency: String,
    pub event_type: String,
    pub occurred_at: DateTime<Utc>,
    pub raw_payload_json: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BillingEventApplyResponse {
    pub provider: String,
    pub provider_event_id: String,
    pub duplicate: bool,
    pub entitlement: CloudEntitlement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncEvent {
    pub client_event_id: String,
    pub operation: String,
    pub entity_type: String,
    pub entity_id: String,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncBatchRequest {
    pub device_id: String,
    pub workspace_id: String,
    pub events: Vec<CloudSyncEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncEventDisposition {
    pub client_event_id: String,
    pub disposition: String,
    pub payload_hash: Option<String>,
    pub entity_revision: Option<i64>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncBatchResult {
    pub accepted: u32,
    pub accepted_event_ids: Vec<String>,
    pub next_cursor: Option<String>,
    pub resolutions: Value,
    #[serde(default)]
    pub event_results: Vec<CloudSyncEventDisposition>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncChange {
    pub cursor: Option<String>,
    pub entity_type: String,
    pub operation: String,
    pub source_device: Option<String>,
    pub entity: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncChangesResult {
    pub next_cursor: String,
    pub changes: Vec<CloudSyncChange>,
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
    pub issued_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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
pub struct RightsManifestResponse {
    pub rights_manifest_id: String,
    pub watermark_uid: String,
    pub manifest_version: u32,
    pub status: String,
    pub training_policy: String,
    pub work_source_declaration: String,
    pub creation_method_declaration: String,
    pub human_edit_level_declaration: String,
    pub authenticity_claim_declaration: String,
    pub custom_terms_url: Option<String>,
    pub custom_terms_hash: Option<String>,
    pub standard_mappings: Value,
    pub manifest_sha256: String,
    pub signature: String,
    pub signed_by: String,
    pub effective_at: DateTime<Utc>,
    pub superseded_by: Option<String>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RightsManifestSummary {
    pub rights_manifest_id: String,
    pub watermark_uid: String,
    pub manifest_version: u32,
    pub status: String,
    pub training_policy: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicRightsRegistrySnapshot {
    pub registry_id: String,
    pub watermark_uid: String,
    pub registry_status: String,
    pub registry_proof_hash: String,
    pub registry_receipt: String,
    pub payload_auth_status: String,
    pub watermark_id_issue_mode: String,
    pub payload_protocol_version: u32,
    pub payload_bytes_length: u32,
    pub parent_watermark_uid: Option<String>,
    pub revision: u32,
    pub anchor_protocol: String,
    pub media_payload_role: String,
    pub rights_source: String,
    pub issued_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicRightsMetadata {
    pub c2pa: String,
    pub iptc: String,
    pub xmp: String,
    pub consistency: String,
    pub standard_mappings: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicTrainingPermissionSnapshot {
    pub policy: String,
    pub label: String,
    pub source: String,
    pub effective_source: String,
    pub legal_conclusion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicRightsQueryResponse {
    pub watermark_uid: String,
    pub scan_status: String,
    pub registry: PublicRightsRegistrySnapshot,
    pub rights_manifest: Option<RightsManifestResponse>,
    #[serde(default)]
    pub history: Vec<RightsManifestSummary>,
    pub public_metadata: PublicRightsMetadata,
    pub training_permission: PublicTrainingPermissionSnapshot,
    #[serde(default)]
    pub warnings: Vec<String>,
    pub resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicRightsBatchRequest {
    pub watermark_uids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicRightsBatchItem {
    pub watermark_uid: String,
    pub status: String,
    pub error_code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<PublicRightsQueryResponse>,
    pub resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicRightsBatchResponse {
    pub results: Vec<PublicRightsBatchItem>,
    pub resolved_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicRightsSignedManifestStore {
    pub format: String,
    pub profile: String,
    pub manifest_store: Value,
    pub manifest_store_hash: String,
    pub signature_algorithm: String,
    pub signature: String,
    pub signed_by: String,
    pub verification_status: String,
    pub legal_conclusion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicRightsMetadataExport {
    pub watermark_uid: String,
    pub export_version: u32,
    pub generated_at: DateTime<Utc>,
    pub legal_conclusion: bool,
    pub boundary: String,
    pub manifest_hash: String,
    pub content_credentials: Value,
    pub signed_manifest_store: PublicRightsSignedManifestStore,
    pub c2pa_assertions: Vec<Value>,
    pub iptc: Value,
    pub xmp: Value,
    pub json_ld: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseApiKeyCreateRequest {
    pub account_id: String,
    pub workspace_id: String,
    pub creator_profile_id: Option<String>,
    pub name: String,
    pub key_prefix: String,
    pub key_hash: String,
    pub scopes: Vec<String>,
    pub created_by_account_id: String,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseApiKeyIssueRequest {
    pub account_id: String,
    pub workspace_id: String,
    pub creator_profile_id: Option<String>,
    pub name: String,
    pub scopes: Vec<String>,
    pub created_by_account_id: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub reason: String,
    #[serde(default)]
    pub delivery_channel: Option<String>,
    #[serde(default)]
    pub recipient_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseApiKeyRotateRequest {
    pub reason: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub scopes: Option<Vec<String>>,
    pub created_by_account_id: String,
    pub grace_period_hours: Option<u32>,
    pub expires_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub delivery_channel: Option<String>,
    #[serde(default)]
    pub recipient_ref: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseApiKeyRecord {
    pub api_key_id: String,
    pub account_id: String,
    pub workspace_id: String,
    pub creator_profile_id: Option<String>,
    pub key_prefix: String,
    pub name: String,
    pub status: String,
    pub scopes: Vec<String>,
    pub created_by_account_id: String,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub revoked_at: Option<DateTime<Utc>>,
    pub revoked_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseApiKeyIssueResponse {
    pub api_key: EnterpriseApiKeyRecord,
    pub cleartext_api_key: String,
    pub key_prefix: String,
    pub hash_algorithm: String,
    pub shown_once: bool,
    pub custody_notice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseApiKeyRotateResponse {
    pub old_api_key: EnterpriseApiKeyRecord,
    pub new_api_key: EnterpriseApiKeyRecord,
    pub cleartext_api_key: String,
    pub key_prefix: String,
    pub hash_algorithm: String,
    pub shown_once: bool,
    pub rotation_deadline_at: DateTime<Utc>,
    pub custody_notice: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseExpiredRotationRevokeRequest {
    #[serde(default)]
    pub now: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseExpiredRotationRevokeItem {
    pub old_api_key_id: String,
    pub new_api_key_id: Option<String>,
    pub account_id: Option<String>,
    pub workspace_id: Option<String>,
    pub rotation_deadline_at: DateTime<Utc>,
    pub outcome: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseExpiredRotationRevokeResponse {
    pub processed: u32,
    pub revoked: u32,
    pub skipped: u32,
    pub items: Vec<EnterpriseExpiredRotationRevokeItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseApiKeyListQuery {
    pub account_id: Option<String>,
    pub workspace_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseApiKeyListResponse {
    pub api_keys: Vec<EnterpriseApiKeyRecord>,
    pub returned: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseApiKeyStatusChangeRequest {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseAdminAuditEventQuery {
    pub operation: Option<String>,
    pub outcome: Option<String>,
    pub account_id: Option<String>,
    pub api_key_id: Option<String>,
    pub from_occurred_at: Option<DateTime<Utc>>,
    pub to_occurred_at: Option<DateTime<Utc>>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseAdminAuditEventRecord {
    pub audit_event_id: String,
    pub operation: String,
    pub outcome: String,
    pub endpoint: String,
    pub account_id: Option<String>,
    pub workspace_id: Option<String>,
    pub api_key_id: Option<String>,
    pub target_id: Option<String>,
    pub reason: String,
    pub details: Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseAdminAuditEventListResponse {
    pub events: Vec<EnterpriseAdminAuditEventRecord>,
    pub returned: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTransparencyLicenseRecord {
    pub license_id: String,
    pub tenant_id: String,
    pub workspace_id: String,
    pub environment: String,
    pub status: String,
    pub issuer_mode: String,
    pub deployment_mode: String,
    pub public_verification_required: bool,
    pub metering_plan_id: String,
    pub effective_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTransparencyProfileEntitlementRecord {
    pub profile_id: String,
    pub profile_kind: String,
    pub status: String,
    pub effective_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub terms_version: String,
    pub approved_by: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTransparencyLicenseDetailResponse {
    pub license: AiTransparencyLicenseRecord,
    pub profile_entitlements: Vec<AiTransparencyProfileEntitlementRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTransparencyProfileEntitlementCheckRequest {
    pub license_id: String,
    pub environment: String,
    pub requested_profile_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTransparencyLicenseDecision {
    pub authorized: bool,
    pub reason_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTransparencyProfileDecision {
    pub profile_id: String,
    pub authorized: bool,
    pub reason_code: String,
    pub profile_kind: Option<String>,
    pub terms_version: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiTransparencyProfileEntitlementCheckResponse {
    pub license_id: String,
    pub authorized: bool,
    pub evaluated_at: DateTime<Utc>,
    pub license_decision: AiTransparencyLicenseDecision,
    pub profile_decisions: Vec<AiTransparencyProfileDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseQuotaBalanceInitRequest {
    pub account_id: String,
    pub workspace_id: String,
    pub quota_type: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub included_units: i64,
    pub overage_allowed: bool,
    pub overage_unit_price_cents: Option<i64>,
    pub currency: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseQuotaBalanceRecord {
    pub quota_balance_id: String,
    pub account_id: String,
    pub workspace_id: String,
    pub quota_type: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub included_units: i64,
    pub used_units: i64,
    pub reserved_units: i64,
    pub overage_allowed: bool,
    pub overage_unit_price_cents: Option<i64>,
    pub currency: String,
    pub updated_at: DateTime<Utc>,
}

pub const ENTERPRISE_PUBLIC_RIGHTS_QUOTA_TYPE: &str = "public_rights_scan_units";

pub const ENTERPRISE_GATEWAY_REQUIRED_STEPS: &[&str] = &[
    "authenticate_api_key",
    "authorize_scope",
    "check_entitlement_api_access",
    "apply_rate_limit",
    "resolve_readonly_public_rights",
    "record_quota_ledger",
    "record_api_audit_event",
];

pub const ENTERPRISE_GATEWAY_STABLE_ERROR_CODES: &[&str] = &[
    "enterprise_api_closed",
    "api_key_missing",
    "api_key_invalid",
    "api_key_paused",
    "api_key_revoked",
    "api_key_expired",
    "scope_denied",
    "api_access_disabled",
    "rate_limited",
    "quota_exhausted",
    "quota_contract_missing",
    "watermark_uid_invalid",
    "not_found",
    "registry_unavailable",
    "internal_error",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseGatewayAuthContext {
    pub api_key_id: String,
    pub account_id: String,
    pub workspace_id: String,
    pub key_prefix: String,
    pub scopes: Vec<String>,
    pub status: String,
    pub api_access: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseGatewayRateLimitPolicy {
    pub policy_id: String,
    pub requests_per_minute: u32,
    pub items_per_minute: u32,
    pub burst_requests: u32,
    pub retry_after_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseGatewayClientFingerprint {
    pub fingerprint_hash: String,
    pub source: String,
    pub trusted_proxy: bool,
    pub rate_limit_subject: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseGatewayQuotaChargePlan {
    pub quota_type: String,
    pub chargeable_units: i64,
    pub idempotency_key: String,
    pub ledger_status: String,
    pub charge_on_not_found: bool,
    pub charge_metadata_export: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseGatewayAuditContract {
    pub endpoint: String,
    pub method: String,
    pub request_id: String,
    pub request_count: u32,
    pub item_count: u32,
    pub status_code: u16,
    pub error_code: Option<String>,
    pub quota_units: i64,
    #[serde(default)]
    pub client_fingerprint: EnterpriseGatewayClientFingerprint,
    pub legal_conclusion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseGatewayReadOnlyScanContract {
    pub auth: EnterpriseGatewayAuthContext,
    pub rate_limit: EnterpriseGatewayRateLimitPolicy,
    pub quota: EnterpriseGatewayQuotaChargePlan,
    pub audit: EnterpriseGatewayAuditContract,
    pub required_steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseGatewayDryRunRequest {
    pub auth: EnterpriseGatewayAuthContext,
    pub required_scope: String,
    pub endpoint: String,
    pub method: String,
    pub request_id: String,
    pub item_count: u32,
    pub quota_type: String,
    pub quota_included_units: i64,
    pub quota_used_units: i64,
    pub quota_reserved_units: i64,
    pub quota_overage_allowed: bool,
    pub rate_limit: EnterpriseGatewayRateLimitPolicy,
    #[serde(default)]
    pub client_fingerprint: EnterpriseGatewayClientFingerprint,
    pub current_window_requests: u32,
    pub current_window_items: u32,
    pub charge_on_not_found: bool,
    pub charge_metadata_export: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseGatewayDryRunDecision {
    pub allowed: bool,
    pub status_code: u16,
    pub error_code: Option<String>,
    pub auth_decision: String,
    pub scope_decision: String,
    pub entitlement_decision: String,
    pub rate_limit_decision: String,
    pub quota_decision: String,
    pub quota: EnterpriseGatewayQuotaChargePlan,
    pub audit: EnterpriseGatewayAuditContract,
    pub required_steps: Vec<String>,
    pub legal_conclusion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseQuotaLedgerRequest {
    pub account_id: String,
    pub workspace_id: String,
    pub api_key_id: Option<String>,
    pub quota_type: String,
    pub units: i64,
    pub direction: String,
    pub event_type: String,
    pub reference_id: String,
    pub idempotency_key: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseQuotaLedgerRecord {
    pub quota_ledger_id: String,
    pub account_id: String,
    pub workspace_id: String,
    pub api_key_id: Option<String>,
    pub quota_type: String,
    pub units: i64,
    pub direction: String,
    pub event_type: String,
    pub reference_id: String,
    pub idempotency_key: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub committed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterpriseApiAuditEventRequest {
    pub account_id: String,
    pub workspace_id: String,
    pub api_key_id: Option<String>,
    pub endpoint: String,
    pub method: String,
    pub request_count: u32,
    pub item_count: u32,
    pub status_code: u16,
    pub error_code: Option<String>,
    pub quota_units: i64,
    pub client_label: Option<String>,
    pub client_fingerprint_hash: Option<String>,
    pub trusted_proxy_status: Option<String>,
    pub request_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterprisePublicRightsBatchRequest {
    pub watermark_uids: Vec<String>,
    #[serde(default)]
    pub idempotency_key: Option<String>,
    #[serde(default)]
    pub client_label: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterprisePublicRightsGateway {
    pub request_id: String,
    pub api_key_id: String,
    pub account_id: String,
    pub workspace_id: String,
    pub quota_type: String,
    pub quota_charged_units: i64,
    pub rate_limit_policy_id: String,
    pub client_fingerprint_hash: Option<String>,
    pub trusted_proxy_status: String,
    pub legal_conclusion: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EnterprisePublicRightsBatchResponse {
    pub gateway: EnterprisePublicRightsGateway,
    pub batch: PublicRightsBatchResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RightsManifestBackfillRequest {
    #[serde(default)]
    pub watermark_uids: Vec<String>,
    pub cursor: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RightsManifestBackfillItem {
    pub watermark_uid: String,
    pub status: String,
    pub error_code: Option<String>,
    pub rights_manifest_id: Option<String>,
    pub manifest_version: Option<u32>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RightsManifestBackfillResponse {
    pub processed: u32,
    pub succeeded: u32,
    pub needs_review: u32,
    pub retryable: u32,
    pub next_cursor: Option<String>,
    pub results: Vec<RightsManifestBackfillItem>,
    pub completed_at: DateTime<Utc>,
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
    #[serde(default)]
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
    #[serde(default)]
    pub contains_proxy: bool,
    #[serde(default)]
    pub items: Vec<VideoUploadManifestItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoUploadManifestItem {
    pub kind: String,
    pub sha256: String,
    pub bytes: u64,
    #[serde(default)]
    pub storage_ref: Option<String>,
    #[serde(default)]
    pub sandbox_profile: Option<String>,
    #[serde(default)]
    pub transcode_profile: Option<String>,
    #[serde(default)]
    pub width: Option<u32>,
    #[serde(default)]
    pub height: Option<u32>,
    #[serde(default)]
    pub frame_count: Option<u32>,
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
pub struct CloudVideoTaskStatusUpdateRequest {
    pub status: String,
    #[serde(default)]
    pub failure_code: Option<String>,
    #[serde(default)]
    pub strategy_digest: Option<String>,
    #[serde(default)]
    pub self_check_threshold: Option<f64>,
    #[serde(default)]
    pub self_check_confidence: Option<f64>,
    #[serde(default)]
    pub checked_frames: Option<u32>,
    #[serde(default)]
    pub watermarked_media_hash: Option<String>,
    #[serde(default)]
    pub server_receipt_signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskCompletionRequest {
    pub strategy_digest: String,
    pub self_check_threshold: f64,
    pub self_check_confidence: f64,
    pub checked_frames: u32,
    pub watermarked_media_hash: String,
    pub output_media_storage_ref: String,
    pub output_media_bytes: u64,
    pub output_media_content_type: String,
    pub worker_receipt_hash: String,
    pub worker_receipt: serde_json::Value,
    pub server_receipt_signature: String,
    pub worker_id: String,
    pub attempt_id: String,
    pub lease_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskClaimRequest {
    pub worker_id: String,
    #[serde(default)]
    pub capability_level: Option<String>,
    #[serde(default)]
    pub lease_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskClaimResponse {
    pub task: CloudVideoTaskRecord,
    pub worker_id: String,
    pub attempt_id: String,
    pub lease_token: String,
    pub lease_expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskFailureRequest {
    pub worker_id: String,
    pub attempt_id: String,
    pub lease_token: String,
    pub failure_code: String,
    #[serde(default)]
    pub failure_stage: Option<String>,
    #[serde(default)]
    pub failure_message: Option<String>,
    #[serde(default)]
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskListQuery {
    pub workspace_id: Option<String>,
    pub status: Option<String>,
    pub limit: Option<u32>,
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
    pub lease_expires_at: Option<DateTime<Utc>>,
    pub last_failure_code: Option<String>,
    pub last_failure_stage: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskListResponse {
    pub tasks: Vec<CloudVideoTaskRecord>,
    pub returned: u32,
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
    pub expires_at: DateTime<Utc>,
    pub signed_upload_url: String,
    pub upload_method: String,
    pub upload_token: String,
    pub privacy_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskObjectUploadQuery {
    pub token: String,
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
    pub expires_at: DateTime<Utc>,
    pub signed_download_url: String,
    pub download_method: String,
    pub download_token: String,
    pub privacy_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudVideoTaskDownloadAuthorizationQuery {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFingerprintNotaryReceipt {
    pub schema_version: String,
    pub notary_id: String,
    pub watermark_uid: String,
    pub source_hash: String,
    pub fingerprint_root: String,
    pub notarized_at: DateTime<Utc>,
    pub server_receipt_signature: String,
    pub usage_ledger_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AnonymousEventOutcome {
    Success,
    Failure,
    Crash,
    Diagnostic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousFeedbackEvent {
    pub event_id: String,
    pub occurred_at: DateTime<Utc>,
    pub install_id: String,
    pub session_id: String,
    pub app_version: String,
    pub feature_name: String,
    pub outcome: AnonymousEventOutcome,
    pub media_type: String,
    pub file_size_bucket: String,
    pub duration_ms: Option<u64>,
    pub error_code: Option<String>,
    pub diagnostic_note: Option<String>,
    pub stack_summary: Option<String>,
    pub pipeline_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousFeedbackBatch {
    pub install_id: String,
    pub session_id: String,
    pub app_version: String,
    pub sent_at: DateTime<Utc>,
    pub events: Vec<AnonymousFeedbackEvent>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousFeedbackBatchAck {
    pub request_id: String,
    pub received_events: usize,
    pub inserted_events: usize,
    pub duplicate_events: usize,
    pub accepted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousFeedbackStatsQuery {
    pub dimension: Option<StatsDimension>,
    pub from: Option<DateTime<Utc>>,
    pub to: Option<DateTime<Utc>>,
    pub app_version: Option<String>,
    pub feature_name: Option<String>,
    pub media_type: Option<String>,
    pub error_code: Option<String>,
    pub outcome: Option<AnonymousEventOutcome>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousFeedbackStatsResponse {
    pub dimension: String,
    pub totals: FeedbackTotals,
    pub rows: Vec<FeedbackStatRow>,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercialMetricsOverviewResponse {
    pub generated_at: DateTime<Utc>,
    pub privacy_boundary: CommercialMetricsPrivacyBoundary,
    pub accounts: CommercialAccountMetrics,
    pub entitlement_distribution: Vec<CommercialEntitlementPlanRow>,
    pub payment_sessions: CommercialPaymentSessionMetrics,
    pub feature_usage: CommercialFeatureUsageMetrics,
    pub cloud_sync: CommercialCloudSyncMetrics,
    pub anonymous_failures: Vec<CommercialAnonymousFailureRow>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercialMetricsPrivacyBoundary {
    pub excludes_original_media: bool,
    pub excludes_watermarked_media: bool,
    pub excludes_local_paths: bool,
    pub excludes_file_names: bool,
    pub excludes_full_media_hashes: bool,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommercialAccountMetrics {
    pub total_accounts: u64,
    pub new_accounts_today: u64,
    pub new_accounts_7d: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercialEntitlementPlanRow {
    pub plan_code: String,
    pub status: String,
    pub accounts: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommercialPaymentSessionMetrics {
    pub total: u64,
    pub created: u64,
    pub pending: u64,
    pub succeeded: u64,
    pub failed: u64,
    pub expired: u64,
    pub closed: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommercialFeatureUsageMetrics {
    pub local_batch_units: u64,
    pub report_export_units: u64,
    pub l2_video_notary_count: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CommercialCloudSyncMetrics {
    pub accepted_events: u64,
    pub failure_events: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommercialAnonymousFailureRow {
    pub feature_name: String,
    pub error_code: String,
    pub events: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackTotals {
    pub total_events: u64,
    pub success_events: u64,
    pub failure_events: u64,
    pub crash_events: u64,
    pub diagnostic_events: u64,
    pub avg_duration_ms: Option<f64>,
    pub last_event_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackStatRow {
    pub label: String,
    pub total_events: u64,
    pub success_events: u64,
    pub failure_events: u64,
    pub crash_events: u64,
    pub diagnostic_events: u64,
    pub avg_duration_ms: Option<f64>,
    pub last_event_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum StatsDimension {
    #[default]
    Day,
    Version,
    Feature,
    ErrorCode,
    MediaType,
    Outcome,
}

impl StatsDimension {
    pub fn as_str(&self) -> &'static str {
        match self {
            StatsDimension::Day => "day",
            StatsDimension::Version => "version",
            StatsDimension::Feature => "feature",
            StatsDimension::ErrorCode => "error_code",
            StatsDimension::MediaType => "media_type",
            StatsDimension::Outcome => "outcome",
        }
    }
}

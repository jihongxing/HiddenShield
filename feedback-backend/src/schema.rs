use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueAccountRequest {
    pub identifier: String,
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
pub struct CloudAccountSession {
    pub access_token: String,
    pub refresh_token: String,
    pub account: CloudAccount,
    pub workspace: CloudWorkspace,
    pub device: CloudDevice,
    pub creator_profile: CloudCreatorProfile,
    pub entitlement: CloudEntitlement,
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
    pub status: String,
    pub features: Value,
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
    pub events: Vec<CloudSyncEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncBatchResult {
    pub accepted: u32,
    pub accepted_event_ids: Vec<String>,
    pub next_cursor: Option<String>,
    pub resolutions: Value,
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

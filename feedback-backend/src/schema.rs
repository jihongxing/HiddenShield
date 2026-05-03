use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

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

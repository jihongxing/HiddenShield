use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;
use reqwest::header::CONTENT_TYPE;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::commands::vault::VaultRecord;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueAccountRequest {
    pub identifier: String,
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
    pub features: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudSyncBatchResult {
    pub accepted: u32,
    pub accepted_event_ids: Vec<String>,
    pub next_cursor: Option<String>,
    pub resolutions: serde_json::Value,
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
    pub failed: u64,
    pub synced: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CloudQueueFlushResult {
    pub attempted: u32,
    pub synced: u32,
    pub failed: u32,
    pub message: String,
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
    pub entitlement_features: serde_json::Value,
    pub last_remote_cursor: Option<String>,
    pub updated_at: String,
}

impl DesktopCloudSyncProfile {
    pub fn from_session(base_url: &str, session: CloudAccountSession) -> Self {
        let entitlement_label = session
            .entitlement
            .plan_name
            .clone()
            .unwrap_or_else(|| session.entitlement.plan_code.clone());
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
            entitlement_features: session.entitlement.features,
            last_remote_cursor: None,
            updated_at: Utc::now().to_rfc3339(),
        }
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

    pub fn continue_account(
        &self,
        request: &ContinueAccountRequest,
    ) -> Result<CloudAccountSession, String> {
        self.post_json("/v1/auth/continue", None, request)
    }

    pub fn send_events_batch(
        &self,
        access_token: &str,
        device_id: &str,
        events: Vec<CloudSyncEvent>,
    ) -> Result<CloudSyncBatchResult, String> {
        if access_token.trim().is_empty() {
            return Err("云同步 access token 为空".to_string());
        }
        if device_id.trim().is_empty() {
            return Err("云同步 deviceId 为空".to_string());
        }
        if events.is_empty() {
            return Err("云同步事件为空".to_string());
        }
        let body = json!({
            "deviceId": device_id,
            "events": events,
        });
        self.post_json("/v1/sync/events:batch", Some(access_token), &body)
    }

    pub fn fetch_changes(
        &self,
        access_token: &str,
        cursor: Option<&str>,
    ) -> Result<CloudSyncChangesResult, String> {
        if access_token.trim().is_empty() {
            return Err("云同步 access token 为空".to_string());
        }
        let mut path = "/v1/sync/changes".to_string();
        if let Some(cursor) = cursor.filter(|value| !value.trim().is_empty()) {
            path.push_str("?cursor=");
            path.push_str(cursor);
        }
        self.get_json(&path, access_token)
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
        let response = request.send().map_err(|e| format!("云同步请求失败: {e}"))?;
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
            .map_err(|e| format!("云同步请求失败: {e}"))?;
        parse_response(response)
    }
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
    CloudSyncEvent {
        client_event_id: format!("desktop-vault-{}-{}", record.id, record.revision),
        operation: "upsertVaultRecord".to_string(),
        entity_type: "vaultRecord".to_string(),
        entity_id: format!("desktop-vault-{}", record.id),
        payload: json!({
            "id": format!("desktop-vault-{}", record.id),
            "kind": desktop_record_kind(record),
            "title": record.file_name,
            "watermark_uid": record.watermark_uid,
            "revision": record.revision,
            "sha256": record.original_hash,
            "parent_watermark_uid": record.parent_watermark_uid,
            "rewrite_reason": record.rewrite_reason,
            "source": "write",
            "sync_status": "pending",
            "created_at": record.created_at,
        }),
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
        return Err(format!(
            "云同步 HTTP {} {}",
            status.as_u16(),
            short_body(&body)
        ));
    }
    serde_json::from_str(&body)
        .map_err(|e| format!("解析云同步响应失败: {e}; body={}", short_body(&body)))
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

fn desktop_record_kind(record: &VaultRecord) -> &'static str {
    let lower = record.file_name.to_lowercase();
    if lower.ends_with(".wav")
        || lower.ends_with(".mp3")
        || lower.ends_with(".flac")
        || lower.ends_with(".aac")
        || lower.ends_with(".ogg")
    {
        "audio"
    } else if lower.ends_with(".jpg")
        || lower.ends_with(".jpeg")
        || lower.ends_with(".png")
        || lower.ends_with(".webp")
        || lower.ends_with(".bmp")
        || lower.ends_with(".tiff")
    {
        "image"
    } else {
        "video"
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

    fn sample_record(file_name: &str) -> VaultRecord {
        VaultRecord {
            id: 7,
            original_hash: "hash-7".to_string(),
            file_name: file_name.to_string(),
            created_at: "2026-06-16T12:00:00.000Z".to_string(),
            duration_secs: 1.0,
            resolution: "1920x1080".to_string(),
            watermark_uid: "uid-7".to_string(),
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
            parent_watermark_uid: Some("uid-parent".to_string()),
            revision: 2,
            rewrite_reason: Some("owner rewrite".to_string()),
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
        assert_eq!(event.payload["sha256"], "hash-7");
        assert_eq!(event.payload["parent_watermark_uid"], "uid-parent");
    }

    #[test]
    fn desktop_record_kind_detects_audio_and_video() {
        assert_eq!(desktop_record_kind(&sample_record("song.wav")), "audio");
        assert_eq!(desktop_record_kind(&sample_record("movie.mp4")), "video");
    }
}

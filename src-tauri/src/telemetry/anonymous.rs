use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use sha2::Digest;

use super::{
    anonymous_device_id, anonymous_install_id, anonymous_session_id, feedback_backoff_due,
    feedback_failure_count, feedback_last_attempt_at, feedback_last_success_at,
    feedback_next_retry_at, is_acknowledged, is_enabled, is_network_enabled,
    record_feedback_attempt, record_feedback_failure, record_feedback_flush,
    record_feedback_success, sanitize_paths,
};

static EVENT_SEQ: AtomicU64 = AtomicU64::new(1);
static BACKGROUND_FLUSHER_STARTED: AtomicBool = AtomicBool::new(false);
const EVENT_DEDUPE_WINDOW_SECONDS: i64 = 5 * 60;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AnonymousEventOutcome {
    Success,
    Failure,
    Crash,
    Diagnostic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AnonymousFeedbackEvent {
    pub event_id: String,
    pub occurred_at: String,
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
    pub sent_at: String,
    pub events: Vec<AnonymousFeedbackEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousFeedbackStatus {
    pub install_id: String,
    pub session_id: String,
    pub queued_events: usize,
    pub queued_bytes: u64,
    pub last_event_at: Option<String>,
    pub last_flush_at: Option<String>,
    pub last_flush_error: Option<String>,
    pub consecutive_failures: u32,
    pub next_retry_at: Option<String>,
    pub last_attempt_at: Option<String>,
    pub last_success_at: Option<String>,
    pub telemetry_enabled: bool,
    pub acknowledged: bool,
    pub network_enabled: bool,
    pub endpoint_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AnonymousFlushResult {
    pub attempted_events: usize,
    pub sent_events: usize,
    pub remaining_events: usize,
    pub endpoint_configured: bool,
    pub flushed_at: Option<String>,
    pub message: String,
}

fn queue_dir(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("logs")
}

fn queue_path(app_data_dir: &Path) -> PathBuf {
    queue_dir(app_data_dir).join("anonymous_feedback.jsonl")
}

fn current_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn generate_event_id() -> String {
    let seq = EVENT_SEQ.fetch_add(1, Ordering::SeqCst);
    let seed = format!(
        "{}:{}:{}",
        current_timestamp(),
        seq,
        anonymous_device_id()
    );
    let hash = sha2::Sha256::digest(seed.as_bytes());
    format!("evt-{}", &format!("{:x}", hash)[..16])
}

fn app_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn ensure_queue_parent(app_data_dir: &Path) {
    let _ = fs::create_dir_all(queue_dir(app_data_dir));
}

fn read_queue(app_data_dir: &Path) -> Vec<AnonymousFeedbackEvent> {
    let path = queue_path(app_data_dir);
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(_) => return Vec::new(),
    };

    let reader = BufReader::new(file);
    reader
        .lines()
        .filter_map(|line| line.ok())
        .filter_map(|line| serde_json::from_str::<AnonymousFeedbackEvent>(&line).ok())
        .collect()
}

fn write_queue(app_data_dir: &Path, events: &[AnonymousFeedbackEvent]) -> Result<(), String> {
    ensure_queue_parent(app_data_dir);
    let path = queue_path(app_data_dir);
    let tmp_path = path.with_extension("jsonl.tmp");
    let mut file = fs::File::create(&tmp_path)
        .map_err(|e| format!("创建匿名反馈队列失败 {}: {e}", tmp_path.display()))?;

    for event in events {
        let line = serde_json::to_string(event)
            .map_err(|e| format!("序列化匿名反馈事件失败: {e}"))?;
        file.write_all(line.as_bytes())
            .and_then(|_| file.write_all(b"\n"))
            .map_err(|e| format!("写入匿名反馈队列失败: {e}"))?;
    }

    file.flush()
        .map_err(|e| format!("刷新匿名反馈队列失败: {e}"))?;
    let _ = fs::remove_file(&path);
    fs::rename(&tmp_path, &path)
        .map_err(|e| format!("提交匿名反馈队列失败 {} -> {}: {e}", tmp_path.display(), path.display()))
}

fn append_event(app_data_dir: &Path, event: &AnonymousFeedbackEvent) -> Result<(), String> {
    ensure_queue_parent(app_data_dir);
    let path = queue_path(app_data_dir);
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| format!("打开匿名反馈队列失败 {}: {e}", path.display()))?;

    let line = serde_json::to_string(event)
        .map_err(|e| format!("序列化匿名反馈事件失败: {e}"))?;
    file.write_all(line.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .and_then(|_| file.flush())
        .map_err(|e| format!("写入匿名反馈队列失败: {e}"))?;
    Ok(())
}

fn bucket_file_size(bytes: u64) -> String {
    const MB: u64 = 1024 * 1024;
    if bytes <= 10 * MB {
        "0-10mb"
    } else if bytes <= 50 * MB {
        "10-50mb"
    } else if bytes <= 200 * MB {
        "50-200mb"
    } else if bytes <= 500 * MB {
        "200-500mb"
    } else {
        "500mb+"
    }
    .to_string()
}

fn truncate_text(text: &str, max_len: usize) -> String {
    let clean = sanitize_paths(text).replace(['\r', '\n'], " ");
    if clean.chars().count() <= max_len {
        clean
    } else {
        clean.chars().take(max_len).collect()
    }
}

fn error_code_from_text(text: &str) -> String {
    let sanitized = sanitize_paths(text);
    let mut slug = String::with_capacity(sanitized.len());
    for ch in sanitized.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
        } else if !slug.ends_with('_') {
            slug.push('_');
        }
    }
    slug.trim_matches('_').chars().take(48).collect()
}

fn make_event(
    install_id: String,
    session_id: String,
    outcome: AnonymousEventOutcome,
    feature_name: impl Into<String>,
    media_type: impl Into<String>,
    file_size_bytes: u64,
    duration_ms: Option<u64>,
    error: Option<String>,
    diagnostic_note: Option<String>,
    stack_summary: Option<String>,
    pipeline_id: Option<String>,
) -> AnonymousFeedbackEvent {
    let error_code = error.as_deref().map(error_code_from_text);
    AnonymousFeedbackEvent {
        event_id: generate_event_id(),
        occurred_at: current_timestamp(),
        install_id,
        session_id,
        app_version: app_version(),
        feature_name: feature_name.into(),
        outcome,
        media_type: media_type.into(),
        file_size_bucket: bucket_file_size(file_size_bytes),
        duration_ms,
        error_code,
        diagnostic_note,
        stack_summary,
        pipeline_id,
    }
}

fn event_signature(
    feature_name: &str,
    outcome: &AnonymousEventOutcome,
    media_type: &str,
    error_code: Option<&str>,
    diagnostic_note: Option<&str>,
    stack_summary: Option<&str>,
    pipeline_id: Option<&str>,
) -> String {
    let outcome = match outcome {
        AnonymousEventOutcome::Success => "success",
        AnonymousEventOutcome::Failure => "failure",
        AnonymousEventOutcome::Crash => "crash",
        AnonymousEventOutcome::Diagnostic => "diagnostic",
    };
    format!(
        "{feature_name}|{outcome}|{media_type}|{}|{}|{}|{}",
        error_code.unwrap_or(""),
        diagnostic_note.unwrap_or(""),
        stack_summary.unwrap_or(""),
        pipeline_id.unwrap_or("")
    )
}

fn should_record_event(
    app_data_dir: &Path,
    signature: String,
    outcome: &AnonymousEventOutcome,
) -> bool {
    let dedupe_window = match outcome {
        AnonymousEventOutcome::Success => chrono::Duration::seconds(10),
        AnonymousEventOutcome::Failure | AnonymousEventOutcome::Crash => {
            chrono::Duration::seconds(EVENT_DEDUPE_WINDOW_SECONDS)
        }
        AnonymousEventOutcome::Diagnostic => chrono::Duration::seconds(EVENT_DEDUPE_WINDOW_SECONDS),
    };
    super::dedupe_recent_event_signature(app_data_dir, signature, dedupe_window)
}

pub fn record_success_event(
    app_data_dir: &Path,
    feature_name: impl Into<String>,
    media_type: impl Into<String>,
    file_size_bytes: u64,
    duration_ms: Option<u64>,
    pipeline_id: Option<String>,
) {
    if !is_enabled() || !is_acknowledged(app_data_dir) {
        return;
    }
    let install_id = anonymous_install_id(app_data_dir);
    let session_id = anonymous_session_id(app_data_dir);
    let feature_name = feature_name.into();
    let media_type = media_type.into();
    let event = make_event(
        install_id,
        session_id,
        AnonymousEventOutcome::Success,
        feature_name,
        media_type,
        file_size_bytes,
        duration_ms,
        None,
        None,
        None,
        pipeline_id,
    );
    let signature = event_signature(
        &event.feature_name,
        &event.outcome,
        &event.media_type,
        event.error_code.as_deref(),
        event.diagnostic_note.as_deref(),
        event.stack_summary.as_deref(),
        event.pipeline_id.as_deref(),
    );
    if should_record_event(app_data_dir, signature, &event.outcome) {
        let _ = append_event(app_data_dir, &event);
    }
}

pub fn record_failure_event(
    app_data_dir: &Path,
    feature_name: impl Into<String>,
    media_type: impl Into<String>,
    file_size_bytes: u64,
    duration_ms: Option<u64>,
    error: impl Into<String>,
    pipeline_id: Option<String>,
) {
    if !is_enabled() || !is_acknowledged(app_data_dir) {
        return;
    }
    let error_text = error.into();
    let feature_name = feature_name.into();
    let media_type = media_type.into();
    let install_id = anonymous_install_id(app_data_dir);
    let session_id = anonymous_session_id(app_data_dir);
    let pipeline_ref = pipeline_id.as_deref();
    let diagnostic = build_diagnostic_note(&feature_name, &media_type, &error_text, pipeline_ref);
    let error_code = error_code_from_text(&error_text);
    let signature = event_signature(
        &feature_name,
        &AnonymousEventOutcome::Failure,
        &media_type,
        Some(error_code.as_str()),
        Some(&truncate_text(&diagnostic, 128)),
        None,
        pipeline_ref,
    );
    if !should_record_event(app_data_dir, signature, &AnonymousEventOutcome::Failure) {
        return;
    }
    let event = make_event(
        install_id,
        session_id,
        AnonymousEventOutcome::Failure,
        feature_name,
        media_type,
        file_size_bytes,
        duration_ms,
        Some(error_text.clone()),
        Some(truncate_text(&diagnostic, 512)),
        None,
        pipeline_id,
    );
    let _ = append_event(app_data_dir, &event);
}

pub fn record_crash_event(
    app_data_dir: &Path,
    feature_name: impl Into<String>,
    media_type: impl Into<String>,
    file_size_bytes: u64,
    duration_ms: Option<u64>,
    diagnostic_note: impl Into<String>,
    stack_summary: impl Into<String>,
    pipeline_id: Option<String>,
) {
    if !is_enabled() || !is_acknowledged(app_data_dir) {
        return;
    }
    let note = diagnostic_note.into();
    let stack = stack_summary.into();
    let install_id = anonymous_install_id(app_data_dir);
    let session_id = anonymous_session_id(app_data_dir);
    let feature_name = feature_name.into();
    let media_type = media_type.into();
    let signature = event_signature(
        &feature_name,
        &AnonymousEventOutcome::Crash,
        &media_type,
        Some(&truncate_text(&note, 128)),
        Some(&truncate_text(&note, 128)),
        Some(&truncate_text(&stack, 128)),
        pipeline_id.as_deref(),
    );
    if !should_record_event(app_data_dir, signature, &AnonymousEventOutcome::Crash) {
        return;
    }
    let event = make_event(
        install_id,
        session_id,
        AnonymousEventOutcome::Crash,
        feature_name,
        media_type,
        file_size_bytes,
        duration_ms,
        Some(note.clone()),
        Some(truncate_text(&note, 512)),
        Some(truncate_text(&stack, 1024)),
        pipeline_id,
    );
    let _ = append_event(app_data_dir, &event);
}

#[allow(dead_code)]
pub fn record_diagnostic_event(
    app_data_dir: &Path,
    feature_name: impl Into<String>,
    media_type: impl Into<String>,
    file_size_bytes: u64,
    duration_ms: Option<u64>,
    diagnostic_note: impl Into<String>,
    pipeline_id: Option<String>,
) {
    if !is_enabled() || !is_acknowledged(app_data_dir) {
        return;
    }
    let note = diagnostic_note.into();
    let install_id = anonymous_install_id(app_data_dir);
    let session_id = anonymous_session_id(app_data_dir);
    let feature_name = feature_name.into();
    let media_type = media_type.into();
    let signature = event_signature(
        &feature_name,
        &AnonymousEventOutcome::Diagnostic,
        &media_type,
        Some(&truncate_text(&note, 128)),
        Some(&truncate_text(&note, 128)),
        None,
        pipeline_id.as_deref(),
    );
    if !should_record_event(app_data_dir, signature, &AnonymousEventOutcome::Diagnostic) {
        return;
    }
    let event = make_event(
        install_id,
        session_id,
        AnonymousEventOutcome::Diagnostic,
        feature_name,
        media_type,
        file_size_bytes,
        duration_ms,
        Some(note.clone()),
        Some(truncate_text(&note, 512)),
        None,
        pipeline_id,
    );
    let _ = append_event(app_data_dir, &event);
}

pub fn record_entitlement_snapshot(
    app_data_dir: &Path,
    status: impl AsRef<str>,
    source: impl AsRef<str>,
) {
    if !is_enabled() || !is_acknowledged(app_data_dir) {
        return;
    }

    let status = status.as_ref().trim().to_string();
    let source = source.as_ref().trim().to_string();
    let note = format!("status={status} | source={source}");
    let feature_name = "entitlement_snapshot";
    let media_type = "system";
    let signature = event_signature(
        feature_name,
        &AnonymousEventOutcome::Diagnostic,
        media_type,
        Some(status.as_str()),
        Some(&note),
        None,
        None,
    );
    if !should_record_event(app_data_dir, signature, &AnonymousEventOutcome::Diagnostic) {
        return;
    }

    let install_id = anonymous_install_id(app_data_dir);
    let session_id = anonymous_session_id(app_data_dir);
    let event = make_event(
        install_id,
        session_id,
        AnonymousEventOutcome::Diagnostic,
        feature_name,
        media_type,
        0,
        None,
        Some(status),
        Some(truncate_text(&note, 512)),
        None,
        None,
    );
    let _ = append_event(app_data_dir, &event);
}

pub fn get_status(app_data_dir: &Path) -> AnonymousFeedbackStatus {
    let events = read_queue(app_data_dir);
    let queued_bytes = fs::metadata(queue_path(app_data_dir))
        .map(|m| m.len())
        .unwrap_or(0);
    let last_event_at = events.last().map(|e| e.occurred_at.clone());
    let config = super::load_config(app_data_dir);

    AnonymousFeedbackStatus {
        install_id: anonymous_install_id(app_data_dir),
        session_id: anonymous_session_id(app_data_dir),
        queued_events: events.len(),
        queued_bytes,
        last_event_at,
        last_flush_at: config.last_feedback_flush_at,
        last_flush_error: config.last_feedback_flush_error,
        consecutive_failures: feedback_failure_count(app_data_dir),
        next_retry_at: feedback_next_retry_at(app_data_dir),
        last_attempt_at: feedback_last_attempt_at(app_data_dir),
        last_success_at: feedback_last_success_at(app_data_dir),
        telemetry_enabled: is_enabled(),
        acknowledged: is_acknowledged(app_data_dir),
        network_enabled: is_network_enabled(app_data_dir),
        endpoint_configured: std::env::var("HIDDENSHIELD_TELEMETRY_URL")
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false),
    }
}

#[allow(dead_code)]
pub fn build_diagnostic_note(
    feature_name: &str,
    media_type: &str,
    error: &str,
    pipeline_id: Option<&str>,
) -> String {
    let mut parts = vec![
        format!("feature={feature_name}"),
        format!("media_type={media_type}"),
        format!("error={}", truncate_text(error, 256)),
    ];
    if let Some(id) = pipeline_id {
        parts.push(format!("pipeline_id={id}"));
    }
    parts.join(" | ")
}

fn flush_queue_impl(app_data_dir: &Path, endpoint_override: Option<&str>) -> AnonymousFlushResult {
    let status = get_status(app_data_dir);
    let events = read_queue(app_data_dir);

    if events.is_empty() {
        record_feedback_flush(app_data_dir, Some(current_timestamp()), None);
        return AnonymousFlushResult {
            attempted_events: 0,
            sent_events: 0,
            remaining_events: 0,
            endpoint_configured: status.endpoint_configured,
            flushed_at: Some(current_timestamp()),
            message: "没有待发送的匿名反馈".to_string(),
        };
    }

    if !status.telemetry_enabled || !status.acknowledged || !status.network_enabled {
        let msg = if !status.telemetry_enabled {
            "匿名统计已关闭，跳过发送"
        } else if !status.acknowledged {
            "尚未完成隐私确认，跳过发送"
        } else {
            "联网已关闭，跳过发送"
        };
        record_feedback_flush(app_data_dir, None, Some(msg.to_string()));
        return AnonymousFlushResult {
            attempted_events: events.len(),
            sent_events: 0,
            remaining_events: events.len(),
            endpoint_configured: status.endpoint_configured,
            flushed_at: None,
            message: msg.to_string(),
        };
    }

    let endpoint = endpoint_override
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            std::env::var("HIDDENSHIELD_TELEMETRY_URL")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
        });

    let Some(endpoint) = endpoint else {
        let msg = "未配置匿名反馈上报地址";
        record_feedback_flush(app_data_dir, None, Some(msg.to_string()));
        return AnonymousFlushResult {
            attempted_events: events.len(),
            sent_events: 0,
            remaining_events: events.len(),
            endpoint_configured: false,
            flushed_at: None,
            message: msg.to_string(),
        };
    };

    let batch = AnonymousFeedbackBatch {
        install_id: status.install_id.clone(),
        session_id: status.session_id.clone(),
        app_version: app_version(),
        sent_at: current_timestamp(),
        events: events.clone(),
    };

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    {
        Ok(client) => client,
        Err(err) => {
            let msg = format!("创建匿名反馈客户端失败: {err}");
            record_feedback_flush(app_data_dir, None, Some(msg.clone()));
            return AnonymousFlushResult {
                attempted_events: events.len(),
                sent_events: 0,
                remaining_events: events.len(),
                endpoint_configured: true,
                flushed_at: None,
                message: msg,
            };
        }
    };

    let body = match serde_json::to_vec(&batch) {
        Ok(body) => body,
        Err(err) => {
            let msg = format!("序列化匿名反馈批次失败: {err}");
            record_feedback_flush(app_data_dir, None, Some(msg.clone()));
            return AnonymousFlushResult {
                attempted_events: events.len(),
                sent_events: 0,
                remaining_events: events.len(),
                endpoint_configured: true,
                flushed_at: None,
                message: msg,
            };
        }
    };

    let attempt_at = current_timestamp();
    record_feedback_attempt(app_data_dir, attempt_at);
    let response = client
        .post(endpoint)
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(body)
        .send();
    match response {
        Ok(resp) if resp.status().is_success() => {
            let _ = write_queue(app_data_dir, &[]);
            let flushed_at = Some(current_timestamp());
            record_feedback_flush(app_data_dir, flushed_at.clone(), None);
            record_feedback_success(app_data_dir, flushed_at.clone().unwrap_or_else(current_timestamp));
            AnonymousFlushResult {
                attempted_events: batch.events.len(),
                sent_events: batch.events.len(),
                remaining_events: 0,
                endpoint_configured: true,
                flushed_at,
                message: "匿名反馈已批量发送".to_string(),
            }
        }
        Ok(resp) => {
            let msg = format!("匿名反馈上报失败: HTTP {}", resp.status());
            record_feedback_flush(app_data_dir, None, Some(msg.clone()));
            record_feedback_failure(app_data_dir, current_timestamp(), msg.clone());
            AnonymousFlushResult {
                attempted_events: batch.events.len(),
                sent_events: 0,
                remaining_events: batch.events.len(),
                endpoint_configured: true,
                flushed_at: None,
                message: msg,
            }
        }
        Err(err) => {
            let msg = format!("匿名反馈上报失败: {err}");
            record_feedback_flush(app_data_dir, None, Some(msg.clone()));
            record_feedback_failure(app_data_dir, current_timestamp(), msg.clone());
            AnonymousFlushResult {
                attempted_events: batch.events.len(),
                sent_events: 0,
                remaining_events: batch.events.len(),
                endpoint_configured: true,
                flushed_at: None,
                message: msg,
            }
        }
    }
}

pub fn flush_queue(app_data_dir: &Path) -> AnonymousFlushResult {
    flush_queue_impl(app_data_dir, None)
}

#[cfg(test)]
pub(crate) fn flush_queue_with_endpoint(app_data_dir: &Path, endpoint: &str) -> AnonymousFlushResult {
    flush_queue_impl(app_data_dir, Some(endpoint))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread;
    use tempfile::TempDir;

    fn setup_temp_app_data() -> (TempDir, PathBuf) {
        let temp = TempDir::new().unwrap();
        let app_data_dir = temp.path().to_path_buf();
        super::super::set_enabled(&app_data_dir, true);
        super::super::set_acknowledged(&app_data_dir);
        super::super::set_network_enabled(&app_data_dir, true);
        (temp, app_data_dir)
    }

    fn queue_path(app_data_dir: &Path) -> PathBuf {
        app_data_dir.join("logs").join("anonymous_feedback.jsonl")
    }

    fn spawn_ok_server() -> (String, mpsc::Receiver<String>, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, rx) = mpsc::channel::<String>();

        let handle = thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut content_length = 0usize;

            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line == "\n" {
                    break;
                }
                let lower = line.to_ascii_lowercase();
                if let Some(value) = lower.strip_prefix("content-length:") {
                    content_length = value.trim().parse().unwrap();
                }
            }

            let mut body = vec![0u8; content_length];
            reader.read_exact(&mut body).unwrap();
            tx.send(String::from_utf8(body).unwrap()).unwrap();

            let mut stream = stream;
            let response = b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nok";
            stream.write_all(response).unwrap();
            stream.flush().unwrap();
        });

        (format!("http://{}", addr), rx, handle)
    }

    #[test]
    fn failure_events_are_sanitized_and_deduped() {
        let (_temp, app_data_dir) = setup_temp_app_data();
        let raw_path = r"C:\Users\jihx\Desktop\secret\clip.mp4";

        record_failure_event(
            &app_data_dir,
            "watermark_video",
            "video",
            42 * 1024 * 1024,
            Some(1234),
            format!("failed to parse {raw_path}"),
            Some("pipe-1".to_string()),
        );
        record_failure_event(
            &app_data_dir,
            "watermark_video",
            "video",
            42 * 1024 * 1024,
            Some(1234),
            format!("failed to parse {raw_path}"),
            Some("pipe-1".to_string()),
        );

        let body = std::fs::read_to_string(queue_path(&app_data_dir)).unwrap();
        assert_eq!(body.lines().count(), 1);
        assert!(!body.contains(raw_path));
        assert!(body.contains("[path]"));
    }

    #[test]
    fn flush_queue_skips_offline_then_sends_when_online() {
        let (_temp, app_data_dir) = setup_temp_app_data();
        super::super::set_network_enabled(&app_data_dir, false);
        let raw_path = r"C:\Users\jihx\Desktop\secret\clip.mp4";

        record_failure_event(
            &app_data_dir,
            "watermark_video",
            "video",
            42 * 1024 * 1024,
            Some(1234),
            format!("failed to parse {raw_path}"),
            Some("pipe-2".to_string()),
        );

        let offline = flush_queue(&app_data_dir);
        assert_eq!(offline.attempted_events, 1);
        assert_eq!(offline.sent_events, 0);
        assert_eq!(offline.remaining_events, 1);
        assert!(offline.message.contains("联网已关闭"));

        super::super::set_network_enabled(&app_data_dir, true);
        let (endpoint, rx, handle) = spawn_ok_server();
        let online = flush_queue_with_endpoint(&app_data_dir, &endpoint);
        let body = rx.recv().unwrap();
        handle.join().unwrap();

        assert_eq!(online.attempted_events, 1);
        assert_eq!(online.sent_events, 1);
        assert_eq!(online.remaining_events, 0);
        assert!(online.message.contains("批量发送"));
        assert!(!body.contains(raw_path));
        assert!(body.contains("watermark_video"));

        let queue_file = std::fs::read_to_string(queue_path(&app_data_dir)).unwrap();
        assert!(queue_file.trim().is_empty());
    }
}

pub fn start_background_flusher(app_data_dir: PathBuf) {
    if BACKGROUND_FLUSHER_STARTED
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }

    thread::spawn(move || loop {
        thread::sleep(Duration::from_secs(60));
        if !is_enabled() || !is_acknowledged(&app_data_dir) || !is_network_enabled(&app_data_dir) {
            continue;
        }
        if !feedback_backoff_due(&app_data_dir) {
            continue;
        }
        let _ = flush_queue(&app_data_dir);
    });
}

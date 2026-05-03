use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use serde::{Deserialize, Serialize};

pub mod anonymous;

static TELEMETRY_ENABLED: AtomicBool = AtomicBool::new(true);

/// Daily report counter — resets each calendar day. Max 5 reports per device per day.
static DAILY_REPORT_COUNT: AtomicU32 = AtomicU32::new(0);
static DAILY_REPORT_DAY: AtomicU32 = AtomicU32::new(0);
const MAX_DAILY_REPORTS: u32 = 5;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrashReport {
    pub panic_message: String,
    pub backtrace: String,
    pub os_version: String,
    pub app_version: String,
    pub anonymous_device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegCrashReport {
    pub exit_code: i32,
    pub stderr_tail: String,
    pub encoder: String,
    pub input_format: String,
    pub app_version: String,
    pub anonymous_device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataUsageInfo {
    pub ffmpeg_size_mb: f64,
    pub db_size_mb: f64,
    pub log_size_mb: f64,
    pub total_size_mb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct TelemetryConfig {
    enabled: bool,
    acknowledged: bool,
    network_enabled: bool,
    install_id: Option<String>,
    session_id: Option<String>,
    session_started_at: Option<String>,
    last_feedback_flush_at: Option<String>,
    last_feedback_flush_error: Option<String>,
    feedback_failure_count: u32,
    feedback_next_retry_at: Option<String>,
    feedback_last_attempt_at: Option<String>,
    feedback_last_success_at: Option<String>,
    recent_event_signatures: Vec<RecentEventSignature>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            acknowledged: false,
            network_enabled: true,
            install_id: None,
            session_id: None,
            session_started_at: None,
            last_feedback_flush_at: None,
            last_feedback_flush_error: None,
            feedback_failure_count: 0,
            feedback_next_retry_at: None,
            feedback_last_attempt_at: None,
            feedback_last_success_at: None,
            recent_event_signatures: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecentEventSignature {
    signature: String,
    recorded_at: String,
}

// ---------------------------------------------------------------------------
// Anonymous Device ID
// ---------------------------------------------------------------------------

/// Generate a stable anonymous device ID based on machine characteristics.
/// Uses hostname + OS + arch hashed with SHA-256. Not reversible.
pub fn anonymous_device_id() -> String {
    use sha2::{Digest, Sha256};

    let hostname = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| "unknown".to_string());
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    let input = format!("hiddenshield:{}:{}:{}", hostname, os, arch);
    let hash = Sha256::digest(input.as_bytes());
    format!("{:x}", hash)[..16].to_string()
}

// ---------------------------------------------------------------------------
// Config persistence
// ---------------------------------------------------------------------------

pub(crate) fn config_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("telemetry_config.json")
}

pub(crate) fn load_config(app_data_dir: &Path) -> TelemetryConfig {
    let path = config_path(app_data_dir);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub(crate) fn save_config(app_data_dir: &Path, config: &TelemetryConfig) {
    let path = config_path(app_data_dir);
    if let Ok(json) = serde_json::to_string_pretty(config) {
        let _ = std::fs::write(path, json);
    }
}

fn generate_anonymous_id(prefix: &str, salt: &str) -> String {
    use sha2::{Digest, Sha256};

    let now = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let pid = std::process::id();
    let seed = format!("{prefix}:{salt}:{now}:{pid}");
    let hash = Sha256::digest(seed.as_bytes());
    format!("{prefix}-{}", &format!("{:x}", hash)[..16])
}

pub(crate) fn ensure_install_id(app_data_dir: &Path) -> String {
    let mut config = load_config(app_data_dir);
    if let Some(id) = config.install_id.clone() {
        return id;
    }
    let id = generate_anonymous_id("inst", &anonymous_device_id());
    config.install_id = Some(id.clone());
    save_config(app_data_dir, &config);
    id
}

pub(crate) fn rotate_session_id(app_data_dir: &Path) -> String {
    let mut config = load_config(app_data_dir);
    let install_id = config
        .install_id
        .clone()
        .unwrap_or_else(|| ensure_install_id(app_data_dir));
    let session_id = generate_anonymous_id("sess", &install_id);
    config.session_id = Some(session_id.clone());
    config.session_started_at = Some(chrono::Utc::now().to_rfc3339());
    save_config(app_data_dir, &config);
    session_id
}

pub fn anonymous_install_id(app_data_dir: &Path) -> String {
    ensure_install_id(app_data_dir)
}

pub fn anonymous_session_id(app_data_dir: &Path) -> String {
    let config = load_config(app_data_dir);
    if let Some(id) = config.session_id {
        id
    } else {
        rotate_session_id(app_data_dir)
    }
}

pub(crate) fn record_feedback_flush(
    app_data_dir: &Path,
    last_flush_at: Option<String>,
    last_flush_error: Option<String>,
) {
    let mut config = load_config(app_data_dir);
    if let Some(last_flush_at) = last_flush_at {
        config.last_feedback_flush_at = Some(last_flush_at);
        config.last_feedback_flush_error = None;
    }
    if let Some(last_flush_error) = last_flush_error {
        config.last_feedback_flush_error = Some(last_flush_error);
    }
    save_config(app_data_dir, &config);
}

pub(crate) fn record_feedback_attempt(app_data_dir: &Path, attempt_at: String) {
    let mut config = load_config(app_data_dir);
    config.feedback_last_attempt_at = Some(attempt_at);
    save_config(app_data_dir, &config);
}

pub(crate) fn record_feedback_success(app_data_dir: &Path, success_at: String) {
    let mut config = load_config(app_data_dir);
    config.feedback_failure_count = 0;
    config.feedback_next_retry_at = None;
    config.feedback_last_success_at = Some(success_at.clone());
    config.feedback_last_attempt_at = Some(success_at);
    config.last_feedback_flush_error = None;
    save_config(app_data_dir, &config);
}

pub(crate) fn record_feedback_failure(app_data_dir: &Path, failure_at: String, error: String) {
    let mut config = load_config(app_data_dir);
    config.feedback_failure_count = config.feedback_failure_count.saturating_add(1);
    config.feedback_last_attempt_at = Some(failure_at);
    config.last_feedback_flush_error = Some(error);
    config.feedback_next_retry_at = Some(
        (chrono::Utc::now() + feedback_backoff_delay(config.feedback_failure_count)).to_rfc3339(),
    );
    save_config(app_data_dir, &config);
}

pub(crate) fn feedback_backoff_due(app_data_dir: &Path) -> bool {
    let config = load_config(app_data_dir);
    let Some(next_retry_at) = config.feedback_next_retry_at else {
        return true;
    };
    match chrono::DateTime::parse_from_rfc3339(&next_retry_at) {
        Ok(ts) => chrono::Utc::now() >= ts.with_timezone(&chrono::Utc),
        Err(_) => true,
    }
}

pub(crate) fn feedback_failure_count(app_data_dir: &Path) -> u32 {
    load_config(app_data_dir).feedback_failure_count
}

pub(crate) fn feedback_last_success_at(app_data_dir: &Path) -> Option<String> {
    load_config(app_data_dir).feedback_last_success_at
}

pub(crate) fn feedback_next_retry_at(app_data_dir: &Path) -> Option<String> {
    load_config(app_data_dir).feedback_next_retry_at
}

pub(crate) fn feedback_last_attempt_at(app_data_dir: &Path) -> Option<String> {
    load_config(app_data_dir).feedback_last_attempt_at
}

fn feedback_backoff_delay(failure_count: u32) -> chrono::Duration {
    let steps = failure_count.saturating_sub(1).min(5);
    let minutes = 1u64 << steps;
    chrono::Duration::seconds((minutes.min(30)) as i64 * 60)
}

pub(crate) fn dedupe_recent_event_signature(
    app_data_dir: &Path,
    signature: impl Into<String>,
    dedupe_window: chrono::Duration,
) -> bool {
    let signature = signature.into();
    let mut config = load_config(app_data_dir);
    let now = chrono::Utc::now();
    let window_start = now - chrono::Duration::hours(24);

    config.recent_event_signatures.retain(|entry| {
        chrono::DateTime::parse_from_rfc3339(&entry.recorded_at)
            .map(|ts| ts.with_timezone(&chrono::Utc) >= window_start)
            .unwrap_or(false)
    });

    if config.recent_event_signatures.iter().any(|entry| {
        entry.signature == signature
            && chrono::DateTime::parse_from_rfc3339(&entry.recorded_at)
                .map(|ts| now - ts.with_timezone(&chrono::Utc) < dedupe_window)
                .unwrap_or(false)
    }) {
        save_config(app_data_dir, &config);
        return false;
    }

    config.recent_event_signatures.push(RecentEventSignature {
        signature,
        recorded_at: now.to_rfc3339(),
    });
    if config.recent_event_signatures.len() > 64 {
        let drain = config.recent_event_signatures.len() - 64;
        config.recent_event_signatures.drain(0..drain);
    }
    save_config(app_data_dir, &config);
    true
}

/// Initialize telemetry state from persisted config.
pub fn init(app_data_dir: &Path) {
    let mut config = load_config(app_data_dir);
    if config.install_id.is_none() {
        config.install_id = Some(generate_anonymous_id("inst", &anonymous_device_id()));
    }
    config.session_id = Some(generate_anonymous_id(
        "sess",
        config.install_id.as_deref().unwrap_or("unknown"),
    ));
    config.session_started_at = Some(chrono::Utc::now().to_rfc3339());
    save_config(app_data_dir, &config);
    TELEMETRY_ENABLED.store(config.enabled, Ordering::SeqCst);

    // Ensure logs directory exists
    let logs_dir = app_data_dir.join("logs");
    let _ = std::fs::create_dir_all(logs_dir);

    anonymous::start_background_flusher(app_data_dir.to_path_buf());
}

pub fn is_enabled() -> bool {
    TELEMETRY_ENABLED.load(Ordering::SeqCst)
}

pub fn set_enabled(app_data_dir: &Path, enabled: bool) {
    TELEMETRY_ENABLED.store(enabled, Ordering::SeqCst);
    let mut config = load_config(app_data_dir);
    config.enabled = enabled;
    save_config(app_data_dir, &config);
}

pub fn is_acknowledged(app_data_dir: &Path) -> bool {
    load_config(app_data_dir).acknowledged
}

pub fn set_acknowledged(app_data_dir: &Path) {
    let mut config = load_config(app_data_dir);
    config.acknowledged = true;
    save_config(app_data_dir, &config);
}

pub fn is_network_enabled(app_data_dir: &Path) -> bool {
    load_config(app_data_dir).network_enabled
}

pub fn set_network_enabled(app_data_dir: &Path, enabled: bool) {
    let mut config = load_config(app_data_dir);
    config.network_enabled = enabled;
    save_config(app_data_dir, &config);
}

// ---------------------------------------------------------------------------
// Crash log
// ---------------------------------------------------------------------------

fn crash_log_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("logs").join("crash.log")
}

/// Append a crash entry to the local crash log file.
pub fn write_crash_log(app_data_dir: &Path, entry: &str) {
    let path = crash_log_path(app_data_dir);
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));

    use std::io::Write;
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let _ = writeln!(file, "[{timestamp}] {entry}");
    }
}

/// Read the crash log contents.
pub fn read_crash_log(app_data_dir: &Path) -> String {
    let path = crash_log_path(app_data_dir);
    std::fs::read_to_string(path).unwrap_or_default()
}

// ---------------------------------------------------------------------------
// Sanitization
// ---------------------------------------------------------------------------

/// Remove local file paths from text, replacing with [path].
pub fn sanitize_paths(text: &str) -> String {
    // Match common path patterns: C:\..., /Users/..., /home/..., etc.
    let re_win = regex_lite::Regex::new(r"[A-Z]:\\[^\s:]+")
        .unwrap_or_else(|_| regex_lite::Regex::new(r"NOMATCH").unwrap());
    let re_unix = regex_lite::Regex::new(r"(/(?:Users|home|tmp|var|opt|mnt)[^\s:]+)")
        .unwrap_or_else(|_| regex_lite::Regex::new(r"NOMATCH").unwrap());

    let result = re_win.replace_all(text, "[path]");
    re_unix.replace_all(&result, "[path]").to_string()
}

/// Collect stderr tail (last N lines) from FFmpeg output.
#[allow(dead_code)]
pub fn collect_stderr_tail(stderr: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = stderr.lines().collect();
    let start = lines.len().saturating_sub(max_lines);
    let tail = lines[start..].join("\n");
    sanitize_paths(&tail)
}

// ---------------------------------------------------------------------------
// Remote reporting (fire-and-forget)
// ---------------------------------------------------------------------------

/// Report a crash to the telemetry endpoint. Non-blocking, fire-and-forget.
/// In MVP, this logs locally. The HTTP endpoint can be configured later.
/// Rate-limited to MAX_DAILY_REPORTS per device per day.
/// PIPL compliance: will not report if user has not acknowledged privacy policy.
pub fn report_crash(app_data_dir: &Path, report: &CrashReport) {
    if !is_enabled() || !is_acknowledged(app_data_dir) || !check_rate_limit() {
        return;
    }

    // Write to local log
    let entry = format!(
        "PANIC: {} | device={} | version={}",
        sanitize_paths(&report.panic_message),
        report.anonymous_device_id,
        report.app_version
    );
    write_crash_log(app_data_dir, &entry);

    // TODO: When telemetry endpoint is configured, POST report here.
    // For MVP, local logging is sufficient. The endpoint URL will be
    // configured via environment variable HIDDENSHIELD_TELEMETRY_URL.
    log::info!("Crash report recorded locally (remote endpoint not yet configured)");

    anonymous::record_crash_event(
        app_data_dir,
        "panic",
        "system",
        0,
        None,
        report.panic_message.clone(),
        report.backtrace.clone(),
        None,
    );
}

/// Report an FFmpeg crash. Non-blocking. Rate-limited.
/// PIPL compliance: will not report if user has not acknowledged privacy policy.
pub fn report_ffmpeg_crash(app_data_dir: &Path, report: &FfmpegCrashReport) {
    if !is_enabled() || !is_acknowledged(app_data_dir) || !check_rate_limit() {
        return;
    }

    let entry = format!(
        "FFMPEG_CRASH: exit_code={} encoder={} format={} | stderr: {}",
        report.exit_code, report.encoder, report.input_format, report.stderr_tail
    );
    write_crash_log(app_data_dir, &entry);

    log::info!("FFmpeg crash report recorded locally");

    anonymous::record_crash_event(
        app_data_dir,
        "ffmpeg_crash",
        "video",
        0,
        None,
        format!(
            "exit_code={} encoder={} format={}",
            report.exit_code, report.encoder, report.input_format
        ),
        report.stderr_tail.clone(),
        None,
    );
}

/// Check and increment the daily rate limit. Returns true if under limit.
fn check_rate_limit() -> bool {
    let today = current_day_number();
    let stored_day = DAILY_REPORT_DAY.load(Ordering::SeqCst);

    if stored_day != today {
        // New day — reset counter
        DAILY_REPORT_DAY.store(today, Ordering::SeqCst);
        DAILY_REPORT_COUNT.store(1, Ordering::SeqCst);
        return true;
    }

    let count = DAILY_REPORT_COUNT.fetch_add(1, Ordering::SeqCst);
    count < MAX_DAILY_REPORTS
}

/// Returns a day number (days since epoch) for rate-limit bucketing.
fn current_day_number() -> u32 {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    (secs / 86400) as u32
}

// ---------------------------------------------------------------------------
// Data usage calculation
// ---------------------------------------------------------------------------

pub fn calculate_data_usage(app_data_dir: &Path) -> DataUsageInfo {
    let ffmpeg_size = file_size_mb(&app_data_dir.join("ffmpeg"))
        + file_size_mb(&app_data_dir.join("ffmpeg.exe"))
        + file_size_mb(&app_data_dir.join("ffprobe"))
        + file_size_mb(&app_data_dir.join("ffprobe.exe"));

    let db_size = file_size_mb(&app_data_dir.join("vault.db"));

    let logs_dir = app_data_dir.join("logs");
    let log_size = dir_size_mb(&logs_dir);

    let total = ffmpeg_size + db_size + log_size;

    DataUsageInfo {
        ffmpeg_size_mb: round2(ffmpeg_size),
        db_size_mb: round2(db_size),
        log_size_mb: round2(log_size),
        total_size_mb: round2(total),
    }
}

fn file_size_mb(path: &Path) -> f64 {
    std::fs::metadata(path)
        .map(|m| m.len() as f64 / 1024.0 / 1024.0)
        .unwrap_or(0.0)
}

fn dir_size_mb(path: &Path) -> f64 {
    if !path.is_dir() {
        return 0.0;
    }
    let mut total: u64 = 0;
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                total += meta.len();
            }
        }
    }
    total as f64 / 1024.0 / 1024.0
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}

// ---------------------------------------------------------------------------
// Panic hook setup
// ---------------------------------------------------------------------------

/// Install a global panic hook that logs crashes and optionally reports them.
pub fn install_panic_hook(app_data_dir: PathBuf) {
    let default_hook = std::panic::take_hook();

    std::panic::set_hook(Box::new(move |info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            s.to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "unknown panic".to_string()
        };

        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "unknown location".to_string());

        let backtrace = std::backtrace::Backtrace::force_capture().to_string();
        let sanitized_bt = sanitize_paths(&backtrace);

        let report = CrashReport {
            panic_message: format!("{message} at {location}"),
            backtrace: sanitized_bt,
            os_version: os_version(),
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            anonymous_device_id: anonymous_device_id(),
        };

        // Always write to local log
        let entry = format!(
            "PANIC: {} at {}\nBacktrace:\n{}",
            sanitize_paths(&message),
            sanitize_paths(&location),
            report.backtrace
        );
        write_crash_log(&app_data_dir, &entry);

        // Report remotely if enabled
        report_crash(&app_data_dir, &report);

        // Call the default hook so the process still aborts normally
        default_hook(info);
    }));
}

fn os_version() -> String {
    format!(
        "{} {} {}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn sanitize_paths_masks_windows_and_unix_paths() {
        let input = "panic at C:\\Users\\jihx\\Desktop\\secret.mp4 and /home/jihx/video.mov";
        let output = sanitize_paths(input);
        assert!(!output.contains("C:\\Users\\jihx\\Desktop\\secret.mp4"));
        assert!(!output.contains("/home/jihx/video.mov"));
        assert!(output.contains("[path]"));
    }

    #[test]
    fn crash_report_writes_sanitized_log() {
        let temp = TempDir::new().unwrap();
        let app_data_dir = temp.path().to_path_buf();
        set_enabled(&app_data_dir, true);
        set_acknowledged(&app_data_dir);

        let report = CrashReport {
            panic_message: "panic at C:\\Users\\jihx\\Desktop\\secret.mp4".to_string(),
            backtrace: "frame /home/jihx/project/src/main.rs".to_string(),
            os_version: "windows".to_string(),
            app_version: "0.1.0".to_string(),
            anonymous_device_id: "device-1".to_string(),
        };

        report_crash(&app_data_dir, &report);
        let log = read_crash_log(&app_data_dir);
        assert!(log.contains("PANIC:"));
        assert!(!log.contains("C:\\Users\\jihx\\Desktop\\secret.mp4"));
        assert!(!log.contains("/home/jihx/project/src/main.rs"));
    }
}

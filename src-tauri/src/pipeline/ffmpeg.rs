use std::path::{Path, PathBuf};
use std::process::Stdio;

use serde::Deserialize;
use tokio::process::{Child, Command};

use super::error::PipelineError;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Cached paths to ffmpeg and ffprobe binaries.
#[derive(Debug, Clone)]
pub struct FfmpegPaths {
    pub ffmpeg: PathBuf,
    pub ffprobe: PathBuf,
}

/// Wrapper around a tokio child process for an FFmpeg invocation.
pub struct FfmpegChild {
    pub child: Child,
}

/// Minimal representation of ffprobe JSON output.
#[derive(Debug, Clone, Deserialize)]
pub struct FfprobeOutput {
    pub streams: Vec<FfprobeStream>,
    pub format: Option<FfprobeFormat>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct FfprobeStream {
    pub codec_type: Option<String>,
    pub codec_name: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    #[serde(default, deserialize_with = "deserialize_string_f64")]
    pub r_frame_rate: Option<f64>,
    pub pix_fmt: Option<String>,
    pub color_transfer: Option<String>,
    pub color_primaries: Option<String>,
    pub color_space: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct FfprobeFormat {
    #[serde(default, deserialize_with = "deserialize_opt_string_f64")]
    pub duration: Option<f64>,
    #[serde(default, deserialize_with = "deserialize_opt_string_u64")]
    pub size: Option<u64>,
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Detect ffmpeg/ffprobe from the system PATH only.
///
/// Production builds must not trust executables stored in user-writable app
/// data directories, because those binaries can be stale or tampered with.
pub async fn detect_ffmpeg() -> Result<FfmpegPaths, PipelineError> {
    if let (Some(ffmpeg), Some(ffprobe)) =
        (which_binary("ffmpeg").await, which_binary("ffprobe").await)
    {
        validate_binary(&ffmpeg, "ffmpeg").await?;
        validate_binary(&ffprobe, "ffprobe").await?;
        return Ok(FfmpegPaths { ffmpeg, ffprobe });
    }

    Err(PipelineError::FfmpegNotFound)
}

// ---------------------------------------------------------------------------
// ffprobe
// ---------------------------------------------------------------------------

/// Run ffprobe on `input` and parse the JSON output.
pub async fn ffprobe_source(ffprobe: &Path, input: &str) -> Result<FfprobeOutput, PipelineError> {
    let output = Command::new(ffprobe)
        .args([
            "-v",
            "quiet",
            "-print_format",
            "json",
            "-show_streams",
            "-show_format",
            input,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| PipelineError::ProbeFailed(format!("spawn ffprobe: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(PipelineError::ProbeFailed(format!(
            "ffprobe exited with {}: {stderr}",
            output.status
        )));
    }

    let json_str = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str::<FfprobeOutput>(&json_str)
        .map_err(|e| PipelineError::ProbeFailed(format!("parse ffprobe JSON: {e}")))
}

// ---------------------------------------------------------------------------
// spawn FFmpeg
// ---------------------------------------------------------------------------

/// Spawn an FFmpeg child process with the given arguments.
/// Uses `kill_on_drop(true)` to ensure child processes are terminated
/// if the parent Tauri process crashes or is force-closed.
pub async fn spawn_ffmpeg(ffmpeg: &Path, args: &[String]) -> Result<FfmpegChild, PipelineError> {
    let child = Command::new(ffmpeg)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null()) // 关键：防止 stdout 管道填满导致子进程死锁
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
        .map_err(|e| PipelineError::FfmpegFailed(format!("spawn ffmpeg: {e}")))?;

    Ok(FfmpegChild { child })
}

// ---------------------------------------------------------------------------
// Progress parsing
// ---------------------------------------------------------------------------

/// Parse a `time=HH:MM:SS.ms` field from an FFmpeg stderr line and return
/// progress as a fraction in `[0.0, 1.0]`.
///
/// Returns `None` if the line does not contain a valid `time=` field or
/// `total_duration` is not positive.
pub fn parse_progress_line(line: &str, total_duration: f64) -> Option<f64> {
    if total_duration <= 0.0 {
        return None;
    }

    let time_str = extract_time_value(line)?;
    let seconds = parse_time_to_seconds(time_str)?;

    let progress = (seconds / total_duration).clamp(0.0, 1.0);
    Some(progress)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn binary_names() -> (&'static str, &'static str) {
    if cfg!(target_os = "windows") {
        ("ffmpeg.exe", "ffprobe.exe")
    } else {
        ("ffmpeg", "ffprobe")
    }
}

async fn which_binary(name: &str) -> Option<PathBuf> {
    let cmd = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    let output = Command::new(cmd)
        .arg(name)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output()
        .await
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let first_line = stdout.lines().next()?.trim();
    if first_line.is_empty() {
        return None;
    }

    let path = PathBuf::from(first_line);
    if path.is_file() {
        std::fs::canonicalize(&path).ok().or(Some(path))
    } else {
        None
    }
}

async fn validate_binary(path: &Path, binary_name: &str) -> Result<(), PipelineError> {
    let output = Command::new(path)
        .arg("-version")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| {
            PipelineError::FfmpegFailed(format!(
                "spawn {binary_name} health check at {}: {e}",
                path.display()
            ))
        })?;

    if output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let banner = first_non_empty_line(&stdout)
            .or_else(|| first_non_empty_line(&stderr))
            .unwrap_or("");
        if is_expected_version_banner(binary_name, banner) {
            return Ok(());
        }

        return Err(PipelineError::FfmpegFailed(format!(
            "{binary_name} health check returned success at {} but banner was unexpected: {}",
            path.display(),
            if banner.is_empty() {
                "no diagnostic output"
            } else {
                banner
            }
        )));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let detail = first_non_empty_line(&stderr)
        .or_else(|| first_non_empty_line(&stdout))
        .unwrap_or("no diagnostic output");

    Err(PipelineError::FfmpegFailed(format!(
        "{binary_name} health check failed at {} with {}: {}",
        path.display(),
        output.status,
        detail
    )))
}

fn first_non_empty_line(text: &str) -> Option<&str> {
    text.lines().map(str::trim).find(|line| !line.is_empty())
}

fn is_expected_version_banner(binary_name: &str, banner: &str) -> bool {
    let banner_lower = banner.to_ascii_lowercase();
    banner_lower.contains(binary_name) && banner_lower.contains("version")
}

fn extract_time_value(line: &str) -> Option<&str> {
    let idx = line.find("time=")?;
    let rest = &line[idx + 5..];
    // Take until the next space or end of string
    let end = rest.find([' ', '\r', '\n']).unwrap_or(rest.len());
    let val = &rest[..end];
    if val.is_empty() {
        None
    } else {
        Some(val)
    }
}

fn parse_time_to_seconds(time_str: &str) -> Option<f64> {
    // Expected format: HH:MM:SS.ms (e.g. "01:23:45.67")
    let parts: Vec<&str> = time_str.split(':').collect();
    if parts.len() != 3 {
        return None;
    }
    let hours: f64 = parts[0].parse().ok()?;
    let minutes: f64 = parts[1].parse().ok()?;
    let seconds: f64 = parts[2].parse().ok()?;
    Some(hours * 3600.0 + minutes * 60.0 + seconds)
}

// ---------------------------------------------------------------------------
// Custom serde helpers for ffprobe JSON (values are strings like "30/1")
// ---------------------------------------------------------------------------

fn deserialize_string_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        None => Ok(None),
        Some(s) => Ok(parse_rational_or_float(&s)),
    }
}

fn deserialize_opt_string_f64<'de, D>(deserializer: D) -> Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        None => Ok(None),
        Some(s) => Ok(s.parse::<f64>().ok()),
    }
}

fn deserialize_opt_string_u64<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s: Option<String> = Option::deserialize(deserializer)?;
    match s {
        None => Ok(None),
        Some(s) => Ok(s.parse::<u64>().ok()),
    }
}

/// Parse "30/1" or "29.97" style strings into f64.
fn parse_rational_or_float(s: &str) -> Option<f64> {
    if let Some((num, den)) = s.split_once('/') {
        let n: f64 = num.parse().ok()?;
        let d: f64 = den.parse().ok()?;
        if d == 0.0 {
            None
        } else {
            Some(n / d)
        }
    } else {
        s.parse::<f64>().ok()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_progress_line_valid() {
        let line = "frame=  100 fps=25 time=00:01:30.50 bitrate=1234kbits/s";
        let result = parse_progress_line(line, 180.0);
        assert!(result.is_some());
        let progress = result.unwrap();
        // 1*60 + 30.50 = 90.50 / 180.0 ≈ 0.5028
        assert!((progress - 90.5 / 180.0).abs() < 0.001);
    }

    #[test]
    fn test_parse_progress_line_no_time() {
        let line = "Press [q] to stop, [?] for help";
        assert!(parse_progress_line(line, 100.0).is_none());
    }

    #[test]
    fn test_parse_progress_line_zero_duration() {
        let line = "time=00:00:10.00";
        assert!(parse_progress_line(line, 0.0).is_none());
    }

    #[test]
    fn test_parse_progress_line_negative_duration() {
        let line = "time=00:00:10.00";
        assert!(parse_progress_line(line, -5.0).is_none());
    }

    #[test]
    fn test_parse_progress_line_clamped_to_one() {
        // time exceeds total duration
        let line = "time=00:05:00.00";
        let result = parse_progress_line(line, 60.0).unwrap();
        assert!((result - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_parse_time_to_seconds() {
        assert_eq!(parse_time_to_seconds("00:00:00.00"), Some(0.0));
        assert_eq!(parse_time_to_seconds("01:00:00.00"), Some(3600.0));
        assert!((parse_time_to_seconds("00:01:30.50").unwrap() - 90.5).abs() < 0.001);
    }

    #[test]
    fn test_parse_rational_or_float() {
        assert_eq!(parse_rational_or_float("30/1"), Some(30.0));
        assert_eq!(
            parse_rational_or_float("60000/1001"),
            Some(60000.0 / 1001.0)
        );
        assert_eq!(parse_rational_or_float("29.97"), Some(29.97));
        assert_eq!(parse_rational_or_float("0/0"), None);
    }

    #[test]
    fn test_extract_time_value() {
        assert_eq!(
            extract_time_value("frame=100 time=00:01:30.50 bitrate=1234"),
            Some("00:01:30.50")
        );
        assert_eq!(extract_time_value("no time here"), None);
        assert_eq!(extract_time_value("time="), None);
    }

    #[test]
    fn test_binary_names() {
        let (ff, fp) = binary_names();
        assert!(!ff.is_empty());
        assert!(!fp.is_empty());
    }

    #[test]
    fn test_first_non_empty_line() {
        assert_eq!(
            first_non_empty_line("\n  \nffmpeg version 7.0"),
            Some("ffmpeg version 7.0")
        );
        assert_eq!(first_non_empty_line("   \n\t"), None);
    }

    #[test]
    fn version_banner_requires_binary_name_and_version() {
        assert!(is_expected_version_banner("ffmpeg", "ffmpeg version 7.1"));
        assert!(is_expected_version_banner(
            "ffprobe",
            "ffprobe version N-12345"
        ));
        assert!(!is_expected_version_banner(
            "ffmpeg",
            "custom wrapper ready"
        ));
        assert!(!is_expected_version_banner("ffprobe", "ffmpeg version 7.1"));
    }
}

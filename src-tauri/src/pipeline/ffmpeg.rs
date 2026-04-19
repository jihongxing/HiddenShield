use std::io::Read;
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

/// Detect ffmpeg/ffprobe: first check `app_data_dir`, then fall back to PATH.
/// Validates that detected binaries are actually executable (not truncated).
///
/// NOTE (AV Heuristic Mitigation): The bundled FFmpeg binary is stored with its
/// original name. If antivirus software (360, McAfee, etc.) flags it due to
/// heuristic behavior analysis (process spawning + large file I/O + binary rewrite),
/// users should add the app data directory to their AV whitelist. Future versions
/// may rename the binary (e.g., `hs_engine`) to reduce false-positive triggers.
pub async fn detect_ffmpeg(app_data_dir: &Path) -> Result<FfmpegPaths, PipelineError> {
    // 1. Check AppData directory
    let (ffmpeg_name, ffprobe_name) = binary_names();
    let local_ffmpeg = app_data_dir.join(ffmpeg_name);
    let local_ffprobe = app_data_dir.join(ffprobe_name);

    if local_ffmpeg.exists() && local_ffprobe.exists() {
        // Validate: try running `ffmpeg -version` to ensure it's not truncated
        if validate_ffmpeg_binary(&local_ffmpeg).await {
            return Ok(FfmpegPaths {
                ffmpeg: local_ffmpeg,
                ffprobe: local_ffprobe,
            });
        }
        // Binary is corrupt/truncated — remove and fall through
        log::warn!("Local FFmpeg binary appears corrupt, removing");
        let _ = std::fs::remove_file(&local_ffmpeg);
        let _ = std::fs::remove_file(&local_ffprobe);
    }

    // 2. Fall back to system PATH via `which` (unix) / `where` (windows)
    if let (Some(ffmpeg), Some(ffprobe)) = (which_binary("ffmpeg").await, which_binary("ffprobe").await) {
        return Ok(FfmpegPaths {
            ffmpeg,
            ffprobe,
        });
    }

    Err(PipelineError::FfmpegNotFound)
}

/// Quick validation: run `ffmpeg -version` and check it exits successfully.
async fn validate_ffmpeg_binary(ffmpeg: &Path) -> bool {
    let result = Command::new(ffmpeg)
        .args(["-version"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await;
    matches!(result, Ok(output) if output.status.success())
}

// ---------------------------------------------------------------------------
// Download
// ---------------------------------------------------------------------------

/// Download FFmpeg binaries from a pre-configured CDN/GitHub Releases URL.
///
/// Steps: download archive → verify SHA-256 → extract ffmpeg + ffprobe.
/// `on_progress` is called with (bytes_downloaded, total_bytes).
pub async fn download_ffmpeg(
    app_data_dir: &Path,
    on_progress: impl Fn(u64, u64),
) -> Result<FfmpegPaths, PipelineError> {
    use sha2::{Digest, Sha256};
    use tokio::io::AsyncWriteExt;

    let (url, expected_sha256) = download_url_and_hash();
    let (ffmpeg_name, ffprobe_name) = binary_names();

    std::fs::create_dir_all(app_data_dir)
        .map_err(|e| PipelineError::FfmpegDownloadFailed(format!("create dir: {e}")))?;

    let archive_ext = if url.ends_with(".tar.xz") { "tar.xz" } else { "zip" };
    let archive_path = app_data_dir.join(format!("ffmpeg-download.{archive_ext}"));
    // Download to .tmp first — only rename after SHA verification to prevent
    // incomplete/corrupt files from being mistaken as valid on next startup.
    let tmp_path = app_data_dir.join(format!("ffmpeg-download.{archive_ext}.tmp"));

    // Stream download
    let response = reqwest::get(url)
        .await
        .map_err(|e| PipelineError::FfmpegDownloadFailed(format!("request: {e}")))?;

    let total = response.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut hasher = Sha256::new();

    let mut file = tokio::fs::File::create(&tmp_path)
        .await
        .map_err(|e| PipelineError::FfmpegDownloadFailed(format!("create file: {e}")))?;

    let mut stream = response.bytes_stream();
    use futures_util::StreamExt;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| PipelineError::FfmpegDownloadFailed(format!("stream: {e}")))?;
        hasher.update(&chunk);
        file.write_all(&chunk)
            .await
            .map_err(|e| PipelineError::FfmpegDownloadFailed(format!("write: {e}")))?;
        downloaded += chunk.len() as u64;
        on_progress(downloaded, total);
    }
    file.flush().await.ok();
    drop(file);

    // Verify SHA-256 (skip if expected hash is empty — dev mode)
    let hash = format!("{:x}", hasher.finalize());
    if !expected_sha256.is_empty() && hash != expected_sha256 {
        let _ = tokio::fs::remove_file(&tmp_path).await;
        return Err(PipelineError::FfmpegDownloadFailed(format!(
            "SHA-256 mismatch: expected {expected_sha256}, got {hash}"
        )));
    }

    // Atomic rename: .tmp → final archive path (only after verification)
    tokio::fs::rename(&tmp_path, &archive_path)
        .await
        .map_err(|e| PipelineError::FfmpegDownloadFailed(format!("rename tmp: {e}")))?;

    // Extract binaries (placeholder: in production this would unzip/untar)
    // For now we assume the archive is a zip containing ffmpeg and ffprobe at the root.
    extract_archive(&archive_path, app_data_dir)
        .map_err(|e| PipelineError::FfmpegDownloadFailed(format!("extract: {e}")))?;

    let _ = std::fs::remove_file(&archive_path);

    let ffmpeg_path = app_data_dir.join(ffmpeg_name);
    let ffprobe_path = app_data_dir.join(ffprobe_name);

    if !ffmpeg_path.exists() || !ffprobe_path.exists() {
        return Err(PipelineError::FfmpegDownloadFailed(
            "extracted archive does not contain ffmpeg/ffprobe".into(),
        ));
    }

    // Make executable on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        let _ = std::fs::set_permissions(&ffmpeg_path, perms.clone());
        let _ = std::fs::set_permissions(&ffprobe_path, perms);
    }

    Ok(FfmpegPaths {
        ffmpeg: ffmpeg_path,
        ffprobe: ffprobe_path,
    })
}

// ---------------------------------------------------------------------------
// ffprobe
// ---------------------------------------------------------------------------

/// Run ffprobe on `input` and parse the JSON output.
pub async fn ffprobe_source(
    ffprobe: &Path,
    input: &str,
) -> Result<FfprobeOutput, PipelineError> {
    let output = Command::new(ffprobe)
        .args([
            "-v", "quiet",
            "-print_format", "json",
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
pub async fn spawn_ffmpeg(
    ffmpeg: &Path,
    args: &[String],
) -> Result<FfmpegChild, PipelineError> {
    let child = Command::new(ffmpeg)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())   // 关键：防止 stdout 管道填满导致子进程死锁
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
    let cmd = if cfg!(target_os = "windows") { "where" } else { "which" };
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
    if path.exists() { Some(path) } else { None }
}

fn extract_time_value(line: &str) -> Option<&str> {
    let idx = line.find("time=")?;
    let rest = &line[idx + 5..];
    // Take until the next space or end of string
    let end = rest.find([' ', '\r', '\n']).unwrap_or(rest.len());
    let val = &rest[..end];
    if val.is_empty() { None } else { Some(val) }
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

/// Platform-specific download URL and expected SHA-256 hash.
/// Uses both OS and ARCH to select the correct binary.
/// SHA-256 set to empty string means "skip verification" (used during development).
/// Before release, pin to a specific version with known hash.
fn download_url_and_hash() -> (&'static str, &'static str) {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    match (os, arch) {
        ("windows", "x86_64") => (
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip",
            "",
        ),
        ("windows", "aarch64") => (
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-winarm64-gpl.zip",
            "",
        ),
        ("macos", "aarch64") => (
            "https://evermeet.cx/ffmpeg/getrelease/zip",
            "",
        ),
        ("macos", "x86_64") => (
            "https://evermeet.cx/ffmpeg/getrelease/zip",
            "",
        ),
        ("linux", "x86_64") => (
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz",
            "",
        ),
        ("linux", "aarch64") => (
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linuxarm64-gpl.tar.xz",
            "",
        ),
        _ => (
            "https://github.com/BtbN/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-linux64-gpl.tar.xz",
            "",
        ),
    }
}

/// Extract ffmpeg/ffprobe from a downloaded archive into `dest_dir`.
/// Supports .zip (Windows/macOS) and .tar.xz (Linux) archives.
fn extract_archive(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    if !archive_path.exists() {
        return Err("archive file not found".into());
    }

    let (ffmpeg_name, ffprobe_name) = binary_names();
    let archive_str = archive_path.to_string_lossy();

    if archive_str.ends_with(".tar.xz") || archive_str.ends_with(".txz") {
        extract_tar_xz(archive_path, dest_dir, ffmpeg_name, ffprobe_name)
    } else {
        extract_zip(archive_path, dest_dir, ffmpeg_name, ffprobe_name)
    }
}

fn extract_zip(
    archive_path: &Path,
    dest_dir: &Path,
    ffmpeg_name: &str,
    ffprobe_name: &str,
) -> Result<(), String> {
    let file = std::fs::File::open(archive_path)
        .map_err(|e| format!("open archive: {e}"))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|e| format!("read zip: {e}"))?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)
            .map_err(|e| format!("zip entry {i}: {e}"))?;

        let entry_name = entry.name().to_string();
        let file_name = Path::new(&entry_name)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if file_name == ffmpeg_name || file_name == ffprobe_name {
            let out_path = dest_dir.join(&file_name);
            let mut out_file = std::fs::File::create(&out_path)
                .map_err(|e| format!("create {file_name}: {e}"))?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)
                .map_err(|e| format!("read {file_name}: {e}"))?;
            std::io::Write::write_all(&mut out_file, &buf)
                .map_err(|e| format!("write {file_name}: {e}"))?;
        }
    }

    Ok(())
}

fn extract_tar_xz(
    archive_path: &Path,
    dest_dir: &Path,
    ffmpeg_name: &str,
    ffprobe_name: &str,
) -> Result<(), String> {
    let file = std::fs::File::open(archive_path)
        .map_err(|e| format!("open archive: {e}"))?;
    let decompressor = xz2::read::XzDecoder::new(file);
    let mut archive = tar::Archive::new(decompressor);

    let entries = archive.entries()
        .map_err(|e| format!("read tar entries: {e}"))?;

    for entry in entries {
        let mut entry = entry.map_err(|e| format!("tar entry: {e}"))?;
        let path = entry.path().map_err(|e| format!("tar path: {e}"))?.into_owned();
        let file_name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        if file_name == ffmpeg_name || file_name == ffprobe_name {
            let out_path = dest_dir.join(&file_name);
            let mut out_file = std::fs::File::create(&out_path)
                .map_err(|e| format!("create {file_name}: {e}"))?;
            let mut buf = Vec::new();
            entry.read_to_end(&mut buf)
                .map_err(|e| format!("read {file_name}: {e}"))?;
            std::io::Write::write_all(&mut out_file, &buf)
                .map_err(|e| format!("write {file_name}: {e}"))?;
        }
    }

    Ok(())
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
        if d == 0.0 { None } else { Some(n / d) }
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
        assert_eq!(parse_rational_or_float("60000/1001"), Some(60000.0 / 1001.0));
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
}

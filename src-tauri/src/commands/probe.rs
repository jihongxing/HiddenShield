use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::encoder::tonemap;
use crate::pipeline::ffmpeg;
use crate::pipeline::scheduler::classify_file;
use crate::pipeline::scheduler::FileType;
use crate::utils::hash;
use crate::AppState;

// ---------------------------------------------------------------------------
// System Check
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemCheckResult {
    pub ffmpeg_available: bool,
    pub ffmpeg_version: String,
    pub gpu_encoder_available: bool,
    pub gpu_encoder_name: String,
    pub disk_free_mb: u64,
    pub disk_sufficient: bool,
    pub output_dir_writable: bool,
    pub output_dir: String,
}

#[tauri::command]
pub async fn system_check(app_handle: AppHandle) -> Result<SystemCheckResult, String> {
    let state = app_handle.state::<AppState>();

    // 1. FFmpeg availability & version
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {e}"))?;

    let (ffmpeg_available, ffmpeg_version) = match ffmpeg::detect_ffmpeg(&app_data_dir).await {
        Ok(paths) => {
            let _ = state.ffmpeg_paths.set(paths.clone());
            let version = get_ffmpeg_version(&paths.ffmpeg).await;
            (true, version)
        }
        Err(_) => (false, "未安装".to_string()),
    };

    // 2. GPU encoder availability (from hw_info cache or detect)
    let (gpu_encoder_available, gpu_encoder_name) = if let Some(hw) = state.hw_info.get() {
        let is_hw = hw.hw_type != crate::encoder::hw_detect::HwEncoderType::Software;
        (is_hw, hw.preferred_encoder.clone())
    } else if let Some(paths) = state.ffmpeg_paths.get() {
        let hw = crate::encoder::hw_detect::detect_hardware(&paths.ffmpeg).await;
        let is_hw = hw.hw_type != crate::encoder::hw_detect::HwEncoderType::Software;
        let name = hw.preferred_encoder.clone();
        let _ = state.hw_info.set(hw);
        (is_hw, name)
    } else {
        (false, "libx264".to_string())
    };

    // 3. Output directory & disk space
    let output_dir = app_data_dir
        .parent()
        .unwrap_or(&app_data_dir)
        .to_string_lossy()
        .to_string();
    let output_path = Path::new(&output_dir);

    let disk_free_mb = get_disk_free_mb(output_path);
    let disk_sufficient = disk_free_mb >= 500; // at least 500 MB

    // 4. Write permission check
    let output_dir_writable = check_write_permission(output_path);

    Ok(SystemCheckResult {
        ffmpeg_available,
        ffmpeg_version,
        gpu_encoder_available,
        gpu_encoder_name,
        disk_free_mb,
        disk_sufficient,
        output_dir_writable,
        output_dir,
    })
}

async fn get_ffmpeg_version(ffmpeg: &Path) -> String {
    use tokio::process::Command;
    let output = Command::new(ffmpeg)
        .args(["-version"])
        .output()
        .await;

    match output {
        Ok(o) => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.lines().next().unwrap_or("unknown").to_string()
        }
        Err(_) => "unknown".to_string(),
    }
}

fn get_disk_free_mb(path: &Path) -> u64 {
    // Reuse the platform-specific logic from system_guard via a simple wrapper
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        #[link(name = "kernel32")]
        extern "system" {
            fn GetDiskFreeSpaceExW(
                lpDirectoryName: *const u16,
                lpFreeBytesAvailableToCaller: *mut u64,
                lpTotalNumberOfBytes: *mut u64,
                lpTotalNumberOfFreeBytes: *mut u64,
            ) -> i32;
        }
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        let mut free_available: u64 = 0;
        let mut total: u64 = 0;
        let mut total_free: u64 = 0;
        let ret = unsafe {
            GetDiskFreeSpaceExW(wide.as_ptr(), &mut free_available, &mut total, &mut total_free)
        };
        if ret == 0 { 0 } else { free_available / (1024 * 1024) }
    }
    #[cfg(unix)]
    {
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let c_path = match CString::new(path.as_os_str().as_bytes()) {
            Ok(p) => p,
            Err(_) => return 0,
        };
        unsafe {
            let mut stat: libc::statvfs = std::mem::zeroed();
            if libc::statvfs(c_path.as_ptr(), &mut stat) != 0 {
                return 0;
            }
            (stat.f_bavail as u64 * stat.f_frsize as u64) / (1024 * 1024)
        }
    }
    #[cfg(not(any(windows, unix)))]
    { 0 }
}

fn check_write_permission(dir: &Path) -> bool {
    let test_file = dir.join(".hs_write_test");
    match std::fs::File::create(&test_file) {
        Ok(_) => {
            let _ = std::fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceMeta {
  pub file_name: String,
  pub path: String,
  pub width: u32,
  pub height: u32,
  pub fps: f64,
  pub duration_secs: f64,
  pub file_size_mb: f64,
  pub is_hdr: bool,
  pub color_profile: String,
  pub sha256: String,
  pub file_type: String,
}

#[tauri::command]
pub async fn probe_source(path: String, app_handle: AppHandle) -> Result<SourceMeta, String> {
  let p = Path::new(&path);

  // --- Cloud-sync placeholder detection ---
  check_cloud_placeholder(p)?;

  let file_name = p
    .file_name()
    .and_then(|item| item.to_str())
    .unwrap_or(&path)
    .to_string();

  let file_size_mb = std::fs::metadata(&path)
    .map(|m| m.len() as f64 / 1024.0 / 1024.0)
    .unwrap_or(0.0);
  let file_size_mb = (file_size_mb * 10.0).round() / 10.0;

  let sha256 = hash::sha256_of_file(&path).map_err(|e| format!("SHA-256 计算失败: {e}"))?;

  let file_type = classify_file(p);

  match file_type {
    FileType::Video => probe_video(&path, file_name, file_size_mb, sha256, &app_handle).await,
    FileType::Image => probe_image(&path, file_name, file_size_mb, sha256),
    FileType::Audio => probe_audio(&path, file_name, file_size_mb, sha256, &app_handle).await,
  }
}

async fn probe_video(
  path: &str,
  file_name: String,
  file_size_mb: f64,
  sha256: String,
  app_handle: &AppHandle,
) -> Result<SourceMeta, String> {
  let ffprobe_path = resolve_ffprobe(app_handle).await?;

  let probe = ffmpeg::ffprobe_source(&ffprobe_path, path)
    .await
    .map_err(|e| format!("ffprobe 失败: {e}"))?;

  let video_stream = probe.streams.iter().find(|s| s.codec_type.as_deref() == Some("video"));

  let (width, height) = video_stream
    .map(|s| (s.width.unwrap_or(0), s.height.unwrap_or(0)))
    .unwrap_or((0, 0));

  let fps = video_stream.and_then(|s| s.r_frame_rate).unwrap_or(0.0);

  let duration_secs = probe
    .format
    .as_ref()
    .and_then(|f| f.duration)
    .unwrap_or(0.0);

  let is_hdr = video_stream
    .map(|s| tonemap::is_hdr(s.color_transfer.as_deref(), s.color_primaries.as_deref()))
    .unwrap_or(false);

  let color_profile = if is_hdr {
    "BT.2020 / PQ".to_string()
  } else {
    "BT.709 / SDR".to_string()
  };

  Ok(SourceMeta {
    file_name,
    path: path.to_string(),
    width,
    height,
    fps,
    duration_secs,
    file_size_mb,
    is_hdr,
    color_profile,
    sha256,
    file_type: "video".to_string(),
  })
}

fn probe_image(
  path: &str,
  file_name: String,
  file_size_mb: f64,
  sha256: String,
) -> Result<SourceMeta, String> {
  let (width, height) =
    image::image_dimensions(path).map_err(|e| format!("图片读取失败: {e}"))?;

  Ok(SourceMeta {
    file_name,
    path: path.to_string(),
    width,
    height,
    fps: 0.0,
    duration_secs: 0.0,
    file_size_mb,
    is_hdr: false,
    color_profile: "sRGB".to_string(),
    sha256,
    file_type: "image".to_string(),
  })
}

async fn probe_audio(
  path: &str,
  file_name: String,
  file_size_mb: f64,
  sha256: String,
  app_handle: &AppHandle,
) -> Result<SourceMeta, String> {
  let ffprobe_path = resolve_ffprobe(app_handle).await?;

  let probe = ffmpeg::ffprobe_source(&ffprobe_path, path)
    .await
    .map_err(|e| format!("ffprobe 失败: {e}"))?;

  let duration_secs = probe
    .format
    .as_ref()
    .and_then(|f| f.duration)
    .unwrap_or(0.0);

  Ok(SourceMeta {
    file_name,
    path: path.to_string(),
    width: 0,
    height: 0,
    fps: 0.0,
    duration_secs,
    file_size_mb,
    is_hdr: false,
    color_profile: String::new(),
    sha256,
    file_type: "audio".to_string(),
  })
}

// ---------------------------------------------------------------------------
// Cloud-sync placeholder detection (OneDrive / iCloud)
// ---------------------------------------------------------------------------

/// Check if a file is a cloud-sync placeholder (not fully downloaded).
/// On Windows, checks FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS.
/// On macOS, checks for iCloud `.icloud` stub files.
fn check_cloud_placeholder(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS = 0x00400000
        // FILE_ATTRIBUTE_OFFLINE = 0x00001000
        const RECALL_ON_DATA_ACCESS: u32 = 0x0040_0000;
        const OFFLINE: u32 = 0x0000_1000;

        if let Ok(meta) = std::fs::metadata(path) {
            let attrs = meta.file_attributes();
            if attrs & RECALL_ON_DATA_ACCESS != 0 || attrs & OFFLINE != 0 {
                return Err(
                    "该文件尚未下载到本地（云盘占位符），请先在资源管理器中双击下载后再拖入隐盾。".to_string()
                );
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        // iCloud stores placeholders as `.filename.icloud` in the same directory
        if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
            if file_name.starts_with('.') && file_name.ends_with(".icloud") {
                return Err(
                    "该文件尚未从 iCloud 下载到本地，请先在 Finder 中双击下载后再拖入隐盾。".to_string()
                );
            }
        }
        // Also check if the file is suspiciously small (< 8KB) for a media file
        // that has a media extension — likely an iCloud placeholder
        if let Ok(meta) = std::fs::metadata(path) {
            if meta.len() < 8192 {
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                let media_exts = ["mp4", "mov", "avi", "mkv", "webm", "flv", "wmv",
                                  "wav", "mp3", "aac", "flac", "ogg", "m4a"];
                if media_exts.iter().any(|e| e.eq_ignore_ascii_case(ext)) {
                    return Err(
                        "该文件可能尚未从 iCloud 下载到本地（文件体积异常小），请先在 Finder 中双击下载后再拖入隐盾。".to_string()
                    );
                }
            }
        }
    }

    // On all platforms: basic existence check
    if !path.exists() {
        return Err("文件不存在或路径无效。".to_string());
    }

    Ok(())
}

/// Resolve the ffprobe binary path from AppState, detecting if needed.
async fn resolve_ffprobe(app_handle: &AppHandle) -> Result<std::path::PathBuf, String> {
  let state = app_handle.state::<AppState>();

  if let Some(paths) = state.ffmpeg_paths.get() {
    return Ok(paths.ffprobe.clone());
  }

  let app_data_dir = app_handle
    .path()
    .app_data_dir()
    .map_err(|e| format!("无法获取应用数据目录: {e}"))?;

  let paths = ffmpeg::detect_ffmpeg(&app_data_dir)
    .await
    .map_err(|e| format!("FFmpeg 不可用: {e}"))?;

  let ffprobe = paths.ffprobe.clone();
  let _ = state.ffmpeg_paths.set(paths);
  Ok(ffprobe)
}

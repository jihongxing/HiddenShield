use std::path::{Path, PathBuf};

use crate::commands::transcode::Platform;

#[allow(dead_code)]
pub fn safe_output_dir(input_path: &str) -> PathBuf {
    let path = Path::new(input_path);
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

/// Returns the Chinese display name for a platform.
fn platform_chinese_name(platform: Platform) -> &'static str {
    match platform {
        Platform::Douyin => "抖音",
        Platform::Bilibili => "B站",
        Platform::Xiaohongshu => "小红书",
    }
}

/// Generates an output file name in the format `{source_stem}_{平台中文名}优化版.mp4`.
///
/// `source_stem` is the file name without extension (e.g. "my_video").
#[allow(dead_code)]
pub fn output_file_name(source_stem: &str, platform: Platform) -> String {
    format!(
        "{}_{}优化版.mp4",
        source_stem,
        platform_chinese_name(platform)
    )
}

/// Creates a temporary directory for pipeline work under the system temp dir.
/// Returns the path to the created directory.
#[allow(dead_code)]
pub fn create_temp_dir(pipeline_id: &str) -> std::io::Result<PathBuf> {
    let dir = std::env::temp_dir().join(format!("hidenshield_{pipeline_id}"));
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Removes a temporary directory and all its contents.
#[allow(dead_code)]
pub fn cleanup_temp_dir(pipeline_id: &str) -> std::io::Result<()> {
    let dir = std::env::temp_dir().join(format!("hidenshield_{pipeline_id}"));
    if dir.exists() {
        std::fs::remove_dir_all(&dir)?;
    }
    Ok(())
}

/// Check if a path contains characters that may cause issues with FFmpeg on Windows
/// (emoji, certain CJK chars in combination with spaces/parens).
/// If problematic, copies the file to a safe temp path and returns it.
/// Otherwise returns the original path.
#[allow(dead_code)]
pub fn safe_input_path(input: &Path, pipeline_id: &str) -> std::io::Result<PathBuf> {
    let name = input.file_name().and_then(|n| n.to_str()).unwrap_or("");

    // Heuristic: if the filename is pure ASCII, it's safe
    if name.is_ascii() {
        return Ok(input.to_path_buf());
    }

    // Copy to temp dir with a safe hash-based name
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("tmp");
    let hash = crate::utils::hash::sha256_of_text(name);
    let safe_name = format!("hs_input_{}.{}", &hash[..12], ext);

    let temp_dir = create_temp_dir(pipeline_id)?;
    let safe_path = temp_dir.join(safe_name);
    std::fs::copy(input, &safe_path)?;
    Ok(safe_path)
}

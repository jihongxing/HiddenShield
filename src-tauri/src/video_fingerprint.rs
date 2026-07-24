use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::Instant;

use image::{DynamicImage, GrayImage};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::sync::cloud::{
    VideoFingerprintBundleForNotary, VideoFingerprintCropWindowForNotary,
    VideoFingerprintFrameForNotary, VideoFingerprintLocalBlockForNotary,
};
use crate::utils::process::hide_window;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VideoFingerprintBundleGeneration {
    pub bundle_path: String,
    pub bundle_sha256: String,
    pub bundle_bytes: u64,
    pub source_hash: String,
    pub watermark_uid: String,
    pub duration_ms: u64,
    pub scene_count: u32,
    pub frame_sample_policy: String,
    pub elapsed_ms: u128,
}

pub fn generate_bundle(
    input_path: &Path,
    output_root: &Path,
    ffmpeg: &Path,
    ffprobe: &Path,
    max_frames: usize,
) -> Result<VideoFingerprintBundleGeneration, String> {
    if !is_supported_video(input_path) {
        return Err("请选择 MP4、MOV、WebM、AVI、MKV 或 M4V 视频".to_string());
    }

    let started = Instant::now();
    let source_hash = sha256_file(input_path)?;
    let duration_ms = probe_duration_ms(ffprobe, input_path)?;
    let source_stem = sanitize_stem(input_path);
    let source_dir = output_root.join(format!(
        "{}-{}",
        source_stem,
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    ));
    let frames_dir = source_dir.join("frames-original");
    fs::create_dir_all(&source_dir).map_err(|error| format!("创建视频指纹目录失败: {error}"))?;

    extract_frames(ffmpeg, input_path, &frames_dir, max_frames, duration_ms)?;
    let fingerprints = fingerprint_frame_dir(&frames_dir)?;
    let bundle = build_bundle(&source_hash, duration_ms, fingerprints);
    let bundle_path = source_dir.join("bundle.json");
    let bundle_body = serde_json::to_string_pretty(&bundle)
        .map_err(|error| format!("序列化视频指纹 bundle 失败: {error}"))?;
    fs::write(&bundle_path, bundle_body)
        .map_err(|error| format!("写入 bundle.json 失败: {error}"))?;
    let bundle_bytes =
        fs::read(&bundle_path).map_err(|error| format!("读取 bundle.json 失败: {error}"))?;
    let bundle_sha256 = format!("sha256:{:x}", Sha256::digest(&bundle_bytes));

    Ok(VideoFingerprintBundleGeneration {
        bundle_path: bundle_path.to_string_lossy().to_string(),
        bundle_sha256,
        bundle_bytes: bundle_bytes.len() as u64,
        source_hash: bundle.source_hash,
        watermark_uid: bundle.watermark_uid,
        duration_ms,
        scene_count: bundle.scene_count as u32,
        frame_sample_policy: bundle.frame_sample_policy,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn build_bundle(
    source_hash: &str,
    duration_ms: u64,
    fingerprints: Vec<VideoFingerprintFrameForNotary>,
) -> VideoFingerprintBundleForNotary {
    let mut signer = Sha256::new();
    signer.update(source_hash.as_bytes());
    signer.update(duration_ms.to_le_bytes());
    for fingerprint in &fingerprints {
        signer.update(fingerprint.phash.as_bytes());
        signer.update(fingerprint.color_hash.as_bytes());
        signer.update(fingerprint.edge_hash.as_bytes());
        for block in &fingerprint.local_blocks {
            signer.update(block.grid.as_bytes());
            signer.update(block.row.to_le_bytes());
            signer.update(block.col.to_le_bytes());
            signer.update(block.phash.as_bytes());
            signer.update(block.edge_hash.as_bytes());
        }
        for crop_window in &fingerprint.crop_windows {
            signer.update(crop_window.region.as_bytes());
            signer.update(crop_window.phash.as_bytes());
            signer.update(crop_window.edge_hash.as_bytes());
        }
    }
    VideoFingerprintBundleForNotary {
        schema_version: "video_fingerprint_v1".to_string(),
        watermark_uid: format!("l2-{}", &source_hash[0..16.min(source_hash.len())]),
        source_hash: format!("sha256:{source_hash}"),
        duration_ms,
        frame_sample_policy: format!("uniform_{}_frames_v1", fingerprints.len()),
        scene_count: fingerprints.len(),
        fingerprints,
        client_signature: format!("sha256:{:x}", signer.finalize()),
    }
}

fn extract_frames(
    ffmpeg: &Path,
    source: &Path,
    output_dir: &Path,
    max_frames: usize,
    duration_ms: u64,
) -> Result<(), String> {
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)
            .map_err(|error| format!("清理抽帧目录 {} 失败: {error}", output_dir.display()))?;
    }
    fs::create_dir_all(output_dir).map_err(|error| format!("创建抽帧目录失败: {error}"))?;
    let pattern = output_dir.join("frame_%03d.png");
    let sample_fps = frame_sample_fps(max_frames, duration_ms);
    let filter = format!(
        "fps={sample_fps:.6},scale=192:108:force_original_aspect_ratio=decrease,pad=192:108:(ow-iw)/2:(oh-ih)/2,format=rgb24"
    );
    let args = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-i".to_string(),
        source.to_string_lossy().to_string(),
        "-vf".to_string(),
        filter,
        "-frames:v".to_string(),
        max_frames.to_string(),
        pattern.to_string_lossy().to_string(),
    ];
    run_command(ffmpeg, &args, "视频指纹抽帧")
}

fn frame_sample_fps(max_frames: usize, duration_ms: u64) -> f64 {
    let duration_secs = duration_ms as f64 / 1000.0;
    if duration_secs <= 0.0 {
        return 1.0;
    }
    (max_frames as f64 / duration_secs).clamp(0.05, 1.0)
}

fn fingerprint_frame_dir(frame_dir: &Path) -> Result<Vec<VideoFingerprintFrameForNotary>, String> {
    let mut frames = Vec::new();
    for entry in fs::read_dir(frame_dir).map_err(|error| format!("读取抽帧目录失败: {error}"))?
    {
        let path = entry
            .map_err(|error| format!("读取抽帧文件失败: {error}"))?
            .path();
        if path.extension().and_then(|value| value.to_str()) == Some("png") {
            frames.push(path);
        }
    }
    frames.sort();
    if frames.is_empty() {
        return Err(format!("未能从视频中抽取帧: {}", frame_dir.display()));
    }

    frames
        .iter()
        .enumerate()
        .map(|(index, path)| fingerprint_frame(path, index))
        .collect()
}

fn fingerprint_frame(
    path: &Path,
    scene_index: usize,
) -> Result<VideoFingerprintFrameForNotary, String> {
    let image = image::open(path)
        .map_err(|error| format!("读取抽帧图片 {} 失败: {error}", path.display()))?;
    Ok(VideoFingerprintFrameForNotary {
        scene_index,
        timestamp_ms: scene_index as u64 * 1000,
        phash: format!("{:016x}", perceptual_hash(&image)),
        color_hash: format!("{:016x}", color_hash(&image)),
        edge_hash: format!("{:016x}", edge_hash(&image)),
        local_blocks: local_block_fingerprints(&image),
        crop_windows: crop_window_fingerprints(&image),
        motion_summary: "static-frame-v1".to_string(),
    })
}

fn perceptual_hash(image: &DynamicImage) -> u64 {
    let gray = image
        .resize_exact(8, 8, image::imageops::FilterType::Triangle)
        .to_luma8();
    let mean = gray.pixels().map(|pixel| pixel[0] as u32).sum::<u32>() / 64;
    bits_from_gray(&gray, |value| value as u32 >= mean)
}

fn color_hash(image: &DynamicImage) -> u64 {
    let small = image
        .resize_exact(4, 4, image::imageops::FilterType::Triangle)
        .to_rgb8();
    let mut hash = 0u64;
    for (index, pixel) in small.pixels().enumerate() {
        let r = pixel[0] as u16;
        let g = pixel[1] as u16;
        let b = pixel[2] as u16;
        let bucket = ((r + g + b) / 96).min(7) as u64;
        hash |= bucket << (index * 3);
    }
    hash
}

fn edge_hash(image: &DynamicImage) -> u64 {
    let gray = image
        .resize_exact(9, 8, image::imageops::FilterType::Triangle)
        .to_luma8();
    let mut hash = 0u64;
    for y in 0..8 {
        for x in 0..8 {
            let left = gray.get_pixel(x, y)[0];
            let right = gray.get_pixel(x + 1, y)[0];
            if left > right {
                hash |= 1 << (y * 8 + x);
            }
        }
    }
    hash
}

fn local_block_fingerprints(image: &DynamicImage) -> Vec<VideoFingerprintLocalBlockForNotary> {
    let normalized = image
        .resize_exact(192, 108, image::imageops::FilterType::Triangle)
        .to_rgb8();
    let normalized = DynamicImage::ImageRgb8(normalized);
    let mut blocks = Vec::new();

    for (grid_name, cols, rows) in [("3x3", 3u32, 3u32), ("4x4", 4u32, 4u32)] {
        let block_width = normalized.width() / cols;
        let block_height = normalized.height() / rows;
        for row in 0..rows {
            for col in 0..cols {
                let block = normalized.crop_imm(
                    col * block_width,
                    row * block_height,
                    block_width,
                    block_height,
                );
                blocks.push(VideoFingerprintLocalBlockForNotary {
                    grid: grid_name.to_string(),
                    row: row as u8,
                    col: col as u8,
                    phash: format!("{:016x}", perceptual_hash(&block)),
                    edge_hash: format!("{:016x}", edge_hash(&block)),
                });
            }
        }
    }

    for (window_width, window_height) in [(32u32, 18u32), (48, 27), (64, 36), (96, 54)] {
        let step_x = (window_width / 2).max(1);
        let step_y = (window_height / 2).max(1);
        let max_x = normalized.width().saturating_sub(window_width);
        let max_y = normalized.height().saturating_sub(window_height);
        let mut y = 0u32;
        while y <= max_y {
            let mut x = 0u32;
            while x <= max_x {
                let block = normalized.crop_imm(x, y, window_width, window_height);
                blocks.push(VideoFingerprintLocalBlockForNotary {
                    grid: format!("dense_{window_width}x{window_height}"),
                    row: (y / step_y) as u8,
                    col: (x / step_x) as u8,
                    phash: format!("{:016x}", perceptual_hash(&block)),
                    edge_hash: format!("{:016x}", edge_hash(&block)),
                });
                if x == max_x {
                    break;
                }
                x = (x + step_x).min(max_x);
            }
            if y == max_y {
                break;
            }
            y = (y + step_y).min(max_y);
        }
    }

    blocks
}

fn crop_window_fingerprints(image: &DynamicImage) -> Vec<VideoFingerprintCropWindowForNotary> {
    let normalized = image
        .resize_exact(192, 108, image::imageops::FilterType::Triangle)
        .to_rgb8();
    let normalized = DynamicImage::ImageRgb8(normalized);
    let mut windows = Vec::new();
    for (region, scale, anchor_x, anchor_y) in [
        ("center_80", 0.80f32, 0.50f32, 0.50f32),
        ("center_70", 0.70, 0.50, 0.50),
        ("center_60", 0.60, 0.50, 0.50),
        ("top_left_80", 0.80, 0.00, 0.00),
        ("top_right_80", 0.80, 1.00, 0.00),
        ("bottom_left_80", 0.80, 0.00, 1.00),
        ("bottom_right_80", 0.80, 1.00, 1.00),
    ] {
        let width = even_dimension((normalized.width() as f32 * scale).round() as u32);
        let height = even_dimension((normalized.height() as f32 * scale).round() as u32);
        let x = anchored_offset(normalized.width(), width, anchor_x);
        let y = anchored_offset(normalized.height(), height, anchor_y);
        let crop = normalized.crop_imm(x, y, width, height);
        windows.push(VideoFingerprintCropWindowForNotary {
            region: region.to_string(),
            phash: format!("{:016x}", perceptual_hash(&crop)),
            edge_hash: format!("{:016x}", edge_hash(&crop)),
        });
    }
    windows
}

fn anchored_offset(full: u32, window: u32, anchor: f32) -> u32 {
    let max = full.saturating_sub(window);
    ((max as f32 * anchor).round() as u32).min(max)
}

fn even_dimension(value: u32) -> u32 {
    let even = value.max(2) & !1;
    even.max(2)
}

fn bits_from_gray<F>(gray: &GrayImage, predicate: F) -> u64
where
    F: Fn(u8) -> bool,
{
    let mut hash = 0u64;
    for (index, pixel) in gray.pixels().enumerate() {
        if predicate(pixel[0]) {
            hash |= 1 << index;
        }
    }
    hash
}

fn probe_duration_ms(ffprobe: &Path, source: &Path) -> Result<u64, String> {
    let source_arg = source.to_string_lossy().to_string();
    let mut command = Command::new(ffprobe);
    hide_window(&mut command);
    let output = command
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration",
            "-of",
            "default=noprint_wrappers=1:nokey=1",
            &source_arg,
        ])
        .output()
        .map_err(|error| format!("启动 ffprobe 失败: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "读取视频时长失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let duration_secs = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|error| format!("解析视频时长失败: {error}"))?;
    Ok((duration_secs * 1000.0).round() as u64)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("读取视频文件失败: {error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn run_command(binary: &Path, args: &[String], label: &str) -> Result<(), String> {
    let mut command = Command::new(binary);
    hide_window(&mut command);
    let output = command
        .args(args)
        .output()
        .map_err(|error| format!("启动 {label} 失败: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{label} 失败: {}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn is_supported_video(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("mp4" | "mov" | "webm" | "avi" | "mkv" | "m4v")
    )
}

fn sanitize_stem(path: &Path) -> String {
    let raw = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("video");
    let sanitized: String = raw
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "video".to_string()
    } else {
        trimmed.chars().take(48).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn l2_fingerprint_accepts_north_star_video_containers() {
        for extension in ["mp4", "mov", "webm", "avi", "mkv", "m4v"] {
            let file_name = format!("sample.{extension}");
            assert!(
                is_supported_video(Path::new(&file_name)),
                "{extension} should be accepted for L2 fingerprint notary"
            );
        }
    }

    #[test]
    fn l2_fingerprint_rejects_unlisted_video_containers() {
        for extension in ["flv", "wmv", "mpg", "gif"] {
            let file_name = format!("sample.{extension}");
            assert!(
                !is_supported_video(Path::new(&file_name)),
                "{extension} should stay outside the L2 formal container matrix"
            );
        }
    }
}

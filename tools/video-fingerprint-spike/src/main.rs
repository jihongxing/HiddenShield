use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use image::{DynamicImage, GrayImage};
use serde::Serialize;
use sha2::{Digest, Sha256};

#[derive(Debug, Clone)]
struct Config {
    video_dir: PathBuf,
    output_dir: PathBuf,
    max_videos: usize,
    max_frames: usize,
    ffmpeg: String,
    ffprobe: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct VideoFingerprintBundle {
    schema_version: &'static str,
    watermark_uid: String,
    source_hash: String,
    duration_ms: u64,
    frame_sample_policy: String,
    scene_count: usize,
    fingerprints: Vec<FrameFingerprint>,
    client_signature: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FrameFingerprint {
    scene_index: usize,
    timestamp_ms: u64,
    phash: String,
    color_hash: String,
    edge_hash: String,
    local_blocks: Vec<BlockFingerprint>,
    crop_windows: Vec<CropWindowFingerprint>,
    motion_summary: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct BlockFingerprint {
    grid: String,
    row: u8,
    col: u8,
    phash: String,
    edge_hash: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct CropWindowFingerprint {
    region: String,
    phash: String,
    edge_hash: String,
}

#[derive(Debug, Serialize)]
struct AttackResult {
    attack: String,
    matched_frames: usize,
    total_frames: usize,
    recall: f64,
    average_block_recall: f64,
    average_crop_distance: f64,
    average_distance: f64,
    success: bool,
    output_path: String,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct SourceResult {
    source: String,
    source_hash: String,
    duration_ms: u64,
    frame_count: usize,
    bundle_path: String,
    elapsed_ms: u128,
    attacks: Vec<AttackResult>,
}

#[derive(Debug, Serialize)]
struct SpikeReport {
    schema_version: &'static str,
    generated_at: String,
    sample_count: usize,
    threshold_recall: f64,
    results: Vec<SourceResult>,
    summary: SpikeSummary,
}

#[derive(Debug, Serialize)]
struct SpikeSummary {
    total_attacks: usize,
    passed_attacks: usize,
    average_recall: f64,
    recommendation: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_args(env::args().skip(1).collect())?;
    fs::create_dir_all(&config.output_dir)
        .map_err(|error| format!("create output dir: {error}"))?;

    ensure_binary(&config.ffmpeg)?;
    ensure_binary(&config.ffprobe)?;

    let sources = collect_videos(&config.video_dir, config.max_videos)?;
    if sources.is_empty() {
        return Err(format!(
            "no supported videos found in {}",
            config.video_dir.display()
        ));
    }

    let run_dir = config.output_dir.join(format!("run-{}", unix_seconds()));
    fs::create_dir_all(&run_dir).map_err(|error| format!("create run dir: {error}"))?;

    let mut results = Vec::new();
    for (index, source) in sources.iter().enumerate() {
        results.push(run_source(&config, &run_dir, source, index)?);
    }

    let summary = summarize(&results);
    let report = SpikeReport {
        schema_version: "video_fingerprint_spike_v1",
        generated_at: now_rfc3339(),
        sample_count: sources.len(),
        threshold_recall: 0.70,
        results,
        summary,
    };

    let report_json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("serialize report: {error}"))?;
    fs::write(run_dir.join("report.json"), report_json)
        .map_err(|error| format!("write report.json: {error}"))?;
    write_markdown_report(&run_dir.join("report.md"), &report)?;

    println!(
        "Video fingerprint spike finished: {}/{} attacks passed",
        report.summary.passed_attacks, report.summary.total_attacks
    );
    println!("Report: {}", run_dir.join("report.md").display());
    Ok(())
}

impl Config {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut video_dir = None;
        let mut output_dir = PathBuf::from("src-tauri/target/video-fingerprint-spike");
        let mut max_videos = 10usize;
        let mut max_frames = 8usize;
        let mut ffmpeg = "ffmpeg".to_string();
        let mut ffprobe = "ffprobe".to_string();

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--video-dir" => {
                    index += 1;
                    video_dir = Some(PathBuf::from(required_value(&args, index, "--video-dir")?));
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(required_value(&args, index, "--output-dir")?);
                }
                "--max-videos" => {
                    index += 1;
                    max_videos = parse_usize(required_value(&args, index, "--max-videos")?)?;
                }
                "--max-frames" => {
                    index += 1;
                    max_frames = parse_usize(required_value(&args, index, "--max-frames")?)?;
                }
                "--ffmpeg" => {
                    index += 1;
                    ffmpeg = required_value(&args, index, "--ffmpeg")?.to_string();
                }
                "--ffprobe" => {
                    index += 1;
                    ffprobe = required_value(&args, index, "--ffprobe")?.to_string();
                }
                "--help" | "-h" => {
                    print_usage();
                    std::process::exit(0);
                }
                unknown => return Err(format!("unknown argument: {unknown}")),
            }
            index += 1;
        }

        Ok(Self {
            video_dir: video_dir.ok_or("missing --video-dir")?,
            output_dir,
            max_videos,
            max_frames,
            ffmpeg,
            ffprobe,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: npm run video:fingerprint-spike -- \\
  --video-dir <dir> [--max-videos 10] [--max-frames 8] [--output-dir src-tauri/target/video-fingerprint-spike]"
    );
}

fn run_source(
    config: &Config,
    run_dir: &Path,
    source: &Path,
    index: usize,
) -> Result<SourceResult, String> {
    let started = Instant::now();
    let source_stem = sanitize_stem(source, index);
    let source_dir = run_dir.join(&source_stem);
    fs::create_dir_all(&source_dir).map_err(|error| format!("create source dir: {error}"))?;

    let source_hash = sha256_file(source)?;
    let duration_ms = probe_duration_ms(&config.ffprobe, source)?;
    let original_frames_dir = source_dir.join("frames-original");
    extract_frames(
        &config.ffmpeg,
        source,
        &original_frames_dir,
        config.max_frames,
        duration_ms,
    )?;

    let fingerprints = fingerprint_frame_dir(&original_frames_dir)?;
    let bundle = build_bundle(&source_hash, duration_ms, fingerprints);
    let bundle_path = source_dir.join("bundle.json");
    fs::write(
        &bundle_path,
        serde_json::to_string_pretty(&bundle)
            .map_err(|error| format!("serialize bundle: {error}"))?,
    )
    .map_err(|error| format!("write bundle: {error}"))?;

    let attacks = run_attacks(config, source, &source_dir, &bundle.fingerprints)?;

    Ok(SourceResult {
        source: source.to_string_lossy().to_string(),
        source_hash,
        duration_ms,
        frame_count: bundle.fingerprints.len(),
        bundle_path: bundle_path.to_string_lossy().to_string(),
        elapsed_ms: started.elapsed().as_millis(),
        attacks,
    })
}

fn build_bundle(
    source_hash: &str,
    duration_ms: u64,
    fingerprints: Vec<FrameFingerprint>,
) -> VideoFingerprintBundle {
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
    VideoFingerprintBundle {
        schema_version: "video_fingerprint_v1",
        watermark_uid: format!("l2-spike-{}", &source_hash[0..16.min(source_hash.len())]),
        source_hash: format!("sha256:{source_hash}"),
        duration_ms,
        frame_sample_policy: format!("uniform_{}_frames_v1", fingerprints.len()),
        scene_count: fingerprints.len(),
        fingerprints,
        client_signature: format!("sha256:{:x}", signer.finalize()),
    }
}

fn run_attacks(
    config: &Config,
    source: &Path,
    source_dir: &Path,
    baseline: &[FrameFingerprint],
) -> Result<Vec<AttackResult>, String> {
    let attacks = [
        ("scale_540p", vec!["-vf", "scale=-2:540", "-c:v", "libx264", "-crf", "28"]),
        (
            "transcode_crf32",
            vec!["-c:v", "libx264", "-crf", "32", "-preset", "veryfast"],
        ),
        (
            "center_crop_80",
            vec![
                "-vf",
                "crop=trunc(iw*0.8/2)*2:trunc(ih*0.8/2)*2:trunc(iw*0.1/2)*2:trunc(ih*0.1/2)*2,scale=-2:min(720\\,ih)",
                "-c:v",
                "libx264",
                "-crf",
                "28",
            ],
        ),
    ];

    let mut results = Vec::new();
    for (name, ffmpeg_args) in attacks {
        let attacked_path = source_dir.join(format!("{name}.mp4"));
        let attack_result =
            create_attacked_video(&config.ffmpeg, source, &attacked_path, &ffmpeg_args)
                .and_then(|_| {
                    let frames_dir = source_dir.join(format!("frames-{name}"));
                    let duration_ms = probe_duration_ms(&config.ffprobe, &attacked_path)?;
                    extract_frames(
                        &config.ffmpeg,
                        &attacked_path,
                        &frames_dir,
                        baseline.len(),
                        duration_ms,
                    )?;
                    let attacked = fingerprint_frame_dir(&frames_dir)?;
                    Ok(compare_fingerprints(
                        name,
                        &attacked_path,
                        baseline,
                        &attacked,
                    ))
                })
                .unwrap_or_else(|error| AttackResult {
                    attack: name.to_string(),
                    matched_frames: 0,
                    total_frames: baseline.len(),
                    recall: 0.0,
                    average_distance: 64.0,
                    average_block_recall: 0.0,
                    average_crop_distance: 64.0,
                    success: false,
                    output_path: attacked_path.to_string_lossy().to_string(),
                    error: Some(error),
                });
        results.push(attack_result);
    }
    Ok(results)
}

fn create_attacked_video(
    ffmpeg: &str,
    source: &Path,
    output: &Path,
    attack_args: &[&str],
) -> Result<(), String> {
    let mut args = vec![
        "-y".to_string(),
        "-hide_banner".to_string(),
        "-loglevel".to_string(),
        "error".to_string(),
        "-i".to_string(),
        source.to_string_lossy().to_string(),
    ];
    args.extend(attack_args.iter().map(|arg| arg.to_string()));
    args.extend([
        "-an".to_string(),
        "-movflags".to_string(),
        "+faststart".to_string(),
        output.to_string_lossy().to_string(),
    ]);
    run_command(ffmpeg, &args, "create attacked video")
}

fn extract_frames(
    ffmpeg: &str,
    source: &Path,
    output_dir: &Path,
    max_frames: usize,
    duration_ms: u64,
) -> Result<(), String> {
    if output_dir.exists() {
        fs::remove_dir_all(output_dir)
            .map_err(|error| format!("clear frame dir {}: {error}", output_dir.display()))?;
    }
    fs::create_dir_all(output_dir).map_err(|error| format!("create frame dir: {error}"))?;
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
    run_command(ffmpeg, &args, "extract frames")
}

fn frame_sample_fps(max_frames: usize, duration_ms: u64) -> f64 {
    let duration_secs = duration_ms as f64 / 1000.0;
    if duration_secs <= 0.0 {
        return 1.0;
    }
    (max_frames as f64 / duration_secs).clamp(0.05, 1.0)
}

fn fingerprint_frame_dir(frame_dir: &Path) -> Result<Vec<FrameFingerprint>, String> {
    let mut frames = Vec::new();
    for entry in fs::read_dir(frame_dir).map_err(|error| format!("read frame dir: {error}"))? {
        let path = entry
            .map_err(|error| format!("read frame entry: {error}"))?
            .path();
        if path.extension().and_then(|value| value.to_str()) == Some("png") {
            frames.push(path);
        }
    }
    frames.sort();
    if frames.is_empty() {
        return Err(format!("no frames extracted in {}", frame_dir.display()));
    }

    frames
        .iter()
        .enumerate()
        .map(|(index, path)| fingerprint_frame(path, index))
        .collect()
}

fn fingerprint_frame(path: &Path, scene_index: usize) -> Result<FrameFingerprint, String> {
    let image =
        image::open(path).map_err(|error| format!("open frame {}: {error}", path.display()))?;
    Ok(FrameFingerprint {
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

fn compare_fingerprints(
    attack: &str,
    output_path: &Path,
    baseline: &[FrameFingerprint],
    attacked: &[FrameFingerprint],
) -> AttackResult {
    let total = baseline.len().min(attacked.len());
    let mut matched = 0usize;
    let mut distance_total = 0u32;
    let mut block_recall_total = 0.0;
    let mut crop_distance_total = 0u32;
    for index in 0..total {
        let distance = hex_hamming(&baseline[index].phash, &attacked[index].phash)
            + hex_hamming(&baseline[index].edge_hash, &attacked[index].edge_hash) / 2;
        distance_total += distance;
        let block_recall =
            compare_local_blocks(&baseline[index].local_blocks, &attacked[index].local_blocks);
        block_recall_total += block_recall;
        let crop_distance = compare_crop_windows(&baseline[index].crop_windows, &attacked[index]);
        crop_distance_total += crop_distance;
        if distance <= 18 || block_recall >= 0.45 || crop_distance <= 18 {
            matched += 1;
        }
    }
    let recall = if total == 0 {
        0.0
    } else {
        matched as f64 / total as f64
    };
    AttackResult {
        attack: attack.to_string(),
        matched_frames: matched,
        total_frames: total,
        recall,
        average_block_recall: if total == 0 {
            0.0
        } else {
            block_recall_total / total as f64
        },
        average_crop_distance: if total == 0 {
            64.0
        } else {
            crop_distance_total as f64 / total as f64
        },
        average_distance: if total == 0 {
            64.0
        } else {
            distance_total as f64 / total as f64
        },
        success: recall >= 0.70,
        output_path: output_path.to_string_lossy().to_string(),
        error: None,
    }
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

fn local_block_fingerprints(image: &DynamicImage) -> Vec<BlockFingerprint> {
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
                blocks.push(BlockFingerprint {
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
                blocks.push(BlockFingerprint {
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

fn compare_local_blocks(baseline: &[BlockFingerprint], attacked: &[BlockFingerprint]) -> f64 {
    if baseline.is_empty() || attacked.is_empty() {
        return 0.0;
    }

    let mut matched = 0usize;
    for attacked_block in attacked {
        let best_distance = baseline
            .iter()
            .filter(|baseline_block| comparable_blocks(baseline_block, attacked_block))
            .map(|baseline_block| {
                hex_hamming(&baseline_block.phash, &attacked_block.phash)
                    + hex_hamming(&baseline_block.edge_hash, &attacked_block.edge_hash) / 2
            })
            .min()
            .unwrap_or(64);
        if best_distance <= 14 {
            matched += 1;
        }
    }

    matched as f64 / attacked.len() as f64
}

fn comparable_blocks(left: &BlockFingerprint, right: &BlockFingerprint) -> bool {
    let left_dense = left.grid.starts_with("dense_");
    let right_dense = right.grid.starts_with("dense_");
    (left_dense && right_dense) || left.grid == right.grid
}

fn crop_window_fingerprints(image: &DynamicImage) -> Vec<CropWindowFingerprint> {
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
        windows.push(CropWindowFingerprint {
            region: region.to_string(),
            phash: format!("{:016x}", perceptual_hash(&crop)),
            edge_hash: format!("{:016x}", edge_hash(&crop)),
        });
    }
    windows
}

fn compare_crop_windows(
    crop_windows: &[CropWindowFingerprint],
    attacked: &FrameFingerprint,
) -> u32 {
    crop_windows
        .iter()
        .map(|crop_window| {
            hex_hamming(&crop_window.phash, &attacked.phash)
                + hex_hamming(&crop_window.edge_hash, &attacked.edge_hash) / 2
        })
        .min()
        .unwrap_or(64)
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

fn hex_hamming(left: &str, right: &str) -> u32 {
    let left = u64::from_str_radix(left, 16).unwrap_or(0);
    let right = u64::from_str_radix(right, 16).unwrap_or(0);
    (left ^ right).count_ones()
}

fn probe_duration_ms(ffprobe: &str, source: &Path) -> Result<u64, String> {
    let source_arg = source.to_string_lossy().to_string();
    let output = Command::new(ffprobe)
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
        .map_err(|error| format!("spawn ffprobe: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "ffprobe failed for {}: {}",
            source.display(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let duration_secs = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<f64>()
        .map_err(|error| format!("parse ffprobe duration: {error}"))?;
    Ok((duration_secs * 1000.0).round() as u64)
}

fn collect_videos(video_dir: &Path, limit: usize) -> Result<Vec<PathBuf>, String> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(video_dir).map_err(|error| format!("read video dir: {error}"))? {
        let path = entry
            .map_err(|error| format!("read video dir entry: {error}"))?
            .path();
        if is_supported_video(&path) {
            paths.push(path);
        }
    }
    paths.sort();
    paths.truncate(limit);
    Ok(paths)
}

fn is_supported_video(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|value| value.to_str())
            .map(|value| value.to_ascii_lowercase())
            .as_deref(),
        Some("mp4" | "mov" | "mkv" | "webm")
    )
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn run_command(binary: &str, args: &[String], label: &str) -> Result<(), String> {
    let output = Command::new(binary)
        .args(args)
        .output()
        .map_err(|error| format!("spawn {label}: {error}"))?;
    if output.status.success() {
        return Ok(());
    }
    Err(format!(
        "{label} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    ))
}

fn ensure_binary(binary: &str) -> Result<(), String> {
    let output = Command::new(binary)
        .arg("-version")
        .output()
        .map_err(|error| format!("{binary} unavailable: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!("{binary} health check failed"))
    }
}

fn summarize(results: &[SourceResult]) -> SpikeSummary {
    let attacks: Vec<&AttackResult> = results
        .iter()
        .flat_map(|result| result.attacks.iter())
        .collect();
    let total_attacks = attacks.len();
    let passed_attacks = attacks.iter().filter(|attack| attack.success).count();
    let average_recall = if attacks.is_empty() {
        0.0
    } else {
        attacks.iter().map(|attack| attack.recall).sum::<f64>() / attacks.len() as f64
    };
    let recommendation = if total_attacks > 0 && passed_attacks == total_attacks {
        "L2 fingerprint fields are viable for a first cloud notary API draft.".to_string()
    } else {
        "Keep L2 local-only until fingerprint sampling and matching thresholds improve.".to_string()
    };
    SpikeSummary {
        total_attacks,
        passed_attacks,
        average_recall,
        recommendation,
    }
}

fn write_markdown_report(path: &Path, report: &SpikeReport) -> Result<(), String> {
    let mut lines = vec![
        "# Video Fingerprint L2 Spike".to_string(),
        String::new(),
        format!("- Generated at: {}", report.generated_at),
        format!("- Samples: {}", report.sample_count),
        format!(
            "- Passed attacks: {}/{}",
            report.summary.passed_attacks, report.summary.total_attacks
        ),
        format!("- Average recall: {:.2}", report.summary.average_recall),
        format!("- Recommendation: {}", report.summary.recommendation),
        String::new(),
        "| Source | Attack | Recall | Block Recall | Crop Distance | Avg Distance | Passed |"
            .to_string(),
        "| --- | --- | ---: | ---: | ---: | ---: | --- |".to_string(),
    ];
    for result in &report.results {
        let source = Path::new(&result.source)
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or(&result.source);
        for attack in &result.attacks {
            lines.push(format!(
                "| {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {} |",
                source,
                attack.attack,
                attack.recall,
                attack.average_block_recall,
                attack.average_crop_distance,
                attack.average_distance,
                if attack.success { "yes" } else { "no" }
            ));
        }
    }
    fs::write(path, lines.join("\n")).map_err(|error| format!("write report.md: {error}"))
}

fn required_value<'a>(args: &'a [String], index: usize, name: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("missing value for {name}"))
}

fn parse_usize(value: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("invalid number '{value}': {error}"))
}

fn sanitize_stem(path: &Path, index: usize) -> String {
    let raw = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("video");
    let safe: String = raw
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect();
    format!("{index:02}-{safe}")
}

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339()
}

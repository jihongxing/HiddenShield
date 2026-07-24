use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use image::{imageops, Rgba, RgbaImage};
use serde::Serialize;
use watermark_core::image_spatial_recovery_v1::{
    embed_spatial_recovery_v1, exact_grid_crops, extract_spatial_recovery_v1,
    extract_spatial_recovery_v1_exact, simulate_spatial_recovery_v1_coverage, SpatialRecoveryRect,
    SpatialRecoveryV1CoverageSimulation,
};
use watermark_core::{PayloadV3MinimalAnchorBuildInput, WatermarkPayloadV3MinimalAnchor};

const RUN_DIRECTORY: &str =
    "artifacts/desktop-image-spatial-recovery-gate/20260722-local-transform";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CropReadResult {
    index: usize,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    watermark_uid: String,
    elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SpatialRecoveryGateSummary {
    schema_version: &'static str,
    status: &'static str,
    question: &'static str,
    source_width: u32,
    source_height: u32,
    expected_watermark_uid: String,
    packet_count: usize,
    exact_full_image_read_ms: u128,
    clean_probe_results: Vec<CleanProbeResult>,
    clean_probe_all_rejected: bool,
    exact_grid_crop_reads: Vec<CropReadResult>,
    exact_grid_all_passed: bool,
    sliding_crop_reads: Vec<CropReadResult>,
    sliding_crop_all_passed: bool,
    reencode_reads: Vec<ReencodeReadResult>,
    reencode_all_passed: bool,
    sliding_quarter_crop_coverage: SpatialRecoveryV1CoverageSimulation,
    geometry_cases: Vec<SpatialRecoveryV1CoverageSimulation>,
    limitations: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CleanProbeResult {
    name: &'static str,
    exact_rejected: bool,
    exact_elapsed_ms: u128,
    scan_rejected: bool,
    scan_elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReencodeReadResult {
    format: &'static str,
    path: String,
    watermark_uid: String,
    elapsed_ms: u128,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let run_directory = PathBuf::from(RUN_DIRECTORY);
    fs::create_dir_all(&run_directory)
        .map_err(|error| format!("create {}: {error}", run_directory.display()))?;
    let source_width = 1920;
    let source_height = 1080;
    let source = build_source(source_width, source_height);
    let anchor = WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput {
        watermark_id: [
            0x21, 0x22, 0x23, 0x24, 0x31, 0x32, 0x33, 0x34, 0x41, 0x42, 0x43, 0x44, 0x51, 0x52,
            0x53, 0x54,
        ],
    })
    .map_err(|error| error.to_string())?;
    let expected_watermark_uid = anchor.watermark_uid();
    let (protected, layout) =
        embed_spatial_recovery_v1(&source, &anchor).map_err(|error| error.to_string())?;
    let exact_read_started = Instant::now();
    let exact_decoded =
        extract_spatial_recovery_v1_exact(&protected).map_err(|error| error.to_string())?;
    let exact_full_image_read_ms = exact_read_started.elapsed().as_millis();
    if exact_decoded.watermark_uid() != expected_watermark_uid {
        return Err("exact full-image read returned a different watermark UID".into());
    }
    let clean_images = [
        ("textured", source.clone()),
        (
            "flat-midgray",
            RgbaImage::from_pixel(source_width, source_height, Rgba([127, 127, 127, 255])),
        ),
        (
            "horizontal-gradient",
            RgbaImage::from_fn(source_width, source_height, |x, _| {
                let value = (x * 255 / (source_width - 1)) as u8;
                Rgba([value, value, value, 255])
            }),
        ),
        (
            "checkerboard",
            RgbaImage::from_fn(source_width, source_height, |x, y| {
                let value = if (x / 8 + y / 8) % 2 == 0 { 32 } else { 224 };
                Rgba([value, value, value, 255])
            }),
        ),
    ];
    let mut clean_probe_results = Vec::new();
    for (name, clean_image) in clean_images {
        let exact_started = Instant::now();
        let exact_rejected = extract_spatial_recovery_v1_exact(&clean_image).is_err();
        let exact_elapsed_ms = exact_started.elapsed().as_millis();
        let scan_started = Instant::now();
        let scan_rejected = extract_spatial_recovery_v1(&clean_image).is_err();
        let scan_elapsed_ms = scan_started.elapsed().as_millis();
        if !exact_rejected || !scan_rejected {
            return Err(format!(
                "clean image {name} produced a spatial-recovery-v1 false positive"
            ));
        }
        clean_probe_results.push(CleanProbeResult {
            name,
            exact_rejected,
            exact_elapsed_ms,
            scan_rejected,
            scan_elapsed_ms,
        });
    }
    let clean_probe_all_rejected = clean_probe_results
        .iter()
        .all(|result| result.exact_rejected && result.scan_rejected);

    let mut exact_grid_crop_reads = Vec::new();
    for (index, crop) in exact_grid_crops(source_width, source_height)
        .into_iter()
        .enumerate()
    {
        let cropped =
            imageops::crop_imm(&protected, crop.x, crop.y, crop.width, crop.height).to_image();
        let started = Instant::now();
        let decoded = extract_spatial_recovery_v1(&cropped).map_err(|error| error.to_string())?;
        let watermark_uid = decoded.watermark_uid();
        if watermark_uid != expected_watermark_uid {
            return Err(format!(
                "crop {index} UID mismatch: expected {expected_watermark_uid}, got {watermark_uid}"
            ));
        }
        exact_grid_crop_reads.push(CropReadResult {
            index,
            x: crop.x,
            y: crop.y,
            width: crop.width,
            height: crop.height,
            watermark_uid,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let mut sliding_crop_reads = Vec::new();
    for (index, crop) in sliding_crop_cases(source_width, source_height)
        .into_iter()
        .enumerate()
    {
        let cropped =
            imageops::crop_imm(&protected, crop.x, crop.y, crop.width, crop.height).to_image();
        let started = Instant::now();
        let decoded = extract_spatial_recovery_v1(&cropped).map_err(|error| error.to_string())?;
        let watermark_uid = decoded.watermark_uid();
        if watermark_uid != expected_watermark_uid {
            return Err(format!(
                "sliding crop {index} UID mismatch: expected {expected_watermark_uid}, got {watermark_uid}"
            ));
        }
        sliding_crop_reads.push(CropReadResult {
            index,
            x: crop.x,
            y: crop.y,
            width: crop.width,
            height: crop.height,
            watermark_uid,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let protected_png_path = run_directory.join("protected.png");
    protected
        .save(&protected_png_path)
        .map_err(|error| format!("save {}: {error}", protected_png_path.display()))?;
    let reencode_specs = [
        (
            "jpeg",
            run_directory.join("protected-q2.jpg"),
            vec!["-c:v", "mjpeg", "-q:v", "2", "-pix_fmt", "yuvj444p"],
        ),
        (
            "webp",
            run_directory.join("protected-q90.webp"),
            vec!["-c:v", "libwebp", "-q:v", "90", "-compression_level", "4"],
        ),
    ];
    let mut reencode_reads = Vec::new();
    for (format, output_path, codec_args) in reencode_specs {
        transcode_image(&protected_png_path, &output_path, &codec_args)?;
        let suspect = image::open(&output_path)
            .map_err(|error| format!("open {}: {error}", output_path.display()))?
            .to_rgba8();
        let started = Instant::now();
        let decoded = extract_spatial_recovery_v1_exact(&suspect).map_err(|error| {
            format!(
                "{format} re-encode recovery failed for {}: {error}",
                output_path.display()
            )
        })?;
        let watermark_uid = decoded.watermark_uid();
        if watermark_uid != expected_watermark_uid {
            return Err(format!(
                "{format} re-encode UID mismatch: expected {expected_watermark_uid}, got {watermark_uid}"
            ));
        }
        reencode_reads.push(ReencodeReadResult {
            format,
            path: output_path.display().to_string(),
            watermark_uid,
            elapsed_ms: started.elapsed().as_millis(),
        });
    }

    let sliding_quarter_crop_coverage =
        simulate_spatial_recovery_v1_coverage(source_width, source_height)
            .map_err(|error| error.to_string())?;
    let geometry_cases = [
        (320, 600),
        (1920, 1080),
        (2048, 2048),
        (5000, 5000),
        (9992, 10000),
    ]
    .into_iter()
    .map(|(width, height)| {
        simulate_spatial_recovery_v1_coverage(width, height).map_err(|error| error.to_string())
    })
    .collect::<Result<Vec<_>, _>>()?;
    let exact_grid_all_passed = exact_grid_crop_reads.len() == 16
        && exact_grid_crop_reads
            .iter()
            .all(|result| result.watermark_uid == expected_watermark_uid);
    let sliding_crop_all_passed = sliding_crop_reads
        .iter()
        .all(|result| result.watermark_uid == expected_watermark_uid);
    let reencode_all_passed = reencode_reads.len() == 2
        && reencode_reads
            .iter()
            .all(|result| result.watermark_uid == expected_watermark_uid);
    if !exact_grid_all_passed
        || !sliding_crop_all_passed
        || !clean_probe_all_rejected
        || !reencode_all_passed
        || !sliding_quarter_crop_coverage.every_quarter_by_quarter_crop_contains_packet
        || sliding_quarter_crop_coverage.exact_grid_uncovered_count != 0
    {
        return Err("spatial-recovery-v1 prototype coverage gate failed".into());
    }

    let summary = SpatialRecoveryGateSummary {
        schema_version: "spatial_recovery_v1_local_transform_gate_v1",
        status: "passed",
        question: "Can local transform packets recover the same V3 watermark from every 4x4 grid crop, representative sliding 1/16 crops, and PNG-to-JPEG/WebP re-encodes without clean-image false positives?",
        source_width,
        source_height,
        expected_watermark_uid,
        packet_count: layout.packet_rects.len(),
        exact_full_image_read_ms,
        clean_probe_results,
        clean_probe_all_rejected,
        exact_grid_crop_reads,
        exact_grid_all_passed,
        sliding_crop_reads,
        sliding_crop_all_passed,
        reencode_reads,
        reencode_all_passed,
        sliding_quarter_crop_coverage,
        geometry_cases,
        limitations: vec![
            "The desktop crop-recovery boundary is promoted only together with the installed comprehensive and 102-sample false-positive evidence.",
            "Combined disturbances, arbitrary-angle rotation, scaling below 80 percent, and recompression below quality 60 remain outside the current promise.",
        ],
    };
    let summary_json = serde_json::to_string_pretty(&summary).map_err(|error| error.to_string())?;
    let summary_path = run_directory.join("summary.json");
    fs::write(&summary_path, &summary_json)
        .map_err(|error| format!("write {}: {error}", summary_path.display()))?;
    println!("{summary_json}");
    Ok(())
}

fn sliding_crop_cases(width: u32, height: u32) -> Vec<SpatialRecoveryRect> {
    let crop_width = width / 4;
    let crop_height = height / 4;
    let maximum_x = width - crop_width;
    let maximum_y = height - crop_height;
    let x_positions = [0, 1, maximum_x / 3, maximum_x / 2, maximum_x - 1, maximum_x];
    let y_positions = [0, 1, maximum_y / 3, maximum_y / 2, maximum_y - 1, maximum_y];
    y_positions
        .into_iter()
        .flat_map(|y| {
            x_positions.into_iter().map(move |x| SpatialRecoveryRect {
                x,
                y,
                width: crop_width,
                height: crop_height,
            })
        })
        .collect()
}

fn transcode_image(input: &Path, output: &Path, codec_args: &[&str]) -> Result<(), String> {
    let mut command = Command::new("ffmpeg");
    command
        .arg("-y")
        .arg("-hide_banner")
        .arg("-loglevel")
        .arg("error")
        .arg("-i")
        .arg(input);
    command.args(codec_args).arg(output);
    let result = command
        .output()
        .map_err(|error| format!("start ffmpeg: {error}"))?;
    if result.status.success() {
        return Ok(());
    }
    Err(format!(
        "ffmpeg failed for {}: {}",
        output.display(),
        String::from_utf8_lossy(&result.stderr)
    ))
}

fn build_source(width: u32, height: u32) -> RgbaImage {
    RgbaImage::from_fn(width, height, |x, y| {
        Rgba([
            ((x * 13 + y * 3) % 256) as u8,
            ((x * 5 + y * 11) % 256) as u8,
            ((x ^ y) % 256) as u8,
            255,
        ])
    })
}

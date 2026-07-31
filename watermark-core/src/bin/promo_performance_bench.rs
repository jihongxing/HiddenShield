use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use image::ImageReader;
use serde::Serialize;
use sha2::{Digest, Sha256};
use watermark_core::{EmbedOptions, MediaInput, MediaOutput, WatermarkPayload, WatermarkService};

const IMAGE_MIN_BYTES: u64 = 8 * 1024 * 1024;
const IMAGE_MAX_BYTES: u64 = 12 * 1024 * 1024;
const AUDIO_MIN_BYTES: u64 = 18 * 1024 * 1024;
const AUDIO_MAX_BYTES: u64 = 22 * 1024 * 1024;
const EXPECTED_WIDTH: u32 = 4_000;
const EXPECTED_HEIGHT: u32 = 3_000;
const EXPECTED_AUDIO_SECONDS: f64 = 180.0;
const EXPECTED_SAMPLE_RATE: u32 = 44_100;
const EXPECTED_CHANNELS: u16 = 2;

#[derive(Debug)]
struct Config {
    fixture_dir: PathBuf,
    output_dir: PathBuf,
    iterations: usize,
    warmups: usize,
    ffmpeg: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    schema_version: &'static str,
    generated_at_unix_seconds: u64,
    build_profile: &'static str,
    fixture_dir: String,
    iterations_per_fixture: usize,
    warmups_per_fixture: usize,
    timing_boundary: TimingBoundary,
    image_bucket: BucketReport,
    audio_bucket: BucketReport,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimingBoundary {
    image_write: &'static str,
    image_read: &'static str,
    audio_prepare: &'static str,
    audio_core_write: &'static str,
    audio_write_total: &'static str,
    audio_read: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BucketReport {
    label: &'static str,
    fixture_count: usize,
    measurements_per_operation: usize,
    fixtures: Vec<FixtureReport>,
    operations: Vec<OperationSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FixtureReport {
    file: String,
    source_bytes: u64,
    source_mib: f64,
    width: Option<u32>,
    height: Option<u32>,
    duration_seconds: Option<f64>,
    sample_rate: Option<u32>,
    channels: Option<u16>,
    output_bytes: u64,
    output_mib: f64,
}

#[derive(Debug, Clone)]
struct Measurement {
    operation: &'static str,
    duration_micros: u128,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationSummary {
    operation: &'static str,
    count: usize,
    mean_ms: f64,
    median_ms: f64,
    p95_ms: f64,
    min_ms: f64,
    max_ms: f64,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_args(env::args().skip(1).collect())?;
    let image_dir = config.fixture_dir.join("images");
    let audio_dir = config.fixture_dir.join("audio");
    let images = collect_files(&image_dir, "jpg")?;
    let audio = collect_files(&audio_dir, "flac")?;
    if images.len() != 5 || audio.len() != 5 {
        return Err(format!(
            "expected exactly 5 JPEG and 5 FLAC fixtures, found {} and {}",
            images.len(),
            audio.len()
        ));
    }

    fs::create_dir_all(&config.output_dir)
        .map_err(|error| format!("create output directory: {error}"))?;
    let prepared_audio_dir = config.output_dir.join("prepared-audio");
    fs::create_dir_all(&prepared_audio_dir)
        .map_err(|error| format!("create prepared audio directory: {error}"))?;

    let (image_fixtures, image_measurements) = bench_images(&images, &config)?;
    let (audio_fixtures, audio_measurements) = bench_audio(&audio, &prepared_audio_dir, &config)?;
    let report = Report {
        schema_version: "hiddenshield-promo-performance-benchmark-v1",
        generated_at_unix_seconds: unix_seconds(),
        build_profile: if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        },
        fixture_dir: display_path(&config.fixture_dir),
        iterations_per_fixture: config.iterations,
        warmups_per_fixture: config.warmups,
        timing_boundary: TimingBoundary {
            image_write: "WatermarkService::embed on original JPEG bytes; includes JPEG decode and protected PNG encode; excludes fixture disk read",
            image_read: "WatermarkService::extract on in-memory protected PNG bytes; excludes disk read",
            audio_prepare: "FFmpeg FLAC-to-WAV decode plus prepared WAV file read",
            audio_core_write: "WatermarkService::embed on prepared WAV bytes",
            audio_write_total: "audio_prepare + audio_core_write",
            audio_read: "WatermarkService::extract on in-memory protected WAV bytes; excludes disk read",
        },
        image_bucket: bucket_report(
            "8–12 MiB, 4000×3000 (12 MP) JPEG",
            image_fixtures,
            &image_measurements,
        ),
        audio_bucket: bucket_report(
            "18–22 MiB, 180 second, 44.1 kHz stereo 16-bit FLAC",
            audio_fixtures,
            &audio_measurements,
        ),
    };

    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("serialize benchmark report: {error}"))?;
    fs::write(config.output_dir.join("report.json"), format!("{json}\n"))
        .map_err(|error| format!("write JSON report: {error}"))?;
    fs::write(
        config.output_dir.join("report.md"),
        render_markdown(&report),
    )
    .map_err(|error| format!("write Markdown report: {error}"))?;
    println!(
        "Promo performance benchmark complete: {}",
        display_path(&config.output_dir.join("report.json"))
    );
    Ok(())
}

impl Config {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut fixture_dir = None;
        let mut output_dir = None;
        let mut iterations = 5usize;
        let mut warmups = 1usize;
        let mut ffmpeg = "ffmpeg".to_string();
        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--fixture-dir" => {
                    index += 1;
                    fixture_dir = Some(PathBuf::from(required_value(
                        &args,
                        index,
                        "--fixture-dir",
                    )?));
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = Some(PathBuf::from(required_value(&args, index, "--output-dir")?));
                }
                "--iterations" => {
                    index += 1;
                    iterations = parse_usize(required_value(&args, index, "--iterations")?)?;
                }
                "--warmups" => {
                    index += 1;
                    warmups = parse_usize(required_value(&args, index, "--warmups")?)?;
                }
                "--ffmpeg" => {
                    index += 1;
                    ffmpeg = required_value(&args, index, "--ffmpeg")?.to_string();
                }
                unknown => return Err(format!("unknown argument: {unknown}")),
            }
            index += 1;
        }
        if iterations == 0 {
            return Err("--iterations must be greater than zero".into());
        }
        Ok(Self {
            fixture_dir: fixture_dir.ok_or_else(|| "missing --fixture-dir".to_string())?,
            output_dir: output_dir.ok_or_else(|| "missing --output-dir".to_string())?,
            iterations,
            warmups,
            ffmpeg,
        })
    }
}

fn bench_images(
    files: &[PathBuf],
    config: &Config,
) -> Result<(Vec<FixtureReport>, Vec<Measurement>), String> {
    let mut fixture_reports = Vec::new();
    let mut measurements = Vec::new();
    for (index, file) in files.iter().enumerate() {
        let source_bytes =
            fs::read(file).map_err(|error| format!("read image {}: {error}", file.display()))?;
        validate_byte_range(
            source_bytes.len() as u64,
            IMAGE_MIN_BYTES,
            IMAGE_MAX_BYTES,
            file,
        )?;
        let reader = ImageReader::new(std::io::Cursor::new(&source_bytes))
            .with_guessed_format()
            .map_err(|error| format!("detect image {}: {error}", file.display()))?;
        let dimensions = reader
            .into_dimensions()
            .map_err(|error| format!("read dimensions {}: {error}", file.display()))?;
        if dimensions != (EXPECTED_WIDTH, EXPECTED_HEIGHT) {
            return Err(format!(
                "{} must be {}×{}, got {}×{}",
                file.display(),
                EXPECTED_WIDTH,
                EXPECTED_HEIGHT,
                dimensions.0,
                dimensions.1
            ));
        }
        let payload = payload_for_source(&source_bytes, index as u64);
        for _ in 0..config.warmups {
            let protected = embed_image(&source_bytes, &payload)?;
            verify_image(&protected, &payload)?;
        }
        let mut last_output = Vec::new();
        for _ in 0..config.iterations {
            let started = Instant::now();
            let protected = embed_image(&source_bytes, &payload)?;
            measurements.push(Measurement {
                operation: "image_write",
                duration_micros: started.elapsed().as_micros(),
            });
            let started = Instant::now();
            verify_image(&protected, &payload)?;
            measurements.push(Measurement {
                operation: "image_read",
                duration_micros: started.elapsed().as_micros(),
            });
            last_output = protected;
        }
        fixture_reports.push(FixtureReport {
            file: display_path(file),
            source_bytes: source_bytes.len() as u64,
            source_mib: mib(source_bytes.len() as u64),
            width: Some(dimensions.0),
            height: Some(dimensions.1),
            duration_seconds: None,
            sample_rate: None,
            channels: None,
            output_bytes: last_output.len() as u64,
            output_mib: mib(last_output.len() as u64),
        });
    }
    Ok((fixture_reports, measurements))
}

fn bench_audio(
    files: &[PathBuf],
    prepared_dir: &Path,
    config: &Config,
) -> Result<(Vec<FixtureReport>, Vec<Measurement>), String> {
    let mut fixture_reports = Vec::new();
    let mut measurements = Vec::new();
    for (index, file) in files.iter().enumerate() {
        let source_size = fs::metadata(file)
            .map_err(|error| format!("read audio metadata {}: {error}", file.display()))?
            .len();
        validate_byte_range(source_size, AUDIO_MIN_BYTES, AUDIO_MAX_BYTES, file)?;
        let payload = payload_for_source(
            &fs::read(file).map_err(|error| format!("read audio {}: {error}", file.display()))?,
            10_000 + index as u64,
        );
        let prepared_path = prepared_dir.join(format!("track-{}.wav", index + 1));
        for warmup in 0..config.warmups {
            let warmup_path = prepared_dir.join(format!("warmup-{}-{warmup}.wav", index + 1));
            let prepared = prepare_audio(file, &warmup_path, &config.ffmpeg)?;
            validate_wav(&prepared, &warmup_path)?;
            let protected = embed_audio(&prepared, &payload)?;
            verify_audio(&protected, &payload)?;
            let _ = fs::remove_file(warmup_path);
        }
        let mut last_output = Vec::new();
        let mut last_prepared = Vec::new();
        for iteration in 0..config.iterations {
            let iteration_path = prepared_dir.join(format!(
                "track-{}-iteration-{}.wav",
                index + 1,
                iteration + 1
            ));
            let started = Instant::now();
            let prepared = prepare_audio(file, &iteration_path, &config.ffmpeg)?;
            let prepare_micros = started.elapsed().as_micros();
            validate_wav(&prepared, &iteration_path)?;
            measurements.push(Measurement {
                operation: "audio_prepare",
                duration_micros: prepare_micros,
            });

            let started = Instant::now();
            let protected = embed_audio(&prepared, &payload)?;
            let core_write_micros = started.elapsed().as_micros();
            measurements.push(Measurement {
                operation: "audio_core_write",
                duration_micros: core_write_micros,
            });
            measurements.push(Measurement {
                operation: "audio_write_total",
                duration_micros: prepare_micros + core_write_micros,
            });

            let started = Instant::now();
            verify_audio(&protected, &payload)?;
            measurements.push(Measurement {
                operation: "audio_read",
                duration_micros: started.elapsed().as_micros(),
            });
            last_output = protected;
            last_prepared = prepared;
            if iteration + 1 != config.iterations {
                let _ = fs::remove_file(iteration_path);
            } else {
                fs::rename(&iteration_path, &prepared_path).map_err(|error| {
                    format!(
                        "preserve prepared WAV {} -> {}: {error}",
                        iteration_path.display(),
                        prepared_path.display()
                    )
                })?;
            }
        }
        fixture_reports.push(FixtureReport {
            file: display_path(file),
            source_bytes: source_size,
            source_mib: mib(source_size),
            width: None,
            height: None,
            duration_seconds: Some(wav_duration_seconds(&last_prepared)?),
            sample_rate: Some(EXPECTED_SAMPLE_RATE),
            channels: Some(EXPECTED_CHANNELS),
            output_bytes: last_output.len() as u64,
            output_mib: mib(last_output.len() as u64),
        });
    }
    Ok((fixture_reports, measurements))
}

fn embed_image(bytes: &[u8], payload: &WatermarkPayload) -> Result<Vec<u8>, String> {
    match WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: bytes.to_vec(),
        },
        payload,
        EmbedOptions::default(),
    )
    .map_err(|error| format!("embed image: {error}"))?
    {
        MediaOutput::ImageBytes { bytes, .. } => Ok(bytes),
        _ => Err("unexpected non-image output".into()),
    }
}

fn verify_image(bytes: &[u8], payload: &WatermarkPayload) -> Result<(), String> {
    let extracted = WatermarkService::extract(MediaInput::ImageBytes {
        bytes: bytes.to_vec(),
    })
    .map_err(|error| format!("extract image: {error}"))?;
    if extracted.watermark_uid() != payload.watermark_uid() {
        return Err("image UID mismatch".into());
    }
    Ok(())
}

fn embed_audio(bytes: &[u8], payload: &WatermarkPayload) -> Result<Vec<u8>, String> {
    match WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: bytes.to_vec(),
        },
        payload,
        EmbedOptions::default(),
    )
    .map_err(|error| format!("embed audio: {error}"))?
    {
        MediaOutput::AudioWavBytes { bytes } => Ok(bytes),
        _ => Err("unexpected non-audio output".into()),
    }
}

fn verify_audio(bytes: &[u8], payload: &WatermarkPayload) -> Result<(), String> {
    let extracted = WatermarkService::extract(MediaInput::AudioWavBytes {
        bytes: bytes.to_vec(),
    })
    .map_err(|error| format!("extract audio: {error}"))?;
    if extracted.watermark_uid() != payload.watermark_uid() {
        return Err("audio UID mismatch".into());
    }
    Ok(())
}

fn prepare_audio(source: &Path, output: &Path, ffmpeg: &str) -> Result<Vec<u8>, String> {
    let command = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-y",
            "-i",
            &source.display().to_string(),
            "-vn",
            "-ar",
            "44100",
            "-ac",
            "2",
            "-c:a",
            "pcm_s16le",
            &output.display().to_string(),
        ])
        .output()
        .map_err(|error| format!("start FFmpeg: {error}"))?;
    if !command.status.success() {
        return Err(format!(
            "FFmpeg failed for {}: {}",
            source.display(),
            String::from_utf8_lossy(&command.stderr)
        ));
    }
    fs::read(output).map_err(|error| format!("read prepared WAV {}: {error}", output.display()))
}

fn validate_wav(bytes: &[u8], path: &Path) -> Result<(), String> {
    let reader = hound::WavReader::new(std::io::Cursor::new(bytes))
        .map_err(|error| format!("open WAV {}: {error}", path.display()))?;
    let spec = reader.spec();
    let duration = reader.duration() as f64 / spec.sample_rate as f64;
    if spec.sample_rate != EXPECTED_SAMPLE_RATE
        || spec.channels != EXPECTED_CHANNELS
        || (duration - EXPECTED_AUDIO_SECONDS).abs() > 0.01
    {
        return Err(format!(
            "{} has unexpected WAV spec: {} Hz, {} channels, {:.3} seconds",
            path.display(),
            spec.sample_rate,
            spec.channels,
            duration
        ));
    }
    Ok(())
}

fn wav_duration_seconds(bytes: &[u8]) -> Result<f64, String> {
    let reader = hound::WavReader::new(std::io::Cursor::new(bytes))
        .map_err(|error| format!("open prepared WAV: {error}"))?;
    let spec = reader.spec();
    Ok(reader.duration() as f64 / spec.sample_rate as f64)
}

fn bucket_report(
    label: &'static str,
    fixtures: Vec<FixtureReport>,
    measurements: &[Measurement],
) -> BucketReport {
    let mut operations = measurements
        .iter()
        .map(|measurement| measurement.operation)
        .collect::<Vec<_>>();
    operations.sort_unstable();
    operations.dedup();
    BucketReport {
        label,
        fixture_count: fixtures.len(),
        measurements_per_operation: measurements
            .iter()
            .filter(|measurement| measurement.operation == operations[0])
            .count(),
        fixtures,
        operations: operations
            .into_iter()
            .map(|operation| summarize(operation, measurements))
            .collect(),
    }
}

fn summarize(operation: &'static str, measurements: &[Measurement]) -> OperationSummary {
    let mut values = measurements
        .iter()
        .filter(|measurement| measurement.operation == operation)
        .map(|measurement| measurement.duration_micros)
        .collect::<Vec<_>>();
    values.sort_unstable();
    let count = values.len();
    let mean = values.iter().copied().sum::<u128>() as f64 / count as f64;
    OperationSummary {
        operation,
        count,
        mean_ms: micros_to_ms(mean),
        median_ms: micros_to_ms(percentile(&values, 0.50) as f64),
        p95_ms: micros_to_ms(percentile(&values, 0.95) as f64),
        min_ms: micros_to_ms(values[0] as f64),
        max_ms: micros_to_ms(values[count - 1] as f64),
    }
}

fn percentile(sorted: &[u128], percentile: f64) -> u128 {
    let rank = ((sorted.len() as f64 * percentile).ceil() as usize).saturating_sub(1);
    sorted[rank.min(sorted.len() - 1)]
}

fn render_markdown(report: &Report) -> String {
    let mut output = String::from("# HiddenShield 宣传性能专项基准\n\n");
    output.push_str(&format!(
        "- 构建模式：`{}`\n- 每个素材预热：`{}` 次\n- 每个素材正式测量：`{}` 次\n\n",
        report.build_profile, report.warmups_per_fixture, report.iterations_per_fixture
    ));
    for bucket in [&report.image_bucket, &report.audio_bucket] {
        output.push_str(&format!("## {}\n\n", bucket.label));
        output.push_str("| Operation | Count | Mean ms | Median ms | P95 ms | Min ms | Max ms |\n");
        output.push_str("| --- | ---: | ---: | ---: | ---: | ---: | ---: |\n");
        for operation in &bucket.operations {
            output.push_str(&format!(
                "| {} | {} | {:.2} | {:.2} | {:.2} | {:.2} | {:.2} |\n",
                operation.operation,
                operation.count,
                operation.mean_ms,
                operation.median_ms,
                operation.p95_ms,
                operation.min_ms,
                operation.max_ms
            ));
        }
        output.push('\n');
    }
    output.push_str("## 计时边界\n\n");
    output.push_str(&format!(
        "- 图片写入：{}\n- 图片读取：{}\n- 音频准备：{}\n- 音频核心写入：{}\n- 音频写入总计：{}\n- 音频读取：{}\n",
        report.timing_boundary.image_write,
        report.timing_boundary.image_read,
        report.timing_boundary.audio_prepare,
        report.timing_boundary.audio_core_write,
        report.timing_boundary.audio_write_total,
        report.timing_boundary.audio_read
    ));
    output
}

fn collect_files(dir: &Path, extension: &str) -> Result<Vec<PathBuf>, String> {
    let mut files = fs::read_dir(dir)
        .map_err(|error| format!("read fixture directory {}: {error}", dir.display()))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.extension()
                .and_then(OsStr::to_str)
                .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        })
        .collect::<Vec<_>>();
    files.sort();
    Ok(files)
}

fn validate_byte_range(bytes: u64, minimum: u64, maximum: u64, path: &Path) -> Result<(), String> {
    if bytes < minimum || bytes > maximum {
        return Err(format!(
            "{} size {} is outside {}..{} bytes",
            path.display(),
            bytes,
            minimum,
            maximum
        ));
    }
    Ok(())
}

fn payload_for_source(bytes: &[u8], salt: u64) -> WatermarkPayload {
    let digest = Sha256::digest(bytes);
    let mut user_seed = [0u8; 8];
    user_seed.copy_from_slice(&digest[0..8]);
    user_seed[0] ^= (salt & 0xFF) as u8;
    let mut device_id = [0u8; 4];
    device_id.copy_from_slice(&digest[8..12]);
    let mut file_hash = [0u8; 2];
    file_hash.copy_from_slice(&digest[12..14]);
    WatermarkPayload::new(
        user_seed,
        1_900_000_000 + salt,
        device_id,
        file_hash,
        Default::default(),
    )
}

fn mib(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

fn micros_to_ms(value: f64) -> f64 {
    value / 1_000.0
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
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

fn unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

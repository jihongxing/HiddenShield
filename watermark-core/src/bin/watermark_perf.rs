use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use image::{DynamicImage, ImageFormat};
use sha2::{Digest, Sha256};
use watermark_core::{EmbedOptions, MediaInput, MediaOutput, WatermarkPayload, WatermarkService};

#[derive(Debug, Clone)]
struct Config {
    image_dir: Option<PathBuf>,
    audio_glob: Option<String>,
    max_images: usize,
    max_audio: usize,
    output_dir: PathBuf,
    ffmpeg: String,
}

#[derive(Debug, Clone)]
struct PerfRow {
    media: &'static str,
    source: String,
    operation: &'static str,
    duration_ms: u128,
    bytes: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_args(env::args().skip(1).collect())?;
    let run_dir = config.output_dir.join(format!("perf-{}", unix_seconds()));
    fs::create_dir_all(&run_dir).map_err(|error| format!("create perf dir: {error}"))?;

    let mut rows = Vec::new();
    for (index, source) in collect_images(config.image_dir.as_deref(), config.max_images)?
        .iter()
        .enumerate()
    {
        rows.extend(run_image_perf(source, index)?);
    }
    for (index, source) in collect_audio(config.audio_glob.as_deref(), config.max_audio)?
        .iter()
        .enumerate()
    {
        rows.extend(run_audio_perf(source, index, &run_dir, &config.ffmpeg)?);
    }

    if rows.is_empty() {
        return Err("no image or audio sources found".into());
    }

    write_report(&run_dir.join("perf.md"), &rows)?;
    println!("Watermark perf finished: {} measurements", rows.len());
    println!("Report: {}", display_path(&run_dir.join("perf.md")));
    print_summary(&rows);
    Ok(())
}

impl Config {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut image_dir = None;
        let mut audio_glob = None;
        let mut max_images = 3usize;
        let mut max_audio = 0usize;
        let mut output_dir = PathBuf::from("watermark-core/target/watermark-perf");
        let mut ffmpeg = "ffmpeg".to_string();

        let mut index = 0;
        while index < args.len() {
            match args[index].as_str() {
                "--image-dir" => {
                    index += 1;
                    image_dir = Some(PathBuf::from(required_value(&args, index, "--image-dir")?));
                }
                "--audio-glob" => {
                    index += 1;
                    audio_glob = Some(required_value(&args, index, "--audio-glob")?.to_string());
                }
                "--max-images" => {
                    index += 1;
                    max_images = parse_usize(required_value(&args, index, "--max-images")?)?;
                }
                "--max-audio" => {
                    index += 1;
                    max_audio = parse_usize(required_value(&args, index, "--max-audio")?)?;
                }
                "--output-dir" => {
                    index += 1;
                    output_dir = PathBuf::from(required_value(&args, index, "--output-dir")?);
                }
                "--ffmpeg" => {
                    index += 1;
                    ffmpeg = required_value(&args, index, "--ffmpeg")?.to_string();
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
            image_dir,
            audio_glob,
            max_images,
            max_audio,
            output_dir,
            ffmpeg,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run --manifest-path watermark-core/Cargo.toml --bin watermark_perf -- \
  --image-dir <dir> [--audio-glob <path/*.mp3>] [--max-images 3] [--max-audio 0]"
    );
}

fn run_image_perf(source: &Path, index: usize) -> Result<Vec<PerfRow>, String> {
    let source_bytes = fs::read(source)
        .map_err(|error| format!("read image source '{}': {error}", source.display()))?;
    let payload = payload_for_source(&source_bytes, index as u64);
    let source_name = display_name(source);

    let started = Instant::now();
    let output = WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: source_bytes.clone(),
        },
        &payload,
        EmbedOptions::default(),
    )
    .map_err(|error| format!("embed image '{}': {error}", source.display()))?;
    let embed_ms = started.elapsed().as_millis();
    let MediaOutput::ImageBytes { bytes, .. } = output else {
        return Err("unexpected non-image output".into());
    };

    let mut rows = vec![PerfRow {
        media: "image",
        source: source_name.clone(),
        operation: "embed_png",
        duration_ms: embed_ms,
        bytes: source_bytes.len(),
    }];

    let started = Instant::now();
    let _ = WatermarkService::extract(MediaInput::ImageBytes {
        bytes: bytes.clone(),
    })
    .map_err(|error| format!("extract embedded image '{}': {error}", source.display()))?;
    rows.push(PerfRow {
        media: "image",
        source: source_name.clone(),
        operation: "extract_embedded_png",
        duration_ms: started.elapsed().as_millis(),
        bytes: bytes.len(),
    });

    let cropped = crop_2_percent_png(&bytes)?;
    let started = Instant::now();
    let _ = WatermarkService::extract(MediaInput::ImageBytes {
        bytes: cropped.clone(),
    })
    .map_err(|error| format!("extract cropped image '{}': {error}", source.display()))?;
    rows.push(PerfRow {
        media: "image",
        source: source_name,
        operation: "extract_crop_2_percent",
        duration_ms: started.elapsed().as_millis(),
        bytes: cropped.len(),
    });

    Ok(rows)
}

fn run_audio_perf(
    source: &Path,
    index: usize,
    run_dir: &Path,
    ffmpeg: &str,
) -> Result<Vec<PerfRow>, String> {
    let audio_dir = run_dir.join(format!("audio-{index:02}"));
    fs::create_dir_all(&audio_dir).map_err(|error| format!("create audio dir: {error}"))?;
    let source_wav = audio_dir.join("source.wav");
    run_ffmpeg(
        ffmpeg,
        &[
            "-y",
            "-i",
            &source.display().to_string(),
            "-t",
            "30",
            "-ar",
            "44100",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            &source_wav.display().to_string(),
        ],
    )?;

    let source_bytes = fs::read(&source_wav).map_err(|error| format!("read wav: {error}"))?;
    let payload = payload_for_source(&source_bytes, 10_000 + index as u64);
    let source_name = display_name(source);

    let started = Instant::now();
    let output = WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: source_bytes.clone(),
        },
        &payload,
        EmbedOptions::default(),
    )
    .map_err(|error| format!("embed audio '{}': {error}", source.display()))?;
    let embed_ms = started.elapsed().as_millis();
    let MediaOutput::AudioWavBytes { bytes } = output else {
        return Err("unexpected non-audio output".into());
    };

    let started = Instant::now();
    let _ = WatermarkService::extract(MediaInput::AudioWavBytes {
        bytes: bytes.clone(),
    })
    .map_err(|error| format!("extract audio '{}': {error}", source.display()))?;
    let extract_ms = started.elapsed().as_millis();

    Ok(vec![
        PerfRow {
            media: "audio",
            source: source_name.clone(),
            operation: "embed_wav_30s",
            duration_ms: embed_ms,
            bytes: source_bytes.len(),
        },
        PerfRow {
            media: "audio",
            source: source_name,
            operation: "extract_wav_30s",
            duration_ms: extract_ms,
            bytes: bytes.len(),
        },
    ])
}

fn crop_2_percent_png(image_bytes: &[u8]) -> Result<Vec<u8>, String> {
    let img =
        image::load_from_memory(image_bytes).map_err(|error| format!("open image: {error}"))?;
    let crop_x = (img.width() / 50).max(1);
    let crop_y = (img.height() / 50).max(1);
    let cropped = img.crop_imm(
        crop_x,
        crop_y,
        img.width() - crop_x * 2,
        img.height() - crop_y * 2,
    );
    encode_image(&cropped, ImageFormat::Png)
}

fn encode_image(img: &DynamicImage, format: ImageFormat) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    img.write_to(&mut cursor, format)
        .map_err(|error| format!("encode image: {error}"))?;
    Ok(cursor.into_inner())
}

fn collect_images(image_dir: Option<&Path>, limit: usize) -> Result<Vec<PathBuf>, String> {
    let Some(image_dir) = image_dir else {
        return Ok(Vec::new());
    };
    if limit == 0 {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(image_dir).map_err(|error| format!("read image dir: {error}"))? {
        let path = entry
            .map_err(|error| format!("read image dir entry: {error}"))?
            .path();
        if is_supported_image(&path) {
            paths.push(path);
        }
    }
    paths.sort();
    paths.truncate(limit);
    Ok(paths)
}

fn collect_audio(audio_glob: Option<&str>, limit: usize) -> Result<Vec<PathBuf>, String> {
    let Some(audio_glob) = audio_glob else {
        return Ok(Vec::new());
    };
    if limit == 0 {
        return Ok(Vec::new());
    }
    let glob_path = PathBuf::from(audio_glob);
    let parent = glob_path
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_pattern = glob_path
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| format!("invalid audio glob: {audio_glob}"))?;
    let extension = file_pattern
        .strip_prefix("*.")
        .ok_or_else(|| "only simple audio globs like path/*.mp3 are supported".to_string())?
        .to_ascii_lowercase();

    let mut paths = Vec::new();
    for entry in fs::read_dir(parent).map_err(|error| format!("read audio dir: {error}"))? {
        let path = entry
            .map_err(|error| format!("read audio dir entry: {error}"))?
            .path();
        if path
            .extension()
            .and_then(OsStr::to_str)
            .map(|ext| ext.eq_ignore_ascii_case(&extension))
            .unwrap_or(false)
        {
            paths.push(path);
        }
    }
    paths.sort();
    paths.truncate(limit);
    Ok(paths)
}

fn is_supported_image(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp" | "bmp" | "tif" | "tiff"
            )
        })
        .unwrap_or(false)
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
        1_800_000_000 + salt,
        device_id,
        file_hash,
        Default::default(),
    )
}

fn run_ffmpeg(ffmpeg: &str, args: &[&str]) -> Result<(), String> {
    let output = Command::new(ffmpeg)
        .args(args)
        .output()
        .map_err(|error| format!("start ffmpeg: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "ffmpeg failed: {}",
            String::from_utf8_lossy(&output.stderr)
                .lines()
                .rev()
                .take(6)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
}

fn write_report(path: &Path, rows: &[PerfRow]) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("# Watermark Performance Bench\n\n");
    out.push_str("| Media | Source | Operation | Duration ms | Bytes |\n");
    out.push_str("| --- | --- | --- | ---: | ---: |\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            row.media,
            escape_md(&row.source),
            row.operation,
            row.duration_ms,
            row.bytes
        ));
    }
    out.push_str("\n## Averages\n\n");
    out.push_str("| Media | Operation | Count | Avg ms | Max ms |\n");
    out.push_str("| --- | --- | ---: | ---: | ---: |\n");
    for (media, operation) in unique_groups(rows) {
        let matching = rows
            .iter()
            .filter(|row| row.media == media && row.operation == operation)
            .collect::<Vec<_>>();
        let sum = matching.iter().map(|row| row.duration_ms).sum::<u128>();
        let max = matching
            .iter()
            .map(|row| row.duration_ms)
            .max()
            .unwrap_or(0);
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            media,
            operation,
            matching.len(),
            sum / matching.len() as u128,
            max
        ));
    }
    fs::write(path, out).map_err(|error| format!("write perf report: {error}"))
}

fn print_summary(rows: &[PerfRow]) {
    for (media, operation) in unique_groups(rows) {
        let matching = rows
            .iter()
            .filter(|row| row.media == media && row.operation == operation)
            .collect::<Vec<_>>();
        let sum = matching.iter().map(|row| row.duration_ms).sum::<u128>();
        let max = matching
            .iter()
            .map(|row| row.duration_ms)
            .max()
            .unwrap_or(0);
        println!(
            "{} {}: count={}, avg={} ms, max={} ms",
            media,
            operation,
            matching.len(),
            sum / matching.len() as u128,
            max
        );
    }
}

fn unique_groups(rows: &[PerfRow]) -> Vec<(&'static str, &'static str)> {
    let mut groups = Vec::new();
    for row in rows {
        let group = (row.media, row.operation);
        if !groups.contains(&group) {
            groups.push(group);
        }
    }
    groups
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

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(OsStr::to_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| path.to_string_lossy().to_string())
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn escape_md(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

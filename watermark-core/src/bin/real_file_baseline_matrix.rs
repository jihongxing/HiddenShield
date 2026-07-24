use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use image::GenericImageView;
use serde::Serialize;
use sha2::{Digest, Sha256};
use watermark_core::{
    EmbedOptions, MediaInput, MediaOutput, WatermarkDecodedPayload, WatermarkPayload,
    WatermarkService,
};

#[derive(Debug)]
struct Config {
    image_dir: Option<PathBuf>,
    audio_dir: Option<PathBuf>,
    audio_filter: Option<String>,
    output: PathBuf,
    ffmpeg: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    schema_version: u32,
    scope: &'static str,
    image_results: Vec<ImageResult>,
    audio_results: Vec<AudioResult>,
    summary: Summary,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Summary {
    total: usize,
    passed: usize,
    failed: usize,
    image_passed: usize,
    image_failed: usize,
    audio_passed: usize,
    audio_failed: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ImageResult {
    source: String,
    input_format: String,
    width: Option<u32>,
    height: Option<u32>,
    output_format: Option<String>,
    input_bytes: usize,
    output_bytes: Option<usize>,
    embed_ms: Option<u128>,
    extract_ms: Option<u128>,
    embed_ok: bool,
    extract_ok: bool,
    payload_matches: bool,
    passed: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AudioResult {
    source: String,
    input_format: String,
    decoded_sample_rate: Option<u32>,
    decoded_channels: Option<u16>,
    decoded_duration_seconds: Option<f64>,
    output_sample_rate: Option<u32>,
    output_channels: Option<u16>,
    decoded_wav_bytes: Option<usize>,
    output_wav_bytes: Option<usize>,
    decode_ms: Option<u128>,
    embed_ms: Option<u128>,
    extract_ms: Option<u128>,
    decode_ok: bool,
    embed_ok: bool,
    extract_ok: bool,
    payload_matches: bool,
    passed: bool,
    error: Option<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_args(env::args().skip(1).collect())?;
    if let Some(parent) = config.output.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create report directory '{}': {error}", parent.display()))?;
    }

    let image_sources = collect_files(config.image_dir.as_deref(), is_supported_image)?;
    let audio_sources = collect_audio(config.audio_dir.as_deref(), config.audio_filter.as_deref())?;
    if image_sources.is_empty() && audio_sources.is_empty() {
        return Err("no matching image or audio sources found".into());
    }

    let mut image_results = Vec::new();
    for (index, source) in image_sources.iter().enumerate() {
        println!("IMAGE {}", source.display());
        image_results.push(run_image_baseline(source, index));
    }

    let work_dir = config
        .output
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("baseline-decoded-audio");
    fs::create_dir_all(&work_dir).map_err(|error| {
        format!(
            "create audio work directory '{}': {error}",
            work_dir.display()
        )
    })?;

    let mut audio_results = Vec::new();
    for (index, source) in audio_sources.iter().enumerate() {
        println!("AUDIO {}", source.display());
        audio_results.push(run_audio_baseline(source, index, &work_dir, &config.ffmpeg));
    }

    let summary = summarize(&image_results, &audio_results);
    let report = Report {
        schema_version: 1,
        scope: "real_file_embed_then_immediate_extract_without_perturbation",
        image_results,
        audio_results,
        summary,
    };
    let json = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("serialize baseline report: {error}"))?;
    fs::write(&config.output, format!("{json}\n"))
        .map_err(|error| format!("write report '{}': {error}", config.output.display()))?;
    write_markdown(&config.output.with_extension("md"), &report)?;

    println!(
        "Baseline matrix finished: {}/{} passed",
        report.summary.passed, report.summary.total
    );
    println!("JSON: {}", config.output.display());
    println!("Markdown: {}", config.output.with_extension("md").display());
    Ok(())
}

impl Config {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut image_dir = None;
        let mut audio_dir = None;
        let mut audio_filter = None;
        let mut output = PathBuf::from("watermark-core/target/real-file-baseline/report.json");
        let mut ffmpeg = "ffmpeg".to_string();
        let mut index = 0;

        while index < args.len() {
            match args[index].as_str() {
                "--image-dir" => {
                    index += 1;
                    image_dir = Some(PathBuf::from(required_value(&args, index, "--image-dir")?));
                }
                "--audio-dir" => {
                    index += 1;
                    audio_dir = Some(PathBuf::from(required_value(&args, index, "--audio-dir")?));
                }
                "--audio-filter" => {
                    index += 1;
                    audio_filter =
                        Some(required_value(&args, index, "--audio-filter")?.to_ascii_lowercase());
                }
                "--output" => {
                    index += 1;
                    output = PathBuf::from(required_value(&args, index, "--output")?);
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
            audio_dir,
            audio_filter,
            output,
            ffmpeg,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run --manifest-path watermark-core/Cargo.toml \
--bin real_file_baseline_matrix -- \
[--image-dir <dir>] [--audio-dir <dir>] [--audio-filter <text>] \
[--output <report.json>] [--ffmpeg <path>]"
    );
}

fn run_image_baseline(source: &Path, index: usize) -> ImageResult {
    let source_name = display_name(source);
    let input_format = extension(source);
    let source_bytes = match fs::read(source) {
        Ok(bytes) => bytes,
        Err(error) => {
            return ImageResult {
                source: source_name,
                input_format,
                width: None,
                height: None,
                output_format: None,
                input_bytes: 0,
                output_bytes: None,
                embed_ms: None,
                extract_ms: None,
                embed_ok: false,
                extract_ok: false,
                payload_matches: false,
                passed: false,
                error: Some(format!("read source: {error}")),
            };
        }
    };
    let dimensions = image::load_from_memory(&source_bytes)
        .ok()
        .map(|image| image.dimensions());
    let payload = payload_for_source(&source_bytes, index as u64);

    let embed_started = Instant::now();
    let output = match WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: source_bytes.clone(),
        },
        &payload,
        EmbedOptions::default(),
    ) {
        Ok(output) => output,
        Err(error) => {
            return ImageResult {
                source: source_name,
                input_format,
                width: dimensions.map(|value| value.0),
                height: dimensions.map(|value| value.1),
                output_format: None,
                input_bytes: source_bytes.len(),
                output_bytes: None,
                embed_ms: Some(embed_started.elapsed().as_millis()),
                extract_ms: None,
                embed_ok: false,
                extract_ok: false,
                payload_matches: false,
                passed: false,
                error: Some(format!("embed: {error}")),
            };
        }
    };
    let embed_ms = embed_started.elapsed().as_millis();
    let MediaOutput::ImageBytes { bytes, format } = output else {
        return ImageResult {
            source: source_name,
            input_format,
            width: dimensions.map(|value| value.0),
            height: dimensions.map(|value| value.1),
            output_format: None,
            input_bytes: source_bytes.len(),
            output_bytes: None,
            embed_ms: Some(embed_ms),
            extract_ms: None,
            embed_ok: false,
            extract_ok: false,
            payload_matches: false,
            passed: false,
            error: Some("embed returned non-image output".into()),
        };
    };

    let extract_started = Instant::now();
    let extracted = WatermarkService::extract(MediaInput::ImageBytes {
        bytes: bytes.clone(),
    });
    let extract_ms = extract_started.elapsed().as_millis();
    let (extract_ok, payload_matches, error) = extraction_result(extracted, &payload);
    ImageResult {
        source: source_name,
        input_format,
        width: dimensions.map(|value| value.0),
        height: dimensions.map(|value| value.1),
        output_format: Some(format!("{format:?}").to_ascii_lowercase()),
        input_bytes: source_bytes.len(),
        output_bytes: Some(bytes.len()),
        embed_ms: Some(embed_ms),
        extract_ms: Some(extract_ms),
        embed_ok: true,
        extract_ok,
        payload_matches,
        passed: extract_ok && payload_matches,
        error,
    }
}

fn run_audio_baseline(source: &Path, index: usize, work_dir: &Path, ffmpeg: &str) -> AudioResult {
    let source_name = display_name(source);
    let input_format = extension(source);
    let decoded_wav = work_dir.join(format!("{index:02}-{}.wav", safe_stem(source)));

    let decode_started = Instant::now();
    if let Err(error) = run_ffmpeg(
        ffmpeg,
        &[
            "-y",
            "-i",
            &source.display().to_string(),
            "-map",
            "0:a:0",
            "-c:a",
            "pcm_s16le",
            &decoded_wav.display().to_string(),
        ],
    ) {
        return AudioResult {
            source: source_name,
            input_format,
            decoded_sample_rate: None,
            decoded_channels: None,
            decoded_duration_seconds: None,
            output_sample_rate: None,
            output_channels: None,
            decoded_wav_bytes: None,
            output_wav_bytes: None,
            decode_ms: Some(decode_started.elapsed().as_millis()),
            embed_ms: None,
            extract_ms: None,
            decode_ok: false,
            embed_ok: false,
            extract_ok: false,
            payload_matches: false,
            passed: false,
            error: Some(format!("decode: {error}")),
        };
    }
    let decode_ms = decode_started.elapsed().as_millis();
    let wav_bytes = match fs::read(&decoded_wav) {
        Ok(bytes) => bytes,
        Err(error) => {
            return AudioResult {
                source: source_name,
                input_format,
                decoded_sample_rate: None,
                decoded_channels: None,
                decoded_duration_seconds: None,
                output_sample_rate: None,
                output_channels: None,
                decoded_wav_bytes: None,
                output_wav_bytes: None,
                decode_ms: Some(decode_ms),
                embed_ms: None,
                extract_ms: None,
                decode_ok: false,
                embed_ok: false,
                extract_ok: false,
                payload_matches: false,
                passed: false,
                error: Some(format!("read decoded wav: {error}")),
            };
        }
    };
    let metadata = wav_metadata(&wav_bytes);
    let payload = payload_for_source(&wav_bytes, 10_000 + index as u64);

    let embed_started = Instant::now();
    let output = match WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: wav_bytes.clone(),
        },
        &payload,
        EmbedOptions::default(),
    ) {
        Ok(output) => output,
        Err(error) => {
            return AudioResult {
                source: source_name,
                input_format,
                decoded_sample_rate: metadata.as_ref().map(|value| value.0),
                decoded_channels: metadata.as_ref().map(|value| value.1),
                decoded_duration_seconds: metadata.as_ref().map(|value| value.2),
                output_sample_rate: None,
                output_channels: None,
                decoded_wav_bytes: Some(wav_bytes.len()),
                output_wav_bytes: None,
                decode_ms: Some(decode_ms),
                embed_ms: Some(embed_started.elapsed().as_millis()),
                extract_ms: None,
                decode_ok: true,
                embed_ok: false,
                extract_ok: false,
                payload_matches: false,
                passed: false,
                error: Some(format!("embed: {error}")),
            };
        }
    };
    let embed_ms = embed_started.elapsed().as_millis();
    let MediaOutput::AudioWavBytes { bytes } = output else {
        return AudioResult {
            source: source_name,
            input_format,
            decoded_sample_rate: metadata.as_ref().map(|value| value.0),
            decoded_channels: metadata.as_ref().map(|value| value.1),
            decoded_duration_seconds: metadata.as_ref().map(|value| value.2),
            output_sample_rate: None,
            output_channels: None,
            decoded_wav_bytes: Some(wav_bytes.len()),
            output_wav_bytes: None,
            decode_ms: Some(decode_ms),
            embed_ms: Some(embed_ms),
            extract_ms: None,
            decode_ok: true,
            embed_ok: false,
            extract_ok: false,
            payload_matches: false,
            passed: false,
            error: Some("embed returned non-audio output".into()),
        };
    };

    let extract_started = Instant::now();
    let extracted = WatermarkService::extract(MediaInput::AudioWavBytes {
        bytes: bytes.clone(),
    });
    let extract_ms = extract_started.elapsed().as_millis();
    let (extract_ok, payload_matches, error) = extraction_result(extracted, &payload);
    let output_metadata = wav_metadata(&bytes);
    let spec_preserved = metadata
        .as_ref()
        .zip(output_metadata.as_ref())
        .map(|(input, output)| input.0 == output.0 && input.1 == output.1)
        .unwrap_or(false);
    let error = if extract_ok && payload_matches && !spec_preserved {
        Some("output WAV sample rate or channel count changed".into())
    } else {
        error
    };
    AudioResult {
        source: source_name,
        input_format,
        decoded_sample_rate: metadata.as_ref().map(|value| value.0),
        decoded_channels: metadata.as_ref().map(|value| value.1),
        decoded_duration_seconds: metadata.as_ref().map(|value| value.2),
        output_sample_rate: output_metadata.as_ref().map(|value| value.0),
        output_channels: output_metadata.as_ref().map(|value| value.1),
        decoded_wav_bytes: Some(wav_bytes.len()),
        output_wav_bytes: Some(bytes.len()),
        decode_ms: Some(decode_ms),
        embed_ms: Some(embed_ms),
        extract_ms: Some(extract_ms),
        decode_ok: true,
        embed_ok: true,
        extract_ok,
        payload_matches,
        passed: extract_ok && payload_matches && spec_preserved,
        error,
    }
}

fn extraction_result(
    result: Result<WatermarkDecodedPayload, watermark_core::WatermarkError>,
    expected: &WatermarkPayload,
) -> (bool, bool, Option<String>) {
    match result {
        Ok(decoded) => {
            let actual_id = match decoded {
                WatermarkDecodedPayload::V2(payload) => payload.watermark_id,
                WatermarkDecodedPayload::V3MinimalAnchor(payload) => payload.watermark_id,
            };
            let matches = actual_id == expected.watermark_id;
            (
                true,
                matches,
                (!matches).then(|| "extracted watermark id does not match embedded payload".into()),
            )
        }
        Err(error) => (false, false, Some(format!("extract: {error}"))),
    }
}

fn wav_metadata(bytes: &[u8]) -> Option<(u32, u16, f64)> {
    let reader = hound::WavReader::new(Cursor::new(bytes)).ok()?;
    let spec = reader.spec();
    let duration = reader.duration() as f64 / spec.sample_rate as f64;
    Some((spec.sample_rate, spec.channels, duration))
}

fn summarize(images: &[ImageResult], audio: &[AudioResult]) -> Summary {
    let image_passed = images.iter().filter(|result| result.passed).count();
    let audio_passed = audio.iter().filter(|result| result.passed).count();
    Summary {
        total: images.len() + audio.len(),
        passed: image_passed + audio_passed,
        failed: images.len() + audio.len() - image_passed - audio_passed,
        image_passed,
        image_failed: images.len() - image_passed,
        audio_passed,
        audio_failed: audio.len() - audio_passed,
    }
}

fn collect_files(
    directory: Option<&Path>,
    predicate: fn(&Path) -> bool,
) -> Result<Vec<PathBuf>, String> {
    let Some(directory) = directory else {
        return Ok(Vec::new());
    };
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("read directory '{}': {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("read directory entry: {error}"))?
            .path();
        if path.is_file() && predicate(&path) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn collect_audio(directory: Option<&Path>, filter: Option<&str>) -> Result<Vec<PathBuf>, String> {
    let mut paths = collect_files(directory, is_supported_audio)?;
    if let Some(filter) = filter {
        paths.retain(|path| display_name(path).to_ascii_lowercase().contains(filter));
    }
    Ok(paths)
}

fn is_supported_image(path: &Path) -> bool {
    matches!(extension(path).as_str(), "png" | "jpg" | "jpeg" | "webp")
}

fn is_supported_audio(path: &Path) -> bool {
    matches!(
        extension(path).as_str(),
        "wav" | "mp3" | "flac" | "ogg" | "m4a"
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
        Err(String::from_utf8_lossy(&output.stderr)
            .lines()
            .rev()
            .take(8)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n"))
    }
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

fn write_markdown(path: &Path, report: &Report) -> Result<(), String> {
    let mut markdown = String::from("# Real-file Watermark Baseline Matrix\n\n");
    markdown.push_str(
        "Scope: embed and immediate extract only. No crop, re-encode, noise, codec, or other perturbation is applied.\n\n",
    );
    markdown.push_str("## Images\n\n");
    markdown.push_str(
        "| Source | Input | Resolution | Output | Embed | Extract | Match | Result | Error |\n",
    );
    markdown.push_str("| --- | --- | --- | --- | ---: | ---: | --- | --- | --- |\n");
    for result in &report.image_results {
        markdown.push_str(&format!(
            "| {} | {} | {}×{} | {} | {} ms | {} ms | {} | {} | {} |\n",
            escape_markdown(&result.source),
            result.input_format,
            optional_number(result.width),
            optional_number(result.height),
            result.output_format.as_deref().unwrap_or("-"),
            optional_number(result.embed_ms),
            optional_number(result.extract_ms),
            yes_no(result.payload_matches),
            pass_fail(result.passed),
            escape_markdown(result.error.as_deref().unwrap_or("")),
        ));
    }

    markdown.push_str("\n## Audio\n\n");
    markdown.push_str("| Source | Container | Input WAV | Output WAV | Duration | Decode | Embed | Extract | Match | Result | Error |\n");
    markdown.push_str("| --- | --- | --- | --- | ---: | ---: | ---: | ---: | --- | --- | --- |\n");
    for result in &report.audio_results {
        markdown.push_str(&format!(
            "| {} | {} | {} Hz / {} ch | {} Hz / {} ch | {:.3} s | {} ms | {} ms | {} ms | {} | {} | {} |\n",
            escape_markdown(&result.source),
            result.input_format,
            optional_number(result.decoded_sample_rate),
            optional_number(result.decoded_channels),
            optional_number(result.output_sample_rate),
            optional_number(result.output_channels),
            result.decoded_duration_seconds.unwrap_or_default(),
            optional_number(result.decode_ms),
            optional_number(result.embed_ms),
            optional_number(result.extract_ms),
            yes_no(result.payload_matches),
            pass_fail(result.passed),
            escape_markdown(result.error.as_deref().unwrap_or("")),
        ));
    }

    markdown.push_str(&format!(
        "\n## Summary\n\n- Total: {}\n- Passed: {}\n- Failed: {}\n",
        report.summary.total, report.summary.passed, report.summary.failed
    ));
    fs::write(path, markdown)
        .map_err(|error| format!("write markdown report '{}': {error}", path.display()))
}

fn required_value<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, String> {
    args.get(index)
        .map(String::as_str)
        .ok_or_else(|| format!("missing value for {flag}"))
}

fn extension(path: &Path) -> String {
    path.extension()
        .and_then(OsStr::to_str)
        .unwrap_or("")
        .to_ascii_lowercase()
}

fn display_name(path: &Path) -> String {
    path.file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_else(|| path.to_str().unwrap_or("<unknown>"))
        .to_string()
}

fn safe_stem(path: &Path) -> String {
    path.file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("audio")
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn optional_number<T: ToString>(value: Option<T>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".into())
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn pass_fail(value: bool) -> &'static str {
    if value {
        "PASS"
    } else {
        "FAIL"
    }
}

fn escape_markdown(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', "<br>")
}

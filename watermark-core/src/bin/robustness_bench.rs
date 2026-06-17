use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use image::codecs::jpeg::JpegEncoder;
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
struct CaseResult {
    media: &'static str,
    source: String,
    transform: String,
    success: bool,
    expected_uid: String,
    extracted_uid: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct BenchSummary {
    image_sources: usize,
    audio_sources: usize,
    passed: usize,
    total: usize,
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

    let image_sources = collect_images(config.image_dir.as_deref(), config.max_images)?;
    let audio_sources = collect_audio(config.audio_glob.as_deref(), config.max_audio)?;
    if image_sources.is_empty() && audio_sources.is_empty() {
        return Err("no image or audio sources found".into());
    }

    let run_dir = config.output_dir.join(format!("run-{}", unix_seconds()));
    fs::create_dir_all(&run_dir).map_err(|error| format!("create run dir: {error}"))?;

    let mut results = Vec::new();
    for (index, source) in image_sources.iter().enumerate() {
        results.extend(run_image_source(source, index, &run_dir)?);
    }
    for (index, source) in audio_sources.iter().enumerate() {
        results.extend(run_audio_source(source, index, &run_dir, &config.ffmpeg)?);
    }

    let summary = summarize(image_sources.len(), audio_sources.len(), &results);
    write_markdown_report(&run_dir.join("report.md"), &summary, &results)?;
    write_json_report(&run_dir.join("report.json"), &summary, &results)?;

    println!(
        "Robustness bench finished: {}/{} passed",
        summary.passed, summary.total
    );
    println!("Report: {}", display_path(&run_dir.join("report.md")));
    Ok(())
}

impl Config {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut image_dir = None;
        let mut audio_glob = None;
        let mut max_images = 3usize;
        let mut max_audio = 3usize;
        let mut output_dir = PathBuf::from("watermark-core/target/robustness-bench");
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
        "Usage: cargo run --manifest-path watermark-core/Cargo.toml --bin robustness_bench -- \\
  --image-dir <dir> --audio-glob <path/*.mp3> [--max-images 3] [--max-audio 3]"
    );
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

fn collect_images(image_dir: Option<&Path>, limit: usize) -> Result<Vec<PathBuf>, String> {
    let Some(image_dir) = image_dir else {
        return Ok(Vec::new());
    };
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

fn run_image_source(
    source: &Path,
    index: usize,
    run_dir: &Path,
) -> Result<Vec<CaseResult>, String> {
    let source_bytes = fs::read(source)
        .map_err(|error| format!("read image source '{}': {error}", source.display()))?;
    let payload = payload_for_source(&source_bytes, index as u64);
    let expected_uid = payload.watermark_uid();
    let output = WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: source_bytes.clone(),
        },
        &payload,
        EmbedOptions::default(),
    )
    .map_err(|error| format!("embed image '{}': {error}", source.display()))?;
    let MediaOutput::ImageBytes { bytes, .. } = output else {
        return Err("unexpected non-image output".into());
    };

    let source_name = display_name(source);
    let image_dir = run_dir.join("images").join(safe_stem(source, index));
    fs::create_dir_all(&image_dir).map_err(|error| format!("create image output dir: {error}"))?;
    fs::write(image_dir.join("embedded.png"), &bytes)
        .map_err(|error| format!("write embedded image: {error}"))?;

    let mut results = Vec::new();
    results.push(verify_image_case(
        &source_name,
        "baseline_png",
        &bytes,
        &expected_uid,
    ));

    let embedded_img = image::load_from_memory(&bytes)
        .map_err(|error| format!("reload embedded image '{}': {error}", source.display()))?;
    for variant in image_variants(&embedded_img)? {
        fs::write(
            image_dir.join(format!("{}.{}", variant.name, variant.extension)),
            &variant.bytes,
        )
        .map_err(|error| format!("write image variant: {error}"))?;
        results.push(verify_image_case(
            &source_name,
            variant.name,
            &variant.bytes,
            &expected_uid,
        ));
    }
    Ok(results)
}

struct ImageVariant {
    name: &'static str,
    extension: &'static str,
    bytes: Vec<u8>,
}

fn image_variants(img: &DynamicImage) -> Result<Vec<ImageVariant>, String> {
    let mut variants = Vec::new();
    variants.push(ImageVariant {
        name: "png_reencode",
        extension: "png",
        bytes: encode_image(img, ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "jpeg_q90",
        extension: "jpg",
        bytes: encode_image(img, ImageFormat::Jpeg, Some(90))?,
    });
    variants.push(ImageVariant {
        name: "jpeg_q75",
        extension: "jpg",
        bytes: encode_image(img, ImageFormat::Jpeg, Some(75))?,
    });

    let resized = img.resize(
        ((img.width() as f32) * 0.85).round().max(1.0) as u32,
        ((img.height() as f32) * 0.85).round().max(1.0) as u32,
        image::imageops::FilterType::Lanczos3,
    );
    variants.push(ImageVariant {
        name: "resize_85",
        extension: "png",
        bytes: encode_image(&resized, ImageFormat::Png, None)?,
    });

    let crop_x = (img.width() / 50).max(1);
    let crop_y = (img.height() / 50).max(1);
    if img.width() > crop_x * 2 && img.height() > crop_y * 2 {
        let cropped = img.crop_imm(
            crop_x,
            crop_y,
            img.width() - crop_x * 2,
            img.height() - crop_y * 2,
        );
        variants.push(ImageVariant {
            name: "crop_2_percent",
            extension: "png",
            bytes: encode_image(&cropped, ImageFormat::Png, None)?,
        });
    }

    Ok(variants)
}

fn encode_image(
    img: &DynamicImage,
    format: ImageFormat,
    jpeg_quality: Option<u8>,
) -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    if format == ImageFormat::Jpeg {
        let mut encoder = JpegEncoder::new_with_quality(&mut cursor, jpeg_quality.unwrap_or(90));
        encoder
            .encode_image(img)
            .map_err(|error| format!("encode jpeg: {error}"))?;
    } else {
        img.write_to(&mut cursor, format)
            .map_err(|error| format!("encode image: {error}"))?;
    }
    Ok(cursor.into_inner())
}

fn verify_image_case(
    source: &str,
    transform: &str,
    bytes: &[u8],
    expected_uid: &str,
) -> CaseResult {
    verify_case("image", source, transform, expected_uid, || {
        WatermarkService::extract(MediaInput::ImageBytes {
            bytes: bytes.to_vec(),
        })
        .map_err(|error| error.to_string())
    })
}

fn run_audio_source(
    source: &Path,
    index: usize,
    run_dir: &Path,
    ffmpeg: &str,
) -> Result<Vec<CaseResult>, String> {
    let audio_dir = run_dir.join("audio").join(safe_stem(source, index));
    fs::create_dir_all(&audio_dir).map_err(|error| format!("create audio output dir: {error}"))?;
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

    let source_bytes =
        fs::read(&source_wav).map_err(|error| format!("read converted wav: {error}"))?;
    let payload = payload_for_source(&source_bytes, 10_000 + index as u64);
    let expected_uid = payload.watermark_uid();
    let output = WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: source_bytes,
        },
        &payload,
        EmbedOptions::default(),
    )
    .map_err(|error| format!("embed audio '{}': {error}", source.display()))?;
    let MediaOutput::AudioWavBytes { bytes } = output else {
        return Err("unexpected non-audio output".into());
    };
    let embedded_wav = audio_dir.join("embedded.wav");
    fs::write(&embedded_wav, &bytes).map_err(|error| format!("write embedded wav: {error}"))?;

    let source_name = display_name(source);
    let mut results = Vec::new();
    results.push(verify_audio_file(
        &source_name,
        "baseline_wav",
        &embedded_wav,
        &expected_uid,
    ));

    for variant in audio_variants(&audio_dir, &embedded_wav, ffmpeg)? {
        results.push(verify_audio_file(
            &source_name,
            variant.name,
            &variant.output_wav,
            &expected_uid,
        ));
    }
    Ok(results)
}

struct AudioVariant {
    name: &'static str,
    output_wav: PathBuf,
}

fn audio_variants(
    audio_dir: &Path,
    embedded_wav: &Path,
    ffmpeg: &str,
) -> Result<Vec<AudioVariant>, String> {
    let mut variants = Vec::new();
    let specs = [
        (
            "wav_reencode",
            vec!["-ar", "44100", "-ac", "1", "-c:a", "pcm_s16le"],
        ),
        (
            "volume_80",
            vec![
                "-filter:a",
                "volume=0.8",
                "-ar",
                "44100",
                "-ac",
                "1",
                "-c:a",
                "pcm_s16le",
            ],
        ),
        (
            "volume_120",
            vec![
                "-filter:a",
                "volume=1.2",
                "-ar",
                "44100",
                "-ac",
                "1",
                "-c:a",
                "pcm_s16le",
            ],
        ),
        (
            "resample_22050",
            vec!["-ar", "22050", "-ac", "1", "-c:a", "pcm_s16le"],
        ),
    ];

    for (name, params) in specs {
        let output_wav = audio_dir.join(format!("{name}.wav"));
        let mut args = vec![
            "-y".to_string(),
            "-i".to_string(),
            embedded_wav.display().to_string(),
        ];
        args.extend(params.into_iter().map(String::from));
        args.push(output_wav.display().to_string());
        run_ffmpeg_owned(ffmpeg, &args)?;
        variants.push(AudioVariant { name, output_wav });
    }

    let mp3 = audio_dir.join("mp3_192.mp3");
    run_ffmpeg(
        ffmpeg,
        &[
            "-y",
            "-i",
            &embedded_wav.display().to_string(),
            "-codec:a",
            "libmp3lame",
            "-b:a",
            "192k",
            &mp3.display().to_string(),
        ],
    )?;
    let mp3_roundtrip_wav = audio_dir.join("mp3_192_roundtrip.wav");
    run_ffmpeg(
        ffmpeg,
        &[
            "-y",
            "-i",
            &mp3.display().to_string(),
            "-ar",
            "44100",
            "-ac",
            "1",
            "-c:a",
            "pcm_s16le",
            &mp3_roundtrip_wav.display().to_string(),
        ],
    )?;
    variants.push(AudioVariant {
        name: "mp3_192_roundtrip",
        output_wav: mp3_roundtrip_wav,
    });

    Ok(variants)
}

fn verify_audio_file(
    source: &str,
    transform: &str,
    wav_path: &Path,
    expected_uid: &str,
) -> CaseResult {
    match fs::read(wav_path) {
        Ok(bytes) => verify_case("audio", source, transform, expected_uid, || {
            WatermarkService::extract(MediaInput::AudioWavBytes { bytes })
                .map_err(|error| error.to_string())
        }),
        Err(error) => CaseResult {
            media: "audio",
            source: source.to_string(),
            transform: transform.to_string(),
            success: false,
            expected_uid: expected_uid.to_string(),
            extracted_uid: None,
            error: Some(format!("read transformed wav: {error}")),
        },
    }
}

fn verify_case<F>(
    media: &'static str,
    source: &str,
    transform: &str,
    expected_uid: &str,
    extract: F,
) -> CaseResult
where
    F: FnOnce() -> Result<WatermarkPayload, String>,
{
    match extract() {
        Ok(payload) => {
            let extracted_uid = payload.watermark_uid();
            let success = extracted_uid == expected_uid;
            CaseResult {
                media,
                source: source.to_string(),
                transform: transform.to_string(),
                success,
                expected_uid: expected_uid.to_string(),
                extracted_uid: Some(extracted_uid),
                error: None,
            }
        }
        Err(error) => CaseResult {
            media,
            source: source.to_string(),
            transform: transform.to_string(),
            success: false,
            expected_uid: expected_uid.to_string(),
            extracted_uid: None,
            error: Some(error),
        },
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

fn run_ffmpeg(ffmpeg: &str, args: &[&str]) -> Result<(), String> {
    let owned = args.iter().map(|arg| arg.to_string()).collect::<Vec<_>>();
    run_ffmpeg_owned(ffmpeg, &owned)
}

fn run_ffmpeg_owned(ffmpeg: &str, args: &[String]) -> Result<(), String> {
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

fn summarize(image_sources: usize, audio_sources: usize, results: &[CaseResult]) -> BenchSummary {
    BenchSummary {
        image_sources,
        audio_sources,
        passed: results.iter().filter(|result| result.success).count(),
        total: results.len(),
    }
}

fn write_markdown_report(
    path: &Path,
    summary: &BenchSummary,
    results: &[CaseResult],
) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("# Watermark Robustness Bench\n\n");
    out.push_str(&format!(
        "- Image sources: {}\n- Audio sources: {}\n- Passed: {}/{}\n\n",
        summary.image_sources, summary.audio_sources, summary.passed, summary.total
    ));
    out.push_str("| Media | Source | Transform | Result | Extracted UID | Error |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- |\n");
    for result in results {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            result.media,
            escape_md(&result.source),
            result.transform,
            if result.success { "PASS" } else { "FAIL" },
            result.extracted_uid.as_deref().unwrap_or(""),
            escape_md(result.error.as_deref().unwrap_or(""))
        ));
    }
    fs::write(path, out).map_err(|error| format!("write markdown report: {error}"))
}

fn write_json_report(
    path: &Path,
    summary: &BenchSummary,
    results: &[CaseResult],
) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("{\n");
    out.push_str(&format!(
        "  \"summary\": {{ \"imageSources\": {}, \"audioSources\": {}, \"passed\": {}, \"total\": {} }},\n",
        summary.image_sources, summary.audio_sources, summary.passed, summary.total
    ));
    out.push_str("  \"results\": [\n");
    for (index, result) in results.iter().enumerate() {
        out.push_str(&format!(
            "    {{ \"media\": \"{}\", \"source\": \"{}\", \"transform\": \"{}\", \"success\": {}, \"expectedUid\": \"{}\", \"extractedUid\": {}, \"error\": {} }}{}\n",
            json_escape(result.media),
            json_escape(&result.source),
            json_escape(&result.transform),
            result.success,
            json_escape(&result.expected_uid),
            json_option(result.extracted_uid.as_deref()),
            json_option(result.error.as_deref()),
            if index + 1 == results.len() { "" } else { "," }
        ));
    }
    out.push_str("  ]\n}\n");
    fs::write(path, out).map_err(|error| format!("write json report: {error}"))
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

fn safe_stem(path: &Path, index: usize) -> String {
    let stem = path
        .file_stem()
        .and_then(OsStr::to_str)
        .unwrap_or("source")
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("{index:02}_{stem}")
}

fn escape_md(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn json_option(value: Option<&str>) -> String {
    value
        .map(|value| format!("\"{}\"", json_escape(value)))
        .unwrap_or_else(|| "null".to_string())
}

fn json_escape(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '"' => "\\\"".chars().collect::<Vec<_>>(),
            '\\' => "\\\\".chars().collect::<Vec<_>>(),
            '\n' => "\\n".chars().collect::<Vec<_>>(),
            '\r' => "\\r".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

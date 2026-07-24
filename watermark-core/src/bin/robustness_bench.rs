use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use image::codecs::jpeg::JpegEncoder;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
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
    audio_matrix: bool,
}

#[derive(Debug, Clone)]
struct CaseResult {
    media: &'static str,
    source: String,
    transform: String,
    success: bool,
    write_ms: Option<u128>,
    read_ms: Option<u128>,
    source_duration_secs: Option<f64>,
    source_bit_rate: Option<u64>,
    source_sample_rate: Option<u32>,
    source_channels: Option<u16>,
    output_sample_rate: Option<u32>,
    output_channels: Option<u16>,
    spec_preserved: Option<bool>,
    source_mean_volume_db: Option<f32>,
    source_max_volume_db: Option<f32>,
    expected_uid: String,
    extracted_uid: Option<String>,
    error: Option<String>,
}

#[derive(Debug, Clone, Default)]
struct AudioSourceMeta {
    duration_secs: Option<f64>,
    bit_rate: Option<u64>,
    sample_rate: Option<u32>,
    channels: Option<u16>,
    mean_volume_db: Option<f32>,
    max_volume_db: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct AudioSpec {
    sample_rate: u32,
    channels: u16,
}

impl AudioSpec {
    fn from_source_meta(meta: &AudioSourceMeta) -> Option<Self> {
        Some(Self {
            sample_rate: meta.sample_rate?,
            channels: meta.channels?,
        })
    }
}

#[derive(Debug, Clone)]
struct BenchSummary {
    image_sources: usize,
    audio_sources: usize,
    audio_matrix: bool,
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
        results.extend(run_audio_source(
            source,
            index,
            &run_dir,
            &config.ffmpeg,
            config.audio_matrix,
        )?);
    }

    let summary = summarize(
        image_sources.len(),
        audio_sources.len(),
        config.audio_matrix,
        &results,
    );
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
        let mut audio_matrix = false;

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
                "--audio-matrix" => {
                    audio_matrix = true;
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
            audio_matrix,
        })
    }
}

fn print_usage() {
    println!(
        "Usage: cargo run --manifest-path watermark-core/Cargo.toml --bin robustness_bench -- \\
  --image-dir <dir> --audio-glob <path/*.mp3> [--max-images 3] [--max-audio 3] [--audio-matrix]"
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
    let started = Instant::now();
    let output = WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: source_bytes.clone(),
        },
        &payload,
        EmbedOptions::default(),
    )
    .map_err(|error| format!("embed image '{}': {error}", source.display()))?;
    let write_ms = Some(started.elapsed().as_millis());
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
        write_ms,
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
            write_ms,
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
    variants.push(ImageVariant {
        name: "brightness_90",
        extension: "png",
        bytes: encode_image(&adjust_brightness(img, 0.9), ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "brightness_110",
        extension: "png",
        bytes: encode_image(&adjust_brightness(img, 1.1), ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "pepper_noise_1_percent",
        extension: "png",
        bytes: encode_image(&pepper_noise(img, 0.01), ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "mask_center_15_percent",
        extension: "png",
        bytes: encode_image(&mask_center(img, 0.15), ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "vertical_cut_15_percent",
        extension: "png",
        bytes: encode_image(&vertical_cut(img, 0.15), ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "horizontal_cut_15_percent",
        extension: "png",
        bytes: encode_image(&horizontal_cut(img, 0.15), ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "rotate_90",
        extension: "png",
        bytes: encode_image(&img.rotate90(), ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "rotate_180",
        extension: "png",
        bytes: encode_image(&img.rotate180(), ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "rotate_270",
        extension: "png",
        bytes: encode_image(&img.rotate270(), ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "mirror_horizontal",
        extension: "png",
        bytes: encode_image(&img.fliph(), ImageFormat::Png, None)?,
    });
    variants.push(ImageVariant {
        name: "mirror_vertical",
        extension: "png",
        bytes: encode_image(&img.flipv(), ImageFormat::Png, None)?,
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

fn adjust_brightness(img: &DynamicImage, factor: f32) -> DynamicImage {
    let mut out = img.to_rgba8();
    for pixel in out.pixels_mut() {
        for channel in 0..3 {
            pixel[channel] = ((pixel[channel] as f32 * factor).round()).clamp(0.0, 255.0) as u8;
        }
    }
    DynamicImage::ImageRgba8(out)
}

fn pepper_noise(img: &DynamicImage, ratio: f32) -> DynamicImage {
    let mut out = img.to_rgba8();
    let threshold = (ratio.clamp(0.0, 1.0) * 10_000.0).round() as u32;
    for (x, y, pixel) in out.enumerate_pixels_mut() {
        let value = deterministic_noise_value(x, y);
        if value < threshold {
            let color = if value % 2 == 0 { 0 } else { 255 };
            *pixel = Rgba([color, color, color, pixel[3]]);
        }
    }
    DynamicImage::ImageRgba8(out)
}

fn deterministic_noise_value(x: u32, y: u32) -> u32 {
    let mut state = x.wrapping_mul(0x45d9f3b) ^ y.wrapping_mul(0x119de1f3);
    state ^= state >> 16;
    state = state.wrapping_mul(0x45d9f3b);
    state ^= state >> 16;
    state % 10_000
}

fn mask_center(img: &DynamicImage, ratio: f32) -> DynamicImage {
    let mut out = img.to_rgba8();
    let (w, h) = out.dimensions();
    let mask_w = ((w as f32 * ratio).round() as u32).clamp(1, w);
    let mask_h = ((h as f32 * ratio).round() as u32).clamp(1, h);
    fill_rect(
        &mut out,
        (w - mask_w) / 2,
        (h - mask_h) / 2,
        mask_w,
        mask_h,
        Rgba([0, 0, 0, 255]),
    );
    DynamicImage::ImageRgba8(out)
}

fn vertical_cut(img: &DynamicImage, ratio: f32) -> DynamicImage {
    let mut out = img.to_rgba8();
    let (w, h) = out.dimensions();
    let cut_w = ((w as f32 * ratio).round() as u32).clamp(1, w);
    fill_rect(&mut out, (w - cut_w) / 2, 0, cut_w, h, Rgba([0, 0, 0, 255]));
    DynamicImage::ImageRgba8(out)
}

fn horizontal_cut(img: &DynamicImage, ratio: f32) -> DynamicImage {
    let mut out = img.to_rgba8();
    let (w, h) = out.dimensions();
    let cut_h = ((h as f32 * ratio).round() as u32).clamp(1, h);
    fill_rect(&mut out, 0, (h - cut_h) / 2, w, cut_h, Rgba([0, 0, 0, 255]));
    DynamicImage::ImageRgba8(out)
}

fn fill_rect(img: &mut RgbaImage, x: u32, y: u32, width: u32, height: u32, color: Rgba<u8>) {
    let max_y = (y + height).min(img.height());
    let max_x = (x + width).min(img.width());
    for py in y..max_y {
        for px in x..max_x {
            img.put_pixel(px, py, color);
        }
    }
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
    write_ms: Option<u128>,
) -> CaseResult {
    verify_case(
        "image",
        source,
        transform,
        expected_uid,
        || {
            WatermarkService::extract(MediaInput::ImageBytes {
                bytes: bytes.to_vec(),
            })
            .map(|payload| payload.watermark_uid())
            .map_err(|error| error.to_string())
        },
        write_ms,
        None,
        None,
    )
}

fn run_audio_source(
    source: &Path,
    index: usize,
    run_dir: &Path,
    ffmpeg: &str,
    audio_matrix: bool,
) -> Result<Vec<CaseResult>, String> {
    let audio_dir = run_dir.join("audio").join(safe_stem(source, index));
    fs::create_dir_all(&audio_dir).map_err(|error| format!("create audio output dir: {error}"))?;
    let source_meta = probe_audio_source(source, ffmpeg);
    let source_spec = AudioSpec::from_source_meta(&source_meta).ok_or_else(|| {
        format!(
            "source audio sample rate or channel count unavailable: '{}'",
            source.display()
        )
    })?;
    let source_wav = audio_dir.join("source.wav");
    let mut source_args = vec![
        "-y".to_string(),
        "-i".to_string(),
        source.display().to_string(),
        "-vn".to_string(),
        "-c:a".to_string(),
        "pcm_s16le".to_string(),
    ];
    append_audio_spec_args(&mut source_args, source_spec);
    source_args.push(source_wav.display().to_string());
    run_ffmpeg_owned(ffmpeg, &source_args)?;
    assert_wav_spec(&source_wav, source_spec, "source_decode")?;

    let source_bytes =
        fs::read(&source_wav).map_err(|error| format!("read converted wav: {error}"))?;
    let payload = payload_for_source(&source_bytes, 10_000 + index as u64);
    let expected_uid = payload.watermark_uid();
    let started = Instant::now();
    let output = WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: source_bytes,
        },
        &payload,
        EmbedOptions::default(),
    )
    .map_err(|error| format!("embed audio '{}': {error}", source.display()))?;
    let write_ms = Some(started.elapsed().as_millis());
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
        write_ms,
        &source_meta,
        source_spec,
    ));

    for variant in audio_variants(&audio_dir, &embedded_wav, ffmpeg, audio_matrix, source_spec)? {
        results.push(verify_audio_file(
            &source_name,
            &variant.name,
            &variant.output_wav,
            &expected_uid,
            write_ms,
            &source_meta,
            source_spec,
        ));
    }
    Ok(results)
}

struct AudioVariant {
    name: String,
    output_wav: PathBuf,
}

fn audio_variants(
    audio_dir: &Path,
    embedded_wav: &Path,
    ffmpeg: &str,
    audio_matrix: bool,
    source_spec: AudioSpec,
) -> Result<Vec<AudioVariant>, String> {
    let mut variants = Vec::new();
    let specs = [
        ("wav_reencode", vec!["-c:a", "pcm_s16le"]),
        (
            "volume_80",
            vec!["-filter:a", "volume=0.8", "-c:a", "pcm_s16le"],
        ),
        (
            "volume_120",
            vec!["-filter:a", "volume=1.2", "-c:a", "pcm_s16le"],
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
        append_audio_spec_args(&mut args, source_spec);
        args.push(output_wav.display().to_string());
        run_ffmpeg_owned(ffmpeg, &args)?;
        assert_wav_spec(&output_wav, source_spec, name)?;
        variants.push(AudioVariant {
            name: name.to_string(),
            output_wav,
        });
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
    assert_container_spec(&mp3, ffmpeg, source_spec, "mp3_192")?;
    let mp3_roundtrip_wav = audio_dir.join("mp3_192_roundtrip.wav");
    let mut mp3_roundtrip_args = vec![
        "-y".to_string(),
        "-i".to_string(),
        mp3.display().to_string(),
        "-c:a".to_string(),
        "pcm_s16le".to_string(),
    ];
    append_audio_spec_args(&mut mp3_roundtrip_args, source_spec);
    mp3_roundtrip_args.push(mp3_roundtrip_wav.display().to_string());
    run_ffmpeg_owned(ffmpeg, &mp3_roundtrip_args)?;
    assert_wav_spec(&mp3_roundtrip_wav, source_spec, "mp3_192_roundtrip")?;
    variants.push(AudioVariant {
        name: "mp3_192_roundtrip".to_string(),
        output_wav: mp3_roundtrip_wav.clone(),
    });

    let clip_10s_middle = audio_dir.join("clip_10s_middle.wav");
    let mut clip_args = vec![
        "-y".to_string(),
        "-i".to_string(),
        embedded_wav.display().to_string(),
        "-ss".to_string(),
        "10".to_string(),
        "-t".to_string(),
        "10".to_string(),
        "-c:a".to_string(),
        "pcm_s16le".to_string(),
    ];
    append_audio_spec_args(&mut clip_args, source_spec);
    clip_args.push(clip_10s_middle.display().to_string());
    run_ffmpeg_owned(ffmpeg, &clip_args)?;
    assert_wav_spec(&clip_10s_middle, source_spec, "clip_10s_middle")?;
    variants.push(AudioVariant {
        name: "clip_10s_middle".to_string(),
        output_wav: clip_10s_middle,
    });

    if audio_matrix {
        append_audio_clip_matrix(
            &mut variants,
            audio_dir,
            embedded_wav,
            "wav",
            ffmpeg,
            source_spec,
        )?;
        append_audio_clip_matrix(
            &mut variants,
            audio_dir,
            &mp3_roundtrip_wav,
            "mp3_192",
            ffmpeg,
            source_spec,
        )?;
    }

    Ok(variants)
}

fn append_audio_clip_matrix(
    variants: &mut Vec<AudioVariant>,
    audio_dir: &Path,
    input_wav: &Path,
    source_label: &str,
    ffmpeg: &str,
    source_spec: AudioSpec,
) -> Result<(), String> {
    for duration in [5u32, 10, 15] {
        for position in ["start", "middle", "end"] {
            let output_wav =
                audio_dir.join(format!("matrix_{source_label}_{duration}s_{position}.wav"));
            let start = clip_start_for_position(position, duration);
            let mut args = vec![
                "-y".to_string(),
                "-i".to_string(),
                input_wav.display().to_string(),
                "-ss".to_string(),
                start.to_string(),
                "-t".to_string(),
                duration.to_string(),
                "-c:a".to_string(),
                "pcm_s16le".to_string(),
            ];
            append_audio_spec_args(&mut args, source_spec);
            args.push(output_wav.display().to_string());
            run_ffmpeg_owned(ffmpeg, &args)?;
            assert_wav_spec(
                &output_wav,
                source_spec,
                &format!("matrix_{source_label}_{duration}s_{position}"),
            )?;
            variants.push(AudioVariant {
                name: format!("matrix_{source_label}_{duration}s_{position}"),
                output_wav,
            });
        }
    }
    Ok(())
}

fn clip_start_for_position(position: &str, duration: u32) -> u32 {
    match position {
        "start" => 0,
        "middle" => (30 - duration) / 2,
        "end" => 30 - duration,
        _ => 0,
    }
}

fn append_audio_spec_args(args: &mut Vec<String>, spec: AudioSpec) {
    args.extend([
        "-ar".to_string(),
        spec.sample_rate.to_string(),
        "-ac".to_string(),
        spec.channels.to_string(),
    ]);
}

fn wav_spec_from_bytes(bytes: &[u8]) -> Option<AudioSpec> {
    let reader = hound::WavReader::new(Cursor::new(bytes)).ok()?;
    let spec = reader.spec();
    Some(AudioSpec {
        sample_rate: spec.sample_rate,
        channels: spec.channels,
    })
}

fn wav_spec_from_file(path: &Path) -> Result<AudioSpec, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("read WAV spec '{}': {error}", path.display()))?;
    wav_spec_from_bytes(&bytes)
        .ok_or_else(|| format!("read WAV header spec '{}': invalid WAV", path.display()))
}

fn assert_wav_spec(path: &Path, expected: AudioSpec, transform: &str) -> Result<(), String> {
    let actual = wav_spec_from_file(path)?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "audio spec changed during {transform}: expected {} Hz / {} ch, got {} Hz / {} ch",
            expected.sample_rate, expected.channels, actual.sample_rate, actual.channels
        ))
    }
}

fn assert_container_spec(
    path: &Path,
    ffmpeg: &str,
    expected: AudioSpec,
    transform: &str,
) -> Result<(), String> {
    let meta = probe_audio_source(path, ffmpeg);
    let actual = AudioSpec::from_source_meta(&meta)
        .ok_or_else(|| format!("read audio spec during {transform}: unavailable"))?;
    if actual == expected {
        Ok(())
    } else {
        Err(format!(
            "audio spec changed during {transform}: expected {} Hz / {} ch, got {} Hz / {} ch",
            expected.sample_rate, expected.channels, actual.sample_rate, actual.channels
        ))
    }
}

fn verify_audio_file(
    source: &str,
    transform: &str,
    wav_path: &Path,
    expected_uid: &str,
    write_ms: Option<u128>,
    source_meta: &AudioSourceMeta,
    expected_spec: AudioSpec,
) -> CaseResult {
    match fs::read(wav_path) {
        Ok(bytes) => {
            let actual_spec = wav_spec_from_bytes(&bytes);
            let spec_preserved = actual_spec == Some(expected_spec);
            if !spec_preserved {
                return audio_spec_failure(
                    source,
                    transform,
                    expected_uid,
                    write_ms,
                    source_meta,
                    actual_spec,
                    expected_spec,
                );
            }
            verify_case(
                "audio",
                source,
                transform,
                expected_uid,
                || {
                    WatermarkService::extract(MediaInput::AudioWavBytes { bytes })
                        .map(|payload| payload.watermark_uid())
                        .map_err(|error| error.to_string())
                },
                write_ms,
                Some(source_meta),
                actual_spec,
            )
        }
        Err(error) => CaseResult {
            media: "audio",
            source: source.to_string(),
            transform: transform.to_string(),
            success: false,
            write_ms,
            read_ms: None,
            source_duration_secs: source_meta.duration_secs,
            source_bit_rate: source_meta.bit_rate,
            source_sample_rate: source_meta.sample_rate,
            source_channels: source_meta.channels,
            output_sample_rate: None,
            output_channels: None,
            spec_preserved: Some(false),
            source_mean_volume_db: source_meta.mean_volume_db,
            source_max_volume_db: source_meta.max_volume_db,
            expected_uid: expected_uid.to_string(),
            extracted_uid: None,
            error: Some(format!("read transformed wav: {error}")),
        },
    }
}

fn audio_spec_failure(
    source: &str,
    transform: &str,
    expected_uid: &str,
    write_ms: Option<u128>,
    source_meta: &AudioSourceMeta,
    actual_spec: Option<AudioSpec>,
    expected_spec: AudioSpec,
) -> CaseResult {
    CaseResult {
        media: "audio",
        source: source.to_string(),
        transform: transform.to_string(),
        success: false,
        write_ms,
        read_ms: None,
        source_duration_secs: source_meta.duration_secs,
        source_bit_rate: source_meta.bit_rate,
        source_sample_rate: source_meta.sample_rate,
        source_channels: source_meta.channels,
        output_sample_rate: actual_spec.map(|spec| spec.sample_rate),
        output_channels: actual_spec.map(|spec| spec.channels),
        spec_preserved: Some(false),
        source_mean_volume_db: source_meta.mean_volume_db,
        source_max_volume_db: source_meta.max_volume_db,
        expected_uid: expected_uid.to_string(),
        extracted_uid: None,
        error: Some(match actual_spec {
            Some(actual) => format!(
                "audio spec changed: expected {} Hz / {} ch, got {} Hz / {} ch",
                expected_spec.sample_rate,
                expected_spec.channels,
                actual.sample_rate,
                actual.channels
            ),
            None => "transformed output is not a readable WAV".to_string(),
        }),
    }
}

fn verify_case<F>(
    media: &'static str,
    source: &str,
    transform: &str,
    expected_uid: &str,
    extract: F,
    write_ms: Option<u128>,
    source_meta: Option<&AudioSourceMeta>,
    output_spec: Option<AudioSpec>,
) -> CaseResult
where
    F: FnOnce() -> Result<String, String>,
{
    let started = Instant::now();
    match extract() {
        Ok(extracted_uid) => {
            let success = extracted_uid == expected_uid;
            let read_ms = Some(started.elapsed().as_millis());
            CaseResult {
                media,
                source: source.to_string(),
                transform: transform.to_string(),
                success,
                write_ms,
                read_ms,
                source_duration_secs: source_meta.and_then(|meta| meta.duration_secs),
                source_bit_rate: source_meta.and_then(|meta| meta.bit_rate),
                source_sample_rate: source_meta.and_then(|meta| meta.sample_rate),
                source_channels: source_meta.and_then(|meta| meta.channels),
                output_sample_rate: output_spec.map(|spec| spec.sample_rate),
                output_channels: output_spec.map(|spec| spec.channels),
                spec_preserved: output_spec.map(|_| true),
                source_mean_volume_db: source_meta.and_then(|meta| meta.mean_volume_db),
                source_max_volume_db: source_meta.and_then(|meta| meta.max_volume_db),
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
            write_ms,
            read_ms: Some(started.elapsed().as_millis()),
            source_duration_secs: source_meta.and_then(|meta| meta.duration_secs),
            source_bit_rate: source_meta.and_then(|meta| meta.bit_rate),
            source_sample_rate: source_meta.and_then(|meta| meta.sample_rate),
            source_channels: source_meta.and_then(|meta| meta.channels),
            output_sample_rate: output_spec.map(|spec| spec.sample_rate),
            output_channels: output_spec.map(|spec| spec.channels),
            spec_preserved: output_spec.map(|_| true),
            source_mean_volume_db: source_meta.and_then(|meta| meta.mean_volume_db),
            source_max_volume_db: source_meta.and_then(|meta| meta.max_volume_db),
            expected_uid: expected_uid.to_string(),
            extracted_uid: None,
            error: Some(error),
        },
    }
}

fn probe_audio_source(source: &Path, ffmpeg: &str) -> AudioSourceMeta {
    let mut meta = AudioSourceMeta::default();
    if let Ok(output) = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "a:0",
            "-show_entries",
            "format=duration,bit_rate:stream=sample_rate,channels,bit_rate",
            "-of",
            "default=noprint_wrappers=1:nokey=0",
            &source.display().to_string(),
        ])
        .output()
    {
        if output.status.success() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if let Some((key, value)) = line.split_once('=') {
                    match key.trim() {
                        "duration" => meta.duration_secs = value.trim().parse::<f64>().ok(),
                        "bit_rate" => meta.bit_rate = value.trim().parse::<u64>().ok(),
                        "sample_rate" => meta.sample_rate = value.trim().parse::<u32>().ok(),
                        "channels" => meta.channels = value.trim().parse::<u16>().ok(),
                        _ => {}
                    }
                }
            }
        }
    }

    if let Ok(output) = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-nostats",
            "-i",
            &source.display().to_string(),
            "-af",
            "volumedetect",
            "-f",
            "null",
            "-",
        ])
        .output()
    {
        let text = String::from_utf8_lossy(&output.stderr);
        meta.mean_volume_db = parse_ffmpeg_volume(&text, "mean_volume");
        meta.max_volume_db = parse_ffmpeg_volume(&text, "max_volume");
    }

    meta
}

fn parse_ffmpeg_volume(text: &str, label: &str) -> Option<f32> {
    text.lines().find_map(|line| {
        let line = line.trim();
        let needle = format!("{label}:");
        let Some(index) = line.find(&needle) else {
            return None;
        };
        line[index + needle.len()..]
            .trim()
            .split_whitespace()
            .next()
            .and_then(|value| value.parse::<f32>().ok())
    })
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

fn summarize(
    image_sources: usize,
    audio_sources: usize,
    audio_matrix: bool,
    results: &[CaseResult],
) -> BenchSummary {
    BenchSummary {
        image_sources,
        audio_sources,
        audio_matrix,
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
        "- Image sources: {}\n- Audio sources: {}\n- Audio matrix: {}\n- Passed: {}/{}\n\n",
        summary.image_sources,
        summary.audio_sources,
        if summary.audio_matrix {
            "enabled"
        } else {
            "disabled"
        },
        summary.passed,
        summary.total
    ));
    out.push_str("| Media | Source | Transform | Write ms | Read ms | Duration s | Bitrate | Source SR | Source Ch | Output SR | Output Ch | Spec | Mean dB | Max dB | Result | Extracted UID | Error |\n");
    out.push_str(
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n",
    );
    for result in results {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            result.media,
            escape_md(&result.source),
            result.transform,
            format_ms(result.write_ms),
            format_ms(result.read_ms),
            format_opt_f64(result.source_duration_secs),
            format_opt_u64(result.source_bit_rate),
            format_opt_u32(result.source_sample_rate),
            format_opt_u16(result.source_channels),
            format_opt_u32(result.output_sample_rate),
            format_opt_u16(result.output_channels),
            format_opt_bool(result.spec_preserved),
            format_opt_f32(result.source_mean_volume_db),
            format_opt_f32(result.source_max_volume_db),
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
        "  \"summary\": {{ \"imageSources\": {}, \"audioSources\": {}, \"audioMatrix\": {}, \"passed\": {}, \"total\": {} }},\n",
        summary.image_sources,
        summary.audio_sources,
        summary.audio_matrix,
        summary.passed,
        summary.total
    ));
    out.push_str("  \"results\": [\n");
    for (index, result) in results.iter().enumerate() {
        out.push_str(&format!(
            "    {{ \"media\": \"{}\", \"source\": \"{}\", \"transform\": \"{}\", \"success\": {}, \"writeMs\": {}, \"readMs\": {}, \"sourceDurationSecs\": {}, \"sourceBitRate\": {}, \"sourceSampleRate\": {}, \"sourceChannels\": {}, \"outputSampleRate\": {}, \"outputChannels\": {}, \"specPreserved\": {}, \"sourceMeanVolumeDb\": {}, \"sourceMaxVolumeDb\": {}, \"expectedUid\": \"{}\", \"extractedUid\": {}, \"error\": {} }}{}\n",
            json_escape(result.media),
            json_escape(&result.source),
            json_escape(&result.transform),
            result.success,
            json_option_u128(result.write_ms),
            json_option_u128(result.read_ms),
            json_option_f64(result.source_duration_secs),
            json_option_u64(result.source_bit_rate),
            json_option_u32(result.source_sample_rate),
            json_option_u16(result.source_channels),
            json_option_u32(result.output_sample_rate),
            json_option_u16(result.output_channels),
            json_option_bool(result.spec_preserved),
            json_option_f32(result.source_mean_volume_db),
            json_option_f32(result.source_max_volume_db),
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

fn json_option_u128(value: Option<u128>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_option_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_option_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_option_u16(value: Option<u16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_option_f64(value: Option<f64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_option_f32(value: Option<f32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn json_option_bool(value: Option<bool>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_string())
}

fn format_ms(value: Option<u128>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_opt_f64(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "-".to_string())
}

fn format_opt_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_opt_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_opt_u16(value: Option<u16>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}

fn format_opt_bool(value: Option<bool>) -> String {
    value
        .map(|value| if value { "preserved" } else { "changed" }.to_string())
        .unwrap_or_else(|| "-".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_spec_args_preserve_sample_rate_and_channels() {
        let mut args = Vec::new();
        append_audio_spec_args(
            &mut args,
            AudioSpec {
                sample_rate: 48_000,
                channels: 2,
            },
        );
        assert_eq!(args, vec!["-ar", "48000", "-ac", "2"]);
    }

    #[test]
    fn wav_spec_reader_reports_original_spec() {
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(
            &mut cursor,
            hound::WavSpec {
                channels: 2,
                sample_rate: 48_000,
                bits_per_sample: 16,
                sample_format: hound::SampleFormat::Int,
            },
        )
        .unwrap();
        writer.write_sample(0i16).unwrap();
        writer.write_sample(0i16).unwrap();
        writer.finalize().unwrap();

        assert_eq!(
            wav_spec_from_bytes(&cursor.into_inner()),
            Some(AudioSpec {
                sample_rate: 48_000,
                channels: 2,
            })
        );
    }
}
fn format_opt_f32(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "-".to_string())
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

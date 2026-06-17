use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use image::GenericImageView;
use realfft::RealFftPlanner;

const IMAGE_GRID: u32 = 4;
const AUDIO_SLICES: usize = 16;
const FRAME_SIZE: usize = 4096;
const CANONICAL_SAMPLE_RATE: u32 = 44_100;
const BAND_LO_BIN: usize = 186;
const BAND_HI_BIN: usize = 743;
const RELATIVE_PAIR_WIDTH: usize = 4;
const SILENCE_THRESHOLD: f32 = 0.001;

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
struct ProbeRow {
    media: &'static str,
    source: String,
    operation: &'static str,
    duration_ms: u128,
    slices: usize,
    active_slices: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let config = Config::from_args(env::args().skip(1).collect())?;
    let run_dir = config
        .output_dir
        .join(format!("slice-marker-{}", unix_seconds()));
    fs::create_dir_all(&run_dir).map_err(|error| format!("create probe dir: {error}"))?;

    let mut rows = Vec::new();
    for source in collect_images(config.image_dir.as_deref(), config.max_images)? {
        rows.push(probe_image(&source)?);
    }
    for (index, source) in collect_audio(config.audio_glob.as_deref(), config.max_audio)?
        .iter()
        .enumerate()
    {
        rows.push(probe_audio(source, index, &run_dir, &config.ffmpeg)?);
    }

    if rows.is_empty() {
        return Err("no image or audio sources found".into());
    }

    write_report(&run_dir.join("slice-marker-probe.md"), &rows)?;
    println!("Slice marker probe finished: {} measurements", rows.len());
    println!(
        "Report: {}",
        display_path(&run_dir.join("slice-marker-probe.md"))
    );
    print_summary(&rows);
    Ok(())
}

impl Config {
    fn from_args(args: Vec<String>) -> Result<Self, String> {
        let mut image_dir = None;
        let mut audio_glob = None;
        let mut max_images = 3usize;
        let mut max_audio = 1usize;
        let mut output_dir = PathBuf::from("watermark-core/target/slice-marker-probe");
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
        "Usage: cargo run --manifest-path watermark-core/Cargo.toml --bin slice_marker_probe -- \
  --image-dir <dir> [--audio-glob <path/*.mp3>] [--max-images 3] [--max-audio 1]"
    );
}

fn probe_image(source: &Path) -> Result<ProbeRow, String> {
    let bytes = fs::read(source)
        .map_err(|error| format!("read image source '{}': {error}", source.display()))?;
    let image = image::load_from_memory(&bytes)
        .map_err(|error| format!("open image '{}': {error}", source.display()))?;
    let rgb = image.to_rgb8();
    let (width, height) = image.dimensions();

    let started = Instant::now();
    let mut active_slices = 0usize;
    for tile_y in 0..IMAGE_GRID {
        for tile_x in 0..IMAGE_GRID {
            let x0 = width * tile_x / IMAGE_GRID;
            let x1 = width * (tile_x + 1) / IMAGE_GRID;
            let y0 = height * tile_y / IMAGE_GRID;
            let y1 = height * (tile_y + 1) / IMAGE_GRID;
            let score = image_tile_activity_score(&rgb, x0, y0, x1, y1);
            if score > 10.0 {
                active_slices += 1;
            }
        }
    }

    Ok(ProbeRow {
        media: "image",
        source: display_name(source),
        operation: "scan_4x4_tile_markers",
        duration_ms: started.elapsed().as_millis(),
        slices: (IMAGE_GRID * IMAGE_GRID) as usize,
        active_slices,
    })
}

fn image_tile_activity_score(image: &image::RgbImage, x0: u32, y0: u32, x1: u32, y1: u32) -> f64 {
    if x1 <= x0 + 1 || y1 <= y0 + 1 {
        return 0.0;
    }

    let step_x = ((x1 - x0) / 32).max(1);
    let step_y = ((y1 - y0) / 32).max(1);
    let mut total = 0.0;
    let mut count = 0usize;
    let mut y = y0 + step_y;
    while y < y1 {
        let mut x = x0 + step_x;
        while x < x1 {
            let current = luma(image.get_pixel(x, y));
            let left = luma(image.get_pixel(x - step_x, y));
            let up = luma(image.get_pixel(x, y - step_y));
            total += (current - left).abs() + (current - up).abs();
            count += 2;
            x += step_x;
        }
        y += step_y;
    }

    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

fn luma(pixel: &image::Rgb<u8>) -> f64 {
    0.299 * pixel[0] as f64 + 0.587 * pixel[1] as f64 + 0.114 * pixel[2] as f64
}

fn probe_audio(
    source: &Path,
    index: usize,
    run_dir: &Path,
    ffmpeg: &str,
) -> Result<ProbeRow, String> {
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

    let mut reader = hound::WavReader::open(&source_wav)
        .map_err(|error| format!("open wav '{}': {error}", source_wav.display()))?;
    let samples = read_wav_samples(&mut reader)?;
    let started = Instant::now();
    let active_slices = scan_audio_time_slices(&samples, CANONICAL_SAMPLE_RATE)?;

    Ok(ProbeRow {
        media: "audio",
        source: display_name(source),
        operation: "scan_16_time_markers",
        duration_ms: started.elapsed().as_millis(),
        slices: AUDIO_SLICES,
        active_slices,
    })
}

fn scan_audio_time_slices(samples: &[f32], sample_rate: u32) -> Result<usize, String> {
    if samples.len() < FRAME_SIZE {
        return Ok(0);
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let num_frames = samples.len() / FRAME_SIZE;
    let frames_per_slice = num_frames.div_ceil(AUDIO_SLICES).max(1);
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    let marker_pairs = usable_pairs.min(64);
    let mut active = vec![false; AUDIO_SLICES];

    for frame_idx in 0..num_frames {
        let slice_idx = (frame_idx / frames_per_slice).min(AUDIO_SLICES - 1);
        let offset = frame_idx * FRAME_SIZE;
        let frame = &samples[offset..offset + FRAME_SIZE];
        if rms_energy(frame) < SILENCE_THRESHOLD {
            continue;
        }

        let mut input = frame.to_vec();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut input, &mut spectrum)
            .map_err(|error| format!("FFT failed: {error}"))?;

        let mut votes = 0i32;
        for pair_idx in 0..marker_pairs {
            let bin_a = bin_lo + pair_idx * RELATIVE_PAIR_WIDTH;
            if extract_relative_pair(&spectrum, bin_a) {
                votes += 1;
            } else {
                votes -= 1;
            }
        }
        if votes.unsigned_abs() as usize > marker_pairs / 4 {
            active[slice_idx] = true;
        }
    }

    Ok(active.into_iter().filter(|&value| value).count())
}

fn read_wav_samples(
    reader: &mut hound::WavReader<std::io::BufReader<std::fs::File>>,
) -> Result<Vec<f32>, String> {
    let spec = reader.spec();
    if spec.sample_format == hound::SampleFormat::Float {
        reader
            .samples::<f32>()
            .map(|sample| sample.map_err(|error| format!("read WAV sample: {error}")))
            .collect()
    } else {
        let max_val = (1i32 << (spec.bits_per_sample - 1)) as f32;
        reader
            .samples::<i32>()
            .map(|sample| {
                sample
                    .map_err(|error| format!("read WAV sample: {error}"))
                    .map(|value| value as f32 / max_val)
            })
            .collect()
    }
}

fn audio_band_bins(sample_rate: u32) -> (usize, usize) {
    let bin_hz = sample_rate as f32 / FRAME_SIZE as f32;
    let max_bin = FRAME_SIZE / 2 - 1;
    let lo = ((BAND_LO_BIN as f32 * CANONICAL_SAMPLE_RATE as f32 / FRAME_SIZE as f32) / bin_hz)
        .round()
        .max(1.0) as usize;
    let hi = ((BAND_HI_BIN as f32 * CANONICAL_SAMPLE_RATE as f32 / FRAME_SIZE as f32) / bin_hz)
        .round()
        .min(max_bin as f32) as usize;
    (
        lo.min(max_bin - 1),
        hi.max(lo + RELATIVE_PAIR_WIDTH).min(max_bin),
    )
}

fn rms_energy(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum = frame.iter().map(|sample| sample * sample).sum::<f32>();
    (sum / frame.len() as f32).sqrt()
}

fn extract_relative_pair(
    spectrum: &[realfft::num_complex::Complex<f32>],
    group_start: usize,
) -> bool {
    let half_width = RELATIVE_PAIR_WIDTH / 2;
    let left = (0..half_width)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>();
    let right = (half_width..RELATIVE_PAIR_WIDTH)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>();
    left >= right
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

fn write_report(path: &Path, rows: &[ProbeRow]) -> Result<(), String> {
    let mut out = String::new();
    out.push_str("# Slice Marker Probe\n\n");
    out.push_str("| Media | Source | Operation | Duration ms | Slices | Active slices |\n");
    out.push_str("| --- | --- | --- | ---: | ---: | ---: |\n");
    for row in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            row.media,
            escape_md(&row.source),
            row.operation,
            row.duration_ms,
            row.slices,
            row.active_slices
        ));
    }
    fs::write(path, out).map_err(|error| format!("write probe report: {error}"))
}

fn print_summary(rows: &[ProbeRow]) {
    for row in rows {
        println!(
            "{} {} {}: {} ms, active_slices={}/{}",
            row.media, row.source, row.operation, row.duration_ms, row.active_slices, row.slices
        );
    }
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

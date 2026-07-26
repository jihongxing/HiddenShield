use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use image::{DynamicImage, ImageBuffer, Rgb};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use watermark_core::{
    compare_audio_quality, compare_image_quality, AudioQualityInput, AudioQualityReport,
    ImageQualityInput, ImageQualityReport,
};

const MAX_AUDIO_DURATION_SECONDS: f64 = 1_200.0;
const MAX_AUDIO_FILE_BYTES: u64 = 512 * 1024 * 1024;
const ALIGNMENT_PROXY_RATE: usize = 8_000;
const ALIGNMENT_WINDOW_SECONDS: f64 = 30.0;
const ALIGNMENT_MAX_OFFSET_SECONDS: f64 = 0.250;
const DURATION_TOLERANCE_SECONDS: f64 = 0.250;
const WAVEFORM_POINTS: usize = 1_600;
const ABX_CLIP_SECONDS: f64 = 10.0;
const IMAGE_PREVIEW_MAX_EDGE: u32 = 2_048;

#[derive(Default)]
pub struct LabState {
    session_dir: Mutex<Option<PathBuf>>,
}

impl Drop for LabState {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.session_dir.lock() {
            if let Some(path) = guard.take() {
                let _ = fs::remove_dir_all(path);
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MediaKind {
    Image,
    Audio,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaInfo {
    pub path: String,
    pub file_name: String,
    pub extension: String,
    pub file_bytes: u64,
    pub media_kind: MediaKind,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub duration_seconds: Option<f64>,
    pub sample_rate: Option<usize>,
    pub channels: Option<usize>,
    pub codec: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PairInspection {
    pub source: MediaInfo,
    pub candidate: MediaInfo,
    pub same_media_kind: bool,
    pub formally_comparable: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HeatmapAssets {
    pub x1_data_url: String,
    pub x4_data_url: String,
    pub x16_data_url: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageAnalysisResult {
    pub report: ImageQualityReport,
    pub source_preview_data_url: String,
    pub candidate_preview_data_url: String,
    pub heatmaps: HeatmapAssets,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioAlignment {
    pub source_trim_seconds: f64,
    pub candidate_trim_seconds: f64,
    pub detected_offset_seconds: f64,
    pub correlation_score: f64,
    pub common_duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WaveformData {
    pub source: Vec<f32>,
    pub candidate: Vec<f32>,
    pub difference: Vec<f32>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioAnalysisResult {
    pub report: AudioQualityReport,
    pub formally_comparable: bool,
    pub blockers: Vec<String>,
    pub warnings: Vec<String>,
    pub alignment: AudioAlignment,
    pub waveform: WaveformData,
    pub source_clip_path: String,
    pub candidate_clip_path: String,
    pub clip_start_seconds: f64,
    pub clip_duration_seconds: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AbxAssets {
    pub media_kind: MediaKind,
    pub source_asset: String,
    pub candidate_asset: String,
    pub start_seconds: f64,
    pub duration_seconds: f64,
}

#[derive(Debug, Deserialize)]
struct ProbeDocument {
    streams: Vec<ProbeStream>,
    format: Option<ProbeFormat>,
}

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: Option<String>,
    codec_name: Option<String>,
    sample_rate: Option<String>,
    channels: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

#[tauri::command]
pub fn inspect_media_pair(
    source_path: String,
    candidate_path: String,
) -> Result<PairInspection, String> {
    let source = inspect_media(Path::new(&source_path))?;
    let candidate = inspect_media(Path::new(&candidate_path))?;
    Ok(build_pair_inspection(source, candidate))
}

#[tauri::command]
pub fn analyze_image_pair(
    source_path: String,
    candidate_path: String,
    _state: State<'_, LabState>,
) -> Result<ImageAnalysisResult, String> {
    let source = load_rgb_image(Path::new(&source_path))?;
    let candidate = load_rgb_image(Path::new(&candidate_path))?;
    let report = compare_image_quality(ImageQualityInput {
        source: &source,
        candidate: &candidate,
    })?;
    let source_preview = preview_image(&source);
    let candidate_preview = preview_image(&candidate);
    Ok(ImageAnalysisResult {
        report,
        source_preview_data_url: image_data_url(&source_preview)?,
        candidate_preview_data_url: image_data_url(&candidate_preview)?,
        heatmaps: HeatmapAssets {
            x1_data_url: heatmap_data_url(&source_preview, &candidate_preview, 1)?,
            x4_data_url: heatmap_data_url(&source_preview, &candidate_preview, 4)?,
            x16_data_url: heatmap_data_url(&source_preview, &candidate_preview, 16)?,
        },
    })
}

#[tauri::command]
pub fn analyze_audio_pair(
    source_path: String,
    candidate_path: String,
    clip_start_seconds: Option<f64>,
    state: State<'_, LabState>,
) -> Result<AudioAnalysisResult, String> {
    let source_info = inspect_audio(Path::new(&source_path))?;
    let candidate_info = inspect_audio(Path::new(&candidate_path))?;
    let inspection = build_pair_inspection(source_info.clone(), candidate_info.clone());
    let source_rate = source_info
        .sample_rate
        .ok_or_else(|| "source audio sample rate unavailable".to_string())?;
    let candidate_rate = candidate_info
        .sample_rate
        .ok_or_else(|| "candidate audio sample rate unavailable".to_string())?;
    let analysis_rate = if source_rate == candidate_rate {
        source_rate
    } else {
        44_100
    };

    let alignment = estimate_alignment(
        Path::new(&source_path),
        Path::new(&candidate_path),
        source_info.duration_seconds.unwrap_or_default(),
        candidate_info.duration_seconds.unwrap_or_default(),
    )?;
    let source_samples = decode_audio_f32(
        Path::new(&source_path),
        analysis_rate,
        1,
        alignment.source_trim_seconds,
        alignment.common_duration_seconds,
    )?;
    let candidate_samples = decode_audio_f32(
        Path::new(&candidate_path),
        analysis_rate,
        1,
        alignment.candidate_trim_seconds,
        alignment.common_duration_seconds,
    )?;
    let report = compare_audio_quality(AudioQualityInput {
        source: &source_samples,
        candidate: &candidate_samples,
        sample_rate: analysis_rate,
        channels: 1,
    })?;
    let waveform = build_waveform(&source_samples, &candidate_samples);

    let session_dir = session_dir(&state)?;
    let audio_dir = session_dir.join("audio");
    fs::create_dir_all(&audio_dir).map_err(|error| format!("create audio session dir: {error}"))?;
    let requested_start = clip_start_seconds.unwrap_or(0.0).max(0.0);
    let clip_start = requested_start.min((alignment.common_duration_seconds - 0.1).max(0.0));
    let clip_duration =
        ABX_CLIP_SECONDS.min((alignment.common_duration_seconds - clip_start).max(0.1));
    let source_clip = audio_dir.join("source-clip.wav");
    let candidate_clip = audio_dir.join("candidate-clip.wav");
    create_audio_snippet(
        Path::new(&source_path),
        alignment.source_trim_seconds + clip_start,
        clip_duration,
        &source_clip,
    )?;
    create_audio_snippet(
        Path::new(&candidate_path),
        alignment.candidate_trim_seconds + clip_start,
        clip_duration,
        &candidate_clip,
    )?;

    let mut blockers = inspection.blockers;
    if alignment.correlation_score < 0.10 {
        blockers.push("音频相关性过低，可能不是同一素材或无法可靠对齐".to_string());
    }
    if alignment.detected_offset_seconds.abs() > ALIGNMENT_MAX_OFFSET_SECONDS {
        blockers.push("检测到的编码偏移超过 250 ms 正式比较范围".to_string());
    }
    let formally_comparable = inspection.formally_comparable && blockers.is_empty();

    Ok(AudioAnalysisResult {
        report,
        formally_comparable,
        blockers,
        warnings: inspection.warnings,
        alignment,
        waveform,
        source_clip_path: path_string(&source_clip),
        candidate_clip_path: path_string(&candidate_clip),
        clip_start_seconds: clip_start,
        clip_duration_seconds: clip_duration,
    })
}

#[tauri::command]
pub fn prepare_abx_assets(
    source_path: String,
    candidate_path: String,
    start_seconds: Option<f64>,
    state: State<'_, LabState>,
) -> Result<AbxAssets, String> {
    let source = inspect_media(Path::new(&source_path))?;
    let candidate = inspect_media(Path::new(&candidate_path))?;
    if source.media_kind != candidate.media_kind {
        return Err("ABX requires two files of the same media type".to_string());
    }
    match source.media_kind {
        MediaKind::Image => {
            let source_image = load_rgb_image(Path::new(&source_path))?;
            let candidate_image = load_rgb_image(Path::new(&candidate_path))?;
            prepare_image_abx_assets(&source_image, &candidate_image)
        }
        MediaKind::Audio => {
            let session_dir = session_dir(&state)?;
            let abx_dir = session_dir.join("abx");
            fs::create_dir_all(&abx_dir)
                .map_err(|error| format!("create ABX session dir: {error}"))?;
            let alignment = estimate_alignment(
                Path::new(&source_path),
                Path::new(&candidate_path),
                source.duration_seconds.unwrap_or_default(),
                candidate.duration_seconds.unwrap_or_default(),
            )?;
            let start = start_seconds
                .unwrap_or(0.0)
                .max(0.0)
                .min((alignment.common_duration_seconds - 0.1).max(0.0));
            let duration =
                ABX_CLIP_SECONDS.min((alignment.common_duration_seconds - start).max(0.1));
            let source_asset = abx_dir.join("source.wav");
            let candidate_asset = abx_dir.join("candidate.wav");
            create_audio_snippet(
                Path::new(&source_path),
                alignment.source_trim_seconds + start,
                duration,
                &source_asset,
            )?;
            create_audio_snippet(
                Path::new(&candidate_path),
                alignment.candidate_trim_seconds + start,
                duration,
                &candidate_asset,
            )?;
            Ok(AbxAssets {
                media_kind: MediaKind::Audio,
                source_asset: file_data_url(&source_asset, "audio/wav")?,
                candidate_asset: file_data_url(&candidate_asset, "audio/wav")?,
                start_seconds: start,
                duration_seconds: duration,
            })
        }
    }
}

#[tauri::command]
pub fn clear_lab_session(state: State<'_, LabState>) -> Result<(), String> {
    let mut guard = state
        .session_dir
        .lock()
        .map_err(|_| "quality lab session lock poisoned".to_string())?;
    if let Some(path) = guard.take() {
        if path.exists() {
            fs::remove_dir_all(&path)
                .map_err(|error| format!("remove quality lab session: {error}"))?;
        }
    }
    Ok(())
}

fn inspect_media(path: &Path) -> Result<MediaInfo, String> {
    let extension = extension(path)?;
    if is_image_extension(&extension) {
        inspect_image(path)
    } else if is_audio_extension(&extension) {
        inspect_audio(path)
    } else {
        Err(format!(
            "unsupported media extension .{extension}; expected PNG/JPEG/WebP/BMP or WAV/MP3/FLAC/M4A/AAC"
        ))
    }
}

fn inspect_image(path: &Path) -> Result<MediaInfo, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("stat image: {error}"))?;
    let (width, height) =
        image::image_dimensions(path).map_err(|error| format!("read image dimensions: {error}"))?;
    Ok(MediaInfo {
        path: path_string(path),
        file_name: file_name(path),
        extension: extension(path)?,
        file_bytes: metadata.len(),
        media_kind: MediaKind::Image,
        width: Some(width),
        height: Some(height),
        duration_seconds: None,
        sample_rate: None,
        channels: None,
        codec: None,
    })
}

fn inspect_audio(path: &Path) -> Result<MediaInfo, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("stat audio: {error}"))?;
    if metadata.len() > MAX_AUDIO_FILE_BYTES {
        return Err("audio file exceeds the 512 MiB laboratory limit".to_string());
    }
    let output = Command::new(ffprobe_path())
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_type,codec_name,sample_rate,channels",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .map_err(|error| format!("start ffprobe: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "ffprobe failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let document: ProbeDocument = serde_json::from_slice(&output.stdout)
        .map_err(|error| format!("parse ffprobe output: {error}"))?;
    let stream = document
        .streams
        .iter()
        .find(|stream| stream.codec_type.as_deref() == Some("audio"))
        .ok_or_else(|| "file has no readable audio stream".to_string())?;
    let duration_seconds = document
        .format
        .and_then(|format| format.duration)
        .and_then(|value| value.parse::<f64>().ok())
        .ok_or_else(|| "audio duration unavailable".to_string())?;
    if duration_seconds > MAX_AUDIO_DURATION_SECONDS {
        return Err("audio duration exceeds the 20 minute laboratory limit".to_string());
    }
    let sample_rate = stream
        .sample_rate
        .as_deref()
        .and_then(|value| value.parse::<usize>().ok())
        .ok_or_else(|| "audio sample rate unavailable".to_string())?;
    let channels = stream
        .channels
        .ok_or_else(|| "audio channel count unavailable".to_string())?;

    Ok(MediaInfo {
        path: path_string(path),
        file_name: file_name(path),
        extension: extension(path)?,
        file_bytes: metadata.len(),
        media_kind: MediaKind::Audio,
        width: None,
        height: None,
        duration_seconds: Some(duration_seconds),
        sample_rate: Some(sample_rate),
        channels: Some(channels),
        codec: stream.codec_name.clone(),
    })
}

fn build_pair_inspection(source: MediaInfo, candidate: MediaInfo) -> PairInspection {
    let same_media_kind = source.media_kind == candidate.media_kind;
    let mut blockers = Vec::new();
    let mut warnings = Vec::new();
    if !same_media_kind {
        blockers.push("两侧必须选择同一媒体类型".to_string());
    } else {
        match source.media_kind {
            MediaKind::Image => {
                if source.width != candidate.width || source.height != candidate.height {
                    blockers.push("图片尺寸不一致，不能计算正式 PSNR/SSIM".to_string());
                }
                if source.extension != candidate.extension {
                    warnings.push(
                        "图片容器格式不同；指标按解码 RGB 像素计算，不按文件字节比较".to_string(),
                    );
                }
            }
            MediaKind::Audio => {
                if source.sample_rate != candidate.sample_rate {
                    blockers
                        .push("音频采样率不一致，只允许试听和诊断，不给正式阈值结论".to_string());
                }
                if source.channels != candidate.channels {
                    blockers
                        .push("音频声道数不一致，只允许试听和诊断，不给正式阈值结论".to_string());
                }
                let duration_delta = (source.duration_seconds.unwrap_or_default()
                    - candidate.duration_seconds.unwrap_or_default())
                .abs();
                if duration_delta > DURATION_TOLERANCE_SECONDS {
                    blockers.push(format!(
                        "音频时长差 {:.3}s 超过正式比较容差 {:.3}s",
                        duration_delta, DURATION_TOLERANCE_SECONDS
                    ));
                }
                if source.codec != candidate.codec {
                    warnings.push(
                        "音频编码格式不同；指标包含编码、解码和水印写入带来的总体差异".to_string(),
                    );
                }
            }
        }
    }
    PairInspection {
        source,
        candidate,
        same_media_kind,
        formally_comparable: same_media_kind && blockers.is_empty(),
        blockers,
        warnings,
    }
}

fn estimate_alignment(
    source_path: &Path,
    candidate_path: &Path,
    source_duration: f64,
    candidate_duration: f64,
) -> Result<AudioAlignment, String> {
    let proxy_duration = ALIGNMENT_WINDOW_SECONDS
        .min(source_duration)
        .min(candidate_duration)
        .max(1.0);
    let source_proxy = decode_audio_f32(source_path, ALIGNMENT_PROXY_RATE, 1, 0.0, proxy_duration)?;
    let candidate_proxy =
        decode_audio_f32(candidate_path, ALIGNMENT_PROXY_RATE, 1, 0.0, proxy_duration)?;
    let max_shift = (ALIGNMENT_MAX_OFFSET_SECONDS * ALIGNMENT_PROXY_RATE as f64).round() as isize;
    let (shift, score) = best_correlation_shift(&source_proxy, &candidate_proxy, max_shift);
    let detected_offset_seconds = shift as f64 / ALIGNMENT_PROXY_RATE as f64;
    let source_trim_seconds = if shift < 0 {
        -detected_offset_seconds
    } else {
        0.0
    };
    let candidate_trim_seconds = if shift > 0 {
        detected_offset_seconds
    } else {
        0.0
    };
    let common_duration_seconds = (source_duration - source_trim_seconds)
        .min(candidate_duration - candidate_trim_seconds)
        .max(0.0);
    if common_duration_seconds < 1.0 {
        return Err("aligned audio is too short for quality analysis".to_string());
    }
    Ok(AudioAlignment {
        source_trim_seconds,
        candidate_trim_seconds,
        detected_offset_seconds,
        correlation_score: score,
        common_duration_seconds,
    })
}

fn best_correlation_shift(source: &[f32], candidate: &[f32], max_shift: isize) -> (isize, f64) {
    let mut best_shift = 0;
    let mut best_score = f64::NEG_INFINITY;
    for shift in -max_shift..=max_shift {
        let source_start = if shift < 0 { (-shift) as usize } else { 0 };
        let candidate_start = if shift > 0 { shift as usize } else { 0 };
        let len = (source.len().saturating_sub(source_start))
            .min(candidate.len().saturating_sub(candidate_start));
        if len < 1_000 {
            continue;
        }
        let step = 8;
        let mut dot = 0.0;
        let mut source_energy = 0.0;
        let mut candidate_energy = 0.0;
        for index in (0..len).step_by(step) {
            let left = f64::from(source[source_start + index]);
            let right = f64::from(candidate[candidate_start + index]);
            dot += left * right;
            source_energy += left * left;
            candidate_energy += right * right;
        }
        let score = dot / (source_energy.sqrt() * candidate_energy.sqrt()).max(1e-12);
        if score > best_score {
            best_score = score;
            best_shift = shift;
        }
    }
    (best_shift, best_score.max(-1.0))
}

fn decode_audio_f32(
    path: &Path,
    sample_rate: usize,
    channels: usize,
    start_seconds: f64,
    duration_seconds: f64,
) -> Result<Vec<f32>, String> {
    let output = Command::new(ffmpeg_path())
        .args(["-v", "error", "-ss"])
        .arg(format!("{start_seconds:.6}"))
        .arg("-i")
        .arg(path)
        .args(["-t"])
        .arg(format!("{duration_seconds:.6}"))
        .args(["-map", "0:a:0", "-ac"])
        .arg(channels.to_string())
        .arg("-ar")
        .arg(sample_rate.to_string())
        .args(["-f", "f32le", "-acodec", "pcm_f32le", "pipe:1"])
        .output()
        .map_err(|error| format!("start ffmpeg audio decode: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "ffmpeg audio decode failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    if output.stdout.len() % 4 != 0 {
        return Err("ffmpeg returned invalid f32 audio byte length".to_string());
    }
    Ok(output
        .stdout
        .chunks_exact(4)
        .map(|bytes| f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
        .collect())
}

fn create_audio_snippet(
    path: &Path,
    start_seconds: f64,
    duration_seconds: f64,
    output_path: &Path,
) -> Result<(), String> {
    let output = Command::new(ffmpeg_path())
        .args(["-y", "-v", "error", "-ss"])
        .arg(format!("{start_seconds:.6}"))
        .arg("-i")
        .arg(path)
        .arg("-t")
        .arg(format!("{duration_seconds:.6}"))
        .args(["-map", "0:a:0", "-c:a", "pcm_s16le"])
        .arg(output_path)
        .output()
        .map_err(|error| format!("start ffmpeg ABX snippet: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "ffmpeg ABX snippet failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn build_waveform(source: &[f32], candidate: &[f32]) -> WaveformData {
    let len = source.len().min(candidate.len());
    let bucket_size = len.div_ceil(WAVEFORM_POINTS).max(1);
    let mut source_points = Vec::new();
    let mut candidate_points = Vec::new();
    let mut difference_points = Vec::new();
    for start in (0..len).step_by(bucket_size) {
        let end = (start + bucket_size).min(len);
        let source_peak = source[start..end]
            .iter()
            .fold(0.0_f32, |acc, value| acc.max(value.abs()));
        let candidate_peak = candidate[start..end]
            .iter()
            .fold(0.0_f32, |acc, value| acc.max(value.abs()));
        let difference_peak = source[start..end]
            .iter()
            .zip(candidate[start..end].iter())
            .fold(0.0_f32, |acc, (left, right)| {
                acc.max((*left - *right).abs())
            });
        source_points.push(source_peak);
        candidate_points.push(candidate_peak);
        difference_points.push(difference_peak);
    }
    WaveformData {
        source: source_points,
        candidate: candidate_points,
        difference: difference_points,
    }
}

fn heatmap_data_url(
    source: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    candidate: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    amplification: u16,
) -> Result<String, String> {
    if source.dimensions() != candidate.dimensions() {
        return Err("image dimensions differ".to_string());
    }
    let heatmap = ImageBuffer::from_fn(source.width(), source.height(), |x, y| {
        let left = source.get_pixel(x, y);
        let right = candidate.get_pixel(x, y);
        let difference = (0..3)
            .map(|channel| u16::from(left[channel].abs_diff(right[channel])))
            .max()
            .unwrap_or(0)
            .saturating_mul(amplification)
            .min(255) as u8;
        let green = difference.saturating_mul(2).min(180);
        Rgb([difference, green, 18])
    });
    image_data_url(&heatmap)
}

fn image_data_url(image: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> Result<String, String> {
    let mut encoded = Cursor::new(Vec::new());
    DynamicImage::ImageRgb8(image.clone())
        .write_to(&mut encoded, image::ImageFormat::Png)
        .map_err(|error| format!("encode preview PNG: {error}"))?;
    Ok(format!(
        "data:image/png;base64,{}",
        BASE64.encode(encoded.into_inner())
    ))
}

fn file_data_url(path: &Path, mime_type: &str) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("read ABX asset: {error}"))?;
    Ok(format!("data:{mime_type};base64,{}", BASE64.encode(bytes)))
}

fn preview_image(image: &ImageBuffer<Rgb<u8>, Vec<u8>>) -> ImageBuffer<Rgb<u8>, Vec<u8>> {
    if image.width() <= IMAGE_PREVIEW_MAX_EDGE && image.height() <= IMAGE_PREVIEW_MAX_EDGE {
        return image.clone();
    }
    DynamicImage::ImageRgb8(image.clone())
        .thumbnail(IMAGE_PREVIEW_MAX_EDGE, IMAGE_PREVIEW_MAX_EDGE)
        .to_rgb8()
}

fn prepare_image_abx_assets(
    source: &ImageBuffer<Rgb<u8>, Vec<u8>>,
    candidate: &ImageBuffer<Rgb<u8>, Vec<u8>>,
) -> Result<AbxAssets, String> {
    if source.dimensions() != candidate.dimensions() {
        return Err("image ABX requires matching dimensions".to_string());
    }
    Ok(AbxAssets {
        media_kind: MediaKind::Image,
        source_asset: image_data_url(&preview_image(source))?,
        candidate_asset: image_data_url(&preview_image(candidate))?,
        start_seconds: 0.0,
        duration_seconds: 0.0,
    })
}

fn load_rgb_image(path: &Path) -> Result<ImageBuffer<Rgb<u8>, Vec<u8>>, String> {
    image::ImageReader::open(path)
        .map_err(|error| format!("open image: {error}"))?
        .with_guessed_format()
        .map_err(|error| format!("guess image format: {error}"))?
        .decode()
        .map_err(|error| format!("decode image: {error}"))
        .map(|image| image.to_rgb8())
}

fn session_dir(state: &State<'_, LabState>) -> Result<PathBuf, String> {
    let mut guard = state
        .session_dir
        .lock()
        .map_err(|_| "quality lab session lock poisoned".to_string())?;
    if let Some(path) = guard.as_ref() {
        return Ok(path.clone());
    }
    let path = env::temp_dir()
        .join("hiddenshield-perceptual-quality-lab")
        .join(format!("session-{}", unix_millis()));
    fs::create_dir_all(&path).map_err(|error| format!("create quality lab session: {error}"))?;
    *guard = Some(path.clone());
    Ok(path)
}

fn ffmpeg_path() -> String {
    env::var("HIDDENSHIELD_FFMPEG_PATH").unwrap_or_else(|_| "ffmpeg".to_string())
}

fn ffprobe_path() -> String {
    env::var("HIDDENSHIELD_FFPROBE_PATH").unwrap_or_else(|_| "ffprobe".to_string())
}

fn extension(path: &Path) -> Result<String, String> {
    path.extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| "file extension unavailable".to_string())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn path_string(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn is_image_extension(extension: &str) -> bool {
    matches!(extension, "png" | "jpg" | "jpeg" | "webp" | "bmp")
}

fn is_audio_extension(extension: &str) -> bool {
    matches!(extension, "wav" | "mp3" | "flac" | "m4a" | "aac")
}

fn unix_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn correlation_detects_candidate_delay() {
        let source = (0..20_000)
            .map(|index| ((index as f32 / 8_000.0) * std::f32::consts::TAU * 431.0).sin())
            .collect::<Vec<_>>();
        let mut candidate = vec![0.0; 400];
        candidate.extend_from_slice(&source);
        let (shift, score) = best_correlation_shift(&source, &candidate, 800);
        assert_eq!(shift, 400);
        assert!(score > 0.99);
    }

    #[test]
    fn waveform_is_bounded() {
        let source = vec![0.2; 10_000];
        let candidate = vec![0.3; 10_000];
        let waveform = build_waveform(&source, &candidate);
        assert!(waveform.source.len() <= WAVEFORM_POINTS + 1);
        assert!((waveform.difference[0] - 0.1).abs() < 1e-6);
    }

    #[test]
    fn image_preview_data_url_contains_valid_png() {
        let image = ImageBuffer::from_pixel(2, 2, Rgb([12, 34, 56]));
        let data_url = image_data_url(&image).expect("preview should encode");
        let encoded = data_url
            .strip_prefix("data:image/png;base64,")
            .expect("preview should use a PNG data URL");
        let bytes = BASE64.decode(encoded).expect("base64 should decode");
        let decoded = image::load_from_memory(&bytes)
            .expect("PNG should decode")
            .to_rgb8();
        assert_eq!(decoded, image);
    }

    #[test]
    fn image_abx_assets_use_data_urls() {
        let source = ImageBuffer::from_pixel(4, 4, Rgb([12, 34, 56]));
        let candidate = ImageBuffer::from_pixel(4, 4, Rgb([13, 34, 56]));
        let assets =
            prepare_image_abx_assets(&source, &candidate).expect("ABX assets should encode");
        assert_eq!(assets.media_kind, MediaKind::Image);
        assert!(assets.source_asset.starts_with("data:image/png;base64,"));
        assert!(assets.candidate_asset.starts_with("data:image/png;base64,"));
    }

    #[test]
    fn audio_abx_asset_uses_wav_data_url() {
        let file = tempfile::NamedTempFile::new().expect("temp file should exist");
        fs::write(file.path(), b"RIFFtest").expect("fixture should write");
        let data_url = file_data_url(file.path(), "audio/wav").expect("asset should encode");
        assert_eq!(data_url, "data:audio/wav;base64,UklGRnRlc3Q=");
    }
}

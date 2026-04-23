#![allow(dead_code)]

use crate::commands::probe::SourceMeta;
use crate::commands::transcode::{AspectStrategy, EncodingMode, Platform, TranscodeOptions};
use crate::encoder::hw_detect::{DetectedHardware, HwEncoderType};

/// Complete FFmpeg transcode configuration for a single platform.
pub struct TranscodeConfig {
    pub video_filter: String,
    pub video_codec: String,
    pub video_params: Vec<String>,
    pub audio_params: Vec<String>,
    pub container_params: Vec<String>,
    pub output_suffix: String,
}

pub fn platform_label(platform: Platform) -> &'static str {
    match platform {
        Platform::Douyin => "抖音 1080x1920 / H.264 / 30fps",
        Platform::Bilibili => "B站 1920x1080 / HEVC / 60fps",
        Platform::Xiaohongshu => "小红书 1080x1440 / H.264 / 30fps",
    }
}

/// Per-platform spec: (target_w, target_h, codec_family, crf, maxrate_kbps, target_fps)
struct PlatformSpec {
    width: u32,
    height: u32,
    codec_family: CodecFamily,
    crf: u32,
    maxrate_kbps: u32,
    fps: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CodecFamily {
    H264,
    Hevc,
}

fn platform_spec(platform: Platform, source_fps: f64) -> PlatformSpec {
    match platform {
        Platform::Douyin => PlatformSpec {
            width: 1080,
            height: 1920,
            codec_family: CodecFamily::H264,
            crf: 18,
            maxrate_kbps: 12000,
            fps: 30,
        },
        Platform::Bilibili => {
            let fps = if source_fps >= 50.0 { 60 } else { 30 };
            PlatformSpec {
                width: 1920,
                height: 1080,
                codec_family: CodecFamily::Hevc,
                crf: 20,
                maxrate_kbps: 16000,
                fps,
            }
        }
        Platform::Xiaohongshu => PlatformSpec {
            width: 1080,
            height: 1440,
            codec_family: CodecFamily::H264,
            crf: 17,
            maxrate_kbps: 15000,
            fps: 30,
        },
    }
}

/// Pick the video codec string based on platform codec family, encoding mode, and hardware.
fn pick_video_codec(
    family: CodecFamily,
    encoding_mode: &EncodingMode,
    hw: &DetectedHardware,
) -> String {
    match encoding_mode {
        EncodingMode::HighQualityCpu => match family {
            CodecFamily::H264 => "libx264".to_string(),
            CodecFamily::Hevc => "libx265".to_string(),
        },
        EncodingMode::FastGpu => {
            if hw.hw_type == HwEncoderType::Software {
                // No hardware available — fall back to software.
                return match family {
                    CodecFamily::H264 => "libx264".to_string(),
                    CodecFamily::Hevc => "libx265".to_string(),
                };
            }
            // Find a matching hw encoder for the requested codec family.
            let suffix = match family {
                CodecFamily::H264 => "h264_",
                CodecFamily::Hevc => "hevc_",
            };
            hw.available_encoders
                .iter()
                .find(|e| e.starts_with(suffix))
                .cloned()
                .unwrap_or_else(|| match family {
                    CodecFamily::H264 => "libx264".to_string(),
                    CodecFamily::Hevc => "libx265".to_string(),
                })
        }
    }
}

/// Build the video_filter string for scaling/padding or cropping.
fn build_video_filter(
    _source: &SourceMeta,
    spec: &PlatformSpec,
    strategy: &AspectStrategy,
    tonemap_filter: Option<&str>,
) -> String {
    let mut parts: Vec<String> = Vec::new();

    // Tonemap filter goes first if present.
    if let Some(tm) = tonemap_filter {
        parts.push(tm.to_string());
    }

    match strategy {
        AspectStrategy::Letterbox => {
            // Scale to fit within target, then pad to exact target size.
            parts.push(format!(
                "scale={}:{}:force_original_aspect_ratio=decrease",
                spec.width, spec.height
            ));
            parts.push(format!(
                "pad={}:{}:(ow-iw)/2:(oh-ih)/2:black",
                spec.width, spec.height
            ));
        }
        AspectStrategy::SmartCrop => {
            // Scale to cover target, then crop to exact target size.
            parts.push(format!(
                "scale={}:{}:force_original_aspect_ratio=increase",
                spec.width, spec.height
            ));
            parts.push(format!("crop={}:{}", spec.width, spec.height));
        }
    }

    parts.join(",")
}

/// Build a complete `TranscodeConfig` for the given platform + source + hardware + options.
pub fn build_transcode_config(
    platform: Platform,
    source: &SourceMeta,
    hw_encoder: &DetectedHardware,
    options: &TranscodeOptions,
    tonemap_filter: Option<&str>,
) -> TranscodeConfig {
    let spec = platform_spec(platform, source.fps);
    let video_codec = pick_video_codec(spec.codec_family, &options.encoding_mode, hw_encoder);
    let video_filter = build_video_filter(source, &spec, &options.aspect_strategy, tonemap_filter);

    let bufsize = spec.maxrate_kbps * 2;

    let mut video_params = vec![
        "-r".to_string(),
        spec.fps.to_string(),
        "-crf".to_string(),
        spec.crf.to_string(),
        "-maxrate".to_string(),
        format!("{}k", spec.maxrate_kbps),
        "-bufsize".to_string(),
        format!("{}k", bufsize),
    ];

    // Add profile for software codecs.
    match spec.codec_family {
        CodecFamily::H264 => {
            video_params.extend_from_slice(&["-profile:v".to_string(), "high".to_string()]);
        }
        CodecFamily::Hevc => {
            video_params.extend_from_slice(&["-profile:v".to_string(), "main".to_string()]);
        }
    }

    let audio_params = vec![
        "-c:a".to_string(),
        "aac".to_string(),
        "-b:a".to_string(),
        "192k".to_string(),
        "-ar".to_string(),
        "44100".to_string(),
        "-ac".to_string(),
        "2".to_string(),
    ];

    let container_params = vec!["-movflags".to_string(), "+faststart".to_string()];

    let suffix = match platform {
        Platform::Douyin => "_抖音优化版.mp4",
        Platform::Bilibili => "_B站优化版.mp4",
        Platform::Xiaohongshu => "_小红书优化版.mp4",
    };

    TranscodeConfig {
        video_filter,
        video_codec,
        video_params,
        audio_params,
        container_params,
        output_suffix: suffix.to_string(),
    }
}

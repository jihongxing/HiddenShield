use std::path::Path;
use std::process::Stdio;

use serde::{Deserialize, Serialize};
use tokio::process::Command;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HwEncoderType {
    Nvenc,
    VideoToolbox,
    Qsv,
    Amf,
    Software,
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectedHardware {
    pub preferred_encoder: String,
    pub available_encoders: Vec<String>,
    pub hw_type: HwEncoderType,
}

// ---------------------------------------------------------------------------
// Detection
// ---------------------------------------------------------------------------

/// Detect available hardware encoders by actually running ffmpeg with each
/// candidate encoder. Returns the best available encoder (or software fallback).
pub async fn detect_hardware(ffmpeg: &Path) -> DetectedHardware {
    let candidates = platform_candidates();
    let mut available: Vec<String> = Vec::new();

    for encoder in &candidates {
        if test_encoder(ffmpeg, encoder).await {
            available.push(encoder.to_string());
        }
    }

    // Always include software fallback encoders in the available list.
    for sw in &["libx264", "libx265"] {
        if !available.contains(&sw.to_string()) {
            available.push(sw.to_string());
        }
    }

    let (preferred, hw_type) = pick_preferred(&available);

    DetectedHardware {
        preferred_encoder: preferred,
        available_encoders: available,
        hw_type,
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Return the list of hardware encoder candidates for the current platform.
fn platform_candidates() -> Vec<&'static str> {
    if cfg!(target_os = "macos") {
        vec!["h264_videotoolbox", "hevc_videotoolbox"]
    } else {
        // Windows / Linux: NVENC → QSV → AMF priority
        vec![
            "h264_nvenc",
            "hevc_nvenc",
            "h264_qsv",
            "hevc_qsv",
            "h264_amf",
            "hevc_amf",
        ]
    }
}

/// Test whether a specific encoder is usable by running a minimal ffmpeg encode.
async fn test_encoder(ffmpeg: &Path, encoder: &str) -> bool {
    let result = Command::new(ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "nullsrc=s=256x256:d=1",
            "-frames:v",
            "1",
            "-c:v",
            encoder,
            "-f",
            "null",
            "-",
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .await;

    matches!(result, Ok(output) if output.status.success())
}

/// Pick the best encoder from the available list and determine the hw type.
fn pick_preferred(available: &[String]) -> (String, HwEncoderType) {
    // Priority order matches the candidate lists above.
    let priority: &[(&str, HwEncoderType)] = if cfg!(target_os = "macos") {
        &[
            ("h264_videotoolbox", HwEncoderType::VideoToolbox),
            ("hevc_videotoolbox", HwEncoderType::VideoToolbox),
        ]
    } else {
        &[
            ("h264_nvenc", HwEncoderType::Nvenc),
            ("hevc_nvenc", HwEncoderType::Nvenc),
            ("h264_qsv", HwEncoderType::Qsv),
            ("hevc_qsv", HwEncoderType::Qsv),
            ("h264_amf", HwEncoderType::Amf),
            ("hevc_amf", HwEncoderType::Amf),
        ]
    };

    for (name, hw_type) in priority {
        if available.iter().any(|e| e == name) {
            return (name.to_string(), *hw_type);
        }
    }

    // Fallback to software
    ("libx264".to_string(), HwEncoderType::Software)
}

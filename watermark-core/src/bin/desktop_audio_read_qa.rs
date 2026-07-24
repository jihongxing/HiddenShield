use std::{env, fs, path::Path};

use serde_json::json;
use watermark_core::{MediaInput, WatermarkService};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let path = env::args()
        .nth(1)
        .ok_or_else(|| "usage: desktop_audio_read_qa <protected.wav>".to_string())?;
    let bytes = fs::read(&path).map_err(|error| format!("read {path}: {error}"))?;
    let decoded = WatermarkService::extract(MediaInput::AudioWavBytes { bytes })
        .map_err(|error| format!("extract {}: {error}", Path::new(&path).display()))?;
    println!(
        "{}",
        serde_json::to_string(&json!({
            "status": "verified",
            "path": path,
            "watermarkUid": decoded.watermark_uid(),
            "payloadProtocolVersion": decoded.protocol_version(),
        }))
        .map_err(|error| error.to_string())?
    );
    Ok(())
}

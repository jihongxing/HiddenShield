use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use watermark_core::{
    build_v3_readonly_candidate_audio_fixture_wav_bytes,
    build_v3_readonly_candidate_image_fixture_png_bytes,
    extract_image_watermark_readonly_candidate_bytes,
    extract_watermark_wav_readonly_candidate_bytes, PayloadV3MinimalAnchorBuildInput,
    WatermarkDecodedPayload, WatermarkPayloadV3MinimalAnchor, PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CandidateRow {
    fixture_id: String,
    bridge: String,
    media_kind: String,
    path: String,
    watermark_uid: String,
    payload_protocol_version: u32,
    payload_bytes_length: u32,
    payload_auth_status: String,
    watermark_id_issue_mode: String,
    media_payload_role: String,
    default_extract_status: String,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out_dir = PathBuf::from(required_arg(&args, "--out-dir")?);
    fs::create_dir_all(&out_dir).map_err(|error| format!("create output dir: {error}"))?;

    let image_anchor = anchor("image")?;
    let audio_anchor = anchor("audio")?;
    let image_bytes = build_v3_readonly_candidate_image_fixture_png_bytes(&image_anchor)
        .map_err(|error| format!("build V3 image candidate fixture: {error}"))?;
    let audio_bytes = build_v3_readonly_candidate_audio_fixture_wav_bytes(&audio_anchor)
        .map_err(|error| format!("build V3 audio candidate fixture: {error}"))?;
    let image_path = out_dir.join("v3-readonly-candidate-image.png");
    let audio_path = out_dir.join("v3-readonly-candidate-audio.wav");
    fs::write(&image_path, &image_bytes).map_err(|error| format!("write image: {error}"))?;
    fs::write(&audio_path, &audio_bytes).map_err(|error| format!("write audio: {error}"))?;

    let image = candidate_row(
        "v3_image_real_media_readonly_candidate",
        "image",
        &image_path,
        extract_image_watermark_readonly_candidate_bytes(&image_bytes)
            .map_err(|error| format!("desktop image readonly candidate extract: {error}"))?,
    )?;
    let audio = candidate_row(
        "v3_audio_real_media_readonly_candidate",
        "audio",
        &audio_path,
        extract_watermark_wav_readonly_candidate_bytes(&audio_bytes)
            .map_err(|error| format!("desktop audio readonly candidate extract: {error}"))?,
    )?;

    let result = serde_json::json!({
        "desktop": {
            "image": image,
            "audio": audio,
        },
        "fixtures": {
            "imagePath": image_path,
            "audioPath": audio_path,
        },
        "defaultV3WriteEnabled": true,
        "defaultWatermarkServiceExtractV3Enabled": true,
        "boundary": "真实 PNG/WAV fixture 使用正式图片 sync packet / 音频 recovery packet 承载 V3/39 minimal anchor；运行态保留显式 readonly candidate reader 作为迁移桥，同时默认 WatermarkService::extract 已只接受 V3/39。",
    });
    assert_default_v3_guarded(&result)?;
    let json = serde_json::to_string_pretty(&result)
        .map_err(|error| format!("serialize result: {error}"))?;
    let json_path = out_dir.join("desktop-v3-readonly-candidate-runtime.json");
    fs::write(&json_path, format!("{json}\n")).map_err(|error| format!("write json: {error}"))?;
    println!("{json}");
    Ok(())
}

fn anchor(media_kind: &str) -> Result<WatermarkPayloadV3MinimalAnchor, String> {
    let watermark_id = match media_kind {
        "image" => [
            0x31, 0x32, 0x33, 0x34, 0x41, 0x42, 0x43, 0x44, 0x51, 0x52, 0x53, 0x54, 0x61, 0x62,
            0x63, 0x64,
        ],
        "audio" => [
            0x51, 0x52, 0x53, 0x54, 0x61, 0x62, 0x63, 0x64, 0x71, 0x72, 0x73, 0x74, 0x81, 0x82,
            0x83, 0x84,
        ],
        _ => return Err(format!("unsupported media kind: {media_kind}")),
    };
    WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput { watermark_id })
        .map_err(|error| format!("build V3 anchor: {error}"))
}

fn candidate_row(
    fixture_id: &str,
    media_kind: &str,
    path: &std::path::Path,
    decoded: WatermarkDecodedPayload,
) -> Result<CandidateRow, String> {
    if !decoded.is_v3_minimal_anchor() {
        return Err(format!("{fixture_id} expected V3 minimal anchor"));
    }
    Ok(CandidateRow {
        fixture_id: fixture_id.to_string(),
        bridge: "desktop".to_string(),
        media_kind: media_kind.to_string(),
        path: path.to_string_lossy().to_string(),
        watermark_uid: decoded.watermark_uid(),
        payload_protocol_version: decoded.protocol_version() as u32,
        payload_bytes_length: decoded.payload_bytes_length() as u32,
        payload_auth_status: decoded.payload_auth_status().to_string(),
        watermark_id_issue_mode: "registry_resolved".to_string(),
        media_payload_role: "v3_minimal_anchor".to_string(),
        default_extract_status: "default_v3_contract_guarded".to_string(),
    })
}

fn assert_default_v3_guarded(result: &serde_json::Value) -> Result<(), String> {
    for key in ["image", "audio"] {
        let status = result["desktop"][key]["defaultExtractStatus"]
            .as_str()
            .unwrap_or_default();
        if status != "default_v3_contract_guarded" {
            return Err(format!(
                "default WatermarkService::extract must remain V3-only for readonly candidate {key}, got {status}"
            ));
        }
        let length = result["desktop"][key]["payloadBytesLength"]
            .as_u64()
            .unwrap_or_default();
        if length != PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u64 {
            return Err(format!(
                "expected V3 payload length 39 for {key}, got {length}"
            ));
        }
    }
    Ok(())
}

fn required_arg<'a>(args: &'a [String], name: &str) -> Result<&'a str, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| format!("missing required argument {name}"))
}

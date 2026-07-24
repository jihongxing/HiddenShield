use hound::{SampleFormat, WavSpec, WavWriter};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;
use watermark_core::{
    AIContentFlags, EmbedOptions, MediaInput, MediaOutput, PayloadV2BuildInput, WatermarkIssueMode,
    WatermarkMediaType, WatermarkPayload, WatermarkService,
};

const SAMPLE_RATE: usize = 44_100;
const SAMPLE_SECONDS: usize = 30;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct FixtureResult {
    generator: &'static str,
    platform: &'static str,
    path: String,
    watermark_uid: String,
    sha256: String,
    bytes: u64,
    payload_protocol_version: u8,
    payload_bytes_length: usize,
    audio_strategy_version: &'static str,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let out_path = PathBuf::from(required_arg(&args, "--out-path")?);
    let watermark_uid = required_arg(&args, "--uid")?;
    let parent = out_path
        .parent()
        .ok_or_else(|| "out path must have a parent directory".to_string())?;
    fs::create_dir_all(parent).map_err(|error| format!("create fixture dir: {error}"))?;

    let source = encode_field_noise_wav()?;
    let payload = payload_from_uid(watermark_uid, &source)?;
    let output = WatermarkService::embed(
        MediaInput::AudioWavBytes { bytes: source },
        &payload,
        EmbedOptions {
            allow_rewrite: true,
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("desktop fixture embed: {error}"))?;
    let MediaOutput::AudioWavBytes { bytes } = output else {
        return Err("desktop fixture returned non-audio output".to_string());
    };
    let decoded = WatermarkService::extract(MediaInput::AudioWavBytes {
        bytes: bytes.clone(),
    })
    .map_err(|error| format!("desktop fixture extract: {error}"))?;
    if decoded.watermark_uid() != payload.watermark_uid()
        || decoded.protocol_version() != 3
        || decoded.payload_bytes_length() != watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES
    {
        return Err("desktop fixture did not roundtrip as V3/39".to_string());
    }
    fs::write(&out_path, &bytes).map_err(|error| format!("write desktop fixture: {error}"))?;

    let result = FixtureResult {
        generator: "src-tauri/src/bin/audio_noise_floor_migration_desktop_fixture.rs",
        platform: "desktop_legacy",
        path: out_path.to_string_lossy().replace('\\', "/"),
        watermark_uid: payload.watermark_uid(),
        sha256: sha256_hex(&bytes),
        bytes: bytes.len() as u64,
        payload_protocol_version: decoded.protocol_version(),
        payload_bytes_length: decoded.payload_bytes_length(),
        audio_strategy_version: "v3_recovery_2_8k_legacy",
    };
    let json = serde_json::to_string_pretty(&result)
        .map_err(|error| format!("serialize result: {error}"))?;
    println!("{json}");
    Ok(())
}

fn payload_from_uid(uid: &str, source: &[u8]) -> Result<WatermarkPayload, String> {
    let watermark_id = parse_uid(uid)?;
    let original_sha256: [u8; 32] = Sha256::digest(source).into();
    WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id,
        parent_watermark_id: None,
        revision: 1,
        issued_at: 1_788_192_000,
        original_sha256,
        ai_flags: AIContentFlags::default(),
        issue_mode: WatermarkIssueMode::OfflineGenerated,
        media_type: WatermarkMediaType::Audio,
        registry_proof_hash: Some(sha256_prefix_16(b"desktop-audio-noise-floor-migration")),
        creator_binding: Some("HiddenShield desktop audio noise-floor migration fixture"),
    })
    .map_err(|error| format!("build payload: {error}"))
}

fn encode_field_noise_wav() -> Result<Vec<u8>, String> {
    let mut cursor = Cursor::new(Vec::new());
    let spec = WavSpec {
        channels: 1,
        sample_rate: SAMPLE_RATE as u32,
        bits_per_sample: 16,
        sample_format: SampleFormat::Int,
    };
    {
        let mut writer =
            WavWriter::new(&mut cursor, spec).map_err(|error| format!("wav: {error}"))?;
        for index in 0..(SAMPLE_RATE * SAMPLE_SECONDS) {
            let t = index as f32 / SAMPLE_RATE as f32;
            let deterministic_noise = (index as u32)
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            let noise = ((deterministic_noise >> 8) & 0xffff) as f32 / 32768.0 - 1.0;
            let sample = 0.11 * noise + 0.08 * (2.0 * std::f32::consts::PI * 130.0 * t).sin();
            writer
                .write_sample((sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16)
                .map_err(|error| format!("write wav sample: {error}"))?;
        }
        writer
            .finalize()
            .map_err(|error| format!("finalize wav: {error}"))?;
    }
    Ok(cursor.into_inner())
}

fn parse_uid(uid: &str) -> Result<[u8; 16], String> {
    let compact = uid
        .trim()
        .strip_prefix("HS-")
        .unwrap_or(uid.trim())
        .replace('-', "");
    if compact.len() != 32 {
        return Err("uid must contain 32 hex chars".to_string());
    }
    let mut out = [0_u8; 16];
    for index in 0..16 {
        out[index] = u8::from_str_radix(&compact[index * 2..index * 2 + 2], 16)
            .map_err(|error| format!("parse uid: {error}"))?;
    }
    Ok(out)
}

fn sha256_prefix_16(bytes: &[u8]) -> [u8; 16] {
    let digest = Sha256::digest(bytes);
    let mut out = [0_u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn required_arg<'a>(args: &'a [String], name: &str) -> Result<&'a str, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| format!("missing required argument {name}"))
}

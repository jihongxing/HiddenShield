use hidden_shield_mobile_bridge::api::{embed_audio_wav_for_mobile, MobileMediaPayload};
use hound::{SampleFormat, WavSpec, WavWriter};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::path::PathBuf;

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
    let result = embed_audio_wav_for_mobile(
        source.clone(),
        MobileMediaPayload {
            creator_identity: "HiddenShield Android audio noise-floor migration fixture"
                .to_string(),
            device_identity: "android-native-legacy-v3-field-noise".to_string(),
            media_bytes: source,
            timestamp: 1_788_192_000,
            reserved_watermark_uid: Some(watermark_uid.to_string()),
            registry_proof_hash: Some(hex::encode(sha256_prefix_16(
                b"android-audio-noise-floor-migration",
            ))),
            parent_watermark_uid: None,
            revision: 1,
            media_type: Some("audio".to_string()),
        },
        true,
    )
    .map_err(|error| format!("android native fixture embed: {error}"))?;
    fs::write(&out_path, &result.bytes)
        .map_err(|error| format!("write android fixture: {error}"))?;

    let output = FixtureResult {
        generator: "mobile_app/rust/src/bin/audio_noise_floor_migration_android_fixture.rs",
        platform: "android_native_legacy",
        path: out_path.to_string_lossy().replace('\\', "/"),
        watermark_uid: result.watermark_uid,
        sha256: result.sha256,
        bytes: result.bytes.len() as u64,
        payload_protocol_version: 3,
        payload_bytes_length: watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
        audio_strategy_version: "v3_recovery_2_8k_legacy",
    };
    let json = serde_json::to_string_pretty(&output)
        .map_err(|error| format!("serialize result: {error}"))?;
    println!("{json}");
    Ok(())
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

fn sha256_prefix_16(bytes: &[u8]) -> [u8; 16] {
    let digest = Sha256::digest(bytes);
    let mut out = [0_u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

fn required_arg<'a>(args: &'a [String], name: &str) -> Result<&'a str, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| format!("missing required argument {name}"))
}

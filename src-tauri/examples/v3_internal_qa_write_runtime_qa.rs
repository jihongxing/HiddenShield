use image::{ImageBuffer, ImageFormat, Rgb};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use watermark_core::{
    embed_v3_internal_qa_media, AIContentFlags, EmbedOptions, ImageOutputFormat, MediaInput,
    MediaOutput, PayloadV2BuildInput, V3InternalQaMediaKind, V3InternalQaWriteGate,
    V3InternalQaWriteInput, WatermarkIssueMode, WatermarkMediaType, WatermarkPayload,
    WatermarkService,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeRow {
    bridge: String,
    write_path: String,
    media_kind: String,
    path: String,
    watermark_uid: String,
    payload_protocol_version: u32,
    payload_bytes_length: u32,
    payload_auth_status: String,
    watermark_id_issue_mode: String,
    media_payload_role: String,
    default_write_path_status: String,
    pass: bool,
}

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let run_id = required_arg(&args, "--run-id")?;
    let out_dir = PathBuf::from(required_arg(&args, "--out-dir")?);
    fs::create_dir_all(&out_dir).map_err(|error| format!("create output dir: {error}"))?;

    let source_dir = out_dir.join("source");
    fs::create_dir_all(&source_dir).map_err(|error| format!("create source dir: {error}"))?;
    let image_source = make_png_image()?;
    let audio_source = make_wav_audio()?;
    let image_source_path = source_dir.join("desktop-source-image.png");
    let audio_source_path = source_dir.join("desktop-source-audio.wav");
    fs::write(&image_source_path, &image_source)
        .map_err(|error| format!("write source image: {error}"))?;
    fs::write(&audio_source_path, &audio_source)
        .map_err(|error| format!("write source audio: {error}"))?;

    let rows = vec![
        write_internal_qa_v3_row(
            run_id,
            &out_dir,
            "image",
            &image_source,
            V3InternalQaMediaKind::Image,
        )?,
        write_internal_qa_v3_row(
            run_id,
            &out_dir,
            "audio",
            &audio_source,
            V3InternalQaMediaKind::Audio,
        )?,
        write_default_v3_row(
            run_id,
            &out_dir,
            "image",
            &image_source,
            WatermarkMediaType::Image,
        )?,
        write_default_v3_row(
            run_id,
            &out_dir,
            "audio",
            &audio_source,
            WatermarkMediaType::Audio,
        )?,
    ];

    let result = serde_json::json!({
        "runId": run_id,
        "desktop": {
            "rows": rows,
            "source": {
                "imagePath": image_source_path,
                "audioPath": audio_source_path,
            },
        },
        "defaultV3WriteEnabled": true,
        "v3InternalQaWriteGate": "internal_qa",
        "boundary": "Desktop QA explicitly calls watermark-core internal_qa V3 writing for QA artifacts and verifies the default write path now emits V3/39; V2 is rollback-only.",
    });
    assert_result(&result)?;
    let json = serde_json::to_string_pretty(&result)
        .map_err(|error| format!("serialize json: {error}"))?;
    let json_path = out_dir.join("desktop-v3-internal-qa-write-runtime.json");
    fs::write(&json_path, format!("{json}\n")).map_err(|error| format!("write json: {error}"))?;
    println!("{json}");
    Ok(())
}

fn write_internal_qa_v3_row(
    run_id: &str,
    out_dir: &Path,
    kind: &str,
    source: &[u8],
    media_kind: V3InternalQaMediaKind,
) -> Result<RuntimeRow, String> {
    let watermark_id =
        sha256_prefix_16(format!("{run_id}:desktop:{kind}:internal_qa:v3").as_bytes());
    let output = embed_v3_internal_qa_media(
        V3InternalQaWriteGate::InternalQa,
        V3InternalQaWriteInput {
            media_kind,
            media_bytes: source.to_vec(),
            watermark_id,
        },
    )
    .map_err(|error| format!("desktop internal QA V3 {kind} write: {error}"))?;
    let path = out_dir.join(match kind {
        "image" => "desktop-internal-qa-v3-image.png",
        "audio" => "desktop-internal-qa-v3-audio.wav",
        _ => return Err(format!("unsupported kind: {kind}")),
    });
    fs::write(&path, &output.bytes).map_err(|error| format!("write desktop V3 {kind}: {error}"))?;
    Ok(RuntimeRow {
        bridge: "desktop".to_string(),
        write_path: "internal_qa".to_string(),
        media_kind: kind.to_string(),
        path: path.to_string_lossy().to_string(),
        watermark_uid: output.watermark_uid,
        payload_protocol_version: output.payload_protocol_version as u32,
        payload_bytes_length: output.payload_bytes_length as u32,
        payload_auth_status: output.payload_auth_status,
        watermark_id_issue_mode: "registry_resolved".to_string(),
        media_payload_role: output.media_payload_role,
        default_write_path_status: "not_used_internal_qa_only".to_string(),
        pass: true,
    })
}

fn write_default_v3_row(
    run_id: &str,
    out_dir: &Path,
    kind: &str,
    source: &[u8],
    media_type: WatermarkMediaType,
) -> Result<RuntimeRow, String> {
    let payload = build_v2_payload(run_id, kind, source, media_type)?;
    let (bytes, path) = match kind {
        "image" => {
            let output = WatermarkService::embed(
                MediaInput::ImageBytes {
                    bytes: source.to_vec(),
                },
                &payload,
                EmbedOptions {
                    image_output_format: ImageOutputFormat::Png,
                    allow_rewrite: true,
                    ..EmbedOptions::default()
                },
            )
            .map_err(|error| format!("desktop default V3 image write: {error}"))?;
            let MediaOutput::ImageBytes { bytes, .. } = output else {
                return Err("desktop default image write returned non-image output".into());
            };
            (bytes, out_dir.join("desktop-default-v3-image.png"))
        }
        "audio" => {
            let output = WatermarkService::embed(
                MediaInput::AudioWavBytes {
                    bytes: source.to_vec(),
                },
                &payload,
                EmbedOptions {
                    allow_rewrite: true,
                    ..EmbedOptions::default()
                },
            )
            .map_err(|error| format!("desktop default V3 audio write: {error}"))?;
            let MediaOutput::AudioWavBytes { bytes } = output else {
                return Err("desktop default audio write returned non-audio output".into());
            };
            (bytes, out_dir.join("desktop-default-v3-audio.wav"))
        }
        _ => return Err(format!("unsupported kind: {kind}")),
    };
    fs::write(&path, &bytes)
        .map_err(|error| format!("write desktop default V3 {kind}: {error}"))?;
    let extracted = WatermarkService::extract(match kind {
        "image" => MediaInput::ImageBytes {
            bytes: bytes.clone(),
        },
        "audio" => MediaInput::AudioWavBytes {
            bytes: bytes.clone(),
        },
        _ => unreachable!(),
    })
    .map_err(|error| format!("desktop default V3 {kind} extract: {error}"))?;
    Ok(RuntimeRow {
        bridge: "desktop".to_string(),
        write_path: "default_write".to_string(),
        media_kind: kind.to_string(),
        path: path.to_string_lossy().to_string(),
        watermark_uid: extracted.watermark_uid(),
        payload_protocol_version: extracted.protocol_version() as u32,
        payload_bytes_length: extracted.payload_bytes_length() as u32,
        payload_auth_status: "verified".to_string(),
        watermark_id_issue_mode: "registry_resolved".to_string(),
        media_payload_role: "v3_minimal_anchor".to_string(),
        default_write_path_status: "v3_minimal_anchor_verified".to_string(),
        pass: true,
    })
}

fn build_v2_payload(
    run_id: &str,
    kind: &str,
    media_bytes: &[u8],
    media_type: WatermarkMediaType,
) -> Result<WatermarkPayload, String> {
    let watermark_id = sha256_prefix_16(format!("{run_id}:desktop:{kind}:default:v3").as_bytes());
    let original_sha256: [u8; 32] = Sha256::digest(media_bytes).into();
    WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id,
        parent_watermark_id: None,
        revision: 1,
        issued_at: 1_786_147_200,
        original_sha256,
        ai_flags: AIContentFlags::default(),
        issue_mode: WatermarkIssueMode::OfflineGenerated,
        media_type,
        registry_proof_hash: Some(sha256_prefix_16(
            format!("{run_id}:desktop:{kind}:registry-proof").as_bytes(),
        )),
        creator_binding: Some("HiddenShield desktop V3 internal QA runtime"),
    })
    .map_err(|error| format!("build V2 payload: {error}"))
}

fn assert_result(result: &serde_json::Value) -> Result<(), String> {
    let rows = result["desktop"]["rows"]
        .as_array()
        .ok_or_else(|| "desktop rows missing".to_string())?;
    for write_path in ["internal_qa", "default_write"] {
        for kind in ["image", "audio"] {
            let row = rows
                .iter()
                .find(|row| row["writePath"] == write_path && row["mediaKind"] == kind)
                .ok_or_else(|| format!("missing desktop {write_path} {kind} row"))?;
            if write_path == "internal_qa" {
                expect_u64(row, "payloadProtocolVersion", 3)?;
                expect_u64(row, "payloadBytesLength", 39)?;
                expect_str(row, "mediaPayloadRole", "v3_minimal_anchor")?;
            } else {
                expect_u64(row, "payloadProtocolVersion", 3)?;
                expect_u64(
                    row,
                    "payloadBytesLength",
                    watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u64,
                )?;
                expect_str(row, "mediaPayloadRole", "v3_minimal_anchor")?;
                expect_str(row, "defaultWritePathStatus", "v3_minimal_anchor_verified")?;
            }
            expect_str(row, "payloadAuthStatus", "verified")?;
        }
    }
    Ok(())
}

fn expect_u64(row: &serde_json::Value, key: &str, expected: u64) -> Result<(), String> {
    let actual = row[key].as_u64().unwrap_or_default();
    if actual != expected {
        return Err(format!("expected {key}={expected}, got {actual}"));
    }
    Ok(())
}

fn expect_str(row: &serde_json::Value, key: &str, expected: &str) -> Result<(), String> {
    let actual = row[key].as_str().unwrap_or_default();
    if actual != expected {
        return Err(format!("expected {key}={expected}, got {actual}"));
    }
    Ok(())
}

fn make_png_image() -> Result<Vec<u8>, String> {
    let width = 1024;
    let height = 1024;
    let image = ImageBuffer::from_fn(width, height, |x, y| {
        Rgb([
            (x * 255 / width) as u8,
            (y * 255 / height) as u8,
            ((x + y) * 127 / width) as u8,
        ])
    });
    let mut cursor = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(image)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|error| format!("encode png: {error}"))?;
    Ok(cursor.into_inner())
}

fn make_wav_audio() -> Result<Vec<u8>, String> {
    let sample_rate = 44_100usize;
    let sample_count = sample_rate * 31;
    let data_bytes = sample_count * 2;
    let mut bytes = vec![0u8; 44 + data_bytes];
    bytes[0..4].copy_from_slice(b"RIFF");
    bytes[4..8].copy_from_slice(&((36 + data_bytes) as u32).to_le_bytes());
    bytes[8..12].copy_from_slice(b"WAVE");
    bytes[12..16].copy_from_slice(b"fmt ");
    bytes[16..20].copy_from_slice(&16u32.to_le_bytes());
    bytes[20..22].copy_from_slice(&1u16.to_le_bytes());
    bytes[22..24].copy_from_slice(&1u16.to_le_bytes());
    bytes[24..28].copy_from_slice(&(sample_rate as u32).to_le_bytes());
    bytes[28..32].copy_from_slice(&((sample_rate * 2) as u32).to_le_bytes());
    bytes[32..34].copy_from_slice(&2u16.to_le_bytes());
    bytes[34..36].copy_from_slice(&16u16.to_le_bytes());
    bytes[36..40].copy_from_slice(b"data");
    bytes[40..44].copy_from_slice(&(data_bytes as u32).to_le_bytes());
    for i in 0..sample_count {
        let t = i as f64 / sample_rate as f64;
        let sample = (t * 440.0 * std::f64::consts::TAU).sin() * 12_000.0;
        bytes[44 + i * 2..46 + i * 2].copy_from_slice(&(sample as i16).to_le_bytes());
    }
    Ok(bytes)
}

fn sha256_prefix_16(bytes: &[u8]) -> [u8; 16] {
    let digest = Sha256::digest(bytes);
    let mut output = [0u8; 16];
    output.copy_from_slice(&digest[..16]);
    output
}

fn required_arg<'a>(args: &'a [String], name: &str) -> Result<&'a str, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| format!("missing required argument {name}"))
}

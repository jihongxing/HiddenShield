use image::{ImageBuffer, ImageFormat, Rgb};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use watermark_core::{
    embed_v3_internal_qa_media, extract_image_watermark_readonly_candidate_bytes,
    extract_watermark_wav_readonly_candidate_bytes, AIContentFlags, EmbedOptions,
    ImageOutputFormat, MediaInput, MediaOutput, PayloadV2BuildInput, V3InternalQaMediaKind,
    V3InternalQaWriteGate, V3InternalQaWriteInput, WatermarkDecodedPayload, WatermarkIssueMode,
    WatermarkMediaType, WatermarkPayload, WatermarkService, PAYLOAD_BYTES,
};

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let run_id = required_arg(&args, "--run-id")?;
    let out_dir = PathBuf::from(required_arg(&args, "--out-dir")?);
    fs::create_dir_all(&out_dir).map_err(|error| format!("create output dir: {error}"))?;

    let image_source = make_png_image()?;
    let audio_source = make_wav_audio()?;
    let image_id = sha256_prefix_16(format!("{run_id}:image:v3-gate").as_bytes());
    let audio_id = sha256_prefix_16(format!("{run_id}:audio:v3-gate").as_bytes());

    let rows = vec![
        write_default_v3_row(
            run_id,
            &out_dir,
            "off",
            "image",
            &image_source,
            WatermarkMediaType::Image,
        )?,
        write_default_v3_row(
            run_id,
            &out_dir,
            "off",
            "audio",
            &audio_source,
            WatermarkMediaType::Audio,
        )?,
        write_v3_internal_qa_row(
            &out_dir,
            "internal_qa",
            "image",
            &image_source,
            image_id,
            V3InternalQaMediaKind::Image,
        )?,
        write_v3_internal_qa_row(
            &out_dir,
            "internal_qa",
            "audio",
            &audio_source,
            audio_id,
            V3InternalQaMediaKind::Audio,
        )?,
        write_v2_row(
            run_id,
            &out_dir,
            "force_v2_rollback",
            "image",
            &image_source,
            WatermarkMediaType::Image,
        )?,
        write_v2_row(
            run_id,
            &out_dir,
            "force_v2_rollback",
            "audio",
            &audio_source,
            WatermarkMediaType::Audio,
        )?,
    ];

    let json = format!(
        "{{\n  \"runId\": \"{}\",\n  \"defaultV3WriteEnabled\": true,\n  \"v3InternalQaWriteImplemented\": true,\n  \"rows\": [\n{}\n  ]\n}}\n",
        json_escape(run_id),
        rows.iter()
            .map(|row| format!("    {}", row.json))
            .collect::<Vec<_>>()
            .join(",\n"),
    );
    let json_path = out_dir.join("v3-feature-gate-rollback-matrix.json");
    fs::write(&json_path, &json).map_err(|error| format!("write matrix json: {error}"))?;
    print!("{json}");
    Ok(())
}

struct MatrixRow {
    json: String,
}

fn write_default_v3_row(
    run_id: &str,
    out_dir: &Path,
    gate: &str,
    kind: &str,
    source: &[u8],
    media_type: WatermarkMediaType,
) -> Result<MatrixRow, String> {
    let payload = build_v2_payload(run_id, gate, kind, source, media_type)?;
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
            .map_err(|error| format!("embed default V3 image: {error}"))?;
            let MediaOutput::ImageBytes { bytes, .. } = output else {
                return Err("default V3 image embed returned non-image output".into());
            };
            let path = out_dir.join(format!("{gate}-default-v3-image.png"));
            (bytes, path)
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
            .map_err(|error| format!("embed default V3 audio: {error}"))?;
            let MediaOutput::AudioWavBytes { bytes } = output else {
                return Err("default V3 audio embed returned non-audio output".into());
            };
            let path = out_dir.join(format!("{gate}-default-v3-audio.wav"));
            (bytes, path)
        }
        _ => return Err(format!("unsupported kind: {kind}")),
    };
    fs::write(&path, &bytes).map_err(|error| format!("write default V3 {kind}: {error}"))?;
    let extracted = WatermarkService::extract(match kind {
        "image" => MediaInput::ImageBytes {
            bytes: bytes.clone(),
        },
        "audio" => MediaInput::AudioWavBytes {
            bytes: bytes.clone(),
        },
        _ => unreachable!(),
    })
    .map_err(|error| format!("extract default V3 {kind}: {error}"))?;
    let WatermarkDecodedPayload::V3MinimalAnchor(anchor) = extracted else {
        return Err(format!("expected default V3 minimal anchor for {kind}"));
    };
    Ok(MatrixRow {
        json: row_json(
            gate,
            kind,
            &path,
            &anchor.watermark_uid(),
            anchor.protocol_version,
            watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
            "verified",
            "v3_minimal_anchor",
        ),
    })
}

fn write_v2_row(
    run_id: &str,
    out_dir: &Path,
    gate: &str,
    kind: &str,
    source: &[u8],
    media_type: WatermarkMediaType,
) -> Result<MatrixRow, String> {
    let payload = build_v2_payload(run_id, gate, kind, source, media_type)?;
    if kind == "image" {
        let error = WatermarkService::embed_v2(
            MediaInput::ImageBytes {
                bytes: source.to_vec(),
            },
            &payload,
            EmbedOptions {
                image_output_format: ImageOutputFormat::Png,
                allow_rewrite: true,
                payload_write_mode: watermark_core::PayloadWriteMode::ForceV2Rollback,
                ..EmbedOptions::default()
            },
        )
        .expect_err("V2 image rollback must be retired");
        let message = error.to_string();
        if !message.contains("v2_image_rollback_retired") {
            return Err(format!("unexpected V2 image retirement error: {message}"));
        }
        return Ok(MatrixRow {
            json: retired_v2_image_row_json(gate, &message),
        });
    }

    let (bytes, path) = match kind {
        "audio" => {
            let output = WatermarkService::embed_v2(
                MediaInput::AudioWavBytes {
                    bytes: source.to_vec(),
                },
                &payload,
                EmbedOptions {
                    allow_rewrite: true,
                    payload_write_mode: watermark_core::PayloadWriteMode::ForceV2Rollback,
                    ..EmbedOptions::default()
                },
            )
            .map_err(|error| format!("embed V2 audio: {error}"))?;
            let MediaOutput::AudioWavBytes { bytes } = output else {
                return Err("V2 audio embed returned non-audio output".into());
            };
            let path = out_dir.join(format!("{gate}-v2-audio.wav"));
            (bytes, path)
        }
        _ => return Err(format!("unsupported kind: {kind}")),
    };
    fs::write(&path, &bytes).map_err(|error| format!("write V2 {kind}: {error}"))?;
    let extracted = WatermarkService::extract_v2(match kind {
        "audio" => MediaInput::AudioWavBytes {
            bytes: bytes.clone(),
        },
        _ => unreachable!(),
    })
    .map_err(|error| format!("extract V2 {kind}: {error}"))?;
    Ok(MatrixRow {
        json: row_json(
            gate,
            kind,
            &path,
            &extracted.watermark_uid(),
            extracted.protocol_version,
            PAYLOAD_BYTES,
            "verified",
            "v2_full_record",
        ),
    })
}

fn write_v3_internal_qa_row(
    out_dir: &Path,
    gate: &str,
    kind: &str,
    source: &[u8],
    watermark_id: [u8; 16],
    media_kind: V3InternalQaMediaKind,
) -> Result<MatrixRow, String> {
    let output = embed_v3_internal_qa_media(
        V3InternalQaWriteGate::InternalQa,
        V3InternalQaWriteInput {
            media_kind,
            media_bytes: source.to_vec(),
            watermark_id,
        },
    )
    .map_err(|error| format!("embed V3 internal QA {kind}: {error}"))?;
    let path = out_dir.join(match kind {
        "image" => "internal_qa-v3-image.png",
        "audio" => "internal_qa-v3-audio.wav",
        _ => return Err(format!("unsupported kind: {kind}")),
    });
    fs::write(&path, &output.bytes).map_err(|error| format!("write V3 {kind}: {error}"))?;
    let decoded = match kind {
        "image" => extract_image_watermark_readonly_candidate_bytes(&output.bytes),
        "audio" => extract_watermark_wav_readonly_candidate_bytes(&output.bytes),
        _ => unreachable!(),
    }
    .map_err(|error| format!("readonly extract V3 {kind}: {error}"))?;
    let WatermarkDecodedPayload::V3MinimalAnchor(_) = decoded else {
        return Err(format!("expected V3 minimal anchor for {kind}"));
    };
    Ok(MatrixRow {
        json: row_json(
            gate,
            kind,
            &path,
            &output.watermark_uid,
            output.payload_protocol_version,
            output.payload_bytes_length,
            &output.payload_auth_status,
            &output.media_payload_role,
        ),
    })
}

fn retired_v2_image_row_json(gate: &str, error: &str) -> String {
    format!(
        "{{\"gate\":\"{}\",\"kind\":\"image\",\"path\":null,\"watermarkUid\":null,\"payloadProtocolVersion\":null,\"payloadBytesLength\":null,\"payloadAuthStatus\":\"not_applicable\",\"mediaPayloadRole\":\"retired_v2_image_rollback\",\"expectedOutcome\":\"rejected\",\"reasonCode\":\"v2_image_rollback_retired\",\"error\":\"{}\",\"pass\":true}}",
        json_escape(gate),
        json_escape(error),
    )
}

fn row_json(
    gate: &str,
    kind: &str,
    path: &Path,
    watermark_uid: &str,
    payload_protocol_version: u8,
    payload_bytes_length: usize,
    payload_auth_status: &str,
    media_payload_role: &str,
) -> String {
    format!(
        "{{\"gate\":\"{}\",\"kind\":\"{}\",\"path\":\"{}\",\"watermarkUid\":\"{}\",\"payloadProtocolVersion\":{},\"payloadBytesLength\":{},\"payloadAuthStatus\":\"{}\",\"mediaPayloadRole\":\"{}\",\"pass\":true}}",
        json_escape(gate),
        json_escape(kind),
        json_escape(&path.display().to_string()),
        json_escape(watermark_uid),
        payload_protocol_version,
        payload_bytes_length,
        json_escape(payload_auth_status),
        json_escape(media_payload_role),
    )
}

fn build_v2_payload(
    run_id: &str,
    gate: &str,
    kind: &str,
    media_bytes: &[u8],
    media_type: WatermarkMediaType,
) -> Result<WatermarkPayload, String> {
    let watermark_id = sha256_prefix_16(format!("{run_id}:{gate}:{kind}:v2").as_bytes());
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
            format!("{run_id}:{gate}:{kind}:registry-proof").as_bytes(),
        )),
        creator_binding: Some("HiddenShield V3 feature gate rollback QA"),
    })
    .map_err(|error| format!("build V2 payload: {error}"))
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

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn required_arg<'a>(args: &'a [String], name: &str) -> Result<&'a str, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| format!("missing required argument {name}"))
}

use image::{ImageBuffer, ImageFormat, Rgb};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
use watermark_core::{
    AIContentFlags, EmbedOptions, ImageOutputFormat, MediaInput, MediaOutput, PayloadV2BuildInput,
    WatermarkIssueMode, WatermarkMediaType, WatermarkPayload, WatermarkService,
};

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let command = args.next().ok_or_else(usage)?;
    let rest: Vec<String> = args.collect();
    match command.as_str() {
        "generate-desktop" => generate_desktop(&rest),
        "generate-image" => generate_image(&rest),
        "verify-file" => verify_file(&rest),
        _ => Err(usage()),
    }
}

fn generate_image(args: &[String]) -> Result<(), String> {
    let run_id = required_arg(args, "--run-id")?;
    let output_path = PathBuf::from(required_arg(args, "--output")?);
    let watermark_uid = required_arg(args, "--watermark-uid")?;
    let format = required_arg(args, "--format")?.to_ascii_lowercase();
    let media_type = WatermarkMediaType::Image;
    let image_source = make_png_image()?;
    let watermark_id = parse_watermark_uid(watermark_uid)?;
    let original_sha256: [u8; 32] = Sha256::digest(&image_source).into();
    let payload = WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id,
        parent_watermark_id: None,
        revision: 1,
        issued_at: 1_786_147_200,
        original_sha256,
        ai_flags: AIContentFlags::default(),
        issue_mode: WatermarkIssueMode::ServerConfirmed,
        media_type,
        registry_proof_hash: Some(sha256_prefix_16(
            format!("{run_id}:{watermark_uid}:registry-proof").as_bytes(),
        )),
        creator_binding: Some("HiddenShield public metadata embed QA"),
    })
    .map_err(|error| format!("build payload: {error}"))?;
    let output_format = match format.as_str() {
        "png" => ImageOutputFormat::Png,
        "jpeg" | "jpg" => ImageOutputFormat::Jpeg,
        _ => return Err("format must be png or jpeg".to_string()),
    };
    let image_output = WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: image_source,
        },
        &payload,
        EmbedOptions {
            image_output_format: output_format,
            allow_rewrite: true,
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("embed image: {error}"))?;
    let MediaOutput::ImageBytes { bytes, .. } = image_output else {
        return Err("image embed returned non-image output".into());
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create output dir: {error}"))?;
    }
    fs::write(&output_path, &bytes).map_err(|error| format!("write image output: {error}"))?;
    let verify = extract_file("image", &output_path)?;
    if verify.watermark_uid != watermark_uid {
        return Err(format!(
            "generated image self-check UID mismatch: expected {watermark_uid}, got {}",
            verify.watermark_uid
        ));
    }
    print!(
        "{}",
        format!(
            concat!(
                "{{\n",
                "  \"runId\": \"{}\",\n",
                "  \"path\": \"{}\",\n",
                "  \"format\": \"{}\",\n",
                "  \"sha256\": \"{}\",\n",
                "  \"watermarkUid\": \"{}\",\n",
                "  \"payloadProtocolVersion\": {},\n",
                "  \"payloadBytesLength\": {}\n",
                "}}\n"
            ),
            json_escape(run_id),
            json_escape(&output_path.display().to_string()),
            json_escape(if matches!(format.as_str(), "jpg") {
                "jpeg"
            } else {
                &format
            }),
            sha256_hex(&bytes),
            json_escape(&verify.watermark_uid),
            verify.payload_protocol_version,
            verify.payload_bytes_length,
        )
    );
    Ok(())
}

fn generate_desktop(args: &[String]) -> Result<(), String> {
    let run_id = required_arg(args, "--run-id")?;
    let out_dir = PathBuf::from(required_arg(args, "--out-dir")?);
    fs::create_dir_all(&out_dir).map_err(|error| format!("create output dir: {error}"))?;

    let image_source = make_png_image()?;
    let audio_source = make_wav_audio(31)?;
    let image_payload = build_payload(
        run_id,
        "desktop-image",
        &image_source,
        WatermarkMediaType::Image,
    )?;
    let audio_payload = build_payload(
        run_id,
        "desktop-audio",
        &audio_source,
        WatermarkMediaType::Audio,
    )?;

    let image_output = WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: image_source,
        },
        &image_payload,
        EmbedOptions {
            image_output_format: ImageOutputFormat::Png,
            allow_rewrite: true,
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("embed desktop image: {error}"))?;
    let MediaOutput::ImageBytes {
        bytes: image_bytes, ..
    } = image_output
    else {
        return Err("desktop image embed returned non-image output".into());
    };

    let audio_output = WatermarkService::embed(
        MediaInput::AudioWavBytes {
            bytes: audio_source,
        },
        &audio_payload,
        EmbedOptions {
            allow_rewrite: true,
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("embed desktop audio: {error}"))?;
    let MediaOutput::AudioWavBytes { bytes: audio_bytes } = audio_output else {
        return Err("desktop audio embed returned non-audio output".into());
    };

    let image_path = out_dir.join(format!("desktop-protected-image-{run_id}.png"));
    let audio_path = out_dir.join(format!("desktop-protected-audio-{run_id}.wav"));
    fs::write(&image_path, &image_bytes).map_err(|error| format!("write image output: {error}"))?;
    fs::write(&audio_path, &audio_bytes).map_err(|error| format!("write audio output: {error}"))?;

    let image_verify = extract_file("image", &image_path)?;
    let audio_verify = extract_file("audio", &audio_path)?;
    if image_verify.watermark_uid != image_payload.watermark_uid() {
        return Err("desktop image self-check UID mismatch".into());
    }
    if audio_verify.watermark_uid != audio_payload.watermark_uid() {
        return Err("desktop audio self-check UID mismatch".into());
    }

    let json = format!(
        concat!(
            "{{\n",
            "  \"runId\": \"{}\",\n",
            "  \"desktop\": {{\n",
            "    \"image\": {},\n",
            "    \"audio\": {}\n",
            "  }}\n",
            "}}\n"
        ),
        json_escape(run_id),
        artifact_json("image", &image_path, &image_bytes, &image_verify),
        artifact_json("audio", &audio_path, &audio_bytes, &audio_verify),
    );
    fs::write(out_dir.join("desktop-artifacts.json"), json)
        .map_err(|error| format!("write desktop artifacts json: {error}"))?;
    Ok(())
}

fn verify_file(args: &[String]) -> Result<(), String> {
    let kind = required_arg(args, "--kind")?;
    let path = PathBuf::from(required_arg(args, "--path")?);
    let expected_uid = required_arg(args, "--expected-uid")?;
    let json_out = args
        .windows(2)
        .find(|pair| pair[0] == "--json-out")
        .map(|pair| PathBuf::from(&pair[1]));
    let extracted = extract_file(kind, &path)?;
    let pass = extracted.watermark_uid == expected_uid;
    let bytes = fs::read(&path).map_err(|error| format!("read verify file: {error}"))?;
    let json = format!(
        concat!(
            "{{\n",
            "  \"kind\": \"{}\",\n",
            "  \"path\": \"{}\",\n",
            "  \"sha256\": \"{}\",\n",
            "  \"expectedWatermarkUid\": \"{}\",\n",
            "  \"extractedWatermarkUid\": \"{}\",\n",
            "  \"payloadProtocolVersion\": {},\n",
            "  \"payloadBytesLength\": {},\n",
            "  \"pass\": {}\n",
            "}}\n"
        ),
        json_escape(kind),
        json_escape(&path.display().to_string()),
        sha256_hex(&bytes),
        json_escape(expected_uid),
        json_escape(&extracted.watermark_uid),
        extracted.payload_protocol_version,
        extracted.payload_bytes_length,
        if pass { "true" } else { "false" },
    );
    if let Some(json_out) = json_out {
        fs::write(json_out, json).map_err(|error| format!("write verify json: {error}"))?;
    } else {
        print!("{json}");
    }
    if pass {
        Ok(())
    } else {
        Err(format!(
            "{kind} UID mismatch: expected {expected_uid}, got {}",
            extracted.watermark_uid
        ))
    }
}

fn extract_file(kind: &str, path: &Path) -> Result<Extracted, String> {
    let bytes = fs::read(path).map_err(|error| format!("read '{}': {error}", path.display()))?;
    let input = match kind {
        "image" => MediaInput::ImageBytes { bytes },
        "audio" => MediaInput::AudioWavBytes { bytes },
        _ => return Err(format!("unsupported kind: {kind}")),
    };
    let payload = WatermarkService::extract(input).map_err(|error| {
        format!(
            "extract {kind} watermark from '{}': {error}",
            path.display()
        )
    })?;
    Ok(Extracted {
        watermark_uid: payload.watermark_uid(),
        payload_protocol_version: payload.protocol_version(),
        payload_bytes_length: payload.payload_bytes_length(),
    })
}

fn build_payload(
    run_id: &str,
    label: &str,
    media_bytes: &[u8],
    media_type: WatermarkMediaType,
) -> Result<WatermarkPayload, String> {
    let watermark_id = sha256_prefix_16(format!("{run_id}:{label}:watermark-id").as_bytes());
    let original_sha256: [u8; 32] = Sha256::digest(media_bytes).into();
    let registry_proof_hash =
        sha256_prefix_16(format!("{run_id}:{label}:registry-proof").as_bytes());
    WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id,
        parent_watermark_id: None,
        revision: 1,
        issued_at: 1_786_147_200,
        original_sha256,
        ai_flags: AIContentFlags::default(),
        issue_mode: WatermarkIssueMode::OfflineGenerated,
        media_type,
        registry_proof_hash: Some(registry_proof_hash),
        creator_binding: Some("HiddenShield file-flow desktop QA"),
    })
    .map_err(|error| format!("build payload: {error}"))
}

fn make_png_image() -> Result<Vec<u8>, String> {
    let width = 512;
    let height = 512;
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

fn make_wav_audio(seconds: usize) -> Result<Vec<u8>, String> {
    let sample_rate = 44_100usize;
    let sample_count = sample_rate * seconds;
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
        let sample = ((i as f64 * 440.0 * std::f64::consts::TAU / sample_rate as f64).sin()
            * 12_000.0) as i16;
        bytes[44 + i * 2..46 + i * 2].copy_from_slice(&sample.to_le_bytes());
    }
    Ok(bytes)
}

fn artifact_json(kind: &str, path: &Path, bytes: &[u8], extracted: &Extracted) -> String {
    format!(
        concat!(
            "{{\n",
            "      \"kind\": \"{}\",\n",
            "      \"path\": \"{}\",\n",
            "      \"sha256\": \"{}\",\n",
            "      \"watermarkUid\": \"{}\",\n",
            "      \"payloadProtocolVersion\": {},\n",
            "      \"payloadBytesLength\": {}\n",
            "    }}"
        ),
        json_escape(kind),
        json_escape(&path.display().to_string()),
        sha256_hex(bytes),
        json_escape(&extracted.watermark_uid),
        extracted.payload_protocol_version,
        extracted.payload_bytes_length,
    )
}

fn required_arg<'a>(args: &'a [String], name: &str) -> Result<&'a str, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| format!("missing required argument {name}\n{}", usage()))
}

fn usage() -> String {
    "Usage: protected_copy_file_flow_qa generate-desktop --run-id <id> --out-dir <dir>\n       protected_copy_file_flow_qa generate-image --run-id <id> --watermark-uid <uid> --format <png|jpeg> --output <file>\n       protected_copy_file_flow_qa verify-file --kind <image|audio> --path <file> --expected-uid <uid> [--json-out <file>]".into()
}

fn parse_watermark_uid(uid: &str) -> Result<[u8; 16], String> {
    let hex = uid
        .trim()
        .strip_prefix("HS-")
        .ok_or_else(|| "watermark uid must start with HS-".to_string())?
        .replace('-', "");
    if hex.len() != 32 {
        return Err("watermark uid must contain 16 bytes".to_string());
    }
    let mut out = [0u8; 16];
    for (index, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let text = std::str::from_utf8(chunk).map_err(|error| format!("invalid uid: {error}"))?;
        out[index] = u8::from_str_radix(text, 16)
            .map_err(|error| format!("invalid uid hex byte '{text}': {error}"))?;
    }
    Ok(out)
}

fn sha256_prefix_16(bytes: &[u8]) -> [u8; 16] {
    let digest = Sha256::digest(bytes);
    let mut out = [0u8; 16];
    out.copy_from_slice(&digest[..16]);
    out
}

fn sha256_hex(bytes: &[u8]) -> String {
    hex_lower(&Sha256::digest(bytes))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

fn json_escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}

struct Extracted {
    watermark_uid: String,
    payload_protocol_version: u8,
    payload_bytes_length: usize,
}

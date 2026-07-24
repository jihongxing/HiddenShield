use std::env;
use std::fs;
use std::path::PathBuf;

use sha2::{Digest, Sha256};
use watermark_core::{
    AIContentFlags, EmbedOptions, ImageOutputFormat, MediaInput, MediaOutput, PayloadV2BuildInput,
    WatermarkIssueMode, WatermarkMediaType, WatermarkPayload, WatermarkService,
};

fn main() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let input_path = PathBuf::from(
        args.next()
            .ok_or_else(|| "usage: desktop_image_write_qa <input> <output>".to_string())?,
    );
    let output_path = PathBuf::from(args.next().ok_or_else(|| {
        "usage: desktop_image_write_qa <input> <output> [watermark-uid]".to_string()
    })?);
    let requested_uid = args.next();
    let source =
        fs::read(&input_path).map_err(|error| format!("read {}: {error}", input_path.display()))?;
    let source_sha256: [u8; 32] = Sha256::digest(&source).into();
    let watermark_id = requested_uid
        .as_deref()
        .map(parse_watermark_uid)
        .transpose()?
        .unwrap_or_else(|| source_sha256[..16].try_into().unwrap());
    let payload = WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id,
        parent_watermark_id: None,
        revision: 1,
        issued_at: 1_786_147_200,
        original_sha256: source_sha256,
        ai_flags: AIContentFlags::default(),
        issue_mode: WatermarkIssueMode::OfflineGenerated,
        media_type: WatermarkMediaType::Image,
        registry_proof_hash: None,
        creator_binding: Some("HiddenShield desktop image write QA"),
    })
    .map_err(|error| format!("build payload: {error}"))?;
    let output = WatermarkService::embed(
        MediaInput::ImageBytes { bytes: source },
        &payload,
        EmbedOptions {
            image_output_format: ImageOutputFormat::Png,
            allow_rewrite: true,
            ..EmbedOptions::default()
        },
    )
    .map_err(|error| format!("embed {}: {error}", input_path.display()))?;
    let MediaOutput::ImageBytes { bytes, .. } = output else {
        return Err("image write returned non-image output".into());
    };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    fs::write(&output_path, &bytes)
        .map_err(|error| format!("write {}: {error}", output_path.display()))?;
    let decoded = WatermarkService::extract(MediaInput::ImageBytes { bytes })
        .map_err(|error| format!("self-check {}: {error}", output_path.display()))?;
    println!(
        "{{\"path\":{},\"watermarkUid\":{},\"payloadProtocolVersion\":{}}}",
        serde_json::to_string(&output_path.display().to_string()).unwrap(),
        serde_json::to_string(&decoded.watermark_uid()).unwrap(),
        decoded.protocol_version()
    );
    Ok(())
}

fn parse_watermark_uid(value: &str) -> Result<[u8; 16], String> {
    let compact = value.strip_prefix("HS-").unwrap_or(value).replace('-', "");
    if compact.len() != 32 {
        return Err(format!(
            "invalid watermark UID {value}: expected 32 hexadecimal characters"
        ));
    }

    let mut decoded = [0_u8; 16];
    for (index, byte) in decoded.iter_mut().enumerate() {
        let start = index * 2;
        *byte = u8::from_str_radix(&compact[start..start + 2], 16)
            .map_err(|_| format!("invalid watermark UID {value}: expected hexadecimal digits"))?;
    }
    Ok(decoded)
}

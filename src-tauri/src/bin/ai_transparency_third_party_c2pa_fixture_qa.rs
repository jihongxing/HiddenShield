use std::{fs::File, io::Cursor, path::PathBuf};

use c2pa::{Builder, EphemeralSigner, Reader};
use serde_json::json;
use sha2::{Digest, Sha256};
use watermark_core::{
    watermark_id_from_uid, AIContentFlags, EmbedOptions, ImageOutputFormat, MediaInput,
    MediaOutput, PayloadV2BuildInput, WatermarkIssueMode, WatermarkMediaType, WatermarkPayload,
    WatermarkService,
};

const FIXTURE: &str =
    "../docs/fixtures/ai-transparency-third-party-c2pa-v1/contentauth-c2pa-fixtures-C.jpg";
const EXPECTED_SHA256: &str = "cf250bee1d27d12281ac11a4cc407ffeb9392de25f04edcb3ab2318c38f3d7e4";
const VISUAL_FIXTURE: &str =
    "../docs/fixtures/ai-transparency-third-party-visual-watermark-v1/watermarkreco-synthetic.jpg";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE);
    let bytes = std::fs::read(&path)?;
    let digest = format!("{:x}", Sha256::digest(&bytes));
    if digest != EXPECTED_SHA256 {
        return Err(format!("fixture SHA-256 mismatch: {digest}").into());
    }

    let reader = Reader::default().with_stream("image/jpeg", File::open(&path)?)?;
    if reader.active_manifest().is_none() {
        return Err("expected third-party C2PA active manifest".into());
    }

    if WatermarkService::extract(MediaInput::ImageBytes { bytes }).is_ok() {
        return Err(
            "third-party C2PA fixture must not be classified as a HiddenShield V3 anchor".into(),
        );
    }

    let visual_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(VISUAL_FIXTURE);
    let output_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../tmp-ui-qa/ai-transparency-synthetic-three-layer-v1");
    std::fs::create_dir_all(&output_dir)?;
    let c2pa_path = output_dir.join("visual-watermark-self-signed-c2pa.jpg");
    let mut builder = Builder::default().with_definition(
        json!({
            "title": "Internal synthetic three-layer fixture",
            "format": "image/jpeg",
            "claim_generator_info": [{"name": "HiddenShield internal QA", "version": "1"}]
        })
        .to_string(),
    )?;
    let signer = EphemeralSigner::new("hiddenshield-internal-qa.local")?;
    let mut visual_source = File::open(&visual_path)?;
    let mut c2pa_output = File::create(&c2pa_path)?;
    builder.sign(&signer, "image/jpeg", &mut visual_source, &mut c2pa_output)?;
    if Reader::default()
        .with_stream("image/jpeg", File::open(&c2pa_path)?)?
        .active_manifest()
        .is_none()
    {
        return Err("synthetic C2PA intermediate has no active manifest".into());
    }

    let visual_bytes = std::fs::read(&c2pa_path)?;
    let payload = WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id: watermark_id_from_uid("HS-ABCDEF01-23456789-ABCDEF01-23456789")?,
        parent_watermark_id: None,
        revision: 1,
        issued_at: 1_786_147_200,
        original_sha256: Sha256::digest(&visual_bytes).into(),
        ai_flags: AIContentFlags::default(),
        issue_mode: WatermarkIssueMode::OfflineGenerated,
        media_type: WatermarkMediaType::Image,
        registry_proof_hash: None,
        creator_binding: Some("HiddenShield synthetic three-layer QA"),
    })?;
    let MediaOutput::ImageBytes { bytes, .. } = WatermarkService::embed(
        MediaInput::ImageBytes {
            bytes: visual_bytes,
        },
        &payload,
        EmbedOptions {
            image_output_format: ImageOutputFormat::Png,
            allow_rewrite: false,
            ..EmbedOptions::default()
        },
    )?
    else {
        return Err("synthetic three-layer write returned non-image output".into());
    };
    let final_png_c2pa_status = classify_c2pa_png(&bytes);
    let unsigned_png_path = output_dir.join("visual-watermark-v3-unsigned.png");
    std::fs::write(&unsigned_png_path, &bytes)?;
    let decoded = WatermarkService::extract(MediaInput::ImageBytes {
        bytes: bytes.clone(),
    })?;
    if !decoded.is_v3_minimal_anchor() {
        return Err("synthetic three-layer output has no V3 anchor".into());
    }
    if final_png_c2pa_status != "manifest_absent_after_png_reencode" {
        return Err(
            format!("unexpected final PNG C2PA classification: {final_png_c2pa_status}").into(),
        );
    }

    let post_embed_signed_path = output_dir.join("visual-watermark-v3-post-embed-c2pa.png");
    let mut post_embed_builder = Builder::default().with_definition(
        json!({
            "title": "HiddenShield internal post-embed C2PA fixture",
            "format": "image/png",
            "claim_generator_info": [{"name": "HiddenShield internal QA", "version": "1"}]
        })
        .to_string(),
    )?;
    let post_embed_signer = EphemeralSigner::new("hiddenshield-post-embed-qa.local")?;
    {
        let mut unsigned_png = File::open(&unsigned_png_path)?;
        let mut signed_png = File::create(&post_embed_signed_path)?;
        post_embed_builder.sign(
            &post_embed_signer,
            "image/png",
            &mut unsigned_png,
            &mut signed_png,
        )?;
    }
    let post_embed_signed_bytes = std::fs::read(&post_embed_signed_path)?;
    let post_embed_c2pa_status = classify_c2pa_png(&post_embed_signed_bytes);
    if !post_embed_c2pa_status.starts_with("manifest_present") {
        return Err(format!(
            "post-embed signed PNG C2PA is not readable: {post_embed_c2pa_status}"
        )
        .into());
    }
    let post_embed_decoded = WatermarkService::extract(MediaInput::ImageBytes {
        bytes: post_embed_signed_bytes,
    })?;
    if !post_embed_decoded.is_v3_minimal_anchor()
        || post_embed_decoded.watermark_uid() != decoded.watermark_uid()
        || post_embed_decoded.payload_auth_status() != "verified"
    {
        return Err("post-embed C2PA signing did not preserve the verified V3 anchor".into());
    }

    println!(
        "{{\"fixture\":\"contentauth-c2pa-fixtures-C\",\"c2paManifestPresent\":true,\"hiddenShieldV3AnchorPresent\":false,\"syntheticThreeLayerIntermediateC2paManifestPresent\":true,\"syntheticThreeLayerOutputV3AnchorPresent\":true,\"finalPngC2paStatus\":\"{final_png_c2pa_status}\",\"postEmbedC2paStatus\":\"{post_embed_c2pa_status}\",\"postEmbedV3AnchorPresent\":true,\"postEmbedV3AuthStatus\":\"verified\",\"outputContainerGate\":\"internal_post_embed_resign_verified_nonproduction\",\"externalPlatformAcceptanceAuthorized\":false,\"legalConclusion\":false}}"
    );
    Ok(())
}

fn classify_c2pa_png(bytes: &[u8]) -> String {
    match Reader::default().with_stream("image/png", Cursor::new(bytes)) {
        Ok(reader) if reader.active_manifest().is_some() => {
            if reader
                .validation_status()
                .is_some_and(|status| !status.is_empty())
            {
                "manifest_present_with_validation_findings".to_string()
            } else {
                "manifest_present_and_readable".to_string()
            }
        }
        Ok(_) => "manifest_absent_after_png_reencode".to_string(),
        Err(error) => {
            let message = error.to_string().to_ascii_lowercase();
            if message.contains("manifest")
                || message.contains("jumbf")
                || message.contains("not found")
            {
                "manifest_absent_after_png_reencode".to_string()
            } else {
                format!("reader_error:{message}")
            }
        }
    }
}

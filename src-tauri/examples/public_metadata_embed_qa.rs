use std::env;
use std::fs;
use std::path::PathBuf;

use hidden_shield_lib::commands::public_metadata::{
    build_public_metadata_json_packet, build_xmp_packet, c2pa_format_for_public_metadata_format,
    embed_c2pa_signed_manifest, embed_jpeg_xmp, embed_mp4_public_metadata, embed_png_xmp,
    embed_wav_public_metadata, verify_c2pa_active_manifest,
};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn main() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let source_path = PathBuf::from(required_arg(&args, "--source")?);
    let metadata_path = PathBuf::from(required_arg(&args, "--metadata")?);
    let output_path = PathBuf::from(required_arg(&args, "--output")?);
    let format = required_arg(&args, "--format")?.to_ascii_lowercase();
    let json_out = args
        .windows(2)
        .find(|pair| pair[0] == "--json-out")
        .map(|pair| PathBuf::from(&pair[1]));

    let source_bytes = fs::read(&source_path).map_err(|error| format!("read source: {error}"))?;
    let metadata_text =
        fs::read_to_string(&metadata_path).map_err(|error| format!("read metadata: {error}"))?;
    let metadata: Value =
        serde_json::from_str(&metadata_text).map_err(|error| format!("parse metadata: {error}"))?;
    let packet = build_xmp_packet(&metadata)?;
    let json_packet = build_public_metadata_json_packet(&metadata)?;
    let normalized_format = if format == "jpg" {
        "jpeg"
    } else {
        format.as_str()
    };
    let (output_bytes, c2pa_manifest_hash, c2pa_signer_status, propagation_layer) =
        match format.as_str() {
            "png" => {
                let xmp_bytes = embed_png_xmp(&source_bytes, &packet)?;
                let signed = embed_c2pa_signed_manifest(&xmp_bytes, "image/png", &metadata)?;
                (
                    signed.bytes,
                    Some(signed.manifest_hash),
                    Some(signed.signer_status),
                    "xmp_itxt",
                )
            }
            "jpeg" | "jpg" => {
                let xmp_bytes = embed_jpeg_xmp(&source_bytes, &packet)?;
                let signed = embed_c2pa_signed_manifest(&xmp_bytes, "image/jpeg", &metadata)?;
                (
                    signed.bytes,
                    Some(signed.manifest_hash),
                    Some(signed.signer_status),
                    "xmp_app1",
                )
            }
            "wav" => {
                let propagation_bytes = embed_wav_public_metadata(&source_bytes, &json_packet)?;
                let signed = embed_c2pa_signed_manifest(
                    &propagation_bytes,
                    c2pa_format_for_public_metadata_format(normalized_format)?,
                    &metadata,
                )?;
                (
                    signed.bytes,
                    Some(signed.manifest_hash),
                    Some(signed.signer_status),
                    "riff_hsPM_plus_c2pa",
                )
            }
            "mp4" | "m4a" | "mov" => {
                let propagation_bytes = embed_mp4_public_metadata(&source_bytes, &json_packet)?;
                let signed = embed_c2pa_signed_manifest(
                    &propagation_bytes,
                    c2pa_format_for_public_metadata_format(normalized_format)?,
                    &metadata,
                )?;
                (
                    signed.bytes,
                    Some(signed.manifest_hash),
                    Some(signed.signer_status),
                    "bmff_uuid_plus_c2pa",
                )
            }
            _ => return Err("format must be png, jpeg, wav, mp4, m4a, or mov".to_string()),
        };
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create output dir: {error}"))?;
    }
    fs::write(&output_path, &output_bytes).map_err(|error| format!("write output: {error}"))?;

    let haystack = String::from_utf8_lossy(&output_bytes);
    let watermark_uid = metadata
        .get("watermarkUid")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let manifest_hash = metadata
        .get("manifestHash")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let has_container = match format.as_str() {
        "png" => output_bytes.windows(4).any(|window| window == b"iTXt"),
        "jpeg" | "jpg" => output_bytes.windows(2).any(|window| window == [0xFF, 0xE1]),
        "wav" => output_bytes.windows(4).any(|window| window == b"hsPM"),
        "mp4" | "m4a" | "mov" => output_bytes.windows(4).any(|window| window == b"uuid"),
        _ => false,
    };
    let has_namespace = match format.as_str() {
        "png" => haystack.contains("XML:com.adobe.xmp"),
        "jpeg" | "jpg" => output_bytes
            .windows(b"http://ns.adobe.com/xap/1.0/\0".len())
            .any(|window| window == b"http://ns.adobe.com/xap/1.0/\0"),
        "wav" | "mp4" | "m4a" | "mov" => {
            haystack.contains("hidden-shield-public-rights-embedded-metadata")
        }
        _ => false,
    };
    let has_c2pa_active_manifest = verify_c2pa_active_manifest(
        &output_bytes,
        c2pa_format_for_public_metadata_format(normalized_format)?,
    )
    .unwrap_or(false);
    let signed_manifest_hash = metadata
        .get("signedManifestStore")
        .and_then(|value| value.get("manifestStoreHash"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    let result = serde_json::json!({
        "sourcePath": source_path,
        "metadataPath": metadata_path,
        "outputPath": output_path,
        "format": normalized_format,
        "propagationLayer": propagation_layer,
        "watermarkUid": watermark_uid,
        "manifestHash": manifest_hash,
        "c2paManifestHash": c2pa_manifest_hash,
        "c2paSignerStatus": c2pa_signer_status,
        "outputSha256": sha256_hex(&output_bytes),
        "checks": {
            "hasContainer": has_container,
            "hasNamespace": has_namespace,
            "hasC2paActiveManifest": has_c2pa_active_manifest,
            "hasWatermarkUid": !watermark_uid.is_empty() && haystack.contains(watermark_uid),
            "hasManifestHash": !manifest_hash.is_empty() && haystack.contains(manifest_hash),
            "hasSignedManifestHash": signed_manifest_hash.is_empty() || haystack.contains(signed_manifest_hash),
            "hasLegalConclusionFalse": haystack.contains("legalConclusion=&quot;false&quot;") ||
                haystack.contains("legalConclusion\\\":false") ||
                haystack.contains("\"legalConclusion\":false") ||
                haystack.contains("hs:legalConclusion=\"false\"")
        }
    });
    let json = serde_json::to_string_pretty(&result)
        .map_err(|error| format!("serialize result: {error}"))?;
    if let Some(json_out) = json_out {
        fs::write(json_out, format!("{json}\n")).map_err(|error| format!("write json: {error}"))?;
    } else {
        println!("{json}");
    }
    let checks = result
        .get("checks")
        .and_then(Value::as_object)
        .ok_or_else(|| "missing checks".to_string())?;
    if checks.values().all(|value| value.as_bool() == Some(true)) {
        Ok(())
    } else {
        Err(format!("embedded metadata byte checks failed: {json}"))
    }
}

fn required_arg<'a>(args: &'a [String], name: &str) -> Result<&'a str, String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].as_str())
        .ok_or_else(|| format!("missing required argument {name}"))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        write!(&mut out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

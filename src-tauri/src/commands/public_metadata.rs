use std::io::Cursor;
use std::path::{Path, PathBuf};

use c2pa::{Builder, EphemeralSigner, Reader, SigningAlg};
use chrono::Utc;
use crc32fast::Hasher;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Manager, State};

use crate::db::queries;
use crate::AppState;

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1a\n";
const PNG_ITXT_KEYWORD: &str = "XML:com.adobe.xmp";
const JPEG_XMP_NAMESPACE: &[u8] = b"http://ns.adobe.com/xap/1.0/\0";
const HS_MP4_UUID: [u8; 16] = [
    0x48, 0x53, 0x50, 0x4d, 0x55, 0x42, 0x52, 0x49, 0x47, 0x48, 0x54, 0x53, 0x49, 0x47, 0x4e, 0x31,
];
const EMBED_BOUNDARY: &str =
    "creator_declaration_registry_snapshot_not_legal_advice_public_metadata_copy";

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportPublicRightsEmbeddedImageInput {
    pub record_id: u32,
    pub metadata: Value,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublicRightsEmbeddedImageExportResult {
    pub record_id: u32,
    pub watermark_uid: String,
    pub source_path: String,
    pub output_path: String,
    pub output_dir: String,
    pub file_format: String,
    pub embedded_standards: Vec<&'static str>,
    pub embedded_at: String,
    pub output_sha256: String,
    pub c2pa_manifest_status: String,
    pub c2pa_manifest_hash: Option<String>,
    pub c2pa_signer_status: String,
    pub legal_conclusion: bool,
    pub boundary: String,
}

#[tauri::command]
pub async fn export_public_rights_embedded_image(
    app_handle: AppHandle,
    state: State<'_, AppState>,
    input: ExportPublicRightsEmbeddedImageInput,
) -> Result<PublicRightsEmbeddedImageExportResult, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data directory: {e}"))?;
    let record = {
        let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
        queries::list_records(&conn)
            .into_iter()
            .find(|record| record.id == input.record_id)
            .ok_or_else(|| format!("未找到版权记录: {}", input.record_id))?
    };
    if input
        .metadata
        .get("legalConclusion")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err("公开元数据不能声明 legalConclusion=true".to_string());
    }
    let metadata_uid = input
        .metadata
        .get("watermarkUid")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .trim();
    if metadata_uid != record.watermark_uid {
        return Err("公开元数据 watermarkUid 与本地版权记录不一致，已阻断嵌入导出。".to_string());
    }
    let source_path = record
        .protected_copy_path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            "该记录没有可访问的保护副本路径，无法导出嵌入元数据图片副本。".to_string()
        })?;
    let source_path = PathBuf::from(source_path);
    let source_bytes = std::fs::read(&source_path).map_err(|e| format!("读取保护副本失败: {e}"))?;
    let file_format = detect_supported_image_format(&source_path, &source_bytes)?;
    let packet = build_xmp_packet(&input.metadata)?;
    let xmp_embedded_bytes = match file_format.as_str() {
        "png" => embed_png_xmp(&source_bytes, &packet)?,
        "jpeg" => embed_jpeg_xmp(&source_bytes, &packet)?,
        _ => return Err("暂仅支持 PNG / JPEG 图片嵌入公开元数据。".to_string()),
    };
    let c2pa_result = embed_c2pa_signed_manifest(
        &xmp_embedded_bytes,
        c2pa_format_for_public_metadata_format(&file_format)?,
        &input.metadata,
    );
    let (output_bytes, c2pa_manifest_status, c2pa_manifest_hash, c2pa_signer_status) =
        match c2pa_result {
            Ok(result) => (
                result.bytes,
                result.status,
                Some(result.manifest_hash),
                result.signer_status,
            ),
            Err(error) => (
                xmp_embedded_bytes,
                format!("fallback_xmp_only:{error}"),
                None,
                "c2pa_signing_failed_or_unconfigured".to_string(),
            ),
        };

    let output_dir = app_data_dir.join("public-rights-metadata");
    std::fs::create_dir_all(&output_dir).map_err(|e| format!("创建公开元数据目录失败: {e}"))?;
    let extension = if file_format == "jpeg" { "jpg" } else { "png" };
    let output_path = output_dir.join(format!(
        "{}-public-rights-embedded.{}",
        sanitize_file_name(&record.watermark_uid),
        extension
    ));
    std::fs::write(&output_path, &output_bytes)
        .map_err(|e| format!("写入嵌入元数据图片副本失败: {e}"))?;
    let output_sha256 = sha256_hex(&output_bytes);

    Ok(PublicRightsEmbeddedImageExportResult {
        record_id: record.id,
        watermark_uid: record.watermark_uid,
        source_path: source_path.to_string_lossy().to_string(),
        output_dir: output_dir.to_string_lossy().to_string(),
        output_path: output_path.to_string_lossy().to_string(),
        file_format,
        embedded_standards: vec![
            "XMP",
            "IPTC/PLUS JSON-LD mapping",
            "C2PA/CAWG JSON-LD mapping",
        ],
        embedded_at: Utc::now().to_rfc3339(),
        output_sha256,
        c2pa_manifest_status,
        c2pa_manifest_hash,
        c2pa_signer_status,
        legal_conclusion: false,
        boundary: EMBED_BOUNDARY.to_string(),
    })
}

fn detect_supported_image_format(path: &Path, bytes: &[u8]) -> Result<String, String> {
    if bytes.starts_with(PNG_SIGNATURE) {
        return Ok("png".to_string());
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        return Ok("jpeg".to_string());
    }
    let ext = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    Err(format!(
        "暂仅支持 PNG / JPEG 图片嵌入公开元数据，当前文件扩展名为 {ext}"
    ))
}

pub fn build_xmp_packet(metadata: &Value) -> Result<Vec<u8>, String> {
    let json = serde_json::to_string(metadata).map_err(|e| format!("序列化公开元数据失败: {e}"))?;
    let xmp = metadata
        .get("xmp")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    let iptc = metadata
        .get("iptc")
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    let json_ld = metadata
        .get("jsonLd")
        .or_else(|| metadata.get("json_ld"))
        .cloned()
        .unwrap_or(Value::Object(Default::default()));
    let c2pa_assertions = metadata
        .get("c2paAssertions")
        .or_else(|| metadata.get("c2pa_assertions"))
        .cloned()
        .unwrap_or(Value::Array(Vec::new()));
    let xmp_json = serde_json::to_string(&xmp).map_err(|e| format!("序列化 XMP 映射失败: {e}"))?;
    let iptc_json =
        serde_json::to_string(&iptc).map_err(|e| format!("序列化 IPTC 映射失败: {e}"))?;
    let json_ld_json =
        serde_json::to_string(&json_ld).map_err(|e| format!("序列化 JSON-LD 映射失败: {e}"))?;
    let c2pa_json = serde_json::to_string(&c2pa_assertions)
        .map_err(|e| format!("序列化 C2PA/CAWG 映射失败: {e}"))?;
    let packet = format!(
        r#"<?xpacket begin="" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" xmlns:hs="https://hiddenshield.local/ns#" xmlns:xmpRights="http://ns.adobe.com/xap/1.0/rights/">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      hs:boundary="{boundary}"
      hs:watermarkUid="{watermark_uid}"
      hs:manifestHash="{manifest_hash}"
      hs:trainingPolicy="{training_policy}"
      hs:legalConclusion="false">
      <hs:xmp>{xmp_json}</hs:xmp>
      <hs:iptcPlus>{iptc_json}</hs:iptcPlus>
      <hs:c2paCawg>{c2pa_json}</hs:c2paCawg>
      <hs:jsonLd>{json_ld_json}</hs:jsonLd>
      <hs:metadataExport>{json}</hs:metadataExport>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#,
        boundary = xml_escape(EMBED_BOUNDARY),
        watermark_uid = xml_escape(
            metadata
                .get("watermarkUid")
                .and_then(Value::as_str)
                .unwrap_or_default()
        ),
        manifest_hash = xml_escape(
            metadata
                .get("manifestHash")
                .and_then(Value::as_str)
                .unwrap_or_default()
        ),
        training_policy = xml_escape(
            metadata
                .get("jsonLd")
                .or_else(|| metadata.get("json_ld"))
                .and_then(|value| value.get("hs:trainingPolicy"))
                .and_then(Value::as_str)
                .unwrap_or_default()
        ),
        xmp_json = xml_escape(&xmp_json),
        iptc_json = xml_escape(&iptc_json),
        c2pa_json = xml_escape(&c2pa_json),
        json_ld_json = xml_escape(&json_ld_json),
        json = xml_escape(&json),
    );
    Ok(packet.into_bytes())
}

pub fn embed_png_xmp(bytes: &[u8], xmp_packet: &[u8]) -> Result<Vec<u8>, String> {
    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err("PNG 文件头无效".to_string());
    }
    let mut offset = PNG_SIGNATURE.len();
    let mut output = Vec::with_capacity(bytes.len() + xmp_packet.len() + 128);
    output.extend_from_slice(PNG_SIGNATURE);
    let mut inserted = false;
    while offset + 12 <= bytes.len() {
        let length = u32::from_be_bytes(
            bytes[offset..offset + 4]
                .try_into()
                .map_err(|_| "PNG chunk 长度无效".to_string())?,
        ) as usize;
        let chunk_type_start = offset + 4;
        let data_start = offset + 8;
        let chunk_end = data_start
            .checked_add(length)
            .and_then(|value| value.checked_add(4))
            .ok_or_else(|| "PNG chunk 溢出".to_string())?;
        if chunk_end > bytes.len() {
            return Err("PNG chunk 超出文件边界".to_string());
        }
        let chunk_type = &bytes[chunk_type_start..chunk_type_start + 4];
        if !inserted && chunk_type == b"IDAT" {
            output.extend_from_slice(&png_itxt_chunk(xmp_packet)?);
            inserted = true;
        }
        output.extend_from_slice(&bytes[offset..chunk_end]);
        offset = chunk_end;
        if chunk_type == b"IEND" {
            break;
        }
    }
    if !inserted {
        return Err("未找到 PNG IDAT chunk，无法嵌入 XMP。".to_string());
    }
    Ok(output)
}

fn png_itxt_chunk(xmp_packet: &[u8]) -> Result<Vec<u8>, String> {
    let mut data = Vec::with_capacity(PNG_ITXT_KEYWORD.len() + xmp_packet.len() + 5);
    data.extend_from_slice(PNG_ITXT_KEYWORD.as_bytes());
    data.push(0);
    data.push(0);
    data.push(0);
    data.push(0);
    data.push(0);
    data.extend_from_slice(xmp_packet);
    let length = u32::try_from(data.len()).map_err(|_| "XMP packet 过大".to_string())?;
    let mut chunk = Vec::with_capacity(data.len() + 12);
    chunk.extend_from_slice(&length.to_be_bytes());
    chunk.extend_from_slice(b"iTXt");
    chunk.extend_from_slice(&data);
    let mut hasher = Hasher::new();
    hasher.update(b"iTXt");
    hasher.update(&data);
    chunk.extend_from_slice(&hasher.finalize().to_be_bytes());
    Ok(chunk)
}

pub fn embed_jpeg_xmp(bytes: &[u8], xmp_packet: &[u8]) -> Result<Vec<u8>, String> {
    if !bytes.starts_with(&[0xFF, 0xD8]) {
        return Err("JPEG 文件头无效".to_string());
    }
    let segment_len = JPEG_XMP_NAMESPACE.len() + xmp_packet.len() + 2;
    if segment_len > u16::MAX as usize {
        return Err("XMP packet 超出 JPEG APP1 segment 限制".to_string());
    }
    let mut segment = Vec::with_capacity(segment_len + 2);
    segment.extend_from_slice(&[0xFF, 0xE1]);
    segment.extend_from_slice(&(segment_len as u16).to_be_bytes());
    segment.extend_from_slice(JPEG_XMP_NAMESPACE);
    segment.extend_from_slice(xmp_packet);

    let mut output = Vec::with_capacity(bytes.len() + segment.len());
    output.extend_from_slice(&bytes[..2]);
    output.extend_from_slice(&segment);
    output.extend_from_slice(&bytes[2..]);
    Ok(output)
}

pub fn build_public_metadata_json_packet(metadata: &Value) -> Result<Vec<u8>, String> {
    if metadata
        .get("legalConclusion")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err("公开元数据不能声明 legalConclusion=true".to_string());
    }
    let packet = serde_json::json!({
        "boundary": EMBED_BOUNDARY,
        "format": "hidden-shield-public-rights-embedded-metadata",
        "watermarkUid": metadata.get("watermarkUid").cloned().unwrap_or(Value::Null),
        "manifestHash": metadata.get("manifestHash").cloned().unwrap_or(Value::Null),
        "legalConclusion": false,
        "signedManifestStore": metadata.get("signedManifestStore").cloned().unwrap_or(Value::Null),
        "metadataExport": metadata,
    });
    serde_json::to_vec(&packet).map_err(|e| format!("序列化公开元数据 JSON packet 失败: {e}"))
}

pub fn embed_c2pa_signed_manifest(
    source_bytes: &[u8],
    format: &str,
    metadata: &Value,
) -> Result<PublicRightsC2paEmbeddedManifestResult, String> {
    if metadata
        .get("legalConclusion")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err("公开元数据不能声明 legalConclusion=true".to_string());
    }
    let watermark_uid = metadata
        .get("watermarkUid")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let manifest_hash = metadata
        .get("manifestHash")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let definition = serde_json::json!({
        "title": format!("HiddenShield public rights metadata {watermark_uid}"),
        "format": format,
        "claim_generator_info": [{
            "name": "HiddenShield",
            "version": env!("CARGO_PKG_VERSION")
        }]
    });
    let mut builder = Builder::default()
        .with_definition(definition)
        .map_err(|error| format!("build C2PA manifest definition: {error}"))?;
    builder
        .add_assertion_json(
            "cawg.training-and-data-mining",
            &metadata
                .get("c2paAssertions")
                .and_then(Value::as_array)
                .and_then(|assertions| {
                    assertions
                        .iter()
                        .find(|assertion| {
                            assertion
                                .get("label")
                                .and_then(Value::as_str)
                                .unwrap_or_default()
                                == "cawg.training-and-data-mining"
                        })
                        .and_then(|assertion| assertion.get("data"))
                })
                .cloned()
                .unwrap_or_else(|| {
                    serde_json::json!({
                        "watermarkUid": watermark_uid,
                        "manifestHash": manifest_hash,
                        "legalConclusion": false
                    })
                }),
        )
        .map_err(|error| format!("add CAWG TDM assertion: {error}"))?;
    builder
        .add_assertion_json(
            "org.hiddenshield.public-rights",
            &serde_json::json!({
                "watermarkUid": watermark_uid,
                "manifestHash": manifest_hash,
                "legalConclusion": false,
                "registryMetadata": metadata,
            }),
        )
        .map_err(|error| format!("add HiddenShield public rights assertion: {error}"))?;

    let signer = c2pa_signer()?;
    let signer_status = c2pa_signer_status();
    let mut source = Cursor::new(source_bytes.to_vec());
    let mut dest = Cursor::new(Vec::new());
    let embedded_manifest = builder
        .sign(signer.as_ref(), format, &mut source, &mut dest)
        .map_err(|error| format!("sign C2PA manifest: {error}"))?;
    dest.set_position(0);
    let reader = Reader::default()
        .with_stream(format, &mut dest)
        .map_err(|error| format!("verify embedded C2PA manifest: {error}"))?;
    if reader.active_manifest().is_none() {
        return Err("C2PA manifest was signed but no active manifest was readable".to_string());
    }
    let validation_status = reader.validation_status();
    Ok(PublicRightsC2paEmbeddedManifestResult {
        bytes: dest.into_inner(),
        manifest_hash: sha256_hex(&embedded_manifest),
        status: if validation_status.is_none() {
            "embedded_c2pa_signed_manifest".to_string()
        } else {
            "embedded_c2pa_signed_manifest_with_validation_warnings".to_string()
        },
        signer_status,
    })
}

pub struct PublicRightsC2paEmbeddedManifestResult {
    pub bytes: Vec<u8>,
    pub manifest_hash: String,
    pub status: String,
    pub signer_status: String,
}

pub fn verify_c2pa_active_manifest(bytes: &[u8], format: &str) -> Result<bool, String> {
    let mut source = Cursor::new(bytes.to_vec());
    Reader::default()
        .with_stream(format, &mut source)
        .map(|reader| reader.active_manifest().is_some())
        .map_err(|error| format!("read C2PA manifest: {error}"))
}

fn c2pa_signer() -> Result<Box<dyn c2pa::Signer + Send + Sync>, String> {
    let cert_pem = std::env::var("HIDDENSHIELD_C2PA_SIGN_CERT_PEM")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    let private_key_pem = std::env::var("HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty());
    if let (Some(cert_pem), Some(private_key_pem)) = (cert_pem, private_key_pem) {
        let alg = match std::env::var("HIDDENSHIELD_C2PA_SIGNING_ALG")
            .unwrap_or_else(|_| "Ed25519".to_string())
            .trim()
            .to_ascii_lowercase()
            .as_str()
        {
            "es256" => SigningAlg::Es256,
            "es384" => SigningAlg::Es384,
            "es512" => SigningAlg::Es512,
            "ps256" => SigningAlg::Ps256,
            "ps384" => SigningAlg::Ps384,
            "ps512" => SigningAlg::Ps512,
            "ed25519" => SigningAlg::Ed25519,
            other => {
                return Err(format!(
                    "unsupported HIDDENSHIELD_C2PA_SIGNING_ALG: {other}"
                ))
            }
        };
        return c2pa::create_signer::from_keys(
            cert_pem.as_bytes(),
            private_key_pem.as_bytes(),
            alg,
            std::env::var("HIDDENSHIELD_C2PA_TSA_URL")
                .ok()
                .filter(|value| !value.trim().is_empty()),
        )
        .map_err(|error| format!("load configured C2PA signer: {error}"));
    }
    EphemeralSigner::new("hiddenshield-local-c2pa-qa")
        .map(|signer| Box::new(signer) as Box<dyn c2pa::Signer + Send + Sync>)
        .map_err(|error| format!("create ephemeral C2PA signer: {error}"))
}

fn c2pa_signer_status() -> String {
    let configured = std::env::var("HIDDENSHIELD_C2PA_SIGN_CERT_PEM")
        .ok()
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
        && std::env::var("HIDDENSHIELD_C2PA_PRIVATE_KEY_PEM")
            .ok()
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false);
    if configured {
        "configured_certificate_chain".to_string()
    } else {
        "ephemeral_development_certificate_not_publicly_trusted".to_string()
    }
}

pub fn c2pa_format_for_public_metadata_format(file_format: &str) -> Result<&'static str, String> {
    match file_format {
        "png" => Ok("image/png"),
        "jpeg" => Ok("image/jpeg"),
        "jpg" => Ok("image/jpeg"),
        "wav" => Ok("audio/wav"),
        "mp4" => Ok("video/mp4"),
        "m4a" => Ok("audio/mp4"),
        "mov" => Ok("video/quicktime"),
        _ => Err(
            "C2PA 公开元数据 signed manifest 暂仅支持 PNG / JPEG / WAV / MP4 / M4A / MOV。"
                .to_string(),
        ),
    }
}

pub fn embed_wav_public_metadata(bytes: &[u8], metadata_packet: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("WAV/RIFF 文件头无效".to_string());
    }
    let padded_len = metadata_packet.len() + (metadata_packet.len() % 2);
    let new_riff_size = bytes
        .len()
        .checked_sub(8)
        .and_then(|value| value.checked_add(8 + padded_len))
        .ok_or_else(|| "WAV 元数据 chunk 过大".to_string())?;
    if new_riff_size > u32::MAX as usize {
        return Err("WAV 文件超过 RIFF 4GB 限制".to_string());
    }
    let mut output = bytes.to_vec();
    output[4..8].copy_from_slice(&(new_riff_size as u32).to_le_bytes());
    output.extend_from_slice(b"hsPM");
    output.extend_from_slice(&(metadata_packet.len() as u32).to_le_bytes());
    output.extend_from_slice(metadata_packet);
    if metadata_packet.len() % 2 == 1 {
        output.push(0);
    }
    Ok(output)
}

pub fn embed_mp4_public_metadata(bytes: &[u8], metadata_packet: &[u8]) -> Result<Vec<u8>, String> {
    if bytes.len() < 12 || !bytes.windows(4).take(16).any(|window| window == b"ftyp") {
        return Err("MP4/M4A/MOV 文件头无效，未找到 ftyp box。".to_string());
    }
    let box_size = metadata_packet
        .len()
        .checked_add(8)
        .and_then(|value| value.checked_add(HS_MP4_UUID.len()))
        .ok_or_else(|| "MP4 公开元数据 box 过大".to_string())?;
    if box_size > u32::MAX as usize {
        return Err("MP4 公开元数据 box 超出 32-bit size 限制".to_string());
    }
    let mut output = Vec::with_capacity(bytes.len() + box_size);
    output.extend_from_slice(bytes);
    output.extend_from_slice(&(box_size as u32).to_be_bytes());
    output.extend_from_slice(b"uuid");
    output.extend_from_slice(&HS_MP4_UUID);
    output.extend_from_slice(metadata_packet);
    Ok(output)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn sanitize_file_name(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if sanitized.trim().is_empty() {
        "unknown".to_string()
    } else {
        sanitized
    }
}

fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_embedding_writes_itxt_xmp_before_idat() {
        let source = minimal_png();
        let output = embed_png_xmp(&source, b"<x:xmpmeta>HiddenShield</x:xmpmeta>").unwrap();
        let text = String::from_utf8_lossy(&output);
        assert!(text.contains(PNG_ITXT_KEYWORD));
        assert!(text.contains("HiddenShield"));
        assert!(output.windows(4).any(|window| window == b"IDAT"));
    }

    #[test]
    fn jpeg_embedding_writes_app1_xmp_segment() {
        let source = [0xFF, 0xD8, 0xFF, 0xD9];
        let output = embed_jpeg_xmp(&source, b"<x:xmpmeta>HiddenShield</x:xmpmeta>").unwrap();
        assert!(output.starts_with(&[0xFF, 0xD8, 0xFF, 0xE1]));
        assert!(output
            .windows(JPEG_XMP_NAMESPACE.len())
            .any(|window| window == JPEG_XMP_NAMESPACE));
        assert!(String::from_utf8_lossy(&output).contains("HiddenShield"));
    }

    #[test]
    fn xmp_packet_preserves_legal_boundary() {
        let packet = build_xmp_packet(&serde_json::json!({
            "watermarkUid": "wm-test",
            "manifestHash": "hash-test",
            "legalConclusion": false,
            "jsonLd": {
                "hs:trainingPolicy": "separate_authorization_required"
            }
        }))
        .unwrap();
        let text = String::from_utf8(packet).unwrap();
        assert!(text.contains("wm-test"));
        assert!(text.contains("legalConclusion=\"false\""));
        assert!(text.contains(EMBED_BOUNDARY));
    }

    fn minimal_png() -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(PNG_SIGNATURE);
        bytes.extend_from_slice(&chunk(b"IHDR", &[0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0, 0, 0]));
        bytes.extend_from_slice(&chunk(b"IDAT", &[0x78, 0x9c, 0x63, 0, 0, 0, 2, 0, 1]));
        bytes.extend_from_slice(&chunk(b"IEND", &[]));
        bytes
    }

    fn chunk(kind: &[u8; 4], data: &[u8]) -> Vec<u8> {
        let mut chunk = Vec::new();
        chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
        chunk.extend_from_slice(kind);
        chunk.extend_from_slice(data);
        let mut hasher = Hasher::new();
        hasher.update(kind);
        hasher.update(data);
        chunk.extend_from_slice(&hasher.finalize().to_be_bytes());
        chunk
    }
}

use serde::Serialize;
use watermark_core::{
    decode_watermark_payload_readonly, embed_v3_readonly_anchor_png_bytes,
    embed_v3_readonly_anchor_wav_bytes, encode_payload_v3_minimal_anchor,
    extract_v3_readonly_anchor_png_bytes, extract_v3_readonly_anchor_wav_bytes,
    PayloadV3MinimalAnchorBuildInput, WatermarkDecodedPayload, WatermarkPayloadV3MinimalAnchor,
    PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct V3ReadonlyBridgeResult {
    pub fixture_id: String,
    pub bridge: String,
    pub media_kind: String,
    pub watermark_uid: String,
    pub payload_protocol_version: u32,
    pub payload_bytes_length: u32,
    pub payload_auth_status: String,
}

pub fn build_v3_readonly_fixture_bytes(media_kind: &str) -> Result<Vec<u8>, String> {
    let watermark_id = v3_fixture_watermark_id(media_kind)?;
    let anchor =
        WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput { watermark_id })
            .map_err(|error| format!("build V3 readonly anchor: {error}"))?;
    Ok(encode_payload_v3_minimal_anchor(&anchor).to_vec())
}

pub fn build_v3_readonly_fixture_media_bytes(media_kind: &str) -> Result<Vec<u8>, String> {
    let watermark_id = v3_fixture_watermark_id(media_kind)?;
    let anchor =
        WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput { watermark_id })
            .map_err(|error| format!("build V3 readonly media anchor: {error}"))?;
    match media_kind {
        "image" => embed_v3_readonly_anchor_png_bytes(&sample_png_bytes()?, &anchor)
            .map_err(|error| format!("embed V3 readonly PNG fixture: {error}")),
        "audio" => embed_v3_readonly_anchor_wav_bytes(&sample_wav_bytes(), &anchor)
            .map_err(|error| format!("embed V3 readonly WAV fixture: {error}")),
        _ => Err(format!(
            "unsupported V3 readonly fixture media kind: {media_kind}"
        )),
    }
}

pub fn decode_v3_readonly_fixture_for_desktop(
    fixture_id: &str,
    media_kind: &str,
    payload_bytes: &[u8],
) -> Result<V3ReadonlyBridgeResult, String> {
    let decoded = decode_watermark_payload_readonly(payload_bytes)
        .map_err(|error| format!("decode V3 readonly fixture: {error}"))?;
    match decoded {
        WatermarkDecodedPayload::V3MinimalAnchor(anchor) => Ok(V3ReadonlyBridgeResult {
            fixture_id: fixture_id.to_string(),
            bridge: "desktop".to_string(),
            media_kind: media_kind.to_string(),
            watermark_uid: anchor.watermark_uid(),
            payload_protocol_version: anchor.protocol_version as u32,
            payload_bytes_length: PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32,
            payload_auth_status: "verified".to_string(),
        }),
        WatermarkDecodedPayload::V2(_) => Err(
            "V3 readonly fixture expected minimal anchor bytes but decoded V2 payload".to_string(),
        ),
    }
}

pub fn decode_v3_readonly_media_fixture_for_desktop(
    fixture_id: &str,
    media_kind: &str,
    media_bytes: &[u8],
) -> Result<V3ReadonlyBridgeResult, String> {
    let decoded = match media_kind {
        "image" => extract_v3_readonly_anchor_png_bytes(media_bytes),
        "audio" => extract_v3_readonly_anchor_wav_bytes(media_bytes),
        _ => {
            return Err(format!(
                "unsupported V3 readonly fixture media kind: {media_kind}"
            ))
        }
    }
    .map_err(|error| format!("decode V3 readonly media fixture: {error}"))?;
    match decoded {
        WatermarkDecodedPayload::V3MinimalAnchor(anchor) => Ok(V3ReadonlyBridgeResult {
            fixture_id: fixture_id.to_string(),
            bridge: "desktop".to_string(),
            media_kind: media_kind.to_string(),
            watermark_uid: anchor.watermark_uid(),
            payload_protocol_version: anchor.protocol_version as u32,
            payload_bytes_length: PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32,
            payload_auth_status: "verified".to_string(),
        }),
        WatermarkDecodedPayload::V2(_) => Err(
            "V3 readonly media fixture expected minimal anchor bytes but decoded V2 payload"
                .to_string(),
        ),
    }
}

fn v3_fixture_watermark_id(media_kind: &str) -> Result<[u8; 16], String> {
    match media_kind {
        "image" => Ok([
            0x31, 0x32, 0x33, 0x34, 0x41, 0x42, 0x43, 0x44, 0x51, 0x52, 0x53, 0x54, 0x61, 0x62,
            0x63, 0x64,
        ]),
        "audio" => Ok([
            0x51, 0x52, 0x53, 0x54, 0x61, 0x62, 0x63, 0x64, 0x71, 0x72, 0x73, 0x74, 0x81, 0x82,
            0x83, 0x84,
        ]),
        _ => Err(format!(
            "unsupported V3 readonly fixture media kind: {media_kind}"
        )),
    }
}

fn sample_png_bytes() -> Result<Vec<u8>, String> {
    let img = image::RgbImage::from_fn(256, 256, |x, y| {
        image::Rgb([
            (x * 255 / 256) as u8,
            (y * 255 / 256) as u8,
            ((x + y) * 127 / 256) as u8,
        ])
    });
    let mut cursor = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .map_err(|error| format!("encode V3 readonly PNG fixture: {error}"))?;
    Ok(cursor.into_inner())
}

fn sample_wav_bytes() -> Vec<u8> {
    let sample_rate = 44_100usize;
    let sample_count = sample_rate;
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
    bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn desktop_v3_readonly_fixture_preserves_anchor_fields() {
        let bytes = build_v3_readonly_fixture_bytes("image").unwrap();
        let result = decode_v3_readonly_fixture_for_desktop(
            "v3_image_desktop_write_mobile_read",
            "image",
            &bytes,
        )
        .unwrap();

        assert_eq!(
            result.watermark_uid,
            "HS-31323334-41424344-51525354-61626364"
        );
        assert_eq!(result.payload_protocol_version, 3);
        assert_eq!(result.payload_bytes_length, 39);
        assert_eq!(result.payload_auth_status, "verified");
    }

    #[test]
    fn desktop_v3_readonly_media_fixture_preserves_anchor_fields() {
        let bytes = build_v3_readonly_fixture_media_bytes("audio").unwrap();
        let result = decode_v3_readonly_media_fixture_for_desktop(
            "v3_audio_desktop_write_mobile_read",
            "audio",
            &bytes,
        )
        .unwrap();

        assert_eq!(
            result.watermark_uid,
            "HS-51525354-61626364-71727374-81828384"
        );
        assert_eq!(result.payload_protocol_version, 3);
        assert_eq!(result.payload_bytes_length, 39);
        assert_eq!(result.payload_auth_status, "verified");
    }
}

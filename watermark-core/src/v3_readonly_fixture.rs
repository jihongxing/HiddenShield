use crate::{
    decode_watermark_payload_readonly, encode_payload_v3_minimal_anchor, WatermarkDecodedPayload,
    WatermarkError, WatermarkPayloadV3MinimalAnchor, PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};

const PNG_SIGNATURE: &[u8; 8] = b"\x89PNG\r\n\x1A\n";
const PNG_TEXT_CHUNK_TYPE: &[u8; 4] = b"tEXt";
const PNG_IHDR_CHUNK_TYPE: &[u8; 4] = b"IHDR";
const PNG_ANCHOR_KEYWORD: &[u8] = b"HiddenShieldV3ReadonlyAnchor";
const WAV_SIGNATURE: &[u8; 4] = b"RIFF";
const WAV_FORMAT: &[u8; 4] = b"WAVE";
const WAV_ANCHOR_CHUNK_ID: &[u8; 4] = b"hsV3";

pub fn embed_v3_readonly_anchor_png_bytes(
    png_bytes: &[u8],
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<Vec<u8>, WatermarkError> {
    let anchor_bytes = encode_payload_v3_minimal_anchor(anchor);
    embed_v3_readonly_anchor_png_payload_bytes(png_bytes, &anchor_bytes)
}

pub fn extract_v3_readonly_anchor_png_bytes(
    png_bytes: &[u8],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    if !png_bytes.starts_with(PNG_SIGNATURE) {
        return Err(WatermarkError::ExtractFailed(
            "V3 readonly PNG fixture requires a PNG container".into(),
        ));
    }
    let mut offset = PNG_SIGNATURE.len();
    while offset + 12 <= png_bytes.len() {
        let length = u32::from_be_bytes(
            png_bytes[offset..offset + 4]
                .try_into()
                .expect("slice length checked"),
        ) as usize;
        let chunk_type = &png_bytes[offset + 4..offset + 8];
        let data_start = offset + 8;
        let data_end = data_start + length;
        let next = data_end + 4;
        if next > png_bytes.len() {
            break;
        }
        if chunk_type == PNG_TEXT_CHUNK_TYPE {
            let data = &png_bytes[data_start..data_end];
            if let Some(hex_payload) = data
                .strip_prefix(PNG_ANCHOR_KEYWORD)
                .and_then(|rest| rest.strip_prefix(&[0]))
            {
                let anchor_bytes = decode_hex(hex_payload)?;
                return decode_v3_readonly_anchor_payload_bytes(&anchor_bytes);
            }
        }
        offset = next;
    }
    Err(WatermarkError::ExtractFailed(
        "V3 readonly PNG anchor chunk not found".into(),
    ))
}

pub fn embed_v3_readonly_anchor_wav_bytes(
    wav_bytes: &[u8],
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<Vec<u8>, WatermarkError> {
    let anchor_bytes = encode_payload_v3_minimal_anchor(anchor);
    embed_v3_readonly_anchor_wav_payload_bytes(wav_bytes, &anchor_bytes)
}

pub fn extract_v3_readonly_anchor_wav_bytes(
    wav_bytes: &[u8],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    if wav_bytes.len() < 12 || &wav_bytes[0..4] != WAV_SIGNATURE || &wav_bytes[8..12] != WAV_FORMAT
    {
        return Err(WatermarkError::ExtractFailed(
            "V3 readonly WAV fixture requires a RIFF/WAVE container".into(),
        ));
    }

    let mut offset = 12usize;
    while offset + 8 <= wav_bytes.len() {
        let chunk_id = &wav_bytes[offset..offset + 4];
        let length = u32::from_le_bytes(
            wav_bytes[offset + 4..offset + 8]
                .try_into()
                .expect("slice length checked"),
        ) as usize;
        let data_start = offset + 8;
        let data_end = data_start + length;
        if data_end > wav_bytes.len() {
            break;
        }
        if chunk_id == WAV_ANCHOR_CHUNK_ID {
            return decode_v3_readonly_anchor_payload_bytes(&wav_bytes[data_start..data_end]);
        }
        offset = data_end + (length % 2);
    }
    Err(WatermarkError::ExtractFailed(
        "V3 readonly WAV anchor chunk not found".into(),
    ))
}

fn embed_v3_readonly_anchor_png_payload_bytes(
    png_bytes: &[u8],
    anchor_bytes: &[u8; PAYLOAD_V3_MINIMAL_ANCHOR_BYTES],
) -> Result<Vec<u8>, WatermarkError> {
    if !png_bytes.starts_with(PNG_SIGNATURE) {
        return Err(WatermarkError::ExtractFailed(
            "V3 readonly PNG fixture requires a PNG container".into(),
        ));
    }
    let ihdr_end = first_png_chunk_end(png_bytes, PNG_IHDR_CHUNK_TYPE)?;
    let mut data = Vec::with_capacity(PNG_ANCHOR_KEYWORD.len() + 1 + anchor_bytes.len() * 2);
    data.extend_from_slice(PNG_ANCHOR_KEYWORD);
    data.push(0);
    data.extend_from_slice(&encode_hex(anchor_bytes));
    let chunk = build_png_chunk(PNG_TEXT_CHUNK_TYPE, &data);

    let mut output = Vec::with_capacity(png_bytes.len() + chunk.len());
    output.extend_from_slice(&png_bytes[..ihdr_end]);
    output.extend_from_slice(&chunk);
    output.extend_from_slice(&png_bytes[ihdr_end..]);
    Ok(output)
}

fn embed_v3_readonly_anchor_wav_payload_bytes(
    wav_bytes: &[u8],
    anchor_bytes: &[u8; PAYLOAD_V3_MINIMAL_ANCHOR_BYTES],
) -> Result<Vec<u8>, WatermarkError> {
    if wav_bytes.len() < 12 || &wav_bytes[0..4] != WAV_SIGNATURE || &wav_bytes[8..12] != WAV_FORMAT
    {
        return Err(WatermarkError::ExtractFailed(
            "V3 readonly WAV fixture requires a RIFF/WAVE container".into(),
        ));
    }

    let mut output = wav_bytes.to_vec();
    output.extend_from_slice(WAV_ANCHOR_CHUNK_ID);
    output.extend_from_slice(&(anchor_bytes.len() as u32).to_le_bytes());
    output.extend_from_slice(anchor_bytes);
    if anchor_bytes.len() % 2 == 1 {
        output.push(0);
    }
    let riff_size = output.len().saturating_sub(8) as u32;
    output[4..8].copy_from_slice(&riff_size.to_le_bytes());
    Ok(output)
}

fn decode_v3_readonly_anchor_payload_bytes(
    anchor_bytes: &[u8],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    let decoded = decode_watermark_payload_readonly(anchor_bytes)?;
    match decoded {
        WatermarkDecodedPayload::V3MinimalAnchor(_) => Ok(decoded),
        WatermarkDecodedPayload::V2(_) => Err(WatermarkError::ExtractFailed(
            "V3 readonly fixture expected minimal anchor bytes but decoded V2 payload".into(),
        )),
    }
}

fn first_png_chunk_end(png_bytes: &[u8], expected_type: &[u8; 4]) -> Result<usize, WatermarkError> {
    if png_bytes.len() < PNG_SIGNATURE.len() + 12 {
        return Err(WatermarkError::ExtractFailed(
            "PNG container too short for V3 readonly fixture".into(),
        ));
    }
    let offset = PNG_SIGNATURE.len();
    let length = u32::from_be_bytes(
        png_bytes[offset..offset + 4]
            .try_into()
            .expect("slice length checked"),
    ) as usize;
    let chunk_type = &png_bytes[offset + 4..offset + 8];
    if chunk_type != expected_type {
        return Err(WatermarkError::ExtractFailed(
            "PNG fixture first chunk is not IHDR".into(),
        ));
    }
    let end = offset + 8 + length + 4;
    if end > png_bytes.len() {
        return Err(WatermarkError::ExtractFailed(
            "PNG IHDR chunk exceeds container length".into(),
        ));
    }
    Ok(end)
}

fn build_png_chunk(chunk_type: &[u8; 4], data: &[u8]) -> Vec<u8> {
    let mut chunk = Vec::with_capacity(12 + data.len());
    chunk.extend_from_slice(&(data.len() as u32).to_be_bytes());
    chunk.extend_from_slice(chunk_type);
    chunk.extend_from_slice(data);
    let crc = png_crc32(&chunk[4..]);
    chunk.extend_from_slice(&crc.to_be_bytes());
    chunk
}

fn png_crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

fn encode_hex(bytes: &[u8]) -> Vec<u8> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = Vec::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(HEX[(byte >> 4) as usize]);
        output.push(HEX[(byte & 0x0F) as usize]);
    }
    output
}

fn decode_hex(bytes: &[u8]) -> Result<Vec<u8>, WatermarkError> {
    if bytes.len() % 2 != 0 {
        return Err(WatermarkError::ExtractFailed(
            "V3 readonly fixture hex payload has odd length".into(),
        ));
    }
    let mut output = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks_exact(2) {
        let high = hex_nibble(pair[0])?;
        let low = hex_nibble(pair[1])?;
        output.push((high << 4) | low);
    }
    Ok(output)
}

fn hex_nibble(byte: u8) -> Result<u8, WatermarkError> {
    match byte {
        b'0'..=b'9' => Ok(byte - b'0'),
        b'a'..=b'f' => Ok(byte - b'a' + 10),
        b'A'..=b'F' => Ok(byte - b'A' + 10),
        _ => Err(WatermarkError::ExtractFailed(
            "V3 readonly fixture hex payload contains non-hex byte".into(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PayloadV3MinimalAnchorBuildInput, WatermarkPayloadV3MinimalAnchor};

    fn sample_anchor() -> WatermarkPayloadV3MinimalAnchor {
        WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput {
            watermark_id: [
                0x31, 0x32, 0x33, 0x34, 0x41, 0x42, 0x43, 0x44, 0x51, 0x52, 0x53, 0x54, 0x61, 0x62,
                0x63, 0x64,
            ],
        })
        .unwrap()
    }

    fn sample_png() -> Vec<u8> {
        let img = image::RgbImage::from_fn(64, 64, |x, y| {
            image::Rgb([(x * 3) as u8, (y * 3) as u8, 128])
        });
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    fn sample_wav() -> Vec<u8> {
        let sample_rate = 8_000usize;
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
        bytes
    }

    #[test]
    fn v3_readonly_png_container_fixture_roundtrips_anchor() {
        let anchor = sample_anchor();
        let media = embed_v3_readonly_anchor_png_bytes(&sample_png(), &anchor).unwrap();
        let decoded = extract_v3_readonly_anchor_png_bytes(&media).unwrap();

        assert!(decoded.is_v3_minimal_anchor());
        assert_eq!(decoded.watermark_uid(), anchor.watermark_uid());
        assert_eq!(decoded.protocol_version(), 3);
        assert_eq!(decoded.payload_bytes_length(), 39);
        assert_eq!(decoded.payload_auth_status(), "verified");
    }

    #[test]
    fn v3_readonly_wav_container_fixture_roundtrips_anchor() {
        let anchor = sample_anchor();
        let media = embed_v3_readonly_anchor_wav_bytes(&sample_wav(), &anchor).unwrap();
        let decoded = extract_v3_readonly_anchor_wav_bytes(&media).unwrap();

        assert!(decoded.is_v3_minimal_anchor());
        assert_eq!(decoded.watermark_uid(), anchor.watermark_uid());
        assert_eq!(decoded.protocol_version(), 3);
        assert_eq!(decoded.payload_bytes_length(), 39);
        assert_eq!(decoded.payload_auth_status(), "verified");
    }
}

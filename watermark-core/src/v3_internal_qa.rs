use serde::{Deserialize, Serialize};

use crate::error::WatermarkError;
use crate::payload::{
    PayloadV3MinimalAnchorBuildInput, WatermarkDecodedPayload, WatermarkPayloadV3MinimalAnchor,
};
use crate::{audio, image};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum V3InternalQaWriteGate {
    Off,
    InternalQa,
    ForceV2Rollback,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum V3InternalQaMediaKind {
    Image,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3InternalQaWriteInput {
    pub media_kind: V3InternalQaMediaKind,
    pub media_bytes: Vec<u8>,
    pub watermark_id: [u8; 16],
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct V3InternalQaWriteOutput {
    pub media_kind: V3InternalQaMediaKind,
    pub bytes: Vec<u8>,
    pub watermark_uid: String,
    pub payload_protocol_version: u8,
    pub payload_bytes_length: usize,
    pub payload_auth_status: String,
    pub media_payload_role: String,
}

pub fn embed_v3_internal_qa_media(
    gate: V3InternalQaWriteGate,
    input: V3InternalQaWriteInput,
) -> Result<V3InternalQaWriteOutput, WatermarkError> {
    let anchor = anchor(input.watermark_id)?;
    let bytes = match gate {
        V3InternalQaWriteGate::InternalQa => match input.media_kind {
            V3InternalQaMediaKind::Image => image::embed_image_v3_bytes(
                &input.media_bytes,
                &anchor,
                ::image::ImageFormat::Png,
                image::DEFAULT_IMAGE_ALPHA,
            )?,
            V3InternalQaMediaKind::Audio => {
                audio::embed_audio_v3_internal_qa_wav_bytes(&input.media_bytes, &anchor)?
            }
        },
        V3InternalQaWriteGate::Off => {
            return Err(WatermarkError::EmbedFailed(
                "v3_internal_qa_write_gate_off: V3 internal QA writing is disabled".into(),
            ))
        }
        V3InternalQaWriteGate::ForceV2Rollback => {
            return Err(WatermarkError::EmbedFailed(
                "v3_internal_qa_force_v2_rollback: V3 internal QA writing is blocked".into(),
            ))
        }
    };

    let decoded = match input.media_kind {
        V3InternalQaMediaKind::Image => {
            image::extract_image_watermark_readonly_candidate_bytes(&bytes)?
        }
        V3InternalQaMediaKind::Audio => {
            audio::extract_watermark_wav_readonly_candidate_bytes(&bytes)?
        }
    };
    output_from_decoded(input.media_kind, bytes, decoded)
}

fn anchor(watermark_id: [u8; 16]) -> Result<WatermarkPayloadV3MinimalAnchor, WatermarkError> {
    WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput { watermark_id })
}

fn output_from_decoded(
    media_kind: V3InternalQaMediaKind,
    bytes: Vec<u8>,
    decoded: WatermarkDecodedPayload,
) -> Result<V3InternalQaWriteOutput, WatermarkError> {
    if !decoded.is_v3_minimal_anchor() {
        return Err(WatermarkError::EmbedFailed(
            "v3_internal_qa expected V3 minimal anchor after write".into(),
        ));
    }
    Ok(V3InternalQaWriteOutput {
        media_kind,
        bytes,
        watermark_uid: decoded.watermark_uid(),
        payload_protocol_version: decoded.protocol_version(),
        payload_bytes_length: decoded.payload_bytes_length(),
        payload_auth_status: decoded.payload_auth_status().to_string(),
        media_payload_role: "v3_minimal_anchor".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(kind: V3InternalQaMediaKind) -> V3InternalQaWriteInput {
        let media_bytes = match kind {
            V3InternalQaMediaKind::Image => sample_png(),
            V3InternalQaMediaKind::Audio => sample_wav(),
        };
        V3InternalQaWriteInput {
            media_kind: kind,
            media_bytes,
            watermark_id: [
                0x91, 0x92, 0x93, 0x94, 0xA1, 0xA2, 0xA3, 0xA4, 0xB1, 0xB2, 0xB3, 0xB4, 0xC1, 0xC2,
                0xC3, 0xC4,
            ],
        }
    }

    #[test]
    fn v3_internal_qa_gate_writes_image_anchor() {
        let output = embed_v3_internal_qa_media(
            V3InternalQaWriteGate::InternalQa,
            input(V3InternalQaMediaKind::Image),
        )
        .unwrap();

        assert_eq!(output.payload_protocol_version, 3);
        assert_eq!(output.payload_bytes_length, 39);
        assert_eq!(output.payload_auth_status, "verified");
        assert_eq!(output.media_payload_role, "v3_minimal_anchor");
    }

    #[test]
    fn v3_internal_qa_gate_writes_audio_anchor() {
        let output = embed_v3_internal_qa_media(
            V3InternalQaWriteGate::InternalQa,
            input(V3InternalQaMediaKind::Audio),
        )
        .unwrap();

        assert_eq!(output.payload_protocol_version, 3);
        assert_eq!(output.payload_bytes_length, 39);
        assert_eq!(output.payload_auth_status, "verified");
        assert_eq!(output.media_payload_role, "v3_minimal_anchor");
    }

    #[test]
    fn v3_internal_qa_gate_off_rejects_v3_write() {
        let error = embed_v3_internal_qa_media(
            V3InternalQaWriteGate::Off,
            input(V3InternalQaMediaKind::Image),
        )
        .unwrap_err();

        assert!(error.to_string().contains("v3_internal_qa_write_gate_off"));
    }

    #[test]
    fn v3_internal_qa_force_rollback_rejects_v3_write() {
        let error = embed_v3_internal_qa_media(
            V3InternalQaWriteGate::ForceV2Rollback,
            input(V3InternalQaMediaKind::Audio),
        )
        .unwrap_err();

        assert!(error
            .to_string()
            .contains("v3_internal_qa_force_v2_rollback"));
    }

    fn sample_png() -> Vec<u8> {
        let img = ::image::RgbImage::from_fn(1024, 1024, |x, y| {
            ::image::Rgb([
                (x * 255 / 1024) as u8,
                (y * 255 / 1024) as u8,
                ((x + y) * 127 / 1024) as u8,
            ])
        });
        let mut cursor = std::io::Cursor::new(Vec::new());
        ::image::DynamicImage::ImageRgb8(img)
            .write_to(&mut cursor, ::image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    fn sample_wav() -> Vec<u8> {
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
        bytes
    }
}

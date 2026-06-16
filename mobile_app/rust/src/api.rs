use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use watermark_core::{
    AIContentFlags, ImageOutputFormat, MediaInput, MediaOutput, TrainingPermission,
    WatermarkPayload, WatermarkService,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobileMediaPayload {
    pub user_seed: Vec<u8>,
    pub timestamp: u64,
    pub device_id: Vec<u8>,
    pub file_hash: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobileImageResult {
    pub bytes: Vec<u8>,
    pub watermark_uid: String,
    pub sha256: String,
    pub format: MobileImageOutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobileExtractResult {
    pub watermark_uid: String,
    pub timestamp: u64,
    pub device_id_hex: String,
    pub file_hash_hex: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MobileImageOutputFormat {
    Png,
    Jpeg,
    WebP,
}

#[derive(Debug, Error)]
pub enum MobileWatermarkError {
    #[error("invalid payload: {0}")]
    InvalidPayload(String),

    #[error("watermark operation failed: {0}")]
    OperationFailed(String),
}

pub fn embed_image_for_mobile(
    image_bytes: Vec<u8>,
    payload: MobileMediaPayload,
    output_format: MobileImageOutputFormat,
    allow_rewrite: bool,
) -> Result<MobileImageResult, MobileWatermarkError> {
    let payload = payload.into_core_payload()?;
    let format = output_format.into_core_format();
    let output = WatermarkService::embed(
        MediaInput::ImageBytes { bytes: image_bytes },
        &payload,
        watermark_core::EmbedOptions {
            image_output_format: format,
            allow_rewrite,
        },
    )
    .map_err(|error| MobileWatermarkError::OperationFailed(error.to_string()))?;

    match output {
        MediaOutput::ImageBytes { bytes, .. } => Ok(MobileImageResult {
            sha256: sha256_hex(&bytes),
            watermark_uid: payload.watermark_uid(),
            bytes,
            format: output_format,
        }),
        _ => Err(MobileWatermarkError::OperationFailed(
            "unexpected non-image output".to_string(),
        )),
    }
}

pub fn extract_image_for_mobile(
    image_bytes: Vec<u8>,
) -> Result<MobileExtractResult, MobileWatermarkError> {
    let payload = WatermarkService::extract(MediaInput::ImageBytes { bytes: image_bytes })
        .map_err(|error| MobileWatermarkError::OperationFailed(error.to_string()))?;
    Ok(payload.into_mobile_extract_result())
}

impl MobileMediaPayload {
    fn into_core_payload(self) -> Result<WatermarkPayload, MobileWatermarkError> {
        let user_seed = fixed_array::<8>(&self.user_seed, "user_seed")?;
        let device_id = fixed_array::<4>(&self.device_id, "device_id")?;
        let file_hash = fixed_array::<2>(&self.file_hash, "file_hash")?;
        Ok(WatermarkPayload::new(
            user_seed,
            self.timestamp,
            device_id,
            file_hash,
            default_ai_flags(),
        ))
    }
}

impl MobileImageOutputFormat {
    fn into_core_format(self) -> ImageOutputFormat {
        match self {
            Self::Png => ImageOutputFormat::Png,
            Self::Jpeg => ImageOutputFormat::Jpeg,
            Self::WebP => ImageOutputFormat::WebP,
        }
    }
}

trait IntoMobileExtractResult {
    fn into_mobile_extract_result(self) -> MobileExtractResult;
}

impl IntoMobileExtractResult for WatermarkPayload {
    fn into_mobile_extract_result(self) -> MobileExtractResult {
        MobileExtractResult {
            watermark_uid: self.watermark_uid(),
            timestamp: self.timestamp,
            device_id_hex: hex::encode(self.device_id),
            file_hash_hex: hex::encode(self.file_hash),
        }
    }
}

fn fixed_array<const N: usize>(
    input: &[u8],
    field: &str,
) -> Result<[u8; N], MobileWatermarkError> {
    if input.len() != N {
        return Err(MobileWatermarkError::InvalidPayload(format!(
            "{field} must be {N} bytes"
        )));
    }

    let mut out = [0u8; N];
    out.copy_from_slice(input);
    Ok(out)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    hex::encode(hash)
}

fn default_ai_flags() -> AIContentFlags {
    AIContentFlags {
        is_ai_generated: false,
        training_permission: TrainingPermission::Prohibited,
        generation_method: watermark_core::GenerationMethod::HumanCreated,
        human_modification_level: watermark_core::ModificationLevel::PureAI,
        authenticity_claim: watermark_core::AuthenticityClaim::Unspecified,
        reserved: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> MobileMediaPayload {
        MobileMediaPayload {
            user_seed: vec![0x42; 8],
            timestamp: 1_700_000_000,
            device_id: vec![0xAB; 4],
            file_hash: vec![0xCD; 2],
        }
    }

    fn make_png_bytes() -> Vec<u8> {
        let img = image::RgbaImage::from_fn(256, 256, |x, y| {
            image::Rgba([
                ((x as f32 / 256.0 * 200.0) as u8).wrapping_add(30),
                ((y as f32 / 256.0 * 200.0) as u8).wrapping_add(30),
                128,
                255,
            ])
        });
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(img)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    #[test]
    fn mobile_image_roundtrip() {
        let result = embed_image_for_mobile(
            make_png_bytes(),
            sample_payload(),
            MobileImageOutputFormat::Png,
            false,
        )
        .unwrap();
        let extracted = extract_image_for_mobile(result.bytes).unwrap();

        assert_eq!(extracted.watermark_uid, result.watermark_uid);
        assert_eq!(extracted.device_id_hex, "abababab");
        assert_eq!(extracted.file_hash_hex, "cdcd");
    }

    #[test]
    fn invalid_payload_is_rejected() {
        let mut payload = sample_payload();
        payload.user_seed.pop();
        let err = embed_image_for_mobile(
            make_png_bytes(),
            payload,
            MobileImageOutputFormat::Png,
            false,
        )
        .unwrap_err();

        assert!(matches!(err, MobileWatermarkError::InvalidPayload(_)));
    }
}

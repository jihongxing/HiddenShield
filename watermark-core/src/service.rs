use serde::{Deserialize, Serialize};

use crate::audio;
use crate::error::WatermarkError;
use crate::image as watermark_image;
use crate::payload::WatermarkPayload;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageOutputFormat {
    Png,
    Jpeg,
    WebP,
    Bmp,
    Tiff,
}

impl Default for ImageOutputFormat {
    fn default() -> Self {
        Self::Png
    }
}

impl From<ImageOutputFormat> for ::image::ImageFormat {
    fn from(value: ImageOutputFormat) -> Self {
        match value {
            ImageOutputFormat::Png => Self::Png,
            ImageOutputFormat::Jpeg => Self::Jpeg,
            ImageOutputFormat::WebP => Self::WebP,
            ImageOutputFormat::Bmp => Self::Bmp,
            ImageOutputFormat::Tiff => Self::Tiff,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WatermarkStrength {
    Balanced,
    Forensic,
}

impl WatermarkStrength {
    pub(crate) fn image_alpha(self) -> f64 {
        match self {
            Self::Balanced => watermark_image::BALANCED_IMAGE_ALPHA,
            Self::Forensic => watermark_image::DEFAULT_IMAGE_ALPHA,
        }
    }

    pub(crate) fn audio_qim_delta(self) -> f32 {
        match self {
            Self::Balanced => audio::BALANCED_QIM_DELTA,
            Self::Forensic => audio::DEFAULT_QIM_DELTA,
        }
    }

    pub(crate) fn extraction_candidates() -> &'static [Self] {
        &[Self::Forensic, Self::Balanced]
    }
}

impl Default for WatermarkStrength {
    fn default() -> Self {
        Self::Forensic
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MediaInput {
    ImageBytes { bytes: Vec<u8> },
    AudioWavBytes { bytes: Vec<u8> },
    AudioSamples { samples: Vec<f32> },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MediaOutput {
    ImageBytes {
        bytes: Vec<u8>,
        format: ImageOutputFormat,
    },
    AudioWavBytes {
        bytes: Vec<u8>,
    },
    AudioSamples {
        samples: Vec<f32>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedOptions {
    pub image_output_format: ImageOutputFormat,
    pub allow_rewrite: bool,
    #[serde(default)]
    pub strength: WatermarkStrength,
}

impl Default for EmbedOptions {
    fn default() -> Self {
        Self {
            image_output_format: ImageOutputFormat::Png,
            allow_rewrite: false,
            strength: WatermarkStrength::default(),
        }
    }
}

pub struct WatermarkService;

impl WatermarkService {
    pub fn embed(
        input: MediaInput,
        payload: &WatermarkPayload,
        options: EmbedOptions,
    ) -> Result<MediaOutput, WatermarkError> {
        let strength = options.strength;
        match input {
            MediaInput::ImageBytes { bytes } => {
                let format = options.image_output_format;
                let bytes = if options.allow_rewrite {
                    watermark_image::embed_image_watermark_bytes_allow_rewrite_with_alpha(
                        &bytes,
                        payload,
                        format.into(),
                        strength.image_alpha(),
                    )?
                } else {
                    watermark_image::embed_image_watermark_bytes_with_alpha(
                        &bytes,
                        payload,
                        format.into(),
                        strength.image_alpha(),
                    )?
                };
                Ok(MediaOutput::ImageBytes { bytes, format })
            }
            MediaInput::AudioWavBytes { bytes } => {
                let bytes = if options.allow_rewrite {
                    audio::embed_watermark_wav_bytes_allow_rewrite_with_delta(
                        &bytes,
                        payload,
                        strength.audio_qim_delta(),
                    )?
                } else {
                    audio::embed_watermark_wav_bytes_with_delta(
                        &bytes,
                        payload,
                        strength.audio_qim_delta(),
                    )?
                };
                Ok(MediaOutput::AudioWavBytes { bytes })
            }
            MediaInput::AudioSamples { mut samples } => {
                if options.allow_rewrite {
                    audio::embed_watermark_samples_allow_rewrite_with_delta(
                        &mut samples,
                        payload,
                        strength.audio_qim_delta(),
                    )?;
                } else {
                    audio::embed_watermark_samples_with_delta(
                        &mut samples,
                        payload,
                        strength.audio_qim_delta(),
                    )?;
                }
                Ok(MediaOutput::AudioSamples { samples })
            }
        }
    }

    pub fn extract(input: MediaInput) -> Result<WatermarkPayload, WatermarkError> {
        match input {
            MediaInput::ImageBytes { bytes } => {
                let mut last_error = None;
                for strength in WatermarkStrength::extraction_candidates() {
                    match watermark_image::extract_image_watermark_bytes_with_alpha(
                        &bytes,
                        strength.image_alpha(),
                    ) {
                        Ok(payload) => return Ok(payload),
                        Err(error) => last_error = Some(error),
                    }
                }
                Err(last_error.unwrap_or_else(|| {
                    WatermarkError::ExtractFailed("no image extraction candidates available".into())
                }))
            }
            MediaInput::AudioWavBytes { bytes } => {
                let mut last_error = None;
                for strength in WatermarkStrength::extraction_candidates() {
                    match audio::extract_watermark_wav_bytes_with_delta(
                        &bytes,
                        strength.audio_qim_delta(),
                    ) {
                        Ok(payload) => return Ok(payload),
                        Err(error) => last_error = Some(error),
                    }
                }
                Err(last_error.unwrap_or_else(|| {
                    WatermarkError::ExtractFailed("no audio extraction candidates available".into())
                }))
            }
            MediaInput::AudioSamples { samples } => {
                let mut last_error = None;
                for strength in WatermarkStrength::extraction_candidates() {
                    match audio::extract_watermark_samples_with_delta(
                        &samples,
                        strength.audio_qim_delta(),
                    ) {
                        Ok(payload) => return Ok(payload),
                        Err(error) => last_error = Some(error),
                    }
                }
                Err(last_error.unwrap_or_else(|| {
                    WatermarkError::ExtractFailed("no audio extraction candidates available".into())
                }))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> WatermarkPayload {
        WatermarkPayload::new(
            [0x42; 8],
            1_700_000_000,
            [0xAB; 4],
            [0xCD; 2],
            Default::default(),
        )
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
        ::image::DynamicImage::ImageRgba8(img)
            .write_to(&mut cursor, ::image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    fn make_wav_bytes() -> Vec<u8> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = std::io::Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for i in 0..8_192 {
            let t = i as f32 / 44_100.0;
            let sample = (t * 440.0 * std::f32::consts::TAU).sin() * 0.2;
            writer.write_sample((sample * 32767.0) as i16).unwrap();
        }
        writer.finalize().unwrap();
        cursor.into_inner()
    }

    #[test]
    fn service_image_roundtrip() {
        let payload = sample_payload();
        let output = WatermarkService::embed(
            MediaInput::ImageBytes {
                bytes: make_png_bytes(),
            },
            &payload,
            EmbedOptions::default(),
        )
        .unwrap();

        let MediaOutput::ImageBytes { bytes, .. } = output else {
            panic!("unexpected output");
        };

        let extracted = WatermarkService::extract(MediaInput::ImageBytes { bytes }).unwrap();
        assert_eq!(extracted.user_seed, payload.user_seed);
        assert_eq!(extracted.device_id, payload.device_id);
    }

    #[test]
    fn service_audio_roundtrip() {
        let payload = sample_payload();
        let output = WatermarkService::embed(
            MediaInput::AudioWavBytes {
                bytes: make_wav_bytes(),
            },
            &payload,
            EmbedOptions::default(),
        )
        .unwrap();

        let MediaOutput::AudioWavBytes { bytes } = output else {
            panic!("unexpected output");
        };

        let extracted = WatermarkService::extract(MediaInput::AudioWavBytes { bytes }).unwrap();
        assert_eq!(extracted.user_seed, payload.user_seed);
        assert_eq!(extracted.device_id, payload.device_id);
    }

    #[test]
    fn service_balanced_strength_roundtrip() {
        let payload = sample_payload();
        let image_output = WatermarkService::embed(
            MediaInput::ImageBytes {
                bytes: make_png_bytes(),
            },
            &payload,
            EmbedOptions {
                strength: WatermarkStrength::Balanced,
                ..EmbedOptions::default()
            },
        )
        .unwrap();
        let MediaOutput::ImageBytes { bytes, .. } = image_output else {
            panic!("unexpected output");
        };
        let extracted = WatermarkService::extract(MediaInput::ImageBytes { bytes }).unwrap();
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());

        let audio_output = WatermarkService::embed(
            MediaInput::AudioWavBytes {
                bytes: make_wav_bytes(),
            },
            &payload,
            EmbedOptions {
                strength: WatermarkStrength::Balanced,
                ..EmbedOptions::default()
            },
        )
        .unwrap();
        let MediaOutput::AudioWavBytes { bytes } = audio_output else {
            panic!("unexpected output");
        };
        let extracted = WatermarkService::extract(MediaInput::AudioWavBytes { bytes }).unwrap();
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
    }

    #[test]
    fn service_rejects_existing_balanced_watermark_by_default() {
        let payload = sample_payload();
        let image_output = WatermarkService::embed(
            MediaInput::ImageBytes {
                bytes: make_png_bytes(),
            },
            &payload,
            EmbedOptions {
                strength: WatermarkStrength::Balanced,
                ..EmbedOptions::default()
            },
        )
        .unwrap();
        let MediaOutput::ImageBytes { bytes, .. } = image_output else {
            panic!("unexpected output");
        };
        let err = WatermarkService::embed(
            MediaInput::ImageBytes { bytes },
            &sample_payload(),
            EmbedOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            WatermarkError::AlreadyWatermarked { existing_uid }
                if existing_uid == payload.watermark_uid()
        ));

        let audio_output = WatermarkService::embed(
            MediaInput::AudioWavBytes {
                bytes: make_wav_bytes(),
            },
            &payload,
            EmbedOptions {
                strength: WatermarkStrength::Balanced,
                ..EmbedOptions::default()
            },
        )
        .unwrap();
        let MediaOutput::AudioWavBytes { bytes } = audio_output else {
            panic!("unexpected output");
        };
        let err = WatermarkService::embed(
            MediaInput::AudioWavBytes { bytes },
            &sample_payload(),
            EmbedOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(
            err,
            WatermarkError::AlreadyWatermarked { existing_uid }
                if existing_uid == payload.watermark_uid()
        ));
    }
}

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
    AudioWavBytes { bytes: Vec<u8> },
    AudioSamples { samples: Vec<f32> },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedOptions {
    pub image_output_format: ImageOutputFormat,
}

impl Default for EmbedOptions {
    fn default() -> Self {
        Self {
            image_output_format: ImageOutputFormat::Png,
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
        match input {
            MediaInput::ImageBytes { bytes } => {
                let format = options.image_output_format;
                let bytes = watermark_image::embed_image_watermark_bytes(&bytes, payload, format.into())?;
                Ok(MediaOutput::ImageBytes { bytes, format })
            }
            MediaInput::AudioWavBytes { bytes } => {
                let bytes = audio::embed_watermark_wav_bytes(&bytes, payload)?;
                Ok(MediaOutput::AudioWavBytes { bytes })
            }
            MediaInput::AudioSamples { mut samples } => {
                audio::embed_watermark_samples(&mut samples, payload)?;
                Ok(MediaOutput::AudioSamples { samples })
            }
        }
    }

    pub fn extract(input: MediaInput) -> Result<WatermarkPayload, WatermarkError> {
        match input {
            MediaInput::ImageBytes { bytes } => watermark_image::extract_image_watermark_bytes(&bytes),
            MediaInput::AudioWavBytes { bytes } => audio::extract_watermark_wav_bytes(&bytes),
            MediaInput::AudioSamples { samples } => audio::extract_watermark_samples(&samples),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_payload() -> WatermarkPayload {
        WatermarkPayload::new([0x42; 8], 1_700_000_000, [0xAB; 4], [0xCD; 2], Default::default())
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
}

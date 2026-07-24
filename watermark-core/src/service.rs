use serde::{Deserialize, Serialize};

use crate::audio;
use crate::error::WatermarkError;
use crate::image as watermark_image;
use crate::payload::{
    PayloadV3MinimalAnchorBuildInput, WatermarkDecodedPayload, WatermarkPayload,
    WatermarkPayloadV3MinimalAnchor,
};

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
    pub(crate) fn image_v3_alpha(self) -> f64 {
        match self {
            Self::Balanced => watermark_image::BALANCED_IMAGE_V3_ALPHA,
            Self::Forensic => watermark_image::DEFAULT_IMAGE_V3_ALPHA,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioProtectionMode {
    StandaloneAudio,
    VideoTrack,
}

impl Default for AudioProtectionMode {
    fn default() -> Self {
        Self::StandaloneAudio
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadWriteMode {
    DefaultV3,
    ForceV2Rollback,
}

impl Default for PayloadWriteMode {
    fn default() -> Self {
        Self::DefaultV3
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmbedOptions {
    pub image_output_format: ImageOutputFormat,
    pub allow_rewrite: bool,
    #[serde(default)]
    pub strength: WatermarkStrength,
    #[serde(default)]
    pub audio_protection_mode: AudioProtectionMode,
    #[serde(default)]
    pub payload_write_mode: PayloadWriteMode,
}

impl Default for EmbedOptions {
    fn default() -> Self {
        Self {
            image_output_format: ImageOutputFormat::Png,
            allow_rewrite: false,
            strength: WatermarkStrength::default(),
            audio_protection_mode: AudioProtectionMode::default(),
            payload_write_mode: PayloadWriteMode::default(),
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
        if options.payload_write_mode == PayloadWriteMode::ForceV2Rollback {
            return Self::embed_v2(input, payload, options);
        }
        Self::embed_v3(input, payload, options)
    }

    pub fn embed_v2(
        input: MediaInput,
        payload: &WatermarkPayload,
        options: EmbedOptions,
    ) -> Result<MediaOutput, WatermarkError> {
        let strength = options.strength;
        match input {
            MediaInput::ImageBytes { .. } => Err(WatermarkError::EmbedFailed(
                "v2_image_rollback_retired: image watermark writing supports V3 only".into(),
            )),
            MediaInput::AudioWavBytes { bytes } => {
                let bytes = if options.allow_rewrite {
                    match options.audio_protection_mode {
                        AudioProtectionMode::StandaloneAudio => {
                            audio::embed_watermark_wav_bytes_allow_rewrite_with_delta(
                                &bytes,
                                payload,
                                strength.audio_qim_delta(),
                            )?
                        }
                        AudioProtectionMode::VideoTrack => {
                            audio::embed_watermark_wav_bytes_allow_rewrite_with_delta_without_min_duration(
                                &bytes,
                                payload,
                                strength.audio_qim_delta(),
                            )?
                        }
                    }
                } else {
                    audio::reject_existing_wav_watermark(&bytes)?;
                    match options.audio_protection_mode {
                        AudioProtectionMode::StandaloneAudio => {
                            audio::embed_watermark_wav_bytes_allow_rewrite_with_delta(
                                &bytes,
                                payload,
                                strength.audio_qim_delta(),
                            )?
                        }
                        AudioProtectionMode::VideoTrack => {
                            audio::embed_watermark_wav_bytes_allow_rewrite_with_delta_without_min_duration(
                                &bytes,
                                payload,
                                strength.audio_qim_delta(),
                            )?
                        }
                    }
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

    pub fn embed_v3(
        input: MediaInput,
        payload: &WatermarkPayload,
        options: EmbedOptions,
    ) -> Result<MediaOutput, WatermarkError> {
        let strength = options.strength;
        let anchor = v3_anchor_from_v2_payload(payload)?;
        match input {
            MediaInput::ImageBytes { bytes } => {
                let format = options.image_output_format;
                if !options.allow_rewrite {
                    reject_existing_image_decoded(&bytes)?;
                }
                let bytes = watermark_image::embed_image_v3_bytes(
                    &bytes,
                    &anchor,
                    format.into(),
                    strength.image_v3_alpha(),
                )?;
                Ok(MediaOutput::ImageBytes { bytes, format })
            }
            MediaInput::AudioWavBytes { bytes } => {
                if !options.allow_rewrite {
                    reject_existing_wav_decoded(&bytes, strength.audio_qim_delta())?;
                }
                let bytes = match options.audio_protection_mode {
                    AudioProtectionMode::StandaloneAudio => {
                        audio::embed_audio_v3_internal_qa_wav_bytes(&bytes, &anchor)?
                    }
                    AudioProtectionMode::VideoTrack => {
                        audio::embed_audio_v3_internal_qa_wav_bytes_without_min_duration(
                            &bytes, &anchor,
                        )?
                    }
                };
                Ok(MediaOutput::AudioWavBytes { bytes })
            }
            MediaInput::AudioSamples { mut samples } => {
                if !options.allow_rewrite {
                    reject_existing_samples_decoded(&samples, strength.audio_qim_delta())?;
                }
                audio::embed_watermark_samples_v3_default(&mut samples, &anchor)?;
                Ok(MediaOutput::AudioSamples { samples })
            }
        }
    }

    pub fn extract(input: MediaInput) -> Result<WatermarkDecodedPayload, WatermarkError> {
        match input {
            MediaInput::ImageBytes { bytes } => {
                require_v3_default(watermark_image::extract_image_v3_bytes(&bytes)?)
            }
            MediaInput::AudioWavBytes { bytes } => {
                let mut last_error = None;
                for strength in WatermarkStrength::extraction_candidates() {
                    match audio::extract_watermark_wav_readonly_candidate_bytes_with_delta(
                        &bytes,
                        strength.audio_qim_delta(),
                    ) {
                        Ok(payload) => match require_v3_default(payload) {
                            Ok(v3) => return Ok(v3),
                            Err(error) => last_error = Some(error),
                        },
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
                    match audio::extract_watermark_samples_readonly_candidate_with_delta(
                        &samples,
                        strength.audio_qim_delta(),
                    ) {
                        Ok(payload) => match require_v3_default(payload) {
                            Ok(v3) => return Ok(v3),
                            Err(error) => last_error = Some(error),
                        },
                        Err(error) => last_error = Some(error),
                    }
                }
                Err(last_error.unwrap_or_else(|| {
                    WatermarkError::ExtractFailed("no audio extraction candidates available".into())
                }))
            }
        }
    }

    pub fn extract_v2(input: MediaInput) -> Result<WatermarkPayload, WatermarkError> {
        match input {
            MediaInput::ImageBytes { .. } => Err(WatermarkError::ExtractFailed(
                "v2_image_rollback_retired: image watermark reading supports V3 only".into(),
            )),
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

fn v3_anchor_from_v2_payload(
    payload: &WatermarkPayload,
) -> Result<WatermarkPayloadV3MinimalAnchor, WatermarkError> {
    WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput {
        watermark_id: payload.watermark_id,
    })
}

fn reject_existing_image_decoded(image_bytes: &[u8]) -> Result<(), WatermarkError> {
    if let Ok(decoded) = watermark_image::extract_image_v3_bytes(image_bytes) {
        if decoded.is_v3_minimal_anchor() {
            return Err(WatermarkError::AlreadyWatermarked {
                existing_uid: decoded.watermark_uid(),
            });
        }
    }
    Ok(())
}

fn reject_existing_wav_decoded(input_wav: &[u8], delta: f32) -> Result<(), WatermarkError> {
    if let Ok(decoded) =
        audio::extract_watermark_wav_readonly_candidate_bytes_with_delta(input_wav, delta)
    {
        if decoded.is_v3_minimal_anchor() {
            return Err(WatermarkError::AlreadyWatermarked {
                existing_uid: decoded.watermark_uid(),
            });
        }
    }
    Ok(())
}

fn reject_existing_samples_decoded(samples: &[f32], delta: f32) -> Result<(), WatermarkError> {
    if let Ok(decoded) =
        audio::extract_watermark_samples_readonly_candidate_with_delta(samples, delta)
    {
        if decoded.is_v3_minimal_anchor() {
            return Err(WatermarkError::AlreadyWatermarked {
                existing_uid: decoded.watermark_uid(),
            });
        }
    }
    Ok(())
}

fn require_v3_default(
    decoded: WatermarkDecodedPayload,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    if decoded.is_v3_minimal_anchor() {
        Ok(decoded)
    } else {
        Err(WatermarkError::ExtractFailed(
            "default payload reader expects V3/39 minimal anchor; V2 is rollback-only".into(),
        ))
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
        let img = image::RgbaImage::from_fn(512, 512, |x, y| {
            image::Rgba([
                ((x as f32 / 512.0 * 200.0) as u8).wrapping_add(30),
                ((y as f32 / 512.0 * 200.0) as u8).wrapping_add(30),
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
        make_wav_bytes_with_seconds(audio::MIN_AUDIO_PROTECTION_SECONDS)
    }

    fn make_wav_bytes_with_seconds(seconds: u32) -> Vec<u8> {
        make_wav_bytes_with_spec(seconds, 44_100, 1)
    }

    fn make_wav_bytes_with_spec(seconds: u32, sample_rate: u32, channels: u16) -> Vec<u8> {
        let spec = hound::WavSpec {
            channels,
            sample_rate,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = std::io::Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
        for i in 0..(sample_rate * seconds) {
            let t = i as f32 / sample_rate as f32;
            for channel in 0..channels {
                let frequency = if channel == 0 { 440.0 } else { 880.0 };
                let sample = (t * frequency * std::f32::consts::TAU).sin() * 0.2;
                writer.write_sample((sample * 32767.0) as i16).unwrap();
            }
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
        assert!(extracted.is_v3_minimal_anchor());
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
        assert_eq!(extracted.protocol_version(), 3);
        assert_eq!(extracted.payload_bytes_length(), 39);
    }

    #[test]
    fn service_image_v3_default_roundtrips_photo_gradient_png() {
        let source = image::ImageBuffer::from_fn(1024, 1024, |x, y| {
            image::Rgb([
                (x * 255 / 1024) as u8,
                (y * 255 / 1024) as u8,
                (((x * 3 + y * 5) & 0xff) as u8).saturating_add(12),
            ])
        });
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(source)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        let payload = sample_payload();
        let output = WatermarkService::embed(
            MediaInput::ImageBytes {
                bytes: cursor.into_inner(),
            },
            &payload,
            EmbedOptions {
                image_output_format: ImageOutputFormat::Png,
                allow_rewrite: true,
                ..EmbedOptions::default()
            },
        )
        .unwrap();
        let MediaOutput::ImageBytes { bytes, .. } = output else {
            panic!("unexpected output");
        };

        let extracted = WatermarkService::extract(MediaInput::ImageBytes { bytes }).unwrap();

        assert!(extracted.is_v3_minimal_anchor());
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
    }

    #[test]
    fn service_image_v3_default_recovers_from_each_sixteenth_crop() {
        let source = image::ImageBuffer::from_fn(1920, 1080, |x, y| {
            image::Rgb([
                ((x * 13 + y * 3) & 0xff) as u8,
                ((x * 5 + y * 11) & 0xff) as u8,
                ((x ^ y) & 0xff) as u8,
            ])
        });
        let mut source_cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(source)
            .write_to(&mut source_cursor, image::ImageFormat::Png)
            .unwrap();
        let payload = sample_payload();
        let output = WatermarkService::embed(
            MediaInput::ImageBytes {
                bytes: source_cursor.into_inner(),
            },
            &payload,
            EmbedOptions {
                image_output_format: ImageOutputFormat::Png,
                allow_rewrite: true,
                ..EmbedOptions::default()
            },
        )
        .unwrap();
        let MediaOutput::ImageBytes { bytes, .. } = output else {
            panic!("unexpected output");
        };
        let protected = image::load_from_memory(&bytes).unwrap().to_rgba8();

        for row in 0..4 {
            for column in 0..4 {
                let x = column * protected.width() / 4;
                let y = row * protected.height() / 4;
                let right = (column + 1) * protected.width() / 4;
                let bottom = (row + 1) * protected.height() / 4;
                let crop =
                    image::imageops::crop_imm(&protected, x, y, right - x, bottom - y).to_image();
                let mut crop_cursor = std::io::Cursor::new(Vec::new());
                image::DynamicImage::ImageRgba8(crop)
                    .write_to(&mut crop_cursor, image::ImageFormat::Png)
                    .unwrap();
                let extracted = WatermarkService::extract(MediaInput::ImageBytes {
                    bytes: crop_cursor.into_inner(),
                })
                .unwrap();

                assert!(extracted.is_v3_minimal_anchor());
                assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
            }
        }
    }

    #[test]
    fn service_image_v3_default_recovers_after_right_angle_rotations() {
        let (protected, expected_uid) = make_protected_image(1920, 1080);
        for transformed in [
            protected.rotate90(),
            protected.rotate180(),
            protected.rotate270(),
        ] {
            let extracted = extract_dynamic_image(transformed).unwrap();
            assert_eq!(extracted.watermark_uid(), expected_uid);
        }
    }

    #[test]
    fn service_image_v3_default_recovers_after_eighty_five_percent_scaling() {
        let (protected, expected_uid) = make_protected_image(1920, 1080);
        let scaled = protected.resize_exact(
            protected.width() * 85 / 100,
            protected.height() * 85 / 100,
            ::image::imageops::FilterType::Lanczos3,
        );

        let extracted = extract_dynamic_image(scaled).unwrap();

        assert_eq!(extracted.watermark_uid(), expected_uid);
    }

    #[test]
    fn service_image_v3_default_recovers_after_jpeg_recompression() {
        let (protected, expected_uid) = make_protected_image(1920, 1080);
        for quality in [75, 60] {
            let mut bytes = Vec::new();
            ::image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, quality)
                .encode_image(&protected)
                .unwrap();
            let extracted = WatermarkService::extract(MediaInput::ImageBytes { bytes }).unwrap();
            assert_eq!(extracted.watermark_uid(), expected_uid);
        }
    }

    fn make_protected_image(width: u32, height: u32) -> (::image::DynamicImage, String) {
        let source = ::image::ImageBuffer::from_fn(width, height, |x, y| {
            ::image::Rgb([
                ((x * 13 + y * 3) & 0xff) as u8,
                ((x * 5 + y * 11) & 0xff) as u8,
                ((x ^ y) & 0xff) as u8,
            ])
        });
        let mut source_cursor = std::io::Cursor::new(Vec::new());
        ::image::DynamicImage::ImageRgb8(source)
            .write_to(&mut source_cursor, ::image::ImageFormat::Png)
            .unwrap();
        let payload = sample_payload();
        let expected_uid = payload.watermark_uid();
        let output = WatermarkService::embed(
            MediaInput::ImageBytes {
                bytes: source_cursor.into_inner(),
            },
            &payload,
            EmbedOptions {
                image_output_format: ImageOutputFormat::Png,
                allow_rewrite: true,
                ..EmbedOptions::default()
            },
        )
        .unwrap();
        let MediaOutput::ImageBytes { bytes, .. } = output else {
            panic!("unexpected output");
        };
        (::image::load_from_memory(&bytes).unwrap(), expected_uid)
    }

    fn extract_dynamic_image(
        image: ::image::DynamicImage,
    ) -> Result<WatermarkDecodedPayload, WatermarkError> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        image
            .write_to(&mut cursor, ::image::ImageFormat::Png)
            .unwrap();
        WatermarkService::extract(MediaInput::ImageBytes {
            bytes: cursor.into_inner(),
        })
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
        assert!(extracted.is_v3_minimal_anchor());
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
        assert_eq!(extracted.protocol_version(), 3);
        assert_eq!(extracted.payload_bytes_length(), 39);
    }

    #[test]
    fn service_audio_roundtrip_preserves_48000_mono_spec() {
        assert_audio_roundtrip_preserves_spec(48_000, 1);
    }

    #[test]
    fn service_audio_roundtrip_preserves_48000_stereo_spec() {
        assert_audio_roundtrip_preserves_spec(48_000, 2);
    }

    fn assert_audio_roundtrip_preserves_spec(sample_rate: u32, channels: u16) {
        let payload = sample_payload();
        let output = WatermarkService::embed(
            MediaInput::AudioWavBytes {
                bytes: make_wav_bytes_with_spec(
                    audio::MIN_AUDIO_PROTECTION_SECONDS + 1,
                    sample_rate,
                    channels,
                ),
            },
            &payload,
            EmbedOptions::default(),
        )
        .unwrap();

        let MediaOutput::AudioWavBytes { bytes } = output else {
            panic!("unexpected output");
        };
        let reader = hound::WavReader::new(std::io::Cursor::new(&bytes)).unwrap();
        assert_eq!(reader.spec().sample_rate, sample_rate);
        assert_eq!(reader.spec().channels, channels);

        let extracted = WatermarkService::extract(MediaInput::AudioWavBytes { bytes }).unwrap();
        assert!(extracted.is_v3_minimal_anchor());
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
    }

    #[test]
    fn service_rejects_short_standalone_audio_but_allows_video_track_mode() {
        let payload = sample_payload();
        let short_wav = make_wav_bytes_with_seconds(10);

        let standalone_error = WatermarkService::embed(
            MediaInput::AudioWavBytes {
                bytes: short_wav.clone(),
            },
            &payload,
            EmbedOptions::default(),
        )
        .unwrap_err();
        assert!(matches!(
            standalone_error,
            WatermarkError::EmbedFailed(message)
                if message.contains("audio_protection_min_duration")
        ));

        let video_track_output = WatermarkService::embed(
            MediaInput::AudioWavBytes { bytes: short_wav },
            &payload,
            EmbedOptions {
                audio_protection_mode: AudioProtectionMode::VideoTrack,
                ..EmbedOptions::default()
            },
        )
        .unwrap();
        let MediaOutput::AudioWavBytes { bytes } = video_track_output else {
            panic!("unexpected output");
        };
        let extracted = WatermarkService::extract(MediaInput::AudioWavBytes { bytes }).unwrap();
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
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

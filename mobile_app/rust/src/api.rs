use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Cursor;
use symphonia::core::audio::{AudioBufferRef, Signal};
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::conv::IntoSample;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;
use thiserror::Error;
use watermark_core::{
    decode_watermark_payload_readonly, AIContentFlags, ImageOutputFormat, MediaInput, MediaOutput,
    PayloadV2BuildInput, TrainingPermission, WatermarkDecodedPayload, WatermarkError,
    WatermarkIssueMode, WatermarkMediaType, WatermarkPayload, WatermarkService,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobileMediaPayload {
    pub creator_identity: String,
    pub device_identity: String,
    pub media_bytes: Vec<u8>,
    pub timestamp: u64,
    pub reserved_watermark_uid: Option<String>,
    pub registry_proof_hash: Option<String>,
    pub parent_watermark_uid: Option<String>,
    pub revision: u32,
    pub media_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobileImageResult {
    pub bytes: Vec<u8>,
    pub watermark_uid: String,
    pub sha256: String,
    pub format: MobileImageOutputFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobileAudioResult {
    pub bytes: Vec<u8>,
    pub watermark_uid: String,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobileV3InternalQaWriteResult {
    pub bytes: Vec<u8>,
    pub watermark_uid: String,
    pub sha256: String,
    pub media_type: String,
    pub payload_protocol_version: u32,
    pub payload_bytes_length: u32,
    pub payload_auth_status: String,
    pub watermark_id_issue_mode: String,
    pub media_payload_role: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MobileExtractResult {
    pub watermark_uid: String,
    pub timestamp: u64,
    pub device_id_hex: String,
    pub file_hash_hex: String,
    pub parent_watermark_uid: Option<String>,
    pub revision: u32,
    pub payload_protocol_version: u32,
    pub payload_bytes_length: u32,
    pub watermark_id_issue_mode: String,
    pub media_type: String,
    pub payload_auth_status: String,
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
    #[error("invalid payload [{code}]: {message}")]
    InvalidPayload { code: String, message: String },

    #[error("watermark operation failed [{code}]: {message}")]
    OperationFailed {
        code: String,
        message: String,
        existing_uid: Option<String>,
    },
}

impl MobileWatermarkError {
    fn from_core(error: watermark_core::WatermarkError) -> Self {
        let code = error.code_str().to_string();
        let message = error.to_string();
        match error {
            watermark_core::WatermarkError::InvalidPayload { .. } => {
                Self::InvalidPayload { code, message }
            }
            other => Self::OperationFailed {
                code,
                message,
                existing_uid: other.existing_uid().map(ToString::to_string),
            },
        }
    }

    fn operation_failed(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::OperationFailed {
            code: code.into(),
            message: message.into(),
            existing_uid: None,
        }
    }
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
            ..watermark_core::EmbedOptions::default()
        },
    )
    .map_err(MobileWatermarkError::from_core)?;

    match output {
        MediaOutput::ImageBytes { bytes, .. } => Ok(MobileImageResult {
            sha256: sha256_hex(&bytes),
            watermark_uid: payload.watermark_uid(),
            bytes,
            format: output_format,
        }),
        _ => Err(MobileWatermarkError::operation_failed(
            "unexpected_output",
            "unexpected non-image output",
        )),
    }
}

pub fn extract_image_for_mobile(
    image_bytes: Vec<u8>,
) -> Result<MobileExtractResult, MobileWatermarkError> {
    let payload = WatermarkService::extract(MediaInput::ImageBytes { bytes: image_bytes })
        .map_err(MobileWatermarkError::from_core)?;
    Ok(mobile_extract_result_from_decoded(payload, "image"))
}

pub fn extract_image_readonly_candidate_for_mobile(
    image_bytes: Vec<u8>,
) -> Result<MobileExtractResult, MobileWatermarkError> {
    let decoded = watermark_core::extract_image_watermark_readonly_candidate_bytes(&image_bytes)
        .map_err(MobileWatermarkError::from_core)?;
    Ok(mobile_extract_result_from_decoded(decoded, "image"))
}

pub fn detect_existing_image_for_mobile(
    image_bytes: Vec<u8>,
) -> Result<Option<MobileExtractResult>, MobileWatermarkError> {
    match WatermarkService::extract(MediaInput::ImageBytes {
        bytes: image_bytes.clone(),
    }) {
        Ok(decoded) => return Ok(Some(mobile_extract_result_from_decoded(decoded, "image"))),
        Err(WatermarkError::ExtractFailed(_)) => {}
        Err(error) => return Err(MobileWatermarkError::from_core(error)),
    }

    match watermark_core::extract_image_watermark_readonly_candidate_bytes(&image_bytes) {
        Ok(decoded) => Ok(Some(mobile_extract_result_from_decoded(decoded, "image"))),
        Err(WatermarkError::ExtractFailed(_)) => Ok(None),
        Err(error) => Err(MobileWatermarkError::from_core(error)),
    }
}

pub fn embed_audio_wav_for_mobile(
    audio_bytes: Vec<u8>,
    payload: MobileMediaPayload,
    allow_rewrite: bool,
) -> Result<MobileAudioResult, MobileWatermarkError> {
    let payload = payload.into_core_payload()?;
    let audio_bytes = normalize_audio_to_wav(audio_bytes)?;
    let output = WatermarkService::embed(
        MediaInput::AudioWavBytes { bytes: audio_bytes },
        &payload,
        watermark_core::EmbedOptions {
            image_output_format: ImageOutputFormat::Png,
            allow_rewrite,
            ..watermark_core::EmbedOptions::default()
        },
    )
    .map_err(MobileWatermarkError::from_core)?;

    match output {
        MediaOutput::AudioWavBytes { bytes } => Ok(MobileAudioResult {
            sha256: sha256_hex(&bytes),
            watermark_uid: payload.watermark_uid(),
            bytes,
        }),
        _ => Err(MobileWatermarkError::operation_failed(
            "unexpected_output",
            "unexpected non-audio output",
        )),
    }
}

pub fn extract_audio_wav_for_mobile(
    audio_bytes: Vec<u8>,
) -> Result<MobileExtractResult, MobileWatermarkError> {
    let audio_bytes = normalize_audio_to_wav(audio_bytes)?;
    let payload = WatermarkService::extract(MediaInput::AudioWavBytes { bytes: audio_bytes })
        .map_err(MobileWatermarkError::from_core)?;
    Ok(mobile_extract_result_from_decoded(payload, "audio"))
}

pub fn extract_audio_wav_readonly_candidate_for_mobile(
    audio_bytes: Vec<u8>,
) -> Result<MobileExtractResult, MobileWatermarkError> {
    let audio_bytes = normalize_audio_to_wav(audio_bytes)?;
    let decoded = watermark_core::extract_watermark_wav_readonly_candidate_bytes(&audio_bytes)
        .map_err(MobileWatermarkError::from_core)?;
    Ok(mobile_extract_result_from_decoded(decoded, "audio"))
}

pub fn embed_v3_internal_qa_for_mobile(
    media_bytes: Vec<u8>,
    media_type: String,
    watermark_uid: String,
) -> Result<MobileV3InternalQaWriteResult, MobileWatermarkError> {
    let watermark_id = parse_watermark_uid(&watermark_uid)?;
    let (media_kind, normalized_bytes) = match media_type.as_str() {
        "image" => (watermark_core::V3InternalQaMediaKind::Image, media_bytes),
        "audio" => (
            watermark_core::V3InternalQaMediaKind::Audio,
            normalize_audio_to_wav(media_bytes)?,
        ),
        other => {
            return Err(MobileWatermarkError::operation_failed(
                "unsupported_v3_internal_qa_media_type",
                format!("unsupported V3 internal QA media type: {other}"),
            ))
        }
    };
    let output = watermark_core::embed_v3_internal_qa_media(
        watermark_core::V3InternalQaWriteGate::InternalQa,
        watermark_core::V3InternalQaWriteInput {
            media_kind,
            media_bytes: normalized_bytes,
            watermark_id,
        },
    )
    .map_err(MobileWatermarkError::from_core)?;
    Ok(MobileV3InternalQaWriteResult {
        sha256: sha256_hex(&output.bytes),
        bytes: output.bytes,
        watermark_uid: output.watermark_uid,
        media_type,
        payload_protocol_version: output.payload_protocol_version as u32,
        payload_bytes_length: output.payload_bytes_length as u32,
        payload_auth_status: output.payload_auth_status,
        watermark_id_issue_mode: "registry_resolved".to_string(),
        media_payload_role: output.media_payload_role,
    })
}

pub fn decode_v3_readonly_fixture_for_mobile(
    payload_bytes: Vec<u8>,
    media_type: String,
) -> Result<MobileExtractResult, MobileWatermarkError> {
    let decoded = decode_watermark_payload_readonly(&payload_bytes)
        .map_err(MobileWatermarkError::from_core)?;
    match decoded {
        WatermarkDecodedPayload::V3MinimalAnchor(anchor) => Ok(MobileExtractResult {
            watermark_uid: anchor.watermark_uid(),
            timestamp: 0,
            device_id_hex: hex::encode(anchor.watermark_id),
            file_hash_hex: String::new(),
            parent_watermark_uid: None,
            revision: 0,
            payload_protocol_version: anchor.protocol_version as u32,
            payload_bytes_length: watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32,
            watermark_id_issue_mode: "registry_resolved".to_string(),
            media_type,
            payload_auth_status: "verified".to_string(),
        }),
        WatermarkDecodedPayload::V2(_) => Err(MobileWatermarkError::operation_failed(
            "v3_fixture_expected",
            "expected V3 minimal anchor fixture bytes",
        )),
    }
}

pub fn decode_v3_readonly_media_fixture_for_mobile(
    media_bytes: Vec<u8>,
    media_type: String,
) -> Result<MobileExtractResult, MobileWatermarkError> {
    let decoded = match media_type.as_str() {
        "image" => watermark_core::extract_v3_readonly_anchor_png_bytes(&media_bytes),
        "audio" => watermark_core::extract_v3_readonly_anchor_wav_bytes(&media_bytes),
        _ => {
            return Err(MobileWatermarkError::operation_failed(
                "unsupported_v3_readonly_fixture_media_type",
                format!("unsupported V3 readonly fixture media type: {media_type}"),
            ))
        }
    }
    .map_err(MobileWatermarkError::from_core)?;
    match decoded {
        WatermarkDecodedPayload::V3MinimalAnchor(anchor) => Ok(MobileExtractResult {
            watermark_uid: anchor.watermark_uid(),
            timestamp: 0,
            device_id_hex: hex::encode(anchor.watermark_id),
            file_hash_hex: String::new(),
            parent_watermark_uid: None,
            revision: 0,
            payload_protocol_version: anchor.protocol_version as u32,
            payload_bytes_length: watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32,
            watermark_id_issue_mode: "registry_resolved".to_string(),
            media_type,
            payload_auth_status: "verified".to_string(),
        }),
        WatermarkDecodedPayload::V2(_) => Err(MobileWatermarkError::operation_failed(
            "v3_fixture_expected",
            "expected V3 minimal anchor fixture media",
        )),
    }
}

fn normalize_audio_to_wav(audio_bytes: Vec<u8>) -> Result<Vec<u8>, MobileWatermarkError> {
    if is_wav_bytes(&audio_bytes) {
        return Ok(audio_bytes);
    }

    decode_audio_to_wav(&audio_bytes)
}

fn is_wav_bytes(bytes: &[u8]) -> bool {
    bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WAVE"
}

fn decode_audio_to_wav(bytes: &[u8]) -> Result<Vec<u8>, MobileWatermarkError> {
    let cursor = Cursor::new(bytes.to_vec());
    let media_source = Box::new(cursor);
    let media_source_stream = MediaSourceStream::new(media_source, Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = audio_extension_hint_from_bytes(bytes) {
        hint.with_extension(extension);
    }
    let probed = symphonia::default::get_probe()
        .format(
            &hint,
            media_source_stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .map_err(|error| {
            MobileWatermarkError::operation_failed(
                "audio_decode_failed",
                format!("decode audio container: {error}"),
            )
        })?;

    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|track| track.codec_params.sample_rate.is_some())
        .or_else(|| format.default_track())
        .ok_or_else(|| {
            MobileWatermarkError::operation_failed("audio_track_missing", "audio track not found")
        })?;
    let track_id = track.id;
    let codec_params = track.codec_params.clone();
    let source_sample_rate = codec_params.sample_rate.ok_or_else(|| {
        MobileWatermarkError::operation_failed(
            "audio_sample_rate_missing",
            "audio sample rate missing",
        )
    })?;
    let mut decoder = symphonia::default::get_codecs()
        .make(&codec_params, &DecoderOptions::default())
        .map_err(|error| {
            MobileWatermarkError::operation_failed(
                "audio_decode_failed",
                format!("decode audio: {error}"),
            )
        })?;

    let mut interleaved_samples = Vec::<f32>::new();
    let mut decoded_channels: Option<u16> = None;
    loop {
        let packet = match format.next_packet() {
            Ok(packet) => packet,
            Err(symphonia::core::errors::Error::IoError(error))
                if error.kind() == std::io::ErrorKind::UnexpectedEof =>
            {
                break
            }
            Err(symphonia::core::errors::Error::ResetRequired) => {
                return Err(MobileWatermarkError::operation_failed(
                    "audio_decode_failed",
                    "audio decoder reset required",
                ));
            }
            Err(error) => {
                return Err(MobileWatermarkError::operation_failed(
                    "audio_decode_failed",
                    format!("read audio packet: {error}"),
                ));
            }
        };

        if packet.track_id() != track_id {
            continue;
        }

        let decoded = match decoder.decode(&packet) {
            Ok(decoded) => decoded,
            Err(symphonia::core::errors::Error::DecodeError(_)) => continue,
            Err(error) => {
                return Err(MobileWatermarkError::operation_failed(
                    "audio_decode_failed",
                    format!("decode audio packet: {error}"),
                ));
            }
        };
        let packet_channels = decoded
            .spec()
            .channels
            .count()
            .max(1)
            .min(u16::MAX as usize) as u16;
        if let Some(expected_channels) = decoded_channels {
            if expected_channels != packet_channels {
                return Err(MobileWatermarkError::operation_failed(
                    "audio_channel_layout_changed",
                    format!(
                        "audio channel layout changed from {expected_channels} to {packet_channels}"
                    ),
                ));
            }
        } else {
            decoded_channels = Some(packet_channels);
        }
        append_interleaved_samples(decoded, &mut interleaved_samples);
    }

    if interleaved_samples.is_empty() {
        return Err(MobileWatermarkError::operation_failed(
            "audio_decode_failed",
            "decoded audio is empty",
        ));
    }

    write_interleaved_wav(
        &interleaved_samples,
        source_sample_rate,
        decoded_channels.unwrap_or(1),
    )
}

fn audio_extension_hint_from_bytes(bytes: &[u8]) -> Option<&'static str> {
    if bytes.starts_with(b"fLaC") {
        return Some("flac");
    }
    if bytes.starts_with(b"OggS") {
        return Some("ogg");
    }
    if bytes
        .windows(2)
        .take(4)
        .any(|window| window[0] == 0xFF && (window[1] & 0x06) == 0x00)
    {
        return Some("aac");
    }
    if bytes.starts_with(b"ID3")
        || bytes
            .windows(2)
            .take(4)
            .any(|window| window[0] == 0xFF && (window[1] & 0xE0) == 0xE0)
    {
        return Some("mp3");
    }
    if bytes.len() >= 12 && &bytes[4..8] == b"ftyp" {
        return Some("m4a");
    }
    None
}

fn append_interleaved_samples(decoded: AudioBufferRef<'_>, out: &mut Vec<f32>) {
    match decoded {
        AudioBufferRef::U8(buffer) => append_interleaved_from_buffer(&buffer, out),
        AudioBufferRef::U16(buffer) => append_interleaved_from_buffer(&buffer, out),
        AudioBufferRef::U24(buffer) => append_interleaved_from_buffer(&buffer, out),
        AudioBufferRef::U32(buffer) => append_interleaved_from_buffer(&buffer, out),
        AudioBufferRef::S8(buffer) => append_interleaved_from_buffer(&buffer, out),
        AudioBufferRef::S16(buffer) => append_interleaved_from_buffer(&buffer, out),
        AudioBufferRef::S24(buffer) => append_interleaved_from_buffer(&buffer, out),
        AudioBufferRef::S32(buffer) => append_interleaved_from_buffer(&buffer, out),
        AudioBufferRef::F32(buffer) => append_interleaved_from_buffer(&buffer, out),
        AudioBufferRef::F64(buffer) => append_interleaved_from_buffer(&buffer, out),
    }
}

fn append_interleaved_from_buffer<S>(
    buffer: &symphonia::core::audio::AudioBuffer<S>,
    out: &mut Vec<f32>,
) where
    S: symphonia::core::sample::Sample + IntoSample<f32>,
{
    let channels = buffer.spec().channels.count();
    let frames = buffer.frames();
    for frame in 0..frames {
        for channel in 0..channels {
            out.push(buffer.chan(channel)[frame].into_sample());
        }
    }
}

fn write_interleaved_wav(
    samples: &[f32],
    sample_rate: u32,
    channels: u16,
) -> Result<Vec<u8>, MobileWatermarkError> {
    let spec = hound::WavSpec {
        channels,
        sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = hound::WavWriter::new(&mut cursor, spec).map_err(|error| {
            MobileWatermarkError::operation_failed(
                "audio_normalize_failed",
                format!("create normalized wav: {error}"),
            )
        })?;
        for sample in samples {
            let sample = sample.clamp(-1.0, 1.0);
            writer
                .write_sample((sample * i16::MAX as f32) as i16)
                .map_err(|error| {
                    MobileWatermarkError::operation_failed(
                        "audio_normalize_failed",
                        format!("write normalized wav: {error}"),
                    )
                })?;
        }
        writer.finalize().map_err(|error| {
            MobileWatermarkError::operation_failed(
                "audio_normalize_failed",
                format!("finalize normalized wav: {error}"),
            )
        })?;
    }
    Ok(cursor.into_inner())
}

impl MobileMediaPayload {
    fn into_core_payload(self) -> Result<WatermarkPayload, MobileWatermarkError> {
        if self.creator_identity.trim().is_empty() {
            return Err(MobileWatermarkError::InvalidPayload {
                code: "missing_creator_identity".to_string(),
                message: "creator identity is required".to_string(),
            });
        }
        if self.device_identity.trim().is_empty() {
            return Err(MobileWatermarkError::InvalidPayload {
                code: "missing_device_identity".to_string(),
                message: "device identity is required".to_string(),
            });
        }

        let (watermark_id, issue_mode, registry_proof_hash) =
            if let Some(uid) = self.reserved_watermark_uid.as_deref() {
                (
                    parse_watermark_uid(uid)?,
                    WatermarkIssueMode::ServerReserved,
                    self.registry_proof_hash
                        .as_deref()
                        .map(parse_hex_16)
                        .transpose()?,
                )
            } else {
                (
                    watermark_core::generate_offline_watermark_id()
                        .map_err(MobileWatermarkError::from_core)?,
                    WatermarkIssueMode::OfflineGenerated,
                    None,
                )
            };
        let parent_watermark_id = self
            .parent_watermark_uid
            .as_deref()
            .map(parse_watermark_uid)
            .transpose()?;
        let media_sha256: [u8; 32] = Sha256::digest(&self.media_bytes).into();

        WatermarkPayload::from_v2(PayloadV2BuildInput {
            watermark_id,
            parent_watermark_id,
            revision: self.revision.max(1),
            issued_at: self.timestamp,
            original_sha256: media_sha256,
            ai_flags: default_ai_flags(),
            issue_mode,
            media_type: parse_media_type(self.media_type.as_deref()),
            registry_proof_hash,
            creator_binding: Some(&self.creator_identity),
        })
        .map_err(MobileWatermarkError::from_core)
    }
}

fn parse_watermark_uid(uid: &str) -> Result<[u8; 16], MobileWatermarkError> {
    let compact = uid
        .trim()
        .strip_prefix("HS-")
        .unwrap_or(uid.trim())
        .replace('-', "");
    let bytes = hex::decode(compact).map_err(|error| {
        MobileWatermarkError::operation_failed(
            "invalid_watermark_uid",
            format!("invalid watermark uid: {error}"),
        )
    })?;
    if bytes.len() != 16 {
        return Err(MobileWatermarkError::operation_failed(
            "invalid_watermark_uid",
            "invalid watermark uid length",
        ));
    }
    let mut output = [0u8; 16];
    output.copy_from_slice(&bytes);
    Ok(output)
}

fn parse_hex_16(value: &str) -> Result<[u8; 16], MobileWatermarkError> {
    let bytes = hex::decode(value.trim()).map_err(|error| {
        MobileWatermarkError::operation_failed(
            "invalid_registry_proof_hash",
            format!("invalid registry proof hash: {error}"),
        )
    })?;
    if bytes.len() != 16 {
        return Err(MobileWatermarkError::operation_failed(
            "invalid_registry_proof_hash",
            "invalid registry proof hash length",
        ));
    }
    let mut output = [0u8; 16];
    output.copy_from_slice(&bytes);
    Ok(output)
}

fn parse_media_type(value: Option<&str>) -> WatermarkMediaType {
    match value.unwrap_or_default() {
        "image" => WatermarkMediaType::Image,
        "audio" => WatermarkMediaType::Audio,
        "video_audio_track" => WatermarkMediaType::VideoAudioTrack,
        "video_visual" => WatermarkMediaType::VideoVisual,
        _ => WatermarkMediaType::Unknown,
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
            timestamp: self.issued_at,
            device_id_hex: hex::encode(self.watermark_id),
            file_hash_hex: hex::encode(self.original_hash_prefix),
            parent_watermark_uid: self.parent_watermark_uid(),
            revision: self.revision,
            payload_protocol_version: self.protocol_version as u32,
            payload_bytes_length: watermark_core::PAYLOAD_BYTES as u32,
            watermark_id_issue_mode: issue_mode_label(self.issue_mode).to_string(),
            media_type: media_type_label(self.media_type).to_string(),
            payload_auth_status: "verified".to_string(),
        }
    }
}

fn mobile_extract_result_from_decoded(
    decoded: WatermarkDecodedPayload,
    fallback_media_type: &str,
) -> MobileExtractResult {
    match decoded {
        WatermarkDecodedPayload::V2(payload) => payload.into_mobile_extract_result(),
        WatermarkDecodedPayload::V3MinimalAnchor(anchor) => MobileExtractResult {
            watermark_uid: anchor.watermark_uid(),
            timestamp: 0,
            device_id_hex: hex::encode(anchor.watermark_id),
            file_hash_hex: String::new(),
            parent_watermark_uid: None,
            revision: 0,
            payload_protocol_version: anchor.protocol_version as u32,
            payload_bytes_length: watermark_core::PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u32,
            watermark_id_issue_mode: "registry_resolved".to_string(),
            media_type: fallback_media_type.to_string(),
            payload_auth_status: "verified".to_string(),
        },
    }
}

fn issue_mode_label(mode: WatermarkIssueMode) -> &'static str {
    match mode {
        WatermarkIssueMode::ServerReserved => "server_reserved",
        WatermarkIssueMode::OfflineGenerated => "offline_generated",
        WatermarkIssueMode::ServerConfirmed => "server_confirmed",
        WatermarkIssueMode::ServerReissued => "server_reissued",
    }
}

fn media_type_label(media_type: WatermarkMediaType) -> &'static str {
    match media_type {
        WatermarkMediaType::Unknown => "unknown",
        WatermarkMediaType::Image => "image",
        WatermarkMediaType::Audio => "audio",
        WatermarkMediaType::VideoAudioTrack => "video_audio_track",
        WatermarkMediaType::VideoVisual => "video_visual",
    }
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
            creator_identity: "mobile-creator".to_string(),
            device_identity: "mobile-device".to_string(),
            media_bytes: b"mobile-media".to_vec(),
            timestamp: 1_700_000_000,
            reserved_watermark_uid: None,
            registry_proof_hash: None,
            parent_watermark_uid: None,
            revision: 1,
            media_type: None,
        }
    }

    fn desktop_fixture_payload(media_bytes: &[u8]) -> WatermarkPayload {
        WatermarkPayload::from_identity_and_media(watermark_core::PayloadBuildInput {
            creator_identity: "desktop-creator",
            device_identity: "desktop-device",
            media_bytes,
            timestamp: 1_700_000_123,
            ai_flags: default_ai_flags(),
        })
        .unwrap()
    }

    fn make_rgb_image() -> image::RgbImage {
        image::RgbImage::from_fn(512, 512, |x, y| {
            image::Rgb([
                ((x as f32 / 512.0 * 200.0) as u8).wrapping_add(30),
                ((y as f32 / 512.0 * 200.0) as u8).wrapping_add(30),
                128,
            ])
        })
    }

    fn encode_rgb_image(format: image::ImageFormat) -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(make_rgb_image())
            .write_to(&mut cursor, format)
            .unwrap();
        cursor.into_inner()
    }

    fn make_png_bytes() -> Vec<u8> {
        encode_rgb_image(image::ImageFormat::Png)
    }

    fn make_large_png_bytes() -> Vec<u8> {
        let img = image::RgbImage::from_fn(1024, 1024, |x, y| {
            image::Rgb([
                ((x as f32 / 1024.0 * 190.0) as u8).wrapping_add(30),
                ((y as f32 / 1024.0 * 190.0) as u8).wrapping_add(35),
                ((x ^ y) & 0x7F) as u8,
            ])
        });
        let mut cursor = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut cursor, image::ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    fn make_jpeg_bytes() -> Vec<u8> {
        encode_rgb_image(image::ImageFormat::Jpeg)
    }

    fn make_webp_bytes() -> Vec<u8> {
        encode_rgb_image(image::ImageFormat::WebP)
    }

    fn make_wav_bytes() -> Vec<u8> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
            for i in 0..(44_100 * 30) {
                let sample = ((i as f32 * 440.0 * std::f32::consts::TAU / 44_100.0).sin()
                    * 0.4
                    * i16::MAX as f32) as i16;
                writer.write_sample(sample).unwrap();
            }
            writer.finalize().unwrap();
        }
        cursor.into_inner()
    }

    fn audio_fixture_bytes(name: &str) -> Vec<u8> {
        std::fs::read(format!(
            "{}/testdata/audio/{name}",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap()
    }

    fn assert_audio_fixture(name: &str, expected_hint: &str) {
        let bytes = audio_fixture_bytes(name);
        assert!(bytes.len() > 1024, "audio fixture is unexpectedly small");
        assert_eq!(audio_extension_hint_from_bytes(&bytes), Some(expected_hint));
        let normalized = normalize_audio_to_wav(bytes).unwrap();
        assert!(is_wav_bytes(&normalized));
        assert!(normalized.len() > 1024, "normalized fixture is too small");
    }

    #[test]
    fn audio_container_fixtures_are_valid() {
        assert_audio_fixture("sine_31s.flac", "flac");
        assert_audio_fixture("sine_31s.mp3", "mp3");
        assert_audio_fixture("sine_31s.ogg", "ogg");
        assert_audio_fixture("sine_31s.m4a", "m4a");
        assert_audio_fixture("sine_31s.aac", "aac");
    }

    #[test]
    fn audio_extension_hint_distinguishes_aac_adts_from_mp3() {
        assert_eq!(
            audio_extension_hint_from_bytes(&[0xFF, 0xF1, 0x50, 0x40]),
            Some("aac")
        );
        assert_eq!(
            audio_extension_hint_from_bytes(&[0xFF, 0xFB, 0x90, 0x64]),
            Some("mp3")
        );
    }

    fn make_stereo_wav_bytes() -> Vec<u8> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();
            for i in 0..(48_000 * 30) {
                let left = ((i as f32 * 440.0 * std::f32::consts::TAU / 48_000.0).sin()
                    * 0.4
                    * i16::MAX as f32) as i16;
                let right = ((i as f32 * 660.0 * std::f32::consts::TAU / 48_000.0).sin()
                    * 0.35
                    * i16::MAX as f32) as i16;
                writer.write_sample(left).unwrap();
                writer.write_sample(right).unwrap();
            }
            writer.finalize().unwrap();
        }
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
        assert_eq!(extracted.timestamp, 0);
        assert_eq!(extracted.device_id_hex.len(), 32);
        assert_eq!(extracted.file_hash_hex.len(), 0);
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
    }

    #[test]
    fn mobile_extract_result_uses_v3_default_and_moves_version_chain_out_of_media() {
        let mut payload = sample_payload();
        payload.reserved_watermark_uid = Some("HS-11111111-22222222-33333333-44444444".to_string());
        payload.registry_proof_hash = Some("90909090909090909090909090909090".to_string());
        payload.parent_watermark_uid = Some("HS-AAAAAAAA-BBBBBBBB-CCCCCCCC-DDDDDDDD".to_string());
        payload.revision = 3;
        payload.media_type = Some("image".to_string());
        let result = embed_image_for_mobile(
            make_png_bytes(),
            payload,
            MobileImageOutputFormat::Png,
            true,
        )
        .unwrap();

        let extracted = extract_image_for_mobile(result.bytes).unwrap();

        assert_eq!(
            extracted.watermark_uid,
            "HS-11111111-22222222-33333333-44444444"
        );
        assert_eq!(extracted.parent_watermark_uid, None);
        assert_eq!(extracted.revision, 0);
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
        assert_eq!(extracted.watermark_id_issue_mode, "registry_resolved");
        assert_eq!(extracted.media_type, "image");
        assert_eq!(extracted.payload_auth_status, "verified");
    }

    #[test]
    fn mobile_readonly_candidate_reads_default_v3_image_report_bridge_fields() {
        let result = embed_image_for_mobile(
            make_large_png_bytes(),
            sample_payload(),
            MobileImageOutputFormat::Png,
            false,
        )
        .unwrap();

        let extracted = extract_image_readonly_candidate_for_mobile(result.bytes).unwrap();

        assert_eq!(extracted.watermark_uid, result.watermark_uid);
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
        assert_eq!(extracted.watermark_id_issue_mode, "registry_resolved");
        assert_eq!(extracted.payload_auth_status, "verified");
    }

    #[test]
    fn mobile_readonly_candidate_reads_default_v3_audio_report_bridge_fields() {
        let result = embed_audio_wav_for_mobile(make_wav_bytes(), sample_payload(), false).unwrap();

        let extracted = extract_audio_wav_readonly_candidate_for_mobile(result.bytes).unwrap();

        assert_eq!(extracted.watermark_uid, result.watermark_uid);
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
        assert_eq!(extracted.watermark_id_issue_mode, "registry_resolved");
        assert_eq!(extracted.payload_auth_status, "verified");
    }

    #[test]
    fn mobile_v3_readonly_fixture_preserves_anchor_fields() {
        let anchor = watermark_core::WatermarkPayloadV3MinimalAnchor::new(
            watermark_core::PayloadV3MinimalAnchorBuildInput {
                watermark_id: [
                    0x31, 0x32, 0x33, 0x34, 0x41, 0x42, 0x43, 0x44, 0x51, 0x52, 0x53, 0x54, 0x61,
                    0x62, 0x63, 0x64,
                ],
            },
        )
        .unwrap();
        let bytes = watermark_core::encode_payload_v3_minimal_anchor(&anchor).to_vec();
        let extracted = decode_v3_readonly_fixture_for_mobile(bytes, "image".to_string()).unwrap();

        assert_eq!(
            extracted.watermark_uid,
            "HS-31323334-41424344-51525354-61626364"
        );
        assert_eq!(extracted.revision, 0);
        assert_eq!(extracted.parent_watermark_uid, None);
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
        assert_eq!(extracted.watermark_id_issue_mode, "registry_resolved");
        assert_eq!(extracted.media_type, "image");
        assert_eq!(extracted.payload_auth_status, "verified");
    }

    #[test]
    fn mobile_v3_readonly_media_fixture_preserves_anchor_fields() {
        let anchor = watermark_core::WatermarkPayloadV3MinimalAnchor::new(
            watermark_core::PayloadV3MinimalAnchorBuildInput {
                watermark_id: [
                    0x51, 0x52, 0x53, 0x54, 0x61, 0x62, 0x63, 0x64, 0x71, 0x72, 0x73, 0x74, 0x81,
                    0x82, 0x83, 0x84,
                ],
            },
        )
        .unwrap();
        let media =
            watermark_core::embed_v3_readonly_anchor_wav_bytes(&make_wav_bytes(), &anchor).unwrap();
        let extracted =
            decode_v3_readonly_media_fixture_for_mobile(media, "audio".to_string()).unwrap();

        assert_eq!(
            extracted.watermark_uid,
            "HS-51525354-61626364-71727374-81828384"
        );
        assert_eq!(extracted.revision, 0);
        assert_eq!(extracted.parent_watermark_uid, None);
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
        assert_eq!(extracted.watermark_id_issue_mode, "registry_resolved");
        assert_eq!(extracted.media_type, "audio");
        assert_eq!(extracted.payload_auth_status, "verified");
    }

    #[test]
    fn mobile_image_fast_preflight_returns_none_for_plain_image() {
        let detected = detect_existing_image_for_mobile(make_png_bytes()).unwrap();

        assert!(detected.is_none());
    }

    #[test]
    fn mobile_image_fast_preflight_detects_existing_image_watermark() {
        let result = embed_image_for_mobile(
            make_large_png_bytes(),
            sample_payload(),
            MobileImageOutputFormat::Png,
            false,
        )
        .unwrap();

        let detected = detect_existing_image_for_mobile(result.bytes)
            .unwrap()
            .unwrap();

        assert_eq!(detected.watermark_uid, result.watermark_uid);
    }

    fn assert_mobile_image_input_is_desktop_core_extractable(source: Vec<u8>) {
        let result = embed_image_for_mobile(
            source,
            sample_payload(),
            MobileImageOutputFormat::Png,
            false,
        )
        .unwrap();
        let extracted = WatermarkService::extract(MediaInput::ImageBytes {
            bytes: result.bytes,
        })
        .unwrap();

        assert_eq!(extracted.watermark_uid(), result.watermark_uid);
        assert_eq!(extracted.protocol_version(), 3);
        assert_eq!(extracted.payload_bytes_length(), 39);
    }

    fn assert_desktop_core_image_input_is_mobile_extractable(source: Vec<u8>) {
        let payload = desktop_fixture_payload(&source);
        let output = WatermarkService::embed(
            MediaInput::ImageBytes { bytes: source },
            &payload,
            watermark_core::EmbedOptions {
                image_output_format: ImageOutputFormat::Png,
                ..watermark_core::EmbedOptions::default()
            },
        )
        .unwrap();
        let MediaOutput::ImageBytes { bytes, .. } = output else {
            panic!("unexpected output");
        };

        let extracted = extract_image_for_mobile(bytes).unwrap();

        assert_eq!(extracted.watermark_uid, payload.watermark_uid());
        assert_eq!(extracted.device_id_hex, hex::encode(payload.watermark_id));
        assert!(extracted.file_hash_hex.is_empty());
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
    }

    #[test]
    fn mobile_image_output_is_desktop_core_extractable() {
        assert_mobile_image_input_is_desktop_core_extractable(make_png_bytes());
    }

    #[test]
    fn desktop_core_image_output_is_mobile_extractable() {
        assert_desktop_core_image_input_is_mobile_extractable(make_png_bytes());
    }

    #[test]
    fn mobile_jpeg_image_input_is_desktop_core_extractable() {
        assert_mobile_image_input_is_desktop_core_extractable(make_jpeg_bytes());
    }

    #[test]
    fn desktop_core_jpeg_image_input_is_mobile_extractable() {
        assert_desktop_core_image_input_is_mobile_extractable(make_jpeg_bytes());
    }

    #[test]
    fn mobile_webp_image_input_is_desktop_core_extractable() {
        assert_mobile_image_input_is_desktop_core_extractable(make_webp_bytes());
    }

    #[test]
    fn desktop_core_webp_image_input_is_mobile_extractable() {
        assert_desktop_core_image_input_is_mobile_extractable(make_webp_bytes());
    }

    #[test]
    fn cross_end_image_bridge_contract_group() {
        assert_mobile_image_input_is_desktop_core_extractable(make_png_bytes());
        assert_desktop_core_image_input_is_mobile_extractable(make_png_bytes());
        assert_mobile_image_input_is_desktop_core_extractable(make_jpeg_bytes());
        assert_desktop_core_image_input_is_mobile_extractable(make_jpeg_bytes());
        assert_mobile_image_input_is_desktop_core_extractable(make_webp_bytes());
        assert_desktop_core_image_input_is_mobile_extractable(make_webp_bytes());
    }

    #[test]
    fn invalid_payload_is_rejected() {
        let mut payload = sample_payload();
        payload.creator_identity.clear();
        let err = embed_image_for_mobile(
            make_png_bytes(),
            payload,
            MobileImageOutputFormat::Png,
            false,
        )
        .unwrap_err();

        assert!(matches!(
            err,
            MobileWatermarkError::InvalidPayload { code, .. }
                if code == "missing_creator_identity"
        ));
    }

    #[test]
    fn mobile_audio_roundtrip() {
        let result = embed_audio_wav_for_mobile(make_wav_bytes(), sample_payload(), false).unwrap();
        let extracted = extract_audio_wav_for_mobile(result.bytes).unwrap();

        assert_eq!(extracted.watermark_uid, result.watermark_uid);
        assert_eq!(extracted.timestamp, 0);
        assert_eq!(extracted.device_id_hex.len(), 32);
        assert_eq!(extracted.file_hash_hex.len(), 0);
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
    }

    #[test]
    fn mobile_audio_output_is_desktop_core_extractable() {
        let result = embed_audio_wav_for_mobile(make_wav_bytes(), sample_payload(), false).unwrap();
        let extracted = WatermarkService::extract(MediaInput::AudioWavBytes {
            bytes: result.bytes,
        })
        .unwrap();

        assert_eq!(extracted.watermark_uid(), result.watermark_uid);
        assert_eq!(extracted.protocol_version(), 3);
        assert_eq!(extracted.payload_bytes_length(), 39);
    }

    fn assert_mobile_audio_input_is_desktop_core_extractable(source: Vec<u8>) {
        let result = embed_audio_wav_for_mobile(source, sample_payload(), false).unwrap();
        let extracted = WatermarkService::extract(MediaInput::AudioWavBytes {
            bytes: result.bytes,
        })
        .unwrap();

        assert_eq!(extracted.watermark_uid(), result.watermark_uid);
        assert_eq!(extracted.protocol_version(), 3);
        assert_eq!(extracted.payload_bytes_length(), 39);
    }

    #[test]
    fn mobile_flac_audio_input_is_desktop_core_extractable() {
        assert_mobile_audio_input_is_desktop_core_extractable(audio_fixture_bytes("sine_31s.flac"));
    }

    #[test]
    fn mobile_mp3_audio_input_is_desktop_core_extractable() {
        assert_mobile_audio_input_is_desktop_core_extractable(audio_fixture_bytes("sine_31s.mp3"));
    }

    #[test]
    fn mobile_ogg_audio_input_is_desktop_core_extractable() {
        assert_mobile_audio_input_is_desktop_core_extractable(audio_fixture_bytes("sine_31s.ogg"));
    }

    #[test]
    fn mobile_m4a_audio_input_is_desktop_core_extractable() {
        assert_mobile_audio_input_is_desktop_core_extractable(audio_fixture_bytes("sine_31s.m4a"));
    }

    #[test]
    fn mobile_aac_audio_input_is_desktop_core_extractable() {
        assert_mobile_audio_input_is_desktop_core_extractable(audio_fixture_bytes("sine_31s.aac"));
    }

    #[test]
    fn desktop_core_audio_output_is_mobile_extractable() {
        let source = make_wav_bytes();
        let payload = desktop_fixture_payload(&source);
        let output = WatermarkService::embed(
            MediaInput::AudioWavBytes { bytes: source },
            &payload,
            watermark_core::EmbedOptions::default(),
        )
        .unwrap();
        let MediaOutput::AudioWavBytes { bytes } = output else {
            panic!("unexpected output");
        };

        let extracted = extract_audio_wav_for_mobile(bytes).unwrap();

        assert_eq!(extracted.watermark_uid, payload.watermark_uid());
        assert_eq!(extracted.device_id_hex, hex::encode(payload.watermark_id));
        assert!(extracted.file_hash_hex.is_empty());
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
    }

    #[test]
    fn cross_end_wav_core_algorithm_group() {
        let result = embed_audio_wav_for_mobile(make_wav_bytes(), sample_payload(), false).unwrap();
        let extracted = WatermarkService::extract(MediaInput::AudioWavBytes {
            bytes: result.bytes,
        })
        .unwrap();
        assert_eq!(extracted.watermark_uid(), result.watermark_uid);
        assert_eq!(extracted.protocol_version(), 3);
        assert_eq!(extracted.payload_bytes_length(), 39);

        let source = make_wav_bytes();
        let payload = desktop_fixture_payload(&source);
        let output = WatermarkService::embed(
            MediaInput::AudioWavBytes { bytes: source },
            &payload,
            watermark_core::EmbedOptions::default(),
        )
        .unwrap();
        let MediaOutput::AudioWavBytes { bytes } = output else {
            panic!("unexpected output");
        };
        let extracted = extract_audio_wav_for_mobile(bytes).unwrap();
        assert_eq!(extracted.watermark_uid, payload.watermark_uid());
        assert_eq!(extracted.device_id_hex, hex::encode(payload.watermark_id));
        assert!(extracted.file_hash_hex.is_empty());
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
    }

    fn assert_mobile_audio_input_normalizes_to_desktop_core_payload(source: Vec<u8>) {
        let result = embed_audio_wav_for_mobile(source.clone(), sample_payload(), false).unwrap();
        let mobile_extracted = WatermarkService::extract(MediaInput::AudioWavBytes {
            bytes: result.bytes,
        })
        .unwrap();
        let normalized = normalize_audio_to_wav(source).unwrap();
        let payload =
            WatermarkPayload::from_identity_and_media(watermark_core::PayloadBuildInput {
                creator_identity: "mobile-creator",
                device_identity: "mobile-device",
                media_bytes: b"mobile-media",
                timestamp: 1_700_000_000,
                ai_flags: default_ai_flags(),
            })
            .unwrap();
        let output = WatermarkService::embed(
            MediaInput::AudioWavBytes { bytes: normalized },
            &payload,
            watermark_core::EmbedOptions::default(),
        )
        .unwrap();
        let MediaOutput::AudioWavBytes { bytes } = output else {
            panic!("unexpected output");
        };

        let extracted = extract_audio_wav_for_mobile(bytes).unwrap();

        assert_eq!(mobile_extracted.watermark_uid(), result.watermark_uid);
        assert_eq!(mobile_extracted.protocol_version(), 3);
        assert_eq!(mobile_extracted.payload_bytes_length(), 39);
        assert_eq!(extracted.watermark_uid, payload.watermark_uid());
        assert_eq!(extracted.device_id_hex, hex::encode(payload.watermark_id));
        assert_eq!(extracted.file_hash_hex, "");
        assert_eq!(extracted.payload_protocol_version, 3);
        assert_eq!(extracted.payload_bytes_length, 39);
    }

    #[test]
    fn mobile_flac_audio_input_normalizes_to_desktop_core_payload() {
        assert_mobile_audio_input_normalizes_to_desktop_core_payload(audio_fixture_bytes(
            "sine_31s.flac",
        ));
    }

    #[test]
    fn mobile_mp3_audio_input_normalizes_to_desktop_core_payload() {
        assert_mobile_audio_input_normalizes_to_desktop_core_payload(audio_fixture_bytes(
            "sine_31s.mp3",
        ));
    }

    #[test]
    fn mobile_ogg_audio_input_normalizes_to_desktop_core_payload() {
        assert_mobile_audio_input_normalizes_to_desktop_core_payload(audio_fixture_bytes(
            "sine_31s.ogg",
        ));
    }

    #[test]
    fn mobile_m4a_audio_input_normalizes_to_desktop_core_payload() {
        assert_mobile_audio_input_normalizes_to_desktop_core_payload(audio_fixture_bytes(
            "sine_31s.m4a",
        ));
    }

    #[test]
    fn mobile_aac_audio_input_normalizes_to_desktop_core_payload() {
        assert_mobile_audio_input_normalizes_to_desktop_core_payload(audio_fixture_bytes(
            "sine_31s.aac",
        ));
    }

    #[test]
    fn cross_end_non_wav_mobile_normalize_group() {
        assert_mobile_audio_input_normalizes_to_desktop_core_payload(audio_fixture_bytes(
            "sine_31s.flac",
        ));
        assert_mobile_audio_input_normalizes_to_desktop_core_payload(audio_fixture_bytes(
            "sine_31s.mp3",
        ));
        assert_mobile_audio_input_normalizes_to_desktop_core_payload(audio_fixture_bytes(
            "sine_31s.ogg",
        ));
        assert_mobile_audio_input_normalizes_to_desktop_core_payload(audio_fixture_bytes(
            "sine_31s.m4a",
        ));
        assert_mobile_audio_input_normalizes_to_desktop_core_payload(audio_fixture_bytes(
            "sine_31s.aac",
        ));
    }

    #[test]
    fn cross_end_non_wav_bridge_contract_group() {
        assert_mobile_audio_input_is_desktop_core_extractable(audio_fixture_bytes("sine_31s.flac"));
        assert_mobile_audio_input_is_desktop_core_extractable(audio_fixture_bytes("sine_31s.mp3"));
        assert_mobile_audio_input_is_desktop_core_extractable(audio_fixture_bytes("sine_31s.ogg"));
        assert_mobile_audio_input_is_desktop_core_extractable(audio_fixture_bytes("sine_31s.m4a"));
        assert_mobile_audio_input_is_desktop_core_extractable(audio_fixture_bytes("sine_31s.aac"));
    }

    #[test]
    fn mobile_audio_preserves_stereo_wav_layout() {
        let result =
            embed_audio_wav_for_mobile(make_stereo_wav_bytes(), sample_payload(), false).unwrap();
        let mut reader = hound::WavReader::new(std::io::Cursor::new(result.bytes)).unwrap();
        let spec = reader.spec();
        assert_eq!(spec.channels, 2);
        assert_eq!(spec.sample_rate, 48_000);
        assert!(reader.samples::<i16>().next().is_some());
    }
}

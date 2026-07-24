use realfft::num_complex::Complex;
use realfft::RealFftPlanner;
use std::io::Cursor;

use crate::error::WatermarkError;
use crate::payload::{
    bits_to_bytes, bytes_to_bits, decode_payload, decode_watermark_payload_readonly,
    encode_payload, encode_payload_v3_minimal_anchor, WatermarkDecodedPayload, WatermarkPayload,
    WatermarkPayloadV3MinimalAnchor, PAYLOAD_BYTES, PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};

const FRAME_SIZE: usize = 4096;
const CANONICAL_SAMPLE_RATE: u32 = 44_100;
pub const MIN_AUDIO_PROTECTION_SECONDS: u32 = 30;
pub const MAX_AUDIO_PROTECTION_SECONDS: u32 = 20 * 60;
pub const MAX_AUDIO_PROTECTION_BYTES: u64 = 512 * 1024 * 1024;
pub const MIN_SUPPORTED_AUDIO_SAMPLE_RATE: u32 = 8_000;
pub const MAX_SUPPORTED_AUDIO_SAMPLE_RATE: u32 = 48_000;
pub const MIN_SUPPORTED_AUDIO_CHANNELS: u16 = 1;
pub const MAX_SUPPORTED_AUDIO_CHANNELS: u16 = 2;
const BAND_LO_BIN: usize = 186;
const BAND_HI_BIN: usize = 743;
const BAND_LO_HZ: f32 = BAND_LO_BIN as f32 * CANONICAL_SAMPLE_RATE as f32 / FRAME_SIZE as f32;
const BAND_HI_HZ: f32 = BAND_HI_BIN as f32 * CANONICAL_SAMPLE_RATE as f32 / FRAME_SIZE as f32;
pub const DEFAULT_QIM_DELTA: f32 = 0.02;
pub const BALANCED_QIM_DELTA: f32 = 0.014;
const KNOWN_QIM_DELTAS: [f32; 2] = [DEFAULT_QIM_DELTA, BALANCED_QIM_DELTA];
const SILENCE_THRESHOLD: f32 = 0.001;
const PAYLOAD_BITS: usize = PAYLOAD_BYTES * 8;
const RELATIVE_PAIR_WIDTH: usize = 4;
const AUDIO_SLICE_MARKERS: usize = 16;
const AUDIO_MARKER_PREAMBLE: [u8; 2] = [0xA7, 0x5C];
const AUDIO_MARKER_BYTES: usize = 8;
const AUDIO_MARKER_BITS: usize = AUDIO_MARKER_BYTES * 8;
const AUDIO_MARKER_REDUNDANCY: usize = 3;
const AUDIO_MARKER_BITS_PER_FRAME: usize = 12;
const AUDIO_MARKER_BIT_LANES: usize = 3;
const AUDIO_RECOVERY_PREAMBLE: [u8; 4] = [0xA7, 0x5C, 0x41, 0x52];
const AUDIO_RECOVERY_CHECKSUM_BYTES: usize = 2;
const AUDIO_RECOVERY_PACKET_BYTES: usize = 4 + PAYLOAD_BYTES + AUDIO_RECOVERY_CHECKSUM_BYTES;
const AUDIO_RECOVERY_V3_READONLY_PACKET_BYTES: usize =
    4 + PAYLOAD_V3_MINIMAL_ANCHOR_BYTES + AUDIO_RECOVERY_CHECKSUM_BYTES;
const AUDIO_RECOVERY_PACKET_BITS: usize = AUDIO_RECOVERY_PACKET_BYTES * 8;
const AUDIO_RECOVERY_V3_READONLY_PACKET_BITS: usize = AUDIO_RECOVERY_V3_READONLY_PACKET_BYTES * 8;
const AUDIO_RECOVERY_REDUNDANCY: usize = 3;
const AUDIO_RECOVERY_BITS_PER_FRAME: usize = 18;
const AUDIO_RECOVERY_BIT_LANES: usize = 3;
const AUDIO_RECOVERY_PREAMBLE_MAX_BIT_ERRORS: usize = 4;
const AUDIO_RECOVERY_PAYLOAD_MAX_BIT_CORRECTIONS: usize = 3;
const AUDIO_MARKER_PAIR_OFFSET: usize = AUDIO_MARKER_BITS_PER_FRAME * AUDIO_MARKER_BIT_LANES + 4;
const AUDIO_RECOVERY_PAIR_OFFSET: usize =
    AUDIO_MARKER_PAIR_OFFSET + AUDIO_RECOVERY_BITS_PER_FRAME * AUDIO_RECOVERY_BIT_LANES + 4;
const AUDIO_PHASE_SCAN_STEPS: [usize; 8] = [0, 512, 1024, 1536, 2048, 2560, 3072, 3584];
const V3_RECOVERY_BASE_CONTRAST: f32 = 0.055;
const V3_RECOVERY_NOISE_CONTRAST_FACTOR: f32 = 0.55;
const V3_RECOVERY_TRANSIENT_CONTRAST_FACTOR: f32 = 0.70;
const V3_RECOVERY_LOW_ENERGY_CONTRAST_FACTOR: f32 = 0.75;
const V3_RECOVERY_MIN_CONTRAST: f32 = 0.016;
const V3_LOW_ENERGY_RMS_THRESHOLD: f32 = 0.045;
const V3_TRANSIENT_CREST_FACTOR_THRESHOLD: f32 = 4.5;
const V3_NOISE_SPECTRAL_FLATNESS_THRESHOLD: f32 = 0.42;
const AUDIO_NOISE_FLOOR_CANDIDATE_SCAN_BANDS: [AudioNoiseFloorCandidateScanBand; 3] = [
    AudioNoiseFloorCandidateScanBand {
        id: "noise_floor_low_mid_0_9_4_8k",
        lo_hz: 900.0,
        hi_hz: 4_800.0,
    },
    AudioNoiseFloorCandidateScanBand {
        id: "noise_floor_mid_shift_1_2_6_2k",
        lo_hz: 1_200.0,
        hi_hz: 6_200.0,
    },
    AudioNoiseFloorCandidateScanBand {
        id: "noise_floor_high_spread_3_8_9_6k",
        lo_hz: 3_800.0,
        hi_hz: 9_600.0,
    },
];

#[derive(Debug, Clone, Copy)]
struct AudioNoiseFloorCandidateScanBand {
    id: &'static str,
    lo_hz: f32,
    hi_hz: f32,
}

#[derive(Debug, Clone, Copy)]
pub struct AudioV3QualityDiagnostics {
    pub frame_count: usize,
    pub short_time_rms_min: f32,
    pub short_time_rms_mean: f32,
    pub short_time_rms_max: f32,
    pub low_energy_frame_ratio: f32,
    pub transient_frame_ratio: f32,
    pub noise_like_frame_ratio: f32,
    pub embedding_strength_min: f32,
    pub embedding_strength_mean: f32,
    pub embedding_strength_max: f32,
    pub modified_pair_ratio: f32,
    pub noise_floor_sparse_recovery: bool,
    pub extraction_confidence: f32,
}

pub const AUDIO_NOISE_FLOOR_MIGRATED_BAND_V1_CANDIDATE_PATH: &str =
    "v3_noise_floor_migrated_band_v1_candidate";
pub const AUDIO_NOISE_FLOOR_LEGACY_V3_FALLBACK_PATH: &str = "v3_recovery_2_8k_legacy";
pub const AUDIO_NOISE_FLOOR_CANDIDATE_FALLBACK_PATH: &str =
    "v3_noise_floor_migrated_band_v1_candidate -> v3_recovery_2_8k_legacy";
pub const AUDIO_NOISE_FLOOR_CANDIDATE_READ_COMPAT_MODE: &str =
    "legacy_v3_read_compat_candidate_interface_fallback";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioNoiseFloorMigrationCandidateFailureCode {
    CandidateInputInvalid,
    CandidateAudioTooShort,
    CandidateNotImplementedNoFrequencyStrategy,
    CandidatePayloadNotFound,
    CandidatePayloadInvalid,
}

impl AudioNoiseFloorMigrationCandidateFailureCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CandidateInputInvalid => "candidate_input_invalid",
            Self::CandidateAudioTooShort => "candidate_audio_too_short",
            Self::CandidateNotImplementedNoFrequencyStrategy => {
                "candidate_not_implemented_no_frequency_strategy"
            }
            Self::CandidatePayloadNotFound => "candidate_payload_not_found",
            Self::CandidatePayloadInvalid => "candidate_payload_invalid",
        }
    }
}

impl std::fmt::Display for AudioNoiseFloorMigrationCandidateFailureCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct AudioNoiseFloorMigrationCandidateReadError {
    pub code: AudioNoiseFloorMigrationCandidateFailureCode,
    pub extractor_path: &'static str,
    pub message: String,
}

impl AudioNoiseFloorMigrationCandidateReadError {
    fn new(code: AudioNoiseFloorMigrationCandidateFailureCode, message: impl Into<String>) -> Self {
        Self {
            code,
            extractor_path: AUDIO_NOISE_FLOOR_MIGRATED_BAND_V1_CANDIDATE_PATH,
            message: message.into(),
        }
    }
}

struct AudioV3RecoveryEmbeddingPlan {
    frame_count: usize,
    rms_min: f32,
    rms_mean: f32,
    rms_max: f32,
    low_energy_frame_ratio: f32,
    transient_frame_ratio: f32,
    noise_like_frame_ratio: f32,
    strength_min: f32,
    strength_mean: f32,
    strength_max: f32,
    modified_pair_ratio: f32,
    noise_floor_sparse_recovery: bool,
}

pub fn embed_watermark(
    samples: &mut [f32],
    payload: &WatermarkPayload,
) -> Result<(), WatermarkError> {
    embed_watermark_samples(samples, payload)
}

pub fn extract_watermark(samples: &[f32]) -> Result<WatermarkPayload, WatermarkError> {
    extract_watermark_samples(samples)
}

pub fn embed_watermark_wav_bytes(
    input_wav: &[u8],
    payload: &WatermarkPayload,
) -> Result<Vec<u8>, WatermarkError> {
    embed_watermark_wav_bytes_with_delta(input_wav, payload, DEFAULT_QIM_DELTA)
}

pub fn embed_watermark_wav_bytes_with_delta(
    input_wav: &[u8],
    payload: &WatermarkPayload,
    delta: f32,
) -> Result<Vec<u8>, WatermarkError> {
    reject_existing_wav_watermark(input_wav)?;
    embed_watermark_wav_bytes_allow_rewrite_with_delta(input_wav, payload, delta)
}

pub fn embed_watermark_wav_bytes_allow_rewrite(
    input_wav: &[u8],
    payload: &WatermarkPayload,
) -> Result<Vec<u8>, WatermarkError> {
    embed_watermark_wav_bytes_allow_rewrite_with_delta(input_wav, payload, DEFAULT_QIM_DELTA)
}

pub fn embed_watermark_wav_bytes_allow_rewrite_with_delta(
    input_wav: &[u8],
    payload: &WatermarkPayload,
    delta: f32,
) -> Result<Vec<u8>, WatermarkError> {
    embed_watermark_wav_bytes_allow_rewrite_with_delta_and_min_duration(
        input_wav,
        payload,
        delta,
        Some(MIN_AUDIO_PROTECTION_SECONDS),
    )
}

pub fn embed_watermark_wav_bytes_allow_rewrite_with_delta_without_min_duration(
    input_wav: &[u8],
    payload: &WatermarkPayload,
    delta: f32,
) -> Result<Vec<u8>, WatermarkError> {
    embed_watermark_wav_bytes_allow_rewrite_with_delta_and_min_duration(
        input_wav, payload, delta, None,
    )
}

fn embed_watermark_wav_bytes_allow_rewrite_with_delta_and_min_duration(
    input_wav: &[u8],
    payload: &WatermarkPayload,
    delta: f32,
    min_duration_seconds: Option<u32>,
) -> Result<Vec<u8>, WatermarkError> {
    let mut reader = hound::WavReader::new(Cursor::new(input_wav))
        .map_err(|e| WatermarkError::EmbedFailed(format!("open WAV: {e}")))?;
    let spec = reader.spec();
    let mut samples = read_wav_samples(&mut reader)?;
    if let Some(min_duration_seconds) = min_duration_seconds {
        validate_audio_protection_input(
            spec.sample_rate,
            spec.channels,
            samples.len() as f64 / spec.channels as f64 / spec.sample_rate as f64,
            min_duration_seconds,
        )
        .map_err(|code| WatermarkError::EmbedFailed(code.to_string()))?;
    }
    embed_watermark_samples_allow_rewrite_with_delta_and_rate(
        &mut samples,
        payload,
        delta,
        spec.sample_rate,
    )?;
    write_wav_samples(&samples, spec)
}

pub fn validate_audio_protection_input(
    sample_rate: u32,
    channels: u16,
    duration_seconds: f64,
    min_duration_seconds: u32,
) -> Result<(), String> {
    if sample_rate < MIN_SUPPORTED_AUDIO_SAMPLE_RATE {
        return Err("audio_protection_sample_rate_too_low".to_string());
    }
    if sample_rate > MAX_SUPPORTED_AUDIO_SAMPLE_RATE {
        return Err("audio_protection_sample_rate_too_high".to_string());
    }
    if !(MIN_SUPPORTED_AUDIO_CHANNELS..=MAX_SUPPORTED_AUDIO_CHANNELS).contains(&channels) {
        return Err("audio_protection_channels_unsupported".to_string());
    }
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        return Err("audio_protection_duration_unknown".to_string());
    }
    if duration_seconds < min_duration_seconds as f64 {
        return Err(format!(
            "audio_protection_min_duration: {duration_seconds:.3} seconds is below required {min_duration_seconds} seconds"
        ));
    }
    if duration_seconds > MAX_AUDIO_PROTECTION_SECONDS as f64 {
        return Err(format!(
            "audio_protection_max_duration: {duration_seconds:.3} seconds exceeds maximum {MAX_AUDIO_PROTECTION_SECONDS} seconds"
        ));
    }
    Ok(())
}

pub fn validate_audio_protection_file_size(file_size_bytes: u64) -> Result<(), String> {
    if file_size_bytes > MAX_AUDIO_PROTECTION_BYTES {
        return Err(format!(
            "audio_protection_file_size_limit_exceeded: {file_size_bytes} bytes exceeds maximum {MAX_AUDIO_PROTECTION_BYTES} bytes"
        ));
    }
    Ok(())
}

pub fn extract_watermark_wav_bytes(input_wav: &[u8]) -> Result<WatermarkPayload, WatermarkError> {
    extract_watermark_wav_bytes_with_delta(input_wav, DEFAULT_QIM_DELTA)
}

pub fn extract_watermark_wav_bytes_with_delta(
    input_wav: &[u8],
    delta: f32,
) -> Result<WatermarkPayload, WatermarkError> {
    let mut reader = hound::WavReader::new(Cursor::new(input_wav))
        .map_err(|e| WatermarkError::ExtractFailed(format!("open WAV: {e}")))?;
    let spec = reader.spec();
    let samples = read_wav_samples(&mut reader)?;
    extract_watermark_samples_with_delta_and_rate(&samples, delta, spec.sample_rate).or_else(
        |original_error| {
            if spec.sample_rate == CANONICAL_SAMPLE_RATE {
                return Err(original_error);
            }
            let canonical_samples =
                resample_linear(&samples, spec.sample_rate, CANONICAL_SAMPLE_RATE);
            extract_watermark_samples_with_delta_and_rate(
                &canonical_samples,
                delta,
                CANONICAL_SAMPLE_RATE,
            )
            .map_err(|_| original_error)
        },
    )
}

pub fn extract_watermark_wav_readonly_candidate_bytes(
    input_wav: &[u8],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    extract_watermark_wav_readonly_candidate_bytes_with_delta(input_wav, DEFAULT_QIM_DELTA)
}

pub fn extract_watermark_wav_readonly_candidate_bytes_with_delta(
    input_wav: &[u8],
    delta: f32,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    let mut reader = hound::WavReader::new(Cursor::new(input_wav))
        .map_err(|e| WatermarkError::ExtractFailed(format!("open WAV: {e}")))?;
    let spec = reader.spec();
    let samples = read_wav_samples(&mut reader)?;
    extract_watermark_samples_readonly_candidate_with_delta_and_rate(
        &samples,
        delta,
        spec.sample_rate,
    )
    .or_else(|original_error| {
        if spec.sample_rate == CANONICAL_SAMPLE_RATE {
            return Err(original_error);
        }
        let canonical_samples = resample_linear(&samples, spec.sample_rate, CANONICAL_SAMPLE_RATE);
        extract_watermark_samples_readonly_candidate_with_delta_and_rate(
            &canonical_samples,
            delta,
            CANONICAL_SAMPLE_RATE,
        )
        .map_err(|_| original_error)
    })
}

/// Read-only interface for the future stable noise-floor migrated-band extractor.
///
/// The interface is intentionally isolated from `WatermarkService::extract`: it reports typed
/// candidate failure codes for migration gates, but it does not write media or select a default
/// frequency strategy. Current callers must keep falling back to legacy V3 recovery.
pub fn extract_audio_noise_floor_migrated_band_v1_candidate_wav_bytes(
    input_wav: &[u8],
) -> Result<WatermarkDecodedPayload, AudioNoiseFloorMigrationCandidateReadError> {
    let mut reader = hound::WavReader::new(Cursor::new(input_wav)).map_err(|error| {
        AudioNoiseFloorMigrationCandidateReadError::new(
            AudioNoiseFloorMigrationCandidateFailureCode::CandidateInputInvalid,
            format!("open WAV: {error}"),
        )
    })?;
    let spec = reader.spec();
    let samples = read_wav_samples(&mut reader).map_err(|error| {
        AudioNoiseFloorMigrationCandidateReadError::new(
            AudioNoiseFloorMigrationCandidateFailureCode::CandidateInputInvalid,
            format!("read WAV samples: {error}"),
        )
    })?;
    extract_audio_noise_floor_migrated_band_v1_candidate_samples_with_rate(
        &samples,
        spec.sample_rate,
    )
}

pub fn extract_audio_noise_floor_migrated_band_v1_candidate_samples_with_rate(
    samples: &[f32],
    sample_rate: u32,
) -> Result<WatermarkDecodedPayload, AudioNoiseFloorMigrationCandidateReadError> {
    if sample_rate == 0 {
        return Err(AudioNoiseFloorMigrationCandidateReadError::new(
            AudioNoiseFloorMigrationCandidateFailureCode::CandidateInputInvalid,
            "sample rate must be greater than zero",
        ));
    }
    if samples.len() < FRAME_SIZE {
        return Err(AudioNoiseFloorMigrationCandidateReadError::new(
            AudioNoiseFloorMigrationCandidateFailureCode::CandidateAudioTooShort,
            "audio too short for migrated-band candidate extraction",
        ));
    }

    extract_audio_noise_floor_migrated_band_v1_candidate_scan(samples, sample_rate).map_err(
        |code| {
            let message = match code {
                AudioNoiseFloorMigrationCandidateFailureCode::CandidateAudioTooShort => {
                    "audio too short for migrated-band candidate scan"
                }
                AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadInvalid => {
                    "migrated-band candidate scan found an invalid V3/39 payload"
                }
                _ => "migrated-band candidate scan did not find a V3/39 payload",
            };
            AudioNoiseFloorMigrationCandidateReadError::new(code, message)
        },
    )
}

/// Builds a WAV fixture that carries a V3/39 minimal anchor in the formal audio recovery lane.
///
/// This helper is for V3 readonly candidate migration QA only. It intentionally stays separate
/// from production `embed_watermark_wav*` paths and must not be exposed as default V3 writing.
pub fn build_v3_readonly_candidate_audio_fixture_wav_bytes(
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<Vec<u8>, WatermarkError> {
    let samples = build_v3_readonly_candidate_audio_fixture_samples(anchor)?;
    write_wav_samples(
        &samples,
        hound::WavSpec {
            channels: 1,
            sample_rate: CANONICAL_SAMPLE_RATE,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        },
    )
}

pub fn embed_audio_v3_internal_qa_wav_bytes(
    input_wav: &[u8],
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<Vec<u8>, WatermarkError> {
    embed_audio_v3_internal_qa_wav_bytes_with_min_duration(
        input_wav,
        anchor,
        Some(MIN_AUDIO_PROTECTION_SECONDS),
    )
}

pub fn embed_audio_v3_internal_qa_wav_bytes_without_min_duration(
    input_wav: &[u8],
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<Vec<u8>, WatermarkError> {
    embed_audio_v3_internal_qa_wav_bytes_with_min_duration(input_wav, anchor, None)
}

fn embed_audio_v3_internal_qa_wav_bytes_with_min_duration(
    input_wav: &[u8],
    anchor: &WatermarkPayloadV3MinimalAnchor,
    min_duration_seconds: Option<u32>,
) -> Result<Vec<u8>, WatermarkError> {
    let mut reader = hound::WavReader::new(Cursor::new(input_wav))
        .map_err(|e| WatermarkError::EmbedFailed(format!("open WAV: {e}")))?;
    let spec = reader.spec();
    let samples = read_wav_samples(&mut reader)?;
    if let Some(min_duration_seconds) = min_duration_seconds {
        validate_wav_duration_for_protection(
            samples.len(),
            spec.sample_rate,
            spec.channels,
            min_duration_seconds,
        )?;
    }
    let output_samples = embed_v3_readonly_candidate_audio_fixture_samples_with_rate(
        &samples,
        anchor,
        spec.sample_rate,
    )?;
    write_wav_samples(&output_samples, spec)
}

pub fn embed_watermark_samples_v3_default(
    samples: &mut [f32],
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<(), WatermarkError> {
    let output_samples = embed_v3_readonly_candidate_audio_fixture_samples(samples, anchor)?;
    samples.copy_from_slice(&output_samples);
    Ok(())
}

pub fn embed_watermark_samples(
    samples: &mut [f32],
    payload: &WatermarkPayload,
) -> Result<(), WatermarkError> {
    embed_watermark_samples_with_delta(samples, payload, DEFAULT_QIM_DELTA)
}

pub fn embed_watermark_samples_with_delta(
    samples: &mut [f32],
    payload: &WatermarkPayload,
    delta: f32,
) -> Result<(), WatermarkError> {
    reject_existing_samples_watermark(samples)?;
    embed_watermark_samples_allow_rewrite_with_delta_and_rate(
        samples,
        payload,
        delta,
        CANONICAL_SAMPLE_RATE,
    )
}

pub fn embed_watermark_samples_allow_rewrite(
    samples: &mut [f32],
    payload: &WatermarkPayload,
) -> Result<(), WatermarkError> {
    embed_watermark_samples_allow_rewrite_with_delta(samples, payload, DEFAULT_QIM_DELTA)
}

pub fn embed_watermark_samples_allow_rewrite_with_delta(
    samples: &mut [f32],
    payload: &WatermarkPayload,
    delta: f32,
) -> Result<(), WatermarkError> {
    embed_watermark_samples_allow_rewrite_with_delta_and_rate(
        samples,
        payload,
        delta,
        CANONICAL_SAMPLE_RATE,
    )
}

pub fn embed_watermark_samples_allow_rewrite_with_delta_and_rate(
    samples: &mut [f32],
    payload: &WatermarkPayload,
    delta: f32,
    sample_rate: u32,
) -> Result<(), WatermarkError> {
    if samples.len() < FRAME_SIZE {
        return Err(WatermarkError::EmbedFailed(
            "audio too short for watermark embedding".into(),
        ));
    }

    let payload_bytes = encode_payload(payload);
    let bits = bytes_to_bits(&payload_bytes);
    let marker_packets = audio_marker_packets(&payload_bytes);
    let recovery_packet = encode_audio_recovery_packet(&payload_bytes);
    let recovery_bits = bytes_to_bits(&recovery_packet);

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let ifft = planner.plan_fft_inverse(FRAME_SIZE);

    let num_frames = samples.len() / FRAME_SIZE;
    let frames_per_marker = audio_frames_per_marker(num_frames);
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    let recovery_enabled = audio_recovery_enabled(num_frames, usable_pairs);
    let payload_pair_offset = if recovery_enabled {
        AUDIO_RECOVERY_PAIR_OFFSET
    } else {
        0
    };
    let payload_pairs = usable_pairs.saturating_sub(payload_pair_offset);

    for frame_idx in 0..num_frames {
        let offset = frame_idx * FRAME_SIZE;
        let frame = &mut samples[offset..offset + FRAME_SIZE];
        if rms_energy(frame) < SILENCE_THRESHOLD {
            continue;
        }

        let mut input = frame.to_vec();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut input, &mut spectrum)
            .map_err(|e| WatermarkError::EmbedFailed(format!("FFT failed: {e}")))?;

        for pair_idx in 0..payload_pairs {
            let bit_idx = (frame_idx * payload_pairs + pair_idx) % PAYLOAD_BITS;
            let bin_a = bin_lo + (payload_pair_offset + pair_idx) * RELATIVE_PAIR_WIDTH;
            embed_relative_pair(&mut spectrum, bin_a, bits[bit_idx], delta);
        }

        if recovery_enabled {
            embed_audio_marker_frame(
                &mut spectrum,
                bin_lo,
                &marker_packets,
                frame_idx,
                frames_per_marker,
                delta,
            );
            embed_audio_recovery_frame(
                &mut spectrum,
                bin_lo,
                &recovery_bits,
                frame_idx,
                0,
                delta * 1.6,
            );
        }

        let mut output = ifft.make_output_vec();
        ifft.process(&mut spectrum, &mut output)
            .map_err(|e| WatermarkError::EmbedFailed(format!("IFFT failed: {e}")))?;

        let scale = 1.0 / FRAME_SIZE as f32;
        for (j, sample) in output.iter().enumerate().take(FRAME_SIZE) {
            frame[j] = sample * scale;
        }
    }

    Ok(())
}

pub(crate) fn reject_existing_wav_watermark(input_wav: &[u8]) -> Result<(), WatermarkError> {
    for delta in KNOWN_QIM_DELTAS {
        if let Ok(payload) = extract_watermark_wav_bytes_with_delta(input_wav, delta) {
            return Err(WatermarkError::AlreadyWatermarked {
                existing_uid: payload.watermark_uid(),
            });
        }
    }
    Ok(())
}

fn reject_existing_samples_watermark(samples: &[f32]) -> Result<(), WatermarkError> {
    for delta in KNOWN_QIM_DELTAS {
        if let Ok(payload) = extract_watermark_samples_with_delta(samples, delta) {
            return Err(WatermarkError::AlreadyWatermarked {
                existing_uid: payload.watermark_uid(),
            });
        }
    }
    Ok(())
}

pub fn extract_watermark_samples(samples: &[f32]) -> Result<WatermarkPayload, WatermarkError> {
    extract_watermark_samples_with_delta_and_rate(samples, DEFAULT_QIM_DELTA, CANONICAL_SAMPLE_RATE)
}

pub fn extract_watermark_samples_with_delta(
    samples: &[f32],
    delta: f32,
) -> Result<WatermarkPayload, WatermarkError> {
    extract_watermark_samples_with_delta_and_rate(samples, delta, CANONICAL_SAMPLE_RATE)
}

pub fn extract_watermark_samples_readonly_candidate(
    samples: &[f32],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    extract_watermark_samples_readonly_candidate_with_delta_and_rate(
        samples,
        DEFAULT_QIM_DELTA,
        CANONICAL_SAMPLE_RATE,
    )
}

pub fn extract_watermark_samples_readonly_candidate_with_delta(
    samples: &[f32],
    delta: f32,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    extract_watermark_samples_readonly_candidate_with_delta_and_rate(
        samples,
        delta,
        CANONICAL_SAMPLE_RATE,
    )
}

#[cfg(test)]
fn detect_audio_marker_count(samples: &[f32], sample_rate: u32) -> Result<usize, WatermarkError> {
    extract_audio_marker_hits(samples, sample_rate).map(|hits| hits.len())
}

pub fn extract_watermark_samples_with_delta_and_rate(
    samples: &[f32],
    delta: f32,
    sample_rate: u32,
) -> Result<WatermarkPayload, WatermarkError> {
    for phase in AUDIO_PHASE_SCAN_STEPS {
        if phase >= samples.len() {
            continue;
        }
        let candidate = &samples[phase..];
        if let Ok(payload) = extract_watermark_samples_relative(candidate, sample_rate)
            .or_else(|_| extract_watermark_samples_recovery(candidate, sample_rate))
            .or_else(|_| extract_watermark_samples_with_markers(candidate, sample_rate))
            .or_else(|_| extract_watermark_samples_legacy_qim(candidate, delta, sample_rate))
        {
            return Ok(payload);
        }
    }

    Err(WatermarkError::ExtractFailed(
        "audio watermark extraction failed for all frame phases".into(),
    ))
}

pub fn extract_watermark_samples_readonly_candidate_with_delta_and_rate(
    samples: &[f32],
    delta: f32,
    sample_rate: u32,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    for phase in AUDIO_PHASE_SCAN_STEPS {
        if phase >= samples.len() {
            continue;
        }
        let candidate = &samples[phase..];
        if let Ok(decoded) =
            extract_watermark_samples_recovery_readonly_candidate(candidate, sample_rate)
        {
            return Ok(decoded);
        }
        if let Ok(payload) = extract_watermark_samples_relative(candidate, sample_rate)
            .or_else(|_| extract_watermark_samples_recovery(candidate, sample_rate))
            .or_else(|_| extract_watermark_samples_with_markers(candidate, sample_rate))
            .or_else(|_| extract_watermark_samples_legacy_qim(candidate, delta, sample_rate))
        {
            return Ok(WatermarkDecodedPayload::V2(payload));
        }
    }

    Err(WatermarkError::ExtractFailed(
        "audio readonly candidate extraction failed for all frame phases".into(),
    ))
}

fn extract_watermark_samples_relative(
    samples: &[f32],
    sample_rate: u32,
) -> Result<WatermarkPayload, WatermarkError> {
    if samples.len() < FRAME_SIZE {
        return Err(WatermarkError::ExtractFailed(
            "audio too short for watermark extraction".into(),
        ));
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);

    let num_frames = samples.len() / FRAME_SIZE;
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    let payload_pair_offset = if audio_recovery_enabled(num_frames, usable_pairs) {
        AUDIO_RECOVERY_PAIR_OFFSET
    } else {
        0
    };
    let payload_pairs = usable_pairs.saturating_sub(payload_pair_offset);
    let mut votes = vec![0i32; PAYLOAD_BITS];

    for frame_idx in 0..num_frames {
        let offset = frame_idx * FRAME_SIZE;
        let frame = &samples[offset..offset + FRAME_SIZE];
        if rms_energy(frame) < SILENCE_THRESHOLD {
            continue;
        }

        let mut input = frame.to_vec();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut input, &mut spectrum)
            .map_err(|e| WatermarkError::ExtractFailed(format!("FFT failed: {e}")))?;

        for pair_idx in 0..payload_pairs {
            let bit_idx = (frame_idx * payload_pairs + pair_idx) % PAYLOAD_BITS;
            let bin_a = bin_lo + (payload_pair_offset + pair_idx) * RELATIVE_PAIR_WIDTH;
            let bit = extract_relative_pair(&spectrum, bin_a);
            if bit {
                votes[bit_idx] += 1;
            } else {
                votes[bit_idx] -= 1;
            }
        }
    }

    let bits: Vec<bool> = votes.iter().map(|&v| v > 0).collect();
    let payload_bytes = bits_to_bytes(&bits);
    let mut arr = [0u8; PAYLOAD_BYTES];
    arr.copy_from_slice(&payload_bytes);
    decode_payload(&arr)
}

fn extract_watermark_samples_with_markers(
    samples: &[f32],
    sample_rate: u32,
) -> Result<WatermarkPayload, WatermarkError> {
    if samples.len() < FRAME_SIZE {
        return Err(WatermarkError::ExtractFailed(
            "audio too short for marker extraction".into(),
        ));
    }

    let marker_hits = extract_audio_marker_hits(samples, sample_rate)?;
    if marker_hits.is_empty() {
        return Err(WatermarkError::ExtractFailed(
            "audio marker not found".into(),
        ));
    }

    for hit in marker_hits {
        if let Ok(payload) =
            extract_watermark_samples_relative_from_slice(samples, sample_rate, hit.slice_id)
        {
            let payload_bytes = encode_payload(&payload);
            if audio_payload_tag(&payload_bytes) == hit.payload_tag {
                return Ok(payload);
            }
        }
    }

    Err(WatermarkError::ExtractFailed(
        "audio marker payload recovery failed".into(),
    ))
}

fn extract_watermark_samples_recovery(
    samples: &[f32],
    sample_rate: u32,
) -> Result<WatermarkPayload, WatermarkError> {
    let num_frames = samples.len() / FRAME_SIZE;
    if num_frames == 0 {
        return Err(WatermarkError::ExtractFailed(
            "audio too short for recovery extraction".into(),
        ));
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    if !audio_recovery_extract_enabled(num_frames, usable_pairs) {
        return Err(WatermarkError::ExtractFailed(
            "audio recovery not available".into(),
        ));
    }

    let recovery_frames = audio_recovery_frames_per_packet();
    let frame_recovery_bits = (0..num_frames)
        .map(|frame_idx| {
            let offset = frame_idx * FRAME_SIZE;
            let frame = &samples[offset..offset + FRAME_SIZE];
            if rms_energy(frame) < SILENCE_THRESHOLD {
                return Ok(None);
            }

            let mut input = frame.to_vec();
            let mut spectrum = fft.make_output_vec();
            fft.process(&mut input, &mut spectrum)
                .map_err(|e| WatermarkError::ExtractFailed(format!("FFT failed: {e}")))?;
            Ok(Some(extract_audio_recovery_frame_bits(&spectrum, bin_lo)))
        })
        .collect::<Result<Vec<_>, WatermarkError>>()?;

    let raw_total = AUDIO_RECOVERY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY;
    let mut starts = Vec::new();
    let scan_limit = recovery_frames.min(num_frames.saturating_sub(recovery_frames) + 1);
    for start in 0..scan_limit {
        starts.push(start);
    }

    for start_frame in starts {
        let mut raw_votes = vec![0i32; raw_total];
        let mut raw_seen = vec![false; raw_total];
        for frame_idx in start_frame..num_frames {
            let Some(frame_bits) = &frame_recovery_bits[frame_idx] else {
                continue;
            };
            let local_frame = (frame_idx - start_frame) % recovery_frames;
            let raw_start = local_frame * AUDIO_RECOVERY_BITS_PER_FRAME;
            if raw_start >= raw_total {
                continue;
            }
            for (slot, bit) in frame_bits.iter().copied().enumerate() {
                let raw_idx = raw_start + slot;
                if raw_idx >= raw_total {
                    break;
                }
                raw_seen[raw_idx] = true;
                raw_votes[raw_idx] += if bit { 1 } else { -1 };
            }
        }

        if raw_seen.iter().any(|seen| !seen) {
            continue;
        }
        let raw_bits = raw_votes.iter().map(|&vote| vote > 0).collect::<Vec<_>>();
        let bits = majority_bits_with_redundancy(
            &raw_bits,
            AUDIO_RECOVERY_PACKET_BITS,
            AUDIO_RECOVERY_REDUNDANCY,
        );
        let bytes = bits_to_bytes(&bits);
        if let Ok(payload) = decode_audio_recovery_packet_tolerant(&bytes) {
            return Ok(payload);
        }
    }

    Err(WatermarkError::ExtractFailed(
        "audio recovery packet not found".into(),
    ))
}

fn extract_watermark_samples_recovery_readonly_candidate(
    samples: &[f32],
    sample_rate: u32,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    let num_frames = samples.len() / FRAME_SIZE;
    if num_frames == 0 {
        return Err(WatermarkError::ExtractFailed(
            "audio too short for readonly recovery extraction".into(),
        ));
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    if !audio_recovery_v3_readonly_extract_enabled(num_frames, usable_pairs) {
        return Err(WatermarkError::ExtractFailed(
            "audio readonly recovery not available".into(),
        ));
    }

    let recovery_frames = audio_recovery_v3_readonly_frames_per_packet();
    let frame_recovery_bits = (0..num_frames)
        .map(|frame_idx| {
            let offset = frame_idx * FRAME_SIZE;
            let frame = &samples[offset..offset + FRAME_SIZE];
            if rms_energy(frame) < SILENCE_THRESHOLD {
                return Ok(None);
            }

            let mut input = frame.to_vec();
            let mut spectrum = fft.make_output_vec();
            fft.process(&mut input, &mut spectrum)
                .map_err(|e| WatermarkError::ExtractFailed(format!("FFT failed: {e}")))?;
            Ok(Some(extract_audio_recovery_frame_bits(&spectrum, bin_lo)))
        })
        .collect::<Result<Vec<_>, WatermarkError>>()?;

    let raw_total = AUDIO_RECOVERY_V3_READONLY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY;
    let mut starts = Vec::new();
    let scan_limit = recovery_frames.min(num_frames.saturating_sub(recovery_frames) + 1);
    for start in 0..scan_limit {
        starts.push(start);
    }

    for start_frame in starts {
        let mut raw_votes = vec![0i32; raw_total];
        let mut raw_seen = vec![false; raw_total];
        for frame_idx in start_frame..num_frames {
            let Some(frame_bits) = &frame_recovery_bits[frame_idx] else {
                continue;
            };
            let local_frame = (frame_idx - start_frame) % recovery_frames;
            let raw_start = local_frame * AUDIO_RECOVERY_BITS_PER_FRAME;
            if raw_start >= raw_total {
                continue;
            }
            for (slot, bit) in frame_bits.iter().copied().enumerate() {
                let raw_idx = raw_start + slot;
                if raw_idx >= raw_total {
                    break;
                }
                raw_seen[raw_idx] = true;
                raw_votes[raw_idx] += if bit { 1 } else { -1 };
            }
        }

        if raw_seen.iter().any(|seen| !seen) {
            continue;
        }
        let raw_bits = raw_votes.iter().map(|&vote| vote > 0).collect::<Vec<_>>();
        let bits = majority_bits_with_redundancy(
            &raw_bits,
            AUDIO_RECOVERY_V3_READONLY_PACKET_BITS,
            AUDIO_RECOVERY_REDUNDANCY,
        );
        let bytes = bits_to_bytes(&bits);
        if let Ok(decoded) = decode_audio_recovery_packet_v3_readonly(&bytes) {
            return Ok(decoded);
        }
    }

    Err(WatermarkError::ExtractFailed(
        "audio readonly recovery packet not found".into(),
    ))
}

fn extract_audio_noise_floor_migrated_band_v1_candidate_scan(
    samples: &[f32],
    sample_rate: u32,
) -> Result<WatermarkDecodedPayload, AudioNoiseFloorMigrationCandidateFailureCode> {
    let num_frames = samples.len() / FRAME_SIZE;
    if num_frames < audio_recovery_v3_readonly_frames_per_packet() {
        return Err(AudioNoiseFloorMigrationCandidateFailureCode::CandidateAudioTooShort);
    }

    let mut saw_invalid_payload = false;
    for phase in AUDIO_PHASE_SCAN_STEPS {
        if phase > 0 && samples.len() <= phase + FRAME_SIZE {
            continue;
        }
        let scan_samples = if phase == 0 {
            samples
        } else {
            &samples[phase..]
        };
        for band in AUDIO_NOISE_FLOOR_CANDIDATE_SCAN_BANDS {
            let _scan_profile_id = band.id;
            let Some((bin_lo, _bin_hi)) = audio_noise_floor_candidate_band_bins(band, sample_rate)
            else {
                continue;
            };
            match extract_audio_noise_floor_candidate_scan_at_bin_lo(scan_samples, bin_lo) {
                Ok(decoded) => return Ok(decoded),
                Err(AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadInvalid) => {
                    saw_invalid_payload = true;
                }
                Err(AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadNotFound) => {}
                Err(code) => return Err(code),
            }
        }
    }

    if saw_invalid_payload {
        Err(AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadInvalid)
    } else {
        Err(AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadNotFound)
    }
}

fn extract_audio_noise_floor_candidate_scan_at_bin_lo(
    samples: &[f32],
    bin_lo: usize,
) -> Result<WatermarkDecodedPayload, AudioNoiseFloorMigrationCandidateFailureCode> {
    let num_frames = samples.len() / FRAME_SIZE;
    if num_frames < audio_recovery_v3_readonly_frames_per_packet() {
        return Err(AudioNoiseFloorMigrationCandidateFailureCode::CandidateAudioTooShort);
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let recovery_frames = audio_recovery_v3_readonly_frames_per_packet();
    let raw_total = AUDIO_RECOVERY_V3_READONLY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY;
    let mut frame_recovery_bits = Vec::with_capacity(num_frames);

    for frame_idx in 0..num_frames {
        let offset = frame_idx * FRAME_SIZE;
        let frame = &samples[offset..offset + FRAME_SIZE];
        if rms_energy(frame) < SILENCE_THRESHOLD {
            frame_recovery_bits.push(None);
            continue;
        }

        let mut input = frame.to_vec();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut input, &mut spectrum)
            .map_err(|_| AudioNoiseFloorMigrationCandidateFailureCode::CandidateInputInvalid)?;
        frame_recovery_bits.push(Some(extract_audio_recovery_frame_bits(&spectrum, bin_lo)));
    }

    let scan_limit = recovery_frames.min(num_frames.saturating_sub(recovery_frames) + 1);
    for start_frame in 0..scan_limit {
        let mut raw_votes = vec![0i32; raw_total];
        let mut raw_seen = vec![false; raw_total];
        for frame_idx in start_frame..num_frames {
            let Some(frame_bits) = &frame_recovery_bits[frame_idx] else {
                continue;
            };
            let local_frame = (frame_idx - start_frame) % recovery_frames;
            let raw_start = local_frame * AUDIO_RECOVERY_BITS_PER_FRAME;
            if raw_start >= raw_total {
                continue;
            }
            for (slot, bit) in frame_bits.iter().copied().enumerate() {
                let raw_idx = raw_start + slot;
                if raw_idx >= raw_total {
                    break;
                }
                raw_seen[raw_idx] = true;
                raw_votes[raw_idx] += if bit { 1 } else { -1 };
            }
        }

        if raw_seen.iter().any(|seen| !seen) {
            continue;
        }
        let raw_bits = raw_votes.iter().map(|&vote| vote > 0).collect::<Vec<_>>();
        let bits = majority_bits_with_redundancy(
            &raw_bits,
            AUDIO_RECOVERY_V3_READONLY_PACKET_BITS,
            AUDIO_RECOVERY_REDUNDANCY,
        );
        let bytes = bits_to_bytes(&bits);
        if bytes.starts_with(&AUDIO_RECOVERY_PREAMBLE) {
            return decode_audio_recovery_packet_v3_readonly(&bytes).map_err(|_| {
                AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadInvalid
            });
        }
    }

    Err(AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadNotFound)
}

fn extract_watermark_samples_relative_from_slice(
    samples: &[f32],
    sample_rate: u32,
    slice_id: usize,
) -> Result<WatermarkPayload, WatermarkError> {
    let num_frames = samples.len() / FRAME_SIZE;
    if num_frames == 0 {
        return Err(WatermarkError::ExtractFailed(
            "audio too short for slice extraction".into(),
        ));
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    let payload_pair_offset = if audio_recovery_enabled(num_frames, usable_pairs) {
        AUDIO_RECOVERY_PAIR_OFFSET
    } else {
        0
    };
    let payload_pairs = usable_pairs.saturating_sub(payload_pair_offset);
    let frames_per_marker = audio_frames_per_marker(num_frames);
    let start_frame = slice_id.saturating_mul(frames_per_marker).min(num_frames);
    let end_frame = ((slice_id + 1) * frames_per_marker).min(num_frames);
    if start_frame >= end_frame {
        return Err(WatermarkError::ExtractFailed(
            "audio slice has no frames".into(),
        ));
    }

    let mut votes = vec![0i32; PAYLOAD_BITS];
    for frame_idx in start_frame..end_frame {
        let offset = frame_idx * FRAME_SIZE;
        let frame = &samples[offset..offset + FRAME_SIZE];
        if rms_energy(frame) < SILENCE_THRESHOLD {
            continue;
        }

        let mut input = frame.to_vec();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut input, &mut spectrum)
            .map_err(|e| WatermarkError::ExtractFailed(format!("FFT failed: {e}")))?;

        for pair_idx in 0..payload_pairs {
            let bit_idx = (frame_idx * payload_pairs + pair_idx) % PAYLOAD_BITS;
            let bin_a = bin_lo + (payload_pair_offset + pair_idx) * RELATIVE_PAIR_WIDTH;
            let bit = extract_relative_pair(&spectrum, bin_a);
            if bit {
                votes[bit_idx] += 1;
            } else {
                votes[bit_idx] -= 1;
            }
        }
    }

    let bits = votes.iter().map(|&vote| vote > 0).collect::<Vec<_>>();
    let payload_bytes = bits_to_bytes(&bits);
    let mut arr = [0u8; PAYLOAD_BYTES];
    arr.copy_from_slice(&payload_bytes);
    decode_payload(&arr)
}

fn extract_watermark_samples_legacy_qim(
    samples: &[f32],
    delta: f32,
    sample_rate: u32,
) -> Result<WatermarkPayload, WatermarkError> {
    if samples.len() < FRAME_SIZE {
        return Err(WatermarkError::ExtractFailed(
            "audio too short for watermark extraction".into(),
        ));
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);

    let num_frames = samples.len() / FRAME_SIZE;
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_bins = bin_hi - bin_lo;
    let mut votes = vec![0i32; PAYLOAD_BITS];

    for frame_idx in 0..num_frames {
        let offset = frame_idx * FRAME_SIZE;
        let frame = &samples[offset..offset + FRAME_SIZE];
        if rms_energy(frame) < SILENCE_THRESHOLD {
            continue;
        }

        let mut input = frame.to_vec();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut input, &mut spectrum)
            .map_err(|e| WatermarkError::ExtractFailed(format!("FFT failed: {e}")))?;

        for (i, bin_idx) in (bin_lo..bin_hi).enumerate() {
            let bit_idx = (frame_idx * usable_bins + i) % PAYLOAD_BITS;
            let bit = qim_extract(spectrum[bin_idx].norm(), delta);
            if bit {
                votes[bit_idx] += 1;
            } else {
                votes[bit_idx] -= 1;
            }
        }
    }

    let bits: Vec<bool> = votes.iter().map(|&v| v > 0).collect();
    let payload_bytes = bits_to_bytes(&bits);
    let mut arr = [0u8; PAYLOAD_BYTES];
    arr.copy_from_slice(&payload_bytes);
    decode_payload(&arr)
}

fn read_wav_samples(
    reader: &mut hound::WavReader<Cursor<&[u8]>>,
) -> Result<Vec<f32>, WatermarkError> {
    let spec = reader.spec();
    let samples = if spec.sample_format == hound::SampleFormat::Float {
        reader
            .samples::<f32>()
            .map(|s| s.map_err(|e| WatermarkError::ExtractFailed(format!("read WAV sample: {e}"))))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        let max_val = (1i32 << (spec.bits_per_sample - 1)) as f32;
        reader
            .samples::<i32>()
            .map(|s| {
                s.map_err(|e| WatermarkError::ExtractFailed(format!("read WAV sample: {e}")))
                    .map(|v| v as f32 / max_val)
            })
            .collect::<Result<Vec<_>, _>>()?
    };
    Ok(samples)
}

fn validate_wav_duration_for_protection(
    sample_count: usize,
    sample_rate: u32,
    channels: u16,
    min_duration_seconds: u32,
) -> Result<(), WatermarkError> {
    let required_samples =
        min_duration_seconds as usize * sample_rate.max(1) as usize * channels.max(1) as usize;
    if sample_count < required_samples {
        return Err(WatermarkError::EmbedFailed(format!(
            "audio_protection_min_duration: audio must be at least {} seconds for copyright protection",
            min_duration_seconds
        )));
    }
    Ok(())
}

fn write_wav_samples(samples: &[f32], spec: hound::WavSpec) -> Result<Vec<u8>, WatermarkError> {
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut cursor, spec)
        .map_err(|e| WatermarkError::EmbedFailed(format!("create WAV: {e}")))?;

    match spec.sample_format {
        hound::SampleFormat::Float if spec.bits_per_sample == 32 => {
            for &sample in samples {
                writer.write_sample(sample.clamp(-1.0, 1.0)).map_err(|e| {
                    WatermarkError::EmbedFailed(format!("write WAV float sample: {e}"))
                })?;
            }
        }
        hound::SampleFormat::Int if (8..=32).contains(&spec.bits_per_sample) => {
            let max_value = ((1i64 << (spec.bits_per_sample - 1)) - 1) as f32;
            let min_value = -((1i64 << (spec.bits_per_sample - 1)) as f32);
            for &sample in samples {
                let value = (sample.clamp(-1.0, 1.0) * max_value)
                    .round()
                    .clamp(min_value, max_value) as i32;
                writer.write_sample(value).map_err(|e| {
                    WatermarkError::EmbedFailed(format!("write WAV integer sample: {e}"))
                })?;
            }
        }
        _ => {
            return Err(WatermarkError::EmbedFailed(format!(
                "unsupported WAV output specification: {:?} {}-bit",
                spec.sample_format, spec.bits_per_sample
            )));
        }
    }
    writer
        .finalize()
        .map_err(|e| WatermarkError::EmbedFailed(format!("finalize WAV: {e}")))?;

    Ok(cursor.into_inner())
}

fn embed_relative_pair(spectrum: &mut [Complex<f32>], group_start: usize, bit: bool, delta: f32) {
    let total = (0..RELATIVE_PAIR_WIDTH)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>()
        .max(f32::EPSILON);
    let contrast = relative_contrast(delta);
    let high = total * (0.5 + contrast / 2.0);
    let low = total * (0.5 - contrast / 2.0);
    let (left_total, right_total) = if bit { (high, low) } else { (low, high) };
    let half_width = RELATIVE_PAIR_WIDTH / 2;
    let left_each = left_total / half_width as f32;
    let right_each = right_total / half_width as f32;

    for offset in 0..half_width {
        let value = spectrum[group_start + offset];
        spectrum[group_start + offset] = Complex::from_polar(left_each, value.arg());
    }
    for offset in half_width..RELATIVE_PAIR_WIDTH {
        let value = spectrum[group_start + offset];
        spectrum[group_start + offset] = Complex::from_polar(right_each, value.arg());
    }
}

fn embed_relative_pair_minimal(
    spectrum: &mut [Complex<f32>],
    group_start: usize,
    bit: bool,
    contrast: f32,
) {
    if !relative_pair_needs_update(spectrum, group_start, bit, contrast) {
        return;
    }
    let half_width = RELATIVE_PAIR_WIDTH / 2;
    let left_total = (0..half_width)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>();
    let right_total = (half_width..RELATIVE_PAIR_WIDTH)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>();
    let total = (left_total + right_total).max(f32::EPSILON);
    let margin = total * contrast.clamp(0.0, 0.49);
    let (target_left, target_right) = if bit {
        ((total + margin) / 2.0, (total - margin) / 2.0)
    } else {
        ((total - margin) / 2.0, (total + margin) / 2.0)
    };
    scale_relative_pair_half(
        spectrum,
        group_start,
        0,
        half_width,
        left_total,
        target_left,
    );
    scale_relative_pair_half(
        spectrum,
        group_start,
        half_width,
        RELATIVE_PAIR_WIDTH,
        right_total,
        target_right,
    );
}

fn embed_recovery_bit_sparse_lane_majority(
    spectrum: &mut [Complex<f32>],
    bin_lo: usize,
    bit_slot: usize,
    bit: bool,
    contrast: f32,
) {
    let mut strong_expected = 0usize;
    for lane in 0..AUDIO_RECOVERY_BIT_LANES {
        let pair_idx = AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
        let bin = bin_lo + pair_idx * RELATIVE_PAIR_WIDTH;
        if !relative_pair_needs_update(spectrum, bin, bit, contrast) {
            strong_expected += 1;
        }
    }
    if strong_expected > AUDIO_RECOVERY_BIT_LANES / 2 {
        return;
    }

    for lane in 0..AUDIO_RECOVERY_BIT_LANES {
        if strong_expected > AUDIO_RECOVERY_BIT_LANES / 2 {
            break;
        }
        let pair_idx = AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
        let bin = bin_lo + pair_idx * RELATIVE_PAIR_WIDTH;
        if relative_pair_needs_update(spectrum, bin, bit, contrast) {
            embed_relative_pair_minimal(spectrum, bin, bit, contrast);
            strong_expected += 1;
        }
    }
}

fn scale_relative_pair_half(
    spectrum: &mut [Complex<f32>],
    group_start: usize,
    start: usize,
    end: usize,
    current_total: f32,
    target_total: f32,
) {
    if current_total <= f32::EPSILON {
        let each = target_total / (end - start).max(1) as f32;
        for offset in start..end {
            let value = spectrum[group_start + offset];
            spectrum[group_start + offset] = Complex::from_polar(each, value.arg());
        }
        return;
    }
    let scale = target_total / current_total;
    for offset in start..end {
        spectrum[group_start + offset] *= scale;
    }
}

fn relative_pair_needs_update(
    spectrum: &[Complex<f32>],
    group_start: usize,
    bit: bool,
    contrast: f32,
) -> bool {
    let half_width = RELATIVE_PAIR_WIDTH / 2;
    let left = (0..half_width)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>();
    let right = (half_width..RELATIVE_PAIR_WIDTH)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>();
    let total = (left + right).max(f32::EPSILON);
    let margin = total * contrast.clamp(0.0, 0.49);
    if bit {
        left - right < margin
    } else {
        right - left < margin
    }
}

fn extract_relative_pair(spectrum: &[Complex<f32>], group_start: usize) -> bool {
    let half_width = RELATIVE_PAIR_WIDTH / 2;
    let left = (0..half_width)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>();
    let right = (half_width..RELATIVE_PAIR_WIDTH)
        .map(|offset| spectrum[group_start + offset].norm())
        .sum::<f32>();
    left >= right
}

fn relative_contrast(delta: f32) -> f32 {
    (delta * 10.0).clamp(0.12, 0.35)
}

#[derive(Debug, Clone, Copy)]
struct AudioMarkerHit {
    slice_id: usize,
    payload_tag: [u8; 2],
}

fn audio_marker_packets(
    payload_bytes: &[u8; PAYLOAD_BYTES],
) -> [[u8; AUDIO_MARKER_BYTES]; AUDIO_SLICE_MARKERS] {
    let payload_tag = audio_payload_tag(payload_bytes);
    std::array::from_fn(|slice_id| encode_audio_marker_packet(slice_id as u8, payload_tag))
}

fn encode_audio_marker_packet(slice_id: u8, payload_tag: [u8; 2]) -> [u8; AUDIO_MARKER_BYTES] {
    let mut packet = [0u8; AUDIO_MARKER_BYTES];
    packet[0..2].copy_from_slice(&AUDIO_MARKER_PREAMBLE);
    packet[2] = 1;
    packet[3] = slice_id;
    packet[4..6].copy_from_slice(&payload_tag);
    let checksum = audio_marker_checksum(&packet[0..6]);
    packet[6..8].copy_from_slice(&checksum);
    packet
}

fn decode_audio_marker_packet(bytes: &[u8]) -> Option<AudioMarkerHit> {
    if bytes.len() < AUDIO_MARKER_BYTES || bytes[0..2] != AUDIO_MARKER_PREAMBLE {
        return None;
    }
    if bytes[2] != 1 {
        return None;
    }
    let checksum = audio_marker_checksum(&bytes[0..6]);
    if bytes[6..8] != checksum {
        return None;
    }
    let slice_id = bytes[3] as usize;
    if slice_id >= AUDIO_SLICE_MARKERS {
        return None;
    }
    Some(AudioMarkerHit {
        slice_id,
        payload_tag: [bytes[4], bytes[5]],
    })
}

fn audio_marker_checksum(bytes: &[u8]) -> [u8; 2] {
    let mut state = 0x6D5Au16;
    for &byte in bytes {
        state = state.rotate_left(3) ^ byte as u16;
        state = state.wrapping_mul(181);
    }
    state.to_be_bytes()
}

fn audio_payload_tag(payload_bytes: &[u8; PAYLOAD_BYTES]) -> [u8; 2] {
    let mut state = 0xB33Fu16;
    for &byte in payload_bytes {
        state = state.rotate_left(5) ^ byte as u16;
        state = state.wrapping_mul(251);
    }
    state.to_be_bytes()
}

fn audio_frames_per_marker(num_frames: usize) -> usize {
    let minimum_marker_frames =
        (AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY).div_ceil(AUDIO_MARKER_BITS_PER_FRAME);
    num_frames
        .div_ceil(AUDIO_SLICE_MARKERS)
        .max(minimum_marker_frames)
}

fn audio_marker_enabled(num_frames: usize, usable_pairs: usize) -> bool {
    let marker_frames =
        (AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY).div_ceil(AUDIO_MARKER_BITS_PER_FRAME);
    num_frames >= marker_frames * 2 && usable_pairs > AUDIO_MARKER_PAIR_OFFSET + 16
}

fn audio_recovery_enabled(num_frames: usize, usable_pairs: usize) -> bool {
    let recovery_frames = audio_recovery_frames_per_packet();
    num_frames >= recovery_frames * 2 && usable_pairs > AUDIO_RECOVERY_PAIR_OFFSET + 16
}

fn audio_recovery_extract_enabled(num_frames: usize, usable_pairs: usize) -> bool {
    let recovery_frames = audio_recovery_frames_per_packet();
    num_frames >= recovery_frames && usable_pairs > AUDIO_RECOVERY_PAIR_OFFSET + 16
}

fn audio_recovery_v3_readonly_extract_enabled(num_frames: usize, usable_pairs: usize) -> bool {
    let recovery_frames = audio_recovery_v3_readonly_frames_per_packet();
    num_frames >= recovery_frames && usable_pairs > AUDIO_RECOVERY_PAIR_OFFSET + 16
}

fn audio_recovery_frames_per_packet() -> usize {
    (AUDIO_RECOVERY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY).div_ceil(AUDIO_RECOVERY_BITS_PER_FRAME)
}

fn audio_recovery_v3_readonly_frames_per_packet() -> usize {
    (AUDIO_RECOVERY_V3_READONLY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY)
        .div_ceil(AUDIO_RECOVERY_BITS_PER_FRAME)
}

fn audio_marker_bits_for_frame(
    marker_packets: &[[u8; AUDIO_MARKER_BYTES]; AUDIO_SLICE_MARKERS],
    frame_idx: usize,
    frames_per_marker: usize,
) -> Option<Vec<(usize, bool)>> {
    let slice_id = frame_idx / frames_per_marker;
    if slice_id >= AUDIO_SLICE_MARKERS {
        return None;
    }
    let local_frame = frame_idx % frames_per_marker;
    let raw_start = local_frame * AUDIO_MARKER_BITS_PER_FRAME;
    let raw_total = AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY;
    if raw_start >= raw_total {
        return None;
    }
    let bits = bytes_to_bits(&marker_packets[slice_id]);
    let mut frame_bits = Vec::new();
    for raw_idx in raw_start..(raw_start + AUDIO_MARKER_BITS_PER_FRAME).min(raw_total) {
        frame_bits.push((raw_idx, bits[raw_idx / AUDIO_MARKER_REDUNDANCY]));
    }
    Some(frame_bits)
}

fn encode_audio_recovery_packet(
    payload_bytes: &[u8; PAYLOAD_BYTES],
) -> [u8; AUDIO_RECOVERY_PACKET_BYTES] {
    let mut packet = [0u8; AUDIO_RECOVERY_PACKET_BYTES];
    packet[0..4].copy_from_slice(&AUDIO_RECOVERY_PREAMBLE);
    packet[4..4 + PAYLOAD_BYTES].copy_from_slice(payload_bytes);
    let checksum = audio_recovery_checksum(payload_bytes);
    packet[4 + PAYLOAD_BYTES..4 + PAYLOAD_BYTES + AUDIO_RECOVERY_CHECKSUM_BYTES]
        .copy_from_slice(&checksum);
    packet
}

fn encode_audio_recovery_packet_v3_readonly(
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> [u8; AUDIO_RECOVERY_V3_READONLY_PACKET_BYTES] {
    let payload_bytes = encode_payload_v3_minimal_anchor(anchor);
    let mut packet = [0u8; AUDIO_RECOVERY_V3_READONLY_PACKET_BYTES];
    packet[0..4].copy_from_slice(&AUDIO_RECOVERY_PREAMBLE);
    packet[4..4 + PAYLOAD_V3_MINIMAL_ANCHOR_BYTES].copy_from_slice(&payload_bytes);
    let checksum = audio_recovery_checksum_bytes(&payload_bytes);
    packet[4 + PAYLOAD_V3_MINIMAL_ANCHOR_BYTES
        ..4 + PAYLOAD_V3_MINIMAL_ANCHOR_BYTES + AUDIO_RECOVERY_CHECKSUM_BYTES]
        .copy_from_slice(&checksum);
    packet
}

fn decode_audio_recovery_packet_v3_readonly(
    bytes: &[u8],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    if bytes.len() < AUDIO_RECOVERY_V3_READONLY_PACKET_BYTES {
        return Err(WatermarkError::ExtractFailed(
            "audio recovery v3 readonly packet too short".into(),
        ));
    }
    if bytes[0..4] != AUDIO_RECOVERY_PREAMBLE {
        return Err(WatermarkError::ExtractFailed(
            "audio recovery v3 readonly preamble mismatch".into(),
        ));
    }

    let payload_start = 4;
    let payload_end = payload_start + PAYLOAD_V3_MINIMAL_ANCHOR_BYTES;
    let payload_bytes = &bytes[payload_start..payload_end];
    let checksum = audio_recovery_checksum_bytes(payload_bytes);
    if bytes[payload_end..payload_end + AUDIO_RECOVERY_CHECKSUM_BYTES] != checksum {
        return Err(WatermarkError::ExtractFailed(
            "audio recovery v3 readonly checksum mismatch".into(),
        ));
    }

    let decoded = decode_watermark_payload_readonly(payload_bytes)?;
    match decoded {
        WatermarkDecodedPayload::V3MinimalAnchor(_) => Ok(decoded),
        WatermarkDecodedPayload::V2(_) => Err(WatermarkError::ExtractFailed(
            "audio recovery v3 readonly expected minimal anchor".into(),
        )),
    }
}

fn decode_audio_recovery_packet(bytes: &[u8]) -> Result<WatermarkPayload, WatermarkError> {
    if bytes.len() < AUDIO_RECOVERY_PACKET_BYTES {
        return Err(WatermarkError::ExtractFailed(
            "audio recovery packet too short".into(),
        ));
    }
    if bytes[0..4] != AUDIO_RECOVERY_PREAMBLE {
        return Err(WatermarkError::ExtractFailed(
            "audio recovery preamble mismatch".into(),
        ));
    }

    let mut payload_bytes = [0u8; PAYLOAD_BYTES];
    payload_bytes.copy_from_slice(&bytes[4..4 + PAYLOAD_BYTES]);
    if bytes[4 + PAYLOAD_BYTES..4 + PAYLOAD_BYTES + AUDIO_RECOVERY_CHECKSUM_BYTES]
        != audio_recovery_checksum(&payload_bytes)
    {
        return Err(WatermarkError::ExtractFailed(
            "audio recovery checksum mismatch".into(),
        ));
    }
    decode_payload(&payload_bytes)
}

fn decode_audio_recovery_packet_tolerant(bytes: &[u8]) -> Result<WatermarkPayload, WatermarkError> {
    if let Ok(payload) = decode_audio_recovery_packet(bytes) {
        return Ok(payload);
    }
    if bytes.len() < AUDIO_RECOVERY_PACKET_BYTES {
        return Err(WatermarkError::ExtractFailed(
            "audio recovery packet too short".into(),
        ));
    }
    if byte_bit_errors(&bytes[0..4], &AUDIO_RECOVERY_PREAMBLE)
        > AUDIO_RECOVERY_PREAMBLE_MAX_BIT_ERRORS
    {
        return Err(WatermarkError::ExtractFailed(
            "audio recovery preamble mismatch".into(),
        ));
    }

    let mut payload_bytes = [0u8; PAYLOAD_BYTES];
    payload_bytes.copy_from_slice(&bytes[4..4 + PAYLOAD_BYTES]);
    let mut checksum = [0u8; AUDIO_RECOVERY_CHECKSUM_BYTES];
    checksum.copy_from_slice(
        &bytes[4 + PAYLOAD_BYTES..4 + PAYLOAD_BYTES + AUDIO_RECOVERY_CHECKSUM_BYTES],
    );
    correct_audio_recovery_payload_bits(
        payload_bytes,
        checksum,
        AUDIO_RECOVERY_PAYLOAD_MAX_BIT_CORRECTIONS,
    )
}

fn correct_audio_recovery_payload_bits(
    mut payload_bytes: [u8; PAYLOAD_BYTES],
    checksum: [u8; AUDIO_RECOVERY_CHECKSUM_BYTES],
    max_corrections: usize,
) -> Result<WatermarkPayload, WatermarkError> {
    if audio_recovery_checksum(&payload_bytes) == checksum {
        return decode_payload(&payload_bytes);
    }
    if max_corrections == 0 {
        return Err(WatermarkError::ExtractFailed(
            "audio recovery checksum mismatch".into(),
        ));
    }

    for first in 0..PAYLOAD_BITS {
        flip_payload_bit(&mut payload_bytes, first);
        if audio_recovery_checksum(&payload_bytes) == checksum {
            if let Ok(payload) = decode_payload(&payload_bytes) {
                return Ok(payload);
            }
        }
        if max_corrections >= 2 {
            for second in first + 1..PAYLOAD_BITS {
                flip_payload_bit(&mut payload_bytes, second);
                if audio_recovery_checksum(&payload_bytes) == checksum {
                    if let Ok(payload) = decode_payload(&payload_bytes) {
                        return Ok(payload);
                    }
                }
                if max_corrections >= 3 {
                    for third in second + 1..PAYLOAD_BITS {
                        flip_payload_bit(&mut payload_bytes, third);
                        if audio_recovery_checksum(&payload_bytes) == checksum {
                            if let Ok(payload) = decode_payload(&payload_bytes) {
                                return Ok(payload);
                            }
                        }
                        flip_payload_bit(&mut payload_bytes, third);
                    }
                }
                flip_payload_bit(&mut payload_bytes, second);
            }
        }
        flip_payload_bit(&mut payload_bytes, first);
    }

    Err(WatermarkError::ExtractFailed(
        "audio recovery checksum mismatch".into(),
    ))
}

fn flip_payload_bit(payload_bytes: &mut [u8; PAYLOAD_BYTES], bit_idx: usize) {
    let byte_idx = bit_idx / 8;
    let mask = 1u8 << (7 - bit_idx % 8);
    payload_bytes[byte_idx] ^= mask;
}

fn byte_bit_errors(left: &[u8], right: &[u8]) -> usize {
    left.iter()
        .zip(right.iter())
        .map(|(left, right)| (left ^ right).count_ones() as usize)
        .sum()
}

fn audio_recovery_checksum(
    payload_bytes: &[u8; PAYLOAD_BYTES],
) -> [u8; AUDIO_RECOVERY_CHECKSUM_BYTES] {
    audio_recovery_checksum_bytes(payload_bytes)
}

fn audio_recovery_checksum_bytes(payload_bytes: &[u8]) -> [u8; AUDIO_RECOVERY_CHECKSUM_BYTES] {
    let mut state = 0xA6D3u16;
    for &byte in payload_bytes {
        state = state.rotate_left(5) ^ byte as u16;
        state = state.wrapping_mul(241);
    }
    state.to_be_bytes()
}

fn embed_audio_recovery_frame(
    spectrum: &mut [Complex<f32>],
    bin_lo: usize,
    recovery_bits: &[bool],
    frame_idx: usize,
    frame_shift: usize,
    delta: f32,
) {
    embed_audio_recovery_frame_for_packet_bits(
        spectrum,
        bin_lo,
        recovery_bits,
        frame_idx,
        frame_shift,
        delta,
        AUDIO_RECOVERY_PACKET_BITS,
    )
}

fn embed_audio_recovery_frame_for_packet_bits(
    spectrum: &mut [Complex<f32>],
    bin_lo: usize,
    recovery_bits: &[bool],
    frame_idx: usize,
    frame_shift: usize,
    delta: f32,
    packet_bits: usize,
) {
    let recovery_frames =
        (packet_bits * AUDIO_RECOVERY_REDUNDANCY).div_ceil(AUDIO_RECOVERY_BITS_PER_FRAME);
    let local_frame = (frame_idx + frame_shift) % recovery_frames;
    let raw_start = local_frame * AUDIO_RECOVERY_BITS_PER_FRAME;
    let raw_total = packet_bits * AUDIO_RECOVERY_REDUNDANCY;
    if raw_start >= raw_total {
        return;
    }

    for raw_idx in raw_start..(raw_start + AUDIO_RECOVERY_BITS_PER_FRAME).min(raw_total) {
        let bit_slot = raw_idx % AUDIO_RECOVERY_BITS_PER_FRAME;
        let bit = recovery_bits[raw_idx / AUDIO_RECOVERY_REDUNDANCY];
        for lane in 0..AUDIO_RECOVERY_BIT_LANES {
            let pair_idx = AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
            embed_relative_pair(
                spectrum,
                bin_lo + pair_idx * RELATIVE_PAIR_WIDTH,
                bit,
                delta,
            );
        }
    }
}

fn embed_audio_marker_frame(
    spectrum: &mut [Complex<f32>],
    bin_lo: usize,
    marker_packets: &[[u8; AUDIO_MARKER_BYTES]; AUDIO_SLICE_MARKERS],
    frame_idx: usize,
    frames_per_marker: usize,
    delta: f32,
) {
    let Some(bits) = audio_marker_bits_for_frame(marker_packets, frame_idx, frames_per_marker)
    else {
        return;
    };
    for (raw_idx, bit) in bits {
        let bit_slot = raw_idx % AUDIO_MARKER_BITS_PER_FRAME;
        for lane in 0..AUDIO_MARKER_BIT_LANES {
            let pair_idx = bit_slot * AUDIO_MARKER_BIT_LANES + lane;
            embed_relative_pair(
                spectrum,
                bin_lo + pair_idx * RELATIVE_PAIR_WIDTH,
                bit,
                delta,
            );
        }
    }
}

fn extract_audio_marker_frame_bits(spectrum: &[Complex<f32>], bin_lo: usize) -> Vec<bool> {
    (0..AUDIO_MARKER_BITS_PER_FRAME)
        .map(|bit_slot| {
            let ones = (0..AUDIO_MARKER_BIT_LANES)
                .filter(|&lane| {
                    let pair_idx = bit_slot * AUDIO_MARKER_BIT_LANES + lane;
                    extract_relative_pair(spectrum, bin_lo + pair_idx * RELATIVE_PAIR_WIDTH)
                })
                .count();
            ones > AUDIO_MARKER_BIT_LANES / 2
        })
        .collect()
}

fn extract_audio_recovery_frame_bits(spectrum: &[Complex<f32>], bin_lo: usize) -> Vec<bool> {
    (0..AUDIO_RECOVERY_BITS_PER_FRAME)
        .map(|bit_slot| {
            let ones = (0..AUDIO_RECOVERY_BIT_LANES)
                .filter(|&lane| {
                    let pair_idx =
                        AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
                    extract_relative_pair(spectrum, bin_lo + pair_idx * RELATIVE_PAIR_WIDTH)
                })
                .count();
            ones > AUDIO_RECOVERY_BIT_LANES / 2
        })
        .collect()
}

fn extract_audio_marker_hits(
    samples: &[f32],
    sample_rate: u32,
) -> Result<Vec<AudioMarkerHit>, WatermarkError> {
    let num_frames = samples.len() / FRAME_SIZE;
    if num_frames == 0 {
        return Ok(Vec::new());
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let frames_per_marker = audio_frames_per_marker(num_frames);
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    if !audio_marker_enabled(num_frames, usable_pairs) {
        return Ok(Vec::new());
    }
    let mut raw_bits_by_slice = vec![Vec::<bool>::new(); AUDIO_SLICE_MARKERS];

    for frame_idx in 0..num_frames {
        let slice_id = frame_idx / frames_per_marker;
        if slice_id >= AUDIO_SLICE_MARKERS {
            continue;
        }
        if raw_bits_by_slice[slice_id].len() >= AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY {
            continue;
        }

        let offset = frame_idx * FRAME_SIZE;
        let frame = &samples[offset..offset + FRAME_SIZE];
        if rms_energy(frame) < SILENCE_THRESHOLD {
            continue;
        }

        let mut input = frame.to_vec();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut input, &mut spectrum)
            .map_err(|e| WatermarkError::ExtractFailed(format!("FFT failed: {e}")))?;
        let remaining =
            AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY - raw_bits_by_slice[slice_id].len();
        let mut frame_bits = extract_audio_marker_frame_bits(&spectrum, bin_lo);
        frame_bits.truncate(remaining);
        raw_bits_by_slice[slice_id].extend(frame_bits);
    }

    let mut hits = Vec::new();
    for raw_bits in raw_bits_by_slice {
        if raw_bits.len() < AUDIO_MARKER_BITS * AUDIO_MARKER_REDUNDANCY {
            continue;
        }
        let marker_bits = majority_bits(&raw_bits, AUDIO_MARKER_BITS);
        let marker_bytes = bits_to_bytes(&marker_bits);
        if let Some(hit) = decode_audio_marker_packet(&marker_bytes) {
            hits.push(hit);
        }
    }
    Ok(hits)
}

fn majority_bits(raw_bits: &[bool], bit_count: usize) -> Vec<bool> {
    majority_bits_with_redundancy(raw_bits, bit_count, AUDIO_MARKER_REDUNDANCY)
}

fn majority_bits_with_redundancy(
    raw_bits: &[bool],
    bit_count: usize,
    redundancy: usize,
) -> Vec<bool> {
    (0..bit_count)
        .map(|bit_idx| {
            let start = bit_idx * redundancy;
            let chunk = &raw_bits[start..start + redundancy];
            let ones = chunk.iter().filter(|&&bit| bit).count();
            ones > chunk.len() / 2
        })
        .collect()
}

fn audio_band_bins(sample_rate: u32) -> (usize, usize) {
    let sample_rate = sample_rate.max(1) as f32;
    let nyquist = sample_rate / 2.0;
    let lo_hz = BAND_LO_HZ.min(nyquist * 0.95);
    let hi_hz = BAND_HI_HZ.min(nyquist * 0.98);
    let mut lo = ((lo_hz * FRAME_SIZE as f32) / sample_rate).round() as usize;
    let mut hi = ((hi_hz * FRAME_SIZE as f32) / sample_rate).round() as usize;
    let max_bin = FRAME_SIZE / 2;
    lo = lo.clamp(1, max_bin.saturating_sub(2));
    hi = hi.clamp(lo + 2, max_bin);
    (lo, hi)
}

fn audio_noise_floor_candidate_band_bins(
    band: AudioNoiseFloorCandidateScanBand,
    sample_rate: u32,
) -> Option<(usize, usize)> {
    let sample_rate = sample_rate.max(1) as f32;
    let nyquist = sample_rate / 2.0;
    let lo_hz = band.lo_hz.min(nyquist * 0.95);
    let hi_hz = band.hi_hz.min(nyquist * 0.98);
    if !(hi_hz > lo_hz) {
        return None;
    }

    let max_bin = FRAME_SIZE / 2;
    let lo = ((lo_hz * FRAME_SIZE as f32) / sample_rate)
        .round()
        .clamp(1.0, max_bin.saturating_sub(2) as f32) as usize;
    let hi = ((hi_hz * FRAME_SIZE as f32) / sample_rate)
        .round()
        .clamp((lo + 2) as f32, max_bin as f32) as usize;
    let usable_pairs = (hi - lo) / RELATIVE_PAIR_WIDTH;
    if audio_recovery_v3_readonly_extract_enabled(
        audio_recovery_v3_readonly_frames_per_packet(),
        usable_pairs,
    ) {
        Some((lo, hi))
    } else {
        None
    }
}

fn resample_linear(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if samples.is_empty() || from_rate == 0 || to_rate == 0 || from_rate == to_rate {
        return samples.to_vec();
    }

    let output_len = ((samples.len() as u64 * to_rate as u64) / from_rate as u64).max(1) as usize;
    let step = from_rate as f64 / to_rate as f64;
    let mut output = Vec::with_capacity(output_len);

    for index in 0..output_len {
        let src_pos = index as f64 * step;
        let left = src_pos.floor() as usize;
        let right = (left + 1).min(samples.len() - 1);
        let frac = (src_pos - left as f64) as f32;
        let sample = samples[left] * (1.0 - frac) + samples[right] * frac;
        output.push(sample);
    }

    output
}

fn build_v3_readonly_candidate_audio_fixture_samples(
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<Vec<f32>, WatermarkError> {
    let samples: Vec<f32> = (0..(FRAME_SIZE * 320))
        .map(|i| {
            let t = i as f32 / CANONICAL_SAMPLE_RATE as f32;
            let tone_a = (t * 440.0 * std::f32::consts::TAU).sin() * 0.16;
            let tone_b = (t * 880.0 * std::f32::consts::TAU).sin() * 0.08;
            tone_a + tone_b
        })
        .collect();
    embed_v3_readonly_candidate_audio_fixture_samples(&samples, anchor)
}

fn embed_v3_readonly_candidate_audio_fixture_samples(
    input_samples: &[f32],
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<Vec<f32>, WatermarkError> {
    embed_v3_readonly_candidate_audio_fixture_samples_with_rate(
        input_samples,
        anchor,
        CANONICAL_SAMPLE_RATE,
    )
}

fn embed_v3_readonly_candidate_audio_fixture_samples_with_rate(
    input_samples: &[f32],
    anchor: &WatermarkPayloadV3MinimalAnchor,
    sample_rate: u32,
) -> Result<Vec<f32>, WatermarkError> {
    let mut samples = input_samples.to_vec();
    let required_samples = FRAME_SIZE * audio_recovery_v3_readonly_frames_per_packet();
    if samples.len() < required_samples {
        return Err(WatermarkError::EmbedFailed(format!(
            "audio_v3_internal_qa_min_duration: need at least {required_samples} samples"
        )));
    }
    let recovery_packet = encode_audio_recovery_packet_v3_readonly(anchor);
    let recovery_bits = bytes_to_bits(&recovery_packet);
    let num_frames = samples.len() / FRAME_SIZE;
    let recovery_frames = audio_recovery_v3_readonly_frames_per_packet();
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    if usable_pairs <= AUDIO_RECOVERY_PAIR_OFFSET + 16 {
        return Err(WatermarkError::EmbedFailed(
            "not enough audio recovery pairs for V3 readonly candidate fixture".into(),
        ));
    }
    let embedding_plan = plan_v3_recovery_embedding(&samples, anchor, sample_rate)?;
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let ifft = planner.plan_fft_inverse(FRAME_SIZE);

    for frame_idx in 0..num_frames {
        let offset = frame_idx * FRAME_SIZE;
        let frame = &mut samples[offset..offset + FRAME_SIZE];
        let frame_rms = rms_energy(frame);
        if frame_rms < SILENCE_THRESHOLD {
            continue;
        }
        let mut input = frame.to_vec();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut input, &mut spectrum)
            .map_err(|error| WatermarkError::EmbedFailed(format!("FFT failed: {error}")))?;
        let frame_profile = AudioV3FrameProfile::from_frame_and_spectrum(
            frame, frame_rms, &spectrum, bin_lo, bin_hi,
        );

        let local_frame = frame_idx % recovery_frames;
        let raw_start = local_frame * AUDIO_RECOVERY_BITS_PER_FRAME;
        let raw_total = AUDIO_RECOVERY_V3_READONLY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY;
        if raw_start < raw_total {
            for raw_idx in raw_start..(raw_start + AUDIO_RECOVERY_BITS_PER_FRAME).min(raw_total) {
                let bit_slot = raw_idx % AUDIO_RECOVERY_BITS_PER_FRAME;
                let bit = recovery_bits[raw_idx / AUDIO_RECOVERY_REDUNDANCY];
                let contrast = v3_recovery_frame_contrast(frame_profile);
                if embedding_plan.noise_floor_sparse_recovery {
                    embed_recovery_bit_sparse_lane_majority(
                        &mut spectrum,
                        bin_lo,
                        bit_slot,
                        bit,
                        contrast,
                    );
                } else {
                    for lane in 0..AUDIO_RECOVERY_BIT_LANES {
                        let pair_idx =
                            AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
                        let bin = bin_lo + pair_idx * RELATIVE_PAIR_WIDTH;
                        embed_relative_pair_minimal(&mut spectrum, bin, bit, contrast);
                    }
                }
            }
        }

        let mut output = ifft.make_output_vec();
        ifft.process(&mut spectrum, &mut output)
            .map_err(|error| WatermarkError::EmbedFailed(format!("IFFT failed: {error}")))?;
        let scale = 1.0 / FRAME_SIZE as f32;
        for (j, sample) in output.iter().enumerate().take(FRAME_SIZE) {
            frame[j] = sample * scale;
        }
    }

    Ok(samples)
}

fn plan_v3_recovery_embedding(
    source_samples: &[f32],
    anchor: &WatermarkPayloadV3MinimalAnchor,
    sample_rate: u32,
) -> Result<AudioV3RecoveryEmbeddingPlan, WatermarkError> {
    let frame_count = source_samples.len() / FRAME_SIZE;
    if frame_count == 0 {
        return Err(WatermarkError::EmbedFailed(
            "audio too short for V3 recovery embedding plan".into(),
        ));
    }

    let recovery_packet = encode_audio_recovery_packet_v3_readonly(anchor);
    let recovery_bits = bytes_to_bits(&recovery_packet);
    let recovery_frames = audio_recovery_v3_readonly_frames_per_packet();
    let raw_total = AUDIO_RECOVERY_V3_READONLY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY;
    let (bin_lo, bin_hi) = audio_band_bins(sample_rate);

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let mut total_occurrences = 0usize;
    let mut full_modified_pairs = 0usize;
    let mut sparse_modified_pairs = 0usize;
    let mut rms_min = f32::MAX;
    let mut rms_max = 0.0_f32;
    let mut rms_sum = 0.0_f32;
    let mut low_energy_frames = 0usize;
    let mut transient_frames = 0usize;
    let mut noise_like_frames = 0usize;
    let mut strength_min = f32::MAX;
    let mut strength_max = 0.0_f32;
    let mut strength_sum = 0.0_f32;
    let mut strength_count = 0usize;

    for frame_idx in 0..frame_count {
        let offset = frame_idx * FRAME_SIZE;
        let frame = &source_samples[offset..offset + FRAME_SIZE];
        let frame_rms = rms_energy(frame);
        rms_min = rms_min.min(frame_rms);
        rms_max = rms_max.max(frame_rms);
        rms_sum += frame_rms;

        let mut input = frame.to_vec();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut input, &mut spectrum)
            .map_err(|error| WatermarkError::EmbedFailed(format!("FFT failed: {error}")))?;
        let frame_profile = AudioV3FrameProfile::from_frame_and_spectrum(
            frame, frame_rms, &spectrum, bin_lo, bin_hi,
        );
        if frame_profile.low_energy {
            low_energy_frames += 1;
        }
        if frame_profile.transient {
            transient_frames += 1;
        }
        if frame_profile.noise_like {
            noise_like_frames += 1;
        }

        if frame_rms < SILENCE_THRESHOLD {
            continue;
        }

        let contrast = v3_recovery_frame_contrast(frame_profile);
        strength_min = strength_min.min(contrast);
        strength_max = strength_max.max(contrast);
        strength_sum += contrast;
        strength_count += 1;

        let local_frame = frame_idx % recovery_frames;
        let raw_start = local_frame * AUDIO_RECOVERY_BITS_PER_FRAME;
        if raw_start >= raw_total {
            continue;
        }
        for raw_idx in raw_start..(raw_start + AUDIO_RECOVERY_BITS_PER_FRAME).min(raw_total) {
            let bit_slot = raw_idx % AUDIO_RECOVERY_BITS_PER_FRAME;
            let expected_bit = recovery_bits[raw_idx / AUDIO_RECOVERY_REDUNDANCY];
            let mut sparse_slot_updates = 0usize;
            let mut sparse_strong_expected = 0usize;
            for lane in 0..AUDIO_RECOVERY_BIT_LANES {
                let pair_idx =
                    AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
                let bin = bin_lo + pair_idx * RELATIVE_PAIR_WIDTH;
                let should_update =
                    relative_pair_needs_update(&spectrum, bin, expected_bit, contrast);
                if should_update {
                    full_modified_pairs += 1;
                } else {
                    sparse_strong_expected += 1;
                }
                total_occurrences += 1;
            }
            if sparse_strong_expected <= AUDIO_RECOVERY_BIT_LANES / 2 {
                for lane in 0..AUDIO_RECOVERY_BIT_LANES {
                    if sparse_strong_expected > AUDIO_RECOVERY_BIT_LANES / 2 {
                        break;
                    }
                    let pair_idx =
                        AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
                    let bin = bin_lo + pair_idx * RELATIVE_PAIR_WIDTH;
                    if relative_pair_needs_update(&spectrum, bin, expected_bit, contrast) {
                        sparse_slot_updates += 1;
                        sparse_strong_expected += 1;
                    }
                }
            }
            sparse_modified_pairs += sparse_slot_updates;
        }
    }
    let rms_mean = rms_sum / frame_count as f32;
    let low_energy_frame_ratio = low_energy_frames as f32 / frame_count as f32;
    let transient_frame_ratio = transient_frames as f32 / frame_count as f32;
    let noise_like_frame_ratio = noise_like_frames as f32 / frame_count as f32;
    let noise_floor_sparse_recovery = is_noise_floor_sparse_recovery_profile(
        rms_min,
        rms_mean,
        rms_max,
        low_energy_frame_ratio,
        transient_frame_ratio,
        noise_like_frame_ratio,
    );
    let modified_pairs = if noise_floor_sparse_recovery {
        sparse_modified_pairs
    } else {
        full_modified_pairs
    };

    Ok(AudioV3RecoveryEmbeddingPlan {
        frame_count,
        rms_min,
        rms_mean,
        rms_max,
        low_energy_frame_ratio,
        transient_frame_ratio,
        noise_like_frame_ratio,
        strength_min: if strength_count == 0 {
            0.0
        } else {
            strength_min
        },
        strength_mean: if strength_count == 0 {
            0.0
        } else {
            strength_sum / strength_count as f32
        },
        strength_max,
        modified_pair_ratio: if total_occurrences == 0 {
            0.0
        } else {
            modified_pairs as f32 / total_occurrences as f32
        },
        noise_floor_sparse_recovery,
    })
}

pub fn audio_v3_quality_diagnostics(
    source_samples: &[f32],
    protected_samples: &[f32],
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<AudioV3QualityDiagnostics, WatermarkError> {
    let plan = plan_v3_recovery_embedding(source_samples, anchor, CANONICAL_SAMPLE_RATE)?;
    let extraction_confidence =
        audio_v3_recovery_extraction_confidence(protected_samples, anchor).unwrap_or(0.0);

    Ok(AudioV3QualityDiagnostics {
        frame_count: plan.frame_count,
        short_time_rms_min: plan.rms_min,
        short_time_rms_mean: plan.rms_mean,
        short_time_rms_max: plan.rms_max,
        low_energy_frame_ratio: plan.low_energy_frame_ratio,
        transient_frame_ratio: plan.transient_frame_ratio,
        noise_like_frame_ratio: plan.noise_like_frame_ratio,
        embedding_strength_min: plan.strength_min,
        embedding_strength_mean: plan.strength_mean,
        embedding_strength_max: plan.strength_max,
        modified_pair_ratio: plan.modified_pair_ratio,
        noise_floor_sparse_recovery: plan.noise_floor_sparse_recovery,
        extraction_confidence,
    })
}

fn audio_v3_recovery_extraction_confidence(
    samples: &[f32],
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<f32, WatermarkError> {
    let expected_packet = encode_audio_recovery_packet_v3_readonly(anchor);
    let expected_bits = bytes_to_bits(&expected_packet);
    let num_frames = samples.len() / FRAME_SIZE;
    if num_frames == 0 {
        return Err(WatermarkError::ExtractFailed(
            "audio too short for V3 confidence".into(),
        ));
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let (bin_lo, bin_hi) = audio_band_bins(CANONICAL_SAMPLE_RATE);
    let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
    if !audio_recovery_v3_readonly_extract_enabled(num_frames, usable_pairs) {
        return Err(WatermarkError::ExtractFailed(
            "audio readonly recovery not available for V3 confidence".into(),
        ));
    }

    let recovery_frames = audio_recovery_v3_readonly_frames_per_packet();
    let frame_recovery_bits = (0..num_frames)
        .map(|frame_idx| {
            let offset = frame_idx * FRAME_SIZE;
            let frame = &samples[offset..offset + FRAME_SIZE];
            if rms_energy(frame) < SILENCE_THRESHOLD {
                return Ok(None);
            }

            let mut input = frame.to_vec();
            let mut spectrum = fft.make_output_vec();
            fft.process(&mut input, &mut spectrum)
                .map_err(|error| WatermarkError::ExtractFailed(format!("FFT failed: {error}")))?;
            Ok(Some(extract_audio_recovery_frame_bits(&spectrum, bin_lo)))
        })
        .collect::<Result<Vec<_>, WatermarkError>>()?;

    let raw_total = AUDIO_RECOVERY_V3_READONLY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY;
    let scan_limit = recovery_frames.min(num_frames.saturating_sub(recovery_frames) + 1);
    let mut best_confidence = 0.0_f32;
    for start_frame in 0..scan_limit {
        let mut raw_votes = vec![0i32; raw_total];
        let mut raw_seen = vec![false; raw_total];
        for frame_idx in start_frame..num_frames {
            let Some(frame_bits) = &frame_recovery_bits[frame_idx] else {
                continue;
            };
            let local_frame = (frame_idx - start_frame) % recovery_frames;
            let raw_start = local_frame * AUDIO_RECOVERY_BITS_PER_FRAME;
            if raw_start >= raw_total {
                continue;
            }
            for (slot, bit) in frame_bits.iter().copied().enumerate() {
                let raw_idx = raw_start + slot;
                if raw_idx >= raw_total {
                    break;
                }
                raw_seen[raw_idx] = true;
                raw_votes[raw_idx] += if bit { 1 } else { -1 };
            }
        }

        if raw_seen.iter().any(|seen| !seen) {
            continue;
        }
        let raw_bits = raw_votes.iter().map(|&vote| vote > 0).collect::<Vec<_>>();
        let bits = majority_bits_with_redundancy(
            &raw_bits,
            AUDIO_RECOVERY_V3_READONLY_PACKET_BITS,
            AUDIO_RECOVERY_REDUNDANCY,
        );
        let matches = bits
            .iter()
            .zip(expected_bits.iter())
            .filter(|(left, right)| left == right)
            .count();
        best_confidence = best_confidence.max(matches as f32 / expected_bits.len() as f32);
    }
    Ok(best_confidence)
}

#[derive(Debug, Clone, Copy)]
struct AudioV3FrameProfile {
    low_energy: bool,
    transient: bool,
    noise_like: bool,
}

impl AudioV3FrameProfile {
    fn from_frame_and_spectrum(
        frame: &[f32],
        rms: f32,
        spectrum: &[Complex<f32>],
        bin_lo: usize,
        bin_hi: usize,
    ) -> Self {
        let peak = frame
            .iter()
            .fold(0.0_f32, |acc, sample| acc.max(sample.abs()));
        let crest_factor = peak / rms.max(f32::EPSILON);
        let spectral_flatness = spectral_flatness(spectrum, bin_lo, bin_hi);
        let low_energy = rms < V3_LOW_ENERGY_RMS_THRESHOLD;
        let transient = crest_factor >= V3_TRANSIENT_CREST_FACTOR_THRESHOLD;
        let noise_like = spectral_flatness >= V3_NOISE_SPECTRAL_FLATNESS_THRESHOLD;
        Self {
            low_energy,
            transient,
            noise_like,
        }
    }
}

fn v3_recovery_frame_contrast(profile: AudioV3FrameProfile) -> f32 {
    let mut contrast = V3_RECOVERY_BASE_CONTRAST;
    if profile.noise_like {
        contrast *= V3_RECOVERY_NOISE_CONTRAST_FACTOR;
    }
    if profile.transient {
        contrast *= V3_RECOVERY_TRANSIENT_CONTRAST_FACTOR;
    }
    if profile.low_energy {
        contrast *= V3_RECOVERY_LOW_ENERGY_CONTRAST_FACTOR;
    }
    contrast.clamp(V3_RECOVERY_MIN_CONTRAST, V3_RECOVERY_BASE_CONTRAST)
}

fn is_noise_floor_sparse_recovery_profile(
    rms_min: f32,
    rms_mean: f32,
    rms_max: f32,
    low_energy_frame_ratio: f32,
    transient_frame_ratio: f32,
    noise_like_frame_ratio: f32,
) -> bool {
    let rms_span_ratio = (rms_max - rms_min) / rms_mean.max(f32::EPSILON);
    (0.06..=0.11).contains(&rms_mean)
        && rms_span_ratio <= 0.035
        && low_energy_frame_ratio <= 0.05
        && transient_frame_ratio <= 0.05
        && (0.35..=0.90).contains(&noise_like_frame_ratio)
}

fn spectral_flatness(spectrum: &[Complex<f32>], bin_lo: usize, bin_hi: usize) -> f32 {
    let mut log_sum = 0.0_f32;
    let mut linear_sum = 0.0_f32;
    let mut count = 0usize;
    for bin in bin_lo..bin_hi.min(spectrum.len()) {
        let magnitude = spectrum[bin].norm().max(1.0e-12);
        log_sum += magnitude.ln();
        linear_sum += magnitude;
        count += 1;
    }
    if count == 0 || linear_sum <= 0.0 {
        return 0.0;
    }
    let geometric = (log_sum / count as f32).exp();
    let arithmetic = linear_sum / count as f32;
    (geometric / arithmetic).clamp(0.0, 1.0)
}

fn qim_extract(mag: f32, delta: f32) -> bool {
    let half = delta / 2.0;
    let idx = (mag / half).round() as i32;
    (idx & 1) == 1
}

fn rms_energy(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = frame.iter().map(|&s| s * s).sum();
    (sum_sq / frame.len() as f32).sqrt()
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

    fn second_payload() -> WatermarkPayload {
        WatermarkPayload::new(
            [0x24; 8],
            1_700_000_100,
            [0xBA; 4],
            [0xDC; 2],
            Default::default(),
        )
    }

    fn sample_v3_anchor() -> WatermarkPayloadV3MinimalAnchor {
        WatermarkPayloadV3MinimalAnchor::new(crate::PayloadV3MinimalAnchorBuildInput {
            watermark_id: [
                0x51, 0x52, 0x53, 0x54, 0x61, 0x62, 0x63, 0x64, 0x71, 0x72, 0x73, 0x74, 0x81, 0x82,
                0x83, 0x84,
            ],
        })
        .unwrap()
    }

    fn make_wav_bytes() -> Vec<u8> {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();

        for i in 0..(44_100 * MIN_AUDIO_PROTECTION_SECONDS as usize) {
            let t = i as f32 / 44_100.0;
            let sample = (t * 440.0 * std::f32::consts::TAU).sin() * 0.2;
            let v = (sample * 32767.0) as i16;
            writer.write_sample(v).unwrap();
        }

        writer.finalize().unwrap();
        cursor.into_inner()
    }

    fn make_v3_recovery_samples(anchor: &WatermarkPayloadV3MinimalAnchor) -> Vec<f32> {
        let mut samples: Vec<f32> = (0..(FRAME_SIZE * 320))
            .map(|i| {
                let t = i as f32 / 44_100.0;
                let tone_a = (t * 440.0 * std::f32::consts::TAU).sin() * 0.16;
                let tone_b = (t * 880.0 * std::f32::consts::TAU).sin() * 0.08;
                tone_a + tone_b
            })
            .collect();
        let recovery_packet = encode_audio_recovery_packet_v3_readonly(anchor);
        let recovery_bits = bytes_to_bits(&recovery_packet);
        let num_frames = samples.len() / FRAME_SIZE;
        let recovery_frames = audio_recovery_v3_readonly_frames_per_packet();
        let (bin_lo, bin_hi) = audio_band_bins(CANONICAL_SAMPLE_RATE);
        let usable_pairs = (bin_hi - bin_lo) / RELATIVE_PAIR_WIDTH;
        assert!(usable_pairs > AUDIO_RECOVERY_PAIR_OFFSET + 16);
        let payload_pair_offset = AUDIO_RECOVERY_PAIR_OFFSET;
        let payload_pairs = usable_pairs.saturating_sub(payload_pair_offset);
        assert!(payload_pairs > 0);

        let mut planner = RealFftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FRAME_SIZE);
        let ifft = planner.plan_fft_inverse(FRAME_SIZE);

        for frame_idx in 0..num_frames {
            let offset = frame_idx * FRAME_SIZE;
            let frame = &mut samples[offset..offset + FRAME_SIZE];
            let mut input = frame.to_vec();
            let mut spectrum = fft.make_output_vec();
            fft.process(&mut input, &mut spectrum).unwrap();

            let local_frame = frame_idx % recovery_frames;
            let raw_start = local_frame * AUDIO_RECOVERY_BITS_PER_FRAME;
            let raw_total = AUDIO_RECOVERY_V3_READONLY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY;
            if raw_start < raw_total {
                for raw_idx in raw_start..(raw_start + AUDIO_RECOVERY_BITS_PER_FRAME).min(raw_total)
                {
                    let bit_slot = raw_idx % AUDIO_RECOVERY_BITS_PER_FRAME;
                    let bit = recovery_bits[raw_idx / AUDIO_RECOVERY_REDUNDANCY];
                    for lane in 0..AUDIO_RECOVERY_BIT_LANES {
                        let pair_idx =
                            AUDIO_MARKER_PAIR_OFFSET + bit_slot * AUDIO_RECOVERY_BIT_LANES + lane;
                        let bin = bin_lo + pair_idx * RELATIVE_PAIR_WIDTH;
                        embed_relative_pair(&mut spectrum, bin, bit, DEFAULT_QIM_DELTA * 1.6);
                    }
                }
            }

            let mut output = ifft.make_output_vec();
            ifft.process(&mut spectrum, &mut output).unwrap();
            let scale = 1.0 / FRAME_SIZE as f32;
            for (j, sample) in output.iter().enumerate().take(FRAME_SIZE) {
                frame[j] = sample * scale;
            }
        }
        samples
    }

    #[test]
    fn wav_bytes_roundtrip() {
        let input = make_wav_bytes();
        let payload = sample_payload();
        let embedded = embed_watermark_wav_bytes(&input, &payload).unwrap();
        let extracted = extract_watermark_wav_bytes(&embedded).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn wav_bytes_rejects_audio_shorter_than_protection_minimum() {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();

        for i in 0..(44_100 * 10) {
            let t = i as f32 / 44_100.0;
            let sample = (t * 440.0 * std::f32::consts::TAU).sin() * 0.2;
            let v = (sample * 32767.0) as i16;
            writer.write_sample(v).unwrap();
        }

        writer.finalize().unwrap();
        let err = embed_watermark_wav_bytes(&cursor.into_inner(), &sample_payload()).unwrap_err();

        assert!(matches!(
            err,
            WatermarkError::EmbedFailed(message)
                if message.contains("audio_protection_min_duration")
                    && message.contains("30 seconds")
        ));
    }

    #[test]
    fn wav_bytes_rejects_existing_watermark_by_default() {
        let input = make_wav_bytes();
        let payload = sample_payload();
        let embedded = embed_watermark_wav_bytes(&input, &payload).unwrap();
        let err = embed_watermark_wav_bytes(&embedded, &second_payload()).unwrap_err();

        assert!(matches!(
            err,
            WatermarkError::AlreadyWatermarked { existing_uid }
                if existing_uid == payload.watermark_uid()
        ));
    }

    #[test]
    fn wav_bytes_allow_rewrite_replaces_existing_watermark() {
        let input = make_wav_bytes();
        let payload = sample_payload();
        let second = second_payload();
        let embedded = embed_watermark_wav_bytes(&input, &payload).unwrap();
        let rewritten = embed_watermark_wav_bytes_allow_rewrite(&embedded, &second).unwrap();
        let extracted = extract_watermark_wav_bytes(&rewritten).unwrap();

        assert_eq!(extracted.watermark_uid(), second.watermark_uid());
    }

    #[test]
    fn samples_roundtrip() {
        let mut samples: Vec<f32> = (0..(FRAME_SIZE * 320))
            .map(|i| {
                let t = i as f32 / 44_100.0;
                (t * 440.0 * std::f32::consts::TAU).sin() * 0.2
            })
            .collect();
        let payload = sample_payload();

        embed_watermark_samples(&mut samples, &payload).unwrap();
        let extracted = extract_watermark_samples(&samples).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn samples_reject_existing_watermark_by_default() {
        let mut samples: Vec<f32> = (0..(FRAME_SIZE * 320))
            .map(|i| {
                let t = i as f32 / 44_100.0;
                (t * 440.0 * std::f32::consts::TAU).sin() * 0.2
            })
            .collect();
        let payload = sample_payload();

        embed_watermark_samples(&mut samples, &payload).unwrap();
        let err = embed_watermark_samples(&mut samples, &second_payload()).unwrap_err();

        assert!(matches!(
            err,
            WatermarkError::AlreadyWatermarked { existing_uid }
                if existing_uid == payload.watermark_uid()
        ));
    }

    #[test]
    fn samples_survive_uniform_volume_changes() {
        let mut samples: Vec<f32> = (0..(FRAME_SIZE * 320))
            .map(|i| {
                let t = i as f32 / 44_100.0;
                let tone_a = (t * 440.0 * std::f32::consts::TAU).sin() * 0.16;
                let tone_b = (t * 880.0 * std::f32::consts::TAU).sin() * 0.08;
                tone_a + tone_b
            })
            .collect();
        let payload = sample_payload();

        embed_watermark_samples(&mut samples, &payload).unwrap();

        let quieter = samples
            .iter()
            .map(|sample| sample * 0.8)
            .collect::<Vec<_>>();
        let extracted = extract_watermark_samples(&quieter).unwrap();
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());

        let louder = samples
            .iter()
            .map(|sample| (sample * 1.2).clamp(-1.0, 1.0))
            .collect::<Vec<_>>();
        let extracted = extract_watermark_samples(&louder).unwrap();
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
    }

    #[test]
    fn low_sample_rate_samples_roundtrip() {
        let mut samples: Vec<f32> = (0..16_384)
            .map(|i| {
                let t = i as f32 / 22_050.0;
                let tone_a = (t * 440.0 * std::f32::consts::TAU).sin() * 0.16;
                let tone_b = (t * 880.0 * std::f32::consts::TAU).sin() * 0.08;
                tone_a + tone_b
            })
            .collect();
        let payload = sample_payload();

        embed_watermark_samples_allow_rewrite_with_delta_and_rate(
            &mut samples,
            &payload,
            DEFAULT_QIM_DELTA,
            22_050,
        )
        .unwrap();
        let extracted =
            extract_watermark_samples_with_delta_and_rate(&samples, DEFAULT_QIM_DELTA, 22_050)
                .unwrap();
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
    }

    #[test]
    fn audio_marker_packet_roundtrip() {
        let payload = sample_payload();
        let payload_bytes = encode_payload(&payload);
        let packets = audio_marker_packets(&payload_bytes);
        let hit = decode_audio_marker_packet(&packets[3]).unwrap();

        assert_eq!(hit.slice_id, 3);
        assert_eq!(hit.payload_tag, audio_payload_tag(&payload_bytes));
    }

    #[test]
    fn audio_recovery_packet_roundtrip() {
        let payload = sample_payload();
        let payload_bytes = encode_payload(&payload);
        let packet = encode_audio_recovery_packet(&payload_bytes);
        let recovered = decode_audio_recovery_packet(&packet).unwrap();

        assert_eq!(recovered.watermark_uid(), payload.watermark_uid());
    }

    #[test]
    fn audio_recovery_packet_v3_readonly_roundtrips_minimal_anchor_without_v2_decode() {
        let anchor = sample_v3_anchor();
        let packet = encode_audio_recovery_packet_v3_readonly(&anchor);

        assert_eq!(packet.len(), AUDIO_RECOVERY_V3_READONLY_PACKET_BYTES);
        assert!(decode_audio_recovery_packet(&packet).is_err());
        let decoded = decode_audio_recovery_packet_v3_readonly(&packet).unwrap();

        assert!(decoded.is_v3_minimal_anchor());
        assert_eq!(decoded.watermark_uid(), anchor.watermark_uid());
        assert_eq!(decoded.protocol_version(), 3);
        assert_eq!(decoded.payload_bytes_length(), 39);
        assert_eq!(decoded.payload_auth_status(), "verified");
    }

    #[test]
    fn audio_readonly_candidate_extracts_v3_recovery_packet_for_migration_bridge() {
        let anchor = sample_v3_anchor();
        let samples = make_v3_recovery_samples(&anchor);

        assert!(extract_watermark_samples(&samples).is_err());
        let decoded = extract_watermark_samples_readonly_candidate(&samples).unwrap();

        assert!(decoded.is_v3_minimal_anchor());
        assert_eq!(decoded.watermark_uid(), anchor.watermark_uid());
        assert_eq!(decoded.protocol_version(), 3);
        assert_eq!(decoded.payload_bytes_length(), 39);
        assert_eq!(decoded.payload_auth_status(), "verified");
    }

    #[test]
    fn noise_floor_migrated_candidate_interface_reports_payload_not_found_for_valid_wav() {
        let input = make_wav_bytes();
        let err = extract_audio_noise_floor_migrated_band_v1_candidate_wav_bytes(&input)
            .expect_err("candidate scan must not report payload for an unprotected WAV");

        assert_eq!(
            err.code,
            AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadNotFound
        );
        assert_eq!(
            err.extractor_path,
            AUDIO_NOISE_FLOOR_MIGRATED_BAND_V1_CANDIDATE_PATH
        );
    }

    #[test]
    fn noise_floor_migrated_candidate_scan_does_not_misdetect_legacy_v3_recovery() {
        let anchor = sample_v3_anchor();
        let samples = make_v3_recovery_samples(&anchor);
        let err = extract_audio_noise_floor_migrated_band_v1_candidate_samples_with_rate(
            &samples,
            CANONICAL_SAMPLE_RATE,
        )
        .expect_err("candidate scan must not claim legacy V3 recovery samples");

        assert_eq!(
            err.code,
            AudioNoiseFloorMigrationCandidateFailureCode::CandidatePayloadNotFound
        );
        let decoded = extract_watermark_samples_readonly_candidate(&samples).unwrap();
        assert_eq!(decoded.watermark_uid(), anchor.watermark_uid());
        assert_eq!(decoded.protocol_version(), 3);
        assert_eq!(decoded.payload_bytes_length(), 39);
    }

    #[test]
    fn noise_floor_migrated_candidate_interface_reports_short_audio() {
        let samples = vec![0.0_f32; FRAME_SIZE - 1];
        let err = extract_audio_noise_floor_migrated_band_v1_candidate_samples_with_rate(
            &samples, 44_100,
        )
        .expect_err("short audio must be classified before candidate fallback");

        assert_eq!(
            err.code,
            AudioNoiseFloorMigrationCandidateFailureCode::CandidateAudioTooShort
        );
    }

    #[test]
    fn samples_embed_detects_audio_slice_markers() {
        let mut samples: Vec<f32> = (0..(FRAME_SIZE * 512))
            .map(|i| {
                let t = i as f32 / 44_100.0;
                let tone_a = (t * 440.0 * std::f32::consts::TAU).sin() * 0.16;
                let tone_b = (t * 880.0 * std::f32::consts::TAU).sin() * 0.08;
                tone_a + tone_b
            })
            .collect();
        let payload = sample_payload();
        let payload_bytes = encode_payload(&payload);

        embed_watermark_samples_allow_rewrite(&mut samples, &payload).unwrap();
        let hits = extract_audio_marker_hits(&samples, CANONICAL_SAMPLE_RATE).unwrap();

        assert!(hits.len() >= 4);
        assert!(hits
            .iter()
            .any(|hit| hit.payload_tag == audio_payload_tag(&payload_bytes)));
    }

    #[test]
    fn clipped_samples_report_v2_marker_boundary() {
        let mut samples: Vec<f32> = (0..(FRAME_SIZE * 512))
            .map(|i| {
                let t = i as f32 / 44_100.0;
                let tone_a = (t * 440.0 * std::f32::consts::TAU).sin() * 0.16;
                let tone_b = (t * 880.0 * std::f32::consts::TAU).sin() * 0.08;
                tone_a + tone_b
            })
            .collect();
        let payload = sample_payload();

        embed_watermark_samples_allow_rewrite(&mut samples, &payload).unwrap();
        let recovery_frames = audio_recovery_frames_per_packet();
        let start = FRAME_SIZE * recovery_frames;
        let end = FRAME_SIZE * (recovery_frames * 3);
        let clipped = samples[start..end].to_vec();
        let marker_count = detect_audio_marker_count(&clipped, CANONICAL_SAMPLE_RATE).unwrap();

        assert_eq!(marker_count, 0);
    }

    #[test]
    fn clipped_samples_recover_v2_payload_from_mid_clip() {
        let mut samples: Vec<f32> = (0..(FRAME_SIZE * 512))
            .map(|i| {
                let t = i as f32 / 44_100.0;
                let tone_a = (t * 440.0 * std::f32::consts::TAU).sin() * 0.16;
                let tone_b = (t * 880.0 * std::f32::consts::TAU).sin() * 0.08;
                tone_a + tone_b
            })
            .collect();
        let payload = sample_payload();

        embed_watermark_samples_allow_rewrite(&mut samples, &payload).unwrap();
        let start = FRAME_SIZE * 160;
        let end = FRAME_SIZE * 360;
        let clipped = samples[start..end].to_vec();

        let recovered =
            extract_watermark_samples_recovery(&clipped, CANONICAL_SAMPLE_RATE).unwrap();

        assert_eq!(recovered, payload);
    }

    #[test]
    fn samples_recover_payload_from_audio_recovery_packet() {
        let mut samples: Vec<f32> = (0..(FRAME_SIZE * 512))
            .map(|i| {
                let t = i as f32 / 44_100.0;
                let tone_a = (t * 440.0 * std::f32::consts::TAU).sin() * 0.16;
                let tone_b = (t * 880.0 * std::f32::consts::TAU).sin() * 0.08;
                tone_a + tone_b
            })
            .collect();
        let payload = sample_payload();

        embed_watermark_samples_allow_rewrite(&mut samples, &payload).unwrap();
        let recovered =
            extract_watermark_samples_recovery(&samples, CANONICAL_SAMPLE_RATE).unwrap();

        assert_eq!(recovered.watermark_uid(), payload.watermark_uid());
    }

    #[test]
    fn resampled_wav_roundtrip_uses_canonical_fallback() {
        let mut samples: Vec<f32> = (0..32_768)
            .map(|i| {
                let t = i as f32 / 44_100.0;
                let tone_a = (t * 440.0 * std::f32::consts::TAU).sin() * 0.16;
                let tone_b = (t * 880.0 * std::f32::consts::TAU).sin() * 0.08;
                tone_a + tone_b
            })
            .collect();
        let payload = sample_payload();

        embed_watermark_samples_allow_rewrite_with_delta_and_rate(
            &mut samples,
            &payload,
            DEFAULT_QIM_DELTA,
            44_100,
        )
        .unwrap();
        let downsampled = resample_linear(&samples, 44_100, 22_050);
        let restored = resample_linear(&downsampled, 22_050, 44_100);
        let extracted =
            extract_watermark_samples_with_delta_and_rate(&restored, DEFAULT_QIM_DELTA, 44_100)
                .unwrap();
        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
    }

    #[test]
    fn audio_protection_input_accepts_common_specs_and_rejects_boundaries() {
        assert!(validate_audio_protection_input(8_000, 1, 30.0, 30).is_ok());
        assert!(validate_audio_protection_input(48_000, 2, 1_200.0, 30).is_ok());
        assert_eq!(
            validate_audio_protection_input(7_999, 1, 30.0, 30).unwrap_err(),
            "audio_protection_sample_rate_too_low"
        );
        assert_eq!(
            validate_audio_protection_input(48_001, 1, 30.0, 30).unwrap_err(),
            "audio_protection_sample_rate_too_high"
        );
        assert_eq!(
            validate_audio_protection_input(48_000, 3, 30.0, 30).unwrap_err(),
            "audio_protection_channels_unsupported"
        );
        assert!(validate_audio_protection_input(48_000, 2, 29.0, 30)
            .unwrap_err()
            .contains("audio_protection_min_duration"));
        assert!(validate_audio_protection_input(48_000, 2, 1_200.001, 30)
            .unwrap_err()
            .contains("audio_protection_max_duration"));
    }

    #[test]
    fn audio_protection_file_size_accepts_exact_limit_and_rejects_next_byte() {
        assert!(validate_audio_protection_file_size(MAX_AUDIO_PROTECTION_BYTES).is_ok());
        assert_eq!(
            validate_audio_protection_file_size(MAX_AUDIO_PROTECTION_BYTES + 1).unwrap_err(),
            format!(
                "audio_protection_file_size_limit_exceeded: {} bytes exceeds maximum {} bytes",
                MAX_AUDIO_PROTECTION_BYTES + 1,
                MAX_AUDIO_PROTECTION_BYTES
            )
        );
    }

    #[test]
    fn wav_writer_preserves_24_bit_and_float_output_specifications() {
        let samples = [0.0, 0.25, -0.25, 0.75, -0.75, 0.5];
        for spec in [
            hound::WavSpec {
                channels: 1,
                sample_rate: 48_000,
                bits_per_sample: 24,
                sample_format: hound::SampleFormat::Int,
            },
            hound::WavSpec {
                channels: 2,
                sample_rate: 44_100,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        ] {
            let bytes = write_wav_samples(&samples, spec).unwrap();
            let reader = hound::WavReader::new(Cursor::new(bytes)).unwrap();
            assert_eq!(reader.spec(), spec);
        }
    }

    #[test]
    fn wav_watermark_roundtrip_preserves_24_bit_and_float_specifications() {
        let payload = sample_payload();
        for spec in [
            hound::WavSpec {
                channels: 1,
                sample_rate: 16_000,
                bits_per_sample: 24,
                sample_format: hound::SampleFormat::Int,
            },
            hound::WavSpec {
                channels: 1,
                sample_rate: 16_000,
                bits_per_sample: 32,
                sample_format: hound::SampleFormat::Float,
            },
        ] {
            let samples = (0..(spec.sample_rate * 31))
                .map(|index| {
                    let time = index as f32 / spec.sample_rate as f32;
                    (time * 440.0 * std::f32::consts::TAU).sin() * 0.2
                })
                .collect::<Vec<_>>();
            let input = write_wav_samples(&samples, spec).unwrap();
            let embedded = embed_watermark_wav_bytes(&input, &payload).unwrap();
            let reader = hound::WavReader::new(Cursor::new(embedded.clone())).unwrap();
            assert_eq!(reader.spec(), spec);
            let extracted = extract_watermark_wav_bytes(&embedded).unwrap();
            assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
        }
    }
}

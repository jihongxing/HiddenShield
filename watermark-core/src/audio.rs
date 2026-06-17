use realfft::num_complex::Complex;
use realfft::RealFftPlanner;
use std::io::Cursor;

use crate::error::WatermarkError;
use crate::payload::{
    bits_to_bytes, bytes_to_bits, decode_payload, encode_payload, WatermarkPayload,
};

const FRAME_SIZE: usize = 4096;
const CANONICAL_SAMPLE_RATE: u32 = 44_100;
pub const MIN_AUDIO_PROTECTION_SECONDS: u32 = 30;
const BAND_LO_BIN: usize = 186;
const BAND_HI_BIN: usize = 743;
const BAND_LO_HZ: f32 = BAND_LO_BIN as f32 * CANONICAL_SAMPLE_RATE as f32 / FRAME_SIZE as f32;
const BAND_HI_HZ: f32 = BAND_HI_BIN as f32 * CANONICAL_SAMPLE_RATE as f32 / FRAME_SIZE as f32;
pub const DEFAULT_QIM_DELTA: f32 = 0.02;
pub const BALANCED_QIM_DELTA: f32 = 0.014;
const KNOWN_QIM_DELTAS: [f32; 2] = [DEFAULT_QIM_DELTA, BALANCED_QIM_DELTA];
const SILENCE_THRESHOLD: f32 = 0.001;
const PAYLOAD_BITS: usize = 32 * 8;
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
const AUDIO_RECOVERY_PACKET_BYTES: usize = 4 + 32 + AUDIO_RECOVERY_CHECKSUM_BYTES;
const AUDIO_RECOVERY_PACKET_BITS: usize = AUDIO_RECOVERY_PACKET_BYTES * 8;
const AUDIO_RECOVERY_REDUNDANCY: usize = 3;
const AUDIO_RECOVERY_BITS_PER_FRAME: usize = 18;
const AUDIO_RECOVERY_BIT_LANES: usize = 3;
const AUDIO_RECOVERY_PREAMBLE_MAX_BIT_ERRORS: usize = 4;
const AUDIO_RECOVERY_PAYLOAD_MAX_BIT_CORRECTIONS: usize = 3;
const AUDIO_MARKER_PAIR_OFFSET: usize = AUDIO_MARKER_BITS_PER_FRAME * AUDIO_MARKER_BIT_LANES + 4;
const AUDIO_RECOVERY_PAIR_OFFSET: usize =
    AUDIO_MARKER_PAIR_OFFSET + AUDIO_RECOVERY_BITS_PER_FRAME * AUDIO_RECOVERY_BIT_LANES + 4;
const AUDIO_PHASE_SCAN_STEPS: [usize; 8] = [0, 512, 1024, 1536, 2048, 2560, 3072, 3584];

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
        validate_wav_duration_for_protection(
            samples.len(),
            spec.sample_rate,
            spec.channels,
            min_duration_seconds,
        )?;
    }
    embed_watermark_samples_allow_rewrite_with_delta_and_rate(
        &mut samples,
        payload,
        delta,
        spec.sample_rate,
    )?;
    write_wav_samples(&samples, spec)
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
    let mut arr = [0u8; 32];
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
    let mut arr = [0u8; 32];
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
    let mut arr = [0u8; 32];
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
    let out_spec = hound::WavSpec {
        channels: spec.channels,
        sample_rate: spec.sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };
    let mut cursor = Cursor::new(Vec::new());
    let mut writer = hound::WavWriter::new(&mut cursor, out_spec)
        .map_err(|e| WatermarkError::EmbedFailed(format!("create WAV: {e}")))?;

    for &sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let int_val = (clamped * 32767.0) as i16;
        writer
            .write_sample(int_val)
            .map_err(|e| WatermarkError::EmbedFailed(format!("write sample: {e}")))?;
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
    payload_bytes: &[u8; 32],
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

fn audio_payload_tag(payload_bytes: &[u8; 32]) -> [u8; 2] {
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

fn audio_recovery_frames_per_packet() -> usize {
    (AUDIO_RECOVERY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY).div_ceil(AUDIO_RECOVERY_BITS_PER_FRAME)
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

fn encode_audio_recovery_packet(payload_bytes: &[u8; 32]) -> [u8; AUDIO_RECOVERY_PACKET_BYTES] {
    let mut packet = [0u8; AUDIO_RECOVERY_PACKET_BYTES];
    packet[0..4].copy_from_slice(&AUDIO_RECOVERY_PREAMBLE);
    packet[4..36].copy_from_slice(payload_bytes);
    let checksum = audio_recovery_checksum(payload_bytes);
    packet[36..38].copy_from_slice(&checksum);
    packet
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

    let mut payload_bytes = [0u8; 32];
    payload_bytes.copy_from_slice(&bytes[4..36]);
    if bytes[36..38] != audio_recovery_checksum(&payload_bytes) {
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

    let mut payload_bytes = [0u8; 32];
    payload_bytes.copy_from_slice(&bytes[4..36]);
    let mut checksum = [0u8; AUDIO_RECOVERY_CHECKSUM_BYTES];
    checksum.copy_from_slice(&bytes[36..38]);
    correct_audio_recovery_payload_bits(
        payload_bytes,
        checksum,
        AUDIO_RECOVERY_PAYLOAD_MAX_BIT_CORRECTIONS,
    )
}

fn correct_audio_recovery_payload_bits(
    mut payload_bytes: [u8; 32],
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

fn flip_payload_bit(payload_bytes: &mut [u8; 32], bit_idx: usize) {
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

fn audio_recovery_checksum(payload_bytes: &[u8; 32]) -> [u8; AUDIO_RECOVERY_CHECKSUM_BYTES] {
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
    let recovery_frames = audio_recovery_frames_per_packet();
    let local_frame = (frame_idx + frame_shift) % recovery_frames;
    let raw_start = local_frame * AUDIO_RECOVERY_BITS_PER_FRAME;
    let raw_total = AUDIO_RECOVERY_PACKET_BITS * AUDIO_RECOVERY_REDUNDANCY;
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

    #[test]
    fn wav_bytes_roundtrip() {
        let input = make_wav_bytes();
        let payload = sample_payload();
        let embedded = embed_watermark_wav_bytes(&input, &payload).unwrap();
        let extracted = extract_watermark_wav_bytes(&embedded).unwrap();

        assert_eq!(extracted.magic, payload.magic);
        assert_eq!(extracted.user_seed, payload.user_seed);
        assert_eq!(extracted.device_id, payload.device_id);
        assert_eq!(extracted.file_hash, payload.file_hash);
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
        let mut samples: Vec<f32> = (0..8_192)
            .map(|i| {
                let t = i as f32 / 44_100.0;
                (t * 440.0 * std::f32::consts::TAU).sin() * 0.2
            })
            .collect();
        let payload = sample_payload();

        embed_watermark_samples(&mut samples, &payload).unwrap();
        let extracted = extract_watermark_samples(&samples).unwrap();

        assert_eq!(extracted.magic, payload.magic);
        assert_eq!(extracted.user_seed, payload.user_seed);
        assert_eq!(extracted.device_id, payload.device_id);
        assert_eq!(extracted.file_hash, payload.file_hash);
    }

    #[test]
    fn samples_reject_existing_watermark_by_default() {
        let mut samples: Vec<f32> = (0..8_192)
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
        let mut samples: Vec<f32> = (0..16_384)
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
    fn clipped_samples_still_detect_audio_slice_markers() {
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
        let end = FRAME_SIZE * 288;
        let clipped = samples[start..end].to_vec();
        let marker_count = detect_audio_marker_count(&clipped, CANONICAL_SAMPLE_RATE).unwrap();

        assert!(marker_count >= 1);
    }

    #[test]
    fn clipped_samples_recover_payload_from_audio_recovery_packet() {
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
        let end = FRAME_SIZE * 288;
        let clipped = samples[start..end].to_vec();
        let extracted =
            extract_watermark_samples_recovery(&clipped, CANONICAL_SAMPLE_RATE).unwrap();

        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
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
}

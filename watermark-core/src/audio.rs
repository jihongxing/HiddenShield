use realfft::num_complex::Complex;
use realfft::RealFftPlanner;
use std::io::Cursor;

use crate::error::WatermarkError;
use crate::payload::{bits_to_bytes, bytes_to_bits, decode_payload, encode_payload, WatermarkPayload};

const FRAME_SIZE: usize = 4096;
const QIM_DELTA: f32 = 0.02;
const SILENCE_THRESHOLD: f32 = 0.001;
const PAYLOAD_BITS: usize = 32 * 8;
const BIN_LO: usize = 186;
const BIN_HI: usize = 743;

pub fn embed_watermark(samples: &mut [f32], payload: &WatermarkPayload) -> Result<(), WatermarkError> {
    embed_watermark_samples(samples, payload)
}

pub fn extract_watermark(samples: &[f32]) -> Result<WatermarkPayload, WatermarkError> {
    extract_watermark_samples(samples)
}

pub fn embed_watermark_wav_bytes(
    input_wav: &[u8],
    payload: &WatermarkPayload,
) -> Result<Vec<u8>, WatermarkError> {
    let mut reader = hound::WavReader::new(Cursor::new(input_wav))
        .map_err(|e| WatermarkError::EmbedFailed(format!("open WAV: {e}")))?;
    let spec = reader.spec();
    let mut samples = read_wav_samples(&mut reader)?;
    embed_watermark_samples(&mut samples, payload)?;
    write_wav_samples(&samples, spec)
}

pub fn extract_watermark_wav_bytes(input_wav: &[u8]) -> Result<WatermarkPayload, WatermarkError> {
    let mut reader = hound::WavReader::new(Cursor::new(input_wav))
        .map_err(|e| WatermarkError::ExtractFailed(format!("open WAV: {e}")))?;
    let samples = read_wav_samples(&mut reader)?;
    extract_watermark_samples(&samples)
}

pub fn embed_watermark_samples(
    samples: &mut [f32],
    payload: &WatermarkPayload,
) -> Result<(), WatermarkError> {
    if samples.len() < FRAME_SIZE {
        return Err(WatermarkError::EmbedFailed(
            "audio too short for watermark embedding".into(),
        ));
    }

    let payload_bytes = encode_payload(payload);
    let bits = bytes_to_bits(&payload_bytes);

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);
    let ifft = planner.plan_fft_inverse(FRAME_SIZE);

    let num_frames = samples.len() / FRAME_SIZE;
    let usable_bins = BIN_HI - BIN_LO;

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

        for (i, bin_idx) in (BIN_LO..BIN_HI).enumerate() {
            let bit_idx = (frame_idx * usable_bins + i) % PAYLOAD_BITS;
            let mag = spectrum[bin_idx].norm();
            let phase = spectrum[bin_idx].arg();
            let new_mag = qim_embed(mag, bits[bit_idx]);
            spectrum[bin_idx] = Complex::from_polar(new_mag, phase);
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

pub fn extract_watermark_samples(samples: &[f32]) -> Result<WatermarkPayload, WatermarkError> {
    if samples.len() < FRAME_SIZE {
        return Err(WatermarkError::ExtractFailed(
            "audio too short for watermark extraction".into(),
        ));
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);

    let num_frames = samples.len() / FRAME_SIZE;
    let usable_bins = BIN_HI - BIN_LO;
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

        for (i, bin_idx) in (BIN_LO..BIN_HI).enumerate() {
            let bit_idx = (frame_idx * usable_bins + i) % PAYLOAD_BITS;
            let bit = qim_extract(spectrum[bin_idx].norm());
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

fn write_wav_samples(
    samples: &[f32],
    spec: hound::WavSpec,
) -> Result<Vec<u8>, WatermarkError> {
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

fn qim_embed(mag: f32, bit: bool) -> f32 {
    let half = QIM_DELTA / 2.0;
    let idx = (mag / half).round() as i32;
    let target_odd = if bit { 1 } else { 0 };
    let adjusted = if (idx & 1) == target_odd {
        idx
    } else if mag > (idx as f32) * half {
        idx + 1
    } else {
        idx - 1
    };
    (adjusted as f32 * half).max(0.0)
}

fn qim_extract(mag: f32) -> bool {
    let half = QIM_DELTA / 2.0;
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
        WatermarkPayload::new([0x42; 8], 1_700_000_000, [0xAB; 4], [0xCD; 2], Default::default())
    }

    fn make_wav_bytes() -> Vec<u8> {
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 44_100,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut cursor = Cursor::new(Vec::new());
        let mut writer = hound::WavWriter::new(&mut cursor, spec).unwrap();

        for i in 0..8_192 {
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
}

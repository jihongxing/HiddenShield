use realfft::num_complex::Complex;
use realfft::RealFftPlanner;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use super::error::PipelineError;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Magic number: "HD5H" = 0x48443548
const MAGIC: [u8; 4] = [0x48, 0x44, 0x35, 0x48];

/// Secret key for HMAC-SHA256 watermark authentication.
/// Obfuscated at compile time to prevent trivial extraction via `strings` or hex editors.
/// In production, derive from hardware-bound secret or user private key.
fn hmac_secret() -> Vec<u8> {
    obfstr::obfbytes!(b"HS_WM_SECRET_v1_2026_do_not_share").to_vec()
}

/// FFT frame size in samples.
const FRAME_SIZE: usize = 4096;

/// Sample rate assumed for bin index calculations.
#[allow(dead_code)]
const SAMPLE_RATE: f32 = 44100.0;

/// QIM quantization step size. Controls embedding strength.
/// Larger = more robust but more audible. 0.02 is imperceptible.
const QIM_DELTA: f32 = 0.02;

/// RMS energy threshold below which a frame is considered silent.
const SILENCE_THRESHOLD: f32 = 0.001;

/// Payload size in bytes (256 bits).
const PAYLOAD_BYTES: usize = 32;

/// Payload size in bits.
const PAYLOAD_BITS: usize = PAYLOAD_BYTES * 8;

/// Low frequency bin index (~2kHz at 44.1kHz, frame 4096).
/// bin = freq * frame_size / sample_rate = 2000 * 4096 / 44100 ≈ 186
const BIN_LO: usize = 186;

/// High frequency bin index (~8kHz at 44.1kHz, frame 4096).
/// bin = 8000 * 4096 / 44100 ≈ 743
const BIN_HI: usize = 743;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// Fixed 256-bit (32-byte) watermark payload — Asset-Bound Identity Model.
///
/// Layout:
/// ```text
/// [0..4]   Magic "HD5H"
/// [4..12]  User Seed (creator identity, 8 bytes)
/// [12..20] Timestamp (nanosecond Unix epoch, 8 bytes)
/// [20..24] Device ID (hardware fingerprint, 4 bytes)
/// [24..28] File Hash prefix (original file SHA-256 first 4 bytes)
/// [28..32] HMAC Auth Tag (anti-tamper signature, 4 bytes)
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatermarkPayload {
    /// Magic number 0x48443548 ("HD5H").
    pub magic: [u8; 4],
    /// Creator identity seed (SHA-256 prefix of user-provided string).
    pub user_seed: [u8; 8],
    /// Nanosecond Unix timestamp.
    pub timestamp: u64,
    /// Device hardware fingerprint (SHA-256 prefix).
    pub device_id: [u8; 4],
    /// Original file SHA-256 prefix (asset binding, anti-transplant).
    pub file_hash: [u8; 4],
    /// HMAC-SHA256 truncated to 4 bytes (anti-tamper).
    pub auth_tag: [u8; 4],
}

type HmacSha256 = Hmac<Sha256>;

/// Compute HMAC-SHA256 over the first 28 bytes and return truncated 4-byte tag.
fn compute_auth_tag(data: &[u8; 28]) -> [u8; 4] {
    let secret = hmac_secret();
    let mut mac = HmacSha256::new_from_slice(&secret).expect("HMAC can take key of any size");
    mac.update(data);
    let result = mac.finalize().into_bytes();
    let mut tag = [0u8; 4];
    tag.copy_from_slice(&result[..4]);
    tag
}

impl WatermarkPayload {
    /// Create a new payload with the fusion identity model.
    pub fn new(user_seed: [u8; 8], timestamp: u64, device_id: [u8; 4], file_hash: [u8; 4]) -> Self {
        let mut buf = [0u8; 28];
        buf[0..4].copy_from_slice(&MAGIC);
        buf[4..12].copy_from_slice(&user_seed);
        buf[12..20].copy_from_slice(&timestamp.to_be_bytes());
        buf[20..24].copy_from_slice(&device_id);
        buf[24..28].copy_from_slice(&file_hash);
        let auth_tag = compute_auth_tag(&buf);
        Self {
            magic: MAGIC,
            user_seed,
            timestamp,
            device_id,
            file_hash,
            auth_tag,
        }
    }

    /// Watermark UID string: `HS-XXXX-XXXX-XXXX` derived from user_seed + device_id.
    /// Globally unique per creator + device combination.
    pub fn watermark_uid(&self) -> String {
        format!(
            "HS-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}",
            self.user_seed[0],
            self.user_seed[1],
            self.user_seed[2],
            self.user_seed[3],
            self.device_id[0],
            self.device_id[1],
        )
    }
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

/// Serialize payload to 32 bytes (big-endian).
pub fn encode_payload(payload: &WatermarkPayload) -> [u8; PAYLOAD_BYTES] {
    let mut buf = [0u8; PAYLOAD_BYTES];
    buf[0..4].copy_from_slice(&payload.magic);
    buf[4..12].copy_from_slice(&payload.user_seed);
    buf[12..20].copy_from_slice(&payload.timestamp.to_be_bytes());
    buf[20..24].copy_from_slice(&payload.device_id);
    buf[24..28].copy_from_slice(&payload.file_hash);
    // Compute HMAC auth tag over first 28 bytes
    let mut data = [0u8; 28];
    data.copy_from_slice(&buf[..28]);
    let tag = compute_auth_tag(&data);
    buf[28..32].copy_from_slice(&tag);
    buf
}

/// Deserialize 32 bytes into a WatermarkPayload, verifying magic and HMAC auth tag.
pub fn decode_payload(bytes: &[u8; PAYLOAD_BYTES]) -> Result<WatermarkPayload, PipelineError> {
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    if magic != MAGIC {
        return Err(PipelineError::WatermarkExtractFailed(format!(
            "magic mismatch: expected {:02X?}, got {:02X?}",
            MAGIC, magic
        )));
    }

    let mut user_seed = [0u8; 8];
    user_seed.copy_from_slice(&bytes[4..12]);
    let timestamp = u64::from_be_bytes(bytes[12..20].try_into().unwrap());
    let mut device_id = [0u8; 4];
    device_id.copy_from_slice(&bytes[20..24]);
    let mut file_hash = [0u8; 4];
    file_hash.copy_from_slice(&bytes[24..28]);
    let mut stored_tag = [0u8; 4];
    stored_tag.copy_from_slice(&bytes[28..32]);

    // Verify HMAC auth tag
    let mut data = [0u8; 28];
    data.copy_from_slice(&bytes[..28]);
    let computed_tag = compute_auth_tag(&data);
    if stored_tag != computed_tag {
        return Err(PipelineError::WatermarkExtractFailed(format!(
            "HMAC auth tag mismatch: stored {:02X?}, computed {:02X?}",
            stored_tag, computed_tag
        )));
    }

    Ok(WatermarkPayload {
        magic,
        user_seed,
        timestamp,
        device_id,
        file_hash,
        auth_tag: stored_tag,
    })
}

/// Raw encode (used internally).
#[allow(dead_code)]
fn encode_payload_raw(payload: &WatermarkPayload) -> [u8; PAYLOAD_BYTES] {
    let mut buf = [0u8; PAYLOAD_BYTES];
    buf[0..4].copy_from_slice(&payload.magic);
    buf[4..12].copy_from_slice(&payload.user_seed);
    buf[12..20].copy_from_slice(&payload.timestamp.to_be_bytes());
    buf[20..24].copy_from_slice(&payload.device_id);
    buf[24..28].copy_from_slice(&payload.file_hash);
    buf[28..32].copy_from_slice(&payload.auth_tag);
    buf
}

// ---------------------------------------------------------------------------
// Embedding (QIM — Quantization Index Modulation)
// ---------------------------------------------------------------------------

/// Embed watermark into PCM f32 audio samples (mono or interleaved, treated as mono).
///
/// The signal is processed in non-overlapping frames of `FRAME_SIZE` samples.
/// For each non-silent frame, payload bits are encoded into FFT bin magnitudes
/// in the 2kHz–8kHz range using QIM: bit 1 → quantize to odd grid, bit 0 → even grid.
pub fn embed_watermark(
    samples: &mut [f32],
    payload: &WatermarkPayload,
) -> Result<(), PipelineError> {
    if samples.len() < FRAME_SIZE {
        return Err(PipelineError::WatermarkEmbedFailed(
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

        // Skip silent frames
        if rms_energy(frame) < SILENCE_THRESHOLD {
            continue;
        }

        // Forward FFT
        let mut input = frame.to_vec();
        let mut spectrum = fft.make_output_vec();
        fft.process(&mut input, &mut spectrum)
            .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("FFT failed: {e}")))?;

        // Embed bits via QIM
        for (i, bin_idx) in (BIN_LO..BIN_HI).enumerate() {
            let bit_idx = (frame_idx * usable_bins + i) % PAYLOAD_BITS;
            let mag = spectrum[bin_idx].norm();
            let phase = spectrum[bin_idx].arg();
            let new_mag = qim_embed(mag, bits[bit_idx]);
            spectrum[bin_idx] = Complex::from_polar(new_mag, phase);
        }

        // Inverse FFT
        let mut output = ifft.make_output_vec();
        ifft.process(&mut spectrum, &mut output)
            .map_err(|e| PipelineError::WatermarkEmbedFailed(format!("IFFT failed: {e}")))?;

        // Normalize (realfft inverse scales by FRAME_SIZE)
        let scale = 1.0 / FRAME_SIZE as f32;
        for (j, sample) in output.iter().enumerate().take(FRAME_SIZE) {
            frame[j] = sample * scale;
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Extraction (QIM decode + majority voting)
// ---------------------------------------------------------------------------

/// Extract watermark from PCM f32 audio samples.
///
/// Uses QIM decoding on each bin and majority voting across frames.
pub fn extract_watermark(samples: &[f32]) -> Result<WatermarkPayload, PipelineError> {
    if samples.len() < FRAME_SIZE {
        return Err(PipelineError::WatermarkExtractFailed(
            "audio too short for watermark extraction".into(),
        ));
    }

    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(FRAME_SIZE);

    let num_frames = samples.len() / FRAME_SIZE;
    let usable_bins = BIN_HI - BIN_LO;

    // Accumulators for majority voting: positive = bit 1, negative = bit 0
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
            .map_err(|e| PipelineError::WatermarkExtractFailed(format!("FFT failed: {e}")))?;

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

    // Reconstruct bits from votes
    let bits: Vec<bool> = votes.iter().map(|&v| v > 0).collect();
    let payload_bytes = bits_to_bytes(&bits);

    let mut arr = [0u8; PAYLOAD_BYTES];
    arr.copy_from_slice(&payload_bytes);
    decode_payload(&arr)
}

// ---------------------------------------------------------------------------
// QIM helpers
// ---------------------------------------------------------------------------

/// Quantize magnitude to encode a bit.
/// bit=0 → nearest even multiple of delta/2 (i.e. multiple of delta)
/// bit=1 → nearest odd multiple of delta/2
fn qim_embed(mag: f32, bit: bool) -> f32 {
    let half = QIM_DELTA / 2.0;
    // Quantize to nearest multiple of half
    let idx = (mag / half).round() as i32;
    let target_odd = if bit { 1 } else { 0 };
    // Adjust index to match desired parity
    let adjusted = if (idx & 1) == target_odd {
        idx
    } else {
        // Pick the closer neighbor with correct parity
        if mag > (idx as f32) * half {
            idx + 1
        } else {
            idx - 1
        }
    };
    (adjusted as f32 * half).max(0.0)
}

/// Extract a bit from a magnitude by checking QIM quantization parity.
fn qim_extract(mag: f32) -> bool {
    let half = QIM_DELTA / 2.0;
    let idx = (mag / half).round() as i32;
    // odd index → bit 1, even index → bit 0
    (idx & 1) == 1
}

// ---------------------------------------------------------------------------
// Bit helpers
// ---------------------------------------------------------------------------

fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for &byte in bytes {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1 == 1);
        }
    }
    bits
}

fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
    bits.chunks(8)
        .map(|chunk| {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit {
                    byte |= 1 << (7 - i);
                }
            }
            byte
        })
        .collect()
}

fn rms_energy(frame: &[f32]) -> f32 {
    if frame.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = frame.iter().map(|&s| s * s).sum();
    (sum_sq / frame.len() as f32).sqrt()
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_roundtrip() {
        let payload = WatermarkPayload::new([0xAB; 8], 1700000000, [0xCD; 4], [0xEF; 4]);
        let encoded = encode_payload(&payload);
        let decoded = decode_payload(&encoded).unwrap();
        assert_eq!(decoded.magic, MAGIC);
        assert_eq!(decoded.user_seed, payload.user_seed);
        assert_eq!(decoded.timestamp, payload.timestamp);
        assert_eq!(decoded.device_id, payload.device_id);
        assert_eq!(decoded.file_hash, payload.file_hash);
        assert_eq!(decoded.auth_tag, payload.auth_tag);
    }

    #[test]
    fn test_decode_bad_magic() {
        let mut encoded = encode_payload(&WatermarkPayload::new([0; 8], 0, [0; 4], [0; 4]));
        encoded[0] = 0xFF;
        assert!(decode_payload(&encoded).is_err());
    }

    #[test]
    fn test_decode_bad_crc() {
        let mut encoded = encode_payload(&WatermarkPayload::new([0; 8], 0, [0; 4], [0; 4]));
        encoded[31] ^= 0xFF;
        assert!(decode_payload(&encoded).is_err());
    }

    #[test]
    fn test_watermark_uid_format() {
        let payload = WatermarkPayload::new(
            [0x01, 0x23, 0x45, 0x67, 0x89, 0xAB, 0xCD, 0xEF],
            0,
            [0xDE, 0xAD, 0xBE, 0xEF],
            [0; 4],
        );
        assert_eq!(payload.watermark_uid(), "HS-0123-4567-DEAD");
    }

    #[test]
    fn test_bytes_bits_roundtrip() {
        let original = vec![0xDE, 0xAD, 0xBE, 0xEF];
        let bits = bytes_to_bits(&original);
        let recovered = bits_to_bytes(&bits);
        assert_eq!(original, recovered);
    }

    #[test]
    fn test_rms_energy_silent() {
        let frame = vec![0.0f32; FRAME_SIZE];
        assert!(rms_energy(&frame) < SILENCE_THRESHOLD);
    }

    #[test]
    fn test_embed_too_short() {
        let mut samples = vec![0.0f32; 100];
        let payload = WatermarkPayload::new([0; 8], 0, [0; 4], [0; 4]);
        assert!(embed_watermark(&mut samples, &payload).is_err());
    }

    #[test]
    fn test_extract_too_short() {
        let samples = vec![0.0f32; 100];
        assert!(extract_watermark(&samples).is_err());
    }

    #[test]
    fn test_embed_extract_roundtrip_sine() {
        // Generate a 440Hz sine wave, long enough for multiple frames
        let num_samples = FRAME_SIZE * 20;
        let mut samples: Vec<f32> = (0..num_samples)
            .map(|i| {
                let t = i as f32 / SAMPLE_RATE;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
            })
            .collect();

        let payload = WatermarkPayload::new([0x42; 8], 1700000000, [0xAB; 4], [0xCD; 4]);
        embed_watermark(&mut samples, &payload).unwrap();
        let extracted = extract_watermark(&samples).unwrap();

        assert_eq!(extracted.magic, payload.magic);
        assert_eq!(extracted.user_seed, payload.user_seed);
        assert_eq!(extracted.timestamp, payload.timestamp);
        assert_eq!(extracted.device_id, payload.device_id);
        assert_eq!(extracted.file_hash, payload.file_hash);
    }
}

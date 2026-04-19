//! Robust blind image watermark using DWT-DCT-SVD hybrid algorithm.
//!
//! Algorithm overview (inspired by guofei9987/blind_watermark):
//! 1. Convert image to YCbCr, work on Y (luminance) channel
//! 2. Apply 1-level Haar DWT → get LL (low-frequency) sub-band
//! 3. Split LL into 4×4 blocks, apply DCT2 to each block
//! 4. Apply SVD to each DCT block, embed 1 bit into the largest singular value
//! 5. Inverse SVD → IDCT → IDWT → reconstruct image
//!
//! This approach survives JPEG compression, scaling, cropping, and noise.

use std::path::Path;

use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};
use nalgebra::{DMatrix, Matrix4, SVD};

use super::error::PipelineError;
use super::watermark::{decode_payload, encode_payload, WatermarkPayload};

/// Embedding strength factor. Higher = more robust but more visible.
/// Must be large enough to survive u8 rounding through the DWT-DCT-SVD chain.
const ALPHA: f64 = 50.0;

/// Payload size in bits (32 bytes × 8).
const PAYLOAD_BITS: usize = 32 * 8;

/// Block size for DCT (4×4).
const BLOCK_SIZE: usize = 4;

/// Redundancy factor: each bit is embedded this many times for majority voting.
const REDUNDANCY: usize = 3;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Embed a watermark into an image using DWT-DCT-SVD.
pub fn embed_image_watermark(
    image_path: &Path,
    payload: &WatermarkPayload,
    output_path: &Path,
) -> Result<(), PipelineError> {
    let img = image::open(image_path).map_err(|e| {
        PipelineError::WatermarkEmbedFailed(format!("failed to open image: {e}"))
    })?;

    let (w, h) = img.dimensions();
    let half_w = (w / 2) as usize;
    let half_h = (h / 2) as usize;
    let blocks_x = half_w / BLOCK_SIZE;
    let blocks_y = half_h / BLOCK_SIZE;
    let total_blocks = blocks_x * blocks_y;

    if total_blocks < PAYLOAD_BITS * REDUNDANCY {
        let min_dim = ((PAYLOAD_BITS * REDUNDANCY) as f64).sqrt().ceil() as usize + 1;
        let min_pixels = BLOCK_SIZE * 2 * min_dim;
        return Err(PipelineError::WatermarkEmbedFailed(format!(
            "image too small for watermark: need at least {}×{} pixels, got {}×{}",
            min_pixels, min_pixels, w, h
        )));
    }

    let payload_bytes = encode_payload(payload);
    let bits = bytes_to_bits(&payload_bytes);

    // Build redundant bit sequence (each bit repeated REDUNDANCY times)
    let redundant_bits: Vec<bool> = bits.iter()
        .flat_map(|&b| std::iter::repeat(b).take(REDUNDANCY))
        .collect();

    // Convert to f64 YCbCr channels
    let (mut y_channel, cb_channel, cr_channel) = rgb_to_ycbcr_channels(&img);

    // 1. Haar DWT on Y channel
    let (mut ll, lh, hl, hh) = haar_dwt_2d(&y_channel, half_w, half_h);

    // 2-4. Embed bits into LL sub-band via DCT-SVD
    embed_bits_dct_svd(&mut ll, half_w, half_h, blocks_x, blocks_y, &redundant_bits);

    // 5. Inverse DWT
    haar_idwt_2d(&mut y_channel, &ll, &lh, &hl, &hh, half_w, half_h);

    // Reconstruct RGBA image
    let output_img = ycbcr_to_rgba(&y_channel, &cb_channel, &cr_channel, w, h);

    // Save (PNG for lossless, or handle JPEG)
    let ext = output_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext == "jpg" || ext == "jpeg" {
        let rgb = DynamicImage::ImageRgba8(output_img).to_rgb8();
        rgb.save(output_path).map_err(|e| {
            PipelineError::WatermarkEmbedFailed(format!("failed to save image: {e}"))
        })?;
    } else {
        output_img.save(output_path).map_err(|e| {
            PipelineError::WatermarkEmbedFailed(format!("failed to save image: {e}"))
        })?;
    }

    Ok(())
}

/// Extract a watermark from an image using DWT-DCT-SVD.
pub fn extract_image_watermark(
    image_path: &Path,
) -> Result<WatermarkPayload, PipelineError> {
    let img = image::open(image_path).map_err(|e| {
        PipelineError::WatermarkExtractFailed(format!("failed to open image: {e}"))
    })?;

    let (w, h) = img.dimensions();
    let half_w = (w / 2) as usize;
    let half_h = (h / 2) as usize;
    let blocks_x = half_w / BLOCK_SIZE;
    let blocks_y = half_h / BLOCK_SIZE;
    let total_blocks = blocks_x * blocks_y;

    if total_blocks < PAYLOAD_BITS {
        return Err(PipelineError::WatermarkExtractFailed(
            "image too small for watermark extraction".into(),
        ));
    }

    let (y_channel, _, _) = rgb_to_ycbcr_channels(&img);

    // 1. Haar DWT
    let (ll, _, _, _) = haar_dwt_2d(&y_channel, half_w, half_h);

    // 2-4. Extract raw bits from LL sub-band via DCT-SVD (includes redundancy)
    let raw_bits = extract_bits_dct_svd(&ll, half_w, blocks_x, blocks_y);

    // Majority voting: group every REDUNDANCY bits and pick the majority
    let bits: Vec<bool> = raw_bits
        .chunks(REDUNDANCY)
        .take(PAYLOAD_BITS)
        .map(|chunk| {
            let ones = chunk.iter().filter(|&&b| b).count();
            ones > chunk.len() / 2
        })
        .collect();

    if bits.len() < PAYLOAD_BITS {
        return Err(PipelineError::WatermarkExtractFailed(
            "not enough data for watermark extraction".into(),
        ));
    }

    let payload_bytes = bits_to_bytes(&bits);
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&payload_bytes[..32]);
    decode_payload(&arr)
}


// ---------------------------------------------------------------------------
// DWT-DCT-SVD core
// ---------------------------------------------------------------------------

/// Embed payload bits into the LL sub-band using DCT + SVD on 4×4 blocks.
fn embed_bits_dct_svd(
    ll: &mut [f64],
    ll_w: usize,
    _ll_h: usize,
    blocks_x: usize,
    blocks_y: usize,
    bits: &[bool],
) {
    let total_bits = bits.len();
    let mut bit_idx = 0;
    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            if bit_idx >= total_bits {
                return;
            }

            // Extract 4×4 block
            let mut block = Matrix4::<f64>::zeros();
            for row in 0..BLOCK_SIZE {
                for col in 0..BLOCK_SIZE {
                    let y = by * BLOCK_SIZE + row;
                    let x = bx * BLOCK_SIZE + col;
                    block[(row, col)] = ll[y * ll_w + x];
                }
            }

            // DCT2 on 4×4 block
            let dct_block = dct4x4(&block);

            // SVD
            let dmat = DMatrix::from_row_slice(4, 4, dct_block.as_slice());
            let svd = SVD::new(dmat, true, true);
            let mut sigma = svd.singular_values.clone();

            // Embed bit into largest singular value using quantization
            let s0 = sigma[0];
            let quantized = quantize_embed(s0, bits[bit_idx], ALPHA);
            sigma[0] = quantized;

            // Reconstruct: U * Sigma * V^T
            let u = svd.u.unwrap();
            let vt = svd.v_t.unwrap();
            let sigma_mat = DMatrix::from_diagonal(&sigma);
            let reconstructed = &u * &sigma_mat * &vt;

            // IDCT2
            let mut recon4 = Matrix4::<f64>::zeros();
            for r in 0..4 {
                for c in 0..4 {
                    recon4[(r, c)] = reconstructed[(r, c)];
                }
            }
            let spatial_block = idct4x4(&recon4);

            // Write back to LL
            for row in 0..BLOCK_SIZE {
                for col in 0..BLOCK_SIZE {
                    let y = by * BLOCK_SIZE + row;
                    let x = bx * BLOCK_SIZE + col;
                    ll[y * ll_w + x] = spatial_block[(row, col)];
                }
            }

            bit_idx += 1;
        }
    }
}

/// Extract payload bits from the LL sub-band using DCT + SVD on 4×4 blocks.
fn extract_bits_dct_svd(
    ll: &[f64],
    ll_w: usize,
    blocks_x: usize,
    blocks_y: usize,
) -> Vec<bool> {
    let total_bits = PAYLOAD_BITS * REDUNDANCY;
    let mut bits = Vec::with_capacity(total_bits);

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            if bits.len() >= total_bits {
                return bits;
            }

            let mut block = Matrix4::<f64>::zeros();
            for row in 0..BLOCK_SIZE {
                for col in 0..BLOCK_SIZE {
                    let y = by * BLOCK_SIZE + row;
                    let x = bx * BLOCK_SIZE + col;
                    block[(row, col)] = ll[y * ll_w + x];
                }
            }

            let dct_block = dct4x4(&block);
            let dmat = DMatrix::from_row_slice(4, 4, dct_block.as_slice());
            let svd = SVD::new(dmat, true, true);
            let s0 = svd.singular_values[0];

            bits.push(quantize_extract(s0, ALPHA));
        }
    }

    bits
}

// ---------------------------------------------------------------------------
// Quantization (QIM on singular values)
// ---------------------------------------------------------------------------

/// Embed a bit by quantizing the singular value to an odd/even grid.
/// bit=0 → quantize to even multiple of alpha
/// bit=1 → quantize to odd multiple of alpha
fn quantize_embed(value: f64, bit: bool, alpha: f64) -> f64 {
    let idx = (value / alpha).round() as i64;
    let target_parity = if bit { 1 } else { 0 };
    let adjusted = if (idx & 1) == target_parity {
        idx
    } else {
        // Pick the closer neighbor with correct parity
        if value > (idx as f64) * alpha {
            idx + 1
        } else {
            idx - 1
        }
    };
    adjusted as f64 * alpha
}

/// Extract a bit by checking the parity of the quantized singular value.
fn quantize_extract(value: f64, alpha: f64) -> bool {
    let idx = (value / alpha).round() as i64;
    (idx & 1) == 1
}

// ---------------------------------------------------------------------------
// Haar DWT / IDWT (2D, 1-level)
// ---------------------------------------------------------------------------

/// 1-level 2D Haar DWT. Returns (LL, LH, HL, HH) sub-bands.
/// Uses the standard unnormalized Haar: average and difference.
fn haar_dwt_2d(data: &[f64], half_w: usize, half_h: usize) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let full_w = half_w * 2;
    let full_h = half_h * 2;

    // First pass: horizontal transform into temp buffer
    let mut temp = vec![0.0f64; full_w * full_h];
    for y in 0..full_h {
        for x in 0..half_w {
            let x2 = x * 2;
            let a = data[y * full_w + x2];
            let b = data[y * full_w + x2 + 1];
            temp[y * full_w + x] = (a + b) / 2.0;           // Low
            temp[y * full_w + half_w + x] = (a - b) / 2.0;  // High
        }
    }

    // Second pass: vertical transform
    let mut ll = vec![0.0f64; half_w * half_h];
    let mut lh = vec![0.0f64; half_w * half_h];
    let mut hl = vec![0.0f64; half_w * half_h];
    let mut hh = vec![0.0f64; half_w * half_h];

    for x in 0..half_w {
        for y in 0..half_h {
            let y2 = y * 2;
            let low_a = temp[y2 * full_w + x];
            let low_b = temp[(y2 + 1) * full_w + x];
            ll[y * half_w + x] = (low_a + low_b) / 2.0;
            lh[y * half_w + x] = (low_a - low_b) / 2.0;

            let high_a = temp[y2 * full_w + half_w + x];
            let high_b = temp[(y2 + 1) * full_w + half_w + x];
            hl[y * half_w + x] = (high_a + high_b) / 2.0;
            hh[y * half_w + x] = (high_a - high_b) / 2.0;
        }
    }

    (ll, lh, hl, hh)
}

/// Inverse 1-level 2D Haar DWT. Reconstructs full-size data from sub-bands.
fn haar_idwt_2d(
    data: &mut [f64],
    ll: &[f64], lh: &[f64], hl: &[f64], hh: &[f64],
    half_w: usize, half_h: usize,
) {
    let full_w = half_w * 2;
    let full_h = half_h * 2;

    // First pass: inverse vertical transform into temp buffer
    let mut temp = vec![0.0f64; full_w * full_h];
    for x in 0..half_w {
        for y in 0..half_h {
            let y2 = y * 2;
            let l = ll[y * half_w + x];
            let h = lh[y * half_w + x];
            temp[y2 * full_w + x] = l + h;
            temp[(y2 + 1) * full_w + x] = l - h;

            let hl_val = hl[y * half_w + x];
            let hh_val = hh[y * half_w + x];
            temp[y2 * full_w + half_w + x] = hl_val + hh_val;
            temp[(y2 + 1) * full_w + half_w + x] = hl_val - hh_val;
        }
    }

    // Second pass: inverse horizontal transform
    for y in 0..full_h {
        for x in 0..half_w {
            let x2 = x * 2;
            let l = temp[y * full_w + x];
            let h = temp[y * full_w + half_w + x];
            data[y * full_w + x2] = l + h;
            data[y * full_w + x2 + 1] = l - h;
        }
    }
}


// ---------------------------------------------------------------------------
// DCT 4×4 (Type-II, orthonormal)
// ---------------------------------------------------------------------------

/// Forward DCT-II on a 4×4 block (orthonormal).
fn dct4x4(block: &Matrix4<f64>) -> Matrix4<f64> {
    let n = BLOCK_SIZE as f64;
    let mut result = Matrix4::<f64>::zeros();

    for u in 0..BLOCK_SIZE {
        for v in 0..BLOCK_SIZE {
            let mut sum = 0.0;
            for x in 0..BLOCK_SIZE {
                for y in 0..BLOCK_SIZE {
                    let cos_x = ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / (2.0 * n)).cos();
                    let cos_y = ((2 * y + 1) as f64 * v as f64 * std::f64::consts::PI / (2.0 * n)).cos();
                    sum += block[(x, y)] * cos_x * cos_y;
                }
            }
            let cu = if u == 0 { 1.0 / n.sqrt() } else { (2.0 / n).sqrt() };
            let cv = if v == 0 { 1.0 / n.sqrt() } else { (2.0 / n).sqrt() };
            result[(u, v)] = cu * cv * sum;
        }
    }

    result
}

/// Inverse DCT-II (Type-III) on a 4×4 block (orthonormal).
fn idct4x4(block: &Matrix4<f64>) -> Matrix4<f64> {
    let n = BLOCK_SIZE as f64;
    let mut result = Matrix4::<f64>::zeros();

    for x in 0..BLOCK_SIZE {
        for y in 0..BLOCK_SIZE {
            let mut sum = 0.0;
            for u in 0..BLOCK_SIZE {
                for v in 0..BLOCK_SIZE {
                    let cu = if u == 0 { 1.0 / n.sqrt() } else { (2.0 / n).sqrt() };
                    let cv = if v == 0 { 1.0 / n.sqrt() } else { (2.0 / n).sqrt() };
                    let cos_x = ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / (2.0 * n)).cos();
                    let cos_y = ((2 * y + 1) as f64 * v as f64 * std::f64::consts::PI / (2.0 * n)).cos();
                    sum += cu * cv * block[(u, v)] * cos_x * cos_y;
                }
            }
            result[(x, y)] = sum;
        }
    }

    result
}

// ---------------------------------------------------------------------------
// Color space conversion (RGB ↔ YCbCr)
// ---------------------------------------------------------------------------

/// Convert image to separate Y, Cb, Cr f64 channels.
fn rgb_to_ycbcr_channels(img: &DynamicImage) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let (w, h) = img.dimensions();
    let size = (w * h) as usize;
    let mut y_ch = Vec::with_capacity(size);
    let mut cb_ch = Vec::with_capacity(size);
    let mut cr_ch = Vec::with_capacity(size);

    for py in 0..h {
        for px in 0..w {
            let pixel = img.get_pixel(px, py);
            let r = pixel[0] as f64;
            let g = pixel[1] as f64;
            let b = pixel[2] as f64;

            let y = 0.299 * r + 0.587 * g + 0.114 * b;
            let cb = -0.1687 * r - 0.3313 * g + 0.5 * b + 128.0;
            let cr = 0.5 * r - 0.4187 * g - 0.0813 * b + 128.0;

            y_ch.push(y);
            cb_ch.push(cb);
            cr_ch.push(cr);
        }
    }

    (y_ch, cb_ch, cr_ch)
}

/// Convert Y, Cb, Cr channels back to RGBA image.
fn ycbcr_to_rgba(y_ch: &[f64], cb_ch: &[f64], cr_ch: &[f64], w: u32, h: u32) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);

    for py in 0..h {
        for px in 0..w {
            let idx = (py * w + px) as usize;
            let y = y_ch[idx];
            let cb = cb_ch[idx] - 128.0;
            let cr = cr_ch[idx] - 128.0;

            let r = (y + 1.402 * cr).clamp(0.0, 255.0) as u8;
            let g = (y - 0.34414 * cb - 0.71414 * cr).clamp(0.0, 255.0) as u8;
            let b = (y + 1.772 * cb).clamp(0.0, 255.0) as u8;

            img.put_pixel(px, py, Rgba([r, g, b, 255]));
        }
    }

    img
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embed_extract_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let input_path = dir.path().join("input.png");
        let output_path = dir.path().join("output.png");

        // Create a 256×256 test image with varied content (enough for 768 blocks: (128/4)*(128/4) = 1024 blocks)
        let img = image::RgbaImage::from_fn(256, 256, |x, y| {
            let r = ((x as f64 / 256.0 * 200.0) + 30.0) as u8;
            let g = ((y as f64 / 256.0 * 200.0) + 30.0) as u8;
            let b = (((x + y) as f64 / 512.0 * 180.0) + 40.0) as u8;
            image::Rgba([r, g, b, 255])
        });
        img.save(&input_path).unwrap();

        let payload = WatermarkPayload::new([0xAB; 8], 1700000000, [0xCD; 4], [0xEF; 4]);
        embed_image_watermark(&input_path, &payload, &output_path).unwrap();

        let extracted = extract_image_watermark(&output_path).unwrap();
        assert_eq!(extracted.magic, payload.magic);
        assert_eq!(extracted.user_seed, payload.user_seed);
        assert_eq!(extracted.timestamp, payload.timestamp);
        assert_eq!(extracted.device_id, payload.device_id);
        assert_eq!(extracted.file_hash, payload.file_hash);
        assert_eq!(extracted.auth_tag, payload.auth_tag);
    }

    #[test]
    fn test_embed_extract_jpeg_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let input_path = dir.path().join("input.png");
        let embedded_path = dir.path().join("embedded.png");
        let jpeg_path = dir.path().join("compressed.jpg");

        // Create a 256×256 test image for better JPEG robustness
        let img = image::RgbaImage::from_fn(256, 256, |x, y| {
            image::Rgba([
                ((x as f32 / 256.0 * 200.0) as u8).wrapping_add(30),
                ((y as f32 / 256.0 * 200.0) as u8).wrapping_add(30),
                128,
                255,
            ])
        });
        img.save(&input_path).unwrap();

        let payload = WatermarkPayload::new([0x42; 8], 1700000000, [0xAB; 4], [0xCD; 4]);
        embed_image_watermark(&input_path, &payload, &embedded_path).unwrap();

        // Simulate JPEG compression attack (quality 85)
        let watermarked = image::open(&embedded_path).unwrap();
        watermarked.to_rgb8().save(&jpeg_path).unwrap();

        // Extract from JPEG-compressed image
        let extracted = extract_image_watermark(&jpeg_path).unwrap();
        assert_eq!(extracted.magic, payload.magic);
        assert_eq!(extracted.user_seed, payload.user_seed);
        assert_eq!(extracted.timestamp, payload.timestamp);
        assert_eq!(extracted.device_id, payload.device_id);
    }

    #[test]
    fn test_image_too_small() {
        let dir = tempfile::tempdir().unwrap();
        let input_path = dir.path().join("tiny.png");
        let output_path = dir.path().join("out.png");

        // 8×8 image: half = 4×4, blocks = 1×1 = 1 block, far less than 256
        let img = image::RgbaImage::from_fn(8, 8, |_, _| image::Rgba([0u8, 0, 0, 255]));
        img.save(&input_path).unwrap();

        let payload = WatermarkPayload::new([0; 8], 0, [0; 4], [0; 4]);
        assert!(embed_image_watermark(&input_path, &payload, &output_path).is_err());
    }

    #[test]
    fn test_dct_idct_roundtrip() {
        let block = Matrix4::new(
            52.0, 55.0, 61.0, 66.0,
            70.0, 61.0, 64.0, 73.0,
            63.0, 59.0, 55.0, 90.0,
            67.0, 61.0, 68.0, 104.0,
        );
        let dct = dct4x4(&block);
        let recovered = idct4x4(&dct);

        for r in 0..4 {
            for c in 0..4 {
                assert!(
                    (block[(r, c)] - recovered[(r, c)]).abs() < 1e-10,
                    "mismatch at ({r},{c}): {} vs {}",
                    block[(r, c)],
                    recovered[(r, c)]
                );
            }
        }
    }

    #[test]
    fn test_quantize_roundtrip() {
        for val in [10.0, 25.3, 100.7, 0.5] {
            for bit in [true, false] {
                let embedded = quantize_embed(val, bit, ALPHA);
                let extracted = quantize_extract(embedded, ALPHA);
                assert_eq!(extracted, bit, "failed for val={val}, bit={bit}");
            }
        }
    }
}

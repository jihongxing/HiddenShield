use std::path::Path;
use std::io::Cursor;

use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use nalgebra::{DMatrix, Matrix4, SVD};

use crate::error::WatermarkError;
use crate::payload::{bits_to_bytes, bytes_to_bits, decode_payload, encode_payload, WatermarkPayload};

const ALPHA: f64 = 50.0;
const PAYLOAD_BITS: usize = 32 * 8;
const BLOCK_SIZE: usize = 4;
const REDUNDANCY: usize = 3;

pub fn embed_image_watermark(
    image_path: &Path,
    payload: &WatermarkPayload,
    output_path: &Path,
) -> Result<(), WatermarkError> {
    let input_bytes = std::fs::read(image_path)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to read image: {e}")))?;
    let output_format = infer_image_format(output_path).unwrap_or(ImageFormat::Png);
    let output_bytes = embed_image_watermark_bytes(&input_bytes, payload, output_format)?;
    std::fs::write(output_path, output_bytes)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to write image: {e}")))?;
    Ok(())
}

pub fn embed_image_watermark_bytes(
    image_bytes: &[u8],
    payload: &WatermarkPayload,
    output_format: ImageFormat,
) -> Result<Vec<u8>, WatermarkError> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to open image: {e}")))?;

    let output_img = embed_image_into_dynamic(&img, payload)?;
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(output_img)
        .write_to(&mut cursor, output_format)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to save image: {e}")))?;
    Ok(cursor.into_inner())
}

pub fn extract_image_watermark(image_path: &Path) -> Result<WatermarkPayload, WatermarkError> {
    let input_bytes = std::fs::read(image_path)
        .map_err(|e| WatermarkError::ExtractFailed(format!("failed to read image: {e}")))?;
    extract_image_watermark_bytes(&input_bytes)
}

pub fn extract_image_watermark_bytes(image_bytes: &[u8]) -> Result<WatermarkPayload, WatermarkError> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| WatermarkError::ExtractFailed(format!("failed to open image: {e}")))?;
    extract_image_from_dynamic(&img)
}

fn embed_image_into_dynamic(
    img: &DynamicImage,
    payload: &WatermarkPayload,
) -> Result<RgbaImage, WatermarkError> {
    let (w, h) = img.dimensions();
    let half_w = (w / 2) as usize;
    let half_h = (h / 2) as usize;
    let blocks_x = half_w / BLOCK_SIZE;
    let blocks_y = half_h / BLOCK_SIZE;
    let total_blocks = blocks_x * blocks_y;

    if total_blocks < PAYLOAD_BITS * REDUNDANCY {
        let min_dim = ((PAYLOAD_BITS * REDUNDANCY) as f64).sqrt().ceil() as usize + 1;
        let min_pixels = BLOCK_SIZE * 2 * min_dim;
        return Err(WatermarkError::EmbedFailed(format!(
            "image too small for watermark: need at least {}×{} pixels, got {}×{}",
            min_pixels, min_pixels, w, h
        )));
    }

    let payload_bytes = encode_payload(payload);
    let bits = bytes_to_bits(&payload_bytes);
    let redundant_bits: Vec<bool> = bits
        .iter()
        .flat_map(|&b| std::iter::repeat(b).take(REDUNDANCY))
        .collect();

    let (mut y_channel, cb_channel, cr_channel) = rgb_to_ycbcr_channels(&img);
    let (mut ll, lh, hl, hh) = haar_dwt_2d(&y_channel, half_w, half_h);
    embed_bits_dct_svd(&mut ll, half_w, half_h, blocks_x, blocks_y, &redundant_bits);
    haar_idwt_2d(&mut y_channel, &ll, &lh, &hl, &hh, half_w, half_h);

    let output_img = ycbcr_to_rgba(&y_channel, &cb_channel, &cr_channel, w, h);
    Ok(output_img)
}

fn extract_image_from_dynamic(img: &DynamicImage) -> Result<WatermarkPayload, WatermarkError> {
    let (w, h) = img.dimensions();
    let half_w = (w / 2) as usize;
    let half_h = (h / 2) as usize;
    let blocks_x = half_w / BLOCK_SIZE;
    let blocks_y = half_h / BLOCK_SIZE;
    let total_blocks = blocks_x * blocks_y;

    if total_blocks < PAYLOAD_BITS {
        return Err(WatermarkError::ExtractFailed(
            "image too small for watermark extraction".into(),
        ));
    }

    let (y_channel, _, _) = rgb_to_ycbcr_channels(&img);
    let (ll, _, _, _) = haar_dwt_2d(&y_channel, half_w, half_h);
    let raw_bits = extract_bits_dct_svd(&ll, half_w, blocks_x, blocks_y);

    let bits: Vec<bool> = raw_bits
        .chunks(REDUNDANCY)
        .take(PAYLOAD_BITS)
        .map(|chunk| {
            let ones = chunk.iter().filter(|&&b| b).count();
            ones > chunk.len() / 2
        })
        .collect();

    if bits.len() < PAYLOAD_BITS {
        return Err(WatermarkError::ExtractFailed(
            "not enough data for watermark extraction".into(),
        ));
    }

    let payload_bytes = bits_to_bytes(&bits);
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&payload_bytes[..32]);
    decode_payload(&arr)
}

fn infer_image_format(path: &Path) -> Option<ImageFormat> {
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    match ext.as_str() {
        "png" => Some(ImageFormat::Png),
        "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
        "webp" => Some(ImageFormat::WebP),
        "bmp" => Some(ImageFormat::Bmp),
        "tif" | "tiff" => Some(ImageFormat::Tiff),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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
        let mut cursor = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(img)
            .write_to(&mut cursor, ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    #[test]
    fn image_bytes_roundtrip() {
        let input = make_png_bytes();
        let payload = sample_payload();
        let embedded = embed_image_watermark_bytes(&input, &payload, ImageFormat::Png).unwrap();
        let extracted = extract_image_watermark_bytes(&embedded).unwrap();

        assert_eq!(extracted.magic, payload.magic);
        assert_eq!(extracted.user_seed, payload.user_seed);
        assert_eq!(extracted.device_id, payload.device_id);
        assert_eq!(extracted.file_hash, payload.file_hash);
    }
}

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
            let mut sigma = svd.singular_values.clone();

            sigma[0] = quantize_embed(sigma[0], bits[bit_idx], ALPHA);

            let u = svd.u.unwrap();
            let vt = svd.v_t.unwrap();
            let sigma_mat = DMatrix::from_diagonal(&sigma);
            let reconstructed = &u * &sigma_mat * &vt;

            let mut recon4 = Matrix4::<f64>::zeros();
            for r in 0..4 {
                for c in 0..4 {
                    recon4[(r, c)] = reconstructed[(r, c)];
                }
            }
            let spatial_block = idct4x4(&recon4);

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

fn extract_bits_dct_svd(ll: &[f64], ll_w: usize, blocks_x: usize, blocks_y: usize) -> Vec<bool> {
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

fn quantize_embed(value: f64, bit: bool, alpha: f64) -> f64 {
    let idx = (value / alpha).round() as i64;
    let target_parity = if bit { 1 } else { 0 };
    let adjusted = if (idx & 1) == target_parity {
        idx
    } else if value > (idx as f64) * alpha {
        idx + 1
    } else {
        idx - 1
    };
    adjusted as f64 * alpha
}

fn quantize_extract(value: f64, alpha: f64) -> bool {
    let idx = (value / alpha).round() as i64;
    (idx & 1) == 1
}

fn haar_dwt_2d(
    data: &[f64],
    half_w: usize,
    half_h: usize,
) -> (Vec<f64>, Vec<f64>, Vec<f64>, Vec<f64>) {
    let full_w = half_w * 2;
    let full_h = half_h * 2;

    let mut temp = vec![0.0f64; full_w * full_h];
    for y in 0..full_h {
        for x in 0..half_w {
            let x2 = x * 2;
            let a = data[y * full_w + x2];
            let b = data[y * full_w + x2 + 1];
            temp[y * full_w + x] = (a + b) / 2.0;
            temp[y * full_w + half_w + x] = (a - b) / 2.0;
        }
    }

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

fn haar_idwt_2d(
    data: &mut [f64],
    ll: &[f64],
    lh: &[f64],
    hl: &[f64],
    hh: &[f64],
    half_w: usize,
    half_h: usize,
) {
    let full_w = half_w * 2;
    let full_h = half_h * 2;

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

fn dct4x4(block: &Matrix4<f64>) -> Matrix4<f64> {
    let n = BLOCK_SIZE as f64;
    let mut result = Matrix4::<f64>::zeros();

    for u in 0..BLOCK_SIZE {
        for v in 0..BLOCK_SIZE {
            let mut sum = 0.0;
            for x in 0..BLOCK_SIZE {
                for y in 0..BLOCK_SIZE {
                    let cos_x =
                        ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / (2.0 * n)).cos();
                    let cos_y =
                        ((2 * y + 1) as f64 * v as f64 * std::f64::consts::PI / (2.0 * n)).cos();
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
                    let cos_x =
                        ((2 * x + 1) as f64 * u as f64 * std::f64::consts::PI / (2.0 * n)).cos();
                    let cos_y =
                        ((2 * y + 1) as f64 * v as f64 * std::f64::consts::PI / (2.0 * n)).cos();
                    sum += cu * cv * block[(u, v)] * cos_x * cos_y;
                }
            }
            result[(x, y)] = sum;
        }
    }

    result
}

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

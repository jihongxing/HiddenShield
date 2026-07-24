use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use nalgebra::{DMatrix, Matrix4, SVD};

use crate::error::WatermarkError;
use crate::image_spatial_recovery_v1::{
    embed_spatial_recovery_v1, extract_spatial_recovery_v1, extract_spatial_recovery_v1_exact,
    extract_spatial_recovery_v1_exact_scaled, spatial_recovery_v1_scaled_magic_errors,
};
use crate::payload::{
    bits_to_bytes, bytes_to_bits, decode_payload, decode_watermark_payload_readonly,
    encode_payload, encode_payload_v3_minimal_anchor, WatermarkDecodedPayload, WatermarkPayload,
    WatermarkPayloadV3MinimalAnchor, PAYLOAD_BYTES, PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};

pub const DEFAULT_IMAGE_ALPHA: f64 = 50.0;
pub const BALANCED_IMAGE_ALPHA: f64 = 36.0;
pub const DEFAULT_IMAGE_V3_ALPHA: f64 = 36.0;
pub const BALANCED_IMAGE_V3_ALPHA: f64 = 24.0;
const KNOWN_IMAGE_ALPHAS: [f64; 2] = [DEFAULT_IMAGE_ALPHA, BALANCED_IMAGE_ALPHA];
const IMAGE_SCALE_CANDIDATES: [f64; 3] = [1.0, 1.0416666666666667, 1.1764705882352942];
const IMAGE_PADDING_CANDIDATES: [f64; 6] = [0.0, 0.015, 0.02, 0.0208, 0.025, 0.03];
const IMAGE_SYNC_PADDING_CANDIDATES: [f64; 5] = [0.02, 0.0208, 0.0, 0.015, 0.025];
const IMAGE_BRIGHTNESS_CANDIDATES: [f32; 6] = [1.0 / 0.9, 1.0 / 1.1, 1.0 / 1.2, 0.9, 1.1, 1.2];
const IMAGE_BRIGHTNESS_ALPHA_FACTORS: [f64; 5] = [1.0, 0.85, 0.9, 1.1, 1.2];
const SPATIAL_RECOVERY_SCALE_CANDIDATES: [f64; 4] = [0.95, 0.9, 0.85, 0.8];
const SPATIAL_RECOVERY_TRANSFORM_SEARCH_MAX_PIXELS: u64 = 25_000_000;
const IMAGE_SYNC_PREAMBLE: [u8; 4] = [0xA7, 0x5C, 0x3D, 0xE2];
const IMAGE_SYNC_CHECKSUM_BYTES: usize = 2;
const IMAGE_SYNC_PACKET_BYTES: usize = 4 + PAYLOAD_BYTES + IMAGE_SYNC_CHECKSUM_BYTES;
const IMAGE_SYNC_V3_READONLY_PACKET_BYTES: usize =
    4 + PAYLOAD_V3_MINIMAL_ANCHOR_BYTES + IMAGE_SYNC_CHECKSUM_BYTES;
const IMAGE_SYNC_PACKET_BITS: usize = IMAGE_SYNC_PACKET_BYTES * 8;
const IMAGE_SYNC_V3_READONLY_PACKET_BITS: usize = IMAGE_SYNC_V3_READONLY_PACKET_BYTES * 8;
const IMAGE_SYNC_PREAMBLE_BITS: usize = IMAGE_SYNC_PREAMBLE.len() * 8;
const IMAGE_SYNC_MAX_COPIES: usize = 6;
const IMAGE_SYNC_V3_MAX_COPIES: usize = 1;
const IMAGE_SYNC_X_MARGIN_RATIO: f64 = 0.04;
const IMAGE_SYNC_RIGHT_ANCHOR_RATIO: f64 = 0.62;
const IMAGE_SYNC_SEARCH_JITTER_COLS: isize = 6;
const IMAGE_SYNC_SEARCH_JITTER_ROWS: isize = 4;
const IMAGE_SYNC_MAX_CORRECTED_BIT_FLIPS: usize = 2;
const PAYLOAD_BITS: usize = PAYLOAD_BYTES * 8;
const BLOCK_SIZE: usize = 4;
const REDUNDANCY: usize = 3;
const IMAGE_SYNC_V3_REDUNDANCY: usize = 3;
const IMAGE_SYNC_V3_PACKET_BLOCKS: usize =
    IMAGE_SYNC_V3_READONLY_PACKET_BITS * IMAGE_SYNC_V3_REDUNDANCY;
const IMAGE_V3_MAX_LL_DELTA: f64 = 3.0;
const IMAGE_V3_LOW_IMPACT_REDUNDANCY: usize = 5;
const IMAGE_DENSE_PHASES: usize = PAYLOAD_BITS * REDUNDANCY;
const IMAGE_SECONDARY_SINGULAR_ALPHA_RATIO: f64 = 20.0 / 36.0;
const IMAGE_PRIMARY_SINGULAR_WEIGHT: i32 = 3;
const IMAGE_SECONDARY_SINGULAR_WEIGHT: i32 = 1;

struct PreparedImageCandidate {
    half_w: usize,
    blocks_x: usize,
    blocks_y: usize,
    ll: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageReferenceRecovery {
    pub payload: WatermarkPayload,
    pub match_score: f64,
    pub coverage_ratio: f64,
    pub crop_x: u32,
    pub crop_y: u32,
    pub crop_width: u32,
    pub crop_height: u32,
}

pub const MAX_IMAGE_PROTECTION_PIXELS: u64 = 100_000_000;
pub const MAX_IMAGE_PROTECTION_BYTES: usize = 512 * 1024 * 1024;

pub fn validate_image_protection_input(width: u32, height: u32) -> Result<(), &'static str> {
    let pixels = u64::from(width).saturating_mul(u64::from(height));
    if pixels > MAX_IMAGE_PROTECTION_PIXELS {
        return Err("image_pixel_limit_exceeded");
    }
    if !image_embed_capacity_sufficient(width, height) {
        return Err("image_capacity_insufficient");
    }
    Ok(())
}

pub fn validate_image_protection_file_size(bytes: usize) -> Result<(), &'static str> {
    if bytes > MAX_IMAGE_PROTECTION_BYTES {
        return Err("image_file_size_limit_exceeded");
    }
    Ok(())
}

pub fn image_embed_capacity_sufficient(width: u32, height: u32) -> bool {
    let blocks_x = (width / 2) as usize / BLOCK_SIZE;
    let blocks_y = (height / 2) as usize / BLOCK_SIZE;
    blocks_x * blocks_y >= PAYLOAD_BITS * REDUNDANCY
}

#[derive(Debug, Clone, Copy)]
struct ImageCropMatch {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    score: f64,
    coverage_ratio: f64,
}

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

pub fn embed_image_watermark_allow_rewrite(
    image_path: &Path,
    payload: &WatermarkPayload,
    output_path: &Path,
) -> Result<(), WatermarkError> {
    let input_bytes = std::fs::read(image_path)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to read image: {e}")))?;
    let output_format = infer_image_format(output_path).unwrap_or(ImageFormat::Png);
    let output_bytes =
        embed_image_watermark_bytes_allow_rewrite(&input_bytes, payload, output_format)?;
    std::fs::write(output_path, output_bytes)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to write image: {e}")))?;
    Ok(())
}

pub fn embed_image_watermark_bytes(
    image_bytes: &[u8],
    payload: &WatermarkPayload,
    output_format: ImageFormat,
) -> Result<Vec<u8>, WatermarkError> {
    embed_image_watermark_bytes_with_alpha(image_bytes, payload, output_format, DEFAULT_IMAGE_ALPHA)
}

pub fn embed_image_watermark_bytes_with_alpha(
    image_bytes: &[u8],
    payload: &WatermarkPayload,
    output_format: ImageFormat,
    alpha: f64,
) -> Result<Vec<u8>, WatermarkError> {
    reject_existing_image_watermark(image_bytes)?;
    embed_image_watermark_bytes_allow_rewrite_with_alpha(image_bytes, payload, output_format, alpha)
}

pub fn embed_image_watermark_bytes_allow_rewrite(
    image_bytes: &[u8],
    payload: &WatermarkPayload,
    output_format: ImageFormat,
) -> Result<Vec<u8>, WatermarkError> {
    embed_image_watermark_bytes_allow_rewrite_with_alpha(
        image_bytes,
        payload,
        output_format,
        DEFAULT_IMAGE_ALPHA,
    )
}

pub fn embed_image_watermark_bytes_allow_rewrite_with_alpha(
    image_bytes: &[u8],
    payload: &WatermarkPayload,
    output_format: ImageFormat,
    alpha: f64,
) -> Result<Vec<u8>, WatermarkError> {
    if validate_image_protection_file_size(image_bytes.len()).is_err() {
        return Err(WatermarkError::EmbedFailed(
            "image file size limit exceeded: maximum 512 MiB".into(),
        ));
    }
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to open image: {e}")))?;

    let output_img = embed_image_into_dynamic(&img, payload, alpha)?;
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

pub fn extract_image_watermark_bytes(
    image_bytes: &[u8],
) -> Result<WatermarkPayload, WatermarkError> {
    extract_image_watermark_bytes_with_alpha(image_bytes, DEFAULT_IMAGE_ALPHA)
}

pub fn extract_image_watermark_bytes_with_alpha(
    image_bytes: &[u8],
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| WatermarkError::ExtractFailed(format!("failed to open image: {e}")))?;
    extract_image_from_dynamic_candidates(&img, alpha)
}

pub fn extract_image_watermark_readonly_candidate_bytes(
    image_bytes: &[u8],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    extract_image_watermark_readonly_candidate_bytes_with_alpha(image_bytes, DEFAULT_IMAGE_ALPHA)
}

pub fn extract_image_watermark_readonly_candidate_bytes_with_alpha(
    image_bytes: &[u8],
    alpha: f64,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| WatermarkError::ExtractFailed(format!("failed to open image: {e}")))?;
    extract_image_readonly_candidate_from_dynamic(&img, alpha)
}

pub fn extract_image_v3_bytes(
    image_bytes: &[u8],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| WatermarkError::ExtractFailed(format!("failed to open image: {e}")))?;
    if let Ok(decoded) = extract_spatial_recovery_v1_from_dynamic(&img) {
        return Ok(decoded);
    }
    if let Ok(decoded) = extract_v3_low_impact_image_from_dynamic(&img) {
        return Ok(decoded);
    }
    Err(WatermarkError::ExtractFailed(
        "V3 image watermark not found".into(),
    ))
}

/// Builds a PNG fixture that carries a V3/39 minimal anchor in the formal image sync lane.
///
/// This helper is for V3 readonly candidate migration QA only. It intentionally stays separate
/// from production `embed_image_watermark*` paths and must not be exposed as default V3 writing.
pub fn build_v3_readonly_candidate_image_fixture_png_bytes(
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<Vec<u8>, WatermarkError> {
    let source = RgbaImage::from_fn(1024, 1024, |x, y| {
        Rgba([
            ((x as f32 / 1024.0 * 180.0) as u8).wrapping_add(40),
            ((y as f32 / 1024.0 * 180.0) as u8).wrapping_add(50),
            ((x ^ y) & 0x7F) as u8,
            255,
        ])
    });
    let img = DynamicImage::ImageRgba8(source);
    let output_img =
        embed_v3_readonly_candidate_image_into_dynamic(&img, anchor, DEFAULT_IMAGE_ALPHA)?;
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(output_img)
        .write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| WatermarkError::EmbedFailed(format!("encode PNG fixture: {e}")))?;
    Ok(cursor.into_inner())
}

pub fn embed_image_v3_bytes(
    image_bytes: &[u8],
    anchor: &WatermarkPayloadV3MinimalAnchor,
    output_format: ImageFormat,
    alpha: f64,
) -> Result<Vec<u8>, WatermarkError> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to open image: {e}")))?;
    let output_img = if matches!(output_format, ImageFormat::Png) {
        embed_v3_low_impact_image_into_dynamic(&img, anchor)?
    } else {
        embed_v3_readonly_candidate_image_into_dynamic(&img, anchor, alpha)?
    };
    let (output_img, _) = embed_spatial_recovery_v1(&output_img, anchor)?;
    let mut cursor = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(output_img)
        .write_to(&mut cursor, output_format)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to save V3 image: {e}")))?;
    Ok(cursor.into_inner())
}

fn embed_v3_low_impact_image_into_dynamic(
    img: &DynamicImage,
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<RgbaImage, WatermarkError> {
    let (w, h) = img.dimensions();
    let packet = encode_image_sync_packet_v3_readonly(anchor);
    let bits = bytes_to_bits(&packet);
    let required_pixels = bits.len() * IMAGE_V3_LOW_IMPACT_REDUNDANCY;
    if (w as usize).saturating_mul(h as usize) < required_pixels {
        return Err(WatermarkError::EmbedFailed(
            "image too small for V3 low-impact PNG lane".into(),
        ));
    }

    let mut output = img.to_rgba8();
    for (bit_index, bit) in bits.into_iter().enumerate() {
        for copy in 0..IMAGE_V3_LOW_IMPACT_REDUNDANCY {
            let pixel_index = bit_index * IMAGE_V3_LOW_IMPACT_REDUNDANCY + copy;
            let x = (pixel_index % w as usize) as u32;
            let y = (pixel_index / w as usize) as u32;
            let pixel = output.get_pixel_mut(x, y);
            let alpha_bit = (pixel[3] & 1) == 1;
            if alpha_bit == bit {
                continue;
            }
            pixel[3] = if bit { 255 } else { 254 };
        }
    }
    Ok(output)
}

fn extract_v3_low_impact_image_from_dynamic(
    img: &DynamicImage,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    let (w, h) = img.dimensions();
    let packet_bits = IMAGE_SYNC_V3_READONLY_PACKET_BITS;
    let required_pixels = packet_bits * IMAGE_V3_LOW_IMPACT_REDUNDANCY;
    if (w as usize).saturating_mul(h as usize) < required_pixels {
        return Err(WatermarkError::ExtractFailed(
            "image too small for V3 low-impact PNG lane".into(),
        ));
    }

    let rgba = img.to_rgba8();
    let bits = (0..packet_bits)
        .map(|bit_index| {
            let mut score = 0i32;
            for copy in 0..IMAGE_V3_LOW_IMPACT_REDUNDANCY {
                let pixel_index = bit_index * IMAGE_V3_LOW_IMPACT_REDUNDANCY + copy;
                let x = (pixel_index % w as usize) as u32;
                let y = (pixel_index / w as usize) as u32;
                let bit = (rgba.get_pixel(x, y)[3] & 1) == 1;
                score += if bit { 1 } else { -1 };
            }
            score > 0
        })
        .collect::<Vec<_>>();
    let bytes = bits_to_bytes(&bits);
    decode_image_sync_packet_v3_readonly_bytes(&bytes)
}

fn embed_v3_readonly_candidate_image_into_dynamic(
    img: &DynamicImage,
    anchor: &WatermarkPayloadV3MinimalAnchor,
    alpha: f64,
) -> Result<RgbaImage, WatermarkError> {
    let (w, h) = img.dimensions();
    let half_w = (w / 2) as usize;
    let half_h = (h / 2) as usize;
    let blocks_x = half_w / BLOCK_SIZE;
    let blocks_y = half_h / BLOCK_SIZE;
    let (mut y_channel, _, _) = rgb_to_ycbcr_channels(&img);
    let original_y_channel = y_channel.clone();
    let (mut ll, lh, hl, hh) = haar_dwt_2d(&y_channel, half_w, half_h);
    let packet = encode_image_sync_packet_v3_readonly(anchor);
    let sync_bits = bytes_to_bits(&packet);
    let sync_redundant_bits = sync_bits
        .iter()
        .flat_map(|&bit| std::iter::repeat(bit).take(IMAGE_SYNC_V3_REDUNDANCY))
        .collect::<Vec<_>>();
    let anchors = image_sync_embed_anchors_for_packet(
        blocks_x,
        blocks_y,
        IMAGE_SYNC_V3_READONLY_PACKET_BITS,
        0,
        IMAGE_SYNC_V3_MAX_COPIES,
        IMAGE_SYNC_V3_REDUNDANCY,
    );
    if anchors.is_empty() {
        return Err(WatermarkError::EmbedFailed(
            "no image sync anchors available for V3 readonly candidate fixture".into(),
        ));
    }
    for (anchor_x, anchor_y) in anchors {
        embed_bits_dct_svd_v3_at_anchor(
            &mut ll,
            half_w,
            blocks_x,
            blocks_y,
            &sync_redundant_bits,
            alpha,
            anchor_x,
            anchor_y,
        );
    }
    haar_idwt_2d(&mut y_channel, &ll, &lh, &hl, &hh, half_w, half_h);
    Ok(apply_luma_delta_to_rgba(
        img,
        &original_y_channel,
        &y_channel,
        w,
        h,
    ))
}

pub fn detect_existing_image_watermark_bytes(
    image_bytes: &[u8],
) -> Result<Option<WatermarkPayload>, WatermarkError> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| WatermarkError::ExtractFailed(format!("failed to open image: {e}")))?;
    Ok(detect_existing_image_watermark_in_dynamic(&img))
}

pub fn extract_image_watermark_bytes_reference_assisted(
    reference_image_bytes: &[u8],
    suspect_image_bytes: &[u8],
) -> Result<ImageReferenceRecovery, WatermarkError> {
    extract_image_watermark_bytes_reference_assisted_with_alpha(
        reference_image_bytes,
        suspect_image_bytes,
        DEFAULT_IMAGE_ALPHA,
    )
}

pub fn extract_image_watermark_bytes_reference_assisted_with_alpha(
    reference_image_bytes: &[u8],
    suspect_image_bytes: &[u8],
    alpha: f64,
) -> Result<ImageReferenceRecovery, WatermarkError> {
    let reference_img = image::load_from_memory(reference_image_bytes).map_err(|e| {
        WatermarkError::ExtractFailed(format!("failed to open reference image: {e}"))
    })?;
    let suspect_img = image::load_from_memory(suspect_image_bytes)
        .map_err(|e| WatermarkError::ExtractFailed(format!("failed to open suspect image: {e}")))?;

    let crop_match = estimate_reference_crop_match(&reference_img, &suspect_img)?;
    if crop_match.coverage_ratio < 0.25 {
        return Err(WatermarkError::ExtractFailed(format!(
            "reference-assisted crop coverage too low: {:.3}",
            crop_match.coverage_ratio
        )));
    }
    if crop_match.score < 0.78 {
        return Err(WatermarkError::ExtractFailed(format!(
            "reference-assisted crop match too weak: {:.3}",
            crop_match.score
        )));
    }

    let (reference_width, reference_height) = reference_img.dimensions();
    let recovered = recover_crop_to_reference_canvas(
        &suspect_img,
        &crop_match,
        reference_width,
        reference_height,
    );
    let payload = extract_image_from_dynamic_candidates(&recovered, alpha)?;
    Ok(ImageReferenceRecovery {
        payload,
        match_score: crop_match.score,
        coverage_ratio: crop_match.coverage_ratio,
        crop_x: crop_match.x,
        crop_y: crop_match.y,
        crop_width: crop_match.width,
        crop_height: crop_match.height,
    })
}

fn reject_existing_image_watermark(image_bytes: &[u8]) -> Result<(), WatermarkError> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to open image: {e}")))?;
    if let Some(payload) = detect_existing_image_watermark_in_dynamic(&img) {
        return Err(WatermarkError::AlreadyWatermarked {
            existing_uid: payload.watermark_uid(),
        });
    }
    Ok(())
}

fn detect_existing_image_watermark_in_dynamic(img: &DynamicImage) -> Option<WatermarkPayload> {
    for alpha in KNOWN_IMAGE_ALPHAS {
        if let Ok(payload) = extract_image_from_dynamic_fast(img, alpha) {
            return Some(payload);
        }
    }
    None
}

fn extract_image_from_dynamic_candidates(
    img: &DynamicImage,
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    let (w, h) = img.dimensions();

    if let Ok(payload) = extract_image_candidate_with_recovery(img, alpha) {
        return Ok(payload);
    }

    for oriented in image_orientation_candidates(img) {
        if let Ok(payload) = extract_image_from_dynamic_fast(&oriented, alpha) {
            return Ok(payload);
        }
        if let Ok(prepared) = prepare_image_candidate(&oriented) {
            if let Ok(payload) = extract_image_sync_packet_from_prepared(&prepared, alpha) {
                return Ok(payload);
            }
            if let Ok(payload) = extract_image_dense_payload_from_prepared(&prepared, alpha) {
                return Ok(payload);
            }
        }
    }

    let likely_crop_candidate = image_padding_candidate_for_ratio(img, 0.02);
    if let Ok(prepared) = prepare_image_candidate(&likely_crop_candidate) {
        if let Ok(payload) = extract_image_sync_packet_from_prepared(&prepared, alpha) {
            return Ok(payload);
        }
        if let Ok(payload) = extract_image_dense_payload_from_prepared(&prepared, alpha) {
            return Ok(payload);
        }
    }

    for adjusted in image_brightness_candidates(img) {
        let prepared = prepare_image_candidate(&adjusted).ok();
        for alpha_factor in IMAGE_BRIGHTNESS_ALPHA_FACTORS {
            if let Ok(payload) = extract_image_from_dynamic_fast(&adjusted, alpha * alpha_factor) {
                return Ok(payload);
            }
            if let Some(prepared) = prepared.as_ref() {
                if let Ok(payload) =
                    extract_image_sync_packet_from_prepared(&prepared, alpha * alpha_factor)
                {
                    return Ok(payload);
                }
                if let Ok(payload) =
                    extract_image_dense_payload_from_prepared(&prepared, alpha * alpha_factor)
                {
                    return Ok(payload);
                }
            }
        }
    }

    for (target_w, target_h) in image_scale_candidates(w, h) {
        if target_w == w && target_h == h {
            continue;
        }
        let candidate = img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3);
        if let Ok(payload) = extract_image_from_dynamic_fast(&candidate, alpha) {
            return Ok(payload);
        }
        if let Ok(prepared) = prepare_image_candidate(&candidate) {
            if let Ok(payload) = extract_image_sync_packet_from_prepared(&prepared, alpha) {
                return Ok(payload);
            }
            if let Ok(payload) = extract_image_dense_payload_from_prepared(&prepared, alpha) {
                return Ok(payload);
            }
        }
    }

    for padded in image_padding_candidates(img) {
        if padded.dimensions() == (w, h)
            || padded.dimensions() == likely_crop_candidate.dimensions()
        {
            continue;
        }
        let (padded_w, padded_h) = padded.dimensions();
        for (target_w, target_h) in image_scale_candidates(padded_w, padded_h) {
            let candidate = if target_w == padded_w && target_h == padded_h {
                padded.clone()
            } else {
                padded.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3)
            };

            if let Ok(payload) = extract_image_from_dynamic_fast(&candidate, alpha) {
                return Ok(payload);
            }
            if let Ok(prepared) = prepare_image_candidate(&candidate) {
                if let Ok(payload) = extract_image_sync_packet_from_prepared(&prepared, alpha) {
                    return Ok(payload);
                }
                if let Ok(payload) = extract_image_dense_payload_from_prepared(&prepared, alpha) {
                    return Ok(payload);
                }
            }
        }
    }

    for candidate in image_sync_padding_candidates(img) {
        if candidate.dimensions() == likely_crop_candidate.dimensions() {
            continue;
        }
        if let Ok(payload) = extract_image_sync_packet_from_dynamic(&candidate, alpha) {
            return Ok(payload);
        }
        if let Ok(payload) = extract_image_dense_payload_from_dynamic(&candidate, alpha) {
            return Ok(payload);
        }
    }

    Err(WatermarkError::ExtractFailed(
        "image extraction failed".into(),
    ))
}

fn extract_image_readonly_candidate_from_dynamic(
    img: &DynamicImage,
    alpha: f64,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    if let Ok(decoded) = extract_spatial_recovery_v1_from_dynamic(img) {
        return Ok(decoded);
    }
    extract_image_readonly_candidate_legacy_from_dynamic(img, alpha)
}

fn extract_image_readonly_candidate_legacy_from_dynamic(
    img: &DynamicImage,
    alpha: f64,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    if let Ok(decoded) = extract_v3_low_impact_image_from_dynamic(img) {
        return Ok(decoded);
    }

    let (w, h) = img.dimensions();

    if let Ok(decoded) = extract_image_sync_packet_readonly_candidate_from_dynamic(img, alpha) {
        return Ok(decoded);
    }

    for oriented in image_orientation_candidates(img) {
        if let Ok(decoded) =
            extract_image_sync_packet_readonly_candidate_from_dynamic(&oriented, alpha)
        {
            return Ok(decoded);
        }
    }

    let likely_crop_candidate = image_padding_candidate_for_ratio(img, 0.02);
    if let Ok(decoded) =
        extract_image_sync_packet_readonly_candidate_from_dynamic(&likely_crop_candidate, alpha)
    {
        return Ok(decoded);
    }

    for adjusted in image_brightness_candidates(img) {
        for alpha_factor in IMAGE_BRIGHTNESS_ALPHA_FACTORS {
            if let Ok(decoded) = extract_image_sync_packet_readonly_candidate_from_dynamic(
                &adjusted,
                alpha * alpha_factor,
            ) {
                return Ok(decoded);
            }
        }
    }

    for (target_w, target_h) in image_scale_candidates(w, h) {
        if target_w == w && target_h == h {
            continue;
        }
        let candidate = img.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3);
        if let Ok(decoded) =
            extract_image_sync_packet_readonly_candidate_from_dynamic(&candidate, alpha)
        {
            return Ok(decoded);
        }
    }

    for padded in image_padding_candidates(img) {
        if padded.dimensions() == (w, h)
            || padded.dimensions() == likely_crop_candidate.dimensions()
        {
            continue;
        }
        let (padded_w, padded_h) = padded.dimensions();
        for (target_w, target_h) in image_scale_candidates(padded_w, padded_h) {
            let candidate = if target_w == padded_w && target_h == padded_h {
                padded.clone()
            } else {
                padded.resize_exact(target_w, target_h, image::imageops::FilterType::Lanczos3)
            };
            if let Ok(decoded) =
                extract_image_sync_packet_readonly_candidate_from_dynamic(&candidate, alpha)
            {
                return Ok(decoded);
            }
        }
    }

    for candidate in image_sync_padding_candidates(img) {
        if candidate.dimensions() == likely_crop_candidate.dimensions() {
            continue;
        }
        if let Ok(decoded) =
            extract_image_sync_packet_readonly_candidate_from_dynamic(&candidate, alpha)
        {
            return Ok(decoded);
        }
    }

    Err(WatermarkError::ExtractFailed(
        "image readonly candidate extraction failed".into(),
    ))
}

fn extract_spatial_recovery_v1_from_dynamic(
    img: &DynamicImage,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    let rgba = img.to_rgba8();
    if let Ok(anchor) =
        extract_spatial_recovery_v1_exact(&rgba).or_else(|_| extract_spatial_recovery_v1(&rgba))
    {
        return Ok(WatermarkDecodedPayload::V3MinimalAnchor(anchor));
    }

    let mut likely_scale = None;
    for source_scale in SPATIAL_RECOVERY_SCALE_CANDIDATES {
        if let Ok(anchor) = extract_spatial_recovery_v1_exact_scaled(&rgba, source_scale) {
            return Ok(WatermarkDecodedPayload::V3MinimalAnchor(anchor));
        }
        if spatial_recovery_v1_scaled_magic_errors(&rgba, source_scale)
            .is_ok_and(|errors| errors <= 2)
        {
            likely_scale = Some(source_scale);
            break;
        }
    }
    if let Some(source_scale) = likely_scale {
        let target_width = (f64::from(img.width()) / source_scale).round().max(1.0) as u32;
        let target_height = (f64::from(img.height()) / source_scale).round().max(1.0) as u32;
        for filter in [
            image::imageops::FilterType::Nearest,
            image::imageops::FilterType::Triangle,
            image::imageops::FilterType::CatmullRom,
            image::imageops::FilterType::Lanczos3,
        ] {
            let scaled = img.resize_exact(target_width, target_height, filter);
            let scaled_rgba = scaled.to_rgba8();
            if let Ok(anchor) = extract_spatial_recovery_v1_exact(&scaled_rgba) {
                return Ok(WatermarkDecodedPayload::V3MinimalAnchor(anchor));
            }
        }
    }

    let pixels = u64::from(img.width()).saturating_mul(u64::from(img.height()));
    if pixels > SPATIAL_RECOVERY_TRANSFORM_SEARCH_MAX_PIXELS {
        return Err(WatermarkError::ExtractFailed(
            "spatial-recovery-v1 transformed search exceeds resource tier".into(),
        ));
    }

    let oriented_candidates = [img.rotate90(), img.rotate180(), img.rotate270()];
    for oriented in &oriented_candidates {
        let oriented_rgba = oriented.to_rgba8();
        if let Ok(anchor) = extract_spatial_recovery_v1_exact(&oriented_rgba) {
            return Ok(WatermarkDecodedPayload::V3MinimalAnchor(anchor));
        }
    }

    Err(WatermarkError::ExtractFailed(
        "spatial-recovery-v1 packet not found in transform candidates".into(),
    ))
}

fn image_padding_candidates(img: &DynamicImage) -> Vec<DynamicImage> {
    image_padding_candidates_for(img, &IMAGE_PADDING_CANDIDATES)
}

fn image_sync_padding_candidates(img: &DynamicImage) -> Vec<DynamicImage> {
    image_padding_candidates_for(img, &IMAGE_SYNC_PADDING_CANDIDATES)
}

fn image_brightness_candidates(img: &DynamicImage) -> Vec<DynamicImage> {
    IMAGE_BRIGHTNESS_CANDIDATES
        .iter()
        .map(|&factor| adjust_brightness_dynamic(img, factor))
        .collect()
}

fn image_orientation_candidates(img: &DynamicImage) -> Vec<DynamicImage> {
    vec![
        img.rotate90(),
        img.rotate180(),
        img.rotate270(),
        img.fliph(),
        img.flipv(),
    ]
}

fn image_padding_candidate_for_ratio(img: &DynamicImage, ratio: f64) -> DynamicImage {
    image_padding_candidates_for(img, &[ratio])
        .pop()
        .unwrap_or_else(|| img.clone())
}

fn image_padding_candidates_for(img: &DynamicImage, ratios: &[f64]) -> Vec<DynamicImage> {
    let mut candidates = Vec::new();
    let (w, h) = img.dimensions();
    for &ratio in ratios {
        let pad_x = ((w as f64) * ratio).round() as u32;
        let pad_y = ((h as f64) * ratio).round() as u32;
        let candidate = if pad_x == 0 && pad_y == 0 {
            img.clone()
        } else {
            pad_image_replicate_edges(img, pad_x, pad_y)
        };
        let dims = candidate.dimensions();
        if !candidates
            .iter()
            .any(|existing: &DynamicImage| existing.dimensions() == dims)
        {
            candidates.push(candidate);
        }
    }
    candidates
}

fn pad_image_replicate_edges(img: &DynamicImage, pad_x: u32, pad_y: u32) -> DynamicImage {
    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();
    let out_w = w + pad_x * 2;
    let out_h = h + pad_y * 2;
    let mut out = RgbaImage::new(out_w, out_h);

    for y in 0..out_h {
        let src_y = y.saturating_sub(pad_y).min(h - 1);
        for x in 0..out_w {
            let src_x = x.saturating_sub(pad_x).min(w - 1);
            out.put_pixel(x, y, *rgba.get_pixel(src_x, src_y));
        }
    }
    DynamicImage::ImageRgba8(out)
}

fn adjust_brightness_dynamic(img: &DynamicImage, factor: f32) -> DynamicImage {
    let mut out = img.to_rgba8();
    for pixel in out.pixels_mut() {
        for channel in 0..3 {
            pixel[channel] = ((pixel[channel] as f32 * factor).round()).clamp(0.0, 255.0) as u8;
        }
    }
    DynamicImage::ImageRgba8(out)
}

fn estimate_reference_crop_match(
    reference_img: &DynamicImage,
    suspect_img: &DynamicImage,
) -> Result<ImageCropMatch, WatermarkError> {
    let (ref_w, ref_h) = reference_img.dimensions();
    let (suspect_w, suspect_h) = suspect_img.dimensions();
    if suspect_w > ref_w || suspect_h > ref_h {
        return Err(WatermarkError::ExtractFailed(format!(
            "suspect image {}x{} is larger than reference {}x{}",
            suspect_w, suspect_h, ref_w, ref_h
        )));
    }

    let ref_gray = grayscale_luma(reference_img);
    let suspect_gray = grayscale_luma(suspect_img);
    let sample_stride = crop_match_sample_stride(suspect_w, suspect_h);
    let coarse_offset_stride = crop_match_offset_stride(ref_w - suspect_w, ref_h - suspect_h);
    let mut best = best_crop_match_in_window(
        &ref_gray,
        ref_w,
        ref_h,
        &suspect_gray,
        suspect_w,
        suspect_h,
        sample_stride,
        coarse_offset_stride,
        0,
        ref_w - suspect_w,
        0,
        ref_h - suspect_h,
    )?;

    if coarse_offset_stride > 1 {
        let refine_radius = coarse_offset_stride * 2;
        let min_x = best.x.saturating_sub(refine_radius);
        let max_x = (best.x + refine_radius).min(ref_w - suspect_w);
        let min_y = best.y.saturating_sub(refine_radius);
        let max_y = (best.y + refine_radius).min(ref_h - suspect_h);
        best = best_crop_match_in_window(
            &ref_gray,
            ref_w,
            ref_h,
            &suspect_gray,
            suspect_w,
            suspect_h,
            sample_stride,
            1,
            min_x,
            max_x,
            min_y,
            max_y,
        )?;
    }

    Ok(best)
}

#[allow(clippy::too_many_arguments)]
fn best_crop_match_in_window(
    ref_gray: &[f64],
    ref_w: u32,
    _ref_h: u32,
    suspect_gray: &[f64],
    suspect_w: u32,
    suspect_h: u32,
    sample_stride: u32,
    offset_stride: u32,
    min_x: u32,
    max_x: u32,
    min_y: u32,
    max_y: u32,
) -> Result<ImageCropMatch, WatermarkError> {
    let suspect_stats = sampled_stats(suspect_gray, suspect_w, suspect_h, sample_stride, 0, 0);
    if suspect_stats.count == 0 || suspect_stats.variance <= f64::EPSILON {
        return Err(WatermarkError::ExtractFailed(
            "suspect image has too little texture for reference matching".into(),
        ));
    }

    let mut best = ImageCropMatch {
        x: min_x,
        y: min_y,
        width: suspect_w,
        height: suspect_h,
        score: f64::NEG_INFINITY,
        coverage_ratio: (suspect_w as f64 * suspect_h as f64) / (ref_w as f64 * _ref_h as f64),
    };

    let mut y = min_y;
    while y <= max_y {
        let mut x = min_x;
        while x <= max_x {
            let score = sampled_normalized_correlation(
                ref_gray,
                ref_w,
                suspect_gray,
                suspect_w,
                suspect_h,
                sample_stride,
                x,
                y,
                &suspect_stats,
            );
            if score > best.score {
                best.x = x;
                best.y = y;
                best.score = score;
            }
            if max_x - x < offset_stride {
                break;
            }
            x += offset_stride;
        }
        if max_y - y < offset_stride {
            break;
        }
        y += offset_stride;
    }

    Ok(best)
}

#[derive(Debug, Clone, Copy)]
struct SampledStats {
    count: usize,
    mean: f64,
    variance: f64,
}

fn sampled_stats(
    gray: &[f64],
    width: u32,
    height: u32,
    sample_stride: u32,
    offset_x: u32,
    offset_y: u32,
) -> SampledStats {
    let mut count = 0usize;
    let mut sum = 0.0;
    let mut sum_sq = 0.0;
    let mut y = 0u32;
    while y < height {
        let mut x = 0u32;
        while x < width {
            let value = gray[((offset_y + y) * width + offset_x + x) as usize];
            count += 1;
            sum += value;
            sum_sq += value * value;
            x += sample_stride;
        }
        y += sample_stride;
    }
    let mean = if count == 0 { 0.0 } else { sum / count as f64 };
    let variance = if count == 0 {
        0.0
    } else {
        (sum_sq / count as f64) - mean * mean
    };
    SampledStats {
        count,
        mean,
        variance: variance.max(0.0),
    }
}

#[allow(clippy::too_many_arguments)]
fn sampled_normalized_correlation(
    ref_gray: &[f64],
    ref_w: u32,
    suspect_gray: &[f64],
    suspect_w: u32,
    suspect_h: u32,
    sample_stride: u32,
    offset_x: u32,
    offset_y: u32,
    suspect_stats: &SampledStats,
) -> f64 {
    let mut count = 0usize;
    let mut ref_sum = 0.0;
    let mut ref_sum_sq = 0.0;
    let mut cross = 0.0;
    let mut y = 0u32;
    while y < suspect_h {
        let mut x = 0u32;
        while x < suspect_w {
            let ref_value = ref_gray[((offset_y + y) * ref_w + offset_x + x) as usize];
            let suspect_value = suspect_gray[(y * suspect_w + x) as usize];
            count += 1;
            ref_sum += ref_value;
            ref_sum_sq += ref_value * ref_value;
            cross += ref_value * (suspect_value - suspect_stats.mean);
            x += sample_stride;
        }
        y += sample_stride;
    }
    if count == 0 {
        return f64::NEG_INFINITY;
    }
    let ref_mean = ref_sum / count as f64;
    let ref_variance = ((ref_sum_sq / count as f64) - ref_mean * ref_mean).max(0.0);
    if ref_variance <= f64::EPSILON || suspect_stats.variance <= f64::EPSILON {
        return f64::NEG_INFINITY;
    }
    let centered_cross = cross - ref_mean * 0.0;
    centered_cross / (count as f64 * ref_variance.sqrt() * suspect_stats.variance.sqrt())
}

fn crop_match_sample_stride(width: u32, height: u32) -> u32 {
    (width.min(height) / 180).clamp(4, 12)
}

fn crop_match_offset_stride(max_x: u32, max_y: u32) -> u32 {
    let positions = (max_x as u64 + 1) * (max_y as u64 + 1);
    if positions <= 20_000 {
        1
    } else {
        ((positions as f64 / 20_000.0).sqrt().ceil() as u32).max(1)
    }
}

fn recover_crop_to_reference_canvas(
    suspect_img: &DynamicImage,
    crop_match: &ImageCropMatch,
    reference_width: u32,
    reference_height: u32,
) -> DynamicImage {
    let suspect = suspect_img.to_rgba8();
    let (suspect_w, suspect_h) = suspect.dimensions();
    let mut out = RgbaImage::new(reference_width, reference_height);

    for y in 0..reference_height {
        let src_y = y
            .saturating_sub(crop_match.y)
            .min(suspect_h.saturating_sub(1));
        for x in 0..reference_width {
            let src_x = x
                .saturating_sub(crop_match.x)
                .min(suspect_w.saturating_sub(1));
            out.put_pixel(x, y, *suspect.get_pixel(src_x, src_y));
        }
    }
    DynamicImage::ImageRgba8(out)
}

fn grayscale_luma(img: &DynamicImage) -> Vec<f64> {
    let (w, h) = img.dimensions();
    let mut gray = Vec::with_capacity((w * h) as usize);
    for y in 0..h {
        for x in 0..w {
            let pixel = img.get_pixel(x, y);
            gray.push(0.299 * pixel[0] as f64 + 0.587 * pixel[1] as f64 + 0.114 * pixel[2] as f64);
        }
    }
    gray
}

fn embed_image_into_dynamic(
    img: &DynamicImage,
    payload: &WatermarkPayload,
    alpha: f64,
) -> Result<RgbaImage, WatermarkError> {
    let (w, h) = img.dimensions();
    let half_w = (w / 2) as usize;
    let half_h = (h / 2) as usize;
    let blocks_x = half_w / BLOCK_SIZE;
    let blocks_y = half_h / BLOCK_SIZE;

    if let Err(reason) = validate_image_protection_input(w, h) {
        if reason == "image_pixel_limit_exceeded" {
            return Err(WatermarkError::EmbedFailed(
                "image pixel limit exceeded: maximum 100 MP".into(),
            ));
        }
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
    embed_bits_dct_svd_periodic(
        &mut ll,
        half_w,
        blocks_x,
        blocks_y,
        &redundant_bits,
        alpha,
        0,
        0,
    );

    let sync_packet = encode_image_sync_packet(payload);
    let sync_bits = bytes_to_bits(&sync_packet);
    let sync_redundant_bits: Vec<bool> = sync_bits
        .iter()
        .flat_map(|&bit| std::iter::repeat(bit).take(REDUNDANCY))
        .collect();
    for (anchor_x, anchor_y) in image_sync_embed_anchors(blocks_x, blocks_y) {
        embed_bits_dct_svd_at_anchor(
            &mut ll,
            half_w,
            blocks_x,
            blocks_y,
            &sync_redundant_bits,
            alpha,
            anchor_x,
            anchor_y,
        );
    }

    haar_idwt_2d(&mut y_channel, &ll, &lh, &hl, &hh, half_w, half_h);

    let output_img = ycbcr_to_rgba(&y_channel, &cb_channel, &cr_channel, w, h);
    Ok(output_img)
}

fn extract_image_from_dynamic_fast(
    img: &DynamicImage,
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    let prepared = prepare_image_candidate(img)?;
    extract_image_from_prepared(&prepared, alpha)
}

fn extract_image_candidate_with_recovery(
    img: &DynamicImage,
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    let prepared = prepare_image_candidate(img)?;
    if let Ok(payload) = extract_image_from_prepared(&prepared, alpha) {
        return Ok(payload);
    }
    if let Ok(payload) = extract_image_sync_packet_from_prepared(&prepared, alpha) {
        return Ok(payload);
    }
    extract_image_dense_payload_from_prepared(&prepared, alpha)
}

fn prepare_image_candidate(img: &DynamicImage) -> Result<PreparedImageCandidate, WatermarkError> {
    let (w, h) = img.dimensions();
    let half_w = (w / 2) as usize;
    let half_h = (h / 2) as usize;
    let blocks_x = half_w / BLOCK_SIZE;
    let blocks_y = half_h / BLOCK_SIZE;
    let total_blocks = blocks_x * blocks_y;

    if total_blocks < PAYLOAD_BITS * REDUNDANCY {
        return Err(WatermarkError::ExtractFailed(
            "image too small for watermark extraction".into(),
        ));
    }

    let (y_channel, _, _) = rgb_to_ycbcr_channels(&img);
    let (ll, _, _, _) = haar_dwt_2d(&y_channel, half_w, half_h);
    Ok(PreparedImageCandidate {
        half_w,
        blocks_x,
        blocks_y,
        ll,
    })
}

fn extract_image_from_prepared(
    candidate: &PreparedImageCandidate,
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    decode_payload_from_ll_anchor(
        &candidate.ll,
        candidate.half_w,
        candidate.blocks_x,
        candidate.blocks_y,
        alpha,
        0,
        0,
    )
}

fn decode_payload_from_ll_anchor(
    ll: &[f64],
    ll_w: usize,
    blocks_x: usize,
    blocks_y: usize,
    alpha: f64,
    anchor_x: usize,
    anchor_y: usize,
) -> Result<WatermarkPayload, WatermarkError> {
    let bits = extract_bits_dct_svd_legacy_at_anchor(
        ll, ll_w, blocks_x, blocks_y, alpha, anchor_x, anchor_y,
    );
    let payload_bytes = bits_to_bytes(&bits);
    let mut arr = [0u8; PAYLOAD_BYTES];
    arr.copy_from_slice(&payload_bytes[..PAYLOAD_BYTES]);
    decode_payload(&arr)
}

fn extract_image_sync_packet_from_dynamic(
    img: &DynamicImage,
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    let prepared = prepare_image_candidate(img)?;
    extract_image_sync_packet_from_prepared(&prepared, alpha)
}

fn extract_image_sync_packet_readonly_candidate_from_dynamic(
    img: &DynamicImage,
    alpha: f64,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    let prepared = prepare_image_candidate(img)?;
    extract_image_sync_packet_readonly_candidate_from_prepared(&prepared, alpha)
}

fn extract_image_dense_payload_from_dynamic(
    img: &DynamicImage,
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    let prepared = prepare_image_candidate(img)?;
    extract_image_dense_payload_from_prepared(&prepared, alpha)
}

fn extract_image_sync_packet_from_prepared(
    candidate: &PreparedImageCandidate,
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    if candidate.blocks_x * candidate.blocks_y < IMAGE_SYNC_PACKET_BITS * REDUNDANCY {
        return Err(WatermarkError::ExtractFailed(
            "image too small for sync packet extraction".into(),
        ));
    }

    let mut last_error = None;
    for (anchor_x, anchor_y) in image_sync_search_anchors(candidate.blocks_x, candidate.blocks_y) {
        let preamble_bits = extract_raw_bit_scores_dct_svd_at_anchor(
            &candidate.ll,
            candidate.half_w,
            candidate.blocks_x,
            candidate.blocks_y,
            alpha,
            anchor_x,
            anchor_y,
            IMAGE_SYNC_PREAMBLE_BITS * REDUNDANCY,
            0,
        );
        if !image_sync_preamble_matches(&preamble_bits) {
            last_error = Some(WatermarkError::ExtractFailed(
                "sync packet preamble mismatch".into(),
            ));
            continue;
        }

        let bits = extract_image_sync_packet_bits_after_preamble(
            candidate,
            alpha,
            anchor_x,
            anchor_y,
            preamble_bits,
        );
        match decode_image_sync_packet_from_raw_bits(&bits) {
            Ok(payload) => return Ok(payload),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| WatermarkError::ExtractFailed("sync packet not found".into())))
}

fn extract_image_sync_packet_readonly_candidate_from_prepared(
    candidate: &PreparedImageCandidate,
    alpha: f64,
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    if candidate.blocks_x * candidate.blocks_y < IMAGE_SYNC_V3_PACKET_BLOCKS {
        return Err(WatermarkError::ExtractFailed(
            "image too small for readonly sync packet extraction".into(),
        ));
    }

    let mut last_error = None;
    for (anchor_x, anchor_y) in image_sync_v3_search_anchors(candidate.blocks_x, candidate.blocks_y)
    {
        let preamble_bits = extract_raw_bit_scores_dct_svd_at_anchor(
            &candidate.ll,
            candidate.half_w,
            candidate.blocks_x,
            candidate.blocks_y,
            alpha,
            anchor_x,
            anchor_y,
            IMAGE_SYNC_PREAMBLE_BITS * IMAGE_SYNC_V3_REDUNDANCY,
            0,
        );
        if !image_sync_preamble_matches_with_redundancy(&preamble_bits, IMAGE_SYNC_V3_REDUNDANCY) {
            last_error = Some(WatermarkError::ExtractFailed(
                "readonly sync packet preamble mismatch".into(),
            ));
            continue;
        }

        if candidate.blocks_x * candidate.blocks_y >= IMAGE_SYNC_PACKET_BITS * REDUNDANCY {
            let v2_bits = extract_image_sync_packet_bits_after_preamble_for_packet_bits(
                candidate,
                alpha,
                anchor_x,
                anchor_y,
                preamble_bits.clone(),
                IMAGE_SYNC_PACKET_BITS,
                REDUNDANCY,
            );
            match decode_image_sync_packet_from_raw_bits(&v2_bits) {
                Ok(payload) => return Ok(WatermarkDecodedPayload::V2(payload)),
                Err(error) => last_error = Some(error),
            }
        }

        let v3_bits = extract_image_sync_packet_bits_after_preamble_for_packet_bits(
            candidate,
            alpha,
            anchor_x,
            anchor_y,
            preamble_bits.clone(),
            IMAGE_SYNC_V3_READONLY_PACKET_BITS,
            IMAGE_SYNC_V3_REDUNDANCY,
        );
        match decode_image_sync_packet_v3_readonly_from_raw_bits(&v3_bits) {
            Ok(decoded) => return Ok(decoded),
            Err(error) => {
                if last_error.is_none() {
                    last_error = Some(error);
                }
            }
        }
    }

    Err(last_error
        .unwrap_or_else(|| WatermarkError::ExtractFailed("readonly sync packet not found".into())))
}

fn extract_image_dense_payload_from_prepared(
    candidate: &PreparedImageCandidate,
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    if candidate.blocks_x * candidate.blocks_y < IMAGE_DENSE_PHASES {
        return Err(WatermarkError::ExtractFailed(
            "image too small for dense payload extraction".into(),
        ));
    }

    let raw_scores = extract_raw_bit_scores_dct_svd_at_anchor(
        &candidate.ll,
        candidate.half_w,
        candidate.blocks_x,
        candidate.blocks_y,
        alpha,
        0,
        0,
        candidate.blocks_x * candidate.blocks_y,
        0,
    );

    let mut last_error = None;
    for phase in 0..IMAGE_DENSE_PHASES {
        let bits = dense_majority_bits_for_phase(&raw_scores, phase);
        let payload_bytes = bits_to_bytes(&bits);
        let mut arr = [0u8; PAYLOAD_BYTES];
        arr.copy_from_slice(&payload_bytes[..PAYLOAD_BYTES]);
        match decode_payload(&arr) {
            Ok(payload) => return Ok(payload),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error
        .unwrap_or_else(|| WatermarkError::ExtractFailed("dense payload not found".into())))
}

fn dense_majority_bits_for_phase(raw_scores: &[i32], phase: usize) -> Vec<bool> {
    let mut scores = [0i32; PAYLOAD_BITS];
    for (index, &score) in raw_scores.iter().enumerate() {
        let redundant_index = (index + phase) % IMAGE_DENSE_PHASES;
        let payload_index = redundant_index / REDUNDANCY;
        scores[payload_index] += score;
    }
    (0..PAYLOAD_BITS).map(|index| scores[index] > 0).collect()
}

fn image_sync_preamble_matches(raw_scores: &[i32]) -> bool {
    image_sync_preamble_matches_with_redundancy(raw_scores, REDUNDANCY)
}

fn image_sync_preamble_matches_with_redundancy(raw_scores: &[i32], redundancy: usize) -> bool {
    if raw_scores.len() < IMAGE_SYNC_PREAMBLE_BITS * redundancy {
        return false;
    }
    let bits = majority_bits_for_window_with_redundancy(
        raw_scores,
        0,
        IMAGE_SYNC_PREAMBLE_BITS,
        redundancy,
    );
    let bytes = bits_to_bytes(&bits);
    bytes.get(..IMAGE_SYNC_PREAMBLE.len()) == Some(&IMAGE_SYNC_PREAMBLE)
}

fn extract_image_sync_packet_bits_after_preamble(
    candidate: &PreparedImageCandidate,
    alpha: f64,
    anchor_x: usize,
    anchor_y: usize,
    preamble_bits: Vec<i32>,
) -> Vec<i32> {
    extract_image_sync_packet_bits_after_preamble_for_packet_bits(
        candidate,
        alpha,
        anchor_x,
        anchor_y,
        preamble_bits,
        IMAGE_SYNC_PACKET_BITS,
        REDUNDANCY,
    )
}

fn extract_image_sync_packet_bits_after_preamble_for_packet_bits(
    candidate: &PreparedImageCandidate,
    alpha: f64,
    anchor_x: usize,
    anchor_y: usize,
    preamble_bits: Vec<i32>,
    packet_bits: usize,
    redundancy: usize,
) -> Vec<i32> {
    let preamble_blocks = IMAGE_SYNC_PREAMBLE_BITS * redundancy;
    let packet_blocks = packet_bits * redundancy;
    let remaining_blocks = packet_blocks.saturating_sub(preamble_blocks);
    let mut bits = preamble_bits;
    bits.extend(extract_raw_bit_scores_dct_svd_at_anchor(
        &candidate.ll,
        candidate.half_w,
        candidate.blocks_x,
        candidate.blocks_y,
        alpha,
        anchor_x,
        anchor_y,
        remaining_blocks,
        preamble_blocks,
    ));
    bits
}

fn image_sync_embed_anchors(blocks_x: usize, blocks_y: usize) -> Vec<(usize, usize)> {
    image_sync_embed_anchors_for_packet(
        blocks_x,
        blocks_y,
        IMAGE_SYNC_PACKET_BITS,
        PAYLOAD_BITS * REDUNDANCY,
        IMAGE_SYNC_MAX_COPIES,
        REDUNDANCY,
    )
}

fn image_sync_embed_anchors_for_packet(
    blocks_x: usize,
    blocks_y: usize,
    packet_bits: usize,
    reserved_blocks: usize,
    max_copies: usize,
    redundancy: usize,
) -> Vec<(usize, usize)> {
    let sync_blocks = packet_bits * redundancy;
    if blocks_x * blocks_y < reserved_blocks + sync_blocks {
        return Vec::new();
    }

    let legacy_rows = reserved_blocks.div_ceil(blocks_x);
    let mut anchors = Vec::new();
    let mut anchor_y = legacy_rows + 2;
    while anchors.len() < max_copies && anchor_y < blocks_y {
        let anchor_x = image_sync_anchor_x_for_index(blocks_x, anchors.len());
        let usable_blocks_x = blocks_x.saturating_sub(anchor_x).max(1);
        let sync_rows = sync_blocks.div_ceil(usable_blocks_x);
        if anchor_y + sync_rows > blocks_y {
            break;
        }
        anchors.push((anchor_x, anchor_y));
        anchor_y += sync_rows + 2;
    }
    anchors
}

fn image_sync_anchor_x_for_index(blocks_x: usize, index: usize) -> usize {
    match index % 3 {
        0 => 0,
        1 => image_sync_inner_anchor_x(blocks_x),
        _ => image_sync_right_anchor_x(blocks_x),
    }
}

fn image_sync_search_anchors(blocks_x: usize, blocks_y: usize) -> Vec<(usize, usize)> {
    let mut anchors = Vec::new();
    for (anchor_x, anchor_y) in image_sync_embed_anchors_for_packet(
        blocks_x,
        blocks_y,
        IMAGE_SYNC_V3_READONLY_PACKET_BITS,
        0,
        IMAGE_SYNC_V3_MAX_COPIES,
        IMAGE_SYNC_V3_REDUNDANCY,
    )
    .into_iter()
    .chain(image_sync_embed_anchors(blocks_x, blocks_y))
    {
        for dy in -IMAGE_SYNC_SEARCH_JITTER_ROWS..=IMAGE_SYNC_SEARCH_JITTER_ROWS {
            let Some(search_y) = anchor_y.checked_add_signed(dy) else {
                continue;
            };
            if search_y >= blocks_y {
                continue;
            }
            for dx in -IMAGE_SYNC_SEARCH_JITTER_COLS..=IMAGE_SYNC_SEARCH_JITTER_COLS {
                let Some(search_x) = anchor_x.checked_add_signed(dx) else {
                    continue;
                };
                if search_x >= blocks_x {
                    continue;
                }
                let anchor = (search_x, search_y);
                if !anchors.contains(&anchor) {
                    anchors.push(anchor);
                }
            }
        }
    }
    anchors
}

fn image_sync_v3_search_anchors(blocks_x: usize, blocks_y: usize) -> Vec<(usize, usize)> {
    let mut anchors = Vec::new();
    for (anchor_x, anchor_y) in image_sync_embed_anchors_for_packet(
        blocks_x,
        blocks_y,
        IMAGE_SYNC_V3_READONLY_PACKET_BITS,
        0,
        IMAGE_SYNC_V3_MAX_COPIES,
        IMAGE_SYNC_V3_REDUNDANCY,
    ) {
        for dy in -IMAGE_SYNC_SEARCH_JITTER_ROWS..=IMAGE_SYNC_SEARCH_JITTER_ROWS {
            let Some(search_y) = anchor_y.checked_add_signed(dy) else {
                continue;
            };
            if search_y >= blocks_y {
                continue;
            }
            for dx in -IMAGE_SYNC_SEARCH_JITTER_COLS..=IMAGE_SYNC_SEARCH_JITTER_COLS {
                let Some(search_x) = anchor_x.checked_add_signed(dx) else {
                    continue;
                };
                if search_x >= blocks_x {
                    continue;
                }
                let anchor = (search_x, search_y);
                if !anchors.contains(&anchor) {
                    anchors.push(anchor);
                }
            }
        }
    }
    anchors
}

fn image_sync_inner_anchor_x(blocks_x: usize) -> usize {
    ((blocks_x as f64) * IMAGE_SYNC_X_MARGIN_RATIO)
        .round()
        .clamp(1.0, blocks_x.saturating_sub(1) as f64) as usize
}

fn image_sync_right_anchor_x(blocks_x: usize) -> usize {
    ((blocks_x as f64) * IMAGE_SYNC_RIGHT_ANCHOR_RATIO)
        .round()
        .clamp(1.0, blocks_x.saturating_sub(1) as f64) as usize
}

fn encode_image_sync_packet(payload: &WatermarkPayload) -> [u8; IMAGE_SYNC_PACKET_BYTES] {
    let payload_bytes = encode_payload(payload);
    let checksum = image_sync_checksum(&payload_bytes);
    let mut packet = [0u8; IMAGE_SYNC_PACKET_BYTES];
    packet[0..4].copy_from_slice(&IMAGE_SYNC_PREAMBLE);
    packet[4..4 + PAYLOAD_BYTES].copy_from_slice(&payload_bytes);
    packet[4 + PAYLOAD_BYTES..4 + PAYLOAD_BYTES + IMAGE_SYNC_CHECKSUM_BYTES]
        .copy_from_slice(&checksum);
    packet
}

fn encode_image_sync_packet_v3_readonly(
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> [u8; IMAGE_SYNC_V3_READONLY_PACKET_BYTES] {
    let payload_bytes = encode_payload_v3_minimal_anchor(anchor);
    let checksum = image_sync_checksum_bytes(&payload_bytes);
    let mut packet = [0u8; IMAGE_SYNC_V3_READONLY_PACKET_BYTES];
    packet[0..4].copy_from_slice(&IMAGE_SYNC_PREAMBLE);
    packet[4..4 + PAYLOAD_V3_MINIMAL_ANCHOR_BYTES].copy_from_slice(&payload_bytes);
    packet[4 + PAYLOAD_V3_MINIMAL_ANCHOR_BYTES
        ..4 + PAYLOAD_V3_MINIMAL_ANCHOR_BYTES + IMAGE_SYNC_CHECKSUM_BYTES]
        .copy_from_slice(&checksum);
    packet
}

fn decode_image_sync_packet_v3_readonly_bytes(
    bytes: &[u8],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    if bytes.len() < IMAGE_SYNC_V3_READONLY_PACKET_BYTES {
        return Err(WatermarkError::ExtractFailed(
            "sync packet v3 readonly too short".into(),
        ));
    }
    if bytes[0..4] != IMAGE_SYNC_PREAMBLE {
        return Err(WatermarkError::ExtractFailed(
            "sync packet v3 readonly preamble mismatch".into(),
        ));
    }

    let payload_start = 4;
    let payload_end = payload_start + PAYLOAD_V3_MINIMAL_ANCHOR_BYTES;
    let payload_bytes = &bytes[payload_start..payload_end];
    let expected_checksum = image_sync_checksum_bytes(payload_bytes);
    if bytes[payload_end..payload_end + IMAGE_SYNC_CHECKSUM_BYTES] != expected_checksum {
        return Err(WatermarkError::ExtractFailed(
            "sync packet v3 readonly checksum mismatch".into(),
        ));
    }

    let decoded = decode_watermark_payload_readonly(payload_bytes)?;
    match decoded {
        WatermarkDecodedPayload::V3MinimalAnchor(_) => Ok(decoded),
        WatermarkDecodedPayload::V2(_) => Err(WatermarkError::ExtractFailed(
            "sync packet v3 readonly expected minimal anchor".into(),
        )),
    }
}

fn decode_image_sync_packet_from_raw_bits(
    raw_scores: &[i32],
) -> Result<WatermarkPayload, WatermarkError> {
    let packet_blocks = IMAGE_SYNC_PACKET_BITS * REDUNDANCY;
    if raw_scores.len() < packet_blocks {
        return Err(WatermarkError::ExtractFailed(
            "not enough data for sync packet extraction".into(),
        ));
    }

    let bits = majority_bits_for_window(raw_scores, 0, IMAGE_SYNC_PACKET_BITS);
    let bytes = bits_to_bytes(&bits);
    decode_image_sync_packet_bytes(&bytes).or_else(|first_error| {
        correct_image_sync_packet_bits(&bits, IMAGE_SYNC_MAX_CORRECTED_BIT_FLIPS).ok_or(first_error)
    })
}

fn decode_image_sync_packet_v3_readonly_from_raw_bits(
    raw_scores: &[i32],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    if raw_scores.len() < IMAGE_SYNC_V3_PACKET_BLOCKS {
        return Err(WatermarkError::ExtractFailed(
            "not enough data for readonly sync packet extraction".into(),
        ));
    }

    let bits = majority_bits_for_window_with_redundancy(
        raw_scores,
        0,
        IMAGE_SYNC_V3_READONLY_PACKET_BITS,
        IMAGE_SYNC_V3_REDUNDANCY,
    );
    let bytes = bits_to_bytes(&bits);
    decode_image_sync_packet_v3_readonly_bytes(&bytes)
}

fn decode_image_sync_packet_bytes(bytes: &[u8]) -> Result<WatermarkPayload, WatermarkError> {
    if bytes.len() < IMAGE_SYNC_PACKET_BYTES {
        return Err(WatermarkError::ExtractFailed(
            "sync packet too short".into(),
        ));
    }
    if bytes[0..4] != IMAGE_SYNC_PREAMBLE {
        return Err(WatermarkError::ExtractFailed(
            "sync packet preamble mismatch".into(),
        ));
    }

    let mut payload_bytes = [0u8; PAYLOAD_BYTES];
    payload_bytes.copy_from_slice(&bytes[4..4 + PAYLOAD_BYTES]);
    let expected_checksum = image_sync_checksum(&payload_bytes);
    if bytes[4 + PAYLOAD_BYTES..4 + PAYLOAD_BYTES + IMAGE_SYNC_CHECKSUM_BYTES] != expected_checksum
    {
        return Err(WatermarkError::ExtractFailed(
            "sync packet checksum mismatch".into(),
        ));
    }

    decode_payload(&payload_bytes)
}

fn image_sync_checksum(payload_bytes: &[u8; PAYLOAD_BYTES]) -> [u8; IMAGE_SYNC_CHECKSUM_BYTES] {
    image_sync_checksum_bytes(payload_bytes)
}

fn image_sync_checksum_bytes(payload_bytes: &[u8]) -> [u8; IMAGE_SYNC_CHECKSUM_BYTES] {
    let mut state = 0xA5C3u16;
    for &byte in payload_bytes {
        state = state.rotate_left(5) ^ byte as u16;
        state = state.wrapping_mul(251);
    }
    state.to_be_bytes()
}

fn correct_image_sync_packet_bits(bits: &[bool], max_flips: usize) -> Option<WatermarkPayload> {
    if bits.len() < IMAGE_SYNC_PACKET_BITS || max_flips == 0 {
        return None;
    }

    let mut bit_buffer = bits[..IMAGE_SYNC_PACKET_BITS].to_vec();
    let mutable_start = IMAGE_SYNC_PREAMBLE_BITS;
    for flips in 1..=max_flips {
        if let Some(payload) =
            correct_image_sync_packet_bits_at_depth(&mut bit_buffer, mutable_start, flips)
        {
            return Some(payload);
        }
    }
    None
}

fn correct_image_sync_packet_bits_at_depth(
    bits: &mut [bool],
    start_index: usize,
    remaining_flips: usize,
) -> Option<WatermarkPayload> {
    if remaining_flips == 0 {
        let bytes = bits_to_bytes(bits);
        return decode_image_sync_packet_bytes(&bytes).ok();
    }

    for index in start_index..IMAGE_SYNC_PACKET_BITS {
        bits[index] = !bits[index];
        if let Some(payload) =
            correct_image_sync_packet_bits_at_depth(bits, index + 1, remaining_flips - 1)
        {
            return Some(payload);
        }
        bits[index] = !bits[index];
    }
    None
}

fn majority_bits_for_window(raw_scores: &[i32], start: usize, bit_count: usize) -> Vec<bool> {
    majority_bits_for_window_with_redundancy(raw_scores, start, bit_count, REDUNDANCY)
}

fn majority_bits_for_window_with_redundancy(
    raw_scores: &[i32],
    start: usize,
    bit_count: usize,
    redundancy: usize,
) -> Vec<bool> {
    (0..bit_count)
        .map(|bit_idx| {
            let bit_start = start + bit_idx * redundancy;
            let chunk = &raw_scores[bit_start..bit_start + redundancy];
            chunk.iter().sum::<i32>() > 0
        })
        .collect()
}

fn image_scale_candidates(w: u32, h: u32) -> Vec<(u32, u32)> {
    let mut candidates = Vec::new();
    for factor in IMAGE_SCALE_CANDIDATES {
        let target_w = ((w as f64) * factor).round().max(1.0) as u32;
        let target_h = ((h as f64) * factor).round().max(1.0) as u32;
        let dims = (target_w, target_h);
        if !candidates.contains(&dims) {
            candidates.push(dims);
        }
    }
    candidates
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

    fn sample_v3_anchor() -> WatermarkPayloadV3MinimalAnchor {
        WatermarkPayloadV3MinimalAnchor::new(crate::PayloadV3MinimalAnchorBuildInput {
            watermark_id: [
                0x31, 0x32, 0x33, 0x34, 0x41, 0x42, 0x43, 0x44, 0x51, 0x52, 0x53, 0x54, 0x61, 0x62,
                0x63, 0x64,
            ],
        })
        .unwrap()
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
        let mut cursor = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(img)
            .write_to(&mut cursor, ImageFormat::Png)
            .unwrap();
        cursor.into_inner()
    }

    #[test]
    fn image_sync_packet_v3_readonly_roundtrips_minimal_anchor_without_v2_decode() {
        let anchor = sample_v3_anchor();
        let packet = encode_image_sync_packet_v3_readonly(&anchor);

        assert_eq!(packet.len(), IMAGE_SYNC_V3_READONLY_PACKET_BYTES);
        assert!(decode_image_sync_packet_bytes(&packet).is_err());
        let decoded = decode_image_sync_packet_v3_readonly_bytes(&packet).unwrap();

        assert!(decoded.is_v3_minimal_anchor());
        assert_eq!(decoded.watermark_uid(), anchor.watermark_uid());
        assert_eq!(decoded.protocol_version(), 3);
        assert_eq!(decoded.payload_bytes_length(), 39);
        assert_eq!(decoded.payload_auth_status(), "verified");
    }

    #[test]
    fn image_v3_low_impact_lane_roundtrip() {
        let input = make_png_bytes();
        let img = image::load_from_memory(&input).unwrap();
        let anchor = sample_v3_anchor();
        let embedded = embed_v3_low_impact_image_into_dynamic(&img, &anchor).unwrap();
        let mut cursor = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(embedded)
            .write_to(&mut cursor, ImageFormat::Png)
            .unwrap();
        let reloaded = image::load_from_memory(&cursor.into_inner()).unwrap();

        let decoded = extract_v3_low_impact_image_from_dynamic(&reloaded).unwrap();

        assert!(decoded.is_v3_minimal_anchor());
        assert_eq!(decoded.watermark_uid(), anchor.watermark_uid());
    }

    #[test]
    fn image_v3_low_impact_lane_roundtrips_varied_anchors() {
        let input = make_png_bytes();
        let img = image::load_from_memory(&input).unwrap();
        for seed in 0u8..32 {
            let mut watermark_id = [0u8; 16];
            for (index, byte) in watermark_id.iter_mut().enumerate() {
                *byte = seed.wrapping_mul(17).wrapping_add(index as u8);
            }
            let anchor =
                WatermarkPayloadV3MinimalAnchor::new(crate::PayloadV3MinimalAnchorBuildInput {
                    watermark_id,
                })
                .unwrap();
            let embedded = embed_v3_low_impact_image_into_dynamic(&img, &anchor).unwrap();
            let mut cursor = Cursor::new(Vec::new());
            DynamicImage::ImageRgba8(embedded)
                .write_to(&mut cursor, ImageFormat::Png)
                .unwrap();
            let reloaded = image::load_from_memory(&cursor.into_inner()).unwrap();

            let decoded = extract_v3_low_impact_image_from_dynamic(&reloaded).unwrap();

            assert_eq!(decoded.watermark_uid(), anchor.watermark_uid());
        }
    }

    #[test]
    fn image_embed_capacity_uses_block_capacity_not_fixed_resolution() {
        assert!(!image_embed_capacity_sufficient(320, 240));
        assert!(image_embed_capacity_sufficient(1920, 1080));
        assert!(image_embed_capacity_sufficient(320, 600));
    }

    #[test]
    fn image_protection_input_enforces_capacity_and_pixel_boundary() {
        assert_eq!(
            validate_image_protection_input(320, 240),
            Err("image_capacity_insufficient")
        );
        assert_eq!(
            validate_image_protection_input(10_001, 10_000),
            Err("image_pixel_limit_exceeded")
        );
        assert!(validate_image_protection_input(9_992, 10_000).is_ok());
    }

    #[test]
    fn image_protection_file_size_enforces_512_mib_boundary() {
        assert!(validate_image_protection_file_size(MAX_IMAGE_PROTECTION_BYTES).is_ok());
        assert_eq!(
            validate_image_protection_file_size(MAX_IMAGE_PROTECTION_BYTES + 1),
            Err("image_file_size_limit_exceeded")
        );
    }
}

fn embed_bits_dct_svd_at_anchor(
    ll: &mut [f64],
    ll_w: usize,
    blocks_x: usize,
    blocks_y: usize,
    bits: &[bool],
    alpha: f64,
    anchor_x: usize,
    anchor_y: usize,
) {
    let mut bit_idx = 0;
    for by in anchor_y..blocks_y {
        for bx in anchor_x..blocks_x {
            if bit_idx >= bits.len() {
                return;
            }
            embed_bit_dct_svd_block(ll, ll_w, bx, by, bits[bit_idx], alpha);
            bit_idx += 1;
        }
    }
}

fn embed_bits_dct_svd_v3_at_anchor(
    ll: &mut [f64],
    ll_w: usize,
    blocks_x: usize,
    blocks_y: usize,
    bits: &[bool],
    alpha: f64,
    anchor_x: usize,
    anchor_y: usize,
) {
    let mut bit_idx = 0;
    for by in anchor_y..blocks_y {
        for bx in anchor_x..blocks_x {
            if bit_idx >= bits.len() {
                return;
            }
            embed_bit_dct_svd_block_v3_light(ll, ll_w, bx, by, bits[bit_idx], alpha);
            bit_idx += 1;
        }
    }
}

fn embed_bits_dct_svd_periodic(
    ll: &mut [f64],
    ll_w: usize,
    blocks_x: usize,
    blocks_y: usize,
    bits: &[bool],
    alpha: f64,
    anchor_x: usize,
    anchor_y: usize,
) {
    if bits.is_empty() {
        return;
    }
    let mut bit_idx = 0usize;
    for by in anchor_y..blocks_y {
        for bx in anchor_x..blocks_x {
            embed_bit_dct_svd_block(ll, ll_w, bx, by, bits[bit_idx % bits.len()], alpha);
            bit_idx += 1;
        }
    }
}

fn embed_bit_dct_svd_block(
    ll: &mut [f64],
    ll_w: usize,
    bx: usize,
    by: usize,
    bit: bool,
    alpha: f64,
) {
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

    sigma[0] = quantize_embed(sigma[0], bit, alpha);
    if sigma.len() > 1 {
        sigma[1] = quantize_embed(sigma[1], bit, secondary_image_alpha(alpha));
    }

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
            let idx = y * ll_w + x;
            let delta = spatial_block[(row, col)] - ll[idx];
            ll[idx] += delta.clamp(-IMAGE_V3_MAX_LL_DELTA, IMAGE_V3_MAX_LL_DELTA);
        }
    }
}

fn embed_bit_dct_svd_block_v3_light(
    ll: &mut [f64],
    ll_w: usize,
    bx: usize,
    by: usize,
    bit: bool,
    alpha: f64,
) {
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

    sigma[0] = quantize_embed(sigma[0], bit, alpha);

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
}

fn extract_bits_dct_svd_legacy_at_anchor(
    ll: &[f64],
    ll_w: usize,
    blocks_x: usize,
    blocks_y: usize,
    alpha: f64,
    anchor_x: usize,
    anchor_y: usize,
) -> Vec<bool> {
    let total_bits = PAYLOAD_BITS * REDUNDANCY;
    let mut raw_scores = Vec::with_capacity(total_bits);

    for by in anchor_y..blocks_y {
        for bx in anchor_x..blocks_x {
            if raw_scores.len() >= total_bits {
                return majority_bits(&raw_scores);
            }

            raw_scores.push(extract_bit_score_dct_svd_block(ll, ll_w, bx, by, alpha));
        }
    }

    majority_bits(&raw_scores)
}

fn extract_raw_bit_scores_dct_svd_at_anchor(
    ll: &[f64],
    ll_w: usize,
    blocks_x: usize,
    blocks_y: usize,
    alpha: f64,
    anchor_x: usize,
    anchor_y: usize,
    len: usize,
    skip: usize,
) -> Vec<i32> {
    let mut raw_scores = Vec::with_capacity(len);
    let mut skipped = 0usize;

    for by in anchor_y..blocks_y {
        for bx in anchor_x..blocks_x {
            if skipped < skip {
                skipped += 1;
                continue;
            }
            if raw_scores.len() >= len {
                return raw_scores;
            }

            raw_scores.push(extract_bit_score_dct_svd_block(ll, ll_w, bx, by, alpha));
        }
    }

    raw_scores
}

fn majority_bits(raw_scores: &[i32]) -> Vec<bool> {
    raw_scores
        .chunks(REDUNDANCY)
        .take(PAYLOAD_BITS)
        .map(|chunk| chunk.iter().sum::<i32>() > 0)
        .collect()
}

fn extract_bit_score_dct_svd_block(
    ll: &[f64],
    ll_w: usize,
    bx: usize,
    by: usize,
    alpha: f64,
) -> i32 {
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
    let svd = SVD::new(dmat, false, false);
    let primary = bit_score(
        quantize_extract(svd.singular_values[0], alpha),
        IMAGE_PRIMARY_SINGULAR_WEIGHT,
    );
    let secondary = if svd.singular_values.len() > 1 {
        bit_score(
            quantize_extract(svd.singular_values[1], secondary_image_alpha(alpha)),
            IMAGE_SECONDARY_SINGULAR_WEIGHT,
        )
    } else {
        0
    };
    primary + secondary
}

fn bit_score(bit: bool, weight: i32) -> i32 {
    if bit {
        weight
    } else {
        -weight
    }
}

fn secondary_image_alpha(alpha: f64) -> f64 {
    (alpha * IMAGE_SECONDARY_SINGULAR_ALPHA_RATIO).max(1.0)
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
            let cu = if u == 0 {
                1.0 / n.sqrt()
            } else {
                (2.0 / n).sqrt()
            };
            let cv = if v == 0 {
                1.0 / n.sqrt()
            } else {
                (2.0 / n).sqrt()
            };
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
                    let cu = if u == 0 {
                        1.0 / n.sqrt()
                    } else {
                        (2.0 / n).sqrt()
                    };
                    let cv = if v == 0 {
                        1.0 / n.sqrt()
                    } else {
                        (2.0 / n).sqrt()
                    };
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

fn apply_luma_delta_to_rgba(
    source: &DynamicImage,
    original_y_ch: &[f64],
    watermarked_y_ch: &[f64],
    w: u32,
    h: u32,
) -> RgbaImage {
    let mut img = RgbaImage::new(w, h);

    for py in 0..h {
        for px in 0..w {
            let idx = (py * w + px) as usize;
            let delta = watermarked_y_ch[idx] - original_y_ch[idx];
            let pixel = source.get_pixel(px, py);
            let r = (f64::from(pixel[0]) + delta).round().clamp(0.0, 255.0) as u8;
            let g = (f64::from(pixel[1]) + delta).round().clamp(0.0, 255.0) as u8;
            let b = (f64::from(pixel[2]) + delta).round().clamp(0.0, 255.0) as u8;
            img.put_pixel(px, py, Rgba([r, g, b, pixel[3]]));
        }
    }

    img
}

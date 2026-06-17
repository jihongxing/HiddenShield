use std::io::Cursor;
use std::path::Path;

use image::{DynamicImage, GenericImageView, ImageFormat, Rgba, RgbaImage};
use nalgebra::{DMatrix, Matrix4, SVD};

use crate::error::WatermarkError;
use crate::payload::{
    bits_to_bytes, bytes_to_bits, decode_payload, encode_payload, WatermarkPayload,
};

pub const DEFAULT_IMAGE_ALPHA: f64 = 50.0;
pub const BALANCED_IMAGE_ALPHA: f64 = 36.0;
const KNOWN_IMAGE_ALPHAS: [f64; 2] = [DEFAULT_IMAGE_ALPHA, BALANCED_IMAGE_ALPHA];
const IMAGE_SCALE_CANDIDATES: [f64; 3] = [1.0, 1.0416666666666667, 1.1764705882352942];
const IMAGE_PADDING_CANDIDATES: [f64; 5] = [0.0, 0.015, 0.02, 0.025, 0.03];
const IMAGE_SYNC_PADDING_CANDIDATES: [f64; 4] = [0.02, 0.0, 0.015, 0.025];
const IMAGE_SYNC_PREAMBLE: [u8; 4] = [0xA7, 0x5C, 0x3D, 0xE2];
const IMAGE_SYNC_CHECKSUM_BYTES: usize = 2;
const IMAGE_SYNC_PACKET_BYTES: usize = 4 + 32 + IMAGE_SYNC_CHECKSUM_BYTES;
const IMAGE_SYNC_PACKET_BITS: usize = IMAGE_SYNC_PACKET_BYTES * 8;
const IMAGE_SYNC_PREAMBLE_BITS: usize = IMAGE_SYNC_PREAMBLE.len() * 8;
const IMAGE_SYNC_MAX_COPIES: usize = 3;
const IMAGE_SYNC_SEARCH_JITTER_ROWS: isize = 1;
const PAYLOAD_BITS: usize = 32 * 8;
const BLOCK_SIZE: usize = 4;
const REDUNDANCY: usize = 3;

struct PreparedImageCandidate {
    half_w: usize,
    blocks_x: usize,
    blocks_y: usize,
    ll: Vec<f64>,
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

fn reject_existing_image_watermark(image_bytes: &[u8]) -> Result<(), WatermarkError> {
    let img = image::load_from_memory(image_bytes)
        .map_err(|e| WatermarkError::EmbedFailed(format!("failed to open image: {e}")))?;
    for alpha in KNOWN_IMAGE_ALPHAS {
        if let Ok(payload) = extract_image_from_dynamic_fast(&img, alpha) {
            return Err(WatermarkError::AlreadyWatermarked {
                existing_uid: payload.watermark_uid(),
            });
        }
    }
    Ok(())
}

fn extract_image_from_dynamic_candidates(
    img: &DynamicImage,
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    let (w, h) = img.dimensions();

    if let Ok(payload) = extract_image_from_dynamic_fast(img, alpha) {
        return Ok(payload);
    }

    let likely_crop_candidate = image_padding_candidate_for_ratio(img, 0.02);
    if let Ok(prepared) = prepare_image_candidate(&likely_crop_candidate) {
        if let Ok(payload) = extract_image_sync_packet_from_prepared(&prepared, alpha) {
            return Ok(payload);
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
        }
    }

    for candidate in image_sync_padding_candidates(img) {
        if candidate.dimensions() == likely_crop_candidate.dimensions() {
            continue;
        }
        if let Ok(payload) = extract_image_sync_packet_from_dynamic(&candidate, alpha) {
            return Ok(payload);
        }
    }

    Err(WatermarkError::ExtractFailed(
        "image extraction failed".into(),
    ))
}

fn image_padding_candidates(img: &DynamicImage) -> Vec<DynamicImage> {
    image_padding_candidates_for(img, &IMAGE_PADDING_CANDIDATES)
}

fn image_sync_padding_candidates(img: &DynamicImage) -> Vec<DynamicImage> {
    image_padding_candidates_for(img, &IMAGE_SYNC_PADDING_CANDIDATES)
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
    embed_bits_dct_svd_at_anchor(
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
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&payload_bytes[..32]);
    decode_payload(&arr)
}

fn extract_image_sync_packet_from_dynamic(
    img: &DynamicImage,
    alpha: f64,
) -> Result<WatermarkPayload, WatermarkError> {
    let prepared = prepare_image_candidate(img)?;
    extract_image_sync_packet_from_prepared(&prepared, alpha)
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
    for start_block in image_sync_search_starts(candidate.blocks_x, candidate.blocks_y) {
        let preamble_bits = extract_raw_bits_dct_svd_range(
            &candidate.ll,
            candidate.half_w,
            candidate.blocks_x,
            candidate.blocks_y,
            alpha,
            start_block,
            IMAGE_SYNC_PREAMBLE_BITS * REDUNDANCY,
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
            start_block,
            preamble_bits,
        );
        match decode_image_sync_packet_from_raw_bits(&bits) {
            Ok(payload) => return Ok(payload),
            Err(error) => last_error = Some(error),
        }
    }

    Err(last_error.unwrap_or_else(|| WatermarkError::ExtractFailed("sync packet not found".into())))
}

fn image_sync_preamble_matches(raw_bits: &[bool]) -> bool {
    if raw_bits.len() < IMAGE_SYNC_PREAMBLE_BITS * REDUNDANCY {
        return false;
    }
    let bits = majority_bits_for_window(raw_bits, 0, IMAGE_SYNC_PREAMBLE_BITS);
    let bytes = bits_to_bytes(&bits);
    bytes.get(..IMAGE_SYNC_PREAMBLE.len()) == Some(&IMAGE_SYNC_PREAMBLE)
}

fn extract_image_sync_packet_bits_after_preamble(
    candidate: &PreparedImageCandidate,
    alpha: f64,
    start_block: usize,
    preamble_bits: Vec<bool>,
) -> Vec<bool> {
    let preamble_blocks = IMAGE_SYNC_PREAMBLE_BITS * REDUNDANCY;
    let packet_blocks = IMAGE_SYNC_PACKET_BITS * REDUNDANCY;
    let remaining_blocks = packet_blocks.saturating_sub(preamble_blocks);
    let mut bits = preamble_bits;
    bits.extend(extract_raw_bits_dct_svd_range(
        &candidate.ll,
        candidate.half_w,
        candidate.blocks_x,
        candidate.blocks_y,
        alpha,
        start_block + preamble_blocks,
        remaining_blocks,
    ));
    bits
}

fn image_sync_embed_anchors(blocks_x: usize, blocks_y: usize) -> Vec<(usize, usize)> {
    let sync_blocks = IMAGE_SYNC_PACKET_BITS * REDUNDANCY;
    if blocks_x * blocks_y < PAYLOAD_BITS * REDUNDANCY + sync_blocks {
        return Vec::new();
    }

    let legacy_rows = (PAYLOAD_BITS * REDUNDANCY).div_ceil(blocks_x);
    let sync_rows = sync_blocks.div_ceil(blocks_x);
    let mut anchors = Vec::new();
    let mut anchor_y = legacy_rows + 2;
    while anchors.len() < IMAGE_SYNC_MAX_COPIES && anchor_y + sync_rows <= blocks_y {
        anchors.push((0, anchor_y));
        anchor_y += sync_rows + 2;
    }
    anchors
}

fn image_sync_search_starts(blocks_x: usize, blocks_y: usize) -> Vec<usize> {
    let mut starts = Vec::new();
    for (anchor_x, anchor_y) in image_sync_embed_anchors(blocks_x, blocks_y) {
        for dy in -IMAGE_SYNC_SEARCH_JITTER_ROWS..=IMAGE_SYNC_SEARCH_JITTER_ROWS {
            let Some(search_y) = anchor_y.checked_add_signed(dy) else {
                continue;
            };
            if search_y >= blocks_y {
                continue;
            }
            let start = search_y * blocks_x + anchor_x;
            if !starts.contains(&start) {
                starts.push(start);
            }
        }
    }
    starts
}

fn encode_image_sync_packet(payload: &WatermarkPayload) -> [u8; IMAGE_SYNC_PACKET_BYTES] {
    let payload_bytes = encode_payload(payload);
    let checksum = image_sync_checksum(&payload_bytes);
    let mut packet = [0u8; IMAGE_SYNC_PACKET_BYTES];
    packet[0..4].copy_from_slice(&IMAGE_SYNC_PREAMBLE);
    packet[4..36].copy_from_slice(&payload_bytes);
    packet[36..38].copy_from_slice(&checksum);
    packet
}

fn decode_image_sync_packet_from_raw_bits(
    raw_bits: &[bool],
) -> Result<WatermarkPayload, WatermarkError> {
    let packet_blocks = IMAGE_SYNC_PACKET_BITS * REDUNDANCY;
    if raw_bits.len() < packet_blocks {
        return Err(WatermarkError::ExtractFailed(
            "not enough data for sync packet extraction".into(),
        ));
    }

    let bits = majority_bits_for_window(raw_bits, 0, IMAGE_SYNC_PACKET_BITS);
    let bytes = bits_to_bytes(&bits);
    decode_image_sync_packet_bytes(&bytes)
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

    let mut payload_bytes = [0u8; 32];
    payload_bytes.copy_from_slice(&bytes[4..36]);
    let expected_checksum = image_sync_checksum(&payload_bytes);
    if bytes[36..38] != expected_checksum {
        return Err(WatermarkError::ExtractFailed(
            "sync packet checksum mismatch".into(),
        ));
    }

    decode_payload(&payload_bytes)
}

fn image_sync_checksum(payload_bytes: &[u8; 32]) -> [u8; IMAGE_SYNC_CHECKSUM_BYTES] {
    let mut state = 0xA5C3u16;
    for &byte in payload_bytes {
        state = state.rotate_left(5) ^ byte as u16;
        state = state.wrapping_mul(251);
    }
    state.to_be_bytes()
}

fn majority_bits_for_window(raw_bits: &[bool], start: usize, bit_count: usize) -> Vec<bool> {
    (0..bit_count)
        .map(|bit_idx| {
            let bit_start = start + bit_idx * REDUNDANCY;
            let chunk = &raw_bits[bit_start..bit_start + REDUNDANCY];
            let ones = chunk.iter().filter(|&&bit| bit).count();
            ones > chunk.len() / 2
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

    #[test]
    fn image_bytes_rejects_existing_watermark_by_default() {
        let input = make_png_bytes();
        let payload = sample_payload();
        let embedded = embed_image_watermark_bytes(&input, &payload, ImageFormat::Png).unwrap();
        let err = embed_image_watermark_bytes(&embedded, &second_payload(), ImageFormat::Png)
            .unwrap_err();

        assert!(matches!(
            err,
            WatermarkError::AlreadyWatermarked { existing_uid }
                if existing_uid == payload.watermark_uid()
        ));
    }

    #[test]
    fn image_bytes_allow_rewrite_replaces_existing_watermark() {
        let input = make_png_bytes();
        let payload = sample_payload();
        let second = second_payload();
        let embedded = embed_image_watermark_bytes(&input, &payload, ImageFormat::Png).unwrap();
        let rewritten =
            embed_image_watermark_bytes_allow_rewrite(&embedded, &second, ImageFormat::Png)
                .unwrap();
        let extracted = extract_image_watermark_bytes(&rewritten).unwrap();

        assert_eq!(extracted.watermark_uid(), second.watermark_uid());
    }

    #[test]
    fn image_bytes_extracts_after_resize_85_percent() {
        let input = make_png_bytes();
        let payload = sample_payload();
        let embedded = embed_image_watermark_bytes(&input, &payload, ImageFormat::Png).unwrap();
        let img = image::load_from_memory(&embedded).unwrap();
        let resized = img.resize(
            ((img.width() as f32) * 0.85).round() as u32,
            ((img.height() as f32) * 0.85).round() as u32,
            image::imageops::FilterType::Lanczos3,
        );
        let mut cursor = Cursor::new(Vec::new());
        resized.write_to(&mut cursor, ImageFormat::Png).unwrap();
        let extracted = extract_image_watermark_bytes(&cursor.into_inner()).unwrap();

        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
    }

    #[test]
    fn image_bytes_extracts_after_crop_2_percent() {
        let input = make_png_bytes();
        let payload = sample_payload();
        let embedded = embed_image_watermark_bytes(&input, &payload, ImageFormat::Png).unwrap();
        let img = image::load_from_memory(&embedded).unwrap();
        let crop_x = (img.width() / 50).max(1);
        let crop_y = (img.height() / 50).max(1);
        let cropped = img.crop_imm(
            crop_x,
            crop_y,
            img.width() - crop_x * 2,
            img.height() - crop_y * 2,
        );
        let mut cursor = Cursor::new(Vec::new());
        cropped.write_to(&mut cursor, ImageFormat::Png).unwrap();
        let extracted = extract_image_watermark_bytes(&cursor.into_inner()).unwrap();

        assert_eq!(extracted.watermark_uid(), payload.watermark_uid());
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

            sigma[0] = quantize_embed(sigma[0], bits[bit_idx], alpha);

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
    let mut raw_bits = Vec::with_capacity(total_bits);

    for by in anchor_y..blocks_y {
        for bx in anchor_x..blocks_x {
            if raw_bits.len() >= total_bits {
                return majority_bits(&raw_bits);
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
            raw_bits.push(quantize_extract(s0, alpha));
        }
    }

    majority_bits(&raw_bits)
}

fn extract_raw_bits_dct_svd_range(
    ll: &[f64],
    ll_w: usize,
    blocks_x: usize,
    blocks_y: usize,
    alpha: f64,
    start_block: usize,
    len: usize,
) -> Vec<bool> {
    let total_blocks = blocks_x * blocks_y;
    if start_block >= total_blocks {
        return Vec::new();
    }
    let end_block = (start_block + len).min(total_blocks);
    let mut raw_bits = Vec::with_capacity(end_block - start_block);

    for block_idx in start_block..end_block {
        let by = block_idx / blocks_x;
        let bx = block_idx % blocks_x;

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
        raw_bits.push(quantize_extract(s0, alpha));
    }

    raw_bits
}

fn majority_bits(raw_bits: &[bool]) -> Vec<bool> {
    raw_bits
        .chunks(REDUNDANCY)
        .take(PAYLOAD_BITS)
        .map(|chunk| {
            let ones = chunk.iter().filter(|&&bit| bit).count();
            ones > chunk.len() / 2
        })
        .collect()
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

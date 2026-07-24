use std::collections::VecDeque;

use image::{Rgba, RgbaImage};
use serde::Serialize;

use crate::error::WatermarkError;
use crate::payload::{PayloadV3MinimalAnchorBuildInput, WatermarkPayloadV3MinimalAnchor};

pub const SPATIAL_RECOVERY_V1_LAYOUT_ID: u8 = 1;
pub const SPATIAL_RECOVERY_V1_GRID_DIVISOR: u32 = 4;
pub const SPATIAL_RECOVERY_V1_PACKET_WIDTH: u32 = 32;
pub const SPATIAL_RECOVERY_V1_PACKET_HEIGHT: u32 = 35;

const SPATIAL_RECOVERY_V1_MAGIC: [u8; 4] = *b"HSR1";
const SPATIAL_RECOVERY_V1_WATERMARK_ID_BYTES: usize = 16;
const SPATIAL_RECOVERY_V1_PACKET_BYTES: usize =
    SPATIAL_RECOVERY_V1_MAGIC.len() + 1 + SPATIAL_RECOVERY_V1_WATERMARK_ID_BYTES + 1;
const SPATIAL_RECOVERY_V1_PACKET_BITS: usize = SPATIAL_RECOVERY_V1_PACKET_BYTES * 8;
const SPATIAL_RECOVERY_V1_MAGIC_BITS: usize = SPATIAL_RECOVERY_V1_MAGIC.len() * 8;
const SPATIAL_RECOVERY_V1_PERMUTED_BITS: usize =
    SPATIAL_RECOVERY_V1_PACKET_BITS - SPATIAL_RECOVERY_V1_MAGIC_BITS;
const SPATIAL_RECOVERY_V1_PACKET_VARIANTS: usize = 25;
const SPATIAL_RECOVERY_V1_BIT_BLOCK_WIDTH: u32 = 2;
const SPATIAL_RECOVERY_V1_BIT_BLOCK_HEIGHT: u32 = 3;
const SPATIAL_RECOVERY_V1_BIT_COLUMNS: usize =
    (SPATIAL_RECOVERY_V1_PACKET_WIDTH / SPATIAL_RECOVERY_V1_BIT_BLOCK_WIDTH) as usize;
const SPATIAL_RECOVERY_V1_HAAR_TARGET: i32 = 20;
const SPATIAL_RECOVERY_V1_MAGIC_MAX_ERRORS: usize = 2;
const SPATIAL_RECOVERY_V1_SOFT_CORRECTION_CANDIDATES: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialRecoveryRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl SpatialRecoveryRect {
    fn contains(self, other: Self) -> bool {
        other.x >= self.x
            && other.y >= self.y
            && other.x.saturating_add(other.width) <= self.x.saturating_add(self.width)
            && other.y.saturating_add(other.height) <= self.y.saturating_add(self.height)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialRecoveryV1Layout {
    pub layout_id: u8,
    pub source_width: u32,
    pub source_height: u32,
    pub minimum_crop_width: u32,
    pub minimum_crop_height: u32,
    pub packet_width: u32,
    pub packet_height: u32,
    pub horizontal_packet_starts: Vec<u32>,
    pub vertical_packet_starts: Vec<u32>,
    pub packet_rects: Vec<SpatialRecoveryRect>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialRecoveryAxisCoverage {
    pub full_length: u32,
    pub crop_length: u32,
    pub packet_length: u32,
    pub packet_starts: Vec<u32>,
    pub tested_crop_starts: u32,
    pub uncovered_crop_starts: u32,
    pub first_uncovered_crop_start: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialRecoveryV1CoverageSimulation {
    pub layout: SpatialRecoveryV1Layout,
    pub horizontal: SpatialRecoveryAxisCoverage,
    pub vertical: SpatialRecoveryAxisCoverage,
    pub every_quarter_by_quarter_crop_contains_packet: bool,
    pub exact_grid_crop_count: usize,
    pub exact_grid_uncovered_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialRecoveryV1PacketDiagnostic {
    pub packet_variant: usize,
    pub packet_rect: SpatialRecoveryRect,
    pub magic_errors: usize,
    pub checksum_valid: bool,
    pub recovered_uid: Option<String>,
    pub differing_packet_bits: Vec<usize>,
    pub differing_uid_bits: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialRecoveryV1CandidateDiagnostic {
    pub recovered_uid: String,
    pub differing_packet_bits: Vec<usize>,
    pub differing_uid_bits: Vec<usize>,
    pub corrected_packet_bits: Vec<usize>,
    pub confidence_cost: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SpatialRecoveryV1ExactDiagnostic {
    pub selected_stage: Option<String>,
    pub selected_packet_variant: Option<usize>,
    pub selected_uid: Option<String>,
    pub packets: Vec<SpatialRecoveryV1PacketDiagnostic>,
    pub consensus: Option<SpatialRecoveryV1CandidateDiagnostic>,
    pub soft_correction: Option<SpatialRecoveryV1CandidateDiagnostic>,
    pub consensus_vote_sums: Vec<i64>,
}

pub fn derive_spatial_recovery_v1_layout(
    width: u32,
    height: u32,
) -> Result<SpatialRecoveryV1Layout, WatermarkError> {
    let minimum_crop_width = width / SPATIAL_RECOVERY_V1_GRID_DIVISOR;
    let minimum_crop_height = height / SPATIAL_RECOVERY_V1_GRID_DIVISOR;
    if minimum_crop_width < SPATIAL_RECOVERY_V1_PACKET_WIDTH
        || minimum_crop_height < SPATIAL_RECOVERY_V1_PACKET_HEIGHT
    {
        return Err(WatermarkError::EmbedFailed(format!(
            "spatial-recovery-v1 requires each 1/16 crop to be at least {}x{} pixels, got {}x{}",
            SPATIAL_RECOVERY_V1_PACKET_WIDTH,
            SPATIAL_RECOVERY_V1_PACKET_HEIGHT,
            minimum_crop_width,
            minimum_crop_height
        )));
    }

    let horizontal_packet_starts =
        derive_axis_packet_starts(width, minimum_crop_width, SPATIAL_RECOVERY_V1_PACKET_WIDTH);
    let vertical_packet_starts = derive_axis_packet_starts(
        height,
        minimum_crop_height,
        SPATIAL_RECOVERY_V1_PACKET_HEIGHT,
    );
    let packet_rects = vertical_packet_starts
        .iter()
        .flat_map(|&y| {
            horizontal_packet_starts
                .iter()
                .map(move |&x| SpatialRecoveryRect {
                    x,
                    y,
                    width: SPATIAL_RECOVERY_V1_PACKET_WIDTH,
                    height: SPATIAL_RECOVERY_V1_PACKET_HEIGHT,
                })
        })
        .collect();

    Ok(SpatialRecoveryV1Layout {
        layout_id: SPATIAL_RECOVERY_V1_LAYOUT_ID,
        source_width: width,
        source_height: height,
        minimum_crop_width,
        minimum_crop_height,
        packet_width: SPATIAL_RECOVERY_V1_PACKET_WIDTH,
        packet_height: SPATIAL_RECOVERY_V1_PACKET_HEIGHT,
        horizontal_packet_starts,
        vertical_packet_starts,
        packet_rects,
    })
}

pub fn embed_spatial_recovery_v1(
    source: &RgbaImage,
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> Result<(RgbaImage, SpatialRecoveryV1Layout), WatermarkError> {
    let layout = derive_spatial_recovery_v1_layout(source.width(), source.height())?;
    let packet = encode_spatial_recovery_v1_packet(anchor);
    let bits = bytes_to_bits(&packet);
    let mut output = source.clone();

    for (packet_variant, packet_rect) in layout.packet_rects.iter().copied().enumerate() {
        write_packet_at(&mut output, packet_rect, &bits, packet_variant);
    }

    Ok((output, layout))
}

pub fn extract_spatial_recovery_v1(
    suspect: &RgbaImage,
) -> Result<WatermarkPayloadV3MinimalAnchor, WatermarkError> {
    if suspect.width() < SPATIAL_RECOVERY_V1_PACKET_WIDTH
        || suspect.height() < SPATIAL_RECOVERY_V1_PACKET_HEIGHT
    {
        return Err(WatermarkError::ExtractFailed(
            "spatial-recovery-v1 suspect is smaller than one recovery packet".into(),
        ));
    }

    let maximum_y = suspect.height() - SPATIAL_RECOVERY_V1_PACKET_HEIGHT;
    let magic_high =
        u16::from_be_bytes([SPATIAL_RECOVERY_V1_MAGIC[0], SPATIAL_RECOVERY_V1_MAGIC[1]]);
    let magic_low =
        u16::from_be_bytes([SPATIAL_RECOVERY_V1_MAGIC[2], SPATIAL_RECOVERY_V1_MAGIC[3]]);
    let mut signature_rows = (0..4)
        .map(|bit_row_y| local_haar_signature_row(suspect, bit_row_y))
        .collect::<VecDeque<_>>();

    for y in 0..=maximum_y {
        let first_magic_row = &signature_rows[0];
        let second_magic_row = &signature_rows[3];
        for x in 0..first_magic_row.len() {
            let magic_errors = (first_magic_row[x] ^ magic_high).count_ones()
                + (second_magic_row[x] ^ magic_low).count_ones();
            if magic_errors > SPATIAL_RECOVERY_V1_MAGIC_MAX_ERRORS as u32 {
                continue;
            }
            let packet_rect = SpatialRecoveryRect {
                x: x as u32,
                y,
                width: SPATIAL_RECOVERY_V1_PACKET_WIDTH,
                height: SPATIAL_RECOVERY_V1_PACKET_HEIGHT,
            };
            for packet_variant in 0..SPATIAL_RECOVERY_V1_PACKET_VARIANTS {
                let packet = read_packet_at(suspect, packet_rect, packet_variant);
                if let Ok(anchor) = decode_spatial_recovery_v1_packet(&packet) {
                    return Ok(anchor);
                }
            }
        }
        if y < maximum_y {
            signature_rows.pop_front();
            signature_rows.push_back(local_haar_signature_row(suspect, y + 4));
        }
    }

    Err(WatermarkError::ExtractFailed(
        "spatial-recovery-v1 packet not found".into(),
    ))
}

fn local_haar_signature_row(image: &RgbaImage, bit_row_y: u32) -> Vec<u16> {
    let candidate_count = (image.width() - SPATIAL_RECOVERY_V1_PACKET_WIDTH + 1) as usize;
    let coefficient_count = image.width().saturating_sub(1) as usize;
    let mut vertical_luma_sums = vec![0i32; image.width() as usize];
    for row in 0..SPATIAL_RECOVERY_V1_BIT_BLOCK_HEIGHT {
        for x in 0..image.width() {
            vertical_luma_sums[x as usize] += pixel_luma(image.get_pixel(x, bit_row_y + row));
        }
    }
    let coefficient_bits = (0..coefficient_count)
        .map(|x| vertical_luma_sums[x] > vertical_luma_sums[x + 1])
        .collect::<Vec<_>>();
    let mut signatures = vec![0u16; candidate_count];

    for parity in 0..SPATIAL_RECOVERY_V1_BIT_BLOCK_WIDTH as usize {
        if parity >= candidate_count {
            break;
        }
        let mut signature = 0u16;
        for bit_column in 0..SPATIAL_RECOVERY_V1_BIT_COLUMNS {
            signature = (signature << 1) | u16::from(coefficient_bits[parity + bit_column * 2]);
        }
        signatures[parity] = signature;

        let mut x = parity + SPATIAL_RECOVERY_V1_BIT_BLOCK_WIDTH as usize;
        while x < candidate_count {
            let next_bit_x = x + (SPATIAL_RECOVERY_V1_BIT_COLUMNS - 1) * 2;
            signature = (signature << 1) | u16::from(coefficient_bits[next_bit_x]);
            signatures[x] = signature;
            x += SPATIAL_RECOVERY_V1_BIT_BLOCK_WIDTH as usize;
        }
    }

    signatures
}

pub fn extract_spatial_recovery_v1_exact(
    suspect: &RgbaImage,
) -> Result<WatermarkPayloadV3MinimalAnchor, WatermarkError> {
    let layout = derive_spatial_recovery_v1_layout(suspect.width(), suspect.height())?;
    let vote_sums = exact_vote_sums(suspect, &layout);
    let consensus_bits = vote_sums.iter().map(|vote| *vote > 0).collect::<Vec<_>>();
    let consensus_packet = bits_to_bytes(&consensus_bits);
    if let Ok(anchor) = decode_spatial_recovery_v1_packet(&consensus_packet) {
        return Ok(anchor);
    }
    if let Ok(anchor) =
        decode_spatial_recovery_v1_consensus_with_soft_correction(&consensus_bits, &vote_sums)
    {
        return Ok(anchor);
    }

    for (packet_variant, packet_rect) in layout.packet_rects.iter().copied().enumerate() {
        if !packet_magic_matches(suspect, packet_rect) {
            continue;
        }
        let packet = read_packet_at(suspect, packet_rect, packet_variant);
        if let Ok(anchor) = decode_spatial_recovery_v1_packet(&packet) {
            return Ok(anchor);
        }
    }
    Err(WatermarkError::ExtractFailed(
        "spatial-recovery-v1 exact packet not found".into(),
    ))
}

pub fn diagnose_spatial_recovery_v1_exact(
    suspect: &RgbaImage,
    expected: &WatermarkPayloadV3MinimalAnchor,
) -> Result<SpatialRecoveryV1ExactDiagnostic, WatermarkError> {
    let layout = derive_spatial_recovery_v1_layout(suspect.width(), suspect.height())?;
    let expected_packet = encode_spatial_recovery_v1_packet(expected);
    let expected_bits = bytes_to_bits(&expected_packet);
    let mut packets = Vec::with_capacity(layout.packet_rects.len());

    for (packet_variant, packet_rect) in layout.packet_rects.iter().copied().enumerate() {
        let packet = read_packet_at(suspect, packet_rect, packet_variant);
        let packet_bits = bytes_to_bits(&packet);
        let differing_packet_bits = differing_bit_indices(&expected_bits, &packet_bits);
        let decoded = decode_spatial_recovery_v1_packet(&packet).ok();
        packets.push(SpatialRecoveryV1PacketDiagnostic {
            packet_variant,
            packet_rect,
            magic_errors: differing_packet_bits
                .iter()
                .filter(|bit_index| **bit_index < SPATIAL_RECOVERY_V1_MAGIC_BITS)
                .count(),
            checksum_valid: decoded.is_some(),
            recovered_uid: decoded.as_ref().map(|anchor| anchor.watermark_uid()),
            differing_uid_bits: uid_bit_indices(&differing_packet_bits),
            differing_packet_bits,
        });
    }

    let vote_sums = exact_vote_sums(suspect, &layout);
    let consensus_bits = vote_sums.iter().map(|vote| *vote > 0).collect::<Vec<_>>();
    let consensus_packet = bits_to_bytes(&consensus_bits);
    let consensus = decode_spatial_recovery_v1_packet(&consensus_packet)
        .ok()
        .map(|anchor| candidate_diagnostic(anchor, &expected_bits, &consensus_bits, Vec::new(), 0));

    let soft_correction = decode_spatial_recovery_v1_consensus_with_soft_correction_details(
        &consensus_bits,
        &vote_sums,
    )
    .ok()
    .map(|(anchor, corrected_packet_bits, confidence_cost)| {
        let mut corrected_bits = consensus_bits.clone();
        for bit_index in &corrected_packet_bits {
            corrected_bits[*bit_index] = !corrected_bits[*bit_index];
        }
        candidate_diagnostic(
            anchor,
            &expected_bits,
            &corrected_bits,
            corrected_packet_bits,
            confidence_cost,
        )
    });
    let first_valid_packet = packets.iter().find(|packet| packet.checksum_valid);
    let (selected_stage, selected_packet_variant, selected_uid) =
        if let Some(candidate) = consensus.as_ref() {
            (
                Some("consensus".to_string()),
                None,
                Some(candidate.recovered_uid.clone()),
            )
        } else if let Some(candidate) = soft_correction.as_ref() {
            (
                Some("softCorrection".to_string()),
                None,
                Some(candidate.recovered_uid.clone()),
            )
        } else if let Some(packet) = first_valid_packet {
            (
                Some("individualPacket".to_string()),
                Some(packet.packet_variant),
                packet.recovered_uid.clone(),
            )
        } else {
            (None, None, None)
        };

    Ok(SpatialRecoveryV1ExactDiagnostic {
        selected_stage,
        selected_packet_variant,
        selected_uid,
        packets,
        consensus,
        soft_correction,
        consensus_vote_sums: vote_sums,
    })
}

fn exact_vote_sums(suspect: &RgbaImage, layout: &SpatialRecoveryV1Layout) -> Vec<i64> {
    (0..SPATIAL_RECOVERY_V1_PACKET_BITS)
        .map(|bit_index| {
            layout
                .packet_rects
                .iter()
                .copied()
                .enumerate()
                .map(|(packet_variant, packet_rect)| {
                    let physical_bit_index =
                        physical_bit_index_for_variant(bit_index, packet_variant);
                    i64::from(read_local_haar_coefficient(
                        suspect,
                        packet_rect,
                        physical_bit_index,
                    ))
                })
                .sum::<i64>()
        })
        .collect()
}

fn decode_spatial_recovery_v1_consensus_with_soft_correction(
    consensus_bits: &[bool],
    vote_sums: &[i64],
) -> Result<WatermarkPayloadV3MinimalAnchor, WatermarkError> {
    decode_spatial_recovery_v1_consensus_with_soft_correction_details(consensus_bits, vote_sums)
        .map(|(anchor, _, _)| anchor)
}

fn decode_spatial_recovery_v1_consensus_with_soft_correction_details(
    consensus_bits: &[bool],
    vote_sums: &[i64],
) -> Result<(WatermarkPayloadV3MinimalAnchor, Vec<usize>, i64), WatermarkError> {
    let mut corrected_bits = consensus_bits.to_vec();
    let magic_errors = corrected_bits
        .iter()
        .take(SPATIAL_RECOVERY_V1_MAGIC.len() * 8)
        .enumerate()
        .filter(|(bit_index, actual)| {
            let byte = SPATIAL_RECOVERY_V1_MAGIC[bit_index / 8];
            let expected = ((byte >> (7 - bit_index % 8)) & 1) == 1;
            **actual != expected
        })
        .count();
    if magic_errors > SPATIAL_RECOVERY_V1_MAGIC_MAX_ERRORS {
        return Err(WatermarkError::ExtractFailed(
            "spatial-recovery-v1 consensus magic mismatch".into(),
        ));
    }
    for bit_index in 0..SPATIAL_RECOVERY_V1_MAGIC.len() * 8 {
        let byte = SPATIAL_RECOVERY_V1_MAGIC[bit_index / 8];
        corrected_bits[bit_index] = ((byte >> (7 - bit_index % 8)) & 1) == 1;
    }

    let layout_start = SPATIAL_RECOVERY_V1_MAGIC.len() * 8;
    let layout_bits = bytes_to_bits(&[SPATIAL_RECOVERY_V1_LAYOUT_ID]);
    let layout_errors = layout_bits
        .iter()
        .enumerate()
        .filter(|(offset, expected)| corrected_bits[layout_start + offset] != **expected)
        .count();
    if layout_errors > 1 {
        return Err(WatermarkError::ExtractFailed(
            "spatial-recovery-v1 consensus layout mismatch".into(),
        ));
    }
    corrected_bits[layout_start..layout_start + 8].copy_from_slice(&layout_bits);

    let correction_start = layout_start + 8;
    let mut correction_candidates =
        (correction_start..SPATIAL_RECOVERY_V1_PACKET_BITS).collect::<Vec<_>>();
    correction_candidates.sort_by_key(|bit_index| vote_sums[*bit_index].abs());
    correction_candidates.truncate(SPATIAL_RECOVERY_V1_SOFT_CORRECTION_CANDIDATES);

    let mut decoded_candidates = Vec::new();
    for &first in &correction_candidates {
        corrected_bits[first] = !corrected_bits[first];
        let packet = bits_to_bytes(&corrected_bits);
        if let Ok(anchor) = decode_spatial_recovery_v1_packet(&packet) {
            decoded_candidates.push((anchor, vec![first], vote_sums[first].abs()));
        }
        corrected_bits[first] = !corrected_bits[first];
    }
    for first_index in 0..correction_candidates.len() {
        for second_index in first_index + 1..correction_candidates.len() {
            let first = correction_candidates[first_index];
            let second = correction_candidates[second_index];
            corrected_bits[first] = !corrected_bits[first];
            corrected_bits[second] = !corrected_bits[second];
            let packet = bits_to_bytes(&corrected_bits);
            if let Ok(anchor) = decode_spatial_recovery_v1_packet(&packet) {
                decoded_candidates.push((
                    anchor,
                    vec![first, second],
                    vote_sums[first].abs() + vote_sums[second].abs(),
                ));
            }
            corrected_bits[first] = !corrected_bits[first];
            corrected_bits[second] = !corrected_bits[second];
        }
    }
    decoded_candidates.sort_by_key(|(_, _, confidence_cost)| *confidence_cost);
    let Some((best, corrected_packet_bits, confidence_cost)) = decoded_candidates.first() else {
        return Err(WatermarkError::ExtractFailed(
            "spatial-recovery-v1 soft correction exhausted".into(),
        ));
    };
    if decoded_candidates
        .iter()
        .skip(1)
        .any(|(candidate, _, _)| candidate.watermark_id != best.watermark_id)
    {
        return Err(WatermarkError::ExtractFailed(
            "spatial-recovery-v1 soft correction is ambiguous".into(),
        ));
    }
    Ok((
        best.clone(),
        corrected_packet_bits.clone(),
        *confidence_cost,
    ))
}

fn candidate_diagnostic(
    anchor: WatermarkPayloadV3MinimalAnchor,
    expected_bits: &[bool],
    candidate_bits: &[bool],
    corrected_packet_bits: Vec<usize>,
    confidence_cost: i64,
) -> SpatialRecoveryV1CandidateDiagnostic {
    let differing_packet_bits = differing_bit_indices(expected_bits, candidate_bits);
    SpatialRecoveryV1CandidateDiagnostic {
        recovered_uid: anchor.watermark_uid(),
        differing_uid_bits: uid_bit_indices(&differing_packet_bits),
        differing_packet_bits,
        corrected_packet_bits,
        confidence_cost,
    }
}

fn differing_bit_indices(expected: &[bool], actual: &[bool]) -> Vec<usize> {
    expected
        .iter()
        .zip(actual)
        .enumerate()
        .filter_map(|(bit_index, (expected, actual))| (expected != actual).then_some(bit_index))
        .collect()
}

fn uid_bit_indices(packet_bit_indices: &[usize]) -> Vec<usize> {
    let uid_start = (SPATIAL_RECOVERY_V1_MAGIC.len() + 1) * 8;
    let uid_end = uid_start + SPATIAL_RECOVERY_V1_WATERMARK_ID_BYTES * 8;
    packet_bit_indices
        .iter()
        .filter_map(|bit_index| {
            (*bit_index >= uid_start && *bit_index < uid_end).then_some(*bit_index - uid_start)
        })
        .collect()
}

pub fn extract_spatial_recovery_v1_exact_scaled(
    suspect: &RgbaImage,
    source_scale: f64,
) -> Result<WatermarkPayloadV3MinimalAnchor, WatermarkError> {
    let consensus_bits = scaled_consensus_bits(suspect, source_scale)?;
    let consensus_packet = bits_to_bytes(&consensus_bits);
    decode_spatial_recovery_v1_packet(&consensus_packet)
}

pub fn spatial_recovery_v1_scaled_magic_errors(
    suspect: &RgbaImage,
    source_scale: f64,
) -> Result<u32, WatermarkError> {
    let consensus_bits = scaled_consensus_bits(suspect, source_scale)?;
    Ok(consensus_bits
        .iter()
        .take(SPATIAL_RECOVERY_V1_MAGIC.len() * 8)
        .enumerate()
        .filter(|(bit_index, actual)| {
            let byte = SPATIAL_RECOVERY_V1_MAGIC[bit_index / 8];
            let expected = ((byte >> (7 - bit_index % 8)) & 1) == 1;
            **actual != expected
        })
        .count() as u32)
}

fn scaled_consensus_bits(
    suspect: &RgbaImage,
    source_scale: f64,
) -> Result<Vec<bool>, WatermarkError> {
    if !(0.0..1.0).contains(&source_scale) {
        return Err(WatermarkError::ExtractFailed(
            "spatial-recovery-v1 scaled reader requires a downscale factor".into(),
        ));
    }
    let source_width = (f64::from(suspect.width()) / source_scale).round() as u32;
    let source_height = (f64::from(suspect.height()) / source_scale).round() as u32;
    let layout = derive_spatial_recovery_v1_layout(source_width, source_height)?;
    let consensus_bits = (0..SPATIAL_RECOVERY_V1_PACKET_BITS)
        .map(|bit_index| {
            layout
                .packet_rects
                .iter()
                .copied()
                .enumerate()
                .map(|(packet_variant, packet_rect)| {
                    let physical_bit_index =
                        physical_bit_index_for_variant(bit_index, packet_variant);
                    read_scaled_local_haar_coefficient(
                        suspect,
                        packet_rect,
                        physical_bit_index,
                        source_scale,
                    )
                })
                .sum::<f64>()
                > 0.0
        })
        .collect::<Vec<_>>();
    Ok(consensus_bits)
}

pub fn simulate_spatial_recovery_v1_coverage(
    width: u32,
    height: u32,
) -> Result<SpatialRecoveryV1CoverageSimulation, WatermarkError> {
    let layout = derive_spatial_recovery_v1_layout(width, height)?;
    let horizontal = simulate_axis_coverage(
        width,
        layout.minimum_crop_width,
        layout.packet_width,
        &layout.horizontal_packet_starts,
    );
    let vertical = simulate_axis_coverage(
        height,
        layout.minimum_crop_height,
        layout.packet_height,
        &layout.vertical_packet_starts,
    );
    let exact_grid_crops = exact_grid_crops(width, height);
    let exact_grid_uncovered_count = exact_grid_crops
        .iter()
        .filter(|crop| {
            !layout
                .packet_rects
                .iter()
                .copied()
                .any(|packet| crop.contains(packet))
        })
        .count();

    Ok(SpatialRecoveryV1CoverageSimulation {
        every_quarter_by_quarter_crop_contains_packet: horizontal.uncovered_crop_starts == 0
            && vertical.uncovered_crop_starts == 0,
        exact_grid_crop_count: exact_grid_crops.len(),
        exact_grid_uncovered_count,
        layout,
        horizontal,
        vertical,
    })
}

pub fn exact_grid_crops(width: u32, height: u32) -> Vec<SpatialRecoveryRect> {
    (0..SPATIAL_RECOVERY_V1_GRID_DIVISOR)
        .flat_map(|row| {
            (0..SPATIAL_RECOVERY_V1_GRID_DIVISOR).map(move |column| {
                let x = column * width / SPATIAL_RECOVERY_V1_GRID_DIVISOR;
                let y = row * height / SPATIAL_RECOVERY_V1_GRID_DIVISOR;
                let right = (column + 1) * width / SPATIAL_RECOVERY_V1_GRID_DIVISOR;
                let bottom = (row + 1) * height / SPATIAL_RECOVERY_V1_GRID_DIVISOR;
                SpatialRecoveryRect {
                    x,
                    y,
                    width: right - x,
                    height: bottom - y,
                }
            })
        })
        .collect()
}

fn derive_axis_packet_starts(full_length: u32, crop_length: u32, packet_length: u32) -> Vec<u32> {
    let coverage_span = crop_length - packet_length;
    let maximum_crop_start = full_length - crop_length;
    let maximum_packet_start = full_length - packet_length;
    if coverage_span == 0 {
        return (0..=maximum_crop_start).collect();
    }

    let mut starts = vec![0];
    while *starts.last().unwrap() < maximum_crop_start {
        let next = starts
            .last()
            .unwrap()
            .saturating_add(coverage_span)
            .min(maximum_packet_start);
        if next == *starts.last().unwrap() {
            break;
        }
        starts.push(next);
    }
    starts
}

fn simulate_axis_coverage(
    full_length: u32,
    crop_length: u32,
    packet_length: u32,
    packet_starts: &[u32],
) -> SpatialRecoveryAxisCoverage {
    let maximum_crop_start = full_length - crop_length;
    let mut uncovered_crop_starts = 0;
    let mut first_uncovered_crop_start = None;
    for crop_start in 0..=maximum_crop_start {
        let crop_end = crop_start + crop_length;
        let covered = packet_starts.iter().copied().any(|packet_start| {
            packet_start >= crop_start && packet_start + packet_length <= crop_end
        });
        if !covered {
            uncovered_crop_starts += 1;
            first_uncovered_crop_start.get_or_insert(crop_start);
        }
    }

    SpatialRecoveryAxisCoverage {
        full_length,
        crop_length,
        packet_length,
        packet_starts: packet_starts.to_vec(),
        tested_crop_starts: maximum_crop_start + 1,
        uncovered_crop_starts,
        first_uncovered_crop_start,
    }
}

fn encode_spatial_recovery_v1_packet(
    anchor: &WatermarkPayloadV3MinimalAnchor,
) -> [u8; SPATIAL_RECOVERY_V1_PACKET_BYTES] {
    let mut packet = [0u8; SPATIAL_RECOVERY_V1_PACKET_BYTES];
    packet[0..4].copy_from_slice(&SPATIAL_RECOVERY_V1_MAGIC);
    packet[4] = SPATIAL_RECOVERY_V1_LAYOUT_ID;
    packet[5..21].copy_from_slice(&anchor.watermark_id);
    packet[21] = spatial_recovery_v1_checksum(&packet[..21]);
    packet
}

fn decode_spatial_recovery_v1_packet(
    packet: &[u8; SPATIAL_RECOVERY_V1_PACKET_BYTES],
) -> Result<WatermarkPayloadV3MinimalAnchor, WatermarkError> {
    if packet[0..4] != SPATIAL_RECOVERY_V1_MAGIC {
        return Err(WatermarkError::ExtractFailed(
            "spatial-recovery-v1 magic mismatch".into(),
        ));
    }
    if packet[4] != SPATIAL_RECOVERY_V1_LAYOUT_ID {
        return Err(WatermarkError::ExtractFailed(format!(
            "unsupported spatial-recovery layout: {}",
            packet[4]
        )));
    }
    if packet[21] != spatial_recovery_v1_checksum(&packet[..21]) {
        return Err(WatermarkError::ExtractFailed(
            "spatial-recovery-v1 checksum mismatch".into(),
        ));
    }
    let watermark_id = packet[5..21].try_into().unwrap();
    WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput { watermark_id })
}

fn write_packet_at(
    image: &mut RgbaImage,
    packet_rect: SpatialRecoveryRect,
    bits: &[bool],
    packet_variant: usize,
) {
    for (logical_bit_index, bit) in bits.iter().copied().enumerate() {
        let physical_bit_index = physical_bit_index_for_variant(logical_bit_index, packet_variant);
        write_local_haar_bit(image, packet_rect, physical_bit_index, bit);
    }
}

fn packet_magic_matches(image: &RgbaImage, packet_rect: SpatialRecoveryRect) -> bool {
    (0..SPATIAL_RECOVERY_V1_MAGIC.len() * 8)
        .filter(|&bit_index| {
            let byte = SPATIAL_RECOVERY_V1_MAGIC[bit_index / 8];
            let expected = ((byte >> (7 - bit_index % 8)) & 1) == 1;
            read_local_haar_bit(image, packet_rect, bit_index) != expected
        })
        .take(SPATIAL_RECOVERY_V1_MAGIC_MAX_ERRORS + 1)
        .count()
        <= SPATIAL_RECOVERY_V1_MAGIC_MAX_ERRORS
}

fn read_packet_at(
    image: &RgbaImage,
    packet_rect: SpatialRecoveryRect,
    packet_variant: usize,
) -> [u8; SPATIAL_RECOVERY_V1_PACKET_BYTES] {
    let bits = (0..SPATIAL_RECOVERY_V1_PACKET_BITS)
        .map(|logical_bit_index| {
            let physical_bit_index =
                physical_bit_index_for_variant(logical_bit_index, packet_variant);
            read_local_haar_bit(image, packet_rect, physical_bit_index)
        })
        .collect::<Vec<_>>();
    bits_to_bytes(&bits)
}

fn physical_bit_index_for_variant(logical_bit_index: usize, packet_variant: usize) -> usize {
    if logical_bit_index < SPATIAL_RECOVERY_V1_MAGIC_BITS {
        return logical_bit_index;
    }
    let variant = packet_variant % SPATIAL_RECOVERY_V1_PACKET_VARIANTS;
    let multipliers = [
        1usize, 5, 7, 11, 13, 17, 19, 23, 25, 29, 31, 35, 37, 41, 43, 47, 49, 53, 55, 59, 61, 65,
        67, 71, 73,
    ];
    let logical_tail = logical_bit_index - SPATIAL_RECOVERY_V1_MAGIC_BITS;
    let physical_tail =
        (logical_tail * multipliers[variant] + variant * 37) % SPATIAL_RECOVERY_V1_PERMUTED_BITS;
    SPATIAL_RECOVERY_V1_MAGIC_BITS + physical_tail
}

fn write_local_haar_bit(
    image: &mut RgbaImage,
    packet_rect: SpatialRecoveryRect,
    bit_index: usize,
    bit: bool,
) {
    let (block_x, block_y) = local_haar_block_origin(packet_rect, bit_index);
    let mut left_sum = 0i32;
    let mut right_sum = 0i32;
    for row in 0..SPATIAL_RECOVERY_V1_BIT_BLOCK_HEIGHT {
        left_sum += pixel_luma(image.get_pixel(block_x, block_y + row));
        right_sum += pixel_luma(image.get_pixel(block_x + 1, block_y + row));
    }
    let center = ((left_sum + right_sum) / 6).clamp(
        SPATIAL_RECOVERY_V1_HAAR_TARGET / 2,
        255 - SPATIAL_RECOVERY_V1_HAAR_TARGET / 2,
    );
    let direction = if bit { 1 } else { -1 };
    let left_target = center + direction * SPATIAL_RECOVERY_V1_HAAR_TARGET / 2;
    let right_target = center - direction * SPATIAL_RECOVERY_V1_HAAR_TARGET / 2;
    for row in 0..SPATIAL_RECOVERY_V1_BIT_BLOCK_HEIGHT {
        set_pixel_luma(image.get_pixel_mut(block_x, block_y + row), left_target);
        set_pixel_luma(
            image.get_pixel_mut(block_x + 1, block_y + row),
            right_target,
        );
    }
}

fn read_local_haar_bit(
    image: &RgbaImage,
    packet_rect: SpatialRecoveryRect,
    bit_index: usize,
) -> bool {
    read_local_haar_coefficient(image, packet_rect, bit_index) > 0
}

fn read_local_haar_coefficient(
    image: &RgbaImage,
    packet_rect: SpatialRecoveryRect,
    bit_index: usize,
) -> i32 {
    let (block_x, block_y) = local_haar_block_origin(packet_rect, bit_index);
    let mut coefficient = 0i32;
    for row in 0..SPATIAL_RECOVERY_V1_BIT_BLOCK_HEIGHT {
        coefficient += pixel_luma(image.get_pixel(block_x, block_y + row));
        coefficient -= pixel_luma(image.get_pixel(block_x + 1, block_y + row));
    }
    coefficient
}

fn read_scaled_local_haar_coefficient(
    image: &RgbaImage,
    source_packet_rect: SpatialRecoveryRect,
    bit_index: usize,
    source_scale: f64,
) -> f64 {
    let block_column = bit_index % SPATIAL_RECOVERY_V1_BIT_COLUMNS;
    let block_row = bit_index / SPATIAL_RECOVERY_V1_BIT_COLUMNS;
    let source_x = source_packet_rect.x + block_column as u32 * 2;
    let source_y = source_packet_rect.y + block_row as u32 * 3;
    let left_x = map_scaled_pixel_center(source_x, source_scale, image.width());
    let mut right_x = map_scaled_pixel_center(source_x + 1, source_scale, image.width());
    if right_x == left_x {
        right_x = (left_x + 1).min(image.width() - 1);
    }
    let mut mapped_rows = Vec::with_capacity(3);
    for row in 0..3 {
        let mapped = map_scaled_pixel_center(source_y + row, source_scale, image.height());
        if !mapped_rows.contains(&mapped) {
            mapped_rows.push(mapped);
        }
    }
    let left = mapped_rows
        .iter()
        .map(|&y| f64::from(pixel_luma(image.get_pixel(left_x, y))))
        .sum::<f64>()
        / mapped_rows.len().max(1) as f64;
    let right = mapped_rows
        .iter()
        .map(|&y| f64::from(pixel_luma(image.get_pixel(right_x, y))))
        .sum::<f64>()
        / mapped_rows.len().max(1) as f64;
    left - right
}

fn map_scaled_pixel_center(source_coordinate: u32, source_scale: f64, limit: u32) -> u32 {
    (((f64::from(source_coordinate) + 0.5) * source_scale - 0.5)
        .round()
        .clamp(0.0, f64::from(limit.saturating_sub(1)))) as u32
}

fn local_haar_block_origin(packet_rect: SpatialRecoveryRect, bit_index: usize) -> (u32, u32) {
    let block_column = bit_index % SPATIAL_RECOVERY_V1_BIT_COLUMNS;
    let block_row = bit_index / SPATIAL_RECOVERY_V1_BIT_COLUMNS;
    (
        packet_rect.x + block_column as u32 * SPATIAL_RECOVERY_V1_BIT_BLOCK_WIDTH,
        packet_rect.y + block_row as u32 * SPATIAL_RECOVERY_V1_BIT_BLOCK_HEIGHT,
    )
}

fn pixel_luma(pixel: &Rgba<u8>) -> i32 {
    (77 * i32::from(pixel[0]) + 150 * i32::from(pixel[1]) + 29 * i32::from(pixel[2]) + 128) >> 8
}

fn set_pixel_luma(pixel: &mut Rgba<u8>, target_luma: i32) {
    let current_luma = pixel_luma(pixel);
    let shift = target_luma - current_luma;
    let shifted = [
        (i32::from(pixel[0]) + shift).clamp(0, 255) as u8,
        (i32::from(pixel[1]) + shift).clamp(0, 255) as u8,
        (i32::from(pixel[2]) + shift).clamp(0, 255) as u8,
    ];
    pixel[0] = shifted[0];
    pixel[1] = shifted[1];
    pixel[2] = shifted[2];
    let corrected_luma = pixel_luma(pixel);
    if (corrected_luma - target_luma).abs() > 8 {
        let grayscale = target_luma.clamp(0, 255) as u8;
        pixel[0] = grayscale;
        pixel[1] = grayscale;
        pixel[2] = grayscale;
    }
}

fn spatial_recovery_v1_checksum(bytes: &[u8]) -> u8 {
    bytes
        .iter()
        .fold(0xA7, |checksum, byte| checksum.rotate_left(1) ^ byte)
}

fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    bytes
        .iter()
        .flat_map(|byte| (0..8).rev().map(move |shift| ((byte >> shift) & 1) == 1))
        .collect()
}

fn bits_to_bytes<const LENGTH: usize>(bits: &[bool]) -> [u8; LENGTH] {
    let mut bytes = [0u8; LENGTH];
    for (bit_index, bit) in bits.iter().copied().enumerate() {
        if bit {
            bytes[bit_index / 8] |= 1 << (7 - bit_index % 8);
        }
    }
    bytes
}

#[cfg(test)]
mod tests {
    use image::{imageops, Rgba};

    use super::*;

    fn anchor() -> WatermarkPayloadV3MinimalAnchor {
        WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput {
            watermark_id: [
                0x21, 0x22, 0x23, 0x24, 0x31, 0x32, 0x33, 0x34, 0x41, 0x42, 0x43, 0x44, 0x51, 0x52,
                0x53, 0x54,
            ],
        })
        .unwrap()
    }

    fn source(width: u32, height: u32) -> RgbaImage {
        RgbaImage::from_fn(width, height, |x, y| {
            Rgba([
                ((x * 13 + y * 3) % 256) as u8,
                ((x * 5 + y * 11) % 256) as u8,
                ((x ^ y) % 256) as u8,
                217,
            ])
        })
    }

    #[test]
    fn local_transform_roundtrip_preserves_v3_uid_and_alpha() {
        let source = source(640, 480);
        let anchor = anchor();
        let expected_uid = anchor.watermark_uid();

        let (protected, _) = embed_spatial_recovery_v1(&source, &anchor).unwrap();
        let decoded = extract_spatial_recovery_v1_exact(&protected).unwrap();

        assert_eq!(decoded, anchor);
        assert_eq!(decoded.watermark_uid(), expected_uid);
        assert!(protected
            .pixels()
            .zip(source.pixels())
            .all(|(protected_pixel, source_pixel)| protected_pixel[3] == source_pixel[3]));
    }

    #[test]
    fn exact_reader_prefers_consensus_over_checksum_valid_conflicting_packet() {
        let source = source(640, 480);
        let expected = anchor();
        let conflicting = WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput {
            watermark_id: [
                0x21, 0x22, 0x23, 0x24, 0x31, 0x32, 0x33, 0x34, 0x41, 0x02, 0x43, 0x44, 0x51, 0x52,
                0x53, 0x54,
            ],
        })
        .unwrap();
        let (mut protected, layout) = embed_spatial_recovery_v1(&source, &expected).unwrap();
        let conflicting_packet = encode_spatial_recovery_v1_packet(&conflicting);
        let conflicting_bits = bytes_to_bits(&conflicting_packet);
        write_packet_at(&mut protected, layout.packet_rects[0], &conflicting_bits, 0);

        let decoded = extract_spatial_recovery_v1_exact(&protected).unwrap();
        let diagnostic = diagnose_spatial_recovery_v1_exact(&protected, &expected).unwrap();

        assert_eq!(decoded, expected);
        assert_eq!(diagnostic.selected_stage.as_deref(), Some("consensus"));
        assert_eq!(
            diagnostic.packets[0].recovered_uid.as_deref(),
            Some(conflicting.watermark_uid().as_str())
        );
    }

    #[test]
    fn local_transform_keeps_the_existing_1920_by_1080_layout() {
        let layout = derive_spatial_recovery_v1_layout(1920, 1080).unwrap();

        assert_eq!(layout.layout_id, SPATIAL_RECOVERY_V1_LAYOUT_ID);
        assert_eq!(layout.packet_width, 32);
        assert_eq!(layout.packet_height, 35);
        assert_eq!(
            layout.horizontal_packet_starts,
            vec![0, 448, 896, 1344, 1792]
        );
        assert_eq!(layout.vertical_packet_starts, vec![0, 235, 470, 705, 940]);
    }

    #[test]
    fn each_exact_grid_crop_recovers_same_v3_uid() {
        let source = source(640, 480);
        let anchor = anchor();
        let expected_uid = anchor.watermark_uid();
        let (protected, _) = embed_spatial_recovery_v1(&source, &anchor).unwrap();

        for crop in exact_grid_crops(source.width(), source.height()) {
            let suspect =
                imageops::crop_imm(&protected, crop.x, crop.y, crop.width, crop.height).to_image();
            let decoded = extract_spatial_recovery_v1(&suspect).unwrap();
            assert_eq!(decoded.watermark_uid(), expected_uid);
        }
    }

    #[test]
    fn clean_image_is_rejected_by_exact_and_scanning_readers() {
        let source = source(640, 480);

        assert!(extract_spatial_recovery_v1_exact(&source).is_err());
        assert!(extract_spatial_recovery_v1(&source).is_err());
    }
}

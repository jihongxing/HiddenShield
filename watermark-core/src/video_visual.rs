use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::encode_payload;
use crate::error::{WatermarkError, WatermarkErrorCode};
use crate::payload::{
    registry_proof_hash_from_hex, watermark_id_from_uid, AIContentFlags, PayloadDigestBuildInput,
    PayloadV2BuildInput, WatermarkIssueMode, WatermarkMediaType, WatermarkPayload, PAYLOAD_BYTES,
};

pub const VIDEO_VISUAL_STRATEGY_SCHEMA_VERSION: &str = "video_strategy_v1";
pub(crate) const VIDEO_VISUAL_SYNC_MARKER_V1: [bool; 16] = [
    true, false, true, true, false, true, false, false, true, true, true, false, false, false,
    true, false,
];
const VIDEO_VISUAL_ECC_REPEAT: usize = 3;
const VIDEO_VISUAL_SYNC_MAX_BIT_ERRORS: usize = 2;
const VIDEO_VISUAL_DCT_EMBED_DELTA: f32 = 96.0;
const VIDEO_VISUAL_DCT_MAX_STREAM_REPEATS: usize = 3;
const VIDEO_VISUAL_TEXTURE_HINTS_PER_FRAME: usize = 512;
const VIDEO_VISUAL_MAIN_BATTLEFIELD_MIN_LONG_EDGE: u32 = 1920;
const VIDEO_VISUAL_MAIN_BATTLEFIELD_MIN_SHORT_EDGE: u32 = 1080;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoVisualProfile {
    Luma8SyntheticV1,
    LumaDctMidBandV1,
}

impl VideoVisualProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Luma8SyntheticV1 => "luma8_synthetic_v1",
            Self::LumaDctMidBandV1 => "luma_dct_mid_band_v1",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoVisualComplexityTier {
    Small,
    Standard,
    High,
}

impl VideoVisualComplexityTier {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Small => "small",
            Self::Standard => "standard",
            Self::High => "high",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoVisualComplexityBudget {
    pub tier: VideoVisualComplexityTier,
    pub sampled_frames: u32,
    pub candidate_blocks_per_frame: u32,
    pub selected_coeff_pairs: u32,
    pub estimated_operations: u64,
    pub max_roundtrip_ms: u64,
}

pub fn derive_video_visual_complexity_budget(
    tier: VideoVisualComplexityTier,
    feature_bundle: &VideoFeatureBundle,
) -> Result<VideoVisualComplexityBudget, WatermarkError> {
    if feature_bundle.profile != VideoVisualProfile::LumaDctMidBandV1 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "DCT complexity budget requires LumaDctMidBandV1 profile",
        ));
    }
    if feature_bundle.frame_count == 0 || feature_bundle.width < 8 || feature_bundle.height < 8 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::FeatureBundleInvalid,
            "DCT complexity budget requires at least one 8x8 frame",
        ));
    }

    let (max_sampled_frames, max_candidate_blocks, max_roundtrip_ms) = match tier {
        VideoVisualComplexityTier::Small => (4u32, 512u32, 1_500u64),
        VideoVisualComplexityTier::Standard => (8u32, 768u32, 3_000u64),
        VideoVisualComplexityTier::High => (12u32, 1_024u32, 6_000u64),
    };
    let total_blocks_per_frame = (feature_bundle.width / 8) * (feature_bundle.height / 8);
    let sampled_frames = feature_bundle.frame_count.min(max_sampled_frames);
    let candidate_blocks_per_frame = total_blocks_per_frame.min(max_candidate_blocks);
    let selected_coeff_pairs = default_luma_dct_mid_band_pairs().len() as u32;
    let estimated_operations =
        sampled_frames as u64 * candidate_blocks_per_frame as u64 * selected_coeff_pairs as u64;

    Ok(VideoVisualComplexityBudget {
        tier,
        sampled_frames,
        candidate_blocks_per_frame,
        selected_coeff_pairs,
        estimated_operations,
        max_roundtrip_ms,
    })
}

pub fn sample_video_visual_frame_indices(
    feature_bundle: &VideoFeatureBundle,
    budget: &VideoVisualComplexityBudget,
) -> Result<Vec<u32>, WatermarkError> {
    if feature_bundle.frame_count == 0 || budget.sampled_frames == 0 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::FeatureBundleInvalid,
            "video frame sampling requires at least one frame",
        ));
    }
    if budget.sampled_frames >= feature_bundle.frame_count {
        return Ok((0..feature_bundle.frame_count).collect());
    }

    let last = feature_bundle.frame_count - 1;
    let denominator = budget.sampled_frames - 1;
    let mut indices = Vec::with_capacity(budget.sampled_frames as usize);
    for index in 0..budget.sampled_frames {
        let numerator = index * last;
        let sampled = (numerator + denominator / 2) / denominator;
        if indices.last().copied() != Some(sampled) {
            indices.push(sampled);
        }
    }
    Ok(indices)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoLumaBitDepth {
    Eight,
    Ten,
    Twelve,
}

impl VideoLumaBitDepth {
    fn bits(self) -> u8 {
        match self {
            Self::Eight => 8,
            Self::Ten => 10,
            Self::Twelve => 12,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoLumaColorRange {
    Full,
    Limited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodedVideoLumaPlane<'a> {
    pub width: u32,
    pub height: u32,
    pub stride_samples: usize,
    pub samples: &'a [u16],
    pub bit_depth: VideoLumaBitDepth,
    pub color_range: VideoLumaColorRange,
    pub target_profile: VideoVisualProfile,
}

pub fn video_frame_plane_from_decoded_luma(
    input: DecodedVideoLumaPlane<'_>,
) -> Result<VideoFramePlane, WatermarkError> {
    if input.target_profile != VideoVisualProfile::LumaDctMidBandV1 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "decoded Y plane currently maps only to LumaDctMidBandV1",
        ));
    }
    if input.width == 0 || input.height == 0 || input.width < 8 || input.height < 8 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "decoded Y plane dimensions must be at least 8x8",
        ));
    }
    if input.stride_samples < input.width as usize {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "decoded Y plane stride must be at least the frame width",
        ));
    }
    let expected = input
        .stride_samples
        .checked_mul(input.height as usize)
        .ok_or_else(|| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::UnsupportedVideoProfile,
                "decoded Y plane buffer size overflow",
            )
        })?;
    if input.samples.len() < expected {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::FeatureBundleInvalid,
            "decoded Y plane buffer is shorter than stride * height",
        ));
    }

    let mut pixels = Vec::with_capacity(input.width as usize * input.height as usize);
    for y in 0..input.height as usize {
        let row_start = y * input.stride_samples;
        for x in 0..input.width as usize {
            pixels.push(normalize_luma_sample_to_u8(
                input.samples[row_start + x],
                input.bit_depth,
                input.color_range,
            ));
        }
    }

    VideoFramePlane::new_luma_dct_mid_band(input.width, input.height, input.width as usize, pixels)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VideoFramePlane {
    pub profile: VideoVisualProfile,
    pub width: u32,
    pub height: u32,
    pub stride: usize,
    pixels: Vec<u8>,
}

impl VideoFramePlane {
    pub fn new_luma8(
        width: u32,
        height: u32,
        stride: usize,
        pixels: Vec<u8>,
    ) -> Result<Self, WatermarkError> {
        if width == 0 || height == 0 {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::UnsupportedVideoProfile,
                "video frame dimensions are required",
            ));
        }
        if stride < width as usize {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::UnsupportedVideoProfile,
                "video frame stride must be at least the frame width",
            ));
        }
        let expected = stride.checked_mul(height as usize).ok_or_else(|| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::UnsupportedVideoProfile,
                "video frame buffer size overflow",
            )
        })?;
        if pixels.len() < expected {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::FeatureBundleInvalid,
                "video frame buffer is shorter than stride * height",
            ));
        }

        Ok(Self {
            profile: VideoVisualProfile::Luma8SyntheticV1,
            width,
            height,
            stride,
            pixels,
        })
    }

    pub fn new_luma_dct_mid_band(
        width: u32,
        height: u32,
        stride: usize,
        pixels: Vec<u8>,
    ) -> Result<Self, WatermarkError> {
        let mut frame = Self::new_luma8(width, height, stride, pixels)?;
        if width < 8 || height < 8 {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::UnsupportedVideoProfile,
                "DCT video frame dimensions must be at least 8x8",
            ));
        }
        frame.profile = VideoVisualProfile::LumaDctMidBandV1;
        Ok(frame)
    }

    pub fn visible_rows(&self) -> impl Iterator<Item = &[u8]> {
        let width = self.width as usize;
        self.pixels
            .chunks(self.stride)
            .take(self.height as usize)
            .map(move |row| &row[..width])
    }

    pub fn luma_pixels(&self) -> Vec<u8> {
        self.visible_rows()
            .flat_map(|row| row.iter().copied())
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoVisualPayloadBuildInput<'a> {
    pub creator_identity: &'a str,
    pub device_identity: &'a str,
    pub source_video_sha256: [u8; 32],
    pub timestamp: u64,
    pub ai_flags: AIContentFlags,
}

pub fn build_video_visual_payload(
    input: VideoVisualPayloadBuildInput<'_>,
) -> Result<WatermarkPayload, WatermarkError> {
    WatermarkPayload::from_identity_and_media_sha256(PayloadDigestBuildInput {
        creator_identity: input.creator_identity,
        device_identity: input.device_identity,
        media_sha256: input.source_video_sha256,
        timestamp: input.timestamp,
        ai_flags: input.ai_flags,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoVisualReservedPayloadBuildInput<'a> {
    pub watermark_uid: &'a str,
    pub creator_identity: &'a str,
    pub source_video_sha256: [u8; 32],
    pub timestamp: u64,
    pub ai_flags: AIContentFlags,
    pub registry_proof_hash: Option<&'a str>,
}

pub fn build_video_visual_payload_from_reserved_uid(
    input: VideoVisualReservedPayloadBuildInput<'_>,
) -> Result<WatermarkPayload, WatermarkError> {
    let creator_identity = input.creator_identity.trim();
    if creator_identity.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::MissingCreatorIdentity,
            "creator identity is required",
        ));
    }
    let registry_proof_hash = input
        .registry_proof_hash
        .map(registry_proof_hash_from_hex)
        .transpose()?;

    WatermarkPayload::from_v2(PayloadV2BuildInput {
        watermark_id: watermark_id_from_uid(input.watermark_uid)?,
        parent_watermark_id: None,
        revision: 1,
        issued_at: input.timestamp,
        original_sha256: input.source_video_sha256,
        ai_flags: input.ai_flags,
        issue_mode: WatermarkIssueMode::ServerReserved,
        media_type: WatermarkMediaType::VideoVisual,
        registry_proof_hash,
        creator_binding: Some(creator_identity),
    })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoFeatureBundle {
    pub profile: VideoVisualProfile,
    pub width: u32,
    pub height: u32,
    pub frame_count: u32,
    pub duration_ms: u64,
    pub source_video_sha256: [u8; 32],
    pub feature_digest: [u8; 32],
    pub texture_hints: Vec<VideoVisualTextureHint>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VideoVisualTextureHint {
    pub frame_index: u32,
    pub x: u32,
    pub y: u32,
    pub score: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VideoFeatureBundleBuildInput<'a> {
    pub frames: &'a [VideoFramePlane],
    pub source_video_sha256: [u8; 32],
    pub duration_ms: u64,
}

pub fn build_video_feature_bundle(
    input: VideoFeatureBundleBuildInput<'_>,
) -> Result<VideoFeatureBundle, WatermarkError> {
    let first = input.frames.first().ok_or_else(|| {
        WatermarkError::invalid_payload(
            WatermarkErrorCode::FeatureBundleInvalid,
            "at least one synthetic video frame is required",
        )
    })?;

    let mut hasher = Sha256::new();
    hasher.update(VIDEO_VISUAL_STRATEGY_SCHEMA_VERSION.as_bytes());
    hasher.update(first.profile.as_str().as_bytes());
    hasher.update(first.width.to_be_bytes());
    hasher.update(first.height.to_be_bytes());
    hasher.update(input.duration_ms.to_be_bytes());
    hasher.update(input.source_video_sha256);

    for frame in input.frames {
        if frame.profile != first.profile
            || frame.width != first.width
            || frame.height != first.height
        {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::FeatureBundleInvalid,
                "all synthetic video frames must share profile and dimensions",
            ));
        }
        for row in frame.visible_rows() {
            hasher.update(row);
        }
    }

    let feature_digest: [u8; 32] = hasher.finalize().into();
    let texture_hints =
        collect_video_visual_texture_hints(input.frames, VIDEO_VISUAL_TEXTURE_HINTS_PER_FRAME);
    Ok(VideoFeatureBundle {
        profile: first.profile,
        width: first.width,
        height: first.height,
        frame_count: input.frames.len() as u32,
        duration_ms: input.duration_ms,
        source_video_sha256: input.source_video_sha256,
        feature_digest,
        texture_hints,
    })
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoVisualRegion {
    pub frame_index: u32,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub strength: f32,
    pub redundancy_group: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoVisualRegionSelectionMode {
    SeededRandom,
    CenterSafeGrid,
    DistributedGrid,
    TextureAware,
    TranscodeStable,
}

impl VideoVisualRegionSelectionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SeededRandom => "seeded_random",
            Self::CenterSafeGrid => "center_safe_grid",
            Self::DistributedGrid => "distributed_grid",
            Self::TextureAware => "texture_aware",
            Self::TranscodeStable => "transcode_stable",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoVisualStrategy {
    pub schema_version: &'static str,
    pub task_id: String,
    pub watermark_uid: String,
    pub target_profile: VideoVisualProfile,
    pub regions: Vec<VideoVisualRegion>,
    pub self_check_threshold: f32,
    pub expires_at: u64,
    pub strategy_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoVisualStrategyBuildInput<'a> {
    pub task_id: &'a str,
    pub payload: &'a WatermarkPayload,
    pub feature_bundle: &'a VideoFeatureBundle,
    pub target_profile: VideoVisualProfile,
    pub expires_at: u64,
    pub self_check_threshold: f32,
    pub max_regions: u32,
}

pub fn derive_video_visual_strategy(
    input: VideoVisualStrategyBuildInput<'_>,
) -> Result<VideoVisualStrategy, WatermarkError> {
    let region_selection_mode = default_video_visual_region_selection(input.feature_bundle);
    derive_video_visual_strategy_with_region_selection(input, region_selection_mode)
}

pub fn derive_video_visual_strategy_with_region_selection(
    input: VideoVisualStrategyBuildInput<'_>,
    region_selection_mode: VideoVisualRegionSelectionMode,
) -> Result<VideoVisualStrategy, WatermarkError> {
    let task_id = input.task_id.trim();
    if task_id.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "task_id is required",
        ));
    }
    if input.feature_bundle.profile != input.target_profile {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "target profile must match the feature bundle profile",
        ));
    }
    if input.feature_bundle.frame_count == 0
        || input.feature_bundle.width == 0
        || input.feature_bundle.height == 0
    {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::FeatureBundleInvalid,
            "feature bundle must contain frame dimensions",
        ));
    }
    if !(0.0..=1.0).contains(&input.self_check_threshold) {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "self_check_threshold must be between 0 and 1",
        ));
    }
    if input.max_regions == 0 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "max_regions must be greater than zero",
        ));
    }

    let payload_bytes = encode_payload(input.payload);
    let seed = strategy_seed(task_id, &payload_bytes, input.feature_bundle);
    let region_count = input.max_regions;
    let regions = derive_regions(
        &seed,
        input.feature_bundle,
        region_count,
        region_selection_mode,
    );
    let strategy_digest = digest_strategy(
        task_id,
        input.payload,
        input.feature_bundle,
        input.target_profile,
        input.expires_at,
        input.self_check_threshold,
        &regions,
    );

    Ok(VideoVisualStrategy {
        schema_version: VIDEO_VISUAL_STRATEGY_SCHEMA_VERSION,
        task_id: task_id.to_string(),
        watermark_uid: input.payload.watermark_uid(),
        target_profile: input.target_profile,
        regions,
        self_check_threshold: input.self_check_threshold,
        expires_at: input.expires_at,
        strategy_digest,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoVisualSelfCheckInput<'a> {
    pub strategy: &'a VideoVisualStrategy,
    pub observed_strategy_digest: &'a str,
    pub checked_frames: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VideoVisualSelfCheckFramesInput<'a> {
    pub strategy: &'a VideoVisualStrategy,
    pub observed_strategy_digest: &'a str,
    pub frames: &'a [VideoFramePlane],
    pub expected_payload: &'a WatermarkPayload,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VideoVisualSelfCheckResult {
    pub passed: bool,
    pub checked_frames: u32,
    pub confidence: f32,
    pub self_check_threshold: f32,
    pub strategy_digest: String,
    pub watermark_uid: String,
}

pub fn self_check_video_visual_watermark(
    input: VideoVisualSelfCheckInput<'_>,
) -> Result<VideoVisualSelfCheckResult, WatermarkError> {
    if input.strategy.strategy_digest.trim().is_empty() || input.strategy.regions.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "strategy must contain digest and regions",
        ));
    }
    if input.observed_strategy_digest != input.strategy.strategy_digest {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::SelfCheckFailed,
            "observed strategy digest does not match the expected strategy",
        ));
    }

    let expected_frames = input
        .strategy
        .regions
        .iter()
        .map(|region| region.frame_index)
        .collect::<std::collections::BTreeSet<_>>()
        .len() as u32;
    let confidence = if expected_frames == 0 {
        0.0
    } else {
        (input.checked_frames.min(expected_frames) as f32) / (expected_frames as f32)
    };
    let passed = confidence >= input.strategy.self_check_threshold;

    if !passed {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::SelfCheckFailed,
            "checked frames did not reach the self-check threshold",
        ));
    }

    Ok(VideoVisualSelfCheckResult {
        passed,
        checked_frames: input.checked_frames,
        confidence,
        self_check_threshold: input.strategy.self_check_threshold,
        strategy_digest: input.strategy.strategy_digest.clone(),
        watermark_uid: input.strategy.watermark_uid.clone(),
    })
}

pub fn embed_video_visual_frame(
    frame_index: u32,
    _frame: &mut VideoFramePlane,
    strategy: &VideoVisualStrategy,
    payload: &WatermarkPayload,
) -> Result<(), WatermarkError> {
    if strategy.watermark_uid != payload.watermark_uid() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "strategy watermark_uid must match the payload watermark_uid",
        ));
    }

    let offsets = strategy_offsets(frame_index, _frame, strategy)?;
    let payload_bits = bytes_to_bits(&encode_payload(payload));
    if offsets.len() < payload_bits.len() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "strategy regions do not provide enough synthetic frame capacity",
        ));
    }

    for (offset, bit) in offsets.into_iter().zip(payload_bits) {
        _frame.pixels[offset] = (_frame.pixels[offset] & 0b1111_1110) | u8::from(bit);
    }
    Ok(())
}

pub fn embed_video_visual_frames(
    frames: &mut [VideoFramePlane],
    strategy: &VideoVisualStrategy,
    payload: &WatermarkPayload,
) -> Result<u32, WatermarkError> {
    let frame_indices = strategy_frame_indices(strategy);
    if frame_indices.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "strategy must contain at least one frame region",
        ));
    }

    let mut embedded = 0u32;
    for frame_index in frame_indices {
        let frame = frames.get_mut(frame_index as usize).ok_or_else(|| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::StrategyInvalid,
                "strategy frame index exceeds the synthetic frame set",
            )
        })?;
        embed_video_visual_frame(frame_index, frame, strategy, payload)?;
        embedded += 1;
    }
    Ok(embedded)
}

pub fn extract_video_visual_watermark(
    frame_index: u32,
    frame: &VideoFramePlane,
    strategy: &VideoVisualStrategy,
) -> Result<WatermarkPayload, WatermarkError> {
    let offsets = strategy_offsets(frame_index, frame, strategy)?;
    if offsets.len() < PAYLOAD_BYTES * 8 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "strategy regions do not provide enough synthetic frame capacity",
        ));
    }

    let bits = offsets
        .into_iter()
        .take(PAYLOAD_BYTES * 8)
        .map(|offset| (frame.pixels[offset] & 1) == 1)
        .collect::<Vec<_>>();
    let bytes = bits_to_bytes(&bits);
    let payload_bytes: [u8; PAYLOAD_BYTES] = bytes.try_into().map_err(|_| {
        WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "synthetic frame payload byte length is invalid",
        )
    })?;

    crate::decode_payload(&payload_bytes).map_err(|error| {
        WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            format!("synthetic frame payload decode failed: {error}"),
        )
    })
}

pub fn extract_video_visual_watermark_from_frames(
    frames: &[VideoFramePlane],
    strategy: &VideoVisualStrategy,
) -> Result<WatermarkPayload, WatermarkError> {
    let mut first_error = None;
    for frame_index in strategy_frame_indices(strategy) {
        let Some(frame) = frames.get(frame_index as usize) else {
            first_error.get_or_insert_with(|| {
                WatermarkError::invalid_payload(
                    WatermarkErrorCode::StrategyInvalid,
                    "strategy frame index exceeds the synthetic frame set",
                )
            });
            continue;
        };
        match extract_video_visual_watermark(frame_index, frame, strategy) {
            Ok(payload) => return Ok(payload),
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
    }

    Err(first_error.unwrap_or_else(|| {
        WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "no strategy frames could be extracted",
        )
    }))
}

pub fn self_check_video_visual_frames(
    input: VideoVisualSelfCheckFramesInput<'_>,
) -> Result<VideoVisualSelfCheckResult, WatermarkError> {
    if input.strategy.strategy_digest.trim().is_empty() || input.strategy.regions.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "strategy must contain digest and regions",
        ));
    }
    if input.observed_strategy_digest != input.strategy.strategy_digest {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::SelfCheckFailed,
            "observed strategy digest does not match the expected strategy",
        ));
    }

    let frame_indices = strategy_frame_indices(input.strategy);
    if frame_indices.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "strategy must contain at least one frame region",
        ));
    }

    let expected_uid = input.expected_payload.watermark_uid();
    let mut checked_frames = 0u32;
    let mut matched_frames = 0u32;
    for frame_index in &frame_indices {
        let Some(frame) = input.frames.get(*frame_index as usize) else {
            continue;
        };
        checked_frames += 1;
        if let Ok(payload) = extract_video_visual_watermark(*frame_index, frame, input.strategy) {
            if payload == *input.expected_payload && payload.watermark_uid() == expected_uid {
                matched_frames += 1;
            }
        }
    }

    let confidence = matched_frames as f32 / frame_indices.len() as f32;
    let passed = confidence >= input.strategy.self_check_threshold;
    if !passed {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::SelfCheckFailed,
            "extracted synthetic frames did not reach the self-check threshold",
        ));
    }

    Ok(VideoVisualSelfCheckResult {
        passed,
        checked_frames,
        confidence,
        self_check_threshold: input.strategy.self_check_threshold,
        strategy_digest: input.strategy.strategy_digest.clone(),
        watermark_uid: expected_uid,
    })
}

pub fn embed_video_visual_dct_frames(
    frames: &mut [VideoFramePlane],
    strategy: &VideoVisualStrategy,
    payload: &WatermarkPayload,
) -> Result<u32, WatermarkError> {
    embed_luma_dct_mid_band_frames(frames, strategy, payload)
}

pub fn extract_video_visual_dct_from_frames(
    frames: &[VideoFramePlane],
    strategy: &VideoVisualStrategy,
) -> Result<WatermarkPayload, WatermarkError> {
    extract_luma_dct_mid_band_from_frames(frames, strategy)
}

pub fn self_check_video_visual_dct_frames(
    input: VideoVisualSelfCheckFramesInput<'_>,
) -> Result<VideoVisualSelfCheckResult, WatermarkError> {
    self_check_luma_dct_mid_band_frames(input)
}

#[allow(dead_code)]
pub(crate) fn embed_luma_dct_mid_band_frame(
    frame_index: u32,
    frame: &mut VideoFramePlane,
    strategy: &VideoVisualStrategy,
    payload: &WatermarkPayload,
) -> Result<usize, WatermarkError> {
    if frame.profile != VideoVisualProfile::LumaDctMidBandV1
        || strategy.target_profile != VideoVisualProfile::LumaDctMidBandV1
    {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "DCT mid-band frame embedding requires LumaDctMidBandV1 profile",
        ));
    }
    if strategy.watermark_uid != payload.watermark_uid() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "strategy watermark_uid must match the payload watermark_uid",
        ));
    }

    let blocks = strategy_dct_blocks(frame_index, frame, strategy)?;
    let stream = encode_video_visual_bitstream(&bytes_to_bits(&encode_payload(payload)));
    embed_luma_dct_mid_band_stream_into_frame(
        frame,
        &blocks,
        &stream,
        VIDEO_VISUAL_DCT_EMBED_DELTA,
    )?;
    Ok(stream.len())
}

#[allow(dead_code)]
pub(crate) fn extract_luma_dct_mid_band_frame(
    frame_index: u32,
    frame: &VideoFramePlane,
    strategy: &VideoVisualStrategy,
) -> Result<WatermarkPayload, WatermarkError> {
    if frame.profile != VideoVisualProfile::LumaDctMidBandV1
        || strategy.target_profile != VideoVisualProfile::LumaDctMidBandV1
    {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "DCT mid-band frame extraction requires LumaDctMidBandV1 profile",
        ));
    }

    let blocks = strategy_dct_blocks(frame_index, frame, strategy)?;
    let stream_len =
        VIDEO_VISUAL_SYNC_MARKER_V1.len() + PAYLOAD_BYTES * 8 * VIDEO_VISUAL_ECC_REPEAT;
    let streams = extract_luma_dct_mid_band_streams_from_frame(frame, &blocks, stream_len)?;
    extract_luma_dct_mid_band_payload_from_streams(&streams)
}

#[allow(dead_code)]
pub(crate) fn embed_luma_dct_mid_band_frames(
    frames: &mut [VideoFramePlane],
    strategy: &VideoVisualStrategy,
    payload: &WatermarkPayload,
) -> Result<u32, WatermarkError> {
    let frame_indices = strategy_frame_indices(strategy);
    if frame_indices.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT strategy must contain at least one frame region",
        ));
    }

    let mut embedded = 0u32;
    for frame_index in frame_indices {
        let frame = frames.get_mut(frame_index as usize).ok_or_else(|| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::StrategyInvalid,
                "DCT strategy frame index exceeds the frame set",
            )
        })?;
        embed_luma_dct_mid_band_frame(frame_index, frame, strategy, payload)?;
        embedded += 1;
    }
    Ok(embedded)
}

#[allow(dead_code)]
pub(crate) fn extract_luma_dct_mid_band_from_frames(
    frames: &[VideoFramePlane],
    strategy: &VideoVisualStrategy,
) -> Result<WatermarkPayload, WatermarkError> {
    let mut first_error = None;
    let mut streams = Vec::new();
    let stream_len =
        VIDEO_VISUAL_SYNC_MARKER_V1.len() + PAYLOAD_BYTES * 8 * VIDEO_VISUAL_ECC_REPEAT;
    for frame_index in strategy_frame_indices(strategy) {
        let Some(frame) = frames.get(frame_index as usize) else {
            first_error.get_or_insert_with(|| {
                WatermarkError::invalid_payload(
                    WatermarkErrorCode::StrategyInvalid,
                    "DCT strategy frame index exceeds the frame set",
                )
            });
            continue;
        };
        match extract_luma_dct_mid_band_frame(frame_index, frame, strategy) {
            Ok(payload) => return Ok(payload),
            Err(error) => {
                first_error.get_or_insert(error);
                if let Ok(blocks) = strategy_dct_blocks(frame_index, frame, strategy) {
                    if let Ok(frame_streams) =
                        extract_luma_dct_mid_band_streams_from_frame(frame, &blocks, stream_len)
                    {
                        streams.extend(frame_streams);
                    }
                }
            }
        }
    }

    if let Ok(payload) = extract_luma_dct_mid_band_payload_from_streams(&streams) {
        return Ok(payload);
    }

    Err(first_error.unwrap_or_else(|| {
        WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "no DCT strategy frames could be extracted",
        )
    }))
}

#[allow(dead_code)]
pub(crate) fn self_check_luma_dct_mid_band_frames(
    input: VideoVisualSelfCheckFramesInput<'_>,
) -> Result<VideoVisualSelfCheckResult, WatermarkError> {
    if input.strategy.target_profile != VideoVisualProfile::LumaDctMidBandV1 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "DCT self-check requires LumaDctMidBandV1 profile",
        ));
    }
    if input.strategy.strategy_digest.trim().is_empty() || input.strategy.regions.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT strategy must contain digest and regions",
        ));
    }
    if input.observed_strategy_digest != input.strategy.strategy_digest {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::SelfCheckFailed,
            "observed strategy digest does not match the expected DCT strategy",
        ));
    }

    let frame_indices = strategy_frame_indices(input.strategy);
    if frame_indices.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT strategy must contain at least one frame region",
        ));
    }

    let expected_uid = input.expected_payload.watermark_uid();
    let mut checked_frames = 0u32;
    let mut matched_frames = 0u32;
    let mut streams = Vec::new();
    let stream_len =
        VIDEO_VISUAL_SYNC_MARKER_V1.len() + PAYLOAD_BYTES * 8 * VIDEO_VISUAL_ECC_REPEAT;
    for frame_index in &frame_indices {
        let Some(frame) = input.frames.get(*frame_index as usize) else {
            continue;
        };
        checked_frames += 1;
        if let Ok(blocks) = strategy_dct_blocks(*frame_index, frame, input.strategy) {
            if let Ok(frame_streams) =
                extract_luma_dct_mid_band_streams_from_frame(frame, &blocks, stream_len)
            {
                if let Ok(payload) = extract_luma_dct_mid_band_payload_from_streams(&frame_streams)
                {
                    if payload == *input.expected_payload && payload.watermark_uid() == expected_uid
                    {
                        matched_frames += 1;
                    }
                }
                streams.extend(frame_streams);
            }
        }
    }

    let fused_match = extract_luma_dct_mid_band_payload_from_streams(&streams)
        .map(|payload| {
            payload == *input.expected_payload && payload.watermark_uid() == expected_uid
        })
        .unwrap_or(false);
    let all_strategy_frames_checked = checked_frames as usize == frame_indices.len();
    let confidence = if all_strategy_frames_checked && fused_match {
        1.0
    } else {
        matched_frames as f32 / frame_indices.len() as f32
    };
    let passed = confidence >= input.strategy.self_check_threshold;
    if !passed {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::SelfCheckFailed,
            "extracted DCT frames did not reach the self-check threshold",
        ));
    }

    Ok(VideoVisualSelfCheckResult {
        passed,
        checked_frames,
        confidence,
        self_check_threshold: input.strategy.self_check_threshold,
        strategy_digest: input.strategy.strategy_digest.clone(),
        watermark_uid: expected_uid,
    })
}

fn strategy_seed(
    task_id: &str,
    payload_bytes: &[u8; PAYLOAD_BYTES],
    feature_bundle: &VideoFeatureBundle,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(VIDEO_VISUAL_STRATEGY_SCHEMA_VERSION.as_bytes());
    hasher.update(task_id.as_bytes());
    hasher.update(payload_bytes);
    hasher.update(feature_bundle.feature_digest);
    hasher.update(feature_bundle.source_video_sha256);
    hasher.finalize().into()
}

fn default_video_visual_region_selection(
    feature_bundle: &VideoFeatureBundle,
) -> VideoVisualRegionSelectionMode {
    let long_edge = feature_bundle.width.max(feature_bundle.height);
    let short_edge = feature_bundle.width.min(feature_bundle.height);
    if feature_bundle.profile == VideoVisualProfile::LumaDctMidBandV1
        && long_edge >= VIDEO_VISUAL_MAIN_BATTLEFIELD_MIN_LONG_EDGE
        && short_edge >= VIDEO_VISUAL_MAIN_BATTLEFIELD_MIN_SHORT_EDGE
        && !feature_bundle.texture_hints.is_empty()
    {
        VideoVisualRegionSelectionMode::TranscodeStable
    } else {
        VideoVisualRegionSelectionMode::SeededRandom
    }
}

fn derive_regions(
    seed: &[u8; 32],
    feature_bundle: &VideoFeatureBundle,
    region_count: u32,
    region_selection_mode: VideoVisualRegionSelectionMode,
) -> Vec<VideoVisualRegion> {
    let region_width = (feature_bundle.width / 8).clamp(1, feature_bundle.width);
    let region_height = (feature_bundle.height / 8).clamp(1, feature_bundle.height);
    let max_x = feature_bundle.width.saturating_sub(region_width);
    let max_y = feature_bundle.height.saturating_sub(region_height);

    if region_selection_mode == VideoVisualRegionSelectionMode::TextureAware
        && !feature_bundle.texture_hints.is_empty()
    {
        return derive_texture_aware_regions(seed, feature_bundle, region_count);
    }
    if region_selection_mode == VideoVisualRegionSelectionMode::TranscodeStable
        && !feature_bundle.texture_hints.is_empty()
    {
        return derive_transcode_stable_regions(seed, feature_bundle, region_count);
    }

    (0..region_count)
        .map(|index| {
            let offset = ((index as usize) * 7) % seed.len();
            let frame_index = index % feature_bundle.frame_count;
            let (x, y) = derive_region_origin(
                seed,
                offset,
                index,
                region_count,
                max_x,
                max_y,
                region_selection_mode,
            );
            VideoVisualRegion {
                frame_index,
                x,
                y,
                width: region_width,
                height: region_height,
                strength: 0.08 + ((seed[(offset + 6) % seed.len()] as f32) / 255.0) * 0.04,
                redundancy_group: (index % 4) as u8,
            }
        })
        .collect()
}

fn derive_region_origin(
    seed: &[u8; 32],
    offset: usize,
    index: u32,
    region_count: u32,
    max_x: u32,
    max_y: u32,
    region_selection_mode: VideoVisualRegionSelectionMode,
) -> (u32, u32) {
    match region_selection_mode {
        VideoVisualRegionSelectionMode::SeededRandom => {
            seeded_region_origin(seed, offset, max_x, max_y)
        }
        VideoVisualRegionSelectionMode::CenterSafeGrid => {
            center_safe_grid_region_origin(index, region_count, max_x, max_y)
        }
        VideoVisualRegionSelectionMode::DistributedGrid => {
            distributed_grid_region_origin(index, region_count, max_x, max_y)
        }
        VideoVisualRegionSelectionMode::TextureAware => {
            seeded_region_origin(seed, offset, max_x, max_y)
        }
        VideoVisualRegionSelectionMode::TranscodeStable => {
            seeded_region_origin(seed, offset, max_x, max_y)
        }
    }
}

fn derive_texture_aware_regions(
    seed: &[u8; 32],
    feature_bundle: &VideoFeatureBundle,
    region_count: u32,
) -> Vec<VideoVisualRegion> {
    let region_width = (feature_bundle.width / 8).clamp(1, feature_bundle.width);
    let region_height = (feature_bundle.height / 8).clamp(1, feature_bundle.height);
    let max_x = feature_bundle.width.saturating_sub(region_width);
    let max_y = feature_bundle.height.saturating_sub(region_height);

    (0..region_count)
        .map(|index| {
            let offset = ((index as usize) * 11) % seed.len();
            let frame_index = index % feature_bundle.frame_count;
            let frame_hint_indices = feature_bundle
                .texture_hints
                .iter()
                .enumerate()
                .filter_map(|(hint_index, hint)| {
                    (hint.frame_index == frame_index).then_some(hint_index)
                })
                .collect::<Vec<_>>();
            let hint_index = if frame_hint_indices.is_empty() {
                u16_from_seed(seed, offset) as usize % feature_bundle.texture_hints.len()
            } else {
                frame_hint_indices[u16_from_seed(seed, offset) as usize % frame_hint_indices.len()]
            };
            let hint = &feature_bundle.texture_hints[hint_index];
            let jitter_x = ((seed[(offset + 3) % seed.len()] as i32 % 17) - 8) * 8;
            let jitter_y = ((seed[(offset + 5) % seed.len()] as i32 % 17) - 8) * 8;
            let x = (hint.x as i32 + jitter_x).clamp(0, max_x as i32) as u32;
            let y = (hint.y as i32 + jitter_y).clamp(0, max_y as i32) as u32;
            VideoVisualRegion {
                frame_index,
                x: (x / 8) * 8,
                y: (y / 8) * 8,
                width: region_width,
                height: region_height,
                strength: 0.09 + ((hint.score.min(255) as f32) / 255.0) * 0.03,
                redundancy_group: (index % 4) as u8,
            }
        })
        .collect()
}

fn derive_transcode_stable_regions(
    seed: &[u8; 32],
    feature_bundle: &VideoFeatureBundle,
    region_count: u32,
) -> Vec<VideoVisualRegion> {
    let region_width = (feature_bundle.width / 8).clamp(1, feature_bundle.width);
    let region_height = (feature_bundle.height / 8).clamp(1, feature_bundle.height);
    let max_x = feature_bundle.width.saturating_sub(region_width);
    let max_y = feature_bundle.height.saturating_sub(region_height);
    let safe_left = feature_bundle.width / 20;
    let safe_right = feature_bundle.width.saturating_sub(safe_left);
    let safe_top = feature_bundle.height / 20;
    let safe_bottom = feature_bundle.height.saturating_sub(safe_top);

    (0..region_count)
        .map(|index| {
            let frame_index = index % feature_bundle.frame_count;
            let candidates = transcode_stable_hint_indices(feature_bundle, frame_index);
            let hint_index = if candidates.is_empty() {
                let offset = ((index as usize) * 13) % seed.len();
                u16_from_seed(seed, offset) as usize % feature_bundle.texture_hints.len()
            } else {
                candidates[(index as usize / feature_bundle.frame_count.max(1) as usize)
                    % candidates.len()]
            };
            let hint = &feature_bundle.texture_hints[hint_index];
            let x = hint.x.clamp(
                safe_left.min(max_x),
                safe_right.saturating_sub(region_width).min(max_x),
            );
            let y = hint.y.clamp(
                safe_top.min(max_y),
                safe_bottom.saturating_sub(region_height).min(max_y),
            );
            VideoVisualRegion {
                frame_index,
                x: (x / 8) * 8,
                y: (y / 8) * 8,
                width: region_width,
                height: region_height,
                strength: 0.10 + ((hint.score.min(255) as f32) / 255.0) * 0.02,
                redundancy_group: (index % 4) as u8,
            }
        })
        .collect()
}

fn transcode_stable_hint_indices(
    feature_bundle: &VideoFeatureBundle,
    frame_index: u32,
) -> Vec<usize> {
    let mut frame_hints = feature_bundle
        .texture_hints
        .iter()
        .enumerate()
        .filter(|(_, hint)| hint.frame_index == frame_index)
        .collect::<Vec<_>>();
    if frame_hints.is_empty() {
        frame_hints = feature_bundle.texture_hints.iter().enumerate().collect();
    }
    if frame_hints.is_empty() {
        return Vec::new();
    }

    let min_score = frame_hints
        .iter()
        .map(|(_, hint)| hint.score)
        .min()
        .unwrap_or(0);
    let max_score = frame_hints
        .iter()
        .map(|(_, hint)| hint.score)
        .max()
        .unwrap_or(min_score);
    let span = max_score.saturating_sub(min_score);
    let lower = min_score + span / 4;
    let upper = min_score + (span * 3) / 4;
    let stable = frame_hints
        .iter()
        .filter_map(|(index, hint)| (hint.score >= lower && hint.score <= upper).then_some(*index))
        .collect::<Vec<_>>();
    if stable.is_empty() {
        let hint_count = frame_hints.len();
        frame_hints
            .into_iter()
            .skip(hint_count / 4)
            .take((hint_count / 2).max(1))
            .map(|(index, _)| index)
            .collect()
    } else {
        stable
    }
}

fn collect_video_visual_texture_hints(
    frames: &[VideoFramePlane],
    max_hints_per_frame: usize,
) -> Vec<VideoVisualTextureHint> {
    let mut hints = Vec::new();
    if max_hints_per_frame == 0 {
        return hints;
    }

    for (frame_index, frame) in frames.iter().enumerate() {
        if frame.width < 8 || frame.height < 8 {
            continue;
        }
        let mut frame_hints = Vec::new();
        for y in (0..=frame.height - 8).step_by(8) {
            for x in (0..=frame.width - 8).step_by(8) {
                let score = luma_texture_score_8x8(frame, x, y);
                if score > 0 {
                    frame_hints.push(VideoVisualTextureHint {
                        frame_index: frame_index as u32,
                        x,
                        y,
                        score,
                    });
                }
            }
        }
        frame_hints.sort_by(|left, right| {
            right
                .score
                .cmp(&left.score)
                .then_with(|| left.y.cmp(&right.y))
                .then_with(|| left.x.cmp(&right.x))
        });
        hints.extend(frame_hints.into_iter().take(max_hints_per_frame));
    }
    hints
}

fn luma_texture_score_8x8(frame: &VideoFramePlane, x: u32, y: u32) -> u32 {
    let mut score = 0u32;
    for row in 0..8u32 {
        for col in 0..8u32 {
            let current = frame.pixels
                [(y as usize + row as usize) * frame.stride + x as usize + col as usize]
                as i16;
            if col < 7 {
                let right = frame.pixels
                    [(y as usize + row as usize) * frame.stride + x as usize + col as usize + 1]
                    as i16;
                score += current.abs_diff(right) as u32;
            }
            if row < 7 {
                let down = frame.pixels
                    [(y as usize + row as usize + 1) * frame.stride + x as usize + col as usize]
                    as i16;
                score += current.abs_diff(down) as u32;
            }
        }
    }
    score
}

fn seeded_region_origin(seed: &[u8; 32], offset: usize, max_x: u32, max_y: u32) -> (u32, u32) {
    let x = if max_x == 0 {
        0
    } else {
        u16_from_seed(seed, offset + 2) as u32 % (max_x + 1)
    };
    let y = if max_y == 0 {
        0
    } else {
        u16_from_seed(seed, offset + 4) as u32 % (max_y + 1)
    };
    (x, y)
}

fn center_safe_grid_region_origin(
    index: u32,
    region_count: u32,
    max_x: u32,
    max_y: u32,
) -> (u32, u32) {
    grid_region_origin(index, region_count, max_x, max_y, 4, 5)
}

fn distributed_grid_region_origin(
    index: u32,
    region_count: u32,
    max_x: u32,
    max_y: u32,
) -> (u32, u32) {
    grid_region_origin(index, region_count, max_x, max_y, 0, 1)
}

fn grid_region_origin(
    index: u32,
    region_count: u32,
    max_x: u32,
    max_y: u32,
    margin_divisor: u32,
    span_divisor: u32,
) -> (u32, u32) {
    if region_count == 0 {
        return (0, 0);
    }
    let columns = (region_count as f64).sqrt().ceil().max(1.0) as u32;
    let rows = region_count.div_ceil(columns).max(1);
    let column = index % columns;
    let row = index / columns;
    let (left, right) = if span_divisor <= 1 {
        (0, max_x)
    } else {
        let margin_x = max_x / margin_divisor.max(1);
        (margin_x, max_x.saturating_sub(margin_x))
    };
    let (top, bottom) = if span_divisor <= 1 {
        (0, max_y)
    } else {
        let margin_y = max_y / margin_divisor.max(1);
        (margin_y, max_y.saturating_sub(margin_y))
    };
    let x_span = right.saturating_sub(left);
    let y_span = bottom.saturating_sub(top);
    let x = if columns <= 1 {
        left + x_span / 2
    } else {
        left + (x_span * column) / (columns - 1)
    };
    let y = if rows <= 1 {
        top + y_span / 2
    } else {
        top + (y_span * row.min(rows - 1)) / (rows - 1)
    };
    (x.min(max_x), y.min(max_y))
}

fn digest_strategy(
    task_id: &str,
    payload: &WatermarkPayload,
    feature_bundle: &VideoFeatureBundle,
    target_profile: VideoVisualProfile,
    expires_at: u64,
    self_check_threshold: f32,
    regions: &[VideoVisualRegion],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(VIDEO_VISUAL_STRATEGY_SCHEMA_VERSION.as_bytes());
    hasher.update(task_id.as_bytes());
    hasher.update(encode_payload(payload));
    hasher.update(feature_bundle.feature_digest);
    hasher.update(feature_bundle.source_video_sha256);
    hasher.update(target_profile.as_str().as_bytes());
    hasher.update(expires_at.to_be_bytes());
    hasher.update(self_check_threshold.to_bits().to_be_bytes());
    for region in regions {
        hasher.update(region.frame_index.to_be_bytes());
        hasher.update(region.x.to_be_bytes());
        hasher.update(region.y.to_be_bytes());
        hasher.update(region.width.to_be_bytes());
        hasher.update(region.height.to_be_bytes());
        hasher.update(region.strength.to_bits().to_be_bytes());
        hasher.update([region.redundancy_group]);
    }
    format!("sha256:{}", hex_lower(&hasher.finalize()))
}

fn strategy_offsets(
    frame_index: u32,
    frame: &VideoFramePlane,
    strategy: &VideoVisualStrategy,
) -> Result<Vec<usize>, WatermarkError> {
    if frame.profile != strategy.target_profile {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "frame profile must match the strategy target profile",
        ));
    }
    if strategy.regions.is_empty() || strategy.strategy_digest.trim().is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "strategy must contain digest and regions",
        ));
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut offsets = Vec::new();
    for region in strategy
        .regions
        .iter()
        .filter(|region| region.frame_index == frame_index)
    {
        if region.width == 0 || region.height == 0 {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::StrategyInvalid,
                "strategy regions must have non-zero dimensions",
            ));
        }
        let end_x = region.x.checked_add(region.width).ok_or_else(|| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::StrategyInvalid,
                "strategy region x range overflow",
            )
        })?;
        let end_y = region.y.checked_add(region.height).ok_or_else(|| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::StrategyInvalid,
                "strategy region y range overflow",
            )
        })?;
        if end_x > frame.width || end_y > frame.height {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::StrategyInvalid,
                "strategy region exceeds the synthetic frame bounds",
            ));
        }

        for y in region.y..end_y {
            for x in region.x..end_x {
                let offset = y as usize * frame.stride + x as usize;
                if seen.insert(offset) {
                    offsets.push(offset);
                }
            }
        }
    }

    if offsets.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "strategy has no regions for the requested frame",
        ));
    }

    Ok(offsets)
}

#[allow(dead_code)]
fn strategy_dct_blocks(
    frame_index: u32,
    frame: &VideoFramePlane,
    strategy: &VideoVisualStrategy,
) -> Result<Vec<(u32, u32)>, WatermarkError> {
    if frame.profile != strategy.target_profile {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "frame profile must match the strategy target profile",
        ));
    }
    if frame.width < 8 || frame.height < 8 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::UnsupportedVideoProfile,
            "DCT mid-band frames must be at least 8x8",
        ));
    }

    let mut seen = std::collections::BTreeSet::new();
    let mut blocks = Vec::new();
    for region in strategy
        .regions
        .iter()
        .filter(|region| region.frame_index == frame_index)
    {
        if region.width < 8 || region.height < 8 {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::StrategyInvalid,
                "DCT strategy regions must be at least 8x8",
            ));
        }
        let end_x = region.x.checked_add(region.width).ok_or_else(|| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::StrategyInvalid,
                "strategy region x range overflow",
            )
        })?;
        let end_y = region.y.checked_add(region.height).ok_or_else(|| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::StrategyInvalid,
                "strategy region y range overflow",
            )
        })?;
        if end_x > frame.width || end_y > frame.height {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::StrategyInvalid,
                "strategy region exceeds the DCT frame bounds",
            ));
        }

        let start_x = (region.x / 8) * 8;
        let start_y = (region.y / 8) * 8;
        let block_end_x = ((end_x / 8) * 8).min(frame.width.saturating_sub(7));
        let block_end_y = ((end_y / 8) * 8).min(frame.height.saturating_sub(7));
        for y in (start_y..block_end_y).step_by(8) {
            for x in (start_x..block_end_x).step_by(8) {
                if x + 8 <= frame.width && y + 8 <= frame.height && seen.insert((x, y)) {
                    blocks.push((x, y));
                }
            }
        }
    }

    if blocks.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "strategy has no DCT blocks for the requested frame",
        ));
    }
    Ok(blocks)
}

fn strategy_frame_indices(strategy: &VideoVisualStrategy) -> Vec<u32> {
    strategy
        .regions
        .iter()
        .map(|region| region.frame_index)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn bytes_to_bits(bytes: &[u8; PAYLOAD_BYTES]) -> Vec<bool> {
    bytes
        .iter()
        .flat_map(|byte| (0..8).rev().map(move |index| ((byte >> index) & 1) == 1))
        .collect()
}

fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
    bits.chunks(8)
        .map(|chunk| {
            let mut byte = 0u8;
            for (index, bit) in chunk.iter().enumerate() {
                if *bit {
                    byte |= 1 << (7 - index);
                }
            }
            byte
        })
        .collect()
}

fn normalize_luma_sample_to_u8(
    sample: u16,
    bit_depth: VideoLumaBitDepth,
    color_range: VideoLumaColorRange,
) -> u8 {
    let (min, max) = luma_sample_bounds(bit_depth, color_range);
    let clamped = sample.clamp(min, max);
    let numerator = (clamped - min) as u32 * 255;
    let denominator = (max - min).max(1) as u32;
    ((numerator + denominator / 2) / denominator) as u8
}

fn luma_sample_bounds(
    bit_depth: VideoLumaBitDepth,
    color_range: VideoLumaColorRange,
) -> (u16, u16) {
    let bits = bit_depth.bits() as u16;
    let max_full = (1u16 << bits) - 1;
    match color_range {
        VideoLumaColorRange::Full => (0, max_full),
        VideoLumaColorRange::Limited => {
            let shift = bits - 8;
            (16u16 << shift, 235u16 << shift)
        }
    }
}

#[allow(dead_code)]
pub(crate) fn dct_8x8_forward(block: &[f32; 64]) -> [f32; 64] {
    let mut coeffs = [0.0f32; 64];
    for v in 0..8 {
        for u in 0..8 {
            let mut sum = 0.0f32;
            for y in 0..8 {
                for x in 0..8 {
                    let pixel = block[y * 8 + x];
                    let cos_x =
                        (((2 * x + 1) as f32 * u as f32 * std::f32::consts::PI) / 16.0).cos();
                    let cos_y =
                        (((2 * y + 1) as f32 * v as f32 * std::f32::consts::PI) / 16.0).cos();
                    sum += pixel * cos_x * cos_y;
                }
            }
            coeffs[v * 8 + u] = 0.25 * dct_alpha(u) * dct_alpha(v) * sum;
        }
    }
    coeffs
}

#[allow(dead_code)]
pub(crate) fn dct_8x8_inverse(coeffs: &[f32; 64]) -> [f32; 64] {
    let mut block = [0.0f32; 64];
    for y in 0..8 {
        for x in 0..8 {
            let mut sum = 0.0f32;
            for v in 0..8 {
                for u in 0..8 {
                    let cos_x =
                        (((2 * x + 1) as f32 * u as f32 * std::f32::consts::PI) / 16.0).cos();
                    let cos_y =
                        (((2 * y + 1) as f32 * v as f32 * std::f32::consts::PI) / 16.0).cos();
                    sum += dct_alpha(u) * dct_alpha(v) * coeffs[v * 8 + u] * cos_x * cos_y;
                }
            }
            block[y * 8 + x] = 0.25 * sum;
        }
    }
    block
}

#[allow(dead_code)]
fn dct_alpha(index: usize) -> f32 {
    if index == 0 {
        std::f32::consts::FRAC_1_SQRT_2
    } else {
        1.0
    }
}

#[allow(dead_code)]
pub(crate) fn embed_luma_dct_mid_band_bit(
    coeffs: &mut [f32; 64],
    bit: bool,
    pair: ((usize, usize), (usize, usize)),
    delta: f32,
) -> Result<(), WatermarkError> {
    if delta <= 0.0 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT mid-band delta must be greater than zero",
        ));
    }

    let a = pair.0 .1 * 8 + pair.0 .0;
    let b = pair.1 .1 * 8 + pair.1 .0;
    if a >= coeffs.len() || b >= coeffs.len() || a == 0 || b == 0 || a == b {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT mid-band coefficient pair is invalid",
        ));
    }

    if bit {
        if coeffs[a] - coeffs[b] < delta {
            let midpoint = (coeffs[a] + coeffs[b]) / 2.0;
            coeffs[a] = midpoint + delta / 2.0;
            coeffs[b] = midpoint - delta / 2.0;
        }
    } else if coeffs[b] - coeffs[a] < delta {
        let midpoint = (coeffs[a] + coeffs[b]) / 2.0;
        coeffs[a] = midpoint - delta / 2.0;
        coeffs[b] = midpoint + delta / 2.0;
    }

    Ok(())
}

#[allow(dead_code)]
pub(crate) fn extract_luma_dct_mid_band_bit(
    coeffs: &[f32; 64],
    pair: ((usize, usize), (usize, usize)),
) -> Result<bool, WatermarkError> {
    let a = pair.0 .1 * 8 + pair.0 .0;
    let b = pair.1 .1 * 8 + pair.1 .0;
    if a >= coeffs.len() || b >= coeffs.len() || a == 0 || b == 0 || a == b {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT mid-band coefficient pair is invalid",
        ));
    }
    Ok(coeffs[a] >= coeffs[b])
}

#[allow(dead_code)]
pub(crate) fn encode_video_visual_bitstream(payload_bits: &[bool]) -> Vec<bool> {
    let mut stream = Vec::with_capacity(
        VIDEO_VISUAL_SYNC_MARKER_V1.len() + payload_bits.len() * VIDEO_VISUAL_ECC_REPEAT,
    );
    stream.extend_from_slice(&VIDEO_VISUAL_SYNC_MARKER_V1);
    for bit in payload_bits {
        stream.extend(std::iter::repeat(*bit).take(VIDEO_VISUAL_ECC_REPEAT));
    }
    stream
}

#[allow(dead_code)]
pub(crate) fn decode_video_visual_bitstream(stream: &[bool]) -> Result<Vec<bool>, WatermarkError> {
    let start = locate_video_visual_sync_marker(stream).ok_or_else(|| {
        WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "sync_marker_v1 was not found in DCT bitstream",
        )
    })?;
    let encoded = &stream[start + VIDEO_VISUAL_SYNC_MARKER_V1.len()..];
    if encoded.is_empty() || encoded.len() % VIDEO_VISUAL_ECC_REPEAT != 0 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "DCT bitstream ECC repeat groups are incomplete",
        ));
    }

    Ok(encoded
        .chunks(VIDEO_VISUAL_ECC_REPEAT)
        .map(|chunk| chunk.iter().filter(|bit| **bit).count() * 2 >= VIDEO_VISUAL_ECC_REPEAT)
        .collect())
}

fn locate_video_visual_sync_marker(stream: &[bool]) -> Option<usize> {
    stream
        .windows(VIDEO_VISUAL_SYNC_MARKER_V1.len())
        .enumerate()
        .filter_map(|(index, window)| {
            let distance = window
                .iter()
                .zip(VIDEO_VISUAL_SYNC_MARKER_V1.iter())
                .filter(|(left, right)| left != right)
                .count();
            (distance <= VIDEO_VISUAL_SYNC_MAX_BIT_ERRORS).then_some((index, distance))
        })
        .min_by_key(|(_, distance)| *distance)
        .map(|(index, _)| index)
}

#[allow(dead_code)]
fn decode_luma_dct_mid_band_payload_bits(
    payload_bits: &[bool],
) -> Result<WatermarkPayload, WatermarkError> {
    let payload_bytes = bits_to_bytes(payload_bits);
    let payload_bytes: [u8; PAYLOAD_BYTES] = payload_bytes.try_into().map_err(|_| {
        WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "DCT mid-band payload byte length is invalid",
        )
    })?;

    crate::decode_payload(&payload_bytes).map_err(|error| {
        WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            format!("DCT mid-band payload decode failed: {error}"),
        )
    })
}

#[allow(dead_code)]
fn extract_luma_dct_mid_band_payload_from_streams(
    streams: &[Vec<bool>],
) -> Result<WatermarkPayload, WatermarkError> {
    if streams.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "DCT mid-band multiframe fusion requires at least one bitstream",
        ));
    }
    let stream_len = streams[0].len();
    if stream_len == 0 || streams.iter().any(|stream| stream.len() != stream_len) {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "DCT mid-band multiframe fusion requires equal-length bitstreams",
        ));
    }

    let mut fused = Vec::with_capacity(stream_len);
    for bit_index in 0..stream_len {
        let true_count = streams.iter().filter(|stream| stream[bit_index]).count();
        fused.push(true_count * 2 >= streams.len());
    }

    let payload_bits = decode_video_visual_bitstream(&fused)?;
    decode_luma_dct_mid_band_payload_bits(&payload_bits)
}

#[allow(dead_code)]
pub(crate) fn embed_luma_dct_mid_band_bitstream(
    coeff_blocks: &mut [[f32; 64]],
    stream: &[bool],
    pairs: &[((usize, usize), (usize, usize))],
    delta: f32,
) -> Result<(), WatermarkError> {
    if pairs.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT mid-band bitstream requires at least one coefficient pair",
        ));
    }
    let capacity = coeff_blocks.len() * pairs.len();
    if stream.len() > capacity {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT mid-band bitstream exceeds block capacity",
        ));
    }

    for (bit_index, bit) in stream.iter().enumerate() {
        let block_index = bit_index / pairs.len();
        let pair = pairs[bit_index % pairs.len()];
        embed_luma_dct_mid_band_bit(&mut coeff_blocks[block_index], *bit, pair, delta)?;
    }
    Ok(())
}

#[allow(dead_code)]
pub(crate) fn extract_luma_dct_mid_band_bitstream(
    coeff_blocks: &[[f32; 64]],
    bit_count: usize,
    pairs: &[((usize, usize), (usize, usize))],
) -> Result<Vec<bool>, WatermarkError> {
    if pairs.is_empty() {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT mid-band bitstream requires at least one coefficient pair",
        ));
    }
    let capacity = coeff_blocks.len() * pairs.len();
    if bit_count > capacity {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "DCT mid-band bitstream read exceeds block capacity",
        ));
    }

    let mut bits = Vec::with_capacity(bit_count);
    for bit_index in 0..bit_count {
        let block_index = bit_index / pairs.len();
        let pair = pairs[bit_index % pairs.len()];
        bits.push(extract_luma_dct_mid_band_bit(
            &coeff_blocks[block_index],
            pair,
        )?);
    }
    Ok(bits)
}

#[allow(dead_code)]
fn embed_luma_dct_mid_band_stream_into_frame(
    frame: &mut VideoFramePlane,
    blocks: &[(u32, u32)],
    stream: &[bool],
    delta: f32,
) -> Result<(), WatermarkError> {
    let pairs = default_luma_dct_mid_band_pairs();
    let capacity = blocks.len() * pairs.len();
    if stream.len() > capacity {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT mid-band frame bitstream exceeds block capacity",
        ));
    }

    let repeat_count = dct_stream_repeat_count(capacity, stream.len());
    for repeat_index in 0..repeat_count {
        let stream_offset = repeat_index * stream.len();
        for (chunk_index, chunk) in stream.chunks(pairs.len()).enumerate() {
            let bit_offset = stream_offset + chunk_index * pairs.len();
            let block_index = bit_offset / pairs.len();
            let (x, y) = blocks[block_index];
            let block = read_luma_block(frame, x, y)?;
            let mut coeffs = dct_8x8_forward(&block);
            for (pair, bit) in pairs.iter().zip(chunk.iter()) {
                embed_luma_dct_mid_band_bit(&mut coeffs, *bit, *pair, delta)?;
            }
            write_luma_block(frame, x, y, &dct_8x8_inverse(&coeffs))?;
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn extract_luma_dct_mid_band_streams_from_frame(
    frame: &VideoFramePlane,
    blocks: &[(u32, u32)],
    stream_len: usize,
) -> Result<Vec<Vec<bool>>, WatermarkError> {
    let pairs = default_luma_dct_mid_band_pairs();
    let capacity = blocks.len() * pairs.len();
    if stream_len > capacity {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "DCT mid-band frame bitstream read exceeds block capacity",
        ));
    }

    let repeat_count = dct_stream_repeat_count(capacity, stream_len);
    let mut streams = Vec::with_capacity(repeat_count);
    for repeat_index in 0..repeat_count {
        streams.push(extract_luma_dct_mid_band_stream_from_frame_at(
            frame,
            blocks,
            stream_len,
            repeat_index * stream_len,
        )?);
    }
    Ok(streams)
}

fn dct_stream_repeat_count(capacity: usize, stream_len: usize) -> usize {
    (capacity / stream_len)
        .max(1)
        .min(VIDEO_VISUAL_DCT_MAX_STREAM_REPEATS)
}

#[allow(dead_code)]
fn extract_luma_dct_mid_band_stream_from_frame(
    frame: &VideoFramePlane,
    blocks: &[(u32, u32)],
    bit_count: usize,
) -> Result<Vec<bool>, WatermarkError> {
    extract_luma_dct_mid_band_stream_from_frame_at(frame, blocks, bit_count, 0)
}

fn extract_luma_dct_mid_band_stream_from_frame_at(
    frame: &VideoFramePlane,
    blocks: &[(u32, u32)],
    bit_count: usize,
    bit_offset: usize,
) -> Result<Vec<bool>, WatermarkError> {
    let pairs = default_luma_dct_mid_band_pairs();
    let capacity = blocks.len() * pairs.len();
    if bit_offset + bit_count > capacity {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::VisualExtractFailed,
            "DCT mid-band frame bitstream read exceeds block capacity",
        ));
    }

    let mut bits = Vec::with_capacity(bit_count);
    for (chunk_index, chunk_start) in (0..bit_count).step_by(pairs.len()).enumerate() {
        let chunk_len = (bit_count - chunk_start).min(pairs.len());
        let absolute_bit = bit_offset + chunk_index * pairs.len();
        let block_index = absolute_bit / pairs.len();
        let (x, y) = blocks[block_index];
        let block = read_luma_block(frame, x, y)?;
        let coeffs = dct_8x8_forward(&block);
        for pair in pairs.iter().take(chunk_len) {
            bits.push(extract_luma_dct_mid_band_bit(&coeffs, *pair)?);
        }
    }
    Ok(bits)
}

#[allow(dead_code)]
fn read_luma_block(frame: &VideoFramePlane, x: u32, y: u32) -> Result<[f32; 64], WatermarkError> {
    if x + 8 > frame.width || y + 8 > frame.height {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT block exceeds frame bounds",
        ));
    }

    let mut block = [0.0f32; 64];
    for row in 0..8usize {
        for col in 0..8usize {
            let offset = (y as usize + row) * frame.stride + x as usize + col;
            block[row * 8 + col] = frame.pixels[offset] as f32;
        }
    }
    Ok(block)
}

#[allow(dead_code)]
fn write_luma_block(
    frame: &mut VideoFramePlane,
    x: u32,
    y: u32,
    block: &[f32; 64],
) -> Result<(), WatermarkError> {
    if x + 8 > frame.width || y + 8 > frame.height {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::StrategyInvalid,
            "DCT block exceeds frame bounds",
        ));
    }

    for row in 0..8usize {
        for col in 0..8usize {
            let offset = (y as usize + row) * frame.stride + x as usize + col;
            frame.pixels[offset] = block[row * 8 + col].round().clamp(0.0, 255.0) as u8;
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn default_luma_dct_mid_band_pairs() -> [((usize, usize), (usize, usize)); 3] {
    [((1, 2), (2, 1)), ((1, 3), (3, 1)), ((2, 2), (3, 0))]
}

fn u16_from_seed(seed: &[u8; 32], offset: usize) -> u16 {
    let first = seed[offset % seed.len()];
    let second = seed[(offset + 1) % seed.len()];
    u16::from_be_bytes([first, second])
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_visual_payload_uses_core_identity_and_media_digest() {
        let source_sha = sha256_32(b"source-video");
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "creator",
            device_identity: "device",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();

        assert_eq!(&payload.original_hash_prefix, &source_sha[..16]);
        assert_eq!(
            crate::decode_payload(&encode_payload(&payload)).unwrap(),
            payload
        );
    }

    #[test]
    fn video_visual_reserved_payload_binds_registry_uid() {
        let source_sha = sha256_32(b"reserved-video");
        let payload =
            build_video_visual_payload_from_reserved_uid(VideoVisualReservedPayloadBuildInput {
                watermark_uid: "HS-10111213-20212223-30313233-40414243",
                creator_identity: "creator",
                source_video_sha256: source_sha,
                timestamp: 1_786_147_200,
                ai_flags: AIContentFlags::default(),
                registry_proof_hash: Some("sha256:aabbccddeeff00112233445566778899ffeeddcc"),
            })
            .unwrap();

        assert_eq!(
            payload.watermark_uid(),
            "HS-10111213-20212223-30313233-40414243"
        );
        assert_eq!(payload.issue_mode, WatermarkIssueMode::ServerReserved);
        assert_eq!(payload.media_type, WatermarkMediaType::VideoVisual);
        assert_eq!(&payload.original_hash_prefix, &source_sha[..16]);
        assert_eq!(
            hex_lower(&payload.registry_proof_hash),
            "aabbccddeeff00112233445566778899"
        );
    }

    #[test]
    fn video_visual_luma_dct_mid_band_profile_is_exposed() {
        assert_eq!(
            VideoVisualProfile::LumaDctMidBandV1.as_str(),
            "luma_dct_mid_band_v1"
        );
    }

    #[test]
    fn video_visual_dct_complexity_tiers_are_explicit() {
        let frames = dct_frames(16, 512, 512);
        let feature_bundle = dct_feature_bundle(&frames);

        let small = derive_video_visual_complexity_budget(
            VideoVisualComplexityTier::Small,
            &feature_bundle,
        )
        .unwrap();
        let standard = derive_video_visual_complexity_budget(
            VideoVisualComplexityTier::Standard,
            &feature_bundle,
        )
        .unwrap();
        let high =
            derive_video_visual_complexity_budget(VideoVisualComplexityTier::High, &feature_bundle)
                .unwrap();

        assert_eq!(small.tier.as_str(), "small");
        assert_eq!(small.sampled_frames, 4);
        assert_eq!(small.candidate_blocks_per_frame, 512);
        assert_eq!(small.selected_coeff_pairs, 3);
        assert_eq!(small.estimated_operations, 4 * 512 * 3);
        assert_eq!(small.max_roundtrip_ms, 1_500);

        assert_eq!(standard.sampled_frames, 8);
        assert_eq!(standard.candidate_blocks_per_frame, 768);
        assert_eq!(standard.estimated_operations, 8 * 768 * 3);
        assert_eq!(standard.max_roundtrip_ms, 3_000);

        assert_eq!(high.sampled_frames, 12);
        assert_eq!(high.candidate_blocks_per_frame, 1_024);
        assert_eq!(high.estimated_operations, 12 * 1_024 * 3);
        assert_eq!(high.max_roundtrip_ms, 6_000);
    }

    #[test]
    fn video_visual_dct_frame_sampling_is_even_and_deterministic() {
        let frames = dct_frames(10, 256, 256);
        let feature_bundle = dct_feature_bundle(&frames);
        let budget = derive_video_visual_complexity_budget(
            VideoVisualComplexityTier::Small,
            &feature_bundle,
        )
        .unwrap();

        let first = sample_video_visual_frame_indices(&feature_bundle, &budget).unwrap();
        let second = sample_video_visual_frame_indices(&feature_bundle, &budget).unwrap();

        assert_eq!(first, second);
        assert_eq!(first, vec![0, 3, 6, 9]);
    }

    #[test]
    fn video_visual_dct_complexity_rejects_non_dct_profile() {
        let frames = synthetic_frames(4, 128, 72);
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: sha256_32(b"synthetic-profile"),
            duration_ms: 1_000,
        })
        .unwrap();

        let result = derive_video_visual_complexity_budget(
            VideoVisualComplexityTier::Small,
            &feature_bundle,
        )
        .unwrap_err();

        assert_eq!(result.code(), WatermarkErrorCode::UnsupportedVideoProfile);
    }

    #[test]
    fn video_visual_decoded_y_plane_normalizes_limited_10_bit_with_stride() {
        let frame = fixed_decoded_limited_10bit_y_plane_fixture(16, 8, 20);
        let first_row = frame.visible_rows().next().unwrap();

        assert_eq!(frame.profile, VideoVisualProfile::LumaDctMidBandV1);
        assert_eq!(frame.width, 16);
        assert_eq!(frame.height, 8);
        assert_eq!(frame.stride, 16);
        assert_eq!(first_row[0], 0);
        assert_eq!(first_row[1], 1);
        assert_eq!(first_row[15], 15);
    }

    #[test]
    fn video_visual_decoded_y_plane_rejects_short_buffer() {
        let samples = vec![64u16; 16 * 7];
        let result = video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
            width: 16,
            height: 8,
            stride_samples: 16,
            samples: &samples,
            bit_depth: VideoLumaBitDepth::Ten,
            color_range: VideoLumaColorRange::Limited,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
        })
        .unwrap_err();

        assert_eq!(result.code(), WatermarkErrorCode::FeatureBundleInvalid);
    }

    #[test]
    fn video_visual_decoded_y_plane_rejects_synthetic_profile() {
        let samples = vec![0u16; 16 * 8];
        let result = video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
            width: 16,
            height: 8,
            stride_samples: 16,
            samples: &samples,
            bit_depth: VideoLumaBitDepth::Eight,
            color_range: VideoLumaColorRange::Full,
            target_profile: VideoVisualProfile::Luma8SyntheticV1,
        })
        .unwrap_err();

        assert_eq!(result.code(), WatermarkErrorCode::UnsupportedVideoProfile);
    }

    #[test]
    fn video_visual_fixed_y_plane_fixture_roundtrips_dct_payload() {
        let mut frames = (0..4)
            .map(|index| {
                let mut frame =
                    fixed_textured_decoded_limited_10bit_y_plane_fixture(1024, 1024, 1040, index);
                apply_uniform_luma_shift(std::slice::from_mut(&mut frame), index as i16);
                frame
            })
            .collect::<Vec<_>>();
        let source_sha = sha256_32(b"fixed-decoded-y-plane-source");
        let feature_bundle = dct_feature_bundle_with_source(&frames, source_sha);
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "creator",
            device_identity: "decoder-boundary-device",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();
        let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
            task_id: "task-fixed-y-plane",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: 96,
        })
        .unwrap();

        embed_luma_dct_mid_band_frames(&mut frames, &strategy, &payload).unwrap();
        let extracted = extract_luma_dct_mid_band_from_frames(&frames, &strategy).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn video_visual_dct_8x8_roundtrips_luma_block() {
        let block = sample_luma_block();
        let coeffs = dct_8x8_forward(&block);
        let restored = dct_8x8_inverse(&coeffs);

        for (original, decoded) in block.iter().zip(restored.iter()) {
            assert!(
                (original - decoded).abs() < 0.01,
                "DCT roundtrip mismatch: original={original}, decoded={decoded}"
            );
        }
    }

    #[test]
    fn video_visual_dct_mid_band_bit_embed_extracts_both_values() {
        let pair = ((2, 3), (3, 2));
        let block = sample_luma_block();

        let mut coeffs_one = dct_8x8_forward(&block);
        embed_luma_dct_mid_band_bit(&mut coeffs_one, true, pair, 18.0).unwrap();
        assert!(extract_luma_dct_mid_band_bit(&coeffs_one, pair).unwrap());

        let mut coeffs_zero = dct_8x8_forward(&block);
        embed_luma_dct_mid_band_bit(&mut coeffs_zero, false, pair, 18.0).unwrap();
        assert!(!extract_luma_dct_mid_band_bit(&coeffs_zero, pair).unwrap());

        let restored_one = dct_8x8_inverse(&coeffs_one);
        assert!(restored_one.iter().all(|value| value.is_finite()));
    }

    #[test]
    fn video_visual_bitstream_sync_marker_and_ecc_recover_payload_bits() {
        let payload_bits = vec![true, false, true, true, false, false, true, false];
        let mut stream = encode_video_visual_bitstream(&payload_bits);
        let payload_start = VIDEO_VISUAL_SYNC_MARKER_V1.len();
        stream[payload_start + 1] = !stream[payload_start + 1];
        stream[payload_start + 8] = !stream[payload_start + 8];

        let mut prefixed = vec![false, false, true, false];
        prefixed.extend(stream);
        let recovered = decode_video_visual_bitstream(&prefixed).unwrap();

        assert_eq!(recovered, payload_bits);
    }

    #[test]
    fn video_visual_bitstream_tolerates_small_sync_marker_damage() {
        let payload_bits = vec![true, false, true, true, false, false, true, false];
        let mut stream = encode_video_visual_bitstream(&payload_bits);
        stream[2] = !stream[2];
        stream[11] = !stream[11];

        let recovered = decode_video_visual_bitstream(&stream).unwrap();

        assert_eq!(recovered, payload_bits);
    }

    #[test]
    fn video_visual_dct_mid_band_bitstream_embed_extracts_with_sync_and_ecc() {
        let pairs = [((2, 3), (3, 2)), ((2, 4), (4, 2)), ((3, 4), (4, 3))];
        let payload_bits = vec![true, false, true, false, false, true, true, false];
        let stream = encode_video_visual_bitstream(&payload_bits);
        let block_count = stream.len().div_ceil(pairs.len());
        let block = sample_luma_block();
        let mut coeff_blocks = (0..block_count)
            .map(|index| {
                let mut varied = block;
                varied[index % 64] += index as f32;
                dct_8x8_forward(&varied)
            })
            .collect::<Vec<_>>();

        embed_luma_dct_mid_band_bitstream(&mut coeff_blocks, &stream, &pairs, 18.0).unwrap();
        let mut extracted =
            extract_luma_dct_mid_band_bitstream(&coeff_blocks, stream.len(), &pairs).unwrap();
        let payload_start = VIDEO_VISUAL_SYNC_MARKER_V1.len();
        extracted[payload_start + 2] = !extracted[payload_start + 2];

        let recovered = decode_video_visual_bitstream(&extracted).unwrap();
        assert_eq!(recovered, payload_bits);
    }

    #[test]
    fn video_visual_dct_mid_band_frame_roundtrips_payload_with_sync_and_ecc() {
        let (mut frames, payload, strategy) = dct_frame_fixture(1, 1024, 1024, 96, 1.0);

        let written_bits =
            embed_luma_dct_mid_band_frame(0, &mut frames[0], &strategy, &payload).unwrap();
        let extracted = extract_luma_dct_mid_band_frame(0, &frames[0], &strategy).unwrap();

        assert_eq!(
            written_bits,
            VIDEO_VISUAL_SYNC_MARKER_V1.len() + PAYLOAD_BYTES * 8 * VIDEO_VISUAL_ECC_REPEAT
        );
        assert_eq!(extracted, payload);
        assert_eq!(extracted.watermark_uid(), strategy.watermark_uid);
    }

    #[test]
    fn video_visual_dct_mid_band_frame_rejects_insufficient_capacity() {
        let (mut frames, payload, strategy) = dct_frame_fixture(1, 128, 128, 1, 1.0);

        let result =
            embed_luma_dct_mid_band_frame(0, &mut frames[0], &strategy, &payload).unwrap_err();

        assert_eq!(result.code(), WatermarkErrorCode::StrategyInvalid);
    }

    #[test]
    fn video_visual_dct_mid_band_frame_rejects_non_dct_profile() {
        let (frames, payload, strategy) = multiframe_fixture(1, 128, 64, 1.0);
        let mut frame = frames[0].clone();

        let result = embed_luma_dct_mid_band_frame(0, &mut frame, &strategy, &payload).unwrap_err();

        assert_eq!(result.code(), WatermarkErrorCode::UnsupportedVideoProfile);
    }

    #[test]
    fn video_visual_dct_mid_band_multiframe_self_check_roundtrips() {
        let (mut frames, payload, strategy) = dct_frame_fixture(4, 1024, 1024, 96, 0.75);

        let embedded = embed_luma_dct_mid_band_frames(&mut frames, &strategy, &payload).unwrap();
        let extracted = extract_luma_dct_mid_band_from_frames(&frames, &strategy).unwrap();
        let result = self_check_luma_dct_mid_band_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();

        assert_eq!(embedded, 4);
        assert_eq!(extracted, payload);
        assert!(result.passed);
        assert_eq!(result.checked_frames, 4);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn video_visual_dct_mid_band_multiframe_fuses_corrupted_payload_streams() {
        let (mut frames, payload, strategy) = dct_frame_fixture(5, 1024, 1024, 120, 1.0);
        embed_luma_dct_mid_band_frames(&mut frames, &strategy, &payload).unwrap();

        let frame_indices = strategy_frame_indices(&strategy);
        for (index, frame_index) in frame_indices.iter().enumerate() {
            let encoded_payload_bit = index * 11;
            let stream_len =
                VIDEO_VISUAL_SYNC_MARKER_V1.len() + PAYLOAD_BYTES * 8 * VIDEO_VISUAL_ECC_REPEAT;
            let blocks =
                strategy_dct_blocks(*frame_index, &frames[*frame_index as usize], &strategy)
                    .unwrap();
            let repeat_count = dct_stream_repeat_count(
                blocks.len() * default_luma_dct_mid_band_pairs().len(),
                stream_len,
            );
            let mut corrupted_positions = Vec::new();
            for repeat_index in 0..repeat_count {
                let payload_start = repeat_index * stream_len
                    + VIDEO_VISUAL_SYNC_MARKER_V1.len()
                    + encoded_payload_bit * VIDEO_VISUAL_ECC_REPEAT;
                corrupted_positions.extend([payload_start, payload_start + 1, payload_start + 2]);
            }
            flip_dct_stream_bits_in_frame(
                *frame_index,
                &mut frames[*frame_index as usize],
                &strategy,
                &corrupted_positions,
            );
        }

        for frame_index in &frame_indices {
            assert!(
                extract_luma_dct_mid_band_frame(
                    *frame_index,
                    &frames[*frame_index as usize],
                    &strategy
                )
                .is_err(),
                "single corrupted DCT frame should not decode"
            );
        }

        let extracted = extract_luma_dct_mid_band_from_frames(&frames, &strategy).unwrap();
        let result = self_check_luma_dct_mid_band_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();

        assert_eq!(extracted, payload);
        assert!(result.passed);
        assert_eq!(result.checked_frames, 5);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn video_visual_dct_mid_band_frame_fuses_repeated_payload_streams() {
        let (mut frames, payload, strategy) = dct_frame_fixture(1, 1536, 1536, 240, 1.0);
        embed_luma_dct_mid_band_frames(&mut frames, &strategy, &payload).unwrap();

        let stream_len =
            VIDEO_VISUAL_SYNC_MARKER_V1.len() + PAYLOAD_BYTES * 8 * VIDEO_VISUAL_ECC_REPEAT;
        let blocks = strategy_dct_blocks(0, &frames[0], &strategy).unwrap();
        assert_eq!(
            dct_stream_repeat_count(
                blocks.len() * default_luma_dct_mid_band_pairs().len(),
                stream_len
            ),
            3
        );

        for repeat_index in 0..3 {
            let encoded_payload_bit = repeat_index * 17;
            let payload_start = repeat_index * stream_len
                + VIDEO_VISUAL_SYNC_MARKER_V1.len()
                + encoded_payload_bit * VIDEO_VISUAL_ECC_REPEAT;
            flip_dct_stream_bits_in_frame(
                0,
                &mut frames[0],
                &strategy,
                &[payload_start, payload_start + 1, payload_start + 2],
            );
        }

        let streams =
            extract_luma_dct_mid_band_streams_from_frame(&frames[0], &blocks, stream_len).unwrap();
        assert_eq!(streams.len(), 3);
        for stream in &streams {
            assert!(
                extract_luma_dct_mid_band_payload_from_streams(std::slice::from_ref(stream))
                    .is_err()
            );
        }

        let extracted = extract_luma_dct_mid_band_frame(0, &frames[0], &strategy).unwrap();

        assert_eq!(extracted, payload);
    }

    #[test]
    fn video_visual_dct_public_staged_api_roundtrips() {
        let (mut frames, payload, strategy) = dct_frame_fixture(4, 1024, 1024, 96, 0.75);

        let embedded = embed_video_visual_dct_frames(&mut frames, &strategy, &payload).unwrap();
        let extracted = extract_video_visual_dct_from_frames(&frames, &strategy).unwrap();
        let result = self_check_video_visual_dct_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();

        assert_eq!(embedded, 4);
        assert_eq!(extracted, payload);
        assert!(result.passed);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn video_visual_dct_mid_band_multiframe_tolerates_missing_frames() {
        let (mut frames, payload, strategy) = dct_frame_fixture(4, 1024, 1024, 96, 0.5);
        embed_luma_dct_mid_band_frames(&mut frames, &strategy, &payload).unwrap();

        frames.truncate(2);

        let result = self_check_luma_dct_mid_band_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();

        assert!(result.passed);
        assert_eq!(result.checked_frames, 2);
        assert_eq!(result.confidence, 0.5);
    }

    #[test]
    fn video_visual_dct_mid_band_multiframe_detects_erased_frames() {
        let (mut frames, payload, strategy) = dct_frame_fixture(4, 1024, 1024, 96, 0.75);
        embed_luma_dct_mid_band_frames(&mut frames, &strategy, &payload).unwrap();
        for frame in &mut frames {
            frame.pixels.fill(127);
        }

        let result = self_check_luma_dct_mid_band_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap_err();

        assert_eq!(result.code(), WatermarkErrorCode::SelfCheckFailed);
    }

    #[test]
    fn video_visual_dct_mid_band_tolerates_uniform_luma_shift() {
        let (mut frames, payload, strategy) = dct_frame_fixture(4, 1024, 1024, 96, 0.75);
        embed_luma_dct_mid_band_frames(&mut frames, &strategy, &payload).unwrap();
        apply_uniform_luma_shift(&mut frames, 8);

        let result = self_check_luma_dct_mid_band_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();

        assert!(result.passed);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn video_visual_dct_mid_band_tolerates_conservative_quantization() {
        let (mut frames, payload, strategy) = dct_frame_fixture(4, 1024, 1024, 96, 0.75);
        embed_luma_dct_mid_band_frames(&mut frames, &strategy, &payload).unwrap();
        quantize_luma(&mut frames, 2);

        let result = self_check_luma_dct_mid_band_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();

        assert!(result.passed);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn video_visual_dct_mid_band_reports_v2_nearest_resample_boundary() {
        let (mut frames, payload, strategy) = dct_frame_fixture(6, 1024, 1024, 160, 0.75);
        embed_luma_dct_mid_band_frames(&mut frames, &strategy, &payload).unwrap();
        downsample_then_nearest_upsample(&mut frames, 4);

        let result = self_check_luma_dct_mid_band_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap_err();

        assert_eq!(result.code(), WatermarkErrorCode::SelfCheckFailed);
    }

    #[test]
    fn video_visual_dct_mid_band_performance_baseline_for_multiframe_roundtrip() {
        let (mut frames, payload, strategy) = dct_frame_fixture(4, 1024, 1024, 96, 0.75);
        let started = std::time::Instant::now();

        let embedded = embed_luma_dct_mid_band_frames(&mut frames, &strategy, &payload).unwrap();
        let extracted = extract_luma_dct_mid_band_from_frames(&frames, &strategy).unwrap();
        let result = self_check_luma_dct_mid_band_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();

        let elapsed = started.elapsed();
        assert_eq!(embedded, 4);
        assert_eq!(extracted, payload);
        assert!(result.passed);
        assert!(
            elapsed.as_millis() < 12_000,
            "DCT L3 core multiframe roundtrip took {elapsed:?}, expected < 12000ms"
        );
    }

    #[test]
    fn video_feature_bundle_is_deterministic_for_synthetic_frames() {
        let frames = synthetic_frames(4, 32, 24);
        let source_sha = sha256_32(b"source-video");

        let first = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: 2_000,
        })
        .unwrap();
        let second = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: 2_000,
        })
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(first.frame_count, 4);
    }

    #[test]
    fn video_visual_strategy_is_deterministic_and_self_checks() {
        let frames = synthetic_frames(6, 64, 40);
        let source_sha = sha256_32(b"source-video");
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: 3_000,
        })
        .unwrap();
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "creator",
            device_identity: "desktop-device",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();
        let input = VideoVisualStrategyBuildInput {
            task_id: "task-1",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::Luma8SyntheticV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: 16,
        };

        let first = derive_video_visual_strategy(input.clone()).unwrap();
        let second = derive_video_visual_strategy(input).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.schema_version, VIDEO_VISUAL_STRATEGY_SCHEMA_VERSION);
        assert_eq!(first.watermark_uid, payload.watermark_uid());
        assert!(first.strategy_digest.starts_with("sha256:"));

        let expected_checked = first
            .regions
            .iter()
            .map(|region| region.frame_index)
            .collect::<std::collections::BTreeSet<_>>()
            .len() as u32;
        let result = self_check_video_visual_watermark(VideoVisualSelfCheckInput {
            strategy: &first,
            observed_strategy_digest: &first.strategy_digest,
            checked_frames: expected_checked,
        })
        .unwrap();
        assert!(result.passed);
        assert_eq!(result.strategy_digest, first.strategy_digest);
    }

    #[test]
    fn video_visual_region_selection_modes_are_explicit() {
        let frames = dct_frames(16, 256, 144);
        let source_sha = sha256_32(b"source-video-region-selection");
        let feature_bundle = dct_feature_bundle_with_source(&frames, source_sha);
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "creator",
            device_identity: "desktop-device",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();
        let input = VideoVisualStrategyBuildInput {
            task_id: "task-region-selection",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: 16,
        };

        let default_strategy = derive_video_visual_strategy(input.clone()).unwrap();
        let seeded_strategy = derive_video_visual_strategy_with_region_selection(
            input.clone(),
            VideoVisualRegionSelectionMode::SeededRandom,
        )
        .unwrap();
        let center_strategy = derive_video_visual_strategy_with_region_selection(
            input.clone(),
            VideoVisualRegionSelectionMode::CenterSafeGrid,
        )
        .unwrap();
        let distributed_strategy = derive_video_visual_strategy_with_region_selection(
            input.clone(),
            VideoVisualRegionSelectionMode::DistributedGrid,
        )
        .unwrap();
        let texture_strategy = derive_video_visual_strategy_with_region_selection(
            input.clone(),
            VideoVisualRegionSelectionMode::TextureAware,
        )
        .unwrap();
        let transcode_stable_strategy = derive_video_visual_strategy_with_region_selection(
            input,
            VideoVisualRegionSelectionMode::TranscodeStable,
        )
        .unwrap();

        assert_eq!(default_strategy.regions, seeded_strategy.regions);
        assert_ne!(seeded_strategy.regions, center_strategy.regions);
        assert_ne!(center_strategy.regions, distributed_strategy.regions);
        assert_ne!(seeded_strategy.regions, texture_strategy.regions);
        assert_ne!(texture_strategy.regions, transcode_stable_strategy.regions);
        assert_eq!(
            VideoVisualRegionSelectionMode::SeededRandom.as_str(),
            "seeded_random"
        );
        assert_eq!(
            VideoVisualRegionSelectionMode::CenterSafeGrid.as_str(),
            "center_safe_grid"
        );
        assert_eq!(
            VideoVisualRegionSelectionMode::DistributedGrid.as_str(),
            "distributed_grid"
        );
        assert_eq!(
            VideoVisualRegionSelectionMode::TextureAware.as_str(),
            "texture_aware"
        );
        assert_eq!(
            VideoVisualRegionSelectionMode::TranscodeStable.as_str(),
            "transcode_stable"
        );
        assert!(texture_strategy
            .regions
            .iter()
            .all(|region| region.x % 8 == 0 && region.y % 8 == 0));
        assert!(transcode_stable_strategy
            .regions
            .iter()
            .all(|region| region.x % 8 == 0 && region.y % 8 == 0));
    }

    #[test]
    fn video_visual_default_region_selection_uses_transcode_stable_for_main_battlefield() {
        let source_sha = sha256_32(b"source-video-default-transcode-stable");
        let small_frames = dct_frames(4, 512, 512);
        let small_bundle = dct_feature_bundle_with_source(&small_frames, source_sha);
        let main_landscape_frames = dct_frames(4, 1920, 1080);
        let main_landscape_bundle =
            dct_feature_bundle_with_source(&main_landscape_frames, source_sha);
        let main_vertical_frames = dct_frames(4, 1080, 1920);
        let main_vertical_bundle =
            dct_feature_bundle_with_source(&main_vertical_frames, source_sha);
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "creator",
            device_identity: "desktop-device",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();

        let small_input = VideoVisualStrategyBuildInput {
            task_id: "task-default-small",
            payload: &payload,
            feature_bundle: &small_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: 16,
        };
        let small_default = derive_video_visual_strategy(small_input.clone()).unwrap();
        let small_seeded = derive_video_visual_strategy_with_region_selection(
            small_input,
            VideoVisualRegionSelectionMode::SeededRandom,
        )
        .unwrap();

        let main_landscape_input = VideoVisualStrategyBuildInput {
            task_id: "task-default-main-landscape",
            payload: &payload,
            feature_bundle: &main_landscape_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: 96,
        };
        let main_landscape_default =
            derive_video_visual_strategy(main_landscape_input.clone()).unwrap();
        let main_landscape_transcode_stable = derive_video_visual_strategy_with_region_selection(
            main_landscape_input,
            VideoVisualRegionSelectionMode::TranscodeStable,
        )
        .unwrap();

        let main_vertical_input = VideoVisualStrategyBuildInput {
            task_id: "task-default-main-vertical",
            payload: &payload,
            feature_bundle: &main_vertical_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 0.75,
            max_regions: 96,
        };
        let main_vertical_default =
            derive_video_visual_strategy(main_vertical_input.clone()).unwrap();
        let main_vertical_transcode_stable = derive_video_visual_strategy_with_region_selection(
            main_vertical_input,
            VideoVisualRegionSelectionMode::TranscodeStable,
        )
        .unwrap();

        assert_eq!(small_default.regions, small_seeded.regions);
        assert_eq!(
            main_landscape_default.regions,
            main_landscape_transcode_stable.regions
        );
        assert_eq!(
            main_vertical_default.regions,
            main_vertical_transcode_stable.regions
        );
    }

    #[test]
    fn video_visual_transcode_stable_regions_do_not_drift_with_task_id() {
        let source_sha = sha256_32(b"source-video-transcode-stable-task-id");
        let frames = dct_frames(4, 1920, 1080);
        let feature_bundle = dct_feature_bundle_with_source(&frames, source_sha);
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "creator",
            device_identity: "desktop-device",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();

        let first = derive_video_visual_strategy_with_region_selection(
            VideoVisualStrategyBuildInput {
                task_id: "task-transcode-stable-a",
                payload: &payload,
                feature_bundle: &feature_bundle,
                target_profile: VideoVisualProfile::LumaDctMidBandV1,
                expires_at: 1_786_150_000,
                self_check_threshold: 0.75,
                max_regions: 96,
            },
            VideoVisualRegionSelectionMode::TranscodeStable,
        )
        .unwrap();
        let second = derive_video_visual_strategy_with_region_selection(
            VideoVisualStrategyBuildInput {
                task_id: "task-transcode-stable-b",
                payload: &payload,
                feature_bundle: &feature_bundle,
                target_profile: VideoVisualProfile::LumaDctMidBandV1,
                expires_at: 1_786_150_000,
                self_check_threshold: 0.75,
                max_regions: 96,
            },
            VideoVisualRegionSelectionMode::TranscodeStable,
        )
        .unwrap();

        assert_eq!(first.regions, second.regions);
        assert_ne!(first.strategy_digest, second.strategy_digest);
    }

    #[test]
    fn video_visual_texture_hints_are_core_derived_and_deterministic() {
        let frames = dct_frames(4, 256, 144);
        let first = dct_feature_bundle(&frames);
        let second = dct_feature_bundle(&frames);

        assert_eq!(first.texture_hints, second.texture_hints);
        assert!(!first.texture_hints.is_empty());
        assert!(first.texture_hints.len() <= 4 * VIDEO_VISUAL_TEXTURE_HINTS_PER_FRAME);
        assert!(first
            .texture_hints
            .iter()
            .all(|hint| hint.x % 8 == 0 && hint.y % 8 == 0 && hint.score > 0));
    }

    #[test]
    fn video_visual_synthetic_frame_embed_extract_roundtrips_payload() {
        let mut frames = synthetic_frames(1, 256, 128);
        let source_sha = sha256_32(b"source-video");
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: 1_000,
        })
        .unwrap();
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "creator",
            device_identity: "desktop-device",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();
        let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
            task_id: "task-roundtrip",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::Luma8SyntheticV1,
            expires_at: 1_786_150_000,
            self_check_threshold: 1.0,
            max_regions: 32,
        })
        .unwrap();

        embed_video_visual_frame(0, &mut frames[0], &strategy, &payload).unwrap();
        let extracted = extract_video_visual_watermark(0, &frames[0], &strategy).unwrap();

        assert_eq!(extracted, payload);
        assert_eq!(extracted.watermark_uid(), strategy.watermark_uid);
    }

    #[test]
    fn video_visual_multiframe_embed_extract_and_self_check_roundtrip() {
        let (mut frames, payload, strategy) = multiframe_fixture(4, 192, 128, 0.75);

        let embedded = embed_video_visual_frames(&mut frames, &strategy, &payload).unwrap();
        assert!(embedded >= 4);

        let extracted = extract_video_visual_watermark_from_frames(&frames, &strategy).unwrap();
        assert_eq!(extracted, payload);

        let result = self_check_video_visual_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();
        assert!(result.passed);
        assert_eq!(result.checked_frames, embedded);
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn video_visual_multiframe_self_check_fails_when_confidence_drops() {
        let (mut frames, payload, strategy) = multiframe_fixture(4, 192, 128, 0.75);
        embed_video_visual_frames(&mut frames, &strategy, &payload).unwrap();

        erase_lsb_in_frames(&mut frames, 2);

        let result = self_check_video_visual_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap_err();

        assert_eq!(result.code(), WatermarkErrorCode::SelfCheckFailed);
    }

    #[test]
    fn video_visual_robustness_tolerates_missing_synthetic_frames() {
        let (mut frames, payload, strategy) = multiframe_fixture(6, 192, 128, 0.5);
        embed_video_visual_frames(&mut frames, &strategy, &payload).unwrap();

        frames.truncate(4);

        let result = self_check_video_visual_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();

        assert!(result.passed);
        assert!(result.confidence >= 0.5);
    }

    #[test]
    fn video_visual_robustness_tolerates_luma_offset_when_lsb_survives() {
        let (mut frames, payload, strategy) = multiframe_fixture(4, 192, 128, 0.75);
        embed_video_visual_frames(&mut frames, &strategy, &payload).unwrap();
        apply_luma_offset_preserving_lsb(&mut frames, 12);

        let extracted = extract_video_visual_watermark_from_frames(&frames, &strategy).unwrap();
        assert_eq!(extracted, payload);

        let result = self_check_video_visual_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();
        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn video_visual_robustness_detects_local_erasure() {
        let (mut frames, payload, strategy) = multiframe_fixture(4, 192, 128, 0.75);
        embed_video_visual_frames(&mut frames, &strategy, &payload).unwrap();
        erase_lsb_in_frames(&mut frames, 2);

        let result = self_check_video_visual_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap_err();

        assert_eq!(result.code(), WatermarkErrorCode::SelfCheckFailed);
    }

    #[test]
    fn video_visual_robustness_tolerates_edge_crop_simulation() {
        let (mut frames, payload, strategy) = multiframe_fixture(4, 160, 96, 0.75);
        embed_video_visual_frames(&mut frames, &strategy, &payload).unwrap();
        erase_synthetic_edge_crop_preserving_strategy_regions(&mut frames, &strategy, 8);

        let result = self_check_video_visual_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap();

        assert_eq!(result.confidence, 1.0);
    }

    #[test]
    fn video_visual_robustness_tolerates_quantized_compression_when_lsb_survives() {
        let (mut frames, payload, strategy) = multiframe_fixture(4, 160, 96, 0.75);
        embed_video_visual_frames(&mut frames, &strategy, &payload).unwrap();
        quantize_luma_preserving_lsb(&mut frames, 16);

        let extracted = extract_video_visual_watermark_from_frames(&frames, &strategy).unwrap();
        assert_eq!(extracted, payload);
    }

    #[test]
    fn video_visual_robustness_detects_destructive_compression_simulation() {
        let (mut frames, payload, strategy) = multiframe_fixture(4, 160, 96, 0.75);
        embed_video_visual_frames(&mut frames, &strategy, &payload).unwrap();
        quantize_luma_clearing_lsb(&mut frames, 16);

        let result = self_check_video_visual_frames(VideoVisualSelfCheckFramesInput {
            strategy: &strategy,
            observed_strategy_digest: &strategy.strategy_digest,
            frames: &frames,
            expected_payload: &payload,
        })
        .unwrap_err();

        assert_eq!(result.code(), WatermarkErrorCode::SelfCheckFailed);
    }

    #[test]
    fn video_visual_performance_baseline_for_synthetic_multiframe_roundtrip() {
        let budgets = [
            (4, 192, 128, 500u128),
            (12, 256, 144, 900u128),
            (24, 320, 180, 2_800u128),
        ];

        for (frame_count, width, height, budget_ms) in budgets {
            let (mut frames, payload, strategy) =
                multiframe_fixture(frame_count, width, height, 0.75);
            let started = std::time::Instant::now();

            let embedded = embed_video_visual_frames(&mut frames, &strategy, &payload).unwrap();
            let extracted = extract_video_visual_watermark_from_frames(&frames, &strategy).unwrap();
            let result = self_check_video_visual_frames(VideoVisualSelfCheckFramesInput {
                strategy: &strategy,
                observed_strategy_digest: &strategy.strategy_digest,
                frames: &frames,
                expected_payload: &payload,
            })
            .unwrap();

            let elapsed = started.elapsed();
            assert_eq!(embedded, frame_count);
            assert_eq!(extracted, payload);
            assert!(result.passed);
            assert!(
                elapsed.as_millis() < budget_ms,
                "synthetic L3 core roundtrip for {frame_count} frames at {width}x{height} took {elapsed:?}, expected < {budget_ms}ms"
            );
        }
    }

    #[test]
    fn video_visual_spike_returns_l3_error_codes() {
        let empty = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &[],
            source_video_sha256: sha256_32(b"source"),
            duration_ms: 0,
        })
        .unwrap_err();
        assert_eq!(empty.code(), WatermarkErrorCode::FeatureBundleInvalid);

        let bad_frame = VideoFramePlane::new_luma8(10, 10, 4, vec![0; 40]).unwrap_err();
        assert_eq!(
            bad_frame.code(),
            WatermarkErrorCode::UnsupportedVideoProfile
        );

        let frames = synthetic_frames(2, 32, 24);
        let source_sha = sha256_32(b"source-video");
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: 1_000,
        })
        .unwrap();
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "creator",
            device_identity: "device",
            source_video_sha256: source_sha,
            timestamp: 1,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();
        let invalid_strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
            task_id: "",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::Luma8SyntheticV1,
            expires_at: 1,
            self_check_threshold: 0.8,
            max_regions: 4,
        })
        .unwrap_err();
        assert_eq!(invalid_strategy.code(), WatermarkErrorCode::StrategyInvalid);

        let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
            task_id: "task-1",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::Luma8SyntheticV1,
            expires_at: 1,
            self_check_threshold: 0.8,
            max_regions: 4,
        })
        .unwrap();
        let self_check = self_check_video_visual_watermark(VideoVisualSelfCheckInput {
            strategy: &strategy,
            observed_strategy_digest: "sha256:not-the-same",
            checked_frames: 1,
        })
        .unwrap_err();
        assert_eq!(self_check.code(), WatermarkErrorCode::SelfCheckFailed);

        let frame_index = strategy.regions[0].frame_index;
        let extract =
            extract_video_visual_watermark(frame_index, &frames[frame_index as usize], &strategy)
                .unwrap_err();
        assert_eq!(extract.code(), WatermarkErrorCode::VisualExtractFailed);

        let mut frame = frames[0].clone();
        let other_payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "other-creator",
            device_identity: "device",
            source_video_sha256: source_sha,
            timestamp: 1,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();
        let embed = embed_video_visual_frame(0, &mut frame, &strategy, &other_payload).unwrap_err();
        assert_eq!(embed.code(), WatermarkErrorCode::StrategyInvalid);
    }

    fn synthetic_frames(count: u32, width: u32, height: u32) -> Vec<VideoFramePlane> {
        (0..count)
            .map(|frame_index| {
                let pixels = (0..height)
                    .flat_map(|y| {
                        (0..width).map(move |x| ((x * 3 + y * 5 + frame_index * 17) % 251) as u8)
                    })
                    .collect::<Vec<_>>();
                VideoFramePlane::new_luma8(width, height, width as usize, pixels).unwrap()
            })
            .collect()
    }

    fn dct_frames(count: u32, width: u32, height: u32) -> Vec<VideoFramePlane> {
        (0..count)
            .map(|frame_index| {
                let pixels = (0..height)
                    .flat_map(|y| {
                        (0..width).map(move |x| {
                            let block_bias = ((x / 8 + y / 8 + frame_index) % 17) as u8;
                            48u8.saturating_add(((x * 5 + y * 7) % 151) as u8)
                                .saturating_add(block_bias)
                        })
                    })
                    .collect::<Vec<_>>();
                VideoFramePlane::new_luma_dct_mid_band(width, height, width as usize, pixels)
                    .unwrap()
            })
            .collect()
    }

    fn fixed_decoded_limited_10bit_y_plane_fixture(
        width: u32,
        height: u32,
        stride_samples: usize,
    ) -> VideoFramePlane {
        let mut samples = vec![0u16; stride_samples * height as usize];
        for y in 0..height as usize {
            for x in 0..width as usize {
                let normalized = ((x + y * 3) % 256) as u32;
                samples[y * stride_samples + x] = (64 + ((normalized * 876 + 127) / 255)) as u16;
            }
            for x in width as usize..stride_samples {
                samples[y * stride_samples + x] = 940;
            }
        }
        video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
            width,
            height,
            stride_samples,
            samples: &samples,
            bit_depth: VideoLumaBitDepth::Ten,
            color_range: VideoLumaColorRange::Limited,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
        })
        .unwrap()
    }

    fn fixed_textured_decoded_limited_10bit_y_plane_fixture(
        width: u32,
        height: u32,
        stride_samples: usize,
        frame_index: u32,
    ) -> VideoFramePlane {
        let mut samples = vec![0u16; stride_samples * height as usize];
        for y in 0..height {
            for x in 0..width {
                let block_bias = ((x / 8 + y / 8 + frame_index) % 17) as u32;
                let normalized = 48 + ((x * 5 + y * 7) % 151) + block_bias;
                samples[y as usize * stride_samples + x as usize] =
                    (64 + ((normalized * 876 + 127) / 255)) as u16;
            }
            for x in width as usize..stride_samples {
                samples[y as usize * stride_samples + x] = 940;
            }
        }
        video_frame_plane_from_decoded_luma(DecodedVideoLumaPlane {
            width,
            height,
            stride_samples,
            samples: &samples,
            bit_depth: VideoLumaBitDepth::Ten,
            color_range: VideoLumaColorRange::Limited,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
        })
        .unwrap()
    }

    fn multiframe_fixture(
        count: u32,
        width: u32,
        height: u32,
        self_check_threshold: f32,
    ) -> (Vec<VideoFramePlane>, WatermarkPayload, VideoVisualStrategy) {
        let frames = synthetic_frames(count, width, height);
        let source_sha = sha256_32(b"source-video-multiframe");
        let feature_bundle = build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames: &frames,
            source_video_sha256: source_sha,
            duration_ms: 4_000,
        })
        .unwrap();
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "creator",
            device_identity: "desktop-device",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();
        let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
            task_id: "task-multiframe",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::Luma8SyntheticV1,
            expires_at: 1_786_150_000,
            self_check_threshold,
            max_regions: count * 64,
        })
        .unwrap();
        (frames, payload, strategy)
    }

    fn dct_frame_fixture(
        count: u32,
        width: u32,
        height: u32,
        max_regions: u32,
        self_check_threshold: f32,
    ) -> (Vec<VideoFramePlane>, WatermarkPayload, VideoVisualStrategy) {
        let frames = dct_frames(count, width, height);
        let source_sha = sha256_32(b"source-video-dct-frame");
        let feature_bundle = dct_feature_bundle_with_source(&frames, source_sha);
        let payload = build_video_visual_payload(VideoVisualPayloadBuildInput {
            creator_identity: "creator",
            device_identity: "desktop-device",
            source_video_sha256: source_sha,
            timestamp: 1_786_147_200,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();
        let strategy = derive_video_visual_strategy(VideoVisualStrategyBuildInput {
            task_id: "task-dct-frame",
            payload: &payload,
            feature_bundle: &feature_bundle,
            target_profile: VideoVisualProfile::LumaDctMidBandV1,
            expires_at: 1_786_150_000,
            self_check_threshold,
            max_regions,
        })
        .unwrap();
        (frames, payload, strategy)
    }

    fn dct_feature_bundle(frames: &[VideoFramePlane]) -> VideoFeatureBundle {
        dct_feature_bundle_with_source(frames, sha256_32(b"source-video-dct-feature-bundle"))
    }

    fn dct_feature_bundle_with_source(
        frames: &[VideoFramePlane],
        source_sha: [u8; 32],
    ) -> VideoFeatureBundle {
        build_video_feature_bundle(VideoFeatureBundleBuildInput {
            frames,
            source_video_sha256: source_sha,
            duration_ms: 4_000,
        })
        .unwrap()
    }

    fn erase_lsb_in_frames(frames: &mut [VideoFramePlane], count: usize) {
        for frame in frames.iter_mut().take(count) {
            for pixel in frame.pixels.iter_mut() {
                *pixel &= 0b1111_1110;
            }
        }
    }

    fn apply_luma_offset_preserving_lsb(frames: &mut [VideoFramePlane], offset: u8) {
        for frame in frames {
            for pixel in &mut frame.pixels {
                let bit = *pixel & 1;
                *pixel = pixel.saturating_add(offset) & 0b1111_1110 | bit;
            }
        }
    }

    fn erase_synthetic_edge_crop_preserving_strategy_regions(
        frames: &mut [VideoFramePlane],
        strategy: &VideoVisualStrategy,
        margin: u32,
    ) {
        let protected = strategy
            .regions
            .iter()
            .map(|region| {
                (
                    region.frame_index,
                    region.x,
                    region.y,
                    region.x + region.width,
                    region.y + region.height,
                )
            })
            .collect::<Vec<_>>();

        for (frame_index, frame) in frames.iter_mut().enumerate() {
            for y in 0..frame.height {
                for x in 0..frame.width {
                    let in_edge = x < margin
                        || y < margin
                        || x >= frame.width.saturating_sub(margin)
                        || y >= frame.height.saturating_sub(margin);
                    let in_strategy_region =
                        protected
                            .iter()
                            .any(|(region_frame, left, top, right, bottom)| {
                                *region_frame == frame_index as u32
                                    && x >= *left
                                    && x < *right
                                    && y >= *top
                                    && y < *bottom
                            });
                    if in_edge && !in_strategy_region {
                        let offset = y as usize * frame.stride + x as usize;
                        frame.pixels[offset] = 0;
                    }
                }
            }
        }
    }

    fn quantize_luma_preserving_lsb(frames: &mut [VideoFramePlane], step: u8) {
        for frame in frames {
            for pixel in &mut frame.pixels {
                let bit = *pixel & 1;
                *pixel = (*pixel / step) * step | bit;
            }
        }
    }

    fn quantize_luma_clearing_lsb(frames: &mut [VideoFramePlane], step: u8) {
        for frame in frames {
            for pixel in &mut frame.pixels {
                *pixel = ((*pixel / step) * step) & 0b1111_1110;
            }
        }
    }

    fn flip_dct_stream_bits_in_frame(
        frame_index: u32,
        frame: &mut VideoFramePlane,
        strategy: &VideoVisualStrategy,
        bit_indices: &[usize],
    ) {
        let pairs = default_luma_dct_mid_band_pairs();
        let blocks = strategy_dct_blocks(frame_index, frame, strategy).unwrap();
        for bit_index in bit_indices {
            let block_index = *bit_index / pairs.len();
            let pair = pairs[*bit_index % pairs.len()];
            let (x, y) = blocks[block_index];
            let block = read_luma_block(frame, x, y).unwrap();
            let mut coeffs = dct_8x8_forward(&block);
            let current = extract_luma_dct_mid_band_bit(&coeffs, pair).unwrap();
            embed_luma_dct_mid_band_bit(&mut coeffs, !current, pair, 24.0).unwrap();
            write_luma_block(frame, x, y, &dct_8x8_inverse(&coeffs)).unwrap();
        }
    }

    fn apply_uniform_luma_shift(frames: &mut [VideoFramePlane], delta: i16) {
        for frame in frames {
            for pixel in &mut frame.pixels {
                *pixel = ((*pixel as i16) + delta).clamp(0, 255) as u8;
            }
        }
    }

    fn quantize_luma(frames: &mut [VideoFramePlane], step: u8) {
        for frame in frames {
            for pixel in &mut frame.pixels {
                *pixel = (*pixel / step) * step;
            }
        }
    }

    fn downsample_then_nearest_upsample(frames: &mut [VideoFramePlane], factor: u32) {
        for frame in frames {
            let original = frame.pixels.clone();
            for y in 0..frame.height {
                for x in 0..frame.width {
                    let sample_x = (x / factor) * factor;
                    let sample_y = (y / factor) * factor;
                    let source = sample_y as usize * frame.stride + sample_x as usize;
                    let target = y as usize * frame.stride + x as usize;
                    frame.pixels[target] = original[source];
                }
            }
        }
    }

    fn sha256_32(bytes: &[u8]) -> [u8; 32] {
        Sha256::digest(bytes).into()
    }

    fn sample_luma_block() -> [f32; 64] {
        let mut block = [0.0f32; 64];
        for y in 0..8 {
            for x in 0..8 {
                block[y * 8 + x] = 32.0 + (x as f32 * 5.0) + (y as f32 * 3.0);
            }
        }
        block
    }
}

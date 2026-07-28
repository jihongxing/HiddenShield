mod audio;
mod delivery_envelope;
mod error;
#[allow(dead_code)]
mod image;
pub mod image_spatial_recovery_v1;
mod payload;
pub mod quality;
mod service;
mod v3_internal_qa;
mod v3_readonly_fixture;
mod video_visual;

pub use audio::{
    audio_v3_quality_diagnostics, build_v3_readonly_candidate_audio_fixture_wav_bytes,
    embed_watermark, embed_watermark_samples, embed_watermark_samples_allow_rewrite,
    embed_watermark_samples_allow_rewrite_with_delta, embed_watermark_samples_with_delta,
    embed_watermark_wav_bytes, embed_watermark_wav_bytes_allow_rewrite,
    embed_watermark_wav_bytes_allow_rewrite_with_delta,
    embed_watermark_wav_bytes_allow_rewrite_with_delta_without_min_duration,
    embed_watermark_wav_bytes_with_delta,
    extract_audio_noise_floor_migrated_band_v1_candidate_samples_with_rate,
    extract_audio_noise_floor_migrated_band_v1_candidate_wav_bytes, extract_watermark,
    extract_watermark_samples, extract_watermark_samples_readonly_candidate,
    extract_watermark_samples_readonly_candidate_with_delta,
    extract_watermark_samples_readonly_candidate_with_delta_and_rate,
    extract_watermark_samples_with_delta, extract_watermark_wav_bytes,
    extract_watermark_wav_bytes_with_delta, extract_watermark_wav_readonly_candidate_bytes,
    extract_watermark_wav_readonly_candidate_bytes_with_delta, validate_audio_protection_file_size,
    validate_audio_protection_input, AudioNoiseFloorMigrationCandidateFailureCode,
    AudioNoiseFloorMigrationCandidateReadError, AudioV3QualityDiagnostics,
    AUDIO_NOISE_FLOOR_CANDIDATE_FALLBACK_PATH, AUDIO_NOISE_FLOOR_CANDIDATE_READ_COMPAT_MODE,
    AUDIO_NOISE_FLOOR_LEGACY_V3_FALLBACK_PATH, AUDIO_NOISE_FLOOR_MIGRATED_BAND_V1_CANDIDATE_PATH,
    MAX_AUDIO_PROTECTION_BYTES, MAX_AUDIO_PROTECTION_SECONDS, MAX_SUPPORTED_AUDIO_CHANNELS,
    MAX_SUPPORTED_AUDIO_SAMPLE_RATE, MIN_AUDIO_PROTECTION_SECONDS, MIN_SUPPORTED_AUDIO_CHANNELS,
    MIN_SUPPORTED_AUDIO_SAMPLE_RATE,
};
pub use delivery_envelope::{
    ai_delivery_envelope_digest, ai_delivery_profile_identity_digest,
    ai_delivery_retrieval_receipt_digest, canonical_json_sha256, seal_ai_delivery_envelope,
    seal_ai_delivery_retrieval_receipt, validate_ai_delivery_envelope, validate_ai_delivery_import,
    AiConfirmedArtifactDeliveryEnvelope, AiDeliveryEnvelopeError, AiDeliveryEnvelopeErrorCode,
    AiDeliveryEnvelopeValidationResult, AiDeliveryImportAdmission, AiDeliveryProfileIdentity,
    AiDeliveryRetrievalReceipt, AI_DELIVERY_ENVELOPE_SCHEMA_VERSION,
    AI_DELIVERY_RETRIEVAL_RECEIPT_SCHEMA_VERSION,
};
pub use error::{WatermarkError, WatermarkErrorCode};
pub use image::{
    build_v3_readonly_candidate_image_fixture_png_bytes, embed_image_v3_bytes,
    extract_image_v3_bytes, extract_image_watermark_readonly_candidate_bytes,
    extract_image_watermark_readonly_candidate_bytes_with_alpha, image_embed_capacity_sufficient,
    validate_image_protection_file_size, validate_image_protection_input, ImageReferenceRecovery,
    MAX_IMAGE_PROTECTION_BYTES, MAX_IMAGE_PROTECTION_PIXELS,
};
pub use payload::{
    decode_payload, decode_payload_v3_minimal_anchor, decode_watermark_payload_readonly,
    encode_payload, encode_payload_v3_minimal_anchor, generate_offline_watermark_id,
    registry_proof_hash_from_hex, watermark_id_from_uid, AIContentFlags, AuthenticityClaim,
    GenerationMethod, IdentityBuildInput, ModificationLevel, PayloadBuildInput,
    PayloadDigestBuildInput, PayloadV2BuildInput, PayloadV3MinimalAnchorBuildInput,
    TrainingPermission, WatermarkDecodedPayload, WatermarkIdentity, WatermarkIssueMode,
    WatermarkMediaType, WatermarkPayload, WatermarkPayloadV3MinimalAnchor, PAYLOAD_BYTES,
    PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
};
pub use quality::{
    compare_audio_quality, compare_image_quality, AudioBandEnergyReport, AudioPerceptualDiagnosis,
    AudioQualityInput, AudioQualityReport, AudioSegmentSnrReport, ImageQualityInput,
    ImageQualityReport, QualityThresholdProfile, QualityThresholdResult,
    AUDIO_BALANCED_MAX_LUFS_DELTA, AUDIO_BALANCED_MAX_PEAK_DELTA, AUDIO_BALANCED_MIN_SNR,
    AUDIO_FORENSIC_MAX_LUFS_DELTA, AUDIO_FORENSIC_MAX_PEAK_DELTA, AUDIO_FORENSIC_MIN_SNR,
    AUDIO_MAX_NEW_CLIPPING, AUDIO_RELEASE_MAX_LUFS_DELTA, AUDIO_RELEASE_MAX_PEAK_DELTA,
    AUDIO_RELEASE_MIN_SNR, IMAGE_BALANCED_MIN_PSNR, IMAGE_BALANCED_MIN_SSIM,
    IMAGE_FORENSIC_MIN_PSNR, IMAGE_FORENSIC_MIN_SSIM, IMAGE_RELEASE_MIN_PSNR,
    IMAGE_RELEASE_MIN_SSIM,
};
pub use service::{
    AudioProtectionMode, EmbedOptions, ImageOutputFormat, MediaInput, MediaOutput,
    PayloadWriteMode, WatermarkService, WatermarkStrength,
};
pub use v3_internal_qa::{
    embed_v3_internal_qa_media, V3InternalQaMediaKind, V3InternalQaWriteGate,
    V3InternalQaWriteInput, V3InternalQaWriteOutput,
};
pub use v3_readonly_fixture::{
    embed_v3_readonly_anchor_png_bytes, embed_v3_readonly_anchor_wav_bytes,
    extract_v3_readonly_anchor_png_bytes, extract_v3_readonly_anchor_wav_bytes,
};
pub use video_visual::{
    build_video_feature_bundle, build_video_visual_payload,
    build_video_visual_payload_from_reserved_uid, derive_video_visual_complexity_budget,
    derive_video_visual_strategy, derive_video_visual_strategy_with_region_selection,
    embed_video_visual_dct_frames, embed_video_visual_frame, embed_video_visual_frames,
    extract_video_visual_dct_from_frames, extract_video_visual_watermark,
    extract_video_visual_watermark_from_frames, sample_video_visual_frame_indices,
    self_check_video_visual_dct_frames, self_check_video_visual_frames,
    self_check_video_visual_watermark, video_frame_plane_from_decoded_luma, DecodedVideoLumaPlane,
    VideoFeatureBundle, VideoFeatureBundleBuildInput, VideoFramePlane, VideoLumaBitDepth,
    VideoLumaColorRange, VideoVisualComplexityBudget, VideoVisualComplexityTier,
    VideoVisualPayloadBuildInput, VideoVisualProfile, VideoVisualRegion,
    VideoVisualRegionSelectionMode, VideoVisualReservedPayloadBuildInput,
    VideoVisualSelfCheckFramesInput, VideoVisualSelfCheckInput, VideoVisualSelfCheckResult,
    VideoVisualStrategy, VideoVisualStrategyBuildInput, VideoVisualTextureHint,
    VIDEO_VISUAL_STRATEGY_SCHEMA_VERSION,
};

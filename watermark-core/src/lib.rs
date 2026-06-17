mod audio;
mod error;
mod image;
mod payload;
mod service;

pub use audio::{
    embed_watermark, embed_watermark_samples, embed_watermark_samples_allow_rewrite,
    embed_watermark_samples_allow_rewrite_with_delta, embed_watermark_samples_with_delta,
    embed_watermark_wav_bytes, embed_watermark_wav_bytes_allow_rewrite,
    embed_watermark_wav_bytes_allow_rewrite_with_delta,
    embed_watermark_wav_bytes_allow_rewrite_with_delta_without_min_duration,
    embed_watermark_wav_bytes_with_delta, extract_watermark, extract_watermark_samples,
    extract_watermark_samples_with_delta, extract_watermark_wav_bytes,
    extract_watermark_wav_bytes_with_delta,
};
pub use error::WatermarkError;
pub use image::{
    embed_image_watermark, embed_image_watermark_allow_rewrite, embed_image_watermark_bytes,
    embed_image_watermark_bytes_allow_rewrite,
    embed_image_watermark_bytes_allow_rewrite_with_alpha, embed_image_watermark_bytes_with_alpha,
    extract_image_watermark, extract_image_watermark_bytes,
    extract_image_watermark_bytes_with_alpha,
};
pub use payload::{
    decode_payload, encode_payload, AIContentFlags, AuthenticityClaim, GenerationMethod,
    ModificationLevel, TrainingPermission, WatermarkPayload,
};
pub use service::{
    AudioProtectionMode, EmbedOptions, ImageOutputFormat, MediaInput, MediaOutput,
    WatermarkService, WatermarkStrength,
};

mod audio;
mod error;
mod image;
mod payload;
mod service;

pub use audio::{
    embed_watermark, embed_watermark_samples, embed_watermark_samples_allow_rewrite,
    embed_watermark_wav_bytes, embed_watermark_wav_bytes_allow_rewrite, extract_watermark,
    extract_watermark_samples, extract_watermark_wav_bytes,
};
pub use error::WatermarkError;
pub use image::{
    embed_image_watermark, embed_image_watermark_allow_rewrite, embed_image_watermark_bytes,
    embed_image_watermark_bytes_allow_rewrite, extract_image_watermark,
    extract_image_watermark_bytes,
};
pub use payload::{
    decode_payload, encode_payload, AIContentFlags, AuthenticityClaim, GenerationMethod,
    ModificationLevel, TrainingPermission, WatermarkPayload,
};
pub use service::{EmbedOptions, ImageOutputFormat, MediaInput, MediaOutput, WatermarkService};

use crate::pipeline::error::PipelineError;

pub use watermark_core::{
    encode_payload, AIContentFlags, AuthenticityClaim, GenerationMethod, ModificationLevel,
    TrainingPermission, WatermarkPayload,
};

#[allow(dead_code)]
pub fn embed_watermark(
    samples: &mut [f32],
    payload: &WatermarkPayload,
) -> Result<(), PipelineError> {
    watermark_core::embed_watermark(samples, payload)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(e.to_string()))
}

#[allow(dead_code)]
pub fn extract_watermark(samples: &[f32]) -> Result<WatermarkPayload, PipelineError> {
    watermark_core::extract_watermark(samples)
        .map_err(|e| PipelineError::WatermarkExtractFailed(e.to_string()))
}

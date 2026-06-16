use std::path::Path;

use crate::pipeline::error::PipelineError;
use crate::pipeline::watermark::WatermarkPayload;

#[allow(dead_code)]
pub fn embed_image_watermark(
    image_path: &Path,
    payload: &WatermarkPayload,
    output_path: &Path,
) -> Result<(), PipelineError> {
    watermark_core::embed_image_watermark(image_path, payload, output_path)
        .map_err(|e| PipelineError::WatermarkEmbedFailed(e.to_string()))
}

#[allow(dead_code)]
pub fn extract_image_watermark(image_path: &Path) -> Result<WatermarkPayload, PipelineError> {
    watermark_core::extract_image_watermark(image_path)
        .map_err(|e| PipelineError::WatermarkExtractFailed(e.to_string()))
}

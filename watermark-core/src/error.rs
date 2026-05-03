#[derive(Debug, thiserror::Error)]
pub enum WatermarkError {
    #[error("watermark embedding failed: {0}")]
    EmbedFailed(String),

    #[error("watermark extraction failed: {0}")]
    ExtractFailed(String),
}


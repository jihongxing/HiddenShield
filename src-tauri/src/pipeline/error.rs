#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum PipelineError {
    #[error("FFmpeg not found in AppData or PATH")]
    FfmpegNotFound,

    #[error("FFmpeg download failed: {0}")]
    FfmpegDownloadFailed(String),

    #[error("ffprobe failed: {0}")]
    ProbeFailed(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("Insufficient disk space: need {needed_mb}MB, available {available_mb}MB")]
    InsufficientDiskSpace { needed_mb: u64, available_mb: u64 },

    #[error("FFmpeg process failed: {0}")]
    FfmpegFailed(String),

    #[error("Watermark embedding failed: {0}")]
    WatermarkEmbedFailed(String),

    #[error("Watermark extraction failed: {0}")]
    WatermarkExtractFailed(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Sleep inhibition failed: {0}")]
    SleepInhibitFailed(String),

    #[error("Pipeline cancelled")]
    Cancelled,
}

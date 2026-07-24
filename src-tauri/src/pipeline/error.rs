#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub enum PipelineError {
    #[error("FFmpeg not found in system PATH")]
    FfmpegNotFound,

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

    #[error("Watermark failed [{code}]: {message}")]
    WatermarkFailed {
        code: String,
        message: String,
        existing_uid: Option<String>,
    },

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Sleep inhibition failed: {0}")]
    SleepInhibitFailed(String),

    #[error("Pipeline cancelled")]
    Cancelled,
}

impl PipelineError {
    pub fn watermark_failure(
        code: impl Into<String>,
        message: impl Into<String>,
        existing_uid: Option<String>,
    ) -> Self {
        Self::WatermarkFailed {
            code: code.into(),
            message: message.into(),
            existing_uid,
        }
    }

    pub fn watermark_code(&self) -> Option<&str> {
        match self {
            Self::WatermarkFailed { code, .. } => Some(code.as_str()),
            Self::WatermarkEmbedFailed(message)
                if message.contains("audio_protection_duration_unknown") =>
            {
                Some("audio_duration_unknown")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("audio_protection_min_duration") =>
            {
                Some("audio_too_short")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("audio_protection_max_duration") =>
            {
                Some("audio_too_long")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("audio_protection_file_size_limit_exceeded") =>
            {
                Some("audio_file_size_limit_exceeded")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("audio_protection_sample_rate_too_low") =>
            {
                Some("audio_sample_rate_too_low")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("audio_protection_sample_rate_too_high") =>
            {
                Some("audio_sample_rate_too_high")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("audio_protection_channels_unsupported") =>
            {
                Some("audio_channels_unsupported")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("audio_protection_spec_unknown") =>
            {
                Some("audio_spec_unknown")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("image pixel limit exceeded") =>
            {
                Some("image_pixel_limit_exceeded")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("image file size limit exceeded") =>
            {
                Some("image_file_size_limit_exceeded")
            }
            Self::WatermarkEmbedFailed(message) if message.contains("image format unsupported") => {
                Some("image_format_unsupported")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("image too small for watermark") =>
            {
                Some("image_capacity_insufficient")
            }
            Self::WatermarkEmbedFailed(message)
                if message.contains("missing_creator_identity")
                    || message.contains("创作者身份") =>
            {
                Some("missing_creator_identity")
            }
            Self::WatermarkEmbedFailed(_) => Some("embed_failed"),
            Self::WatermarkExtractFailed(_) => Some("extract_failed"),
            _ => None,
        }
    }

    pub fn existing_watermark_uid(&self) -> Option<&str> {
        match self {
            Self::WatermarkFailed { existing_uid, .. } => existing_uid.as_deref(),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PipelineError;

    #[test]
    fn maps_missing_creator_identity_before_generic_embed_failure() {
        let error = PipelineError::WatermarkEmbedFailed(
            "[missing_creator_identity] 请先完成创作者身份设置，再生成保护副本。".to_string(),
        );

        assert_eq!(error.watermark_code(), Some("missing_creator_identity"));
    }

    #[test]
    fn maps_legacy_missing_identity_message_before_generic_embed_failure() {
        let error = PipelineError::WatermarkEmbedFailed(
            "请先完成创作者身份设置，再生成保护副本。".to_string(),
        );

        assert_eq!(error.watermark_code(), Some("missing_creator_identity"));
    }

    #[test]
    fn maps_image_boundary_errors_to_stable_codes() {
        assert_eq!(
            PipelineError::WatermarkEmbedFailed(
                "image pixel limit exceeded: maximum 100 MP".to_string()
            )
            .watermark_code(),
            Some("image_pixel_limit_exceeded")
        );
        assert_eq!(
            PipelineError::WatermarkEmbedFailed(
                "image file size limit exceeded: maximum 512 MiB".to_string()
            )
            .watermark_code(),
            Some("image_file_size_limit_exceeded")
        );
    }

    #[test]
    fn maps_audio_resource_boundary_errors_to_stable_codes() {
        assert_eq!(
            PipelineError::WatermarkEmbedFailed(
                "audio_protection_max_duration: 1201 seconds exceeds maximum 1200 seconds"
                    .to_string()
            )
            .watermark_code(),
            Some("audio_too_long")
        );
        assert_eq!(
            PipelineError::WatermarkEmbedFailed(
                "audio_protection_file_size_limit_exceeded: 536870913 bytes exceeds maximum 536870912 bytes"
                    .to_string()
            )
            .watermark_code(),
            Some("audio_file_size_limit_exceeded")
        );
    }
}

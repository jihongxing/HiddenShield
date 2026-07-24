#[derive(Debug, thiserror::Error)]
pub enum WatermarkError {
    #[error("invalid watermark payload [{code}]: {message}")]
    InvalidPayload {
        code: WatermarkErrorCode,
        message: String,
    },

    #[error("watermark embedding failed: {0}")]
    EmbedFailed(String),

    #[error("watermark extraction failed: {0}")]
    ExtractFailed(String),

    #[error("watermark already exists in source media: {existing_uid}")]
    AlreadyWatermarked { existing_uid: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatermarkErrorCode {
    InvalidPayload,
    MissingCreatorIdentity,
    MissingDeviceIdentity,
    MissingMediaBytes,
    EmbedFailed,
    ExtractFailed,
    AlreadyWatermarked,
    StrategyInvalid,
    FeatureBundleInvalid,
    SelfCheckFailed,
    VisualExtractFailed,
    UnsupportedVideoProfile,
}

impl WatermarkErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidPayload => "invalid_payload",
            Self::MissingCreatorIdentity => "missing_creator_identity",
            Self::MissingDeviceIdentity => "missing_device_identity",
            Self::MissingMediaBytes => "missing_media_bytes",
            Self::EmbedFailed => "embed_failed",
            Self::ExtractFailed => "extract_failed",
            Self::AlreadyWatermarked => "already_watermarked",
            Self::StrategyInvalid => "strategy_invalid",
            Self::FeatureBundleInvalid => "feature_bundle_invalid",
            Self::SelfCheckFailed => "self_check_failed",
            Self::VisualExtractFailed => "visual_extract_failed",
            Self::UnsupportedVideoProfile => "unsupported_video_profile",
        }
    }
}

impl std::fmt::Display for WatermarkErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl WatermarkError {
    pub fn invalid_payload(code: WatermarkErrorCode, message: impl Into<String>) -> Self {
        Self::InvalidPayload {
            code,
            message: message.into(),
        }
    }

    pub fn code(&self) -> WatermarkErrorCode {
        match self {
            Self::InvalidPayload { code, .. } => *code,
            Self::EmbedFailed(_) => WatermarkErrorCode::EmbedFailed,
            Self::ExtractFailed(_) => WatermarkErrorCode::ExtractFailed,
            Self::AlreadyWatermarked { .. } => WatermarkErrorCode::AlreadyWatermarked,
        }
    }

    pub fn code_str(&self) -> &'static str {
        self.code().as_str()
    }

    pub fn existing_uid(&self) -> Option<&str> {
        match self {
            Self::AlreadyWatermarked { existing_uid } => Some(existing_uid),
            _ => None,
        }
    }
}

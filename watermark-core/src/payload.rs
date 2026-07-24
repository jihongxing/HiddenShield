use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

use crate::error::{WatermarkError, WatermarkErrorCode};

pub(crate) const MAGIC: [u8; 4] = [0x48, 0x53, 0x50, 0x32];
pub(crate) const V3_MAGIC: [u8; 4] = [0x48, 0x53, 0x50, 0x33];
pub(crate) const PAYLOAD_PROTOCOL_VERSION: u8 = 2;
pub(crate) const PAYLOAD_V3_PROTOCOL_VERSION: u8 = 3;
pub(crate) const WATERMARK_ID_BYTES: usize = 16;
pub(crate) const CREATOR_BINDING_HASH_BYTES: usize = 16;
pub(crate) const PAYLOAD_AUTH_TAG_BYTES: usize = 16;
pub const PAYLOAD_BYTES: usize = 119;
pub const PAYLOAD_V3_MINIMAL_ANCHOR_BYTES: usize =
    4 + 1 + 2 + WATERMARK_ID_BYTES + PAYLOAD_AUTH_TAG_BYTES;

const PAYLOAD_LENGTH_OFFSET: usize = 5;
const WATERMARK_ID_OFFSET: usize = 7;
const PARENT_WATERMARK_ID_OFFSET: usize = 23;
const REVISION_OFFSET: usize = 39;
const ISSUED_AT_OFFSET: usize = 43;
const ORIGINAL_HASH_PREFIX_OFFSET: usize = 51;
const AI_FLAGS_OFFSET: usize = 67;
const ISSUE_MODE_OFFSET: usize = 69;
const MEDIA_TYPE_OFFSET: usize = 70;
const REGISTRY_PROOF_HASH_OFFSET: usize = 71;
const CREATOR_BINDING_HASH_OFFSET: usize = 87;
const AUTH_TAG_OFFSET: usize = 103;
const V3_PAYLOAD_LENGTH_OFFSET: usize = 5;
const V3_WATERMARK_ID_OFFSET: usize = 7;
const V3_AUTH_TAG_OFFSET: usize = V3_WATERMARK_ID_OFFSET + WATERMARK_ID_BYTES;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AIContentFlags {
    pub is_ai_generated: bool,
    pub training_permission: TrainingPermission,
    pub generation_method: GenerationMethod,
    pub human_modification_level: ModificationLevel,
    pub authenticity_claim: AuthenticityClaim,
    pub reserved: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TrainingPermission {
    Prohibited = 0b00,
    NonCommercial = 0b01,
    Commercial = 0b10,
    PublicDomain = 0b11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GenerationMethod {
    HumanCreated = 0b000,
    TextToImage = 0b001,
    ImageToImage = 0b010,
    TextToVideo = 0b011,
    VideoToVideo = 0b100,
    AudioGeneration = 0b101,
    Multimodal = 0b110,
    OtherAI = 0b111,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ModificationLevel {
    PureAI = 0b00,
    LightEdit = 0b01,
    ModerateEdit = 0b10,
    HeavyEdit = 0b11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuthenticityClaim {
    Unspecified = 0b00,
    Synthetic = 0b01,
    BasedOnReality = 0b10,
    AuthenticRecord = 0b11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WatermarkIssueMode {
    ServerReserved = 1,
    OfflineGenerated = 2,
    ServerConfirmed = 3,
    ServerReissued = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WatermarkMediaType {
    Unknown = 0,
    Image = 1,
    Audio = 2,
    VideoAudioTrack = 3,
    VideoVisual = 4,
}

impl AIContentFlags {
    pub fn pack(&self) -> u16 {
        let mut bits: u16 = 0;
        if self.is_ai_generated {
            bits |= 1 << 15;
        }
        bits |= ((self.training_permission as u16) & 0b11) << 13;
        bits |= ((self.generation_method as u16) & 0b111) << 10;
        bits |= ((self.human_modification_level as u16) & 0b11) << 8;
        bits |= ((self.authenticity_claim as u16) & 0b11) << 6;
        bits |= (self.reserved as u16) & 0b111111;
        bits
    }

    pub fn unpack(bits: u16) -> Self {
        Self {
            is_ai_generated: (bits & (1 << 15)) != 0,
            training_permission: match (bits >> 13) & 0b11 {
                0b00 => TrainingPermission::Prohibited,
                0b01 => TrainingPermission::NonCommercial,
                0b10 => TrainingPermission::Commercial,
                0b11 => TrainingPermission::PublicDomain,
                _ => unreachable!(),
            },
            generation_method: match (bits >> 10) & 0b111 {
                0b000 => GenerationMethod::HumanCreated,
                0b001 => GenerationMethod::TextToImage,
                0b010 => GenerationMethod::ImageToImage,
                0b011 => GenerationMethod::TextToVideo,
                0b100 => GenerationMethod::VideoToVideo,
                0b101 => GenerationMethod::AudioGeneration,
                0b110 => GenerationMethod::Multimodal,
                0b111 => GenerationMethod::OtherAI,
                _ => unreachable!(),
            },
            human_modification_level: match (bits >> 8) & 0b11 {
                0b00 => ModificationLevel::PureAI,
                0b01 => ModificationLevel::LightEdit,
                0b10 => ModificationLevel::ModerateEdit,
                0b11 => ModificationLevel::HeavyEdit,
                _ => unreachable!(),
            },
            authenticity_claim: match (bits >> 6) & 0b11 {
                0b00 => AuthenticityClaim::Unspecified,
                0b01 => AuthenticityClaim::Synthetic,
                0b10 => AuthenticityClaim::BasedOnReality,
                0b11 => AuthenticityClaim::AuthenticRecord,
                _ => unreachable!(),
            },
            reserved: (bits & 0b111111) as u8,
        }
    }
}

impl Default for AIContentFlags {
    fn default() -> Self {
        Self {
            is_ai_generated: false,
            training_permission: TrainingPermission::Prohibited,
            generation_method: GenerationMethod::HumanCreated,
            human_modification_level: ModificationLevel::PureAI,
            authenticity_claim: AuthenticityClaim::Unspecified,
            reserved: 0,
        }
    }
}

impl WatermarkIssueMode {
    fn from_byte(byte: u8) -> Result<Self, WatermarkError> {
        match byte {
            1 => Ok(Self::ServerReserved),
            2 => Ok(Self::OfflineGenerated),
            3 => Ok(Self::ServerConfirmed),
            4 => Ok(Self::ServerReissued),
            _ => Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::InvalidPayload,
                format!("unknown watermark issue mode: {byte}"),
            )),
        }
    }
}

impl WatermarkMediaType {
    fn from_byte(byte: u8) -> Result<Self, WatermarkError> {
        match byte {
            0 => Ok(Self::Unknown),
            1 => Ok(Self::Image),
            2 => Ok(Self::Audio),
            3 => Ok(Self::VideoAudioTrack),
            4 => Ok(Self::VideoVisual),
            _ => Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::InvalidPayload,
                format!("unknown watermark media type: {byte}"),
            )),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatermarkPayload {
    pub magic: [u8; 4],
    pub protocol_version: u8,
    pub watermark_id: [u8; WATERMARK_ID_BYTES],
    pub parent_watermark_id: [u8; WATERMARK_ID_BYTES],
    pub revision: u32,
    pub issued_at: u64,
    pub original_hash_prefix: [u8; 16],
    pub ai_flags: AIContentFlags,
    pub issue_mode: WatermarkIssueMode,
    pub media_type: WatermarkMediaType,
    pub registry_proof_hash: [u8; 16],
    pub creator_binding_hash: [u8; CREATOR_BINDING_HASH_BYTES],
    pub auth_tag: [u8; PAYLOAD_AUTH_TAG_BYTES],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PayloadBuildInput<'a> {
    pub creator_identity: &'a str,
    pub device_identity: &'a str,
    pub media_bytes: &'a [u8],
    pub timestamp: u64,
    pub ai_flags: AIContentFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadDigestBuildInput<'a> {
    pub creator_identity: &'a str,
    pub device_identity: &'a str,
    pub media_sha256: [u8; 32],
    pub timestamp: u64,
    pub ai_flags: AIContentFlags,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadV2BuildInput<'a> {
    pub watermark_id: [u8; WATERMARK_ID_BYTES],
    pub parent_watermark_id: Option<[u8; WATERMARK_ID_BYTES]>,
    pub revision: u32,
    pub issued_at: u64,
    pub original_sha256: [u8; 32],
    pub ai_flags: AIContentFlags,
    pub issue_mode: WatermarkIssueMode,
    pub media_type: WatermarkMediaType,
    pub registry_proof_hash: Option<[u8; 16]>,
    pub creator_binding: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadV3MinimalAnchorBuildInput {
    pub watermark_id: [u8; WATERMARK_ID_BYTES],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatermarkPayloadV3MinimalAnchor {
    pub magic: [u8; 4],
    pub protocol_version: u8,
    pub watermark_id: [u8; WATERMARK_ID_BYTES],
    pub auth_tag: [u8; PAYLOAD_AUTH_TAG_BYTES],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WatermarkDecodedPayload {
    V2(WatermarkPayload),
    V3MinimalAnchor(WatermarkPayloadV3MinimalAnchor),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WatermarkIdentity {
    pub watermark_id: [u8; WATERMARK_ID_BYTES],
    pub creator_binding_hash: [u8; CREATOR_BINDING_HASH_BYTES],
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityBuildInput<'a> {
    pub creator_identity: &'a str,
    pub device_identity: &'a str,
}

impl WatermarkIdentity {
    pub fn from_identity(input: IdentityBuildInput<'_>) -> Result<Self, WatermarkError> {
        let creator_identity = input.creator_identity.trim();
        if creator_identity.is_empty() {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::MissingCreatorIdentity,
                "creator identity is required",
            ));
        }
        let device_identity = input.device_identity.trim();
        if device_identity.is_empty() {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::MissingDeviceIdentity,
                "device identity is required",
            ));
        }

        let mut seed = Vec::new();
        seed.extend_from_slice(b"HiddenShield-PayloadV2-identity");
        seed.extend_from_slice(creator_identity.as_bytes());
        seed.push(0);
        seed.extend_from_slice(device_identity.as_bytes());
        Ok(Self {
            watermark_id: sha256_prefix::<WATERMARK_ID_BYTES>(&seed),
            creator_binding_hash: sha256_prefix::<CREATOR_BINDING_HASH_BYTES>(
                creator_identity.as_bytes(),
            ),
        })
    }

    pub fn watermark_uid_preview(&self) -> String {
        watermark_uid_from_id(&self.watermark_id)
    }
}

type HmacSha256 = Hmac<Sha256>;

fn hmac_secret() -> Vec<u8> {
    obfstr::obfbytes!(b"HS_WM_SECRET_v2_2026_payload_protocol").to_vec()
}

fn compute_auth_tag(data: &[u8]) -> [u8; PAYLOAD_AUTH_TAG_BYTES] {
    let secret = hmac_secret();
    let mut mac = HmacSha256::new_from_slice(&secret).expect("HMAC can take key of any size");
    mac.update(data);
    let result = mac.finalize().into_bytes();
    let mut tag = [0u8; PAYLOAD_AUTH_TAG_BYTES];
    tag.copy_from_slice(&result[..PAYLOAD_AUTH_TAG_BYTES]);
    tag
}

impl WatermarkPayload {
    pub fn new(
        watermark_id_seed: [u8; 8],
        timestamp: u64,
        device_id_seed: [u8; 4],
        file_hash: [u8; 2],
        ai_flags: AIContentFlags,
    ) -> Self {
        let mut seed = Vec::new();
        seed.extend_from_slice(b"HiddenShield-PayloadV2-legacy-entry");
        seed.extend_from_slice(&watermark_id_seed);
        seed.extend_from_slice(&device_id_seed);
        seed.extend_from_slice(&file_hash);
        seed.extend_from_slice(&timestamp.to_be_bytes());
        let watermark_id = sha256_prefix::<WATERMARK_ID_BYTES>(&seed);
        let mut original_hash_prefix = [0u8; 16];
        original_hash_prefix[0..2].copy_from_slice(&file_hash);
        Self::from_v2(PayloadV2BuildInput {
            watermark_id,
            parent_watermark_id: None,
            revision: 1,
            issued_at: timestamp,
            original_sha256: expand_hash_prefix(original_hash_prefix),
            ai_flags,
            issue_mode: WatermarkIssueMode::OfflineGenerated,
            media_type: WatermarkMediaType::Unknown,
            registry_proof_hash: None,
            creator_binding: None,
        })
        .expect("legacy entrypoint seeds always build a valid V2 payload")
    }

    pub fn from_v2(input: PayloadV2BuildInput<'_>) -> Result<Self, WatermarkError> {
        if input.watermark_id == [0u8; WATERMARK_ID_BYTES] {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::InvalidPayload,
                "watermark id must not be all zeros",
            ));
        }
        if input.revision == 0 {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::InvalidPayload,
                "revision must be at least 1",
            ));
        }
        if input.parent_watermark_id.is_some() && input.revision == 1 {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::InvalidPayload,
                "parent watermark id requires revision greater than 1",
            ));
        }

        let mut original_hash_prefix = [0u8; 16];
        original_hash_prefix.copy_from_slice(&input.original_sha256[..16]);
        let registry_proof_hash = input.registry_proof_hash.unwrap_or([0u8; 16]);
        let creator_binding_hash = input
            .creator_binding
            .map(|value| sha256_prefix::<CREATOR_BINDING_HASH_BYTES>(value.trim().as_bytes()))
            .unwrap_or([0u8; CREATOR_BINDING_HASH_BYTES]);
        let mut payload = Self {
            magic: MAGIC,
            protocol_version: PAYLOAD_PROTOCOL_VERSION,
            watermark_id: input.watermark_id,
            parent_watermark_id: input
                .parent_watermark_id
                .unwrap_or([0u8; WATERMARK_ID_BYTES]),
            revision: input.revision,
            issued_at: input.issued_at,
            original_hash_prefix,
            ai_flags: input.ai_flags,
            issue_mode: input.issue_mode,
            media_type: input.media_type,
            registry_proof_hash,
            creator_binding_hash,
            auth_tag: [0u8; PAYLOAD_AUTH_TAG_BYTES],
        };
        payload.auth_tag = compute_auth_tag(&payload.authenticated_bytes());
        Ok(payload)
    }

    pub fn watermark_uid(&self) -> String {
        watermark_uid_from_id(&self.watermark_id)
    }

    pub fn parent_watermark_uid(&self) -> Option<String> {
        if self.parent_watermark_id == [0u8; WATERMARK_ID_BYTES] {
            None
        } else {
            Some(watermark_uid_from_id(&self.parent_watermark_id))
        }
    }

    pub fn file_hash_prefix(&self) -> [u8; 16] {
        self.original_hash_prefix
    }

    pub fn legacy_file_hash_2bytes(&self) -> [u8; 2] {
        [self.original_hash_prefix[0], self.original_hash_prefix[1]]
    }

    pub fn from_identity_and_media(input: PayloadBuildInput<'_>) -> Result<Self, WatermarkError> {
        let creator_identity = input.creator_identity.trim();
        if creator_identity.is_empty() {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::MissingCreatorIdentity,
                "creator identity is required",
            ));
        }
        let device_identity = input.device_identity.trim();
        if device_identity.is_empty() {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::MissingDeviceIdentity,
                "device identity is required",
            ));
        }
        if input.media_bytes.is_empty() {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::MissingMediaBytes,
                "media bytes are required",
            ));
        }

        let media_sha256: [u8; 32] = Sha256::digest(input.media_bytes).into();
        Self::from_identity_and_media_sha256(PayloadDigestBuildInput {
            creator_identity,
            device_identity,
            media_sha256,
            timestamp: input.timestamp,
            ai_flags: input.ai_flags,
        })
    }

    pub fn from_identity_and_media_sha256(
        input: PayloadDigestBuildInput<'_>,
    ) -> Result<Self, WatermarkError> {
        let creator_identity = input.creator_identity.trim();
        if creator_identity.is_empty() {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::MissingCreatorIdentity,
                "creator identity is required",
            ));
        }
        let device_identity = input.device_identity.trim();
        if device_identity.is_empty() {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::MissingDeviceIdentity,
                "device identity is required",
            ));
        }

        let watermark_id = generate_offline_watermark_id()?;

        Self::from_v2(PayloadV2BuildInput {
            watermark_id,
            parent_watermark_id: None,
            revision: 1,
            issued_at: input.timestamp,
            original_sha256: input.media_sha256,
            ai_flags: input.ai_flags,
            issue_mode: WatermarkIssueMode::OfflineGenerated,
            media_type: WatermarkMediaType::Unknown,
            registry_proof_hash: None,
            creator_binding: Some(creator_identity),
        })
    }

    fn authenticated_bytes(&self) -> [u8; AUTH_TAG_OFFSET] {
        let mut buf = [0u8; AUTH_TAG_OFFSET];
        write_payload_without_auth_tag(self, &mut buf);
        buf
    }
}

impl WatermarkPayloadV3MinimalAnchor {
    pub fn new(input: PayloadV3MinimalAnchorBuildInput) -> Result<Self, WatermarkError> {
        if input.watermark_id == [0u8; WATERMARK_ID_BYTES] {
            return Err(WatermarkError::invalid_payload(
                WatermarkErrorCode::InvalidPayload,
                "v3 watermark id must not be all zeros",
            ));
        }
        let mut payload = Self {
            magic: V3_MAGIC,
            protocol_version: PAYLOAD_V3_PROTOCOL_VERSION,
            watermark_id: input.watermark_id,
            auth_tag: [0u8; PAYLOAD_AUTH_TAG_BYTES],
        };
        payload.auth_tag = compute_auth_tag(&payload.authenticated_bytes());
        Ok(payload)
    }

    pub fn watermark_uid(&self) -> String {
        watermark_uid_from_id(&self.watermark_id)
    }

    fn authenticated_bytes(&self) -> [u8; V3_AUTH_TAG_OFFSET] {
        let mut buf = [0u8; V3_AUTH_TAG_OFFSET];
        write_payload_v3_without_auth_tag(self, &mut buf);
        buf
    }
}

impl WatermarkDecodedPayload {
    pub fn watermark_uid(&self) -> String {
        match self {
            Self::V2(payload) => payload.watermark_uid(),
            Self::V3MinimalAnchor(payload) => payload.watermark_uid(),
        }
    }

    pub fn protocol_version(&self) -> u8 {
        match self {
            Self::V2(payload) => payload.protocol_version,
            Self::V3MinimalAnchor(payload) => payload.protocol_version,
        }
    }

    pub fn payload_bytes_length(&self) -> usize {
        match self {
            Self::V2(_) => PAYLOAD_BYTES,
            Self::V3MinimalAnchor(_) => PAYLOAD_V3_MINIMAL_ANCHOR_BYTES,
        }
    }

    pub fn payload_auth_status(&self) -> &'static str {
        "verified"
    }

    pub fn is_v3_minimal_anchor(&self) -> bool {
        matches!(self, Self::V3MinimalAnchor(_))
    }
}

pub fn generate_offline_watermark_id() -> Result<[u8; WATERMARK_ID_BYTES], WatermarkError> {
    let mut watermark_id = [0u8; WATERMARK_ID_BYTES];
    getrandom::getrandom(&mut watermark_id).map_err(|error| {
        WatermarkError::invalid_payload(
            WatermarkErrorCode::InvalidPayload,
            format!("failed to generate offline watermark id: {error}"),
        )
    })?;
    if watermark_id == [0u8; WATERMARK_ID_BYTES] {
        watermark_id[WATERMARK_ID_BYTES - 1] = 1;
    }
    Ok(watermark_id)
}

pub fn watermark_id_from_uid(value: &str) -> Result<[u8; WATERMARK_ID_BYTES], WatermarkError> {
    let compact = value
        .trim()
        .strip_prefix("HS-")
        .unwrap_or(value.trim())
        .replace('-', "");
    if compact.len() != WATERMARK_ID_BYTES * 2 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::InvalidPayload,
            "watermark uid must contain 16 bytes",
        ));
    }

    let mut out = [0u8; WATERMARK_ID_BYTES];
    for (index, chunk) in compact.as_bytes().chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk).map_err(|_| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::InvalidPayload,
                "watermark uid contains invalid utf8",
            )
        })?;
        out[index] = u8::from_str_radix(text, 16).map_err(|_| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::InvalidPayload,
                "watermark uid contains invalid hex",
            )
        })?;
    }
    if out == [0u8; WATERMARK_ID_BYTES] {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::InvalidPayload,
            "watermark uid must not be all zeros",
        ));
    }
    Ok(out)
}

pub fn registry_proof_hash_from_hex(value: &str) -> Result<[u8; 16], WatermarkError> {
    let compact = value.trim().strip_prefix("sha256:").unwrap_or(value.trim());
    if compact.len() < 32 {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::InvalidPayload,
            "registry proof hash must contain at least 16 bytes",
        ));
    }
    let mut out = [0u8; 16];
    for (index, chunk) in compact.as_bytes()[..32].chunks_exact(2).enumerate() {
        let text = std::str::from_utf8(chunk).map_err(|_| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::InvalidPayload,
                "registry proof hash contains invalid utf8",
            )
        })?;
        out[index] = u8::from_str_radix(text, 16).map_err(|_| {
            WatermarkError::invalid_payload(
                WatermarkErrorCode::InvalidPayload,
                "registry proof hash contains invalid hex",
            )
        })?;
    }
    Ok(out)
}

fn expand_hash_prefix(prefix: [u8; 16]) -> [u8; 32] {
    let mut hash = [0u8; 32];
    hash[..16].copy_from_slice(&prefix);
    hash[16..].copy_from_slice(&prefix);
    hash
}

fn sha256_prefix<const N: usize>(bytes: &[u8]) -> [u8; N] {
    let digest = Sha256::digest(bytes);
    let mut output = [0u8; N];
    output.copy_from_slice(&digest[..N]);
    output
}

fn watermark_uid_from_id(id: &[u8; WATERMARK_ID_BYTES]) -> String {
    format!(
        "HS-{}-{}-{}-{}",
        hex_upper(&id[0..4]),
        hex_upper(&id[4..8]),
        hex_upper(&id[8..12]),
        hex_upper(&id[12..16])
    )
}

fn hex_upper(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(&mut out, "{byte:02X}").expect("writing to String cannot fail");
    }
    out
}

fn write_payload_without_auth_tag(payload: &WatermarkPayload, buf: &mut [u8; AUTH_TAG_OFFSET]) {
    buf[0..4].copy_from_slice(&payload.magic);
    buf[4] = payload.protocol_version;
    buf[PAYLOAD_LENGTH_OFFSET..PAYLOAD_LENGTH_OFFSET + 2]
        .copy_from_slice(&(PAYLOAD_BYTES as u16).to_be_bytes());
    buf[WATERMARK_ID_OFFSET..WATERMARK_ID_OFFSET + WATERMARK_ID_BYTES]
        .copy_from_slice(&payload.watermark_id);
    buf[PARENT_WATERMARK_ID_OFFSET..PARENT_WATERMARK_ID_OFFSET + WATERMARK_ID_BYTES]
        .copy_from_slice(&payload.parent_watermark_id);
    buf[REVISION_OFFSET..REVISION_OFFSET + 4].copy_from_slice(&payload.revision.to_be_bytes());
    buf[ISSUED_AT_OFFSET..ISSUED_AT_OFFSET + 8].copy_from_slice(&payload.issued_at.to_be_bytes());
    buf[ORIGINAL_HASH_PREFIX_OFFSET..ORIGINAL_HASH_PREFIX_OFFSET + 16]
        .copy_from_slice(&payload.original_hash_prefix);
    buf[AI_FLAGS_OFFSET..AI_FLAGS_OFFSET + 2]
        .copy_from_slice(&payload.ai_flags.pack().to_be_bytes());
    buf[ISSUE_MODE_OFFSET] = payload.issue_mode as u8;
    buf[MEDIA_TYPE_OFFSET] = payload.media_type as u8;
    buf[REGISTRY_PROOF_HASH_OFFSET..REGISTRY_PROOF_HASH_OFFSET + 16]
        .copy_from_slice(&payload.registry_proof_hash);
    buf[CREATOR_BINDING_HASH_OFFSET..CREATOR_BINDING_HASH_OFFSET + CREATOR_BINDING_HASH_BYTES]
        .copy_from_slice(&payload.creator_binding_hash);
}

fn write_payload_v3_without_auth_tag(
    payload: &WatermarkPayloadV3MinimalAnchor,
    buf: &mut [u8; V3_AUTH_TAG_OFFSET],
) {
    buf[0..4].copy_from_slice(&payload.magic);
    buf[4] = payload.protocol_version;
    buf[V3_PAYLOAD_LENGTH_OFFSET..V3_PAYLOAD_LENGTH_OFFSET + 2]
        .copy_from_slice(&(PAYLOAD_V3_MINIMAL_ANCHOR_BYTES as u16).to_be_bytes());
    buf[V3_WATERMARK_ID_OFFSET..V3_WATERMARK_ID_OFFSET + WATERMARK_ID_BYTES]
        .copy_from_slice(&payload.watermark_id);
}

pub fn encode_payload(payload: &WatermarkPayload) -> [u8; PAYLOAD_BYTES] {
    let mut buf = [0u8; PAYLOAD_BYTES];
    let mut authenticated = [0u8; AUTH_TAG_OFFSET];
    write_payload_without_auth_tag(payload, &mut authenticated);
    let tag = compute_auth_tag(&authenticated);
    buf[..AUTH_TAG_OFFSET].copy_from_slice(&authenticated);
    buf[AUTH_TAG_OFFSET..AUTH_TAG_OFFSET + PAYLOAD_AUTH_TAG_BYTES].copy_from_slice(&tag);
    buf
}

pub fn decode_payload(bytes: &[u8; PAYLOAD_BYTES]) -> Result<WatermarkPayload, WatermarkError> {
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    if magic != MAGIC {
        return Err(WatermarkError::ExtractFailed(format!(
            "payload v2 magic mismatch: expected {:02X?}, got {:02X?}",
            MAGIC, magic
        )));
    }

    let protocol_version = bytes[4];
    if protocol_version != PAYLOAD_PROTOCOL_VERSION {
        return Err(WatermarkError::ExtractFailed(format!(
            "unsupported payload protocol version: {protocol_version}"
        )));
    }

    let payload_length = u16::from_be_bytes(
        bytes[PAYLOAD_LENGTH_OFFSET..PAYLOAD_LENGTH_OFFSET + 2]
            .try_into()
            .unwrap(),
    ) as usize;
    if payload_length != PAYLOAD_BYTES {
        return Err(WatermarkError::ExtractFailed(format!(
            "payload length mismatch: expected {PAYLOAD_BYTES}, got {payload_length}"
        )));
    }

    let mut stored_tag = [0u8; PAYLOAD_AUTH_TAG_BYTES];
    stored_tag.copy_from_slice(&bytes[AUTH_TAG_OFFSET..AUTH_TAG_OFFSET + PAYLOAD_AUTH_TAG_BYTES]);
    let computed_tag = compute_auth_tag((&bytes[..AUTH_TAG_OFFSET]).try_into().unwrap());
    if stored_tag != computed_tag {
        return Err(WatermarkError::ExtractFailed(format!(
            "payload v2 HMAC auth tag mismatch: stored {:02X?}, computed {:02X?}",
            stored_tag, computed_tag
        )));
    }

    let mut watermark_id = [0u8; WATERMARK_ID_BYTES];
    watermark_id
        .copy_from_slice(&bytes[WATERMARK_ID_OFFSET..WATERMARK_ID_OFFSET + WATERMARK_ID_BYTES]);
    let mut parent_watermark_id = [0u8; WATERMARK_ID_BYTES];
    parent_watermark_id.copy_from_slice(
        &bytes[PARENT_WATERMARK_ID_OFFSET..PARENT_WATERMARK_ID_OFFSET + WATERMARK_ID_BYTES],
    );
    let revision = u32::from_be_bytes(
        bytes[REVISION_OFFSET..REVISION_OFFSET + 4]
            .try_into()
            .unwrap(),
    );
    let issued_at = u64::from_be_bytes(
        bytes[ISSUED_AT_OFFSET..ISSUED_AT_OFFSET + 8]
            .try_into()
            .unwrap(),
    );
    let mut original_hash_prefix = [0u8; 16];
    original_hash_prefix
        .copy_from_slice(&bytes[ORIGINAL_HASH_PREFIX_OFFSET..ORIGINAL_HASH_PREFIX_OFFSET + 16]);
    let ai_flags_bits = u16::from_be_bytes(
        bytes[AI_FLAGS_OFFSET..AI_FLAGS_OFFSET + 2]
            .try_into()
            .unwrap(),
    );
    let ai_flags = AIContentFlags::unpack(ai_flags_bits);
    let issue_mode = WatermarkIssueMode::from_byte(bytes[ISSUE_MODE_OFFSET])?;
    let media_type = WatermarkMediaType::from_byte(bytes[MEDIA_TYPE_OFFSET])?;
    let mut registry_proof_hash = [0u8; 16];
    registry_proof_hash
        .copy_from_slice(&bytes[REGISTRY_PROOF_HASH_OFFSET..REGISTRY_PROOF_HASH_OFFSET + 16]);
    let mut creator_binding_hash = [0u8; CREATOR_BINDING_HASH_BYTES];
    creator_binding_hash.copy_from_slice(
        &bytes
            [CREATOR_BINDING_HASH_OFFSET..CREATOR_BINDING_HASH_OFFSET + CREATOR_BINDING_HASH_BYTES],
    );

    Ok(WatermarkPayload {
        magic,
        protocol_version,
        watermark_id,
        parent_watermark_id,
        revision,
        issued_at,
        original_hash_prefix,
        ai_flags,
        issue_mode,
        media_type,
        registry_proof_hash,
        creator_binding_hash,
        auth_tag: stored_tag,
    })
}

pub fn encode_payload_v3_minimal_anchor(
    payload: &WatermarkPayloadV3MinimalAnchor,
) -> [u8; PAYLOAD_V3_MINIMAL_ANCHOR_BYTES] {
    let mut buf = [0u8; PAYLOAD_V3_MINIMAL_ANCHOR_BYTES];
    let mut authenticated = [0u8; V3_AUTH_TAG_OFFSET];
    write_payload_v3_without_auth_tag(payload, &mut authenticated);
    let tag = compute_auth_tag(&authenticated);
    buf[..V3_AUTH_TAG_OFFSET].copy_from_slice(&authenticated);
    buf[V3_AUTH_TAG_OFFSET..V3_AUTH_TAG_OFFSET + PAYLOAD_AUTH_TAG_BYTES].copy_from_slice(&tag);
    buf
}

pub fn decode_payload_v3_minimal_anchor(
    bytes: &[u8; PAYLOAD_V3_MINIMAL_ANCHOR_BYTES],
) -> Result<WatermarkPayloadV3MinimalAnchor, WatermarkError> {
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    if magic != V3_MAGIC {
        return Err(WatermarkError::ExtractFailed(format!(
            "payload v3 magic mismatch: expected {:02X?}, got {:02X?}",
            V3_MAGIC, magic
        )));
    }

    let protocol_version = bytes[4];
    if protocol_version != PAYLOAD_V3_PROTOCOL_VERSION {
        return Err(WatermarkError::ExtractFailed(format!(
            "unsupported payload v3 protocol version: {protocol_version}"
        )));
    }

    let payload_length = u16::from_be_bytes(
        bytes[V3_PAYLOAD_LENGTH_OFFSET..V3_PAYLOAD_LENGTH_OFFSET + 2]
            .try_into()
            .unwrap(),
    ) as usize;
    if payload_length != PAYLOAD_V3_MINIMAL_ANCHOR_BYTES {
        return Err(WatermarkError::ExtractFailed(format!(
            "payload v3 length mismatch: expected {PAYLOAD_V3_MINIMAL_ANCHOR_BYTES}, got {payload_length}"
        )));
    }

    let mut stored_tag = [0u8; PAYLOAD_AUTH_TAG_BYTES];
    stored_tag
        .copy_from_slice(&bytes[V3_AUTH_TAG_OFFSET..V3_AUTH_TAG_OFFSET + PAYLOAD_AUTH_TAG_BYTES]);
    let computed_tag = compute_auth_tag((&bytes[..V3_AUTH_TAG_OFFSET]).try_into().unwrap());
    if stored_tag != computed_tag {
        return Err(WatermarkError::ExtractFailed(format!(
            "payload v3 HMAC auth tag mismatch: stored {:02X?}, computed {:02X?}",
            stored_tag, computed_tag
        )));
    }

    let mut watermark_id = [0u8; WATERMARK_ID_BYTES];
    watermark_id.copy_from_slice(
        &bytes[V3_WATERMARK_ID_OFFSET..V3_WATERMARK_ID_OFFSET + WATERMARK_ID_BYTES],
    );
    if watermark_id == [0u8; WATERMARK_ID_BYTES] {
        return Err(WatermarkError::invalid_payload(
            WatermarkErrorCode::InvalidPayload,
            "v3 watermark id must not be all zeros",
        ));
    }

    Ok(WatermarkPayloadV3MinimalAnchor {
        magic,
        protocol_version,
        watermark_id,
        auth_tag: stored_tag,
    })
}

pub fn decode_watermark_payload_readonly(
    bytes: &[u8],
) -> Result<WatermarkDecodedPayload, WatermarkError> {
    match bytes.len() {
        PAYLOAD_BYTES => {
            let payload_bytes: &[u8; PAYLOAD_BYTES] = bytes.try_into().map_err(|_| {
                WatermarkError::ExtractFailed("payload v2 readonly length conversion failed".into())
            })?;
            decode_payload(payload_bytes).map(WatermarkDecodedPayload::V2)
        }
        PAYLOAD_V3_MINIMAL_ANCHOR_BYTES => {
            let payload_bytes: &[u8; PAYLOAD_V3_MINIMAL_ANCHOR_BYTES] =
                bytes.try_into().map_err(|_| {
                    WatermarkError::ExtractFailed(
                        "payload v3 readonly length conversion failed".into(),
                    )
                })?;
            decode_payload_v3_minimal_anchor(payload_bytes)
                .map(WatermarkDecodedPayload::V3MinimalAnchor)
        }
        length => Err(WatermarkError::ExtractFailed(format!(
            "unsupported readonly payload length: expected {PAYLOAD_BYTES} or {PAYLOAD_V3_MINIMAL_ANCHOR_BYTES}, got {length}"
        ))),
    }
}

pub(crate) fn bytes_to_bits(bytes: &[u8]) -> Vec<bool> {
    let mut bits = Vec::with_capacity(bytes.len() * 8);
    for &byte in bytes {
        for i in (0..8).rev() {
            bits.push((byte >> i) & 1 == 1);
        }
    }
    bits
}

pub(crate) fn bits_to_bytes(bits: &[bool]) -> Vec<u8> {
    bits.chunks(8)
        .map(|chunk| {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                if bit {
                    byte |= 1 << (7 - i);
                }
            }
            byte
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn media_sha() -> [u8; 32] {
        Sha256::digest(b"sample media").into()
    }

    #[test]
    fn v2_payload_roundtrips_full_record_identity() {
        let payload = WatermarkPayload::from_v2(PayloadV2BuildInput {
            watermark_id: [
                0x10, 0x11, 0x12, 0x13, 0x20, 0x21, 0x22, 0x23, 0x30, 0x31, 0x32, 0x33, 0x40, 0x41,
                0x42, 0x43,
            ],
            parent_watermark_id: Some([
                0xA0, 0xA1, 0xA2, 0xA3, 0xB0, 0xB1, 0xB2, 0xB3, 0xC0, 0xC1, 0xC2, 0xC3, 0xD0, 0xD1,
                0xD2, 0xD3,
            ]),
            revision: 2,
            issued_at: 1_786_147_200,
            original_sha256: media_sha(),
            ai_flags: AIContentFlags {
                is_ai_generated: true,
                training_permission: TrainingPermission::Commercial,
                generation_method: GenerationMethod::TextToImage,
                human_modification_level: ModificationLevel::LightEdit,
                authenticity_claim: AuthenticityClaim::Synthetic,
                reserved: 3,
            },
            issue_mode: WatermarkIssueMode::ServerReserved,
            media_type: WatermarkMediaType::Image,
            registry_proof_hash: Some([0x5A; 16]),
            creator_binding: Some("creator"),
        })
        .unwrap();

        let encoded = encode_payload(&payload);
        assert_eq!(encoded.len(), PAYLOAD_BYTES);
        let decoded = decode_payload(&encoded).unwrap();
        assert_eq!(decoded, payload);
        assert_eq!(
            decoded.watermark_uid(),
            "HS-10111213-20212223-30313233-40414243"
        );
        assert_eq!(
            decoded.parent_watermark_uid().as_deref(),
            Some("HS-A0A1A2A3-B0B1B2B3-C0C1C2C3-D0D1D2D3")
        );
    }

    #[test]
    fn v2_payload_rejects_tampered_fields() {
        let payload = WatermarkPayload::from_v2(PayloadV2BuildInput {
            watermark_id: [0x11; WATERMARK_ID_BYTES],
            parent_watermark_id: None,
            revision: 1,
            issued_at: 1_786_147_200,
            original_sha256: media_sha(),
            ai_flags: AIContentFlags::default(),
            issue_mode: WatermarkIssueMode::OfflineGenerated,
            media_type: WatermarkMediaType::Audio,
            registry_proof_hash: None,
            creator_binding: None,
        })
        .unwrap();
        let mut encoded = encode_payload(&payload);
        encoded[WATERMARK_ID_OFFSET] ^= 0x01;
        assert!(decode_payload(&encoded).is_err());
    }

    #[test]
    fn digest_builder_generates_record_level_v2_payload() {
        let media_digest: [u8; 32] = Sha256::digest(b"large-media").into();

        let first = WatermarkPayload::from_identity_and_media_sha256(PayloadDigestBuildInput {
            creator_identity: "creator",
            device_identity: "device",
            media_sha256: media_digest,
            timestamp: 1_700_000_000,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();
        let second = WatermarkPayload::from_identity_and_media_sha256(PayloadDigestBuildInput {
            creator_identity: "creator",
            device_identity: "device",
            media_sha256: Sha256::digest(b"other-media").into(),
            timestamp: 1_700_000_000,
            ai_flags: AIContentFlags::default(),
        })
        .unwrap();

        assert_ne!(first.watermark_uid(), second.watermark_uid());
        assert_eq!(&first.original_hash_prefix, &media_digest[..16]);
        assert_eq!(first.issue_mode, WatermarkIssueMode::OfflineGenerated);
        assert_eq!(first.protocol_version, PAYLOAD_PROTOCOL_VERSION);
    }

    #[test]
    fn v2_payload_rejects_invalid_version_chain() {
        assert!(WatermarkPayload::from_v2(PayloadV2BuildInput {
            watermark_id: [0x11; WATERMARK_ID_BYTES],
            parent_watermark_id: Some([0x22; WATERMARK_ID_BYTES]),
            revision: 1,
            issued_at: 1,
            original_sha256: media_sha(),
            ai_flags: AIContentFlags::default(),
            issue_mode: WatermarkIssueMode::OfflineGenerated,
            media_type: WatermarkMediaType::Image,
            registry_proof_hash: None,
            creator_binding: None,
        })
        .is_err());
    }

    #[test]
    fn v3_minimal_anchor_roundtrips_without_expanding_v2_payload() {
        let anchor = WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput {
            watermark_id: [
                0xA1, 0xA2, 0xA3, 0xA4, 0xB1, 0xB2, 0xB3, 0xB4, 0xC1, 0xC2, 0xC3, 0xC4, 0xD1, 0xD2,
                0xD3, 0xD4,
            ],
        })
        .unwrap();

        let encoded = encode_payload_v3_minimal_anchor(&anchor);
        assert_eq!(PAYLOAD_BYTES, 119);
        assert_eq!(PAYLOAD_V3_MINIMAL_ANCHOR_BYTES, 39);
        assert_eq!(encoded.len(), PAYLOAD_V3_MINIMAL_ANCHOR_BYTES);
        assert_eq!(&encoded[0..4], &V3_MAGIC);
        assert_eq!(encoded[4], PAYLOAD_V3_PROTOCOL_VERSION);
        assert_eq!(
            u16::from_be_bytes(encoded[5..7].try_into().unwrap()) as usize,
            PAYLOAD_V3_MINIMAL_ANCHOR_BYTES
        );

        let decoded = decode_payload_v3_minimal_anchor(&encoded).unwrap();
        assert_eq!(decoded, anchor);
        assert_eq!(
            decoded.watermark_uid(),
            "HS-A1A2A3A4-B1B2B3B4-C1C2C3C4-D1D2D3D4"
        );
    }

    #[test]
    fn v3_minimal_anchor_rejects_tampered_uid() {
        let anchor = WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput {
            watermark_id: [0x42; WATERMARK_ID_BYTES],
        })
        .unwrap();
        let mut encoded = encode_payload_v3_minimal_anchor(&anchor);
        encoded[V3_WATERMARK_ID_OFFSET] ^= 0x01;
        assert!(decode_payload_v3_minimal_anchor(&encoded).is_err());
    }

    #[test]
    fn v3_minimal_anchor_rejects_zero_uid() {
        assert!(
            WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput {
                watermark_id: [0u8; WATERMARK_ID_BYTES],
            })
            .is_err()
        );
    }

    #[test]
    fn readonly_decoder_accepts_v2_payload() {
        let payload = WatermarkPayload::from_v2(PayloadV2BuildInput {
            watermark_id: [0x21; WATERMARK_ID_BYTES],
            parent_watermark_id: None,
            revision: 1,
            issued_at: 1_786_147_200,
            original_sha256: media_sha(),
            ai_flags: AIContentFlags::default(),
            issue_mode: WatermarkIssueMode::OfflineGenerated,
            media_type: WatermarkMediaType::Image,
            registry_proof_hash: None,
            creator_binding: None,
        })
        .unwrap();
        let encoded = encode_payload(&payload);
        let decoded = decode_watermark_payload_readonly(&encoded).unwrap();
        assert_eq!(decoded.protocol_version(), PAYLOAD_PROTOCOL_VERSION);
        assert_eq!(decoded.payload_bytes_length(), PAYLOAD_BYTES);
        assert_eq!(decoded.payload_auth_status(), "verified");
        assert!(!decoded.is_v3_minimal_anchor());
        assert_eq!(decoded.watermark_uid(), payload.watermark_uid());
    }

    #[test]
    fn readonly_decoder_accepts_v3_minimal_anchor() {
        let anchor = WatermarkPayloadV3MinimalAnchor::new(PayloadV3MinimalAnchorBuildInput {
            watermark_id: [0x31; WATERMARK_ID_BYTES],
        })
        .unwrap();
        let encoded = encode_payload_v3_minimal_anchor(&anchor);
        let decoded = decode_watermark_payload_readonly(&encoded).unwrap();
        assert_eq!(decoded.protocol_version(), PAYLOAD_V3_PROTOCOL_VERSION);
        assert_eq!(
            decoded.payload_bytes_length(),
            PAYLOAD_V3_MINIMAL_ANCHOR_BYTES
        );
        assert_eq!(decoded.payload_auth_status(), "verified");
        assert!(decoded.is_v3_minimal_anchor());
        assert_eq!(decoded.watermark_uid(), anchor.watermark_uid());
    }

    #[test]
    fn readonly_decoder_rejects_unknown_length() {
        let bytes = [0xAA; 17];
        assert!(decode_watermark_payload_readonly(&bytes).is_err());
    }
}

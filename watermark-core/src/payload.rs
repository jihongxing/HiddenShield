use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::error::WatermarkError;

pub(crate) const MAGIC: [u8; 4] = [0x48, 0x44, 0x35, 0x48];
pub(crate) const PAYLOAD_BYTES: usize = 32;

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatermarkPayload {
    pub magic: [u8; 4],
    pub user_seed: [u8; 8],
    pub timestamp: u64,
    pub device_id: [u8; 4],
    pub ai_flags: AIContentFlags,
    pub file_hash: [u8; 2],
    pub auth_tag: [u8; 4],
}

type HmacSha256 = Hmac<Sha256>;

fn hmac_secret() -> Vec<u8> {
    obfstr::obfbytes!(b"HS_WM_SECRET_v1_2026_do_not_share").to_vec()
}

fn compute_auth_tag(data: &[u8; 28]) -> [u8; 4] {
    let secret = hmac_secret();
    let mut mac = HmacSha256::new_from_slice(&secret).expect("HMAC can take key of any size");
    mac.update(data);
    let result = mac.finalize().into_bytes();
    let mut tag = [0u8; 4];
    tag.copy_from_slice(&result[..4]);
    tag
}

impl WatermarkPayload {
    pub fn new(
        user_seed: [u8; 8],
        timestamp: u64,
        device_id: [u8; 4],
        file_hash: [u8; 2],
        ai_flags: AIContentFlags,
    ) -> Self {
        let mut buf = [0u8; 28];
        buf[0..4].copy_from_slice(&MAGIC);
        buf[4..12].copy_from_slice(&user_seed);
        buf[12..20].copy_from_slice(&timestamp.to_be_bytes());
        buf[20..24].copy_from_slice(&device_id);
        let ai_flags_packed = ai_flags.pack().to_be_bytes();
        buf[24..26].copy_from_slice(&ai_flags_packed);
        buf[26..28].copy_from_slice(&file_hash);
        let auth_tag = compute_auth_tag(&buf);
        Self {
            magic: MAGIC,
            user_seed,
            timestamp,
            device_id,
            ai_flags,
            file_hash,
            auth_tag,
        }
    }

    pub fn watermark_uid(&self) -> String {
        format!(
            "HS-{:02X}{:02X}-{:02X}{:02X}-{:02X}{:02X}",
            self.user_seed[0],
            self.user_seed[1],
            self.user_seed[2],
            self.user_seed[3],
            self.device_id[0],
            self.device_id[1],
        )
    }
}

pub fn encode_payload(payload: &WatermarkPayload) -> [u8; PAYLOAD_BYTES] {
    let mut buf = [0u8; PAYLOAD_BYTES];
    buf[0..4].copy_from_slice(&payload.magic);
    buf[4..12].copy_from_slice(&payload.user_seed);
    buf[12..20].copy_from_slice(&payload.timestamp.to_be_bytes());
    buf[20..24].copy_from_slice(&payload.device_id);
    let ai_flags_packed = payload.ai_flags.pack().to_be_bytes();
    buf[24..26].copy_from_slice(&ai_flags_packed);
    buf[26..28].copy_from_slice(&payload.file_hash);
    let mut data = [0u8; 28];
    data.copy_from_slice(&buf[..28]);
    let tag = compute_auth_tag(&data);
    buf[28..32].copy_from_slice(&tag);
    buf
}

pub fn decode_payload(bytes: &[u8; PAYLOAD_BYTES]) -> Result<WatermarkPayload, WatermarkError> {
    let mut magic = [0u8; 4];
    magic.copy_from_slice(&bytes[0..4]);
    if magic != MAGIC {
        return Err(WatermarkError::ExtractFailed(format!(
            "magic mismatch: expected {:02X?}, got {:02X?}",
            MAGIC, magic
        )));
    }

    let mut user_seed = [0u8; 8];
    user_seed.copy_from_slice(&bytes[4..12]);
    let timestamp = u64::from_be_bytes(bytes[12..20].try_into().unwrap());
    let mut device_id = [0u8; 4];
    device_id.copy_from_slice(&bytes[20..24]);
    let ai_flags_bits = u16::from_be_bytes(bytes[24..26].try_into().unwrap());
    let ai_flags = AIContentFlags::unpack(ai_flags_bits);
    let mut file_hash = [0u8; 2];
    file_hash.copy_from_slice(&bytes[26..28]);
    let mut stored_tag = [0u8; 4];
    stored_tag.copy_from_slice(&bytes[28..32]);

    let mut data = [0u8; 28];
    data.copy_from_slice(&bytes[..28]);
    let computed_tag = compute_auth_tag(&data);
    if stored_tag != computed_tag {
        return Err(WatermarkError::ExtractFailed(format!(
            "HMAC auth tag mismatch: stored {:02X?}, computed {:02X?}",
            stored_tag, computed_tag
        )));
    }

    Ok(WatermarkPayload {
        magic,
        user_seed,
        timestamp,
        device_id,
        ai_flags,
        file_hash,
        auth_tag: stored_tag,
    })
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

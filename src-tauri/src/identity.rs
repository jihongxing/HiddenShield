//! Creator identity source data for watermark payloads.
//!
//! The desktop app stores source identity fields only. Runtime watermark
//! identity bytes are derived by `watermark-core`; this module does not
//! implement seed, device, payload, or copyright ID algorithms.

use serde::{Deserialize, Serialize};
use std::path::Path;
use watermark_core::{IdentityBuildInput, WatermarkIdentity};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    pub creator_display_name: String,
    pub device_identity: String,
}

pub type IdentityBytes = WatermarkIdentity;

pub fn load_identity(app_data_dir: &Path) -> Option<Identity> {
    let path = app_data_dir.join("identity.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let identity: Identity = serde_json::from_str(&content).ok()?;
    normalize_identity(identity)
}

pub fn save_identity(app_data_dir: &Path, identity: &Identity) -> Result<(), String> {
    let identity = normalize_identity(identity.clone())
        .ok_or_else(|| "creator identity and device identity are required".to_string())?;
    std::fs::create_dir_all(app_data_dir).map_err(|e| format!("create app data dir: {e}"))?;
    let path = app_data_dir.join("identity.json");
    let json =
        serde_json::to_string_pretty(&identity).map_err(|e| format!("serialize identity: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write identity: {e}"))?;
    Ok(())
}

pub fn current_device_identity() -> String {
    let hostname = hostname::get()
        .ok()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "unknown-host".to_string());
    let user = std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "unknown-user".to_string());
    let computer_name = std::env::var("COMPUTERNAME").unwrap_or_default();
    format!(
        "desktop|{}|{}|{}|{}|{}",
        std::env::consts::OS,
        std::env::consts::ARCH,
        hostname.trim(),
        user.trim(),
        computer_name.trim()
    )
}

pub fn initialize_identity(
    app_data_dir: &Path,
    creator_input: &str,
) -> Result<IdentityBytes, String> {
    let identity = Identity {
        creator_display_name: creator_input.trim().to_string(),
        device_identity: current_device_identity(),
    };
    save_identity(app_data_dir, &identity)?;
    identity_bytes(&identity)
}

pub fn get_identity_bytes(app_data_dir: &Path) -> Option<IdentityBytes> {
    let identity = load_identity(app_data_dir)?;
    identity_bytes(&identity).ok()
}

pub fn identity_bytes(identity: &Identity) -> Result<IdentityBytes, String> {
    WatermarkIdentity::from_identity(IdentityBuildInput {
        creator_identity: &identity.creator_display_name,
        device_identity: &identity.device_identity,
    })
    .map_err(|error| error.to_string())
}

fn normalize_identity(identity: Identity) -> Option<Identity> {
    let creator_display_name = identity.creator_display_name.trim().to_string();
    let device_identity = identity.device_identity.trim().to_string();
    if creator_display_name.is_empty() || device_identity.is_empty() {
        return None;
    }
    Some(Identity {
        creator_display_name,
        device_identity,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_seed_identity_is_not_loaded() {
        let parsed: Result<Identity, _> = serde_json::from_str(
            r#"{"user_seed_hex":"abcd","device_id_hex":"1234","creator_display_name":"旧身份"}"#,
        );
        assert!(parsed.is_err());
    }

    #[test]
    fn identity_bytes_are_derived_by_watermark_core() {
        let identity = Identity {
            creator_display_name: "creator".to_string(),
            device_identity: "desktop-device".to_string(),
        };
        let bytes = identity_bytes(&identity).unwrap();
        assert_eq!(
            bytes,
            WatermarkIdentity::from_identity(IdentityBuildInput {
                creator_identity: "creator",
                device_identity: "desktop-device",
            })
            .unwrap()
        );
    }
}

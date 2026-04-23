//! User identity management for watermark payload.
//!
//! Manages two persistent identity components:
//! - `user_seed`: 8 bytes derived from user-provided creator identity (name/alias)
//! - `device_id`: 4 bytes derived from hardware fingerprint (hostname + MAC-like info)
//!
//! Stored in `AppData/HiddenShield/identity.json`.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::Path;

/// Persistent identity stored on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// User-provided creator seed (SHA-256 hex of their input).
    pub user_seed_hex: String,
    /// Device fingerprint (SHA-256 hex of hardware info).
    pub device_id_hex: String,
}

/// Runtime identity bytes ready for watermark embedding.
#[derive(Debug, Clone)]
pub struct IdentityBytes {
    /// 8 bytes: SHA-256 prefix of user's creator identity.
    pub user_seed: [u8; 8],
    /// 4 bytes: SHA-256 prefix of device hardware fingerprint.
    pub device_id: [u8; 4],
}

/// Load identity from disk. Returns None if not yet initialized.
pub fn load_identity(app_data_dir: &Path) -> Option<Identity> {
    let path = app_data_dir.join("identity.json");
    let content = std::fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save identity to disk.
pub fn save_identity(app_data_dir: &Path, identity: &Identity) -> Result<(), String> {
    let path = app_data_dir.join("identity.json");
    let json =
        serde_json::to_string_pretty(identity).map_err(|e| format!("serialize identity: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("write identity: {e}"))?;
    Ok(())
}

/// Compute user_seed (8 bytes) from a user-provided string (name, alias, phone, etc).
pub fn compute_user_seed(creator_input: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(b"HS_USER_SEED_V1:");
    hasher.update(creator_input.as_bytes());
    let hash = hasher.finalize();
    let mut seed = [0u8; 8];
    seed.copy_from_slice(&hash[..8]);
    seed
}

/// Compute device_id (4 bytes) from hardware fingerprint.
pub fn compute_device_id() -> [u8; 4] {
    let mut hasher = Sha256::new();
    hasher.update(b"HS_DEVICE_V1:");

    // Hostname
    if let Ok(name) = hostname::get() {
        hasher.update(name.to_string_lossy().as_bytes());
    }

    // Additional machine entropy: username + OS info
    if let Ok(user) = std::env::var("USERNAME").or_else(|_| std::env::var("USER")) {
        hasher.update(user.as_bytes());
    }
    hasher.update(std::env::consts::OS.as_bytes());
    hasher.update(std::env::consts::ARCH.as_bytes());

    // Windows: COMPUTERNAME for extra uniqueness
    if let Ok(cn) = std::env::var("COMPUTERNAME") {
        hasher.update(cn.as_bytes());
    }

    let hash = hasher.finalize();
    let mut id = [0u8; 4];
    id.copy_from_slice(&hash[..4]);
    id
}

/// Initialize identity on first launch with user-provided creator string.
pub fn initialize_identity(
    app_data_dir: &Path,
    creator_input: &str,
) -> Result<IdentityBytes, String> {
    let user_seed = compute_user_seed(creator_input);
    let device_id = compute_device_id();

    let identity = Identity {
        user_seed_hex: hex::encode(user_seed),
        device_id_hex: hex::encode(device_id),
    };
    save_identity(app_data_dir, &identity)?;

    Ok(IdentityBytes {
        user_seed,
        device_id,
    })
}

/// Get identity bytes (load from disk or return None if not initialized).
pub fn get_identity_bytes(app_data_dir: &Path) -> Option<IdentityBytes> {
    let identity = load_identity(app_data_dir)?;
    let user_seed = hex_to_8bytes(&identity.user_seed_hex)?;
    let device_id = hex_to_4bytes(&identity.device_id_hex)?;
    Some(IdentityBytes {
        user_seed,
        device_id,
    })
}

fn hex_to_8bytes(hex_str: &str) -> Option<[u8; 8]> {
    let bytes = hex::decode(hex_str).ok()?;
    if bytes.len() < 8 {
        return None;
    }
    let mut arr = [0u8; 8];
    arr.copy_from_slice(&bytes[..8]);
    Some(arr)
}

fn hex_to_4bytes(hex_str: &str) -> Option<[u8; 4]> {
    let bytes = hex::decode(hex_str).ok()?;
    if bytes.len() < 4 {
        return None;
    }
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&bytes[..4]);
    Some(arr)
}

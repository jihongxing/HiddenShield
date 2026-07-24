use tauri::{AppHandle, Manager};

use crate::identity;

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IdentityStatus {
    pub initialized: bool,
    pub watermark_uid_preview: Option<String>,
    pub device_id_hex: Option<String>,
    pub creator_display_name: Option<String>,
}

/// Check if the user has set up their creator identity.
#[tauri::command]
pub async fn get_identity_status(app_handle: AppHandle) -> Result<IdentityStatus, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;

    match identity::get_identity_bytes(&app_data_dir) {
        Some(id) => {
            let identity = identity::load_identity(&app_data_dir);
            Ok(IdentityStatus {
                initialized: true,
                watermark_uid_preview: Some(id.watermark_uid_preview()),
                device_id_hex: Some(hex::encode(id.watermark_id)),
                creator_display_name: identity.map(|value| value.creator_display_name),
            })
        }
        None => Ok(IdentityStatus {
            initialized: false,
            watermark_uid_preview: None,
            device_id_hex: None,
            creator_display_name: None,
        }),
    }
}

/// Initialize the creator identity with user-provided input.
#[tauri::command]
pub async fn setup_identity(
    creator_input: String,
    app_handle: AppHandle,
) -> Result<IdentityStatus, String> {
    if creator_input.trim().is_empty() {
        return Err("创作者标识不能为空".to_string());
    }

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("failed to resolve app data dir: {e}"))?;

    std::fs::create_dir_all(&app_data_dir).map_err(|e| format!("create app data dir: {e}"))?;

    let id = identity::initialize_identity(&app_data_dir, creator_input.trim())?;

    Ok(IdentityStatus {
        initialized: true,
        watermark_uid_preview: Some(id.watermark_uid_preview()),
        device_id_hex: Some(hex::encode(id.watermark_id)),
        creator_display_name: Some(creator_input.trim().to_string()),
    })
}

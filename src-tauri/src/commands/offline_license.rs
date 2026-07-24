use std::path::{Path, PathBuf};

use tauri::{AppHandle, Manager};

use crate::entitlements::{self, OfflineActivationRequestExport, OfflineLicenseStatus};
use crate::AppState;

#[tauri::command]
pub async fn get_offline_license_status(
    app_handle: AppHandle,
) -> Result<OfflineLicenseStatus, String> {
    let state = app_handle.state::<AppState>();
    let conn = state
        .db
        .lock()
        .map_err(|error| format!("db lock error: {error}"))?;
    entitlements::get_offline_license_status(&conn, state.installation_secret_store.as_ref())
}

#[tauri::command]
pub async fn export_offline_activation_request(
    app_handle: AppHandle,
    output_path: Option<String>,
) -> Result<OfflineActivationRequestExport, String> {
    let state = app_handle.state::<AppState>();
    let mut export = {
        let conn = state
            .db
            .lock()
            .map_err(|error| format!("db lock error: {error}"))?;
        entitlements::export_activation_request(&conn, state.installation_secret_store.as_ref())?
    };
    if let Some(output_path) = output_path {
        let path = activation_request_output_path(&output_path, &export.installation_id);
        std::fs::write(&path, export.token.as_bytes())
            .map_err(|error| format!("offline_license_request_write_failed:{error}"))?;
        export.output_path = Some(path.to_string_lossy().to_string());
    }
    Ok(export)
}

#[tauri::command]
pub async fn import_offline_license(
    app_handle: AppHandle,
    token_or_path: String,
) -> Result<OfflineLicenseStatus, String> {
    let token = read_token_or_path(&token_or_path, "HSLIC1.")?;
    let state = app_handle.state::<AppState>();
    let conn = state
        .db
        .lock()
        .map_err(|error| format!("db lock error: {error}"))?;
    entitlements::import_offline_license(&conn, state.installation_secret_store.as_ref(), &token)
}

#[tauri::command]
pub async fn clear_offline_license(app_handle: AppHandle) -> Result<OfflineLicenseStatus, String> {
    let state = app_handle.state::<AppState>();
    let conn = state
        .db
        .lock()
        .map_err(|error| format!("db lock error: {error}"))?;
    entitlements::clear_offline_license(&conn, state.installation_secret_store.as_ref())
}

#[tauri::command]
pub async fn import_offline_revocation_list(
    app_handle: AppHandle,
    token_or_path: String,
) -> Result<OfflineLicenseStatus, String> {
    let token = read_token_or_path(&token_or_path, "HSRVL1.")?;
    let state = app_handle.state::<AppState>();
    let conn = state
        .db
        .lock()
        .map_err(|error| format!("db lock error: {error}"))?;
    entitlements::import_revocation_list(&conn, state.installation_secret_store.as_ref(), &token)
}

fn read_token_or_path(value: &str, prefix: &str) -> Result<String, String> {
    let trimmed = value.trim();
    if trimmed.starts_with(prefix) {
        return Ok(trimmed.to_string());
    }
    std::fs::read_to_string(Path::new(trimmed))
        .map(|token| token.trim().to_string())
        .map_err(|error| format!("offline_license_artifact_read_failed:{error}"))
}

fn activation_request_output_path(output_path: &str, installation_id: &str) -> PathBuf {
    let path = PathBuf::from(output_path);
    if path.is_dir() {
        return path.join(format!(
            "HiddenShield-Activation-{}.hsreq",
            &installation_id[..12]
        ));
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raw_token_or_path_accepts_raw_tokens() {
        let token = "HSLIC1.payload.signature";
        assert_eq!(read_token_or_path(token, "HSLIC1.").unwrap(), token);
    }
}

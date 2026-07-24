use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::config::{load_preferences, save_preferences as persist_preferences, AppPreferences};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreferencesStatus {
    pub default_output_dir: Option<String>,
    pub default_output_dir_writable: bool,
    pub onboarding_completed: bool,
    pub auto_update_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavePreferencesInput {
    pub default_output_dir: Option<String>,
    pub onboarding_completed: Option<bool>,
    pub auto_update_enabled: Option<bool>,
}

#[tauri::command]
pub async fn get_preferences(app_handle: AppHandle) -> Result<PreferencesStatus, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法获取应用数据目录: {error}"))?;
    let preferences = load_preferences(&app_data_dir);
    Ok(preferences_status(preferences))
}

#[tauri::command]
pub async fn save_preferences(
    app_handle: AppHandle,
    input: SavePreferencesInput,
) -> Result<PreferencesStatus, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|error| format!("无法获取应用数据目录: {error}"))?;
    let mut preferences = load_preferences(&app_data_dir);
    if let Some(default_output_dir) = input.default_output_dir {
        preferences.default_output_dir = normalize_output_dir(default_output_dir);
    }
    if let Some(onboarding_completed) = input.onboarding_completed {
        preferences.onboarding_completed = onboarding_completed;
    }
    if let Some(auto_update_enabled) = input.auto_update_enabled {
        preferences.auto_update_enabled = auto_update_enabled;
    }
    persist_preferences(&app_data_dir, &preferences)?;
    Ok(preferences_status(preferences))
}

fn preferences_status(preferences: AppPreferences) -> PreferencesStatus {
    let writable = preferences
        .default_output_dir
        .as_deref()
        .map(PathBuf::from)
        .map(|path| check_write_permission(&path))
        .unwrap_or(true);
    PreferencesStatus {
        default_output_dir: preferences.default_output_dir,
        default_output_dir_writable: writable,
        onboarding_completed: preferences.onboarding_completed,
        auto_update_enabled: preferences.auto_update_enabled,
    }
}

fn normalize_output_dir(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn check_write_permission(dir: &PathBuf) -> bool {
    if !dir.exists() || !dir.is_dir() {
        return false;
    }
    let file_name = format!(
        ".hs_write_test_{}_{}",
        std::process::id(),
        chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
    );
    let test_file = dir.join(file_name);
    match std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&test_file)
    {
        Ok(_) => {
            let _ = std::fs::remove_file(test_file);
            true
        }
        Err(_) => false,
    }
}

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemConfig {
    pub cloud_base_url: String,
    pub lan_debug_port: u16,
}

pub fn load_system_config() -> SystemConfig {
    let default = SystemConfig {
        cloud_base_url: "http://127.0.0.1:43188".to_string(),
        lan_debug_port: 47219,
    };
    for path in candidate_config_paths() {
        if let Ok(body) = std::fs::read_to_string(path) {
            if let Ok(config) = serde_json::from_str(&body) {
                return config;
            }
        }
    }
    default
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub default_output_dir: Option<String>,
    pub onboarding_completed: bool,
    #[serde(default = "default_auto_update_enabled")]
    pub auto_update_enabled: bool,
}

impl Default for AppPreferences {
    fn default() -> Self {
        Self {
            default_output_dir: None,
            onboarding_completed: false,
            auto_update_enabled: default_auto_update_enabled(),
        }
    }
}

fn default_auto_update_enabled() -> bool {
    true
}

pub fn load_preferences(app_data_dir: &Path) -> AppPreferences {
    let path = preferences_path(app_data_dir);
    let Ok(body) = std::fs::read_to_string(path) else {
        return AppPreferences::default();
    };
    serde_json::from_str(&body).unwrap_or_default()
}

pub fn save_preferences(app_data_dir: &Path, preferences: &AppPreferences) -> Result<(), String> {
    std::fs::create_dir_all(app_data_dir)
        .map_err(|error| format!("创建应用数据目录失败: {error}"))?;
    let body = serde_json::to_string_pretty(preferences)
        .map_err(|error| format!("序列化应用设置失败: {error}"))?;
    std::fs::write(preferences_path(app_data_dir), body)
        .map_err(|error| format!("保存应用设置失败: {error}"))
}

pub fn resolve_output_dir(app_data_dir: &Path, input_path: &Path) -> PathBuf {
    let preferences = load_preferences(app_data_dir);
    if let Some(path) = preferences
        .default_output_dir
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        return PathBuf::from(path);
    }
    input_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf()
}

fn preferences_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("preferences.json")
}

fn candidate_config_paths() -> Vec<PathBuf> {
    let mut paths = vec![PathBuf::from("config/hiddenshield.system.json")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("config/hiddenshield.system.json"));
        }
    }
    paths
}

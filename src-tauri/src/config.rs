use serde::Deserialize;

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

fn candidate_config_paths() -> Vec<std::path::PathBuf> {
    let mut paths = vec![std::path::PathBuf::from("config/hiddenshield.system.json")];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join("config/hiddenshield.system.json"));
        }
    }
    paths
}

use std::path::Path;

use serde::Serialize;
use tauri::State;

use crate::db::queries;
use crate::AppState;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultRecord {
  pub id: u32,
  pub original_hash: String,
  pub file_name: String,
  pub created_at: String,
  pub duration_secs: f64,
  pub resolution: String,
  pub watermark_uid: String,
  pub thumbnail_path: Option<String>,
  pub output_douyin: Option<String>,
  pub output_bilibili: Option<String>,
  pub output_xhs: Option<String>,
  pub is_hdr_source: bool,
  pub hw_encoder_used: Option<String>,
  pub process_time_ms: Option<u64>,
  pub tsa_token_path: Option<String>,
  pub network_time: Option<String>,
  pub tsa_source: Option<String>,
}

#[tauri::command]
pub async fn list_vault_records(state: State<'_, AppState>) -> Result<Vec<VaultRecord>, String> {
  let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
  Ok(queries::list_records(&conn))
}

/// Check which file paths still exist on disk.
/// Returns a list of paths that are missing/offline.
#[tauri::command]
pub async fn check_files_exist(paths: Vec<String>) -> Result<Vec<String>, String> {
  let missing: Vec<String> = paths
    .into_iter()
    .filter(|p| !Path::new(p).exists())
    .collect();
  Ok(missing)
}

use tauri::{AppHandle, Manager};

use crate::db::billing::{self, EntitlementState, UsageLedgerSummary};
use crate::telemetry;
use crate::AppState;

#[tauri::command]
pub async fn get_entitlement_state(app_handle: AppHandle) -> Result<EntitlementState, String> {
    let state = app_handle.state::<AppState>();
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    let entitlement = billing::get_entitlement_state(&conn)
        .map_err(|e| format!("读取权益状态失败: {e}"))?;
    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        telemetry::anonymous::record_entitlement_snapshot(
            &app_data_dir,
            entitlement.status.as_str(),
            "billing_get",
        );
    }
    Ok(entitlement)
}

#[tauri::command]
pub async fn set_entitlement_state(
    app_handle: AppHandle,
    entitlement_state: EntitlementState,
) -> Result<EntitlementState, String> {
    let state = app_handle.state::<AppState>();
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    billing::save_entitlement_state(&conn, &entitlement_state)
        .map_err(|e| format!("保存权益状态失败: {e}"))?;
    if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
        telemetry::anonymous::record_entitlement_snapshot(
            &app_data_dir,
            entitlement_state.status.as_str(),
            "billing_set",
        );
    }
    Ok(entitlement_state)
}

#[tauri::command]
pub async fn get_usage_ledger_summary(app_handle: AppHandle) -> Result<UsageLedgerSummary, String> {
    let state = app_handle.state::<AppState>();
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    billing::get_usage_summary(&conn).map_err(|e| format!("读取用量账本失败: {e}"))
}

/// Convenience command for future testing or admin tooling.
#[tauri::command]
pub async fn record_usage_event(
    app_handle: AppHandle,
    feature_name: String,
    media_type: String,
    file_size_bytes: u64,
    pipeline_id: Option<String>,
) -> Result<UsageLedgerSummary, String> {
    let state = app_handle.state::<AppState>();
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    let entitlement = billing::get_entitlement_state(&conn)
        .map_err(|e| format!("读取权益状态失败: {e}"))?;
    let entry = billing::UsageLedgerEntry::success(
        feature_name,
        media_type,
        file_size_bytes,
        &entitlement,
        pipeline_id,
    );
    billing::append_usage_entry(&conn, &entry)
        .map_err(|e| format!("写入用量账本失败: {e}"))?;
    billing::get_usage_summary(&conn).map_err(|e| format!("读取用量账本失败: {e}"))
}

use tauri::{AppHandle, Manager};

use crate::db::local_batch::{self, LocalBatchJob};
use crate::entitlements;
use crate::AppState;

#[tauri::command]
pub async fn list_local_batch_jobs(app_handle: AppHandle) -> Result<Vec<LocalBatchJob>, String> {
    let state = app_handle.state::<AppState>();
    let conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    local_batch::list_local_batch_jobs(&conn).map_err(|e| format!("读取本地批量队列失败: {e}"))
}

#[tauri::command]
pub async fn save_local_batch_job(
    app_handle: AppHandle,
    mut job: LocalBatchJob,
) -> Result<LocalBatchJob, String> {
    let state = app_handle.state::<AppState>();
    let mut conn = state.db.lock().map_err(|e| format!("db lock error: {e}"))?;
    let entitlement = entitlements::resolve_effective_entitlement(
        &conn,
        state.installation_secret_store.as_ref(),
    )
    .map_err(|e| format!("读取权益状态失败: {e}"))?;
    if entitlement.features.get("batch_processing") != Some(&true) {
        return Err("本地批量处理从 Creator 开放".to_string());
    }
    job.entitlement_plan_code = entitlement.plan_code;
    job.entitlement_status = entitlement.status.as_str().to_string();
    local_batch::save_local_batch_job(&mut conn, &job)
        .map_err(|e| format!("保存本地批量队列失败: {e}"))?;
    Ok(job)
}

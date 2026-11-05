use crate::dto::SyncStatus;
use crate::error::AppError;
use crate::state::AppState;
use std::sync::atomic::Ordering;
use tauri::State;

#[tauri::command]
pub fn sync_status(state: State<'_, AppState>) -> Result<SyncStatus, AppError> {
    Ok(crate::drive::sync::status(&state))
}

#[tauri::command]
pub async fn sync_now(state: State<'_, AppState>) -> Result<SyncStatus, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || crate::drive::sync::run_cycle(&state))
        .await
        .map_err(|error| AppError::io(error.to_string()))?
}

#[tauri::command]
pub fn sync_pause(state: State<'_, AppState>) -> Result<SyncStatus, AppError> {
    state.inner.sync.paused.store(true, Ordering::SeqCst);
    let mut status = crate::drive::sync::status(&state);
    status.state = "paused".to_string();
    *state.inner.sync.status.write() = status.clone();
    state.emit("sync://status", status.clone());
    Ok(status)
}

#[tauri::command]
pub fn sync_resume(state: State<'_, AppState>) -> Result<SyncStatus, AppError> {
    state.inner.sync.paused.store(false, Ordering::SeqCst);
    let status = crate::drive::sync::status(&state);
    *state.inner.sync.status.write() = status.clone();
    state.emit("sync://status", status.clone());
    Ok(status)
}

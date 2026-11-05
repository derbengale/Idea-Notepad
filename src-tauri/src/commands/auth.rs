use crate::dto::AuthStatus;
use crate::error::AppError;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn auth_status(state: State<'_, AppState>) -> Result<AuthStatus, AppError> {
    crate::drive::sync::auth_status(&state)
}

#[tauri::command]
pub async fn auth_sign_in(state: State<'_, AppState>) -> Result<AuthStatus, AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || crate::drive::sync::sign_in(&state))
        .await
        .map_err(|error| AppError::io(error.to_string()))?
}

#[tauri::command]
pub async fn auth_sign_out(
    state: State<'_, AppState>,
    delete_local_data: bool,
) -> Result<(), AppError> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        crate::drive::sync::sign_out(&state, delete_local_data)
    })
    .await
    .map_err(|error| AppError::io(error.to_string()))?
}

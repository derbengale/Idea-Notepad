use crate::db::migration_version;
use crate::dto::{AppInfo, Settings};
use crate::error::AppError;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> Result<AppInfo, AppError> {
    Ok(AppInfo {
        version: env!("CARGO_PKG_VERSION").to_string(),
        platform: std::env::consts::OS.to_string(),
        data_dir: state.inner.paths.data_dir.to_string_lossy().to_string(),
        db_path: state.inner.paths.db_path.to_string_lossy().to_string(),
        log_path: state.inner.paths.log_path.to_string_lossy().to_string(),
        mirror_path: state.inner.paths.mirror_dir.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn app_open_data_dir(state: State<'_, AppState>) -> Result<(), AppError> {
    open::that(state.inner.paths.data_dir.as_path())
        .map_err(|error| AppError::io(format!("Cannot open the data folder: {error}")))
}

#[tauri::command]
pub fn settings_get(state: State<'_, AppState>) -> Result<Settings, AppError> {
    Ok(state.settings())
}

#[tauri::command]
pub fn settings_set(state: State<'_, AppState>, settings: Settings) -> Result<Settings, AppError> {
    state.save_settings(settings)
}

#[tauri::command]
pub fn diagnostics(state: State<'_, AppState>) -> Result<serde_json::Value, AppError> {
    let version = migration_version(&state.inner.db)?;
    Ok(serde_json::json!({
        "appVersion": env!("CARGO_PKG_VERSION"),
        "tauriVersion": tauri::VERSION,
        "migrationVersion": version,
        "schemaFormatVersion": 1,
        "bundleFormatVersion": 1,
        "dataDir": state.inner.paths.data_dir.to_string_lossy(),
        "dbPath": state.inner.paths.db_path.to_string_lossy(),
        "logPath": state.inner.paths.log_path.to_string_lossy(),
        "mirrorPath": state.inner.paths.mirror_dir.to_string_lossy(),
    }))
}

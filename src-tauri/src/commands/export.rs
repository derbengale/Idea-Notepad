use crate::dto::{BundlePreview, ExportResult, ImportReport};
use crate::error::AppError;
use crate::state::AppState;
use tauri::State;

#[tauri::command]
pub fn export_entry_markdown(
    state: State<'_, AppState>,
    id: String,
    target_path: String,
) -> Result<ExportResult, AppError> {
    crate::export::export_entry_markdown(&state, &id, &target_path)
}

#[tauri::command]
pub fn bundle_export(
    state: State<'_, AppState>,
    scope: String,
    project_id: Option<String>,
    target_path: String,
) -> Result<ExportResult, AppError> {
    crate::export::bundle_export(&state, &scope, project_id.as_deref(), &target_path)
}

#[tauri::command]
pub fn bundle_inspect(
    state: State<'_, AppState>,
    source_path: String,
) -> Result<BundlePreview, AppError> {
    crate::export::bundle_inspect(&state, &source_path)
}

#[tauri::command]
pub fn bundle_import(
    state: State<'_, AppState>,
    source_path: String,
    mode: String,
) -> Result<ImportReport, AppError> {
    crate::export::bundle_import(&state, &source_path, &mode)
}

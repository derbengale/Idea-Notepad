use crate::db::{repo_entry, repo_project, repo_sync};
use crate::dto::{DeletedCount, ProjectDto, ProjectInput};
use crate::error::AppError;
use crate::state::AppState;
use serde_json::json;
use tauri::State;

#[tauri::command]
pub fn project_list(state: State<'_, AppState>) -> Result<Vec<ProjectDto>, AppError> {
    state.inner.db.with_read(repo_project::list)
}

#[tauri::command]
pub fn project_create(
    state: State<'_, AppState>,
    input: ProjectInput,
) -> Result<ProjectDto, AppError> {
    let project = state
        .inner
        .db
        .with_write(|conn| repo_project::create(conn, &input))?;
    enqueue_projects(&state)?;
    Ok(project)
}

#[tauri::command]
pub fn project_update(
    state: State<'_, AppState>,
    project: ProjectDto,
) -> Result<ProjectDto, AppError> {
    let updated = state
        .inner
        .db
        .with_write(|conn| repo_project::update(conn, &project))?;
    enqueue_projects(&state)?;
    Ok(updated)
}

#[tauri::command]
pub fn project_delete(state: State<'_, AppState>, id: String) -> Result<DeletedCount, AppError> {
    let affected = state
        .inner
        .db
        .with_read(|conn| repo_entry::ids_for_project(conn, &id))?;
    let moved = state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        let moved = repo_project::delete(&tx, &id)?;
        for entry_id in affected {
            repo_sync::enqueue(&tx, "note.write", &entry_id, json!({ "entryId": entry_id }))?;
        }
        tx.commit()?;
        Ok(moved)
    })?;
    enqueue_projects(&state)?;
    Ok(DeletedCount {
        moved_entries: moved,
    })
}

fn enqueue_projects(state: &State<'_, AppState>) -> Result<(), AppError> {
    state
        .inner
        .db
        .with_write(|conn| repo_sync::enqueue(conn, "projects.write", "projects", json!({})))
}

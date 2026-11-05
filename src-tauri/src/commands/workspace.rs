use crate::db::{repo_entry, repo_meta, repo_sync};
use crate::dto::{RebuildReport, WorkspaceStats};
use crate::error::AppError;
use crate::markdown;
use crate::state::AppState;
use crate::util::normalize_body;
use std::time::Instant;
use tauri::State;

#[tauri::command]
pub fn workspace_stats(state: State<'_, AppState>) -> Result<WorkspaceStats, AppError> {
    state.inner.db.with_read(repo_meta::stats)
}

#[tauri::command]
pub fn workspace_rebuild_index(state: State<'_, AppState>) -> Result<RebuildReport, AppError> {
    let started = Instant::now();
    let notes_dir = state.mirror_dir().join("notes");
    let mut files = Vec::new();
    if notes_dir.is_dir() {
        for entry in std::fs::read_dir(&notes_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                files.push(path);
            }
        }
    }

    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_entry::clear_all(&tx)?;
        repo_sync::clear(&tx)?;
        tx.commit()?;
        Ok(())
    })?;

    let mut warnings = Vec::new();
    let mut imported = 0i64;
    for path in &files {
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                warnings.push(format!("Cannot read {}: {error}", path.display()));
                continue;
            }
        };
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                warnings.push(format!("{} is not valid UTF-8", path.display()));
                continue;
            }
        };
        let mut note = match markdown::parse_note(&text) {
            Ok(note) => note,
            Err(error) => {
                warnings.push(format!("{}: {}", path.display(), error.message));
                continue;
            }
        };
        note.body = normalize_body(&note.body);
        let schema = match crate::drive::sync::ensure_schema(&state, &note) {
            Ok(schema) => schema,
            Err(error) => {
                warnings.push(format!("{}: {}", path.display(), error.message));
                continue;
            }
        };
        if let Err(error) = crate::export::insert_note(&state, &note, &schema) {
            warnings.push(format!("{}: {}", path.display(), error.message));
            continue;
        }
        imported += 1;
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
            .to_string();
        if let Ok(Some(synced)) = state
            .inner
            .db
            .with_read(|conn| repo_sync::get_by_path(conn, &format!("notes/{file_name}")))
        {
            let _ = state.inner.db.with_write(|conn| {
                repo_entry::set_drive_meta(
                    conn,
                    &note.id,
                    &synced.drive_file_id,
                    synced.modified_at.as_deref(),
                )?;
                repo_entry::set_sync_state(conn, &note.id, "clean")
            });
        }
    }

    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_entry::normalize_sync_states(&tx)?;
        repo_sync::clear(&tx)?;
        tx.commit()?;
        Ok(())
    })?;
    state.refresh_schemas()?;

    let stats = state.inner.db.with_read(repo_meta::stats)?;
    state.emit_workspace_changed("rebuild");
    Ok(RebuildReport {
        notes: imported,
        attachments: stats.attachments,
        duration_ms: started.elapsed().as_millis() as u64,
        warnings,
    })
}

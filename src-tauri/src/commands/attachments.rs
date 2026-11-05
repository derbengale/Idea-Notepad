use crate::db::{repo_attachment, repo_entry, repo_sync};
use crate::dto::AttachmentDto;
use crate::error::AppError;
use crate::state::AppState;
use crate::util::{new_id, now_iso, sha256_hex};
use base64::Engine;
use serde_json::json;
use std::path::{Path, PathBuf};
use tauri::State;

const MAX_IMAGE_BYTES: u64 = 25 * 1024 * 1024;
const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;
const MAX_ENTRY_BYTES: i64 = 250 * 1024 * 1024;

#[tauri::command]
pub fn attachment_list(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<Vec<AttachmentDto>, AppError> {
    state
        .inner
        .db
        .with_read(|conn| repo_attachment::list(conn, &entry_id))
}

#[tauri::command]
pub fn attachment_add(
    state: State<'_, AppState>,
    entry_id: String,
    source_path: String,
) -> Result<AttachmentDto, AppError> {
    let source = PathBuf::from(&source_path);
    if !source.is_file() {
        return Err(AppError::io("Attachment source does not exist"));
    }
    let metadata = std::fs::metadata(&source)?;
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| AppError::validation("Invalid file name"))?
        .to_string();
    if file_name.chars().count() > 120 {
        return Err(AppError::validation(
            "File name must not exceed 120 characters",
        ));
    }
    let mime = mime_for(&file_name);
    let is_image = mime.starts_with("image/");
    let limit = if is_image {
        MAX_IMAGE_BYTES
    } else {
        MAX_FILE_BYTES
    };
    if metadata.len() > limit {
        return Err(AppError::validation("Attachment exceeds the size limit"));
    }
    let existing_total = state
        .inner
        .db
        .with_read(|conn| repo_attachment::total_bytes(conn, &entry_id))?;
    if existing_total + metadata.len() as i64 > MAX_ENTRY_BYTES {
        return Err(AppError::validation(
            "This note exceeds the attachment size limit",
        ));
    }
    if state
        .inner
        .db
        .with_read(|conn| repo_entry::find_row(conn, &entry_id))?
        .is_none()
    {
        return Err(AppError::not_found("Note not found"));
    }
    let bytes = std::fs::read(&source)?;
    let sha = sha256_hex(&bytes);
    if state
        .inner
        .db
        .with_read(|conn| repo_attachment::find_by_sha(conn, &entry_id, &sha))?
    {
        return Err(AppError::conflict("This file is already attached"));
    }
    let id = new_id();
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.to_lowercase())
        .unwrap_or_default();
    let stored_name = if extension.is_empty() {
        id.clone()
    } else {
        format!("{id}.{extension}")
    };
    let target_dir = state.mirror_dir().join("attachments").join(&entry_id);
    let target = target_dir.join(&stored_name);
    crate::util::atomic_write(&target, &bytes)?;
    let attachment = AttachmentDto {
        id,
        entry_id: entry_id.clone(),
        kind: if is_image { "image" } else { "file" }.to_string(),
        file_name,
        mime_type: mime,
        byte_size: bytes.len() as i64,
        sha256: sha,
        local_path: target.to_string_lossy().to_string(),
        preview_path: None,
        created_at: now_iso(),
    };
    state.inner.db.with_write(|conn| {
        repo_attachment::insert(conn, &attachment)?;
        repo_sync::enqueue(
            conn,
            "attachment.upload",
            &attachment.id,
            json!({ "attachmentId": attachment.id.clone() }),
        )
    })?;
    Ok(attachment)
}

use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SketchLoad {
    pub scene_json: String,
    pub preview_png_base64: Option<String>,
}

#[tauri::command]
pub fn sketch_load(state: State<'_, AppState>, id: String) -> Result<SketchLoad, AppError> {
    let attachment = state
        .inner
        .db
        .with_read(|conn| repo_attachment::get(conn, &id))?;
    if attachment.kind != "sketch" {
        return Err(AppError::validation("Attachment is not a sketch"));
    }
    let scene_json = std::fs::read_to_string(&attachment.local_path)?;
    let preview_png_base64 = attachment.preview_path.and_then(|path| {
        std::fs::read(path)
            .ok()
            .map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes))
    });
    Ok(SketchLoad {
        scene_json,
        preview_png_base64,
    })
}

#[tauri::command]
pub fn attachment_delete(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let attachment = state
        .inner
        .db
        .with_read(|conn| repo_attachment::get(conn, &id))?;
    let path = format!(
        "attachments/{}/{}",
        attachment.entry_id,
        Path::new(&attachment.local_path)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default()
    );
    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_attachment::delete(&tx, &id)?;
        repo_sync::enqueue(
            &tx,
            "attachment.delete",
            &id,
            json!({ "path": path, "attachmentId": id.clone() }),
        )?;
        tx.commit()?;
        Ok(())
    })?;
    let _ = std::fs::remove_file(&attachment.local_path);
    Ok(())
}

#[tauri::command]
pub fn sketch_create(
    state: State<'_, AppState>,
    entry_id: String,
) -> Result<AttachmentDto, AppError> {
    if state
        .inner
        .db
        .with_read(|conn| repo_entry::find_row(conn, &entry_id))?
        .is_none()
    {
        return Err(AppError::not_found("Note not found"));
    }
    let id = new_id();
    let target_dir = state.mirror_dir().join("attachments").join(&entry_id);
    let scene_path = target_dir.join(format!("{id}.excalidraw"));
    let scene = serde_json::to_vec_pretty(&json!({
        "type": "excalidraw",
        "version": 2,
        "source": "better-notepad",
        "elements": [],
        "appState": {},
        "files": {}
    }))?;
    crate::util::atomic_write(&scene_path, &scene)?;
    let attachment = AttachmentDto {
        id,
        entry_id,
        kind: "sketch".to_string(),
        file_name: "Sketch".to_string(),
        mime_type: "application/vnd.excalidraw+json".to_string(),
        byte_size: scene.len() as i64,
        sha256: sha256_hex(&scene),
        local_path: scene_path.to_string_lossy().to_string(),
        preview_path: None,
        created_at: now_iso(),
    };
    state.inner.db.with_write(|conn| {
        repo_attachment::insert(conn, &attachment)?;
        repo_sync::enqueue(
            conn,
            "attachment.upload",
            &attachment.id,
            json!({ "attachmentId": attachment.id.clone() }),
        )
    })?;
    Ok(attachment)
}

#[tauri::command]
pub fn sketch_save(
    state: State<'_, AppState>,
    id: String,
    scene_json: String,
    preview_png_base64: Option<String>,
) -> Result<AttachmentDto, AppError> {
    let attachment = state
        .inner
        .db
        .with_read(|conn| repo_attachment::get(conn, &id))?;
    if attachment.kind != "sketch" {
        return Err(AppError::validation("Attachment is not a sketch"));
    }
    let scene_bytes = scene_json.as_bytes();
    if scene_bytes.len() > 10 * 1024 * 1024 {
        return Err(AppError::validation("Sketch exceeds the size limit"));
    }
    let scene_path = PathBuf::from(&attachment.local_path);
    crate::util::atomic_write(&scene_path, scene_bytes)?;
    let mut preview_path = attachment.preview_path.clone();
    if let Some(encoded) = preview_png_base64 {
        let trimmed = encoded
            .strip_prefix("data:image/png;base64,")
            .unwrap_or(&encoded);
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(trimmed)
            .map_err(|_| AppError::validation("Invalid preview image"))?;
        let path = scene_path.with_extension("png");
        crate::util::atomic_write(&path, &bytes)?;
        preview_path = Some(path.to_string_lossy().to_string());
    }
    let sha = sha256_hex(scene_bytes);
    state.inner.db.with_write(|conn| {
        repo_attachment::update_storage(
            conn,
            &id,
            scene_bytes.len() as i64,
            &sha,
            &attachment.local_path,
            preview_path.as_deref(),
        )?;
        repo_sync::enqueue(
            conn,
            "attachment.upload",
            &id,
            json!({ "attachmentId": id.clone() }),
        )
    })?;
    state
        .inner
        .db
        .with_read(|conn| repo_attachment::get(conn, &id))
}

#[tauri::command]
pub fn sketch_delete(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    attachment_delete(state, id)
}

fn mime_for(file_name: &str) -> String {
    let lower = file_name.to_lowercase();
    if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.ends_with(".webp") {
        "image/webp".to_string()
    } else if lower.ends_with(".gif") {
        "image/gif".to_string()
    } else if lower.ends_with(".svg") {
        "image/svg+xml".to_string()
    } else if lower.ends_with(".pdf") {
        "application/pdf".to_string()
    } else if lower.ends_with(".txt") {
        "text/plain".to_string()
    } else if lower.ends_with(".md") {
        "text/markdown".to_string()
    } else if lower.ends_with(".csv") {
        "text/csv".to_string()
    } else if lower.ends_with(".json") {
        "application/json".to_string()
    } else if lower.ends_with(".zip") {
        "application/zip".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

#[tauri::command]
pub fn attachment_open(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let attachment = state
        .inner
        .db
        .with_read(|conn| repo_attachment::get(conn, &id))?;
    open::that(&attachment.local_path)
        .map_err(|error| AppError::io(format!("Cannot open the attachment: {error}")))
}

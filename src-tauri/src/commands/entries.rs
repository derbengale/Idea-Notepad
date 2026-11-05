use crate::db::{repo_entry, repo_link, repo_meta, repo_project, repo_sync, repo_tag};
use crate::dto::{
    EntryDetail, EntryInput, EntryPage, EntryQuery, LinkBundle, LinkItem, SearchResult, TagDto,
};
use crate::error::AppError;
use crate::markdown;
use crate::schema;
use crate::state::AppState;
use crate::util::{new_id, now_iso, validate_title};
use rusqlite::Connection;
use serde_json::{json, Map, Value};
use tauri::State;

#[tauri::command]
pub fn entry_list(state: State<'_, AppState>, query: EntryQuery) -> Result<EntryPage, AppError> {
    state
        .inner
        .db
        .with_read(|conn| repo_entry::list(conn, &state.inner.schemas, &query))
}

#[tauri::command]
pub fn entry_get(state: State<'_, AppState>, id: String) -> Result<EntryDetail, AppError> {
    state
        .inner
        .db
        .with_read(|conn| repo_entry::get_detail(conn, &state.inner.schemas, &id))
}

#[tauri::command]
pub fn entry_create(
    state: State<'_, AppState>,
    input: EntryInput,
) -> Result<EntryDetail, AppError> {
    let schema = state.inner.schemas.require(&input.schema_id)?;
    let title = validate_title(&input.title)?;
    let project_id = validated_project(&state, input.project_id.as_deref())?;
    let mut values = input.values.clone();
    schema::validate::apply_defaults(&schema, &mut values);
    schema::validate::validate_values(&schema, &values)?;
    validate_tags(&state, &input.tag_ids)?;

    let id = input.id.clone().unwrap_or_else(new_id);
    let now = now_iso();
    let body = resolve_body(&state, &input.body_md)?;
    let md_links = markdown::extract_note_links(&body);
    let extra: Map<String, Value> = Map::new();

    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        let entry = repo_entry::NewEntry {
            schema_id: &schema.id,
            project_id: project_id.as_deref(),
            title: &title,
            body_md: &body,
            extra: &extra,
        };
        repo_entry::insert_entry(&tx, &id, &entry, &now, &now, "dirty")?;
        repo_entry::set_values(&tx, &id, &schema, &values)?;
        repo_tag::set_entry_tags(&tx, &id, &input.tag_ids)?;
        for target in &input.links {
            if repo_entry::find_row(&tx, target)?.is_some() {
                let _ = repo_link::create(&tx, &id, target);
            }
        }
        repo_link::replace_markdown_links(&tx, &id, &md_links)?;
        repo_sync::enqueue(&tx, "note.write", &id, json!({ "entryId": id.clone() }))?;
        tx.commit()?;
        Ok(())
    })?;
    remember_schema(&state, &schema.id)?;
    state
        .inner
        .db
        .with_read(|conn| repo_entry::get_detail(conn, &state.inner.schemas, &id))
}

#[tauri::command]
pub fn entry_update(
    state: State<'_, AppState>,
    input: EntryInput,
) -> Result<EntryDetail, AppError> {
    let id = input
        .id
        .clone()
        .ok_or_else(|| AppError::validation("Missing note id"))?;
    let existing = state
        .inner
        .db
        .with_read(|conn| repo_entry::get_row(conn, &id))?;
    if existing.deleted_at.is_some() {
        return Err(AppError::conflict("The note is in the trash"));
    }
    let schema = state.inner.schemas.require(&existing.schema_id)?;
    let title = validate_title(&input.title)?;
    let project_id = validated_project(&state, input.project_id.as_deref())?;
    schema::validate::validate_values(&schema, &input.values)?;
    validate_tags(&state, &input.tag_ids)?;
    let body = resolve_body(&state, &input.body_md)?;
    let md_links = markdown::extract_note_links(&body);
    let now = now_iso();
    let extra: Map<String, Value> = serde_json::from_str(&existing.extra_json).unwrap_or_default();

    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_entry::update_entry(
            &tx,
            &id,
            project_id.as_deref(),
            &title,
            &body,
            &now,
            &extra,
            input.expected_updated_at.as_deref(),
        )?;
        repo_entry::set_values(&tx, &id, &schema, &input.values)?;
        repo_tag::set_entry_tags(&tx, &id, &input.tag_ids)?;
        sync_entity_links(&tx, &id, &input.links)?;
        repo_link::replace_markdown_links(&tx, &id, &md_links)?;
        repo_sync::enqueue(&tx, "note.write", &id, json!({ "entryId": id.clone() }))?;
        tx.commit()?;
        Ok(())
    })?;
    remember_schema(&state, &schema.id)?;
    state
        .inner
        .db
        .with_read(|conn| repo_entry::get_detail(conn, &state.inner.schemas, &id))
}

#[tauri::command]
pub fn entry_delete(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let now = now_iso();
    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_entry::soft_delete(&tx, &id, &now)?;
        repo_sync::enqueue(&tx, "note.delete", &id, json!({ "entryId": id.clone() }))?;
        tx.commit()?;
        Ok(())
    })
}

#[tauri::command]
pub fn entry_restore(state: State<'_, AppState>, id: String) -> Result<EntryDetail, AppError> {
    let now = now_iso();
    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_entry::restore(&tx, &id, &now)?;
        repo_sync::enqueue(&tx, "note.write", &id, json!({ "entryId": id.clone() }))?;
        tx.commit()?;
        Ok(())
    })?;
    state
        .inner
        .db
        .with_read(|conn| repo_entry::get_detail(conn, &state.inner.schemas, &id))
}

#[tauri::command]
pub fn entry_duplicate(state: State<'_, AppState>, id: String) -> Result<EntryDetail, AppError> {
    let new_entry_id = state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        let new_id = repo_entry::duplicate(&tx, &id)?;
        repo_sync::enqueue(
            &tx,
            "note.write",
            &new_id,
            json!({ "entryId": new_id.clone() }),
        )?;
        tx.commit()?;
        Ok(new_id)
    })?;
    state
        .inner
        .db
        .with_read(|conn| repo_entry::get_detail(conn, &state.inner.schemas, &new_entry_id))
}

#[tauri::command]
pub fn entry_move_project(
    state: State<'_, AppState>,
    id: String,
    project_id: Option<String>,
) -> Result<EntryDetail, AppError> {
    let project_id = validated_project(&state, project_id.as_deref())?;
    let now = now_iso();
    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_entry::set_project(&tx, &id, project_id.as_deref(), &now)?;
        repo_sync::enqueue(&tx, "note.write", &id, json!({ "entryId": id.clone() }))?;
        repo_sync::enqueue(&tx, "projects.write", "projects", json!({}))?;
        tx.commit()?;
        Ok(())
    })?;
    state
        .inner
        .db
        .with_read(|conn| repo_entry::get_detail(conn, &state.inner.schemas, &id))
}

#[tauri::command]
pub fn entry_set_tags(
    state: State<'_, AppState>,
    id: String,
    tag_ids: Vec<String>,
) -> Result<EntryDetail, AppError> {
    validate_tags(&state, &tag_ids)?;
    let now = now_iso();
    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_tag::set_entry_tags(&tx, &id, &tag_ids)?;
        repo_entry::touch(&tx, &id, &now)?;
        repo_sync::enqueue(&tx, "note.write", &id, json!({ "entryId": id.clone() }))?;
        tx.commit()?;
        Ok(())
    })?;
    state
        .inner
        .db
        .with_read(|conn| repo_entry::get_detail(conn, &state.inner.schemas, &id))
}

#[tauri::command]
pub fn link_create(
    state: State<'_, AppState>,
    from_id: String,
    to_id: String,
) -> Result<LinkItem, AppError> {
    let now = now_iso();
    let link = state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        let link = repo_link::create(&tx, &from_id, &to_id)?;
        repo_entry::touch(&tx, &from_id, &now)?;
        repo_entry::touch(&tx, &to_id, &now)?;
        repo_sync::enqueue(
            &tx,
            "note.write",
            &from_id,
            json!({ "entryId": from_id.clone() }),
        )?;
        repo_sync::enqueue(
            &tx,
            "note.write",
            &to_id,
            json!({ "entryId": to_id.clone() }),
        )?;
        tx.commit()?;
        Ok(link)
    })?;
    let target = state
        .inner
        .db
        .with_read(|conn| repo_entry::get_row(conn, &to_id))?;
    let schema_key = state
        .inner
        .schemas
        .require(&target.schema_id)
        .map(|schema| schema.key.clone())
        .unwrap_or_default();
    Ok(LinkItem {
        id: Some(link.id),
        entry_id: to_id,
        title: target.title,
        schema_key,
        deleted: target.deleted_at.is_some(),
    })
}

#[tauri::command]
pub fn link_delete(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    state
        .inner
        .db
        .with_write(|conn| repo_link::delete(conn, &id))
}

#[tauri::command]
pub fn link_list(state: State<'_, AppState>, entry_id: String) -> Result<LinkBundle, AppError> {
    state.inner.db.with_read(|conn| {
        Ok(LinkBundle {
            links: repo_link::outgoing(conn, &entry_id)?,
            backlinks: repo_link::incoming(conn, &entry_id)?,
            markdown_links: repo_link::markdown_outgoing(conn, &entry_id)?,
            markdown_backlinks: repo_link::markdown_incoming(conn, &entry_id)?,
        })
    })
}

#[tauri::command]
pub fn tag_list(state: State<'_, AppState>) -> Result<Vec<TagDto>, AppError> {
    state.inner.db.with_read(repo_tag::list)
}

#[tauri::command]
pub fn tag_create(
    state: State<'_, AppState>,
    name: String,
    color: Option<String>,
) -> Result<TagDto, AppError> {
    state
        .inner
        .db
        .with_write(|conn| repo_tag::create(conn, &name, color.as_deref()))
}

#[tauri::command]
pub fn tag_delete(state: State<'_, AppState>, id: String) -> Result<(), AppError> {
    let affected = state
        .inner
        .db
        .with_read(|conn| repo_tag::entry_ids_for_tag(conn, &id))?;
    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_tag::delete(&tx, &id)?;
        for entry_id in affected {
            repo_sync::enqueue(
                &tx,
                "note.write",
                &entry_id,
                json!({ "entryId": entry_id.clone() }),
            )?;
        }
        tx.commit()?;
        Ok(())
    })
}

#[tauri::command]
pub fn search_query(
    state: State<'_, AppState>,
    text: String,
    limit: Option<i64>,
) -> Result<SearchResult, AppError> {
    state
        .inner
        .db
        .with_read(|conn| repo_entry::search(conn, &text, limit.unwrap_or(50)))
}

fn validate_tags(state: &State<'_, AppState>, tag_ids: &[String]) -> Result<(), AppError> {
    state
        .inner
        .db
        .with_read(|conn| repo_entry::validate_tags(conn, tag_ids))
}

fn validated_project(
    state: &State<'_, AppState>,
    project_id: Option<&str>,
) -> Result<Option<String>, AppError> {
    let Some(project_id) = project_id else {
        return Ok(None);
    };
    let exists = state
        .inner
        .db
        .with_read(|conn| repo_project::get(conn, project_id))
        .is_ok();
    if !exists {
        return Err(AppError::not_found("Project not found"));
    }
    Ok(Some(project_id.to_string()))
}

fn resolve_body(state: &State<'_, AppState>, body: &str) -> Result<String, AppError> {
    let resolved = state.inner.db.with_read(|conn| {
        Ok(markdown::resolve_wiki_links(body, |title| {
            repo_entry::find_by_title(conn, title).unwrap_or_default()
        }))
    })?;
    Ok(crate::util::normalize_body(&resolved))
}

fn sync_entity_links(
    conn: &Connection,
    entry_id: &str,
    targets: &[String],
) -> Result<(), AppError> {
    let existing = repo_link::outgoing(conn, entry_id)?;
    for link in &existing {
        if !targets.contains(&link.entry_id) {
            if let Some(id) = &link.id {
                repo_link::delete(conn, id)?;
            }
        }
    }
    for target in targets {
        if repo_entry::find_row(conn, target)?.is_some() {
            let _ = repo_link::create(conn, entry_id, target);
        }
    }
    Ok(())
}

fn remember_schema(state: &State<'_, AppState>, schema_id: &str) -> Result<(), AppError> {
    let mut settings = state.settings();
    if settings.last_schema_id.as_deref() != Some(schema_id) {
        settings.last_schema_id = Some(schema_id.to_string());
        state.save_settings(settings)?;
    }
    let _ = state
        .inner
        .db
        .with_write(|conn| repo_meta::set(conn, "workspace.lastSchemaId", schema_id));
    Ok(())
}

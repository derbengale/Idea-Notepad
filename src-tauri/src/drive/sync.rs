use crate::db::{
    repo_attachment, repo_entry, repo_link, repo_meta, repo_project, repo_schema, repo_sync,
    repo_tag,
};
use crate::dto::{AuthStatus, FieldDefDto, ProjectDto, SchemaDto, SyncStatus};
use crate::error::{AppError, ErrorCode};
use crate::markdown::{self, NoteFile};
use crate::schema;
use crate::state::AppState;
use crate::util::{new_id, note_file_name, now_iso, truncate_chars};
use chrono::Utc;
use rusqlite::Connection;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::sync::atomic::Ordering;

pub struct Folders {
    pub notes: String,
    pub schemas: String,
    pub attachments: String,
}

impl Folders {
    fn ensure(client: &crate::drive::client::DriveClient) -> Result<Self, AppError> {
        Ok(Self {
            notes: client.ensure_folder("notes", "appDataFolder")?,
            schemas: client.ensure_folder("schemas", "appDataFolder")?,
            attachments: client.ensure_folder("attachments", "appDataFolder")?,
        })
    }
}

pub fn auth_status(state: &AppState) -> Result<AuthStatus, AppError> {
    let signed_in = state
        .inner
        .db
        .with_read(|conn| repo_meta::get(conn, "auth.sub"))?
        .is_some();
    let email = state
        .inner
        .db
        .with_read(|conn| repo_meta::get(conn, "auth.email"))?;
    let name = state
        .inner
        .db
        .with_read(|conn| repo_meta::get(conn, "auth.name"))?;
    let picture = state
        .inner
        .db
        .with_read(|conn| repo_meta::get(conn, "auth.picture"))?;
    Ok(AuthStatus {
        signed_in,
        email,
        name,
        picture,
        has_client_id: state.inner.auth.client_id().is_some(),
    })
}

pub fn access_token(state: &AppState) -> Result<String, AppError> {
    let client_id = state.inner.auth.client_id().ok_or_else(|| {
        AppError::unsupported("No Google OAuth client ID is configured in this build.")
    })?;
    if let Some(cached) = state.inner.auth.cached_token() {
        if cached.expires_at > Utc::now().timestamp() + 60 {
            return Ok(cached.token);
        }
    }
    let sub = state
        .inner
        .db
        .with_read(|conn| repo_meta::get(conn, "auth.sub"))?
        .ok_or_else(|| AppError::auth_required("Not signed in to Google"))?;
    let refresh_token = crate::drive::oauth::load_refresh_token(&sub)?
        .ok_or_else(|| AppError::auth_required("The Google credential is missing"))?;
    let client_secret = state.inner.auth.client_secret();
    match crate::drive::oauth::refresh(&client_id, client_secret.as_deref(), &refresh_token) {
        Ok((token, expires_in)) => {
            state
                .inner
                .auth
                .store_token(token.clone(), Utc::now().timestamp() + expires_in);
            Ok(token)
        }
        Err(error) => {
            if error.code == ErrorCode::AuthRequired {
                let _ = crate::drive::oauth::delete_refresh_token(&sub);
                state.inner.auth.clear();
            }
            Err(error)
        }
    }
}

pub fn sign_in(state: &AppState) -> Result<AuthStatus, AppError> {
    if state.inner.auth.signing_in.swap(true, Ordering::SeqCst) {
        return Err(AppError::conflict("A sign-in is already in progress"));
    }
    let result = (|| {
        let client_id = state.inner.auth.client_id().ok_or_else(|| {
            AppError::unsupported("No Google OAuth client ID is configured in this build.")
        })?;
        let client_secret = state.inner.auth.client_secret();
        let result = crate::drive::oauth::sign_in(&client_id, client_secret.as_deref())?;
        crate::drive::oauth::store_refresh_token(&result.profile.sub, &result.refresh_token)?;
        state.inner.db.with_write(|conn| {
            repo_meta::set(conn, "auth.sub", &result.profile.sub)?;
            if let Some(email) = &result.profile.email {
                repo_meta::set(conn, "auth.email", email)?;
            }
            if let Some(name) = &result.profile.name {
                repo_meta::set(conn, "auth.name", name)?;
            }
            if let Some(picture) = &result.profile.picture {
                repo_meta::set(conn, "auth.picture", picture)?;
            }
            Ok(())
        })?;
        state.inner.auth.store_token(
            result.access_token,
            Utc::now().timestamp() + result.expires_in,
        );
        auth_status(state)
    })();
    state.inner.auth.signing_in.store(false, Ordering::SeqCst);
    if let Err(error) = &result {
        tracing::warn!(
            code = error.code.as_str(),
            message = %error.message,
            "Google sign-in failed"
        );
    }
    if let Ok(status) = &result {
        state.emit("auth://changed", status.clone());
    }
    result
}

pub fn sign_out(state: &AppState, delete_local: bool) -> Result<(), AppError> {
    let sub = state
        .inner
        .db
        .with_read(|conn| repo_meta::get(conn, "auth.sub"))?;
    if let Some(sub) = &sub {
        if let Ok(Some(token)) = crate::drive::oauth::load_refresh_token(sub) {
            crate::drive::oauth::revoke(&token);
        }
        let _ = crate::drive::oauth::delete_refresh_token(sub);
    }
    state.inner.db.with_write(|conn| {
        for key in ["auth.sub", "auth.email", "auth.name", "auth.picture"] {
            repo_meta::delete(conn, key)?;
        }
        Ok(())
    })?;
    state.inner.auth.clear();
    if delete_local {
        state.inner.db.with_write(|conn| {
            repo_entry::clear_all(conn)?;
            repo_project::clear_all(conn)?;
            repo_sync::clear(conn)?;
            repo_sync::clear_files(conn)?;
            Ok(())
        })?;
        let mirror = state.mirror_dir().join("notes");
        let _ = std::fs::remove_dir_all(mirror);
    }
    state.emit(
        "auth://changed",
        AuthStatus {
            signed_in: false,
            email: None,
            name: None,
            picture: None,
            has_client_id: state.inner.auth.client_id().is_some(),
        },
    );
    Ok(())
}

pub fn status(state: &AppState) -> SyncStatus {
    let pending = state
        .inner
        .db
        .with_read(repo_sync::pending_count)
        .unwrap_or(0);
    let conflicts = state
        .inner
        .db
        .with_read(repo_entry::conflicting_count)
        .unwrap_or(0);
    let mut status = state.inner.sync.status.read().clone();
    status.pending_operations = pending;
    status.conflict_count = conflicts;
    status
}

fn publish(state: &AppState, status: SyncStatus) {
    *state.inner.sync.status.write() = status.clone();
    state.emit("sync://status", status);
}

pub fn run_cycle(state: &AppState) -> Result<SyncStatus, AppError> {
    let _guard = state.inner.sync.cycle.lock();
    if state.inner.sync.paused.load(Ordering::SeqCst) {
        let mut paused = status(state);
        paused.state = "paused".to_string();
        publish(state, paused.clone());
        return Ok(paused);
    }
    let mut syncing = status(state);
    syncing.state = "syncing".to_string();
    syncing.last_error = None;
    publish(state, syncing);

    let result = cycle_inner(state);
    match result {
        Ok(()) => {
            let mut done = status(state);
            done.state = "idle".to_string();
            done.last_error = None;
            done.last_sync_at = Some(now_iso());
            let _ = state
                .inner
                .db
                .with_write(|conn| repo_meta::set(conn, "sync.lastSyncAt", &now_iso()));
            publish(state, done.clone());
            Ok(done)
        }
        Err(error) => {
            let mut failed = status(state);
            failed.state = match error.code {
                ErrorCode::AuthRequired => "error".to_string(),
                ErrorCode::Network => "offline".to_string(),
                _ => "error".to_string(),
            };
            failed.last_error = Some(error.clone());
            publish(state, failed);
            Err(error)
        }
    }
}

fn cycle_inner(state: &AppState) -> Result<(), AppError> {
    let token = access_token(state)?;
    let client = crate::drive::client::DriveClient::new(token)?;
    let folders = Folders::ensure(&client)?;
    pull(state, &client, &folders)?;
    push(state, &client, &folders)?;
    Ok(())
}

fn set_page_token(state: &AppState, token: &str) -> Result<(), AppError> {
    state
        .inner
        .db
        .with_write(|conn| repo_meta::set(conn, "sync.pageToken", token))
}

fn clear_page_token(state: &AppState) -> Result<(), AppError> {
    state
        .inner
        .db
        .with_write(|conn| repo_meta::delete(conn, "sync.pageToken"))
}

fn pull(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    folders: &Folders,
) -> Result<(), AppError> {
    let existing = state
        .inner
        .db
        .with_read(|conn| repo_meta::get(conn, "sync.pageToken"))?;
    let Some(mut page_token) = existing else {
        bootstrap(state, client, folders)?;
        return Ok(());
    };
    loop {
        let page = match client.changes(&page_token) {
            Ok(page) => page,
            Err(error) if error.code == ErrorCode::NotFound => {
                clear_page_token(state)?;
                bootstrap(state, client, folders)?;
                return Ok(());
            }
            Err(error) => return Err(error),
        };
        for change in &page.changes {
            if let Err(error) = handle_change(state, client, folders, change) {
                tracing::warn!(error = %error, file = %change.file_id, "skipped Drive change");
            }
        }
        if let Some(new_token) = page.new_start_token {
            set_page_token(state, &new_token)?;
            break;
        }
        match page.next_page_token {
            Some(next) => {
                set_page_token(state, &next)?;
                page_token = next;
            }
            None => break,
        }
    }
    Ok(())
}

fn handle_change(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    folders: &Folders,
    change: &crate::drive::client::DriveChange,
) -> Result<(), AppError> {
    if change.removed {
        return handle_removal(state, &change.file_id);
    }
    let Some(file) = &change.file else {
        return Ok(());
    };
    if file.trashed {
        return handle_removal(state, &change.file_id);
    }
    if crate::drive::client::is_folder(file) {
        return Ok(());
    }
    let parent = file.parents.first().cloned().unwrap_or_default();
    if parent == folders.notes && crate::drive::client::is_markdown(file) {
        let synced = state
            .inner
            .db
            .with_read(|conn| repo_sync::get_by_path(conn, &format!("notes/{}", file.name)))?;
        if let Some(synced) = &synced {
            if synced.modified_at == file.modified_time {
                return Ok(());
            }
        }
        let bytes = client.download(&file.id)?;
        apply_remote_note(state, &bytes, file)?;
        return Ok(());
    }
    if parent == folders.schemas && crate::drive::client::is_json(file) {
        let bytes = client.download(&file.id)?;
        let schema: SchemaDto = serde_json::from_slice(&bytes)?;
        state
            .inner
            .db
            .with_write(|conn| repo_schema::upsert_remote(conn, &schema))?;
        state.refresh_schemas()?;
        state.inner.db.with_write(|conn| {
            record_file(conn, &format!("schemas/{}", file.name), "schema", file)
        })?;
        return Ok(());
    }
    if parent == "appDataFolder" && file.name == "projects.json" {
        let bytes = client.download(&file.id)?;
        let projects: Vec<ProjectDto> = serde_json::from_slice(&bytes)?;
        state.inner.db.with_write(|conn| {
            for project in projects {
                repo_project::insert_remote(
                    conn,
                    &project.id,
                    &project.name,
                    &project.description,
                    &project.color,
                    &project.icon,
                    project.sort_order,
                    &project.created_at,
                    &project.updated_at,
                )?;
            }
            record_file(conn, "projects.json", "projects", file)?;
            Ok(())
        })?;
        return Ok(());
    }
    Ok(())
}

fn record_file(
    conn: &Connection,
    path: &str,
    kind: &str,
    file: &crate::drive::client::DriveFile,
) -> Result<(), AppError> {
    repo_sync::upsert_file(
        conn,
        &repo_sync::SyncFile {
            drive_file_id: file.id.clone(),
            path: path.to_string(),
            kind: kind.to_string(),
            etag: None,
            modified_at: file.modified_time.clone(),
            local_rev: 0,
            sha256: file.md5.clone(),
            last_synced_at: Some(now_iso()),
        },
    )
}

fn handle_removal(state: &AppState, drive_id: &str) -> Result<(), AppError> {
    let entry = state
        .inner
        .db
        .with_read(|conn| repo_entry::find_by_drive_id(conn, drive_id))?;
    if let Some(entry) = entry {
        if entry.sync_state == "dirty" || entry.sync_state == "conflict" {
            state.inner.db.with_write(|conn| {
                repo_sync::enqueue(
                    conn,
                    "note.write",
                    &entry.id,
                    json!({ "entryId": entry.id }),
                )
            })?;
        } else if entry.deleted_at.is_none() {
            state.inner.db.with_write(|conn| {
                repo_entry::soft_delete(conn, &entry.id, &now_iso())?;
                repo_entry::set_sync_state(conn, &entry.id, "clean")
            })?;
        }
    }
    state
        .inner
        .db
        .with_write(|conn| repo_sync::delete_file(conn, drive_id))
}

fn apply_remote_note(
    state: &AppState,
    bytes: &[u8],
    file: &crate::drive::client::DriveFile,
) -> Result<(), AppError> {
    let text = String::from_utf8(bytes.to_vec())
        .map_err(|_| AppError::validation("Remote note is not valid UTF-8"))?;
    let note = markdown::parse_note(&text)?;
    let schema = ensure_schema(state, &note)?;
    let existing = state
        .inner
        .db
        .with_read(|conn| repo_entry::find_row(conn, &note.id))?;

    if let Some(existing) = &existing {
        if existing.sync_state == "dirty" || existing.sync_state == "conflict" {
            let copy_id = create_conflict_copy(state, &note, &schema, existing)?;
            state
                .inner
                .db
                .with_write(|conn| repo_entry::set_sync_state(conn, &existing.id, "conflict"))?;
            state.emit(
                "sync://conflict",
                json!({
                    "entryId": existing.id,
                    "conflictEntryId": copy_id,
                    "title": existing.title
                }),
            );
            return Ok(());
        }
    }

    let project_id = resolve_project(state, &note)?;
    let tag_ids = state
        .inner
        .db
        .with_write(|conn| repo_tag::resolve_ids(conn, &note.tags))?;
    let values = valid_values(&schema, &note.fields);
    let body = note.body.clone();
    let title = truncate_chars(note.title.trim(), 300);
    let extra = note.extra.clone();
    let note_id = note.id.clone();
    let schema_id = schema.id.clone();
    let created_at = note.created.clone();
    let updated_at = note.updated.clone();
    let drive_id = file.id.clone();
    let modified = file.modified_time.clone();
    let path = format!("notes/{}", file.name);
    let file_name = file.name.clone();

    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        let remote = repo_entry::RemoteEntry {
            id: &note_id,
            schema_id: &schema_id,
            project_id: project_id.as_deref(),
            title: &title,
            body_md: &body,
            created_at: &created_at,
            updated_at: &updated_at,
            drive_file_id: &drive_id,
            extra: &extra,
        };
        if repo_entry::find_row(&tx, &note_id)?.is_some() {
            repo_entry::update_remote(&tx, &remote)?;
        } else {
            repo_entry::insert_remote(&tx, &remote)?;
        }
        repo_entry::set_values(&tx, &note_id, &schema, &values)?;
        repo_tag::set_entry_tags(&tx, &note_id, &tag_ids)?;
        repo_link::replace_markdown_links(&tx, &note_id, &markdown::extract_note_links(&body))?;
        for target in &note.links {
            if repo_entry::find_row(&tx, target)?.is_some() {
                let _ = repo_link::create(&tx, &note_id, target);
            }
        }
        record_file(
            &tx,
            &path,
            "note",
            &crate::drive::client::DriveFile {
                id: drive_id.clone(),
                name: file_name.clone(),
                mime_type: file.mime_type.clone(),
                modified_time: modified.clone(),
                md5: file.md5.clone(),
                parents: file.parents.clone(),
                trashed: false,
            },
        )?;
        tx.commit()?;
        Ok(())
    })?;

    let mirror = state.mirror_dir().join("notes").join(&file_name);
    crate::util::atomic_write(&mirror, bytes)?;
    Ok(())
}

fn valid_values(schema: &SchemaDto, fields: &Map<String, Value>) -> Map<String, Value> {
    let mut values = Map::new();
    for field in &schema.fields {
        if field.field_type == "formula" || field.field_type == "tags" {
            continue;
        }
        let Some(value) = fields.get(&field.key) else {
            continue;
        };
        let normalized = match (field.field_type.as_str(), value) {
            ("number", Value::String(text)) => text
                .parse::<f64>()
                .ok()
                .and_then(serde_json::Number::from_f64)
                .map(Value::Number)
                .unwrap_or(Value::Null),
            _ => value.clone(),
        };
        if schema::validate::validate_field(field, Some(&normalized)).is_ok() {
            values.insert(field.key.clone(), normalized);
        }
    }
    values
}

fn resolve_project(state: &AppState, note: &NoteFile) -> Result<Option<String>, AppError> {
    let Some(project_id) = &note.project else {
        return Ok(None);
    };
    let existing = state
        .inner
        .db
        .with_read(repo_project::list)?
        .into_iter()
        .find(|project| &project.id == project_id);
    if let Some(project) = existing {
        return Ok(Some(project.id));
    }
    let name = note
        .project_name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "Imported project".to_string());
    let existing_by_name = state
        .inner
        .db
        .with_read(|conn| repo_project::find_by_name(conn, &name))?;
    if let Some(project) = existing_by_name {
        return Ok(Some(project.id));
    }
    let now = now_iso();
    state.inner.db.with_write(|conn| {
        repo_project::insert_remote(
            conn, project_id, &name, "", "#64748b", "folder", 0, &now, &now,
        )
    })?;
    Ok(Some(project_id.clone()))
}

pub fn ensure_schema(state: &AppState, note: &NoteFile) -> Result<SchemaDto, AppError> {
    if let Some(schema) = state.inner.schemas.get_by_key(&note.schema) {
        return Ok((*schema).clone());
    }
    let mut fields = Vec::new();
    for (index, (key, value)) in note.fields.iter().enumerate() {
        let field_type = match value {
            Value::Number(_) => "number",
            Value::Bool(_) => "checkbox",
            _ => "text",
        };
        let config = match field_type {
            "text" => json!({ "maxLength": 20000 }),
            "number" => json!({}),
            "checkbox" => json!({}),
            _ => json!({}),
        };
        fields.push(FieldDefDto {
            id: new_id(),
            key: key.clone(),
            label: key.clone(),
            field_type: field_type.to_string(),
            required: false,
            order: index as i64,
            config,
            expression: None,
            show_in_list: index < 6,
        });
    }
    let key = if schema::validate::valid_key(&note.schema) {
        note.schema.clone()
    } else {
        "imported".to_string()
    };
    let mut key = key;
    let mut counter = 2;
    while state.inner.schemas.get_by_key(&key).is_some() {
        key = format!(
            "{}_{counter}",
            note.schema.replace(
                |ch: char| !ch.is_ascii_lowercase() && !ch.is_ascii_digit() && ch != '_',
                "_"
            )
        );
        counter += 1;
    }
    let schema = SchemaDto {
        id: new_id(),
        key: key.clone(),
        name: if note.schema.trim().is_empty() {
            key.clone()
        } else {
            note.schema.clone()
        },
        icon: "file-text".to_string(),
        color: "#64748b".to_string(),
        order: state.inner.schemas.list().len() as i64,
        title_field_id: None,
        subtitle_field_id: None,
        badge_field_id: None,
        format_version: 1,
        fields,
    };
    state
        .inner
        .db
        .with_write(|conn| repo_schema::insert(conn, &schema))?;
    state.refresh_schemas()?;
    Ok(schema)
}

fn create_conflict_copy(
    state: &AppState,
    note: &NoteFile,
    schema: &SchemaDto,
    original: &repo_entry::EntryRow,
) -> Result<String, AppError> {
    let new_id = new_id();
    let title = format!(
        "{} (conflict {})",
        truncate_chars(note.title.trim(), 260),
        Utc::now().format("%Y-%m-%d %H:%M")
    );
    let values = valid_values(schema, &note.fields);
    let tag_ids = state
        .inner
        .db
        .with_write(|conn| repo_tag::resolve_ids(conn, &note.tags))?;
    let now = now_iso();
    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        let entry = repo_entry::NewEntry {
            schema_id: &schema.id,
            project_id: original.project_id.as_deref(),
            title: &title,
            body_md: &note.body,
            extra: &note.extra,
        };
        repo_entry::insert_entry(&tx, &new_id, &entry, &now, &now, "local")?;
        repo_entry::set_conflict_of(&tx, &new_id, &original.id)?;
        repo_entry::set_values(&tx, &new_id, schema, &values)?;
        repo_tag::set_entry_tags(&tx, &new_id, &tag_ids)?;
        repo_link::replace_markdown_links(&tx, &new_id, &markdown::extract_note_links(&note.body))?;
        repo_sync::enqueue(
            &tx,
            "note.write",
            &new_id,
            json!({ "entryId": new_id.clone() }),
        )?;
        tx.commit()?;
        Ok(())
    })?;
    Ok(new_id)
}

fn bootstrap(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    folders: &Folders,
) -> Result<(), AppError> {
    let files = client.list_all()?;
    let folder_names: HashMap<String, String> = files
        .iter()
        .filter(|file| crate::drive::client::is_folder(file))
        .map(|file| (file.id.clone(), file.name.clone()))
        .collect();

    for file in &files {
        let parent = file.parents.first().cloned().unwrap_or_default();
        if parent == folders.schemas && crate::drive::client::is_json(file) {
            if let Ok(bytes) = client.download(&file.id) {
                if let Ok(schema) = serde_json::from_slice::<SchemaDto>(&bytes) {
                    let _ = state
                        .inner
                        .db
                        .with_write(|conn| repo_schema::upsert_remote(conn, &schema));
                    let _ = state.inner.db.with_write(|conn| {
                        record_file(conn, &format!("schemas/{}", file.name), "schema", file)
                    });
                }
            }
        }
    }
    state.refresh_schemas()?;

    for file in &files {
        let parent = file.parents.first().cloned().unwrap_or_default();
        if parent == folders.notes && crate::drive::client::is_markdown(file) {
            if let Ok(bytes) = client.download(&file.id) {
                let _ = apply_remote_note(state, &bytes, file);
            }
        }
        if parent == "appDataFolder" && file.name == "projects.json" {
            if let Ok(bytes) = client.download(&file.id) {
                if let Ok(projects) = serde_json::from_slice::<Vec<ProjectDto>>(&bytes) {
                    let _ = state.inner.db.with_write(|conn| {
                        for project in projects {
                            let _ = repo_project::insert_remote(
                                conn,
                                &project.id,
                                &project.name,
                                &project.description,
                                &project.color,
                                &project.icon,
                                project.sort_order,
                                &project.created_at,
                                &project.updated_at,
                            );
                        }
                        record_file(conn, "projects.json", "projects", file)
                    });
                }
            }
        }
        if parent == folders.attachments {
            download_attachment(state, client, file, &folder_names)?;
        }
    }

    let token = client.start_page_token()?;
    set_page_token(state, &token)?;
    state.emit_workspace_changed("sync");
    Ok(())
}

fn download_attachment(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    file: &crate::drive::client::DriveFile,
    folder_names: &HashMap<String, String>,
) -> Result<(), AppError> {
    let parent = file.parents.first().cloned().unwrap_or_default();
    let Some(entry_id) = folder_names.get(&parent) else {
        return Ok(());
    };
    if !crate::util::is_uuid(entry_id) {
        return Ok(());
    }
    let entry_exists = state
        .inner
        .db
        .with_read(|conn| repo_entry::find_row(conn, entry_id))?
        .is_some();
    if !entry_exists {
        return Ok(());
    }
    let bytes = client.download(&file.id)?;
    let target_dir = state.mirror_dir().join("attachments").join(entry_id);
    let target = target_dir.join(&file.name);
    crate::util::atomic_write(&target, &bytes)?;
    let attachment_id = file.name.split('.').next().unwrap_or_default().to_string();
    if !crate::util::is_uuid(&attachment_id) {
        return Ok(());
    }
    let kind = if file.name.ends_with(".excalidraw")
        || (file.name.ends_with(".png") && file.name.contains("sketch"))
    {
        "sketch"
    } else if file.mime_type.starts_with("image/") || file.name.ends_with(".png") {
        "image"
    } else {
        "file"
    };
    let existing = state
        .inner
        .db
        .with_read(|conn| repo_attachment::get(conn, &attachment_id));
    if existing.is_ok() {
        return Ok(());
    }
    let mime = file_extension_mime(&file.name, &file.mime_type);
    let attachment = crate::dto::AttachmentDto {
        id: attachment_id,
        entry_id: entry_id.clone(),
        kind: kind.to_string(),
        file_name: file.name.clone(),
        mime_type: mime,
        byte_size: bytes.len() as i64,
        sha256: crate::util::sha256_hex(&bytes),
        local_path: target.to_string_lossy().to_string(),
        preview_path: None,
        created_at: now_iso(),
    };
    state
        .inner
        .db
        .with_write(|conn| repo_attachment::insert(conn, &attachment))?;
    Ok(())
}

fn file_extension_mime(name: &str, fallback: &str) -> String {
    let lower = name.to_lowercase();
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
    } else if lower.ends_with(".md") {
        "text/markdown".to_string()
    } else if lower.ends_with(".json") || lower.ends_with(".excalidraw") {
        "application/json".to_string()
    } else {
        fallback.to_string()
    }
}

fn push(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    folders: &Folders,
) -> Result<(), AppError> {
    let ops = state
        .inner
        .db
        .with_read(|conn| repo_sync::next_ops(conn, 25))?;
    for op in ops {
        let result = process_op(state, client, folders, &op);
        match result {
            Ok(()) => {
                state
                    .inner
                    .db
                    .with_write(|conn| repo_sync::complete(conn, &op.id))?;
            }
            Err(error) => {
                let attempts = op.attempts + 1;
                let delay = (attempts * attempts * 30).min(300);
                let retry_at = (Utc::now() + chrono::Duration::seconds(delay)).to_rfc3339();
                state
                    .inner
                    .db
                    .with_write(|conn| repo_sync::fail(conn, &op.id, &error.message, &retry_at))?;
                tracing::warn!(op = %op.op, error = %error, "sync operation failed");
            }
        }
    }
    Ok(())
}

fn process_op(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    folders: &Folders,
    op: &repo_sync::OutboxOp,
) -> Result<(), AppError> {
    match op.op.as_str() {
        "note.write" => push_note(state, client, folders, &op.target),
        "note.delete" => delete_note(state, client, &op.target),
        "schema.write" => push_schema(state, client, folders, &op.target),
        "projects.write" => push_projects(state, client),
        "workspace.write" => push_workspace(state, client),
        "attachment.upload" => push_attachment(state, client, folders, &op.target),
        "attachment.delete" => delete_attachment(state, client, &op.payload_json),
        _ => Ok(()),
    }
}

fn delete_attachment(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    payload: &str,
) -> Result<(), AppError> {
    let payload: Value = serde_json::from_str(payload).unwrap_or(Value::Null);
    let path = payload
        .get("path")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if path.is_empty() {
        return Ok(());
    }
    let synced = state
        .inner
        .db
        .with_read(|conn| repo_sync::get_by_path(conn, path))?;
    if let Some(synced) = synced {
        client.delete_file(&synced.drive_file_id)?;
        state
            .inner
            .db
            .with_write(|conn| repo_sync::delete_file(conn, &synced.drive_file_id))?;
    }
    Ok(())
}

fn push_note(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    folders: &Folders,
    entry_id: &str,
) -> Result<(), AppError> {
    let Some(row) = state
        .inner
        .db
        .with_read(|conn| repo_entry::find_row(conn, entry_id))?
    else {
        return Ok(());
    };
    if row.deleted_at.is_some() {
        return delete_remote_note(state, client, &row);
    }
    let (file_name, markdown) = note_payload(state, &row)?;
    let bytes = markdown.into_bytes();
    let path = format!("notes/{file_name}");
    let mirror = state.mirror_dir().join("notes").join(&file_name);
    crate::util::atomic_write(&mirror, &bytes)?;

    let synced = state
        .inner
        .db
        .with_read(|conn| repo_sync::get_by_path(conn, &path))?;
    let drive_id = row
        .drive_file_id
        .clone()
        .or_else(|| synced.as_ref().map(|file| file.drive_file_id.clone()));

    let uploaded = if let Some(drive_id) = drive_id {
        match client.get_metadata(&drive_id) {
            Ok(metadata) => {
                if let Some(synced) = &synced {
                    if metadata.modified_time != synced.modified_at {
                        if let Ok(remote_bytes) = client.download(&drive_id) {
                            if let Ok(remote_text) = String::from_utf8(remote_bytes) {
                                if let Ok(remote_note) = markdown::parse_note(&remote_text) {
                                    let schema = ensure_schema(state, &remote_note)?;
                                    if let Ok(copy_id) =
                                        create_conflict_copy(state, &remote_note, &schema, &row)
                                    {
                                        state.inner.db.with_write(|conn| {
                                            repo_entry::set_sync_state(conn, &row.id, "conflict")
                                        })?;
                                        state.emit(
                                            "sync://conflict",
                                            json!({
                                                "entryId": row.id,
                                                "conflictEntryId": copy_id,
                                                "title": row.title
                                            }),
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
                Some(client.update_file(&drive_id, "text/markdown", &bytes)?)
            }
            Err(_) => Some(client.create_file(
                &file_name,
                &folders.notes,
                "text/markdown",
                &bytes,
                json!({ "bn.kind": "note", "bn.entryId": row.id }),
            )?),
        }
    } else {
        Some(client.create_file(
            &file_name,
            &folders.notes,
            "text/markdown",
            &bytes,
            json!({ "bn.kind": "note", "bn.entryId": row.id }),
        )?)
    };

    if let Some(file) = uploaded {
        let rev = state
            .inner
            .db
            .with_read(|conn| repo_entry::local_rev(conn, &row.id))?;
        state.inner.db.with_write(|conn| {
            let tx = conn.transaction()?;
            record_file(&tx, &path, "note", &file)?;
            repo_entry::set_drive_meta(&tx, &row.id, &file.id, file.modified_time.as_deref())?;
            repo_entry::mark_clean_if_rev(&tx, &row.id, rev)?;
            tx.commit()?;
            Ok(())
        })?;
    }
    Ok(())
}

pub fn note_payload(
    state: &AppState,
    row: &repo_entry::EntryRow,
) -> Result<(String, String), AppError> {
    let schema = state.inner.schemas.require(&row.schema_id)?;
    let conn = state.inner.db.with_read(|conn| {
        let values = repo_entry::load_values(conn, &row.id, &schema)?;
        let tags = repo_tag::resolve_names(conn, &row.id)?;
        let links = repo_link::outgoing(conn, &row.id)?;
        let project_name = match &row.project_id {
            Some(project_id) => repo_project::list(conn)?
                .into_iter()
                .find(|project| &project.id == project_id)
                .map(|project| project.name),
            None => None,
        };
        Ok((values, tags, links, project_name))
    })?;
    let (values, tags, links, project_name) = conn;
    let mut fields = Map::new();
    for field in &schema.fields {
        if field.field_type == "formula" || field.field_type == "tags" {
            continue;
        }
        if let Some(value) = values.get(&field.key) {
            if !value.is_null() {
                fields.insert(field.key.clone(), value.clone());
            }
        }
    }
    let extra: Map<String, Value> = serde_json::from_str(&row.extra_json).unwrap_or_default();
    let note = NoteFile {
        id: row.id.clone(),
        schema: schema.key.clone(),
        title: row.title.clone(),
        project: row.project_id.clone(),
        project_name,
        tags,
        links: links.into_iter().map(|link| link.entry_id).collect(),
        created: row.created_at.clone(),
        updated: row.updated_at.clone(),
        fields,
        extra,
        body: row.body_md.clone(),
    };
    let file_name = note_file_name(&row.title, &row.id);
    Ok((file_name, markdown::serialize_note(&note)))
}

fn delete_note(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    entry_id: &str,
) -> Result<(), AppError> {
    let Some(row) = state
        .inner
        .db
        .with_read(|conn| repo_entry::find_row(conn, entry_id))?
    else {
        return Ok(());
    };
    delete_remote_note(state, client, &row)
}

fn delete_remote_note(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    row: &repo_entry::EntryRow,
) -> Result<(), AppError> {
    if let Some(drive_id) = &row.drive_file_id {
        client.delete_file(drive_id)?;
        state
            .inner
            .db
            .with_write(|conn| repo_sync::delete_file(conn, drive_id))?;
    }
    state
        .inner
        .db
        .with_write(|conn| repo_entry::remove_drive_meta(conn, &row.id))?;
    Ok(())
}

fn push_schema(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    folders: &Folders,
    schema_id: &str,
) -> Result<(), AppError> {
    let schema = state.inner.schemas.require(schema_id)?;
    let bytes = serde_json::to_vec_pretty(&*schema)?;
    let file_name = format!("{}.json", schema.key);
    let path = format!("schemas/{file_name}");
    let synced = state
        .inner
        .db
        .with_read(|conn| repo_sync::get_by_path(conn, &path))?;
    let file = match synced {
        Some(synced) => client.update_file(&synced.drive_file_id, "application/json", &bytes)?,
        None => client.create_file(
            &file_name,
            &folders.schemas,
            "application/json",
            &bytes,
            json!({ "bn.kind": "schema", "bn.schemaId": schema.id }),
        )?,
    };
    state
        .inner
        .db
        .with_write(|conn| record_file(conn, &path, "schema", &file))
}

fn push_projects(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
) -> Result<(), AppError> {
    let projects = state.inner.db.with_read(repo_project::list)?;
    let bytes = serde_json::to_vec_pretty(&projects)?;
    let synced = state
        .inner
        .db
        .with_read(|conn| repo_sync::get_by_path(conn, "projects.json"))?;
    let file = match synced {
        Some(synced) => client.update_file(&synced.drive_file_id, "application/json", &bytes)?,
        None => client.create_file(
            "projects.json",
            "appDataFolder",
            "application/json",
            &bytes,
            json!({ "bn.kind": "projects" }),
        )?,
    };
    state
        .inner
        .db
        .with_write(|conn| record_file(conn, "projects.json", "projects", &file))
}

fn push_workspace(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
) -> Result<(), AppError> {
    let (id, name) = state.inner.db.with_read(|conn| {
        Ok((
            repo_meta::get(conn, "workspace.id")?.unwrap_or_default(),
            repo_meta::get(conn, "workspace.name")?.unwrap_or_default(),
        ))
    })?;
    let bytes = serde_json::to_vec_pretty(&json!({
        "id": id,
        "name": name,
        "format": 1,
        "epoch": now_iso()
    }))?;
    let synced = state
        .inner
        .db
        .with_read(|conn| repo_sync::get_by_path(conn, "workspace.json"))?;
    let file = match synced {
        Some(synced) => client.update_file(&synced.drive_file_id, "application/json", &bytes)?,
        None => client.create_file(
            "workspace.json",
            "appDataFolder",
            "application/json",
            &bytes,
            json!({ "bn.kind": "workspace" }),
        )?,
    };
    state
        .inner
        .db
        .with_write(|conn| record_file(conn, "workspace.json", "workspace", &file))
}

fn push_attachment(
    state: &AppState,
    client: &crate::drive::client::DriveClient,
    folders: &Folders,
    attachment_id: &str,
) -> Result<(), AppError> {
    let attachment = state
        .inner
        .db
        .with_read(|conn| repo_attachment::get(conn, attachment_id))?;
    let bytes = std::fs::read(&attachment.local_path)?;
    let entry_folder = client.ensure_folder(&attachment.entry_id, &folders.attachments)?;
    let path = format!(
        "attachments/{}/{}",
        attachment.entry_id, attachment.file_name
    );
    let synced = state
        .inner
        .db
        .with_read(|conn| repo_sync::get_by_path(conn, &path))?;
    let file = match synced {
        Some(synced) => client.update_file(&synced.drive_file_id, &attachment.mime_type, &bytes)?,
        None => client.create_file(
            &attachment.file_name,
            &entry_folder,
            &attachment.mime_type,
            &bytes,
            json!({ "bn.kind": "attachment", "bn.entryId": attachment.entry_id }),
        )?,
    };
    state.inner.db.with_write(|conn| {
        record_file(conn, &path, "attachment", &file)?;
        repo_attachment::set_drive_id(conn, &attachment.id, &file.id)
    })
}

use crate::db::{
    repo_attachment, repo_entry, repo_link, repo_project, repo_schema, repo_sync, repo_tag,
};
use crate::dto::{
    AttachmentDto, BundleAttachmentEntry, BundleCounts, BundleManifest, BundleNoteEntry,
    BundlePreview, BundleSchemaEntry, BundleScope, ExportResult, ImportReport, PreviewCounts,
    ProjectDto, SchemaDto,
};
use crate::error::AppError;
use crate::markdown::{self, NoteFile};
use crate::state::AppState;
use crate::util::{new_id, note_file_name, now_iso, sha256_hex, slugify, truncate_chars};
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

const MAX_ENTRIES: usize = 20_000;
const MAX_UNCOMPRESSED: u64 = 2 * 1024 * 1024 * 1024;

fn build_note(
    state: &AppState,
    row: &repo_entry::EntryRow,
) -> Result<(NoteFile, Vec<AttachmentDto>), AppError> {
    let schema = state.inner.schemas.require(&row.schema_id)?;
    let (values, tags, links, project_name) = state.inner.db.with_read(|conn| {
        Ok((
            repo_entry::load_values(conn, &row.id, &schema)?,
            repo_tag::resolve_names(conn, &row.id)?,
            repo_link::outgoing(conn, &row.id)?,
            match &row.project_id {
                Some(project_id) => repo_project::list(conn)?
                    .into_iter()
                    .find(|project| &project.id == project_id)
                    .map(|project| project.name),
                None => None,
            },
        ))
    })?;
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
    let attachments = state
        .inner
        .db
        .with_read(|conn| repo_attachment::list(conn, &row.id))?;
    Ok((note, attachments))
}

pub fn export_entry_markdown(
    state: &AppState,
    entry_id: &str,
    target_path: &str,
) -> Result<ExportResult, AppError> {
    let row = state
        .inner
        .db
        .with_read(|conn| repo_entry::get_row(conn, entry_id))?;
    let (mut note, attachments) = build_note(state, &row)?;
    let target = PathBuf::from(target_path);
    let stem = target
        .file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("note")
        .to_string();
    let parent = target
        .parent()
        .ok_or_else(|| AppError::io("Export target has no parent folder"))?;
    let assets_name = format!("{stem}-assets");
    let assets_dir = parent.join(&assets_name);
    let mut copied = 0i64;
    for attachment in &attachments {
        let source = PathBuf::from(&attachment.local_path);
        if !source.is_file() {
            continue;
        }
        let file_name = source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("attachment");
        crate::util::ensure_dir(&assets_dir)?;
        let destination = assets_dir.join(file_name);
        std::fs::copy(&source, &destination)?;
        let original = format!("attachments/{}/{}", attachment.entry_id, file_name);
        note.body = note
            .body
            .replace(&original, &format!("{assets_name}/{file_name}"));
        copied += 1;
    }
    let footer = format!("\n<!-- Exported from Better Notepad on {} -->\n", now_iso());
    note.body = format!("{}{}", note.body.trim_end(), footer);
    let markdown = markdown::serialize_note(&note);
    crate::util::atomic_write(&target, markdown.as_bytes())?;
    Ok(ExportResult {
        path: target.to_string_lossy().to_string(),
        notes: 1,
        attachments: copied,
    })
}

pub fn bundle_export(
    state: &AppState,
    scope: &str,
    project_id: Option<&str>,
    target_path: &str,
) -> Result<ExportResult, AppError> {
    if scope != "workspace" && scope != "project" {
        return Err(AppError::validation("Unknown export scope"));
    }
    let rows = state
        .inner
        .db
        .with_read(|conn| repo_entry::rows_for_export(conn, project_id))?;
    let project_name = match project_id {
        Some(id) => {
            state
                .inner
                .db
                .with_read(|conn| repo_project::get(conn, id))?
                .name
        }
        None => "Workspace".to_string(),
    };
    let projects: Vec<ProjectDto> = state.inner.db.with_read(repo_project::list)?;
    let selected_projects: Vec<ProjectDto> = match project_id {
        Some(id) => projects
            .into_iter()
            .filter(|project| project.id == id)
            .collect(),
        None => projects,
    };
    let mut schema_ids = HashSet::new();
    for row in &rows {
        schema_ids.insert(row.schema_id.clone());
    }
    let schemas: Vec<SchemaDto> = state
        .inner
        .schemas
        .list()
        .into_iter()
        .filter(|schema| schema_ids.contains(&schema.id))
        .collect();

    let mut used_slugs: HashSet<String> = HashSet::new();
    let mut file_names: HashMap<String, String> = HashMap::new();
    for row in &rows {
        let mut slug = slugify(&row.title);
        if slug.is_empty() {
            slug = "note".to_string();
        }
        let mut candidate = format!("{slug}.md");
        let mut counter = 2;
        while used_slugs.contains(&candidate) {
            candidate = format!("{slug}-{counter}.md");
            counter += 1;
        }
        used_slugs.insert(candidate.clone());
        file_names.insert(row.id.clone(), candidate);
    }

    let mut note_entries = Vec::new();
    let mut note_bytes: Vec<(String, Vec<u8>)> = Vec::new();
    let mut attachment_entries = Vec::new();
    let mut attachment_bytes: Vec<(String, Vec<u8>)> = Vec::new();
    for row in &rows {
        let (mut note, attachments) = build_note(state, row)?;
        let file_name = file_names
            .get(&row.id)
            .cloned()
            .unwrap_or_else(|| note_file_name(&row.title, &row.id));
        note.body = markdown::rewrite_note_links(&note.body, |id| {
            file_names.get(id).map(|name| format!("../notes/{name}"))
        });
        let markdown = markdown::serialize_note(&note);
        note_entries.push(BundleNoteEntry {
            id: row.id.clone(),
            file: format!("notes/{file_name}"),
            title: row.title.clone(),
            updated: row.updated_at.clone(),
        });
        note_bytes.push((format!("notes/{file_name}"), markdown.into_bytes()));
        for attachment in attachments {
            let source = PathBuf::from(&attachment.local_path);
            if !source.is_file() {
                continue;
            }
            let stored_name = source
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&attachment.id)
                .to_string();
            let bundle_path = format!("attachments/{}/{}", attachment.entry_id, stored_name);
            if let Ok(bytes) = std::fs::read(&source) {
                attachment_entries.push(BundleAttachmentEntry {
                    id: attachment.id.clone(),
                    entry_id: attachment.entry_id.clone(),
                    file: bundle_path.clone(),
                    mime_type: attachment.mime_type.clone(),
                    sha256: sha256_hex(&bytes),
                    size: bytes.len() as i64,
                });
                attachment_bytes.push((bundle_path, bytes));
            }
        }
    }

    let schema_entries: Vec<BundleSchemaEntry> = schemas
        .iter()
        .map(|schema| BundleSchemaEntry {
            key: schema.key.clone(),
            id: schema.id.clone(),
            file: format!("schemas/{}.json", schema.key),
        })
        .collect();

    let manifest = BundleManifest {
        format: "bn.bundle".to_string(),
        version: 1,
        exported_at: now_iso(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        scope: BundleScope {
            scope_type: scope.to_string(),
            project_id: project_id.map(|id| id.to_string()),
            name: project_name,
        },
        counts: BundleCounts {
            notes: note_entries.len() as i64,
            schemas: schema_entries.len() as i64,
            attachments: attachment_entries.len() as i64,
        },
        schemas: schema_entries,
        notes: note_entries,
        attachments: attachment_entries,
    };

    let target = PathBuf::from(target_path);
    if let Some(parent) = target.parent() {
        crate::util::ensure_dir(parent)?;
    }
    let temp = target.with_extension("bnpkg.tmp");
    let file = File::create(&temp)?;
    let mut zip = ZipWriter::new(file);
    write_zip(
        &mut zip,
        "manifest.json",
        &serde_json::to_vec_pretty(&manifest)?,
    )?;
    let readme = format!(
        "Better Notepad bundle\n\nVersion: 1\nApp version: {}\nExported: {}\nScope: {}\n\nImport this file with Better Notepad → Workspace → Import bundle.\n",
        env!("CARGO_PKG_VERSION"),
        manifest.exported_at,
        manifest.scope.name
    );
    write_zip(&mut zip, "README.txt", readme.as_bytes())?;
    for (name, bytes) in &note_bytes {
        write_zip(&mut zip, name, bytes)?;
    }
    for schema in &schemas {
        let path = format!("schemas/{}.json", schema.key);
        let bytes = serde_json::to_vec_pretty(schema)?;
        write_zip(&mut zip, &path, &bytes)?;
    }
    let projects_json = serde_json::to_vec_pretty(&selected_projects)?;
    write_zip(&mut zip, "projects.json", &projects_json)?;
    for (name, bytes) in &attachment_bytes {
        write_zip(&mut zip, name, bytes)?;
    }
    zip.finish()?;
    std::fs::rename(&temp, &target)?;
    Ok(ExportResult {
        path: target.to_string_lossy().to_string(),
        notes: manifest.counts.notes,
        attachments: manifest.counts.attachments,
    })
}

fn write_zip(zip: &mut ZipWriter<File>, name: &str, bytes: &[u8]) -> Result<(), AppError> {
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o644);
    zip.start_file(name, options)?;
    zip.write_all(bytes)?;
    Ok(())
}

fn safe_relative(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('/')
        && !name.starts_with('\\')
        && !name.contains("..")
        && !name.contains(':')
        && !name.contains('\\')
}

fn read_manifest(path: &Path) -> Result<(BundleManifest, ZipArchive<File>), AppError> {
    let file = File::open(path)?;
    let mut archive = ZipArchive::new(file)?;
    if archive.len() > MAX_ENTRIES {
        return Err(AppError::validation("Bundle contains too many files"));
    }
    let mut total_size = 0u64;
    for index in 0..archive.len() {
        let entry = archive.by_index(index)?;
        if entry.name() == "manifest.json" {
            continue;
        }
        if !safe_relative(entry.name()) {
            return Err(AppError::validation("Bundle contains an unsafe path"));
        }
        if let Some(mode) = entry.unix_mode() {
            if mode & 0o170000 == 0o120000 {
                return Err(AppError::validation("Bundle contains a symbolic link"));
            }
        }
        total_size += entry.size();
    }
    if total_size > MAX_UNCOMPRESSED {
        return Err(AppError::validation(
            "Bundle exceeds the uncompressed size limit",
        ));
    }
    let mut manifest_bytes = Vec::new();
    archive
        .by_name("manifest.json")?
        .read_to_end(&mut manifest_bytes)?;
    let manifest: BundleManifest = serde_json::from_slice(&manifest_bytes)?;
    if manifest.format != "bn.bundle" {
        return Err(AppError::unsupported("Not a Better Notepad bundle"));
    }
    if manifest.version != 1 {
        return Err(AppError::unsupported("Unsupported bundle version"));
    }
    Ok((manifest, archive))
}

pub fn bundle_inspect(state: &AppState, source_path: &str) -> Result<BundlePreview, AppError> {
    let (manifest, mut archive) = read_manifest(Path::new(source_path))?;
    let mut notes = PreviewCounts {
        total: manifest.notes.len() as i64,
        ..Default::default()
    };
    let mut warnings = Vec::new();
    let mut schema_conflicts = Vec::new();
    let mut schema_status = Vec::new();
    for schema_entry in &manifest.schemas {
        let mut bytes = Vec::new();
        match archive.by_name(&schema_entry.file) {
            Ok(mut entry) => {
                entry.read_to_end(&mut bytes)?;
            }
            Err(_) => {
                warnings.push(format!("Missing schema file {}", schema_entry.file));
                continue;
            }
        }
        match serde_json::from_slice::<SchemaDto>(&bytes) {
            Ok(schema) => match state.inner.schemas.get_by_key(&schema_entry.key) {
                Some(existing) => {
                    let differs = existing.fields.iter().any(|field| {
                        schema
                            .fields
                            .iter()
                            .find(|incoming| incoming.key == field.key)
                            .map(|incoming| incoming.field_type != field.field_type)
                            .unwrap_or(false)
                    });
                    if differs {
                        schema_conflicts.push(schema_entry.key.clone());
                    }
                    schema_status.push(format!("matching {}", schema_entry.key));
                }
                None => schema_status.push(format!("new {}", schema_entry.key)),
            },
            Err(_) => warnings.push(format!("Invalid schema file {}", schema_entry.file)),
        }
    }
    for note_entry in &manifest.notes {
        let local = state
            .inner
            .db
            .with_read(|conn| repo_entry::find_row(conn, &note_entry.id))
            .ok()
            .flatten();
        let Some(local) = local else {
            notes.new += 1;
            continue;
        };
        if note_entry.updated > local.updated_at {
            notes.updated += 1;
        } else if note_entry.updated < local.updated_at {
            notes.unchanged += 1;
        } else {
            let mut bytes = Vec::new();
            if let Ok(mut entry) = archive.by_name(&note_entry.file) {
                if entry.read_to_end(&mut bytes).is_ok() {
                    if let Ok(note) = markdown::parse_note(&String::from_utf8_lossy(&bytes)) {
                        if note.body.trim() != local.body_md.trim() || note.title != local.title {
                            notes.conflicts += 1;
                        } else {
                            notes.unchanged += 1;
                        }
                    } else {
                        notes.unchanged += 1;
                    }
                }
            } else {
                notes.unchanged += 1;
            }
        }
    }
    let project_names: Vec<String> = if manifest.scope.project_id.is_some() {
        vec![manifest.scope.name.clone()]
    } else {
        Vec::new()
    };
    Ok(BundlePreview {
        format_version: manifest.version,
        exported_at: manifest.exported_at,
        app_version: manifest.app_version,
        scope: manifest.scope,
        notes,
        schemas: schema_status,
        schema_conflicts,
        projects: project_names,
        attachments_total: manifest.attachments.len() as i64,
        attachments_bytes: manifest.attachments.iter().map(|item| item.size).sum(),
        warnings,
    })
}

pub fn bundle_import(
    state: &AppState,
    source_path: &str,
    mode: &str,
) -> Result<ImportReport, AppError> {
    if !matches!(mode, "merge" | "keepLocal" | "keepIncoming") {
        return Err(AppError::validation("Unknown import mode"));
    }
    let (manifest, mut archive) = read_manifest(Path::new(source_path))?;
    let mut report = ImportReport {
        created: 0,
        updated: 0,
        skipped: 0,
        conflicts: 0,
        attachments_copied: 0,
        warnings: Vec::new(),
    };

    for schema_entry in &manifest.schemas {
        let mut bytes = Vec::new();
        match archive.by_name(&schema_entry.file) {
            Ok(mut entry) => {
                if entry.read_to_end(&mut bytes).is_err() {
                    report
                        .warnings
                        .push(format!("Cannot read {}", schema_entry.file));
                    continue;
                }
            }
            Err(_) => {
                report
                    .warnings
                    .push(format!("Missing {}", schema_entry.file));
                continue;
            }
        }
        match serde_json::from_slice::<SchemaDto>(&bytes) {
            Ok(schema) => {
                if state.inner.schemas.get_by_key(&schema.key).is_none() {
                    state
                        .inner
                        .db
                        .with_write(|conn| repo_schema::insert(conn, &schema))?;
                }
            }
            Err(_) => report
                .warnings
                .push(format!("Invalid schema {}", schema_entry.file)),
        }
    }
    state.refresh_schemas()?;

    let mut projects_bytes = Vec::new();
    if archive.by_name("projects.json").is_ok() {
        if let Ok(mut entry) = archive.by_name("projects.json") {
            let _ = entry.read_to_end(&mut projects_bytes);
        }
        if let Ok(projects) = serde_json::from_slice::<Vec<ProjectDto>>(&projects_bytes) {
            for project in projects {
                let exists = state
                    .inner
                    .db
                    .with_read(|conn| repo_project::find_by_name(conn, &project.name))?;
                if exists.is_none() {
                    state.inner.db.with_write(|conn| {
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
                        )
                    })?;
                }
            }
        }
    }

    let mut imported: Vec<NoteFile> = Vec::new();
    for note_entry in &manifest.notes {
        let mut bytes = Vec::new();
        match archive.by_name(&note_entry.file) {
            Ok(mut entry) => {
                if entry.read_to_end(&mut bytes).is_err() {
                    report.skipped += 1;
                    continue;
                }
            }
            Err(_) => {
                report.skipped += 1;
                report.warnings.push(format!("Missing {}", note_entry.file));
                continue;
            }
        }
        let text = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                report.skipped += 1;
                continue;
            }
        };
        let note = match markdown::parse_note(&text) {
            Ok(note) => note,
            Err(error) => {
                report.skipped += 1;
                report
                    .warnings
                    .push(format!("Skipped {}: {}", note_entry.file, error.message));
                continue;
            }
        };
        let existing = state
            .inner
            .db
            .with_read(|conn| repo_entry::find_row(conn, &note.id))?;
        let schema = match state.inner.schemas.get_by_key(&note.schema) {
            Some(schema) => (*schema).clone(),
            None => {
                report.skipped += 1;
                report
                    .warnings
                    .push(format!("Unknown schema '{}'", note.schema));
                continue;
            }
        };
        match existing {
            None => {
                insert_note(state, &note, &schema)?;
                report.created += 1;
                imported.push(note);
            }
            Some(local) => match mode {
                "keepLocal" => {
                    report.skipped += 1;
                    imported.push(note);
                }
                "keepIncoming" => {
                    replace_note(state, &local.id, &note, &schema)?;
                    report.updated += 1;
                    imported.push(note);
                }
                _ => {
                    if note.updated > local.updated_at {
                        replace_note(state, &local.id, &note, &schema)?;
                        report.updated += 1;
                        imported.push(note);
                    } else if note.updated < local.updated_at
                        || (note.body.trim() == local.body_md.trim() && note.title == local.title)
                    {
                        report.skipped += 1;
                        imported.push(note);
                    } else {
                        let mut copy = note.clone();
                        copy.id = new_id();
                        copy.title = format!(
                            "{} (conflict {})",
                            truncate_chars(&note.title, 240),
                            now_iso()
                        );
                        insert_note(state, &copy, &schema)?;
                        report.conflicts += 1;
                        imported.push(copy);
                    }
                }
            },
        }
    }

    for note in &imported {
        let _ = state.inner.db.with_write(|conn| {
            let tx = conn.transaction()?;
            for target in &note.links {
                if repo_entry::find_row(&tx, target)?.is_some() {
                    let _ = repo_link::create(&tx, &note.id, target);
                }
            }
            repo_link::replace_markdown_links(
                &tx,
                &note.id,
                &markdown::extract_note_links(&note.body),
            )?;
            tx.commit()?;
            Ok(())
        });
    }

    for attachment in &manifest.attachments {
        let mut bytes = Vec::new();
        match archive.by_name(&attachment.file) {
            Ok(mut entry) => {
                if entry.read_to_end(&mut bytes).is_err() {
                    continue;
                }
            }
            Err(_) => continue,
        }
        if sha256_hex(&bytes) != attachment.sha256 {
            report
                .warnings
                .push(format!("Checksum mismatch: {}", attachment.file));
            continue;
        }
        let file_name = Path::new(&attachment.file)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(&attachment.id)
            .to_string();
        if mime_is_blocked(&file_name) {
            report
                .warnings
                .push(format!("Blocked attachment type: {file_name}"));
            continue;
        }
        let target = state
            .mirror_dir()
            .join("attachments")
            .join(&attachment.entry_id)
            .join(&file_name);
        crate::util::atomic_write(&target, &bytes)?;
        let existing = state
            .inner
            .db
            .with_read(|conn| repo_attachment::get(conn, &attachment.id));
        if existing.is_err() {
            let kind = if attachment.mime_type.starts_with("image/") {
                "image"
            } else {
                "file"
            };
            let dto = AttachmentDto {
                id: attachment.id.clone(),
                entry_id: attachment.entry_id.clone(),
                kind: kind.to_string(),
                file_name,
                mime_type: attachment.mime_type.clone(),
                byte_size: bytes.len() as i64,
                sha256: attachment.sha256.clone(),
                local_path: target.to_string_lossy().to_string(),
                preview_path: None,
                created_at: now_iso(),
            };
            if state
                .inner
                .db
                .with_read(|conn| repo_entry::find_row(conn, &dto.entry_id))?
                .is_some()
            {
                state
                    .inner
                    .db
                    .with_write(|conn| repo_attachment::insert(conn, &dto))?;
                report.attachments_copied += 1;
            }
        }
    }

    state.emit_workspace_changed("import");
    Ok(report)
}

fn mime_is_blocked(file_name: &str) -> bool {
    let lower = file_name.to_lowercase();
    [
        ".exe", ".dll", ".bat", ".cmd", ".ps1", ".msi", ".scr", ".js",
    ]
    .iter()
    .any(|extension| lower.ends_with(extension))
}

pub(crate) fn insert_note(
    state: &AppState,
    note: &NoteFile,
    schema: &SchemaDto,
) -> Result<(), AppError> {
    let values = valid_values(schema, &note.fields);
    let tag_ids = state
        .inner
        .db
        .with_write(|conn| repo_tag::resolve_ids(conn, &note.tags))?;
    let project_id = resolve_project(state, note)?;
    let title = truncate_chars(note.title.trim(), 300);
    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        let entry = repo_entry::NewEntry {
            schema_id: &schema.id,
            project_id: project_id.as_deref(),
            title: &title,
            body_md: &note.body,
            extra: &note.extra,
        };
        repo_entry::insert_entry(&tx, &note.id, &entry, &note.created, &note.updated, "dirty")?;
        repo_entry::set_values(&tx, &note.id, schema, &values)?;
        repo_tag::set_entry_tags(&tx, &note.id, &tag_ids)?;
        repo_link::replace_markdown_links(
            &tx,
            &note.id,
            &markdown::extract_note_links(&note.body),
        )?;
        repo_sync::enqueue(
            &tx,
            "note.write",
            &note.id,
            serde_json::json!({ "entryId": note.id }),
        )?;
        tx.commit()?;
        Ok(())
    })
}

fn replace_note(
    state: &AppState,
    id: &str,
    note: &NoteFile,
    schema: &SchemaDto,
) -> Result<(), AppError> {
    let values = valid_values(schema, &note.fields);
    let tag_ids = state
        .inner
        .db
        .with_write(|conn| repo_tag::resolve_ids(conn, &note.tags))?;
    let project_id = resolve_project(state, note)?;
    let title = truncate_chars(note.title.trim(), 300);
    let extra = note.extra.clone();
    state.inner.db.with_write(|conn| {
        let tx = conn.transaction()?;
        repo_entry::update_entry(
            &tx,
            id,
            project_id.as_deref(),
            &title,
            &note.body,
            &note.updated,
            &extra,
            None,
        )?;
        repo_entry::set_values(&tx, id, schema, &values)?;
        repo_tag::set_entry_tags(&tx, id, &tag_ids)?;
        repo_link::replace_markdown_links(&tx, id, &markdown::extract_note_links(&note.body))?;
        repo_sync::enqueue(&tx, "note.write", id, serde_json::json!({ "entryId": id }))?;
        tx.commit()?;
        Ok(())
    })
}

fn resolve_project(state: &AppState, note: &NoteFile) -> Result<Option<String>, AppError> {
    let Some(project_id) = &note.project else {
        return Ok(None);
    };
    let projects = state.inner.db.with_read(repo_project::list)?;
    if let Some(project) = projects.iter().find(|project| &project.id == project_id) {
        return Ok(Some(project.id.clone()));
    }
    if let Some(name) = &note.project_name {
        if let Ok(Some(project)) = state
            .inner
            .db
            .with_read(|conn| repo_project::find_by_name(conn, name))
        {
            return Ok(Some(project.id));
        }
    }
    let now = now_iso();
    state.inner.db.with_write(|conn| {
        repo_project::insert_remote(
            conn,
            project_id,
            note.project_name.as_deref().unwrap_or("Imported project"),
            "",
            "#64748b",
            "folder",
            0,
            &now,
            &now,
        )
    })?;
    Ok(Some(project_id.clone()))
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
        if crate::schema::validate::validate_field(field, Some(&normalized)).is_ok() {
            values.insert(field.key.clone(), normalized);
        }
    }
    values
}

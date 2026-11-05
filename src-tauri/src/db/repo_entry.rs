use crate::dto::{
    EntryDetail, EntryListItem, EntryPage, EntryQuery, LinkBundle, SearchHit, SearchResult,
};
use crate::error::AppError;
use crate::schema::{self, SchemaCache};
use crate::util::{new_id, now_iso};
use base64::Engine;
use rusqlite::types::Value as SqlValue;
use rusqlite::{params, params_from_iter, Connection, Row};
use serde_json::{Map, Value};
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct EntryRow {
    pub id: String,
    pub schema_id: String,
    pub project_id: Option<String>,
    pub title: String,
    pub body_md: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
    pub sync_state: String,
    pub extra_json: String,
    pub drive_file_id: Option<String>,
}

const ROW_COLUMNS: &str = "e.id, e.schema_id, e.project_id, e.title, e.body_md, e.created_at,
     e.updated_at, e.deleted_at, e.sync_state, e.extra_json, e.drive_file_id";

fn row_to_entry(row: &Row<'_>) -> rusqlite::Result<EntryRow> {
    Ok(EntryRow {
        id: row.get(0)?,
        schema_id: row.get(1)?,
        project_id: row.get(2)?,
        title: row.get(3)?,
        body_md: row.get(4)?,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
        deleted_at: row.get(7)?,
        sync_state: row.get(8)?,
        extra_json: row.get(9)?,
        drive_file_id: row.get(10)?,
    })
}

pub fn get_row(conn: &Connection, id: &str) -> Result<EntryRow, AppError> {
    let sql = format!("SELECT {ROW_COLUMNS} FROM entry e WHERE e.id = ?1");
    let row = conn.query_row(&sql, params![id], row_to_entry)?;
    Ok(row)
}

pub fn find_row(conn: &Connection, id: &str) -> Result<Option<EntryRow>, AppError> {
    match get_row(conn, id) {
        Ok(row) => Ok(Some(row)),
        Err(error) if error.code == crate::error::ErrorCode::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

pub fn get_detail(
    conn: &Connection,
    cache: &SchemaCache,
    id: &str,
) -> Result<EntryDetail, AppError> {
    let row = get_row(conn, id)?;
    hydrate(conn, cache, &row)
}

pub fn hydrate(
    conn: &Connection,
    cache: &SchemaCache,
    row: &EntryRow,
) -> Result<EntryDetail, AppError> {
    let schema = cache.require(&row.schema_id)?;
    let values = load_values(conn, &row.id, &schema)?;
    let computed = schema::compute_formulas(&schema, &values);
    let tags = crate::db::repo_tag::tags_for_entry(conn, &row.id)?;
    let links = LinkBundle {
        links: crate::db::repo_link::outgoing(conn, &row.id)?,
        backlinks: crate::db::repo_link::incoming(conn, &row.id)?,
        markdown_links: crate::db::repo_link::markdown_outgoing(conn, &row.id)?,
        markdown_backlinks: crate::db::repo_link::markdown_incoming(conn, &row.id)?,
    };
    let attachments = crate::db::repo_attachment::list(conn, &row.id)?;
    Ok(EntryDetail {
        id: row.id.clone(),
        schema_id: row.schema_id.clone(),
        project_id: row.project_id.clone(),
        title: row.title.clone(),
        body_md: row.body_md.clone(),
        values,
        computed,
        tags,
        links,
        attachments,
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
        deleted_at: row.deleted_at.clone(),
        sync_state: row.sync_state.clone(),
    })
}

pub fn load_values(
    conn: &Connection,
    entry_id: &str,
    schema: &crate::dto::SchemaDto,
) -> Result<Map<String, Value>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT f.key, ev.text_value, ev.num_value, ev.date_value, ev.bool_value
         FROM entry_value ev JOIN field_def f ON f.id = ev.field_id
         WHERE ev.entry_id = ?1",
    )?;
    let rows = stmt
        .query_map(params![entry_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<i64>>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut values = Map::new();
    for (key, text, number, date, boolean) in rows {
        if let Some(field) = schema::field_by_key(schema, &key) {
            values.insert(
                key,
                schema::validate::value_from_storage(field, text, number, date, boolean),
            );
        }
    }
    Ok(values)
}

pub fn set_values(
    conn: &Connection,
    entry_id: &str,
    schema: &crate::dto::SchemaDto,
    values: &Map<String, Value>,
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM entry_value WHERE entry_id = ?1",
        params![entry_id],
    )?;
    for field in &schema.fields {
        if field.field_type == "formula" || field.field_type == "tags" {
            continue;
        }
        let Some(value) = values.get(&field.key) else {
            continue;
        };
        let storage = schema::validate::storage_value(field, value);
        if storage.text.is_none()
            && storage.number.is_none()
            && storage.date.is_none()
            && storage.boolean.is_none()
        {
            continue;
        }
        conn.execute(
            "INSERT INTO entry_value (entry_id, field_id, text_value, num_value, date_value, bool_value)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                entry_id,
                field.id,
                storage.text,
                storage.number,
                storage.date,
                storage.boolean
            ],
        )?;
    }
    rebuild_field_text(conn, entry_id)?;
    Ok(())
}

pub fn rebuild_field_text(conn: &Connection, entry_id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE entry_fts SET field_text = coalesce((
            SELECT group_concat(text_value, ' ') FROM entry_value
            WHERE entry_id = ?1 AND text_value IS NOT NULL), '')
         WHERE entry_id = ?1",
        params![entry_id],
    )?;
    Ok(())
}

fn fts_expression(text: &str) -> Option<String> {
    let tokens: Vec<String> = text
        .split_whitespace()
        .map(|token| {
            token
                .chars()
                .filter(|ch| ch.is_alphanumeric() || *ch == '_')
                .collect::<String>()
        })
        .filter(|token| !token.is_empty())
        .collect();
    if tokens.is_empty() {
        return None;
    }
    let last_index = tokens.len() - 1;
    let parts: Vec<String> = tokens
        .iter()
        .enumerate()
        .map(|(index, token)| {
            if index == last_index {
                format!("\"{token}\"*")
            } else {
                format!("\"{token}\"")
            }
        })
        .collect();
    Some(parts.join(" AND "))
}

struct Filter {
    clauses: Vec<String>,
    args: Vec<SqlValue>,
}

fn build_filter(query: &EntryQuery) -> Filter {
    let mut clauses = Vec::new();
    let mut args = Vec::new();
    if query.include_deleted {
        clauses.push("e.deleted_at IS NOT NULL".to_string());
    } else {
        clauses.push("e.deleted_at IS NULL".to_string());
    }
    for schema_id in &query.schema_ids {
        clauses.push("e.schema_id = ?".to_string());
        args.push(SqlValue::Text(schema_id.clone()));
    }
    if let Some(project_id) = &query.project_id {
        if project_id == "inbox" {
            clauses.push("e.project_id IS NULL".to_string());
        } else {
            clauses.push("e.project_id = ?".to_string());
            args.push(SqlValue::Text(project_id.clone()));
        }
    }
    for tag_id in &query.tag_ids {
        clauses.push(
            "EXISTS (SELECT 1 FROM entry_tag et WHERE et.entry_id = e.id AND et.tag_id = ?)"
                .to_string(),
        );
        args.push(SqlValue::Text(tag_id.clone()));
    }
    if let Some(status) = &query.status_value {
        clauses.push(
            "EXISTS (SELECT 1 FROM entry_value ev JOIN field_def f ON f.id = ev.field_id
                     WHERE ev.entry_id = e.id AND f.key = 'status' AND ev.text_value = ?)"
                .to_string(),
        );
        args.push(SqlValue::Text(status.clone()));
    }
    if let Some(search) = &query.search {
        if let Some(expression) = fts_expression(search) {
            clauses.push(
                "e.id IN (SELECT entry_id FROM entry_fts WHERE entry_fts MATCH ?)".to_string(),
            );
            args.push(SqlValue::Text(expression));
        }
    }
    Filter { clauses, args }
}

fn order_clause(key: &str, direction: &str) -> String {
    let dir = if direction.eq_ignore_ascii_case("asc") {
        "ASC"
    } else {
        "DESC"
    };
    match key {
        "created" => format!("e.created_at {dir}, e.id {dir}"),
        "title" => format!("e.title COLLATE NOCASE {dir}, e.id {dir}"),
        "updated" => format!("e.updated_at {dir}, e.id {dir}"),
        other
            if other
                .chars()
                .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_') =>
        {
            format!(
                "(SELECT COALESCE(ev.num_value, ev.text_value, ev.date_value)
                  FROM entry_value ev JOIN field_def f ON f.id = ev.field_id
                  WHERE ev.entry_id = e.id AND f.key = '{other}' LIMIT 1) {dir},
                 e.updated_at DESC"
            )
        }
        _ => "e.updated_at DESC, e.id DESC".to_string(),
    }
}

fn decode_cursor(cursor: &Option<String>) -> i64 {
    cursor
        .as_ref()
        .and_then(|value| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(value)
                .ok()
        })
        .and_then(|bytes| String::from_utf8(bytes).ok())
        .and_then(|text| text.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0)
}

fn encode_cursor(offset: i64) -> String {
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(offset.to_string())
}

pub fn list(
    conn: &Connection,
    cache: &SchemaCache,
    query: &EntryQuery,
) -> Result<EntryPage, AppError> {
    let filter = build_filter(query);
    let where_sql = filter.clauses.join(" AND ");
    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) FROM entry e WHERE {where_sql}"),
        params_from_iter(filter.args.iter()),
        |row| row.get(0),
    )?;

    let limit = query.limit.unwrap_or(100).clamp(1, 500);
    let offset = decode_cursor(&query.cursor);
    let order = order_clause(&query.sort.key, &query.sort.direction);
    let sql = format!(
        "SELECT {ROW_COLUMNS} FROM entry e WHERE {where_sql}
         ORDER BY {order} LIMIT ? OFFSET ?"
    );
    let mut args = filter.args.clone();
    args.push(SqlValue::Integer(limit));
    args.push(SqlValue::Integer(offset));
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params_from_iter(args.iter()), row_to_entry)?
        .collect::<Result<Vec<_>, _>>()?;

    let mut items = Vec::with_capacity(rows.len());
    for row in &rows {
        items.push(to_list_item(conn, cache, row)?);
    }
    let next_offset = offset + items.len() as i64;
    let next_cursor = if next_offset < total {
        Some(encode_cursor(next_offset))
    } else {
        None
    };
    Ok(EntryPage {
        items,
        total,
        next_cursor,
    })
}

fn to_list_item(
    conn: &Connection,
    cache: &SchemaCache,
    row: &EntryRow,
) -> Result<EntryListItem, AppError> {
    let schema = cache.require(&row.schema_id)?;
    let values = load_values(conn, &row.id, &schema)?;
    let computed = schema::compute_formulas(&schema, &values);
    let subtitle = role_value(&schema, &values, schema.subtitle_field_id.as_deref());
    let (badge, badge_color) = badge_value(&schema, &values, &computed);
    let tags = crate::db::repo_tag::tags_for_entry(conn, &row.id)?;
    Ok(EntryListItem {
        id: row.id.clone(),
        title: row.title.clone(),
        schema_id: row.schema_id.clone(),
        schema_key: schema.key.clone(),
        project_id: row.project_id.clone(),
        tags,
        subtitle,
        badge,
        badge_color,
        computed,
        created_at: row.created_at.clone(),
        updated_at: row.updated_at.clone(),
        deleted_at: row.deleted_at.clone(),
        sync_state: row.sync_state.clone(),
    })
}

fn role_value(
    schema: &crate::dto::SchemaDto,
    values: &Map<String, Value>,
    field_id: Option<&str>,
) -> Option<String> {
    let field_id = field_id?;
    let field = schema::field_by_id(schema, field_id)?;
    let value = values.get(&field.key)?;
    display_value(field, value)
}

fn display_value(field: &crate::dto::FieldDefDto, value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(text) if text.is_empty() => None,
        Value::String(text) => {
            if field.field_type == "status" {
                let options = field
                    .config
                    .get("options")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for option in options {
                    if option.get("key").and_then(Value::as_str) == Some(text.as_str()) {
                        return option
                            .get("label")
                            .and_then(Value::as_str)
                            .map(|label| label.to_string())
                            .or_else(|| Some(text.clone()));
                    }
                }
                Some(text.clone())
            } else {
                Some(text.clone())
            }
        }
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(if *flag {
            "✓".to_string()
        } else {
            "—".to_string()
        }),
        _ => None,
    }
}

fn badge_value(
    schema: &crate::dto::SchemaDto,
    values: &Map<String, Value>,
    computed: &Map<String, Value>,
) -> (Option<String>, Option<String>) {
    let Some(field_id) = schema.badge_field_id.as_deref() else {
        return (None, None);
    };
    let Some(field) = schema::field_by_id(schema, field_id) else {
        return (None, None);
    };
    if field.field_type == "formula" {
        let value = computed.get(&field.key);
        return match value {
            Some(Value::Number(number)) => (Some(number.to_string()), Some("#6366f1".to_string())),
            _ => (None, None),
        };
    }
    match values.get(&field.key) {
        Some(Value::String(text)) if field.field_type == "status" => {
            let options = field
                .config
                .get("options")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            for option in options {
                if option.get("key").and_then(Value::as_str) == Some(text.as_str()) {
                    let label = option
                        .get("label")
                        .and_then(Value::as_str)
                        .map(|label| label.to_string());
                    let color = option
                        .get("color")
                        .and_then(Value::as_str)
                        .map(|color| color.to_string());
                    return (label, color);
                }
            }
            (Some(text.clone()), None)
        }
        Some(value) => (display_value(field, value), None),
        None => (None, None),
    }
}

pub struct NewEntry<'a> {
    pub schema_id: &'a str,
    pub project_id: Option<&'a str>,
    pub title: &'a str,
    pub body_md: &'a str,
    pub extra: &'a Map<String, Value>,
}

pub fn insert_entry(
    conn: &Connection,
    id: &str,
    entry: &NewEntry<'_>,
    created_at: &str,
    updated_at: &str,
    sync_state: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO entry (id, schema_id, project_id, title, body_md, created_at, updated_at,
                            local_rev, sync_state, extra_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8, ?9)",
        params![
            id,
            entry.schema_id,
            entry.project_id,
            entry.title,
            entry.body_md,
            created_at,
            updated_at,
            sync_state,
            Value::Object(entry.extra.clone()).to_string()
        ],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn update_entry(
    conn: &Connection,
    id: &str,
    project_id: Option<&str>,
    title: &str,
    body_md: &str,
    updated_at: &str,
    extra: &Map<String, Value>,
    expected_updated_at: Option<&str>,
) -> Result<(), AppError> {
    let changed = match expected_updated_at {
        Some(expected) => conn.execute(
            "UPDATE entry SET project_id = ?2, title = ?3, body_md = ?4, updated_at = ?5,
                    extra_json = ?6, local_rev = local_rev + 1, sync_state = 'dirty'
             WHERE id = ?1 AND deleted_at IS NULL AND updated_at = ?7",
            params![
                id,
                project_id,
                title,
                body_md,
                updated_at,
                Value::Object(extra.clone()).to_string(),
                expected
            ],
        )?,
        None => conn.execute(
            "UPDATE entry SET project_id = ?2, title = ?3, body_md = ?4, updated_at = ?5,
                    extra_json = ?6, local_rev = local_rev + 1, sync_state = 'dirty'
             WHERE id = ?1 AND deleted_at IS NULL",
            params![
                id,
                project_id,
                title,
                body_md,
                updated_at,
                Value::Object(extra.clone()).to_string()
            ],
        )?,
    };
    if changed == 0 {
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM entry WHERE id = ?1 AND deleted_at IS NULL",
            params![id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(AppError::not_found("Note not found"));
        }
        return Err(AppError::conflict(
            "The note was changed elsewhere. Reload before saving.",
        ));
    }
    Ok(())
}

pub fn soft_delete(conn: &Connection, id: &str, when: &str) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE entry SET deleted_at = ?2, updated_at = ?2, sync_state = 'dirty',
                local_rev = local_rev + 1
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id, when],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Note not found"));
    }
    Ok(())
}

pub fn restore(conn: &Connection, id: &str, when: &str) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE entry SET deleted_at = NULL, updated_at = ?2, sync_state = 'dirty',
                local_rev = local_rev + 1
         WHERE id = ?1",
        params![id, when],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Note not found"));
    }
    Ok(())
}

pub fn set_project(
    conn: &Connection,
    id: &str,
    project_id: Option<&str>,
    when: &str,
) -> Result<(), AppError> {
    let changed = conn.execute(
        "UPDATE entry SET project_id = ?2, updated_at = ?3, sync_state = 'dirty',
                local_rev = local_rev + 1
         WHERE id = ?1 AND deleted_at IS NULL",
        params![id, project_id, when],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Note not found"));
    }
    Ok(())
}

pub fn set_sync_state(conn: &Connection, id: &str, state: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE entry SET sync_state = ?2 WHERE id = ?1",
        params![id, state],
    )?;
    Ok(())
}

pub fn touch(conn: &Connection, id: &str, when: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE entry SET updated_at = ?2, sync_state = 'dirty', local_rev = local_rev + 1
         WHERE id = ?1",
        params![id, when],
    )?;
    Ok(())
}

pub fn set_drive_meta(
    conn: &Connection,
    id: &str,
    drive_file_id: &str,
    modified_at: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE entry SET drive_file_id = ?2, drive_modified_at = ?3 WHERE id = ?1",
        params![id, drive_file_id, modified_at],
    )?;
    Ok(())
}

pub fn find_by_drive_id(conn: &Connection, drive_id: &str) -> Result<Option<EntryRow>, AppError> {
    let sql = format!("SELECT {ROW_COLUMNS} FROM entry e WHERE e.drive_file_id = ?1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query(params![drive_id])?;
    match rows.next()? {
        Some(row) => Ok(Some(row_to_entry(row)?)),
        None => Ok(None),
    }
}

pub fn mark_clean_if_rev(conn: &Connection, id: &str, rev: i64) -> Result<bool, AppError> {
    let changed = conn.execute(
        "UPDATE entry SET sync_state = CASE WHEN sync_state = 'conflict' THEN 'conflict' ELSE 'clean' END
         WHERE id = ?1 AND local_rev = ?2",
        params![id, rev],
    )?;
    Ok(changed > 0)
}

pub fn local_rev(conn: &Connection, id: &str) -> Result<i64, AppError> {
    Ok(conn.query_row(
        "SELECT local_rev FROM entry WHERE id = ?1",
        params![id],
        |row| row.get(0),
    )?)
}

pub struct RemoteEntry<'a> {
    pub id: &'a str,
    pub schema_id: &'a str,
    pub project_id: Option<&'a str>,
    pub title: &'a str,
    pub body_md: &'a str,
    pub created_at: &'a str,
    pub updated_at: &'a str,
    pub drive_file_id: &'a str,
    pub extra: &'a Map<String, Value>,
}

pub fn insert_remote(conn: &Connection, entry: &RemoteEntry<'_>) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO entry (id, schema_id, project_id, title, body_md, created_at, updated_at,
                            deleted_at, drive_file_id, drive_modified_at, local_rev, sync_state,
                            extra_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL, ?8, ?7, 0, 'clean', ?9)",
        params![
            entry.id,
            entry.schema_id,
            entry.project_id,
            entry.title,
            entry.body_md,
            entry.created_at,
            entry.updated_at,
            entry.drive_file_id,
            Value::Object(entry.extra.clone()).to_string()
        ],
    )?;
    Ok(())
}

pub fn update_remote(conn: &Connection, entry: &RemoteEntry<'_>) -> Result<(), AppError> {
    conn.execute(
        "UPDATE entry SET schema_id = ?2, project_id = ?3, title = ?4, body_md = ?5,
                created_at = ?6, updated_at = ?7, deleted_at = NULL, drive_file_id = ?8,
                drive_modified_at = ?7, sync_state = 'clean', extra_json = ?9
         WHERE id = ?1",
        params![
            entry.id,
            entry.schema_id,
            entry.project_id,
            entry.title,
            entry.body_md,
            entry.created_at,
            entry.updated_at,
            entry.drive_file_id,
            Value::Object(entry.extra.clone()).to_string()
        ],
    )?;
    Ok(())
}

pub fn remove_drive_meta(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE entry SET drive_file_id = NULL, drive_modified_at = NULL WHERE id = ?1",
        params![id],
    )?;
    Ok(())
}

pub fn set_conflict_of(conn: &Connection, id: &str, original_id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE entry SET conflict_of = ?2 WHERE id = ?1",
        params![id, original_id],
    )?;
    Ok(())
}

pub fn duplicate(conn: &Connection, id: &str) -> Result<String, AppError> {
    let source = get_row(conn, id)?;
    let copy_id = new_id();
    let now = now_iso();
    let title = format!("{} (copy)", source.title);
    conn.execute(
        "INSERT INTO entry (id, schema_id, project_id, title, body_md, created_at, updated_at,
                            local_rev, sync_state, extra_json)
         SELECT ?1, schema_id, project_id, ?2, body_md, ?3, ?3, 0, 'local', extra_json
         FROM entry WHERE id = ?4",
        params![copy_id, title, now, id],
    )?;
    conn.execute(
        "INSERT INTO entry_value (entry_id, field_id, text_value, num_value, date_value, bool_value)
         SELECT ?1, field_id, text_value, num_value, date_value, bool_value
         FROM entry_value WHERE entry_id = ?2",
        params![copy_id, id],
    )?;
    conn.execute(
        "INSERT INTO entry_tag (entry_id, tag_id) SELECT ?1, tag_id FROM entry_tag WHERE entry_id = ?2",
        params![copy_id, id],
    )?;
    let mut stmt = conn.prepare("SELECT to_id FROM link WHERE from_id = ?1")?;
    let mut rows = stmt.query(params![id])?;
    let mut targets: Vec<String> = Vec::new();
    while let Some(row) = rows.next()? {
        targets.push(row.get::<_, String>(0)?);
    }
    drop(rows);
    drop(stmt);
    for target in targets {
        conn.execute(
            "INSERT OR IGNORE INTO link (id, from_id, to_id, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![new_id(), copy_id, target, now],
        )?;
    }
    rebuild_field_text(conn, &copy_id)?;
    Ok(copy_id)
}

pub fn find_by_title(conn: &Connection, title: &str) -> Result<Vec<(String, String)>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, title FROM entry
         WHERE deleted_at IS NULL AND title = ?1 COLLATE NOCASE
         ORDER BY updated_at DESC LIMIT 2",
    )?;
    let rows = stmt
        .query_map(params![title], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn rows_for_export(
    conn: &Connection,
    project_id: Option<&str>,
) -> Result<Vec<EntryRow>, AppError> {
    let mut sql = format!("SELECT {ROW_COLUMNS} FROM entry e WHERE e.deleted_at IS NULL");
    if project_id.is_some() {
        sql.push_str(" AND e.project_id = ?1");
    }
    sql.push_str(" ORDER BY e.created_at ASC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = match project_id {
        Some(project) => stmt
            .query_map(params![project], row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?,
        None => stmt
            .query_map([], row_to_entry)?
            .collect::<Result<Vec<_>, _>>()?,
    };
    Ok(rows)
}

pub fn conflicting_count(conn: &Connection) -> Result<i64, AppError> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM entry WHERE sync_state = 'conflict' AND deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?)
}

pub fn ids_for_schema(conn: &Connection, schema_id: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare("SELECT id FROM entry WHERE schema_id = ?1")?;
    let rows = stmt
        .query_map(params![schema_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn ids_for_project(conn: &Connection, project_id: &str) -> Result<Vec<String>, AppError> {
    let mut stmt =
        conn.prepare("SELECT id FROM entry WHERE project_id = ?1 AND deleted_at IS NULL")?;
    let rows = stmt
        .query_map(params![project_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn clear_all(conn: &Connection) -> Result<(), AppError> {
    conn.execute_batch(
        "DELETE FROM entry;
         DELETE FROM tag;
         DELETE FROM md_link;",
    )?;
    Ok(())
}

pub fn normalize_sync_states(conn: &Connection) -> Result<(), AppError> {
    conn.execute(
        "UPDATE entry SET sync_state = CASE WHEN sync_state = 'dirty' THEN 'local' ELSE sync_state END",
        [],
    )?;
    Ok(())
}

pub fn search(conn: &Connection, text: &str, limit: i64) -> Result<SearchResult, AppError> {
    let Some(expression) = fts_expression(text) else {
        return Ok(SearchResult {
            items: Vec::new(),
            total: 0,
        });
    };
    let limit = limit.clamp(1, 500);
    let total: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entry_fts JOIN entry e ON e.id = entry_fts.entry_id
         WHERE e.deleted_at IS NULL AND entry_fts MATCH ?1",
        params![expression],
        |row| row.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT entry_fts.entry_id, e.title, s.key,
                snippet(entry_fts, 2, '<mark>', '</mark>', '…', 24),
                bm25(entry_fts), e.updated_at
         FROM entry_fts
         JOIN entry e ON e.id = entry_fts.entry_id
         JOIN schema s ON s.id = e.schema_id
         WHERE e.deleted_at IS NULL AND entry_fts MATCH ?1
         ORDER BY bm25(entry_fts) LIMIT ?2",
    )?;
    let items = stmt
        .query_map(params![expression, limit], |row| {
            Ok(SearchHit {
                entry_id: row.get(0)?,
                title: row.get(1)?,
                schema_key: row.get(2)?,
                snippet: row.get(3)?,
                rank: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SearchResult { items, total })
}

pub fn validate_tags(conn: &Connection, tag_ids: &[String]) -> Result<(), AppError> {
    if tag_ids.is_empty() {
        return Ok(());
    }
    let mut known = HashSet::new();
    let mut stmt = conn.prepare("SELECT id FROM tag")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    for row in rows {
        known.insert(row?);
    }
    for tag_id in tag_ids {
        if !known.contains(tag_id) {
            return Err(AppError::validation("Unknown tag"));
        }
    }
    Ok(())
}

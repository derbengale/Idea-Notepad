use crate::dto::{FieldDefDto, SchemaDto};
use crate::error::AppError;
use crate::util::now_iso;
use rusqlite::{params, Connection};
use serde_json::Value;

pub fn list(conn: &Connection) -> Result<Vec<SchemaDto>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, key, name, icon, color, sort_order, title_field_id, subtitle_field_id,
                badge_field_id, format_version
         FROM schema ORDER BY sort_order ASC, name ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(SchemaDto {
                id: row.get(0)?,
                key: row.get(1)?,
                name: row.get(2)?,
                icon: row.get(3)?,
                color: row.get(4)?,
                order: row.get(5)?,
                title_field_id: row.get(6)?,
                subtitle_field_id: row.get(7)?,
                badge_field_id: row.get(8)?,
                format_version: row.get(9)?,
                fields: Vec::new(),
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut schemas = rows;
    for schema in &mut schemas {
        schema.fields = fields(conn, &schema.id)?;
    }
    Ok(schemas)
}

pub fn fields(conn: &Connection, schema_id: &str) -> Result<Vec<FieldDefDto>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, key, label, type, required, sort_order, config_json, expression, show_in_list
         FROM field_def WHERE schema_id = ?1 ORDER BY sort_order ASC, key ASC",
    )?;
    let rows = stmt
        .query_map(params![schema_id], |row| {
            let config_json: String = row.get(6)?;
            Ok(FieldDefDto {
                id: row.get(0)?,
                key: row.get(1)?,
                label: row.get(2)?,
                field_type: row.get(3)?,
                required: row.get::<_, i64>(4)? != 0,
                order: row.get(5)?,
                config: serde_json::from_str(&config_json).unwrap_or(Value::Null),
                expression: row.get(7)?,
                show_in_list: row.get::<_, i64>(8)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(conn: &Connection, id: &str) -> Result<SchemaDto, AppError> {
    let mut schema = conn.query_row(
        "SELECT id, key, name, icon, color, sort_order, title_field_id, subtitle_field_id,
                badge_field_id, format_version
         FROM schema WHERE id = ?1",
        params![id],
        |row| {
            Ok(SchemaDto {
                id: row.get(0)?,
                key: row.get(1)?,
                name: row.get(2)?,
                icon: row.get(3)?,
                color: row.get(4)?,
                order: row.get(5)?,
                title_field_id: row.get(6)?,
                subtitle_field_id: row.get(7)?,
                badge_field_id: row.get(8)?,
                format_version: row.get(9)?,
                fields: Vec::new(),
            })
        },
    )?;
    schema.fields = fields(conn, &schema.id)?;
    Ok(schema)
}

pub fn get_by_key(conn: &Connection, key: &str) -> Result<Option<SchemaDto>, AppError> {
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM schema WHERE key = ?1",
            params![key],
            |row| row.get(0),
        )
        .ok();
    match id {
        Some(id) => Ok(Some(get(conn, &id)?)),
        None => Ok(None),
    }
}

pub fn insert(conn: &Connection, schema: &SchemaDto) -> Result<(), AppError> {
    let now = now_iso();
    conn.execute(
        "INSERT INTO schema (id, key, name, icon, color, sort_order, title_field_id,
                             subtitle_field_id, badge_field_id, format_version, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
        params![
            schema.id,
            schema.key,
            schema.name,
            schema.icon,
            schema.color,
            schema.order,
            schema.title_field_id,
            schema.subtitle_field_id,
            schema.badge_field_id,
            schema.format_version,
            now
        ],
    )?;
    insert_fields(conn, &schema.id, &schema.fields)?;
    Ok(())
}

pub fn insert_fields(
    conn: &Connection,
    schema_id: &str,
    fields: &[FieldDefDto],
) -> Result<(), AppError> {
    let now = now_iso();
    for field in fields {
        conn.execute(
            "INSERT INTO field_def (id, schema_id, key, label, type, required, sort_order,
                                    config_json, expression, show_in_list, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)",
            params![
                field.id,
                schema_id,
                field.key,
                field.label,
                field.field_type,
                if field.required { 1_i64 } else { 0_i64 },
                field.order,
                serde_json::to_string(&field.config)?,
                field.expression,
                if field.show_in_list { 1_i64 } else { 0_i64 },
                now
            ],
        )?;
    }
    Ok(())
}

pub fn update(conn: &Connection, schema: &SchemaDto) -> Result<(), AppError> {
    let now = now_iso();
    conn.execute(
        "UPDATE schema SET name = ?2, icon = ?3, color = ?4, sort_order = ?5,
                title_field_id = ?6, subtitle_field_id = ?7, badge_field_id = ?8, updated_at = ?9
         WHERE id = ?1",
        params![
            schema.id,
            schema.name,
            schema.icon,
            schema.color,
            schema.order,
            schema.title_field_id,
            schema.subtitle_field_id,
            schema.badge_field_id,
            now
        ],
    )?;
    let keep: Vec<String> = schema.fields.iter().map(|field| field.id.clone()).collect();
    let existing: Vec<String> = fields(conn, &schema.id)?
        .into_iter()
        .map(|field| field.id)
        .collect();
    for id in existing {
        if !keep.contains(&id) {
            conn.execute("DELETE FROM field_def WHERE id = ?1", params![id])?;
        }
    }
    for field in &schema.fields {
        conn.execute(
            "INSERT INTO field_def (id, schema_id, key, label, type, required, sort_order,
                                    config_json, expression, show_in_list, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?11)
             ON CONFLICT(id) DO UPDATE SET
               label = excluded.label, type = excluded.type, required = excluded.required,
               sort_order = excluded.sort_order, config_json = excluded.config_json,
               expression = excluded.expression, show_in_list = excluded.show_in_list,
               updated_at = excluded.updated_at",
            params![
                field.id,
                schema.id,
                field.key,
                field.label,
                field.field_type,
                if field.required { 1_i64 } else { 0_i64 },
                field.order,
                serde_json::to_string(&field.config)?,
                field.expression,
                if field.show_in_list { 1_i64 } else { 0_i64 },
                now
            ],
        )?;
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: &str, reassign_to: &str) -> Result<i64, AppError> {
    if id == reassign_to {
        return Err(AppError::validation("Choose a different target schema"));
    }
    let target = get(conn, reassign_to)?;
    conn.execute(
        "INSERT INTO entry_value (entry_id, field_id, text_value, num_value, date_value, bool_value)
         SELECT ev.entry_id, tf.id, ev.text_value, ev.num_value, ev.date_value, ev.bool_value
         FROM entry_value ev
         JOIN field_def sf ON sf.id = ev.field_id
         JOIN field_def tf ON tf.schema_id = ?2 AND tf.key = sf.key AND tf.type = sf.type
         JOIN entry e ON e.id = ev.entry_id AND e.schema_id = ?1
         WHERE ev.text_value IS NOT NULL OR ev.num_value IS NOT NULL
            OR ev.date_value IS NOT NULL OR ev.bool_value IS NOT NULL
         ON CONFLICT(entry_id, field_id) DO NOTHING",
        params![id, target.id],
    )?;
    let moved = conn.execute(
        "UPDATE entry SET schema_id = ?2, updated_at = ?3, sync_state = 'dirty',
                local_rev = local_rev + 1
         WHERE schema_id = ?1",
        params![id, target.id, now_iso()],
    )?;
    conn.execute("DELETE FROM schema WHERE id = ?1", params![id])?;
    Ok(moved as i64)
}

pub fn reorder(conn: &Connection, ids: &[String]) -> Result<(), AppError> {
    for (index, id) in ids.iter().enumerate() {
        conn.execute(
            "UPDATE schema SET sort_order = ?2, updated_at = ?3 WHERE id = ?1",
            params![id, index as i64, now_iso()],
        )?;
    }
    Ok(())
}

pub fn upsert_remote(conn: &Connection, schema: &SchemaDto) -> Result<(), AppError> {
    match get_by_key(conn, &schema.key)? {
        Some(existing) => {
            let mut merged = schema.clone();
            merged.id = existing.id;
            update(conn, &merged)
        }
        None => insert(conn, schema),
    }
}

pub fn clear_field_values(conn: &Connection, field_id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM entry_value WHERE field_id = ?1",
        params![field_id],
    )?;
    Ok(())
}

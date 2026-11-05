use crate::dto::TagDto;
use crate::error::AppError;
use crate::util::{new_id, now_iso};
use rusqlite::{params, Connection};

pub fn list(conn: &Connection) -> Result<Vec<TagDto>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.color,
                (SELECT COUNT(*) FROM entry_tag et
                 JOIN entry e ON e.id = et.entry_id AND e.deleted_at IS NULL
                 WHERE et.tag_id = t.id) AS usage
         FROM tag t ORDER BY usage DESC, t.name COLLATE NOCASE ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(TagDto {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn create(conn: &Connection, name: &str, color: Option<&str>) -> Result<TagDto, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Tag name must not be empty"));
    }
    if trimmed.chars().count() > 60 {
        return Err(AppError::validation(
            "Tag name must not exceed 60 characters",
        ));
    }
    if let Some(existing) = find_by_name(conn, trimmed)? {
        return Ok(existing);
    }
    let id = new_id();
    conn.execute(
        "INSERT INTO tag (id, name, color, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, trimmed, color.unwrap_or("#64748b"), now_iso()],
    )?;
    Ok(TagDto {
        id,
        name: trimmed.to_string(),
        color: color.unwrap_or("#64748b").to_string(),
    })
}

pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<TagDto>, AppError> {
    let mut stmt =
        conn.prepare("SELECT id, name, color FROM tag WHERE name = ?1 COLLATE NOCASE LIMIT 1")?;
    let mut rows = stmt.query(params![name])?;
    if let Some(row) = rows.next()? {
        Ok(Some(TagDto {
            id: row.get(0)?,
            name: row.get(1)?,
            color: row.get(2)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM tag WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn entry_ids_for_tag(conn: &Connection, tag_id: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare("SELECT entry_id FROM entry_tag WHERE tag_id = ?1")?;
    let rows = stmt
        .query_map(params![tag_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn tags_for_entry(conn: &Connection, entry_id: &str) -> Result<Vec<TagDto>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT t.id, t.name, t.color
         FROM tag t JOIN entry_tag et ON et.tag_id = t.id
         WHERE et.entry_id = ?1 ORDER BY t.name COLLATE NOCASE ASC",
    )?;
    let rows = stmt
        .query_map(params![entry_id], |row| {
            Ok(TagDto {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn set_entry_tags(
    conn: &Connection,
    entry_id: &str,
    tag_ids: &[String],
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM entry_tag WHERE entry_id = ?1",
        params![entry_id],
    )?;
    for tag_id in tag_ids {
        conn.execute(
            "INSERT OR IGNORE INTO entry_tag (entry_id, tag_id) VALUES (?1, ?2)",
            params![entry_id, tag_id],
        )?;
    }
    Ok(())
}

pub fn resolve_names(conn: &Connection, entry_id: &str) -> Result<Vec<String>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT t.name FROM tag t JOIN entry_tag et ON et.tag_id = t.id
         WHERE et.entry_id = ?1 ORDER BY t.name COLLATE NOCASE ASC",
    )?;
    let rows = stmt
        .query_map(params![entry_id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn resolve_ids(conn: &Connection, names: &[String]) -> Result<Vec<String>, AppError> {
    let mut ids = Vec::new();
    for name in names {
        let tag = match find_by_name(conn, name)? {
            Some(tag) => tag,
            None => create(conn, name, None)?,
        };
        ids.push(tag.id);
    }
    Ok(ids)
}

use crate::dto::LinkItem;
use crate::error::AppError;
use crate::util::{new_id, now_iso};
use rusqlite::{params, Connection};

pub struct LinkRow {
    pub id: String,
}

pub fn create(conn: &Connection, from_id: &str, to_id: &str) -> Result<LinkRow, AppError> {
    if from_id == to_id {
        return Err(AppError::validation("A note cannot link to itself"));
    }
    let exists: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entry WHERE id = ?1",
        params![to_id],
        |row| row.get(0),
    )?;
    if exists == 0 {
        return Err(AppError::not_found("Target note not found"));
    }
    if let Some(existing) = find(conn, from_id, to_id)? {
        return Ok(existing);
    }
    let id = new_id();
    conn.execute(
        "INSERT INTO link (id, from_id, to_id, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![id, from_id, to_id, now_iso()],
    )?;
    Ok(LinkRow { id })
}

pub fn find(conn: &Connection, from_id: &str, to_id: &str) -> Result<Option<LinkRow>, AppError> {
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM link WHERE from_id = ?1 AND to_id = ?2",
            params![from_id, to_id],
            |row| row.get(0),
        )
        .ok();
    Ok(id.map(|id| LinkRow { id }))
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM link WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn outgoing(conn: &Connection, entry_id: &str) -> Result<Vec<LinkItem>, AppError> {
    query_items(
        conn,
        "SELECT l.id, e.id, e.title, s.key, e.deleted_at IS NOT NULL
         FROM link l JOIN entry e ON e.id = l.to_id JOIN schema s ON s.id = e.schema_id
         WHERE l.from_id = ?1 ORDER BY e.title COLLATE NOCASE ASC",
        entry_id,
    )
}

pub fn incoming(conn: &Connection, entry_id: &str) -> Result<Vec<LinkItem>, AppError> {
    query_items(
        conn,
        "SELECT l.id, e.id, e.title, s.key, e.deleted_at IS NOT NULL
         FROM link l JOIN entry e ON e.id = l.from_id JOIN schema s ON s.id = e.schema_id
         WHERE l.to_id = ?1 ORDER BY e.title COLLATE NOCASE ASC",
        entry_id,
    )
}

pub fn markdown_outgoing(conn: &Connection, entry_id: &str) -> Result<Vec<LinkItem>, AppError> {
    query_items(
        conn,
        "SELECT NULL, e.id, e.title, s.key, e.deleted_at IS NOT NULL
         FROM md_link m JOIN entry e ON e.id = m.to_entry_id JOIN schema s ON s.id = e.schema_id
         WHERE m.from_entry_id = ?1 ORDER BY e.title COLLATE NOCASE ASC",
        entry_id,
    )
}

pub fn markdown_incoming(conn: &Connection, entry_id: &str) -> Result<Vec<LinkItem>, AppError> {
    query_items(
        conn,
        "SELECT NULL, e.id, e.title, s.key, e.deleted_at IS NOT NULL
         FROM md_link m JOIN entry e ON e.id = m.from_entry_id JOIN schema s ON s.id = e.schema_id
         WHERE m.to_entry_id = ?1 ORDER BY e.title COLLATE NOCASE ASC",
        entry_id,
    )
}

fn query_items(conn: &Connection, sql: &str, entry_id: &str) -> Result<Vec<LinkItem>, AppError> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map(params![entry_id], |row| {
            Ok(LinkItem {
                id: row.get(0)?,
                entry_id: row.get(1)?,
                title: row.get(2)?,
                schema_key: row.get(3)?,
                deleted: row.get::<_, i64>(4)? != 0,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn replace_markdown_links(
    conn: &Connection,
    entry_id: &str,
    target_ids: &[String],
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM md_link WHERE from_entry_id = ?1",
        params![entry_id],
    )?;
    for target in target_ids {
        if target != entry_id {
            conn.execute(
                "INSERT OR IGNORE INTO md_link (from_entry_id, to_entry_id) VALUES (?1, ?2)",
                params![entry_id, target],
            )?;
        }
    }
    Ok(())
}

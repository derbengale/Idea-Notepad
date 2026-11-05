use crate::dto::WorkspaceStats;
use crate::error::AppError;
use rusqlite::{params, Connection, OptionalExtension};

pub fn get(conn: &Connection, key: &str) -> Result<Option<String>, AppError> {
    let value = conn
        .query_row(
            "SELECT value FROM meta WHERE key = ?1",
            params![key],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(value)
}

pub fn set(conn: &Connection, key: &str, value: &str) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO meta (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, key: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM meta WHERE key = ?1", params![key])?;
    Ok(())
}

pub fn stats(conn: &Connection) -> Result<WorkspaceStats, AppError> {
    let entries: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entry WHERE deleted_at IS NULL",
        [],
        |row| row.get(0),
    )?;
    let trash: i64 = conn.query_row(
        "SELECT COUNT(*) FROM entry WHERE deleted_at IS NOT NULL",
        [],
        |row| row.get(0),
    )?;
    let projects: i64 = conn.query_row("SELECT COUNT(*) FROM project", [], |row| row.get(0))?;
    let schemas: i64 = conn.query_row("SELECT COUNT(*) FROM schema", [], |row| row.get(0))?;
    let attachments: i64 =
        conn.query_row("SELECT COUNT(*) FROM attachment", [], |row| row.get(0))?;
    let last_sync_at = get(conn, "sync.lastSyncAt")?;
    Ok(WorkspaceStats {
        entries,
        projects,
        schemas,
        attachments,
        trash,
        last_sync_at,
    })
}

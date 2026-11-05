use crate::error::AppError;
use crate::util::{new_id, now_iso};
use rusqlite::{params, Connection};

#[derive(Debug, Clone)]
pub struct OutboxOp {
    pub id: String,
    pub op: String,
    pub target: String,
    pub payload_json: String,
    pub attempts: i64,
}

#[derive(Debug, Clone)]
pub struct SyncFile {
    pub drive_file_id: String,
    pub path: String,
    pub kind: String,
    pub etag: Option<String>,
    pub modified_at: Option<String>,
    pub local_rev: i64,
    pub sha256: Option<String>,
    pub last_synced_at: Option<String>,
}

pub fn enqueue(
    conn: &Connection,
    op: &str,
    target: &str,
    payload: serde_json::Value,
) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM outbox WHERE target = ?1 AND op = ?2",
        params![target, op],
    )?;
    conn.execute(
        "INSERT INTO outbox (id, op, target, payload_json, attempts, next_attempt_at, created_at)
         VALUES (?1, ?2, ?3, ?4, 0, NULL, ?5)",
        params![new_id(), op, target, payload.to_string(), now_iso()],
    )?;
    Ok(())
}

pub fn pending_count(conn: &Connection) -> Result<i64, AppError> {
    Ok(conn.query_row("SELECT COUNT(*) FROM outbox", [], |row| row.get(0))?)
}

pub fn next_ops(conn: &Connection, limit: i64) -> Result<Vec<OutboxOp>, AppError> {
    let now = now_iso();
    let mut stmt = conn.prepare(
        "SELECT id, op, target, payload_json, attempts FROM outbox
         WHERE next_attempt_at IS NULL OR next_attempt_at <= ?1
         ORDER BY created_at ASC LIMIT ?2",
    )?;
    let rows = stmt
        .query_map(params![now, limit], |row| {
            Ok(OutboxOp {
                id: row.get(0)?,
                op: row.get(1)?,
                target: row.get(2)?,
                payload_json: row.get(3)?,
                attempts: row.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn complete(conn: &Connection, op_id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM outbox WHERE id = ?1", params![op_id])?;
    Ok(())
}

pub fn fail(conn: &Connection, op_id: &str, error: &str, retry_at: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE outbox SET attempts = attempts + 1, last_error = ?2, next_attempt_at = ?3
         WHERE id = ?1",
        params![op_id, error, retry_at],
    )?;
    Ok(())
}

pub fn clear(conn: &Connection) -> Result<(), AppError> {
    conn.execute("DELETE FROM outbox", [])?;
    Ok(())
}

pub fn upsert_file(conn: &Connection, file: &SyncFile) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO sync_file (drive_file_id, path, kind, etag, modified_at, local_rev, sha256,
                                last_synced_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(drive_file_id) DO UPDATE SET
           path = excluded.path,
           kind = excluded.kind,
           etag = excluded.etag,
           modified_at = excluded.modified_at,
           local_rev = excluded.local_rev,
           sha256 = excluded.sha256,
           last_synced_at = excluded.last_synced_at",
        params![
            file.drive_file_id,
            file.path,
            file.kind,
            file.etag,
            file.modified_at,
            file.local_rev,
            file.sha256,
            file.last_synced_at
        ],
    )?;
    Ok(())
}

pub fn get_by_path(conn: &Connection, path: &str) -> Result<Option<SyncFile>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT drive_file_id, path, kind, etag, modified_at, local_rev, sha256, last_synced_at
         FROM sync_file WHERE path = ?1",
    )?;
    let mut rows = stmt.query(params![path])?;
    if let Some(row) = rows.next()? {
        Ok(Some(SyncFile {
            drive_file_id: row.get(0)?,
            path: row.get(1)?,
            kind: row.get(2)?,
            etag: row.get(3)?,
            modified_at: row.get(4)?,
            local_rev: row.get(5)?,
            sha256: row.get(6)?,
            last_synced_at: row.get(7)?,
        }))
    } else {
        Ok(None)
    }
}

pub fn delete_file(conn: &Connection, drive_id: &str) -> Result<(), AppError> {
    conn.execute(
        "DELETE FROM sync_file WHERE drive_file_id = ?1",
        params![drive_id],
    )?;
    Ok(())
}

pub fn clear_files(conn: &Connection) -> Result<(), AppError> {
    conn.execute("DELETE FROM sync_file", [])?;
    Ok(())
}

use crate::dto::AttachmentDto;
use crate::error::AppError;
use rusqlite::{params, Connection};

pub fn list(conn: &Connection, entry_id: &str) -> Result<Vec<AttachmentDto>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, entry_id, kind, file_name, mime_type, byte_size, sha256, local_path,
                preview_path, created_at
         FROM attachment WHERE entry_id = ?1
         ORDER BY CASE kind WHEN 'sketch' THEN 0 ELSE 1 END, created_at ASC",
    )?;
    let rows = stmt
        .query_map(params![entry_id], |row| {
            Ok(AttachmentDto {
                id: row.get(0)?,
                entry_id: row.get(1)?,
                kind: row.get(2)?,
                file_name: row.get(3)?,
                mime_type: row.get(4)?,
                byte_size: row.get(5)?,
                sha256: row.get(6)?,
                local_path: row.get(7)?,
                preview_path: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn insert(conn: &Connection, attachment: &AttachmentDto) -> Result<(), AppError> {
    conn.execute(
        "INSERT INTO attachment (id, entry_id, kind, file_name, mime_type, byte_size, sha256,
                                 local_path, preview_path, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            attachment.id,
            attachment.entry_id,
            attachment.kind,
            attachment.file_name,
            attachment.mime_type,
            attachment.byte_size,
            attachment.sha256,
            attachment.local_path,
            attachment.preview_path,
            attachment.created_at
        ],
    )?;
    Ok(())
}

pub fn get(conn: &Connection, id: &str) -> Result<AttachmentDto, AppError> {
    let mut stmt = conn.prepare(
        "SELECT id, entry_id, kind, file_name, mime_type, byte_size, sha256, local_path,
                preview_path, created_at
         FROM attachment WHERE id = ?1",
    )?;
    let mut rows = stmt.query(params![id])?;
    match rows.next()? {
        Some(row) => Ok(AttachmentDto {
            id: row.get(0)?,
            entry_id: row.get(1)?,
            kind: row.get(2)?,
            file_name: row.get(3)?,
            mime_type: row.get(4)?,
            byte_size: row.get(5)?,
            sha256: row.get(6)?,
            local_path: row.get(7)?,
            preview_path: row.get(8)?,
            created_at: row.get(9)?,
        }),
        None => Err(AppError::not_found("Attachment not found")),
    }
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    conn.execute("DELETE FROM attachment WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn set_drive_id(conn: &Connection, id: &str, drive_file_id: &str) -> Result<(), AppError> {
    conn.execute(
        "UPDATE attachment SET drive_file_id = ?2 WHERE id = ?1",
        params![id, drive_file_id],
    )?;
    Ok(())
}

pub fn update_storage(
    conn: &Connection,
    id: &str,
    byte_size: i64,
    sha256: &str,
    local_path: &str,
    preview_path: Option<&str>,
) -> Result<(), AppError> {
    conn.execute(
        "UPDATE attachment SET byte_size = ?2, sha256 = ?3, local_path = ?4, preview_path = ?5
         WHERE id = ?1",
        params![id, byte_size, sha256, local_path, preview_path],
    )?;
    Ok(())
}

pub fn total_bytes(conn: &Connection, entry_id: &str) -> Result<i64, AppError> {
    let total: i64 = conn.query_row(
        "SELECT COALESCE(SUM(byte_size), 0) FROM attachment WHERE entry_id = ?1",
        params![entry_id],
        |row| row.get(0),
    )?;
    Ok(total)
}

pub fn find_by_sha(conn: &Connection, entry_id: &str, sha256: &str) -> Result<bool, AppError> {
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM attachment WHERE entry_id = ?1 AND sha256 = ?2",
        params![entry_id, sha256],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}

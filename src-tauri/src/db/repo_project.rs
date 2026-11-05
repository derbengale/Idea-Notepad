use crate::dto::{ProjectDto, ProjectInput};
use crate::error::AppError;
use crate::util::{new_id, now_iso};
use rusqlite::{params, Connection};
use std::collections::HashSet;

pub fn list(conn: &Connection) -> Result<Vec<ProjectDto>, AppError> {
    let mut stmt = conn.prepare(
        "SELECT p.id, p.name, p.description, p.color, p.icon, p.sort_order, p.created_at,
                p.updated_at,
                (SELECT COUNT(*) FROM entry e WHERE e.project_id = p.id AND e.deleted_at IS NULL)
         FROM project p ORDER BY p.sort_order ASC, p.name ASC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(ProjectDto {
                id: row.get(0)?,
                name: row.get(1)?,
                description: row.get(2)?,
                color: row.get(3)?,
                icon: row.get(4)?,
                sort_order: row.get(5)?,
                created_at: row.get(6)?,
                updated_at: row.get(7)?,
                note_count: row.get(8)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get(conn: &Connection, id: &str) -> Result<ProjectDto, AppError> {
    let all = list(conn)?;
    match all.into_iter().find(|project| project.id == id) {
        Some(project) => Ok(project),
        None => Err(AppError::not_found("Project not found")),
    }
}

pub fn create(conn: &Connection, input: &ProjectInput) -> Result<ProjectDto, AppError> {
    let name = validate_name(conn, &input.name, None)?;
    let id = input.id.clone().unwrap_or_else(new_id);
    let now = now_iso();
    let order: i64 = conn.query_row(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM project",
        [],
        |row| row.get(0),
    )?;
    conn.execute(
        "INSERT INTO project (id, name, description, color, icon, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?7)",
        params![
            id,
            name,
            input.description.clone().unwrap_or_default(),
            input.color.clone().unwrap_or_else(|| "#64748b".to_string()),
            input.icon.clone().unwrap_or_else(|| "folder".to_string()),
            order,
            now
        ],
    )?;
    get(conn, &id)
}

pub fn update(conn: &Connection, project: &ProjectDto) -> Result<ProjectDto, AppError> {
    let name = validate_name(conn, &project.name, Some(&project.id))?;
    let changed = conn.execute(
        "UPDATE project SET name = ?2, description = ?3, color = ?4, icon = ?5,
                sort_order = ?6, updated_at = ?7
         WHERE id = ?1",
        params![
            project.id,
            name,
            project.description,
            project.color,
            project.icon,
            project.sort_order,
            now_iso()
        ],
    )?;
    if changed == 0 {
        return Err(AppError::not_found("Project not found"));
    }
    get(conn, &project.id)
}

pub fn delete(conn: &Connection, id: &str) -> Result<i64, AppError> {
    let moved = conn.execute(
        "UPDATE entry SET project_id = NULL, updated_at = ?2, sync_state = 'dirty',
                local_rev = local_rev + 1
         WHERE project_id = ?1 AND deleted_at IS NULL",
        params![id, now_iso()],
    )?;
    conn.execute("DELETE FROM project WHERE id = ?1", params![id])?;
    Ok(moved as i64)
}

pub fn find_by_name(conn: &Connection, name: &str) -> Result<Option<ProjectDto>, AppError> {
    let id: Option<String> = conn
        .query_row(
            "SELECT id FROM project WHERE LOWER(name) = LOWER(?1) LIMIT 1",
            params![name],
            |row| row.get(0),
        )
        .ok();
    match id {
        Some(id) => Ok(Some(get(conn, &id)?)),
        None => Ok(None),
    }
}

pub fn clear_all(conn: &Connection) -> Result<(), AppError> {
    conn.execute("DELETE FROM project", [])?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn insert_remote(
    conn: &Connection,
    id: &str,
    name: &str,
    description: &str,
    color: &str,
    icon: &str,
    sort_order: i64,
    created_at: &str,
    updated_at: &str,
) -> Result<(), AppError> {
    conn.execute(
        "INSERT OR IGNORE INTO project (id, name, description, color, icon, sort_order, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![id, name, description, color, icon, sort_order, created_at, updated_at],
    )?;
    Ok(())
}

fn validate_name(conn: &Connection, name: &str, exclude: Option<&str>) -> Result<String, AppError> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Project name must not be empty"));
    }
    if trimmed.chars().count() > 80 {
        return Err(AppError::validation(
            "Project name must not exceed 80 characters",
        ));
    }
    let mut stmt = conn.prepare("SELECT id, name FROM project WHERE LOWER(name) = LOWER(?1)")?;
    let existing: HashSet<String> = stmt
        .query_map(params![trimmed], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .filter_map(|row| row.ok())
        .filter(|(id, _)| Some(id.as_str()) != exclude)
        .map(|(_, name)| name.to_lowercase())
        .collect();
    if !existing.is_empty() {
        return Err(AppError::validation(
            "A project with this name already exists",
        ));
    }
    Ok(trimmed.to_string())
}

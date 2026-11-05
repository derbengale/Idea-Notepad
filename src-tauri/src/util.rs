use crate::error::AppError;
use chrono::{SecondsFormat, Utc};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use uuid::Uuid;

pub fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

pub fn short_id(id: &str) -> String {
    id.chars()
        .filter(|c| c.is_ascii_hexdigit())
        .take(8)
        .collect()
}

pub fn slugify(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    let mut last_dash = false;
    for raw in title.chars() {
        let folded = match raw {
            'ä' | 'Ä' => 'a',
            'ö' | 'Ö' => 'o',
            'ü' | 'Ü' => 'u',
            'ß' => 's',
            'é' | 'É' | 'è' | 'ê' => 'e',
            'à' | 'á' | 'â' => 'a',
            'í' | 'ì' | 'î' => 'i',
            'ó' | 'ò' | 'ô' => 'o',
            'ú' | 'ù' | 'û' => 'u',
            'ç' => 'c',
            'ñ' => 'n',
            other => other,
        };
        if folded.is_ascii_alphanumeric() {
            for lower in folded.to_lowercase() {
                if lower.is_ascii_alphanumeric() {
                    out.push(lower);
                }
            }
            last_dash = false;
        } else if !last_dash && !out.is_empty() {
            out.push('-');
            last_dash = true;
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        out.push_str("note");
    }
    out.truncate(60);
    while out.ends_with('-') {
        out.pop();
    }
    out
}

pub fn note_file_name(title: &str, id: &str) -> String {
    format!("{}--{}.md", slugify(title), short_id(id))
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temp = path.with_extension(format!(
        "{}.tmp",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("file")
    ));
    fs::write(&temp, bytes)?;
    fs::rename(&temp, path)?;
    Ok(())
}

pub fn ensure_dir(path: &Path) -> Result<(), AppError> {
    fs::create_dir_all(path)?;
    Ok(())
}

pub fn normalize_body(body: &str) -> String {
    let unified = body.replace("\r\n", "\n").replace('\r', "\n");
    let trimmed = unified.trim_end_matches('\n');
    format!("{trimmed}\n")
}

pub fn is_uuid(value: &str) -> bool {
    Uuid::parse_str(value).is_ok()
}

pub fn validate_title(title: &str) -> Result<String, AppError> {
    let trimmed = title.trim();
    if trimmed.is_empty() {
        return Err(AppError::validation("Title must not be empty"));
    }
    if trimmed.chars().count() > 300 {
        return Err(AppError::validation("Title must not exceed 300 characters"));
    }
    Ok(trimmed.to_string())
}

pub fn truncate_chars(value: &str, max: usize) -> String {
    value.chars().take(max).collect()
}

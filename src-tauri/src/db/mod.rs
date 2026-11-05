pub mod repo_attachment;
pub mod repo_entry;
pub mod repo_link;
pub mod repo_meta;
pub mod repo_project;
pub mod repo_schema;
pub mod repo_sync;
pub mod repo_tag;

use crate::error::AppError;
use parking_lot::Mutex;
use rusqlite::{Connection, OptionalExtension};
use std::path::{Path, PathBuf};

const MIGRATIONS: &[(i64, &str)] = &[(1, include_str!("migrations/0001_init.sql"))];

pub struct Db {
    path: PathBuf,
    writer: Mutex<Connection>,
    readers: Mutex<Vec<Connection>>,
}

impl Db {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::backup_before_migration(path)?;
        let mut writer = Self::connect(path)?;
        Self::migrate(&mut writer)?;
        let readers = Mutex::new(Vec::new());
        Ok(Self {
            path: path.to_path_buf(),
            writer: Mutex::new(writer),
            readers,
        })
    }

    fn connect(path: &Path) -> Result<Connection, AppError> {
        let conn = Connection::open(path)?;
        conn.busy_timeout(std::time::Duration::from_millis(5000))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA synchronous = NORMAL;
             PRAGMA temp_store = MEMORY;",
        )?;
        Ok(conn)
    }

    fn backup_before_migration(path: &Path) -> Result<(), AppError> {
        if !path.exists() {
            return Ok(());
        }
        let version = crate::util::now_iso().replace([':', '.'], "-");
        let backup = path.with_file_name(format!("better-notepad.db.backup-{version}"));
        std::fs::copy(path, &backup)?;
        let mut backups: Vec<PathBuf> = std::fs::read_dir(
            path.parent()
                .ok_or_else(|| AppError::io("Database path has no parent"))?,
        )?
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|name| name.to_str())
                .map(|name| name.starts_with("better-notepad.db.backup-"))
                .unwrap_or(false)
        })
        .collect();
        backups.sort();
        while backups.len() > 2 {
            if let Some(oldest) = backups.first().cloned() {
                let _ = std::fs::remove_file(oldest);
                backups.remove(0);
            }
        }
        Ok(())
    }

    fn migrate(conn: &mut Connection) -> Result<(), AppError> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS schema_migration (
               version    INTEGER PRIMARY KEY,
               applied_at TEXT NOT NULL
             );",
        )?;
        let current: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migration",
                [],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or(0);
        let newest = MIGRATIONS
            .iter()
            .map(|(version, _)| *version)
            .max()
            .unwrap_or(0);
        if current > newest {
            return Err(AppError::unsupported(
                "The local database was created by a newer version of Better Notepad.",
            ));
        }
        for (version, sql) in MIGRATIONS {
            if *version > current {
                let tx = conn.transaction()?;
                tx.execute_batch(sql)?;
                tx.execute(
                    "INSERT INTO schema_migration (version, applied_at) VALUES (?1, ?2)",
                    rusqlite::params![version, crate::util::now_iso()],
                )?;
                tx.commit()?;
                tracing::info!(version, "applied database migration");
            }
        }
        Ok(())
    }

    pub fn with_read<T>(
        &self,
        f: impl FnOnce(&Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let conn = {
            let mut pool = self.readers.lock();
            pool.pop()
        };
        let conn = match conn {
            Some(conn) => conn,
            None => Self::connect(&self.path)?,
        };
        let result = f(&conn);
        let mut pool = self.readers.lock();
        if pool.len() < 4 {
            pool.push(conn);
        }
        result
    }

    pub fn with_write<T>(
        &self,
        f: impl FnOnce(&mut Connection) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        let mut guard = self.writer.lock();
        f(&mut guard)
    }
}

pub fn migration_version(db: &Db) -> Result<i64, AppError> {
    db.with_read(|conn| {
        let version: Option<i64> = conn
            .query_row("SELECT MAX(version) FROM schema_migration", [], |row| {
                row.get(0)
            })
            .optional()?;
        Ok(version.unwrap_or(0))
    })
}

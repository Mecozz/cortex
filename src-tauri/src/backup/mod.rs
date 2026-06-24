pub mod health;

use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupEntry {
    pub name: String,
    pub filename: String,
    pub size_bytes: u64,
    pub created_at: i64,
}

pub fn backup_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("backups")
}

fn to_rq(e: impl std::error::Error + Send + Sync + 'static) -> rusqlite::Error {
    rusqlite::Error::ToSqlConversionFailure(Box::new(e))
}

pub fn create(conn: &Connection, data_dir: &Path, label: Option<&str>) -> Result<String> {
    let dir = backup_dir(data_dir);
    std::fs::create_dir_all(&dir).map_err(to_rq)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let filename = match label {
        Some(n) if !n.is_empty() => format!("cortex_{}.db", n.replace(' ', "_")),
        _ => format!("cortex_{ts}.db"),
    };
    let path = dir.join(&filename);
    let path_str = path.to_string_lossy();
    conn.execute_batch(&format!("VACUUM INTO '{path_str}'"))?;
    prune(data_dir);
    Ok(filename)
}

pub fn list(data_dir: &Path) -> Vec<BackupEntry> {
    let dir = backup_dir(data_dir);
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return vec![];
    };
    let mut result: Vec<BackupEntry> = entries
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "db").unwrap_or(false))
        .map(|e| {
            let path = e.path();
            let meta = std::fs::metadata(&path).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let created = meta
                .and_then(|m| m.created().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
                .unwrap_or(0);
            let fname = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let name = fname
                .trim_start_matches("cortex_")
                .trim_end_matches(".db")
                .to_string();
            BackupEntry {
                name,
                filename: fname,
                size_bytes: size,
                created_at: created,
            }
        })
        .collect();
    result.sort_by_key(|b| std::cmp::Reverse(b.created_at));
    result
}

pub fn delete(data_dir: &Path, filename: &str) -> std::io::Result<()> {
    std::fs::remove_file(backup_dir(data_dir).join(filename))
}

pub fn prune(data_dir: &Path) {
    let entries = list(data_dir);
    for e in entries.iter().skip(30) {
        let _ = delete(data_dir, &e.filename);
    }
}

pub fn queue_restore(data_dir: &Path, filename: &str) -> std::io::Result<()> {
    let pending = data_dir.join("restore_pending.txt");
    std::fs::write(pending, filename)
}

pub fn check_pending_restore(data_dir: &Path, db_path: &Path) {
    let pending = data_dir.join("restore_pending.txt");
    if let Ok(filename) = std::fs::read_to_string(&pending) {
        let src = backup_dir(data_dir).join(filename.trim());
        if src.exists() {
            let _ = std::fs::copy(&src, db_path);
        }
        let _ = std::fs::remove_file(&pending);
    }
}

pub fn reset(conn: &Connection, level: u8) -> Result<()> {
    let sql = match level {
        2 => "DELETE FROM episodic WHERE created_at > unixepoch() - 86400",
        3 => "DELETE FROM facts; DELETE FROM scores",
        4 => "DELETE FROM tasks",
        5 => "DELETE FROM facts; DELETE FROM episodic; DELETE FROM scores; DELETE FROM tasks",
        6 => "DELETE FROM facts; DELETE FROM episodic; DELETE FROM scores; DELETE FROM tasks; DELETE FROM vault",
        7 => "DELETE FROM facts; DELETE FROM episodic; DELETE FROM scores; DELETE FROM tasks; DELETE FROM vault; DELETE FROM settings; DELETE FROM proj",
        _ => return Ok(()),
    };
    conn.execute_batch(sql)
}

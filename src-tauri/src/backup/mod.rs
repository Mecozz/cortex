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

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// A SQLite sidecar path next to `db_path` (e.g. `cortex.db-wal`).
fn sidecar(db_path: &Path, suffix: &str) -> PathBuf {
    let mut s = db_path.as_os_str().to_owned();
    s.push(suffix);
    PathBuf::from(s)
}

/// `VACUUM INTO` fails if the destination exists, so clear the target and its
/// WAL/shm sidecars first. Single quotes in the path are escaped for the SQL
/// string literal.
fn vacuum_into(conn: &Connection, dest: &Path) -> Result<()> {
    let _ = std::fs::remove_file(dest);
    let _ = std::fs::remove_file(sidecar(dest, "-wal"));
    let _ = std::fs::remove_file(sidecar(dest, "-shm"));
    let esc = dest.to_string_lossy().replace('\'', "''");
    conn.execute_batch(&format!("VACUUM INTO '{esc}'"))
}

/// True if `path` is a non-empty SQLite database that passes a quick integrity
/// check and contains the expected schema. Guards swaps against truncated /
/// mid-sync / corrupt files clobbering the live brain.
pub fn is_valid_db(path: &Path) -> bool {
    if std::fs::metadata(path).map(|m| m.len()).unwrap_or(0) == 0 {
        return false;
    }
    match Connection::open(path) {
        Ok(c) => {
            let ok = c
                .query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
                .map(|s| s == "ok")
                .unwrap_or(false);
            let has_schema = c
                .query_row(
                    "SELECT 1 FROM sqlite_master WHERE type='table' AND name='facts'",
                    [],
                    |_| Ok(true),
                )
                .unwrap_or(false);
            ok && has_schema
        }
        Err(_) => false,
    }
}

/// Best-effort copy of the current live DB into the backups dir before an
/// overwrite, so a bad restore/sync is recoverable. Named so `prune` won't reap it.
fn snapshot_before_swap(data_dir: &Path, db_path: &Path) {
    if !db_path.exists() {
        return;
    }
    let dir = backup_dir(data_dir);
    let _ = std::fs::create_dir_all(&dir);
    let dest = dir.join(format!("cortex_pre_restore_{}.db", now_secs()));
    let _ = std::fs::copy(db_path, &dest);
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
    vacuum_into(conn, &path)?;
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
    // Only auto (bare-timestamp) backups are pruned. Named/labeled snapshots
    // (anything whose name isn't all digits) are kept forever, so a user's named
    // backup — or a pre_restore safety copy — can't be silently deleted.
    let auto: Vec<BackupEntry> = list(data_dir)
        .into_iter()
        .filter(|e| !e.name.is_empty() && e.name.chars().all(|c| c.is_ascii_digit()))
        .collect();
    for e in auto.iter().skip(30) {
        let _ = delete(data_dir, &e.filename);
    }
}

pub fn queue_restore(data_dir: &Path, filename: &str) -> std::io::Result<()> {
    let pending = data_dir.join("restore_pending.txt");
    std::fs::write(pending, filename)
}

pub fn check_pending_restore(data_dir: &Path, db_path: &Path) {
    let pending = data_dir.join("restore_pending.txt");
    let Ok(path_str) = std::fs::read_to_string(&pending) else {
        return;
    };
    let path_str = path_str.trim();
    let src = if Path::new(path_str).is_absolute() {
        PathBuf::from(path_str)
    } else {
        backup_dir(data_dir).join(path_str)
    };
    // Only swap in a source that's a valid, non-empty SQLite brain — a truncated,
    // mid-sync, or corrupt file must never silently overwrite the live DB. Snapshot
    // the current DB first so a bad swap is recoverable.
    if src.exists() && is_valid_db(&src) {
        snapshot_before_swap(data_dir, db_path);
        if std::fs::copy(&src, db_path).is_ok() {
            // Drop stale WAL/shm so they can't replay over the swapped-in DB.
            let _ = std::fs::remove_file(sidecar(db_path, "-wal"));
            let _ = std::fs::remove_file(sidecar(db_path, "-shm"));
        }
    }
    let _ = std::fs::remove_file(&pending);
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

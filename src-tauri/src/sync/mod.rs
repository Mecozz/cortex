pub mod health;

use rusqlite::Connection;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
pub struct SyncStatus {
    pub folder: String,
    pub incoming_newer: bool,
    pub incoming_ts: i64,
    pub local_ts: i64,
}

pub fn export(conn: &Connection, sync_folder: &str) -> rusqlite::Result<()> {
    let dest = PathBuf::from(sync_folder).join("cortex.db");
    let path_str = dest.to_string_lossy();
    conn.execute_batch(&format!("VACUUM INTO '{path_str}'"))
}

pub fn status(db_path: &Path, sync_folder: &str) -> SyncStatus {
    let src = PathBuf::from(sync_folder).join("cortex.db");
    let ts = |p: &Path| -> i64 {
        std::fs::metadata(p)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    };
    let local_ts = ts(db_path);
    let incoming_ts = ts(&src);
    SyncStatus {
        folder: sync_folder.to_string(),
        incoming_newer: incoming_ts > local_ts,
        incoming_ts,
        local_ts,
    }
}

pub fn queue_import(data_dir: &Path, sync_folder: &str) -> std::io::Result<()> {
    let src = PathBuf::from(sync_folder).join("cortex.db");
    if !src.exists() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "cortex.db not found in sync folder",
        ));
    }
    let pending = data_dir.join("restore_pending.txt");
    std::fs::write(pending, src.to_string_lossy().as_ref())
}

pub mod migrate;

use rusqlite::Connection;
use std::path::Path;

pub fn init(path: &Path) -> Result<Connection, Box<dyn std::error::Error>> {
    let conn = Connection::open(path)?;
    // busy_timeout so concurrent writers (the chat path's DbState connection and
    // the background lib-cycle's separate connection) wait instead of instantly
    // failing with SQLITE_BUSY and silently dropping the write.
    conn.execute_batch(
        "PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;",
    )?;
    migrate::run(&conn)?;
    Ok(conn)
}

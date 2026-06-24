pub mod migrate;

use rusqlite::Connection;
use std::path::Path;

pub fn init(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::open(path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
    migrate::run(&conn)?;
    Ok(())
}

use rusqlite::Connection;
use super::conf::Fact;

/// PASS1 — fast retrieval layer.
/// Returns all current high-confidence facts for a project.
pub fn retrieve(conn: &Connection, proj_id: &str, limit: usize) -> rusqlite::Result<Vec<Fact>> {
    super::conf::current(conn, proj_id, limit)
}

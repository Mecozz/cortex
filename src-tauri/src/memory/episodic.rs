use rusqlite::{Connection, Result};
use uuid::Uuid;

pub struct EpisodicEntry {
    pub role: String,
    pub content: String,
    pub conversation_id: String,
}

pub fn log(conn: &Connection, proj_id: &str, entry: &EpisodicEntry) -> Result<()> {
    let now = now_secs();
    conn.execute(
        "INSERT INTO episodic (id, proj_id, conversation_id, role, content, timestamp, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?6)",
        rusqlite::params![
            Uuid::new_v4().to_string(),
            proj_id,
            entry.conversation_id,
            entry.role,
            entry.content,
            now
        ],
    )?;
    Ok(())
}

pub fn count(conn: &Connection, proj_id: &str) -> Result<i64> {
    conn.query_row(
        "SELECT COUNT(*) FROM episodic WHERE proj_id = ?1",
        rusqlite::params![proj_id],
        |row| row.get(0),
    )
}

pub fn search(
    conn: &Connection,
    proj_id: &str,
    query: &str,
    limit: usize,
) -> Result<Vec<(String, String)>> {
    let pattern = format!("%{query}%");
    let mut stmt = conn.prepare(
        "SELECT role, content FROM episodic
         WHERE proj_id = ?1 AND content LIKE ?2
         ORDER BY timestamp DESC
         LIMIT ?3",
    )?;
    let rows = stmt.query_map(rusqlite::params![proj_id, pattern, limit as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    rows.collect()
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

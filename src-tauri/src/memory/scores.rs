use rusqlite::{Connection, Result};
use uuid::Uuid;

pub fn buffer(conn: &Connection, content: &str, score: f64, conversation_id: &str) -> Result<()> {
    let now = now_secs();
    let expires = now + 86_400;
    conn.execute(
        "INSERT INTO scores (id, content, score, conversation_id, buffered_at, expires_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            Uuid::new_v4().to_string(),
            content,
            score,
            conversation_id,
            now,
            expires
        ],
    )?;
    Ok(())
}

pub fn flush_expired(conn: &Connection) -> Result<()> {
    let now = now_secs();
    conn.execute(
        "DELETE FROM scores WHERE expires_at < ?1",
        rusqlite::params![now],
    )?;
    Ok(())
}

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

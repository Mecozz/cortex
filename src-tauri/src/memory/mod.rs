pub mod conf;
pub mod episodic;
pub mod health;
pub mod instcap;
pub mod pass1;
pub mod pass2;
pub mod rollext;
pub mod scores;

use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub fn default_project_id(conn: &Connection) -> rusqlite::Result<String> {
    if let Ok(id) = conn.query_row(
        "SELECT id FROM proj WHERE name = 'default' LIMIT 1",
        [],
        |row| row.get::<_, String>(0),
    ) {
        return Ok(id);
    }
    let id = Uuid::new_v4().to_string();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    conn.execute(
        "INSERT INTO proj (id, name, description, is_active, created_at, updated_at)
         VALUES (?1, 'default', 'Default project', 1, ?2, ?2)",
        params![id, now],
    )?;
    Ok(id)
}

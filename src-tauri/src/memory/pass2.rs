use super::conf::Fact;
use rusqlite::{Connection, Result};

pub fn retrieve(conn: &Connection, proj_id: &str, limit: usize) -> Result<Vec<Fact>> {
    let mut stmt = conn.prepare(
        "SELECT content, category, confidence_score, proj_id FROM facts
         WHERE proj_id = ?1 AND is_current = 1
         ORDER BY confidence_score DESC, created_at DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![proj_id, limit as i64], |row| {
        Ok(Fact {
            content: row.get(0)?,
            category: row.get(1)?,
            confidence: row.get::<_, f64>(2)? as f32,
            proj_id: row.get(3)?,
        })
    })?;
    rows.collect()
}

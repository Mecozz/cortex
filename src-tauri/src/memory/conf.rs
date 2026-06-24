use rusqlite::{params, Connection};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

pub struct Fact {
    pub content: String,
    pub category: String,
    pub confidence: f32,
    pub proj_id: String,
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

/// Store a fact. For identity/preference categories, invalidates all older facts
/// in the same category first so the newer info takes precedence.
pub fn upsert(conn: &Connection, fact: &Fact) -> rusqlite::Result<()> {
    let now = now_secs();
    if matches!(fact.category.as_str(), "identity" | "preference") {
        conn.execute(
            "UPDATE facts SET is_current = 0, valid_until = ?1
             WHERE proj_id = ?2 AND category = ?3 AND is_current = 1",
            params![now, fact.proj_id, fact.category],
        )?;
    }
    conn.execute(
        "INSERT INTO facts
         (id, proj_id, content, category, is_current, valid_from,
          last_confirmed_at, confidence_score, importance_score, created_at)
         VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5, ?6, ?6, ?5)",
        params![
            Uuid::new_v4().to_string(),
            fact.proj_id,
            fact.content,
            fact.category,
            now,
            fact.confidence as f64,
        ],
    )?;
    Ok(())
}

/// Return all current facts for a project, highest confidence first.
pub fn current(conn: &Connection, proj_id: &str, limit: usize) -> rusqlite::Result<Vec<Fact>> {
    let mut stmt = conn.prepare(
        "SELECT content, category, confidence_score FROM facts
         WHERE proj_id = ?1 AND is_current = 1
         ORDER BY confidence_score DESC, created_at DESC
         LIMIT ?2",
    )?;
    let facts = stmt
        .query_map(params![proj_id, limit as i64], |row| {
            Ok(Fact {
                content: row.get(0)?,
                category: row.get(1)?,
                confidence: row.get::<_, f64>(2)? as f32,
                proj_id: proj_id.to_string(),
            })
        })?
        .filter_map(Result::ok)
        .collect();
    Ok(facts)
}

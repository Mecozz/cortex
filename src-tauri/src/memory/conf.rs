use rusqlite::{params, Connection, OptionalExtension};
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

/// Store a fact.
///
/// If an identical current fact already exists for the project, just refresh its
/// confirmation time and keep the higher confidence (dedup + re-confirmation) —
/// this both prevents unbounded duplicates and resets decay when the user
/// re-mentions something. Otherwise insert a new current fact.
///
/// We deliberately do NOT retire a whole category: "identity" covers name, job,
/// location, and family, so category-wide invalidation wiped unrelated facts
/// (saving a location used to drop the user's name). True contradiction handling
/// (same attribute, new value) is left to higher-level resolution.
pub fn upsert(conn: &Connection, fact: &Fact) -> rusqlite::Result<()> {
    let now = now_secs();
    let existing: Option<i64> = conn
        .query_row(
            "SELECT rowid FROM facts
             WHERE proj_id = ?1 AND content = ?2 AND is_current = 1
             LIMIT 1",
            params![fact.proj_id, fact.content],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(rowid) = existing {
        conn.execute(
            "UPDATE facts
             SET last_confirmed_at = ?1,
                 confidence_score = MAX(confidence_score, ?2)
             WHERE rowid = ?3",
            params![now, fact.confidence as f64, rowid],
        )?;
        return Ok(());
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

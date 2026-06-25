use super::conf::Fact;
use rusqlite::{types::Value, Connection};

/// PASS1 — fast retrieval layer.
/// Returns all current high-confidence facts for a project.
pub fn retrieve(conn: &Connection, proj_id: &str, limit: usize) -> rusqlite::Result<Vec<Fact>> {
    super::conf::current(conn, proj_id, limit)
}

/// Common words that carry no retrieval signal — matching on them drowns out
/// the meaningful terms (e.g. "what do you know about my IBEW grievance").
const STOPWORDS: &[&str] = &[
    "the", "and", "you", "your", "yours", "for", "with", "that", "this", "have", "has", "had",
    "what", "know", "about", "who", "whom", "why", "how", "when", "where", "does", "did", "can",
    "could", "would", "should", "tell", "give", "are", "was", "were", "from", "not", "but", "all",
    "any", "get", "got", "its", "our", "out", "now", "than", "then", "them", "they", "there",
    "here", "into", "over", "just", "like", "want", "need", "please", "whats", "anything",
    "something", "everything", "some", "more", "most", "much", "many", "been", "being", "his",
    "her", "she", "him", "their", "mine", "me",
];

/// Query-aware retrieval: current facts matching the meaningful words in `query`,
/// ranked by how many distinct terms they hit, then by confidence. Returns empty
/// if the query has no usable terms (caller should fall back to `retrieve`).
pub fn search(
    conn: &Connection,
    proj_id: &str,
    query: &str,
    limit: usize,
) -> rusqlite::Result<Vec<Fact>> {
    let terms: Vec<String> = query
        .split(|c: char| !c.is_alphanumeric())
        .map(|w| w.to_lowercase())
        .filter(|w| w.len() >= 3 && !STOPWORDS.contains(&w.as_str()))
        .take(8)
        .collect();
    if terms.is_empty() {
        return Ok(vec![]);
    }
    let ors = terms
        .iter()
        .map(|_| "LOWER(content) LIKE ?")
        .collect::<Vec<_>>()
        .join(" OR ");
    // Pull a generous candidate set matching ANY term, then rank in Rust by how
    // many distinct terms each fact hits (so "ibew grievance" beats a fact that
    // only mentions "ibew" in passing).
    let sql = format!(
        "SELECT content, category, confidence_score, proj_id FROM facts
         WHERE proj_id = ? AND is_current = 1 AND ({ors})
         ORDER BY confidence_score DESC
         LIMIT 200"
    );
    let mut p: Vec<Value> = vec![Value::Text(proj_id.to_string())];
    for t in &terms {
        p.push(Value::Text(format!("%{t}%")));
    }

    let mut stmt = conn.prepare(&sql)?;
    let mut candidates: Vec<Fact> = stmt
        .query_map(rusqlite::params_from_iter(p), |row| {
            Ok(Fact {
                content: row.get(0)?,
                category: row.get(1)?,
                confidence: row.get::<_, f64>(2)? as f32,
                proj_id: row.get(3)?,
            })
        })?
        .filter_map(Result::ok)
        .collect();

    let score = |f: &Fact| -> usize {
        let lc = f.content.to_lowercase();
        terms.iter().filter(|t| lc.contains(t.as_str())).count()
    };
    candidates.sort_by(|a, b| {
        score(b)
            .cmp(&score(a))
            .then(b.confidence.partial_cmp(&a.confidence).unwrap_or(std::cmp::Ordering::Equal))
    });
    candidates.truncate(limit);
    Ok(candidates)
}

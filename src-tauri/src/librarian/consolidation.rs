//! REM-cycle consolidation: turn raw episodic chatter into durable knowledge.
//!
//! Three jobs, run from the background lib cycle:
//!   1. relationships — extract subject-predicate-object triples → `rel`
//!   2. decay         — fade confidence of stale facts, retire the faded ones
//!   3. summaries     — condense finished conversations → `convo`
//!
//! LLM calls (async) take plain data only; DB work (sync) takes `&Connection`.
//! Keeping them separate stops a non-Send `&Connection` from being held across
//! an `.await`, so the whole cycle stays spawnable on the Tokio runtime.

use rusqlite::{params, Connection};
use serde::Deserialize;
use uuid::Uuid;

use crate::core::types::Message;

const DAY: i64 = 86_400;

fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn snippet(messages: &[Message], n: usize) -> String {
    messages
        .iter()
        .rev()
        .take(n)
        .rev()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n")
}

/// One Haiku call (Anthropic API if a key is set, else the subscription),
/// returning the assistant text or None.
async fn haiku_json(api_key: &str, system: &str, user: &str, max_tokens: u32) -> Option<String> {
    crate::core::llm::haiku(api_key, system, user, max_tokens).await
}

// ── Relationships ────────────────────────────────────────────────────────────

pub struct Triple {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
}

#[derive(Deserialize)]
struct RawTriple {
    subject: String,
    predicate: String,
    object: String,
    #[serde(default = "default_conf")]
    confidence: f32,
}
fn default_conf() -> f32 {
    0.6
}

const REL_SYSTEM: &str = "Extract factual relationships between entities from the conversation as subject-predicate-object triples. Example: [{\"subject\":\"Darren\",\"predicate\":\"owns\",\"object\":\"a gaming PC\",\"confidence\":0.9}]. Concrete relationships only, not opinions or questions. Return ONLY a JSON array, [] if none. No other text.";

/// LLM step: pull triples from recent messages. No DB access.
pub async fn extract_relationship_triples(messages: &[Message], api_key: &str) -> Vec<Triple> {
    let text = snippet(messages, 12);
    let raw = match haiku_json(api_key, REL_SYSTEM, &text, 768).await {
        Some(t) => t,
        None => return vec![],
    };
    serde_json::from_str::<Vec<RawTriple>>(crate::core::llm::json_slice(&raw))
        .unwrap_or_default()
        .into_iter()
        .filter(|t| !t.subject.trim().is_empty() && !t.object.trim().is_empty())
        .map(|t| Triple {
            subject: t.subject,
            predicate: t.predicate,
            object: t.object,
            confidence: t.confidence.clamp(0.0, 1.0),
        })
        .collect()
}

/// DB step: store new triples (skips exact duplicates). Returns # inserted.
pub fn store_relationships(conn: &Connection, triples: &[Triple], source: &str) -> usize {
    let now = now_secs();
    let mut inserted = 0;
    for t in triples {
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM rel WHERE subject=?1 AND predicate=?2 AND object=?3 LIMIT 1",
                params![t.subject, t.predicate, t.object],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if exists {
            continue;
        }
        if conn
            .execute(
                "INSERT INTO rel (id, subject, predicate, object, confidence, source_conversation, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    Uuid::new_v4().to_string(),
                    t.subject,
                    t.predicate,
                    t.object,
                    t.confidence as f64,
                    source,
                    now
                ],
            )
            .is_ok()
        {
            inserted += 1;
        }
    }
    inserted
}

// ── Decay ────────────────────────────────────────────────────────────────────

/// Gently fade confidence of facts unconfirmed for >30 days, then retire
/// (is_current=0) any that fall below the floor. Identity facts are never
/// auto-retired. Returns # retired.
///
/// The cycle runs hourly, so the per-run factor is tiny (0.9995 ≈ half-life ~58
/// days past the staleness threshold; ~6 months of neglect before a fact retires
/// from 0.85). Re-mentioning a fact refreshes `last_confirmed_at` (see
/// `conf::upsert`), which pulls it back out of the stale window entirely.
pub fn decay_facts(conn: &Connection) -> usize {
    let now = now_secs();
    let stale_before = now - 30 * DAY;
    let _ = conn.execute(
        "UPDATE facts SET confidence_score = confidence_score * 0.9995
         WHERE is_current = 1 AND last_confirmed_at IS NOT NULL AND last_confirmed_at < ?1",
        params![stale_before],
    );
    conn.execute(
        "UPDATE facts SET is_current = 0, valid_until = ?1
         WHERE is_current = 1 AND confidence_score < 0.15 AND category != 'identity'",
        params![now],
    )
    .unwrap_or(0)
}

// ── Conversation summaries ───────────────────────────────────────────────────

pub struct SummaryCandidate {
    pub conv_id: String,
    pub msg_count: i64,
    pub started: i64,
    pub ended: i64,
    pub messages: Vec<Message>,
}

/// DB step: conversations with ≥4 messages, idle >1h, not yet summarized.
pub fn conversations_needing_summary(
    conn: &Connection,
    proj_id: &str,
    max: usize,
) -> Vec<SummaryCandidate> {
    let now = now_secs();
    let ids: Vec<(String, i64, i64, i64)> = {
        let mut stmt = match conn.prepare(
            "SELECT conversation_id, COUNT(*), MIN(timestamp), MAX(timestamp) FROM episodic
             WHERE proj_id = ?1 AND conversation_id NOT IN (SELECT id FROM convo)
             GROUP BY conversation_id HAVING COUNT(*) >= 4 AND MAX(timestamp) < ?2",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        let rows = stmt.query_map(params![proj_id, now - 3600], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
        });
        match rows {
            Ok(rr) => rr.filter_map(Result::ok).take(max).collect(),
            Err(_) => return vec![],
        }
    };
    let mut out = Vec::new();
    for (conv_id, msg_count, started, ended) in ids {
        let messages: Vec<Message> = {
            let mut stmt = match conn.prepare(
                "SELECT role, content FROM episodic WHERE conversation_id = ?1 ORDER BY timestamp ASC LIMIT 40",
            ) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let rows = stmt.query_map(params![conv_id], |r| {
                Ok(Message {
                    role: r.get(0)?,
                    content: r.get(1)?,
                })
            });
            match rows {
                Ok(rr) => rr.filter_map(Result::ok).collect(),
                Err(_) => continue,
            }
        };
        out.push(SummaryCandidate {
            conv_id,
            msg_count,
            started,
            ended,
            messages,
        });
    }
    out
}

#[derive(Deserialize)]
struct RawSummary {
    #[serde(default)]
    title: String,
    #[serde(default)]
    summary: String,
}

const SUMMARY_SYSTEM: &str = "Summarize this conversation. Return ONLY JSON: {\"title\": \"short 3-6 word title\", \"summary\": \"2-3 sentence summary of what was discussed and decided\"}. No other text.";

/// LLM step: condense one conversation to (title, summary). No DB access.
pub async fn summarize_one(messages: &[Message], api_key: &str) -> (String, String) {
    let text = snippet(messages, 40);
    let raw = match haiku_json(api_key, SUMMARY_SYSTEM, &text, 400).await {
        Some(t) => t,
        None => return (String::new(), String::new()),
    };
    let s: RawSummary = serde_json::from_str(crate::core::llm::json_slice(&raw)).unwrap_or(RawSummary {
        title: String::new(),
        summary: String::new(),
    });
    (s.title, s.summary)
}

/// DB step: write a conversation summary row.
pub fn store_convo(conn: &Connection, c: &SummaryCandidate, proj_id: &str, title: &str, summary: &str) {
    let now = now_secs();
    let title = if title.trim().is_empty() {
        "Conversation"
    } else {
        title
    };
    let _ = conn.execute(
        "INSERT OR IGNORE INTO convo
         (id, proj_id, title, summary, message_count, started_at, ended_at, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            c.conv_id, proj_id, title, summary, c.msg_count, c.started, c.ended, now
        ],
    );
}

pub mod health;

use crate::core::types::Message;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub content: String,
    pub status: String,
    pub proj_id: String,
}

pub fn open(conn: &Connection, proj_id: &str, limit: usize) -> Result<Vec<Task>> {
    let mut stmt = conn.prepare(
        "SELECT id, content, status, proj_id FROM tasks
         WHERE proj_id = ?1 AND status = 'open'
         ORDER BY created_at DESC
         LIMIT ?2",
    )?;
    let rows = stmt.query_map(rusqlite::params![proj_id, limit as i64], |row| {
        Ok(Task {
            id: row.get(0)?,
            content: row.get(1)?,
            status: row.get(2)?,
            proj_id: row.get(3)?,
        })
    })?;
    rows.collect()
}

pub fn insert(conn: &Connection, proj_id: &str, content: &str, source_convo: &str) -> Result<()> {
    // Dedup: task extraction runs every turn, so skip if an identical open task
    // already exists (otherwise the same todo piles up turn after turn).
    let exists: bool = conn
        .query_row(
            "SELECT 1 FROM tasks
             WHERE proj_id = ?1 AND content = ?2 AND status = 'open' LIMIT 1",
            rusqlite::params![proj_id, content],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if exists {
        return Ok(());
    }
    let now = now_secs();
    conn.execute(
        "INSERT INTO tasks
         (id, proj_id, content, status, source_conversation, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'open', ?4, ?5, ?5)",
        rusqlite::params![
            Uuid::new_v4().to_string(),
            proj_id,
            content,
            source_convo,
            now
        ],
    )?;
    Ok(())
}

pub fn close(conn: &Connection, task_id: &str) -> Result<()> {
    let now = now_secs();
    conn.execute(
        "UPDATE tasks SET status = 'done', updated_at = ?1 WHERE id = ?2",
        rusqlite::params![now, task_id],
    )?;
    Ok(())
}

const TASKEXT_SYSTEM: &str = "You are a task extraction engine. \
Given conversation messages, extract any actionable tasks, goals, or todos mentioned by the user. \
Return a JSON array of strings, one task per entry. Return [] if no tasks found.";

/// Extract actionable tasks. Uses the Anthropic API if an API key is set, else
/// the Claude Code subscription.
pub async fn extract(messages: &[Message], api_key: &str) -> Vec<String> {
    if messages.is_empty() {
        return vec![];
    }
    let history = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");
    let text = match crate::core::llm::haiku(api_key, TASKEXT_SYSTEM, &history, 512).await {
        Some(t) => t,
        None => return vec![],
    };
    serde_json::from_str(crate::core::llm::json_slice(&text)).unwrap_or_default()
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

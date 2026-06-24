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

pub async fn extract(messages: &[Message], api_key: &str) -> Vec<String> {
    if api_key.is_empty() {
        return vec![];
    }
    let history = messages
        .iter()
        .map(|m| format!("{}: {}", m.role, m.content))
        .collect::<Vec<_>>()
        .join("\n");
    let body = serde_json::json!({
        "model": "claude-haiku-4-5-20251001",
        "max_tokens": 512,
        "system": TASKEXT_SYSTEM,
        "messages": [{"role": "user", "content": history}]
    });
    let client = reqwest::Client::new();
    let resp = match client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(_) => return vec![],
    };
    let json: serde_json::Value = match resp.json().await {
        Ok(j) => j,
        Err(_) => return vec![],
    };
    let text = json["content"][0]["text"].as_str().unwrap_or("");
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str(cleaned).unwrap_or_default()
}

fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

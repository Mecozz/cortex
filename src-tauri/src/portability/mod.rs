pub mod health;

use rusqlite::{params, types::ValueRef, Connection};
use serde::Deserialize;
use serde_json::{Map, Value};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

fn rows_as_json(conn: &Connection, sql: &str) -> Vec<Value> {
    let mut stmt = match conn.prepare(sql) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    let names: Vec<String> = stmt
        .column_names()
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    stmt.query_map([], |row| {
        let mut obj = Map::new();
        for (i, n) in names.iter().enumerate() {
            let v = match row.get_ref(i).unwrap_or(ValueRef::Null) {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(x) => Value::Number(x.into()),
                ValueRef::Real(f) => serde_json::Number::from_f64(f)
                    .map(Value::Number)
                    .unwrap_or(Value::Null),
                ValueRef::Text(s) => Value::String(String::from_utf8_lossy(s).into()),
                ValueRef::Blob(_) => Value::Null,
            };
            obj.insert(n.clone(), v);
        }
        Ok(Value::Object(obj))
    })
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn to_sql_val(v: &Value) -> rusqlite::types::Value {
    match v {
        Value::Null | Value::Array(_) | Value::Object(_) => rusqlite::types::Value::Null,
        Value::Bool(b) => rusqlite::types::Value::Integer(if *b { 1 } else { 0 }),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                rusqlite::types::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                rusqlite::types::Value::Real(f)
            } else {
                rusqlite::types::Value::Null
            }
        }
        Value::String(s) => rusqlite::types::Value::Text(s.clone()),
    }
}

fn import_table(conn: &Connection, table: &str, rows: &[Value]) -> usize {
    let mut count = 0;
    for row in rows {
        let Some(obj) = row.as_object() else {
            continue;
        };
        let cols: Vec<&str> = obj.keys().map(|s| s.as_str()).collect();
        let ph: Vec<String> = (1..=cols.len()).map(|i| format!("?{i}")).collect();
        let sql = format!(
            "INSERT OR IGNORE INTO {table} ({}) VALUES ({})",
            cols.join(", "),
            ph.join(", ")
        );
        let vals: Vec<rusqlite::types::Value> = obj.values().map(to_sql_val).collect();
        if conn.execute(&sql, rusqlite::params_from_iter(vals)).is_ok() {
            count += 1;
        }
    }
    count
}

pub fn export_json(conn: &Connection, data_dir: &Path) -> Result<String, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let data = serde_json::json!({
        "version": "1",
        "exported_at": now,
        "proj": rows_as_json(conn, "SELECT * FROM proj"),
        "facts": rows_as_json(conn, "SELECT * FROM facts"),
        "episodic": rows_as_json(conn, "SELECT * FROM episodic"),
        "tasks": rows_as_json(conn, "SELECT * FROM tasks"),
        "convo": rows_as_json(conn, "SELECT * FROM convo"),
        "rel": rows_as_json(conn, "SELECT * FROM rel"),
        "tools": rows_as_json(conn, "SELECT * FROM tools"),
    });
    let path = data_dir.join("cortex_export.json");
    let json = serde_json::to_string_pretty(&data).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into())
}

/// One memory from an external brain export (e.g. the Transformer brain).
#[derive(Deserialize)]
struct ExternalMemory {
    #[serde(default, rename = "type")]
    mem_type: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    source: String,
    #[serde(default)]
    created_at: i64,
    #[serde(default)]
    access_count: i64,
}

/// Import an external memories JSON array into the `facts` table so it's
/// recallable in chat. Maps semantic/procedural -> high confidence, episodic ->
/// archive tier. Title is folded into content for self-contained facts.
/// Generic IMPORT path — any tool can produce this shape; the Transformer
/// brain is the first source. Returns # imported.
pub fn import_memories(conn: &Connection, proj_id: &str, path: &str) -> Result<usize, String> {
    let raw = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mems: Vec<ExternalMemory> = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let _ = conn.execute_batch("BEGIN");
    // Self-healing: collapse duplicate-content facts within THIS project (e.g.
    // from a double-click import) down to one each. Scoped to proj_id so it can't
    // touch other projects, and inside the transaction so it rolls back on error.
    let _ = conn.execute(
        "DELETE FROM facts WHERE proj_id = ?1 AND rowid NOT IN
         (SELECT MIN(rowid) FROM facts WHERE proj_id = ?1 GROUP BY content)",
        params![proj_id],
    );

    let mut imported = 0;
    for m in mems {
        let content = if m.title.trim().is_empty() {
            m.content.clone()
        } else {
            format!("{} — {}", m.title.trim(), m.content)
        };
        if content.trim().is_empty() {
            continue;
        }
        // Idempotent: skip if this exact fact is already present.
        let exists: bool = conn
            .query_row(
                "SELECT 1 FROM facts WHERE content = ?1 LIMIT 1",
                params![content],
                |_| Ok(true),
            )
            .unwrap_or(false);
        if exists {
            continue;
        }
        let category = if m.mem_type.trim().is_empty() {
            "imported".to_string()
        } else {
            m.mem_type.clone()
        };
        let confidence: f64 = match m.mem_type.as_str() {
            "semantic" | "procedural" => 0.85,
            "episodic" => 0.5,
            _ => 0.6,
        };
        let importance = (0.5 + m.access_count as f64 * 0.01).min(1.0);
        let created = if m.created_at > 0 { m.created_at } else { now };
        if conn
            .execute(
                "INSERT INTO facts
                 (id, proj_id, content, category, is_current, valid_from,
                  last_confirmed_at, confidence_score, importance_score,
                  source_conversation, created_at)
                 VALUES (?1, ?2, ?3, ?4, 1, ?5, ?5, ?6, ?7, ?8, ?5)",
                params![
                    Uuid::new_v4().to_string(),
                    proj_id,
                    content,
                    category,
                    created,
                    confidence,
                    importance,
                    m.source,
                ],
            )
            .is_ok()
        {
            imported += 1;
        }
    }
    let _ = conn.execute_batch("COMMIT");
    Ok(imported)
}

pub fn import_json(conn: &Connection, path: &str) -> Result<usize, String> {
    let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let data: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    let tables = [
        "proj", "facts", "episodic", "tasks", "convo", "rel", "tools",
    ];
    // One transaction for the whole import — 45k+ autocommits would be glacial.
    let _ = conn.execute_batch("BEGIN");
    let mut total = 0;
    for table in &tables {
        if let Some(rows) = data[table].as_array() {
            total += import_table(conn, table, rows);
        }
    }
    let _ = conn.execute_batch("COMMIT");
    Ok(total)
}

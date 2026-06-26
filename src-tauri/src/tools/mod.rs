pub mod health;

use rhai::{Dynamic, Engine, Scope};
use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub id: String,
    pub name: String,
    pub description: String,
    pub code: String,
    pub version: String,
    pub is_active: bool,
    pub created_at: i64,
}

fn row_to_tool(row: &rusqlite::Row) -> rusqlite::Result<Tool> {
    Ok(Tool {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        code: row.get(3)?,
        version: row.get(4)?,
        is_active: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
    })
}

fn make_engine() -> Engine {
    let mut e = Engine::new();
    e.set_max_operations(100_000);
    e.set_max_string_size(10_000);
    e.set_max_array_size(1_000);
    e.set_max_map_size(1_000);
    e.disable_symbol("eval");
    e
}

pub fn validate(code: &str) -> Result<(), String> {
    make_engine()
        .compile(code)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

pub fn tcat(conn: &Connection, name: &str) -> Option<Tool> {
    conn.query_row(
        "SELECT id, name, description, code, version, is_active, created_at \
         FROM tools WHERE name = ?1 AND is_active = 1",
        params![name],
        row_to_tool,
    )
    .ok()
}

pub fn get(conn: &Connection, id: &str) -> Option<Tool> {
    conn.query_row(
        "SELECT id, name, description, code, version, is_active, created_at \
         FROM tools WHERE id = ?1",
        params![id],
        row_to_tool,
    )
    .ok()
}

pub fn list(conn: &Connection) -> Result<Vec<Tool>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, description, code, version, is_active, created_at \
         FROM tools ORDER BY created_at DESC",
    )?;
    let rows: Result<Vec<Tool>> = stmt.query_map([], row_to_tool)?.collect();
    rows
}

pub fn upsert(conn: &Connection, tool: &Tool) -> Result<()> {
    conn.execute(
        "INSERT INTO tools (id, name, description, code, version, is_active, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, unixepoch(), unixepoch())
         ON CONFLICT(name) DO UPDATE SET
           description = excluded.description,
           code        = excluded.code,
           version     = excluded.version,
           updated_at  = unixepoch()",
        params![
            tool.id,
            tool.name,
            tool.description,
            tool.code,
            tool.version,
            tool.is_active as i64
        ],
    )?;
    Ok(())
}

pub fn delete(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM tools WHERE id = ?1", params![id])?;
    Ok(())
}

pub fn run(code: &str, args: Vec<String>) -> Result<String, String> {
    let engine = make_engine();
    let mut scope = Scope::new();
    let rhai_args: rhai::Array = args.into_iter().map(Dynamic::from).collect();
    scope.push("args", rhai_args);
    engine
        .eval_with_scope::<Dynamic>(&mut scope, code)
        .map(|d| d.to_string())
        .map_err(|e| e.to_string())
}

const FORGE_SYSTEM: &str = "Write a Rhai script for the described task. \
Rhai is a Rust-embedded scripting language. Rules: \
(1) Input args are in an Array<String> called `args`. \
(2) The last evaluated expression is the return value — it must be a String. \
(3) Standard math, string, and array operations are available. No file I/O or network. \
Return ONLY the Rhai code. No markdown fences, no explanation.";

/// Generate a Rhai tool from a description. Uses the Anthropic API if a key is
/// set, else the Claude Code subscription (so Forge works on either auth).
pub async fn forge(description: &str, api_key: &str) -> Result<String, String> {
    let raw = crate::core::llm::generate(api_key, FORGE_SYSTEM, description, 1024)
        .await
        .ok_or("Tool generation failed (no API key, and Claude subscription unavailable)")?;
    // Strip any code fences the model may add despite instructions.
    let code = raw
        .trim()
        .trim_start_matches("```rhai")
        .trim_start_matches("```rust")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim()
        .to_string();
    validate(&code)?;
    Ok(code)
}

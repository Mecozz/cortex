//! Read-side Tauri commands: memory queries, tasks, and vault. Split out of
//! commands.rs to keep files under the 400-line limit. (Top-level file, so it's
//! exempt from the feature-module boundary rules and may import freely.)

use serde::Serialize;
use tauri::State;

use crate::commands::{DbState, VaultState};
use crate::{memory, tasks, vault};

// ── Memory query ──────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
pub struct FactResult {
    pub content: String,
    pub category: String,
    pub confidence: f32,
}

#[tauri::command]
pub fn get_facts(state: State<DbState>) -> Result<Vec<FactResult>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let proj_id = memory::default_project_id(&conn).map_err(|e| e.to_string())?;
    let facts = memory::pass1::retrieve(&conn, &proj_id, 50).map_err(|e| e.to_string())?;
    Ok(facts
        .into_iter()
        .map(|f| FactResult {
            content: f.content,
            category: f.category,
            confidence: f.confidence,
        })
        .collect())
}

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub role: String,
    pub content: String,
}

#[tauri::command]
pub fn search_memory(query: String, state: State<DbState>) -> Result<Vec<SearchResult>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let proj_id = memory::default_project_id(&conn).map_err(|e| e.to_string())?;
    let hits = memory::episodic::search(&conn, &proj_id, &query, 20).map_err(|e| e.to_string())?;
    Ok(hits
        .into_iter()
        .map(|(role, content)| SearchResult { role, content })
        .collect())
}

// ── Tasks ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_tasks(state: State<DbState>) -> Result<Vec<tasks::Task>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let proj_id = memory::default_project_id(&conn).map_err(|e| e.to_string())?;
    tasks::open(&conn, &proj_id, 50).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn close_task(task_id: String, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    tasks::close(&conn, &task_id).map_err(|e| e.to_string())
}

// ── Vault ─────────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_vault_keys(state: State<DbState>) -> Result<Vec<vault::VaultEntry>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    vault::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_vault_item(
    key: String,
    value: String,
    description: String,
    state: State<DbState>,
    vault_state: State<VaultState>,
) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let vk = vault_state.0.lock().map_err(|e| e.to_string())?;
    let desc = (!description.is_empty()).then_some(description.as_str());
    vault::set(&conn, &vk, &key, &value, desc).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_vault_item(key: String, state: State<DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    vault::delete(&conn, &key).map_err(|e| e.to_string())
}

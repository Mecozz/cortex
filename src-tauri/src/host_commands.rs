//! Host-level Tauri commands (brain status, backups, sync, tools, telegram,
//! import/export, oauth). Split out of lib.rs to keep files under the 400-line
//! limit. Top-level file, so it may import feature modules freely.

use tauri::State;

use crate::commands::{DbPath, DbState, VaultState, WatchState};
use crate::core::health::HealthCheck;
use crate::{
    backup, inject, librarian, memory, oauth, portability, providers, sync, tasks, telegram, tools,
    vault, watch, cost,
};

#[tauri::command]
pub fn get_brain_status(
    state: State<DbState>,
    watch_state: State<WatchState>,
    db_path: State<DbPath>,
) -> watch::BrainStatus {
    let cb = watch_state.0.lock().ok();

    // Live metrics for the data-driven modules.
    let (facts, episodic, has_key) = state
        .0
        .lock()
        .map(|conn| {
            let facts = conn
                .query_row("SELECT COUNT(*) FROM facts WHERE is_current = 1", [], |r| {
                    r.get::<_, i64>(0)
                })
                .unwrap_or(0);
            let episodic = conn
                .query_row("SELECT COUNT(*) FROM episodic", [], |r| r.get::<_, i64>(0))
                .unwrap_or(0);
            let has_key = !conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'api_key_anthropic'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_or_default()
                .is_empty();
            (facts, episodic, has_key)
        })
        .unwrap_or((0, 0, false));
    let backup_count = backup::list(db_path.0.parent().unwrap_or(&db_path.0)).len();

    let reports = vec![
        memory::health::MemoryHealth { facts, episodic }.health(),
        tasks::health::TaskHealth.health(),
        inject::health::InjectHealth.health(),
        cost::health::CostHealth.health(),
        librarian::health::LibrarianHealth.health(),
        vault::health::VaultHealth.health(),
        backup::health::BackupHealth {
            count: backup_count,
        }
        .health(),
        sync::health::SyncHealth.health(),
        tools::health::ToolsHealth.health(),
        telegram::health::TelegramHealth.health(),
        portability::health::PortabilityHealth.health(),
        watch::health::WatchHealth.health(),
        providers::health::ProvidersHealth {
            cloud_available: has_key,
            local_available: true,
        }
        .health(),
    ];
    let modules: Vec<watch::ModuleHealth> = reports
        .into_iter()
        .map(|r| {
            let failures = cb.as_ref().map(|c| c.failure_count(&r.module)).unwrap_or(0);
            let disabled = cb
                .as_ref()
                .map(|c| c.is_disabled(&r.module))
                .unwrap_or(false);
            // Overlay circuit-breaker state: a disabled module is red; one with
            // recent failures drops an otherwise-green module to yellow.
            let base = format!("{:?}", r.status).to_lowercase();
            let status = if disabled {
                "red".to_string()
            } else if failures > 0 && base == "green" {
                "yellow".to_string()
            } else {
                base
            };
            watch::ModuleHealth {
                module: r.module,
                status,
                message: r.message,
                failures,
                disabled,
            }
        })
        .collect();
    let overall = if modules.iter().any(|m| m.disabled || m.status == "red") {
        "red".into()
    } else if modules.iter().any(|m| m.status == "yellow") {
        "yellow".into()
    } else {
        "green".into()
    };
    watch::BrainStatus { overall, modules }
}

#[tauri::command]
pub async fn run_lib_now(db_path: State<'_, DbPath>) -> Result<(), String> {
    crate::run_lib_cycle(&db_path.0).await;
    Ok(())
}

#[tauri::command]
pub fn create_backup(
    name: Option<String>,
    state: State<'_, DbState>,
    db_path: State<'_, DbPath>,
) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    backup::create(&conn, data_dir, name.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_backups(db_path: State<'_, DbPath>) -> Vec<backup::BackupEntry> {
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    backup::list(data_dir)
}

#[tauri::command]
pub fn restore_backup(filename: String, db_path: State<'_, DbPath>) -> Result<(), String> {
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    backup::queue_restore(data_dir, &filename).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_backup(filename: String, db_path: State<'_, DbPath>) -> Result<(), String> {
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    backup::delete(data_dir, &filename).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_level(
    level: u8,
    state: State<'_, DbState>,
    db_path: State<'_, DbPath>,
) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    if level == 1 {
        let backups = backup::list(data_dir);
        let latest = backups.first().ok_or("No backups found for rollback")?;
        backup::queue_restore(data_dir, &latest.filename).map_err(|e| e.to_string())?;
        return Ok("restart_required".into());
    }
    backup::create(&conn, data_dir, Some("pre_reset")).map_err(|e| e.to_string())?;
    backup::reset(&conn, level).map_err(|e| e.to_string())?;
    Ok("done".into())
}

#[tauri::command]
pub fn sync_export(sync_folder: String, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    sync::export(&conn, &sync_folder).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sync_status(sync_folder: String, db_path: State<'_, DbPath>) -> sync::SyncStatus {
    sync::status(&db_path.0, &sync_folder)
}

#[tauri::command]
pub fn sync_import(sync_folder: String, db_path: State<'_, DbPath>) -> Result<(), String> {
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    sync::queue_import(data_dir, &sync_folder).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_update() -> Result<String, String> {
    let client = reqwest::Client::builder()
        .user_agent("cortex-app/0.1")
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get("https://api.github.com/repos/Mecozz/cortex/releases/latest")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let json: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(json["tag_name"].as_str().unwrap_or("").to_string())
}

#[tauri::command]
pub fn list_tools(state: State<'_, DbState>) -> Result<Vec<tools::Tool>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    tools::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_tool(mut tool: tools::Tool, state: State<'_, DbState>) -> Result<(), String> {
    if tool.id.is_empty() {
        tool.id = uuid::Uuid::new_v4().to_string();
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    tools::upsert(&conn, &tool).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_tool(id: String, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    tools::delete(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn run_tool(id: String, args: Vec<String>, state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let tool = tools::get(&conn, &id).ok_or("Tool not found")?;
    tools::run(&tool.code, args)
}

#[tauri::command]
pub async fn forge_tool(description: String, state: State<'_, DbState>) -> Result<String, String> {
    let api_key = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        conn.query_row(
            "SELECT value FROM settings WHERE key = 'api_key_anthropic'",
            [],
            |r| r.get::<_, String>(0),
        )
        .unwrap_or_default()
    };
    tools::forge(&description, &api_key).await
}

#[tauri::command]
pub async fn telegram_send(
    message: String,
    state: State<'_, DbState>,
    vault_state: State<'_, VaultState>,
) -> Result<(), String> {
    let (bot_token, chat_id) = {
        let conn = state.0.lock().map_err(|e| e.to_string())?;
        let vk = vault_state.0.lock().map_err(|e| e.to_string())?;
        let token = vault::get(&conn, &vk, "telegram_bot_token").unwrap_or_default();
        let chat = vault::get(&conn, &vk, "telegram_chat_id").unwrap_or_default();
        (token, chat)
    };
    telegram::notify(&message, &bot_token, &chat_id).await
}

#[tauri::command]
pub fn export_data(state: State<'_, DbState>, db_path: State<'_, DbPath>) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    portability::export_json(&conn, data_dir)
}

#[tauri::command]
pub fn import_data(path: String, state: State<'_, DbState>) -> Result<usize, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    portability::import_json(&conn, &path)
}

/// Import an external memories JSON (e.g. a Transformer-brain export) into facts.
#[tauri::command]
pub fn import_memories(path: String, state: State<'_, DbState>) -> Result<usize, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let proj_id = memory::default_project_id(&conn).map_err(|e| e.to_string())?;
    portability::import_memories(&conn, &proj_id, &path)
}

#[tauri::command]
pub fn read_claude_credentials() -> Result<String, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "Cannot find home directory".to_string())?;
    let path = std::path::Path::new(&home)
        .join(".claude")
        .join(".credentials.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|_| "Claude Code not found. Install Claude Code and sign in first.".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    v["claudeAiOauth"]["accessToken"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No access token found in Claude Code credentials".to_string())
}

#[tauri::command]
pub async fn oauth_login(app: tauri::AppHandle) -> Result<String, String> {
    oauth::begin_oauth(&app).await
}

pub mod backup;
pub mod commands;
pub mod core;
pub mod cost;
pub mod db;
pub mod inject;
pub mod librarian;
pub mod memory;
pub mod portability;
pub mod providers;
pub mod sync;
pub mod tasks;
pub mod telegram;
pub mod tools;
pub mod vault;
pub mod watch;

use crate::core::health::HealthCheck;
use commands::{DbPath, DbState, VaultState, WatchState};
use std::sync::Mutex;
use tauri::{Manager, State};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("cortex.db");
            backup::check_pending_restore(&data_dir, &db_path);
            let conn = db::init(&db_path).map_err(|e| e.to_string())?;
            let vault_key =
                vault::VaultKey::load_or_create(&data_dir).map_err(|e| e.to_string())?;
            app.manage(DbState(Mutex::new(conn)));
            app.manage(WatchState(Mutex::new(watch::CircuitBreaker::default())));
            app.manage(DbPath(db_path.clone()));
            app.manage(VaultState(Mutex::new(vault_key)));

            let lib_db = db_path.clone();
            tauri::async_runtime::spawn(async move {
                lib_background_task(lib_db).await;
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::chat_message,
            commands::remember_turn,
            commands::get_facts,
            commands::search_memory,
            commands::get_tasks,
            commands::close_task,
            commands::get_vault_keys,
            commands::set_vault_item,
            commands::delete_vault_item,
            get_brain_status,
            run_lib_now,
            create_backup,
            list_backups,
            restore_backup,
            delete_backup,
            reset_level,
            sync_export,
            sync_status,
            sync_import,
            check_update,
            list_tools,
            save_tool,
            delete_tool,
            run_tool,
            forge_tool,
            telegram_send,
            export_data,
            read_claude_credentials,
            import_data,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn lib_background_task(db_path: std::path::PathBuf) {
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
    loop {
        interval.tick().await;
        run_lib_cycle(&db_path).await;
    }
}

pub async fn run_lib_cycle(db_path: &std::path::Path) {
    let conn = match db::init(db_path) {
        Ok(c) => c,
        Err(_) => return,
    };
    let _ = memory::scores::flush_expired(&conn);
    let api_key = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'api_key_anthropic'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_default();
    if api_key.is_empty() {
        return;
    }
    let proj_id = match memory::default_project_id(&conn) {
        Ok(id) => id,
        Err(_) => return,
    };
    let last_run: i64 = conn
        .query_row(
            "SELECT value FROM settings WHERE key = 'lib_last_run'",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_default()
        .parse()
        .unwrap_or(0);
    let msgs = memory::episodic::since(&conn, &proj_id, last_run, 100).unwrap_or_default();
    if !msgs.is_empty() {
        let facts = memory::instcap::extract(&msgs, &api_key).await;
        for fact in facts {
            let _ = memory::conf::upsert(
                &conn,
                &memory::conf::Fact {
                    content: fact.content,
                    category: fact.category,
                    confidence: fact.confidence,
                    proj_id: proj_id.clone(),
                },
            );
        }
    }
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string();
    let _ = conn.execute(
        "INSERT INTO settings (key, value) VALUES ('lib_last_run', ?1)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = unixepoch()",
        rusqlite::params![now],
    );
}

#[tauri::command]
fn get_brain_status(state: State<DbState>, watch_state: State<WatchState>) -> watch::BrainStatus {
    let cb = watch_state.0.lock().ok();
    let mut reports = vec![
        memory::health::MemoryHealth.health(),
        tasks::health::TaskHealth.health(),
        inject::health::InjectHealth.health(),
        cost::health::CostHealth.health(),
        librarian::health::LibrarianHealth.health(),
        vault::health::VaultHealth.health(),
        backup::health::BackupHealth.health(),
        sync::health::SyncHealth.health(),
        tools::health::ToolsHealth.health(),
        telegram::health::TelegramHealth.health(),
        portability::health::PortabilityHealth.health(),
        portability::health::PortabilityHealth.health(),
        watch::health::WatchHealth.health(),
    ];
    let has_key = state
        .0
        .lock()
        .map(|conn| {
            !conn
                .query_row(
                    "SELECT value FROM settings WHERE key = 'api_key_anthropic'",
                    [],
                    |r| r.get::<_, String>(0),
                )
                .unwrap_or_default()
                .is_empty()
        })
        .unwrap_or(false);
    reports.push(
        providers::health::ProvidersHealth {
            cloud_available: has_key,
            local_available: true,
        }
        .health(),
    );
    let modules: Vec<watch::ModuleHealth> = reports
        .into_iter()
        .map(|r| {
            let failures = cb.as_ref().map(|c| c.failure_count(&r.module)).unwrap_or(0);
            let disabled = cb
                .as_ref()
                .map(|c| c.is_disabled(&r.module))
                .unwrap_or(false);
            watch::ModuleHealth {
                module: r.module,
                status: format!("{:?}", r.status).to_lowercase(),
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
async fn run_lib_now(db_path: State<'_, DbPath>) -> Result<(), String> {
    run_lib_cycle(&db_path.0).await;
    Ok(())
}

#[tauri::command]
fn create_backup(
    name: Option<String>,
    state: State<'_, DbState>,
    db_path: State<'_, DbPath>,
) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    backup::create(&conn, data_dir, name.as_deref()).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_backups(db_path: State<'_, DbPath>) -> Vec<backup::BackupEntry> {
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    backup::list(data_dir)
}

#[tauri::command]
fn restore_backup(filename: String, db_path: State<'_, DbPath>) -> Result<(), String> {
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    backup::queue_restore(data_dir, &filename).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_backup(filename: String, db_path: State<'_, DbPath>) -> Result<(), String> {
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    backup::delete(data_dir, &filename).map_err(|e| e.to_string())
}

#[tauri::command]
fn reset_level(
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
fn sync_export(sync_folder: String, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    sync::export(&conn, &sync_folder).map_err(|e| e.to_string())
}

#[tauri::command]
fn sync_status(sync_folder: String, db_path: State<'_, DbPath>) -> sync::SyncStatus {
    sync::status(&db_path.0, &sync_folder)
}

#[tauri::command]
fn sync_import(sync_folder: String, db_path: State<'_, DbPath>) -> Result<(), String> {
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    sync::queue_import(data_dir, &sync_folder).map_err(|e| e.to_string())
}

#[tauri::command]
async fn check_update() -> Result<String, String> {
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
fn list_tools(state: State<'_, DbState>) -> Result<Vec<tools::Tool>, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    tools::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_tool(mut tool: tools::Tool, state: State<'_, DbState>) -> Result<(), String> {
    if tool.id.is_empty() {
        tool.id = uuid::Uuid::new_v4().to_string();
    }
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    tools::upsert(&conn, &tool).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_tool(id: String, state: State<'_, DbState>) -> Result<(), String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    tools::delete(&conn, &id).map_err(|e| e.to_string())
}

#[tauri::command]
fn run_tool(id: String, args: Vec<String>, state: State<'_, DbState>) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let tool = tools::get(&conn, &id).ok_or("Tool not found")?;
    tools::run(&tool.code, args)
}

#[tauri::command]
async fn forge_tool(description: String, state: State<'_, DbState>) -> Result<String, String> {
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
async fn telegram_send(
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
fn export_data(state: State<'_, DbState>, db_path: State<'_, DbPath>) -> Result<String, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    let data_dir = db_path.0.parent().unwrap_or(&db_path.0);
    portability::export_json(&conn, data_dir)
}

#[tauri::command]
fn import_data(path: String, state: State<'_, DbState>) -> Result<usize, String> {
    let conn = state.0.lock().map_err(|e| e.to_string())?;
    portability::import_json(&conn, &path)
}

#[tauri::command]
fn read_claude_credentials() -> Result<String, String> {
    let home = std::env::var("USERPROFILE")
        .or_else(|_| std::env::var("HOME"))
        .map_err(|_| "Cannot find home directory".to_string())?;
    let path = std::path::Path::new(&home).join(".claude").join(".credentials.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|_| "Claude Code not found. Install Claude Code and sign in first.".to_string())?;
    let v: serde_json::Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
    v["claudeAiOauth"]["accessToken"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "No access token found in Claude Code credentials".to_string())
}

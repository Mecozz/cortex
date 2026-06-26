pub mod backup;
pub mod commands;
pub mod core;
pub mod cost;
pub mod db;
pub mod host_commands;
pub mod inject;
pub mod librarian;
pub mod memory;
pub mod oauth;
pub mod portability;
pub mod providers;
pub mod query_commands;
pub mod sync;
pub mod tasks;
pub mod telegram;
pub mod tools;
pub mod vault;
pub mod watch;

use commands::{DbPath, DbState, VaultState, WatchState};
use std::sync::Mutex;
use tauri::Manager;

#[tauri::command]
fn clear_claude_session(s: tauri::State<'_, commands::ClaudeSessionState>) -> Result<(), String> {
    *s.0.lock().map_err(|e| e.to_string())? = None;
    Ok(())
}

/// Kill the in-flight `claude` subprocess (and its child tree). No-op if idle.
#[tauri::command]
fn stop_chat(abort: tauri::State<'_, commands::AbortState>) -> Result<(), String> {
    let pid = abort.0.lock().map_err(|e| e.to_string())?.take();
    if let Some(pid) = pid {
        #[cfg(target_os = "windows")]
        let _ = std::process::Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .output();
        #[cfg(not(target_os = "windows"))]
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
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
            app.manage(commands::ClaudeSessionState(Mutex::new(None)));
            app.manage(commands::AbortState(Mutex::new(None)));
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
            query_commands::get_facts,
            query_commands::search_memory,
            query_commands::get_tasks,
            query_commands::close_task,
            query_commands::get_vault_keys,
            query_commands::set_vault_item,
            query_commands::delete_vault_item,
            host_commands::get_brain_status,
            host_commands::run_lib_now,
            host_commands::create_backup,
            host_commands::list_backups,
            host_commands::restore_backup,
            host_commands::delete_backup,
            host_commands::reset_level,
            host_commands::sync_export,
            host_commands::sync_status,
            host_commands::sync_import,
            host_commands::check_update,
            host_commands::list_tools,
            host_commands::save_tool,
            host_commands::delete_tool,
            host_commands::run_tool,
            host_commands::forge_tool,
            host_commands::telegram_send,
            host_commands::export_data,
            host_commands::oauth_login,
            host_commands::read_claude_credentials,
            host_commands::import_data,
            host_commands::import_memories,
            clear_claude_session,
            stop_chat,
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
    // No early-return on empty api_key: the capture/consolidation helpers fall
    // back to the Claude Code subscription, so the cycle runs on either auth.
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

    // REM cycle: consolidate episodic chatter into durable knowledge.
    // (LLM steps take plain data; DB steps are sync — keeps this future Send.)
    let triples = librarian::consolidation::extract_relationship_triples(&msgs, &api_key).await;
    librarian::consolidation::store_relationships(&conn, &triples, "lib_cycle");
    librarian::consolidation::decay_facts(&conn);
    for cand in librarian::consolidation::conversations_needing_summary(&conn, &proj_id, 5) {
        let (title, summary) = librarian::consolidation::summarize_one(&cand.messages, &api_key).await;
        librarian::consolidation::store_convo(&conn, &cand, &proj_id, &title, &summary);
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

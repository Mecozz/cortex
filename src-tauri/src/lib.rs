pub mod commands;
pub mod core;
pub mod cost;
pub mod db;
pub mod inject;
pub mod librarian;
pub mod memory;
pub mod providers;
pub mod tasks;
pub mod watch;

use commands::{DbPath, DbState, WatchState};
use std::sync::Mutex;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let db_path = data_dir.join("cortex.db");
            let conn = db::init(&db_path).map_err(|e| e.to_string())?;
            app.manage(DbState(Mutex::new(conn)));
            app.manage(WatchState(Mutex::new(watch::CircuitBreaker::default())));
            app.manage(DbPath(db_path.clone()));

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
            commands::get_brain_status,
            commands::run_lib_now,
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

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

use crate::core::health::HealthCheck;
use commands::{DbPath, DbState, WatchState};
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
            get_brain_status,
            run_lib_now,
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

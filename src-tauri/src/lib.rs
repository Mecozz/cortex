pub mod commands;
pub mod core;
pub mod cost;
pub mod db;
pub mod inject;
pub mod memory;
pub mod providers;

use commands::DbState;
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
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::chat_message,
            commands::remember_turn,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

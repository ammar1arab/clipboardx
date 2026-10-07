mod commands;
mod database;
mod error;

use database::History;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let directory = app.path().app_data_dir()?;
            std::fs::create_dir_all(&directory)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))?;
            }
            let history =
                tauri::async_runtime::block_on(History::open(&directory.join("history.sqlite")))?;
            app.manage(history);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_history,
            commands::capture_text,
            commands::edit_text,
            commands::delete_history,
            commands::clear_history,
            commands::set_pinned,
            commands::get_retention,
            commands::set_retention
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

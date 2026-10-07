mod clipboard;
mod commands;
mod database;
mod error;

use database::History;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_autostart::Builder::new().build())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
            app.manage(clipboard::CaptureMonitor::start(app.handle().clone())?);
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
            commands::set_retention,
            commands::get_capture_status,
            commands::retry_capture,
            commands::hide_window
        ])
        .build(tauri::generate_context!());
    match app {
        Ok(app) => app.run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                handle.state::<clipboard::CaptureMonitor>().stop();
            }
        }),
        Err(_) => {
            eprintln!("ClipboardX could not start. Check app-data access and desktop support.")
        }
    }
}

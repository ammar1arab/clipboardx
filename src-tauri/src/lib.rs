mod clipboard;
mod commands;
mod database;
mod error;
mod images;
mod platform;

use database::History;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    std::env::set_var("GTK_OVERLAY_SCROLLING", "0");
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, args, _| {
            if !args.iter().any(|arg| arg == "--background") {
                if let Err(error) = platform::toggle(app) {
                    use tauri::Emitter;
                    let _ = app.emit("native-error", error);
                    eprintln!("ClipboardX could not toggle its window.");
                }
            }
        }))
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--background"])
                .build(),
        )
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
            let settings = tauri::async_runtime::block_on(history.settings())
                .map_err(|error| std::io::Error::other(error.message))?;
            let frame = platform::WindowFrame::new(settings.window_width, settings.window_height);
            app.manage(history);
            app.manage(frame);
            #[cfg(target_os = "linux")]
            app.manage(platform::cosmic::FocusTracker::start()?);
            let clipboard = clipboard::writer::NativeClipboard::default();
            app.manage(clipboard.own_writes());
            app.manage(clipboard);
            app.manage(clipboard::CaptureMonitor::start(app.handle().clone())?);
            #[cfg(not(target_os = "linux"))]
            {
                use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
                app.global_shortcut()
                    .on_shortcut("Super+V", |app, _, event| {
                        if event.state == ShortcutState::Pressed {
                            let _ = platform::toggle(app);
                        }
                    })?;
            }
            if let Some(window) = app.get_webview_window("main") {
                if !std::env::args().any(|arg| arg == "--background") {
                    platform::apply_saved_size(app.handle(), &window);
                    window.show()?;
                    window.set_focus()?;
                }
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
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
            commands::hide_window,
            commands::stage_selection,
            commands::get_settings,
            commands::set_settings,
            commands::quit_app,
            commands::paste_entries
        ])
        .build(tauri::generate_context!());
    match app {
        Ok(app) => app.run(|handle, event| {
            if let tauri::RunEvent::Exit = event {
                handle.state::<clipboard::CaptureMonitor>().stop();
                handle.state::<clipboard::writer::NativeClipboard>().close();
                #[cfg(target_os = "linux")]
                handle.state::<platform::cosmic::FocusTracker>().stop();
            }
        }),
        Err(_) => {
            eprintln!("ClipboardX could not start. Check app-data access and desktop support.")
        }
    }
}

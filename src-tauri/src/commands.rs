use crate::database::{History, HistoryItem, Retention, Settings};
use crate::error::AppError;

#[tauri::command]
pub async fn list_history(
    history: tauri::State<'_, History>,
) -> Result<Vec<HistoryItem>, AppError> {
    history.list().await
}

#[tauri::command]
pub async fn capture_text(
    text: String,
    history: tauri::State<'_, History>,
) -> Result<(), AppError> {
    history.capture_text(&text).await
}

#[tauri::command]
pub async fn edit_text(
    id: String,
    text: String,
    history: tauri::State<'_, History>,
) -> Result<(), AppError> {
    history.edit_text(&id, &text).await
}

#[tauri::command]
pub async fn delete_history(
    ids: Vec<String>,
    history: tauri::State<'_, History>,
) -> Result<(), AppError> {
    history.delete(&ids).await
}

#[tauri::command]
pub async fn set_pinned(
    id: String,
    pinned: bool,
    history: tauri::State<'_, History>,
) -> Result<(), AppError> {
    history.set_pinned(&id, pinned).await
}

#[tauri::command]
pub async fn get_retention(history: tauri::State<'_, History>) -> Result<Retention, AppError> {
    history.retention().await
}

#[tauri::command]
pub async fn set_retention(
    retention: Retention,
    history: tauri::State<'_, History>,
) -> Result<(), AppError> {
    history.set_retention(retention).await
}

#[tauri::command]
pub async fn clear_history(
    include_pinned: bool,
    history: tauri::State<'_, History>,
) -> Result<(), AppError> {
    history.clear(include_pinned).await
}

#[tauri::command]
pub fn get_capture_status(
    monitor: tauri::State<'_, crate::clipboard::CaptureMonitor>,
) -> Result<crate::clipboard::CaptureStatus, AppError> {
    monitor.status()
}

#[tauri::command]
pub fn retry_capture(monitor: tauri::State<'_, crate::clipboard::CaptureMonitor>) {
    monitor.retry();
}

#[tauri::command]
pub fn hide_window(window: tauri::WebviewWindow) -> Result<(), AppError> {
    window.hide().map_err(|_| AppError {
        code: "window_error",
        message: "Could not hide ClipboardX. Try again.".into(),
    })
}

#[tauri::command]
pub async fn stage_selection(
    ids: Vec<String>,
    history: tauri::State<'_, History>,
    clipboard: tauri::State<'_, crate::clipboard::writer::NativeClipboard>,
) -> Result<(), AppError> {
    clipboard.write_selection(&history, &ids).await
}

#[tauri::command]
pub async fn paste_entries(
    ids: Vec<String>,
    window: tauri::WebviewWindow,
    history: tauri::State<'_, History>,
    clipboard: tauri::State<'_, crate::clipboard::writer::NativeClipboard>,
    app: tauri::AppHandle,
) -> Result<(), AppError> {
    let _operation = clipboard.operation.lock().await;
    clipboard.write_selection_unlocked(&history, &ids).await?;
    history.mark_used(&ids).await?;
    #[cfg(target_os = "linux")]
    {
        use tauri::Manager;
        crate::platform::paste(
            &window,
            &app.state::<crate::platform::cosmic::FocusTracker>(),
        )
        .await
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = (window, app);
        Err(AppError::from(
            "Native paste is currently implemented for COSMIC Wayland.",
        ))
    }
}

#[tauri::command]
pub async fn get_settings(history: tauri::State<'_, History>) -> Result<Settings, AppError> {
    history.settings().await
}

#[tauri::command]
pub async fn set_settings(
    settings: Settings,
    history: tauri::State<'_, History>,
    frame: tauri::State<'_, crate::platform::WindowFrame>,
) -> Result<(), AppError> {
    let width = settings.window_width;
    let height = settings.window_height;
    history.set_settings(settings).await?;
    frame.set(width, height);
    Ok(())
}

#[tauri::command]
pub fn quit_app(app: tauri::AppHandle) {
    app.exit(0);
}

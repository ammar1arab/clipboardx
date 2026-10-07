use crate::database::{History, HistoryItem, Retention};
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

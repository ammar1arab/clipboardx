use crate::error::AppError;
use std::sync::{Arc, Mutex};

#[derive(Clone, Default)]
pub struct NativeClipboard {
    state: Arc<Mutex<Option<arboard::Clipboard>>>,
    pub operation: Arc<tauri::async_runtime::Mutex<()>>,
}

impl NativeClipboard {
    pub async fn write_text(&self, text: String) -> Result<(), AppError> {
        let _operation = self.operation.lock().await;
        self.write_unlocked(text).await
    }

    pub async fn write_unlocked(&self, text: String) -> Result<(), AppError> {
        let state = self.state.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let mut clipboard = state.lock().map_err(|_| AppError {
                code: "clipboard_error",
                message: "Clipboard unavailable. Restart ClipboardX.".into(),
            })?;
            if clipboard.is_none() {
                *clipboard = Some(arboard::Clipboard::new().map_err(|_| {
                    AppError {
                        code: "clipboard_error",
                        message:
                            "Cannot access the system clipboard. Check desktop clipboard support."
                                .into(),
                    }
                })?);
            }
            let clipboard = clipboard
                .as_mut()
                .ok_or_else(|| AppError::from("Clipboard unavailable"))?;
            #[cfg(target_os = "linux")]
            let result = {
                use arboard::SetExtLinux;
                clipboard.set().exclude_from_history().text(text)
            };
            #[cfg(not(target_os = "linux"))]
            let result = clipboard.set_text(text);
            result.map_err(|_| AppError {
                code: "clipboard_error",
                message: "Could not write clipboard contents. Try again.".into(),
            })
        })
        .await
        .map_err(|_| AppError::from("Clipboard write task stopped"))?
    }

    pub fn close(&self) {
        if let Ok(mut clipboard) = self.state.lock() {
            clipboard.take();
        }
    }
}

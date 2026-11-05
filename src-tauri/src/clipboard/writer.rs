use crate::error::AppError;
use std::sync::{Arc, Mutex};

enum Contents {
    Text(String),
    Image(arboard::ImageData<'static>),
}

#[derive(Clone, Default)]
pub struct NativeClipboard {
    state: Arc<Mutex<Option<arboard::Clipboard>>>,
    pub operation: Arc<tauri::async_runtime::Mutex<()>>,
    own_writes: super::OwnWrites,
}

fn note_write(writes: &super::OwnWrites, contents: &Contents) {
    match contents {
        Contents::Text(text) => writes.note_text(text),
        Contents::Image(image) => {
            let (Ok(width), Ok(height)) = (u32::try_from(image.width), u32::try_from(image.height))
            else {
                return;
            };
            writes.note_image(&crate::images::rgba_hash(
                width,
                height,
                image.bytes.as_ref(),
            ));
        }
    }
}

impl NativeClipboard {
    pub fn own_writes(&self) -> super::OwnWrites {
        self.own_writes.clone()
    }

    pub async fn write_selection(
        &self,
        history: &crate::database::History,
        ids: &[String],
    ) -> Result<(), AppError> {
        let _operation = self.operation.lock().await;
        self.write_selection_unlocked(history, ids).await
    }

    pub async fn write_selection_unlocked(
        &self,
        history: &crate::database::History,
        ids: &[String],
    ) -> Result<(), AppError> {
        let contents = if let Some(image) = history.selected_image(ids).await? {
            Contents::Image(image)
        } else {
            Contents::Text(
                history
                    .selected_text(ids, &history.settings().await?.paste_separator)
                    .await?,
            )
        };
        self.write_unlocked(contents).await
    }

    async fn write_unlocked(&self, contents: Contents) -> Result<(), AppError> {
        let state = self.state.clone();
        let own_writes = self.own_writes.clone();
        tauri::async_runtime::spawn_blocking(move || {
            note_write(&own_writes, &contents);
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
                match contents {
                    Contents::Text(text) => clipboard.set().exclude_from_history().text(text),
                    Contents::Image(image) => clipboard.set().exclude_from_history().image(image),
                }
            };
            #[cfg(not(target_os = "linux"))]
            let result = match contents {
                Contents::Text(text) => clipboard.set_text(text),
                Contents::Image(image) => clipboard.set_image(image),
            };
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

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires live Wayland; replaces clipboard with a synthetic image and text"]
    fn live_native_selection_writes_images_and_text() {
        tauri::async_runtime::block_on(async {
            let root =
                std::env::temp_dir().join(format!("clipboardx-native-{}", uuid::Uuid::new_v4()));
            std::fs::create_dir(&root).unwrap();
            let history = crate::database::History::open(&root.join("history.sqlite"))
                .await
                .unwrap();
            let mut bytes = std::io::Cursor::new(Vec::new());
            image::DynamicImage::new_rgba8(12, 8)
                .write_to(&mut bytes, image::ImageFormat::Png)
                .unwrap();
            history.capture_image(bytes.into_inner()).await.unwrap();
            let writer = NativeClipboard::default();
            let image_id = history.list().await.unwrap()[0].id.clone();
            writer.write_selection(&history, &[image_id]).await.unwrap();
            let mut reader = arboard::Clipboard::new().unwrap();
            let image = reader.get_image().unwrap();
            assert_eq!((image.width, image.height), (12, 8));
            history
                .capture_text("ClipboardX synthetic native test")
                .await
                .unwrap();
            let text_id = history.list().await.unwrap()[0].id.clone();
            writer.write_selection(&history, &[text_id]).await.unwrap();
            assert!(reader.get_text().unwrap() == "ClipboardX synthetic native test");
            writer.close();
            drop(reader);
            drop(history);
            std::fs::remove_dir_all(root).unwrap();
        });
    }
}

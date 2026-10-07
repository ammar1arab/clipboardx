use super::{database_error, validate_id, History};
use crate::error::AppError;
use sqlx::Row;

impl History {
    pub async fn capture_image(&self, bytes: Vec<u8>) -> Result<(), AppError> {
        let store = self
            .images
            .as_ref()
            .ok_or_else(|| AppError::from("Image storage unavailable"))?
            .clone();
        let _operation = store.operation.lock().await;
        let worker = store.clone();
        let image = tauri::async_runtime::spawn_blocking(move || worker.store(&bytes))
            .await
            .map_err(|_| AppError::from("Image task stopped"))??;
        let mut transaction = self.pool.begin().await.map_err(database_error)?;
        sqlx::query("INSERT INTO clipboard_items (id, kind, image_path, thumbnail_path, width, height, hash, created_at, captured_at)
            VALUES (?, 'image', ?, ?, ?, ?, ?, ?, MAX(?, COALESCE((SELECT MAX(captured_at) + 1 FROM clipboard_items), 0)))
            ON CONFLICT(kind, hash) DO UPDATE SET captured_at = excluded.captured_at")
            .bind(uuid::Uuid::new_v4().to_string()).bind(image.path).bind(image.thumbnail).bind(image.width).bind(image.height).bind(image.hash)
            .bind(chrono::Utc::now().timestamp_millis()).bind(chrono::Utc::now().timestamp_millis()).execute(&mut *transaction).await.map_err(database_error)?;
        Self::prune(&mut transaction)
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        self.cleanup_images_unlocked(&store).await
    }

    async fn cleanup_images_unlocked(
        &self,
        store: &std::sync::Arc<crate::images::ImageStore>,
    ) -> Result<(), AppError> {
        let rows = sqlx::query(
            "SELECT image_path, thumbnail_path FROM clipboard_items WHERE kind = 'image'",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(database_error)?;
        let referenced = rows
            .iter()
            .flat_map(|row| {
                [
                    row.get::<String, _>("image_path"),
                    row.get::<String, _>("thumbnail_path"),
                ]
            })
            .collect();
        let store = store.clone();
        tauri::async_runtime::spawn_blocking(move || store.cleanup(&referenced))
            .await
            .map_err(|_| AppError::from("Image cleanup task stopped"))?
    }

    pub async fn cleanup_images(&self) -> Result<(), AppError> {
        if let Some(store) = &self.images {
            let _operation = store.operation.lock().await;
            self.cleanup_images_unlocked(store).await?;
        }
        Ok(())
    }

    pub async fn selected_image(
        &self,
        ids: &[String],
    ) -> Result<Option<arboard::ImageData<'static>>, AppError> {
        if ids.len() != 1 {
            return Ok(None);
        }
        validate_id(&ids[0])?;
        let path: Option<String> = sqlx::query_scalar(
            "SELECT image_path FROM clipboard_items WHERE id = ? AND kind = 'image'",
        )
        .bind(&ids[0])
        .fetch_optional(&self.pool)
        .await
        .map_err(database_error)?;
        let Some(path) = path else {
            return Ok(None);
        };
        let store = self
            .images
            .as_ref()
            .ok_or_else(|| AppError::from("Image storage unavailable"))?
            .clone();
        let _operation = store.operation.lock().await;
        let worker = store.clone();
        tauri::async_runtime::spawn_blocking(move || worker.read(&path))
            .await
            .map_err(|_| AppError::from("Image read task stopped"))?
            .map(Some)
    }
}

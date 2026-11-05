use crate::error::AppError;
mod images;
mod migrations;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{sqlite::SqliteConnectOptions, Row, Sqlite, SqlitePool, Transaction};
use std::path::Path;

const MAX_TEXT_BYTES: usize = 1024 * 1024;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Retention {
    pub max_items: u32,
    pub max_age_days: u32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Settings {
    pub capture_images: bool,
    pub paste_separator: String,
    pub window_width: u32,
    pub window_height: u32,
    pub stage_selection: bool,
    pub show_shortcut_bar: bool,
}

fn validate_text(text: &str) -> Result<(), AppError> {
    if text.is_empty() || text.len() > MAX_TEXT_BYTES || text.contains('\0') {
        return Err("Clipboard text must contain 1–1048576 bytes and no NUL characters".into());
    }
    Ok(())
}

fn validate_id(id: &str) -> Result<(), AppError> {
    uuid::Uuid::parse_str(id)
        .map(|_| ())
        .map_err(|_| "Invalid history item ID".into())
}

fn database_error(_: sqlx::Error) -> AppError {
    AppError::storage("Could not update clipboard history")
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryItem {
    pub id: String,
    pub kind: String,
    pub text: Option<String>,
    pub hash: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub image_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub pinned: bool,
    pub captured_at: i64,
}

pub struct History {
    pool: SqlitePool,
    images: Option<std::sync::Arc<crate::images::ImageStore>>,
}

impl History {
    pub async fn open(path: &Path) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true);
        let pool = sqlx::sqlite::SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        let mut history = Self::initialize(pool).await?;
        let root = path.with_extension("images");
        history.images = Some(std::sync::Arc::new(
            crate::images::ImageStore::open(root)
                .map_err(|_| sqlx::Error::Protocol("Image storage unavailable".into()))?,
        ));
        history
            .cleanup_images()
            .await
            .map_err(|_| sqlx::Error::Protocol("Image cleanup failed".into()))?;
        Ok(history)
    }

    async fn initialize(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        migrations::run(&pool).await?;
        let history = Self { pool, images: None };
        let mut transaction = history.pool.begin().await?;
        Self::prune(&mut transaction).await?;
        transaction.commit().await?;
        Ok(history)
    }

    pub async fn capture_text(&self, text: &str) -> Result<(), AppError> {
        validate_text(text)?;
        let mut transaction = self.pool.begin().await.map_err(database_error)?;
        let hash = hex::encode(Sha256::digest(text.as_bytes()));
        sqlx::query(
            "INSERT INTO clipboard_items (id, kind, text, hash, created_at, captured_at)
             VALUES (?, 'text', ?, ?, ?, MAX(?, COALESCE((SELECT MAX(captured_at) + 1 FROM clipboard_items), 0)))
             ON CONFLICT(kind, hash) DO UPDATE SET captured_at = excluded.captured_at",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind(text)
        .bind(hash)
        .bind(chrono::Utc::now().timestamp_millis())
        .bind(chrono::Utc::now().timestamp_millis())
        .execute(&mut *transaction)
        .await
        .map_err(|_| AppError::storage("Could not save clipboard history"))?;
        Self::prune(&mut transaction)
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        self.cleanup_images().await
    }

    async fn prune(transaction: &mut Transaction<'_, Sqlite>) -> Result<(), sqlx::Error> {
        sqlx::query(
            "DELETE FROM clipboard_items WHERE pinned = 0 AND captured_at <
             ? - (SELECT max_age_days * 86400000 FROM retention WHERE singleton = 1)",
        )
        .bind(chrono::Utc::now().timestamp_millis())
        .execute(&mut **transaction)
        .await?;
        sqlx::query(
            "DELETE FROM clipboard_items WHERE id IN (
                SELECT id FROM clipboard_items WHERE pinned = 0
                ORDER BY captured_at DESC, rowid DESC
                LIMIT -1 OFFSET (SELECT max_items FROM retention WHERE singleton = 1)
             )",
        )
        .execute(&mut **transaction)
        .await?;
        Ok(())
    }

    pub async fn retention(&self) -> Result<Retention, AppError> {
        let row = sqlx::query("SELECT max_items, max_age_days FROM retention WHERE singleton = 1")
            .fetch_one(&self.pool)
            .await
            .map_err(database_error)?;
        Ok(Retention {
            max_items: row.get("max_items"),
            max_age_days: row.get("max_age_days"),
        })
    }

    pub async fn set_retention(&self, retention: Retention) -> Result<(), AppError> {
        if !(1..=10000).contains(&retention.max_items)
            || !(1..=3650).contains(&retention.max_age_days)
        {
            return Err("Retention must be 1–10000 items and 1–3650 days".into());
        }
        let mut transaction = self.pool.begin().await.map_err(database_error)?;
        sqlx::query("UPDATE retention SET max_items = ?, max_age_days = ? WHERE singleton = 1")
            .bind(retention.max_items)
            .bind(retention.max_age_days)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;
        Self::prune(&mut transaction)
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        self.cleanup_images().await
    }

    pub async fn set_pinned(&self, id: &str, pinned: bool) -> Result<(), AppError> {
        validate_id(id)?;
        let mut transaction = self.pool.begin().await.map_err(database_error)?;
        let result = sqlx::query("UPDATE clipboard_items SET pinned = ? WHERE id = ?")
            .bind(pinned)
            .bind(id)
            .execute(&mut *transaction)
            .await
            .map_err(database_error)?;
        if result.rows_affected() == 0 {
            return Err(AppError::not_found());
        }
        Self::prune(&mut transaction)
            .await
            .map_err(database_error)?;
        transaction.commit().await.map_err(database_error)?;
        self.cleanup_images().await
    }

    pub async fn edit_text(&self, id: &str, text: &str) -> Result<(), AppError> {
        validate_id(id)?;
        validate_text(text)?;
        let result = sqlx::query(
            "UPDATE clipboard_items SET text = ?, hash = ? WHERE id = ? AND kind = 'text'",
        )
        .bind(text)
        .bind(hex::encode(Sha256::digest(text.as_bytes())))
        .bind(id)
        .execute(&self.pool)
        .await
        .map_err(|error| {
            if error
                .as_database_error()
                .is_some_and(|error| error.is_unique_violation())
            {
                AppError::duplicate()
            } else {
                database_error(error)
            }
        })?;
        if result.rows_affected() == 0 {
            return Err(AppError::not_found());
        }
        Ok(())
    }

    pub async fn delete(&self, ids: &[String]) -> Result<(), AppError> {
        if ids.is_empty() || ids.len() > 10000 {
            return Err("Select 1–10000 history items to delete".into());
        }
        for id in ids {
            validate_id(id)?;
        }
        let mut transaction = self.pool.begin().await.map_err(database_error)?;
        for id in ids {
            sqlx::query("DELETE FROM clipboard_items WHERE id = ?")
                .bind(id)
                .execute(&mut *transaction)
                .await
                .map_err(database_error)?;
        }
        transaction.commit().await.map_err(database_error)?;
        self.cleanup_images().await
    }

    pub async fn clear(&self, include_pinned: bool) -> Result<(), AppError> {
        sqlx::query("DELETE FROM clipboard_items WHERE pinned = 0 OR ?")
            .bind(include_pinned)
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
        self.cleanup_images().await
    }

    pub async fn settings(&self) -> Result<Settings, AppError> {
        let row = sqlx::query(
            "SELECT capture_images, paste_separator, window_width, window_height, stage_selection, show_shortcut_bar FROM preferences WHERE singleton = 1",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(Settings {
            capture_images: row.get("capture_images"),
            paste_separator: row.get("paste_separator"),
            window_width: row.get("window_width"),
            window_height: row.get("window_height"),
            stage_selection: row.get("stage_selection"),
            show_shortcut_bar: row.get("show_shortcut_bar"),
        })
    }

    pub async fn set_settings(&self, settings: Settings) -> Result<(), AppError> {
        if settings.paste_separator.len() > 1024 || settings.paste_separator.contains('\0') {
            return Err("Paste separator must be at most 1024 bytes without NUL characters".into());
        }
        if !(360..=4096).contains(&settings.window_width)
            || !(360..=4096).contains(&settings.window_height)
        {
            return Err("Window width and height must be between 360 and 4096.".into());
        }
        sqlx::query(
            "UPDATE preferences SET capture_images = ?, paste_separator = ?, window_width = ?, window_height = ?, stage_selection = ?, show_shortcut_bar = ? WHERE singleton = 1",
        )
        .bind(settings.capture_images)
        .bind(settings.paste_separator)
        .bind(settings.window_width)
        .bind(settings.window_height)
        .bind(settings.stage_selection)
        .bind(settings.show_shortcut_bar)
        .execute(&self.pool)
        .await
        .map_err(database_error)?;
        Ok(())
    }

    pub async fn mark_used(&self, ids: &[String]) -> Result<(), AppError> {
        for id in ids {
            validate_id(id)?;
            sqlx::query("UPDATE clipboard_items SET last_used_at = ? WHERE id = ?")
                .bind(chrono::Utc::now().timestamp_millis())
                .bind(id)
                .execute(&self.pool)
                .await
                .map_err(database_error)?;
        }
        Ok(())
    }

    pub async fn selected_text(&self, ids: &[String], separator: &str) -> Result<String, AppError> {
        if ids.is_empty() || ids.len() > 10000 || separator.len() > 1024 {
            return Err("Select 1–10000 entries with a separator of at most 1024 bytes".into());
        }
        for id in ids {
            validate_id(id)?;
        }
        let mut query = sqlx::QueryBuilder::<Sqlite>::new(
            "SELECT kind, text FROM clipboard_items WHERE id IN (",
        );
        let mut bindings = query.separated(",");
        for id in ids {
            bindings.push_bind(id);
        }
        bindings.push_unseparated(") ORDER BY pinned DESC, captured_at DESC, rowid DESC");
        let rows = query
            .build()
            .fetch_all(&self.pool)
            .await
            .map_err(database_error)?;
        if rows.len() != ids.iter().collect::<std::collections::HashSet<_>>().len() {
            return Err(AppError::not_found());
        }
        let mut result = String::new();
        for row in rows {
            if row.get::<String, _>("kind") != "text" {
                return Err("Select one image, or only text, to paste them together.".into());
            }
            let text: String = row.get("text");
            if !result.is_empty() {
                result.push_str(separator);
            }
            if result.len() + text.len() > 16 * 1024 * 1024 {
                return Err("Combined clipboard text exceeds the 16 MiB limit".into());
            }
            result.push_str(&text);
        }
        Ok(result)
    }

    pub async fn list(&self) -> Result<Vec<HistoryItem>, AppError> {
        let mut transaction = self.pool.begin().await.map_err(database_error)?;
        Self::prune(&mut transaction)
            .await
            .map_err(database_error)?;
        let rows = sqlx::query(
            "SELECT * FROM clipboard_items
             ORDER BY pinned DESC, captured_at DESC, rowid DESC",
        )
        .fetch_all(&mut *transaction)
        .await
        .map_err(|_| AppError::storage("Could not read clipboard history"))?;
        transaction.commit().await.map_err(database_error)?;
        self.cleanup_images().await?;
        Ok(rows
            .into_iter()
            .map(|row| HistoryItem {
                id: row.get("id"),
                kind: row.get("kind"),
                text: row.get("text"),
                hash: row.get("hash"),
                created_at: row.get("created_at"),
                last_used_at: row.get("last_used_at"),
                image_path: row.get("image_path"),
                thumbnail_path: row.get("thumbnail_path"),
                width: row.get("width"),
                height: row.get("height"),
                pinned: row.get("pinned"),
                captured_at: row.get("captured_at"),
            })
            .collect())
    }
}

#[cfg(test)]
mod tests;

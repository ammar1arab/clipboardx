use crate::error::AppError;
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
        Self::initialize(pool).await
    }

    async fn initialize(pool: SqlitePool) -> Result<Self, sqlx::Error> {
        migrations::run(&pool).await?;
        let history = Self { pool };
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
        Ok(())
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
        transaction.commit().await.map_err(database_error)
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
        transaction.commit().await.map_err(database_error)
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
        transaction.commit().await.map_err(database_error)
    }

    pub async fn clear(&self, include_pinned: bool) -> Result<(), AppError> {
        sqlx::query("DELETE FROM clipboard_items WHERE pinned = 0 OR ?")
            .bind(include_pinned)
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
        Ok(())
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

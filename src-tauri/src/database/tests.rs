use super::*;

#[test]
fn persists_exact_text_and_deduplicates_across_restart() {
    tauri::async_runtime::block_on(async {
        let path = std::env::temp_dir().join(format!("clipboardx-{}.sqlite", uuid::Uuid::new_v4()));
        let history = History::open(&path).await.unwrap();
        history.capture_text("  synthetic fixture\n").await.unwrap();
        let original_id = history.list().await.unwrap()[0].id.clone();
        history.pool.close().await;
        let reopened = History::open(&path).await.unwrap();
        reopened
            .capture_text("  synthetic fixture\n")
            .await
            .unwrap();
        let items = reopened.list().await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, original_id);
        assert_eq!(items[0].text.as_deref(), Some("  synthetic fixture\n"));
        reopened.pool.close().await;
        std::fs::remove_file(path).unwrap();
    });
}

#[test]
fn rejects_invalid_text_without_writing() {
    tauri::async_runtime::block_on(async {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        let history = History::initialize(pool).await.unwrap();
        for text in ["", "synthetic\0fixture", &"x".repeat(MAX_TEXT_BYTES + 1)] {
            assert!(history.capture_text(text).await.is_err());
        }
        assert!(history.list().await.unwrap().is_empty());
    });
}

async fn memory_history() -> History {
    History::initialize(SqlitePool::connect("sqlite::memory:").await.unwrap())
        .await
        .unwrap()
}

#[test]
fn edits_preserve_identity_and_pin_and_duplicate_edits_are_atomic() {
    tauri::async_runtime::block_on(async {
        let history = memory_history().await;
        history.capture_text("first fixture").await.unwrap();
        history.capture_text("second fixture").await.unwrap();
        let items = history.list().await.unwrap();
        let id = &items[1].id;
        history.set_pinned(id, true).await.unwrap();
        assert!(history.edit_text(id, "second fixture").await.is_err());
        assert_eq!(
            history.list().await.unwrap()[0].text.as_deref(),
            Some("first fixture")
        );
        history.edit_text(id, "modified fixture").await.unwrap();
        history.capture_text("modified fixture").await.unwrap();
        let updated = history.list().await.unwrap();
        assert_eq!(updated.len(), 2);
        assert_eq!(updated[0].id, *id);
        assert!(updated[0].pinned);
        assert_eq!(updated[0].created_at, items[1].created_at);
        assert_eq!(
            updated[0].hash,
            hex::encode(Sha256::digest(b"modified fixture"))
        );
    });
}

#[test]
fn rapid_recapture_promotes_same_entry_without_duplicates() {
    tauri::async_runtime::block_on(async {
        let history = memory_history().await;
        history.capture_text("first").await.unwrap();
        let id = history.list().await.unwrap()[0].id.clone();
        history.capture_text("second").await.unwrap();
        history.capture_text("first").await.unwrap();
        let items = history.list().await.unwrap();
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].id, id);
        assert!(items[0].captured_at > items[1].captured_at);
    });
}

#[test]
fn retention_preserves_pins_and_applies_on_unpin() {
    tauri::async_runtime::block_on(async {
        let history = memory_history().await;
        history.capture_text("pinned fixture").await.unwrap();
        let pinned_id = history.list().await.unwrap()[0].id.clone();
        history.set_pinned(&pinned_id, true).await.unwrap();
        history.capture_text("old fixture").await.unwrap();
        let old_id = history.list().await.unwrap()[1].id.clone();
        history.capture_text("new fixture").await.unwrap();
        history
            .set_retention(Retention {
                max_items: 1,
                max_age_days: 1,
            })
            .await
            .unwrap();
        let items = history.list().await.unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|item| item.id != old_id));
        sqlx::query("UPDATE clipboard_items SET captured_at = 0")
            .execute(&history.pool)
            .await
            .unwrap();
        assert_eq!(history.list().await.unwrap().len(), 1);
        history.set_pinned(&pinned_id, false).await.unwrap();
        assert!(history.list().await.unwrap().is_empty());
    });
}

#[test]
fn invalid_operations_do_not_mutate_history_or_settings() {
    tauri::async_runtime::block_on(async {
        let history = memory_history().await;
        history.capture_text("fixture").await.unwrap();
        let id = history.list().await.unwrap()[0].id.clone();
        assert!(history
            .delete(&[id.clone(), "bad ID".into()])
            .await
            .is_err());
        assert!(history.edit_text(&id, "").await.is_err());
        assert!(history.set_pinned("bad ID", true).await.is_err());
        let missing = uuid::Uuid::new_v4().to_string();
        assert!(history.edit_text(&missing, "other").await.is_err());
        assert!(history.set_pinned(&missing, true).await.is_err());
        assert!(history
            .set_retention(Retention {
                max_items: 0,
                max_age_days: 30
            })
            .await
            .is_err());
        assert_eq!(
            history.retention().await.unwrap(),
            Retention {
                max_items: 500,
                max_age_days: 30
            }
        );
        assert_eq!(
            history.list().await.unwrap()[0].text.as_deref(),
            Some("fixture")
        );
        history.delete(&[id.clone(), id]).await.unwrap();
        assert!(history.list().await.unwrap().is_empty());
    });
}

#[test]
fn clear_unpinned_preserves_pins_and_clear_all_removes_them() {
    tauri::async_runtime::block_on(async {
        let history = memory_history().await;
        history.capture_text("pin fixture").await.unwrap();
        let id = history.list().await.unwrap()[0].id.clone();
        history.set_pinned(&id, true).await.unwrap();
        history.capture_text("ordinary fixture").await.unwrap();
        history.clear(false).await.unwrap();
        let remaining = history.list().await.unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, id);
        history.clear(true).await.unwrap();
        assert!(history.list().await.unwrap().is_empty());
    });
}

#[test]
fn migration_preserves_legacy_history_and_settings_and_is_repeatable() {
    tauri::async_runtime::block_on(async {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();
        sqlx::raw_sql(include_str!("../../migrations/0001_text_history.sql"))
            .execute(&pool)
            .await
            .unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        sqlx::query("INSERT INTO text_history VALUES (?, ?, ?, 1, ?)")
            .bind(&id)
            .bind("legacy synthetic fixture")
            .bind(hex::encode(Sha256::digest(b"legacy synthetic fixture")))
            .bind(chrono::Utc::now().timestamp_millis())
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("UPDATE retention SET max_items = 25, max_age_days = 7")
            .execute(&pool)
            .await
            .unwrap();
        let history = History::initialize(pool).await.unwrap();
        migrations::run(&history.pool).await.unwrap();
        let items = history.list().await.unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, id);
        assert!(items[0].pinned);
        assert_eq!(items[0].kind, "text");
        assert_eq!(items[0].created_at, items[0].captured_at);
        assert_eq!(
            history.retention().await.unwrap(),
            Retention {
                max_items: 25,
                max_age_days: 7
            }
        );
    });
}

#[test]
fn settings_and_pins_survive_reopen() {
    tauri::async_runtime::block_on(async {
        let path = std::env::temp_dir().join(format!("clipboardx-{}.sqlite", uuid::Uuid::new_v4()));
        let history = History::open(&path).await.unwrap();
        history.capture_text("persistent fixture").await.unwrap();
        let id = history.list().await.unwrap()[0].id.clone();
        history.set_pinned(&id, true).await.unwrap();
        let settings = Retention {
            max_items: 12,
            max_age_days: 3,
        };
        history.set_retention(settings.clone()).await.unwrap();
        history.pool.close().await;
        let reopened = History::open(&path).await.unwrap();
        assert_eq!(reopened.retention().await.unwrap(), settings);
        assert!(reopened.list().await.unwrap()[0].pinned);
        reopened.pool.close().await;
        std::fs::remove_file(path).unwrap();
    });
}

#[test]
fn selection_text_follows_history_order_and_rejects_missing_entries() {
    tauri::async_runtime::block_on(async {
        let history = memory_history().await;
        history.capture_text("first synthetic").await.unwrap();
        history.capture_text("second synthetic").await.unwrap();
        let items = history.list().await.unwrap();
        let ids = [items[1].id.clone(), items[0].id.clone()];
        assert_eq!(
            history.selected_text(&ids, "\n---\n").await.unwrap(),
            "second synthetic\n---\nfirst synthetic"
        );
        history.set_pinned(&ids[0], true).await.unwrap();
        assert_eq!(
            history.selected_text(&ids, "\n").await.unwrap(),
            "first synthetic\nsecond synthetic"
        );
        assert!(history
            .selected_text(&[uuid::Uuid::new_v4().to_string()], "\n")
            .await
            .is_err());
        assert!(history.selected_text(&[], "\n").await.is_err());
    });
}

#[test]
fn preferences_persist_and_invalid_separator_is_atomic() {
    tauri::async_runtime::block_on(async {
        let path = std::env::temp_dir().join(format!("clipboardx-{}.sqlite", uuid::Uuid::new_v4()));
        let history = History::open(&path).await.unwrap();
        history
            .set_settings(Settings {
                capture_images: false,
                paste_separator: " | ".into(),
                window_width: 800,
                window_height: 640,
                always_on_top: true,
            })
            .await
            .unwrap();
        assert!(history
            .set_settings(Settings {
                capture_images: true,
                paste_separator: "x".repeat(1025),
                window_width: 800,
                window_height: 640,
                always_on_top: true,
            })
            .await
            .is_err());
        assert!(history
            .set_settings(Settings {
                capture_images: false,
                paste_separator: " | ".into(),
                window_width: 200,
                window_height: 640,
                always_on_top: false,
            })
            .await
            .is_err());
        history.pool.close().await;
        let reopened = History::open(&path).await.unwrap();
        let settings = reopened.settings().await.unwrap();
        assert!(!settings.capture_images);
        assert_eq!(settings.paste_separator, " | ");
        assert_eq!((settings.window_width, settings.window_height), (800, 640));
        assert!(settings.always_on_top);
        reopened.pool.close().await;
        std::fs::remove_file(path).unwrap();
    });
}

#[test]
fn images_persist_deduplicate_and_deletion_cleans_files() {
    tauri::async_runtime::block_on(async {
        let root = std::env::temp_dir().join(format!("clipboardx-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&root).unwrap();
        let path = root.join("history.sqlite");
        let history = History::open(&path).await.unwrap();
        let image = image::DynamicImage::new_rgba8(16, 16);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        history
            .capture_image(bytes.get_ref().clone())
            .await
            .unwrap();
        history
            .capture_image(bytes.get_ref().clone())
            .await
            .unwrap();
        let items = history.list().await.unwrap();
        assert_eq!(items.len(), 1);
        let id = items[0].id.clone();
        let file = items[0].image_path.clone().unwrap();
        let thumbnail = items[0].thumbnail_path.clone().unwrap();
        history.set_pinned(&id, true).await.unwrap();
        history.clear(false).await.unwrap();
        assert!(Path::new(&file).exists());
        assert!(history
            .selected_text(std::slice::from_ref(&id), "\n")
            .await
            .is_err());
        assert_eq!(
            history
                .selected_image(std::slice::from_ref(&id))
                .await
                .unwrap()
                .unwrap()
                .width,
            16
        );
        history.pool.close().await;
        let reopened = History::open(&path).await.unwrap();
        assert!(reopened.list().await.unwrap()[0].pinned);
        reopened.delete(&[id]).await.unwrap();
        assert!(!Path::new(&file).exists());
        assert!(!Path::new(&thumbnail).exists());
        reopened.pool.close().await;
        std::fs::remove_dir_all(root).unwrap();
    });
}

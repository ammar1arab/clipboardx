use sqlx::{Row, SqlitePool};

const MIGRATIONS: &[(i64, &str)] = &[
    (1, include_str!("../../migrations/0001_text_history.sql")),
    (2, include_str!("../../migrations/0002_clipboard_items.sql")),
    (3, include_str!("../../migrations/0003_settings.sql")),
    (4, include_str!("../../migrations/0004_window_size.sql")),
    (5, include_str!("../../migrations/0005_window_mode.sql")),
];

pub(super) async fn run(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let mut transaction = pool.begin().await?;
    sqlx::query("CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY)")
        .execute(&mut *transaction)
        .await?;
    let current: i64 =
        sqlx::query("SELECT COALESCE(MAX(version), 0) AS version FROM schema_migrations")
            .fetch_one(&mut *transaction)
            .await?
            .get("version");
    if current > MIGRATIONS.last().map_or(0, |migration| migration.0) {
        return Err(sqlx::Error::Protocol(
            "Database was created by a newer ClipboardX version".into(),
        ));
    }
    for (version, sql) in MIGRATIONS {
        if *version > current {
            sqlx::raw_sql(sql).execute(&mut *transaction).await?;
            sqlx::query("INSERT INTO schema_migrations VALUES (?)")
                .bind(version)
                .execute(&mut *transaction)
                .await?;
        }
    }
    transaction.commit().await
}

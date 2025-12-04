use std::path::Path;

use anyhow::{Context, Result};
use sqlx::{sqlite::SqlitePoolOptions, SqlitePool};
use tracing::info;

pub async fn init_pool(path: &Path) -> Result<SqlitePool> {
    let url = format!("sqlite://{}", path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .context("failed to open sqlite database")?;
    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> Result<()> {
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS boards (
            name TEXT PRIMARY KEY,
            created_at TEXT NOT NULL
        );
    "#,
    )
    .execute(pool)
    .await
    .context("failed to create boards table")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS threads (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            board_name TEXT NOT NULL REFERENCES boards(name) ON DELETE CASCADE,
            title TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
    "#,
    )
    .execute(pool)
    .await
    .context("failed to create threads table")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            thread_id INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
            author TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
    "#,
    )
    .execute(pool)
    .await
    .context("failed to create messages table")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS message_markers (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            message_id INTEGER NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
            kind TEXT NOT NULL,
            value TEXT NOT NULL
        );
    "#,
    )
    .execute(pool)
    .await
    .context("failed to create message_markers table")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS inbox_messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            persona TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL
        );
    "#,
    )
    .execute(pool)
    .await
    .context("failed to create inbox_messages table")?;

    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS common_items (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            key TEXT UNIQUE NOT NULL,
            content TEXT NOT NULL,
            created_at TEXT NOT NULL,
            expires_at TEXT
        );
    "#,
    )
    .execute(pool)
    .await
    .context("failed to create common_items table")?;

    info!("database migrations complete");
    Ok(())
}

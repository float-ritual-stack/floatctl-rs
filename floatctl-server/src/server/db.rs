use std::path::Path;

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use tracing::info;

use crate::server::error::{ServerError, ServerResult};

pub async fn init_pool(path: &Path) -> ServerResult<SqlitePool> {
    let options = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true)
        .journal_mode(SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    Ok(pool)
}

pub async fn run_migrations(pool: &SqlitePool) -> ServerResult<()> {
    info!("running migrations");
    sqlx::migrate!("./migrations").run(pool).await?;
    Ok(())
}

pub async fn cleanup_common(pool: &SqlitePool) -> ServerResult<()> {
    sqlx::query!(
        "DELETE FROM common_items WHERE expires_at IS NOT NULL AND expires_at <= strftime('%s','now')"
    )
    .execute(pool)
    .await?;
    Ok(())
}

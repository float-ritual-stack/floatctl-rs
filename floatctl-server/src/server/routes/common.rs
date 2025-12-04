use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::server::{
    db,
    error::ServerResult,
    models::ts_to_datetime,
    models::{CommonItem, CommonItemCreate},
    AppState,
};

pub async fn list_common(
    State(state): State<Arc<AppState>>,
) -> ServerResult<Json<Vec<CommonItem>>> {
    db::cleanup_common(&state.pool).await?;

    let rows = sqlx::query!(
        "SELECT key, content, created_at, expires_at FROM common_items WHERE expires_at IS NULL OR expires_at > strftime('%s','now') ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool)
    .await?;

    let items = rows
        .into_iter()
        .map(|row| CommonItem {
            key: row.key,
            content: row.content,
            created_at: ts_to_datetime(row.created_at),
            expires_at: row.expires_at.map(ts_to_datetime),
        })
        .collect();

    Ok(Json(items))
}

pub async fn create_common(
    State(state): State<Arc<AppState>>,
    Json(body): Json<CommonItemCreate>,
) -> ServerResult<Json<CommonItem>> {
    db::cleanup_common(&state.pool).await?;
    let key = body
        .key
        .clone()
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let expires_at = body
        .ttl_seconds
        .map(|ttl| chrono::Utc::now().timestamp() + ttl);

    sqlx::query!(
        "INSERT OR REPLACE INTO common_items (key, content, expires_at) VALUES (?, ?, ?)",
        key,
        body.content,
        expires_at
    )
    .execute(&state.pool)
    .await?;

    let row = sqlx::query!(
        "SELECT key, content, created_at, expires_at FROM common_items WHERE key = ?",
        key
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(CommonItem {
        key: row.key,
        content: row.content,
        created_at: ts_to_datetime(row.created_at),
        expires_at: row.expires_at.map(ts_to_datetime),
    }))
}

pub async fn get_common(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> ServerResult<Json<CommonItem>> {
    db::cleanup_common(&state.pool).await?;

    let row = sqlx::query!(
        "SELECT key, content, created_at, expires_at FROM common_items WHERE key = ?",
        key
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(CommonItem {
        key: row.key,
        content: row.content,
        created_at: ts_to_datetime(row.created_at),
        expires_at: row.expires_at.map(ts_to_datetime),
    }))
}

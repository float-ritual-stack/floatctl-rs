use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::{Duration, Utc};
use sqlx::Row;

use crate::server::error::{AppError, AppResult};
use crate::server::models::{CommonItem, CommonItemRequest};
use crate::server::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_items).post(create_item))
        .route("/:key", get(get_item))
}

async fn list_items(State(state): State<AppState>) -> AppResult<Json<Vec<CommonItem>>> {
    cleanup_expired(&state).await?;
    let rows = sqlx::query(
        "SELECT id, key, content, created_at, expires_at FROM common_items ORDER BY created_at DESC",
    )
    .fetch_all(&state.pool)
    .await?;

    let items = rows.into_iter().filter_map(|row| map_item(row)).collect();

    Ok(Json(items))
}

async fn create_item(
    State(state): State<AppState>,
    Json(payload): Json<CommonItemRequest>,
) -> AppResult<Json<CommonItem>> {
    if payload.key.trim().is_empty() {
        return Err(AppError::BadRequest("key cannot be empty".into()));
    }

    let created_at = Utc::now();
    let expires_at = payload
        .ttl_seconds
        .map(|ttl| created_at + Duration::seconds(ttl));

    let row = sqlx::query(
        "INSERT INTO common_items (key, content, created_at, expires_at) VALUES (?, ?, ?, ?) \
         ON CONFLICT(key) DO UPDATE SET content=excluded.content, expires_at=excluded.expires_at \
         RETURNING id, key, content, created_at, expires_at",
    )
    .bind(&payload.key)
    .bind(&payload.content)
    .bind(created_at)
    .bind(expires_at)
    .fetch_one(&state.pool)
    .await?;

    map_item(row)
        .map(Json)
        .ok_or(AppError::BadRequest("failed to store item".into()))
}

async fn get_item(
    State(state): State<AppState>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> AppResult<Json<CommonItem>> {
    cleanup_expired(&state).await?;
    let row = sqlx::query(
        "SELECT id, key, content, created_at, expires_at FROM common_items WHERE key = ?",
    )
    .bind(&key)
    .fetch_one(&state.pool)
    .await
    .map_err(|_| AppError::NotFound)?;

    map_item(row).map(Json).ok_or(AppError::NotFound)
}

async fn cleanup_expired(state: &AppState) -> AppResult<()> {
    sqlx::query("DELETE FROM common_items WHERE expires_at IS NOT NULL AND expires_at <= ?")
        .bind(Utc::now())
        .execute(&state.pool)
        .await?;
    Ok(())
}

fn map_item(row: sqlx::sqlite::SqliteRow) -> Option<CommonItem> {
    let created_at: String = row.get("created_at");
    let expires_at: Option<String> = row.get("expires_at");
    Some(CommonItem {
        id: row.get("id"),
        key: row.get("key"),
        content: row.get("content"),
        created_at: created_at.parse().ok()?,
        expires_at: expires_at.and_then(|v| v.parse().ok()),
    })
}

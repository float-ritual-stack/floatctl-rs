use axum::{
    extract::State,
    routing::{delete, get, post},
    Json, Router,
};
use chrono::Utc;
use sqlx::Row;

use crate::server::error::{AppError, AppResult};
use crate::server::models::{InboxMessage, PersonaMessageRequest};
use crate::server::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/:persona", get(list_messages).post(post_message))
        .route("/:persona/:id", delete(delete_message))
}

async fn list_messages(
    State(state): State<AppState>,
    axum::extract::Path(persona): axum::extract::Path<String>,
) -> AppResult<Json<Vec<InboxMessage>>> {
    let rows = sqlx::query(
        "SELECT id, persona, content, created_at FROM inbox_messages WHERE persona = ? ORDER BY created_at ASC",
    )
    .bind(&persona)
    .fetch_all(&state.pool)
    .await?;

    let messages = rows
        .into_iter()
        .filter_map(|row| {
            let created_at: String = row.get("created_at");
            Some(InboxMessage {
                id: row.get("id"),
                persona: row.get("persona"),
                content: row.get("content"),
                created_at: created_at.parse().ok()?,
            })
        })
        .collect();

    Ok(Json(messages))
}

async fn post_message(
    State(state): State<AppState>,
    axum::extract::Path(persona): axum::extract::Path<String>,
    Json(payload): Json<PersonaMessageRequest>,
) -> AppResult<Json<InboxMessage>> {
    if payload.content.trim().is_empty() {
        return Err(AppError::BadRequest("content cannot be empty".into()));
    }

    let created_at = Utc::now();
    let id = sqlx::query_scalar(
        "INSERT INTO inbox_messages (persona, content, created_at) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(&persona)
    .bind(&payload.content)
    .bind(created_at)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(InboxMessage {
        id,
        persona,
        content: payload.content,
        created_at,
    }))
}

async fn delete_message(
    State(state): State<AppState>,
    axum::extract::Path((persona, id)): axum::extract::Path<(String, i64)>,
) -> AppResult<Json<()>> {
    let affected = sqlx::query("DELETE FROM inbox_messages WHERE persona = ? AND id = ?")
        .bind(persona)
        .bind(id)
        .execute(&state.pool)
        .await?
        .rows_affected();

    if affected == 0 {
        return Err(AppError::NotFound);
    }

    Ok(Json(()))
}

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    Json,
};

use crate::server::{
    error::ServerResult,
    models::ts_to_datetime,
    models::{InboxMessage, InboxMessageCreate},
    AppState,
};

pub async fn get_inbox(
    State(state): State<Arc<AppState>>,
    Path(persona): Path<String>,
) -> ServerResult<Json<Vec<InboxMessage>>> {
    let rows = sqlx::query!(
        "SELECT id, persona, author, content, created_at FROM inbox_messages WHERE persona = ? ORDER BY created_at DESC",
        persona
    )
    .fetch_all(&state.pool)
    .await?;

    let messages = rows
        .into_iter()
        .map(|row| InboxMessage {
            id: row.id,
            persona: row.persona,
            author: row.author,
            content: row.content,
            created_at: ts_to_datetime(row.created_at),
        })
        .collect();

    Ok(Json(messages))
}

pub async fn send_inbox(
    State(state): State<Arc<AppState>>,
    Path(persona): Path<String>,
    Json(body): Json<InboxMessageCreate>,
) -> ServerResult<Json<InboxMessage>> {
    sqlx::query!(
        "INSERT INTO inbox_messages (persona, author, content) VALUES (?, ?, ?)",
        persona,
        body.author,
        body.content
    )
    .execute(&state.pool)
    .await?;

    let row = sqlx::query!(
        "SELECT id, persona, author, content, created_at FROM inbox_messages WHERE id = last_insert_rowid()"
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(InboxMessage {
        id: row.id,
        persona: row.persona,
        author: row.author,
        content: row.content,
        created_at: ts_to_datetime(row.created_at),
    }))
}

pub async fn delete_inbox_message(
    State(state): State<Arc<AppState>>,
    Path((persona, id)): Path<(String, i64)>,
) -> ServerResult<Json<InboxMessage>> {
    let row = sqlx::query!(
        "SELECT id, persona, author, content, created_at FROM inbox_messages WHERE persona = ? AND id = ?",
        persona,
        id
    )
    .fetch_one(&state.pool)
    .await?;

    sqlx::query!("DELETE FROM inbox_messages WHERE id = ?", id)
        .execute(&state.pool)
        .await?;

    Ok(Json(InboxMessage {
        id: row.id,
        persona: row.persona,
        author: row.author,
        content: row.content,
        created_at: ts_to_datetime(row.created_at),
    }))
}

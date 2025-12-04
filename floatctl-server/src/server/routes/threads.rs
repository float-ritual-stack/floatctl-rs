use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use sqlx::{Row, SqlitePool};

use crate::server::{
    error::ServerResult,
    models::ts_to_datetime,
    models::{Marker, Message, MessageCreate, Thread, ThreadCreate, ThreadDetail},
    AppState,
};

#[derive(Deserialize)]
pub struct ThreadQuery {
    pub project: Option<String>,
}

pub async fn list_threads(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ThreadQuery>,
) -> ServerResult<Json<Vec<Thread>>> {
    let threads = load_threads(&state.pool, query.project.as_deref(), None).await?;
    Ok(Json(threads))
}

pub async fn list_board_threads(
    State(state): State<Arc<AppState>>,
    Path(board): Path<String>,
    Query(query): Query<ThreadQuery>,
) -> ServerResult<Json<Vec<Thread>>> {
    let threads = load_threads(&state.pool, query.project.as_deref(), Some(&board)).await?;
    Ok(Json(threads))
}

pub async fn get_thread(
    State(state): State<Arc<AppState>>,
    Path(id): Path<i64>,
) -> ServerResult<Json<ThreadDetail>> {
    let row = sqlx::query!(
        "SELECT id, board_name, title, author, created_at FROM threads WHERE id = ?",
        id
    )
    .fetch_one(&state.pool)
    .await?;

    let markers = load_thread_markers(&state.pool, id).await?;
    let messages = load_thread_messages(&state.pool, id).await?;

    let thread = Thread {
        id: row.id,
        board_name: row.board_name,
        title: row.title,
        author: row.author,
        created_at: ts_to_datetime(row.created_at),
        markers,
    };

    Ok(Json(ThreadDetail { thread, messages }))
}

pub async fn create_thread(
    State(state): State<Arc<AppState>>,
    Path(board): Path<String>,
    Json(body): Json<ThreadCreate>,
) -> ServerResult<Json<Thread>> {
    sqlx::query!("INSERT OR IGNORE INTO boards (name) VALUES (?)", board)
        .execute(&state.pool)
        .await?;

    let result = sqlx::query!(
        "INSERT INTO threads (board_name, title, author) VALUES (?, ?, ?)",
        board,
        body.title,
        body.author
    )
    .execute(&state.pool)
    .await?;

    let thread_id = result.last_insert_rowid();
    let mut markers = Vec::new();
    markers.extend(crate::server::extract_markers(&body.title));
    if let Some(content) = &body.content {
        markers.extend(crate::server::extract_markers(content));
    }

    for marker in &markers {
        let _ = sqlx::query!(
            "INSERT OR IGNORE INTO thread_markers (thread_id, kind, value) VALUES (?, ?, ?)",
            thread_id,
            marker.kind,
            marker.value
        )
        .execute(&state.pool)
        .await?;
    }

    if let Some(content) = body.content {
        sqlx::query!(
            "INSERT INTO messages (thread_id, author, content) VALUES (?, ?, ?)",
            thread_id,
            body.author,
            content
        )
        .execute(&state.pool)
        .await?;
    }

    let markers = load_thread_markers(&state.pool, thread_id).await?;

    let row = sqlx::query!(
        "SELECT id, board_name, title, author, created_at FROM threads WHERE id = ?",
        thread_id
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(Thread {
        id: row.id,
        board_name: row.board_name,
        title: row.title,
        author: row.author,
        created_at: ts_to_datetime(row.created_at),
        markers,
    }))
}

pub async fn add_message(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<i64>,
    Json(body): Json<MessageCreate>,
) -> ServerResult<Json<Message>> {
    sqlx::query!(
        "INSERT INTO messages (thread_id, author, content) VALUES (?, ?, ?)",
        thread_id,
        body.author,
        body.content
    )
    .execute(&state.pool)
    .await?;

    let message_row = sqlx::query!(
        "SELECT id, thread_id, author, content, created_at FROM messages WHERE id = last_insert_rowid()"
    )
    .fetch_one(&state.pool)
    .await?;

    let markers = crate::server::extract_markers(&message_row.content);
    for marker in &markers {
        let _ = sqlx::query!(
            "INSERT OR IGNORE INTO thread_markers (thread_id, kind, value) VALUES (?, ?, ?)",
            thread_id,
            marker.kind,
            marker.value
        )
        .execute(&state.pool)
        .await?;
    }

    let message = Message {
        id: message_row.id,
        thread_id: message_row.thread_id,
        author: message_row.author,
        content: message_row.content.clone(),
        created_at: ts_to_datetime(message_row.created_at),
        markers,
    };

    Ok(Json(message))
}

pub async fn load_thread_markers(pool: &SqlitePool, id: i64) -> ServerResult<Vec<Marker>> {
    let rows = sqlx::query!(
        "SELECT kind, value FROM thread_markers WHERE thread_id = ? ORDER BY kind, value",
        id
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| Marker {
            kind: row.kind,
            value: row.value,
        })
        .collect())
}

async fn load_thread_messages(pool: &SqlitePool, thread_id: i64) -> ServerResult<Vec<Message>> {
    let rows = sqlx::query!(
        "SELECT id, thread_id, author, content, created_at FROM messages WHERE thread_id = ? ORDER BY created_at ASC",
        thread_id
    )
    .fetch_all(pool)
    .await?;

    let mut messages = Vec::new();
    for row in rows {
        let markers = crate::server::extract_markers(&row.content);
        messages.push(Message {
            id: row.id,
            thread_id: row.thread_id,
            author: row.author.clone(),
            content: row.content.clone(),
            created_at: ts_to_datetime(row.created_at),
            markers,
        });
    }

    Ok(messages)
}

async fn load_threads(
    pool: &SqlitePool,
    project: Option<&str>,
    board: Option<&str>,
) -> ServerResult<Vec<Thread>> {
    let mut query =
        String::from("SELECT id, board_name, title, author, created_at FROM threads WHERE 1=1");
    if let Some(board) = board {
        query.push_str(" AND board_name = ?");
    }
    if project.is_some() {
        query.push_str(" AND id IN (SELECT thread_id FROM thread_markers WHERE kind = 'project' AND value = ?)");
    }
    query.push_str(" ORDER BY created_at DESC");

    let mut q = sqlx::query(&query);
    if let Some(board) = board {
        q = q.bind(board);
    }
    if let Some(project) = project {
        q = q.bind(project);
    }

    let rows = q.fetch_all(pool).await?;

    let mut threads = Vec::new();
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let markers = load_thread_markers(pool, id).await?;
        threads.push(Thread {
            id,
            board_name: row.try_get("board_name")?,
            title: row.try_get("title")?,
            author: row.try_get("author")?,
            created_at: ts_to_datetime(row.try_get("created_at")?),
            markers,
        });
    }

    Ok(threads)
}

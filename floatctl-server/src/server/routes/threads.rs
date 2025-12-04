use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use regex::Regex;
use sqlx::{Row, Sqlite, Transaction};

use crate::server::error::{AppError, AppResult};
use crate::server::models::{
    CreateMessageRequest, Message, ThreadDetail, ThreadQuery, ThreadSummary,
};
use crate::server::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_threads))
        .route("/:id", get(get_thread))
        .route("/:id/messages", post(add_message))
}

async fn list_threads(
    State(state): State<AppState>,
    axum::extract::Query(filters): axum::extract::Query<ThreadQuery>,
) -> AppResult<Json<Vec<ThreadSummary>>> {
    let mut query = String::from(
        "SELECT t.id, t.board_name, t.title, t.created_at, COUNT(DISTINCT m.id) as message_count \
         FROM threads t \
         LEFT JOIN messages m ON m.thread_id = t.id",
    );
    let mut args: Vec<(String, String)> = Vec::new();

    if filters.project.is_some() || filters.ctx.is_some() {
        query.push_str(" LEFT JOIN message_markers mk ON mk.message_id = m.id");
    }

    let mut conditions: Vec<String> = Vec::new();
    if let Some(board) = filters.board {
        conditions.push("t.board_name = ?".to_string());
        args.push(("board".into(), board));
    }
    if let Some(project) = filters.project {
        conditions.push("(mk.kind = 'project' AND mk.value = ? )".to_string());
        args.push(("project".into(), project));
    }
    if let Some(ctx) = filters.ctx {
        conditions.push("(mk.kind = 'ctx' AND mk.value = ? )".to_string());
        args.push(("ctx".into(), ctx));
    }
    if !conditions.is_empty() {
        query.push_str(" WHERE ");
        query.push_str(&conditions.join(" AND "));
    }

    query.push_str(" GROUP BY t.id ORDER BY t.created_at DESC");

    let mut q = sqlx::query(&query);
    for (_name, value) in args {
        q = q.bind(value);
    }

    let rows = q.fetch_all(&state.pool).await?;
    let threads = rows
        .into_iter()
        .filter_map(|row| {
            let created_at: String = row.get("created_at");
            let created_at = created_at.parse().ok()?;
            Some(ThreadSummary {
                id: row.get("id"),
                board_name: row.get("board_name"),
                title: row.get("title"),
                created_at,
                message_count: row.get("message_count"),
            })
        })
        .collect();

    Ok(Json(threads))
}

async fn get_thread(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
) -> AppResult<Json<ThreadDetail>> {
    let thread_row =
        sqlx::query("SELECT id, board_name, title, created_at FROM threads WHERE id = ?")
            .bind(id)
            .fetch_one(&state.pool)
            .await?;

    let created_at: String = thread_row.get("created_at");
    let messages_rows = sqlx::query(
        r#"
        SELECT m.id, m.thread_id, m.author, m.content, m.created_at,
               mk.kind as marker_kind, mk.value as marker_value
        FROM messages m
        LEFT JOIN message_markers mk ON mk.message_id = m.id
        WHERE m.thread_id = ?
        ORDER BY m.created_at ASC
    "#,
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    let mut messages: Vec<Message> = Vec::new();
    for row in messages_rows {
        let message_id: i64 = row.get("id");
        if let Some(existing) = messages.iter_mut().find(|m| m.id == message_id) {
            if let Some(kind) = row.get::<Option<String>, _>("marker_kind") {
                if let Some(value) = row.get::<Option<String>, _>("marker_value") {
                    existing
                        .markers
                        .push(crate::server::models::Marker { kind, value });
                }
            }
            continue;
        }
        let created_at: String = row.get("created_at");
        let mut markers = Vec::new();
        if let Some(kind) = row.get::<Option<String>, _>("marker_kind") {
            if let Some(value) = row.get::<Option<String>, _>("marker_value") {
                markers.push(crate::server::models::Marker { kind, value });
            }
        }
        messages.push(Message {
            id: message_id,
            thread_id: row.get("thread_id"),
            author: row.get("author"),
            content: row.get("content"),
            created_at: created_at
                .parse()
                .map_err(|_| AppError::BadRequest("invalid timestamp in database".into()))?,
            markers,
        });
    }

    Ok(Json(ThreadDetail {
        id: thread_row.get("id"),
        board_name: thread_row.get("board_name"),
        title: thread_row.get("title"),
        created_at: created_at
            .parse()
            .map_err(|_| AppError::BadRequest("invalid timestamp in database".into()))?,
        messages,
    }))
}

async fn add_message(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<i64>,
    Json(payload): Json<CreateMessageRequest>,
) -> AppResult<Json<Message>> {
    if payload.author.trim().is_empty() {
        return Err(AppError::BadRequest("author cannot be empty".into()));
    }
    if payload.content.trim().is_empty() {
        return Err(AppError::BadRequest("content cannot be empty".into()));
    }

    let created_at = Utc::now();
    let mut tx = state.pool.begin().await?;

    let message_id = sqlx::query_scalar(
        "INSERT INTO messages (thread_id, author, content, created_at) VALUES (?, ?, ?, ?) RETURNING id",
    )
    .bind(id)
    .bind(&payload.author)
    .bind(&payload.content)
    .bind(created_at)
    .fetch_one(&mut *tx)
    .await?;

    store_markers(&mut tx, message_id, &payload.content).await?;
    tx.commit().await?;

    Ok(Json(Message {
        id: message_id,
        thread_id: id,
        author: payload.author,
        content: payload.content,
        created_at,
        markers: Vec::new(),
    }))
}

pub async fn store_markers(
    tx: &mut Transaction<'_, Sqlite>,
    message_id: i64,
    content: &str,
) -> AppResult<()> {
    let markers = extract_markers(content);
    for (kind, value) in markers {
        sqlx::query("INSERT INTO message_markers (message_id, kind, value) VALUES (?, ?, ?)")
            .bind(message_id)
            .bind(&kind)
            .bind(&value)
            .execute(&mut **tx)
            .await?;
    }
    Ok(())
}

fn extract_markers(content: &str) -> Vec<(String, String)> {
    static MARKER: once_cell::sync::Lazy<Regex> = once_cell::sync::Lazy::new(|| {
        Regex::new(r"(?P<kind>ctx|project|mode|bridge)::(?P<value>[\w\-/]+)")
            .expect("valid marker regex")
    });
    MARKER
        .captures_iter(content)
        .filter_map(|cap| {
            let kind = cap.name("kind")?.as_str().to_string();
            let value = cap.name("value")?.as_str().to_string();
            Some((kind, value))
        })
        .collect()
}

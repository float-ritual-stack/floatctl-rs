use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use sqlx::Row;
use tracing::info;

use crate::server::error::{AppError, AppResult};
use crate::server::models::{Board, CreateBoardRequest, ThreadSummary};
use crate::server::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_boards).post(create_board))
        .route("/:name", get(get_board))
        .route(
            "/:name/threads",
            get(list_threads_for_board).post(create_thread_for_board),
        )
}

async fn list_boards(State(state): State<AppState>) -> AppResult<Json<Vec<Board>>> {
    let rows = sqlx::query("SELECT name, created_at FROM boards ORDER BY created_at DESC")
        .fetch_all(&state.pool)
        .await?;

    let boards = rows
        .into_iter()
        .filter_map(|row| {
            let created_at: String = row.get("created_at");
            let parsed = created_at.parse().ok()?;
            Some(Board {
                name: row.get("name"),
                created_at: parsed,
            })
        })
        .collect();

    Ok(Json(boards))
}

async fn create_board(
    State(state): State<AppState>,
    Json(payload): Json<CreateBoardRequest>,
) -> AppResult<Json<Board>> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest("board name cannot be empty".into()));
    }
    let created_at = Utc::now();
    sqlx::query("INSERT OR IGNORE INTO boards (name, created_at) VALUES (?, ?)")
        .bind(&payload.name)
        .bind(created_at)
        .execute(&state.pool)
        .await?;

    info!("created board" = %payload.name);

    Ok(Json(Board {
        name: payload.name,
        created_at,
    }))
}

async fn get_board(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> AppResult<Json<Vec<ThreadSummary>>> {
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.board_name, t.title, t.created_at, COUNT(m.id) as message_count
        FROM threads t
        LEFT JOIN messages m ON m.thread_id = t.id
        WHERE t.board_name = ?
        GROUP BY t.id
        ORDER BY t.created_at DESC
        LIMIT 20
    "#,
    )
    .bind(&name)
    .fetch_all(&state.pool)
    .await?;

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

use crate::server::models::CreateThreadRequest;

async fn create_thread_for_board(
    State(state): State<AppState>,
    axum::extract::Path(board): axum::extract::Path<String>,
    Json(payload): Json<CreateThreadRequest>,
) -> AppResult<Json<ThreadSummary>> {
    if payload.title.trim().is_empty() {
        return Err(AppError::BadRequest("thread title cannot be empty".into()));
    }

    let created_at = Utc::now();
    let mut tx = state.pool.begin().await?;

    let thread_id = sqlx::query_scalar(
        "INSERT INTO threads (board_name, title, created_at) VALUES (?, ?, ?) RETURNING id",
    )
    .bind(&board)
    .bind(&payload.title)
    .bind(created_at)
    .fetch_one(&mut *tx)
    .await?;

    sqlx::query(
        "INSERT INTO messages (thread_id, author, content, created_at) VALUES (?, ?, ?, ?)",
    )
    .bind(thread_id)
    .bind(&payload.author)
    .bind(&payload.content)
    .bind(created_at)
    .execute(&mut *tx)
    .await?;

    super::threads::store_markers(&mut tx, thread_id, &payload.content).await?;
    tx.commit().await?;

    Ok(Json(ThreadSummary {
        id: thread_id,
        board_name: board,
        title: payload.title,
        created_at,
        message_count: 1,
    }))
}

async fn list_threads_for_board(
    State(state): State<AppState>,
    axum::extract::Path(board): axum::extract::Path<String>,
) -> AppResult<Json<Vec<ThreadSummary>>> {
    let rows = sqlx::query(
        r#"
        SELECT t.id, t.board_name, t.title, t.created_at, COUNT(m.id) as message_count
        FROM threads t
        LEFT JOIN messages m ON m.thread_id = t.id
        WHERE t.board_name = ?
        GROUP BY t.id
        ORDER BY t.created_at DESC
    "#,
    )
    .bind(&board)
    .fetch_all(&state.pool)
    .await?;

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

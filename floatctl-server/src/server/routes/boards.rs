use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use sqlx::Row;

use crate::server::{
    error::ServerResult,
    models::ts_to_datetime,
    models::{Board, BoardCreate, BoardWithThreads, Marker},
    routes::threads::load_thread_markers,
    AppState,
};

#[derive(Deserialize)]
pub struct BoardQuery {
    pub project: Option<String>,
}

pub async fn list_boards(State(state): State<Arc<AppState>>) -> ServerResult<Json<Vec<Board>>> {
    let rows = sqlx::query!("SELECT name, created_at FROM boards ORDER BY created_at DESC")
        .fetch_all(&state.pool)
        .await?;

    let boards = rows
        .into_iter()
        .map(|row| Board {
            name: row.name,
            created_at: ts_to_datetime(row.created_at),
        })
        .collect();

    Ok(Json(boards))
}

pub async fn create_board(
    State(state): State<Arc<AppState>>,
    Json(body): Json<BoardCreate>,
) -> ServerResult<Json<Board>> {
    sqlx::query!("INSERT OR IGNORE INTO boards (name) VALUES (?)", body.name)
        .execute(&state.pool)
        .await?;

    let row = sqlx::query!(
        "SELECT name, created_at FROM boards WHERE name = ?",
        body.name
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(Board {
        name: row.name,
        created_at: ts_to_datetime(row.created_at),
    }))
}

pub async fn get_board(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Query(query): Query<BoardQuery>,
) -> ServerResult<Json<BoardWithThreads>> {
    let board_row = sqlx::query!("SELECT name, created_at FROM boards WHERE name = ?", name)
        .fetch_one(&state.pool)
        .await?;

    let threads = load_threads(&state, &board_row.name, query.project.as_deref()).await?;

    Ok(Json(BoardWithThreads {
        board: Board {
            name: board_row.name,
            created_at: ts_to_datetime(board_row.created_at),
        },
        threads,
    }))
}

async fn load_threads(
    state: &Arc<AppState>,
    board: &str,
    project: Option<&str>,
) -> ServerResult<Vec<crate::server::models::Thread>> {
    let mut query = String::from(
        "SELECT id, board_name, title, author, created_at FROM threads WHERE board_name = ?",
    );
    if project.is_some() {
        query.push_str(" AND id IN (SELECT thread_id FROM thread_markers WHERE kind = 'project' AND value = ?)");
    }
    query.push_str(" ORDER BY created_at DESC");

    let rows = if let Some(project) = project {
        sqlx::query(&query)
            .bind(board)
            .bind(project)
            .fetch_all(&state.pool)
            .await?
    } else {
        sqlx::query(&query)
            .bind(board)
            .fetch_all(&state.pool)
            .await?
    };

    let mut threads = Vec::new();
    for row in rows {
        let id: i64 = row.try_get("id")?;
        let markers: Vec<Marker> = load_thread_markers(&state.pool, id).await?;
        threads.push(crate::server::models::Thread {
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

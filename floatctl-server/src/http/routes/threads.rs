//! Thread endpoints - Spec 2.3

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::repos::{ThreadRepo, ThreadWithCount, Thread};
use crate::http::error::ApiError;
use crate::http::server::AppState;
use crate::models::{BoardName, ThreadTitle, MessageContent, Paginated, Pagination, PaginationParams};

/// Create thread request
#[derive(Deserialize)]
pub struct CreateThreadRequest {
    pub title: String,
    pub first_message: Option<FirstMessageRequest>,
}

/// First message in thread
#[derive(Deserialize)]
pub struct FirstMessageRequest {
    pub content: String,
    pub author: Option<String>,
}

/// Thread response
#[derive(Serialize)]
pub struct ThreadResponse {
    pub id: Uuid,
    pub board_name: String,
    pub title: String,
    pub created_at: String,
    pub message_count: Option<i64>,
}

impl From<Thread> for ThreadResponse {
    fn from(t: Thread) -> Self {
        Self {
            id: t.id,
            board_name: t.board_name,
            title: t.title,
            created_at: t.created_at.to_rfc3339(),
            message_count: None,
        }
    }
}

impl From<ThreadWithCount> for ThreadResponse {
    fn from(t: ThreadWithCount) -> Self {
        Self {
            id: t.id,
            board_name: t.board_name,
            title: t.title,
            created_at: t.created_at.to_rfc3339(),
            message_count: Some(t.message_count),
        }
    }
}

/// GET /boards/{name}/threads - list threads for a board
async fn list_threads(
    State(state): State<Arc<AppState>>,
    Path(board_name): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<ThreadResponse>>, ApiError> {
    let page = Pagination::from(params);
    let result = ThreadRepo::new(&state.pool)
        .list_for_board(&board_name, page)
        .await?;

    Ok(Json(Paginated {
        items: result.items.into_iter().map(ThreadResponse::from).collect(),
        total: result.total,
        page: result.page,
        per_page: result.per_page,
    }))
}

/// POST /boards/{name}/threads - create a new thread
async fn create_thread(
    State(state): State<Arc<AppState>>,
    Path(board_name): Path<String>,
    Json(req): Json<CreateThreadRequest>,
) -> Result<(StatusCode, Json<ThreadResponse>), ApiError> {
    let board = BoardName::new(&board_name)?;
    let title = ThreadTitle::new(&req.title)?;

    let first_message = if let Some(msg) = req.first_message {
        let content = MessageContent::new(&msg.content)?;
        Some((content, msg.author))
    } else {
        None
    };

    let thread = ThreadRepo::new(&state.pool)
        .create_with_message(board, title, first_message)
        .await?;

    Ok((StatusCode::CREATED, Json(ThreadResponse::from(thread))))
}

/// GET /threads/{id} - get a single thread
async fn get_thread(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<ThreadResponse>, ApiError> {
    let thread = ThreadRepo::new(&state.pool).get(id).await?;
    Ok(Json(ThreadResponse::from(thread)))
}

/// Thread routes
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/boards/{name}/threads", get(list_threads).post(create_thread))
        .route("/threads/{id}", get(get_thread))
}

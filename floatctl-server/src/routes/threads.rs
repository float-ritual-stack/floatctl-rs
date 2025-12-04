//! Thread routes - Threaded discussions within boards

use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use crate::db::Database;
use crate::error::{ServerError, ServerResult};
use crate::models::{
    CreateMessageRequest, CreateThreadRequest, Message, Thread, ThreadQueryParams, ThreadSummary,
};

/// GET /boards/:name/threads - List threads in a board
pub async fn list_threads(
    State(db): State<Database>,
    Path(name): Path<String>,
    Query(params): Query<ThreadQueryParams>,
) -> ServerResult<Json<Vec<ThreadSummary>>> {
    let board = db
        .get_board(&name)?
        .ok_or_else(|| ServerError::NotFound(format!("Board '{}' not found", name)))?;

    let limit = params.limit.unwrap_or(50);
    let offset = params.offset.unwrap_or(0);

    let threads = db.list_threads_for_board(board.id, limit, offset)?;
    Ok(Json(threads))
}

/// POST /boards/:name/threads - Create a new thread
pub async fn create_thread(
    State(db): State<Database>,
    Path(name): Path<String>,
    Json(req): Json<CreateThreadRequest>,
) -> ServerResult<Json<Thread>> {
    if req.title.is_empty() {
        return Err(ServerError::BadRequest("Thread title cannot be empty".into()));
    }

    let board = db
        .get_board(&name)?
        .ok_or_else(|| ServerError::NotFound(format!("Board '{}' not found", name)))?;

    let thread = db.create_thread(board.id, &req)?;
    Ok(Json(thread))
}

/// GET /threads/:id - Get thread with all messages
pub async fn get_thread(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
) -> ServerResult<Json<Thread>> {
    let thread = db
        .get_thread(id)?
        .ok_or_else(|| ServerError::NotFound(format!("Thread {} not found", id)))?;

    Ok(Json(thread))
}

/// POST /threads/:id/messages - Add a message to thread
pub async fn add_message(
    State(db): State<Database>,
    Path(id): Path<Uuid>,
    Json(req): Json<CreateMessageRequest>,
) -> ServerResult<Json<Message>> {
    if req.content.is_empty() {
        return Err(ServerError::BadRequest(
            "Message content cannot be empty".into(),
        ));
    }

    let message = db.add_message(id, &req)?;
    Ok(Json(message))
}

/// GET /threads - Search/list all threads (with optional filters)
pub async fn search_threads(
    State(db): State<Database>,
    Query(params): Query<ThreadQueryParams>,
) -> ServerResult<Json<Vec<ThreadSummary>>> {
    let threads = db.search_threads(&params)?;
    Ok(Json(threads))
}

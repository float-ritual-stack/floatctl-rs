use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};

use crate::server::{
    error::{AppError, AppResult},
    models::{CreateMessageRequest, ThreadDetail, ThreadQuery, ThreadSummary},
    AppState,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(list_threads))
        .route("/:id", get(get_thread))
        .route("/:id/messages", post(add_message))
        .with_state(state)
}

async fn list_threads(
    State(state): State<AppState>,
    Query(filter): Query<ThreadQuery>,
) -> AppResult<Json<Vec<ThreadSummary>>> {
    let threads = state.db.list_threads(None, filter).await?;
    Ok(Json(threads))
}

async fn get_thread(
    Path(id): Path<i64>,
    State(state): State<AppState>,
) -> AppResult<Json<ThreadDetail>> {
    let thread = state
        .db
        .get_thread(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("thread {id}")))?;
    Ok(Json(thread))
}

async fn add_message(
    Path(id): Path<i64>,
    State(state): State<AppState>,
    Json(body): Json<CreateMessageRequest>,
) -> AppResult<Json<ThreadDetail>> {
    state.db.add_message(id, body).await?;
    let thread = state
        .db
        .get_thread(id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("thread {id}")))?;
    Ok(Json(thread))
}

//! Board endpoints - Spec 2.2

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::db::repos::{BoardRepo, BoardWithCount};
use crate::http::error::ApiError;
use crate::http::server::AppState;
use crate::models::{BoardName, Paginated, Pagination, PaginationParams};

/// Create board request
#[derive(Deserialize)]
pub struct CreateBoardRequest {
    pub name: String,
}

/// Board response
#[derive(Serialize)]
pub struct BoardResponse {
    pub name: String,
    pub created_at: String,
    pub thread_count: i64,
}

impl From<BoardWithCount> for BoardResponse {
    fn from(b: BoardWithCount) -> Self {
        Self {
            name: b.name,
            created_at: b.created_at.to_rfc3339(),
            thread_count: b.thread_count,
        }
    }
}

/// GET /boards - list all boards with pagination
async fn list_boards(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<BoardResponse>>, ApiError> {
    let page = Pagination::from(params);
    let result = BoardRepo::new(&state.pool).list(page).await?;

    Ok(Json(Paginated {
        items: result.items.into_iter().map(BoardResponse::from).collect(),
        total: result.total,
        page: result.page,
        per_page: result.per_page,
    }))
}

/// POST /boards - create a new board
async fn create_board(
    State(state): State<Arc<AppState>>,
    Json(req): Json<CreateBoardRequest>,
) -> Result<(StatusCode, Json<BoardResponse>), ApiError> {
    let name = BoardName::new(&req.name)?;
    // create() returns BoardWithCount directly (single query with CTE + JOIN)
    let board = BoardRepo::new(&state.pool).create(name).await?;

    Ok((StatusCode::CREATED, Json(BoardResponse::from(board))))
}

/// GET /boards/{name} - get a single board
async fn get_board(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<BoardResponse>, ApiError> {
    let board = BoardRepo::new(&state.pool).get(&name).await?;
    Ok(Json(BoardResponse::from(board)))
}

/// Board routes
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/boards", get(list_boards).post(create_board))
        .route("/boards/{name}", get(get_board))
}

#[cfg(test)]
mod tests {
    // Integration tests with test database
    // Run with: DATABASE_URL=... cargo test -p floatctl-server -- --ignored
}

//! Board routes - Message boards (workspace areas)

use axum::{
    extract::{Path, State},
    Json,
};

use crate::db::Database;
use crate::error::{ServerError, ServerResult};
use crate::models::{Board, BoardWithThreads, CreateBoardRequest};

/// GET /boards - List all boards
pub async fn list_boards(State(db): State<Database>) -> ServerResult<Json<Vec<Board>>> {
    let boards = db.list_boards()?;
    Ok(Json(boards))
}

/// POST /boards - Create a new board
pub async fn create_board(
    State(db): State<Database>,
    Json(req): Json<CreateBoardRequest>,
) -> ServerResult<Json<Board>> {
    if req.name.is_empty() {
        return Err(ServerError::BadRequest("Board name cannot be empty".into()));
    }

    // Validate board name (alphanumeric, dashes, underscores)
    if !req
        .name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
    {
        return Err(ServerError::BadRequest(
            "Board name must be alphanumeric with dashes/underscores".into(),
        ));
    }

    let board = db.create_board(&req)?;
    Ok(Json(board))
}

/// GET /boards/:name - Get board with recent threads
pub async fn get_board(
    State(db): State<Database>,
    Path(name): Path<String>,
) -> ServerResult<Json<BoardWithThreads>> {
    let board = db
        .get_board_with_threads(&name, 20)?
        .ok_or_else(|| ServerError::NotFound(format!("Board '{}' not found", name)))?;

    Ok(Json(board))
}

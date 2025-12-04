use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use futures::future::try_join_all;

use crate::server::{
    error::{AppError, AppResult},
    models::{Board, CreateBoardRequest, CreateThreadRequest, ThreadDetail, ThreadQuery},
    AppState,
};

#[derive(serde::Serialize)]
struct BoardWithThreads {
    board: Board,
    threads: Vec<ThreadDetail>,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(list_boards).post(create_board))
        .route("/:name", get(get_board))
        .route(
            "/:name/threads",
            get(list_board_threads).post(create_thread),
        )
        .with_state(state)
}

async fn list_boards(State(state): State<AppState>) -> AppResult<Json<Vec<Board>>> {
    let boards = state.db.list_boards().await?;
    Ok(Json(boards))
}

async fn create_board(
    State(state): State<AppState>,
    Json(body): Json<CreateBoardRequest>,
) -> AppResult<Json<Board>> {
    let board = state.db.create_board(&body.name).await?;
    Ok(Json(board))
}

async fn get_board(
    Path(name): Path<String>,
    State(state): State<AppState>,
) -> AppResult<Json<BoardWithThreads>> {
    let board = state
        .db
        .get_board(&name)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("board {name}")))?;

    let threads = state
        .db
        .list_threads(Some(&name), ThreadQuery { project: None })
        .await?;

    let db = state.db.clone();
    let details = try_join_all(threads.into_iter().map(|t| {
        let db = db.clone();
        async move {
            db.get_thread(t.id)
                .await?
                .ok_or_else(|| AppError::NotFound("thread missing".into()))
        }
    }))
    .await?;

    Ok(Json(BoardWithThreads {
        board,
        threads: details,
    }))
}

async fn list_board_threads(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Query(filter): Query<ThreadQuery>,
) -> AppResult<Json<Vec<ThreadDetail>>> {
    let threads = state.db.list_threads(Some(&name), filter).await?;

    let db = state.db.clone();
    let details = try_join_all(threads.into_iter().map(|t| {
        let db = db.clone();
        async move {
            db.get_thread(t.id)
                .await?
                .ok_or_else(|| AppError::NotFound("thread missing".into()))
        }
    }))
    .await?;

    Ok(Json(details))
}

async fn create_thread(
    Path(name): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<CreateThreadRequest>,
) -> AppResult<Json<ThreadDetail>> {
    let thread = state.db.create_thread(&name, body).await?;
    Ok(Json(thread))
}

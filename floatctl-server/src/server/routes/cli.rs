use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};

use crate::server::{
    error::AppResult,
    models::{CliRequest, CliResponse},
    AppState,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/:command", post(run_command))
        .with_state(state)
}

async fn run_command(
    Path(command): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<CliRequest>,
) -> AppResult<Json<CliResponse>> {
    let response = state.cli.invoke(&command, body).await?;
    Ok(Json(response))
}

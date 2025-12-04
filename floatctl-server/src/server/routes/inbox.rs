use axum::{
    extract::{Path, State},
    routing::{delete, get, post},
    Json, Router,
};

use crate::server::{
    error::AppResult,
    models::{InboxMessage, SendInboxRequest},
    AppState,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/:persona", get(fetch_inbox).post(send_inbox))
        .route("/:persona/:id", delete(delete_inbox))
        .with_state(state)
}

async fn fetch_inbox(
    Path(persona): Path<String>,
    State(state): State<AppState>,
) -> AppResult<Json<Vec<InboxMessage>>> {
    let items = state.db.fetch_inbox(&persona).await?;
    Ok(Json(items))
}

async fn send_inbox(
    Path(persona): Path<String>,
    State(state): State<AppState>,
    Json(body): Json<SendInboxRequest>,
) -> AppResult<Json<InboxMessage>> {
    let item = state.db.send_inbox(&persona, &body.content).await?;
    Ok(Json(item))
}

async fn delete_inbox(
    Path((persona, id)): Path<(String, i64)>,
    State(state): State<AppState>,
) -> AppResult<()> {
    state.db.delete_inbox(&persona, id).await?;
    Ok(())
}

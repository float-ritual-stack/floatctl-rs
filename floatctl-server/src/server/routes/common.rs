use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};

use crate::server::{
    error::AppResult,
    models::{CommonItem, CommonItemRequest},
    AppState,
};

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(list_common).post(put_common))
        .route("/:key", get(get_common))
        .with_state(state)
}

async fn list_common(State(state): State<AppState>) -> AppResult<Json<Vec<CommonItem>>> {
    let items = state.db.list_common().await?;
    Ok(Json(items))
}

async fn put_common(
    State(state): State<AppState>,
    Json(body): Json<CommonItemRequest>,
) -> AppResult<Json<CommonItem>> {
    let (key, value, ttl) = body.resolved_key();
    let item = state.db.put_common(&key, value, ttl).await?;
    Ok(Json(item))
}

async fn get_common(
    Path(key): Path<String>,
    State(state): State<AppState>,
) -> AppResult<Json<CommonItem>> {
    let Some(item) = state.db.get_common(&key).await? else {
        return Err(crate::server::error::AppError::NotFound(format!(
            "common item {key}"
        )));
    };
    Ok(Json(item))
}

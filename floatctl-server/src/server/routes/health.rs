use std::sync::Arc;

use axum::{response::IntoResponse, Json};

use crate::server::{models::HealthResponse, AppState};

pub async fn health(_state: axum::extract::State<Arc<AppState>>) -> impl IntoResponse {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

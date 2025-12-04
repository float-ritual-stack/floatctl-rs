use axum::{routing::get, Json, Router};

use crate::server::error::AppResult;

#[derive(serde::Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn router() -> Router {
    Router::new().route("/health", get(health))
}

async fn health() -> AppResult<Json<HealthResponse>> {
    Ok(Json(HealthResponse { status: "ok" }))
}

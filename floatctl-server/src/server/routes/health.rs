use axum::{routing::get, Json, Router};
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    status: &'static str,
}

pub fn router() -> Router {
    Router::new().route(
        "/health",
        get(|| async { Json(HealthResponse { status: "ok" }) }),
    )
}

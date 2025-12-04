//! Health check route

use std::sync::Arc;
use std::time::Instant;

use axum::{extract::Extension, Json};
use tokio::sync::RwLock;

use crate::db::Database;
use crate::models::{DatabaseHealth, HealthResponse};

/// Server start time for uptime calculation
pub struct ServerState {
    pub db: Database,
    pub start_time: Instant,
}

impl ServerState {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            start_time: Instant::now(),
        }
    }
}

/// Shared state wrapper
pub type SharedState = Arc<RwLock<ServerState>>;

/// GET /health - Health check endpoint
pub async fn health_check(Extension(state): Extension<SharedState>) -> Json<HealthResponse> {
    let state = state.read().await;
    let uptime = state.start_time.elapsed();

    let db_path = state.db.path().display().to_string();
    let db_size = state.db.size_bytes();

    Json(HealthResponse {
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime.as_secs(),
        database: DatabaseHealth {
            connected: true,
            path: db_path,
            size_bytes: db_size,
        },
    })
}

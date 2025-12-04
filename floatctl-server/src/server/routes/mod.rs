use std::sync::Arc;

use axum::{
    routing::{delete, get, post},
    Router,
};

use super::AppState;
use crate::server::routes::{boards::*, cli::*, common::*, health::*, inbox::*, threads::*};

pub mod boards;
pub mod cli;
pub mod common;
pub mod health;
pub mod inbox;
pub mod threads;

pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/boards", get(list_boards).post(create_board))
        .route("/boards/:name", get(get_board))
        .route(
            "/boards/:name/threads",
            get(list_board_threads).post(create_thread),
        )
        .route("/threads", get(list_threads))
        .route("/threads/:id", get(get_thread))
        .route("/threads/:id/messages", post(add_message))
        .route("/inbox/:persona", get(get_inbox).post(send_inbox))
        .route("/inbox/:persona/:id", delete(delete_inbox_message))
        .route("/common", get(list_common).post(create_common))
        .route("/common/:key", get(get_common))
        .route("/cli/:command", post(run_command))
        .with_state(state)
}

use axum::Router;

use crate::server::AppState;

mod boards;
mod cli;
mod common;
mod health;
mod inbox;
mod threads;

pub fn build_router(state: AppState) -> Router {
    let boards_router = boards::router(state.clone());
    let threads_router = threads::router(state.clone());
    let inbox_router = inbox::router(state.clone());
    let common_router = common::router(state.clone());
    let cli_router = cli::router(state.clone());

    Router::new()
        .merge(health::router())
        .nest("/boards", boards_router)
        .nest("/threads", threads_router)
        .nest("/inbox", inbox_router)
        .nest("/common", common_router)
        .nest("/cli", cli_router)
}

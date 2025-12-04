//! BBS routes

use axum::routing::{get, patch, post};
use axum::Router;

use crate::bbs::handlers;
use crate::state::AppState;

/// BBS routes: /bbs/*
pub fn bbs_router() -> Router<AppState> {
    Router::new()
        // Boards
        .route("/boards", get(handlers::list_boards).post(handlers::create_board))
        .route(
            "/boards/{slug}",
            get(handlers::get_board)
                .patch(handlers::update_board)
                .delete(handlers::delete_board),
        )
        // Threads (within a board)
        .route("/boards/{board_slug}/threads", get(handlers::list_threads))
        // Threads (direct access)
        .route("/threads", post(handlers::create_thread))
        .route(
            "/threads/{thread_id}",
            get(handlers::get_thread).patch(handlers::update_thread),
        )
        // Posts
        .route("/threads/{thread_id}/posts", get(handlers::list_posts))
        .route("/posts", post(handlers::create_post))
        .route(
            "/posts/{post_id}",
            patch(handlers::update_post).delete(handlers::delete_post),
        )
        // Inbox
        .route("/inbox/{recipient}", get(handlers::list_inbox))
        .route("/inbox/{recipient}/stats", get(handlers::inbox_stats))
        .route("/inbox", post(handlers::send_message))
        .route("/inbox/{msg_id}/read", post(handlers::mark_read))
        .route("/inbox/{msg_id}/archive", post(handlers::archive_message))
        // Commons
        .route("/commons", get(handlers::list_commons).post(handlers::create_common))
        .route("/commons/{slug}", get(handlers::get_common))
        .route("/commons/{common_slug}/artifacts", get(handlers::list_artifacts))
        .route("/artifacts", post(handlers::create_artifact))
}

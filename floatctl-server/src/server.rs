//! Main server module - Axum setup and router configuration
//!
//! Starts an HTTP server with BBS routes and CLI wrapper endpoints.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use axum::{
    routing::{delete, get, post},
    Router,
};
use clap::Parser;
use tokio::net::TcpListener;
use tokio::signal;
use tokio::sync::RwLock;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{info, warn};

use crate::db::Database;
use crate::routes::{self, health::ServerState};

/// Server command-line arguments
#[derive(Parser, Debug, Clone)]
pub struct ServerArgs {
    /// Port to listen on
    #[arg(short, long, default_value = "3030")]
    pub port: u16,

    /// Bind address
    #[arg(short, long, default_value = "127.0.0.1")]
    pub bind: String,

    /// Database file path (default: ~/.floatctl/bbs.db)
    #[arg(long)]
    pub db_path: Option<PathBuf>,

    /// Request timeout in seconds
    #[arg(long, default_value = "30")]
    pub timeout: u64,
}

impl Default for ServerArgs {
    fn default() -> Self {
        Self {
            port: 3030,
            bind: "127.0.0.1".to_string(),
            db_path: None,
            timeout: 30,
        }
    }
}

/// Run the server with the given arguments
pub async fn run_server(args: ServerArgs) -> anyhow::Result<()> {
    // Determine database path
    let db_path = args.db_path.unwrap_or_else(|| {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".floatctl")
            .join("bbs.db")
    });

    info!("Opening database at {}", db_path.display());
    let db = Database::open(&db_path)?;

    // Create shared state
    let state = Arc::new(RwLock::new(ServerState::new(db.clone())));

    // Build router
    let app = create_router(db, state, args.timeout);

    // Bind address
    let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
        .parse()
        .expect("Invalid bind address");

    info!("Starting floatctl-server on http://{}", addr);
    info!("Database: {}", db_path.display());

    // Create listener
    let listener = TcpListener::bind(addr).await?;

    // Run with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    info!("Server shutdown complete");
    Ok(())
}

/// Create the Axum router with all routes
fn create_router(
    db: Database,
    state: Arc<RwLock<ServerState>>,
    timeout_secs: u64,
) -> Router {
    // CORS layer for local development
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Middleware stack
    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(timeout_secs)))
        .layer(cors);

    // Build routes
    Router::new()
        // Health
        .route("/health", get(routes::health_check))
        // Boards
        .route("/boards", get(routes::list_boards).post(routes::create_board))
        .route("/boards/{name}", get(routes::get_board))
        .route(
            "/boards/{name}/threads",
            get(routes::list_threads).post(routes::create_thread),
        )
        // Threads
        .route("/threads", get(routes::search_threads))
        .route("/threads/{id}", get(routes::get_thread))
        .route("/threads/{id}/messages", post(routes::add_message))
        // Inbox
        .route(
            "/inbox/{persona}",
            get(routes::list_inbox).post(routes::send_to_inbox),
        )
        .route("/inbox/{persona}/{id}", delete(routes::delete_inbox_message))
        .route("/inbox/{persona}/{id}/read", post(routes::mark_inbox_read))
        // Common
        .route(
            "/common",
            get(routes::list_common).post(routes::create_common),
        )
        .route(
            "/common/{key}",
            get(routes::get_common)
                .put(routes::update_common)
                .delete(routes::delete_common),
        )
        .route("/common/_cleanup", post(routes::cleanup_common))
        // CLI wrapper
        .route("/cli", get(routes::list_cli_commands))
        .route("/cli/{command}", post(routes::execute_cli))
        // State
        .with_state(db)
        // Health needs full state for uptime
        .layer(axum::Extension(state))
        .layer(middleware)
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            warn!("Received Ctrl+C, initiating graceful shutdown...");
        }
        _ = terminate => {
            warn!("Received SIGTERM, initiating graceful shutdown...");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_health_endpoint() {
        let db = Database::open_in_memory().unwrap();
        let state = Arc::new(RwLock::new(ServerState::new(db.clone())));
        let app = create_router(db, state, 30);

        let response = app
            .oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_boards_crud() {
        let db = Database::open_in_memory().unwrap();
        let state = Arc::new(RwLock::new(ServerState::new(db.clone())));
        let app = create_router(db, state, 30);

        // Create board
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/boards")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"name": "test-board", "description": "Test"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // List boards
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/boards").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

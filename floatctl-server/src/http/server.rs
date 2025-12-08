//! Axum server setup - Spec 2.1
//!
//! Server skeleton with:
//! - Localhost-only CORS by default
//! - Tracing middleware
//! - Graceful shutdown on SIGTERM/Ctrl+C

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use super::routes;
use crate::bbs::BbsConfig;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Address to bind to (default: 127.0.0.1:3030)
    pub bind_addr: SocketAddr,

    /// Allow permissive CORS (default: false = localhost only)
    ///
    /// WARNING: Setting this to true allows any origin.
    /// Only use for development or documented use cases.
    pub cors_permissive: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            bind_addr: SocketAddr::from(([127, 0, 0, 1], 3030)),
            cors_permissive: false,
        }
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    /// BBS configuration (file-based bulletin board)
    pub bbs_config: BbsConfig,
}

/// Run the HTTP server.
///
/// # Example
///
/// ```ignore
/// let pool = create_pool(&database_url).await?;
/// let config = ServerConfig::default();
/// run_server(pool, config).await?;
/// ```
pub async fn run_server(pool: PgPool, config: ServerConfig) -> Result<(), ServerError> {
    let bbs_config = BbsConfig::from_env();
    tracing::info!(bbs_root = %bbs_config.root_dir.display(), "BBS config loaded");
    let state = AppState { pool, bbs_config };

    // CORS configuration
    let cors = if config.cors_permissive {
        tracing::warn!("CORS: Permissive mode enabled - all origins allowed");
        CorsLayer::permissive()
    } else {
        // Localhost only
        CorsLayer::new()
            .allow_origin([
                "http://localhost:3000".parse().unwrap(),
                "http://localhost:3030".parse().unwrap(),
                "http://127.0.0.1:3000".parse().unwrap(),
                "http://127.0.0.1:3030".parse().unwrap(),
            ])
            .allow_methods(Any)
            .allow_headers(Any)
    };

    // Build router
    let app = Router::new()
        .merge(routes::health::router())
        .merge(routes::boards::router())
        .merge(routes::threads::router())
        .merge(routes::messages::router())
        .merge(routes::inbox::router())
        .merge(routes::scratchpad::router())
        .merge(routes::cli::router())
        .merge(routes::dispatch::router())
        .merge(routes::bbs_api::router())
        .merge(routes::magic::router())
        .merge(routes::status::router())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(Arc::new(state));

    // Bind listener
    let listener = TcpListener::bind(config.bind_addr).await?;
    tracing::info!("Server listening on {}", config.bind_addr);

    // Run with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server shutdown complete");
    Ok(())
}

/// Wait for shutdown signal (Ctrl+C or SIGTERM).
async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, starting shutdown");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting shutdown");
        }
    }
}

/// Server error type
#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.bind_addr.port(), 3030);
        assert!(!config.cors_permissive);
    }
}

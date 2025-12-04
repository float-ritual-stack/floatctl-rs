use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use axum::Router;
use once_cell::sync::Lazy;
use regex::Regex;
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::server::{
    db::{cleanup_common, init_pool, run_migrations},
    error::ServerResult,
    models::Marker,
};

pub mod db;
pub mod error;
pub mod models;
pub mod routes;

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
    pub binary_path: PathBuf,
}

#[derive(Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub database_path: PathBuf,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 3030,
            database_path: default_db_path(),
        }
    }
}

pub async fn run_server(config: ServerConfig) -> ServerResult<()> {
    if let Some(parent) = config.database_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let pool = init_pool(&config.database_path).await?;
    run_migrations(&pool).await?;
    cleanup_common(&pool).await?;

    let state = Arc::new(AppState {
        pool,
        binary_path: std::env::current_exe()?,
    });

    let cors = CorsLayer::permissive();
    let middleware = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(cors);

    let app = routes::router(state).layer(middleware);

    let addr: SocketAddr = ([0, 0, 0, 0], config.port).into();
    info!("starting floatctl server", %addr, db = %config.database_path.display());

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

fn default_db_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| Path::new(".").to_path_buf())
        .join(".floatctl")
        .join("server.sqlite")
}

pub fn extract_markers(text: &str) -> Vec<Marker> {
    static RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?P<key>[A-Za-z0-9_-]+)::(?P<value>[A-Za-z0-9_.-]+)").expect("valid regex")
    });
    let mut markers = Vec::new();
    for caps in RE.captures_iter(text) {
        let key = caps["key"].to_lowercase();
        if matches!(key.as_str(), "ctx" | "project" | "mode" | "bridge") {
            markers.push(Marker {
                kind: key,
                value: caps["value"].to_string(),
            });
        }
    }
    markers
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

use std::net::SocketAddr;
use std::path::PathBuf;

use anyhow::{Context, Result};
use axum::Router;
use tokio::signal;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{info, warn};

use crate::server::db::{init_pool, run_migrations};
use crate::server::routes::{boards, cli, common, health, inbox, threads};

pub mod db;
pub mod error;
pub mod models;
pub mod routes;

#[derive(Clone, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub db_path: PathBuf,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: sqlx::SqlitePool,
}

pub async fn run_server(config: ServerConfig) -> Result<()> {
    if let Some(parent) = config.db_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent).context("failed to create database directory")?;
        }
    }

    let pool = init_pool(&config.db_path).await?;
    run_migrations(&pool).await?;

    let state = AppState { pool };

    let app = Router::new()
        .merge(health::router())
        .nest("/boards", boards::router())
        .nest("/threads", threads::router())
        .nest("/inbox", inbox::router())
        .nest("/common", common::router())
        .nest("/cli", cli::router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("floatctl server listening on {}", addr);

    axum::serve(
        tokio::net::TcpListener::bind(addr).await?,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = signal::ctrl_c().await {
            warn!("failed to listen for ctrl+c: {err}");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        if let Ok(mut stream) = signal(SignalKind::terminate()) {
            let _ = stream.recv().await;
        }
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

pub fn default_db_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let db_dir = home.join(".floatctl");
    Ok(db_dir.join("floatctl-server.sqlite"))
}

impl ServerConfig {
    pub fn new(port: u16, db_path: Option<PathBuf>) -> Result<Self> {
        let path = match db_path {
            Some(path) => path,
            None => default_db_path()?,
        };
        Ok(Self {
            port,
            db_path: path,
        })
    }
}

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use axum::Router;
use tokio::signal;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

pub mod db;
pub mod error;
pub mod models;
mod routes;

use crate::server::routes::build_router;

#[async_trait]
pub trait CliInvoker: Send + Sync + 'static {
    async fn invoke(
        &self,
        command: &str,
        request: models::CliRequest,
    ) -> Result<models::CliResponse, error::AppError>;
}

#[derive(Clone)]
pub struct ProcessInvoker {
    binary: PathBuf,
}

impl ProcessInvoker {
    pub fn new(binary: PathBuf) -> Self {
        Self { binary }
    }
}

#[async_trait]
impl CliInvoker for ProcessInvoker {
    async fn invoke(
        &self,
        command: &str,
        request: models::CliRequest,
    ) -> Result<models::CliResponse, error::AppError> {
        use std::process::Stdio;

        use tokio::io::AsyncWriteExt;
        use tokio::process::Command;

        if command == "server" {
            return Err(error::AppError::BadRequest(
                "calling server subcommand from server is not supported".to_string(),
            ));
        }

        let mut cmd = Command::new(&self.binary);
        cmd.arg(command);
        if let Some(args) = request.args {
            cmd.args(args);
        }

        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(input) = request.stdin {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(input.as_bytes()).await?;
            }
        }

        let output = child.wait_with_output().await?;
        Ok(models::CliResponse {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: db::Database,
    pub cli: Arc<dyn CliInvoker>,
}

#[derive(Clone)]
pub struct ServerConfig {
    pub port: u16,
    pub db_path: PathBuf,
    pub cli: Arc<dyn CliInvoker>,
}

pub async fn run_server(config: ServerConfig) -> anyhow::Result<()> {
    let db = db::Database::connect(&config.db_path)
        .await
        .with_context(|| format!("failed to open database at {}", config.db_path.display()))?;

    let state = AppState {
        db,
        cli: config.cli.clone(),
    };

    let app = Router::new()
        .merge(build_router(state))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    info!("starting floatctl-server", %addr);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install CTRL+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        use tokio::signal::unix::{signal, SignalKind};
        let mut term = signal(SignalKind::terminate()).expect("failed to install signal handler");
        term.recv().await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

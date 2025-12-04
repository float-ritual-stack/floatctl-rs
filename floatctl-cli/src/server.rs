use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use clap::Parser;
use floatctl_server::server::{ProcessInvoker, ServerConfig};

#[derive(Parser, Debug)]
pub struct ServerArgs {
    /// Port to bind the HTTP server to
    #[arg(long, default_value_t = 3030)]
    pub port: u16,

    /// Path to the SQLite database backing the BBS
    #[arg(long)]
    pub db_path: Option<PathBuf>,
}

pub async fn run_server(args: ServerArgs) -> Result<()> {
    let db_path = args
        .db_path
        .unwrap_or_else(|| default_db_path().expect("home directory required"));

    let binary = std::env::current_exe().context("could not determine current executable")?;
    let config = ServerConfig {
        port: args.port,
        db_path,
        cli: Arc::new(ProcessInvoker::new(binary)),
    };

    floatctl_server::server::run_server(config).await?;
    Ok(())
}

fn default_db_path() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".floatctl").join("server.db"))
}

//! HTTP server command for floatctl BBS API
//!
//! Runs the floatctl HTTP server with all routes including dispatch capture.

use anyhow::{Context, Result};
use clap::Parser;
use std::net::SocketAddr;

use floatctl_server::db::create_pool;
use floatctl_server::http::{run_server, ServerConfig};

/// Arguments for the serve command
#[derive(Parser, Debug)]
pub struct ServeArgs {
    /// Address to bind to (default: 127.0.0.1:3030)
    #[arg(long, short = 'b', default_value = "127.0.0.1:3030")]
    pub bind: SocketAddr,

    /// Allow permissive CORS (all origins) - use with caution
    #[arg(long)]
    pub cors_permissive: bool,

    /// Database URL (overrides config/environment)
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: Option<String>,
}

/// Run the HTTP server
pub async fn run_serve(args: ServeArgs) -> Result<()> {
    // Load database URL from args, env, or config
    let database_url = args
        .database_url
        .or_else(|| std::env::var("DATABASE_URL").ok())
        .context("DATABASE_URL not set. Set via --database-url, DATABASE_URL env, or ~/.floatctl/.env")?;

    tracing::info!("Starting floatctl server on {}", args.bind);

    // Create database pool
    let pool = create_pool(&database_url)
        .await
        .context("Failed to create database pool")?;

    // Configure server
    let config = ServerConfig {
        bind_addr: args.bind,
        cors_permissive: args.cors_permissive,
    };

    // Run server (blocks until shutdown)
    run_server(pool, config)
        .await
        .context("Server error")?;

    Ok(())
}

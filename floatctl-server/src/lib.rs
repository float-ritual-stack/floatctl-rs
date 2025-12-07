//! # floatctl-server
//!
//! HTTP server for floatctl providing:
//! - Board/thread/message management
//! - Marker-based search
//! - Per-persona inbox
//! - Common scratchpad with TTL
//! - CLI command proxy (allowlisted)
//!
//! ## Architecture
//!
//! ```text
//! floatctl-server/
//! ├── db/          # Database layer (pool, repos)
//! ├── models/      # Domain models with validation
//! ├── http/        # Axum server and routes
//! └── cli/         # CLI invoker trait
//! ```
//!
//! ## Quick Start
//!
//! ```ignore
//! use floatctl_server::{db, http, ServerConfig};
//!
//! let pool = db::create_pool(&database_url).await?;
//! let config = ServerConfig::default();
//! http::run_server(pool, config).await?;
//! ```

pub mod db;
pub mod models;
pub mod http;
pub mod cli;
pub mod bbs;

// Re-exports for convenience
pub use db::create_pool;
pub use http::{run_server, ServerConfig};
pub use models::ValidationError;

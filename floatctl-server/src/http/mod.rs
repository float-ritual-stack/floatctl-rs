//! HTTP server layer - Specs 2.1+
//!
//! Axum server with:
//! - CORS (localhost only by default)
//! - Request tracing
//! - Graceful shutdown
//! - JSON error responses

pub mod server;
pub mod error;
pub mod extractors;
pub mod routes;

pub use server::{run_server, ServerConfig};
pub use error::ApiError;

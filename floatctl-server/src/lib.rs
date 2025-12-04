//! floatctl-server - HTTP server with BBS primitives for multi-agent collaboration
//!
//! This crate provides:
//! - An Axum-based HTTP server accessible via `floatctl server`
//! - BBS primitives: boards, threads, inbox, and common areas
//! - CLI command wrapping via REST endpoints
//! - SQLite storage with automatic migrations

pub mod db;
pub mod error;
pub mod models;
pub mod routes;
pub mod server;

pub use db::Database;
pub use error::{ServerError, ServerResult};
pub use models::*;
pub use server::{run_server, ServerArgs};

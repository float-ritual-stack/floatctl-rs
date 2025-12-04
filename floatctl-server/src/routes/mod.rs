//! Route handlers for floatctl-server BBS
//!
//! Organized by resource type:
//! - boards: Message boards (workspace areas)
//! - threads: Threaded discussions
//! - inbox: Per-persona message queues
//! - common: Shared scratch space
//! - cli: CLI command wrapper
//! - health: Health check endpoint

pub mod boards;
pub mod cli;
pub mod common;
pub mod health;
pub mod inbox;
pub mod threads;

pub use boards::*;
pub use cli::*;
pub use common::*;
pub use health::*;
pub use inbox::*;
pub use threads::*;

//! BBS (Bulletin Board System) primitives for authentic collaboration
//!
//! Provides:
//! - **Boards**: Topic-organized spaces (like forums or channels)
//! - **Threads**: Discussion threads within boards
//! - **Posts**: Individual messages within threads
//! - **Inbox**: Personal message queues for async communication
//! - **Commons**: Shared spaces for real-time collaboration

pub mod handlers;
pub mod migrations;
pub mod models;

pub use handlers::*;
pub use models::*;

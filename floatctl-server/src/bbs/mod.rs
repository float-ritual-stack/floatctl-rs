//! BBS file operations - file-based bulletin board system
//!
//! Provides async file I/O for:
//! - Inbox (per-persona messaging)
//! - Memory (per-persona persistent notes)
//! - Board (shared posting spaces)
//!
//! All content uses YAML frontmatter + markdown body format.

pub mod config;
pub mod frontmatter;
pub mod inbox;
pub mod memory;
pub mod board;

pub use config::BbsConfig;
pub use frontmatter::{parse_frontmatter, write_with_frontmatter, slugify, generate_message_id, generate_content_id};

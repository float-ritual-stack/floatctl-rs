//! Float Control TUI - Menu-driven, hierarchical terminal interface
//!
//! A TV-centric TUI for float control with:
//! - Hierarchical navigation (boards → posts → actions)
//! - Left scratch pane (persistent command input/notes)
//! - Dynamic main area with tabs (home, boards, search, dashboards)
//! - Normal/Edit mode switching (vim-style)
//! - Action palette for context-aware operations
//! - RAG search integration with scope filtering

pub mod app;
pub mod components;
pub mod event;
pub mod sources;
pub mod terminal;
pub mod ui;

pub use app::{App, Mode};
pub use terminal::run;

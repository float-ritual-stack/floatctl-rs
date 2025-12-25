//! Data sources for the TUI
//!
//! Sources provide data to the list navigator and define actions for items.

pub mod traits;
pub mod boards;
pub mod filesystem;
pub mod home;

pub use traits::{Action, ActionContext, ActionResult, Source, SourceItem};

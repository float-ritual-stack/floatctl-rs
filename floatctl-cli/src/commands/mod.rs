//! Command implementations for floatctl CLI

pub mod bridge;
pub mod claude;
pub mod script;

// Re-export main dispatcher functions for flat access from main.rs
pub use bridge::run_bridge;
pub use claude::run_claude;
pub use script::run_script;

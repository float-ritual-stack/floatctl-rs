//! Command implementations for floatctl CLI

pub mod script;

// Re-export main dispatcher functions for flat access from main.rs
pub use script::run_script;

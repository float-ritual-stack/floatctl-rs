//! Command implementations for floatctl CLI

pub mod ask;
pub mod bbs;
pub mod bridge;
pub mod claude;
pub mod ctx;
pub mod evna;
pub mod script;
#[cfg(feature = "server")]
pub mod serve;
pub mod status;
pub mod system;

// Re-export main dispatcher functions for flat access from main.rs
pub use ask::run_ask;
pub use bbs::run_bbs;
pub use bridge::run_bridge;
pub use claude::run_claude;
pub use ctx::run_ctx;
pub use evna::run_evna;
pub use script::run_script;
#[cfg(feature = "server")]
pub use serve::run_serve;
pub use status::run_status;
pub use system::run_system;

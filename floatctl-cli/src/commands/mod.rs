//! Command implementations for floatctl CLI

pub mod ask;
pub mod bbs;
pub mod claude;
pub mod ctx;
pub mod evna;
pub mod normalize;
pub mod script;
#[cfg(feature = "server")]
pub mod serve;
pub mod status;
pub mod system;

// Re-export main dispatcher functions for flat access from main.rs
pub use ask::run_ask;
pub use bbs::run_bbs;
pub use claude::run_claude;
pub use ctx::run_ctx;
pub use evna::run_evna;
pub use normalize::run_normalize;
pub use script::run_script;
#[cfg(feature = "server")]
pub use serve::run_serve;
pub use status::run_status;
pub use system::run_system;

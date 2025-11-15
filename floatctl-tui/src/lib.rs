pub mod block;
pub mod board;
pub mod agent;
pub mod ui;
pub mod input;
pub mod sync;
pub mod config;
pub mod db;

// Re-export commonly used types
pub use block::{Block, BlockId, AgentId, BoardId, Annotation, ScratchParser};
pub use db::BlockStore;

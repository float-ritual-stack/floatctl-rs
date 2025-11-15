pub mod app;
pub mod block;
pub mod board;
pub mod agent;
pub mod ui;
pub mod input;
pub mod sync;
pub mod config;
pub mod db;
pub mod mode;

// Re-export commonly used types
pub use app::App;
pub use block::{Block, BlockId, AgentId, BoardId, Annotation, ScratchParser};
pub use db::BlockStore;
pub use mode::{AppMode, Pane};
pub use ui::UI;

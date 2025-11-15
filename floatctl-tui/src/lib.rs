pub mod agent;
pub mod app;
pub mod block;
pub mod board;
pub mod config;
pub mod db;
pub mod input;
pub mod mode;
pub mod sync;
pub mod ui;

// Re-export commonly used types
pub use app::App;
pub use block::{AgentId, Annotation, Block, BlockId, BoardId, ScratchParser};
pub use db::BlockStore;
pub use mode::{AppMode, Pane};
pub use ui::UI;

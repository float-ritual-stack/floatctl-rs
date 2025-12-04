//! Repository implementations for database access
//!
//! Each repository follows these patterns:
//! - Uses JOINs for list operations (no N+1)
//! - Handles conflicts via ON CONFLICT (no check-then-insert)
//! - Uses transactions for multi-step operations

pub mod boards;
pub mod threads;
pub mod messages;
pub mod inbox;
pub mod scratchpad;

pub use boards::{BoardRepo, Board, BoardWithCount, DbError};
pub use threads::{ThreadRepo, Thread, ThreadWithCount};
pub use messages::{MessageRepo, Message, MessageWithMarkers};
pub use inbox::{InboxRepo, InboxMessage};
pub use scratchpad::{ScratchpadRepo, ScratchpadItem};

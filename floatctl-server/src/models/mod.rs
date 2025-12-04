//! Domain models with validation at construction
//!
//! All user input is validated when creating these types.
//! Invalid input returns ValidationError, not panic.

pub mod validation;
pub mod board;
pub mod thread;
pub mod message;
pub mod marker;
pub mod persona;
pub mod pagination;

pub use validation::ValidationError;
pub use board::BoardName;
pub use thread::ThreadTitle;
pub use message::MessageContent;
pub use marker::{Marker, MarkerKind};
pub use persona::Persona;
pub use pagination::{Pagination, Paginated, PaginationParams};

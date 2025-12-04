//! Database layer - connection pool and repositories
//!
//! # Design Principles
//!
//! - Connection pool (max 5 connections) - no Arc<Mutex<Connection>>
//! - All list operations use JOINs - no N+1 queries
//! - Rely on DB constraints, handle conflicts - no check-then-insert
//! - Transactions for multi-step operations

pub mod pool;
pub mod repos;

pub use pool::create_pool;
pub use repos::*;

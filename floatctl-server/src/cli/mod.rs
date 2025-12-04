//! CLI command execution - Spec 4.1
//!
//! Provides a testable interface for executing floatctl commands.

pub mod invoker;

pub use invoker::{CliInvoker, Output, RealInvoker, MockInvoker, execute_with_timeout, CLI_TIMEOUT_SECS};

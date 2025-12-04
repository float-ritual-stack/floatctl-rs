//! CLI invoker trait and implementations - Spec 4.1
//!
//! Provides a trait for executing CLI commands, with:
//! - Real implementation using tokio::process
//! - Mock implementation for testing
//! - Timeout enforcement

use std::process::Stdio;
use std::time::Duration;

use async_trait::async_trait;
use tokio::process::Command;

use crate::http::error::ApiError;

/// CLI execution timeout in seconds
pub const CLI_TIMEOUT_SECS: u64 = 30;

/// Output from CLI execution
#[derive(Debug, Clone)]
pub struct Output {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

/// Trait for CLI command execution (testable)
#[async_trait]
pub trait CliInvoker: Send + Sync {
    async fn invoke(&self, command: &str, args: Vec<String>) -> Result<Output, InvokeError>;
}

/// Error during CLI invocation
#[derive(Debug, thiserror::Error)]
pub enum InvokeError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("command not found: {0}")]
    NotFound(String),
}

/// Real CLI invoker using tokio::process
pub struct RealInvoker;

#[async_trait]
impl CliInvoker for RealInvoker {
    async fn invoke(&self, command: &str, args: Vec<String>) -> Result<Output, InvokeError> {
        let output = Command::new("floatctl")
            .arg(command)
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?
            .wait_with_output()
            .await?;

        Ok(Output {
            status: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// Mock CLI invoker for testing
#[derive(Default)]
pub struct MockInvoker {
    responses: std::sync::Mutex<Vec<Output>>,
}

impl MockInvoker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a response to return on the next invocation
    pub fn add_response(&self, output: Output) {
        self.responses.lock().unwrap().push(output);
    }
}

#[async_trait]
impl CliInvoker for MockInvoker {
    async fn invoke(&self, _command: &str, _args: Vec<String>) -> Result<Output, InvokeError> {
        let mut responses = self.responses.lock().unwrap();
        if responses.is_empty() {
            Ok(Output {
                status: 0,
                stdout: String::new(),
                stderr: String::new(),
            })
        } else {
            Ok(responses.remove(0))
        }
    }
}

/// Execute CLI command with timeout
pub async fn execute_with_timeout(
    invoker: &dyn CliInvoker,
    command: &str,
    args: Vec<String>,
) -> Result<Output, ApiError> {
    let timeout = Duration::from_secs(CLI_TIMEOUT_SECS);

    match tokio::time::timeout(timeout, invoker.invoke(command, args)).await {
        Ok(Ok(output)) => Ok(output),
        Ok(Err(e)) => Err(ApiError::Internal {
            message: format!("CLI error: {}", e),
        }),
        Err(_) => Err(ApiError::Timeout {
            seconds: CLI_TIMEOUT_SECS,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_invoker_returns_response() {
        let mock = MockInvoker::new();
        mock.add_response(Output {
            status: 0,
            stdout: "hello".into(),
            stderr: String::new(),
        });

        let output = mock.invoke("test", vec![]).await.unwrap();
        assert_eq!(output.stdout, "hello");
    }

    #[tokio::test]
    async fn mock_invoker_empty_response() {
        let mock = MockInvoker::new();
        let output = mock.invoke("test", vec![]).await.unwrap();
        assert_eq!(output.status, 0);
        assert!(output.stdout.is_empty());
    }

    #[tokio::test]
    async fn timeout_returns_error() {
        // Create a mock that never returns
        struct SlowInvoker;

        #[async_trait]
        impl CliInvoker for SlowInvoker {
            async fn invoke(&self, _: &str, _: Vec<String>) -> Result<Output, InvokeError> {
                tokio::time::sleep(Duration::from_secs(60)).await;
                Ok(Output {
                    status: 0,
                    stdout: String::new(),
                    stderr: String::new(),
                })
            }
        }

        // Use a short timeout for testing
        let timeout = Duration::from_millis(10);
        let result = tokio::time::timeout(timeout, SlowInvoker.invoke("test", vec![])).await;

        assert!(result.is_err()); // Timed out
    }
}

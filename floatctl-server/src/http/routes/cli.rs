//! CLI proxy endpoints - Spec 4.1
//!
//! SECURITY: Only allowlisted commands can be executed.
//! Timeout enforced at 30 seconds.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::cli::RealInvoker;
use crate::http::error::ApiError;
use crate::http::server::AppState;

/// Hard-coded allowlist of commands
const ALLOWED_COMMANDS: &[&str] = &["search", "ctx", "query", "claude"];

/// Explicitly blocked commands (even if someone tries to add to allowlist)
const BLOCKED_COMMANDS: &[&str] = &["server", "embed", "sync"];

/// CLI execution request
#[derive(Deserialize)]
pub struct CliRequest {
    #[serde(default)]
    pub args: Vec<String>,
}

/// CLI execution response
#[derive(Serialize)]
pub struct CliResponse {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

/// POST /cli/{command} - execute CLI command
async fn execute_cli(
    State(_state): State<Arc<AppState>>,
    Path(command): Path<String>,
    Json(req): Json<CliRequest>,
) -> Result<Json<CliResponse>, ApiError> {
    // Check blocked list first
    if BLOCKED_COMMANDS.contains(&command.as_str()) {
        return Err(ApiError::Forbidden {
            reason: format!("command '{}' is explicitly blocked", command),
        });
    }

    // Check allowlist
    if !ALLOWED_COMMANDS.contains(&command.as_str()) {
        return Err(ApiError::Forbidden {
            reason: format!(
                "command '{}' not in allowlist: {:?}",
                command, ALLOWED_COMMANDS
            ),
        });
    }

    // Execute with timeout
    let invoker = RealInvoker;
    let output = crate::cli::execute_with_timeout(&invoker, &command, req.args).await?;

    Ok(Json(CliResponse {
        status: output.status,
        stdout: output.stdout,
        stderr: output.stderr,
    }))
}

/// CLI routes
pub fn router() -> Router<Arc<AppState>> {
    Router::new().route("/cli/{command}", post(execute_cli))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_commands() {
        assert!(ALLOWED_COMMANDS.contains(&"search"));
        assert!(ALLOWED_COMMANDS.contains(&"ctx"));
        assert!(ALLOWED_COMMANDS.contains(&"query"));
        assert!(!ALLOWED_COMMANDS.contains(&"embed"));
    }

    #[test]
    fn blocked_commands() {
        assert!(BLOCKED_COMMANDS.contains(&"server"));
        assert!(BLOCKED_COMMANDS.contains(&"embed"));
        assert!(BLOCKED_COMMANDS.contains(&"sync"));
    }
}

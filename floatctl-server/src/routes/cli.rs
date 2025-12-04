//! CLI wrapper routes - Expose floatctl commands as REST endpoints

use std::process::Stdio;
use std::time::Instant;

use axum::{
    extract::Path,
    Json,
};
use tokio::process::Command;

use crate::error::{ServerError, ServerResult};
use crate::models::{CliRequest, CliResponse};

/// POST /cli/:command - Execute a floatctl command
///
/// Wraps the floatctl CLI and returns structured output.
/// Commands are executed as subprocesses.
///
/// # Allowed Commands
/// Only a subset of commands are exposed for safety:
/// - `daily` - Daily notes
/// - `sync` - Sync operations
/// - `bridge` - Bridge maintenance
/// - `system` - System diagnostics
/// - `config` - Configuration queries (get/list only)
///
/// # Example
/// ```json
/// POST /cli/sync
/// {
///   "args": ["status"]
/// }
/// ```
pub async fn execute_cli(
    Path(command): Path<String>,
    Json(req): Json<CliRequest>,
) -> ServerResult<Json<CliResponse>> {
    // Allowlist of safe commands
    let allowed_commands = [
        "daily",
        "sync",
        "bridge",
        "system",
        "config",
        "ctx",
        "script",
    ];

    if !allowed_commands.contains(&command.as_str()) {
        return Err(ServerError::BadRequest(format!(
            "Command '{}' not allowed. Allowed: {:?}",
            command, allowed_commands
        )));
    }

    // Block dangerous config operations
    if command == "config" {
        let first_arg = req.args.first().map(|s| s.as_str());
        if !matches!(first_arg, Some("get") | Some("list") | Some("path")) {
            return Err(ServerError::BadRequest(
                "Only 'get', 'list', and 'path' config operations allowed".into(),
            ));
        }
    }

    let start = Instant::now();

    // Build command
    let mut cmd = Command::new("floatctl");
    cmd.arg(&command);
    cmd.args(&req.args);

    if let Some(cwd) = &req.cwd {
        cmd.current_dir(cwd);
    }

    // Capture output
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd
        .output()
        .await
        .map_err(|e| ServerError::Internal(format!("Failed to execute command: {}", e)))?;

    let duration = start.elapsed();

    Ok(Json(CliResponse {
        success: output.status.success(),
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        duration_ms: duration.as_millis() as u64,
    }))
}

/// GET /cli/commands - List available CLI commands
pub async fn list_cli_commands() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "commands": [
            {
                "name": "daily",
                "description": "Daily notes operations",
                "example_args": ["--date", "2024-01-15"]
            },
            {
                "name": "sync",
                "description": "R2 sync daemon management",
                "example_args": ["status"]
            },
            {
                "name": "bridge",
                "description": "Bridge file maintenance",
                "example_args": ["index", "--path", "~/notes"]
            },
            {
                "name": "system",
                "description": "System diagnostics",
                "example_args": ["health"]
            },
            {
                "name": "config",
                "description": "Configuration queries (get/list only)",
                "example_args": ["list"]
            },
            {
                "name": "ctx",
                "description": "Context marker capture",
                "example_args": ["capture", "project::pharmacy"]
            },
            {
                "name": "script",
                "description": "Script management",
                "example_args": ["list"]
            }
        ]
    }))
}

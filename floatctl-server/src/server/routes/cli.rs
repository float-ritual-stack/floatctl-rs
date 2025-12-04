use axum::{extract::State, routing::post, Json, Router};
use std::process::Stdio;
use tokio::process::Command;
use tracing::warn;

use crate::server::error::{AppError, AppResult};
use crate::server::models::{CliCommandRequest, CliCommandResponse};
use crate::server::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/:command", post(run_command))
}

async fn run_command(
    State(_state): State<AppState>,
    axum::extract::Path(command): axum::extract::Path<String>,
    Json(payload): Json<CliCommandRequest>,
) -> AppResult<Json<CliCommandResponse>> {
    if command == "server" {
        return Err(AppError::BadRequest(
            "nesting the server command is not allowed".into(),
        ));
    }

    let exe = std::env::current_exe().map_err(|err| AppError::Anyhow(err.into()))?;
    let mut cmd = Command::new(exe);
    cmd.arg(&command)
        .args(&payload.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = cmd
        .output()
        .await
        .map_err(|err| AppError::Anyhow(err.into()))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    let status = output.status.code().unwrap_or(-1);
    if status != 0 {
        warn!("command {command} returned status {status}");
    }

    Ok(Json(CliCommandResponse {
        command,
        status,
        stdout,
        stderr,
        request_id: uuid::Uuid::new_v4(),
    }))
}

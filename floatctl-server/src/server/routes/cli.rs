use std::{process::Stdio, sync::Arc};

use axum::{
    extract::{Path, State},
    Json,
};
use tokio::process::Command;

use crate::server::{
    error::{ServerError, ServerResult},
    models::{CliCommandRequest, CliCommandResponse},
    AppState,
};

pub async fn run_command(
    State(state): State<Arc<AppState>>,
    Path(command): Path<String>,
    Json(body): Json<CliCommandRequest>,
) -> ServerResult<Json<CliCommandResponse>> {
    let binary = &state.binary_path;
    let args = body.args.unwrap_or_default();

    let mut cmd = Command::new(binary);
    cmd.arg(command);
    for arg in args {
        cmd.arg(arg);
    }

    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let output = cmd
        .output()
        .await
        .map_err(|err| ServerError::Command(err.to_string()))?;

    let response = CliCommandResponse {
        id: uuid::Uuid::new_v4(),
        command,
        status: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    };

    Ok(Json(response))
}

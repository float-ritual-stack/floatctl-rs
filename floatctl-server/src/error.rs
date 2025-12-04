//! Error types for floatctl-server

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type ServerResult<T> = Result<T, ServerError>;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let (status, error_message) = match &self {
            ServerError::Database(e) => {
                tracing::error!("Database error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, self.to_string())
            }
            ServerError::NotFound(msg) => (StatusCode::NOT_FOUND, msg.clone()),
            ServerError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg.clone()),
            ServerError::Conflict(msg) => (StatusCode::CONFLICT, msg.clone()),
            ServerError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, msg.clone())
            }
            ServerError::Json(e) => (StatusCode::BAD_REQUEST, format!("JSON parse error: {}", e)),
            ServerError::Io(e) => {
                tracing::error!("IO error: {}", e);
                (StatusCode::INTERNAL_SERVER_ERROR, format!("IO error: {}", e))
            }
        };

        let body = Json(json!({
            "error": error_message,
            "status": status.as_u16()
        }));

        (status, body).into_response()
    }
}

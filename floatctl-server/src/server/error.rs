use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ServerError {
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("configuration error: {0}")]
    Config(String),
    #[error("command failed: {0}")]
    Command(String),
}

#[derive(Serialize)]
struct ErrorBody {
    message: String,
}

impl IntoResponse for ServerError {
    fn into_response(self) -> Response {
        let status = match self {
            ServerError::Config(_) => StatusCode::BAD_REQUEST,
            ServerError::Command(_) => StatusCode::BAD_REQUEST,
            ServerError::Database(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
            ServerError::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            ServerError::Io(_) | ServerError::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = Json(ErrorBody {
            message: self.to_string(),
        });

        (status, body).into_response()
    }
}

pub type ServerResult<T> = Result<T, ServerError>;

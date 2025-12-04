use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("not found")]
    NotFound,
    #[error("bad request: {0}")]
    BadRequest(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Anyhow(#[from] anyhow::Error),
}

#[derive(Serialize)]
pub struct ErrorResponse {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match self {
            AppError::NotFound => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Sqlx(sqlx::Error::RowNotFound) => StatusCode::NOT_FOUND,
            AppError::Sqlx(_) | AppError::Anyhow(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let message = self.to_string();
        (status, Json(ErrorResponse { error: message })).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;

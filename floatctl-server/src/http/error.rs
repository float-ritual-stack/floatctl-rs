//! API error types with IntoResponse - Spec 2.2
//!
//! Errors are converted to JSON responses with appropriate status codes.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use crate::db::repos::DbError;
use crate::models::ValidationError;

/// API error type with automatic HTTP status mapping
#[derive(Debug)]
pub enum ApiError {
    /// Validation failed (400)
    Validation(ValidationError),

    /// Resource not found (404)
    NotFound { resource: &'static str, id: String },

    /// Database error (500, logged)
    Database(DbError),

    /// CLI command not allowed (403)
    Forbidden { reason: String },

    /// CLI timeout (504)
    Timeout { seconds: u64 },

    /// Internal error (500)
    Internal { message: String },
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, body) = match &self {
            Self::Validation(e) => (
                StatusCode::BAD_REQUEST,
                json!({
                    "error": "validation_error",
                    "message": e.to_string()
                }),
            ),
            Self::NotFound { resource, id } => (
                StatusCode::NOT_FOUND,
                json!({
                    "error": "not_found",
                    "message": format!("{} '{}' not found", resource, id)
                }),
            ),
            Self::Database(e) => {
                // Log the actual error, return generic message
                tracing::error!("Database error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({
                        "error": "internal_error",
                        "message": "an internal error occurred"
                    }),
                )
            }
            Self::Forbidden { reason } => (
                StatusCode::FORBIDDEN,
                json!({
                    "error": "forbidden",
                    "message": reason
                }),
            ),
            Self::Timeout { seconds } => (
                StatusCode::GATEWAY_TIMEOUT,
                json!({
                    "error": "timeout",
                    "message": format!("operation timed out after {} seconds", seconds)
                }),
            ),
            Self::Internal { message } => {
                tracing::error!("Internal error: {}", message);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    json!({
                        "error": "internal_error",
                        "message": "an internal error occurred"
                    }),
                )
            }
        };

        (status, Json(body)).into_response()
    }
}

impl From<ValidationError> for ApiError {
    fn from(e: ValidationError) -> Self {
        Self::Validation(e)
    }
}

impl From<DbError> for ApiError {
    fn from(e: DbError) -> Self {
        match e {
            DbError::NotFound { resource, id } => Self::NotFound { resource, id },
            _ => Self::Database(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn validation_error_is_400() {
        let err = ApiError::Validation(ValidationError::Empty { field: "name" });
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn not_found_is_404() {
        let err = ApiError::NotFound {
            resource: "board",
            id: "test".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn forbidden_is_403() {
        let err = ApiError::Forbidden {
            reason: "command not allowed".into(),
        };
        let response = err.into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}

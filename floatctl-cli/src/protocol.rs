//! Agent Protocol - Structured JSON output for LLM/machine consumption
//!
//! When `--json` is passed globally, all output is wrapped in a standard envelope:
//! ```json
//! {
//!   "status": "success" | "error",
//!   "data": { ... },
//!   "error": { "code": "ERR_...", "message": "..." }
//! }
//! ```
//!
//! This module provides:
//! - `ApiResponse<T>` - The standard envelope type
//! - `ApiError` - Structured error with code and message
//! - `ErrorCode` - Enumerated error codes for agent parsing
//! - Helper functions for mapping anyhow errors to structured responses

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::OnceLock;

/// Global JSON mode state (set by --json flag)
static JSON_MODE: OnceLock<bool> = OnceLock::new();

/// Initialize JSON mode from the --json flag
pub fn init_json_mode(json_flag: bool) {
    JSON_MODE.set(json_flag).ok();
}

/// Check if we're in JSON mode
pub fn is_json_mode() -> bool {
    *JSON_MODE.get().unwrap_or(&false)
}

/// Standard API response envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T: Serialize> {
    /// Response status: "success" or "error"
    pub status: ResponseStatus,

    /// The actual command output (present on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,

    /// Error details (present on error)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ApiError>,
}

/// Response status enum
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    Success,
    Error,
}

/// Structured error for agent consumption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiError {
    /// Machine-readable error code (e.g., "ERR_FILE_NOT_FOUND")
    pub code: ErrorCode,

    /// Human-readable error message
    pub message: String,

    /// Additional context/details (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Enumerated error codes for consistent agent parsing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    // File/IO errors
    ErrFileNotFound,
    ErrFileReadFailed,
    ErrFileWriteFailed,
    ErrDirectoryNotFound,
    ErrPermissionDenied,

    // Validation errors
    ErrInvalidInput,
    ErrMissingRequired,
    ErrInvalidFormat,
    ErrValidationFailed,

    // Database errors
    ErrDatabaseConnection,
    ErrDatabaseQuery,
    ErrNotFound,

    // Network errors
    ErrNetworkFailed,
    ErrTimeout,
    ErrAuthFailed,

    // Configuration errors
    ErrConfigNotFound,
    ErrConfigInvalid,

    // Command-specific errors
    ErrCommandFailed,
    ErrNotImplemented,
    ErrCancelled,

    // Catch-all
    ErrInternal,
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Serialize to get the SCREAMING_SNAKE_CASE representation
        let s = serde_json::to_string(self).unwrap_or_else(|_| "\"ERR_INTERNAL\"".to_string());
        // Remove quotes
        write!(f, "{}", s.trim_matches('"'))
    }
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a success response with data
    pub fn success(data: T) -> Self {
        Self {
            status: ResponseStatus::Success,
            data: Some(data),
            error: None,
        }
    }

    /// Create an error response
    pub fn error(code: ErrorCode, message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            status: ResponseStatus::Error,
            data: None,
            error: Some(ApiError {
                code,
                message: message.into(),
                details: None,
            }),
        }
    }

    /// Create an error response with details
    pub fn error_with_details(
        code: ErrorCode,
        message: impl Into<String>,
        details: impl Into<String>,
    ) -> ApiResponse<()> {
        ApiResponse {
            status: ResponseStatus::Error,
            data: None,
            error: Some(ApiError {
                code,
                message: message.into(),
                details: Some(details.into()),
            }),
        }
    }

    /// Print this response as JSON to stdout
    pub fn print(&self) {
        if let Ok(json) = serde_json::to_string_pretty(self) {
            println!("{}", json);
        }
    }
}

/// Map an anyhow error to an ApiResponse
///
/// Attempts to classify the error into an appropriate ErrorCode based on
/// the error message and chain.
pub fn map_error(err: &anyhow::Error) -> ApiResponse<()> {
    let message = err.to_string();
    let full_chain = format!("{:?}", err);

    // Classify error based on message patterns
    let code = classify_error(&message, &full_chain);

    // Include error chain in details if there's more context
    let details = if err.chain().count() > 1 {
        Some(
            err.chain()
                .skip(1)
                .map(|e| e.to_string())
                .collect::<Vec<_>>()
                .join(" → "),
        )
    } else {
        None
    };

    ApiResponse {
        status: ResponseStatus::Error,
        data: None,
        error: Some(ApiError {
            code,
            message,
            details,
        }),
    }
}

/// Classify an error message into an ErrorCode
fn classify_error(message: &str, full_chain: &str) -> ErrorCode {
    let lower = message.to_lowercase();
    let chain_lower = full_chain.to_lowercase();

    // File/IO errors
    if lower.contains("no such file")
        || lower.contains("file not found")
        || lower.contains("does not exist")
    {
        return ErrorCode::ErrFileNotFound;
    }
    if lower.contains("directory not found") || lower.contains("no such directory") {
        return ErrorCode::ErrDirectoryNotFound;
    }
    if lower.contains("permission denied") || lower.contains("access denied") {
        return ErrorCode::ErrPermissionDenied;
    }
    if lower.contains("failed to read") || chain_lower.contains("read") {
        return ErrorCode::ErrFileReadFailed;
    }
    if lower.contains("failed to write") || lower.contains("write failed") {
        return ErrorCode::ErrFileWriteFailed;
    }

    // Validation errors
    if lower.contains("invalid") && lower.contains("input") {
        return ErrorCode::ErrInvalidInput;
    }
    if lower.contains("missing") || lower.contains("required") {
        return ErrorCode::ErrMissingRequired;
    }
    if lower.contains("invalid format") || lower.contains("parse error") {
        return ErrorCode::ErrInvalidFormat;
    }
    if lower.contains("validation") {
        return ErrorCode::ErrValidationFailed;
    }

    // Database errors
    if lower.contains("database") && lower.contains("connection") {
        return ErrorCode::ErrDatabaseConnection;
    }
    if lower.contains("query") || lower.contains("sql") {
        return ErrorCode::ErrDatabaseQuery;
    }
    if lower.contains("not found") && !lower.contains("file") {
        return ErrorCode::ErrNotFound;
    }

    // Network errors
    if lower.contains("network")
        || lower.contains("connection refused")
        || lower.contains("dns")
        || lower.contains("unreachable")
    {
        return ErrorCode::ErrNetworkFailed;
    }
    if lower.contains("timeout") || lower.contains("timed out") {
        return ErrorCode::ErrTimeout;
    }
    if lower.contains("unauthorized")
        || lower.contains("forbidden")
        || lower.contains("authentication")
    {
        return ErrorCode::ErrAuthFailed;
    }

    // Configuration errors
    if lower.contains("config") && lower.contains("not found") {
        return ErrorCode::ErrConfigNotFound;
    }
    if lower.contains("config") && lower.contains("invalid") {
        return ErrorCode::ErrConfigInvalid;
    }

    // Command errors
    if lower.contains("not implemented") || lower.contains("unimplemented") {
        return ErrorCode::ErrNotImplemented;
    }
    if lower.contains("cancelled") || lower.contains("canceled") || lower.contains("interrupted") {
        return ErrorCode::ErrCancelled;
    }

    // Default to internal error
    ErrorCode::ErrInternal
}

/// Output helper - prints JSON in json mode, or runs the human closure otherwise
pub fn output<T, F>(data: T, human_output: F)
where
    T: Serialize,
    F: FnOnce(&T),
{
    if is_json_mode() {
        ApiResponse::success(data).print();
    } else {
        human_output(&data);
    }
}

/// Output helper for simple messages
pub fn output_message(message: impl Into<String>) {
    let msg = message.into();
    if is_json_mode() {
        ApiResponse::success(serde_json::json!({ "message": msg })).print();
    } else {
        println!("{}", msg);
    }
}

/// Handle a result, outputting appropriate JSON on error in json mode
pub fn handle_result<T: Serialize>(result: anyhow::Result<T>) -> anyhow::Result<T> {
    match result {
        Ok(data) => Ok(data),
        Err(err) => {
            if is_json_mode() {
                map_error(&err).print();
            }
            Err(err)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_success_response() {
        let resp = ApiResponse::success(serde_json::json!({"count": 42}));
        assert_eq!(resp.status, ResponseStatus::Success);
        assert!(resp.data.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_error_response() {
        let resp = ApiResponse::<()>::error(ErrorCode::ErrFileNotFound, "File not found: test.txt");
        assert_eq!(resp.status, ResponseStatus::Error);
        assert!(resp.data.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.as_ref().unwrap().code, ErrorCode::ErrFileNotFound);
    }

    #[test]
    fn test_error_classification() {
        assert_eq!(
            classify_error("No such file or directory", ""),
            ErrorCode::ErrFileNotFound
        );
        assert_eq!(
            classify_error("Permission denied", ""),
            ErrorCode::ErrPermissionDenied
        );
        assert_eq!(
            classify_error("Connection timed out", ""),
            ErrorCode::ErrTimeout
        );
        assert_eq!(
            classify_error("Something weird happened", ""),
            ErrorCode::ErrInternal
        );
    }

    #[test]
    fn test_json_serialization() {
        let resp = ApiResponse::success(serde_json::json!({"test": true}));
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"status\":\"success\""));
        assert!(json.contains("\"test\":true"));
    }
}

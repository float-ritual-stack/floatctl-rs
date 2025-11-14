/// Structured error types for floatctl-core library.
///
/// Uses `thiserror` for better API surface and error composition.
/// Binary crates (floatctl-cli) can still use `anyhow` for convenience,
/// but library consumers get structured, composable errors.

use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Main error type for floatctl-core operations
#[derive(Error, Debug)]
pub enum FloatError {
    /// I/O operation failed
    #[error("I/O error: {source}")]
    Io {
        #[from]
        source: io::Error,
    },

    /// JSON parsing or serialization failed
    #[error("JSON error at {context}: {source}")]
    Json {
        context: String,
        source: serde_json::Error,
    },

    /// Invalid format detected (expected JSON array or NDJSON)
    #[error("Invalid format in file {path:?}: {reason}")]
    InvalidFormat { path: PathBuf, reason: String },

    /// Conversation parsing failed
    #[error("Failed to parse conversation: {reason}")]
    ConversationParse { reason: String },

    /// Message parsing failed
    #[error("Failed to parse message at index {index}: {reason}")]
    MessageParse { index: usize, reason: String },

    /// Required field missing
    #[error("Missing required field '{field}' in {context}")]
    MissingField { field: String, context: String },

    /// Invalid timestamp format
    #[error("Invalid timestamp '{value}': {reason}")]
    InvalidTimestamp { value: String, reason: String },

    /// File or directory not found
    #[error("Path not found: {path:?}")]
    PathNotFound { path: PathBuf },

    /// Empty input file (cannot detect format)
    #[error("Empty input file: {path:?}")]
    EmptyFile { path: PathBuf },

    /// Configuration error
    #[error("Configuration error: {reason}")]
    Config { reason: String },
}

/// Result type alias for floatctl-core operations
pub type Result<T> = std::result::Result<T, FloatError>;

impl FloatError {
    /// Create a JSON error with context
    pub fn json(context: impl Into<String>, source: serde_json::Error) -> Self {
        Self::Json {
            context: context.into(),
            source,
        }
    }

    /// Create an invalid format error
    pub fn invalid_format(path: impl Into<PathBuf>, reason: impl Into<String>) -> Self {
        Self::InvalidFormat {
            path: path.into(),
            reason: reason.into(),
        }
    }

    /// Create a conversation parse error
    pub fn conversation_parse(reason: impl Into<String>) -> Self {
        Self::ConversationParse {
            reason: reason.into(),
        }
    }

    /// Create a message parse error
    pub fn message_parse(index: usize, reason: impl Into<String>) -> Self {
        Self::MessageParse {
            index,
            reason: reason.into(),
        }
    }

    /// Create a missing field error
    pub fn missing_field(field: impl Into<String>, context: impl Into<String>) -> Self {
        Self::MissingField {
            field: field.into(),
            context: context.into(),
        }
    }

    /// Create an invalid timestamp error
    pub fn invalid_timestamp(value: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidTimestamp {
            value: value.into(),
            reason: reason.into(),
        }
    }

    /// Create a path not found error
    pub fn path_not_found(path: impl Into<PathBuf>) -> Self {
        Self::PathNotFound { path: path.into() }
    }

    /// Create an empty file error
    pub fn empty_file(path: impl Into<PathBuf>) -> Self {
        Self::EmptyFile { path: path.into() }
    }

    /// Create a config error
    pub fn config(reason: impl Into<String>) -> Self {
        Self::Config {
            reason: reason.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = FloatError::missing_field("timestamp", "message");
        assert_eq!(
            err.to_string(),
            "Missing required field 'timestamp' in message"
        );

        let err = FloatError::invalid_format("/tmp/test.json", "empty input file");
        assert!(err.to_string().contains("Invalid format"));
        assert!(err.to_string().contains("/tmp/test.json"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_err = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let float_err: FloatError = io_err.into();

        assert!(matches!(float_err, FloatError::Io { .. }));
    }
}

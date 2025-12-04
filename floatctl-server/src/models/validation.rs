//! Validation error types

use std::fmt;

/// Validation error for domain models
#[derive(Debug, Clone)]
pub enum ValidationError {
    /// Field is empty when it shouldn't be
    Empty { field: &'static str },

    /// Field exceeds maximum length
    TooLong { field: &'static str, max: usize },

    /// String doesn't match required format (e.g., slug)
    InvalidFormat { field: &'static str, reason: &'static str },

    /// Invalid enum variant
    InvalidVariant { field: &'static str, value: String },
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty { field } => write!(f, "{} cannot be empty", field),
            Self::TooLong { field, max } => {
                write!(f, "{} exceeds maximum length of {} characters", field, max)
            }
            Self::InvalidFormat { field, reason } => {
                write!(f, "{}: {}", field, reason)
            }
            Self::InvalidVariant { field, value } => {
                write!(f, "invalid {} value: '{}'", field, value)
            }
        }
    }
}

impl std::error::Error for ValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = ValidationError::TooLong {
            field: "title",
            max: 256,
        };
        assert_eq!(
            err.to_string(),
            "title exceeds maximum length of 256 characters"
        );
    }
}

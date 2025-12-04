//! Board name validation - Spec 1.2
//!
//! Slug format: lowercase alphanumeric with hyphens/underscores

use once_cell::sync::Lazy;
use regex::Regex;

use super::ValidationError;

/// Maximum length for board names
const MAX_BOARD_NAME_LEN: usize = 64;

/// Slug pattern: starts with alphanumeric, allows hyphens/underscores
/// Matches DB constraint: ^[a-z0-9][a-z0-9_-]{0,63}$
static SLUG_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^[a-z0-9][a-z0-9_-]{0,63}$").expect("invalid slug regex")
});

/// Validated board name (slug format)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoardName(String);

impl BoardName {
    /// Create a new board name, validating slug format.
    ///
    /// # Rules
    /// - Max 64 characters
    /// - Lowercase alphanumeric, hyphens, underscores
    /// - Must start with alphanumeric
    ///
    /// # Example
    /// ```
    /// use floatctl_server::models::BoardName;
    ///
    /// assert!(BoardName::new("my-board-123").is_ok());
    /// assert!(BoardName::new("MyBoard").is_err());  // uppercase
    /// assert!(BoardName::new("-dash-start").is_err());  // starts with dash
    /// ```
    pub fn new(s: &str) -> Result<Self, ValidationError> {
        if s.is_empty() {
            return Err(ValidationError::Empty { field: "board name" });
        }

        if s.len() > MAX_BOARD_NAME_LEN {
            return Err(ValidationError::TooLong {
                field: "board name",
                max: MAX_BOARD_NAME_LEN,
            });
        }

        if !SLUG_RE.is_match(s) {
            return Err(ValidationError::InvalidFormat {
                field: "board name",
                reason: "must be lowercase alphanumeric with hyphens/underscores, starting with alphanumeric",
            });
        }

        Ok(Self(s.to_owned()))
    }

    /// Get the board name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for BoardName {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_slugs() {
        assert!(BoardName::new("my-board").is_ok());
        assert!(BoardName::new("my_board").is_ok());
        assert!(BoardName::new("myboard123").is_ok());
        assert!(BoardName::new("a").is_ok());
        assert!(BoardName::new("1board").is_ok());
    }

    #[test]
    fn rejects_uppercase() {
        let err = BoardName::new("MyBoard").unwrap_err();
        assert!(matches!(err, ValidationError::InvalidFormat { .. }));
    }

    #[test]
    fn rejects_spaces() {
        let err = BoardName::new("my board").unwrap_err();
        assert!(matches!(err, ValidationError::InvalidFormat { .. }));
    }

    #[test]
    fn rejects_dash_start() {
        let err = BoardName::new("-myboard").unwrap_err();
        assert!(matches!(err, ValidationError::InvalidFormat { .. }));
    }

    #[test]
    fn rejects_empty() {
        let err = BoardName::new("").unwrap_err();
        assert!(matches!(err, ValidationError::Empty { .. }));
    }

    #[test]
    fn max_length() {
        // 64 chars should work
        let name_64 = "a".repeat(64);
        assert!(BoardName::new(&name_64).is_ok());

        // 65 chars should fail
        let name_65 = "a".repeat(65);
        let err = BoardName::new(&name_65).unwrap_err();
        assert!(matches!(err, ValidationError::TooLong { max: 64, .. }));
    }
}

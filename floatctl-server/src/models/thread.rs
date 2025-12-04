//! Thread title validation - Spec 1.2

use super::ValidationError;

/// Maximum length for thread titles
const MAX_TITLE_LEN: usize = 256;

/// Validated thread title
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThreadTitle(String);

impl ThreadTitle {
    /// Create a new thread title.
    ///
    /// # Rules
    /// - Non-empty (after trimming whitespace)
    /// - Max 256 characters
    ///
    /// # Example
    /// ```
    /// use floatctl_server::models::ThreadTitle;
    ///
    /// assert!(ThreadTitle::new("My Thread").is_ok());
    /// assert!(ThreadTitle::new("").is_err());
    /// assert!(ThreadTitle::new("   ").is_err());  // whitespace only
    /// ```
    pub fn new(s: &str) -> Result<Self, ValidationError> {
        let trimmed = s.trim();

        if trimmed.is_empty() {
            return Err(ValidationError::Empty { field: "title" });
        }

        if trimmed.len() > MAX_TITLE_LEN {
            return Err(ValidationError::TooLong {
                field: "title",
                max: MAX_TITLE_LEN,
            });
        }

        Ok(Self(trimmed.to_owned()))
    }

    /// Get the title as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string.
    pub fn into_string(self) -> String {
        self.0
    }
}

impl AsRef<str> for ThreadTitle {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_titles() {
        assert!(ThreadTitle::new("My Thread").is_ok());
        assert!(ThreadTitle::new("a").is_ok());
        assert!(ThreadTitle::new("  Trimmed  ").is_ok());
    }

    #[test]
    fn rejects_empty() {
        assert!(matches!(
            ThreadTitle::new("").unwrap_err(),
            ValidationError::Empty { .. }
        ));
    }

    #[test]
    fn rejects_whitespace_only() {
        assert!(matches!(
            ThreadTitle::new("   ").unwrap_err(),
            ValidationError::Empty { .. }
        ));
    }

    #[test]
    fn max_length() {
        // 256 chars should work
        let title_256 = "a".repeat(256);
        assert!(ThreadTitle::new(&title_256).is_ok());

        // 257 chars should fail
        let title_257 = "a".repeat(257);
        let err = ThreadTitle::new(&title_257).unwrap_err();
        assert!(matches!(err, ValidationError::TooLong { max: 256, .. }));
    }

    #[test]
    fn trims_whitespace() {
        let title = ThreadTitle::new("  hello  ").unwrap();
        assert_eq!(title.as_str(), "hello");
    }
}

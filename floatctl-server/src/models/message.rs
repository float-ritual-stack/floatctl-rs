//! Message content validation - Spec 1.2

use super::{Marker, ValidationError};

/// Maximum length for message content (64KB)
const MAX_CONTENT_LEN: usize = 65536;

/// Validated message content
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageContent(String);

impl MessageContent {
    /// Create new message content.
    ///
    /// # Rules
    /// - Max 64KB (65536 bytes)
    /// - Empty content is allowed (for some use cases)
    ///
    /// # Example
    /// ```
    /// use floatctl_server::models::MessageContent;
    ///
    /// assert!(MessageContent::new("Hello world").is_ok());
    /// assert!(MessageContent::new("").is_ok());  // empty allowed
    /// ```
    pub fn new(s: &str) -> Result<Self, ValidationError> {
        if s.len() > MAX_CONTENT_LEN {
            return Err(ValidationError::TooLong {
                field: "content",
                max: MAX_CONTENT_LEN,
            });
        }

        Ok(Self(s.to_owned()))
    }

    /// Get the content as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume and return the inner string.
    pub fn into_string(self) -> String {
        self.0
    }

    /// Extract markers from content.
    ///
    /// Looks for patterns like `ctx::value`, `project::value`, etc.
    pub fn extract_markers(&self) -> Vec<Marker> {
        Marker::extract_from(&self.0)
    }
}

impl AsRef<str> for MessageContent {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MarkerKind;

    #[test]
    fn valid_content() {
        assert!(MessageContent::new("Hello world").is_ok());
        assert!(MessageContent::new("").is_ok());
    }

    #[test]
    fn max_length() {
        // 64KB should work
        let content_64k = "a".repeat(65536);
        assert!(MessageContent::new(&content_64k).is_ok());

        // 64KB + 1 should fail
        let content_over = "a".repeat(65537);
        let err = MessageContent::new(&content_over).unwrap_err();
        assert!(matches!(err, ValidationError::TooLong { max: 65536, .. }));
    }

    #[test]
    fn extracts_markers() {
        let content = MessageContent::new("ctx::review project::api").unwrap();
        let markers = content.extract_markers();

        assert_eq!(markers.len(), 2);
        assert!(markers.iter().any(|m| m.kind == MarkerKind::Ctx && m.value == "review"));
        assert!(markers.iter().any(|m| m.kind == MarkerKind::Project && m.value == "api"));
    }

    #[test]
    fn extracts_multiple_same_kind() {
        let content = MessageContent::new("ctx::a ctx::b ctx::c").unwrap();
        let markers = content.extract_markers();

        let ctx_markers: Vec<_> = markers.iter().filter(|m| m.kind == MarkerKind::Ctx).collect();
        assert_eq!(ctx_markers.len(), 3);
    }
}

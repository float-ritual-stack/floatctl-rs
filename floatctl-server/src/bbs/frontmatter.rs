//! YAML frontmatter parsing and writing (gray-matter equivalent)
//!
//! Handles markdown files with YAML frontmatter:
//! ```markdown
//! ---
//! title: "Example"
//! date: "2025-12-07T05:00:00Z"
//! ---
//!
//! Content here
//! ```

use chrono::Utc;
use serde::{de::DeserializeOwned, Serialize};
use thiserror::Error;

const FRONTMATTER_DELIMITER: &str = "---";

/// Frontmatter parsing/writing errors
#[derive(Debug, Error)]
pub enum FrontmatterError {
    #[error("no frontmatter found - file must start with ---")]
    NoFrontmatter,

    #[error("unclosed frontmatter - missing second ---")]
    Unclosed,

    #[error("YAML parse error: {0}")]
    YamlParse(#[from] serde_yaml::Error),
}

/// Parse markdown file with YAML frontmatter
///
/// Returns (frontmatter struct, body content)
pub fn parse_frontmatter<T: DeserializeOwned>(content: &str) -> Result<(T, String), FrontmatterError> {
    let trimmed = content.trim_start();

    if !trimmed.starts_with(FRONTMATTER_DELIMITER) {
        return Err(FrontmatterError::NoFrontmatter);
    }

    // Skip the first "---" and find the closing one
    let after_first = &trimmed[3..];
    let end_pos = after_first
        .find(FRONTMATTER_DELIMITER)
        .ok_or(FrontmatterError::Unclosed)?;

    let yaml_content = after_first[..end_pos].trim();
    let body_start = 3 + end_pos + 3; // "---" + content + "---"
    let body = trimmed
        .get(body_start..)
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    let frontmatter: T = serde_yaml::from_str(yaml_content)?;

    Ok((frontmatter, body))
}

/// Write content with YAML frontmatter
pub fn write_with_frontmatter<T: Serialize>(frontmatter: &T, body: &str) -> Result<String, serde_yaml::Error> {
    let yaml = serde_yaml::to_string(frontmatter)?;

    Ok(format!("---\n{}---\n\n{}", yaml, body.trim()))
}

/// Generate slug from title (for filenames)
///
/// - Lowercase
/// - Non-alphanumeric → dash
/// - Collapse multiple dashes
/// - Max 50 chars
pub fn slugify(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(50)
        .collect()
}

/// Generate timestamped message ID
///
/// Format: `YYYY-MM-DD-HHMM-from-{sender}-{short_uuid}`
pub fn generate_message_id(from: &str) -> String {
    let now = Utc::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H%M").to_string();
    // Add short UUID suffix for uniqueness when multiple messages same minute
    let short_id = &uuid::Uuid::new_v4().to_string()[..8];
    format!("{}-{}-from-{}-{}", date_str, time_str, from, short_id)
}

/// Generate timestamped content ID (for memories, posts)
///
/// Format: `YYYY-MM-DD-{slug}`
pub fn generate_content_id(title: &str) -> String {
    let date_str = Utc::now().format("%Y-%m-%d").to_string();
    let slug = slugify(title);
    format!("{}-{}", date_str, slug)
}

/// Generate preview from content (first N chars, no newlines)
pub fn generate_preview(content: &str, max_len: usize) -> String {
    content
        .chars()
        .take(max_len)
        .map(|c| if c == '\n' { ' ' } else { c })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestFrontmatter {
        title: String,
        author: String,
    }

    #[test]
    fn test_parse_frontmatter() {
        let content = r#"---
title: Test Post
author: kitty
---

This is the body content.
"#;

        let (fm, body): (TestFrontmatter, String) = parse_frontmatter(content).unwrap();
        assert_eq!(fm.title, "Test Post");
        assert_eq!(fm.author, "kitty");
        assert!(body.contains("body content"));
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "Just plain text";
        let result: Result<(TestFrontmatter, String), _> = parse_frontmatter(content);
        assert!(matches!(result, Err(FrontmatterError::NoFrontmatter)));
    }

    #[test]
    fn test_parse_unclosed_frontmatter() {
        let content = "---\ntitle: Test\n";
        let result: Result<(TestFrontmatter, String), _> = parse_frontmatter(content);
        assert!(matches!(result, Err(FrontmatterError::Unclosed)));
    }

    #[test]
    fn test_write_with_frontmatter() {
        let fm = TestFrontmatter {
            title: "Test".to_string(),
            author: "kitty".to_string(),
        };

        let result = write_with_frontmatter(&fm, "Body here").unwrap();
        assert!(result.starts_with("---\n"));
        assert!(result.contains("title: Test"));
        assert!(result.contains("author: kitty"));
        assert!(result.contains("Body here"));
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World!"), "hello-world");
        assert_eq!(slugify("Test 123 @#$"), "test-123");
        assert_eq!(slugify("Multiple   Spaces"), "multiple-spaces");
        assert_eq!(slugify("---dashes---"), "dashes");
    }

    #[test]
    fn test_slugify_max_length() {
        let long_title = "a".repeat(100);
        let slug = slugify(&long_title);
        assert_eq!(slug.len(), 50);
    }

    #[test]
    fn test_generate_message_id() {
        let id = generate_message_id("kitty");
        // Format: YYYY-MM-DD-HHMM-from-kitty-{short_uuid}
        assert!(id.contains("-from-kitty-"));
        assert!(id.len() > 28); // date(10) + time(5) + from-kitty(11) + uuid(9) = 35+
    }

    #[test]
    fn test_generate_content_id() {
        let id = generate_content_id("My Cool Pattern");
        // Format: YYYY-MM-DD-my-cool-pattern
        assert!(id.contains("-my-cool-pattern"));
    }

    #[test]
    fn test_generate_preview() {
        let content = "Line one\nLine two\nLine three";
        let preview = generate_preview(content, 20);
        assert!(!preview.contains('\n'));
        assert!(preview.len() <= 20);
    }
}

use once_cell::sync::Lazy;
use regex::Regex;

use super::types::{Annotation, Block};

/// Regex for matching ctx:: entries
/// Format: ctx::YYYY-MM-DD @ HH:MM:SS [AM|PM] - description
static CTX_MARKER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^ctx::([\d-]+\s*@\s*[\d:]+\s*(?:AM|PM)?)\s*-?\s*(.*)$").unwrap()
});

/// Regex for matching annotations in text
/// Format: word::value (project::name, meeting::id, etc)
static ANNOTATION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[([a-zA-Z_]+)::([\w\s\-./]+)\]").unwrap()
});

/// Regex for matching wikilinks
/// Format: [[target]]
static WIKILINK_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[\[([^\]]+)\]\]").unwrap()
});

/// Parse scratch input into blocks
pub struct ScratchParser;

impl ScratchParser {
    /// Parse a line of scratch input into a block
    pub fn parse_line(line: &str) -> Option<Block> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Check if it's a ctx:: entry
        if let Some(captures) = CTX_MARKER_RE.captures(line) {
            let marker = captures.get(1).map(|m| m.as_str()).unwrap_or("");
            let description = captures.get(2).map(|m| m.as_str()).unwrap_or("");

            // Full marker includes "ctx::" prefix
            let full_marker = format!("ctx::{}", marker);

            // Parse annotations from the description
            let annotations = Self::extract_annotations(description);

            // For now, description is a single-line content
            let content = vec![description.to_string()];

            return Some(Block::new_context_entry(full_marker, content, annotations));
        }

        // Otherwise, treat as plain text
        Some(Block::new_text(line.to_string()))
    }

    /// Parse a multi-line scratch entry into blocks
    /// Handles bullet points under ctx:: markers
    pub fn parse_entry(text: &str) -> Vec<Block> {
        let lines: Vec<&str> = text.lines().collect();
        let mut blocks = Vec::new();
        let mut current_ctx: Option<(String, Vec<String>, Vec<Annotation>)> = None;

        for line in lines {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            // Check if it's a new ctx:: marker
            if let Some(captures) = CTX_MARKER_RE.captures(trimmed) {
                // Save previous ctx entry if exists
                if let Some((marker, content, annotations)) = current_ctx.take() {
                    blocks.push(Block::new_context_entry(marker, content, annotations));
                }

                // Start new ctx entry
                let marker = captures.get(1).map(|m| m.as_str()).unwrap_or("");
                let description = captures.get(2).map(|m| m.as_str()).unwrap_or("");
                let full_marker = format!("ctx::{}", marker);
                let annotations = Self::extract_annotations(description);

                current_ctx = Some((full_marker, vec![description.to_string()], annotations));
            } else if let Some((_, ref mut content, ref mut annotations)) = current_ctx {
                // Continuation of current ctx entry (bullet point or text)
                let cleaned = trimmed.trim_start_matches(&['•', '-', '*', '–'][..]).trim();

                // Extract annotations from this line too
                let line_annotations = Self::extract_annotations(cleaned);
                annotations.extend(line_annotations);

                content.push(cleaned.to_string());
            } else {
                // Standalone text line
                blocks.push(Block::new_text(trimmed.to_string()));
            }
        }

        // Don't forget the last ctx entry
        if let Some((marker, content, annotations)) = current_ctx {
            blocks.push(Block::new_context_entry(marker, content, annotations));
        }

        blocks
    }

    /// Extract annotations from text
    /// Looks for [key::value] patterns
    pub fn extract_annotations(text: &str) -> Vec<Annotation> {
        ANNOTATION_RE
            .captures_iter(text)
            .filter_map(|cap| {
                let key = cap.get(1)?.as_str();
                let value = cap.get(2)?.as_str();
                Annotation::parse(&format!("{}::{}", key, value))
            })
            .collect()
    }

    /// Extract wikilink targets from text
    pub fn extract_wikilinks(text: &str) -> Vec<String> {
        WIKILINK_RE
            .captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_ctx() {
        let line = "ctx::2025-11-15 @ 14:30 - brain boot";
        let block = ScratchParser::parse_line(line).unwrap();

        match block {
            Block::ContextEntry { marker, content, .. } => {
                assert!(marker.starts_with("ctx::"));
                assert_eq!(content.len(), 1);
                assert_eq!(content[0], "brain boot");
            }
            _ => panic!("Expected ContextEntry"),
        }
    }

    #[test]
    fn test_parse_ctx_with_annotations() {
        let line = "ctx::2025-11-15 @ 14:30 - [project::rangle] working on pharmacy";
        let block = ScratchParser::parse_line(line).unwrap();

        match block {
            Block::ContextEntry { annotations, .. } => {
                assert_eq!(annotations.len(), 1);
                assert_eq!(annotations[0], Annotation::Project("rangle".into()));
            }
            _ => panic!("Expected ContextEntry"),
        }
    }

    #[test]
    fn test_parse_multi_line_entry() {
        let text = r#"ctx::2025-11-15 @ 14:30 - brain boot
  - good morning
  - [project::rangle] check pharmacy
  - standup prep

ctx::2025-11-15 @ 14:35 - docker issues
  - staging broken
  - orphaned containers"#;

        let blocks = ScratchParser::parse_entry(text);
        assert_eq!(blocks.len(), 2);

        // First block
        match &blocks[0] {
            Block::ContextEntry { content, annotations, .. } => {
                assert_eq!(content.len(), 4);
                assert_eq!(content[0], "brain boot");
                assert_eq!(content[1], "good morning");
                assert!(annotations.iter().any(|a| matches!(a, Annotation::Project(p) if p == "rangle")));
            }
            _ => panic!("Expected ContextEntry"),
        }

        // Second block
        match &blocks[1] {
            Block::ContextEntry { content, .. } => {
                assert_eq!(content.len(), 3);
                assert_eq!(content[0], "docker issues");
            }
            _ => panic!("Expected ContextEntry"),
        }
    }

    #[test]
    fn test_parse_plain_text() {
        let line = "just some regular text";
        let block = ScratchParser::parse_line(line).unwrap();

        match block {
            Block::Text { content, .. } => {
                assert_eq!(content, "just some regular text");
            }
            _ => panic!("Expected Text block"),
        }
    }

    #[test]
    fn test_extract_annotations() {
        let text = "working on [project::rangle] for [meeting::standup]";
        let annotations = ScratchParser::extract_annotations(text);

        assert_eq!(annotations.len(), 2);
        assert!(annotations.contains(&Annotation::Project("rangle".into())));
        assert!(annotations.contains(&Annotation::Meeting("standup".into())));
    }

    #[test]
    fn test_extract_wikilinks() {
        let text = "see [[other note]] and [[another one]]";
        let links = ScratchParser::extract_wikilinks(text);

        assert_eq!(links.len(), 2);
        assert_eq!(links[0], "other note");
        assert_eq!(links[1], "another one");
    }

    #[test]
    fn test_empty_input() {
        assert!(ScratchParser::parse_line("").is_none());
        assert!(ScratchParser::parse_line("   ").is_none());

        let blocks = ScratchParser::parse_entry("");
        assert_eq!(blocks.len(), 0);
    }

    #[test]
    fn test_different_ctx_formats() {
        let cases = vec![
            "ctx::2025-11-15 @ 14:30 PM - description",
            "ctx::2025-11-15 @ 14:30 - description",
            "ctx::2025-11-15@14:30-description",
        ];

        for input in cases {
            let block = ScratchParser::parse_line(input);
            assert!(block.is_some(), "Failed to parse: {}", input);
            assert!(matches!(block.unwrap(), Block::ContextEntry { .. }));
        }
    }
}

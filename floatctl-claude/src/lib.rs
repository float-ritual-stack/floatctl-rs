/*!
 * floatctl-claude - Query and analyze Claude Code session logs
 *
 * This crate provides utilities for reading and analyzing JSONL logs
 * from Claude Code sessions (~/.claude/projects/).
 */

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod stream;
pub mod parser;
pub mod commands;

/// Claude Code log entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    #[serde(rename = "type")]
    pub entry_type: String,
    pub timestamp: String,
    #[serde(default)]
    pub operation: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub message: Option<MessageData>,
    #[serde(rename = "sessionId", default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(rename = "gitBranch", default)]
    pub git_branch: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(rename = "parentUuid", default)]
    pub parent_uuid: Option<String>,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(rename = "isSidechain", default)]
    pub is_sidechain: Option<bool>,
    #[serde(rename = "userType", default)]
    pub user_type: Option<String>,
    #[serde(rename = "agentId", default)]
    pub agent_id: Option<String>,
    #[serde(rename = "requestId", default)]
    pub request_id: Option<String>,
}

/// Message data from Claude API
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageData {
    pub model: String,
    pub id: String,
    #[serde(rename = "type")]
    pub message_type: String,
    pub role: String,
    pub content: Vec<ContentBlock>,
    pub stop_reason: Option<String>,
    pub usage: Option<Usage>,
}

/// Content block (can be text, thinking, tool_use, tool_result)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    Text { text: String },
    Thinking { thinking: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(default)]
        is_error: bool,
    },
}

/// Token usage stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    #[serde(default)]
    pub cache_creation_input_tokens: u32,
    #[serde(default)]
    pub cache_read_input_tokens: u32,
    pub output_tokens: u32,
}

/// Simplified message for output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub timestamp: String,
    pub content: String,
    pub truncated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Tool call information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    pub input: serde_json::Value,
}

/// Session summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub started: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ended: Option<String>,
    pub stats: SessionStats,
}

/// Session statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionStats {
    pub turn_count: usize,
    pub tool_calls: usize,
    pub failures: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_input_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_output_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_tokens: Option<u32>,
}

impl Default for SessionStats {
    fn default() -> Self {
        Self {
            turn_count: 0,
            tool_calls: 0,
            failures: 0,
            total_input_tokens: None,
            total_output_tokens: None,
            cache_read_tokens: None,
            cache_creation_tokens: None,
        }
    }
}

/// Smart truncation with sentence/word boundary awareness
/// Returns (truncated_text, was_truncated)
///
/// Algorithm (copied from evna's active-context-stream.ts):
/// 1. If text <= max_len, return as-is
/// 2. Search backwards from max_len + 50 for sentence ending (. ! ?)
/// 3. Fallback to word boundary if no sentence found
/// 4. Last resort: respect UTF-8 boundaries and hard truncate
pub fn smart_truncate(text: &str, max_len: usize) -> (String, bool) {
    if text.len() <= max_len {
        return (text.to_string(), false);
    }

    // Search backwards from max_len + 50 to find last sentence ending
    let search_start = max_len.saturating_sub(50);
    let search_end = (max_len + 50).min(text.len());

    // Find sentence boundary
    if let Some(pos) = text[search_start..search_end]
        .rfind(|c| c == '.' || c == '!' || c == '?')
    {
        let cut_point = search_start + pos + 1;
        if text.is_char_boundary(cut_point) {
            return (text[..cut_point].to_string(), true);
        }
    }

    // Fallback: word boundary
    // Need to find a safe substring first that respects UTF-8 boundaries
    let safe_max = {
        let mut pos = max_len.min(text.len());
        while pos > 0 && !text.is_char_boundary(pos) {
            pos -= 1;
        }
        pos
    };

    if safe_max > 0 {
        if let Some(pos) = text[..safe_max].rfind(char::is_whitespace) {
            return (text[..pos].to_string(), true);
        }
    }

    // Last resort: respect UTF-8 boundaries
    let mut cut_point = max_len.min(text.len());
    while cut_point > 0 && !text.is_char_boundary(cut_point) {
        cut_point -= 1;
    }
    (text[..cut_point].to_string(), true)
}

/// Find all session log files in a directory
pub fn find_session_logs(projects_dir: &Path) -> Result<Vec<PathBuf>> {
    let mut logs = Vec::new();

    if !projects_dir.exists() {
        return Ok(logs);
    }

    for entry in walkdir::WalkDir::new(projects_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("jsonl")
            && !path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .starts_with('.')
        {
            logs.push(path.to_path_buf());
        }
    }

    Ok(logs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_smart_truncate_short_text() {
        let text = "Hello world";
        let (truncated, was_truncated) = smart_truncate(text, 100);
        assert_eq!(truncated, text);
        assert!(!was_truncated);
    }

    #[test]
    fn test_smart_truncate_sentence_boundary() {
        let text = "This is the first sentence. This is the second sentence. This is the third.";
        let (truncated, was_truncated) = smart_truncate(text, 50);

        assert!(was_truncated);
        // Should cut at sentence boundary
        assert!(truncated.ends_with('.') || truncated.ends_with('!') || truncated.ends_with('?'));
        assert!(truncated.len() <= 100); // max_len + 50
    }

    #[test]
    fn test_smart_truncate_word_boundary() {
        let text = "This is a long text without any sentence endings and it keeps going";
        let (truncated, was_truncated) = smart_truncate(text, 30);

        assert!(was_truncated);
        // Should cut at word boundary
        assert!(truncated.chars().last().map(|c| c.is_whitespace() || c.is_alphanumeric()).unwrap_or(true));
    }

    #[test]
    fn test_smart_truncate_utf8_boundary() {
        let text = "Hello 世界 this is unicode text";
        let (truncated, was_truncated) = smart_truncate(text, 10);

        assert!(was_truncated);
        // Should not panic and respect UTF-8 boundaries
        assert!(truncated.len() <= 10);
    }

    #[test]
    fn test_smart_truncate_exact_length() {
        let text = "Exactly fifty characters in this text right here!";
        let (truncated, was_truncated) = smart_truncate(text, 50);

        assert!(!was_truncated);
        assert_eq!(truncated, text);
    }
}

/*!
 * Parse and extract information from Claude Code log entries
 */

use crate::{ContentBlock, LogEntry, Message, SessionStats, ToolCall};

/// Extract messages from log entries
/// Filters for user/assistant entries and extracts text content
pub fn extract_messages(entries: &[LogEntry]) -> Vec<Message> {
    entries
        .iter()
        .filter_map(|entry| {
            // Only process user and assistant messages
            if entry.entry_type != "user" && entry.entry_type != "assistant" {
                return None;
            }

            let message = entry.message.as_ref()?;
            let role = message.role.clone();
            let timestamp = entry.timestamp.clone();

            // Extract text content and tool calls
            let mut text_parts = Vec::new();
            let mut tool_calls = Vec::new();

            for block in &message.content {
                match block {
                    ContentBlock::Text { text } => {
                        text_parts.push(text.clone());
                    }
                    ContentBlock::Thinking { thinking } => {
                        // Optionally include thinking blocks
                        // For now, skip them - can add a flag later
                        let _ = thinking;
                    }
                    ContentBlock::ToolUse { id: _, name, input } => {
                        tool_calls.push(ToolCall {
                            name: name.clone(),
                            input: input.clone(),
                        });
                    }
                    ContentBlock::ToolResult { .. } => {
                        // Tool results are in user messages, skip for content extraction
                    }
                }
            }

            // If we have queue-operation entries with content, use that for user messages
            let content = if text_parts.is_empty() && entry.entry_type == "user" {
                entry.content.clone().unwrap_or_default()
            } else {
                text_parts.join("\n")
            };

            if content.is_empty() && tool_calls.is_empty() {
                return None;
            }

            Some(Message {
                role,
                timestamp,
                content,
                truncated: false,
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(tool_calls)
                },
            })
        })
        .collect()
}

/// Calculate session statistics from log entries
pub fn calculate_stats(entries: &[LogEntry]) -> SessionStats {
    let mut stats = SessionStats::default();

    let mut user_turns = 0;
    let mut assistant_turns = 0;
    let mut total_input = 0u32;
    let mut total_output = 0u32;
    let mut total_cache_read = 0u32;
    let mut total_cache_creation = 0u32;

    for entry in entries {
        match entry.entry_type.as_str() {
            "user" => user_turns += 1,
            "assistant" => {
                assistant_turns += 1;

                // Accumulate token usage
                if let Some(message) = &entry.message {
                    if let Some(usage) = &message.usage {
                        total_input += usage.input_tokens;
                        total_output += usage.output_tokens;
                        total_cache_read += usage.cache_read_input_tokens;
                        total_cache_creation += usage.cache_creation_input_tokens;
                    }

                    // Count tool calls
                    for block in &message.content {
                        if matches!(block, ContentBlock::ToolUse { .. }) {
                            stats.tool_calls += 1;
                        }
                    }
                }
            }
            _ => {}
        }

        // Check for tool failures (heuristic: tool_result with error indicators)
        if entry.entry_type == "user" {
            if let Some(message) = &entry.message {
                for block in &message.content {
                    if let ContentBlock::ToolResult { content, is_error, .. } = block {
                        if *is_error
                            || content.to_lowercase().contains("error")
                            || content.to_lowercase().contains("failed")
                            || content.to_lowercase().contains("not found")
                        {
                            stats.failures += 1;
                        }
                    }
                }
            }
        }
    }

    stats.turn_count = user_turns + assistant_turns;
    stats.total_input_tokens = if total_input > 0 { Some(total_input) } else { None };
    stats.total_output_tokens = if total_output > 0 { Some(total_output) } else { None };
    stats.cache_read_tokens = if total_cache_read > 0 {
        Some(total_cache_read)
    } else {
        None
    };
    stats.cache_creation_tokens = if total_cache_creation > 0 {
        Some(total_cache_creation)
    } else {
        None
    };

    stats
}

/// Get session metadata from log entries
pub fn get_session_metadata(entries: &[LogEntry]) -> Option<SessionMetadata> {
    if entries.is_empty() {
        return None;
    }

    let first = &entries[0];
    let last = &entries[entries.len() - 1];

    Some(SessionMetadata {
        session_id: first.session_id.clone().unwrap_or_default(),
        project: first.cwd.clone().unwrap_or_default(),
        branch: first.git_branch.clone(),
        version: first.version.clone(),
        started: first.timestamp.clone(),
        ended: last.timestamp.clone(),
    })
}

/// Session metadata extracted from log entries
#[derive(Debug, Clone)]
pub struct SessionMetadata {
    pub session_id: String,
    pub project: String,
    pub branch: Option<String>,
    pub version: Option<String>,
    pub started: String,
    pub ended: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentBlock, MessageData, Usage};

    fn create_test_entry(entry_type: &str, role: &str, content_text: &str) -> LogEntry {
        LogEntry {
            entry_type: entry_type.to_string(),
            timestamp: "2025-11-09T01:13:40.906Z".to_string(),
            operation: None,
            content: Some(content_text.to_string()),
            message: Some(MessageData {
                model: Some("claude-sonnet-4-5".to_string()),
                id: Some("msg_123".to_string()),
                message_type: Some("message".to_string()),
                role: role.to_string(),
                content: vec![ContentBlock::Text {
                    text: content_text.to_string(),
                }],
                stop_reason: None,
                usage: Some(Usage {
                    input_tokens: 100,
                    cache_creation_input_tokens: 0,
                    cache_read_input_tokens: 0,
                    output_tokens: 50,
                }),
            }),
            session_id: Some("test-session".to_string()),
            cwd: Some("/home/user/project".to_string()),
            git_branch: Some("main".to_string()),
            version: Some("1.0.0".to_string()),
            parent_uuid: None,
            uuid: Some("uuid-123".to_string()),
            is_sidechain: None,
            user_type: None,
            agent_id: None,
            request_id: None,
        }
    }

    #[test]
    fn test_extract_messages() {
        let entries = vec![
            create_test_entry("user", "user", "Hello"),
            create_test_entry("assistant", "assistant", "Hi there"),
            create_test_entry("queue-operation", "user", "ignored"),
        ];

        let messages = extract_messages(&entries);

        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].role, "user");
        assert_eq!(messages[0].content, "Hello");
        assert_eq!(messages[1].role, "assistant");
        assert_eq!(messages[1].content, "Hi there");
    }

    #[test]
    fn test_calculate_stats() {
        let entries = vec![
            create_test_entry("user", "user", "Question 1"),
            create_test_entry("assistant", "assistant", "Answer 1"),
            create_test_entry("user", "user", "Question 2"),
            create_test_entry("assistant", "assistant", "Answer 2"),
        ];

        let stats = calculate_stats(&entries);

        assert_eq!(stats.turn_count, 4);
        assert_eq!(stats.total_input_tokens, Some(200)); // 100 * 2 assistant messages
        assert_eq!(stats.total_output_tokens, Some(100)); // 50 * 2 assistant messages
    }

    #[test]
    fn test_get_session_metadata() {
        let entries = vec![
            create_test_entry("user", "user", "First"),
            create_test_entry("assistant", "assistant", "Second"),
        ];

        let metadata = get_session_metadata(&entries).expect("Should extract metadata");

        assert_eq!(metadata.session_id, "test-session");
        assert_eq!(metadata.project, "/home/user/project");
        assert_eq!(metadata.branch, Some("main".to_string()));
    }

    #[test]
    fn test_empty_entries() {
        let entries: Vec<LogEntry> = vec![];
        let messages = extract_messages(&entries);
        let metadata = get_session_metadata(&entries);

        assert_eq!(messages.len(), 0);
        assert!(metadata.is_none());
    }
}

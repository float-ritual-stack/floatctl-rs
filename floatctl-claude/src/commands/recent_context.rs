/*!
 * Recent context command - EVNA's primary use case
 *
 * Extracts first/last messages from recent Claude Code sessions
 * for system prompt injection
 */

use crate::{find_session_logs, parser, smart_truncate, stream, Message, SessionStats};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Options for recent context extraction
#[derive(Debug, Clone)]
pub struct RecentContextOptions {
    /// Number of recent sessions to include
    pub sessions: usize,
    /// Number of first messages per session
    pub first: usize,
    /// Number of last messages per session
    pub last: usize,
    /// Maximum characters per message (0 = no truncation)
    pub truncate: usize,
    /// Project filter (matches if project path contains this string)
    pub project_filter: Option<String>,
}

impl Default for RecentContextOptions {
    fn default() -> Self {
        Self {
            sessions: 3,
            first: 3,
            last: 3,
            truncate: 400,
            project_filter: None,
        }
    }
}

/// Session context with first/last messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContext {
    pub session_id: String,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub started: String,
    pub first_messages: Vec<Message>,
    pub last_messages: Vec<Message>,
    pub stats: SessionStats,
}

/// Result of recent context extraction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecentContextResult {
    pub sessions: Vec<SessionContext>,
}

/// Extract recent context from Claude Code sessions
pub fn recent_context(
    projects_dir: &Path,
    options: &RecentContextOptions,
) -> Result<RecentContextResult> {
    // Find all session log files
    let mut session_logs = find_session_logs(projects_dir)?;

    // Sort by modification time (most recent first)
    session_logs.sort_by_key(|path| {
        fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .map(|t| std::cmp::Reverse(t))
    });

    // Filter by project if specified
    if let Some(ref filter) = options.project_filter {
        session_logs.retain(|path| {
            path.to_str()
                .map(|s| s.contains(filter))
                .unwrap_or(false)
        });
    }

    // Take N most recent sessions
    let recent_logs: Vec<PathBuf> = session_logs.into_iter().take(options.sessions).collect();

    // Process each session
    let mut sessions = Vec::new();

    for log_path in recent_logs {
        match process_session(&log_path, options) {
            Ok(Some(session)) => sessions.push(session),
            Ok(None) => continue, // Empty session, skip
            Err(e) => {
                eprintln!(
                    "Warning: Failed to process {}: {}",
                    log_path.display(),
                    e
                );
                continue;
            }
        }
    }

    Ok(RecentContextResult { sessions })
}

/// Process a single session log file
fn process_session(
    log_path: &Path,
    options: &RecentContextOptions,
) -> Result<Option<SessionContext>> {
    // Read all log entries
    let entries = stream::read_log_file(log_path)
        .with_context(|| format!("Failed to read log file: {}", log_path.display()))?;

    if entries.is_empty() {
        return Ok(None);
    }

    // Extract metadata
    let metadata = parser::get_session_metadata(&entries)
        .context("Failed to extract session metadata")?;

    // Extract all messages
    let mut all_messages = parser::extract_messages(&entries);

    // Apply truncation if requested
    if options.truncate > 0 {
        for msg in &mut all_messages {
            let (truncated, was_truncated) = smart_truncate(&msg.content, options.truncate);
            msg.content = truncated;
            msg.truncated = was_truncated;
        }
    }

    // Get first N and last N messages
    let first_messages: Vec<Message> = all_messages
        .iter()
        .take(options.first)
        .cloned()
        .collect();

    let last_messages: Vec<Message> = if all_messages.len() <= options.first {
        // If total messages <= first N, don't duplicate - return empty last
        Vec::new()
    } else {
        all_messages
            .iter()
            .rev()
            .take(options.last)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    };

    // Calculate stats
    let stats = parser::calculate_stats(&entries);

    Ok(Some(SessionContext {
        session_id: metadata.session_id,
        project: metadata.project,
        branch: metadata.branch,
        started: metadata.started,
        first_messages,
        last_messages,
        stats,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ContentBlock, LogEntry, MessageData, Usage};
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    fn create_test_log_entry(entry_type: &str, role: &str, text: &str) -> LogEntry {
        LogEntry {
            entry_type: entry_type.to_string(),
            timestamp: Some("2025-11-09T01:13:40.906Z".to_string()),
            operation: None,
            content: Some(text.to_string()),
            message: Some(MessageData {
                model: Some("claude-sonnet-4-5".to_string()),
                id: Some("msg_123".to_string()),
                message_type: Some("message".to_string()),
                role: role.to_string(),
                content: vec![ContentBlock::Text {
                    text: text.to_string(),
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
            cwd: Some("/home/user/test-project".to_string()),
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

    fn create_test_session_file() -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;

        // Write a session with 5 messages
        for i in 1..=5 {
            let role = if i % 2 == 1 { "user" } else { "assistant" };
            let entry_type = role;
            let text = format!("Message {}", i);
            let entry = create_test_log_entry(entry_type, role, &text);
            writeln!(file, "{}", serde_json::to_string(&entry)?)?;
        }

        file.flush()?;
        Ok(file)
    }

    #[test]
    fn test_process_session() -> Result<()> {
        let file = create_test_session_file()?;
        let options = RecentContextOptions {
            first: 2,
            last: 2,
            ..Default::default()
        };

        let session = process_session(file.path(), &options)?
            .expect("Should process session");

        assert_eq!(session.session_id, "test-session");
        assert_eq!(session.project, "/home/user/test-project");
        assert_eq!(session.first_messages.len(), 2);
        assert_eq!(session.last_messages.len(), 2);

        // Check first messages
        assert!(session.first_messages[0].content.contains("Message 1"));
        assert!(session.first_messages[1].content.contains("Message 2"));

        // Check last messages
        assert!(session.last_messages[0].content.contains("Message 4"));
        assert!(session.last_messages[1].content.contains("Message 5"));

        Ok(())
    }

    #[test]
    fn test_truncation() -> Result<()> {
        let mut file = NamedTempFile::new()?;

        let long_text = "This is a very long message that should be truncated. ".repeat(20);
        let entry = create_test_log_entry("user", "user", &long_text);
        writeln!(file, "{}", serde_json::to_string(&entry)?)?;
        file.flush()?;

        let options = RecentContextOptions {
            first: 1,
            last: 0,
            truncate: 100,
            ..Default::default()
        };

        let session = process_session(file.path(), &options)?
            .expect("Should process session");

        assert_eq!(session.first_messages.len(), 1);
        let msg = &session.first_messages[0];
        assert!(msg.truncated);
        assert!(msg.content.len() <= 150); // max_len + 50 for sentence boundary

        Ok(())
    }

    #[test]
    fn test_recent_context_integration() -> Result<()> {
        let dir = tempdir()?;
        let project_dir = dir.path().join("test-project");
        fs::create_dir(&project_dir)?;

        // Create a session file
        let session_path = project_dir.join("session1.jsonl");
        let entry = create_test_log_entry("user", "user", "Test message");
        fs::write(&session_path, serde_json::to_string(&entry)?)?;

        let options = RecentContextOptions::default();
        let result = recent_context(dir.path(), &options)?;

        assert!(!result.sessions.is_empty());

        Ok(())
    }

    #[test]
    fn test_no_duplicate_messages() -> Result<()> {
        let mut file = NamedTempFile::new()?;

        // Only 2 messages total
        for i in 1..=2 {
            let role = if i % 2 == 1 { "user" } else { "assistant" };
            let entry = create_test_log_entry(role, role, &format!("Message {}", i));
            writeln!(file, "{}", serde_json::to_string(&entry)?)?;
        }
        file.flush()?;

        let options = RecentContextOptions {
            first: 2,
            last: 2,
            ..Default::default()
        };

        let session = process_session(file.path(), &options)?
            .expect("Should process session");

        // First should have all messages, last should be empty (to avoid duplication)
        assert_eq!(session.first_messages.len(), 2);
        assert_eq!(session.last_messages.len(), 0);

        Ok(())
    }
}

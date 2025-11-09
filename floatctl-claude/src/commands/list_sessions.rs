/*!
 * List recent Claude Code sessions
 *
 * Replaces evna's TypeScript implementation (list_recent_claude_sessions)
 * Reads ~/.claude/history.jsonl for session metadata
 */

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

/// Session entry from history.jsonl
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub timestamp: String,
    pub project: String,
    #[serde(default)]
    pub display: String,
    #[serde(rename = "sessionId", default)]
    pub session_id: Option<String>,
}

/// Options for listing sessions
#[derive(Debug, Clone)]
pub struct ListSessionsOptions {
    pub limit: usize,
    pub project_filter: Option<String>,
}

impl Default for ListSessionsOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            project_filter: None,
        }
    }
}

/// List recent Claude Code sessions from history.jsonl
pub fn list_sessions(history_path: &Path, options: &ListSessionsOptions) -> Result<Vec<HistoryEntry>> {
    let file = File::open(history_path)
        .with_context(|| format!("Failed to open history file: {}", history_path.display()))?;

    let reader = BufReader::new(file);
    let mut sessions = Vec::new();

    // Read all lines
    for line in reader.lines() {
        let line = line?;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Parse as JSON
        match serde_json::from_str::<HistoryEntry>(trimmed) {
            Ok(entry) => {
                // Apply project filter if specified
                if let Some(ref filter) = options.project_filter {
                    if !entry.project.contains(filter) {
                        continue;
                    }
                }
                sessions.push(entry);
            }
            Err(_) => {
                // Skip malformed lines
                continue;
            }
        }
    }

    // Take last N sessions (most recent)
    let start_idx = sessions.len().saturating_sub(options.limit);
    let recent = sessions[start_idx..].to_vec();

    // Reverse to show most recent first
    Ok(recent.into_iter().rev().collect())
}

/// Get default history path (~/.claude/history.jsonl)
pub fn default_history_path() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".claude")
        .join("history.jsonl")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn create_test_history() -> Result<NamedTempFile> {
        let mut file = NamedTempFile::new()?;

        // Write sample history entries
        writeln!(
            file,
            r#"{{"timestamp":"2025-11-09T01:00:00Z","project":"/home/user/project1","display":"Session 1","sessionId":"abc123"}}"#
        )?;
        writeln!(
            file,
            r#"{{"timestamp":"2025-11-09T02:00:00Z","project":"/home/user/project2","display":"Session 2","sessionId":"def456"}}"#
        )?;
        writeln!(
            file,
            r#"{{"timestamp":"2025-11-09T03:00:00Z","project":"/home/user/project1","display":"Session 3","sessionId":"ghi789"}}"#
        )?;

        file.flush()?;
        Ok(file)
    }

    #[test]
    fn test_list_sessions_basic() -> Result<()> {
        let file = create_test_history()?;
        let options = ListSessionsOptions::default();

        let sessions = list_sessions(file.path(), &options)?;

        assert_eq!(sessions.len(), 3);
        // Should be reversed (most recent first)
        assert_eq!(sessions[0].display, "Session 3");
        assert_eq!(sessions[1].display, "Session 2");
        assert_eq!(sessions[2].display, "Session 1");

        Ok(())
    }

    #[test]
    fn test_list_sessions_with_limit() -> Result<()> {
        let file = create_test_history()?;
        let options = ListSessionsOptions {
            limit: 2,
            ..Default::default()
        };

        let sessions = list_sessions(file.path(), &options)?;

        assert_eq!(sessions.len(), 2);
        assert_eq!(sessions[0].display, "Session 3");
        assert_eq!(sessions[1].display, "Session 2");

        Ok(())
    }

    #[test]
    fn test_list_sessions_with_project_filter() -> Result<()> {
        let file = create_test_history()?;
        let options = ListSessionsOptions {
            limit: 10,
            project_filter: Some("project1".to_string()),
        };

        let sessions = list_sessions(file.path(), &options)?;

        assert_eq!(sessions.len(), 2);
        assert!(sessions.iter().all(|s| s.project.contains("project1")));

        Ok(())
    }

    #[test]
    fn test_empty_history() -> Result<()> {
        let file = NamedTempFile::new()?;
        let options = ListSessionsOptions::default();

        let sessions = list_sessions(file.path(), &options)?;

        assert_eq!(sessions.len(), 0);

        Ok(())
    }
}

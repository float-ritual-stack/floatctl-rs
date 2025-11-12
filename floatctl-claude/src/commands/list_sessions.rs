/*!
 * List recent Claude Code sessions
 *
 * Scans ~/.claude/projects/ for session .jsonl files and extracts metadata
 */

use crate::{parser, stream};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Check if a session ID represents an agent session
/// Agent sessions have IDs starting with "agent-" (from nested Agent SDK calls)
fn is_agent_session(session_id: &str) -> bool {
    session_id.starts_with("agent-")
}

/// Session summary for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub session_id: String,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    pub started: String,
    pub ended: String,
    pub turn_count: usize,
    pub tool_calls: usize,
}

/// Options for listing sessions
#[derive(Debug, Clone)]
pub struct ListSessionsOptions {
    pub limit: usize,
    pub project_filter: Option<String>,
    pub include_agents: bool,
}

impl Default for ListSessionsOptions {
    fn default() -> Self {
        Self {
            limit: 10,
            project_filter: None,
            include_agents: false,
        }
    }
}

/// List recent Claude Code sessions from projects directory
pub fn list_sessions(projects_dir: &Path, options: &ListSessionsOptions) -> Result<Vec<SessionSummary>> {
    let mut sessions = Vec::new();

    // Walk through projects directory finding .jsonl files
    for entry in WalkDir::new(projects_dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();

        // Skip if not a .jsonl file
        if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }

        // Try to extract session metadata
        match extract_session_summary(path) {
            Ok(Some(summary)) => {
                // Apply project filter if specified
                if let Some(ref filter) = options.project_filter {
                    if !summary.project.contains(filter) {
                        continue;
                    }
                }

                // Filter out agent sessions unless --include-agents specified
                if !options.include_agents && is_agent_session(&summary.session_id) {
                    continue;
                }

                sessions.push(summary);
            }
            Ok(None) => {
                // Empty or malformed session, skip
                continue;
            }
            Err(_) => {
                // Failed to parse, skip
                continue;
            }
        }
    }

    // Sort by started timestamp (most recent first)
    sessions.sort_by(|a, b| b.started.cmp(&a.started));

    // Take limit
    sessions.truncate(options.limit);

    Ok(sessions)
}

/// Extract session summary from a .jsonl log file
fn extract_session_summary(log_path: &Path) -> Result<Option<SessionSummary>> {
    // Read all log entries
    let entries = stream::read_log_file(log_path)?;

    if entries.is_empty() {
        return Ok(None);
    }

    // Extract session_id from filename
    let session_id = log_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    // Find first user or assistant entry for metadata
    // (file-history-snapshot entries lack cwd/timestamp/branch)
    let first_message = entries.iter()
        .find(|e| e.entry_type == "user" || e.entry_type == "assistant");

    let last_message = entries.iter()
        .rev()
        .find(|e| e.entry_type == "user" || e.entry_type == "assistant");

    // Get metadata from first message entry
    let (project, branch, started) = if let Some(entry) = first_message {
        (
            entry.cwd.clone().unwrap_or_default(),
            entry.git_branch.clone(),
            entry.timestamp.clone().unwrap_or_default(),
        )
    } else {
        // No user/assistant messages, skip this session
        return Ok(None);
    };

    let ended = last_message
        .and_then(|e| e.timestamp.clone())
        .unwrap_or_default();

    // Calculate stats
    let stats = parser::calculate_stats(&entries);

    Ok(Some(SessionSummary {
        session_id,
        project,
        branch,
        started,
        ended,
        turn_count: stats.turn_count,
        tool_calls: stats.tool_calls,
    }))
}

/// Get default projects directory (~/.claude/projects)
pub fn default_projects_dir() -> PathBuf {
    dirs::home_dir()
        .expect("Could not determine home directory")
        .join(".claude")
        .join("projects")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_session(dir: &Path, session_id: &str, project: &str, branch: &str) -> Result<PathBuf> {
        let session_path = dir.join(format!("{}.jsonl", session_id));
        let mut file = fs::File::create(&session_path)?;

        // Write sample session entries
        writeln!(
            file,
            r#"{{"type":"user","timestamp":"2025-11-09T01:00:00Z","sessionId":"{}","cwd":"{}","gitBranch":"{}","message":{{"role":"user","content":[{{"type":"text","text":"test"}}]}}}}"#,
            session_id, project, branch
        )?;
        writeln!(
            file,
            r#"{{"type":"assistant","timestamp":"2025-11-09T01:01:00Z","sessionId":"{}","cwd":"{}","gitBranch":"{}","message":{{"role":"assistant","content":[{{"type":"text","text":"response"}}]}}}}"#,
            session_id, project, branch
        )?;

        Ok(session_path)
    }

    #[test]
    fn test_list_sessions_basic() -> Result<()> {
        let temp_dir = TempDir::new()?;

        create_test_session(temp_dir.path(), "session1", "/home/user/project1", "main")?;
        create_test_session(temp_dir.path(), "session2", "/home/user/project2", "develop")?;

        let options = ListSessionsOptions::default();
        let sessions = list_sessions(temp_dir.path(), &options)?;

        assert_eq!(sessions.len(), 2);
        assert!(sessions.iter().any(|s| s.session_id == "session1"));
        assert!(sessions.iter().any(|s| s.session_id == "session2"));

        Ok(())
    }

    #[test]
    fn test_list_sessions_with_filter() -> Result<()> {
        let temp_dir = TempDir::new()?;

        create_test_session(temp_dir.path(), "session1", "/home/user/project1", "main")?;
        create_test_session(temp_dir.path(), "session2", "/home/user/project2", "develop")?;

        let options = ListSessionsOptions {
            limit: 10,
            project_filter: Some("project1".to_string()),
        };
        let sessions = list_sessions(temp_dir.path(), &options)?;

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, "session1");

        Ok(())
    }

    #[test]
    fn test_list_sessions_with_limit() -> Result<()> {
        let temp_dir = TempDir::new()?;

        create_test_session(temp_dir.path(), "session1", "/home/user/project1", "main")?;
        create_test_session(temp_dir.path(), "session2", "/home/user/project2", "main")?;
        create_test_session(temp_dir.path(), "session3", "/home/user/project3", "main")?;

        let options = ListSessionsOptions {
            limit: 2,
            ..Default::default()
        };
        let sessions = list_sessions(temp_dir.path(), &options)?;

        assert_eq!(sessions.len(), 2);

        Ok(())
    }
}

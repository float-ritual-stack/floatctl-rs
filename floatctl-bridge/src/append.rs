/*!
 * Bridge Append - Real-time content capture
 * Active appending of conversation content to bridge files
 */

use anyhow::{bail, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{parse_annotations, AnnotationMetadata};

/// Options for appending to bridges
#[derive(Debug, Clone)]
pub struct AppendOptions {
    /// Minimum content length (default: 100)
    pub min_length: usize,
    /// Require both project and issue annotations (default: false)
    pub require_both: bool,
    /// Skip messages that look like commands (default: true)
    pub skip_commands: bool,
    /// Deduplication window in seconds (default: 60)
    pub dedup_window_secs: u64,
}

impl Default for AppendOptions {
    fn default() -> Self {
        Self {
            min_length: 100,
            require_both: false,
            skip_commands: true,
            dedup_window_secs: 60,
        }
    }
}

/// Result of append operation
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum AppendResult {
    Success {
        bridge_updated: String,
        project: String,
        issue: String,
        content_length: usize,
        timestamp: String,
    },
    Skipped {
        reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        content_length: Option<usize>,
        #[serde(skip_serializing_if = "Option::is_none")]
        min_length: Option<usize>,
    },
}

/// Append content to bridge file
pub fn append_to_bridge(
    content: &str,
    bridges_dir: &Path,
    options: &AppendOptions,
) -> Result<AppendResult> {
    // 1. Parse annotations
    let metadata = parse_annotations(content)?;

    // 2. Apply filters
    if let Some(skip_reason) = check_filters(&metadata, content, options) {
        return Ok(skip_reason);
    }

    // 3. Extract content (remove annotation lines)
    let clean_content = extract_content(content, &metadata);

    // Check if extracted content is too short
    if clean_content.len() < options.min_length {
        return Ok(AppendResult::Skipped {
            reason: "content_too_short_after_extraction".to_string(),
            content_length: Some(clean_content.len()),
            min_length: Some(options.min_length),
        });
    }

    // 4. Get bridge path
    let (bridge_path, bridge_filename, project, issue) =
        get_bridge_path(&metadata, bridges_dir)?;

    // 5. Check for duplicates
    if is_duplicate(&bridge_path, &clean_content, options.dedup_window_secs)? {
        return Ok(AppendResult::Skipped {
            reason: "duplicate".to_string(),
            content_length: None,
            min_length: None,
        });
    }

    // 6. Append to bridge
    append_section(&bridge_path, &metadata, &clean_content)?;

    let timestamp = Utc::now();
    Ok(AppendResult::Success {
        bridge_updated: bridge_filename,
        project,
        issue,
        content_length: clean_content.len(),
        timestamp: timestamp.to_rfc3339(),
    })
}

/// Check if content should be filtered out
fn check_filters(
    metadata: &AnnotationMetadata,
    content: &str,
    options: &AppendOptions,
) -> Option<AppendResult> {
    // Fuzzy compiler approach: If there are ANY :: annotations, consider it worth capturing
    let has_any_annotations = !metadata.annotations.is_empty();

    // Check for required annotations (if strict mode)
    if options.require_both {
        if metadata.project.is_none() {
            return Some(AppendResult::Skipped {
                reason: "missing_project".to_string(),
                content_length: None,
                min_length: None,
            });
        }
        if metadata.issue.is_none() {
            return Some(AppendResult::Skipped {
                reason: "missing_issue".to_string(),
                content_length: None,
                min_length: None,
            });
        }
    } else {
        // Relaxed mode: Accept if we have ANY annotations OR explicit identifiers
        if metadata.project.is_none()
            && metadata.issue.is_none()
            && !has_any_annotations
        {
            return Some(AppendResult::Skipped {
                reason: "missing_annotations".to_string(),
                content_length: None,
                min_length: None,
            });
        }
    }

    // Check content length (before extraction)
    if content.len() < options.min_length {
        return Some(AppendResult::Skipped {
            reason: "content_too_short".to_string(),
            content_length: Some(content.len()),
            min_length: Some(options.min_length),
        });
    }

    // Check if it looks like a command
    if options.skip_commands && looks_like_command(content) {
        return Some(AppendResult::Skipped {
            reason: "looks_like_command".to_string(),
            content_length: None,
            min_length: None,
        });
    }

    None
}

/// Detect if content looks like a command
fn looks_like_command(content: &str) -> bool {
    let trimmed = content.trim();

    // Starts with slash command
    if trimmed.starts_with('/') {
        return true;
    }

    // Simple command patterns like "fix X", "help with Y"
    let words: Vec<&str> = trimmed.split_whitespace().collect();
    if words.len() <= 3 {
        let first_word = words.first().map(|w| w.to_lowercase());
        if let Some(first) = first_word {
            if matches!(first.as_str(), "fix" | "help" | "update" | "change" | "add" | "remove") {
                return true;
            }
        }
    }

    false
}

/// Extract content after removing annotation lines
fn extract_content(raw: &str, _metadata: &AnnotationMetadata) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut skip_leading_empty = true;

    for line in raw.lines() {
        let trimmed = line.trim();

        // Skip empty lines at start
        if skip_leading_empty && trimmed.is_empty() {
            continue;
        }

        // Skip lines that are purely annotations
        if is_annotation_only(trimmed) {
            continue;
        }

        // Found real content
        skip_leading_empty = false;
        lines.push(line);
    }

    lines.join("\n").trim().to_string()
}

/// Check if line is annotation-only or metadata line
fn is_annotation_only(line: &str) -> bool {
    let annotation_count = line.matches("::").count();
    if annotation_count == 0 {
        return false;
    }

    // Always skip ctx:: lines (they're handled as metadata)
    if line.trim().starts_with("ctx::") {
        return true;
    }

    // Count words (excluding :: patterns)
    let word_count = line
        .split_whitespace()
        .filter(|w| !w.contains("::"))
        .count();

    // Skip if annotation-heavy and light on content
    annotation_count > 0 && word_count < 4
}

/// Get bridge path from metadata
///
/// Naming priority (date-first for chronological sorting):
/// 1. project + issue → YYYY-MM-DD-{project}-issue-{num}.md
/// 2. project + lf1m → YYYY-MM-DD-{project}-lf1m-{value}.md
/// 3. project + meeting → YYYY-MM-DD-{project}-meeting-{value}.md
/// 4. project + mode → YYYY-MM-DD-{project}-{mode}.md
/// 5. project only → YYYY-MM-DD-{project}.md
/// 6. annotation fallback → YYYY-MM-DD-{annotation}.md
fn get_bridge_path(
    metadata: &AnnotationMetadata,
    bridges_dir: &Path,
) -> Result<(PathBuf, String, String, String)> {
    use crate::slugify;

    // Date prefix for chronological sorting
    let date_prefix = Utc::now().format("%Y-%m-%d").to_string();

    // Try to find lf1m annotation from the full annotations list
    let lf1m = metadata.annotations.iter()
        .find(|a| a.annotation_type == "lf1m")
        .map(|a| a.value.clone());

    let (bridge_filename, _identifier_type, identifier_value) = match (&metadata.project, &metadata.issue, &lf1m, &metadata.meeting, &metadata.mode) {
        // Priority 1: project + issue
        (Some(project), Some(issue), _, _, _) => {
            let project_slug = slugify(project);
            let issue_number: String = issue.chars().filter(|c| c.is_numeric()).collect();
            if !issue_number.is_empty() {
                (
                    format!("{}-{}-issue-{}.md", date_prefix, project_slug, issue_number),
                    "issue".to_string(),
                    issue.clone(),
                )
            } else {
                // Issue exists but no number - just use project
                (
                    format!("{}-{}.md", date_prefix, project_slug),
                    "project".to_string(),
                    project.clone(),
                )
            }
        }
        // Priority 2: project + lf1m
        (Some(project), _, Some(lf1m_val), _, _) => {
            let project_slug = slugify(project);
            let lf1m_slug = slugify(lf1m_val);
            (
                format!("{}-{}-lf1m-{}.md", date_prefix, project_slug, lf1m_slug),
                "lf1m".to_string(),
                lf1m_val.clone(),
            )
        }
        // Priority 3: project + meeting
        (Some(project), None, None, Some(meeting), _) => {
            let project_slug = slugify(project);
            let meeting_slug = slugify(meeting);
            (
                format!("{}-{}-meeting-{}.md", date_prefix, project_slug, meeting_slug),
                "meeting".to_string(),
                meeting.clone(),
            )
        }
        // Priority 4: project + mode
        (Some(project), None, None, None, Some(mode)) => {
            let project_slug = slugify(project);
            let mode_slug = slugify(mode);
            (
                format!("{}-{}-{}.md", date_prefix, project_slug, mode_slug),
                "mode".to_string(),
                mode.clone(),
            )
        }
        // Priority 5: project only
        (Some(project), None, None, None, None) => {
            let project_slug = slugify(project);
            (
                format!("{}-{}.md", date_prefix, project_slug),
                "project".to_string(),
                project.clone(),
            )
        }
        // Priority 6: issue only (rare)
        (None, Some(issue), _, _, _) => {
            let issue_number: String = issue.chars().filter(|c| c.is_numeric()).collect();
            if !issue_number.is_empty() {
                (
                    format!("{}-issue-{}.md", date_prefix, issue_number),
                    "issue".to_string(),
                    issue.clone(),
                )
            } else {
                bail!("No valid project or issue annotation found");
            }
        }
        // Fallback: Create daily bridge from annotations
        _ => {
            // If we have ANY annotations, create a daily bridge
            if !metadata.annotations.is_empty() {
                let slug = metadata.annotations.first()
                    .map(|a| slugify(&a.annotation_type).to_string())
                    .unwrap_or_else(|| "capture".to_string());
                (
                    format!("{}-{}.md", date_prefix, slug),
                    "annotation".to_string(),
                    date_prefix.clone(),
                )
            } else {
                bail!("At least one annotation required (::)")
            }
        }
    };

    let bridge_path = bridges_dir.join(&bridge_filename);

    // Return project or empty string for compatibility
    let project = metadata.project.clone().unwrap_or_default();

    Ok((bridge_path, bridge_filename, project, identifier_value))
}

/// Check if content is duplicate of recent append
fn is_duplicate(bridge_path: &Path, content: &str, _window_secs: u64) -> Result<bool> {
    if !bridge_path.exists() {
        return Ok(false);
    }

    let existing_content = fs::read_to_string(bridge_path)?;

    // Simple hash-based deduplication
    let content_hash = format!("{:x}", md5::compute(content.trim()));

    // Look for this hash in the last section
    // TODO: More sophisticated time-window check
    Ok(existing_content.contains(&content_hash))
}

/// Append section to bridge file
fn append_section(
    bridge_path: &Path,
    metadata: &AnnotationMetadata,
    content: &str,
) -> Result<()> {
    let timestamp = Utc::now();
    let date_str = timestamp.format("%Y-%m-%d").to_string();
    let time_str = timestamp.format("%I:%M %p").to_string();
    let datetime_str = format!("{} @ {}", date_str, time_str);

    // Build section
    let mut section = format!("\n## Update: {}\n\n", datetime_str);

    // Add ctx if present
    if let Some(ctx) = &metadata.ctx {
        section.push_str(&format!("ctx::{}\n\n", ctx));
    }

    section.push_str(content);
    section.push('\n');

    // Ensure bridges directory exists
    if let Some(parent) = bridge_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if bridge_path.exists() {
        // Append to existing bridge
        let mut existing = fs::read_to_string(bridge_path)?;
        existing.push_str(&section);
        fs::write(bridge_path, existing)?;
    } else {
        // Create new bridge with flexible frontmatter
        let mut frontmatter_fields = vec![
            format!("type: work_log"),
            format!("created: {}", timestamp.to_rfc3339()),
        ];

        // Add whatever identifiers we have
        if let Some(project) = &metadata.project {
            frontmatter_fields.push(format!("project: {}", project));
        }
        if let Some(issue) = &metadata.issue {
            frontmatter_fields.push(format!("issue: {}", issue));
        }
        if let Some(mode) = &metadata.mode {
            frontmatter_fields.push(format!("mode: {}", mode));
        }
        if let Some(meeting) = &metadata.meeting {
            frontmatter_fields.push(format!("meeting: {}", meeting));
        }

        // Check for lf1m annotation
        if let Some(lf1m_ann) = metadata.annotations.iter().find(|a| a.annotation_type == "lf1m") {
            frontmatter_fields.push(format!("lf1m: {}", lf1m_ann.value));
        }

        let frontmatter = format!("---\n{}\n---\n", frontmatter_fields.join("\n"));

        // Generate title based on what we have
        let title = if let Some(project) = &metadata.project {
            if let Some(issue) = &metadata.issue {
                let issue_number: String = issue.chars().filter(|c| c.is_numeric()).collect();
                if !issue_number.is_empty() {
                    format!("# {} - Issue #{}\n", project, issue_number)
                } else {
                    format!("# {}\n", project)
                }
            } else {
                format!("# {}\n", project)
            }
        } else if let Some(issue) = &metadata.issue {
            let issue_number: String = issue.chars().filter(|c| c.is_numeric()).collect();
            format!("# Issue #{}\n", issue_number)
        } else {
            "# Work Log\n".to_string()
        };

        let new_bridge = format!("{}{}{}", frontmatter, title, section);
        fs::write(bridge_path, new_bridge)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_looks_like_command() {
        assert!(looks_like_command("/clear"));
        assert!(looks_like_command("fix this"));
        assert!(looks_like_command("help me"));
        assert!(!looks_like_command("Working on the switch node bug. Found race condition."));
    }

    #[test]
    fn test_is_annotation_only() {
        assert!(is_annotation_only("ctx::2025-10-31"));
        assert!(is_annotation_only("project::foo issue::1"));
        assert!(!is_annotation_only("Working on issue::633 with lots of content"));
    }

    #[test]
    fn test_extract_content() {
        let raw = r#"
ctx::2025-10-31 @ 09:45 PM - [project::rangle/pharmacy] - [issue::633]

Working on the switch node bug. Found that the zustand state was racing.
"#;

        let metadata = parse_annotations(raw).unwrap();
        let content = extract_content(raw, &metadata);

        assert!(!content.contains("ctx::"));
        assert!(content.contains("Working on the switch node"));
    }

    #[test]
    fn test_append_to_bridge_success() {
        let temp_dir = TempDir::new().unwrap();
        let bridges_dir = temp_dir.path();

        let content = r#"
ctx::2025-10-31 @ 09:45 PM - [project::rangle/pharmacy] - [issue::633]

Working on the switch node bug in pharmacy. Found that the zustand state
update was racing with the React render cycle. Added useMemo to fix the issue.
"#;

        let options = AppendOptions::default();
        let result = append_to_bridge(content, bridges_dir, &options).unwrap();

        match result {
            AppendResult::Success { bridge_updated, .. } => {
                // Bridge filenames now include date prefix for chronological sorting
                assert!(bridge_updated.ends_with("-rangle-pharmacy-issue-633.md"),
                    "Expected filename ending with '-rangle-pharmacy-issue-633.md', got: {}", bridge_updated);
            }
            _ => panic!("Expected success, got: {:?}", result),
        }
    }

    #[test]
    fn test_append_to_bridge_skip_short() {
        let temp_dir = TempDir::new().unwrap();
        let content = "project::test issue::1 short";
        let options = AppendOptions::default();

        let result = append_to_bridge(content, temp_dir.path(), &options).unwrap();

        match result {
            AppendResult::Skipped { reason, .. } => {
                assert_eq!(reason, "content_too_short");
            }
            _ => panic!("Expected skipped, got: {:?}", result),
        }
    }
}

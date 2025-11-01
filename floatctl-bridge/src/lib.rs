/*!
 * Bridge Maintenance
 * Passive indexing of :: annotations to create bridge stubs
 * Active appending of conversation content to bridges
 */

pub mod append;

use anyhow::{Context, Result};
use chrono::Utc;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

/// Parsed annotation from :: markers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub annotation_type: String,
    pub value: String,
    pub line_number: usize,
}

/// Metadata extracted from annotations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationMetadata {
    pub project: Option<String>,
    pub issue: Option<String>,
    pub ctx: Option<String>,
    pub mode: Option<String>,
    pub meeting: Option<String>,
    pub annotations: Vec<Annotation>,
}

/// Bridge indexing result
#[derive(Debug, Serialize, Deserialize)]
pub struct IndexResult {
    pub bridges_created: Vec<String>,
    pub bridges_updated: Vec<String>,
    pub references_added: usize,
}

/// Parse :: annotations from markdown content
pub fn parse_annotations(content: &str) -> Result<AnnotationMetadata> {
    // Regex patterns:
    // 1. Single-token annotations: word::token (e.g., project::float/evna, issue::123)
    // 2. Full-line patterns like ctx:: need special handling
    let single_token_regex = Regex::new(r"(\w+)::(\S+)")?;
    let ctx_regex = Regex::new(r"ctx::\s*(.+?)$")?;

    let mut annotations = Vec::new();
    let mut project = None;
    let mut issue = None;
    let mut ctx = None;
    let mut mode = None;
    let mut meeting = None;

    for (line_num, line) in content.lines().enumerate() {
        // Special handling for ctx:: (captures full line)
        if let Some(cap) = ctx_regex.captures(line) {
            let value = cap[1].trim().to_string();
            ctx = Some(value.clone());

            // Parse [project::X] and [issue::Y] from ctx:: value using string operations
            if let Some(start) = value.find("[project::") {
                let after_prefix = &value[start + "[project::".len()..];
                if let Some(end) = after_prefix.find(']') {
                    if project.is_none() {
                        project = Some(after_prefix[..end].trim().to_string());
                    }
                }
            }

            if let Some(start) = value.find("[issue::") {
                let after_prefix = &value[start + "[issue::".len()..];
                if let Some(end) = after_prefix.find(']') {
                    if issue.is_none() {
                        let issue_val = after_prefix[..end].replace('#', "").trim().to_string();
                        issue = Some(issue_val);
                    }
                }
            }

            annotations.push(Annotation {
                annotation_type: "ctx".to_string(),
                value,
                line_number: line_num + 1,
            });

            // Skip single-token processing for lines with ctx::
            continue;
        }

        // Find all single-token annotations in this line
        for cap in single_token_regex.captures_iter(line) {
            let annotation_type = cap[1].to_string();
            let value = cap[2].trim().to_string();

            // Skip ctx:: as it's handled separately above
            if annotation_type == "ctx" {
                continue;
            }

            // Extract key metadata
            match annotation_type.as_str() {
                "project" => {
                    project = Some(value.clone());
                }
                "issue" => {
                    issue = Some(value.replace('#', "").trim().to_string());
                }
                "mode" => {
                    mode = Some(value.clone());
                }
                "meeting" => {
                    meeting = Some(value.clone());
                }
                _ => {}
            }

            annotations.push(Annotation {
                annotation_type,
                value,
                line_number: line_num + 1,
            });
        }
    }

    Ok(AnnotationMetadata {
        project,
        issue,
        ctx,
        mode,
        meeting,
        annotations,
    })
}

/// Slugify text for filenames
pub fn slugify(text: &str) -> String {
    text.to_lowercase()
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

/// Index a single file's annotations into bridge stubs
pub fn index_file(file_path: &Path, bridges_dir: &Path) -> Result<IndexResult> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let metadata = parse_annotations(&content)?;

    let mut bridges_created = Vec::new();
    let mut bridges_updated = Vec::new();
    let mut references_added = 0;

    // Only create bridge if we have project + issue
    if let (Some(project), Some(issue)) = (&metadata.project, &metadata.issue) {
        let project_slug = slugify(project);
        let issue_number = issue.chars().filter(|c| c.is_numeric()).collect::<String>();

        if !issue_number.is_empty() {
            let bridge_filename = format!("{}-issue-{}.md", project_slug, issue_number);
            let bridge_path = bridges_dir.join(&bridge_filename);

            // Ensure bridges directory exists
            fs::create_dir_all(bridges_dir)?;

            let timestamp = Utc::now();
            let date_str = timestamp.format("%Y-%m-%d").to_string();
            let time_str = timestamp.format("%I:%M %p").to_string();
            let datetime_str = format!("{} @ {}", date_str, time_str);

            // Create reference entry
            let reference_section = format!(
                "\n## Reference: {}\n\n**Indexed**: {}\n\nSee: `{}`\n",
                datetime_str,
                timestamp.to_rfc3339(),
                file_path.display()
            );

            if bridge_path.exists() {
                // Append to existing bridge
                let mut existing = fs::read_to_string(&bridge_path)?;
                existing.push_str(&reference_section);
                fs::write(&bridge_path, existing)?;
                bridges_updated.push(bridge_filename);
            } else {
                // Create new bridge stub
                let frontmatter = format!(
                    "---\ntype: auto_indexed\nproject: {}\nissue: {}\nindexed: {}\n---\n",
                    project, issue, timestamp.to_rfc3339()
                );

                let title = format!("# {} - Issue #{}\n", project, issue_number);
                let intro = "\n## Auto-Indexed References\n\nThis bridge was automatically created by indexing :: annotations.\n";

                let new_bridge = format!("{}{}{}{}", frontmatter, title, intro, reference_section);
                fs::write(&bridge_path, new_bridge)?;
                bridges_created.push(bridge_filename);
            }

            references_added += 1;
        }
    }

    Ok(IndexResult {
        bridges_created,
        bridges_updated,
        references_added,
    })
}

/// Index all markdown files in a directory
pub fn index_directory(dir_path: &Path, bridges_dir: &Path, recursive: bool) -> Result<IndexResult> {
    let mut combined_result = IndexResult {
        bridges_created: Vec::new(),
        bridges_updated: Vec::new(),
        references_added: 0,
    };

    if recursive {
        let entries = walkdir::WalkDir::new(dir_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"));

        for entry in entries {
            match index_file(entry.path(), bridges_dir) {
                Ok(result) => {
                    combined_result.bridges_created.extend(result.bridges_created);
                    combined_result.bridges_updated.extend(result.bridges_updated);
                    combined_result.references_added += result.references_added;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to index {}: {}", entry.path().display(), e);
                }
            }
        }
    } else {
        let entries = fs::read_dir(dir_path)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path().extension().and_then(|s| s.to_str()) == Some("md")
                    && e.path().is_file()
            });

        for entry in entries {
            match index_file(&entry.path(), bridges_dir) {
                Ok(result) => {
                    combined_result.bridges_created.extend(result.bridges_created);
                    combined_result.bridges_updated.extend(result.bridges_updated);
                    combined_result.references_added += result.references_added;
                }
                Err(e) => {
                    eprintln!("Warning: Failed to index {}: {}", entry.path().display(), e);
                }
            }
        }
    }

    Ok(combined_result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_annotations_with_project_and_issue() {
        let content = r#"
ctx::2025-10-31 @ 05:12:10 PM - [project::rangle/pharmacy] - [issue::656]
About to start issue implementation
"#;

        let metadata = parse_annotations(content).unwrap();
        assert_eq!(metadata.project.as_deref(), Some("rangle/pharmacy"));
        assert_eq!(metadata.issue.as_deref(), Some("656"));
    }

    #[test]
    fn test_parse_annotations_standalone() {
        let content = "project::float/evna issue::123 mode::feature-dev";

        let metadata = parse_annotations(content).unwrap();
        assert_eq!(metadata.project.as_deref(), Some("float/evna"));
        assert_eq!(metadata.issue.as_deref(), Some("123"));
        assert_eq!(metadata.mode.as_deref(), Some("feature-dev"));
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("rangle/pharmacy"), "rangle-pharmacy");
        assert_eq!(slugify("Float Hub Operations"), "float-hub-operations");
        assert_eq!(slugify("test@#$%123"), "test-123");
    }
}

//! Script management for floatctl
//!
//! This crate provides script registration, listing, and execution with doc block parsing.

use anyhow::{anyhow, Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Parsed documentation from script header comments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptDoc {
    /// One-line description of what the script does
    pub description: Option<String>,
    /// Usage string (e.g., "script-name <arg1> <arg2>")
    pub usage: Option<String>,
    /// List of arguments with descriptions
    pub args: Vec<ScriptArg>,
    /// Example usage string
    pub example: Option<String>,
}

/// Script argument documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptArg {
    pub name: String,
    pub description: Option<String>,
}

/// Script metadata for list output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptInfo {
    pub name: String,
    pub size: u64,
    pub path: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub doc: Option<ScriptDoc>,
}

/// Parse doc block from script file
///
/// Looks for structured comments after shebang:
/// ```bash
/// #!/bin/bash
/// # Description: One-line summary
/// # Usage: script-name <arg1>
/// # Args:
/// #   arg1 - Description
/// # Example:
/// #   script-name foo
/// ```
pub fn parse_doc_block(script_path: &Path) -> Result<ScriptDoc> {
    let content = fs::read_to_string(script_path)
        .with_context(|| format!("Failed to read script: {}", script_path.display()))?;

    let lines: Vec<&str> = content.lines().collect();

    // Skip shebang if present
    let start_idx = if lines.first().map(|l| l.starts_with("#!")).unwrap_or(false) {
        1
    } else {
        0
    };

    // Only look at first 50 lines after shebang for doc block
    let doc_lines: Vec<&str> = lines
        .iter()
        .skip(start_idx)
        .take(50)
        .copied()
        .take_while(|line| line.trim().is_empty() || line.trim().starts_with('#'))
        .filter(|line| line.trim().starts_with('#') && !line.trim().starts_with("#!"))
        .collect();

    let mut description = None;
    let mut usage = None;
    let mut args = Vec::new();
    let mut example = None;
    let mut in_args_section = false;
    let mut in_example_section = false;

    // Regex patterns
    let desc_re = Regex::new(r"^#\s*(?:Description:|DESC:)?\s*(.+)$").unwrap();
    let usage_re = Regex::new(r"^#\s*Usage:\s*(.+)$").unwrap();
    let args_header_re = Regex::new(r"^#\s*Args:?\s*$").unwrap();
    let arg_re = Regex::new(r"^#\s+(\w+)\s*-\s*(.+)$").unwrap();
    let example_header_re = Regex::new(r"^#\s*Examples?:?\s*$").unwrap();
    let example_re = Regex::new(r"^#\s+(.+)$").unwrap();

    for line in doc_lines {
        let trimmed = line.trim();

        // Check for section headers
        if args_header_re.is_match(trimmed) {
            in_args_section = true;
            in_example_section = false;
            continue;
        }
        if example_header_re.is_match(trimmed) {
            in_example_section = true;
            in_args_section = false;
            continue;
        }

        // Parse based on current section
        if in_args_section {
            if let Some(caps) = arg_re.captures(trimmed) {
                args.push(ScriptArg {
                    name: caps[1].to_string(),
                    description: Some(caps[2].to_string()),
                });
            } else if !trimmed.starts_with("#") || trimmed.len() <= 1 {
                // End of args section
                in_args_section = false;
            }
        } else if in_example_section {
            if let Some(caps) = example_re.captures(trimmed) {
                let ex = caps[1].to_string();
                if !ex.is_empty() {
                    example = Some(ex);
                }
                // Only take first example line
                in_example_section = false;
            }
        } else {
            // Try to match description or usage
            if description.is_none() {
                if let Some(caps) = desc_re.captures(trimmed) {
                    let desc = caps[1].trim().to_string();
                    if !desc.is_empty() && !desc.starts_with("V") && !desc.starts_with("!") {
                        description = Some(desc);
                        continue;
                    }
                }
            }
            if usage.is_none() {
                if let Some(caps) = usage_re.captures(trimmed) {
                    usage = Some(caps[1].to_string());
                }
            }
        }
    }

    Ok(ScriptDoc {
        description,
        usage,
        args,
        example,
    })
}

/// Get scripts directory (~/.floatctl/scripts)
pub fn get_scripts_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let scripts_dir = home.join(".floatctl").join("scripts");

    // Create if doesn't exist
    if !scripts_dir.exists() {
        fs::create_dir_all(&scripts_dir)
            .with_context(|| format!("Failed to create {}", scripts_dir.display()))?;
    }

    Ok(scripts_dir)
}

/// List all registered scripts with optional doc parsing
pub fn list_scripts(parse_docs: bool) -> Result<Vec<ScriptInfo>> {
    let scripts_dir = get_scripts_dir()?;

    let mut scripts = Vec::new();

    for entry in fs::read_dir(&scripts_dir)
        .context("Failed to read scripts directory")?
    {
        let entry = entry?;
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        let name = entry.file_name().to_string_lossy().to_string();
        let metadata = entry.metadata()?;
        let size = metadata.len();

        let doc = if parse_docs {
            parse_doc_block(&path).ok()
        } else {
            None
        };

        scripts.push(ScriptInfo {
            name,
            size,
            path,
            doc,
        });
    }

    // Sort by name
    scripts.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(scripts)
}

/// Show (cat) a script to stdout
pub fn show_script(script_name: &str) -> Result<String> {
    let scripts_dir = get_scripts_dir()?;
    let script_path = scripts_dir.join(script_name);

    if !script_path.exists() {
        return Err(anyhow!(
            "Script '{}' not found. List scripts with: floatctl script list",
            script_name
        ));
    }

    fs::read_to_string(&script_path)
        .with_context(|| format!("Failed to read script: {}", script_path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_parse_doc_block_full() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let mut file = fs::File::create(&script_path).unwrap();
        file.write_all(
            b"#!/bin/bash
# Description: Split a file into chunks
# Usage: split-to-md <input_file> <size>
# Args:
#   input_file - File to split
#   size - Chunk size (100 lines, 10k bytes)
# Example:
#   split-to-md large.txt 100

echo 'script body'
",
        )
        .unwrap();

        let doc = parse_doc_block(&script_path).unwrap();

        assert_eq!(
            doc.description,
            Some("Split a file into chunks".to_string())
        );
        assert_eq!(
            doc.usage,
            Some("split-to-md <input_file> <size>".to_string())
        );
        assert_eq!(doc.args.len(), 2);
        assert_eq!(doc.args[0].name, "input_file");
        assert_eq!(doc.args[0].description, Some("File to split".to_string()));
        assert_eq!(
            doc.example,
            Some("split-to-md large.txt 100".to_string())
        );
    }

    #[test]
    fn test_parse_doc_block_minimal() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let mut file = fs::File::create(&script_path).unwrap();
        file.write_all(
            b"#!/bin/bash
# Generate metadata using Ollama

echo 'script body'
",
        )
        .unwrap();

        let doc = parse_doc_block(&script_path).unwrap();

        assert_eq!(
            doc.description,
            Some("Generate metadata using Ollama".to_string())
        );
        assert_eq!(doc.usage, None);
        assert_eq!(doc.args.len(), 0);
        assert_eq!(doc.example, None);
    }

    #[test]
    fn test_parse_doc_block_no_shebang() {
        let temp_dir = TempDir::new().unwrap();
        let script_path = temp_dir.path().join("test.sh");

        let mut file = fs::File::create(&script_path).unwrap();
        file.write_all(
            b"# Description: Test script
# Usage: test.sh

echo 'no shebang'
",
        )
        .unwrap();

        let doc = parse_doc_block(&script_path).unwrap();

        assert_eq!(doc.description, Some("Test script".to_string()));
        assert_eq!(doc.usage, Some("test.sh".to_string()));
    }
}

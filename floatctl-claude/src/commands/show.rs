/*!
 * Show command - Pretty-print a Claude Code session
 */

use crate::{parser, stream, ContentBlock};
use anyhow::{Context, Result};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::path::Path;

/// Output format for show command
#[derive(Debug, Clone, PartialEq)]
pub enum OutputFormat {
    Text,
    Markdown,
    Json,
}

/// Options for showing a session
#[derive(Debug, Clone)]
pub struct ShowOptions {
    pub with_thinking: bool,
    pub with_tools: bool,
    pub format: OutputFormat,
}

impl Default for ShowOptions {
    fn default() -> Self {
        Self {
            with_thinking: false,
            with_tools: true,
            format: OutputFormat::Text,
        }
    }
}

/// Decode base64 data and return the actual byte length
/// Returns None if decoding fails
fn get_decoded_image_size(base64_data: &str) -> Option<usize> {
    STANDARD.decode(base64_data).ok().map(|decoded| decoded.len())
}

/// Pretty-print a session log file
pub fn show(log_path: &Path, options: &ShowOptions) -> Result<()> {
    // Read all log entries
    let entries = stream::read_log_file(log_path)
        .with_context(|| format!("Failed to read log file: {}", log_path.display()))?;

    if entries.is_empty() {
        println!("(empty session)");
        return Ok(());
    }

    // Dispatch based on format
    match options.format {
        OutputFormat::Text => show_text(&entries, options),
        OutputFormat::Markdown => show_markdown(&entries, options),
        OutputFormat::Json => show_json(&entries, options),
    }
}

/// Show session in text format (current format)
fn show_text(entries: &[crate::LogEntry], options: &ShowOptions) -> Result<()> {
    // Get session metadata
    let metadata = parser::get_session_metadata(entries)
        .context("Failed to extract session metadata")?;

    // Print header
    println!("╭─────────────────────────────────────────────");
    println!("│ Session: {}", metadata.session_id);
    println!("│ Project: {}", metadata.project);
    if let Some(ref branch) = metadata.branch {
        println!("│ Branch:  {}", branch);
    }
    println!("│ Started: {}", metadata.started);
    println!("│ Ended:   {}", metadata.ended);
    println!("╰─────────────────────────────────────────────\n");

    // Track stats
    let mut turn_count = 0;
    let mut tool_count = 0;

    // Print messages
    for entry in entries {
        // Skip non-message entries
        if entry.entry_type != "user" && entry.entry_type != "assistant" {
            continue;
        }

        let Some(ref message) = entry.message else {
            continue;
        };

        turn_count += 1;

        // Format timestamp
        let timestamp = entry.timestamp
            .as_ref()
            .and_then(|ts| ts.split('T').nth(1))
            .and_then(|t| t.split('.').next())
            .unwrap_or("--:--:--");

        // Print role header with color
        match message.role.as_str() {
            "user" => println!("\n┌─ 👤 User ({}) ────────", timestamp),
            "assistant" => println!("\n┌─ 🤖 Assistant ({}) ──", timestamp),
            _ => println!("\n┌─ {} ({}) ───", message.role, timestamp),
        }

        // Print content blocks
        for block in &message.content {
            match block {
                ContentBlock::Text { text } => {
                    for line in text.lines() {
                        println!("│ {}", line);
                    }
                }
                ContentBlock::Thinking { thinking } => {
                    if options.with_thinking {
                        println!("│");
                        println!("│ 💭 Thinking:");
                        for line in thinking.lines().take(5) {
                            println!("│   {}", line);
                        }
                        if thinking.lines().count() > 5 {
                            println!("│   ... ({} more lines)", thinking.lines().count() - 5);
                        }
                    }
                }
                ContentBlock::ToolUse { id, name, input } => {
                    tool_count += 1;
                    if options.with_tools {
                        println!("│");
                        println!("│ 🔧 Tool: {}", name);
                        println!("│   ID: {}", id);
                        let input_str = serde_json::to_string_pretty(&input).unwrap_or_default();
                        for line in input_str.lines().take(10) {
                            println!("│   {}", line);
                        }
                        if input_str.lines().count() > 10 {
                            println!("│   ... ({} more lines)", input_str.lines().count() - 10);
                        }
                    }
                }
                ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                    if options.with_tools {
                        println!("│");
                        println!("│ {} Tool result ({})",
                            if *is_error { "❌" } else { "✅" },
                            tool_use_id
                        );
                        // Extract text from nested content blocks
                        let text = crate::extract_text_from_blocks(&content);
                        let lines: Vec<&str> = text.lines().collect();
                        for line in lines.iter().take(10) {
                            println!("│   {}", line);
                        }
                        if lines.len() > 10 {
                            println!("│   ... ({} more lines)", lines.len() - 10);
                        }
                    }
                }
                ContentBlock::Image { source } => {
                    println!("│");
                    let size_str = match get_decoded_image_size(&source.data) {
                        Some(size) => format!("{} bytes", size),
                        None => "unknown size".to_string(),
                    };
                    println!("│ 🖼️  Image: {} ({})",
                        source.media_type,
                        size_str
                    );
                }
            }
        }

        // Print usage if available
        if let Some(ref usage) = message.usage {
            println!("│");
            println!("│ 📊 Tokens: in={} out={} (cache: creation={} read={})",
                usage.input_tokens,
                usage.output_tokens,
                usage.cache_creation_input_tokens,
                usage.cache_read_input_tokens
            );
        }

        println!("└────────────────────────────────────────────");
    }

    // Calculate and print final stats
    let stats = parser::calculate_stats(&entries);

    println!("\n╭─ Summary ──────────────────────────────────");
    println!("│ Turns: {}", turn_count);
    println!("│ Tool calls: {}", tool_count);
    if let Some(input) = stats.total_input_tokens {
        println!("│ Total input tokens: {}", input);
    }
    if let Some(output) = stats.total_output_tokens {
        println!("│ Total output tokens: {}", output);
    }
    if let Some(cache_read) = stats.cache_read_tokens {
        let cache_created = stats.cache_creation_tokens.unwrap_or(0);
        if cache_created > 0 {
            let efficiency = (cache_read as f64 / (cache_read + cache_created) as f64) * 100.0;
            println!("│ Cache efficiency: {:.1}% ({} read / {} created)",
                efficiency, cache_read, cache_created);
        }
    }
    println!("╰────────────────────────────────────────────");

    Ok(())
}

/// Show session in markdown format (glow-friendly)
fn show_markdown(entries: &[crate::LogEntry], options: &ShowOptions) -> Result<()> {
    // Get session metadata
    let metadata = parser::get_session_metadata(entries)
        .context("Failed to extract session metadata")?;

    // Print header
    println!("# Session: {}\n", metadata.session_id);
    println!("**Project:** {}", metadata.project);
    if let Some(ref branch) = metadata.branch {
        println!("**Branch:** {}", branch);
    }
    println!("**Started:** {}", metadata.started);
    println!("**Ended:** {}", metadata.ended);
    println!();

    // Print messages
    for entry in entries {
        // Skip non-message entries
        if entry.entry_type != "user" && entry.entry_type != "assistant" {
            continue;
        }

        let Some(ref message) = entry.message else {
            continue;
        };

        // Format timestamp
        let timestamp = entry.timestamp
            .as_ref()
            .and_then(|ts| ts.split('T').nth(1))
            .and_then(|t| t.split('.').next())
            .unwrap_or("--:--:--");

        // Print role header
        match message.role.as_str() {
            "user" => println!("## 👤 User ({})\n", timestamp),
            "assistant" => println!("## 🤖 Assistant ({})\n", timestamp),
            _ => println!("## {} ({})\n", message.role, timestamp),
        }

        // Print content blocks
        for block in &message.content {
            match block {
                ContentBlock::Text { text } => {
                    println!("{}\n", text);
                }
                ContentBlock::Thinking { thinking } => {
                    if options.with_thinking {
                        println!("> [!NOTE] **Thinking**");
                        for line in thinking.lines() {
                            println!("> {}", line);
                        }
                        println!();
                    }
                }
                ContentBlock::ToolUse { id, name, input } => {
                    if options.with_tools {
                        println!("> [!TIP] **Tool:** `{}`  ", name);
                        println!("> **ID:** `{}`\n", id);
                        println!("```json");
                        println!("{}", serde_json::to_string_pretty(&input).unwrap_or_default());
                        println!("```\n");
                    }
                }
                ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                    if options.with_tools {
                        println!("> [!{}] **Tool Result** (`{}`)\n",
                            if *is_error { "CAUTION" } else { "TIP" },
                            tool_use_id
                        );
                        let text = crate::extract_text_from_blocks(&content);
                        // Detect if content looks like code/json
                        if text.trim().starts_with('{') || text.trim().starts_with('[') {
                            println!("```");
                            println!("{}", text);
                            println!("```\n");
                        } else {
                            println!("{}\n", text);
                        }
                    }
                }
                ContentBlock::Image { source } => {
                    let size_str = match get_decoded_image_size(&source.data) {
                        Some(size) => format!("{} bytes", size),
                        None => "unknown size".to_string(),
                    };
                    println!("> [!NOTE] **Image:** {} ({})\n",
                        source.media_type,
                        size_str
                    );
                }
            }
        }

        // Print usage if available
        if let Some(ref usage) = message.usage {
            println!("---");
            println!("**Tokens:** in={} out={} | cache: creation={} read={}",
                usage.input_tokens,
                usage.output_tokens,
                usage.cache_creation_input_tokens,
                usage.cache_read_input_tokens
            );
            println!();
        }
    }

    // Print summary
    let stats = parser::calculate_stats(entries);
    println!("---\n");
    println!("## Summary\n");
    println!("- **Turns:** {}", stats.turn_count);
    println!("- **Tool calls:** {}", stats.tool_calls);
    if let Some(input) = stats.total_input_tokens {
        println!("- **Total input tokens:** {}", input);
    }
    if let Some(output) = stats.total_output_tokens {
        println!("- **Total output tokens:** {}", output);
    }
    if let Some(cache_read) = stats.cache_read_tokens {
        let cache_created = stats.cache_creation_tokens.unwrap_or(0);
        if cache_created > 0 {
            let efficiency = (cache_read as f64 / (cache_read + cache_created) as f64) * 100.0;
            println!("- **Cache efficiency:** {:.1}% ({} read / {} created)",
                efficiency, cache_read, cache_created);
        }
    }

    Ok(())
}

/// Show session in JSON format
fn show_json(entries: &[crate::LogEntry], _options: &ShowOptions) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(entries)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_decoded_image_size() {
        // Test valid base64 data
        // "Hello, World!" in base64 is "SGVsbG8sIFdvcmxkIQ=="
        let base64_data = "SGVsbG8sIFdvcmxkIQ==";
        let size = get_decoded_image_size(base64_data);
        assert_eq!(size, Some(13)); // "Hello, World!" is 13 bytes

        // Test invalid base64 data
        let invalid_data = "not valid base64!!!";
        let size = get_decoded_image_size(invalid_data);
        assert_eq!(size, None);

        // Test empty string
        let empty_data = "";
        let size = get_decoded_image_size(empty_data);
        assert_eq!(size, Some(0));

        // Test typical PNG image header (first few bytes of a PNG in base64)
        // This represents 8 bytes: 0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A
        let png_header = "iVBORw0KGgo=";
        let size = get_decoded_image_size(png_header);
        assert_eq!(size, Some(8));
    }
}

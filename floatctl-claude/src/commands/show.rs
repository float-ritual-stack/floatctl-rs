/*!
 * Show command - Pretty-print a Claude Code session
 */

use crate::{parser, stream, ContentBlock};
use anyhow::{Context, Result};
use std::path::Path;

/// Options for showing a session
#[derive(Debug, Clone)]
pub struct ShowOptions {
    pub with_thinking: bool,
    pub with_tools: bool,
}

impl Default for ShowOptions {
    fn default() -> Self {
        Self {
            with_thinking: false,
            with_tools: true,
        }
    }
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

    // Get session metadata
    let metadata = parser::get_session_metadata(&entries)
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
    for entry in &entries {
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
            .split('T')
            .nth(1)
            .and_then(|t| t.split('.').next())
            .unwrap_or(&entry.timestamp);

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
                        let input_str = serde_json::to_string_pretty(input).unwrap_or_default();
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
                        for line in content.lines().take(10) {
                            println!("│   {}", line);
                        }
                        if content.lines().count() > 10 {
                            println!("│   ... ({} more lines)", content.lines().count() - 10);
                        }
                    }
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

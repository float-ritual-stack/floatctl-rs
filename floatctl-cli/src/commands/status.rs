//! Status command - Manage system-wide status broadcast
//!
//! Status is displayed in evna-remote MCP tool descriptions as ambient awareness.
//! This provides the CLI interface to set/clear/show status.
//!
//! Status files: ~/.floatctl/status/{focus,notice}.json
//! Format: { "content": "...", "set_at": "ISO8601", "set_by": "..." }

use anyhow::{Context, Result};
use chrono::{DateTime, Local, Utc};
use chrono_tz::America::Toronto;
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, BufRead, IsTerminal};
use std::path::PathBuf;

/// Status entry with timestamp metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct StatusEntry {
    pub content: String,
    pub set_at: String,      // ISO 8601 timestamp (Toronto time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_by: Option<String>,
}

impl StatusEntry {
    fn new(content: String, set_by: Option<String>) -> Self {
        // Use Toronto time
        let now = Utc::now().with_timezone(&Toronto);
        Self {
            content,
            set_at: now.to_rfc3339(),
            set_by,
        }
    }
}

#[derive(Parser, Debug)]
#[command(about = "Manage system-wide status broadcast (displayed in evna tool descriptions)")]
pub struct StatusArgs {
    #[command(subcommand)]
    pub command: StatusCommand,
}

#[derive(Subcommand, Debug)]
pub enum StatusCommand {
    /// Set work focus (shown to other agents via [FOCUS] tag)
    ///
    /// Message can be provided as argument or piped via stdin:
    ///   floatctl status focus "my focus"
    ///   echo "my focus" | floatctl status focus
    ///   floatctl status focus << 'EOF'
    ///   multiline focus
    ///   EOF
    Focus {
        /// The focus message (reads from stdin if not provided)
        message: Option<String>,
        /// Who is setting this (e.g., "kitty", "evan")
        #[arg(long, short = 'b')]
        set_by: Option<String>,
        /// Suppress progress spinners and bars (for LLM/script consumption)
        #[arg(long, short = 'q')]
        quiet: bool,
    },
    /// Set sysop notice (break warnings, meeting status, etc.)
    ///
    /// Message can be provided as argument or piped via stdin:
    ///   floatctl status notice "my notice"
    ///   echo "my notice" | floatctl status notice
    Notice {
        /// The notice message (reads from stdin if not provided)
        message: Option<String>,
        /// Who is setting this (e.g., "kitty", "evan")
        #[arg(long, short = 'b')]
        set_by: Option<String>,
        /// Suppress progress spinners and bars (for LLM/script consumption)
        #[arg(long, short = 'q')]
        quiet: bool,
    },
    /// Clear status entries
    Clear {
        /// What to clear: focus, notice, or all
        #[arg(value_enum)]
        target: ClearTarget,
    },
    /// Show current status
    Show {
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(clap::ValueEnum, Debug, Clone, Copy)]
pub enum ClearTarget {
    Focus,
    Notice,
    All,
}

fn get_status_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let status_dir = home.join(".floatctl").join("status");

    // Create if doesn't exist
    if !status_dir.exists() {
        fs::create_dir_all(&status_dir)
            .context(format!("Failed to create {}", status_dir.display()))?;
    }

    Ok(status_dir)
}

fn write_status_entry(name: &str, entry: &StatusEntry) -> Result<()> {
    let status_dir = get_status_dir()?;
    let path = status_dir.join(format!("{}.json", name));

    let json = serde_json::to_string_pretty(entry)?;
    fs::write(&path, json)?;

    Ok(())
}

fn read_status_entry(name: &str) -> Result<Option<StatusEntry>> {
    let status_dir = get_status_dir()?;
    let path = status_dir.join(format!("{}.json", name));

    if !path.exists() {
        // Try legacy .txt format
        let txt_path = status_dir.join(format!("{}.txt", name));
        if txt_path.exists() {
            let content = fs::read_to_string(&txt_path)?;
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                return Ok(Some(StatusEntry {
                    content: trimmed.to_string(),
                    set_at: "unknown".to_string(),
                    set_by: None,
                }));
            }
        }
        return Ok(None);
    }

    let json = fs::read_to_string(&path)?;
    let entry: StatusEntry = serde_json::from_str(&json)?;
    Ok(Some(entry))
}

/// Read message from argument or stdin
fn resolve_message(message: Option<String>) -> Result<String> {
    match message {
        Some(m) => Ok(m),
        None => {
            let stdin = io::stdin();
            if stdin.is_terminal() {
                anyhow::bail!("No message provided. Either pass as argument or pipe via stdin.");
            }
            let mut lines = Vec::new();
            for line in stdin.lock().lines() {
                lines.push(line?);
            }
            let content = lines.join("\n").trim().to_string();
            if content.is_empty() {
                anyhow::bail!("Empty message from stdin");
            }
            Ok(content)
        }
    }
}

fn clear_status_entry(name: &str) -> Result<bool> {
    let status_dir = get_status_dir()?;
    let json_path = status_dir.join(format!("{}.json", name));
    let txt_path = status_dir.join(format!("{}.txt", name));

    let mut cleared = false;
    if json_path.exists() {
        fs::remove_file(&json_path)?;
        cleared = true;
    }
    if txt_path.exists() {
        fs::remove_file(&txt_path)?;
        cleared = true;
    }

    Ok(cleared)
}

fn format_time_ago(iso_timestamp: &str) -> String {
    if iso_timestamp == "unknown" {
        return "unknown".to_string();
    }

    match DateTime::parse_from_rfc3339(iso_timestamp) {
        Ok(set_time) => {
            let now = Utc::now();
            let diff = now.signed_duration_since(set_time.with_timezone(&Utc));
            let mins = diff.num_minutes();

            if mins < 1 {
                "just now".to_string()
            } else if mins < 60 {
                format!("{}min ago", mins)
            } else if mins < 1440 {
                format!("{}h ago", mins / 60)
            } else {
                format!("{}d ago", mins / 1440)
            }
        }
        Err(_) => iso_timestamp.to_string(),
    }
}

fn format_toronto_time(iso_timestamp: &str) -> String {
    if iso_timestamp == "unknown" {
        return "unknown".to_string();
    }

    match DateTime::parse_from_rfc3339(iso_timestamp) {
        Ok(dt) => {
            let toronto_time = dt.with_timezone(&Toronto);
            toronto_time.format("%b %d @ %I:%M %p").to_string()
        }
        Err(_) => iso_timestamp.to_string(),
    }
}

pub fn run_status(args: StatusArgs) -> Result<()> {
    match args.command {
        StatusCommand::Focus { message, set_by, quiet } => {
            let resolved = resolve_message(message)?;
            let entry = StatusEntry::new(resolved.clone(), set_by.clone());
            write_status_entry("focus", &entry)?;

            if !quiet {
                let by = set_by.map(|s| format!(" by {}", s)).unwrap_or_default();
                println!("✓ Focus set{}: {}", by, resolved);
                println!("  ({})", format_toronto_time(&entry.set_at));
            }
        }

        StatusCommand::Notice { message, set_by, quiet } => {
            let resolved = resolve_message(message)?;
            let entry = StatusEntry::new(resolved.clone(), set_by.clone());
            write_status_entry("notice", &entry)?;

            if !quiet {
                let by = set_by.map(|s| format!(" by {}", s)).unwrap_or_default();
                println!("✓ Notice set{}: {}", by, resolved);
                println!("  ({})", format_toronto_time(&entry.set_at));
            }
        }

        StatusCommand::Clear { target } => {
            match target {
                ClearTarget::Focus => {
                    if clear_status_entry("focus")? {
                        println!("✓ Focus cleared");
                    } else {
                        println!("No focus was set");
                    }
                }
                ClearTarget::Notice => {
                    if clear_status_entry("notice")? {
                        println!("✓ Notice cleared");
                    } else {
                        println!("No notice was set");
                    }
                }
                ClearTarget::All => {
                    let focus_cleared = clear_status_entry("focus")?;
                    let notice_cleared = clear_status_entry("notice")?;
                    if focus_cleared || notice_cleared {
                        println!("✓ All status cleared");
                    } else {
                        println!("No status was set");
                    }
                }
            }
        }

        StatusCommand::Show { json } => {
            let focus = read_status_entry("focus")?;
            let notice = read_status_entry("notice")?;

            if json {
                let output = serde_json::json!({
                    "focus": focus,
                    "notice": notice,
                    "current_time": Local::now().with_timezone(&Toronto).to_rfc3339(),
                });
                println!("{}", serde_json::to_string_pretty(&output)?);
            } else {
                let now = Local::now().with_timezone(&Toronto);
                println!("━━━ SYSTEM STATUS ━━━");
                println!("🕐 {} (Toronto)", now.format("%a %b %d @ %I:%M %p"));

                if let Some(ref f) = focus {
                    let by = f.set_by.as_ref().map(|s| format!(" by {}", s)).unwrap_or_default();
                    println!("[FOCUS] {} (set {}{})", f.content, format_time_ago(&f.set_at), by);
                }

                if let Some(ref n) = notice {
                    let by = n.set_by.as_ref().map(|s| format!(" by {}", s)).unwrap_or_default();
                    println!("[NOTICE] {} (set {}{})", n.content, format_time_ago(&n.set_at), by);
                }

                if focus.is_none() && notice.is_none() {
                    println!("(no status set)");
                }

                println!("━━━━━━━━━━━━━━━━━━━━━");
            }
        }
    }

    Ok(())
}

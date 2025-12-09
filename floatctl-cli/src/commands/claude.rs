//! Claude Code session management commands
//!
//! Commands: list, recent-context, show

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

// === Arg Structs (moved from main.rs for high cohesion) ===

#[derive(Parser, Debug)]
pub struct ClaudeArgs {
    #[command(subcommand)]
    pub command: ClaudeCommands,
}

#[derive(Subcommand, Debug)]
pub enum ClaudeCommands {
    /// List recent Claude Code sessions from ~/.claude/projects/
    #[command(alias = "list-sessions")]
    List(ListSessionsArgs),
    /// Extract recent context for system prompt injection (evna's primary use case)
    RecentContext(RecentContextArgs),
    /// Pretty-print a Claude Code session log
    Show(ShowArgs),
}

#[derive(Parser, Debug)]
pub struct ListSessionsArgs {
    /// Number of sessions to list (default: 10)
    #[arg(short = 'n', long, default_value = "10")]
    limit: usize,

    /// Filter by project path (matches substring)
    #[arg(short = 'p', long)]
    project: Option<String>,

    /// Include agent sessions (excluded by default to reduce noise)
    #[arg(long)]
    include_agents: bool,

    /// Claude projects directory (default: ~/.claude/projects)
    #[arg(long)]
    projects_dir: Option<PathBuf>,

    /// Output format (json or text)
    #[arg(long, default_value = "text")]
    format: String,
}

#[derive(Parser, Debug)]
pub struct RecentContextArgs {
    /// Number of recent sessions to process (default: 3)
    #[arg(short = 's', long, default_value = "3")]
    sessions: usize,

    /// Number of first messages per session (default: 3)
    #[arg(short = 'f', long, default_value = "3")]
    first: usize,

    /// Number of last messages per session (default: 3)
    #[arg(short = 'l', long, default_value = "3")]
    last: usize,

    /// Truncate messages to N characters (0 = no truncation, default: 400)
    #[arg(short = 't', long, default_value = "400")]
    truncate: usize,

    /// Filter by project path (matches substring)
    #[arg(short = 'p', long)]
    project: Option<String>,

    /// Claude projects directory (default: ~/.claude/projects)
    #[arg(long)]
    projects_dir: Option<PathBuf>,

    /// Output format (json or text)
    #[arg(long, default_value = "json")]
    format: String,
}

#[derive(Parser, Debug)]
pub struct ShowArgs {
    /// Session ID or path to session log file
    session: String,

    /// Show only first N messages
    #[arg(long)]
    first: Option<usize>,

    /// Show only last N messages
    #[arg(long)]
    last: Option<usize>,

    /// Hide thinking blocks
    #[arg(long)]
    no_thinking: bool,

    /// Hide tool calls and results
    #[arg(long)]
    no_tools: bool,

    /// Output format (text, markdown, json)
    #[arg(long, default_value = "text")]
    format: String,

    /// Claude projects directory (default: ~/.claude/projects)
    #[arg(long)]
    projects_dir: Option<PathBuf>,
}

// === Command Implementations ===

pub fn run_claude(args: ClaudeArgs) -> Result<()> {
    match args.command {
        ClaudeCommands::List(list_args) => run_claude_list_sessions(list_args),
        ClaudeCommands::RecentContext(context_args) => run_claude_recent_context(context_args),
        ClaudeCommands::Show(show_args) => run_claude_show(show_args),
    }
}

fn run_claude_list_sessions(args: ListSessionsArgs) -> Result<()> {
    use floatctl_claude::commands::list_sessions::{
        default_projects_dir, list_sessions, ListSessionsOptions,
    };

    // Get projects directory (default or from args)
    let projects_dir = args
        .projects_dir
        .unwrap_or_else(default_projects_dir);

    // Build options
    let options = ListSessionsOptions {
        limit: args.limit,
        project_filter: args.project,
        include_agents: args.include_agents,
    };

    // List sessions
    let sessions = list_sessions(&projects_dir, &options)
        .context("Failed to list Claude Code sessions")?;

    // Output
    if args.format == "json" {
        println!("{}", serde_json::to_string_pretty(&sessions)?);
    } else {
        // Text format
        if sessions.is_empty() {
            println!("No recent Claude Code sessions found.");
        } else {
            println!("# Recent Claude Code Sessions ({})\n", sessions.len());
            for (idx, session) in sessions.iter().enumerate() {
                // Format started timestamp
                let started = chrono::DateTime::parse_from_rfc3339(&session.started)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|_| session.started.clone());

                println!("{}. **{}**", idx + 1, session.session_id);
                println!("   Project: {}", session.project);
                if let Some(ref branch) = session.branch {
                    println!("   Branch: {}", branch);
                }
                println!("   Started: {}", started);
                println!("   Turns: {}, Tool calls: {}", session.turn_count, session.tool_calls);
                println!();
            }
        }
    }

    Ok(())
}

fn run_claude_recent_context(args: RecentContextArgs) -> Result<()> {
    use floatctl_claude::commands::recent_context::{recent_context, RecentContextOptions};

    // Get projects directory (default or from args)
    let projects_dir = args.projects_dir.unwrap_or_else(|| {
        dirs::home_dir()
            .expect("Could not determine home directory")
            .join(".claude")
            .join("projects")
    });

    // Build options
    let options = RecentContextOptions {
        sessions: args.sessions,
        first: args.first,
        last: args.last,
        truncate: args.truncate,
        project_filter: args.project,
    };

    // Extract recent context
    let result = recent_context(&projects_dir, &options)
        .context("Failed to extract recent context from Claude Code sessions")?;

    // Output
    if args.format == "json" {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        // Text format
        if result.sessions.is_empty() {
            println!("No recent Claude Code sessions found.");
        } else {
            for session in &result.sessions {
                println!("## Session: {}", session.project);
                if let Some(ref branch) = session.branch {
                    println!("Branch: {}", branch);
                }
                println!("Started: {}", session.started);
                println!();

                if !session.first_messages.is_empty() {
                    println!("**First messages:**");
                    for msg in &session.first_messages {
                        println!(
                            "  [{}]: {}{}",
                            msg.role,
                            msg.content,
                            if msg.truncated { "..." } else { "" }
                        );
                    }
                    println!();
                }

                if !session.last_messages.is_empty() {
                    println!("**Last messages:**");
                    for msg in &session.last_messages {
                        println!(
                            "  [{}]: {}{}",
                            msg.role,
                            msg.content,
                            if msg.truncated { "..." } else { "" }
                        );
                    }
                    println!();
                }

                println!(
                    "**Stats:** {} turns, {} tool calls, {} failures",
                    session.stats.turn_count, session.stats.tool_calls, session.stats.failures
                );
                println!("\n---\n");
            }
        }
    }

    Ok(())
}

fn run_claude_show(args: ShowArgs) -> Result<()> {
    use floatctl_claude::commands::show::{show, ShowOptions};
    use std::path::PathBuf;
    use walkdir::WalkDir;

    // Resolve session path
    let log_path = if args.session.starts_with('/') || args.session.starts_with('~') {
        // Absolute path provided
        
        if args.session.starts_with('~') {
            dirs::home_dir()
                .context("Could not determine home directory")?
                .join(&args.session[2..])
        } else {
            PathBuf::from(&args.session)
        }
    } else if args.session.ends_with(".jsonl") {
        // Relative path to a .jsonl file
        PathBuf::from(&args.session)
    } else {
        // Session ID - search in projects directory
        let projects_dir = args.projects_dir.unwrap_or_else(|| {
            dirs::home_dir()
                .expect("Could not determine home directory")
                .join(".claude")
                .join("projects")
        });

        // Find all matching session files
        let mut found = Vec::new();

        for entry in WalkDir::new(&projects_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|s| s.to_str()) == Some("jsonl")
                && path.file_name()
                    .and_then(|s| s.to_str())
                    .map(|s| s.starts_with(&args.session))
                    .unwrap_or(false)
            {
                found.push(path.to_path_buf());
            }
        }

        if found.is_empty() {
            return Err(anyhow!("Session not found: {}", args.session));
        }

        if found.len() > 1 {
            eprintln!("Multiple sessions found matching '{}':", args.session);
            for path in &found {
                eprintln!("  {}", path.display());
            }
            return Err(anyhow!("Please specify a more specific session ID or use full path"));
        }

        found.into_iter().next().unwrap()
    };

    // Parse format
    use floatctl_claude::commands::show::OutputFormat;
    let format = match args.format.as_str() {
        "markdown" | "md" => OutputFormat::Markdown,
        "json" => OutputFormat::Json,
        _ => OutputFormat::Text,
    };

    // Build options
    let options = ShowOptions {
        with_thinking: !args.no_thinking,
        with_tools: !args.no_tools,
        format,
        first: args.first,
        last: args.last,
    };

    // Show the session
    show(&log_path, &options)
        .with_context(|| format!("Failed to show session: {}", log_path.display()))?;

    Ok(())
}

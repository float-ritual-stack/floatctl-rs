//! Wizard Mode - Interactive TUI fallback for missing arguments
//!
//! When a command is called without required arguments (e.g., `floatctl full-extract`),
//! instead of erroring, we launch an interactive wizard using the `inquire` crate.
//!
//! # Design Philosophy
//!
//! - **Guided Discovery**: Help new users understand what each argument means
//! - **Context-Aware Prompts**: Use domain-specific language, not generic "input"/"output"
//! - **Smart Defaults**: Pre-fill with sensible defaults where possible
//! - **Escape Hatch**: Users can always Ctrl+C to exit
//!
//! # Example
//!
//! ```text
//! $ floatctl full-extract
//!
//! 🧙 Full Extract Wizard
//!
//! ? Input file (JSON/ZIP export): conversations.json
//! ? Output directory: ~/.floatctl/conversation-exports
//! ? Output formats: [x] Markdown  [x] JSON  [ ] NDJSON
//! ? Preview only (dry run)? No
//!
//! Running: floatctl full-extract --in conversations.json --out ~/.floatctl/conversation-exports --format md,json
//! ```

use anyhow::{Context, Result};
use inquire::{Confirm, MultiSelect, Select, Text};
use std::io::IsTerminal;
use std::path::PathBuf;

/// Check if we're in a context where wizard mode is available
/// (interactive TTY, not in JSON mode)
pub fn can_use_wizard() -> bool {
    std::io::stdin().is_terminal()
        && std::io::stdout().is_terminal()
        && !crate::protocol::is_json_mode()
}

/// Helper trait for wizard-capable arguments
pub trait WizardFillable {
    /// Check if required arguments are missing
    fn needs_wizard(&self) -> bool;

    /// Fill missing arguments interactively
    fn fill_with_wizard(&mut self) -> Result<()>;
}

// ============================================================================
// Full Extract Wizard
// ============================================================================

/// Interactive wizard for `full-extract` command
pub fn wizard_full_extract() -> Result<FullExtractWizardResult> {
    println!("\n🧙 Full Extract Wizard\n");
    println!("Extract and organize conversations from LLM exports.\n");

    // Input file
    let input = Text::new("Input file (JSON array, ZIP, or NDJSON):")
        .with_help_message("Path to your ChatGPT/Claude export file")
        .with_placeholder("conversations.json")
        .prompt()
        .context("Failed to get input file")?;

    // Validate input exists
    let input_path = PathBuf::from(&input);
    if !input_path.exists() {
        println!("⚠️  Warning: File '{}' not found", input);
    }

    // Output directory
    let default_output = dirs::home_dir()
        .map(|h| h.join(".floatctl").join("conversation-exports"))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "./exports".to_string());

    let output = Text::new("Output directory:")
        .with_help_message("Where to save organized conversation folders")
        .with_default(&default_output)
        .prompt()
        .context("Failed to get output directory")?;

    // Output formats
    let format_options = vec!["Markdown (.md)", "JSON (.json)", "NDJSON (.ndjson)"];
    let formats = MultiSelect::new("Output formats:", format_options.clone())
        .with_help_message("Select one or more output formats")
        .with_default(&[0, 1]) // Default: MD and JSON
        .prompt()
        .context("Failed to get output formats")?;

    // Map to format strings
    let format_str: Vec<&str> = formats
        .iter()
        .map(|&f| match f {
            "Markdown (.md)" => "md",
            "JSON (.json)" => "json",
            "NDJSON (.ndjson)" => "ndjson",
            _ => "md",
        })
        .collect();

    // Dry run
    let dry_run = Confirm::new("Preview only (dry run)?")
        .with_default(false)
        .with_help_message("If yes, shows what would be done without writing files")
        .prompt()
        .context("Failed to get dry run preference")?;

    // Keep intermediate NDJSON
    let keep_ndjson = if !dry_run {
        Confirm::new("Keep intermediate NDJSON file?")
            .with_default(false)
            .with_help_message("Useful for re-processing or embedding later")
            .prompt()
            .context("Failed to get keep_ndjson preference")?
    } else {
        false
    };

    Ok(FullExtractWizardResult {
        input,
        output,
        formats: format_str.join(","),
        dry_run,
        keep_ndjson,
    })
}

#[derive(Debug)]
pub struct FullExtractWizardResult {
    pub input: String,
    pub output: String,
    pub formats: String,
    pub dry_run: bool,
    pub keep_ndjson: bool,
}

// ============================================================================
// Bridge Append Wizard
// ============================================================================

/// Interactive wizard for `bridge append` command
///
/// This wizard clarifies the domain-specific concepts:
/// - "Project Scope" instead of generic "subject"
/// - "Entry Summary" instead of generic "title"
pub fn wizard_bridge_append() -> Result<BridgeAppendWizardResult> {
    println!("\n🌉 Bridge Append Wizard\n");
    println!("Append conversation content to a project bridge file.\n");

    // List available projects (scan bridges directory)
    let bridges_dir = get_bridges_dir();
    let projects = list_bridge_projects(&bridges_dir);

    // Project selection
    let project = if !projects.is_empty() {
        let mut options = projects.clone();
        options.push("[Enter new project name]".to_string());

        let selection = Select::new("Project Scope:", options)
            .with_help_message("Which project does this conversation relate to?")
            .prompt()
            .context("Failed to select project")?;

        if selection == "[Enter new project name]" {
            Text::new("New project name:")
                .with_help_message("Use lowercase with hyphens (e.g., 'my-project')")
                .prompt()
                .context("Failed to get project name")?
        } else {
            selection
        }
    } else {
        Text::new("Project Scope:")
            .with_help_message("Project name (e.g., 'floatctl', 'evna-next')")
            .prompt()
            .context("Failed to get project name")?
    };

    // Issue number or identifier
    let issue = Text::new("Issue/Topic Identifier:")
        .with_help_message("Issue number, feature name, or topic (e.g., '42', 'auth-refactor')")
        .prompt()
        .context("Failed to get issue identifier")?;

    // Content source - only include stdin option when stdin is NOT a TTY
    // (i.e., when data is being piped in), otherwise it would block forever
    let mut content_sources = vec![
        "Enter text directly",
        "Read from file",
        "Read from clipboard",
    ];

    // Only offer stdin option if stdin is not a TTY (data is being piped)
    let stdin_is_pipe = !std::io::stdin().is_terminal();
    if stdin_is_pipe {
        content_sources.push("Read from stdin (pipe)");
    }

    let source = Select::new("Content source:", content_sources)
        .with_help_message("Where is the conversation content?")
        .prompt()
        .context("Failed to select content source")?;

    let content = match source {
        "Enter text directly" => {
            println!("\nEnter content (press Enter twice to finish):\n");
            let mut lines = Vec::new();
            let mut empty_count = 0;

            loop {
                let line = Text::new("")
                    .prompt()
                    .unwrap_or_default();

                if line.is_empty() {
                    empty_count += 1;
                    if empty_count >= 2 {
                        break;
                    }
                    lines.push(String::new());
                } else {
                    empty_count = 0;
                    lines.push(line);
                }
            }

            lines.join("\n").trim().to_string()
        }
        "Read from file" => {
            let file_path = Text::new("File path:")
                .prompt()
                .context("Failed to get file path")?;

            std::fs::read_to_string(&file_path)
                .with_context(|| format!("Failed to read file: {}", file_path))?
        }
        "Read from clipboard" => {
            cli_clipboard::get_contents()
                .map_err(|e| anyhow::anyhow!("Failed to read from clipboard: {}", e))?
        }
        "Read from stdin (pipe)" if stdin_is_pipe => {
            use std::io::Read;
            let mut buffer = String::new();
            std::io::stdin()
                .read_to_string(&mut buffer)
                .context("Failed to read from stdin")?;
            buffer
        }
        _ => String::new(),
    };

    // Dry run
    let dry_run = Confirm::new("Preview only (dry run)?")
        .with_default(false)
        .prompt()
        .context("Failed to get dry run preference")?;

    Ok(BridgeAppendWizardResult {
        project,
        issue,
        content,
        dry_run,
    })
}

#[derive(Debug)]
pub struct BridgeAppendWizardResult {
    pub project: String,
    pub issue: String,
    pub content: String,
    pub dry_run: bool,
}

// ============================================================================
// Embed Wizard
// ============================================================================

/// Interactive wizard for `embed` command
pub fn wizard_embed() -> Result<EmbedWizardResult> {
    println!("\n🔮 Embed Wizard\n");
    println!("Generate vector embeddings for semantic search.\n");

    // Input source
    let input_options = vec![
        "NDJSON file (messages.ndjson)",
        "Directory of conversation folders",
        "Single conversation file",
    ];

    let input_type = Select::new("Input source type:", input_options)
        .with_help_message("What would you like to embed?")
        .prompt()
        .context("Failed to select input type")?;

    let input = Text::new("Input path:")
        .with_help_message(match input_type {
            "NDJSON file (messages.ndjson)" => "Path to the NDJSON file with messages",
            "Directory of conversation folders" => "Path to directory with conversation folders",
            _ => "Path to conversation file",
        })
        .prompt()
        .context("Failed to get input path")?;

    // Project filter (optional)
    let use_project_filter = Confirm::new("Filter by project?")
        .with_default(false)
        .with_help_message("Only embed messages with specific project:: marker")
        .prompt()
        .context("Failed to get project filter preference")?;

    let project = if use_project_filter {
        Some(
            Text::new("Project name:")
                .prompt()
                .context("Failed to get project name")?,
        )
    } else {
        None
    };

    // Batch size
    let batch_size_str = Text::new("Batch size:")
        .with_default("100")
        .with_help_message("Number of messages to embed per API call")
        .prompt()
        .context("Failed to get batch size")?;

    let batch_size = batch_size_str
        .parse::<usize>()
        .with_context(|| format!("Invalid batch size '{}': must be a positive integer", batch_size_str))?;

    Ok(EmbedWizardResult {
        input,
        project,
        batch_size,
    })
}

#[derive(Debug)]
pub struct EmbedWizardResult {
    pub input: String,
    pub project: Option<String>,
    pub batch_size: usize,
}

// ============================================================================
// Search/Query Wizard
// ============================================================================

/// Interactive wizard for `query` and `search` commands
pub fn wizard_search() -> Result<SearchWizardResult> {
    println!("\n🔍 Search Wizard\n");
    println!("Search your conversation archive.\n");

    // Search mode
    let modes = vec![
        "Semantic search (pgvector embeddings)",
        "AI Search (Cloudflare AutoRAG)",
    ];

    let mode = Select::new("Search mode:", modes)
        .with_help_message("How would you like to search?")
        .prompt()
        .context("Failed to select search mode")?;

    let use_autorag = mode.contains("AutoRAG");

    // Search query
    let query = Text::new("Search query:")
        .with_help_message("What are you looking for?")
        .prompt()
        .context("Failed to get search query")?;

    // Result limit
    let limit_str = Text::new("Maximum results:")
        .with_default("10")
        .with_help_message("How many results to return")
        .prompt()
        .context("Failed to get limit")?;

    let limit = limit_str
        .parse::<usize>()
        .with_context(|| format!("Invalid limit '{}': must be a positive integer", limit_str))?;

    // Project filter
    let use_project = Confirm::new("Filter by project?")
        .with_default(false)
        .prompt()
        .context("Failed to get project filter preference")?;

    let project = if use_project {
        Some(
            Text::new("Project name:")
                .prompt()
                .context("Failed to get project name")?,
        )
    } else {
        None
    };

    Ok(SearchWizardResult {
        query,
        limit,
        project,
        use_autorag,
    })
}

#[derive(Debug)]
pub struct SearchWizardResult {
    pub query: String,
    pub limit: usize,
    pub project: Option<String>,
    pub use_autorag: bool,
}

// ============================================================================
// BBS Wizard
// ============================================================================

/// Interactive wizard for `bbs` commands
pub fn wizard_bbs() -> Result<BbsWizardResult> {
    println!("\n📮 BBS Wizard\n");
    println!("Bulletin Board System for agent messaging.\n");

    // Persona selection
    let personas = vec!["kitty", "daddy", "cowboy", "evna"];
    let persona = Select::new("Persona:", personas)
        .with_help_message("Which agent persona?")
        .prompt()
        .context("Failed to select persona")?
        .to_string();

    // Action
    let actions = vec![
        "inbox - List messages",
        "send - Send a message",
        "read - Mark message as read",
        "memory list - List memories",
        "memory save - Save a memory",
        "board list - List board posts",
        "board post - Post to board",
    ];

    let action = Select::new("Action:", actions)
        .prompt()
        .context("Failed to select action")?;

    let action_type = action.split(" - ").next().unwrap_or("inbox").to_string();

    Ok(BbsWizardResult {
        persona,
        action: action_type,
    })
}

#[derive(Debug)]
pub struct BbsWizardResult {
    pub persona: String,
    pub action: String,
}

// ============================================================================
// Helper Functions
// ============================================================================

/// Get the bridges directory from config or default
fn get_bridges_dir() -> PathBuf {
    dirs::home_dir()
        .map(|h| h.join("float-hub").join("float.dispatch").join("bridges"))
        .unwrap_or_else(|| PathBuf::from("./bridges"))
}

/// List existing bridge projects by scanning the bridges directory
fn list_bridge_projects(bridges_dir: &PathBuf) -> Vec<String> {
    if !bridges_dir.exists() {
        return Vec::new();
    }

    std::fs::read_dir(bridges_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|name| !name.starts_with('.'))
                .collect()
        })
        .unwrap_or_default()
}

/// Print the equivalent command that would be run
///
/// Uses proper POSIX shell escaping via shlex to handle special characters,
/// quotes, and other shell metacharacters safely.
pub fn print_equivalent_command(command: &str, args: &[(&str, &str)]) {
    let args_str: Vec<String> = args
        .iter()
        .filter(|(_, v)| !v.is_empty())
        .map(|(k, v)| {
            if *v == "true" {
                // Boolean flag - just the flag name
                format!("--{}", k)
            } else {
                // Use shlex for proper shell escaping
                let escaped = shlex::try_quote(v).unwrap_or_else(|_| (*v).into());
                format!("--{} {}", k, escaped)
            }
        })
        .collect();

    println!("\n📋 Equivalent command:");
    println!("   floatctl {} {}\n", command, args_str.join(" "));
}

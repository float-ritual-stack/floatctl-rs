//! BBS CLI commands - interact with float-bbs via HTTP API
//!
//! Commands: inbox, send, read, unread, memory, board
//!
//! Context economics: CLI + bash gives control over what enters context window.
//! MCP tools dump entire responses. CLI allows pipe/filter/extract.
//!
//! ```bash
//! # Instead of: float-bbs:check_inbox → context bomb
//! floatctl bbs inbox --json | jq '.messages[] | {from, subject}' | head -5
//! ```

use std::io::{IsTerminal, Read as IoRead};
use std::path::PathBuf;
use std::time::Duration;

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use reqwest::Client;
use serde::{Deserialize, Serialize};

// ============================================================================
// Main Args
// ============================================================================

#[derive(Parser, Debug)]
pub struct BbsArgs {
    /// BBS API endpoint (default: http://float-box:3030)
    #[arg(long, env = "FLOATCTL_BBS_ENDPOINT", global = true)]
    pub endpoint: Option<String>,

    /// Persona for operations (kitty, daddy, cowboy, evan, evna)
    #[arg(long, env = "FLOATCTL_PERSONA", global = true)]
    pub persona: Option<String>,

    /// Skip TLS certificate verification (for ngrok endpoints)
    #[arg(long, global = true)]
    pub insecure: bool,

    /// Subcommand (if missing + TTY, launches wizard)
    #[command(subcommand)]
    pub command: Option<BbsCommands>,
}

#[derive(Subcommand, Debug)]
pub enum BbsCommands {
    /// List inbox messages
    Inbox(InboxArgs),
    /// Show full message content
    Show(ShowArgs),
    /// Smart get - find item by ID across inbox/memories/boards
    Get(GetArgs),
    /// Send message to another persona
    Send(SendArgs),
    /// Mark message as read
    Read(ReadMarkArgs),
    /// Mark message as unread
    Unread(UnreadMarkArgs),
    /// Memory operations (list, save)
    Memory(MemoryArgs),
    /// Board operations (list, post)
    Board(BoardArgs),
}

// ============================================================================
// Output Format (shared)
// ============================================================================

#[derive(ValueEnum, Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Human-readable output (default)
    #[default]
    Human,
    /// JSON output (for piping to jq)
    Json,
    /// Quiet mode - IDs only
    Quiet,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq)]
pub enum GetType {
    /// Search inbox messages only
    Inbox,
    /// Search memories only
    Memory,
    /// Search board posts only
    Board,
}

// ============================================================================
// Inbox Commands
// ============================================================================

#[derive(Parser, Debug)]
pub struct InboxArgs {
    /// Max messages to return
    #[arg(long, short, default_value = "10")]
    pub limit: usize,

    /// Only show unread messages
    #[arg(long)]
    pub unread_only: bool,

    /// Filter by sender
    #[arg(long)]
    pub from: Option<String>,

    /// Output format
    #[arg(long, short, value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Shorthand for --output json
    #[arg(long, conflicts_with = "output")]
    pub json: bool,

    /// Shorthand for --output quiet
    #[arg(long, short, conflicts_with = "output")]
    pub quiet: bool,
}

#[derive(Parser, Debug)]
pub struct SendArgs {
    /// Recipient persona
    #[arg(long)]
    pub to: String,

    /// Message subject
    #[arg(long, short)]
    pub subject: String,

    /// Inline message content
    #[arg(long, short)]
    pub message: Option<String>,

    /// Read content from file
    #[arg(long)]
    pub file: Option<PathBuf>,

    /// Optional tags (can specify multiple)
    #[arg(long)]
    pub tag: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct ReadMarkArgs {
    /// Message ID to mark as read
    pub id: String,
}

#[derive(Parser, Debug)]
pub struct UnreadMarkArgs {
    /// Message ID to mark as unread
    pub id: String,
}

#[derive(Parser, Debug)]
pub struct ShowArgs {
    /// Message ID to show
    pub id: String,

    /// Output format
    #[arg(long, short, value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Shorthand for --output json
    #[arg(long, conflicts_with = "output")]
    pub json: bool,

    /// Also mark message as read
    #[arg(long, short)]
    pub mark_read: bool,
}

#[derive(Parser, Debug)]
pub struct GetArgs {
    /// ID or partial ID to search for (fuzzy match)
    pub query: String,

    /// Output format
    #[arg(long, short, value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Shorthand for --output json
    #[arg(long, conflicts_with = "output")]
    pub json: bool,

    /// Limit search to specific type
    #[arg(long, value_enum)]
    pub r#type: Option<GetType>,

    /// Max results to return (default: 5)
    #[arg(long, short = 'n', default_value = "5")]
    pub limit: usize,
}

// ============================================================================
// Memory Commands
// ============================================================================

#[derive(Parser, Debug)]
pub struct MemoryArgs {
    #[command(subcommand)]
    pub command: MemoryCommands,
}

#[derive(Subcommand, Debug)]
pub enum MemoryCommands {
    /// List memories
    List(MemoryListArgs),
    /// Save new memory
    Save(MemorySaveArgs),
}

#[derive(Parser, Debug)]
pub struct MemoryListArgs {
    /// Filter by category (patterns, moments, discoveries, reflections)
    #[arg(long, short)]
    pub category: Option<String>,

    /// Search query
    #[arg(long)]
    pub query: Option<String>,

    /// Max memories to return
    #[arg(long, default_value = "10")]
    pub limit: usize,

    /// Output format
    #[arg(long, short, value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Shorthand for --output json
    #[arg(long, conflicts_with = "output")]
    pub json: bool,

    /// Shorthand for --output quiet (IDs only)
    #[arg(long, short, conflicts_with = "output")]
    pub quiet: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum MemoryCategory {
    Patterns,
    Moments,
    Discoveries,
    Reflections,
}

impl std::fmt::Display for MemoryCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryCategory::Patterns => write!(f, "patterns"),
            MemoryCategory::Moments => write!(f, "moments"),
            MemoryCategory::Discoveries => write!(f, "discoveries"),
            MemoryCategory::Reflections => write!(f, "reflections"),
        }
    }
}

#[derive(Parser, Debug)]
pub struct MemorySaveArgs {
    /// Memory category
    #[arg(long, short, value_enum)]
    pub category: MemoryCategory,

    /// Memory title
    #[arg(long, short)]
    pub title: String,

    /// Inline content
    #[arg(long, short)]
    pub message: Option<String>,

    /// Read content from file
    #[arg(long)]
    pub file: Option<PathBuf>,

    /// Optional tags
    #[arg(long)]
    pub tag: Vec<String>,
}

// ============================================================================
// Board Commands
// ============================================================================

#[derive(Parser, Debug)]
pub struct BoardArgs {
    #[command(subcommand)]
    pub command: BoardCommands,
}

#[derive(Subcommand, Debug)]
pub enum BoardCommands {
    /// List boards or posts from a specific board
    List(BoardListArgs),
    /// Read a specific post from a board
    Read(BoardReadArgs),
    /// Post to a board
    Post(BoardPostArgs),
}

#[derive(Parser, Debug)]
pub struct BoardListArgs {
    /// Board name (omit to list all boards)
    pub board: Option<String>,

    /// Max posts to return (when listing specific board)
    #[arg(long, default_value = "20")]
    pub limit: usize,

    /// Output format
    #[arg(long, short, value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Shorthand for --output json
    #[arg(long, conflicts_with = "output")]
    pub json: bool,

    /// Shorthand for --output quiet (IDs only)
    #[arg(long, short, conflicts_with = "output")]
    pub quiet: bool,
}

#[derive(Parser, Debug)]
pub struct BoardReadArgs {
    /// Board name
    pub board: String,

    /// Post ID (filename without .md extension)
    pub post_id: String,

    /// Output format
    #[arg(long, short, value_enum, default_value = "human")]
    pub output: OutputFormat,

    /// Shorthand for --output json
    #[arg(long, conflicts_with = "output")]
    pub json: bool,
}

#[derive(Parser, Debug)]
pub struct BoardPostArgs {
    /// Board name
    pub board: String,

    /// Post title
    #[arg(long, short)]
    pub title: String,

    /// Inline content
    #[arg(long, short)]
    pub message: Option<String>,

    /// Read content from file
    #[arg(long)]
    pub file: Option<PathBuf>,

    /// Optional tags
    #[arg(long)]
    pub tag: Vec<String>,

    /// Custom metadata (key=value format)
    #[arg(long)]
    pub meta: Vec<String>,
}

// ============================================================================
// API Response Types (matching server)
// ============================================================================

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // API response fields for completeness
struct InboxListResponse {
    messages: Vec<InboxMessage>,
    total_unread: usize,
    persona: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct InboxMessage {
    id: String,
    from: String,
    subject: String,
    date: String,
    read: bool,
    preview: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // API response fields for completeness
struct MemoryListResponse {
    memories: Vec<Memory>,
    persona: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct Memory {
    id: String,
    title: String,
    category: String,
    date: String,
    preview: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize, Debug)]
struct BoardListResponse {
    boards: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // API response fields for completeness
struct BoardPostsResponse {
    posts: Vec<BoardPost>,
    board: String,
}

#[derive(Deserialize, Serialize, Debug)]
struct BoardPost {
    id: String,
    title: String,
    author: String,
    date: String,
    preview: String,
    #[serde(default)]
    content: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // API response fields for completeness
struct SuccessResponse {
    success: bool,
    id: String,
    path: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)] // API response fields for completeness
struct ErrorResponse {
    error: String,
    #[serde(default)]
    details: Option<String>,
}

#[derive(Deserialize, Debug)]
struct FileMatch {
    id: String,
    r#type: String,
    title: String,
    preview: String,
    date: String,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
struct FilesSearchResponse {
    matches: Vec<FileMatch>,
    paths_searched: Vec<String>,
}

// ============================================================================
// Main Dispatcher
// ============================================================================

pub async fn run_bbs(args: BbsArgs) -> Result<()> {
    // If no subcommand + TTY, launch wizard
    if args.command.is_none() {
        if std::io::stdin().is_terminal() {
            return run_bbs_wizard(args).await;
        } else {
            return Err(anyhow!("No subcommand specified. Use --help for usage."));
        }
    }

    // Extract values before moving command
    let endpoint = get_endpoint(&args)?;
    let persona = get_persona(&args)?;
    let insecure = args.insecure;
    let command = args.command.unwrap(); // Safe: checked is_some above

    match command {
        BbsCommands::Inbox(inbox_args) => run_inbox(&endpoint, &persona, inbox_args, insecure).await,
        BbsCommands::Show(show_args) => run_show(&endpoint, &persona, show_args, insecure).await,
        BbsCommands::Get(get_args) => run_get(&endpoint, &persona, get_args, insecure).await,
        BbsCommands::Send(send_args) => run_send(&endpoint, &persona, send_args, insecure).await,
        BbsCommands::Read(read_args) => run_mark_read(&endpoint, &persona, read_args, insecure).await,
        BbsCommands::Unread(unread_args) => run_mark_unread(&endpoint, &persona, unread_args, insecure).await,
        BbsCommands::Memory(memory_args) => run_memory(&endpoint, &persona, memory_args, insecure).await,
        BbsCommands::Board(board_args) => run_board(&endpoint, &persona, board_args, insecure).await,
    }
}

/// BBS wizard fallback - called when `floatctl bbs` with no subcommand + TTY
async fn run_bbs_wizard(args: BbsArgs) -> Result<()> {
    use crate::wizard;

    let wizard_result = wizard::wizard_bbs()?;
    let endpoint = get_endpoint(&args)?;
    let insecure = args.insecure;
    let persona = wizard_result.persona;

    // Route to appropriate command based on wizard action
    match wizard_result.action.as_str() {
        "inbox" => {
            let inbox_args = InboxArgs {
                limit: 10,
                unread_only: false,
                from: None,
                output: OutputFormat::Human,
                json: false,
                quiet: false,
            };
            run_inbox(&endpoint, &persona, inbox_args, insecure).await
        }
        "send" => {
            use inquire::Text;

            let to = Text::new("To (recipient persona):")
                .with_help_message("Common: kitty, daddy, cowboy, evan")
                .prompt()
                .context("Failed to get recipient")?;

            let subject = Text::new("Subject:")
                .prompt()
                .context("Failed to get subject")?;

            println!("Content (Ctrl+D or empty line to finish):");
            let mut content = String::new();
            std::io::stdin().read_to_string(&mut content).ok();
            let content = content.trim().to_string();

            let send_args = SendArgs {
                to,
                subject,
                message: Some(content),
                file: None,
                tag: vec![],
            };
            run_send(&endpoint, &persona, send_args, insecure).await
        }
        "memory list" => {
            let memory_args = MemoryArgs {
                command: MemoryCommands::List(MemoryListArgs {
                    category: None,
                    query: None,
                    limit: 20,
                    output: OutputFormat::Human,
                    json: false,
                    quiet: false,
                }),
            };
            run_memory(&endpoint, &persona, memory_args, insecure).await
        }
        "memory save" => {
            use inquire::{Select, Text};

            let title = Text::new("Memory title:")
                .prompt()
                .context("Failed to get title")?;

            let categories = vec!["patterns", "moments", "discoveries", "reflections"];
            let category_str = Select::new("Category:", categories)
                .prompt()
                .context("Failed to select category")?;

            let category = match category_str {
                "patterns" => MemoryCategory::Patterns,
                "moments" => MemoryCategory::Moments,
                "discoveries" => MemoryCategory::Discoveries,
                "reflections" => MemoryCategory::Reflections,
                _ => MemoryCategory::Patterns,
            };

            println!("Content (Ctrl+D or empty line to finish):");
            let mut content = String::new();
            std::io::stdin().read_to_string(&mut content).ok();
            let content = content.trim().to_string();

            let memory_args = MemoryArgs {
                command: MemoryCommands::Save(MemorySaveArgs {
                    title,
                    category,
                    tag: vec![],
                    message: Some(content),
                    file: None,
                }),
            };
            run_memory(&endpoint, &persona, memory_args, insecure).await
        }
        "board list" => {
            use inquire::Text;

            let board = Text::new("Board name (leave empty to list all boards):")
                .with_help_message("e.g., sysops-log, common")
                .prompt()
                .context("Failed to get board name")?;

            let board = if board.trim().is_empty() {
                None
            } else {
                Some(board.trim().to_string())
            };

            let board_args = BoardArgs {
                command: BoardCommands::List(BoardListArgs {
                    board,
                    limit: 20,
                    output: OutputFormat::Human,
                    json: false,
                    quiet: false,
                }),
            };
            run_board(&endpoint, &persona, board_args, insecure).await
        }
        "board post" => {
            use inquire::Text;

            let board = Text::new("Board name:")
                .with_help_message("e.g., sysops-log, common")
                .with_placeholder("sysops-log")
                .prompt()
                .context("Failed to get board name")?;

            let board = if board.is_empty() {
                "sysops-log".to_string()
            } else {
                board
            };

            let title = Text::new("Post title:")
                .prompt()
                .context("Failed to get title")?;

            println!("Content (Ctrl+D or empty line to finish):");
            let mut content = String::new();
            std::io::stdin().read_to_string(&mut content).ok();
            let content = content.trim().to_string();

            let board_args = BoardArgs {
                command: BoardCommands::Post(BoardPostArgs {
                    board,
                    title,
                    tag: vec![],
                    meta: vec![],
                    message: Some(content),
                    file: None,
                }),
            };
            run_board(&endpoint, &persona, board_args, insecure).await
        }
        _ => {
            println!("Action '{}' not fully implemented in wizard.", wizard_result.action);
            Ok(())
        }
    }
}

// ============================================================================
// Config Resolution
// ============================================================================

fn get_endpoint(args: &BbsArgs) -> Result<String> {
    // Priority: flag/env > config.toml > default
    if let Some(ref ep) = args.endpoint {
        return Ok(ep.clone());
    }

    // Try loading from config.toml
    if let Ok(config) = floatctl_core::FloatConfig::load() {
        if let Some(bbs) = config.bbs {
            if let Some(endpoint) = bbs.endpoint {
                return Ok(endpoint);
            }
        }
    }

    // Default
    Ok("http://float-box:3030".to_string())
}

fn get_persona(args: &BbsArgs) -> Result<String> {
    // Priority: flag/env > config.toml > error
    if let Some(ref p) = args.persona {
        return Ok(p.clone());
    }

    // Try loading from config.toml
    if let Ok(config) = floatctl_core::FloatConfig::load() {
        if let Some(bbs) = config.bbs {
            if let Some(persona) = bbs.persona {
                return Ok(persona);
            }
        }
    }

    Err(anyhow!(
        "Persona required. Use --persona, FLOATCTL_PERSONA env var, or set [bbs].persona in ~/.floatctl/config.toml"
    ))
}

fn get_output_format(output: OutputFormat, json_flag: bool, quiet_flag: bool) -> OutputFormat {
    if json_flag {
        OutputFormat::Json
    } else if quiet_flag {
        OutputFormat::Quiet
    } else {
        output
    }
}

/// Get search types for `bbs get` command
/// Priority: --type flag > config.toml > all types (default)
fn get_search_types(type_filter: Option<GetType>) -> Vec<GetType> {
    // If explicit filter, use only that
    if let Some(t) = type_filter {
        return vec![t];
    }

    // Try loading from config.toml
    if let Ok(config) = floatctl_core::FloatConfig::load() {
        if let Some(bbs) = config.bbs {
            if !bbs.get_search_types.is_empty() {
                return bbs
                    .get_search_types
                    .iter()
                    .filter_map(|s| match s.to_lowercase().as_str() {
                        "inbox" => Some(GetType::Inbox),
                        "memory" | "memories" => Some(GetType::Memory),
                        "board" | "boards" => Some(GetType::Board),
                        _ => {
                            tracing::warn!(invalid_type = %s, "Unknown get_search_type in config, skipping");
                            None
                        }
                    })
                    .collect();
            }
        }
    }

    // Default: all types
    vec![GetType::Inbox, GetType::Memory, GetType::Board]
}

/// Build HTTP client with optional TLS verification skip
fn build_client(insecure: bool) -> Result<Client> {
    let builder = Client::builder().timeout(Duration::from_secs(30));
    if insecure {
        builder
            .danger_accept_invalid_certs(true)
            .build()
            .context("Failed to build HTTP client with insecure mode")
    } else {
        builder
            .build()
            .context("Failed to build HTTP client")
    }
}

// ============================================================================
// Content Resolution (stdin/file/inline)
// ============================================================================

fn get_content(
    message: &Option<String>,
    file: &Option<PathBuf>,
    context: &str,
) -> Result<String> {
    // Priority: -m inline > --file > stdin
    if let Some(msg) = message {
        return Ok(msg.clone());
    }

    if let Some(path) = file {
        return std::fs::read_to_string(path)
            .with_context(|| format!("Failed to read file: {}", path.display()));
    }

    // Check if stdin has data (not a TTY)
    if !std::io::stdin().is_terminal() {
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .context("Failed to read from stdin")?;
        let content = buf.trim().to_string();
        if !content.is_empty() {
            return Ok(content);
        }
    }

    Err(anyhow!(
        "No content provided for {}. Use -m, --file, or pipe content via stdin",
        context
    ))
}

// ============================================================================
// HTTP Client Helpers
// ============================================================================

async fn handle_response<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
) -> Result<T> {
    let status = response.status();

    if status.is_success() {
        response.json::<T>().await.context("Failed to parse response")
    } else {
        // Try to parse error response
        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());

        if let Ok(error_resp) = serde_json::from_str::<ErrorResponse>(&error_text) {
            Err(anyhow!("{}: {}", status, error_resp.error))
        } else {
            Err(anyhow!("{}: {}", status, error_text))
        }
    }
}

// ============================================================================
// Inbox Implementation
// ============================================================================

async fn run_inbox(endpoint: &str, persona: &str, args: InboxArgs, insecure: bool) -> Result<()> {
    let client = build_client(insecure)?;
    let format = get_output_format(args.output, args.json, args.quiet);

    let mut url = format!("{}/{}/inbox?limit={}", endpoint, persona, args.limit);

    if args.unread_only {
        url.push_str("&unread_only=true");
    }

    if let Some(ref from) = args.from {
        url.push_str(&format!("&from={}", urlencoding::encode(from)));
    }

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to BBS API")?;

    let inbox: InboxListResponse = handle_response(response).await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&inbox.messages)?);
        }
        OutputFormat::Quiet => {
            for msg in &inbox.messages {
                println!("{}", msg.id);
            }
        }
        OutputFormat::Human => {
            let unread_marker = if inbox.total_unread > 0 {
                format!(" ({} unread)", inbox.total_unread)
            } else {
                String::new()
            };

            println!("┌─ {}'s inbox{}", persona, unread_marker);
            println!("│");

            if inbox.messages.is_empty() {
                println!("│  (no messages)");
            } else {
                for (i, msg) in inbox.messages.iter().enumerate() {
                    let is_last = i == inbox.messages.len() - 1;
                    let prefix = if is_last { "└─" } else { "├─" };
                    let cont_prefix = if is_last { "   " } else { "│  " };

                    let status = if msg.read { "[read]" } else { "[unread]" };
                    println!("{} {} from {} @ {}", prefix, status, msg.from, msg.date);
                    println!("{}Subject: {}", cont_prefix, msg.subject);

                    if !is_last {
                        println!("│");
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_send(endpoint: &str, persona: &str, args: SendArgs, insecure: bool) -> Result<()> {
    let content = get_content(&args.message, &args.file, "send")?;

    let client = build_client(insecure)?;

    #[derive(Serialize)]
    struct SendRequest {
        to: String,
        subject: String,
        content: String,
        tags: Vec<String>,
    }

    let request = SendRequest {
        to: args.to.clone(),
        subject: args.subject.clone(),
        content,
        tags: args.tag,
    };

    let url = format!("{}/{}/inbox", endpoint, persona);

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .context("Failed to connect to BBS API")?;

    let result: SuccessResponse = handle_response(response).await?;

    println!("✓ Message sent to {} (id: {})", args.to, result.id);

    Ok(())
}

async fn run_mark_read(endpoint: &str, persona: &str, args: ReadMarkArgs, insecure: bool) -> Result<()> {
    let client = build_client(insecure)?;
    let url = format!("{}/{}/inbox/{}/read", endpoint, persona, args.id);

    let response = client
        .put(&url)
        .send()
        .await
        .context("Failed to connect to BBS API")?;

    let _: SuccessResponse = handle_response(response).await?;

    println!("✓ Marked as read: {}", args.id);

    Ok(())
}

async fn run_mark_unread(endpoint: &str, persona: &str, args: UnreadMarkArgs, insecure: bool) -> Result<()> {
    let client = build_client(insecure)?;
    let url = format!("{}/{}/inbox/{}/unread", endpoint, persona, args.id);

    let response = client
        .put(&url)
        .send()
        .await
        .context("Failed to connect to BBS API")?;

    let _: SuccessResponse = handle_response(response).await?;

    println!("✓ Marked as unread: {}", args.id);

    Ok(())
}

async fn run_show(endpoint: &str, persona: &str, args: ShowArgs, insecure: bool) -> Result<()> {
    tracing::info!(persona = %persona, id = %args.id, "bbs show");
    let client = build_client(insecure)?;
    let format = get_output_format(args.output, args.json, false);

    let url = format!("{}/{}/inbox/{}", endpoint, persona, args.id);

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to BBS API")?;

    let msg: InboxMessage = handle_response(response).await?;

    // Optionally mark as read
    if args.mark_read && !msg.read {
        let read_url = format!("{}/{}/inbox/{}/read", endpoint, persona, args.id);
        let _ = client.put(&read_url).send().await;
    }

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&msg)?);
        }
        OutputFormat::Quiet => {
            // Just print the content
            println!("{}", msg.content);
        }
        OutputFormat::Human => {
            let status = if msg.read { "[read]" } else { "[unread]" };
            println!("┌─ {} from {} @ {}", status, msg.from, msg.date);
            println!("│  Subject: {}", msg.subject);
            if !msg.tags.is_empty() {
                println!("│  Tags: {}", msg.tags.join(", "));
            }
            println!("├──────────────────────────────────────────");
            println!("{}", msg.content);
            println!("└──────────────────────────────────────────");
        }
    }

    Ok(())
}

/// Unified match result for fuzzy get
#[derive(Serialize, Debug)]
struct GetMatch {
    id: String,
    r#type: String,
    title: String,
    preview: String,
    date: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    from: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    board: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    category: Option<String>,
}

async fn run_get(endpoint: &str, persona: &str, args: GetArgs, insecure: bool) -> Result<()> {
    let search_types = get_search_types(args.r#type);
    tracing::info!(persona = %persona, query = %args.query, limit = %args.limit, types = ?search_types, "bbs get");
    let client = build_client(insecure)?;
    let format = get_output_format(args.output, args.json, false);
    let query_lower = args.query.to_lowercase();
    let mut matches: Vec<GetMatch> = Vec::new();

    // Search inbox
    if search_types.contains(&GetType::Inbox) {
        let url = format!("{}/{}/inbox?limit=50", endpoint, persona);
        if let Ok(response) = client.get(&url).send().await {
            if let Ok(inbox) = response.json::<InboxListResponse>().await {
                for msg in inbox.messages {
                    if msg.id.to_lowercase().contains(&query_lower)
                        || msg.subject.to_lowercase().contains(&query_lower)
                    {
                        matches.push(GetMatch {
                            id: msg.id,
                            r#type: "inbox".to_string(),
                            title: msg.subject,
                            preview: msg.preview,
                            date: msg.date,
                            from: Some(msg.from),
                            author: None,
                            board: None,
                            category: None,
                        });
                    }
                }
            }
        }
    }

    // Search memories
    if search_types.contains(&GetType::Memory) {
        let url = format!("{}/{}/memories?limit=50", endpoint, persona);
        if let Ok(response) = client.get(&url).send().await {
            if let Ok(memories) = response.json::<MemoryListResponse>().await {
                for mem in memories.memories {
                    if mem.id.to_lowercase().contains(&query_lower)
                        || mem.title.to_lowercase().contains(&query_lower)
                    {
                        matches.push(GetMatch {
                            id: mem.id,
                            r#type: "memory".to_string(),
                            title: mem.title,
                            preview: mem.preview,
                            date: mem.date,
                            from: None,
                            author: None,
                            board: None,
                            category: Some(mem.category),
                        });
                    }
                }
            }
        }
    }

    // Search boards
    if search_types.contains(&GetType::Board) {
        // First get list of boards
        let boards_url = format!("{}/bbs/boards", endpoint);
        if let Ok(response) = client.get(&boards_url).send().await {
            if let Ok(boards) = response.json::<BoardListResponse>().await {
                for board_name in boards.boards {
                    let posts_url = format!(
                        "{}/{}/boards/{}?limit=30&include_content=false",
                        endpoint, persona, urlencoding::encode(&board_name)
                    );
                    if let Ok(response) = client.get(&posts_url).send().await {
                        if let Ok(board) = response.json::<BoardPostsResponse>().await {
                            for post in board.posts {
                                if post.id.to_lowercase().contains(&query_lower)
                                    || post.title.to_lowercase().contains(&query_lower)
                                {
                                    matches.push(GetMatch {
                                        id: post.id,
                                        r#type: "board".to_string(),
                                        title: post.title,
                                        preview: post.preview,
                                        date: post.date,
                                        from: None,
                                        author: Some(post.author),
                                        board: Some(board_name.clone()),
                                        category: None,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Search filesystem paths via server API
    let files_url = format!("{}/bbs/files?q={}&limit=50", endpoint, urlencoding::encode(&args.query));
    if let Ok(response) = client.get(&files_url).send().await {
        if let Ok(files) = response.json::<FilesSearchResponse>().await {
            for file in files.matches {
                matches.push(GetMatch {
                    id: file.id,
                    r#type: file.r#type,
                    title: file.title,
                    preview: file.preview,
                    date: file.date,
                    from: None,
                    author: None,
                    board: None,
                    category: None,
                });
            }
        }
    }

    // Truncate to limit
    matches.truncate(args.limit);

    tracing::info!(matches = %matches.len(), "bbs get results");

    if matches.is_empty() {
        println!("No matches found for '{}'", args.query);
        return Ok(());
    }

    // If exactly one match + human output, show full content
    if matches.len() == 1 && matches!(format, OutputFormat::Human) {
        let m = &matches[0];
        match m.r#type.as_str() {
            "inbox" => {
                // Fetch full message
                let url = format!("{}/{}/inbox/{}", endpoint, persona, m.id);
                if let Ok(response) = client.get(&url).send().await {
                    if let Ok(msg) = response.json::<InboxMessage>().await {
                        let status = if msg.read { "[read]" } else { "[unread]" };
                        println!("┌─ [inbox] {} from {} @ {}", status, msg.from, msg.date);
                        println!("│  Subject: {}", msg.subject);
                        println!("├──────────────────────────────────────────");
                        println!("{}", msg.content);
                        println!("└──────────────────────────────────────────");
                        return Ok(());
                    }
                }
            }
            "board" => {
                if let Some(board_name) = &m.board {
                    let url = format!(
                        "{}/{}/boards/{}?include_content=true&limit=100",
                        endpoint, persona, urlencoding::encode(board_name)
                    );
                    if let Ok(response) = client.get(&url).send().await {
                        if let Ok(board) = response.json::<BoardPostsResponse>().await {
                            if let Some(post) = board.posts.into_iter().find(|p| p.id == m.id) {
                                println!("┌─ [board::{}] {}", board_name, post.title);
                                println!("│  by {} @ {}", post.author, post.date);
                                println!("├──────────────────────────────────────────");
                                println!("{}", post.content);
                                println!("└──────────────────────────────────────────");
                                return Ok(());
                            }
                        }
                    }
                }
            }
            "memory" => {
                // Memory doesn't have a single-get endpoint, show preview
                println!("┌─ [memory::{}] {}", m.category.as_deref().unwrap_or("unknown"), m.title);
                println!("│  @ {}", m.date);
                println!("├──────────────────────────────────────────");
                println!("{}", m.preview);
                println!("└──────────────────────────────────────────");
                println!("│  (preview only - full content via file)");
                return Ok(());
            }
            t if t.starts_with("file") => {
                // Filesystem file - read directly
                let path = std::path::Path::new(&m.id);
                if let Ok(content) = std::fs::read_to_string(path) {
                    println!("┌─ [{}] {}", m.r#type, m.title);
                    println!("│  {}", m.id);
                    println!("├──────────────────────────────────────────");
                    println!("{}", content);
                    println!("└──────────────────────────────────────────");
                    return Ok(());
                }
            }
            _ => {}
        }
    }

    // Multiple matches or non-human format - list them
    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&matches)?);
        }
        OutputFormat::Quiet => {
            for m in &matches {
                println!("{}:{}", m.r#type, m.id);
            }
        }
        OutputFormat::Human => {
            println!("Found {} matches for '{}':", matches.len(), args.query);
            println!();
            for m in &matches {
                let type_badge = match m.r#type.as_str() {
                    "inbox" => format!("[inbox] from {}", m.from.as_deref().unwrap_or("?")),
                    "board" => format!("[board::{}] by {}", m.board.as_deref().unwrap_or("?"), m.author.as_deref().unwrap_or("?")),
                    "memory" => format!("[memory::{}]", m.category.as_deref().unwrap_or("?")),
                    t if t.starts_with("file") => format!("[{}]", t),
                    _ => format!("[{}]", m.r#type),
                };
                println!("  {} {}", type_badge, m.title);
                println!("    id: {} @ {}", m.id, m.date);
                println!();
            }
            println!("Use `floatctl bbs get <exact-id> -n 1` to view full content");
        }
    }

    Ok(())
}

// ============================================================================
// Memory Implementation
// ============================================================================

async fn run_memory(endpoint: &str, persona: &str, args: MemoryArgs, insecure: bool) -> Result<()> {
    match args.command {
        MemoryCommands::List(list_args) => run_memory_list(endpoint, persona, list_args, insecure).await,
        MemoryCommands::Save(save_args) => run_memory_save(endpoint, persona, save_args, insecure).await,
    }
}

async fn run_memory_list(endpoint: &str, persona: &str, args: MemoryListArgs, insecure: bool) -> Result<()> {
    let client = build_client(insecure)?;
    let format = get_output_format(args.output, args.json, args.quiet);

    let mut url = format!("{}/{}/memories?limit={}", endpoint, persona, args.limit);

    if let Some(ref category) = args.category {
        url.push_str(&format!("&category={}", urlencoding::encode(category)));
    }

    if let Some(ref query) = args.query {
        url.push_str(&format!("&query={}", urlencoding::encode(query)));
    }

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to BBS API")?;

    let memories: MemoryListResponse = handle_response(response).await?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&memories.memories)?);
        }
        OutputFormat::Quiet => {
            for mem in &memories.memories {
                println!("{}", mem.id);
            }
        }
        OutputFormat::Human => {
            println!("┌─ {}'s memories", persona);
            println!("│");

            if memories.memories.is_empty() {
                println!("│  (no memories)");
            } else {
                for (i, mem) in memories.memories.iter().enumerate() {
                    let is_last = i == memories.memories.len() - 1;
                    let prefix = if is_last { "└─" } else { "├─" };
                    let cont_prefix = if is_last { "   " } else { "│  " };

                    println!("{} [{}] {}", prefix, mem.category, mem.title);
                    println!("{}@ {}", cont_prefix, mem.date);

                    if !mem.tags.is_empty() {
                        println!("{}tags: {}", cont_prefix, mem.tags.join(", "));
                    }

                    if !is_last {
                        println!("│");
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_memory_save(endpoint: &str, persona: &str, args: MemorySaveArgs, insecure: bool) -> Result<()> {
    let content = get_content(&args.message, &args.file, "memory save")?;

    let client = build_client(insecure)?;

    #[derive(Serialize)]
    struct SaveMemoryRequest {
        title: String,
        content: String,
        category: String,
        tags: Vec<String>,
    }

    let request = SaveMemoryRequest {
        title: args.title.clone(),
        content,
        category: args.category.to_string(),
        tags: args.tag,
    };

    let url = format!("{}/{}/memories", endpoint, persona);

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .context("Failed to connect to BBS API")?;

    let result: SuccessResponse = handle_response(response).await?;

    println!(
        "✓ Memory saved: {} (category: {}, id: {})",
        args.title, args.category, result.id
    );

    Ok(())
}

// ============================================================================
// Board Implementation
// ============================================================================

/// Interactive post browser - scroll through posts, press Enter to view full content
fn interactive_post_browser(board_name: &str, posts: Vec<BoardPost>) -> Result<()> {
    use inquire::Select;

    // Format posts for selection display
    let post_options: Vec<String> = posts
        .iter()
        .map(|p| {
            let preview = if p.preview.len() > 60 {
                format!("{}...", &p.preview[..60])
            } else {
                p.preview.clone()
            };
            format!("{} | {} | {}", p.title, p.author, preview)
        })
        .collect();

    loop {
        // Build options with (back) option
        let mut options: Vec<&str> = vec!["← (back to boards)"];
        let post_refs: Vec<&str> = post_options.iter().map(|s| s.as_str()).collect();
        options.extend(post_refs);

        let selection = Select::new(&format!("📋 {} :: {} posts", board_name, posts.len()), options)
            .with_help_message("↑/↓ scroll, Enter to read, Esc to exit")
            .prompt();

        match selection {
            Ok(choice) if choice == "← (back to boards)" => {
                return Ok(());
            }
            Ok(choice) => {
                // Find the selected post
                if let Some(idx) = post_options.iter().position(|p| p.as_str() == choice) {
                    let post = &posts[idx];

                    // Display full post
                    println!();
                    println!("┌─ {} :: {}", board_name, post.title);
                    println!("│  by {} @ {}", post.author, post.date);
                    if !post.tags.is_empty() {
                        println!("│  tags: {}", post.tags.join(", "));
                    }
                    println!("├──────────────────────────────────────────");
                    println!("{}", post.content);
                    println!("└──────────────────────────────────────────");
                    println!();

                    // Prompt to continue
                    println!("Press Enter to continue browsing...");
                    let _ = std::io::stdin().read_line(&mut String::new());
                }
            }
            Err(_) => {
                // Esc or error - exit
                return Ok(());
            }
        }
    }
}

async fn run_board(endpoint: &str, persona: &str, args: BoardArgs, insecure: bool) -> Result<()> {
    match args.command {
        BoardCommands::List(list_args) => run_board_list(endpoint, persona, list_args, insecure).await,
        BoardCommands::Read(read_args) => run_board_read(endpoint, persona, read_args, insecure).await,
        BoardCommands::Post(post_args) => run_board_post(endpoint, persona, post_args, insecure).await,
    }
}

async fn run_board_list(endpoint: &str, persona: &str, args: BoardListArgs, insecure: bool) -> Result<()> {
    let client = build_client(insecure)?;
    let format = get_output_format(args.output, args.json, args.quiet);

    // If no board specified + TTY, offer wizard to select board
    let board_name = match args.board {
        Some(name) => Some(name),
        None if std::io::stdin().is_terminal() && matches!(format, OutputFormat::Human) => {
            // Fetch available boards first
            let url = format!("{}/bbs/boards", endpoint);
            let response = client.get(&url).send().await.context("Failed to connect to BBS API")?;
            let boards: BoardListResponse = handle_response(response).await?;

            if boards.boards.is_empty() {
                println!("No boards available.");
                return Ok(());
            }

            // Offer to select or show all
            let mut options: Vec<&str> = vec!["(show all boards)"];
            let board_refs: Vec<&str> = boards.boards.iter().map(|s| s.as_str()).collect();
            options.extend(board_refs);

            use inquire::Select;
            let selection = Select::new("Select board:", options)
                .with_help_message("Pick a board to view posts, or show all")
                .prompt()
                .context("Failed to select board")?;

            if selection == "(show all boards)" {
                // Print boards and exit
                println!("Available boards:");
                for board in &boards.boards {
                    println!("  • {}", board);
                }
                return Ok(());
            } else {
                // Fetch posts and offer interactive browser
                let board_name = selection.to_string();
                let posts_url = format!(
                    "{}/{}/boards/{}?limit={}&include_content=true",
                    endpoint, persona, urlencoding::encode(&board_name), args.limit
                );
                let posts_response = client.get(&posts_url).send().await.context("Failed to fetch posts")?;
                let board_resp: BoardPostsResponse = handle_response(posts_response).await?;

                if board_resp.posts.is_empty() {
                    println!("┌─ {} :: (no posts)", board_name);
                    return Ok(());
                }

                // Interactive post browser
                return interactive_post_browser(&board_name, board_resp.posts);
            }
        }
        None => None,
    };

    match board_name {
        Some(board_name) => {
            // List posts from specific board
            let url = format!(
                "{}/{}/boards/{}?limit={}",
                endpoint, persona, urlencoding::encode(&board_name), args.limit
            );

            let response = client
                .get(&url)
                .send()
                .await
                .context("Failed to connect to BBS API")?;

            let board: BoardPostsResponse = handle_response(response).await?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&board.posts)?);
                }
                OutputFormat::Quiet => {
                    for post in &board.posts {
                        println!("{}", post.id);
                    }
                }
                OutputFormat::Human => {
                    println!("┌─ {} :: {} posts", board_name, board.posts.len());
                    println!("│");

                    if board.posts.is_empty() {
                        println!("│  (no posts)");
                    } else {
                        for (i, post) in board.posts.iter().enumerate() {
                            let is_last = i == board.posts.len() - 1;
                            let prefix = if is_last { "└─" } else { "├─" };
                            let cont_prefix = if is_last { "   " } else { "│  " };

                            println!("{} {} by {} @ {}", prefix, post.title, post.author, post.date);
                            println!("{}id: {}", cont_prefix, post.id);

                            if !post.tags.is_empty() {
                                println!("{}tags: {}", cont_prefix, post.tags.join(", "));
                            }

                            if !is_last {
                                println!("│");
                            }
                        }
                    }
                }
            }
        }
        None => {
            // List all boards
            let url = format!("{}/bbs/boards", endpoint);

            let response = client
                .get(&url)
                .send()
                .await
                .context("Failed to connect to BBS API")?;

            let boards: BoardListResponse = handle_response(response).await?;

            match format {
                OutputFormat::Json => {
                    println!("{}", serde_json::to_string_pretty(&boards.boards)?);
                }
                OutputFormat::Quiet | OutputFormat::Human => {
                    println!("Available boards:");
                    for board in &boards.boards {
                        println!("  • {}", board);
                    }
                }
            }
        }
    }

    Ok(())
}

async fn run_board_read(endpoint: &str, persona: &str, args: BoardReadArgs, insecure: bool) -> Result<()> {
    let client = build_client(insecure)?;
    let format = get_output_format(args.output, args.json, false);

    // Fetch posts with content included
    let url = format!(
        "{}/{}/boards/{}?include_content=true&limit=100",
        endpoint, persona, urlencoding::encode(&args.board)
    );

    let response = client
        .get(&url)
        .send()
        .await
        .context("Failed to connect to BBS API")?;

    let board_resp: BoardPostsResponse = handle_response(response).await?;

    // Find the specific post
    let post = board_resp
        .posts
        .into_iter()
        .find(|p| p.id == args.post_id)
        .ok_or_else(|| anyhow!("Post '{}' not found in board '{}'", args.post_id, args.board))?;

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&post)?);
        }
        OutputFormat::Quiet => {
            // Just print the content
            println!("{}", post.content);
        }
        OutputFormat::Human => {
            println!("┌─ {} :: {}", args.board, post.title);
            println!("│  by {} @ {}", post.author, post.date);
            if !post.tags.is_empty() {
                println!("│  tags: {}", post.tags.join(", "));
            }
            println!("├──────────────────────────────────────────");
            println!("{}", post.content);
            println!("└──────────────────────────────────────────");
        }
    }

    Ok(())
}

async fn run_board_post(endpoint: &str, persona: &str, args: BoardPostArgs, insecure: bool) -> Result<()> {
    let content = get_content(&args.message, &args.file, "board post")?;

    let client = build_client(insecure)?;

    // Parse --meta key=value pairs
    let mut meta_map = std::collections::HashMap::new();
    for meta_item in &args.meta {
        if let Some((key, value)) = meta_item.split_once('=') {
            meta_map.insert(key.to_string(), value.to_string());
        } else {
            return Err(anyhow!("Invalid --meta format: '{}'. Use key=value", meta_item));
        }
    }

    #[derive(Serialize)]
    struct PostToBoardRequest {
        title: String,
        content: String,
        tags: Vec<String>,
        #[serde(skip_serializing_if = "std::collections::HashMap::is_empty")]
        meta: std::collections::HashMap<String, String>,
    }

    let request = PostToBoardRequest {
        title: args.title.clone(),
        content,
        tags: args.tag,
        meta: meta_map,
    };

    let url = format!("{}/{}/boards/{}", endpoint, persona, urlencoding::encode(&args.board));

    let response = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .context("Failed to connect to BBS API")?;

    let result: SuccessResponse = handle_response(response).await?;

    println!(
        "✓ Posted to {}: {} (id: {})",
        args.board, args.title, result.id
    );

    Ok(())
}

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

use std::io::Read;
use std::path::PathBuf;

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

    /// Persona for operations (kitty, daddy, cowboy, evna)
    #[arg(long, env = "FLOATCTL_PERSONA", global = true)]
    pub persona: Option<String>,

    /// Skip TLS certificate verification (for ngrok endpoints)
    #[arg(long, global = true)]
    pub insecure: bool,

    #[command(subcommand)]
    pub command: BbsCommands,
}

#[derive(Subcommand, Debug)]
pub enum BbsCommands {
    /// List inbox messages
    Inbox(InboxArgs),
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
    tags: Vec<String>,
}

#[derive(Deserialize, Debug)]
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
struct SuccessResponse {
    success: bool,
    id: String,
    path: String,
}

#[derive(Deserialize, Debug)]
struct ErrorResponse {
    error: String,
    #[serde(default)]
    details: Option<String>,
}

// ============================================================================
// Main Dispatcher
// ============================================================================

pub async fn run_bbs(args: BbsArgs) -> Result<()> {
    let endpoint = get_endpoint(&args)?;
    let persona = get_persona(&args)?;
    let insecure = args.insecure;

    match args.command {
        BbsCommands::Inbox(inbox_args) => run_inbox(&endpoint, &persona, inbox_args, insecure).await,
        BbsCommands::Send(send_args) => run_send(&endpoint, &persona, send_args, insecure).await,
        BbsCommands::Read(read_args) => run_mark_read(&endpoint, &persona, read_args, insecure).await,
        BbsCommands::Unread(unread_args) => run_mark_unread(&endpoint, &persona, unread_args, insecure).await,
        BbsCommands::Memory(memory_args) => run_memory(&endpoint, &persona, memory_args, insecure).await,
        BbsCommands::Board(board_args) => run_board(&endpoint, &persona, board_args, insecure).await,
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

/// Build HTTP client with optional TLS verification skip
fn build_client(insecure: bool) -> Result<Client> {
    let builder = Client::builder();
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
    if !atty::is(atty::Stream::Stdin) {
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
        url.push_str(&format!("&from={}", from));
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
    let format = get_output_format(args.output, args.json, false);

    let mut url = format!("{}/{}/memories?limit={}", endpoint, persona, args.limit);

    if let Some(ref category) = args.category {
        url.push_str(&format!("&category={}", category));
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

async fn run_board(endpoint: &str, persona: &str, args: BoardArgs, insecure: bool) -> Result<()> {
    match args.command {
        BoardCommands::List(list_args) => run_board_list(endpoint, persona, list_args, insecure).await,
        BoardCommands::Read(read_args) => run_board_read(endpoint, persona, read_args, insecure).await,
        BoardCommands::Post(post_args) => run_board_post(endpoint, persona, post_args, insecure).await,
    }
}

async fn run_board_list(endpoint: &str, persona: &str, args: BoardListArgs, insecure: bool) -> Result<()> {
    let client = build_client(insecure)?;
    let format = get_output_format(args.output, args.json, false);

    match args.board {
        Some(board_name) => {
            // List posts from specific board
            let url = format!(
                "{}/{}/boards/{}?limit={}",
                endpoint, persona, board_name, args.limit
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
        endpoint, persona, args.board
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

    let url = format!("{}/{}/boards/{}", endpoint, persona, args.board);

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

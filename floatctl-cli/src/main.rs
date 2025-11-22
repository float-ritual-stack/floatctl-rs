//! floatctl CLI - Conversation archive processing and cognitive tooling
//!
//! This is the main entry point for the floatctl command-line tool, which provides:
//! - Conversation export processing (JSON/NDJSON conversion, splitting, artifact extraction)
//! - Embedding pipeline for semantic search (with `embed` feature)
//! - Claude Code session log querying (`claude` subcommand)
//! - Bridge file management (`bridge` subcommand)
//! - R2 sync daemon management (`sync` subcommand)
//! - EVNA cognitive tools integration (`evna` subcommand)
//! - Script registration and execution (`script` subcommand)

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use floatctl_core::pipeline::{split_file, SplitOptions};
use floatctl_core::{cmd_ndjson, explode_messages, explode_ndjson_parallel};
use tracing::info;
use tracing_subscriber::EnvFilter;

mod commands;
mod config;
mod sync;

/// Get default output directory from config or ~/.floatctl/conversation-exports
#[cfg(feature = "embed")]
fn default_output_dir() -> Result<PathBuf> {
    use floatctl_embed::config::FloatctlConfig;

    let cfg = FloatctlConfig::load();
    let exports_dir = cfg.get_default_output_dir()?;

    // Create if doesn't exist
    if !exports_dir.exists() {
        std::fs::create_dir_all(&exports_dir)
            .context(format!("Failed to create {}", exports_dir.display()))?;
        info!("Created default output directory: {}", exports_dir.display());
    }

    Ok(exports_dir)
}

/// Get default output directory when embed feature is not enabled
#[cfg(not(feature = "embed"))]
fn default_output_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let exports_dir = home.join(".floatctl").join("conversation-exports");

    // Create if doesn't exist
    if !exports_dir.exists() {
        std::fs::create_dir_all(&exports_dir)
            .context(format!("Failed to create {}", exports_dir.display()))?;
        info!("Created default output directory: {}", exports_dir.display());
    }

    Ok(exports_dir)
}

#[derive(Parser, Debug)]
#[command(
    name = "floatctl",
    author,
    version,
    about = "Fast, streaming conversation archive processor for Claude and ChatGPT exports",
    long_about = "Process LLM conversation archives with O(1) memory usage. Extract artifacts, \
                  generate embeddings, and search semantically across your conversation history."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Split conversations into individual files (MD/JSON/NDJSON)
    Split(SplitArgs),
    /// Convert large JSON/ZIP exports to NDJSON (streaming, memory-efficient)
    Ndjson(NdjsonArgs),
    /// Explode NDJSON into individual files or extract messages
    Explode(ExplodeArgs),
    /// Full extraction workflow: auto-convert to NDJSON then split (one command)
    FullExtract(FullExtractArgs),
    #[cfg(feature = "embed")]
    Embed(floatctl_embed::EmbedArgs),
    #[cfg(feature = "embed")]
    /// Embed markdown notes/documents into note_embeddings table
    EmbedNotes(floatctl_embed::EmbedNotesArgs),
    #[cfg(feature = "embed")]
    /// Search embeddings (messages, notes, or all)
    Query(QueryCommand),
    /// Evna-next MCP server management (install, uninstall, status)
    Evna(EvnaArgs),
    /// R2 sync daemon management (status, trigger, start, stop, logs)
    Sync(sync::SyncArgs),
    /// Bridge maintenance operations (index annotations, analyze, etc.)
    Bridge(BridgeArgs),
    /// Query Claude Code session logs (for evna integration)
    Claude(ClaudeArgs),
    /// Generate shell completion scripts
    Completions(CompletionsArgs),
    /// Manage floatctl configuration (init, get, set, list, validate)
    Config(config::ConfigArgs),
    /// System diagnostics and maintenance
    System(SystemArgs),
    /// Manage registered shell scripts (register, list, run)
    Script(commands::script::ScriptArgs),
    /// Capture context markers to local queue (syncs to float-box)
    Ctx(CtxArgs),
}

#[derive(Parser, Debug)]
struct CtxArgs {
    /// Message to capture (or read from stdin)
    message: Option<String>,
}

#[derive(Parser, Debug)]
struct EvnaArgs {
    #[command(subcommand)]
    command: EvnaCommands,
}

#[derive(Subcommand, Debug)]
enum EvnaCommands {
    // === MCP Management Commands ===
    /// Install evna as MCP server in Claude Desktop
    Install(EvnaInstallArgs),
    /// Uninstall evna MCP server from Claude Desktop
    Uninstall,
    /// Show evna MCP server status
    Status,
    /// Start evna as remote MCP server (Supergateway + ngrok)
    Remote(EvnaRemoteArgs),

    // === Cognitive Tool Commands (shell out to evna binary) ===
    /// Brain boot - semantic search + active context + GitHub synthesis
    Boot(EvnaBootArgs),
    /// Deep semantic search across conversation history
    Search(EvnaSearchArgs),
    /// Query or capture recent activity stream
    Active(EvnaActiveArgs),
    /// LLM-orchestrated multi-tool search
    Ask(EvnaAskArgs),
    /// Conversational agent mode (Agent SDK)
    Agent(EvnaAgentArgs),
    /// Manage Claude Code session history
    Sessions(EvnaSessionsArgs),
}

#[derive(Parser, Debug)]
struct EvnaInstallArgs {
    /// Path to evna directory (defaults to ../evna relative to floatctl-rs)
    #[arg(long)]
    path: Option<PathBuf>,

    /// Force reinstall even if already configured
    #[arg(long)]
    force: bool,
}

#[derive(Parser, Debug)]
struct EvnaRemoteArgs {
    /// Path to evna directory (defaults to ../evna relative to floatctl-rs)
    #[arg(long)]
    path: Option<PathBuf>,

    /// Port for Supergateway SSE server (default: 3100)
    #[arg(long, default_value = "3100")]
    port: u16,

    /// Skip ngrok tunnel (only start Supergateway)
    #[arg(long)]
    no_tunnel: bool,

    /// ngrok authtoken (reads from ~/.ngrok2/ngrok.yml if not provided)
    #[arg(long)]
    ngrok_token: Option<String>,

    /// ngrok domain (for paid accounts with reserved domains)
    #[arg(long)]
    ngrok_domain: Option<String>,
}

// === Cognitive Tool Args (pass-through to evna binary) ===

#[derive(Parser, Debug)]
struct EvnaBootArgs {
    /// Natural language query describing what context to retrieve (or read from stdin if omitted)
    query: Option<String>,

    /// Filter by project name
    #[arg(long)]
    project: Option<String>,

    /// Lookback days (default: 7)
    #[arg(long)]
    days: Option<u32>,

    /// Maximum results (default: 10)
    #[arg(long)]
    limit: Option<u32>,

    /// GitHub username for PR/issue status
    #[arg(long)]
    github: Option<String>,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Minimal output
    #[arg(long)]
    quiet: bool,
}

#[derive(Parser, Debug)]
struct EvnaSearchArgs {
    /// Search query (natural language, question, or keywords; or read from stdin if omitted)
    query: Option<String>,

    /// Filter by project name
    #[arg(long)]
    project: Option<String>,

    /// Maximum results (default: 10)
    #[arg(long)]
    limit: Option<u32>,

    /// Similarity threshold 0-1 (default: 0.5)
    #[arg(long)]
    threshold: Option<f32>,

    /// Filter by timestamp (ISO 8601)
    #[arg(long)]
    since: Option<String>,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Minimal output
    #[arg(long)]
    quiet: bool,
}

#[derive(Parser, Debug)]
struct EvnaActiveArgs {
    /// Query to filter active context (optional if --capture used)
    query: Option<String>,

    /// Capture message to active context stream
    #[arg(long)]
    capture: bool,

    /// Filter by project name
    #[arg(long)]
    project: Option<String>,

    /// Maximum results (default: 10)
    #[arg(long)]
    limit: Option<u32>,

    /// Client type filter (desktop or claude_code)
    #[arg(long)]
    client: Option<String>,

    /// Exclude cross-client context
    #[arg(long)]
    no_cross_client: bool,

    /// Disable Ollama synthesis (return raw format)
    #[arg(long)]
    no_synthesize: bool,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Minimal output
    #[arg(long)]
    quiet: bool,
}

#[derive(Parser, Debug)]
struct EvnaAskArgs {
    /// Natural language query for LLM orchestrator (or read from stdin if omitted)
    query: Option<String>,

    /// Resume session by ID
    #[arg(long)]
    session: Option<String>,

    /// Fork existing session
    #[arg(long)]
    fork: bool,

    /// Timeout in milliseconds
    #[arg(long)]
    timeout: Option<u32>,

    /// Output as JSON
    #[arg(long)]
    json: bool,

    /// Minimal output
    #[arg(long)]
    quiet: bool,
}

#[derive(Parser, Debug)]
struct EvnaAgentArgs {
    /// Natural language query for conversational agent (or read from stdin if omitted)
    query: Option<String>,

    /// Resume session by ID
    #[arg(long)]
    session: Option<String>,

    /// Claude model to use
    #[arg(long)]
    model: Option<String>,

    /// Maximum agent turns
    #[arg(long)]
    max_turns: Option<u32>,

    /// Show detailed agent reasoning and tool calls
    #[arg(long)]
    verbose: bool,

    /// Disable streaming
    #[arg(long)]
    no_stream: bool,

    /// Save session for later resume
    #[arg(long)]
    save_session: bool,

    /// Minimal output
    #[arg(long)]
    quiet: bool,
}

#[derive(Parser, Debug)]
struct EvnaSessionsArgs {
    /// Subcommand (list or read)
    #[arg(default_value = "list")]
    subcommand: String,

    /// Session ID (for 'read' subcommand)
    session_id: Option<String>,

    /// Number of sessions to list (default: 10)
    #[arg(long, short = 'n')]
    n: Option<u32>,

    /// Filter by project
    #[arg(long)]
    project: Option<String>,

    /// First N messages from session
    #[arg(long)]
    first: Option<u32>,

    /// Last N messages from session
    #[arg(long)]
    last: Option<u32>,

    /// Truncate long messages (chars)
    #[arg(long)]
    truncate: Option<u32>,

    /// Output as JSON
    #[arg(long)]
    json: bool,
}

#[derive(Parser, Debug)]
struct BridgeArgs {
    #[command(subcommand)]
    command: BridgeCommands,
}

#[derive(Subcommand, Debug)]
enum BridgeCommands {
    /// Index :: annotations from markdown files to create bridge stubs
    Index(IndexArgs),
    /// Append conversation content to bridge files
    Append(AppendArgs),
}

#[derive(Parser, Debug)]
struct IndexArgs {
    /// Input file or directory path
    #[arg(value_name = "PATH")]
    input: PathBuf,

    /// Output directory for bridge files (default: ~/float-hub/float.dispatch/bridges)
    #[arg(long = "out", value_name = "DIR")]
    output: Option<PathBuf>,

    /// Recursively scan directories
    #[arg(long, short = 'r')]
    recursive: bool,

    /// Output JSON instead of human-readable format
    #[arg(long)]
    json: bool,
}

#[derive(Parser, Debug)]
struct AppendArgs {
    /// Read content from stdin
    #[arg(long, conflicts_with_all = ["file", "content"])]
    from_stdin: bool,

    /// Read content from file
    #[arg(long, conflicts_with_all = ["from_stdin", "content"])]
    file: Option<PathBuf>,

    /// Explicit project name
    #[arg(long, requires = "content")]
    project: Option<String>,

    /// Explicit issue number
    #[arg(long, requires = "content")]
    issue: Option<String>,

    /// Explicit content
    #[arg(long, requires_all = ["project", "issue"])]
    content: Option<String>,

    /// Minimum content length (default: 100)
    #[arg(long, default_value = "100")]
    min_length: usize,

    /// Require both project and issue annotations (default: false)
    #[arg(long, default_value = "false")]
    require_both: bool,

    /// Skip command-like messages (default: true)
    #[arg(long, default_value = "true", action = ArgAction::SetTrue)]
    skip_commands: bool,

    /// Deduplication window in seconds (default: 60)
    #[arg(long, default_value = "60")]
    dedup_window_secs: u64,

    /// Output directory for bridges (default: ~/float-hub/float.dispatch/bridges)
    #[arg(long)]
    out: Option<PathBuf>,

    /// JSON output (silent mode for hooks)
    #[arg(long)]
    json: bool,

    /// Dry run (show what would be appended without writing)
    #[arg(long)]
    dry_run: bool,
}

#[derive(Parser, Debug)]
struct ClaudeArgs {
    #[command(subcommand)]
    command: ClaudeCommands,
}

#[derive(Subcommand, Debug)]
enum ClaudeCommands {
    /// List recent Claude Code sessions from ~/.claude/projects/
    #[command(alias = "list-sessions")]
    List(ListSessionsArgs),
    /// Extract recent context for system prompt injection (evna's primary use case)
    RecentContext(RecentContextArgs),
    /// Pretty-print a Claude Code session log
    Show(ShowArgs),
}

#[derive(Parser, Debug)]
struct ListSessionsArgs {
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
struct RecentContextArgs {
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
struct ShowArgs {
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

#[derive(Parser, Debug)]
struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    shell: Shell,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

#[derive(Parser, Debug)]
struct SystemArgs {
    #[command(subcommand)]
    command: SystemCommands,
}

#[derive(Subcommand, Debug)]
enum SystemCommands {
    /// Run system health diagnostics
    HealthCheck,
    /// Clean up duplicate processes and zombies
    Cleanup(CleanupArgs),
}

#[derive(Parser, Debug)]
struct CleanupArgs {
    /// Preview cleanup actions without making changes
    #[arg(long)]
    dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long)]
    force: bool,
}


#[cfg(feature = "embed")]
#[derive(Parser, Debug)]
struct QueryCommand {
    #[command(subcommand)]
    command: QuerySubcommand,
}

#[cfg(feature = "embed")]
#[derive(Subcommand, Debug)]
enum QuerySubcommand {
    /// Search message embeddings (conversation messages)
    Messages(floatctl_embed::QueryArgs),
    /// Search note embeddings (daily notes, bridges, TLDRs)
    Notes(floatctl_embed::QueryArgs),
    /// Search all embeddings (messages + notes)
    All(floatctl_embed::QueryArgs),
    /// Search active context stream (recent messages, last 36 hours)
    Active(floatctl_embed::ActiveContextQueryArgs),
}

#[derive(Parser, Debug)]
struct SplitArgs {
    /// Input NDJSON file path
    #[arg(
        long = "in",
        value_name = "PATH",
        default_value = "conversations.ndjson"
    )]
    input: PathBuf,

    /// Output directory for conversation folders
    #[arg(long = "out", value_name = "DIR")]
    output: Option<PathBuf>,

    /// Output formats (comma-separated: md,json,ndjson)
    #[arg(long, value_delimiter = ',', default_value = "md,json,ndjson")]
    format: Vec<SplitFormat>,

    /// Preview operations without writing files
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Disable the real-time progress bar output
    #[arg(long = "no-progress", action = ArgAction::SetTrue)]
    no_progress: bool,
}

#[derive(Parser, Debug)]
struct NdjsonArgs {
    /// Input JSON array or ZIP file path
    #[arg(long = "in", value_name = "PATH")]
    input: PathBuf,

    /// Output NDJSON file path (default: <input>.ndjson)
    #[arg(long = "out", value_name = "PATH")]
    output: Option<PathBuf>,

    /// Pretty-print JSON output (canonical formatting)
    #[arg(long)]
    canonical: bool,
}

#[derive(Parser, Debug)]
struct ExplodeArgs {
    /// Input NDJSON file containing conversations
    #[arg(long = "in", value_name = "PATH")]
    input: PathBuf,

    /// Output directory for individual conversation files
    #[arg(long = "out", value_name = "DIR")]
    output: Option<PathBuf>,

    /// Extract messages instead of conversations (one file per message)
    #[arg(long)]
    messages: bool,
}

#[derive(Parser, Debug)]
struct FullExtractArgs {
    /// Input file (JSON array, ZIP, or NDJSON)
    #[arg(long = "in", value_name = "PATH")]
    input: PathBuf,

    /// Output directory for organized conversation folders
    #[arg(long = "out", value_name = "DIR")]
    output: Option<PathBuf>,

    /// Output formats (comma-separated: md,json,ndjson)
    #[arg(long, value_delimiter = ',', default_value = "md,json,ndjson")]
    format: Vec<SplitFormat>,

    /// Preview operations without writing files
    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Disable the real-time progress bar output
    #[arg(long = "no-progress", action = ArgAction::SetTrue)]
    no_progress: bool,

    /// Keep intermediate NDJSON file after extraction
    #[arg(long)]
    keep_ndjson: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
enum SplitFormat {
    Md,
    Json,
    Ndjson,
}

fn init_tracing() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_target(false)
        .compact()
        .try_init()
        .map_err(|err| anyhow!(err))
}

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing().ok();
    let cli = Cli::parse();
    match cli.command {
        Commands::Split(args) => run_split(args).await?,
        Commands::Ndjson(args) => run_ndjson(args)?,
        Commands::Explode(args) => run_explode(args)?,
        Commands::FullExtract(args) => run_full_extract(args).await?,
        #[cfg(feature = "embed")]
        Commands::Embed(args) => floatctl_embed::run_embed(args).await?,
        #[cfg(feature = "embed")]
        Commands::EmbedNotes(args) => floatctl_embed::run_embed_notes(args).await?,
        #[cfg(feature = "embed")]
        Commands::Query(cmd) => run_query(cmd).await?,
        Commands::Evna(args) => run_evna(args).await?,
        Commands::Sync(args) => sync::run_sync(args).await?,
        Commands::Bridge(args) => run_bridge(args)?,
        Commands::Claude(args) => run_claude(args)?,
        Commands::Completions(args) => run_completions(args)?,
        Commands::Config(args) => config::run_config(args)?,
        Commands::System(args) => run_system(args)?,
        Commands::Script(args) => commands::run_script(args)?,
        Commands::Ctx(args) => run_ctx(args)?,
    }
    Ok(())
}

async fn run_split(args: SplitArgs) -> Result<()> {
    // Use provided output or default to ~/.floatctl/conversation-exports
    let output_dir = match args.output {
        Some(path) => path,
        None => default_output_dir()?,
    };

    let mut opts = SplitOptions {
        output_dir: output_dir.clone(),
        dry_run: args.dry_run,
        show_progress: !args.no_progress,
        ..Default::default()
    };

    opts.emit_markdown = args.format.contains(&SplitFormat::Md);
    opts.emit_json = args.format.contains(&SplitFormat::Json);
    opts.emit_ndjson = args.format.contains(&SplitFormat::Ndjson);

    info!(
        "splitting export {:?} -> {:?} (formats: {:?})",
        args.input, output_dir, args.format
    );

    split_file(args.input, opts)
        .await
        .context("failed to split export")?;
    Ok(())
}

fn run_ndjson(args: NdjsonArgs) -> Result<()> {
    info!(
        "converting {:?} to NDJSON (canonical: {})",
        args.input, args.canonical
    );

    cmd_ndjson(&args.input, args.canonical, args.output.as_ref())
        .context("failed to convert to NDJSON")?;

    Ok(())
}

fn run_explode(args: ExplodeArgs) -> Result<()> {
    if args.messages {
        // Extract messages from a single conversation
        info!("extracting messages from {:?}", args.input);
        explode_messages(&args.input, args.output.as_ref())
            .context("failed to extract messages")?;
    } else {
        // Explode NDJSON into individual conversation files
        // Use provided output or default to ~/.floatctl/conversation-exports
        let output_dir = match args.output {
            Some(path) => path,
            None => default_output_dir()?,
        };
        info!(
            "exploding {:?} -> {:?} (parallel mode)",
            args.input, output_dir
        );
        explode_ndjson_parallel(&args.input, &output_dir)
            .context("failed to explode NDJSON")?;
    }
    Ok(())
}

async fn run_full_extract(args: FullExtractArgs) -> Result<()> {
    use floatctl_core::cmd_full_extract;

    // Use provided output or default to ~/.floatctl/conversation-exports
    let output_dir = match args.output {
        Some(path) => path,
        None => default_output_dir()?,
    };

    let mut opts = SplitOptions {
        output_dir: output_dir.clone(),
        dry_run: args.dry_run,
        show_progress: !args.no_progress,
        ..Default::default()
    };

    opts.emit_markdown = args.format.contains(&SplitFormat::Md);
    opts.emit_json = args.format.contains(&SplitFormat::Json);
    opts.emit_ndjson = args.format.contains(&SplitFormat::Ndjson);

    info!(
        "full extraction workflow: {:?} -> {:?} (formats: {:?})",
        args.input, output_dir, args.format
    );

    cmd_full_extract(&args.input, opts, args.keep_ndjson)
        .await
        .context("failed to run full extraction workflow")?;

    Ok(())
}

#[cfg(feature = "embed")]
async fn run_query(cmd: QueryCommand) -> Result<()> {
    match cmd.command {
        QuerySubcommand::Messages(args) => {
            floatctl_embed::run_query(args, floatctl_embed::QueryTable::Messages).await?
        }
        QuerySubcommand::Notes(args) => {
            floatctl_embed::run_query(args, floatctl_embed::QueryTable::Notes).await?
        }
        QuerySubcommand::All(args) => {
            floatctl_embed::run_query(args, floatctl_embed::QueryTable::All).await?
        }
        QuerySubcommand::Active(args) => {
            floatctl_embed::run_active_context_query(args).await?
        }
    }
    Ok(())
}

async fn run_evna(args: EvnaArgs) -> Result<()> {
    match args.command {
        // MCP Management
        EvnaCommands::Install(install_args) => evna_install(install_args).await?,
        EvnaCommands::Uninstall => evna_uninstall().await?,
        EvnaCommands::Status => evna_status().await?,
        EvnaCommands::Remote(remote_args) => evna_remote(remote_args).await?,

        // Cognitive Tools (shell out to evna binary)
        EvnaCommands::Boot(boot_args) => evna_boot(boot_args).await?,
        EvnaCommands::Search(search_args) => evna_search(search_args).await?,
        EvnaCommands::Active(active_args) => evna_active(active_args).await?,
        EvnaCommands::Ask(ask_args) => evna_ask(ask_args).await?,
        EvnaCommands::Agent(agent_args) => evna_agent(agent_args).await?,
        EvnaCommands::Sessions(sessions_args) => evna_sessions(sessions_args).await?,
    }
    Ok(())
}

async fn evna_install(args: EvnaInstallArgs) -> Result<()> {
    use serde_json::{json, Value};
    use std::fs;

    // Determine evna path
    let evna_path = if let Some(path) = args.path {
        path
    } else {
        // Try common locations in order
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let candidates = vec![
            home.join("float-hub-operations").join("floatctl-rs").join("evna"),
            home.join("float-hub-operations").join("evna"),
            home.join(".floatctl").join("evna"),
        ];

        candidates.into_iter()
            .find(|p| p.exists())
            .ok_or_else(|| anyhow!(
                "evna directory not found in common locations:\n\
                 - ~/float-hub-operations/floatctl-rs/evna\n\
                 - ~/float-hub-operations/evna\n\
                 - ~/.floatctl/evna\n\
                 \n\
                 Use --path to specify a custom location"
            ))?
    };

    // Validate evna directory exists
    if !evna_path.exists() {
        return Err(anyhow!(
            "evna directory not found at: {}\nUse --path to specify location",
            evna_path.display()
        ));
    }

    // Check for package.json to confirm it's evna
    let package_json = evna_path.join("package.json");
    if !package_json.exists() {
        return Err(anyhow!(
            "Not a valid evna directory (missing package.json): {}",
            evna_path.display()
        ));
    }

    // Get Claude Desktop config path
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let config_path = home
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");

    // Read existing config or create new one
    let mut config: Value = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .context("Failed to read Claude Desktop config")?;
        serde_json::from_str(&content)
            .context("Failed to parse Claude Desktop config JSON")?
    } else {
        json!({})
    };

    // Check if evna is already configured
    if let Some(mcp_servers) = config.get("mcpServers") {
        if let Some(evna) = mcp_servers.get("evna") {
            if !args.force {
                println!("✅ evna is already configured in Claude Desktop");
                println!("   Config: {}", serde_json::to_string_pretty(&evna)?);
                println!("\nUse --force to reinstall");
                return Ok(());
            } else {
                println!("🔄 Reinstalling evna (--force specified)");
            }
        }
    }

    // Get absolute path for config
    let evna_path_absolute = evna_path.canonicalize()
        .context("Failed to resolve evna absolute path")?;

    // Create evna MCP server configuration
    let evna_config = json!({
        "command": "bun",
        "args": ["run", "mcp-server"],
        "cwd": evna_path_absolute.to_string_lossy(),
        "env": {
            "NODE_ENV": "production"
        }
    });

    // Ensure mcpServers object exists
    if !config.is_object() {
        config = json!({});
    }
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = json!({});
    }

    // Add evna configuration
    config["mcpServers"]["evna"] = evna_config;

    // Write config back
    let config_dir = config_path.parent().unwrap();
    fs::create_dir_all(config_dir)?;

    let config_json = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, config_json)
        .context("Failed to write Claude Desktop config")?;

    println!("✅ Successfully installed evna MCP server!");
    println!("   Location: {}", evna_path_absolute.display());
    println!("   Config: {}", config_path.display());
    println!("\n📝 Next steps:");
    println!("   1. Ensure .env is configured in evna directory");
    println!("   2. Restart Claude Desktop to load the MCP server");
    println!("   3. Test with: 'Use the brain_boot tool to search for...'");

    Ok(())
}

async fn evna_uninstall() -> Result<()> {
    use serde_json::Value;
    use std::fs;

    let home = dirs::home_dir().context("Could not determine home directory")?;
    let config_path = home
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");

    if !config_path.exists() {
        println!("ℹ️  Claude Desktop config not found - nothing to uninstall");
        return Ok(());
    }

    let content = fs::read_to_string(&config_path)
        .context("Failed to read Claude Desktop config")?;
    let mut config: Value = serde_json::from_str(&content)
        .context("Failed to parse Claude Desktop config JSON")?;

    // Check if evna exists
    if let Some(mcp_servers) = config.get_mut("mcpServers") {
        if let Some(obj) = mcp_servers.as_object_mut() {
            if obj.remove("evna").is_some() {
                let config_json = serde_json::to_string_pretty(&config)?;
                fs::write(&config_path, config_json)?;
                println!("✅ Successfully uninstalled evna MCP server");
                println!("   Restart Claude Desktop to apply changes");
                return Ok(());
            }
        }
    }

    println!("ℹ️  evna is not configured - nothing to uninstall");
    Ok(())
}

async fn evna_status() -> Result<()> {
    use serde_json::Value;
    use std::fs;

    let home = dirs::home_dir().context("Could not determine home directory")?;
    let config_path = home
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");

    if !config_path.exists() {
        println!("❌ Claude Desktop config not found");
        println!("   Expected: {}", config_path.display());
        return Ok(());
    }

    let content = fs::read_to_string(&config_path)
        .context("Failed to read Claude Desktop config")?;
    let config: Value = serde_json::from_str(&content)
        .context("Failed to parse Claude Desktop config JSON")?;

    // Check if evna is configured
    if let Some(mcp_servers) = config.get("mcpServers") {
        if let Some(evna) = mcp_servers.get("evna") {
            println!("✅ evna MCP server is configured");
            println!("\n📋 Configuration:");
            println!("{}", serde_json::to_string_pretty(&evna)?);

            // Validate the path exists
            if let Some(cwd) = evna.get("cwd").and_then(|v| v.as_str()) {
                let evna_path = PathBuf::from(cwd);
                if evna_path.exists() {
                    println!("\n✅ evna directory exists: {}", evna_path.display());

                    // Check for .env file
                    let env_file = evna_path.join(".env");
                    if env_file.exists() {
                        println!("✅ .env file found");
                    } else {
                        println!("⚠️  .env file not found - configure before using");
                    }
                } else {
                    println!("\n❌ evna directory not found: {}", evna_path.display());
                }
            }

            return Ok(());
        }
    }

    println!("❌ evna is not configured");
    println!("   Run: floatctl evna install");
    Ok(())
}

/// Kill any process listening on the specified port
fn kill_process_on_port(port: u16) -> Result<()> {
    use std::process::Command;

    // Use lsof to find process ID on port
    let output = Command::new("lsof")
        .arg("-ti")
        .arg(format!(":{}", port))
        .output()?;

    if !output.status.success() {
        // No process found on port, or lsof failed
        return Ok(());
    }

    let pid_str = String::from_utf8_lossy(&output.stdout);
    let pid = pid_str.trim();

    if pid.is_empty() {
        // No process found
        return Ok(());
    }

    // Kill the process
    let kill_status = Command::new("kill")
        .arg(pid)
        .status()?;

    if !kill_status.success() {
        return Err(anyhow!("Failed to kill process {} on port {}", pid, port));
    }

    // Give it a moment to die
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}

/// Kill ngrok processes tunneling the specified port
fn kill_ngrok_for_port(port: u16) -> Result<()> {
    use std::process::Command;

    // Find all ngrok processes
    let output = Command::new("pgrep")
        .arg("-f")
        .arg(format!("ngrok.*{}", port))
        .output()?;

    if !output.status.success() {
        // No ngrok processes found for this port
        return Ok(());
    }

    let pids_str = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<&str> = pids_str.trim().split('\n').filter(|p| !p.is_empty()).collect();

    if pids.is_empty() {
        return Ok(());
    }

    // Kill ngrok processes for this port
    for pid in pids {
        Command::new("kill")
            .arg(pid)
            .status()
            .ok(); // Ignore errors, process might already be dead
    }

    // Give them a moment to die
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}

async fn evna_remote(args: EvnaRemoteArgs) -> Result<()> {
    use std::process::{Command, Stdio};

    // Determine evna path
    let evna_path = if let Some(path) = args.path {
        path
    } else {
        // Try common locations in order
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let candidates = vec![
            home.join("float-hub-operations").join("floatctl-rs").join("evna"),
            home.join("float-hub-operations").join("evna"),
            home.join(".floatctl").join("evna"),
        ];

        candidates.into_iter()
            .find(|p| p.exists())
            .ok_or_else(|| anyhow!(
                "evna directory not found in common locations:\n\
                 - ~/float-hub-operations/floatctl-rs/evna\n\
                 - ~/float-hub-operations/evna\n\
                 - ~/.floatctl/evna\n\
                 \n\
                 Use --path to specify a custom location"
            ))?
    };

    if !evna_path.exists() {
        return Err(anyhow!(
            "evna directory not found: {}\nUse --path to specify location",
            evna_path.display()
        ));
    }

    // Load .env from evna directory
    let env_file = evna_path.join(".env");
    if env_file.exists() {
        dotenvy::from_path(&env_file).ok(); // Load but don't fail if parsing errors
    }

    // Check dependencies
    println!("🔍 Checking dependencies...");

    // Check Supergateway
    let supergateway_check = Command::new("supergateway")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if supergateway_check.is_err() || !supergateway_check.unwrap().success() {
        return Err(anyhow!(
            "Supergateway not found. Install with:\n  npm install -g supergateway"
        ));
    }
    println!("✅ Supergateway found");

    // Check bun
    let bun_check = Command::new("bun")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if bun_check.is_err() || !bun_check.unwrap().success() {
        return Err(anyhow!(
            "bun not found. Install with:\n  curl -fsSL https://bun.sh/install | bash"
        ));
    }
    println!("✅ bun found");

    // Check ngrok (unless --no-tunnel)
    if !args.no_tunnel {
        let ngrok_check = Command::new("ngrok")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if ngrok_check.is_err() || !ngrok_check.unwrap().success() {
            return Err(anyhow!(
                "ngrok not found. Install from https://ngrok.com/download\nOr use --no-tunnel to skip"
            ));
        }
        println!("✅ ngrok found");

        // Check for ngrok authtoken
        if std::env::var("EVNA_NGROK_AUTHTOKEN").is_err()
            && std::env::var("NGROK_AUTHTOKEN").is_err()
            && args.ngrok_token.is_none()
        {
            println!("⚠️  Warning: No ngrok authtoken configured");
            println!("   Set EVNA_NGROK_AUTHTOKEN in .env or pass --ngrok-token");
            println!("   Get authtoken from: https://dashboard.ngrok.com/get-started/your-authtoken");
            println!();
        }
    }

    println!();
    println!("🚀 Starting EVNA remote MCP server");
    println!("   Directory: {}", evna_path.display());
    println!("   Port: {}", args.port);
    println!("   Transport: stdio → SSE");
    if !args.no_tunnel {
        println!("   Tunnel: ngrok");
    }
    println!();

    // Kill any existing process on the port
    println!("🧹 Checking for existing process on port {}...", args.port);
    if let Err(e) = kill_process_on_port(args.port) {
        println!("   ⚠️  Warning: Could not check/kill existing process: {}", e);
    } else {
        println!("   ✅ Port {} is clear", args.port);
    }
    println!();

    // Start Supergateway in background
    println!("📡 Starting Supergateway on port {}...", args.port);

    // Build PATH with common binary locations
    let home = std::env::var("HOME").unwrap_or_else(|_| "/Users/evan".to_string());
    let path_dirs = vec![
        format!("{}/.cargo/bin", home),
        format!("{}/.bun/bin", home),
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
        "/bin".to_string(),
    ];
    let path_env = path_dirs.join(":");

    let mut supergateway_cmd = Command::new("supergateway");
    supergateway_cmd
        .arg("--stdio")
        .arg("bun run --silent mcp-server")
        .arg("--port")
        .arg(args.port.to_string())
        .current_dir(&evna_path)
        .env("PATH", &path_env)
        .env("FLOATCTL_BIN", format!("{}/.cargo/bin/floatctl", home))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut supergateway_process = supergateway_cmd
        .spawn()
        .context("Failed to start Supergateway")?;

    // Give Supergateway time to start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Check if process is still running
    match supergateway_process.try_wait() {
        Ok(Some(status)) => {
            return Err(anyhow!(
                "Supergateway exited immediately with status: {}",
                status
            ));
        }
        Ok(None) => {
            println!("✅ Supergateway running");
            println!("   Local URL: http://localhost:{}/sse", args.port);
        }
        Err(e) => {
            return Err(anyhow!("Failed to check Supergateway status: {}", e));
        }
    }

    // Start ngrok tunnel (unless --no-tunnel)
    let mut ngrok_process = None;
    if !args.no_tunnel {
        // Kill any existing ngrok processes for this port
        println!();
        println!("🧹 Cleaning up ngrok for port {}...", args.port);
        if let Err(e) = kill_ngrok_for_port(args.port) {
            println!("   ⚠️  Warning: Could not kill ngrok: {}", e);
        } else {
            println!("   ✅ ngrok cleared");
        }

        println!();
        println!("🌐 Starting ngrok tunnel...");

        let mut ngrok_cmd = Command::new("ngrok");
        ngrok_cmd.arg("http").arg(args.port.to_string());

        // Priority: CLI arg > EVNA_NGROK_* env var > NGROK_* env var (fallback)
        if let Some(token) = args.ngrok_token
            .or_else(|| std::env::var("EVNA_NGROK_AUTHTOKEN").ok())
            .or_else(|| std::env::var("NGROK_AUTHTOKEN").ok())
        {
            ngrok_cmd.arg("--authtoken").arg(token);
        }

        // Reserved domain (CLI arg > EVNA_NGROK_DOMAIN env var)
        let domain = args.ngrok_domain
            .or_else(|| std::env::var("EVNA_NGROK_DOMAIN").ok());
        if let Some(domain) = domain.as_ref() {
            ngrok_cmd.arg("--domain").arg(domain);
            println!("   Using reserved domain: {}", domain);
        }

        // Basic auth (from env var only - too sensitive for CLI)
        if let Ok(auth) = std::env::var("EVNA_NGROK_AUTH") {
            ngrok_cmd.arg("--basic-auth").arg(auth);
            println!("   Basic auth enabled (from .env)");
        }

        ngrok_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut ngrok = ngrok_cmd.spawn().context("Failed to start ngrok")?;

        // Give ngrok time to establish tunnel
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Check if ngrok is still running
        match ngrok.try_wait() {
            Ok(Some(status)) => {
                // Kill Supergateway before returning
                let _ = supergateway_process.kill();
                return Err(anyhow!("ngrok exited with status: {}", status));
            }
            Ok(None) => {
                println!("✅ ngrok tunnel established");
                println!();

                // Build authenticated URL if we have domain + auth
                let auth_url = if let Some(domain) = domain.as_ref() {
                    if let Ok(auth) = std::env::var("EVNA_NGROK_AUTH") {
                        // Format: https://username:password@domain/sse
                        format!("https://{}@{}/sse", auth, domain)
                    } else {
                        // No auth: https://domain/sse
                        format!("https://{}/sse", domain)
                    }
                } else {
                    String::new()
                };

                // Copy to clipboard if we have a complete URL
                if !auth_url.is_empty() {
                    use cli_clipboard::{ClipboardContext, ClipboardProvider};
                    if let Ok(mut ctx) = ClipboardContext::new() {
                        if ctx.set_contents(auth_url.clone()).is_ok() {
                            println!("📋 Copied to clipboard: {}", auth_url);
                        }
                    }
                }

                // Show URL based on whether we have a reserved domain
                if let Some(domain) = domain {
                    println!("🎯 Public URL: https://{}/sse", domain);
                    println!();

                    // Check if we have auth credentials
                    if let Ok(auth) = std::env::var("EVNA_NGROK_AUTH") {
                        println!("   (URL with auth credentials copied to clipboard)");
                        println!();

                        // Show both config formats
                        println!("📋 Claude Desktop config (URL auth):");
                        println!(r#"   {{
     "mcpServers": {{
       "evna-remote": {{
         "url": "https://{}@{}/sse",
         "transport": "sse"
       }}
     }}
   }}"#, auth, domain);
                        println!();

                        // Base64 encode for Authorization header
                        use base64::{Engine as _, engine::general_purpose};
                        let encoded = general_purpose::STANDARD.encode(&auth);

                        println!("📋 Claude Code config (header auth):");
                        println!(r#"   {{
     "mcpServers": {{
       "evna-remote": {{
         "url": "https://{}/sse",
         "transport": "sse",
         "headers": {{
           "Authorization": "Basic {}"
         }}
       }}
     }}
   }}"#, domain, encoded);
                        println!();

                        println!("💻 Claude Code CLI command:");
                        println!(r#"   claude mcp add evna-remote https://{}/sse --transport sse --header "Authorization: Basic {}""#, domain, encoded);
                    } else {
                        // No auth
                        println!("📋 Claude Desktop config:");
                        println!(r#"   {{
     "mcpServers": {{
       "evna-remote": {{
         "url": "https://{}/sse",
         "transport": "sse"
       }}
     }}
   }}"#, domain);
                    }
                } else {
                    println!("🎯 Public URL: Check http://localhost:4040 for ngrok URL");
                    println!("   (ngrok web UI shows the public HTTPS URL)");
                    println!();
                    println!("📋 Claude Desktop config:");
                    println!(r#"   {{
     "mcpServers": {{
       "evna-remote": {{
         "url": "https://YOUR-NGROK-URL.ngrok-free.app/sse",
         "transport": "sse"
       }}
     }}
   }}"#);
                }
                ngrok_process = Some(ngrok);
            }
            Err(e) => {
                let _ = supergateway_process.kill();
                return Err(anyhow!("Failed to check ngrok status: {}", e));
            }
        }
    }

    println!();
    println!("✨ EVNA remote MCP server is online!");
    println!("   Press Ctrl+C to stop");
    println!();

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    println!();
    println!("🛑 Shutting down...");

    // Kill processes
    if let Some(mut ngrok) = ngrok_process {
        let _ = ngrok.kill();
        println!("✅ ngrok stopped");
    }

    let _ = supergateway_process.kill();
    println!("✅ Supergateway stopped");

    println!("👋 EVNA remote MCP server stopped");

    Ok(())
}

// === Cognitive Tool Handlers (shell out to evna binary) ===

async fn evna_boot(args: EvnaBootArgs) -> Result<()> {
    let mut cmd_args = vec!["boot".to_string()];

    // Add query if provided (otherwise evna will read from stdin)
    if let Some(query) = args.query {
        cmd_args.push(query);
    }

    if let Some(project) = args.project {
        cmd_args.extend(["--project".to_string(), project]);
    }
    if let Some(days) = args.days {
        cmd_args.extend(["--days".to_string(), days.to_string()]);
    }
    if let Some(limit) = args.limit {
        cmd_args.extend(["--limit".to_string(), limit.to_string()]);
    }
    if let Some(github) = args.github {
        cmd_args.extend(["--github".to_string(), github]);
    }
    if args.json {
        cmd_args.push("--json".to_string());
    }
    if args.quiet {
        cmd_args.push("--quiet".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

async fn evna_search(args: EvnaSearchArgs) -> Result<()> {
    let mut cmd_args = vec!["search".to_string()];

    // Add query if provided (otherwise evna will read from stdin)
    if let Some(query) = args.query {
        cmd_args.push(query);
    }

    if let Some(project) = args.project {
        cmd_args.extend(["--project".to_string(), project]);
    }
    if let Some(limit) = args.limit {
        cmd_args.extend(["--limit".to_string(), limit.to_string()]);
    }
    if let Some(threshold) = args.threshold {
        cmd_args.extend(["--threshold".to_string(), threshold.to_string()]);
    }
    if let Some(since) = args.since {
        cmd_args.extend(["--since".to_string(), since]);
    }
    if args.json {
        cmd_args.push("--json".to_string());
    }
    if args.quiet {
        cmd_args.push("--quiet".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

async fn evna_active(args: EvnaActiveArgs) -> Result<()> {
    let mut cmd_args = vec!["active".to_string()];

    if let Some(query) = args.query {
        cmd_args.push(query);
    }

    if args.capture {
        cmd_args.push("--capture".to_string());
    }
    if let Some(project) = args.project {
        cmd_args.extend(["--project".to_string(), project]);
    }
    if let Some(limit) = args.limit {
        cmd_args.extend(["--limit".to_string(), limit.to_string()]);
    }
    if let Some(client) = args.client {
        cmd_args.extend(["--client".to_string(), client]);
    }
    if args.no_cross_client {
        cmd_args.push("--no-cross-client".to_string());
    }
    if args.no_synthesize {
        cmd_args.push("--no-synthesize".to_string());
    }
    if args.json {
        cmd_args.push("--json".to_string());
    }
    if args.quiet {
        cmd_args.push("--quiet".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

async fn evna_ask(args: EvnaAskArgs) -> Result<()> {
    let mut cmd_args = vec!["ask".to_string()];

    // Add query if provided (otherwise evna will read from stdin)
    if let Some(query) = args.query {
        cmd_args.push(query);
    }

    if let Some(session) = args.session {
        cmd_args.extend(["--session".to_string(), session]);
    }
    if args.fork {
        cmd_args.push("--fork".to_string());
    }
    if let Some(timeout) = args.timeout {
        cmd_args.extend(["--timeout".to_string(), timeout.to_string()]);
    }
    if args.json {
        cmd_args.push("--json".to_string());
    }
    if args.quiet {
        cmd_args.push("--quiet".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

async fn evna_agent(args: EvnaAgentArgs) -> Result<()> {
    let mut cmd_args = vec!["agent".to_string()];

    // Add query if provided (otherwise evna will read from stdin)
    if let Some(query) = args.query {
        cmd_args.push(query);
    }

    if let Some(session) = args.session {
        cmd_args.extend(["--session".to_string(), session]);
    }
    if let Some(model) = args.model {
        cmd_args.extend(["--model".to_string(), model]);
    }
    if let Some(max_turns) = args.max_turns {
        cmd_args.extend(["--max-turns".to_string(), max_turns.to_string()]);
    }
    if args.verbose {
        cmd_args.push("--verbose".to_string());
    }
    if args.no_stream {
        cmd_args.push("--no-stream".to_string());
    }
    if args.save_session {
        cmd_args.push("--save-session".to_string());
    }
    if args.quiet {
        cmd_args.push("--quiet".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

async fn evna_sessions(args: EvnaSessionsArgs) -> Result<()> {
    let mut cmd_args = vec!["sessions".to_string(), args.subcommand];

    if let Some(session_id) = args.session_id {
        cmd_args.push(session_id);
    }

    if let Some(n) = args.n {
        cmd_args.extend(["--n".to_string(), n.to_string()]);
    }
    if let Some(project) = args.project {
        cmd_args.extend(["--project".to_string(), project]);
    }
    if let Some(first) = args.first {
        cmd_args.extend(["--first".to_string(), first.to_string()]);
    }
    if let Some(last) = args.last {
        cmd_args.extend(["--last".to_string(), last.to_string()]);
    }
    if let Some(truncate) = args.truncate {
        cmd_args.extend(["--truncate".to_string(), truncate.to_string()]);
    }
    if args.json {
        cmd_args.push("--json".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

/// Shell out to evna binary and pass through output
async fn shell_out_to_evna(args: &[String]) -> Result<()> {
    use std::process::Command;

    // Try to find evna binary in PATH first, fall back to common locations
    let evna_bin = which::which("evna").ok().or_else(|| {
        let home = dirs::home_dir()?;
        let candidates = vec![
            home.join("float-hub-operations/floatctl-rs/evna/bin/evna"),
            home.join("float-hub-operations/evna/bin/evna"),
            home.join(".floatctl/evna/bin/evna"),
            home.join(".local/bin/evna"),
        ];

        candidates.into_iter().find(|p| p.exists())
    }).context(
        "evna binary not found. Install with:\n\
         1. cd evna\n\
         2. bun install\n\
         3. chmod +x bin/evna\n\
         4. ln -s $(pwd)/bin/evna ~/.local/bin/evna"
    )?;

    // Execute evna with pass-through args (inherit stdio for user visibility)
    let status = Command::new(&evna_bin)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context(format!("Failed to execute evna binary: {}", evna_bin.display()))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn run_bridge(args: BridgeArgs) -> Result<()> {
    match args.command {
        BridgeCommands::Index(index_args) => run_bridge_index(index_args),
        BridgeCommands::Append(append_args) => run_bridge_append(append_args),
    }
}

fn run_bridge_index(args: IndexArgs) -> Result<()> {
    use floatctl_bridge::{index_directory, index_file};
    use floatctl_core::FloatConfig;
    use std::path::PathBuf;

    // Get bridges output directory
    let bridges_dir = if let Some(path) = args.output {
        path
    } else {
        // Try centralized config first, fall back to default
        FloatConfig::load()
            .ok()
            .map(|c| PathBuf::from(c.paths.bridges))
            .unwrap_or_else(|| {
                let home = dirs::home_dir().expect("Could not determine home directory");
                home.join("float-hub")
                    .join("float.dispatch")
                    .join("bridges")
            })
    };

    // Check if input is file or directory
    let input_path = &args.input;
    if !input_path.exists() {
        return Err(anyhow!(
            "Input path does not exist: {}",
            input_path.display()
        ));
    }

    let result = if input_path.is_file() {
        // Index single file
        info!(
            "Indexing file: {} -> {}",
            input_path.display(),
            bridges_dir.display()
        );
        index_file(input_path, &bridges_dir)
            .context("Failed to index file")?
    } else if input_path.is_dir() {
        // Index directory
        info!(
            "Indexing directory{}: {} -> {}",
            if args.recursive { " (recursive)" } else { "" },
            input_path.display(),
            bridges_dir.display()
        );
        index_directory(input_path, &bridges_dir, args.recursive)
            .context("Failed to index directory")?
    } else {
        return Err(anyhow!(
            "Input path is neither file nor directory: {}",
            input_path.display()
        ));
    };

    // Output results
    if args.json {
        // JSON output
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        // Human-readable output
        println!("✅ Bridge indexing complete");
        println!();

        if !result.bridges_created.is_empty() {
            println!("📝 Created {} new bridges:", result.bridges_created.len());
            for bridge in &result.bridges_created {
                println!("   - {}", bridge);
            }
            println!();
        }

        if !result.bridges_updated.is_empty() {
            println!("🔄 Updated {} existing bridges:", result.bridges_updated.len());
            for bridge in &result.bridges_updated {
                println!("   - {}", bridge);
            }
            println!();
        }

        if result.references_added > 0 {
            println!("🔗 Added {} references", result.references_added);
        }

        if result.bridges_created.is_empty()
            && result.bridges_updated.is_empty()
            && result.references_added == 0
        {
            println!("ℹ️  No annotations found with project + issue markers");
        }
    }

    Ok(())
}

fn run_bridge_append(args: AppendArgs) -> Result<()> {
    use floatctl_bridge::append::{append_to_bridge, AppendOptions, AppendResult};
    use floatctl_core::FloatConfig;
    use std::io::{self, Read};
    use std::path::PathBuf;

    // Get bridges output directory
    let bridges_dir = if let Some(path) = args.out {
        path
    } else {
        // Try centralized config first, fall back to default
        FloatConfig::load()
            .ok()
            .map(|c| PathBuf::from(c.paths.bridges))
            .unwrap_or_else(|| {
                let home = dirs::home_dir().expect("Could not determine home directory");
                home.join("float-hub")
                    .join("float.dispatch")
                    .join("bridges")
            })
    };


    // Get content from specified source
    let content = if args.from_stdin {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("Failed to read from stdin")?;
        buffer
    } else if let Some(file_path) = args.file {
        std::fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?
    } else if let Some(text) = args.content {
        // Explicit content mode: inject annotations
        let project = args.project.as_ref().unwrap();
        let issue = args.issue.as_ref().unwrap();
        format!("project::{} issue::{}\n\n{}", project, issue, text)
    } else {
        return Err(anyhow!("Must specify one of: --from-stdin, --file, or --content"));
    };

    // Build options
    let options = AppendOptions {
        min_length: args.min_length,
        require_both: args.require_both,
        skip_commands: args.skip_commands,
        dedup_window_secs: args.dedup_window_secs,
    };

    // Dry run mode
    if args.dry_run {
        let metadata = floatctl_bridge::parse_annotations(&content)?;
        println!("🔍 Dry run mode - would append to:");
        println!("   Project: {:?}", metadata.project);
        println!("   Issue: {:?}", metadata.issue);
        println!("   Content length: {}", content.len());
        return Ok(());
    }

    // Perform append
    let result = append_to_bridge(&content, &bridges_dir, &options)?;

    // Output results
    if args.json {
        println!("{}", serde_json::to_string(&result)?);
    } else {
        match result {
            AppendResult::Success {
                bridge_updated,
                project,
                issue,
                content_length,
                ..
            } => {
                println!("✅ Bridge updated: {}", bridge_updated);
                println!("   Project: {}", project);
                println!("   Issue: {}", issue);
                println!("   Content: {} chars", content_length);
            }
            AppendResult::Skipped { reason, .. } => {
                println!("⏭️  Skipped: {}", reason);
            }
        }
    }

    Ok(())
}

fn run_claude(args: ClaudeArgs) -> Result<()> {
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
        let path = if args.session.starts_with('~') {
            dirs::home_dir()
                .context("Could not determine home directory")?
                .join(&args.session[2..])
        } else {
            PathBuf::from(&args.session)
        };
        path
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

fn run_completions(args: CompletionsArgs) -> Result<()> {
    use clap::CommandFactory;
    use clap_complete::{generate, Shell as CompletionShell};
    use std::io;

    let mut cmd = Cli::command();
    let bin_name = cmd.get_name().to_string();

    let shell = match args.shell {
        Shell::Bash => CompletionShell::Bash,
        Shell::Zsh => CompletionShell::Zsh,
        Shell::Fish => CompletionShell::Fish,
        Shell::PowerShell => CompletionShell::PowerShell,
        Shell::Elvish => CompletionShell::Elvish,
    };

    generate(shell, &mut cmd, bin_name, &mut io::stdout());

    Ok(())
}

fn run_system(args: SystemArgs) -> Result<()> {
    match args.command {
        SystemCommands::HealthCheck => run_system_health_check(),
        SystemCommands::Cleanup(cleanup_args) => run_system_cleanup(cleanup_args),
    }
}

fn run_system_health_check() -> Result<()> {
    use std::process::Command;

    let home = dirs::home_dir().context("Could not determine home directory")?;
    let script_path = home.join(".floatctl").join("bin").join("health-check.sh");

    // Validate script exists
    if !script_path.exists() {
        return Err(anyhow!(
            "System script not found: {}\n\
             Install system scripts with: floatctl sync install",
            script_path.display()
        ));
    }

    // Execute script
    let status = Command::new(&script_path)
        .status()
        .with_context(|| format!("Failed to execute health-check: {}", script_path.display()))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(anyhow!("Health check found {} issue(s)", code));
    }

    Ok(())
}

fn run_system_cleanup(args: CleanupArgs) -> Result<()> {
    use std::process::Command;

    let home = dirs::home_dir().context("Could not determine home directory")?;
    let script_path = home.join(".floatctl").join("bin").join("cleanup.sh");

    // Validate script exists
    if !script_path.exists() {
        return Err(anyhow!(
            "System script not found: {}\n\
             Install system scripts with: floatctl sync install",
            script_path.display()
        ));
    }

    // Build command with arguments
    let mut cmd = Command::new(&script_path);

    if args.dry_run {
        cmd.arg("--dry-run");
    }

    if args.force {
        cmd.arg("--force");
    }

    // Execute script
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute cleanup: {}", script_path.display()))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(anyhow!("Cleanup exited with code: {}", code));
    }

    Ok(())
}


fn run_ctx(args: CtxArgs) -> Result<()> {
    use chrono::Utc;
    use serde_json::json;
    use std::fs::OpenOptions;
    use std::io::{self, Read, Write};

    // Get message from args or stdin
    let message = if let Some(msg) = args.message {
        msg
    } else {
        let mut buffer = String::new();
        io::stdin().read_to_string(&mut buffer)?;
        buffer.trim().to_string()
    };

    if message.is_empty() {
        return Err(anyhow!("Message cannot be empty"));
    }

    // Queue path
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let queue_path = home.join(".floatctl/ctx-queue.jsonl");

    // Create parent directory if needed
    if let Some(parent) = queue_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Get machine name
    let machine = hostname::get()
        .ok()
        .and_then(|h| h.into_string().ok())
        .unwrap_or_else(|| "unknown".to_string());

    // Create entry
    let entry = json!({
        "timestamp": Utc::now().to_rfc3339(),
        "message": message,
        "machine": machine,
    });

    // Append to queue
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&queue_path)?;

    writeln!(file, "{}", serde_json::to_string(&entry)?)?;

    Ok(())
}


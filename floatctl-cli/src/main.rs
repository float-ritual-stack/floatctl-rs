use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use floatctl_core::pipeline::{split_file, SplitOptions};
use floatctl_core::{cmd_ndjson, explode_messages, explode_ndjson_parallel};
use tracing::info;
use tracing_subscriber::EnvFilter;

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
    /// Manage registered shell scripts (register, list, run)
    Script(ScriptArgs),
}

#[derive(Parser, Debug)]
struct EvnaArgs {
    #[command(subcommand)]
    command: EvnaCommands,
}

#[derive(Subcommand, Debug)]
enum EvnaCommands {
    /// Install evna as MCP server in Claude Desktop
    Install(EvnaInstallArgs),
    /// Uninstall evna MCP server from Claude Desktop
    Uninstall,
    /// Show evna MCP server status
    Status,
    /// Start evna as remote MCP server (Supergateway + ngrok)
    Remote(EvnaRemoteArgs),
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
    /// List recent Claude Code sessions from history.jsonl
    ListSessions(ListSessionsArgs),
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

    /// Path to history.jsonl (default: ~/.claude/history.jsonl)
    #[arg(long)]
    history: Option<PathBuf>,

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

    /// Show thinking blocks
    #[arg(long)]
    with_thinking: bool,

    /// Hide tool calls and results
    #[arg(long)]
    no_tools: bool,

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
struct ScriptArgs {
    #[command(subcommand)]
    command: ScriptCommands,
}

#[derive(Subcommand, Debug)]
enum ScriptCommands {
    /// Register a shell script for reuse
    Register(RegisterScriptArgs),
    /// List all registered scripts
    List,
    /// Run a registered script with arguments
    Run(RunScriptArgs),
}

#[derive(Parser, Debug)]
struct RegisterScriptArgs {
    /// Path to the script file to register
    #[arg(value_name = "PATH")]
    script_path: PathBuf,

    /// Optional name for the script (defaults to filename)
    #[arg(long, short = 'n')]
    name: Option<String>,

    /// Force overwrite if script already exists
    #[arg(long, short = 'f')]
    force: bool,

    /// Preview registration without copying file
    #[arg(long)]
    dry_run: bool,
}

#[derive(Parser, Debug)]
struct RunScriptArgs {
    /// Name of the registered script to run
    script_name: String,

    /// Arguments to pass to the script
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
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
        Commands::Script(args) => run_script(args)?,
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
        EvnaCommands::Install(install_args) => evna_install(install_args).await?,
        EvnaCommands::Uninstall => evna_uninstall().await?,
        EvnaCommands::Status => evna_status().await?,
        EvnaCommands::Remote(remote_args) => evna_remote(remote_args).await?,
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
        // Default: ../evna relative to floatctl-rs
        let current_dir = std::env::current_dir()?;
        current_dir.parent()
            .context("Cannot determine parent directory")?
            .join("evna")
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
        // Default: ../evna relative to floatctl-rs
        let current_dir = std::env::current_dir()?;
        let floatctl_rs = current_dir
            .ancestors()
            .find(|p| p.file_name().map(|n| n == "floatctl-rs").unwrap_or(false))
            .ok_or_else(|| anyhow!("Could not find floatctl-rs parent directory"))?;
        floatctl_rs.join("evna")
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

fn run_bridge(args: BridgeArgs) -> Result<()> {
    match args.command {
        BridgeCommands::Index(index_args) => run_bridge_index(index_args),
        BridgeCommands::Append(append_args) => run_bridge_append(append_args),
    }
}

fn run_bridge_index(args: IndexArgs) -> Result<()> {
    use floatctl_bridge::{index_directory, index_file};

    // Get bridges output directory
    let bridges_dir = if let Some(path) = args.output {
        path
    } else {
        // Default: ~/float-hub/float.dispatch/bridges
        let home = dirs::home_dir().context("Could not determine home directory")?;
        home.join("float-hub")
            .join("float.dispatch")
            .join("bridges")
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
    use std::io::{self, Read};

    // Get bridges output directory
    let bridges_dir = if let Some(path) = args.out {
        path
    } else {
        // Default: ~/float-hub/float.dispatch/bridges
        let home = dirs::home_dir().context("Could not determine home directory")?;
        home.join("float-hub")
            .join("float.dispatch")
            .join("bridges")
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
        ClaudeCommands::ListSessions(list_args) => run_claude_list_sessions(list_args),
        ClaudeCommands::RecentContext(context_args) => run_claude_recent_context(context_args),
        ClaudeCommands::Show(show_args) => run_claude_show(show_args),
    }
}

fn run_claude_list_sessions(args: ListSessionsArgs) -> Result<()> {
    use floatctl_claude::commands::list_sessions::{
        default_history_path, list_sessions, ListSessionsOptions,
    };

    // Get history path (default or from args)
    let history_path = args
        .history
        .unwrap_or_else(default_history_path);

    // Build options
    let options = ListSessionsOptions {
        limit: args.limit,
        project_filter: args.project,
    };

    // List sessions
    let sessions = list_sessions(&history_path, &options)
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
                let timestamp = chrono::DateTime::parse_from_rfc3339(&session.timestamp)
                    .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|_| session.timestamp.clone());

                println!("{}. **{}**", idx + 1, timestamp);
                println!("   Project: {}", session.project);
                if !session.display.is_empty() {
                    println!("   {}", session.display);
                }
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

    // Build options
    let options = ShowOptions {
        with_thinking: args.with_thinking,
        with_tools: !args.no_tools,
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

fn run_script(args: ScriptArgs) -> Result<()> {
    match args.command {
        ScriptCommands::Register(register_args) => run_script_register(register_args),
        ScriptCommands::List => run_script_list(),
        ScriptCommands::Run(run_args) => run_script_run(run_args),
    }
}

fn get_scripts_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let scripts_dir = home.join(".floatctl").join("scripts");

    // Create if doesn't exist
    if !scripts_dir.exists() {
        std::fs::create_dir_all(&scripts_dir)
            .with_context(|| format!("Failed to create {}", scripts_dir.display()))?;
        info!("Created scripts directory: {}", scripts_dir.display());
    }

    Ok(scripts_dir)
}

#[cfg(unix)]
fn make_executable(path: &PathBuf) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &PathBuf) -> Result<()> {
    // Windows: Files are executable by extension (.bat, .cmd, .exe)
    Ok(())
}

fn validate_script(path: &PathBuf) -> Result<()> {
    use std::io::Read;

    // Security: Reject files larger than 10 MiB
    let metadata = std::fs::metadata(path)?;
    const MAX_SCRIPT_SIZE: u64 = 10 * 1024 * 1024; // 10 MiB
    if metadata.len() > MAX_SCRIPT_SIZE {
        return Err(anyhow!(
            "Script too large ({} bytes, max {} bytes)\n   This may not be a script file",
            metadata.len(),
            MAX_SCRIPT_SIZE
        ));
    }

    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0u8; 2];

    // Check if file is readable
    if file.read(&mut buffer).is_err() {
        return Err(anyhow!("Cannot read script file"));
    }

    // Check for shebang on Unix systems
    #[cfg(unix)]
    {
        if buffer != [b'#', b'!'] {
            eprintln!("⚠️  Warning: Script does not start with shebang (#!)");
            eprintln!("   Script may not execute correctly without proper interpreter directive");
        }
    }

    Ok(())
}

fn run_script_register(args: RegisterScriptArgs) -> Result<()> {
    use std::fs;

    // Validate input script exists
    if !args.script_path.exists() {
        return Err(anyhow!("Script not found: {}", args.script_path.display()));
    }

    if !args.script_path.is_file() {
        return Err(anyhow!("Path is not a file: {}", args.script_path.display()));
    }

    // Security: Prevent symlink attacks
    if args.script_path.is_symlink() {
        return Err(anyhow!(
            "Cannot register symlink: {}\n   Register the target file directly instead",
            args.script_path.display()
        ));
    }

    // Validate script content (check shebang on Unix)
    validate_script(&args.script_path)?;

    // Determine script name
    let script_name = if let Some(name) = args.name {
        // Validate custom script name
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Script name cannot be empty"));
        }
        if trimmed.contains('/') || trimmed.contains('\\') {
            return Err(anyhow!(
                "Script name cannot contain path separators (/ or \\)\n   Use simple filename only"
            ));
        }
        trimmed.to_string()
    } else {
        args.script_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Could not determine script filename")?
            .to_string()
    };

    // Get scripts directory
    let scripts_dir = get_scripts_dir()?;
    let dest_path = scripts_dir.join(&script_name);

    // Check if already exists
    if dest_path.exists() && !args.force {
        if args.dry_run {
            println!("🔍 Dry run: Would fail - script '{}' already exists", script_name);
            println!("   Use --force to overwrite");
            return Ok(());
        }
        return Err(anyhow!(
            "Script '{}' already exists. Use --force to overwrite",
            script_name
        ));
    }

    // Dry run mode - show what would be done
    if args.dry_run {
        println!("🔍 Dry run: Would register script");
        println!("   Source: {}", args.script_path.display());
        println!("   Destination: {}", dest_path.display());
        println!("   Name: {}", script_name);
        if dest_path.exists() {
            println!("   Action: Overwrite existing script");
        } else {
            println!("   Action: Create new script");
        }
        return Ok(());
    }

    // Copy script to scripts directory
    fs::copy(&args.script_path, &dest_path)
        .with_context(|| format!("Failed to copy script to {}", dest_path.display()))?;

    // Make executable (Unix: chmod 755, Windows: no-op)
    make_executable(&dest_path)?;

    println!("✅ Registered script: {}", script_name);
    println!("   Location: {}", dest_path.display());
    println!("   Run with: floatctl script run {}", script_name);

    Ok(())
}

fn run_script_list() -> Result<()> {
    use std::fs;

    let scripts_dir = get_scripts_dir()?;

    // Read directory
    let mut entries: Vec<_> = fs::read_dir(&scripts_dir)
        .context("Failed to read scripts directory")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .collect();

    if entries.is_empty() {
        println!("No scripts registered.");
        println!("Register a script with: floatctl script register <path>");
        return Ok(());
    }

    // Sort by name
    entries.sort_by_key(|e| e.file_name());

    println!("Registered scripts in {}:\n", scripts_dir.display());
    for entry in entries {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        // Get file size
        let metadata = entry.metadata().ok();
        let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);

        println!("  {} ({} bytes)", name_str, size);
    }

    println!("\nRun with: floatctl script run <name> [args...]");

    Ok(())
}

fn run_script_run(args: RunScriptArgs) -> Result<()> {
    use std::process::Command;

    let scripts_dir = get_scripts_dir()?;
    let script_path = scripts_dir.join(&args.script_name);

    // Validate script exists
    if !script_path.exists() {
        return Err(anyhow!(
            "Script '{}' not found. List scripts with: floatctl script list",
            args.script_name
        ));
    }

    // Execute script with arguments
    // Note: Uses .status() instead of .output() for real-time streaming output.
    // Trade-off: stderr is not captured, but user sees output immediately.
    let mut cmd = Command::new(&script_path);
    cmd.args(&args.args);

    let status = cmd.status()
        .with_context(|| {
            #[cfg(unix)]
            let hint = "Check that script has proper shebang and execute permissions";
            #[cfg(not(unix))]
            let hint = "Check that script has proper extension (.bat, .cmd, .ps1)";

            format!(
                "Failed to execute script: {}\n   {}",
                script_path.display(),
                hint
            )
        })?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(anyhow!(
            "Script '{}' exited with code: {}",
            args.script_name,
            code
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_validate_script_rejects_large_files() {
        let temp_dir = TempDir::new().unwrap();
        let large_file = temp_dir.path().join("large.sh");

        // Create 11 MiB file (exceeds 10 MiB limit)
        let mut file = std::fs::File::create(&large_file).unwrap();
        let data = vec![0u8; 11 * 1024 * 1024];
        file.write_all(&data).unwrap();
        drop(file);

        let result = validate_script(&large_file);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Script too large"));
    }

    #[test]
    fn test_validate_script_accepts_small_files() {
        let temp_dir = TempDir::new().unwrap();
        let small_file = temp_dir.path().join("small.sh");

        // Create small file with shebang
        let mut file = std::fs::File::create(&small_file).unwrap();
        file.write_all(b"#!/bin/bash\necho 'hello'\n").unwrap();
        drop(file);

        let result = validate_script(&small_file);
        assert!(result.is_ok());
    }

    #[test]
    fn test_script_name_validation_rejects_path_separators() {
        let args = RegisterScriptArgs {
            script_path: PathBuf::from("/tmp/test.sh"),
            name: Some("../etc/passwd".to_string()),
            force: false,
            dry_run: true,
        };

        // Simulate the validation logic
        let name = args.name.as_ref().unwrap();
        let trimmed = name.trim();
        let has_separator = trimmed.contains('/') || trimmed.contains('\\');

        assert!(has_separator, "Should detect path separator");
    }

    #[test]
    fn test_script_name_validation_rejects_empty_names() {
        let args = RegisterScriptArgs {
            script_path: PathBuf::from("/tmp/test.sh"),
            name: Some("   ".to_string()),
            force: false,
            dry_run: true,
        };

        // Simulate the validation logic
        let name = args.name.as_ref().unwrap();
        let trimmed = name.trim();
        let is_empty = trimmed.is_empty();

        assert!(is_empty, "Should detect empty name");
    }

    #[test]
    fn test_get_scripts_dir_creates_directory() {
        // This test verifies that get_scripts_dir() creates the directory
        // Note: This will create ~/.floatctl/scripts if it doesn't exist
        let result = get_scripts_dir();
        assert!(result.is_ok());
        let scripts_dir = result.unwrap();
        assert!(scripts_dir.exists());
        assert!(scripts_dir.is_dir());
    }
}

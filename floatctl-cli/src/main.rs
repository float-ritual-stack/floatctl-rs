use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use floatctl_core::pipeline::{split_file, SplitOptions};
use floatctl_core::{cmd_ndjson, explode_messages, explode_ndjson_parallel};
use tracing::info;
use tracing_subscriber::EnvFilter;

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
}

#[derive(Parser, Debug)]
struct EvnaArgs {
    #[command(subcommand)]
    command: EvnaCommands,
}

#[derive(Subcommand, Debug)]
enum EvnaCommands {
    /// Install evna-next as MCP server in Claude Desktop
    Install(EvnaInstallArgs),
    /// Uninstall evna-next MCP server from Claude Desktop
    Uninstall,
    /// Show evna-next MCP server status
    Status,
}

#[derive(Parser, Debug)]
struct EvnaInstallArgs {
    /// Path to evna-next directory (defaults to ../evna-next relative to floatctl-rs)
    #[arg(long)]
    path: Option<PathBuf>,

    /// Force reinstall even if already configured
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
    }
    Ok(())
}

async fn evna_install(args: EvnaInstallArgs) -> Result<()> {
    use serde_json::{json, Value};
    use std::fs;

    // Determine evna-next path
    let evna_path = if let Some(path) = args.path {
        path
    } else {
        // Default: ../evna-next relative to floatctl-rs
        let current_dir = std::env::current_dir()?;
        current_dir.parent()
            .context("Cannot determine parent directory")?
            .join("evna-next")
    };

    // Validate evna-next directory exists
    if !evna_path.exists() {
        return Err(anyhow!(
            "evna-next directory not found at: {}\nUse --path to specify location",
            evna_path.display()
        ));
    }

    // Check for package.json to confirm it's evna-next
    let package_json = evna_path.join("package.json");
    if !package_json.exists() {
        return Err(anyhow!(
            "Not a valid evna-next directory (missing package.json): {}",
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

    // Check if evna-next is already configured
    if let Some(mcp_servers) = config.get("mcpServers") {
        if let Some(evna) = mcp_servers.get("evna-next") {
            if !args.force {
                println!("✅ evna-next is already configured in Claude Desktop");
                println!("   Config: {}", serde_json::to_string_pretty(&evna)?);
                println!("\nUse --force to reinstall");
                return Ok(());
            } else {
                println!("🔄 Reinstalling evna-next (--force specified)");
            }
        }
    }

    // Get absolute path for config
    let evna_path_absolute = evna_path.canonicalize()
        .context("Failed to resolve evna-next absolute path")?;

    // Create evna-next MCP server configuration
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

    // Add evna-next configuration
    config["mcpServers"]["evna-next"] = evna_config;

    // Write config back
    let config_dir = config_path.parent().unwrap();
    fs::create_dir_all(config_dir)?;

    let config_json = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, config_json)
        .context("Failed to write Claude Desktop config")?;

    println!("✅ Successfully installed evna-next MCP server!");
    println!("   Location: {}", evna_path_absolute.display());
    println!("   Config: {}", config_path.display());
    println!("\n📝 Next steps:");
    println!("   1. Ensure .env is configured in evna-next directory");
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

    // Check if evna-next exists
    if let Some(mcp_servers) = config.get_mut("mcpServers") {
        if let Some(obj) = mcp_servers.as_object_mut() {
            if obj.remove("evna-next").is_some() {
                let config_json = serde_json::to_string_pretty(&config)?;
                fs::write(&config_path, config_json)?;
                println!("✅ Successfully uninstalled evna-next MCP server");
                println!("   Restart Claude Desktop to apply changes");
                return Ok(());
            }
        }
    }

    println!("ℹ️  evna-next is not configured - nothing to uninstall");
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

    // Check if evna-next is configured
    if let Some(mcp_servers) = config.get("mcpServers") {
        if let Some(evna) = mcp_servers.get("evna-next") {
            println!("✅ evna-next MCP server is configured");
            println!("\n📋 Configuration:");
            println!("{}", serde_json::to_string_pretty(&evna)?);

            // Validate the path exists
            if let Some(cwd) = evna.get("cwd").and_then(|v| v.as_str()) {
                let evna_path = PathBuf::from(cwd);
                if evna_path.exists() {
                    println!("\n✅ evna-next directory exists: {}", evna_path.display());

                    // Check for .env file
                    let env_file = evna_path.join(".env");
                    if env_file.exists() {
                        println!("✅ .env file found");
                    } else {
                        println!("⚠️  .env file not found - configure before using");
                    }
                } else {
                    println!("\n❌ evna-next directory not found: {}", evna_path.display());
                }
            }

            return Ok(());
        }
    }

    println!("❌ evna-next is not configured");
    println!("   Run: floatctl evna install");
    Ok(())
}

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
mod ui;

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
    /// Suppress progress spinners and bars (for LLM/script consumption)
    #[arg(long, short = 'q', global = true)]
    quiet: bool,

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
    Evna(commands::evna::EvnaArgs),
    /// Ask questions (cognitive query alias - use `ask evna` for evna queries)
    Ask(commands::ask::AskArgs),
    /// R2 sync daemon management (status, trigger, start, stop, logs)
    Sync(sync::SyncArgs),
    /// Bridge maintenance operations (index annotations, analyze, etc.)
    Bridge(commands::bridge::BridgeArgs),
    /// Query Claude Code session logs (for evna integration)
    Claude(commands::claude::ClaudeArgs),
    /// BBS bulletin board operations (inbox, send, memory, board)
    Bbs(commands::bbs::BbsArgs),
    /// Generate shell completion scripts
    Completions(CompletionsArgs),
    /// Manage floatctl configuration (init, get, set, list, validate)
    Config(config::ConfigArgs),
    /// System diagnostics and maintenance
    System(commands::system::SystemArgs),
    /// Manage registered shell scripts (register, list, run)
    Script(commands::script::ScriptArgs),
    /// Capture context markers to local queue (syncs to float-box)
    Ctx(commands::ctx::CtxArgs),
    /// Run HTTP API server (BBS routes, dispatch capture, etc.)
    #[cfg(feature = "server")]
    Serve(commands::serve::ServeArgs),
    /// Search via Cloudflare AI Search with FloatQL pattern recognition
    Search(floatctl_search::SearchArgs),
    /// Manage system-wide status broadcast (focus, notices - shown in evna tool descriptions)
    Status(commands::status::StatusArgs),
}

#[derive(Parser, Debug)]
struct CompletionsArgs {
    /// Shell to generate completions for
    #[arg(value_enum)]
    shell: Shell,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
#[allow(clippy::enum_variant_names)] // PowerShell is a proper noun, not a suffix
enum Shell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
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

    // Initialize UI quiet mode from flag, env var, and TTY detection
    ui::init_quiet_mode(cli.quiet);

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
        Commands::Evna(args) => commands::run_evna(args).await?,
        Commands::Ask(args) => commands::run_ask(args).await?,
        Commands::Sync(args) => sync::run_sync(args).await?,
        Commands::Bridge(args) => commands::run_bridge(args)?,
        Commands::Claude(args) => commands::run_claude(args)?,
        Commands::Bbs(args) => commands::run_bbs(args).await?,
        Commands::Completions(args) => run_completions(args)?,
        Commands::Config(args) => config::run_config(args)?,
        Commands::System(args) => commands::run_system(args)?,
        Commands::Script(args) => commands::run_script(args)?,
        Commands::Ctx(args) => commands::run_ctx(args)?,
        #[cfg(feature = "server")]
        Commands::Serve(args) => commands::run_serve(args).await?,
        Commands::Search(args) => floatctl_search::run_search(args).await?,
        Commands::Status(args) => commands::run_status(args)?,
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



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
//!
//! ## Dual-Mode Architecture
//!
//! floatctl supports two interaction patterns:
//!
//! ### Human Mode (Interactive Wizard)
//! When required arguments are missing and stdin is a TTY, floatctl launches
//! an interactive wizard using `inquire` to guide the user through the options.
//!
//! ### Agent Mode (Machine Protocol)
//! When `--json` is passed, all output is wrapped in a standard JSON envelope:
//! ```json
//! { "status": "success"|"error", "data": {...}, "error": {...} }
//! ```
//!
//! The `floatctl reflect` command outputs the full CLI schema for agent introspection.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, CommandFactory, Parser, Subcommand, ValueEnum};
use floatctl_core::pipeline::{split_file, SplitOptions};
use floatctl_core::{cmd_ndjson, explode_messages, explode_ndjson_parallel};
use tracing::info;

mod commands;
mod config;
pub mod protocol;
pub mod reflect;
mod sync;
mod tracing_setup;
mod ui;
pub mod wizard;

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
                  generate embeddings, and search semantically across your conversation history.\n\n\
                  DUAL-MODE: Use --json for agent/machine consumption (structured JSON output). \
                  Without flags, interactive wizards guide you through missing arguments."
)]
struct Cli {
    /// Suppress progress spinners and bars (for LLM/script consumption)
    #[arg(long, short = 'q', global = true)]
    quiet: bool,

    /// Enable debug logging (sets RUST_LOG=debug)
    #[arg(long, global = true)]
    debug: bool,

    /// Export traces to OpenTelemetry OTLP endpoint (requires --features telemetry)
    #[arg(long, global = true)]
    otel: bool,

    /// Output JSON envelope for agent/machine consumption (no interactive prompts)
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Commands>,
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
    /// Output CLI schema in JSON for agent introspection (read the manual programmatically)
    Reflect(ReflectArgs),
}

#[derive(Parser, Debug)]
struct ReflectArgs {
    /// Output only a specific command's schema
    #[arg(long)]
    command: Option<String>,

    /// Include hidden commands and arguments
    #[arg(long)]
    include_hidden: bool,

    /// Compact output (no pretty printing)
    #[arg(long)]
    compact: bool,
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

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize tracing with debug/otel options
    let tracing_config = tracing_setup::TracingConfig {
        debug: cli.debug,
        otel: cli.otel,
    };
    tracing_setup::init(&tracing_config).ok();

    // Initialize UI quiet mode and JSON protocol mode
    ui::init_quiet_mode(cli.quiet || cli.json);
    protocol::init_json_mode(cli.json);

    // Handle no command - show help or interactive menu
    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            if wizard::can_use_wizard() {
                // Interactive mode: show command picker
                return run_interactive_menu().await;
            } else {
                // Non-interactive: show help
                Cli::command().print_help()?;
                return Ok(());
            }
        }
    };

    // Execute command with error handling wrapper
    let result = execute_command(command).await;

    // Handle result based on mode
    let final_result = match result {
        Ok(()) => {
            if protocol::is_json_mode() {
                // Success with no specific data - output generic success
                // (most commands output their own JSON already)
            }
            Ok(())
        }
        Err(err) => {
            if protocol::is_json_mode() {
                // Print structured JSON error
                protocol::map_error(&err).print();

                // Flush any pending OpenTelemetry traces before exit
                tracing_setup::shutdown_otel();

                // Exit with non-zero code so CI/callers see failure
                // We print JSON error above, then exit explicitly to avoid
                // clap's unstructured error output while still signaling failure
                std::process::exit(1);
            } else {
                Err(err)
            }
        }
    };

    // Flush any pending OpenTelemetry traces before exit
    tracing_setup::shutdown_otel();

    final_result
}

/// Execute a command (the main dispatch logic)
async fn execute_command(command: Commands) -> Result<()> {
    match command {
        Commands::Split(args) => run_split(args).await,
        Commands::Ndjson(args) => run_ndjson(args),
        Commands::Explode(args) => run_explode(args),
        Commands::FullExtract(args) => run_full_extract(args).await,
        #[cfg(feature = "embed")]
        Commands::Embed(args) => floatctl_embed::run_embed(args).await,
        #[cfg(feature = "embed")]
        Commands::EmbedNotes(args) => floatctl_embed::run_embed_notes(args).await,
        #[cfg(feature = "embed")]
        Commands::Query(cmd) => run_query(cmd).await,
        Commands::Evna(args) => commands::run_evna(args).await,
        Commands::Ask(args) => commands::run_ask(args).await,
        Commands::Sync(args) => sync::run_sync(args).await,
        Commands::Bridge(args) => commands::run_bridge(args),
        Commands::Claude(args) => commands::run_claude(args),
        Commands::Bbs(args) => commands::run_bbs(args).await,
        Commands::Completions(args) => run_completions(args),
        Commands::Config(args) => config::run_config(args),
        Commands::System(args) => commands::run_system(args),
        Commands::Script(args) => commands::run_script(args),
        Commands::Ctx(args) => commands::run_ctx(args),
        #[cfg(feature = "server")]
        Commands::Serve(args) => commands::run_serve(args).await,
        Commands::Search(args) => floatctl_search::run_search(args).await,
        Commands::Status(args) => commands::run_status(args),
        Commands::Reflect(args) => run_reflect(args),
    }
}

/// Interactive menu when no command is specified
async fn run_interactive_menu() -> Result<()> {
    use inquire::Select;

    println!("\n🚀 floatctl - Conversation Archive Processor\n");

    let commands = vec![
        "full-extract  - Extract and organize conversation exports",
        "search        - Search conversations (AI-powered)",
        "query         - Semantic search (pgvector)",
        "bridge        - Manage bridge files",
        "bbs           - Bulletin board messaging",
        "ctx           - Capture context markers",
        "sync          - R2 sync management",
        "reflect       - Output CLI schema (for agents)",
        "help          - Show full help",
    ];

    let selection = Select::new("What would you like to do?", commands)
        .with_help_message("Use arrow keys to navigate, Enter to select")
        .prompt()
        .context("Command selection cancelled")?;

    let cmd_name = selection.split_whitespace().next().unwrap_or("help");

    match cmd_name {
        "full-extract" => {
            let wizard_result = wizard::wizard_full_extract()?;
            wizard::print_equivalent_command(
                "full-extract",
                &[
                    ("in", &wizard_result.input),
                    ("out", &wizard_result.output),
                    ("format", &wizard_result.formats),
                    ("dry-run", if wizard_result.dry_run { "true" } else { "" }),
                    (
                        "keep-ndjson",
                        if wizard_result.keep_ndjson { "true" } else { "" },
                    ),
                ],
            );

            // Execute the command
            let args = FullExtractArgs {
                input: PathBuf::from(&wizard_result.input),
                output: Some(PathBuf::from(&wizard_result.output)),
                format: parse_formats(&wizard_result.formats),
                dry_run: wizard_result.dry_run,
                no_progress: false,
                keep_ndjson: wizard_result.keep_ndjson,
            };
            run_full_extract(args).await
        }
        "search" => {
            let wizard_result = wizard::wizard_search()?;

            if wizard_result.use_autorag {
                wizard::print_equivalent_command(
                    "search",
                    &[
                        ("query", &wizard_result.query),
                        ("folder", wizard_result.project.as_deref().unwrap_or("")),
                    ],
                );

                // Direct execution with minimal args
                let args = floatctl_search::SearchArgs {
                    query: Some(wizard_result.query),
                    rag: "sysops-beta".to_string(),
                    max_results: wizard_result.limit,
                    threshold: 0.3,
                    folder: wizard_result.project,
                    format: floatctl_search::OutputFormat::default(),
                    raw: false,
                    no_rewrite: false,
                    no_rerank: false,
                    model: "@cf/meta/llama-3.3-70b-instruct-fp8-fast".to_string(),
                    rerank_model: "@cf/baai/bge-reranker-base".to_string(),
                    system_prompt: None,
                    parse_only: false,
                    no_parse: false,
                    quiet: false,
                };
                floatctl_search::run_search(args).await
            } else {
                wizard::print_equivalent_command(
                    "query all",
                    &[
                        ("query", &wizard_result.query),
                        ("limit", &wizard_result.limit.to_string()),
                        ("project", wizard_result.project.as_deref().unwrap_or("")),
                    ],
                );

                #[cfg(feature = "embed")]
                {
                    let args = floatctl_embed::QueryArgs {
                        query: wizard_result.query,
                        mode: floatctl_embed::QueryMode::Semantic,
                        project: wizard_result.project,
                        limit: Some(wizard_result.limit as i64),
                        days: None,
                        threshold: None,
                        json: false,
                    };
                    floatctl_embed::run_query(args, floatctl_embed::QueryTable::All).await
                }
                #[cfg(not(feature = "embed"))]
                {
                    anyhow::bail!("Embed feature not enabled. Use 'floatctl search' instead.");
                }
            }
        }
        "reflect" => run_reflect(ReflectArgs {
            command: None,
            include_hidden: false,
            compact: false,
        }),
        "help" => {
            Cli::command().print_help()?;
            Ok(())
        }
        _ => {
            println!("Command '{}' wizard not yet implemented.", cmd_name);
            println!("Run: floatctl {} --help", cmd_name);
            Ok(())
        }
    }
}

/// Parse format string into SplitFormat vec
fn parse_formats(formats: &str) -> Vec<SplitFormat> {
    formats
        .split(',')
        .filter_map(|f| match f.trim().to_lowercase().as_str() {
            "md" | "markdown" => Some(SplitFormat::Md),
            "json" => Some(SplitFormat::Json),
            "ndjson" => Some(SplitFormat::Ndjson),
            _ => None,
        })
        .collect()
}

/// Run the reflect command - output CLI schema
fn run_reflect(args: ReflectArgs) -> Result<()> {
    let cmd = Cli::command();
    let schema = reflect::extract_schema(&cmd);

    // Filter to specific command if requested
    let output = if let Some(ref cmd_name) = args.command {
        // Find the specific command
        let found = schema
            .commands
            .iter()
            .find(|c| c.name == *cmd_name)
            .cloned();

        match found {
            Some(cmd_schema) => serde_json::to_value(&cmd_schema)?,
            None => {
                return Err(anyhow!(
                    "Command '{}' not found. Available: {}",
                    cmd_name,
                    schema
                        .commands
                        .iter()
                        .map(|c| c.name.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    } else {
        serde_json::to_value(&schema)?
    };

    // Output
    let json_str = if args.compact {
        serde_json::to_string(&output)?
    } else {
        serde_json::to_string_pretty(&output)?
    };

    println!("{}", json_str);
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



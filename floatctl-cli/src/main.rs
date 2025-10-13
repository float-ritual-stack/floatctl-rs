use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use floatctl_core::pipeline::{split_file, SplitOptions};
use floatctl_core::{cmd_ndjson, explode_messages, explode_ndjson_parallel};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "floatctl",
    author,
    version,
    about = "Split LLM exports, embed messages, and query them semantically."
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
    Query(floatctl_embed::QueryArgs),
}

#[derive(Parser, Debug)]
struct SplitArgs {
    #[arg(
        long = "in",
        value_name = "PATH",
        default_value = "conversations.ndjson"
    )]
    input: PathBuf,

    #[arg(long = "out", value_name = "DIR", default_value = "conv_out")]
    output: PathBuf,

    #[arg(long, value_delimiter = ',', default_value = "md,json,ndjson")]
    format: Vec<SplitFormat>,

    #[arg(long = "dry-run")]
    dry_run: bool,

    /// Disable the real-time progress bar output.
    #[arg(long = "no-progress", action = ArgAction::SetTrue)]
    no_progress: bool,
}

#[derive(Parser, Debug)]
struct NdjsonArgs {
    #[arg(long = "in", value_name = "PATH")]
    input: PathBuf,

    #[arg(long = "out", value_name = "PATH")]
    output: Option<PathBuf>,

    #[arg(long, help = "Pretty-print JSON output (canonical formatting)")]
    canonical: bool,
}

#[derive(Parser, Debug)]
struct ExplodeArgs {
    #[arg(long = "in", value_name = "PATH")]
    input: PathBuf,

    #[arg(long = "out", value_name = "DIR")]
    output: Option<PathBuf>,

    #[arg(long, help = "Extract messages instead of conversations")]
    messages: bool,
}

#[derive(Parser, Debug)]
struct FullExtractArgs {
    #[arg(long = "in", value_name = "PATH")]
    input: PathBuf,

    #[arg(long = "out", value_name = "DIR", default_value = "conv_out")]
    output: PathBuf,

    #[arg(long, value_delimiter = ',', default_value = "md,json,ndjson")]
    format: Vec<SplitFormat>,

    #[arg(long = "dry-run")]
    dry_run: bool,

    #[arg(long = "no-progress", action = ArgAction::SetTrue)]
    no_progress: bool,

    #[arg(long, help = "Keep intermediate NDJSON file after extraction")]
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
        Commands::Query(args) => floatctl_embed::run_query(args).await?,
    }
    Ok(())
}

async fn run_split(args: SplitArgs) -> Result<()> {
    let mut opts = SplitOptions {
        output_dir: args.output.clone(),
        dry_run: args.dry_run,
        show_progress: !args.no_progress,
        ..Default::default()
    };

    opts.emit_markdown = args.format.contains(&SplitFormat::Md);
    opts.emit_json = args.format.contains(&SplitFormat::Json);
    opts.emit_ndjson = args.format.contains(&SplitFormat::Ndjson);

    info!(
        "splitting export {:?} -> {:?} (formats: {:?})",
        args.input, args.output, args.format
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
        let output_dir = args
            .output
            .unwrap_or_else(|| PathBuf::from("./conversations"));
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

    let mut opts = SplitOptions {
        output_dir: args.output.clone(),
        dry_run: args.dry_run,
        show_progress: !args.no_progress,
        ..Default::default()
    };

    opts.emit_markdown = args.format.contains(&SplitFormat::Md);
    opts.emit_json = args.format.contains(&SplitFormat::Json);
    opts.emit_ndjson = args.format.contains(&SplitFormat::Ndjson);

    info!(
        "full extraction workflow: {:?} -> {:?} (formats: {:?})",
        args.input, args.output, args.format
    );

    cmd_full_extract(&args.input, opts, args.keep_ndjson)
        .await
        .context("failed to run full extraction workflow")?;

    Ok(())
}

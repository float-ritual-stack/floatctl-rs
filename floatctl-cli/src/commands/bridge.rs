//! Bridge file management commands
//!
//! Commands: index, append

use anyhow::{anyhow, Context, Result};
use clap::{ArgAction, Parser, Subcommand};
use std::path::PathBuf;
use tracing::info;

// === Arg Structs (moved from main.rs for high cohesion) ===

#[derive(Parser, Debug)]
pub struct BridgeArgs {
    #[command(subcommand)]
    pub command: BridgeCommands,
}

#[derive(Subcommand, Debug)]
pub enum BridgeCommands {
    /// Index :: annotations from markdown files to create bridge stubs
    Index(IndexArgs),
    /// Append conversation content to bridge files
    Append(AppendArgs),
}

#[derive(Parser, Debug)]
pub struct IndexArgs {
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
pub struct AppendArgs {
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

// === Command Implementations ===

pub fn run_bridge(args: BridgeArgs) -> Result<()> {
    match args.command {
        BridgeCommands::Index(index_args) => run_bridge_index(index_args),
        BridgeCommands::Append(append_args) => run_bridge_append(append_args),
    }
}

fn run_bridge_index(args: IndexArgs) -> Result<()> {
    use floatctl_bridge::{index_directory, index_file};
    use floatctl_core::FloatConfig;

    // Get bridges output directory
    let bridges_dir = if let Some(path) = args.output {
        path
    } else {
        // Try centralized config first, fall back to default
        FloatConfig::load()
            .ok()
            .map(|c| c.paths.bridges)
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

    // Get bridges output directory
    let bridges_dir = if let Some(path) = args.out {
        path
    } else {
        // Try centralized config first, fall back to default
        FloatConfig::load()
            .ok()
            .map(|c| c.paths.bridges)
            .unwrap_or_else(|| {
                let home = dirs::home_dir().expect("Could not determine home directory");
                home.join("float-hub")
                    .join("float.dispatch")
                    .join("bridges")
            })
    };


    // Get content from specified source
    let mut content = if args.from_stdin {
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

    // If content looks like JSON (from hook), try to extract the prompt field
    if content.trim_start().starts_with('{') {
        if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(prompt) = json_value.get("prompt").and_then(|p| p.as_str()) {
                content = prompt.to_string();
            }
        }
    }

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

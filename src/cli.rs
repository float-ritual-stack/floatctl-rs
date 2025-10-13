use std::path::PathBuf;

use clap::{ArgAction, Parser, ValueEnum};

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum CliFormat {
    Md,
    Json,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum DateFrom {
    Utc,
    Local,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, ValueEnum)]
pub enum SourceSelector {
    Auto,
    Chatgpt,
    Anthropic,
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "Split LLM conversation exports into Markdown/JSON archives."
)]
pub struct Cli {
    /// Path to the conversations export JSON or ZIP archive.
    #[arg(long = "in", value_name = "PATH", default_value = "conversations.json")]
    pub input: PathBuf,

    /// Output directory for emitted conversations.
    #[arg(long = "out", value_name = "DIR")]
    pub output: Option<PathBuf>,

    /// Output formats (comma separated): md, json.
    #[arg(long, value_delimiter = ',')]
    pub format: Option<Vec<CliFormat>>,

    /// Only include conversations created on/after this date (YYYY-MM-DD).
    #[arg(long)]
    pub since: Option<String>,

    /// Only include conversations created on/before this date (YYYY-MM-DD).
    #[arg(long)]
    pub until: Option<String>,

    /// Timezone to present timestamps in meta data (e.g., America/Toronto).
    #[arg(long = "tz")]
    pub timezone: Option<String>,

    /// Use UTC or local time for filename date prefix.
    #[arg(long = "date-from", value_enum)]
    pub date_from: Option<DateFrom>,

    /// Force a specific parser for the input source.
    #[arg(long = "source", value_enum)]
    pub source: Option<SourceSelector>,

    /// Re-emit all conversations ignoring state tracking.
    #[arg(long, action = ArgAction::SetTrue)]
    pub force: bool,

    /// Preview what would be processed without writing files.
    #[arg(long = "dry-run", action = ArgAction::SetTrue)]
    pub dry_run: bool,

    /// Path to the TOML configuration file.
    #[arg(long)]
    pub config: Option<PathBuf>,
}

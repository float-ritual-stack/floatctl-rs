//! Ask command - top-level alias for cognitive queries
//!
//! Provides `floatctl ask evna` as an alternative to `floatctl evna ask`
//! to match the mental model of "ask evna [question]"

use anyhow::Result;
use clap::{Parser, Subcommand};

use super::evna::{evna_ask, EvnaAskArgs};

#[derive(Parser, Debug)]
pub struct AskArgs {
    #[command(subcommand)]
    pub command: AskCommands,
}

#[derive(Subcommand, Debug)]
pub enum AskCommands {
    /// Ask evna a question (LLM-orchestrated multi-tool search)
    Evna(EvnaAskArgs),
}

/// Run the ask command dispatcher
pub async fn run_ask(args: AskArgs) -> Result<()> {
    match args.command {
        AskCommands::Evna(evna_args) => evna_ask(evna_args).await,
    }
}

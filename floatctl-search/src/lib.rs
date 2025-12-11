//! floatctl-search - FloatQL parser and Cloudflare AutoRAG search
//!
//! This crate provides:
//! - FloatQL parser for natural language pattern extraction
//! - Cloudflare AutoRAG (AI Search) client
//! - CLI argument handling for `floatctl search` subcommand
//!
//! ## Architecture
//!
//! ```text
//! Input (query/stdin) → FloatQL Parser → AutoRAG Client → Output
//!                              ↓
//!                     ParsedQuery {
//!                       text_terms,
//!                       float_patterns,
//!                       wikilinks,
//!                       commands,
//!                       directives
//!                     }
//! ```

pub mod autorag;
pub mod parser;

use std::io::IsTerminal;
use std::time::Duration;

use anyhow::Result;
use clap::Args;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::instrument;

pub use autorag::{AutoRAGClient, AiSearchResponse, SearchOptions, SearchResult};
pub use parser::{FloatQLParser, ParsedQuery, TemporalFilter};

/// Search subcommand arguments
#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Search query (reads from stdin if not provided)
    #[arg(value_name = "QUERY")]
    pub query: Option<String>,

    /// RAG instance to search (default: sysops-beta)
    #[arg(long, default_value = "sysops-beta")]
    pub rag: String,

    /// Maximum results to return
    #[arg(short = 'n', long, default_value = "10")]
    pub max_results: usize,

    /// Score threshold (0.0-1.0)
    #[arg(long, default_value = "0.3")]
    pub threshold: f64,

    /// Folder filter (e.g., "bridges/", "dispatch/")
    #[arg(long)]
    pub folder: Option<String>,

    /// Output format (text, json, inline)
    #[arg(long, short = 'f', default_value = "text")]
    pub format: OutputFormat,

    /// Search only mode (no LLM synthesis)
    #[arg(long)]
    pub raw: bool,

    /// Disable query rewriting
    #[arg(long)]
    pub no_rewrite: bool,

    /// Disable BGE reranking
    #[arg(long)]
    pub no_rerank: bool,

    /// Model for AI search synthesis (llama-3.3-70b-instruct-fp8-fast, llama-4-scout-17b-16e-instruct, qwen3-30b-a3b-fp8)
    #[arg(long, default_value = "@cf/meta/llama-3.3-70b-instruct-fp8-fast")]
    pub model: String,

    /// Model for reranking results (default: @cf/baai/bge-reranker-base)
    #[arg(long, default_value = "@cf/baai/bge-reranker-base")]
    pub rerank_model: String,

    /// System prompt for generating the answer
    #[arg(long)]
    pub system_prompt: Option<String>,

    /// Show parsed FloatQL patterns without searching
    #[arg(long)]
    pub parse_only: bool,

    /// Bypass FloatQL parsing - send query directly to AutoRAG
    /// Useful for debugging: isolate "is it the prompt or FloatQL?"
    #[arg(long)]
    pub no_parse: bool,

    /// Suppress progress spinner (for LLM/script consumption)
    #[arg(long, short = 'q')]
    pub quiet: bool,
}

/// Helper to create a spinner (respects quiet mode and TTY)
fn spinner(msg: &str, quiet: bool) -> Option<ProgressBar> {
    if quiet || !std::io::stderr().is_terminal() {
        return None;
    }
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    pb.set_message(msg.to_string());
    pb.enable_steady_tick(Duration::from_millis(80));
    Some(pb)
}

/// Output format options
#[derive(Debug, Clone, clap::ValueEnum, Default)]
pub enum OutputFormat {
    /// Human-readable markdown
    #[default]
    Text,
    /// JSON for machine consumption
    Json,
    /// Inline text for piping
    Inline,
}

/// Execute the search command
#[instrument(skip_all, fields(rag = %args.rag, raw = args.raw, parse_only = args.parse_only))]
pub async fn run_search(args: SearchArgs) -> Result<()> {
    // Load .env files (floatctl standard locations)
    if let Some(home) = dirs::home_dir() {
        let _ = dotenvy::from_path(home.join(".floatctl/.env"));
    }
    let _ = dotenvy::dotenv(); // Also check cwd

    // Get query from args or stdin
    let query = if let Some(q) = args.query {
        q
    } else {
        // Read from stdin
        use std::io::{self, BufRead};
        let stdin = io::stdin();
        let mut lines = Vec::new();
        for line in stdin.lock().lines() {
            lines.push(line?);
        }
        lines.join("\n")
    };

    if query.trim().is_empty() {
        anyhow::bail!("No query provided. Pass a query argument or pipe input via stdin.");
    }

    // Build search options - either via FloatQL parsing or raw passthrough
    let options = if args.no_parse {
        // Bypass FloatQL - send query directly to AutoRAG
        // Useful for debugging: isolate "is it the prompt or FloatQL?"
        SearchOptions {
            query: query.clone(),
            rag_id: args.rag,
            max_results: args.max_results,
            rewrite_query: !args.no_rewrite,
            score_threshold: args.threshold,
            enable_reranking: !args.no_rerank,
            folder_filter: args.folder,
            model: args.model,
            system_prompt: args.system_prompt,
            rerank_model: args.rerank_model,
        }
    } else {
        // Parse the query with FloatQL
        let parser = FloatQLParser::new();
        let parsed = parser.parse(&query);

        // Parse-only mode: just show what was extracted
        if args.parse_only {
            return print_parsed(&parsed, &args.format);
        }

        // Build search options from parsed query + args
        let search_terms = parser.extract_search_terms(&parsed);
        SearchOptions {
            query: search_terms,
            rag_id: args.rag,
            max_results: args.max_results,
            rewrite_query: !args.no_rewrite,
            score_threshold: args.threshold,
            enable_reranking: !args.no_rerank,
            folder_filter: args.folder.or_else(|| {
                // Auto-detect folder from patterns
                if parsed.float_patterns.contains(&"dispatch".to_string()) {
                    Some("dispatch".to_string())
                } else if parsed.float_patterns.contains(&"bridge".to_string()) {
                    Some("bridges".to_string())
                } else {
                    None
                }
            }),
            model: args.model,
            system_prompt: args.system_prompt,
            rerank_model: args.rerank_model,
        }
    };

    // Execute search with progress feedback
    let client = AutoRAGClient::from_env()?;

    if args.raw {
        // Raw search mode - no LLM synthesis
        let pb = spinner("Searching...", args.quiet);
        let results = client.search(options).await?;
        if let Some(pb) = pb {
            pb.finish_and_clear();
        }
        print_results(None, &results, &args.format)?;
    } else {
        // AI search mode - retrieval + synthesis
        let pb = spinner("Searching and synthesizing...", args.quiet);
        let response = client.ai_search(options).await?;
        if let Some(pb) = pb {
            pb.finish_and_clear();
        }
        print_results(Some(&response.answer), &response.sources, &args.format)?;
    }

    Ok(())
}

fn print_parsed(parsed: &ParsedQuery, format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            #[derive(serde::Serialize)]
            struct ParsedJson<'a> {
                text_terms: &'a [String],
                float_patterns: &'a [String],
                persona_patterns: &'a [String],
                bridge_ids: &'a [String],
                wikilinks: &'a [String],
                commands: &'a [String],
                directives: &'a [(String, Option<String>)],
                type_filters: &'a [String],
                raw_query: &'a str,
            }
            let json = ParsedJson {
                text_terms: &parsed.text_terms,
                float_patterns: &parsed.float_patterns,
                persona_patterns: &parsed.persona_patterns,
                bridge_ids: &parsed.bridge_ids,
                wikilinks: &parsed.wikilinks,
                commands: &parsed.commands,
                directives: &parsed.directives,
                type_filters: &parsed.type_filters,
                raw_query: &parsed.raw_query,
            };
            println!("{}", serde_json::to_string_pretty(&json)?);
        }
        _ => {
            println!("## FloatQL Parse Results\n");
            println!("**Text Terms**: {:?}", parsed.text_terms);
            println!("**FLOAT Patterns**: {:?}", parsed.float_patterns);
            println!("**Persona Patterns**: {:?}", parsed.persona_patterns);
            println!("**Bridge IDs**: {:?}", parsed.bridge_ids);
            println!("**Wikilinks**: {:?}", parsed.wikilinks);
            println!("**Commands**: {:?}", parsed.commands);
            println!("**Directives**: {:?}", parsed.directives);
            println!("**Type Filters**: {:?}", parsed.type_filters);
            if let Some(ref temporal) = parsed.temporal_filter {
                println!("**Temporal Filter**: {:?}", temporal);
            }
        }
    }
    Ok(())
}

fn print_results(answer: Option<&str>, sources: &[SearchResult], format: &OutputFormat) -> Result<()> {
    match format {
        OutputFormat::Json => {
            let json = AutoRAGClient::format_json(answer.unwrap_or(""), sources)?;
            println!("{}", json);
        }
        OutputFormat::Inline => {
            // Just the answer text for piping
            if let Some(ans) = answer {
                print!("{}", ans);
            } else if let Some(first) = sources.first() {
                if let Some(chunk) = first.content.first() {
                    print!("{}", chunk.text);
                }
            }
        }
        OutputFormat::Text => {
            let output = AutoRAGClient::format_results(answer.unwrap_or("(raw search)"), sources);
            println!("{}", output);
        }
    }
    Ok(())
}

//! Context capture command for queuing ctx:: messages
//!
//! Command: ctx

use anyhow::{anyhow, Context, Result};
use clap::Parser;

// === Arg Structs (moved from main.rs for high cohesion) ===

#[derive(Parser, Debug)]
pub struct CtxArgs {
    /// Message to capture (or read from stdin)
    pub message: Option<String>,
}

// === Command Implementation ===

pub fn run_ctx(args: CtxArgs) -> Result<()> {
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

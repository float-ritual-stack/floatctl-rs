use anyhow::{anyhow, Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tracing::{info, instrument};

use crate::pipeline::{split_file, SplitOptions};
use crate::stream::RawValueStream;

/// Convert conversations.json or .zip to NDJSON format (one conversation per line)
/// This is optimized for speed - streams raw JSON values without parsing into Conversation structs.
/// Uses direct to_writer() to avoid intermediate String allocations.
#[instrument(skip_all)]
pub fn cmd_ndjson(
    input: impl AsRef<Path>,
    canonical: bool,
    output: Option<impl AsRef<Path>>,
) -> Result<()> {
    let input_path = input.as_ref();

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} {pos} conversations | {msg}")
            .context("failed to create progress style")?
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.enable_steady_tick(Duration::from_millis(100));
    pb.set_message("streaming...");

    // Use RawValueStream to avoid expensive Conversation parsing
    let stream = RawValueStream::from_path(input_path)
        .with_context(|| format!("failed to open {:?}", input_path))?;

    // Setup output writer
    let mut out: Box<dyn Write> = if let Some(out_path) = output {
        let file = fs::File::create(out_path.as_ref())
            .with_context(|| format!("failed to create {:?}", out_path.as_ref()))?;
        Box::new(BufWriter::new(file))
    } else {
        Box::new(BufWriter::new(std::io::stdout()))
    };

    let mut n = 0u64;

    for (idx, result) in stream.enumerate() {
        let value = result.with_context(|| format!("failed to parse conversation #{}", idx + 1))?;

        // Write JSON directly to BufWriter - no intermediate String
        if canonical {
            serde_json::to_writer_pretty(&mut out, &value)?;
        } else {
            serde_json::to_writer(&mut out, &value)?;
        }
        out.write_all(b"\n")?;

        n += 1;

        // Update progress every 50 conversations to avoid overhead
        if n.is_multiple_of(50) {
            pb.set_position(n);
            if let Some(title) = value.get("title").or_else(|| value.get("name")).and_then(|v| v.as_str()) {
                let truncated = truncate_title(title, 40);
                pb.set_message(format!("latest: {}", truncated));
            }
        }
    }

    out.flush()?;

    pb.finish_with_message(format!("Done. {} conversations written", n));
    info!("NDJSON conversion complete: {} conversations", n);

    Ok(())
}

/// Explode NDJSON into individual conversation JSON files (parallel writes)
#[instrument(skip_all)]
pub fn explode_ndjson_parallel(
    input: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
) -> Result<()> {
    let input_path = input.as_ref();
    let out_dir = output_dir.as_ref();

    fs::create_dir_all(out_dir)
        .with_context(|| format!("failed to create directory {:?}", out_dir))?;

    let file = fs::File::open(input_path)
        .with_context(|| format!("failed to open {:?}", input_path))?;
    let reader = BufReader::new(file);

    // Read all lines into memory for parallel processing
    // For truly huge NDJSON files, we could process in batches instead
    let lines: Vec<String> = reader
        .lines()
        .collect::<Result<Vec<_>, _>>()
        .context("failed to read input lines")?;

    let pb = ProgressBar::new(lines.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} {elapsed_precise} [{bar:30.cyan/blue}] {pos}/{len} {msg}",
        )
        .context("failed to create progress style")?
        .progress_chars("█▉▊▋▌▍▎▏ "),
    );

    // Limit parallelism to avoid overwhelming the filesystem
    let threads = num_cpus::get().min(8);
    rayon::ThreadPoolBuilder::new()
        .num_threads(threads)
        .build()
        .context("failed to build thread pool")?
        .install(|| {
            lines.par_iter().for_each(|line| {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    return;
                }

                if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                    // Extract conversation ID
                    let conv_id = value
                        .get("uuid")
                        .or_else(|| value.get("id"))
                        .or_else(|| value.get("conv_id"))
                        .and_then(|v| v.as_str());

                    if let Some(id) = conv_id {
                        let filename = sanitize_filename(id);
                        let path = out_dir.join(format!("{}.json", filename));

                        if let Ok(json) = serde_json::to_vec_pretty(&value) {
                            let _ = fs::write(&path, json);
                            pb.inc(1);
                        }
                    }
                }
            });
        });

    pb.finish_with_message("All conversations written");
    info!(
        "Exploded {} conversations into {:?} (using {} threads)",
        lines.len(),
        out_dir,
        threads
    );

    Ok(())
}

/// Explode a single conversation JSON into message-level NDJSON
#[instrument(skip_all)]
pub fn explode_messages(
    conv_json: impl AsRef<Path>,
    output: Option<impl AsRef<Path>>,
) -> Result<()> {
    let input_path = conv_json.as_ref();
    let raw_bytes = fs::read(input_path)
        .with_context(|| format!("failed to read {:?}", input_path))?;

    let raw: Value = serde_json::from_slice(&raw_bytes)
        .with_context(|| format!("failed to parse JSON from {:?}", input_path))?;

    // Setup output writer
    let mut out: Box<dyn Write> = if let Some(out_path) = output {
        let file = fs::File::create(out_path.as_ref())
            .with_context(|| format!("failed to create {:?}", out_path.as_ref()))?;
        Box::new(BufWriter::new(file))
    } else {
        Box::new(BufWriter::new(std::io::stdout()))
    };

    let conv_id = raw
        .get("uuid")
        .or_else(|| raw.get("id"))
        .or_else(|| raw.get("conv_id"))
        .and_then(|v| v.as_str());

    if let Some(messages) = raw.get("messages").and_then(|m| m.as_array()) {
        for (i, msg) in messages.iter().enumerate() {
            let line = serde_json::json!({
                "conv_id": conv_id,
                "index": i,
                "role": msg.get("role"),
                "timestamp": msg.get("timestamp"),
                "content": msg.get("content"),
            });
            writeln!(out, "{}", serde_json::to_string(&line)?)?;
        }
        out.flush()?;
        info!("Exploded {} messages from conversation", messages.len());
    } else if let Some(chat_messages) = raw.get("chat_messages").and_then(|m| m.as_array()) {
        // Handle Anthropic format
        for (i, msg) in chat_messages.iter().enumerate() {
            let line = serde_json::json!({
                "conv_id": conv_id,
                "index": i,
                "role": msg.get("sender"),
                "timestamp": msg.get("created_at"),
                "content": msg.get("content"),
            });
            writeln!(out, "{}", serde_json::to_string(&line)?)?;
        }
        out.flush()?;
        info!(
            "Exploded {} messages from conversation (Anthropic format)",
            chat_messages.len()
        );
    } else {
        return Err(anyhow::anyhow!(
            "no messages found in conversation (expected 'messages' or 'chat_messages' field)"
        ));
    }

    Ok(())
}

/// Truncate a string to a maximum length, adding ellipsis if needed
fn truncate_title(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let mut result = String::new();
        for (i, ch) in s.chars().enumerate() {
            if i >= max_len - 1 {
                result.push('…');
                break;
            }
            result.push(ch);
        }
        result
    }
}

/// Sanitize a filename by replacing invalid characters
fn sanitize_filename(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c => c,
        })
        .collect()
}

/// Full extraction workflow: auto-convert to NDJSON (if needed) then split
/// This is the one-command convenience wrapper that handles the entire workflow.
#[instrument(skip_all)]
pub async fn cmd_full_extract(
    input: impl AsRef<Path>,
    split_opts: SplitOptions,
    keep_ndjson: bool,
) -> Result<()> {
    let input_path = input.as_ref();

    // Detect format by peeking at first non-whitespace byte
    let first_byte = {
        let mut peek_file = File::open(input_path)
            .with_context(|| format!("failed to open {:?}", input_path))?;
        first_non_whitespace_byte(&mut peek_file)
            .with_context(|| format!("failed to detect format of {:?}", input_path))?
    };

    let ndjson_path: PathBuf;
    let needs_cleanup: bool;

    if first_byte == b'[' {
        // JSON array - needs conversion to NDJSON
        info!("detected JSON array format, converting to NDJSON first...");

        // Create temp NDJSON file
        let temp_dir = std::env::temp_dir();
        let temp_name = format!(
            "floatctl_temp_{}.ndjson",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
        ndjson_path = temp_dir.join(&temp_name);

        info!("creating temporary NDJSON at {:?}", ndjson_path);

        // Convert to NDJSON
        cmd_ndjson(input_path, false, Some(&ndjson_path))
            .context("failed to convert to NDJSON")?;

        needs_cleanup = !keep_ndjson;
    } else {
        // Already NDJSON or other line-delimited format
        info!("detected NDJSON format, proceeding directly to split...");
        ndjson_path = input_path.to_path_buf();
        needs_cleanup = false;
    }

    // Run split on the NDJSON
    info!("running split on {:?}", ndjson_path);
    split_file(&ndjson_path, split_opts)
        .await
        .context("failed to split conversations")?;

    // Cleanup temp file if needed
    if needs_cleanup {
        info!("cleaning up temporary NDJSON file {:?}", ndjson_path);
        if let Err(e) = fs::remove_file(&ndjson_path) {
            eprintln!("warning: failed to remove temp file {:?}: {}", ndjson_path, e);
        }
    } else if first_byte == b'[' && keep_ndjson {
        info!("keeping intermediate NDJSON file at {:?}", ndjson_path);
    }

    Ok(())
}

/// Reads bytes until finding the first non-whitespace byte.
fn first_non_whitespace_byte<R: Read>(reader: &mut R) -> Result<u8> {
    let mut buf = [0u8; 1];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => return Err(anyhow!("empty input file")),
            Ok(_) => {
                if !buf[0].is_ascii_whitespace() {
                    return Ok(buf[0]);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(anyhow!("I/O error: {}", e)),
        }
    }
}

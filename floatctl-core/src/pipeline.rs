use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result};
use chrono::Datelike;
use indicatif::{ProgressBar, ProgressStyle};
use tracing::{debug, info, instrument};

use crate::artifacts::Artifact;
use crate::conversation::Conversation;
use crate::ndjson::{MessageRecord, NdjsonWriter};
use crate::stream::ConvStream;

#[derive(Debug, Clone)]
pub struct SplitOptions {
    pub output_dir: PathBuf,
    pub emit_markdown: bool,
    pub emit_json: bool,
    pub emit_ndjson: bool,
    pub dry_run: bool,
    pub show_progress: bool,
}

impl Default for SplitOptions {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("conv_out"),
            emit_markdown: true,
            emit_json: true,
            emit_ndjson: true,
            dry_run: false,
            show_progress: true,
        }
    }
}

/// Generate a filesystem-safe slug from conversation title and date
fn generate_slug(conv: &Conversation) -> String {
    let date_str = format!(
        "{:04}-{:02}-{:02}",
        conv.meta.created_at.year(),
        conv.meta.created_at.month(),
        conv.meta.created_at.day()
    );

    let title = conv.meta.title.as_deref().unwrap_or("conversation");

    // Strip any existing date prefix from title (e.g., "2024-01-15 - Title")
    let title_without_date = strip_leading_date(title);

    // Slugify: lowercase, replace spaces/special chars with hyphens
    let slug = slugify(&title_without_date);

    // Combine date + slug
    if slug.is_empty() {
        format!("{}-conversation", date_str)
    } else {
        format!("{}-{}", date_str, slug)
    }
}

/// Remove leading date patterns from a string (e.g., "2024-01-15 - ", "2024-01-15: ")
fn strip_leading_date(s: &str) -> String {
    use once_cell::sync::Lazy;
    use regex::Regex;

    // Match patterns like "2024-01-15 - " or "2024-01-15: " or "2024-01-15 "
    static DATE_PREFIX_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"^(\d{4}-\d{2}-\d{2}[\s\-:]+)").unwrap()
    });

    DATE_PREFIX_RE.replace(s, "").trim().to_string()
}

/// Convert string to filesystem-safe slug
fn slugify(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'a'..='z' | '0'..='9' => c,
            'A'..='Z' => c.to_ascii_lowercase(),
            ' ' | '_' | '-' => '-',
            _ => '-',
        })
        .collect::<String>()
        .split('-')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(100) // Limit length
        .collect()
}

/// Map artifact MIME type to file extension
fn artifact_type_to_extension(artifact_type: &str) -> &str {
    match artifact_type {
        // Markdown
        "text/markdown" => "md",
        // React/JavaScript
        "application/vnd.ant.react" => "jsx",
        "application/vnd.ant.code" => "jsx",
        "text/javascript" => "js",
        "application/javascript" => "js",
        // TypeScript
        "text/typescript" => "ts",
        "application/typescript" => "ts",
        // HTML/SVG
        "text/html" => "html",
        "image/svg+xml" => "svg",
        // CSS
        "text/css" => "css",
        // JSON
        "application/json" => "json",
        // Python
        "text/x-python" | "application/x-python" => "py",
        // Other code
        "text/x-java" => "java",
        "text/x-c" => "c",
        "text/x-cpp" => "cpp",
        "text/x-rust" => "rs",
        "text/x-go" => "go",
        // Plain text fallback
        _ => "txt",
    }
}

/// Extract artifacts from conversation messages
fn extract_artifacts(conv: &Conversation) -> Vec<Artifact> {
    let mut artifacts = Vec::new();

    for msg in &conv.messages {
        // Look for tool_use blocks with artifacts in content
        if let Some(content_array) = msg.raw.get("content").and_then(|c| c.as_array()) {
            for (block_idx, block) in content_array.iter().enumerate() {
                if block.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                    && block.get("name").and_then(|n| n.as_str()) == Some("artifacts")
                {
                    if let Some(input) = block.get("input") {
                        let title = input
                            .get("title")
                            .and_then(|t| t.as_str())
                            .unwrap_or("artifact")
                            .to_string();
                        let content = input
                            .get("content")
                            .and_then(|c| c.as_str())
                            .unwrap_or("")
                            .to_string();

                        // Get artifact type and map to extension
                        let artifact_type = input
                            .get("type")
                            .and_then(|t| t.as_str())
                            .unwrap_or("text/plain");
                        let ext = artifact_type_to_extension(artifact_type);

                        let language = input
                            .get("language")
                            .and_then(|l| l.as_str())
                            .map(|s| s.to_string());

                        let filename = format!("{:02}-{}.{}", block_idx, slugify(&title), ext);

                        let mut artifact = Artifact::new_code(msg.idx, title, filename, content);
                        artifact.language = language;
                        artifacts.push(artifact);
                    }
                }
            }
        }
    }

    artifacts
}

#[instrument(skip_all)]
pub fn write_conversation(conv: &Conversation, opts: &SplitOptions) -> Result<()> {
    if opts.dry_run {
        debug!(conv_id = %conv.meta.conv_id, "dry-run: skipping write");
        return Ok(());
    }

    // Generate slug for folder and filenames
    let slug = generate_slug(conv);
    let conv_dir = opts.output_dir.join(&slug);

    // Create conversation directory
    fs::create_dir_all(&conv_dir)
        .with_context(|| format!("failed to create directory {:?}", conv_dir))?;

    // Write NDJSON
    if opts.emit_ndjson {
        let path = conv_dir.join(format!("{}.ndjson", slug));
        let mut writer = NdjsonWriter::create(&path)?;
        for record in MessageRecord::from_conversation(conv) {
            writer.write_record(&record)?;
        }
    }

    // Write JSON
    if opts.emit_json {
        let path = conv_dir.join(format!("{}.json", slug));
        let json = serde_json::to_string_pretty(&conv.raw)?;
        fs::write(path, json)?;
    }

    // Write Markdown
    if opts.emit_markdown {
        let path = conv_dir.join(format!("{}.md", slug));
        fs::write(path, render_markdown(conv)?)?;
    }

    // Extract and write artifacts
    let artifacts = extract_artifacts(conv);
    if !artifacts.is_empty() {
        let artifacts_dir = conv_dir.join("artifacts");
        fs::create_dir_all(&artifacts_dir)
            .with_context(|| format!("failed to create artifacts directory {:?}", artifacts_dir))?;

        for artifact in artifacts {
            let artifact_path = artifacts_dir.join(&artifact.filename);
            fs::write(artifact_path, artifact.body)?;
        }
    }

    Ok(())
}

fn render_markdown(conv: &Conversation) -> Result<String> {
    let mut md = String::new();

    // YAML frontmatter
    md.push_str("---\n");
    md.push_str(&format!("id: {}\n", conv.meta.conv_id));
    if let Some(title) = &conv.meta.title {
        md.push_str(&format!("title: \"{}\"\n", title.replace('"', "\\\"")));
    }
    md.push_str(&format!("created: {}\n", conv.meta.created_at.to_rfc3339()));
    if let Some(updated) = conv.meta.updated_at {
        md.push_str(&format!("updated: {}\n", updated.to_rfc3339()));
    }
    md.push_str(&format!("messages: {}\n", conv.messages.len()));

    // Add markers if present
    let projects: Vec<_> = conv.meta.markers.iter()
        .filter(|m| m.starts_with("project::"))
        .collect();
    if !projects.is_empty() {
        md.push_str("projects:\n");
        for proj in projects {
            md.push_str(&format!("  - {}\n", proj));
        }
    }

    let meetings: Vec<_> = conv.meta.markers.iter()
        .filter(|m| m.starts_with("meeting::") || m.starts_with("standup::"))
        .collect();
    if !meetings.is_empty() {
        md.push_str("meetings:\n");
        for meeting in meetings {
            md.push_str(&format!("  - {}\n", meeting));
        }
    }

    md.push_str("---\n\n");

    // Title
    md.push_str(&format!(
        "# {}\n\n",
        conv.meta.title.as_deref().unwrap_or("Conversation")
    ));

    // Messages
    for message in &conv.messages {
        let role_str = match message.role {
            crate::conversation::MessageRole::User => "👤 User",
            crate::conversation::MessageRole::Assistant => "🤖 Assistant",
            crate::conversation::MessageRole::System => "⚙️  System",
            crate::conversation::MessageRole::Tool => "🔧 Tool",
            crate::conversation::MessageRole::Other => "Other",
        };

        md.push_str(&format!("## {}\n\n", role_str));
        md.push_str(&format!("*{}*\n\n", message.timestamp.format("%Y-%m-%d %H:%M:%S")));

        if !message.content.is_empty() {
            md.push_str(&message.content);
            md.push_str("\n\n");
        }

        // Note artifacts if present
        if let Some(content_array) = message.raw.get("content").and_then(|c| c.as_array()) {
            for block in content_array {
                if block.get("type").and_then(|t| t.as_str()) == Some("tool_use")
                    && block.get("name").and_then(|n| n.as_str()) == Some("artifacts")
                {
                    if let Some(input) = block.get("input") {
                        if let Some(title) = input.get("title").and_then(|t| t.as_str()) {
                            md.push_str(&format!("📎 **Artifact**: {}\n\n", title));
                        }
                    }
                }
            }
        }

        md.push_str("---\n\n");
    }

    Ok(md)
}

#[instrument(skip_all)]
pub async fn split_file(path: impl AsRef<Path>, opts: SplitOptions) -> Result<()> {
    let input_path = path.as_ref();
    let output_dir = opts.output_dir.clone();
    if !opts.dry_run {
        std::fs::create_dir_all(&output_dir)
            .with_context(|| format!("failed to create {:?}", output_dir))?;
    }

    let mut aggregate_writer = if opts.emit_ndjson && !opts.dry_run {
        let path = output_dir.join("messages.ndjson");
        Some(NdjsonWriter::create(path)?)
    } else {
        None
    };

    // Create progress bar
    let progress_bar = maybe_spinner_pb(opts.show_progress);
    let fallback_logging = progress_bar.is_none() && opts.show_progress;

    if let Some(pb) = progress_bar.as_ref() {
        pb.set_message("opening file and detecting format...");
    }

    // Use ConvStream for unified streaming of both JSON arrays and NDJSON
    let stream = ConvStream::from_path(input_path)
        .with_context(|| format!("failed to open {:?}", input_path))?;

    if let Some(pb) = progress_bar.as_ref() {
        pb.set_message("streaming conversations...");
    }

    let mut processed = 0usize;
    for (idx, result) in stream.enumerate() {
        let conv = result.with_context(|| format!("failed to parse conversation #{}", idx + 1))?;
        process_conversation(idx, &conv, &opts, aggregate_writer.as_mut())?;
        processed += 1;

        if let Some(pb) = progress_bar.as_ref() {
            update_progress(pb, processed, &conv);
        } else if fallback_logging {
            log_progress_line(processed, &conv);
        }
    }

    let summary = format!(
        "Split complete: {} conversation(s) written under {:?}",
        processed, opts.output_dir
    );

    if let Some(pb) = progress_bar {
        pb.finish_with_message(summary.clone());
    } else {
        println!("{}", summary);
    }
    info!(target = "floatctl::split", "{}", summary);

    Ok(())
}

fn process_conversation(
    idx: usize,
    conv: &Conversation,
    opts: &SplitOptions,
    aggregate: Option<&mut NdjsonWriter<std::fs::File>>,
) -> Result<()> {
    debug!(index = idx, conv_id = %conv.meta.conv_id, "writing conversation");

    if let Some(writer) = aggregate {
        for record in MessageRecord::from_conversation(conv) {
            writer.write_record(&record)?;
        }
    }

    write_conversation(conv, opts)
}

fn new_spinner_pb() -> ProgressBar {
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner:.green} processed {pos}: {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.enable_steady_tick(Duration::from_millis(120));
    pb
}

#[allow(dead_code)]
fn new_bounded_pb(len: usize) -> ProgressBar {
    let pb = ProgressBar::new(len.max(1) as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} {elapsed_precise} [{bar:30.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap()
        .progress_chars("█▉▊▋▌▍▎▏ "),
    );
    pb.set_length(len as u64);
    pb
}

fn maybe_spinner_pb(show_progress: bool) -> Option<ProgressBar> {
    if !show_progress {
        return None;
    }
    let pb = new_spinner_pb();
    if pb.is_hidden() {
        None
    } else {
        Some(pb)
    }
}

#[allow(dead_code)]
fn maybe_bounded_pb(show_progress: bool, len: usize) -> Option<ProgressBar> {
    if !show_progress {
        return None;
    }
    let pb = new_bounded_pb(len);
    if pb.is_hidden() {
        None
    } else {
        Some(pb)
    }
}

fn update_progress(pb: &ProgressBar, processed: usize, conv: &Conversation) {
    let title = progress_label(conv);
    pb.inc(1);
    pb.set_message(title);
    pb.set_position(processed as u64);
}

fn progress_label(conv: &Conversation) -> String {
    let raw = conv.meta.title.as_deref().unwrap_or(&conv.meta.conv_id);
    const LIMIT: usize = 60;
    const HEAD_LIMIT: usize = LIMIT.saturating_sub(1);
    let mut truncated = String::new();
    for (idx, ch) in raw.chars().enumerate() {
        if idx >= HEAD_LIMIT {
            truncated.push('…');
            return truncated;
        }
        truncated.push(ch);
    }
    truncated
}

fn log_progress_line(processed: usize, conv: &Conversation) {
    if processed == 1 || processed % 25 == 0 {
        println!(
            "Processed {:>5} conversations (latest: {})",
            processed,
            progress_label(conv)
        );
    }
}

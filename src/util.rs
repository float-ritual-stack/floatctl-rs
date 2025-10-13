use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Utc;
use directories::ProjectDirs;
use once_cell::sync::Lazy;
use sha2::{Digest, Sha256};

use crate::cli::SourceSelector;
use crate::config::{Config, FilenameStrategy, OutputFormat};
use crate::filters::FilterContext;
use crate::input::load_input;
use crate::model::{Conversation, Source, canonicalize_value, parse_conversation};
use crate::render_json;
use crate::render_md;
use crate::slug::{SlugState, conversation_base_name, slugify, strip_leading_date};
use crate::state::{RunRecord, StateHandle};
use regex::Regex;
use serde_json::{Value, json};

pub static DEFAULT_CONFIG_PATH: Lazy<PathBuf> = Lazy::new(|| {
    let proj = project_dirs();
    proj.config_dir().join("conv_split.toml")
});

pub static DEFAULT_STATE_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let proj = project_dirs();
    proj.data_dir().join("state")
});

pub static DEFAULT_TMP_DIR: Lazy<PathBuf> = Lazy::new(|| {
    let proj = project_dirs();
    proj.cache_dir().join("tmp")
});

static DOUBLE_COLON_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?i)\b([a-z0-9._-]+::)").unwrap());
static FLOAT_DOT_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(float\.[a-z0-9._-]+)").unwrap());

fn project_dirs() -> ProjectDirs {
    ProjectDirs::from("dev", "float", "floatctl")
        .expect("unable to determine platform project directories")
}

#[derive(Debug, Clone, Copy)]
pub enum ErrorKind {
    Input,
    Io,
    Validation,
    Config,
}

#[derive(Debug)]
pub struct AppError {
    pub(crate) kind: ErrorKind,
    pub(crate) source: anyhow::Error,
}

impl AppError {
    pub fn new(kind: ErrorKind, source: anyhow::Error) -> Self {
        Self { kind, source }
    }

    pub fn input<E: Into<anyhow::Error>>(err: E) -> Self {
        Self::new(ErrorKind::Input, err.into())
    }

    pub fn io<E: Into<anyhow::Error>>(err: E) -> Self {
        Self::new(ErrorKind::Io, err.into())
    }

    pub fn validation<E: Into<anyhow::Error>>(err: E) -> Self {
        Self::new(ErrorKind::Validation, err.into())
    }

    pub fn config<E: Into<anyhow::Error>>(err: E) -> Self {
        Self::new(ErrorKind::Config, err.into())
    }

    pub fn exit_code(&self) -> std::process::ExitCode {
        match self.kind {
            ErrorKind::Input => std::process::ExitCode::from(1),
            ErrorKind::Io => std::process::ExitCode::from(2),
            ErrorKind::Validation | ErrorKind::Config => std::process::ExitCode::from(3),
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.source)
    }
}

pub fn expand_path(path: &Path) -> anyhow::Result<PathBuf> {
    let path_str = path.to_string_lossy();
    if let Some(stripped) = path_str.strip_prefix("~/") {
        let home = home::home_dir().context("unable to resolve home directory")?;
        Ok(home.join(stripped))
    } else {
        Ok(path.to_path_buf())
    }
}

pub fn init_logger() {
    static INIT: std::sync::Once = std::sync::Once::new();
    INIT.call_once(|| {
        let _ = env_logger::builder().is_test(false).try_init();
    });
}

pub struct Outcome {
    pub processed: usize,
    #[allow(dead_code)]
    pub skipped: usize,
    pub dry_run: bool,
    pub had_input: bool,
}

impl Outcome {
    pub fn success(processed: usize, skipped: usize, dry_run: bool, had_input: bool) -> Self {
        Self {
            processed,
            skipped,
            dry_run,
            had_input,
        }
    }

    pub fn into_exit_code(self) -> std::process::ExitCode {
        if self.processed == 0 && !self.dry_run && self.had_input {
            std::process::ExitCode::from(4)
        } else {
            std::process::ExitCode::SUCCESS
        }
    }
}

pub struct OutputPaths {
    pub dir: PathBuf,
    pub md: Option<PathBuf>,
    pub json: Option<PathBuf>,
    pub artifacts_dir: PathBuf,
}

pub fn build_output_paths(
    config: &Config,
    filter_ctx: &FilterContext,
    slug_state: &mut SlugState,
    conversation: &Conversation,
) -> OutputPaths {
    let date_prefix = filter_ctx
        .filename_prefix_date(conversation.created)
        .format("%Y-%m-%d")
        .to_string();

    let base_source = match config.filename_from {
        FilenameStrategy::Title => conversation_base_name(conversation),
        FilenameStrategy::Id => conversation.conv_id.clone(),
        FilenameStrategy::FirstHumanLine => conversation
            .messages
            .iter()
            .find(|message| message.role.as_str() == "human")
            .and_then(|message| message.channels.first())
            .map(|channel| strip_leading_date(&channel.text))
            .map(|text| text.trim().to_string())
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| conversation_base_name(conversation)),
    };

    let mut slug_tail = slugify(base_source.trim());
    if slug_tail.is_empty() {
        slug_tail = "untitled".to_string();
    }

    let base_slug = format!("{date_prefix}-{}", slug_tail);
    let unique_slug = slug_state.next_slug(&base_slug);

    let has_md = config
        .formats
        .iter()
        .any(|format| matches!(format, OutputFormat::Md));
    let has_json = config
        .formats
        .iter()
        .any(|format| matches!(format, OutputFormat::Json));

    let dir = config.output_dir.join(&unique_slug);
    let md_path = has_md.then(|| dir.join(format!("{unique_slug}.md")));
    let json_path = has_json.then(|| dir.join(format!("{unique_slug}.json")));
    let artifacts_dir = dir.join("artifacts");

    OutputPaths {
        dir,
        md: md_path,
        json: json_path,
        artifacts_dir,
    }
}

pub fn execute(config: Config, force: bool, dry_run: bool) -> Result<Outcome, AppError> {
    let run_id = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let filter_ctx = FilterContext::new(&config).map_err(AppError::config)?;

    let input_bundle = load_input(&config, &run_id).map_err(AppError::input)?;

    match config.source {
        SourceSelector::Auto => {}
        SourceSelector::Chatgpt => {
            if input_bundle.source != Source::ChatGpt {
                return Err(AppError::input(anyhow::anyhow!(
                    "expected chatgpt export but detected {}",
                    input_bundle.source.as_str()
                )));
            }
        }
        SourceSelector::Anthropic => {
            if input_bundle.source != Source::Anthropic {
                return Err(AppError::input(anyhow::anyhow!(
                    "expected anthropic export but detected {}",
                    input_bundle.source.as_str()
                )));
            }
        }
    }

    let mut state = StateHandle::load(&config.state_dir).map_err(AppError::io)?;

    let crate::input::InputBundle {
        source: bundle_source,
        ndjson_path,
        fingerprint,
    } = input_bundle;

    let ndjson_file = File::open(&ndjson_path)
        .with_context(|| format!("failed to open {}", ndjson_path.display()))
        .map_err(AppError::io)?;
    let reader = BufReader::new(ndjson_file);

    let mut had_input = false;

    if !dry_run {
        fs::create_dir_all(&config.output_dir)
            .with_context(|| format!("failed to create {}", config.output_dir.display()))
            .map_err(AppError::io)?;
    }

    let mut slug_state = SlugState::new();
    let mut processed_count = 0usize;
    let mut skipped_count = 0usize;
    let mut processed_map: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for (idx, line) in reader.lines().enumerate() {
        if (idx + 1) % 200 == 0 || idx == 0 {
            eprintln!("processing conversations: {}", idx + 1);
        }

        let line = line.map_err(AppError::io)?;
        if line.trim().is_empty() {
            continue;
        }

        let raw: Value = serde_json::from_str(&line).map_err(AppError::validation)?;
        let conversation = parse_conversation(bundle_source, raw).map_err(AppError::validation)?;

        let canonical = canonicalize_value(&conversation.raw);
        let bytes = serde_json::to_vec(&canonical).map_err(AppError::validation)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        let source_hash = format!("sha256:{}", hex::encode(hasher.finalize()));

        if !filter_ctx.includes(&conversation) {
            continue;
        }

        had_input = true;

        let already_seen = state.state.seen.get(&conversation.conv_id);
        let is_duplicate = if config.dedupe {
            matches!(already_seen, Some(seen) if seen.hash == source_hash)
        } else {
            already_seen.is_some()
        };
        if is_duplicate && !force {
            skipped_count += 1;
            continue;
        }

        let output_paths = build_output_paths(&config, &filter_ctx, &mut slug_state, &conversation);

        if dry_run {
            if let Some(path) = output_paths.md.as_ref().or(output_paths.json.as_ref()) {
                println!(
                    "[dry-run] would write conversation {} to {}",
                    conversation.conv_id,
                    path.display()
                );
            } else {
                println!(
                    "[dry-run] would process conversation {}",
                    conversation.conv_id
                );
            }
            let _ = io::stdout().flush();
        } else {
            if output_paths.dir.exists() {
                fs::remove_dir_all(&output_paths.dir)
                    .with_context(|| format!("failed to clean {}", output_paths.dir.display()))
                    .map_err(AppError::io)?;
            }
            fs::create_dir_all(&output_paths.dir)
                .with_context(|| format!("failed to create {}", output_paths.dir.display()))
                .map_err(AppError::io)?;

            let artifact_rel_paths: Vec<Vec<String>> = conversation
                .messages
                .iter()
                .enumerate()
                .map(|(msg_idx, message)| {
                    message
                        .artifacts
                        .iter()
                        .enumerate()
                        .map(|(art_idx, artifact)| {
                            format!(
                                "artifacts/{}",
                                artifact_file_name(msg_idx, art_idx, artifact)
                            )
                        })
                        .collect()
                })
                .collect();

            if let Some(ref path) = output_paths.md {
                render_md::write_conversation_md(
                    &conversation,
                    &filter_ctx,
                    Some(&artifact_rel_paths),
                    path,
                )
                .map_err(AppError::io)?;
            }
            if let Some(ref path) = output_paths.json {
                render_json::write_conversation_json(
                    &conversation.raw,
                    config.pretty_json_indent,
                    path,
                )
                .map_err(AppError::io)?;
            }

            write_conversation_ndjson(
                &conversation,
                &artifact_rel_paths,
                &output_paths.dir,
                &filter_ctx,
            )
            .map_err(AppError::io)?;

            if conversation
                .messages
                .iter()
                .any(|message| !message.artifacts.is_empty())
            {
                fs::create_dir_all(&output_paths.artifacts_dir).map_err(AppError::io)?;

                for (msg_idx, message) in conversation.messages.iter().enumerate() {
                    for (art_idx, artifact) in message.artifacts.iter().enumerate() {
                        let Some(code) = artifact.code.as_deref() else {
                            continue;
                        };
                        if code.trim().is_empty() {
                            continue;
                        }
                        let rel_path = &artifact_rel_paths[msg_idx][art_idx];
                        let path = output_paths.dir.join(rel_path);
                        let mut file = fs::File::create(&path)
                            .with_context(|| format!("failed to create {}", path.display()))
                            .map_err(AppError::io)?;
                        file.write_all(code.as_bytes())
                            .with_context(|| format!("failed to write {}", path.display()))
                            .map_err(AppError::io)?;
                    }
                }
            }
        }

        processed_count += 1;

        processed_map
            .entry(conversation.source.as_str().to_string())
            .or_default()
            .push(conversation.conv_id.clone());

        if !dry_run {
            state.update_seen(
                &conversation.conv_id,
                &conversation
                    .created
                    .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                conversation.source,
                &source_hash,
            );
        }
    }

    if !dry_run {
        for list in processed_map.values_mut() {
            list.sort();
        }
        state.record_run(RunRecord {
            run_id: run_id.clone(),
            input_fingerprint: fingerprint,
            processed: processed_map,
        });
        state.save().map_err(AppError::io)?;
    }

    if dry_run {
        println!(
            "[dry-run] {} conversation(s) would be written, {} skipped",
            processed_count, skipped_count
        );
    } else {
        println!(
            "Processed {} conversation(s), skipped {}",
            processed_count, skipped_count
        );
    }

    Ok(Outcome::success(
        processed_count,
        skipped_count,
        dry_run,
        had_input,
    ))
}

fn write_conversation_ndjson(
    conversation: &Conversation,
    artifact_paths: &[Vec<String>],
    dir: &Path,
    filter_ctx: &FilterContext,
) -> Result<(), anyhow::Error> {
    let path = dir.join("conversation.ndjson");
    let file =
        File::create(&path).with_context(|| format!("failed to create {}", path.display()))?;
    let mut writer = BufWriter::new(file);

    let message_markers: Vec<Vec<String>> = conversation
        .messages
        .iter()
        .map(|message| {
            let mut set = BTreeSet::new();
            for channel in &message.channels {
                for marker in extract_markers(&channel.text) {
                    set.insert(marker);
                }
            }
            set.into_iter().collect()
        })
        .collect();

    let mut all_markers = BTreeSet::new();
    for markers in &message_markers {
        for marker in markers {
            all_markers.insert(marker.clone());
        }
    }

    let meta = json!({
        "type": "meta",
        "conv_id": conversation.conv_id,
        "source": conversation.source.as_str(),
        "created_at": conversation
            .created
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "created_local": filter_ctx.display_timestamp(Some(conversation.created)),
        "updated_at": conversation
            .updated
            .map(|ts| ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        "updated_local": filter_ctx.display_timestamp(conversation.updated),
        "title": conversation.title,
        "summary": conversation.summary,
        "model": conversation.model,
        "participants": conversation.participants.iter().collect::<Vec<_>>(),
        "markers": all_markers.into_iter().collect::<Vec<_>>(),
    });
    serde_json::to_writer(&mut writer, &meta)?;
    writer.write_all(b"\n")?;

    for (idx, message) in conversation.messages.iter().enumerate() {
        let rel_paths = artifact_paths.get(idx).cloned().unwrap_or_else(Vec::new);
        let markers = message_markers.get(idx).cloned().unwrap_or_default();

        let channels: Vec<_> = message
            .channels
            .iter()
            .map(|channel| {
                json!({
                    "channel": channel.channel,
                    "text": channel.text,
                })
            })
            .collect();

        let attachments: Vec<_> = message
            .attachments
            .iter()
            .map(|att| {
                json!({
                    "name": att.name,
                    "uri": att.uri,
                    "mime": att.mime,
                    "sha256": att.sha256,
                    "width": att.width,
                    "height": att.height,
                })
            })
            .collect();

        let tool_calls: Vec<_> = message
            .tool_calls
            .iter()
            .map(|call| {
                json!({
                    "name": call.name,
                    "args": call.args,
                    "result": call.result,
                })
            })
            .collect();

        let artifact_meta: Vec<_> = message
            .artifacts
            .iter()
            .enumerate()
            .map(|(art_idx, artifact)| {
                json!({
                    "kind": artifact.kind,
                    "lang": artifact.lang,
                    "code_path": rel_paths.get(art_idx),
                })
            })
            .collect();

        let record = json!({
            "type": "message",
            "idx": idx + 1,
            "message_id": message.message_id,
            "role": message.role.as_str(),
            "timestamp": message
                .timestamp
                .map(|ts| ts.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            "timestamp_local": filter_ctx.display_timestamp(message.timestamp),
            "channels": channels,
            "attachments": attachments,
            "tool_calls": tool_calls,
            "artifacts": artifact_meta,
            "markers": markers,
        });

        serde_json::to_writer(&mut writer, &record)?;
        writer.write_all(b"\n")?;
    }

    writer.flush()?;

    Ok(())
}

fn extract_markers(text: &str) -> Vec<String> {
    let mut set = BTreeSet::new();
    for caps in DOUBLE_COLON_RE.captures_iter(text) {
        if let Some(m) = caps.get(1) {
            set.insert(m.as_str().to_lowercase());
        }
    }
    for caps in FLOAT_DOT_RE.captures_iter(text) {
        if let Some(m) = caps.get(1) {
            set.insert(m.as_str().to_lowercase());
        }
    }
    set.into_iter().collect()
}

fn artifact_file_name(msg_idx: usize, art_idx: usize, artifact: &crate::model::Artifact) -> String {
    let base_slug = artifact
        .kind
        .as_deref()
        .map(slugify)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "artifact".to_string());
    let ext = extension_from_lang(artifact.lang.as_deref(), artifact.kind.as_deref());
    format!(
        "{}-{:03}-{:03}.{}",
        base_slug,
        msg_idx + 1,
        art_idx + 1,
        ext
    )
}

fn extension_from_lang(lang: Option<&str>, kind: Option<&str>) -> String {
    if let Some(lang) = lang {
        let trimmed = lang.trim();
        if let Some(mapped) = match trimmed {
            "text/html" => Some("html"),
            "application/json" => Some("json"),
            "application/vnd.ant.canvas+json" => Some("json"),
            "text/markdown" => Some("md"),
            "text/plain" => Some("txt"),
            "application/javascript" | "text/javascript" => Some("js"),
            "application/typescript" => Some("ts"),
            "application/vnd.ant.react" | "application/vnd.ant.react+json" => Some("jsx"),
            "text/css" => Some("css"),
            _ => None,
        } {
            return mapped.to_string();
        }

        if let Some(idx) = trimmed.rfind('/') {
            let ext = slugify(&trimmed[idx + 1..]);
            if !ext.is_empty() {
                return ext;
            }
        }
        let ext = slugify(trimmed);
        if !ext.is_empty() {
            return ext;
        }
    }

    if let Some(kind) = kind {
        let lower = kind.to_lowercase();
        for suffix in [".html", ".md", ".json", ".txt", ".jsx", ".tsx"] {
            if lower.ends_with(suffix) {
                return suffix.trim_start_matches('.').to_string();
            }
        }
        let slug = slugify(&lower);
        if !slug.is_empty() {
            return slug;
        }
    }

    "txt".to_string()
}

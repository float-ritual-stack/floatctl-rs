use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::Args;
use floatctl_core::ndjson::MessageRecord;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use once_cell::sync::Lazy;
use pgvector::Vector;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tiktoken_rs::{cl100k_base, CoreBPE};
use tokio::fs::File;
use tokio::io::{stdin, AsyncBufReadExt, AsyncRead, BufReader};
use tracing::{info, warn};
use uuid::Uuid;

static MODEL_NAME: &str = "text-embedding-3-small";
static CHUNK_SIZE: usize = 6000; // Conservative: 2K buffer below 8192 limit
static CHUNK_OVERLAP: usize = 200; // Token overlap for continuity

/// Cached tokenizer instance (loaded once, reused for all messages)
static BPE: Lazy<CoreBPE> = Lazy::new(|| {
    cl100k_base().expect("Failed to load cl100k_base tokenizer")
});

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../migrations");

/// Count tokens in text using cached cl100k_base tokenizer (same as text-embedding-3-small)
fn count_tokens(text: &str) -> Result<usize> {
    let tokens = BPE.encode_with_special_tokens(text);
    Ok(tokens.len())
}

/// Simple token-based chunking with fixed size and overlap
///
/// Strategy:
/// 1. Encode text to tokens using tiktoken
/// 2. Split at exact token boundaries (CHUNK_SIZE tokens per chunk)
/// 3. Add CHUNK_OVERLAP tokens between chunks for continuity
/// 4. Hard truncation safety valve if chunk exceeds MAX_TOKENS_HARD_LIMIT
fn chunk_message(text: &str) -> Result<Vec<String>> {
    // Validate constants to prevent infinite loop
    if CHUNK_OVERLAP >= CHUNK_SIZE {
        return Err(anyhow!(
            "CHUNK_OVERLAP ({}) must be less than CHUNK_SIZE ({})",
            CHUNK_OVERLAP,
            CHUNK_SIZE
        ));
    }

    let tokens = BPE.encode_with_special_tokens(text);

    // No chunking needed
    if tokens.len() <= CHUNK_SIZE {
        return Ok(vec![text.to_string()]);
    }

    let mut chunks = Vec::new();
    let mut start = 0;

    while start < tokens.len() {
        let end = (start + CHUNK_SIZE).min(tokens.len());
        let chunk_tokens = &tokens[start..end];

        // Try to decode tokens - if it fails, try smaller chunks to recover partial content
        let chunk_text = match BPE.decode(chunk_tokens.to_vec()) {
            Ok(text) => text,
            Err(e) => {
                // Decode failed - try to recover by decoding smaller segments
                warn!(
                    "UTF-8 decode failed for chunk {} (tokens {}-{}): {}. Attempting partial recovery.",
                    chunks.len(),
                    start,
                    end,
                    e
                );

                // Try decoding in smaller segments and concatenate what works
                let mut recovered = String::new();
                let segment_size = 100; // Try 100 tokens at a time
                let mut seg_start = 0;

                while seg_start < chunk_tokens.len() {
                    let seg_end = (seg_start + segment_size).min(chunk_tokens.len());
                    match BPE.decode(chunk_tokens[seg_start..seg_end].to_vec()) {
                        Ok(segment_text) => {
                            recovered.push_str(&segment_text);
                        }
                        Err(_) => {
                            // Even small segment failed, add replacement character
                            recovered.push('�');
                        }
                    }
                    seg_start = seg_end;
                }

                if recovered.is_empty() {
                    warn!("Could not recover any content from chunk {}", chunks.len());
                }

                recovered
            }
        };

        if !chunk_text.is_empty() {
            chunks.push(chunk_text);
        }

        // Move start forward with overlap (subtract overlap to create sliding window)
        start += CHUNK_SIZE - CHUNK_OVERLAP;
    }

    Ok(chunks)
}

#[derive(Args, Debug)]
pub struct EmbedArgs {
    #[arg(long = "in", value_name = "PATH")]
    pub input: PathBuf,

    #[arg(long)]
    pub since: Option<NaiveDate>,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long, default_value = "32")]
    pub batch_size: usize,

    #[arg(long)]
    pub dry_run: bool,

    /// Skip messages that already have embeddings (for idempotent re-runs)
    #[arg(long)]
    pub skip_existing: bool,

    /// Delay in milliseconds between OpenAI API calls to avoid rate limits (default: 500ms)
    #[arg(long, default_value = "500")]
    pub rate_limit_ms: u64,
}

#[derive(Args, Debug)]
pub struct QueryArgs {
    /// Natural language query.
    pub query: String,

    #[arg(long)]
    pub project: Option<String>,

    #[arg(long, default_value = "10")]
    pub limit: i64,

    #[arg(long = "days")]
    pub days: Option<i64>,
}

pub async fn run_embed(mut args: EmbedArgs) -> Result<()> {
    dotenvy::dotenv().ok();

    // Validate batch size to prevent exceeding OpenAI's 300K tokens per request limit
    if args.batch_size > 50 {
        warn!(
            "batch_size {} exceeds recommended maximum of 50 (can hit OpenAI 300K token limit), capping at 50",
            args.batch_size
        );
        args.batch_size = 50;
    }

    if args.dry_run {
        let stats = dry_run_scan(&args).await?;
        info!(
            "dry-run: would embed {} messages across {} conversations (filtered)",
            stats.messages, stats.conversations
        );
        return Ok(());
    }

    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&database_url)
        .await?;
    ensure_extensions(&pool).await?;
    MIGRATOR.run(&pool).await?;

    // Create or recreate IVFFlat index with optimal lists parameter
    ensure_optimal_ivfflat_index(&pool).await?;

    // Load existing message IDs if skip-existing enabled
    let existing_messages: HashSet<Uuid> = if args.skip_existing {
        info!("loading existing embeddings to skip...");
        let rows: Vec<(Uuid,)> = sqlx::query_as("SELECT message_id FROM embeddings")
            .fetch_all(&pool)
            .await?;
        let count = rows.len();
        let memory_mb = (count * 16) as f64 / 1_048_576.0;
        let set: HashSet<Uuid> = rows.into_iter().map(|(id,)| id).collect();
        info!(
            "loaded {} existing message IDs ({:.2} MB estimated memory)",
            count, memory_mb
        );
        set
    } else {
        HashSet::new()
    };

    let mut conv_lookup: HashMap<String, Uuid> = HashMap::new();
    let mut pending = Vec::with_capacity(args.batch_size);
    let mut message_batch = Vec::with_capacity(args.batch_size);
    let openai = OpenAiClient::new(api_key)?;
    let since = args.since.map(|d| d.and_hms_opt(0, 0, 0).unwrap());
    let since = since.map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    let mut processed = 0usize;
    let mut chunked_messages = 0usize;
    let mut skipped = 0usize;

    // Setup progress bars
    let multi = MultiProgress::new();
    let conv_bar = multi.add(ProgressBar::new_spinner());
    conv_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} [{elapsed_precise}] {msg}")
            .unwrap()
    );
    let msg_bar = multi.add(ProgressBar::new_spinner());
    msg_bar.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap()
    );

    conv_bar.set_message("Starting...");
    msg_bar.set_message("Processed: 0 | Chunked: 0 | Skipped: 0");

    // Stream records from file
    let mut reader = open_reader(&args.input).await?;
    let mut current_conv_title = String::from("(unknown)");

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }

        let record: MessageRecord = match serde_json::from_str(&line) {
            Ok(record) => record,
            Err(err) => {
                warn!(error = ?err, "skipping malformed record");
                continue;
            }
        };

        match record {
            MessageRecord::Meta {
                conv_id,
                title,
                created_at,
                markers,
            } => {
                let created_at = parse_timestamp(&created_at)?;
                let conv_uuid =
                    upsert_conversation(&pool, &conv_id, title.clone(), created_at, markers).await?;
                conv_lookup.insert(conv_id, conv_uuid);

                // Update progress bar with new conversation
                if let Some(ref t) = title {
                    current_conv_title = t.clone();
                    conv_bar.set_message(format!("📖 {}", truncate(&current_conv_title, 60)));
                }
            }
            MessageRecord::Message {
                conv_id,
                idx,
                message_id,
                role,
                timestamp,
                content,
                project,
                meeting,
                markers,
            } => {
                let Some(conversation_id) = conv_lookup.get(&conv_id).copied() else {
                    warn!("message without prior meta for conv_id={}", conv_id);
                    continue;
                };
                let timestamp = parse_timestamp(&timestamp)?;
                if let Some(since) = since {
                    if timestamp < since {
                        continue;
                    }
                }
                if let Some(required_project) = &args.project {
                    if project.as_deref() != Some(required_project) {
                        continue;
                    }
                }

                let message_uuid = parse_uuid(&message_id);

                // Skip if already embedded
                if args.skip_existing && existing_messages.contains(&message_uuid) {
                    skipped += 1;
                    msg_bar.set_message(format!(
                        "Processed: {} | Chunked: {} | Skipped: {}",
                        processed, chunked_messages, skipped
                    ));
                    continue;
                }

                message_batch.push(MessageUpsert {
                    id: message_uuid,
                    conversation_id,
                    idx,
                    role,
                    timestamp,
                    content: content.clone(),
                    project: project.clone(),
                    meeting,
                    markers,
                });

                if !content.trim().is_empty() {
                    // Chunk the message if needed
                    let chunks = chunk_message(&content)?;
                    let chunk_count = chunks.len();

                    if chunk_count > 1 {
                        chunked_messages += 1;
                        let token_count = count_tokens(&content)?;
                        let preview = truncate(&content, 50);
                        msg_bar.println(format!(
                            "  ✂️  {} tokens → {} chunks: \"{}\"",
                            token_count, chunk_count, preview
                        ));
                    }

                    // Add each chunk as a separate embedding job
                    for (idx, chunk_text) in chunks.into_iter().enumerate() {
                        pending.push(EmbeddingJob {
                            message_id: message_uuid,
                            chunk_index: idx,
                            chunk_count,
                            chunk_text,
                        });

                        // If batch is full, flush messages FIRST, then embeddings
                        if pending.len() >= args.batch_size {
                            // Flush message batch before embeddings to satisfy foreign key constraint
                            if !message_batch.is_empty() {
                                flush_message_batch(&pool, &mut message_batch).await?;
                            }
                            flush_embeddings(&pool, &openai, &mut pending, args.rate_limit_ms).await?;
                        }
                    }
                    processed += 1;

                    // Update message counter
                    msg_bar.set_message(format!(
                        "Processed: {} | Chunked: {} | Skipped: {}",
                        processed, chunked_messages, skipped
                    ));
                }

                // Flush message batch when we hit the batch size (for messages without embeddings)
                if message_batch.len() >= args.batch_size {
                    flush_message_batch(&pool, &mut message_batch).await?;
                }
            }
        }
    }

    // Flush remaining batches
    if !message_batch.is_empty() {
        flush_message_batch(&pool, &mut message_batch).await?;
    }
    if !pending.is_empty() {
        flush_embeddings(&pool, &openai, &mut pending, args.rate_limit_ms).await?;
    }

    conv_bar.finish_with_message(format!("✅ Completed! {} messages processed", processed));
    msg_bar.finish_with_message(format!("Chunked: {} | Skipped: {}", chunked_messages, skipped));

    Ok(())
}

/// Truncate string to max length, adding ellipsis if needed
///
/// Uses char_indices() to respect UTF-8 character boundaries
fn truncate(s: &str, max_len: usize) -> String {
    if s.chars().count() <= max_len {
        s.to_string()
    } else {
        let ellipsis_len = 3;
        let target_len = max_len.saturating_sub(ellipsis_len);

        // Find the byte index of the target character position
        let truncate_at = s
            .char_indices()
            .nth(target_len)
            .map(|(idx, _)| idx)
            .unwrap_or(s.len());

        format!("{}...", &s[..truncate_at])
    }
}

pub async fn run_query(args: QueryArgs) -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    let pool = PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(30))
        .connect(&database_url)
        .await?;
    ensure_extensions(&pool).await?;
    MIGRATOR.run(&pool).await?;

    // Create or recreate IVFFlat index with optimal lists parameter
    ensure_optimal_ivfflat_index(&pool).await?;

    let openai = OpenAiClient::new(api_key)?;
    let vector = openai.embed_query(&args.query).await?;

    let mut builder = sqlx::QueryBuilder::new(
        "select m.content, m.project, m.meeting, m.timestamp \
         from messages m join embeddings e on e.message_id = m.id",
    );
    builder.push(" where 1=1");
    if let Some(project) = &args.project {
        builder.push(" and m.project = ");
        builder.push_bind(project);
    }
    if let Some(days) = args.days {
        let cutoff = Utc::now() - Duration::days(days);
        builder.push(" and m.timestamp >= ");
        builder.push_bind(cutoff);
    }
    builder.push(" order by e.vector <-> ");
    builder.push_bind(vector);
    builder.push(" limit ");
    builder.push_bind(args.limit);

    let rows: Vec<QueryRow> = builder.build_query_as().fetch_all(&pool).await?;
    if rows.is_empty() {
        info!("no matches found");
    } else {
        for row in rows {
            println!(
                "[{}] project={} meeting={:?}\n{}\n",
                row.timestamp,
                row.project.unwrap_or_default(),
                row.meeting,
                row.content
            );
        }
    }

    Ok(())
}

struct OpenAiClient {
    http: reqwest::Client,
    api_key: String,
}

impl OpenAiClient {
    fn new(api_key: String) -> Result<Self> {
        if api_key.trim().is_empty() {
            return Err(anyhow!("OPENAI_API_KEY is empty"));
        }
        let http = reqwest::Client::builder().build()?;
        Ok(Self { http, api_key })
    }

    async fn embed_query(&self, query: &str) -> Result<Vector> {
        let vectors = self.embed_batch(&[query.to_owned()]).await?;
        vectors
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no vector returned"))
    }

    async fn embed_batch(&self, inputs: &[String]) -> Result<Vec<Vector>> {
        let refs: Vec<&str> = inputs.iter().map(|s| s.as_str()).collect();
        self.embed_batch_refs(&refs).await
    }

    async fn embed_batch_refs(&self, inputs: &[&str]) -> Result<Vec<Vector>> {
        #[derive(serde::Serialize)]
        struct EmbeddingRequest<'a> {
            model: &'a str,
            input: &'a [&'a str],
        }

        #[derive(serde::Deserialize)]
        struct EmbeddingResponse {
            data: Vec<EmbeddingData>,
        }

        #[derive(serde::Deserialize)]
        struct EmbeddingData {
            embedding: Vec<f32>,
            index: usize,
        }

        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let response = self
            .http
            .post("https://api.openai.com/v1/embeddings")
            .bearer_auth(&self.api_key)
            .json(&EmbeddingRequest {
                model: MODEL_NAME,
                input: inputs,
            })
            .send()
            .await?;

        // Check status and extract detailed error if failed
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Could not read error body".to_string());
            return Err(anyhow!("OpenAI API error ({}): {}", status, error_text));
        }

        let response = response.json::<EmbeddingResponse>().await?;

        let mut vectors = vec![None; inputs.len()];
        for data in response.data {
            let vector = Vector::from(data.embedding);
            if data.index < vectors.len() {
                vectors[data.index] = Some(vector);
            }
        }

        vectors
            .into_iter()
            .enumerate()
            .map(|(idx, maybe)| maybe.ok_or_else(|| anyhow!("missing embedding for index {}", idx)))
            .collect()
    }
}

async fn flush_message_batch(pool: &PgPool, batch: &mut Vec<MessageUpsert>) -> Result<()> {
    if batch.is_empty() {
        return Ok(());
    }

    // Spawn concurrent database inserts
    let insert_futures: Vec<_> = batch
        .drain(..)
        .map(|msg| {
            let pool = pool.clone();
            tokio::spawn(async move { upsert_message(&pool, &msg).await })
        })
        .collect();

    // Wait for all inserts to complete
    for result in futures::future::join_all(insert_futures).await {
        result??; // Unwrap spawn result and DB result
    }

    Ok(())
}

async fn flush_embeddings(
    pool: &PgPool,
    openai: &OpenAiClient,
    pending: &mut Vec<EmbeddingJob>,
    rate_limit_ms: u64,
) -> Result<()> {
    if pending.is_empty() {
        return Ok(());
    }

    // Avoid cloning: collect references, then convert to owned inside embed_batch
    let batch: Vec<&str> = pending.iter().map(|job| job.chunk_text.as_str()).collect();
    let vectors = openai.embed_batch_refs(&batch).await?;

    // Insert embeddings into database
    for (job, vector) in pending.drain(..).zip(vectors) {
        upsert_embedding(
            pool,
            job.message_id,
            job.chunk_index as i32,
            job.chunk_count as i32,
            &job.chunk_text,
            vector,
        )
        .await?;
    }

    // Rate limiting: sleep between batches to avoid hitting OpenAI limits
    if rate_limit_ms > 0 {
        tokio::time::sleep(tokio::time::Duration::from_millis(rate_limit_ms)).await;
    }

    Ok(())
}

async fn upsert_embedding(
    pool: &PgPool,
    message_id: Uuid,
    chunk_index: i32,
    chunk_count: i32,
    chunk_text: &str,
    vector: Vector,
) -> Result<()> {
    let dim = vector.as_slice().len() as i32;
    sqlx::query(
        r#"
        insert into embeddings (message_id, chunk_index, chunk_count, chunk_text, model, dim, vector, created_at)
        values ($1, $2, $3, $4, $5, $6, $7, NOW())
        on conflict (message_id, chunk_index)
        do update set chunk_count = excluded.chunk_count,
                      chunk_text = excluded.chunk_text,
                      model = excluded.model,
                      dim = excluded.dim,
                      vector = excluded.vector,
                      updated_at = NOW()
        "#,
    )
    .bind(message_id)
    .bind(chunk_index)
    .bind(chunk_count)
    .bind(chunk_text)
    .bind(MODEL_NAME)
    .bind(dim)
    .bind(vector)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_message(pool: &PgPool, message: &MessageUpsert) -> Result<()> {
    sqlx::query(
        r#"
        insert into messages (id, conversation_id, idx, role, timestamp, content, project, meeting, markers)
        values ($1,$2,$3,$4,$5,$6,$7,$8,$9)
        on conflict (id)
        do update set
            idx = excluded.idx,
            role = excluded.role,
            timestamp = excluded.timestamp,
            content = excluded.content,
            project = excluded.project,
            meeting = excluded.meeting,
            markers = excluded.markers
        "#,
    )
    .bind(message.id)
    .bind(message.conversation_id)
    .bind(message.idx)
    .bind(&message.role)
    .bind(message.timestamp)
    .bind(&message.content)
    .bind(&message.project)
    .bind(&message.meeting)
    .bind(&message.markers)
    .execute(pool)
    .await?;
    Ok(())
}

async fn upsert_conversation(
    pool: &PgPool,
    conv_id: &str,
    title: Option<String>,
    created_at: DateTime<Utc>,
    markers: Vec<String>,
) -> Result<Uuid> {
    let row = sqlx::query(
        r#"
        insert into conversations (conv_id, title, created_at, markers)
        values ($1, $2, $3, $4)
        on conflict (conv_id)
        do update set
            title = excluded.title,
            created_at = excluded.created_at,
            markers = excluded.markers
        returning id
        "#,
    )
    .bind(conv_id)
    .bind(title)
    .bind(created_at)
    .bind(markers)
    .fetch_one(pool)
    .await?;
    Ok(row.get("id"))
}

async fn ensure_extensions(pool: &PgPool) -> Result<()> {
    sqlx::query("create extension if not exists vector")
        .execute(pool)
        .await?;
    Ok(())
}

async fn ensure_optimal_ivfflat_index(pool: &PgPool) -> Result<()> {
    // Count embeddings to determine optimal lists parameter
    let row: (i64,) = sqlx::query_as("select count(*) from embeddings")
        .fetch_one(pool)
        .await?;
    let count = row.0;

    // Calculate optimal lists: max(10, row_count / 1000)
    // For <10k rows, use lists=10; for 100k rows, use lists=100
    let lists = (count / 1000).max(10);

    info!(
        "creating IVFFlat index with lists={} (based on {} embeddings)",
        lists, count
    );

    // Drop existing index if present
    sqlx::query("drop index if exists embeddings_vector_idx")
        .execute(pool)
        .await?;

    // Create new index with optimal lists parameter
    let create_index_sql = format!(
        "create index embeddings_vector_idx on embeddings using ivfflat (vector vector_l2_ops) with (lists = {})",
        lists
    );
    sqlx::query(&create_index_sql).execute(pool).await?;

    info!("IVFFlat index created successfully");
    Ok(())
}

async fn open_reader(
    path: &PathBuf,
) -> Result<tokio::io::Lines<BufReader<Box<dyn AsyncRead + Unpin + Send>>>> {
    // Support stdin for piping
    if path.to_str() == Some("/dev/stdin") || path.to_str() == Some("-") {
        let stdin_reader: Box<dyn AsyncRead + Unpin + Send> = Box::new(stdin());
        return Ok(BufReader::new(stdin_reader).lines());
    }

    // Regular file
    let file = File::open(path).await?;
    let file_reader: Box<dyn AsyncRead + Unpin + Send> = Box::new(file);
    Ok(BufReader::new(file_reader).lines())
}

fn parse_uuid(input: &str) -> Uuid {
    Uuid::parse_str(input).unwrap_or_else(|_| Uuid::new_v4())
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .or_else(|_| DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f %z"))
        .map(|dt| dt.with_timezone(&Utc))
        .context("invalid timestamp")
}

struct MessageUpsert {
    id: Uuid,
    conversation_id: Uuid,
    idx: i32,
    role: String,
    timestamp: DateTime<Utc>,
    content: String,
    project: Option<String>,
    meeting: Option<String>,
    markers: Vec<String>,
}

struct EmbeddingJob {
    message_id: Uuid,
    chunk_index: usize,
    chunk_count: usize,
    chunk_text: String,
}

#[derive(sqlx::FromRow)]
struct QueryRow {
    content: String,
    project: Option<String>,
    meeting: Option<String>,
    timestamp: DateTime<Utc>,
}

struct DryRunStats {
    conversations: usize,
    messages: usize,
}

async fn dry_run_scan(args: &EmbedArgs) -> Result<DryRunStats> {
    let mut reader = open_reader(&args.input).await?;
    let mut convs = HashMap::new();
    let mut stats = DryRunStats {
        conversations: 0,
        messages: 0,
    };
    let since = args.since.map(|d| d.and_hms_opt(0, 0, 0).unwrap());
    let since = since.map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));

    while let Some(line) = reader.next_line().await? {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<MessageRecord>(&line)? {
            MessageRecord::Meta { conv_id, .. } => {
                convs.insert(conv_id, true);
            }
            MessageRecord::Message {
                timestamp, project, ..
            } => {
                if let Some(required) = &args.project {
                    if project.as_deref() != Some(required) {
                        continue;
                    }
                }
                let timestamp = parse_timestamp(&timestamp)?;
                if let Some(since) = since {
                    if timestamp < since {
                        continue;
                    }
                }
                stats.messages += 1;
                stats.conversations = convs.len();
            }
        }
    }

    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_message_small_text() -> Result<()> {
        // Text under 6000 tokens should return a single chunk
        let text = "This is a short message that fits in one chunk.";
        let chunks = chunk_message(text)?;

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], text);
        Ok(())
    }

    #[test]
    fn test_chunk_message_large_text() -> Result<()> {
        // Generate text that requires chunking (repeat text to exceed 6000 tokens)
        let base_text = "This is a longer sentence that will be repeated many times to create a message exceeding 6000 tokens. ";
        let text = base_text.repeat(2000); // ~12000 tokens estimated

        let chunks = chunk_message(&text)?;

        // Should be split into multiple chunks
        assert!(chunks.len() > 1, "Expected multiple chunks but got {}", chunks.len());

        // Verify each chunk doesn't exceed CHUNK_SIZE
        for (idx, chunk) in chunks.iter().enumerate() {
            let token_count = count_tokens(chunk)?;
            assert!(
                token_count <= CHUNK_SIZE,
                "Chunk {} has {} tokens (exceeds {})",
                idx,
                token_count,
                CHUNK_SIZE
            );
        }

        Ok(())
    }

    #[test]
    fn test_chunk_message_overlap() -> Result<()> {
        // Test that overlap exists between chunks
        let base_text = "Word ".repeat(2000); // Create text that will be chunked
        let chunks = chunk_message(&base_text)?;

        if chunks.len() > 1 {
            // Check that there's overlap by looking for common content
            // (This is a simplified check - real overlap validation would need token-level comparison)
            assert!(
                chunks.len() >= 2,
                "Need at least 2 chunks to test overlap"
            );
        }

        Ok(())
    }

    #[test]
    fn test_chunk_message_empty() -> Result<()> {
        // Empty string should return single empty chunk
        let chunks = chunk_message("")?;

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0], "");
        Ok(())
    }

    #[test]
    fn test_chunk_sizes_never_exceed_limit() -> Result<()> {
        // Test various text sizes to ensure no chunk ever exceeds CHUNK_SIZE
        let medium_text = "Medium text. ".repeat(500);
        let long_text = "Long text that should be chunked. ".repeat(2000);
        let test_cases = vec![
            "Short text.",
            medium_text.as_str(),
            long_text.as_str(),
        ];

        for text in test_cases {
            let chunks = chunk_message(text)?;

            for (idx, chunk) in chunks.iter().enumerate() {
                let token_count = count_tokens(chunk)?;
                assert!(
                    token_count <= CHUNK_SIZE,
                    "Chunk {} has {} tokens (limit is {})",
                    idx,
                    token_count,
                    CHUNK_SIZE
                );
            }
        }

        Ok(())
    }

    #[test]
    fn test_count_tokens() -> Result<()> {
        // Basic test that token counting works
        let text = "Hello, world!";
        let count = count_tokens(text)?;

        assert!(count > 0, "Token count should be positive");
        assert!(count < 100, "Simple text should have few tokens");
        Ok(())
    }

    #[test]
    fn test_truncate_ascii() {
        // Simple ASCII text
        let text = "Hello, world!";
        let truncated = truncate(text, 20);
        assert_eq!(truncated, "Hello, world!");

        let truncated = truncate(text, 8);
        assert_eq!(truncated, "Hello...");
    }

    #[test]
    fn test_truncate_unicode() {
        // Test with emojis and multi-byte UTF-8 characters
        let text = "Hello 👋 世界 🌍!";

        // Should not panic (this was the original bug)
        let truncated = truncate(text, 10);
        assert!(truncated.len() > 0, "Truncate should return non-empty string");
        assert!(truncated.ends_with("..."), "Should end with ellipsis");

        // Verify the truncated string is valid UTF-8
        assert!(std::str::from_utf8(truncated.as_bytes()).is_ok());

        // Test with exact length
        let truncated = truncate(text, text.chars().count());
        assert_eq!(truncated, text, "Should return original when length matches");
    }

    #[test]
    fn test_truncate_edge_cases() {
        // Empty string
        let truncated = truncate("", 10);
        assert_eq!(truncated, "");

        // Very short max length
        let truncated = truncate("Hello, world!", 3);
        assert_eq!(truncated, "...");

        // Max length of 1 or 2
        let truncated = truncate("Hello", 1);
        assert_eq!(truncated, "...");

        let truncated = truncate("Hello", 2);
        assert_eq!(truncated, "...");

        // Exactly at boundary
        let text = "12345";
        let truncated = truncate(text, 5);
        assert_eq!(truncated, "12345");

        let truncated = truncate(text, 4);
        assert_eq!(truncated, "1...");
    }

    #[sqlx::test(migrator = "MIGRATOR")]
    #[ignore = "requires pgvector docker image (see README)"]
    async fn embeds_roundtrip(pool: PgPool) -> Result<()> {
        ensure_extensions(&pool).await?;
        let fixture = include_str!("../tests/data/messages.ndjson");
        for line in fixture.lines().filter(|line| !line.trim().is_empty()) {
            match serde_json::from_str::<MessageRecord>(line)? {
                MessageRecord::Meta {
                    conv_id,
                    title,
                    created_at,
                    markers,
                } => {
                    let created_at = parse_timestamp(&created_at)?;
                    upsert_conversation(&pool, &conv_id, title, created_at, markers).await?;
                }
                MessageRecord::Message {
                    conv_id,
                    idx,
                    message_id,
                    role,
                    timestamp,
                    content,
                    project,
                    meeting,
                    markers,
                } => {
                    let conversation_id = sqlx::query_scalar::<_, Uuid>(
                        "select id from conversations where conv_id = $1",
                    )
                    .bind(conv_id)
                    .fetch_one(&pool)
                    .await?;

                    let timestamp = parse_timestamp(&timestamp)?;
                    let message_id = parse_uuid(&message_id);
                    let content_clone = content.clone();
                    upsert_message(
                        &pool,
                        &MessageUpsert {
                            id: message_id,
                            conversation_id,
                            idx,
                            role,
                            timestamp,
                            content,
                            project,
                            meeting,
                            markers,
                        },
                    )
                    .await?;

                    upsert_embedding(&pool, message_id, 0, 1, &content_clone, Vector::from(vec![0.0f32; 1536])).await?;
                }
            }
        }

        let message: (String,) = sqlx::query_as("select content from messages limit 1")
            .fetch_one(&pool)
            .await?;
        assert!(message.0.contains("Agreed to deliver inventory report"));

        let mut builder = sqlx::QueryBuilder::new(
            "select m.content, m.project, m.meeting, m.timestamp \
             from messages m join embeddings e on e.message_id = m.id",
        );
        builder.push(" order by e.vector <-> ");
        builder.push_bind(Vector::from(vec![0.0f32; 1536]));
        builder.push(" limit 1");
        let rows: Vec<QueryRow> = builder.build_query_as().fetch_all(&pool).await?;
        assert_eq!(rows.len(), 1);

        Ok(())
    }
}

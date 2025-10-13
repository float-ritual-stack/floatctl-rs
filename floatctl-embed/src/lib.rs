use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use clap::Args;
use floatctl_core::ndjson::MessageRecord;
use pgvector::Vector;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tracing::{info, warn};
use uuid::Uuid;

static MODEL_NAME: &str = "text-embedding-3-small";

pub static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("../migrations");

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

pub async fn run_embed(args: EmbedArgs) -> Result<()> {
    dotenvy::dotenv().ok();

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
        .max_connections(5)
        .connect(&database_url)
        .await?;
    ensure_extensions(&pool).await?;
    MIGRATOR.run(&pool).await?;

    let mut reader = open_reader(&args.input).await?;
    let mut conv_lookup: HashMap<String, Uuid> = HashMap::new();
    let mut pending = Vec::with_capacity(args.batch_size);
    let openai = OpenAiClient::new(api_key)?;
    let since = args.since.map(|d| d.and_hms_opt(0, 0, 0).unwrap());
    let since = since.map(|dt| DateTime::<Utc>::from_naive_utc_and_offset(dt, Utc));
    let mut processed = 0usize;

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
                    upsert_conversation(&pool, &conv_id, title, created_at, markers).await?;
                conv_lookup.insert(conv_id, conv_uuid);
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
                upsert_message(
                    &pool,
                    &MessageUpsert {
                        id: message_uuid,
                        conversation_id,
                        idx,
                        role,
                        timestamp,
                        content: content.clone(),
                        project: project.clone(),
                        meeting,
                        markers,
                    },
                )
                .await?;

                if content.trim().is_empty() {
                    continue;
                }
                pending.push(EmbeddingJob {
                    message_id: message_uuid,
                    content,
                });
                processed += 1;

                if pending.len() >= args.batch_size {
                    flush_embeddings(&pool, &openai, &mut pending).await?;
                }
            }
        }
    }

    if !pending.is_empty() {
        flush_embeddings(&pool, &openai, &mut pending).await?;
    }

    info!("embedded {} messages", processed);

    Ok(())
}

pub async fn run_query(args: QueryArgs) -> Result<()> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL not set")?;
    let api_key = std::env::var("OPENAI_API_KEY").context("OPENAI_API_KEY not set")?;
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    ensure_extensions(&pool).await?;
    MIGRATOR.run(&pool).await?;

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
        #[derive(serde::Serialize)]
        struct EmbeddingRequest<'a> {
            model: &'a str,
            input: &'a [String],
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
            .await?
            .error_for_status()?
            .json::<EmbeddingResponse>()
            .await?;

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

async fn flush_embeddings(
    pool: &PgPool,
    openai: &OpenAiClient,
    pending: &mut Vec<EmbeddingJob>,
) -> Result<()> {
    let batch: Vec<String> = pending.iter().map(|job| job.content.clone()).collect();
    let vectors = openai.embed_batch(&batch).await?;

    for (job, vector) in pending.drain(..).zip(vectors) {
        upsert_embedding(pool, job.message_id, vector).await?;
    }
    Ok(())
}

async fn upsert_embedding(pool: &PgPool, message_id: Uuid, vector: Vector) -> Result<()> {
    let dim = vector.as_slice().len() as i32;
    sqlx::query(
        r#"
        insert into embeddings (message_id, model, dim, vector)
        values ($1, $2, $3, $4)
        on conflict (message_id)
        do update set model = excluded.model,
                      dim = excluded.dim,
                      vector = excluded.vector
        "#,
    )
    .bind(message_id)
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

async fn open_reader(path: &PathBuf) -> Result<tokio::io::Lines<BufReader<File>>> {
    let file = File::open(path).await?;
    Ok(BufReader::new(file).lines())
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
    content: String,
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

                    upsert_embedding(&pool, message_id, Vector::from(vec![0.0f32; 1536])).await?;
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

# floatctl-embed

Optional vector embedding and semantic search integration for conversation archives.

## Purpose

`floatctl-embed` extends `floatctl-core` with semantic search capabilities by ingesting conversation messages into a PostgreSQL database with pgvector embeddings. This enables natural language queries over your entire conversation history.

## Features

- **pgvector Integration**: Store conversations in Postgres with vector embeddings
- **OpenAI Embeddings**: Uses `text-embedding-3-small` model (1536 dimensions)
- **Batch Processing**: Efficient embedding generation with configurable batch sizes
- **Marker-based Filtering**: Filter by project, meeting, date ranges
- **Semantic Search**: Find relevant messages using natural language queries
- **NDJSON Input**: Reads message records from `floatctl-core` output
- **Upsert Operations**: Safe re-ingestion without duplicates

## Database Setup

### Requirements

- PostgreSQL with pgvector extension
- OpenAI API key for embeddings

### Using Docker

```bash
# Run PostgreSQL with pgvector
docker run --rm \
  --name pgvector \
  -e POSTGRES_PASSWORD=postgres \
  -p 5433:5432 \
  ankane/pgvector

# Or use Docker Compose
docker-compose up -d
```

### Run Migrations

The migrations create the database schema automatically on first run. You can also run them manually:

```bash
cargo sqlx migrate run --source migrations/
```

## Configuration

### Environment Variables

Create a `.env` file in the project root:

```bash
DATABASE_URL="postgresql://postgres:postgres@localhost:5433/floatctl"
OPENAI_API_KEY="sk-..."
```

Or export them:

```bash
export DATABASE_URL="postgresql://postgres:postgres@localhost:5433/floatctl"
export OPENAI_API_KEY="sk-..."
```

### Database Schema

The system creates three tables:

**`conversations`**
```sql
- id: uuid (PK)
- conv_id: text (unique)
- title: text
- created_at: timestamptz
- markers: text[]
```

**`messages`**
```sql
- id: uuid (PK)
- conversation_id: uuid (FK)
- idx: int
- role: text
- timestamp: timestamptz
- content: text
- project: text
- meeting: text
- markers: text[]
```

**`embeddings`**
```sql
- message_id: uuid (PK, FK)
- model: text
- dim: int
- vector: vector(1536)
```

Indexes:
- `messages_project_idx` on `project`
- `messages_timestamp_idx` on `timestamp`
- `embeddings_vector_idx` (IVFFlat) on `vector`

## Commands

### `embed`

Ingest messages from NDJSON into the database with embeddings.

```bash
floatctl embed --in messages.ndjson

# Options
--in <PATH>          Input NDJSON file (required)
--project <NAME>     Only ingest messages from this project
--since <DATE>       Only ingest messages after this date (YYYY-MM-DD)
--batch-size <N>     Embedding batch size (default: 32)
--dry-run            Preview what would be ingested without writing
```

**Example workflow:**
```bash
# Step 1: Extract conversations to NDJSON
floatctl full-extract --in export.json --out ./archive/

# Step 2: Ingest the aggregate messages file
floatctl embed --in archive/messages.ndjson --project myproject

# Or ingest from a specific conversation
floatctl embed --in archive/2024-12-04-my-conversation/2024-12-04-my-conversation.ndjson
```

**Dry run example:**
```bash
floatctl embed --in messages.ndjson --project rangle --since 2024-12-01 --dry-run
# Output: "dry-run: would embed 347 messages across 12 conversations (filtered)"
```

### `query`

Semantic search over ingested messages.

```bash
floatctl query "what did we decide about the API design?"

# Options
<QUERY>              Natural language search query (required)
--project <NAME>     Filter by project marker
--days <N>           Limit to last N days
--limit <N>          Max results to return (default: 10)
```

**Examples:**
```bash
# Basic query
floatctl query "authentication implementation"

# Filter by project
floatctl query "API endpoints" --project rangle/pharmacy

# Recent messages only
floatctl query "deployment issues" --days 7

# Limit results
floatctl query "refactoring" --limit 5 --project myproject
```

**Query output format:**
```
[2024-12-04 15:32:10] project=rangle/pharmacy meeting=Some("daily_scrum")
Discussed authentication flow for the new API. We decided to use JWT tokens
with refresh tokens stored in httpOnly cookies...

[2024-12-04 10:15:33] project=rangle/pharmacy meeting=None
Working on the authentication middleware. Need to handle token expiration...
```

## Integration with Other Crates

### With `floatctl-core`

`floatctl-embed` consumes NDJSON message records produced by `floatctl-core`:

```rust
use floatctl_core::ndjson::MessageRecord;

// floatctl-core produces these records
MessageRecord::Meta {
    conv_id: String,
    title: Option<String>,
    created_at: String,
    markers: Vec<String>,
}

MessageRecord::Message {
    conv_id: String,
    idx: i32,
    message_id: String,
    role: String,
    timestamp: String,
    content: String,
    project: Option<String>,
    meeting: Option<String>,
    markers: Vec<String>,
}
```

### With `floatctl-cli`

The CLI integrates both commands when built with the `embed` feature:

```bash
# Build with embedding support
cargo build --release --features embed

# Then use embed/query commands
./target/release/floatctl-cli embed --in messages.ndjson
./target/release/floatctl-cli query "search term"
```

## Programmatic Usage

```rust
use floatctl_embed::{EmbedArgs, QueryArgs, run_embed, run_query};

// Ingest messages
let embed_args = EmbedArgs {
    input: PathBuf::from("messages.ndjson"),
    since: None,
    project: Some("myproject".to_string()),
    batch_size: 32,
    dry_run: false,
};
run_embed(embed_args).await?;

// Query messages
let query_args = QueryArgs {
    query: "authentication implementation".to_string(),
    project: Some("myproject".to_string()),
    limit: 10,
    days: Some(7),
};
run_query(query_args).await?;
```

## Performance Characteristics

| Operation | Speed | Notes |
|-----------|-------|-------|
| **Embedding batch (32 msgs)** | ~1-2s | OpenAI API call |
| **Database insert (batch)** | ~50ms | Upsert with conflict handling |
| **Vector search** | <100ms | IVFFlat index on 10K+ vectors |
| **Dry run scan** | ~500MB/s | No embeddings, just counting |

**Recommended batch sizes:**
- Small datasets (<1000 msgs): `--batch-size 50`
- Large datasets (10K+ msgs): `--batch-size 32` (default, respects rate limits)
- Re-ingestion: `--batch-size 100` (faster, already have embeddings)

## Error Handling

The system handles:
- **Missing environment variables**: Clear error messages for `DATABASE_URL` and `OPENAI_API_KEY`
- **Malformed records**: Skips invalid NDJSON lines with warnings
- **Orphan messages**: Messages without prior metadata record are skipped
- **Network failures**: Retries OpenAI API calls (handled by reqwest)
- **Database conflicts**: Upserts prevent duplicate key errors

## Testing

### Unit Tests

```bash
cargo test -p floatctl-embed
```

### Integration Tests

**Requirements**: Running PostgreSQL with pgvector

```bash
# Start test database
docker run --rm \
  -e POSTGRES_PASSWORD=postgres \
  -p 5433:5432 \
  ankane/pgvector

# Run tests
cargo test -p floatctl-embed -- --ignored
```

The integration test (`embeds_roundtrip`) validates:
- Conversation upsert
- Message upsert with metadata
- Embedding storage
- Vector similarity search

## Common Workflows

### Daily Ingestion

```bash
#!/bin/bash
# Cron job to ingest new conversations daily

# Export from Claude/ChatGPT, then:
floatctl full-extract --in latest-export.json --out ~/conversations/
floatctl embed --in ~/conversations/messages.ndjson --since $(date -d "yesterday" +%Y-%m-%d)
```

### Project-Specific Search

```bash
# Ingest project conversations
floatctl embed --in messages.ndjson --project rangle/pharmacy

# Query within project
floatctl query "authentication flow" --project rangle/pharmacy --days 30
```

### Re-ingestion

Safe to re-run - upserts prevent duplicates:

```bash
# Re-ingest everything (updates changed messages)
floatctl embed --in messages.ndjson
```

## Troubleshooting

### "DATABASE_URL not set"

Ensure `.env` file exists with correct connection string:
```bash
DATABASE_URL="postgresql://postgres:postgres@localhost:5433/floatctl"
```

### "OPENAI_API_KEY not set"

Add your OpenAI API key to `.env`:
```bash
OPENAI_API_KEY="sk-..."
```

Get a key from: https://platform.openai.com/api-keys

### "connection refused"

Check that PostgreSQL is running:
```bash
docker ps | grep pgvector
```

### "extension vector does not exist"

Use the `ankane/pgvector` Docker image, not plain PostgreSQL:
```bash
docker run --rm -p 5433:5432 -e POSTGRES_PASSWORD=postgres ankane/pgvector
```

### Slow queries

If queries are slow:
1. Check index exists: `\d embeddings` in psql
2. Increase `lists` parameter in migration for larger datasets
3. Consider filtering by `--project` or `--days`

### OpenAI rate limits

Reduce batch size to avoid rate limits:
```bash
floatctl embed --in messages.ndjson --batch-size 16
```

## Architecture Details

### Embedding Pipeline

1. Read NDJSON line-by-line
2. Parse `MessageRecord::Meta` → upsert conversation
3. Parse `MessageRecord::Message` → upsert message
4. Accumulate messages in batch buffer
5. When batch is full (or end of file):
   - Call OpenAI embeddings API
   - Upsert embeddings to database
6. Continue until file exhausted

### Query Pipeline

1. Parse user query string
2. Call OpenAI embeddings API for query vector
3. Build SQL with filters (project, date range)
4. Execute vector similarity search (cosine distance `<->`)
5. Return top N results ordered by similarity
6. Format output with timestamps and metadata

### Vector Search

Uses pgvector's IVFFlat index for approximate nearest neighbor search:
- **Index type**: `ivfflat` with L2 distance
- **Lists parameter**: 100 (adjust based on dataset size)
- **Distance operator**: `<->` (L2 distance, equivalent to cosine for normalized vectors)

For exact search on small datasets, drop the index.

## See Also

- **[floatctl-cli](../floatctl-cli/README.md)**: CLI interface with embed commands
- **[floatctl-core](../floatctl-core/README.md)**: Core library for conversation processing
- **[Main README](../README.md)**: Full project documentation
- **[ARCHITECTURE.md](../ARCHITECTURE.md)**: System design

## Dependencies

- `sqlx`: Database access with compile-time query checking
- `pgvector`: Rust bindings for pgvector types
- `reqwest`: HTTP client for OpenAI API
- `tokio`: Async runtime
- `floatctl-core`: Message record types
- `chrono`: Date/time handling
- `uuid`: Unique identifiers

## License

MIT - See LICENSE file

# floatctl Workspace

This branch rebuilds the `floatctl` toolchain as a Cargo workspace with three crates:

- `floatctl-core`: shared models, marker extraction, and NDJSON utilities.
- `floatctl-cli`: end-user binary (`floatctl`) with `split`, `embed`, and `query` subcommands.
- `floatctl-embed`: embedding + Postgres ingestion helpers, including migrations and tests.

## Quick Start

```bash
cp .env.example .env
# edit DATABASE_URL (Supabase or local pgvector) and OPENAI_API_KEY

cargo build

# split raw exports into Markdown/JSON/NDJSON
cargo run -p floatctl-cli -- split --in conversations.ndjson --out conv_out
# add `--no-progress` if you prefer plain logging

# ingest message NDJSON into Postgres + pgvector
cargo run -p floatctl-cli -- embed --in conv_out/messages.ndjson --project rangle/pharmacy

# semantic search the archive
cargo run -p floatctl-cli -- query "what did I agree to doing?" --project rangle/pharmacy --days 7
```

## Database Setup

- Bring up a pgvector-enabled database (e.g. `docker run --rm -e POSTGRES_PASSWORD=postgres -p 5433:5432 ankane/pgvector`).
- Point `DATABASE_URL` to the instance, then run `cargo sqlx migrate run -p floatctl-embed` or let `floatctl embed` run migrations automatically.

## Testing

- `cargo fmt && cargo clippy -- -D warnings`
- `cargo test -p floatctl-core`
- `cargo test -p floatctl-embed` (requires pgvector; see ignored test notes)

Golden fixtures live under `floatctl-embed/tests/data/` and mirror the NDJSON shape produced by `floatctl split`.

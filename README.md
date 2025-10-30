# floatctl-rs

**Fast, streaming conversation archive processor for Claude and ChatGPT exports**

`floatctl` is a Rust-based toolchain for processing, organizing, and semantically searching LLM conversation archives. It handles large exports (100MB+) with O(1) memory usage through streaming JSON parsing.

## Features

### 🚀 Streaming Performance
- **O(1) memory usage**: Process 772MB files in ~7 seconds
- **Custom JSON array streamer**: Handles both JSON arrays and NDJSON
- **Progress tracking**: Real-time progress bars with conversation titles

### 📁 Organization
- **Folder-per-conversation**: Clean directory structure with date-prefixed slugs
- **Artifact extraction**: Automatically extracts code, SVG, HTML, React components with correct extensions
- **Multiple formats**: Generate Markdown (with YAML frontmatter), JSON, and NDJSON simultaneously
- **Smart slug generation**: `YYYY-MM-DD-slugified-title` with automatic deduplication

### 🔄 Format Support
- **Anthropic/Claude exports**: Native support for `chat_messages` format
- **ChatGPT exports**: Native support for `messages` format
- **Auto-detection**: Automatically detects and handles both formats

### 🔍 Semantic Search (optional)
- **pgvector integration**: Store conversations in Postgres with vector embeddings
- **OpenAI embeddings**: Generate embeddings for semantic search
- **Marker-based filtering**: Filter by project, meeting, date ranges

## Quick Start

### Installation

**Local build:**
```bash
git clone https://github.com/float-ritual-stack/floatctl-rs.git
cd floatctl-rs
cargo build --release
```

The binary will be at `./target/release/floatctl`

**Global installation (recommended):**
```bash
# Install as global command
cargo install --path floatctl-cli --features embed

# Create global config
mkdir -p ~/.floatctl
cp .env.example ~/.floatctl/.env
# Edit ~/.floatctl/.env with your credentials

# Use from anywhere
floatctl query "search term"
```

📖 **See [INSTALL.md](./INSTALL.md) for complete global installation guide**

### Basic Usage

**Local build:**
```bash
# One-command extraction: JSON/ZIP → organized folders with artifacts
./target/release/floatctl full-extract --in conversations.json

# Convert to NDJSON (for faster re-processing)
./target/release/floatctl ndjson --in conversations.json --out conversations.ndjson

# Split with custom formats
./target/release/floatctl split --in conversations.ndjson --format md,json
```

**After global install:**
```bash
# Works from anywhere! No --out needed (defaults to ~/.floatctl/conversation-exports)
floatctl full-extract --in ~/Downloads/export.json

# Query semantic search
floatctl query "error handling patterns" --limit 5

# Embed conversations
floatctl embed --in messages.ndjson

# Explode NDJSON into individual files (parallel)
floatctl explode --in conversations.ndjson
```

## Output Structure

```
archive/
├── 2024-12-03-conversation/
│   ├── 2024-12-03-conversation.md       # Markdown with YAML frontmatter
│   ├── 2024-12-03-conversation.json     # Raw conversation JSON
│   ├── 2024-12-03-conversation.ndjson   # Message-level records
│   └── artifacts/                       # Extracted artifacts (if any)
│       ├── 00-component-name.jsx        # React components
│       ├── 01-diagram.svg               # SVG graphics
│       └── 02-document.md               # Markdown docs
├── 2024-12-04-implementing-feature-x/
│   └── ...
└── messages.ndjson                      # Aggregate of all messages
```

## Commands

### `full-extract`
**The recommended command** - handles everything in one step:
- Auto-detects JSON array vs NDJSON
- Converts to NDJSON if needed (with temp file cleanup)
- Extracts to organized folder structure
- Extracts artifacts with correct file extensions

```bash
floatctl full-extract --in export.json --out ./archive/
```

### `ndjson`
Convert large JSON arrays to NDJSON (streaming, memory-efficient):

```bash
floatctl ndjson --in conversations.json --out conversations.ndjson
```

**Performance**: 772MB → 756MB in ~4 seconds

### `split`
Process conversations into multiple output formats:

```bash
floatctl split --in conversations.ndjson --out ./archive/
```

Options:
- `--format md,json,ndjson` - Choose output formats
- `--dry-run` - Preview without writing
- `--no-progress` - Disable progress bar

### `explode`
Split NDJSON into individual files (with parallel writes):

```bash
# Explode conversations
floatctl explode --in conversations.ndjson --out ./individual/

# Extract messages from a single conversation
floatctl explode --in conversation.json --out messages.ndjson --messages
```

### `evna` (MCP Server Management)
Manage evna-next MCP server integration with Claude Desktop:

```bash
# Install evna-next MCP server
floatctl evna install --path ./evna-next

# Check installation status
floatctl evna status

# Uninstall MCP server
floatctl evna uninstall
```

Options for `install`:
- `--path <PATH>` - Path to evna-next directory (defaults to ../evna-next)
- `--force` - Force reinstall even if already configured

See [Evna-Next Integration](#evna-next-integration) for more details.

## Workspace Structure

This is a Cargo workspace with three crates:

- **`floatctl-core`**: Core functionality (streaming, parsing, rendering)
- **`floatctl-cli`**: CLI binary with all commands
- **`floatctl-embed`**: Optional vector search with pgvector (feature-gated)

## Semantic Search (Optional)

Enable the `embed` feature to use pgvector integration:

```bash
# Setup
cp .env.example .env
# Edit DATABASE_URL and OPENAI_API_KEY

# Ingest conversations
cargo run -p floatctl-cli --features embed -- embed \
  --in archive/messages.ndjson \
  --project my-project

# Query
cargo run -p floatctl-cli --features embed -- query \
  "what did we decide about the API design?" \
  --project my-project \
  --days 7
```

### Database Setup

```bash
# Run pgvector with Docker
docker run --rm \
  -e POSTGRES_PASSWORD=postgres \
  -p 5433:5432 \
  ankane/pgvector

# Run migrations
cargo sqlx migrate run -p floatctl-embed
```

### Evna-Next Integration

`floatctl` includes commands to easily install and manage the evna-next MCP server for Claude Desktop.

```bash
# Install evna-next as MCP server in Claude Desktop
floatctl evna install --path ./evna-next

# Check installation status
floatctl evna status

# Uninstall if needed
floatctl evna uninstall
```

**What it does:**
- Automatically configures Claude Desktop's `claude_desktop_config.json`
- Sets up the evna-next MCP server with correct paths and environment
- Validates the installation and checks for required .env configuration

**Requirements:**
- evna-next directory with package.json
- Bun runtime installed
- .env configured in evna-next directory

After installation, restart Claude Desktop to load the MCP server.

## R2 Sync Daemon Management

`floatctl` provides commands to manage R2 sync daemons that automatically backup daily notes and dispatch content to Cloudflare R2 storage.

### Commands

```bash
# Check daemon status (PID, last sync time, transfer stats)
floatctl sync status

# Check specific daemon
floatctl sync status --daemon daily
floatctl sync status --daemon dispatch

# Manually trigger sync
floatctl sync trigger --daemon daily --wait

# View daemon logs (formatted with emoji indicators and human-friendly timestamps)
floatctl sync logs daily --lines 50

# Start/stop daemons
floatctl sync start --daemon daily
floatctl sync stop --daemon all
```

**Human-Friendly Timestamps**: All command output displays timestamps in Toronto EST 12-hour format (e.g., `oct 30 02:48pm`) for easy readability. JSONL logs store UTC ISO 8601 for machine parsing.

### Unified Logging Architecture

All R2 sync operations emit structured JSONL events to `~/.floatctl/logs/{daemon}.jsonl`:

**Event Types:**
- `daemon_start` - Daemon process launched (includes PID, config)
- `file_change` - File modification detected (triggers debounce)
- `sync_start` - Sync operation initiated (manual/auto/cron)
- `sync_complete` - Sync finished (includes files transferred, bytes, duration, rate)
- `sync_error` - Sync failure with error context

**Formatted Output Example** (`floatctl sync logs daily`):
```
📝 Last 5 events from daily daemon:

🚀 [oct 30 01:24pm] Daemon started (PID: 45011)
   Config: {"watch_dir": "/path/to/daily", "debounce_ms": "300000"}
▶️  [oct 30 02:38pm] Sync started (trigger: auto)
✅ [oct 30 02:38pm] Sync completed in 1000ms
   Files: 1, Bytes: 7009, Rate: 5392 bytes/sec
```

**Raw JSONL Storage** (`~/.floatctl/logs/daily.jsonl`):
```json
{"event":"daemon_start","timestamp":"2025-10-30T18:24:51Z","daemon":"daily","pid":45011,"config":{"watch_dir":"/path/to/daily","debounce_ms":"300000"}}
{"event":"file_change","timestamp":"2025-10-30T18:25:15Z","daemon":"daily","path":"/path/to/2025-10-30.md","debounce_ms":300000}
{"event":"sync_start","timestamp":"2025-10-30T18:30:15Z","daemon":"daily","trigger":"auto"}
{"event":"sync_complete","timestamp":"2025-10-30T18:30:17Z","daemon":"daily","success":true,"files_transferred":1,"bytes_transferred":3108,"duration_ms":1500,"transfer_rate_bps":2072,"error_message":null}
```

### Querying Logs with jq

**Last sync time (fast - reads last line only):**
```bash
tail -1 ~/.floatctl/logs/daily.jsonl | jq '.timestamp'
```

**All syncs in last 24 hours:**
```bash
jq 'select(.timestamp > (now - 86400 | strftime("%Y-%m-%dT%H:%M:%SZ")))' \
  ~/.floatctl/logs/daily.jsonl
```

**Average transfer stats:**
```bash
jq -s 'map(select(.event == "sync_complete")) | {
  avg_bytes: (map(.bytes_transferred) | add / length),
  avg_duration_ms: (map(.duration_ms) | add / length),
  total_files: (map(.files_transferred) | add)
}' ~/.floatctl/logs/daily.jsonl
```

**Error analysis:**
```bash
jq -s 'group_by(.error_type) | map({
  error: .[0].error_type,
  count: length
})' ~/.floatctl/logs/daily.jsonl
```

**Files changed most frequently:**
```bash
jq -s 'map(select(.event == "file_change")) |
  group_by(.path) |
  map({file: .[0].path, changes: length}) |
  sort_by(-.changes)' ~/.floatctl/logs/daily.jsonl
```

### Log Schema

See `floatctl-core/src/sync_events.rs` for the complete Rust type definitions. All events include:
- `timestamp` - UTC timestamp (ISO 8601)
- `daemon` - Daemon name ("daily" or "dispatch")
- `event` - Event type discriminator

Events are type-safe in Rust (serde validation) and machine-parseable via jq.

## Performance

### Microbenchmarks (criterion)

Tested with 3-conversation fixture on Apple M-series:

| Operation | Time | Notes |
|-----------|------|-------|
| `RawValueStream` | 22 µs | Raw JSON streaming (no parsing) |
| `ConvStream` | 35 µs | Full conversation parsing |
| `Conversation::from_export` | 4.9 µs | Parse single conversation |

Run benchmarks yourself:
```bash
cargo bench -p floatctl-core
```

### Large File Performance

Informal measurements from development (772MB file, 2912 conversations):

| Operation | Time | Memory |
|-----------|------|--------|
| Convert to NDJSON | ~4s | <100MB |
| Full extract | ~7s | <100MB |
| Split (dry-run) | ~1s | <100MB |

See [LESSONS.md](LESSONS.md) for detailed performance analysis.

## Development

```bash
# Build
cargo build

# Test
cargo test

# Lint
cargo clippy -- -D warnings
cargo fmt

# Build optimized binary
cargo build --release
```

## Documentation

- **[ARCHITECTURE.md](ARCHITECTURE.md)** - System design and architecture
- **[LESSONS.md](LESSONS.md)** - Performance lessons learned
- **[CLAUDE.md](CLAUDE.md)** - Claude Code integration guide

## Best Practices

### For Large Files (>100MB)
1. Convert to NDJSON first (one-time): `floatctl ndjson`
2. Process from NDJSON: `floatctl split --in conversations.ndjson`
3. NDJSON can be re-processed instantly (no re-parsing)

### For Daily Workflow
Use `full-extract` - it handles everything automatically:
```bash
floatctl full-extract --in latest-export.json --out ~/conversations/
```

### For Programmatic Access
Use the `floatctl-core` crate directly:
```rust
use floatctl_core::{ConvStream, Conversation};

let stream = ConvStream::from_path("conversations.json")?;
for result in stream {
    let conv: Conversation = result?;
    // Process conversation
}
```

## License

MIT License - see LICENSE file for details

## Contributing

Issues and pull requests welcome at: https://github.com/float-ritual-stack/floatctl-rs

---

**Note**: This is a complete rewrite of the original `floatctl` with focus on streaming performance and artifact extraction. For the previous version, see the `legacy` branch.

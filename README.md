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

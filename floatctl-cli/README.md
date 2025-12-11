# floatctl-cli

Command-line interface for conversation archive processing.

## Purpose

`floatctl-cli` is the user-facing binary that provides commands for processing conversation exports. It wraps the `floatctl-core` library with a clean CLI interface.

## Commands

### `full-extract`
One-command extraction workflow (recommended).

```bash
floatctl full-extract --in conversations.json --out ./archive/

# Options
--in <PATH>          Input file (JSON array or NDJSON)
--out <DIR>          Output directory (default: conv_out)
--format <LIST>      Output formats: md,json,ndjson (default: all)
--dry-run            Preview without writing files
--no-progress        Disable progress bar
--keep-ndjson        Keep intermediate NDJSON file
```

### `ndjson`
Convert JSON arrays to NDJSON format.

```bash
floatctl ndjson --in conversations.json --out conversations.ndjson

# Options
--in <PATH>          Input file (required)
--out <PATH>         Output file (optional, defaults to stdout)
--canonical          Pretty-print output (slower, larger files)
```

### `split`
Process conversations into multiple formats.

```bash
floatctl split --in conversations.ndjson --out ./archive/

# Options
--in <PATH>          Input file (default: conversations.ndjson)
--out <DIR>          Output directory (default: conv_out)
--format <LIST>      Formats: md,json,ndjson (default: all)
--dry-run            Preview without writing
--no-progress        Disable progress bar
```

### `explode`
Split NDJSON into individual files (parallel processing).

```bash
# Explode conversations
floatctl explode --in conversations.ndjson --out ./individual/

# Extract messages from one conversation
floatctl explode --in conv.json --out messages.ndjson --messages

# Options
--in <PATH>          Input file (required)
--out <DIR|PATH>     Output directory or file (required)
--messages           Extract messages instead of conversations
```

### `embed` (optional feature)
Ingest conversations into pgvector database.

```bash
floatctl embed --in messages.ndjson --project my-project

# Options
--in <PATH>          Input NDJSON file (required)
--project <NAME>     Project filter
--batch-size <N>     Batch size for embedding (default: 50)
```

**Requires**: `--features embed` during build

### `query` (optional feature)
Semantic search over archived conversations.

```bash
floatctl query "what did we discuss about the API?" --project my-project --days 7

# Options
<QUERY>              Search query (required)
--project <NAME>     Filter by project
--days <N>           Limit to last N days
--limit <N>          Max results (default: 10)
```

**Requires**: `--features embed` during build

## Building

### Standard Build
```bash
# Development
cargo build -p floatctl-cli

# Release (optimized)
cargo build --release -p floatctl-cli
```

### With Embedding Support
```bash
cargo build --release -p floatctl-cli --features embed
```

## Installation

### From Source
```bash
git clone https://github.com/float-ritual-stack/floatctl-rs.git
cd floatctl-rs
cargo install --path floatctl-cli
```

### Direct Run
```bash
cargo run -p floatctl-cli -- full-extract --in export.json --out ./archive/
```

## Configuration

### Environment Variables

For `embed` and `query` commands:
```bash
export DATABASE_URL="postgresql://user:pass@localhost/db"
export OPENAI_API_KEY="sk-..."
```

Or use `.env` file:
```bash
cp .env.example .env
# Edit .env with your credentials
```

## Output Structure

```
archive/
├── YYYY-MM-DD-conversation-title/
│   ├── YYYY-MM-DD-conversation-title.md       # Markdown
│   ├── YYYY-MM-DD-conversation-title.json     # Raw JSON
│   ├── YYYY-MM-DD-conversation-title.ndjson   # Message records
│   └── artifacts/                              # Extracted artifacts
│       ├── 00-component.jsx
│       ├── 01-diagram.svg
│       └── 02-document.md
└── messages.ndjson                             # All messages aggregate
```

## Examples

### Daily Workflow
```bash
# Export from Claude/ChatGPT, then:
floatctl full-extract --in latest-export.json --out ~/conversations/
```

### Large File Processing
```bash
# Step 1: Convert to NDJSON (one-time, ~4s for 772MB)
floatctl ndjson --in huge-export.json --out archive.ndjson

# Step 2: Process from NDJSON (instant re-processing)
floatctl split --in archive.ndjson --out ./organized/
```

### Markdown-Only Output
```bash
floatctl full-extract --in export.json --out ./docs/ --format md
```

### With Embedding
```bash
# Build with feature
cargo build --release --features embed

# Extract conversations
./target/release/floatctl full-extract --in export.json --out ./archive/

# Ingest for search
./target/release/floatctl embed --in archive/messages.ndjson --project myproject

# Query
./target/release/floatctl query "API design decisions" --project myproject
```

## Performance

| Command | 772MB File (2912 convos) | Memory |
|---------|--------------------------|--------|
| `full-extract` | ~7 seconds | <100MB |
| `ndjson` | ~4 seconds | <100MB |
| `split` (NDJSON) | ~1 second | <100MB |
| `explode` (parallel) | ~2 seconds | ~200MB |

## Exit Codes

- `0`: Success
- `1`: General error
- `2`: Invalid arguments
- `3`: File not found or I/O error

## Troubleshooting

### Progress Bar Not Showing
Use `--no-progress` flag for plain logging output.

### Large File Hangs
First convert to NDJSON:
```bash
floatctl ndjson --in huge.json --out huge.ndjson
```

### Artifacts Not Extracted
Check that conversations contain `tool_use` blocks with `name: "artifacts"`. Only Anthropic exports with artifacts will have these.

## See Also

- **[floatctl-core](../floatctl-core/README.md)**: Core library documentation
- **[floatctl-embed](../floatctl-embed/README.md)**: Embedding system details
- **[Main README](../README.md)**: Full project documentation
- **[ARCHITECTURE.md](../ARCHITECTURE.md)**: System design

## License

MIT - See LICENSE file

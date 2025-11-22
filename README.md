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

### 📌 Context Capture (ctx command)
- **Instant-return capture**: Queue context markers locally (<50ms)
- **Background sync**: Automatic flush to remote server every 30s
- **Network resilience**: Queues locally when SSH fails, auto-retries
- **Multi-line support**: JSON escaping prevents SSH pipe breakage
- **Claude Code integration**: Hook captures ctx:: markers without timeouts

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

# Context capture (instant queue + background sync)
floatctl ctx "your context message here"
echo "multi-line message" | floatctl ctx

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

# Start evna as remote MCP server (Supergateway + ngrok)
floatctl evna remote --path ./evna
```

Options for `install`:
- `--path <PATH>` - Path to evna-next directory (defaults to ../evna-next)
- `--force` - Force reinstall even if already configured

Options for `remote`:
- `--path <PATH>` - Path to evna directory (defaults to ../evna)
- `--port <PORT>` - Supergateway SSE server port (default: 3100)
- `--no-tunnel` - Skip ngrok tunnel (only start Supergateway)
- `--ngrok-token <TOKEN>` - ngrok authtoken (or set EVNA_NGROK_AUTHTOKEN in .env)
- `--ngrok-domain <DOMAIN>` - Reserved ngrok domain (or set EVNA_NGROK_DOMAIN in .env)

See [Evna-Next Integration](#evna-next-integration) for more details.

### `claude` (Claude Code Session Logs)
Query and analyze Claude Code session logs:

```bash
# List recent sessions (agent sessions excluded by default)
floatctl claude list

# Include agent sessions
floatctl claude list --include-agents

# Filter by project
floatctl claude list --project float-hub

# Extract recent context for evna
floatctl claude recent-context --limit 20

# Pretty-print a session
floatctl claude show <session-id>

# Show just last 2 messages (timeout visibility)
floatctl claude show <session-id> --last 2 --no-tools
```

See [Claude Code Session Log Querying](#claude-code-session-log-querying) for more details.

### `bridge` (Bridge Maintenance)
Manage bridge files for organizing conversation content:

```bash
# Index :: annotations to create bridge stubs
floatctl bridge index --dir ./daily-notes

# Append content to bridge files
floatctl bridge append --content "text" --project my-project
```

See [Bridge Maintenance](#bridge-maintenance) for more details.

### `script` (Script Management)
Register and run reusable shell scripts:

```bash
# Register a script
floatctl script register ./my-script.sh

# List registered scripts
floatctl script list

# Run a registered script
floatctl script run my-script.sh arg1 arg2
```

See [Script Management](#script-management) for more details.

## Workspace Structure

This is a Cargo workspace with multiple crates:

- **`floatctl-core`**: Core functionality (streaming, parsing, rendering)
- **`floatctl-cli`**: CLI binary with all commands
- **`floatctl-embed`**: Optional vector search with pgvector (feature-gated)
- **`floatctl-claude`**: Claude Code session log parsing and querying
- **`floatctl-bridge`**: Bridge file management for annotation-based organization
- **`floatctl-script`**: Script registration and execution management

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

# Start evna as remote MCP server (for Claude Code or external access)
floatctl evna remote --path ./evna
```

**What it does:**
- `install`: Automatically configures Claude Desktop's `claude_desktop_config.json`
- `status`: Validates the installation and checks for required .env configuration
- `remote`: Starts evna with Supergateway (stdio → SSE) and ngrok tunneling for remote access

**Requirements for local MCP:**
- evna-next directory with package.json
- Bun runtime installed
- .env configured in evna-next directory

**Requirements for remote MCP:**
- evna directory with package.json
- Bun runtime installed
- Supergateway installed (`npm install -g supergateway`)
- ngrok installed (from https://ngrok.com/download)
- EVNA_NGROK_AUTHTOKEN configured in .env (get from https://dashboard.ngrok.com/get-started/your-authtoken)

After installation, restart Claude Desktop to load the MCP server.

## Claude Code Session Log Querying

Query and analyze Claude Code session logs for evna integration and context extraction.

```bash
# List recent Claude Code sessions (agent sessions excluded by default)
floatctl claude list

# Include agent sessions (nested Agent SDK calls)
floatctl claude list --include-agents

# Filter by project path (fuzzy substring match)
floatctl claude list --project float-hub

# Backward compatibility (old command still works)
floatctl claude list-sessions

# Extract recent context for system prompt injection (evna's primary use)
floatctl claude recent-context --limit 20

# Pretty-print a specific session log
floatctl claude show <session-id>

# Show just last N messages (timeout visibility, partial progress)
floatctl claude show <session-id> --last 2 --no-tools
```

**Recent improvements (2025-11-12)**:
- Renamed `list-sessions` to `list` (old name still works as alias)
- Added agent session filtering: sessions with IDs starting with "agent-" are excluded by default to reduce noise from nested Agent SDK calls (use `--include-agents` to show them)
- Added `--first N` and `--last N` options to `show` command for partial session viewing
- Timeout visibility: evna now automatically shows partial progress when timing out
- Project filter already supported with fuzzy substring matching
- Fixed UTF-8 char boundary panic in `recent-context` truncation (both search boundaries)

**Primary use case:** The `recent-context` command is designed for evna to inject relevant context from Claude Code sessions into its system prompt, enabling seamless context awareness across Desktop and Code interfaces.

**What it does:**
- Streams and parses JSONL logs from `~/.claude/projects/`
- Handles both user and API message formats
- Provides formatted output for human reading or machine processing

## Bridge Maintenance

Manage bridge files for organizing conversation content by projects, issues, and meetings.

```bash
# Index :: annotations from markdown files to create bridge stubs
floatctl bridge index --dir ./daily-notes

# Append conversation content to appropriate bridge files
floatctl bridge append --content "conversation text" --project my-project
```

**Supported annotation types:**
- `project::name` - Project-based organization
- `issue::number` - GitHub issue tracking
- `lf1m::topic` - "Looking for one more" collaboration requests
- `meeting::identifier` - Meeting notes and discussions

## Script Management

Register and manage reusable shell scripts for quick access.

```bash
# Register a script (copies to ~/.floatctl/scripts/)
floatctl script register ./my-script.sh
floatctl script register /path/to/script.sh --name custom-name
floatctl script register ./script.sh --force  # Overwrite existing

# List registered scripts
floatctl script list

# Run a registered script with arguments
floatctl script run my-script.sh
floatctl script run my-script.sh arg1 "arg with spaces" --flag
```

**Platform notes:**
- Unix/Linux/macOS: Scripts are made executable (chmod 755) and validated for shebang
- Windows: Scripts use extension-based execution (.bat, .cmd, .ps1)
- Security: Symlink protection prevents directory traversal attacks

**Use case:** Save ad-hoc scripts that prove useful during development for easy reuse across sessions.

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

# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`floatctl-rs` (formerly `claude_convo_exporter`) is a Rust toolchain for processing, organizing, and semantically searching LLM conversation archives from Claude and ChatGPT. It handles large exports (100MB+) with O(1) memory usage through streaming JSON parsing.

**Core capabilities**:
- Streaming JSON/NDJSON parser with custom array handling
- Folder-per-conversation organization with artifact extraction
- Optional semantic search via pgvector embeddings
- Multi-format output (Markdown, JSON, NDJSON)
- Smart deduplication and incremental processing

## Workspace Structure

This is a Cargo workspace with multiple crates:
- **`floatctl-core`**: Core streaming, parsing, and rendering functionality
- **`floatctl-cli`**: CLI binary with all commands
- **`floatctl-embed`**: Optional vector search with pgvector (feature-gated with `embed` feature)
- **`floatctl-claude`**: Claude Code session log parsing and querying (for evna integration)
- **`floatctl-bridge`**: Bridge file management for annotation-based organization
- **`floatctl-script`**: Script registration and execution management

## Build and Development Commands

### Build
```bash
cargo build                      # Debug build
cargo build --release            # Release build
cargo build --features embed     # Build with embedding support
```

### Global Installation (Recommended)

Install `floatctl` as a global command accessible from any directory:

```bash
# Install to ~/.cargo/bin/floatctl
cargo install --path floatctl-cli --features embed

# Install sync scripts to ~/.floatctl/bin/ and ~/.floatctl/lib/
floatctl sync install

# Create global config directory (if not already created)
mkdir -p ~/.floatctl

# Copy environment variables to global config
cp .env.example ~/.floatctl/.env
# Edit ~/.floatctl/.env with your credentials

# Use from anywhere
cd /any/directory
floatctl query "search term"
floatctl embed --in messages.ndjson
```

**Upgrading:**
```bash
# Update binary and scripts together
cargo install --path floatctl-cli --features embed
floatctl sync install --force
```

**Configuration priority:**
1. Current directory `.env` (highest priority - local overrides)
2. `~/.floatctl/.env` (global defaults)
3. Environment variables already set (lowest priority)

This allows:
- Global installation with shared config
- Per-project overrides (create `.env` in project directory)
- Zero configuration changes when switching directories

See [INSTALL.md](./INSTALL.md) for complete installation guide.

### Run CLI Commands
```bash
# Recommended: Full extraction (one command)
cargo run -p floatctl-cli -- full-extract --in export.json --out ./archive/

# Convert JSON array to NDJSON (streaming, O(1) memory)
cargo run -p floatctl-cli -- ndjson --in conversations.json --out conversations.ndjson

# Split conversations into folders
cargo run -p floatctl-cli -- split --in conversations.ndjson --out ./archive/

# With embedding support (requires --features embed)
cargo run -p floatctl-cli --features embed -- embed --in messages.ndjson
cargo run -p floatctl-cli --features embed -- query "search term"
```

### Script Management

Register and manage reusable shell scripts for quick access:

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

# Use case: Save ad-hoc scripts that prove useful during development
```

**Platform notes:**
- Unix/Linux/macOS: Scripts are made executable (chmod 755) and validated for shebang
- Windows: Scripts use extension-based execution (.bat, .cmd, .ps1)

### Testing
```bash
cargo test                          # Run all tests
cargo test -p floatctl-core         # Test core crate
cargo test -p floatctl-embed        # Test embedding crate (without pgvector tests)
cargo test -p floatctl-embed -- --ignored  # Run pgvector integration tests (requires Docker)
cargo bench -p floatctl-core        # Run performance benchmarks
```

### Linting
```bash
cargo clippy                        # Run linter
cargo clippy -- -D warnings         # Fail on warnings
cargo fmt                           # Format code
cargo fmt -- --check                # Check formatting without modifying
```

### Database Setup (for embeddings)
```bash
# Start pgvector with Docker
docker run --rm -e POSTGRES_PASSWORD=postgres -p 5433:5432 ankane/pgvector

# Set environment variables
export DATABASE_URL="postgresql://postgres:postgres@localhost:5433/floatctl"
export OPENAI_API_KEY="sk-..."

# Migrations run automatically, but can run manually:
cargo sqlx migrate run --source migrations/
```

### ngrok Configuration (for `floatctl evna remote`)
The `floatctl evna remote` command starts evna as a remote MCP server with ngrok tunneling. Configure via environment variables in `.env`:

```bash
# ngrok authtoken (required for tunnel)
# Get from: https://dashboard.ngrok.com/get-started/your-authtoken
EVNA_NGROK_AUTHTOKEN=your-ngrok-authtoken-here

# Optional: Reserved ngrok domain (paid accounts only)
EVNA_NGROK_DOMAIN=your-reserved-domain.ngrok-free.app

# Optional: Basic auth for ngrok tunnel (format: username:password)
EVNA_NGROK_AUTH=username:password
```

**Usage:**
```bash
# Start remote MCP server with ngrok tunnel
floatctl evna remote --path ./evna

# Skip tunnel (only start Supergateway on localhost)
floatctl evna remote --no-tunnel
```

**Note:** The command will display a warning if `EVNA_NGROK_AUTHTOKEN` is not set and ngrok tunnel is enabled.

## Architecture

### Streaming Layer (`floatctl-core/src/stream.rs`)

**Critical for performance** - This is the foundation of O(1) memory usage:

**`JsonArrayStream`**: Custom JSON array parser that yields elements one-at-a-time
- **Problem**: `serde_json::StreamDeserializer` treats `[...]` as a single value, loading entire 772MB file before yielding
- **Solution**: Manual parsing of array structure (detect `[`, skip commas, read elements individually)
- **Performance**: Process 772MB in ~4 seconds with <100MB memory

**`RawValueStream` vs `ConvStream`**:
- `RawValueStream`: Returns raw `serde_json::Value` (used by `ndjson` command for speed)
- `ConvStream`: Parses into `Conversation` structs (used by `split` command)
- Both auto-detect JSON arrays vs NDJSON format

### Core Data Flow (floatctl-core)

1. **Streaming Input** (`stream.rs`):
   - `JsonArrayStream` or NDJSON line-by-line reader
   - Auto-detects format by peeking first byte (`[` = JSON array, `{` = NDJSON)
   - Yields one conversation at a time with O(1) memory

2. **Conversation Parsing** (`conversation.rs`):
   - `Conversation::from_export()` normalizes both ChatGPT and Anthropic formats
   - Preserves raw JSON via `clone()` before mutation (for JSON output)
   - Uses `std::mem::take()` to move message arrays without cloning
   - **Anthropic format**: `chat_messages` array with `content` blocks
   - **ChatGPT format**: `mapping` object (node-based structure)

3. **Pipeline Processing** (`pipeline.rs`):
   - Generates filesystem-safe slugs: `YYYY-MM-DD-title` format
   - Extracts artifacts from `tool_use` blocks (maps `artifact_type` to file extensions)
   - Creates folder-per-conversation structure
   - Renders Markdown with YAML frontmatter and emoji role indicators

4. **Output Formats**:
   - **Markdown**: YAML frontmatter + formatted messages + artifact references
   - **JSON**: Preserved raw conversation JSON
   - **NDJSON**: Message-level records with metadata (for embeddings)

### Embedding Architecture (floatctl-embed)

1. **Message Ingestion**:
   - Reads NDJSON message records from `floatctl-core` output
   - Chunks long messages (>6000 tokens) with 200-token overlap
   - Uses cached tokenizer (`once_cell::Lazy`) for 2-3x speedup
   - Batches OpenAI API calls (default: 32 messages per batch)

2. **Database Schema** (PostgreSQL + pgvector):
   - `conversations`: Stores conversation metadata with markers
   - `messages`: Stores message content with project/meeting/markers
   - `embeddings`: Stores vector embeddings with chunk support
   - Primary key: `(message_id, chunk_index)` enables multi-chunk messages

3. **Vector Search**:
   - IVFFlat index for approximate nearest neighbor search
   - Smart index management: only recreates if >20% row count change
   - Query filters: project, date range, result limit

### Claude Code Session Log Querying (floatctl-claude)

**Purpose**: Provides evna integration for accessing Claude Code session history and context.

1. **JSONL Streaming Parser** (`floatctl-claude/src/stream.rs`):
   - Streams log entries from `~/.claude/projects/<project-id>/history.jsonl`
   - Handles multiple log entry types: user messages, API responses, tool calls
   - Memory-efficient: processes line-by-line without loading entire file

2. **Message Parsing** (`floatctl-claude/src/parser.rs`):
   - Parses both user and API message formats
   - Extracts content from nested message structures
   - Handles text content, tool use, and thinking blocks

3. **Commands** (`floatctl-claude/src/commands/`):
   - `list`: Discovers and lists recent session directories (renamed from `list-sessions`, old name still works as alias)
   - Agent filtering: By default, excludes sessions with IDs starting with "agent-" (nested Agent SDK calls), use `--include-agents` to show all
   - Project filtering: `--project` flag for fuzzy substring matching on session paths
   - `recent-context`: Extracts N most recent messages for system prompt injection
   - `show`: Pretty-prints full session with formatted output
     - New `--first N` and `--last N` options for partial session viewing (timeout visibility)
     - Filters to user/assistant messages only (skips file-history-snapshot noise)

**Primary use case**: The `recent-context` command is used by evna to inject Claude Code session context into its system prompt, enabling seamless context awareness across Claude Desktop and Claude Code interfaces.

**Security**: Uses `execFile()` instead of shell execution in evna integration to prevent command injection.

### Bridge Maintenance (floatctl-bridge)

**Purpose**: Organizes conversation content into bridge files based on `::` annotations.

1. **Annotation Parsing**:
   - Parses `project::name`, `issue::number`, `lf1m::topic`, `meeting::identifier` markers
   - Extracts metadata from markdown files
   - Tracks annotation locations and context

2. **Commands**:
   - `index`: Scans markdown files for `::` annotations, creates bridge stub files
   - `append`: Appends conversation content to appropriate bridge files based on annotations

3. **Smart Features**:
   - Duplicate detection: Prevents re-adding identical content
   - Content extraction: Strips metadata, preserves conversation substance
   - Automatic file organization: Creates bridge files in appropriate directories

**Use case**: Maintains curated collections of related conversations organized by project, issue, collaboration request, or meeting.

### Script Management (floatctl-script)

**Purpose**: Register and manage reusable shell scripts for quick access.

1. **Script Storage**:
   - Scripts stored in `~/.floatctl/scripts/` directory
   - Original filename preserved during registration
   - Optional custom naming with `--name` parameter

2. **Security Features**:
   - **Symlink protection**: Validates source and destination are regular files
   - **Shebang validation** (Unix): Ensures scripts have valid `#!/...` line
   - **Extension-based execution** (Windows): Validates `.bat`, `.cmd`, `.ps1` extensions
   - **Directory traversal protection**: Prevents `../` in script names

3. **Cross-platform Support**:
   - Unix/Linux/macOS: Makes scripts executable (`chmod 755`), validates shebang
   - Windows: Uses extension-based execution, validates file types
   - Platform-specific error messages and validation

4. **Commands**:
   - `register`: Copy script to `~/.floatctl/scripts/` with validation
   - `list`: Show all registered scripts with file sizes
   - `run`: Execute registered script with argument passthrough

**Use case**: Save ad-hoc scripts that prove useful during development for easy reuse across sessions without polluting PATH or managing script locations.

### Sync Scripts (`scripts/`)

**Purpose**: Version-controlled canonical scripts for R2 sync daemons.

**Structure**:
```
scripts/
├── bin/          # Executable scripts (copied to ~/.floatctl/bin/)
│   ├── watch-and-sync.sh      # File watcher daemon for daily notes
│   └── sync-daily-to-r2.sh    # R2 sync script for daily notes
└── lib/          # Library/helper scripts (copied to ~/.floatctl/lib/)
    ├── log_event.sh           # Structured logging helpers
    └── parse_rclone.sh        # Rclone output parsing
```

**Installation**: `floatctl sync install` copies scripts from repo to `~/.floatctl/`

**Development workflow**:
1. Edit canonical scripts in `scripts/bin/` or `scripts/lib/`
2. Test locally: `floatctl sync install --force`
3. Commit changes to repo
4. Users upgrade: `cargo install --path floatctl-cli && floatctl sync install --force`

**Duplicate prevention**: `watch-and-sync.sh` uses PID file at `~/.floatctl/run/daily-sync.pid` to prevent multiple daemon instances.

**Platform requirements**:
- **macOS** (primary platform): Uses launchd for daemon management, fswatch for file watching
- **fswatch**: Required for file watcher daemon (`brew install fswatch` on macOS)
- **rclone**: Required for R2 sync (`brew install rclone` on macOS)
- **Linux/Windows**: Scripts currently macOS-specific (launchctl, fswatch paths). Cross-platform support planned but not yet implemented.

**Known limitations**:
- `floatctl sync start/stop` commands use macOS launchctl - not compatible with systemd (Linux) or Windows services
- `watch-and-sync.sh` searches for fswatch in PATH with fallback to common macOS locations (/opt/homebrew/bin, /usr/local/bin)
- Scripts will fail gracefully with clear error messages if dependencies are missing

## Performance Characteristics

**Benchmarks** (criterion, 3-conversation fixture on Apple M-series):
- `RawValueStream`: 22 µs
- `ConvStream`: 35 µs
- `Conversation::from_export`: 4.9 µs

**Real-world performance** (772MB file, 2912 conversations):
- Convert to NDJSON: ~4s, <100MB memory
- Full extract: ~7s, <100MB memory
- Embedding batch (32 msgs): ~1-2s (OpenAI API call)

Run benchmarks: `cargo bench -p floatctl-core`

## Key Implementation Details

### Performance Optimizations
1. **Avoid cloning message arrays**: Use `std::mem::take()` to move `Vec` instead of cloning
2. **Use `to_writer()` not `to_string()`**: Direct serialization to output buffer avoids intermediate allocations
3. **Lazy regex compilation**: Use `once_cell::sync::Lazy` for static regexes
4. **Tokenizer caching**: `Lazy<CoreBPE>` provides 2-3x speedup for embedding token counting
5. **Skip existing embeddings**: `--skip-existing` loads HashSet for O(1) lookup (~1MB per 65K messages)

### Streaming Architecture
- **Manual JSON array parsing**: Required for true streaming (serde treats arrays as single values)
- **Clone before mutation**: Preserve raw JSON by cloning before `std::mem::take()` operations
- **O(1) memory usage**: At any point, only holds 1 conversation (~10-50KB) + buffers (~16KB)

### Artifact Extraction
- Searches for `tool_use` blocks with `name: "artifacts"` in Anthropic conversations
- Maps artifact types to extensions: `application/vnd.ant.react` → `.jsx`, `text/html` → `.html`, etc.
- Generates numbered filenames: `{index:02}-{slugified-title}.{ext}`
- Saved to `artifacts/` subdirectory per conversation

### Message Chunking (floatctl-embed)
- **Fixed-size token-based chunks**: 6000 tokens (2K buffer below OpenAI 8,192 limit)
- **200-token overlap**: Preserves semantic continuity between chunks
- **Cached tokenizer**: `once_cell::Lazy` initialization avoids 50ms overhead per message
- **Database schema**: `(message_id, chunk_index)` composite primary key enables multi-chunk messages

## Recent Updates (November 2025)

### Sync Start/Stop Commands + Script Versioning (PR #23)

**Implemented**:
- `floatctl sync start/stop` commands for daemon lifecycle management
- Version-controlled canonical scripts in `scripts/` directory
- `floatctl sync install` command to deploy scripts to `~/.floatctl/`

**Details**:
- `sync start`: Uses `launchctl load` to start daily sync daemon
- `sync stop`: Uses `launchctl unload` to prevent auto-restart (handles KeepAlive=true)
- PID file duplicate prevention at `~/.floatctl/run/daily-sync.pid`
- Scripts tracked in repo with deployment workflow

**Files**:
- `floatctl-cli/src/sync.rs`: New start/stop/install commands
- `scripts/bin/watch-and-sync.sh`: File watcher daemon with PID file protection
- `scripts/bin/sync-daily-to-r2.sh`: R2 sync script
- `scripts/lib/log_event.sh`: Structured logging helpers
- `scripts/lib/parse_rclone.sh`: Rclone output parsing

**Fixes**: Issue with 4+ duplicate `watch-and-sync.sh` processes spawning due to lack of duplicate prevention mechanism.

### Script Management (PR #15)
- New `floatctl script` commands for registering and running reusable shell scripts
- Security features: symlink protection, shebang validation, directory traversal prevention
- Cross-platform support: Unix/Linux/macOS (chmod 755, shebang validation), Windows (extension-based execution)
- Scripts stored in `~/.floatctl/scripts/` for easy access
- Unit tests for script validation and cross-platform compatibility

### Script Enhancements Phase 4 (November 2025)

**Implemented**:
- `floatctl script describe <name>` - Shows full parsed documentation from doc blocks
- Arg parsing already existed in floatctl-script crate - just needed CLI command
- Shell completions for describe command (regenerated via `floatctl completions zsh`)

**generate-diz script enhancement**:
- Now accepts file/glob patterns as arguments
- Usage: `floatctl script run generate-diz *.md` or specific files
- Defaults to `*` (all files) if no arguments provided
- Added proper doc block to generate-diz with Args and Example sections

**Testing**:
- `floatctl script describe generate-diz` - Shows full documentation with args and examples
- `floatctl script describe split-to-md` - Works with minimal doc blocks
- generate-diz with `*.md` pattern - Correctly processes only markdown files
- generate-diz with no args - Falls back to `*` (original behavior)

**Files modified**:
- `floatctl-cli/src/main.rs` - Added `Describe` command and `run_script_describe()` function
- `~/.floatctl/scripts/generate-diz` - Added doc block, file/glob pattern support (V4)

**Completes**: Phase 4 from floatctl-script-enhancements-spec.md (Arg parsing + describe command)

### Claude Code Integration (PR #14, #16, + improvements 2025-11-12)
- New `floatctl-claude` crate for querying Claude Code session logs
- Three commands: `list`, `recent-context`, `show`
  - `list` (renamed from `list-sessions`, alias still works):
    - Agent session filtering: Excludes "agent-xyz" sessions by default (use `--include-agents` to show)
    - Project filtering: `--project` flag for fuzzy substring matching
    - Reduces noise from nested Agent SDK calls (e.g., ask_evna sub-agents)
  - `recent-context`: UTF-8 char boundary panic fixed in truncation logic
  - `show`: Pretty-prints full session with formatted output
- JSONL streaming parser for `~/.claude/projects/` history files
- Primary use case: evna integration for context injection across Desktop and Code
- Security hardened: uses `execFile()` instead of shell execution
- Handles both user and API message formats

### Bridge Maintenance
- New `floatctl-bridge` crate for annotation-based content organization
- Commands: `index` (scan for `::` annotations), `append` (add content to bridges)
- Supports `project::`, `issue::`, `lf1m::`, `meeting::` annotation types
- Smart duplicate detection and content extraction
- Maintains curated collections organized by project/issue/meeting

### Embedding Pipeline Improvements (October 2025)

**Message Chunking** (PR #2, #3):
- Replaced paragraph/sentence splitter with token-based chunking (45 lines vs 118)
- Fixed chunk size: 6000 tokens with 200-token overlap
- Database schema supports multi-chunk messages via composite PK: `(message_id, chunk_index)`
- Added 6 unit tests for chunking edge cases

**Performance Optimizations** (PR #3, #5):
- Tokenizer caching with `once_cell::sync::Lazy` (2-3x speedup)
- Removed unnecessary `DISTINCT` from queries (message_id is already unique)
- Memory usage logging for skip-existing HashSet
- Fixed UTF-8 character boundary panic in `truncate()` function (PR #5)

**New Features**:
- `--skip-existing`: Idempotent re-runs with O(1) lookup
- Progress tracking: "Processed | Chunked | Skipped" counters
- Batch size validation: Warns if >50 (prevents 300K token limit errors)
- `--rate-limit-ms`: Control API call delays (default: 500ms)

**Testing**:
- Unit tests: `cargo test -p floatctl-embed`
- Integration test: `embeds_roundtrip` requires pgvector Docker container
- Run ignored tests: `cargo test -p floatctl-embed -- --ignored`

## Key Files by Feature

### Core Streaming
- `floatctl-core/src/stream.rs`: Custom JSON array parser
- `floatctl-core/src/conversation.rs`: Dual-format conversation parsing
- `floatctl-core/src/pipeline.rs`: Slug generation, artifact extraction

### Embeddings
- `floatctl-embed/src/lib.rs:42-115`: Token-based chunking with UTF-8 recovery
- `floatctl-embed/src/lib.rs:158-388`: Embedding pipeline with progress bars
- `floatctl-embed/src/lib.rs:411-484`: Query implementation with metadata display
- `migrations/0003_add_chunk_support.sql`: Multi-chunk schema

### Claude Code Integration
- `floatctl-claude/src/stream.rs`: JSONL streaming parser for history files
- `floatctl-claude/src/parser.rs`: Message format parser (user + API formats)
- `floatctl-claude/src/commands/list_sessions.rs`: Session discovery
- `floatctl-claude/src/commands/recent_context.rs`: Context extraction for evna
- `floatctl-claude/src/commands/show.rs`: Pretty-print session logs

### Bridge Maintenance
- `floatctl-bridge/src/index.rs`: Annotation indexing and bridge stub creation
- `floatctl-bridge/src/append.rs`: Content appending with deduplication
- `floatctl-bridge/src/lib.rs`: Annotation parsing and metadata extraction

### Script Management
- `floatctl-script/src/lib.rs`: Core script management logic
- `floatctl-script/src/register.rs`: Script registration with security validation
- `floatctl-script/src/list.rs`: Script listing
- `floatctl-script/src/run.rs`: Script execution with argument passthrough

### Testing
- `floatctl-embed/src/lib.rs:671-745`: `embeds_roundtrip` integration test
- `floatctl-core/benches/`: Criterion performance benchmarks
- `floatctl-script/tests/`: Cross-platform script validation tests

## Development Best Practices

1. **Always prefer streaming**: Use `RawValueStream`/`ConvStream`, never load entire JSON
2. **Test with pgvector**: Run `docker run --rm -p 5433:5432 -e POSTGRES_PASSWORD=postgres ankane/pgvector`
3. **Respect UTF-8 boundaries**: Use `char_indices()` when truncating strings
4. **Cache expensive operations**: Use `once_cell::Lazy` for tokenizers, regexes
5. **Document performance**: Add informal benchmarks to PR descriptions for large changes
- 💡 RUST BEST PRACTICES
📝 Code Design Principles
Ownership Model: Understand borrowing and lifetimes
Zero-Cost Abstractions: Write high-level code with low-level performance
Error Handling: Use Result and Option types effectively
Memory Safety: Eliminate data races and memory bugs
Performance: Leverage compiler optimizations
Concurrency: Safe parallel programming patterns
🎯 Advanced Patterns
Type System: Leverage advanced type features
Macros: Write declarative and procedural macros
Unsafe Code: When and how to use unsafe blocks
FFI: Foreign function interface patterns
Embedded: Bare metal and embedded development
WebAssembly: Compile to WASM targets
📚 RUST LEARNING RESOURCES
🎓 Recommended Topics
Core Rust: Ownership, borrowing, lifetimes
Advanced Features: Traits, generics, macros
Async Programming: Tokio, async/await patterns
Systems Programming: Low-level development
Web Development: Axum, Warp, Rocket frameworks
Performance: Profiling, optimization techniques
🔧 Essential Tools
Toolchain: rustc, cargo, rustup, clippy
IDEs: VS Code with rust-analyzer, IntelliJ Rust
Testing: Built-in test framework, proptest, criterion
Debugging: gdb, lldb, rr (record and replay)
Profiling: perf, valgrind, cargo-flamegraph
Cross-compilation: cross, cargo-zigbuild
🌟 Ecosystem Highlights
Web Frameworks: Axum, Actix-web, Warp, Rocket
Async Runtime: Tokio, async-std, smol
Serialization: Serde, bincode, postcard
Databases: SQLx, Diesel, sea-orm
CLI Tools: Clap, structopt, colored
Graphics: wgpu, bevy, ggez, nannou
- ... this is a tool for just me that we are using right now.. like, i would be fine with nuking the tablre and repopualting from scrathc, stop
 having enterprise multi-user concerns crepe intro my personal tooling that only i am using
- evna mcp is in the evna subfolder, @evna/CLAUDE.md
- md the source for evna is in the evna folder
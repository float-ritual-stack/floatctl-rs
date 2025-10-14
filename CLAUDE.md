# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`claude_convo_exporter` is a Rust CLI tool that processes LLM conversation exports (from ChatGPT and Anthropic/Claude) and converts them into organized Markdown and/or JSON files. It supports both `.json` and `.zip` archive inputs, performs deduplication via SHA-256 hashing, and maintains state to avoid re-processing conversations.

## Build and Development Commands

### Build
```bash
cargo build              # Debug build
cargo build --release    # Release build
```

### Run
```bash
cargo run -- --help                              # Show help
cargo run -- --in conversations.json             # Process default input
cargo run -- --in export.zip --out ./output      # Process ZIP archive
cargo run -- --dry-run --in conversations.json   # Preview without writing
cargo run -- --force --in conversations.json     # Force re-process all
```

### Testing
```bash
cargo test               # Run all tests
cargo test <test_name>   # Run specific test
```

### Linting
```bash
cargo clippy             # Run linter
cargo fmt                # Format code
cargo fmt -- --check     # Check formatting without modifying
```

## Architecture

### Core Data Flow

1. **Input Loading** (`input.rs`):
   - Handles both JSON and ZIP file inputs
   - Computes SHA-256 fingerprints for deduplication
   - ZIP archives are extracted to temp directories and all JSON files are merged
   - Auto-detects conversation source (Anthropic vs ChatGPT) via `detect_source()`

2. **Conversation Parsing** (`model.rs`):
   - Defines `Conversation` and `Message` structs representing normalized conversation format
   - Contains source-specific parsers: `parse_anthropic_conversation()` and `parse_chatgpt_conversation()`
   - Extracts metadata: timestamps, participants, model info, artifacts, attachments
   - **Anthropic format**: Uses `chat_messages` array with `content` blocks for tool use and artifacts
   - **ChatGPT format**: Uses `mapping` object (node-based structure) with nested message content

3. **State Management** (`state.rs`):
   - Tracks processed conversations in `conv_split.json` (stored in state directory)
   - Uses file locking (`conv_split.lock`) to prevent concurrent access
   - `SeenRecord` stores conversation hash for deduplication
   - `RunRecord` tracks each execution with input fingerprint and processed conversation IDs

4. **Filtering** (`filters.rs`):
   - `FilterContext` handles date filtering (`since`/`until`)
   - Manages timezone conversions for display timestamps
   - Generates filename date prefixes based on `date_from` config (UTC vs local)

5. **Slug Generation** (`slug.rs`):
   - Converts conversation titles to filesystem-safe slugs
   - `SlugState` tracks used slugs to ensure uniqueness (adds `-2`, `-3`, etc. suffixes)
   - Three filename strategies: `title` (default), `id`, or `first-human-line`

6. **Rendering**:
   - `render_md.rs`: Generates Markdown with YAML frontmatter, message sections, and artifact references
   - `render_json.rs`: Outputs prettified raw conversation JSON
   - Artifacts are extracted to `artifacts/` subdirectories with numbered filenames

### Configuration System

Configuration merges in this order (later overrides earlier):
1. Built-in defaults
2. `~/.config/floatctl/conv_split.toml` (or platform-specific config dir)
3. Fallback configs: `~/.floatctl/conv_split.toml` or `~/.floatctl/local_config.toml`
4. Explicit config file via `--config` flag
5. CLI arguments (highest priority)

Key config options:
- `out_dir`: Output directory for conversations
- `formats`: Array of `"md"` and/or `"json"`
- `tz`: Timezone for displaying timestamps (e.g., "America/Toronto")
- `date_from`: Use `"utc"` or `"local"` for filename date prefix
- `dedupe`: Enable/disable deduplication (default: true)
- `filename_from`: Strategy for filename generation (`"title"`, `"id"`, or `"first-human-line"`)
- `[filters]`: Date range filtering with `since` and `until` (YYYY-MM-DD format)
- `[state]`: Custom state directory location

### Main Execution Flow (`util.rs::execute`)

1. Load configuration and input bundle
2. Load state from disk (or create new)
3. For each conversation:
   - Canonicalize JSON and compute SHA-256 hash
   - Parse into normalized `Conversation` struct
   - Check date filters
   - Skip if already seen (unless `--force` or content changed)
   - Generate unique output paths based on slug strategy
   - Render to Markdown and/or JSON
   - Extract artifacts to separate files
   - Update state with new hash
4. Save updated state to disk

### Error Handling

`AppError` wrapper categorizes errors by kind:
- `Input`: Problems with input file format or parsing
- `Io`: File system operations
- `Validation`: Data validation failures
- `Config`: Configuration issues

Each error kind maps to a distinct exit code (1-3), plus exit code 4 for "nothing processed" scenarios.

## Key Implementation Details

- **Deduplication**: Uses canonical JSON representation (sorted keys) + SHA-256 to detect conversation changes
- **Artifact extraction**: Searches for `tool_use` and `tool_result` blocks with `name: "artifacts"` in Anthropic conversations
- **Slug collision**: The `SlugState` in-memory tracker ensures no duplicate filenames within a single run
- **Message channels**: Different message types (message/reply/reasoning/system/tool) are separated into distinct sections in Markdown output
- **Date stripping**: `strip_leading_date()` removes date prefixes from titles to avoid redundancy with filename dates

## State and Data Directories

- **Config**: `~/.config/floatctl/conv_split.toml` (or OS-specific config dir)
- **State**: `~/.local/share/floatctl/state/conv_split/` (or OS-specific data dir)
- **Temp files**: `~/.cache/floatctl/tmp/` (or OS-specific cache dir)

The project uses the `directories` crate to determine platform-appropriate paths.

## Recent Architecture Updates (October 2025)

### Embedding Pipeline Improvements

The `floatctl-embed` crate received significant updates for performance and reliability:

**Message Chunking** (PR #2, #3):
- Replaced complex paragraph/sentence splitter with simple token-based chunking
- Fixed chunk size: 6000 tokens (2K buffer below OpenAI's 8,192 limit)
- 200-token overlap for context continuity
- Database schema updated to support multiple chunks per message (migration 0003)
- Added 6 comprehensive unit tests for chunking logic

**Performance Optimizations** (PR #3):
- Tokenizer caching with `once_cell::sync::Lazy` (2-3x speedup)
- Removed unnecessary `DISTINCT` from database queries
- Memory usage logging for existing embeddings HashSet

**New Features**:
- `--skip-existing` flag for idempotent re-runs (only embed new messages)
- Progress tracking shows "Processed | Chunked | Skipped" counters
- Batch size validation (warns if >50 to prevent 300K token limit errors)
- `--rate-limit-ms` flag for controlling API call delays (default: 500ms)

**Testing**:
- Unit tests: `test_chunk_message_*` for size limits, overlap, edge cases
- Integration test: `embeds_roundtrip` validates full pipeline with pgvector
- Run with: `cargo test -p floatctl-embed`

### GitHub Actions Integration

Two Claude Code workflows were added:

**`.github/workflows/claude-code-review.yml`**:
- Automated PR review using Claude Code
- Runs on pull_request events
- Reviews code quality, architecture, tests

**`.github/workflows/claude.yml`**:
- Claude PR assistant
- Helps with PR creation and management

### Key Implementation Files

When working on embeddings:
- **Core logic**: `floatctl-embed/src/lib.rs:42-76` (chunking), `lib.rs:120-350` (pipeline)
- **Database**: `migrations/0003_add_chunk_support.sql` (chunk schema)
- **Tests**: `floatctl-embed/src/lib.rs:793-897` (unit tests)
- **Documentation**: `floatctl-embed/README.md` (user guide), `ARCHITECTURE.md` (technical design)

### Development Workflow

Typical PR workflow with recent changes:
1. Make changes on feature branch
2. Run unit tests: `cargo test -p floatctl-embed`
3. Check with clippy: `cargo clippy`
4. Create PR → GitHub Actions run Claude Code review
5. Address review comments → Push updates
6. Merge when approved

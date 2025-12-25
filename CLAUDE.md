# CLAUDE.md

Rust toolchain for processing LLM conversation archives. Streaming parser (O(1) memory), Cloudflare AutoRAG search, pgvector embeddings, Claude Code integration.

**Version**: 0.3.0

## Quick Reference

**Build**: `cargo build --release --features embed`
**Install**: `cargo install --path floatctl-cli --features embed`
**Test**: `cargo test` | `cargo clippy`
**Typecheck**: `cargo check --all-features`

**Workspace crates** (8 total):
- `floatctl-core` - Streaming JSON/NDJSON parser, conversation types, markers
- `floatctl-cli` - Main binary with dual-mode architecture (human/agent)
- `floatctl-embed` - OpenAI embeddings, pgvector search, token chunking
- `floatctl-claude` - Claude Code session log parser (JSONL streaming)
- `floatctl-bridge` - Bridge file management (annotations, indexing)
- `floatctl-script` - Shell script registration and execution
- `floatctl-server` - HTTP server (BBS routes, file search, dispatch capture)
- `floatctl-search` - FloatQL parser, Cloudflare AutoRAG client

## Common Tasks

**Extract conversations**: `floatctl full-extract --in export.json --out ./archive/`
**AI Search**: `floatctl search "query"` (Cloudflare AutoRAG with FloatQL)
**Semantic search**: `floatctl query all "term"` (pgvector, requires --features embed)
**Capture context**: `floatctl ctx "message"` (queues locally, syncs to float-box)

**Evna tools**: `floatctl evna boot|search|active|ask|sessions` (shells out to evna binary in `evna/`)

## BBS Commands

`floatctl bbs inbox|send|read|unread|memory|board|show|get`

**Global flags**:
- `--persona <name>` - Required: which agent (kitty, daddy, cowboy, evna)
- `--endpoint <url>` - BBS API endpoint (default: http://float-box:3030)
- `--insecure` - Skip TLS verification (required for ngrok endpoints)

**Subcommands**:
- `inbox` - List messages (--limit, --unread-only, --json, --quiet)
- `show <id>` - Display full message content
- `get <query>` - Fuzzy search across inbox/memories/boards/files
- `send --to <persona> --subject <subj>` - Send message
- `read/unread <id>` - Toggle read status
- `memory list|save` - Manage memories (--category, --tag)
- `board list|post` - Board operations (--board/-b flag for posts)

**Get command features**:
- `--type inbox|memory|board` - Filter search type
- `-n <limit>` - Max results (default: 5)
- File search via R2 bucket (bridges, imprints, daily notes on float-box)
- Auto-displays content when exactly one match found

## Status Commands

`floatctl status focus|notice|clear|show`

- `focus "message"` - Set work focus (--set-by for attribution)
- `notice "message"` - Set sysop notice (break warnings, meeting status)
- `clear focus|notice|all` - Clear status entries
- `show [--json]` - Display current status
- Files: `~/.floatctl/status/{focus,notice}.json`
- Used by: evna-remote MCP server for ambient awareness

## AI Search (floatctl search)

Cloudflare AutoRAG with FloatQL pattern recognition:

```bash
floatctl search "query"
floatctl search "dispatch:: floatctl performance" --folder dispatch/
floatctl search "bridge:: CB-20251201-1430" --raw  # retrieval only
```

**Flags**:
- `--parse-only` - Show parsed FloatQL patterns without searching
- `--no-parse` - Bypass FloatQL, send raw query to AutoRAG
- `--raw` - Retrieval only, no LLM synthesis
- `--folder <path>` - Filter to folder prefix
- `--no-rewrite` - Disable AutoRAG query rewriting
- `--no-rerank` - Disable BGE reranking
- `--model <id>` - LLM for synthesis (default: llama-3.3-70b)
- `-q/--quiet` - Suppress spinner (for scripts/LLMs)

**FloatQL patterns recognized**:
- `dispatch::`, `bridge::`, `ctx::` - Float markers (auto-detects folder)
- `[evna::]`, `[sysop::]` - Persona patterns
- `CB-YYYYMMDD-HHMM-XXXX` - Bridge IDs
- `today`, `last 3 days`, `2025-11-26` - Temporal (parsed but NOT sent to API)
- `type:bridge`, `is:bridge` - Type filters (parsed but NOT sent to API)

**Environment**: `CLOUDFLARE_ACCOUNT_ID`, `CLOUDFLARE_API_TOKEN` (or `AUTORAG_API_TOKEN`)

## Dual-Mode Architecture

**Human Mode** (Interactive):
- Missing required args → interactive wizard via `inquire`
- Progress bars, colored output, emoji feedback
- `floatctl` with no args shows command picker

**Agent Mode** (Machine):
- `--json` flag → all output wrapped in JSON envelope
- `{ "status": "success"|"error", "data": {...}, "error": {...} }`
- `floatctl reflect` outputs full CLI schema for agent introspection
- `--quiet` suppresses spinners/progress bars

## Key Patterns

- **Streaming first**: Use `RawValueStream`/`ConvStream`, never load full JSON
- **O(1) memory**: Custom `JsonArrayStream` parser (serde treats arrays as single values)
- **UTF-8 safety**: Use `char_indices()` when truncating strings
- **Cache expensive ops**: `once_cell::Lazy` for tokenizers, regexes
- **Token chunking**: 6000 tokens with 200 overlap (text-embedding-3-small)

## Architecture Pointers

- Streaming layer: `floatctl-core/src/stream.rs` (custom JSON array parser)
- Conversation parsing: `floatctl-core/src/conversation.rs` (ChatGPT + Anthropic formats)
- Embeddings: `floatctl-embed/src/lib.rs` (token chunking, pgvector, OpenAI)
- Claude Code logs: `floatctl-claude/src/` (JSONL streaming, smart truncation)
- AI Search: `floatctl-search/src/` (FloatQL parser, AutoRAG client)
- BBS client: `floatctl-cli/src/commands/bbs.rs` (HTTP client, fuzzy search)
- HTTP server: `floatctl-server/src/lib.rs` (Axum, file search, BBS routes)

## Configuration

**TOML config** (priority: CLI args → project config → user config → defaults):
- User config: `~/.floatctl/config.toml`
- Project config: `./floatctl.toml`

**Key options**:
- `general.default_output_dir` - Conversation export location
- `query.default_limit` - Default search results (default: 10)
- `embedding.batch_size` - API batch size 1-50 (default: 32)
- `bbs.get_search_types` - Default types for `bbs get` (inbox, memory, board)
- `bbs.get_search_paths` - Filesystem paths for file search

**Environment files** (priority: cwd → ~/.floatctl/.env → already-set):
- `DATABASE_URL` - PostgreSQL connection string
- `OPENAI_API_KEY` - For embeddings
- `FLOATCTL_BBS_ENDPOINT` - BBS API URL
- `FLOATCTL_PERSONA` - Default persona

## Testing

```bash
cargo test                    # Run all tests
cargo test -p floatctl-core   # Test specific crate
cargo clippy                  # Lint check
cargo bench                   # Run benchmarks (floatctl-core)
```

## Personal Tool Notes

- Single-user tooling - no enterprise concerns needed
- OK to nuke/repopulate tables
- evna source in `evna/` subfolder (see `evna/CLAUDE.md`)

## Sync Architecture

**Float-box as hub**: MacBook → float-box (rsync) → R2 (rclone)

```
MacBook ──rsync──> float-box ──rclone──> R2
         (local)   (systemd)   (sysops-beta/)
```

**Commands**:
- `floatctl sync status` - Shows full pipeline
- `floatctl sync status --remote` - Detailed float-box systemd status
- `floatctl sync trigger --daemon daily --wait` - Routes through float-box

**Daemon types**: daily, dispatch, projects

**Key files**:
- `floatctl-cli/src/sync.rs` - trigger_via_float_box(), status display
- `scripts/bin/watch-and-sync.sh` - inotifywait watcher
- `scripts/bin/sync-{daily,dispatch,projects}-to-r2.sh` - rclone scripts

## Full Docs

- `INSTALL.md` - Installation guide
- `ARCHITECTURE.md` - System design
- `CHANGELOG.md` - Release notes
- `scripts/README.md` - Script documentation
- `evna/CLAUDE.md` - evna-specific documentation

# CLAUDE.md

Rust toolchain for processing LLM conversation archives. Streaming parser (O(1) memory), pgvector search, Claude Code integration.

## Quick Reference

**Build**: `cargo build --release --features embed`
**Install**: `cargo install --path floatctl-cli --features embed`
**Test**: `cargo test` | `cargo clippy`

**Workspace crates**: core, cli, embed, claude, bridge, script, server, search

## Common Tasks

**Extract conversations**: `floatctl full-extract --in export.json --out ./archive/`
**Search history**: `floatctl query "search term"`
**Capture context**: `floatctl ctx "message"` (queues locally, syncs to float-box)

**Evna tools**: `floatctl evna boot|search|active|ask|sessions` (shells out to evna binary in `evna/`)

**BBS Commands**: `floatctl bbs inbox|send|read|unread|memory|board`
- `--persona kitty` - Required: which agent (kitty, daddy, cowboy, evna)
- `--endpoint http://float-box:3030` - BBS API endpoint (default: localhost:3030)
- `--insecure` - Skip TLS verification (required for ngrok endpoints)
- `--json` - JSON output for piping to jq
- `--quiet` - IDs only (for scripting)
- Subcommands: `inbox` (list), `send` (message), `read/unread` (toggle), `memory list/save`, `board list/post`

**Status Commands**: `floatctl status focus|notice|clear|show`
- `focus "message"` - Set work focus (--set-by for attribution)
- `notice "message"` - Set sysop notice (break warnings, meeting status)
- `clear focus|notice|all` - Clear status entries
- `show [--json]` - Display current status
- Files: `~/.floatctl/status/{focus,notice}.json`
- Used by: evna-remote MCP server (dynamic tool descriptions for ambient awareness)

**AI Search**: `floatctl search "query"` (Cloudflare AutoRAG with FloatQL parsing)
- `--parse-only` - Show parsed FloatQL patterns without searching
- `--no-parse` - Bypass FloatQL, send raw query to AutoRAG (debugging)
- `--raw` - Retrieval only, no LLM synthesis
- `--folder bridges/` - Filter to folder prefix
- `--no-rewrite` - Disable AutoRAG query rewriting
- `--no-rerank` - Disable BGE reranking
- Env: `CLOUDFLARE_ACCOUNT_ID`, `CLOUDFLARE_API_TOKEN` (or `AUTORAG_API_TOKEN`)

**FloatQL patterns recognized**:
- `dispatch::`, `bridge::`, `ctx::` - Float markers → folder auto-detection
- `[evna::]`, `[sysop::]` - Persona patterns
- `CB-YYYYMMDD-HHMM-XXXX` - Bridge IDs
- `today`, `last 3 days`, `2025-11-26` - Temporal (parsed but NOT sent to API)
- `type:bridge`, `is:bridge` - Type filters (parsed but NOT sent to API)

**Debugging search**: Use `--parse-only` to see what FloatQL extracts, `--no-parse` to bypass it entirely

## Key Patterns

- **Streaming first**: Use `RawValueStream`/`ConvStream`, never load full JSON
- **O(1) memory**: Custom `JsonArrayStream` parser (serde treats arrays as single values)
- **UTF-8 safety**: Use `char_indices()` when truncating
- **Cache expensive**: `once_cell::Lazy` for tokenizers, regexes

## Architecture Pointers

- Streaming layer: `floatctl-core/src/stream.rs` (custom JSON array parser)
- Conversation parsing: `floatctl-core/src/conversation.rs` (ChatGPT + Anthropic formats)
- Embeddings: `floatctl-embed/src/lib.rs` (token chunking, pgvector)
- Claude Code logs: `floatctl-claude/src/` (JSONL streaming, evna integration)
- AI Search: `floatctl-search/src/` (FloatQL parser, AutoRAG client)

## Personal Tool Notes

- This is single-user tooling - no enterprise concerns needed
- OK to nuke/repopulate tables
- evna source in `evna/` subfolder (see `evna/CLAUDE.md`)

## Full Docs

See repo for:
- `INSTALL.md` - Installation guide
- `scripts/bin/` - R2 sync scripts (platform-aware: macOS fswatch / Linux inotifywait)
- `scripts/systemd/` - Systemd user services for float-box deployment
- `~/.floatctl/logs/` - Daemon logs (JSONL format)
- Individual crate READMEs for deep dives

## Sync Architecture

**Float-box as hub** (2025-11-26): MacBook → float-box (rsync) → R2 (rclone)

```
MacBook ──rsync──> float-box ──rclone──> R2
         (local)   (systemd)   (sysops-beta/)
```

**Commands**:
- `floatctl sync status` - Shows full pipeline (MacBook→float-box→R2)
- `floatctl sync status --remote` - Detailed float-box systemd status
- `floatctl sync trigger --daemon daily --wait` - Routes through float-box

**Daemon types**: daily, dispatch, projects

**Key files**:
- `floatctl-cli/src/sync.rs` - trigger_via_float_box(), status display
- `scripts/bin/watch-and-sync.sh` - inotifywait watcher (uses moved_to for rsync)
- `scripts/bin/sync-{daily,dispatch,projects}-to-r2.sh` - rclone sync scripts

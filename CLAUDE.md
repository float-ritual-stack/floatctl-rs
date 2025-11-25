# CLAUDE.md

Rust toolchain for processing LLM conversation archives. Streaming parser (O(1) memory), pgvector search, Claude Code integration.

## Quick Reference

**Build**: `cargo build --release --features embed`
**Install**: `cargo install --path floatctl-cli --features embed`
**Test**: `cargo test` | `cargo clippy`

**Workspace crates**: core, cli, embed, claude, bridge, script, ctx

## Common Tasks

**Extract conversations**: `floatctl full-extract --in export.json --out ./archive/`
**Search history**: `floatctl query "search term"`
**Capture context**: `floatctl ctx "message"` (queues locally, syncs to float-box)

**Evna tools**: `floatctl evna boot|search|active|ask|sessions` (shells out to evna binary in `evna/`)

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

**Float-box as hub** (2025-11-25): MacBook → float-box (rsync) → R2 (rclone)
- `floatctl sync status --remote` checks float-box systemd services via SSH
- Daemon types: daily, dispatch, projects

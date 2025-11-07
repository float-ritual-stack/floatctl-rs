# Repository Guidelines

## Build, Test, and Development Commands
```bash
cargo build                          # Debug build all crates
cargo build --release                # Release build
cargo build -p floatctl-cli --features embed  # Build with embeddings
cargo test                           # Run all tests
cargo test -p floatctl-core          # Test single crate
cargo test test_chunk_message        # Run single test
cargo clippy -- -D warnings          # Lint
cargo fmt                            # Format
cargo bench -p floatctl-core         # Benchmarks
```

## Project Structure
Cargo workspace with 4 crates: **floatctl-core** (streaming/parsing/rendering), **floatctl-cli** (CLI commands), **floatctl-embed** (pgvector search, feature-gated), **floatctl-bridge** (R2 sync). Core streaming in `floatctl-core/src/stream.rs` uses `JsonArrayStream` for O(1) memory. Database migrations in `migrations/`, config in `.env`/`~/.floatctl/.env`, logs in `~/.floatctl/logs/*.jsonl`.

## Coding Style & Naming Conventions
Standard Rust: 4-space indents, 100-char wrap, snake_case for functions/modules, UpperCamelCase for types, SCREAMING_SNAKE for constants. Use `std::mem::take()` over cloning arrays, `to_writer()` over `to_string()`, `once_cell::Lazy` for regexes/tokenizers. Preserve UTF-8 boundaries with `char_indices()`. All errors use `anyhow::Result` with `.with_context()`. Run `cargo fmt` and keep `clippy` clean before commits.

## Architecture & Key Concepts
**Streaming-first**: Custom `JsonArrayStream` parses JSON arrays element-by-element (serde treats `[...]` as single value). Clone raw JSON before mutations to preserve for output. Dual-format support for ChatGPT (`messages`) and Anthropic (`chat_messages`). Artifacts extracted from `tool_use` blocks. Embedding chunking uses fixed 6000-token size with 200-token overlap. See ARCHITECTURE.md for full data flow.

## Testing & Performance
Unit tests in `#[cfg(test)]`, integration tests in `tests/`. Fixtures in `tests/data/`. Use descriptive names like `test_chunk_message_overlap`. pgvector tests require Docker: `docker run --rm -p 5433:5432 -e POSTGRES_PASSWORD=postgres ankane/pgvector`, run with `cargo test -p floatctl-embed -- --ignored`. Performance: 772MB in ~4s with <100MB memory. Always benchmark large changes with `cargo bench`.

## Personal Tooling Note
This is single-user personal tooling. No enterprise multi-user concerns needed - prefer simple solutions over complex abstractions. Nuking/repopulating tables is fine. Keep it pragmatic.

## EVNA MCP Server (evna/ subfolder)
TypeScript/Bun agent using Claude Agent SDK. Commands: `bun run dev` (CLI), `bun run tui` (terminal UI), `bun run mcp-server` (MCP for Claude Desktop), `bun run typecheck` (REQUIRED before commits). Tools: brain_boot, semantic_search, active_context, ask_evna orchestrator. Database: Supabase PostgreSQL/pgvector with migrations in `evna/migrations/`. See `evna/CLAUDE.md` for full architecture (dual-source search, session management, daily:// resources).

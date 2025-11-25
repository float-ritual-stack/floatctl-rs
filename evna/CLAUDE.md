# CLAUDE.md

AI agent for context synthesis and brain boot. Uses Cloudflare AutoRAG (historical) + PostgreSQL (recent activity). Exposes tools via MCP.

## Quick Reference

**Build**: `bun install` | `bun run typecheck` (REQUIRED before commits)
**Run**: `bun run dev` (CLI with reload) | `bun run mcp-server` (for Claude Desktop)
**Install**: Via `floatctl evna install` (recommended) or standalone `ln -s bin/evna ~/.local/bin/`

## Dependencies

**Required**:
- floatctl (Rust CLI) - for Claude Code session logs (`cargo install --path floatctl-cli`)
- PostgreSQL/pgvector (Supabase)
- OpenAI API (embeddings)
- Cohere API (optional - reranking, graceful fallback)

**Config**: `~/.floatctl/.env` (zero-config from any directory)

## Common Commands

```bash
# Via floatctl (recommended)
floatctl evna boot "yesterday's work"
floatctl evna search "performance" --project floatctl
floatctl evna active "recent notes"
floatctl evna ask "help debug X" --session abc-123

# Direct binary (also works)
evna boot "yesterday's work"
evna search "performance"
```

**CLI vs Agent**:
- Direct mode: Fast, subcommand-based (like git)
- Agent mode: Conversational, LLM-driven (higher cost)

## Architecture Pointers

**Core separation**: Shared logic in `src/tools/`, thin adapters in `src/interfaces/`

**Key files**:
- `src/tools/brain-boot.ts` - Multi-source fusion (semantic + GitHub + daily notes)
- `src/tools/pgvector-search.ts` - Dual-source (embeddings + active_context)
- `src/tools/ask-evna.ts` - LLM orchestrator (~600 lines)
- `src/lib/active-context-stream.ts` - Real-time ctx:: capture
- `src/mcp-server.ts` - External MCP (tools + resources)

**Two MCP servers**:
1. Internal (`src/interfaces/mcp.ts`) - For TUI/CLI agent
2. External (`src/mcp-server.ts`) - For Claude Desktop (includes resources)

**Dual-source search**: 70% embeddings / 30% active_context, semantic filtering with cosine similarity

## Tool Descriptions

### brain_boot
Morning check-in: semantic search + recent activity + GitHub + daily notes
- `query` (required), `project`, `lookbackDays`, `maxResults`, `githubUsername`, `includeDailyNote`

### semantic_search
Deep search across history via AutoRAG with structural filtering
- `query` (required), `limit`, `project`, `since`, `threshold`
- **Structural filter**: Project parameter filters to dispatch/ only (excludes personal daily/ notes)
- **Semantic matching**: AutoRAG query rewriting + BGE reranker find project-relevant content
- **Why**: Project is YAML metadata (`project: floatctl-rs`), not folder path. Trust semantic layer.

### active_context
Capture/query recent activity with ctx:: parsing
- Dual modes: `capture` (store) or `query` (retrieve)
- Cross-client surfacing (Desktop ↔ Claude Code)

### ask_evna
LLM orchestrator - interprets queries, coordinates tools
- Uses Anthropic SDK (not Agent SDK) for SearchSession control
- Multi-turn via database sessions
- Early termination with quality scoring (prevents 138k token burns)

## Recent Changes

**November 22, 2025**:
- **Structural filtering (CORRECTED)**: AutoRAG folder_filter changed to structural filtering
  - Project parameter filters to dispatch/ only (excludes personal daily notes)
  - Trust AutoRAG semantic matching for project relevance (query rewriting + BGE reranker)
  - Fixes flawed assumption: project is YAML frontmatter, not folder path
  - See: `sysops-log/2025-11-22-evna-autorag-structural-filtering.md`

**November 2025**:
- Fractal prevention: ask_evna can't call itself (separate internal MCP server)
- Timeout visibility: Shows partial progress via `floatctl claude show`
- Session management: Multi-turn conversations via Supabase

**October 2025**:
- ask_evna orchestrator with early termination logic
- Semantic filtering for active_context (true cosine similarity)
- MCP daily notes resources (`daily://today`, `daily://recent`, `daily://week`)

**Evolution note**: Migrated from pgvector-only to AutoRAG + active_context_stream (see vestigial organs in repo history)

## Development Patterns

1. **Typecheck before commits**: `bun run typecheck`
2. **Keep dual-source balance**: 70/30 embeddings/active without real data justification
3. **Structural filters, trust semantic matching**: Use folder_filter for structure (dispatch/ vs daily/), not semantics (project matching). Let AutoRAG handle semantic relevance.
4. **Smart truncation**: 400 chars, sentence-boundary aware (see `active-context-stream.ts`)
5. **MCP descriptions**: Operational focus, move philosophy to `evna-system-prompt.md`
6. **Cohere fallback**: All reranking must gracefully handle missing API key

## Full Docs

Planning docs in `/Users/evan/float-hub/float.dispatch/evna/docs/`:
- `ask-evna-implementation-plan.md` - Orchestrator design
- `future-enhancements.md` - Deferred features

Logs: `~/.floatctl/logs/evna-mcp.jsonl` (requires `EVNA_DEBUG=true`)

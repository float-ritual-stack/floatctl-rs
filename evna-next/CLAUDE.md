# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**EVNA-Next** is an AI agent built with the Claude Agent SDK that provides rich context synthesis and "brain boot" functionality for cognitive ecosystem workflows. It uses PostgreSQL/pgvector for semantic search across conversation history and exposes tools via the Model Context Protocol (MCP).

**Core purpose**: Morning check-ins, context restoration, and semantic search across past work with intelligent multi-source ranking (recent activity + historical embeddings + Cohere reranking).

## Build and Development Commands

```bash
# Development
bun run dev              # CLI with auto-reload
bun run start            # CLI (production)
bun run tui              # Terminal UI (OpenTUI-based)
bun run mcp-server       # MCP server for Claude Desktop

# Quality checks
bun run typecheck        # TypeScript type checking (REQUIRED before commits)
```

## Architecture: Dual-Source Search + Multi-Interface

### Core Pattern: Separation of Concerns

**Problem prevented**: "Three EVNAs" - CLI, TUI, and MCP implementations drifting out of sync.

**Solution**: Shared core logic with thin interface adapters.

```
src/
├── index.ts                    # Export-only public API (NO business logic)
├── core/
│   └── config.ts               # SINGLE source of truth: query options, system prompt, model config
├── tools/
│   ├── index.ts                # All Agent SDK tool definitions (brain_boot, semantic_search, active_context)
│   ├── brain-boot.ts           # Brain boot implementation (dual-source + Cohere reranking)
│   ├── pgvector-search.ts      # Dual-source search: embeddings + active_context
│   └── registry-zod.ts         # Zod schemas → JSON Schema conversion for MCP
├── interfaces/
│   ├── cli.ts                  # CLI runner (thin adapter)
│   ├── mcp.ts                  # Agent SDK MCP server (for TUI/CLI internal use)
│   └── tui/                    # Terminal UI (OpenTUI-based)
├── mcp-server.ts               # External MCP server for Claude Desktop (tools + resources)
└── lib/
    ├── db.ts                   # PostgreSQL/pgvector client
    ├── embeddings.ts           # OpenAI embeddings helper
    ├── active-context-stream.ts # Real-time message capture with ctx:: parsing
    ├── annotation-parser.ts    # ctx::, project::, meeting:: marker extraction
    └── cohere-reranker.ts      # Multi-source fusion with Cohere rerank API
```

### Key Architectural Principles

1. **DRY via Shared Config**: `core/config.ts` prevents duplication across CLI, TUI, MCP
2. **System Prompt Separation**: Internal knowledge (workspace context, project aliases) lives in `evna-system-prompt.md`, tool descriptions focus on operational "what/when/how"
3. **Dual-Source Search**: `pgvector-search.ts` combines recent activity (active_context_stream) + historical embeddings with semantic filtering
4. **Multi-Source Ranking**: Cohere reranks: semantic results + daily notes + GitHub status
5. **Export-Only Public API**: `src/index.ts` exposes clean interface, no business logic

### Dual-Source Search Pipeline (src/tools/pgvector-search.ts)

**Problem**: Recent activity dominated results (fake similarity 1.00), pushed historical embeddings off results.

**Solution**: Rabbit-turtle balance with true cosine similarity.

1. **Fetch dual sources in parallel**:
   - Active context stream (30% allocation, min 3)
   - Embeddings (2x limit for deduplication headroom)

2. **Semantic filtering** (300ms latency, accepted for relevance):
   - Embed query + batch embed active_context messages
   - Calculate cosine similarity, filter by threshold
   - NO fake 1.00 scores - only semantically relevant content surfaces

3. **Deduplication** with composite key:
   - `conversation_id + timestamp + content_prefix`
   - Fixes Rust CLI empty string IDs collision

4. **Source attribution**:
   - `source: 'active_context' | 'embeddings'`
   - Used by brain_boot for display formatting

**Result**: 70/30 historical/recent balance with honest similarity scores (0.45-1.00 gradient).

### Brain Boot Multi-Source Fusion (src/tools/brain-boot.ts)

**Architecture**: Parallel fetch → Cohere reranking → synthesis.

1. **Parallel fetch** (4 sources):
   - Dual-source semantic search (pgvector-search.ts)
   - Recent messages (last 20)
   - GitHub user status (PRs, issues)
   - Daily notes (last N days)

2. **Cohere reranking** (if API key provided):
   - Fuses all sources by relevance to query
   - Returns top N with `relevanceScore` (0-1)
   - Fallback: No Cohere → use semantic results as-is

3. **Smart truncation**:
   - 400 chars default, sentence/word boundary aware
   - Copied from `active-context-stream.ts` (proven pattern)

4. **Full daily note access**:
   - `includeDailyNote` parameter (defaults false)
   - Returns full note verbatim when true
   - Bridge solution before Phase 3 (burp-aware brain boot)

### MCP Architecture: Internal vs External

**Two separate MCP servers** for different boundaries:

1. **Internal MCP** (`src/interfaces/mcp.ts`):
   - Agent SDK's `createSdkMcpServer()`
   - Exposes tools to TUI/CLI agent
   - Agent has filesystem access, doesn't need resources
   - **Limitation**: Agent SDK doesn't support MCP resources yet

2. **External MCP** (`src/mcp-server.ts`):
   - Standard MCP SDK `Server` class
   - Exposes tools AND resources to Claude Desktop
   - Stdio transport for external clients
   - Resources: `daily://today` (+ TODOs for more)

**Why two servers?** Agent SDK's internal MCP wrapper doesn't support resources property. External clients (Claude Desktop) need both tools and resources in one server.

### Active Context Stream (src/lib/active-context-stream.ts)

**Purpose**: Real-time message capture with annotation parsing for cross-client context surfacing.

**Annotation system** (`ctx::`, `project::`, `meeting::`, `mode::`):
- Parsed by `annotation-parser.ts`
- Stored in `active_context_stream` table
- Enables: project filtering, meeting tracing, mode detection, persona tracking

**Cross-client surfacing**:
- Desktop ↔ Claude Code context sharing
- `client_type` field distinguishes origin
- `include_cross_client` parameter (defaults true)

**Smart truncation** (400 chars, sentence-boundary aware):
- Searches backwards from maxLength + 50 to find last sentence ending
- Falls back to word boundary if no good sentence break
- Last resort: hard truncate at maxLength
- **Copied to brain-boot.ts** for consistency

## Database Schema

PostgreSQL/pgvector with Supabase:

- `conversations` - Conversation metadata with ctx:: markers
- `messages` - Message content with project/meeting/mode annotations
- `embeddings` - Vector embeddings (OpenAI text-embedding-3-small)
- `active_context_stream` - Real-time capture with client_type

**Semantic search function**: `migrations/0001_semantic_search_function.sql` (already deployed).

## Environment Variables

Required in `.env`:

```bash
ANTHROPIC_API_KEY=...          # Claude API (required)
OPENAI_API_KEY=...             # Embeddings (required)
DATABASE_URL=postgresql://...  # Supabase/PostgreSQL (required)
SUPABASE_URL=...               # Supabase project URL (required)
SUPABASE_SERVICE_KEY=...       # Supabase service role key (required)
COHERE_API_KEY=...             # Cohere reranking (optional - graceful fallback)
```

## Tool Descriptions: Operational Focus

**MCP Best Practice**: Tool descriptions should be concise, focusing on "what/when/how" for operational use. Internal knowledge (workspace context, project aliases, philosophy) lives in `evna-system-prompt.md`.

**Result**: 38% token reduction (4.5k → 2.8k) by extracting workspace context to system prompt.

### brain_boot

Morning brain boot: Semantic search + recent context + GitHub status synthesis.

**Parameters**:
- `query` (required): Natural language description of what to retrieve
- `project` (optional): Filter by project name (e.g., "rangle/pharmacy")
- `lookbackDays` (optional): Days to look back (default: 7)
- `maxResults` (optional): Max results (default: 10)
- `githubUsername` (optional): Fetch GitHub PR/issue status
- `includeDailyNote` (optional): Return full daily note verbatim (default: false)

### semantic_search

Deep semantic search across conversation history using pgvector embeddings.

**Parameters**:
- `query` (required): Search query (natural language, question, or keywords)
- `limit` (optional): Max results (default: 10)
- `project` (optional): Filter by project name
- `since` (optional): Filter by timestamp (ISO 8601 format)
- `threshold` (optional): Similarity threshold 0-1 (default: 0.5, lower = more results)

### active_context

Capture and query recent activity with annotation parsing.

**Dual modes**:
1. **Capture mode** (`capture` parameter): Store annotated messages, parse ctx::, project::, meeting::
2. **Query mode** (`query` parameter): Retrieve recent context with cross-client surfacing

**Parameters**:
- `query` (optional): Search query for filtering context
- `capture` (optional): Message to capture to active context stream
- `limit` (optional): Max results (default: 10)
- `project` (optional): Filter by project name (fuzzy matching)
- `client_type` (optional): Filter by client ('desktop' | 'claude_code')
- `include_cross_client` (optional): Include context from other client (default: true)

## Recent Implementation (October 2025)

### Phase 2.2: Semantic Filtering + True Similarity (Commit 6279271)

**Problem**: Active_context ALWAYS returned (fake similarity 1.00) regardless of query relevance.

**Solution**:
1. Embed query + batch embed active_context messages
2. Calculate cosine similarity, filter by threshold
3. Replace fake 1.0 with actual similarity scores
4. Add `source` field ('active_context' | 'embeddings')

**Trade-off**: Accept 300ms latency for semantic relevance (embedding cache deferred to Phase 2.3).

### Brain Boot Improvements (Commit b95e0db)

1. Smart truncation (400 chars, sentence-boundary aware)
2. `includeDailyNote` parameter for full daily note access
3. MCP resource for daily notes (`daily://today`)

### TUI TypeScript Fixes (Commit 197eecd)

Fixed all 11 TypeScript errors in OpenTUI components:
- Changed constructor params from `CliRenderer` to `RenderContext` (correct OpenTUI API)
- Used definite assignment assertions for properties initialized in `setupUI()`
- Removed duplicate `focused` property - used protected `_focused` from base class

### MCP Consolidation (Commit 21de1ae)

Merged `mcp-external.ts` into `mcp-server.ts`:
- Single external MCP server exposing both tools AND resources
- Claude Desktop connects to one server, gets complete functionality

## Phase 3: Burp-Aware Brain Boot (DEFERRED, 4-6 hours)

**Vision**: Parse user's morning ramble for entities, questions, temporal markers, orchestrate tools adaptively.

**User quote**: "The morning ramble IS the context for what to surface"

**Why deferred**: Dogfood Phase 2.2 dual-source improvements first, observe real usage patterns before adding LLM synthesis layer.

**Implementation path** (when ready):
1. Burp Parser: Extract entities, questions, temporal markers from user message
2. Daily Note Structure Awareness: Parse timelog sections, identify projects/meetings
3. Adaptive Synthesis: Different strategies for morning ramble vs specific question vs return from break

**Bridge solution**: `includeDailyNote` parameter provides interim full-note access.

## Development Best Practices

1. **Always run typecheck before commits**: `bun run typecheck`
2. **Preserve dual-source balance**: Don't change rabbit-turtle allocation (30% active, 2x embeddings) without real usage data
3. **Use smart truncation**: Copy pattern from `active-context-stream.ts` (400 chars, sentence-boundary aware)
4. **MCP tool descriptions**: Keep operational, move internal knowledge to `evna-system-prompt.md`
5. **Cross-client context**: Test Desktop ↔ Claude Code surfacing when modifying active_context
6. **Cohere graceful degradation**: All reranking code must have fallback if `COHERE_API_KEY` missing

## File Modification Frequency

**High-change files** (likely to need updates):
- `src/tools/brain-boot.ts` - Brain boot synthesis logic
- `src/tools/pgvector-search.ts` - Dual-source search implementation
- `src/lib/active-context-stream.ts` - Active context capture
- `src/mcp-server.ts` - External MCP tools + resources

**Low-change files** (rarely modified):
- `src/core/config.ts` - Shared configuration (change propagates to all interfaces)
- `src/lib/db.ts` - Database client (stable)
- `src/lib/embeddings.ts` - OpenAI embeddings wrapper (stable)

## Integration Context

Part of the QTB cognitive ecosystem:
- **floatctl-rs**: Conversation export and embedding pipeline (Rust)
- **evna-next**: Context synthesis and brain boot (TypeScript/Agent SDK)
- **Claude Desktop**: Primary interface via MCP

Key principles:
- **LLMs as fuzzy compilers**: User burps → LLM parses → agent uses tools
- **Remember forward**: Continuously integrate past experiences into present understanding
- **Technology-mediated thought**: Externalizing memory through structured annotation

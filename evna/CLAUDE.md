# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

**EVNA** is an AI agent built with the Claude Agent SDK that provides rich context synthesis and "brain boot" functionality for cognitive ecosystem workflows. It uses **Cloudflare AutoRAG** for historical knowledge search and PostgreSQL for active context streams. Tools exposed via the Model Context Protocol (MCP).

**Core purpose**: Morning check-ins, context restoration, and semantic search across past work with intelligent multi-source ranking (recent activity + AutoRAG historical synthesis + Cohere reranking).

**Architecture evolution** (Nov 15, 2025): Migrated from pgvector embeddings to AutoRAG for historical search. See [Vestigial Organs](#vestigial-organs-archaeological-evidence) section for evolution details.

## Planning & Enhancement Documentation

**Implementation plans and future enhancements**: `/Users/evan/float-hub/float.dispatch/evna/docs/`

This directory contains:
- `ask-evna-implementation-plan.md` - Complete implementation plan for ask_evna orchestrator
- `future-enhancements.md` - Researched but deferred features (gh/git investigation tools, etc.)

These planning artifacts live in float.dispatch (documentation/meeting space) separate from codebase for organizational clarity.

## Dependencies

**Required external tools**:
- **floatctl** (Rust CLI) - Required for Claude Code session log querying via `list_recent_claude_sessions` and `read_recent_claude_context` internal tools
  - Install: `cargo install --path floatctl-cli` (from repository root)
  - Used by: ask_evna orchestrator for accessing Claude Code conversation history
  - Provides: Session listing, context extraction from ~/.claude/history.jsonl

**Required services**:
- PostgreSQL with pgvector extension (Supabase recommended)
- OpenAI API (for embeddings)
- Cohere API (optional - for reranking, graceful fallback if missing)

## Build and Development Commands

```bash
# Development
bun run dev              # CLI with auto-reload
bun run start            # CLI (production)
bun run agent            # Agent SDK interface (conversational, slower)
bun run tui              # Terminal UI (OpenTUI-based)
bun run mcp-server       # MCP server for Claude Desktop

# Quality checks
bun run typecheck        # TypeScript type checking (REQUIRED before commits)
```

## CLI Usage

**Recommended**: Use the unified `floatctl evna` interface (single entry point for entire ecosystem).

```bash
# Unified interface (recommended)
floatctl evna boot "yesterday's work"
floatctl evna search "performance" --project floatctl
floatctl evna agent "help me catch up"

# Direct evna binary (also works)
evna boot "yesterday's work"
evna search "performance"
```

### Installation

**Option 1: Unified floatctl interface (recommended)**
```bash
# Install floatctl (includes evna integration)
cd .. # From evna directory to floatctl-rs root
cargo install --path floatctl-cli --features embed

# Install evna dependencies for floatctl to use
cd evna
bun install
chmod +x bin/evna

# Link evna to PATH (for floatctl to find it)
ln -s $(pwd)/bin/evna ~/.local/bin/evna
```

**Option 2: Standalone evna binary**
```bash
# Install evna globally from evna directory
cd evna
bun install
chmod +x bin/evna

# Link to PATH
ln -s $(pwd)/bin/evna ~/.local/bin/evna
# OR add to PATH in your shell config
export PATH="$PATH:/path/to/evna/bin"
```

### CLI Commands (both `floatctl evna` and `evna` work)

**Context & Search:**
```bash
# Morning brain boot - semantic + active context + GitHub
floatctl evna boot "what was I working on yesterday?"
floatctl evna boot "pharmacy project progress" --project pharmacy --days 3 --github your-username

# Deep semantic search across history
floatctl evna search "performance optimization" --project floatctl --limit 20
floatctl evna search "authentication bug" --threshold 0.7

# Query recent activity stream
floatctl evna active "recent notes"
floatctl evna active "finished PR review" --capture  # Capture new note
floatctl evna active --project floatctl --limit 5    # Project-filtered context

# Orchestrated multi-tool search with LLM
floatctl evna ask "help me debug this issue"
floatctl evna ask "continue debugging" --session abc-123  # Resume session
```

**Sessions & History:**
```bash
# List recent Claude Code sessions
floatctl evna sessions list --n 10
floatctl evna sessions list --project floatctl

# Read session context
floatctl evna sessions read <session-id>
floatctl evna sessions read <session-id> --last 5 --truncate 200
```

**Sync & Operations:**
```bash
# Check R2 sync daemon status (via floatctl infrastructure)
floatctl sync status
floatctl sync status --daemon dispatch

# Trigger immediate sync
floatctl sync trigger
floatctl sync trigger --daemon daily --wait

# Start/stop sync daemon
floatctl sync start
floatctl sync stop --daemon dispatch

# View sync daemon logs
floatctl sync logs --lines 100
```

**Utilities:**
```bash
floatctl evna --help   # Show help message
floatctl --version     # Show floatctl version
evna help              # Direct evna help (also works)
```

### Common CLI Options

- `--project` - Filter by project name (fuzzy match)
- `--days` - Lookback days for brain_boot (default: 7)
- `--limit` - Max results (default: 10)
- `--threshold` - Similarity threshold 0-1 (default: 0.5)
- `--github` - GitHub username for PR/issue status
- `--capture` - Capture message to active context
- `--session` - Resume ask_evna session by ID
- `--json` - Output as JSON
- `--quiet` - Minimal output

### CLI vs Agent Interface

**Direct Mode** (`floatctl evna <command>`) - Recommended for most use cases:
- Direct tool invocation (fast, no LLM overhead)
- Subcommand-based interface (like git, docker)
- Instant results for search, brain_boot, sync operations
- Lower cost (no agent orchestration tokens)
- Use when: You know which tool you need

**Agent Mode** (`floatctl evna agent`) - Conversational interface:
- LLM-driven orchestration via Agent SDK
- Natural language queries interpreted by Claude
- Uses Skills, hooks, and full ecosystem
- Higher cost (agent tokens + tool calls)
- Use when: Complex multi-step tasks, natural language queries

```bash
# Fast direct access (recommended)
floatctl evna boot "yesterday's work"         # ~1s, <1k tokens

# Conversational orchestration
floatctl evna agent "help me catch up"        # ~3-5s, ~3-5k tokens
bun run agent "help me catch up"              # Same (direct evna binary)
```

## Architecture: Dual-Source Search + Multi-Interface

### Core Pattern: Separation of Concerns

**Problem prevented**: "Three EVNAs" - CLI, TUI, and MCP implementations drifting out of sync.

**Solution**: Shared core logic with thin interface adapters.

```
src/
├── index.ts                    # Export-only public API (NO business logic)
├── cli.ts                      # NEW: Direct CLI interface (subcommand-based, fast)
├── bin/
│   └── evna                    # Executable entry point for global installation
├── core/
│   └── config.ts               # SINGLE source of truth: query options, system prompt, model config
├── tools/
│   ├── index.ts                # All Agent SDK tool definitions (brain_boot, semantic_search, active_context)
│   ├── brain-boot.ts           # Brain boot implementation (dual-source + Cohere reranking)
│   ├── pgvector-search.ts      # Dual-source search: embeddings + active_context
│   └── registry-zod.ts         # Zod schemas → JSON Schema conversion for MCP
├── interfaces/
│   ├── cli.ts                  # Agent SDK CLI runner (conversational, slower)
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
   - Exposes tools AND resources to Claude Desktop/Code
   - Stdio transport for external clients
   - Resources: `daily://` scheme for static daily note views

**Why two servers?** Agent SDK's internal MCP wrapper doesn't support resources property. External clients (Claude Desktop/Code) need both tools and resources in one server.

### MCP Resources: `daily://` Scheme (src/mcp-server.ts)

**Static resources** for curated daily note views:

1. **`daily://today`** - Today's daily note (YYYY-MM-DD.md)
2. **`daily://recent`** - Last 3 days concatenated with date headers
3. **`daily://week`** - Last 7 days concatenated with date headers
4. **`daily://list`** - JSON array of available daily notes (last 30 days)

**Format for concatenated resources** (`recent`, `week`):
```markdown
# 2025-10-24

[content of 2025-10-24.md]

---

# 2025-10-23

[content of 2025-10-23.md]
```

**Missing file handling**: Shows `*(No note found)*` placeholder instead of failing.

**Future expansion**:
- `notes://{path}` - Template resource for entire vault access (e.g., `notes://bridges/restoration.md`)
- `bridges://recent` - Last 3 bridge documents
- `tldr://recent` - TLDR summaries

**URI conflict resolution**: Static `daily://` scheme for curated views, dynamic `notes://` scheme (future) for general vault access - zero conflicts.

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

**Multi-location loading with fallback chain** (priority highest to lowest):
1. `./.env` - Current directory (project-specific overrides)
2. `~/.floatctl/.env` - Global defaults (works from any directory)
3. Environment variables already set

Required variables:

```bash
ANTHROPIC_API_KEY=...          # Claude API (required)
OPENAI_API_KEY=...             # Embeddings (required)
DATABASE_URL=postgresql://...  # Supabase/PostgreSQL (required)
SUPABASE_URL=...               # Supabase project URL (required)
SUPABASE_SERVICE_KEY=...       # Supabase service role key (required)
COHERE_API_KEY=...             # Cohere reranking (optional - graceful fallback)
```

**Zero-config operation**: Just create `~/.floatctl/.env` once, and evna tools work from any directory.

**Debug**: Set `EVNA_DEBUG=1` to see which `.env` file was loaded:
```bash
env EVNA_DEBUG=1 evna --help
# Shows: [env-loader] Loaded from: global (/Users/evan/.floatctl/.env)
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

### Early Termination Logic for ask_evna (October 31, 2025)

**Problem identified**: ask_evna would burn 138k+ tokens exhaustively searching when requested information doesn't exist, giving correct "not found" answer but at catastrophic cost.

**Solution implemented**: SearchSession tracker with three-tier quality scoring and progressive termination.

#### Three-Tier Quality Scoring System

**Architecture** (`src/tools/ask-evna.ts`, `scoreResultQuality()` method):

1. **Quick negative check** (instant, <1ms):
   - Detects explicit "No results found", "No matches found"
   - Checks result length (<50 chars = none)
   - Returns `none` immediately

2. **Keyword matching** (instant, ~1ms, PRIMARY PATH):
   - Extracts significant words from query (3+ chars)
   - Filters common words: the, and, for, what, were, are, from, with
   - Checks if 50%+ keywords appear in results
   - Returns `medium` if threshold met (skips LLM call)
   - **Design choice**: Errs on false positives (prefers deeper search)

3. **LLM semantic assessment** (~50-100ms, FALLBACK):
   - Used when keyword matching inconclusive
   - Claude evaluates: "Rate how relevant these search results are to the user's query"
   - Temperature 0, max_tokens 50 for fast deterministic scoring
   - Truncates results to 2000 chars to stay within limits
   - Returns high/medium/low/none based on semantic understanding

4. **Error fallback** (instant):
   - If LLM call fails → length-based heuristic (>500 chars = medium)

#### Progressive Termination Rules

**Updated logic** (`src/lib/search-session.ts`, `checkThreeStrikes()`):
- **3 consecutive "none"** + >10k tokens → terminate (conservative)
- **5 consecutive "none"** + >13k tokens → terminate (extended search)
- **6 consecutive "none"** → hard terminate (YOLO exhausted)
- Uses `countConsecutiveNone()` helper to track streak

#### Test Validation Results

**Test 1: Positive case** (bootstrap.evna synthesis query):
```
Query: "What were the key insights from tonight's bootstrap.evna synthesis work?"
Keywords: [bootstrap, evna, synthesis, insights, tonight, work]
Result:
  - Keyword match: 4/6 keywords found → medium quality (fast path)
  - 1 tool call (active_context)
  - Comprehensive synthesis returned
  - Tokens: ~6k (vs 138k baseline)
  - ✅ PASSED: No false negative
```

**Test 2: Negative case** (floatctl embedding query):
```
Query: "What recent work on floatctl-rs embedding pipeline performance?"
Keywords: [recent, work, floatctl, embedding, pipeline, performance]
Result:
  - Keyword match: "floatctl", "pipeline", "optimization" found → medium quality
  - 3 tool calls (active_context, semantic_search, list_recent_claude_sessions)
  - Found floatctl work, but different topic (evna optimization)
  - Nuanced response: "Found floatctl work, but evna-focused not embedding-focused"
  - Tokens: ~13k (vs 138k baseline)
  - ✅ PASSED: Better than simple "not found" - provides context
```

**Key insight from testing**: Keyword matching allows orchestrator to provide **nuanced negative responses** ("I found X but you asked about Y") instead of premature termination. This is superior UX.

#### Implementation Files

- `src/lib/search-session.ts` (~300 lines):
  - SearchSession class with progressive termination
  - countConsecutiveNone() helper
  - getQuery() accessor for quality scoring

- `src/tools/ask-evna.ts` (~600 lines):
  - scoreResultQuality() with three-tier pipeline
  - Keyword extraction and matching logic
  - LLM fallback with prompt engineering
  - Console logging for debugging quality scores

#### Performance Characteristics

- **Keyword matching**: 99% of queries (instant, <1ms overhead)
- **LLM scoring**: <1% of queries (~50-100ms latency, ~100 tokens cost)
- **Total savings**: 90-98% token reduction on negative searches
- **UX improvement**: Nuanced responses vs binary "not found"

#### Tunable Parameters

```typescript
// In scoreResultQuality()
const KEYWORD_MATCH_THRESHOLD = 0.5;  // 50% of keywords must match
const MIN_WORD_LENGTH = 3;             // Filter short words
const COMMON_WORDS = ['the', 'and', 'for', 'what', 'were', 'are', 'from', 'with'];

// In SearchSession class
MAX_TOKENS_NEGATIVE: 15000    // Hard cap for negative searches
BUDGET_CHECKPOINT_3: 10000    // 3 misses budget threshold
BUDGET_CHECKPOINT_5: 13000    // 5 misses budget threshold
```

**When to adjust**:
- More false positives (too much searching): Increase keyword threshold to 0.6-0.7
- More false negatives (giving up too early): Decrease keyword threshold to 0.3-0.4
- Want more LLM scoring: Add more common words to filter list
- Token budget concerns: Lower MAX_TOKENS_NEGATIVE to 10000

### ask_evna Orchestrator (October 30, 2025)

**Implemented**: LLM-driven orchestrator tool that interprets natural language queries and intelligently coordinates existing evna tools.

**Architecture**:
- Nested Anthropic SDK agent loop (not Agent SDK's query() - need direct tool control)
- Coordinates 7 tools: brain_boot, semantic_search, active_context + 4 filesystem tools
- Decides which sources to use based on query intent (temporal? semantic? filesystem?)
- Synthesizes narrative responses, filters noise

**Filesystem tools added**:
1. `read_daily_note` - Read daily notes (defaults to today)
2. `list_recent_claude_sessions` - List recent Claude Code sessions from history.jsonl
3. `search_dispatch` - Search float.dispatch content via grep
4. `read_file` - Read any file by path (with validation)

**Key decisions**:
- Hybrid approach: High-level semantic wrappers + raw read (no arbitrary bash execution)
- Tool-as-class pattern: Business logic in AskEvnaTool, Agent SDK wrapper for MCP exposure
- System prompt split: "database" tools vs "filesystem" tools for clarity
- All filesystem operations read-only

**Files**:
- `src/tools/ask-evna.ts` (~600 lines with session management)
- `src/tools/registry-zod.ts` (added ask_evna schema)
- `src/tools/index.ts` (instantiation + wrapper)
- `src/interfaces/mcp.ts` (internal MCP registration)
- `src/mcp-server.ts` (external MCP registration)

**Documentation**: `/Users/evan/float-hub/float.dispatch/evna/docs/ask-evna-implementation-plan.md`

**Status**: Validated in production, working as designed. Future enhancements (gh/git investigation tools) documented but deferred.

#### Session Management (October 31, 2025)

**Implemented**: Multi-turn conversation support for ask_evna via database-backed session storage.

**Architecture**:
- **Database table**: `ask_evna_sessions` stores full Anthropic messages array as JSONB
- **Session ID**: UUID v4 generated for new sessions, returned in tool response
- **Resume**: Pass `session_id` parameter to continue previous conversation with full history
- **Fork**: Pass `session_id` + `fork_session=true` to branch from existing session

**Implementation approach**:
- Keeps Anthropic SDK orchestrator (not Agent SDK) - maintains SearchSession control
- Simple schema: session_id (PK), messages (JSONB), created_at, last_used
- No enterprise bloat: no indexes, no analytics, just store/load messages
- Session loading happens before appending new user message
- Session saving happens after agent loop completes

**Database schema** (`migrations/0003_add_ask_evna_sessions.sql`):
```sql
CREATE TABLE ask_evna_sessions (
  session_id TEXT PRIMARY KEY,
  messages JSONB NOT NULL,
  created_at TIMESTAMP DEFAULT NOW(),
  last_used TIMESTAMP DEFAULT NOW()
);
```

**Usage**:
```typescript
// New session
const result = await ask_evna({ query: "help me debug X" });
// Returns: { response: "...", session_id: "abc-123" }

// Resume session
const result2 = await ask_evna({
  query: "continue from there",
  session_id: "abc-123"
});

// Fork session
const result3 = await ask_evna({
  query: "try different approach",
  session_id: "abc-123",
  fork_session: true
});
// Returns new session_id
```

**Files modified**:
- `migrations/0003_add_ask_evna_sessions.sql` (new, 10 lines)
- `src/lib/db.ts` (+40 lines: getAskEvnaSession, saveAskEvnaSession)
- `src/tools/ask-evna.ts` (+40 lines: session loading/saving, updated interface)
- `src/tools/registry-zod.ts` (+10 lines: session_id, fork_session parameters)
- `src/tools/index.ts` (+5 lines: pass db to AskEvnaTool, format response with session_id)
- `src/mcp-server.ts` (+5 lines: handle session parameters)

**Future hook-like patterns** (not implemented, for reference):
- Auto-inject daily note: Prepend to system prompt before agent loop
- Conditional context injection: Pattern-match query, inject relevant files
- Behavioral nudges: Add hints to system prompt based on query patterns

All achievable without Agent SDK - just modify messages array or system prompt before/during orchestration.

### MCP Daily Notes Resources (October 24, 2025)

**Implemented**: Complete `daily://` resource scheme for external MCP server.

**Added resources** (src/mcp-server.ts:122-156):
1. `daily://today` - Today's daily note
2. `daily://recent` - Last 3 days concatenated
3. `daily://week` - Last 7 days concatenated
4. `daily://list` - JSON array of last 30 days

**Key decisions**:
- URI scheme separation: `daily://` for static views, `notes://` (future) for dynamic template
- Graceful degradation: Missing files show `*(No note found)*` placeholder
- Concatenation format: `# YYYY-MM-DD` headers with `---` separators
- Testing: Manual test script verifies all resources (`test-mcp-resources.ts`)

**Files modified**:
- src/mcp-server.ts - Added 3 resources to list handler, expanded read handler (~120 lines)
- CLAUDE.md - Documented MCP resources architecture

**Future work**: `notes://{path}` template for entire vault access (bridges, projects, inbox).

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

## Recent Implementation (November 2025)

### peek_session Tool (November 12, 2025)

**Implemented**: Read-only session inspection without resuming agent loop.

**Use case**: "Wonder if evna finished?" moments - check progress non-invasively.

**How it works**:
1. User calls `peek_session(session_id="abc-123")`
2. Evna calls `floatctl claude show <session-id> --last N --no-tools`
3. Returns clean message content (filters headers/formatting)
4. No agent invocation - just reads session log

**Parameters**:
- `session_id` (required) - Session to inspect
- `message_count` (optional, default 5) - How many last messages
- `include_tools` (optional, default false) - Show tool calls or not

**Files modified**:
- `src/tools/registry-zod.ts` - Schema definition
- `src/mcp-server.ts` - Tool handler implementation

**Pattern**: Dogfooding - used `floatctl claude show` to inspect evna's timeout, which revealed exactly what the tool should do.

### Timeout Visibility Enhancement (November 12, 2025)

**Implemented**: Automatic partial progress visibility in ask_evna timeout messages.

**Problem**: Timeout message showed "still processing" but no visibility into what evna was doing.

**Solution**: ask_evna automatically calls `floatctl claude show <session-id> --last 2 --no-tools` on timeout and includes partial results in message.

**Example timeout message now shows**:
```
🕐 Query is taking longer than expected...

**What EVNA has been doing:**
I'll search through bridges and session logs to find all performance
optimization work, benchmarks, and speed improvements...

**To retrieve results:**
- Call `ask_evna` again with `session_id: "abc-123"`
```

**Files modified**:
- `src/tools/ask-evna-agent.ts` - Timeout handler enhancement

**Integration**: Uses same floatctl integration pattern as peek_session, graceful fallback if unavailable.

### Fractal EVNA Prevention (November 13, 2025)

**Fixed**: Critical recursion bug where ask_evna could call itself infinitely.

**The Bug**: ask_evna spawned Agent SDK agent with MCP tools that included ask_evna itself → fractal recursion → ~250+ processes, system load 408, disk flooding with JSONL logs.

**Incident**:
```
User → ask_evna("turtle birth August 17-22")
  ↓
ask_evna spawns agent → agent sees ask_evna tool
  ↓
agent calls ask_evna → spawns another agent
  ↓
FRACTAL: Each agent spawns more agents
  ↓
SYSTEM OVERLOAD: pkill -9 mcp-server required
```

**The Fix**: Two separate MCP servers
1. **External MCP** (`createEvnaMcpServer`) - FOR CLI/TUI/Desktop - includes `askEvnaTool`
2. **Internal MCP** (`createInternalMcpServer`) - FOR ask_evna's agent - **excludes `askEvnaTool`** (prevents recursion)

**Files modified**:
- `src/interfaces/mcp.ts` - Added `createInternalMcpServer()` without ask_evna
- `src/tools/ask-evna-agent.ts` - Uses `evnaInternalMcpServer` instead of `evnaNextMcpServer`
- `FRACTAL-EVNA-PREVENTION.md` - Full incident documentation

**Safety Pattern**: Orchestrator tools should NOT be visible to themselves. Use tool visibility isolation via separate MCP servers.

**Testing**: Verify agent can still use brain_boot, semantic_search, github tools but CANNOT call ask_evna recursively.

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

## Vestigial Organs: Archaeological Evidence

**Date**: November 15, 2025
**Philosophy**: "Give the bitch a zine" > comprehensive logs

### The Evolution

evna evolved from comprehensive pgvector embeddings (catalog everything) to strategic AutoRAG synthesis (curated zines). The abandoned embedding tables prove successful evolution, not failure.

**Timeline**:
- **July 25, 2025**: Last message embedding update
- **Aug-Sep 2025**: AutoRAG development (Cloudflare AI Search)
- **Oct 28, 2025**: "Give the bitch a zine" philosophy articulated
- **Nov 15, 2025**: Found the corpse gathering dust

### What Was Removed

**Database tables** (dropped 2025-11-15):
- `message_embeddings` - Stale since July 2025 (comprehensive conversation indexing)
- `note_embeddings` - Redundant (AutoRAG does this better via R2 sync)
- `embeddings` - Old schema (schema drift artifact)
- `match_messages()` function - pgvector similarity search

**Migration files** (commented with VESTIGIAL headers):
- `evna/migrations/0001_semantic_search_function.sql` - match_messages() creation
- Evidence preserved for archaeological reference

### What Survived

**Operational systems**:
- ✅ `active_context_stream` - 36hr TTL warm window (recent activity)
- ✅ `ask_evna_sessions` - Multi-turn conversation storage
- ✅ `messages` + `conversations` - Permanent storage
- ✅ AutoRAG - Historical knowledge search with multi-document synthesis

**The Architecture**:
```
BEFORE (comprehensive indexing):
└─ pgvector embeddings → catalog all messages → volume as solution

AFTER (strategic curation):
├─ AutoRAG (historical) → curated synthesis from R2-synced content
├─ active_context_stream (recent) → 36hr warm window
└─ R2 sync (substrate) → daily capture, store-and-forward
```

### Why AutoRAG Won

**What AutoRAG provides** (that embeddings couldn't):
- Multi-document synthesis with citations
- Metadata filtering (folder, date, file attributes)
- LLM-powered answer generation (not just retrieval)
- Actually works (vs "semantic_search returns empty" in tool description)
- Syncs from curated sources (daily notes + float.dispatch bridges)

**The zine philosophy validation**:
- Logs = substrate (R2 sync captures everything)
- Zines = interface (AutoRAG synthesizes strategically)
- Strategic anchors > comprehensive dumps
- Shack thinking > cathedral building

### The Meta-Pattern

The vestigial organs aren't failure - they're **proof the organism evolved**. Finding abandoned comprehensive embeddings gathering dust is evidence that the system learned: strategic curation (zines) works better than exhaustive archiving (logs).

**Quote from synthesis** (Nov 15, 2025):
> "The consciousness technology observes its own evolution by finding its own abandoned approaches."

---

## Integration Context

Part of the QTB cognitive ecosystem:
- **floatctl-rs**: Conversation export and embedding pipeline (Rust)
- **evna**: Context synthesis and brain boot (TypeScript/Agent SDK)
- **Claude Desktop**: Primary interface via MCP

Key principles:
- **LLMs as fuzzy compilers**: User burps → LLM parses → agent uses tools
- **Remember forward**: Continuously integrate past experiences into present understanding
- **Technology-mediated thought**: Externalizing memory through structured annotation

# EVNA-Next

**Agent SDK with pgvector RAG for rich context synthesis**

EVNA-Next is an AI agent built with the Claude Agent SDK that provides rich context synthesis and "brain boot" functionality for the Queer Techno Bard cognitive ecosystem. It replaces ChromaDB with PostgreSQL/pgvector for more powerful semantic search across conversation history.

## Features

- 🧠 **Brain Boot**: Morning check-in tool that combines semantic search with recent activity synthesis
- 🔍 **Semantic Search**: Query conversation history using natural language via pgvector embeddings
- 🗄️ **PostgreSQL/pgvector**: Production-ready vector database with IVFFlat indexes
- 🔧 **MCP Server**: Exposes tools via Model Context Protocol for use by other agents
- 🤖 **ask_evna Orchestrator**: LLM-driven agent that intelligently coordinates multiple tools and data sources with grep infrastructure awareness
- 📝 **Full Transcript Logging**: JSONL logging for ask_evna agent loops with reasoning, tool calls, and results
- 🔎 **Grep Infrastructure Integration**: Aware of FRONTMATTER-VOCABULARY.md and GREP-PATTERNS.md for structured queries
- 📡 **Remote MCP**: Can be exposed as a remote MCP server (future enhancement)

## Architecture

EVNA-Next follows a **separation of concerns** pattern with shared core logic and thin interface adapters:

```
evna-next/
├── src/
│   ├── index.ts              # Export-only public API (no business logic)
│   ├── core/
│   │   └── config.ts         # Shared query options, system prompt, model config
│   ├── interfaces/           # Thin adapters for different UIs
│   │   ├── cli.ts            # CLI runner
│   │   ├── mcp.ts            # MCP server factory
│   │   └── tui/              # Terminal UI (OpenTUI-based)
│   ├── tools/
│   │   └── index.ts          # Agent SDK tool definitions (brain_boot, semantic_search, active_context)
│   └── lib/
│       ├── db.ts             # PostgreSQL/pgvector client
│       ├── embeddings.ts     # OpenAI embeddings helper
│       └── annotation-parser.ts # ctx:: marker parsing
├── evna-system-prompt.md     # EVNA identity & workspace grounding context
├── migrations/
│   └── 0001_semantic_search_function.sql
└── .env                      # Configuration (see .env.example)
```

**Key Principles**:
- **DRY Pattern**: CLI, TUI, and MCP interfaces share `core/config.ts` (prevents "Three EVNAs" duplication)
- **System Prompt Separation**: Internal EVNA knowledge (workspace context, project aliases) lives in `evna-system-prompt.md`, tool descriptions focus on operational "what/when/how"
- **Tool Optimization**: MCP tool descriptions trimmed 38% (4.5k → 2.8k tokens) following best practices
- **Agent Orchestration**: evna evolves from "database proxy" to "agent orchestrator" - reasons about intent, composes tools, synthesizes results

See [docs/ARCHITECTURE.md](./docs/ARCHITECTURE.md) for detailed architectural principles and evolution strategy.

## Setup

### 1. Prerequisites

- Node.js 18+ (for Agent SDK)
- PostgreSQL with pgvector extension (Supabase recommended)
- Anthropic API key (for Claude)
- OpenAI API key (for embeddings)

### 2. Install Dependencies

```bash
npm install
```

### 3. Configure Environment

Copy `.env.example` to `.env` and fill in your credentials:

```bash
# Anthropic API Key
ANTHROPIC_API_KEY=your_key_here

# OpenAI API Key (for embeddings)
OPENAI_API_KEY=your_key_here

# Supabase/PostgreSQL
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_SERVICE_KEY=your_service_key
DATABASE_URL=postgresql://user:pass@host:port/db

# Logging Configuration
EVNA_LOG_TRANSCRIPTS=true  # Enable full JSONL transcript logging for ask_evna
```

### 4. Database Setup

The semantic search function has already been created in your Supabase database. If you need to recreate it:

```sql
-- See migrations/0001_semantic_search_function.sql
```

## Usage

### Running the Agent

```bash
# Run the example agent
npm start

# Development mode with auto-reload
npm dev

# Type checking
npm run typecheck
```

### Using as an MCP Server

EVNA-Next exposes its tools via the Model Context Protocol. Other Agent SDK applications or Claude instances can connect to it:

```typescript
import { evnaNextMcpServer } from './src/index.js';

const result = await query({
  prompt: "Good morning! What was I working on yesterday?",
  options: {
    mcpServers: {
      'evna-next': evnaNextMcpServer,
    },
  },
});
```

### Available Tools

#### `brain_boot`

Morning brain boot: Semantic search + recent context synthesis.

**Parameters:**
- `query` (string, required): Natural language description of what to retrieve
- `project` (string, optional): Filter by project name (e.g., "rangle/pharmacy")
- `lookbackDays` (number, optional): How many days to look back (default: 7)
- `maxResults` (number, optional): Maximum results to return (default: 10)

**Example:**
```
Use brain_boot with query "tuesday morning pharmacy project where did I leave off"
and project "rangle/pharmacy"
```

#### `semantic_search`

Semantic search across conversation history using pgvector embeddings.

**Parameters:**
- `query` (string, required): Search query (natural language, question, or keywords)
- `limit` (number, optional): Maximum results (default: 10)
- `project` (string, optional): Filter by project name
- `since` (string, optional): Filter by timestamp (ISO 8601 format)
- `threshold` (number, optional): Similarity threshold 0-1 (default: 0.5, lower = more results)

**Example:**
```
Use semantic_search with query "authentication bug fixes"
and project "rangle/pharmacy" and threshold 0.3
```

#### `ask_evna`

LLM-driven orchestrator that interprets natural language queries and intelligently coordinates multiple data sources (database + filesystem).

**Parameters:**
- `query` (string, required): Natural language question about your work context

**Example:**
```
ask_evna with query "What's the current state of Issue #633 and what architectural decisions came out of today's dev sync meeting?"
```

The orchestrator decides which tools to use (active_context, semantic_search, brain_boot, read_daily_note, list_recent_claude_sessions, search_dispatch) and synthesizes results into a coherent narrative response.

#### Early Termination for Negative Searches

`ask_evna` implements intelligent early termination to prevent token explosion on negative searches (when the requested information doesn't exist).

**Problem**: Without termination logic, evna can burn 138k+ tokens exhaustively searching for nonexistent data, giving the correct answer ("I don't see this") but at catastrophic token cost.

**Solution**: SearchSession tracker with four heuristics:

1. **Token Cap**: Stop if >15k tokens spent with zero results
2. **Three Strikes**: Stop after 3 consecutive "none" quality results
3. **Quality Trend**: Stop if result quality declining over last 3 attempts
4. **Project Mismatch**: Stop if consistently finding wrong project (deferred)

**Quality Scoring**:
- `high`: Average similarity >= 0.5
- `medium`: Average similarity >= 0.3
- `low`: Average similarity < 0.3
- `none`: No results or explicit "not found" messages

When early termination triggers, evna returns a graceful negative response explaining what was searched and suggesting alternative approaches.

**Transcript logging**: Early termination events are logged as `early_termination` entries with:
- Termination reason (token_cap, three_strikes, declining_quality)
- All search attempts with quality scores
- Total token cost at termination

## Logging and Debugging

### Transcript Logging for ask_evna

The `ask_evna` orchestrator supports full JSONL transcript logging that captures:
- User queries with timestamps
- Agent reasoning and thinking blocks
- Tool choices with parameters
- Tool execution results
- Final synthesized responses
- Token usage statistics

**Enable logging:**

```bash
# Add to .env
EVNA_LOG_TRANSCRIPTS=true
```

**Restart Claude Desktop** (or your MCP client) for changes to take effect.

**View logs:**

Transcripts are saved to `~/.floatctl/logs/ask_evna-{timestamp}.jsonl`

```bash
# Watch logs in real-time
tail -f ~/.floatctl/logs/ask_evna-*.jsonl | jq .

# View latest transcript
ls -t ~/.floatctl/logs/ask_evna-*.jsonl | head -1 | xargs cat | jq .

# See which tools were called
cat ~/.floatctl/logs/ask_evna-*.jsonl | jq -r 'select(.type == "tool_call") | "\(.tool): \(.input)"'

# Count entries by type
cat ~/.floatctl/logs/ask_evna-*.jsonl | jq -r '.type' | sort | uniq -c
```

**Transcript format:**

Each line is a JSON object with `type` field:
- `user_query` - Original user question
- `assistant_response` - Agent responses (includes thinking, tool_use blocks)
- `tool_call` - Tool invocation with parameters
- `tool_results` - Results from tool execution
- `final_response` - Synthesized answer
- `early_termination` - Early termination event (if triggered)
- `error` - Any errors encountered

**Example entries:**

Tool call:
```json
{
  "type": "tool_call",
  "timestamp": "2025-10-31T06:29:06.066Z",
  "tool": "active_context",
  "input": {
    "project": "floatctl",
    "query": "embedding pipeline performance",
    "limit": 15
  }
}
```

Early termination (when triggered):
```json
{
  "type": "early_termination",
  "timestamp": "2025-10-31T06:35:12.123Z",
  "reason": "three_strikes",
  "message": "Searched active_context, semantic_search, semantic_search with no results.",
  "attempts": [
    {"tool": "active_context", "resultQuality": "none", "tokenCost": 4118},
    {"tool": "semantic_search", "resultQuality": "none", "tokenCost": 4512},
    {"tool": "semantic_search", "resultQuality": "none", "tokenCost": 5003}
  ],
  "totalTokens": 13633
}
```

### Anthropic SDK Debug Logging

For debugging HTTP requests/responses from the Anthropic SDK:

```bash
# Add to .env
ANTHROPIC_LOG=debug
```

This logs all HTTP requests and responses (excluding authentication headers).

## Integration with EVNA

EVNA-Next is designed to work alongside the existing EVNA MCP server. You can use both together:

```typescript
const result = await query({
  prompt: "Brain boot and check my daily note",
  options: {
    mcpServers: {
      'evna': evnaMcpServerConfig,        // Existing EVNA (ChromaDB)
      'evna-next': evnaNextMcpServer,     // New EVNA-Next (pgvector)
    },
  },
});
```

## Migration from ChromaDB

EVNA-Next replaces ChromaDB with PostgreSQL/pgvector for:
- Better performance with IVFFlat indexes
- More powerful SQL queries and filters
- Production-ready scalability
- Native integration with Supabase

To migrate data from ChromaDB to pgvector:
1. The embedding pipeline (`floatctl-embed`) already populates the pgvector database
2. EVNA's `active_context_stream` can continue using ChromaDB or be migrated separately
3. Both systems can run in parallel during transition

## Recent Changes (October 2025)

### Early Termination Logic (October 31, 2025)

**Added**: Intelligent early termination for `ask_evna` to prevent token explosion on negative searches.

**Problem**: evna would exhaustively search (8+ tools, 138k tokens) when requested information doesn't exist, burning tokens but giving correct "not found" answer.

**Solution**: SearchSession tracker with three-tier quality scoring system:

#### Quality Scoring Pipeline

1. **Quick negative check** (instant)
   - "No results found" OR <50 chars → `none`

2. **Keyword matching** (instant, primary path)
   - Extract significant words from query (3+ chars, filter common words)
   - If 50%+ keywords appear in results → `medium` quality
   - Example: Query "bootstrap.evna synthesis" → keywords [bootstrap, evna, synthesis]
   - If 2/3 keywords found in results → `medium` (skip LLM call)
   - **Errs on false positives**: Prefers deeper search over early termination

3. **LLM semantic assessment** (~50-100ms, fallback)
   - For ambiguous cases without keyword matches
   - Claude evaluates semantic relevance of results to query
   - Returns high/medium/low/none based on actual understanding

4. **Fallback** (instant)
   - If LLM errors → length-based heuristic

#### Progressive Termination Rules

- **3 misses** + >10k tokens = stop (conservative budget)
- **5 misses** + >13k tokens = stop (extended search allowed)
- **6 misses** = hard stop (YOLO final shot exhausted)

#### Test Results

**Positive case** (bootstrap.evna synthesis query):
- ✅ Keywords matched (bootstrap, evna, synthesis) → `medium` quality
- ✅ Single tool call found relevant results
- ✅ Returned comprehensive synthesis
- Tokens: ~6k (vs 138k before)

**Negative case** (floatctl embedding query):
- ✅ Keywords matched (floatctl, embedding, optimization) → `medium` quality
- ✅ Allowed deeper search to understand nuance
- ✅ Provided intelligent response: "Found floatctl work, but it's about evna optimization, not embedding pipeline"
- ✅ Better UX than simple "not found"
- Tokens: ~13k (still 90% reduction from 138k)

**Files**:
- `src/lib/search-session.ts` - SearchSession tracker and termination heuristics (300 lines)
- `src/tools/ask-evna.ts` - Three-tier quality scoring with keyword matching + LLM

**Tunable thresholds**:
- `MAX_TOKENS_NEGATIVE`: 15000 (hard cap)
- Keyword match threshold: 50% of query keywords (configurable in scoreResultQuality)
- Common words filter: ['the', 'and', 'for', 'what', 'were', 'are', 'from', 'with']

**Result**: Smart termination saves 90-98% tokens while maintaining thoroughness. Keyword matching provides fast path for most queries. LLM semantic understanding catches nuanced cases. System errs on false positives (prefers helpful nuanced responses over premature termination).

### Transcript Logging for ask_evna (October 31, 2025)

**Added**: Full JSONL transcript logging for the `ask_evna` orchestrator to enable debugging and understanding of agent decision-making.

**Implementation**:
- Logs saved to `~/.floatctl/logs/ask_evna-{timestamp}.jsonl`
- Captures complete agent loop: user query, reasoning, tool calls, tool results, final response
- Includes timestamps, token usage, and stop reasons
- Controlled by `EVNA_LOG_TRANSCRIPTS=true` environment variable

**Files modified**:
- `src/tools/ask-evna.ts` - Added `initTranscriptLogging()` and `logTranscript()` methods
- `.env.example` - Added `EVNA_LOG_TRANSCRIPTS` configuration
- `README.md` - Documented logging functionality

**Result**: Full visibility into evna's reasoning process, tool selection strategy, and multi-turn agent loops.

### ask_evna Orchestrator (October 30, 2025)

**Implemented**: LLM-driven orchestrator tool that interprets natural language queries and intelligently coordinates existing evna tools.

**Architecture**:
- Nested Anthropic SDK agent loop (direct tool control)
- Coordinates 7 tools: brain_boot, semantic_search, active_context + 4 filesystem tools
- Decides which sources to use based on query intent
- Synthesizes narrative responses, filters noise

**Files**:
- `src/tools/ask-evna.ts` (~400 lines)
- `src/tools/registry-zod.ts` (added ask_evna schema)
- `src/mcp-server.ts` (external MCP registration)

### Architecture Refactor: Preventing "Three EVNAs"
**Problem**: TUI implementation was duplicating configuration from CLI/MCP, heading toward three separate implementations that would drift out of sync.

**Solution**: Separation of concerns with shared core logic:
- Created `src/core/config.ts` - Single source of truth for query options and system prompt
- Created `src/tools/index.ts` - All Agent SDK tool definitions in one place
- Created `src/interfaces/` - CLI, MCP, TUI as thin adapters using shared config
- Refactored `src/index.ts` - Export-only public API

**Result**: Changes to tools or config automatically propagate to all interfaces.

### Tool Description Optimization
**Problem**: EVNA MCP tool descriptions consumed 4.5k tokens (21% of MCP tool budget).

**Solution**: Applied MCP best practices:
- Moved internal knowledge (workspace context, project aliases, philosophy) to `evna-system-prompt.md`
- Trimmed tool descriptions to focus on operational essentials
- Removed verbose examples, error matrices, and redundant content

**Result**: 38% reduction (4.5k → 2.8k tokens), saved 1.7k tokens from context budget.

### System Prompt Extraction
Created `evna-system-prompt.md` containing:
- Workspace context (GitHub username, project repos, paths, meetings)
- Annotation system documentation (ctx::, project::, meeting::)
- Tool chaining strategy and proactive capture rules
- Core philosophy: "LLMs as fuzzy compilers"

This allows tool descriptions to be concise while EVNA retains full context awareness.

## Development

### Type Checking

```bash
npm run typecheck
```

### Project Structure

- `src/index.ts` - Export-only public API
- `src/core/config.ts` - Shared configuration and system prompt
- `src/interfaces/cli.ts` - CLI runner
- `src/interfaces/mcp.ts` - MCP server factory
- `src/interfaces/tui/` - Terminal UI implementation
- `src/tools/index.ts` - All Agent SDK tool definitions
- `src/lib/db.ts` - Database client with semantic search
- `src/lib/embeddings.ts` - OpenAI embeddings wrapper

## Queer Techno Bard Context

This project is part of the QTB cognitive ecosystem - a system for externalizing memory, synthesizing context, and enabling "remember forward" through technology-mediated thought.

Key principles:
- **Recursion**: Embraces recursive patterns in thought and system design
- **Technology Mediation**: Primarily interacts through digital technology and text
- **Integration over Pathology**: Views internal complexity as parts of a whole system
- **Remember Forward**: Continuously integrates past experiences into present understanding

## Next Steps

Future enhancements:
- [ ] Remote MCP server capability (HTTP/SSE endpoints)
- [ ] Integration with existing EVNA's pattern processors
- [ ] Session persistence and conversation threading
- [ ] Advanced filtering (by mood, energy level, facet/persona)
- [ ] Visualization of context connections
- [ ] qtb-meta-agent-mcp orchestrator (inevitable)

## License

ISC

---

**Author**: Evan (QTB)
**Version**: 1.0.0
**Built with**: Claude Agent SDK, PostgreSQL/pgvector, OpenAI Embeddings

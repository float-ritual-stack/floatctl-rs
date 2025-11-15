# floatctl & evna Current State Documentation

**Generated:** 2025-11-15
**Purpose:** Ground new development in operational reality - what actually exists, what's used, what's deprecated

---

## Executive Summary

### Active vs Deprecated

| Component | Status | Primary Use |
|-----------|--------|-------------|
| **pgvector (Supabase)** | ✅ ACTIVE | evna's primary vector search backend |
| **Cloudflare Vectorize** | ❌ NOT FOUND | Never implemented or removed |
| **floatctl query commands** | ✅ ACTIVE | evna delegates all embedding searches here |
| **floatctl bridge commands** | ✅ ACTIVE | Bridge file management (index, append) |
| **floatctl claude commands** | ✅ ACTIVE | Claude Code session log parsing |
| **Chromadb** | ⚠️ UNKNOWN | Not found in current codebase |

**Key insight:** evna uses pgvector via floatctl CLI delegation pattern - evna orchestrates, floatctl executes searches.

---

## floatctl Command Reference

### Actual Commands (from main.rs)

```bash
floatctl <COMMAND>

# Conversation processing (core functionality)
floatctl split                  # Split conversations into individual files
floatctl ndjson                 # Convert JSON/ZIP to NDJSON (streaming)
floatctl explode                # Explode NDJSON into individual files
floatctl full-extract           # One-command workflow (auto-convert + split)

# Vector search (requires --features embed)
floatctl embed                  # Embed messages into pgvector
floatctl embed-notes            # Embed markdown notes into pgvector
floatctl query <SUBCOMMAND>     # Search embeddings
  ├── messages                  # Search message_embeddings table
  ├── notes                     # Search note_embeddings table
  ├── all                       # Search both tables
  └── active                    # Search active_context_stream (36hr hot cache)

# Integration tools
floatctl evna <SUBCOMMAND>      # Evna MCP server management
  ├── install                   # Install evna in Claude Desktop
  ├── uninstall                 # Uninstall evna
  ├── status                    # Show evna MCP status
  └── remote                    # Start remote MCP server (ngrok)

floatctl bridge <SUBCOMMAND>    # Bridge file management
  ├── index                     # Index :: annotations, create bridge stubs
  └── append                    # Append content to bridge files

floatctl claude <SUBCOMMAND>    # Claude Code session logs
  ├── list-sessions             # List recent sessions from ~/.claude/projects/
  ├── recent-context            # Extract context for system prompt injection
  └── show                      # Pretty-print session log

floatctl script <SUBCOMMAND>    # Shell script management
  ├── register                  # Register script to ~/.floatctl/scripts/
  ├── list                      # List registered scripts
  └── run                       # Run registered script with args

floatctl sync                   # R2 sync daemon management (implementation TBD)
floatctl completions <SHELL>    # Generate shell completions
```

---

## floatctl Implementation Details

### 1. Conversation Processing Pipeline

**Purpose:** Extract, organize, and split ChatGPT/Anthropic conversation exports.

#### `floatctl ndjson`

**What it does:**
- Streams large JSON arrays or ZIP files to NDJSON format
- O(1) memory usage via custom `JsonArrayStream` parser
- Handles both ChatGPT and Anthropic export formats

**How it works:**
- Custom JSON array parser (serde treats `[...]` as single value)
- Manually parses `[`, `,`, `]` structure
- Yields one conversation at a time
- Performance: 772MB → 756MB in ~4 seconds, <100MB memory

**CLI:**
```bash
floatctl ndjson --in conversations.json --out conversations.ndjson
floatctl ndjson --in export.zip --out conversations.ndjson  # ZIP support
```

**Implementation:** `floatctl-core/src/stream.rs:33-112` (JsonArrayStream)

#### `floatctl split`

**What it does:**
- Splits NDJSON into folder-per-conversation structure
- Generates Markdown, JSON, NDJSON outputs
- Extracts artifacts from `tool_use` blocks
- Creates filesystem-safe slugs: `YYYY-MM-DD-title`

**Output structure:**
```
~/.floatctl/conversation-exports/
├── 2025-11-10-conversation-title/
│   ├── 2025-11-10-conversation-title.md        # Markdown with YAML frontmatter
│   ├── 2025-11-10-conversation-title.json      # Preserved raw JSON
│   ├── 2025-11-10-conversation-title.ndjson    # Message-level records
│   └── artifacts/                               # Extracted code/artifacts
│       ├── 00-component-name.jsx
│       └── 01-visualization.html
└── messages.ndjson                              # Aggregate for embeddings
```

**CLI:**
```bash
floatctl split --in conversations.ndjson --out ./archive/ --format md,json,ndjson
floatctl split --in conversations.ndjson --dry-run  # Preview without writing
```

**Artifact extraction:**
- Searches for `tool_use` blocks with `name: "artifacts"`
- Maps `artifact_type` to extensions:
  - `application/vnd.ant.react` → `.jsx`
  - `text/html` → `.html`
  - `image/svg+xml` → `.svg`
  - `text/markdown` → `.md`

**Implementation:** `floatctl-core/src/pipeline.rs:95-126`

#### `floatctl full-extract`

**What it does:**
- One-command workflow: auto-detects format → converts → splits
- Handles JSON arrays, ZIP files, or pre-converted NDJSON

**Logic:**
1. Peek first byte of input file
2. If `[` → JSON array: convert to temp NDJSON
3. If `{` → Already NDJSON: skip conversion
4. If ZIP → extract, then convert
5. Run `split` on NDJSON
6. Clean up temp file (unless `--keep-ndjson`)

**CLI:**
```bash
floatctl full-extract --in export.json --out ./archive/
floatctl full-extract --in export.zip --out ./archive/ --keep-ndjson
```

**Performance:** 772MB, 2912 conversations in ~7 seconds, <100MB memory

---

### 2. Vector Search (pgvector via floatctl CLI)

**Architecture:** evna delegates all vector searches to floatctl CLI commands via `execFile()`.

#### `floatctl query messages`

**What it does:**
- Semantic search over `message_embeddings` table (pgvector)
- Filters by project, date range, similarity threshold
- Returns JSON for evna consumption

**Database function used:** `migrations/0001_semantic_search_function.sql`

**CLI:**
```bash
floatctl query messages "search term" \
  --json \
  --limit 10 \
  --project rangle/pharmacy \
  --days 7 \
  --threshold 0.5
```

**JSON output:**
```json
[
  {
    "content": "...",
    "role": "user",
    "project": "rangle/pharmacy",
    "meeting": "standup",
    "timestamp": "2025-11-10T14:30:00Z",
    "markers": ["ctx::rangle", "issue::656"],
    "conversation_title": "Performance optimization discussion",
    "conv_id": "abc-123",
    "similarity": 0.87
  }
]
```

**evna usage:** `evna/src/lib/db.ts:84-186` (semanticSearch method)

#### `floatctl query notes`

**What it does:**
- Semantic search over `note_embeddings` table
- Searches daily notes, bridges, imprints, TLDRs
- Same interface as `query messages`

**CLI:**
```bash
floatctl query notes "mycelial architecture patterns" \
  --json \
  --limit 10 \
  --threshold 0.5
```

**evna usage:** `evna/src/lib/db.ts:188-275` (semanticSearchNotes method)

#### `floatctl query active`

**What it does:**
- Queries `active_context_stream` table (36-hour hot cache)
- Recent activity across Desktop + Claude Code
- Cross-client context surfacing

**CLI:**
```bash
floatctl query active "recent work" \
  --json \
  --limit 10 \
  --project rangle/pharmacy \
  --client-type desktop
```

**Database table:** `active_context_stream` with fields:
- `message_id`, `conversation_id`, `role`, `content`, `timestamp`
- `client_type` ('desktop' | 'claude_code')
- `metadata` (JSONB with project, markers, etc.)
- `persisted_to_long_term` (boolean - double-write pattern)
- `persisted_message_id` (link to permanent `messages` table)

#### `floatctl embed`

**What it does:**
- Batch embed messages from NDJSON into pgvector
- Uses OpenAI text-embedding-3-small (1536 dimensions)
- Token-based chunking: 6000 tokens with 200-token overlap
- Idempotent: `--skip-existing` loads HashSet for O(1) lookup

**Chunking logic:**
- Messages >6000 tokens split into overlapping chunks
- Cached tokenizer (`once_cell::Lazy`) for 2-3x speedup
- Database schema: `(message_id, chunk_index)` composite PK

**CLI:**
```bash
floatctl embed --in messages.ndjson --skip-existing --batch-size 32
floatctl embed --in messages.ndjson --rate-limit-ms 500
```

**Implementation:** `floatctl-embed/src/lib.rs:42-115` (chunk_message function)

#### `floatctl embed-notes`

**What it does:**
- Embed markdown notes (daily notes, bridges, imprints) into `note_embeddings` table
- Same chunking/batching logic as `embed`
- Separate table for curated knowledge vs conversation history

**CLI:**
```bash
floatctl embed-notes --in ~/.evans-notes/daily/ --recursive
floatctl embed-notes --in ~/float-hub/float.dispatch/bridges/ --skip-existing
```

---

### 3. Bridge Commands

**Purpose:** Organize conversation content into bridge files based on `::` annotations.

#### `floatctl bridge index`

**What it does:**
- Scans markdown files for `project::` + `issue::` annotations
- Creates bridge stub files in `~/float-hub/float.dispatch/bridges/`
- Format: `{project}-{issue}.bridge.md`

**Annotation parsing:**
- `project::rangle/pharmacy` → project marker
- `issue::656` → issue marker
- `lf1m::topic` → collaboration request marker
- `meeting::identifier` → meeting marker

**CLI:**
```bash
floatctl bridge index ~/.evans-notes/daily/ --recursive --out ~/float-hub/float.dispatch/bridges/
floatctl bridge index conversation.md --json  # JSON output for scripting
```

**Output:**
```json
{
  "bridges_created": ["rangle-pharmacy-656.bridge.md", "float-hub-infrastructure.bridge.md"],
  "bridges_updated": ["existing-bridge.bridge.md"],
  "references_added": 5
}
```

**Implementation:** `floatctl-bridge/src/index.rs`

#### `floatctl bridge append`

**What it does:**
- Appends conversation content to appropriate bridge files
- Parses annotations from content
- Smart deduplication (content hash + timestamp window)
- Filters: min length, skip command-like messages

**CLI:**
```bash
# From stdin (hook usage)
echo "project::rangle/pharmacy issue::656\n\nContent here" | \
  floatctl bridge append --from-stdin --json

# From file
floatctl bridge append --file conversation.md --dry-run

# Explicit annotations
floatctl bridge append \
  --project rangle/pharmacy \
  --issue 656 \
  --content "Discussion about weighted blending performance"
```

**Options:**
- `--min-length 100` - Minimum content length (default: 100)
- `--require-both` - Require both project AND issue (default: false)
- `--skip-commands` - Skip command-like messages (default: true)
- `--dedup-window-secs 60` - Deduplication window (default: 60)

**Implementation:** `floatctl-bridge/src/append.rs`

---

### 4. Claude Code Session Logs

**Purpose:** Parse Claude Code session logs for evna integration (context injection).

#### `floatctl claude list-sessions`

**What it does:**
- Lists recent sessions from `~/.claude/projects/*/history.jsonl`
- Shows session ID, project, branch, timestamps, turn count

**CLI:**
```bash
floatctl claude list-sessions --limit 10
floatctl claude list-sessions --project floatctl-rs --format json
```

**Text output:**
```
# Recent Claude Code Sessions (10)

1. **1110-0832-realness**
   Project: /Users/evan/float-hub/floatctl-rs
   Branch: claude/feature-branch
   Started: 2025-11-10 08:32:00
   Turns: 47, Tool calls: 123

2. **1109-2235-champrad**
   ...
```

**Implementation:** `floatctl-claude/src/commands/list_sessions.rs`

#### `floatctl claude recent-context`

**What it does:**
- Extracts first N + last M messages from recent sessions
- Truncates messages to N characters (sentence-boundary aware)
- **Primary use case:** evna's `ask_evna` tool uses this for Claude Code context injection

**CLI:**
```bash
floatctl claude recent-context \
  --sessions 3 \
  --first 3 \
  --last 3 \
  --truncate 400 \
  --project floatctl-rs \
  --format json
```

**JSON output:**
```json
{
  "sessions": [
    {
      "session_id": "1110-0832-realness",
      "project": "/Users/evan/float-hub/floatctl-rs",
      "branch": "claude/feature-branch",
      "started": "2025-11-10T08:32:00Z",
      "first_messages": [
        {"role": "user", "content": "...", "truncated": true},
        {"role": "assistant", "content": "...", "truncated": false}
      ],
      "last_messages": [...]
      "stats": {"turn_count": 47, "tool_calls": 123, "failures": 2}
    }
  ]
}
```

**evna integration:** `evna/src/tools/ask-evna.ts` (calls via execFile)

**Implementation:** `floatctl-claude/src/commands/recent_context.rs`

#### `floatctl claude show`

**What it does:**
- Pretty-prints full session log
- Formats: text, markdown, json
- Options to hide thinking blocks or tool calls

**CLI:**
```bash
floatctl claude show 1110-0832-realness
floatctl claude show 1110-0832-realness --no-thinking --no-tools --format markdown
floatctl claude show ~/.claude/projects/xyz/1110-0832-realness.jsonl
```

**Implementation:** `floatctl-claude/src/commands/show.rs`

---

### 5. Script Management

**Purpose:** Register and run reusable shell scripts.

#### `floatctl script register`

**Security features:**
- Rejects symlinks (prevents symlink attacks)
- Validates shebang on Unix (checks for `#!/...`)
- Extension validation on Windows (`.bat`, `.cmd`, `.ps1`)
- Size limit: 10 MiB (prevents large file attacks)
- Path separator rejection (no `../` in names)

**CLI:**
```bash
floatctl script register ./useful-script.sh
floatctl script register ./script.sh --name custom-name --force
floatctl script register ./script.sh --dry-run  # Preview
```

**Storage:** `~/.floatctl/scripts/`

**Implementation:** `floatctl-cli/src/main.rs:1743-1831`

#### `floatctl script list`

**CLI:**
```bash
floatctl script list

# Output:
# Registered scripts in /Users/evan/.floatctl/scripts/:
#   backup-exports.sh (2341 bytes)
#   cleanup-temp.sh (856 bytes)
```

#### `floatctl script run`

**What it does:**
- Executes registered script with argument passthrough
- Real-time streaming output (not captured)
- Cross-platform: Unix uses shebang, Windows uses extension

**CLI:**
```bash
floatctl script run backup-exports.sh
floatctl script run cleanup-temp.sh --force --dry-run
```

---

### 6. Evna MCP Management

**Purpose:** Install/manage evna as MCP server in Claude Desktop.

#### `floatctl evna install`

**What it does:**
- Adds evna to `~/Library/Application Support/Claude/claude_desktop_config.json`
- Configures `bun run mcp-server` command
- Validates evna directory exists and has `package.json`

**CLI:**
```bash
floatctl evna install
floatctl evna install --path /custom/path/to/evna --force
```

**Config generated:**
```json
{
  "mcpServers": {
    "evna": {
      "command": "bun",
      "args": ["run", "mcp-server"],
      "cwd": "/Users/evan/float-hub/evna",
      "env": {
        "NODE_ENV": "production"
      }
    }
  }
}
```

#### `floatctl evna status`

**What it does:**
- Checks if evna is configured
- Validates evna directory exists
- Checks for `.env` file

#### `floatctl evna remote`

**What it does:**
- Starts remote MCP server (Supergateway + ngrok)
- Enables evna access from Claude Desktop/Code anywhere
- Supports reserved domains and basic auth

**CLI:**
```bash
floatctl evna remote
floatctl evna remote --port 3100 --no-tunnel  # Skip ngrok
floatctl evna remote --ngrok-domain my-domain.ngrok-free.app
```

**Output:** Generates both Claude Desktop and Claude Code config snippets with auth.

---

## evna Architecture & Database Usage

### Tables Used (PostgreSQL + pgvector via Supabase)

1. **`conversations`** - Conversation metadata
   - `id` (UUID PK), `conv_id` (string), `title`, `created_at`, `markers` (string[])

2. **`messages`** - Message content
   - `id` (UUID PK), `conversation_id` (FK), `idx`, `role`, `timestamp`
   - `content` (text), `project`, `meeting`, `markers` (string[])

3. **`embeddings`** - Vector embeddings (via floatctl, not direct access)
   - `(message_id, chunk_index)` composite PK
   - `embedding` (vector(1536)), `chunk_text`, `chunk_count`

4. **`note_embeddings`** - Note embeddings (bridges, daily notes, imprints)
   - Same schema as `embeddings`
   - Separate table for curated knowledge vs conversation history

5. **`active_context_stream`** - 36-hour hot cache
   - `message_id`, `conversation_id`, `role`, `content`, `timestamp`
   - `client_type` ('desktop' | 'claude_code')
   - `metadata` (JSONB - project, markers, ctx::, etc.)
   - `persisted_to_long_term` (boolean), `persisted_message_id` (UUID FK)

6. **`ask_evna_sessions`** - Multi-turn conversation sessions
   - `session_id` (text PK), `messages` (JSONB - Anthropic.MessageParam[])
   - `created_at`, `last_used`

### evna → floatctl Delegation Pattern

**Critical:** evna NEVER queries embeddings tables directly. All vector searches delegate to floatctl CLI.

**Why:**
- Separation of concerns: evna orchestrates, floatctl executes
- Security: `execFile()` prevents command injection
- Performance: Rust CLI is faster than Node.js for vector ops
- Maintainability: One implementation of search logic

**Code:**
```typescript
// evna/src/lib/db.ts:84-186
async semanticSearch(queryText: string, options: {...}): Promise<SearchResult[]> {
  const floatctlBin = process.env.FLOATCTL_BIN ?? 'floatctl';

  const args = ['query', 'messages', queryText, '--json', '--limit', String(limit)];
  if (project) args.push('--project', project);
  if (days) args.push('--days', String(days));

  const { stdout } = await execFileAsync(floatctlBin, args, {
    maxBuffer: 10 * 1024 * 1024,
    timeout: 60_000,
    env: { ...process.env, RUST_LOG: 'off' },
  });

  return JSON.parse(stdout).map(row => ({
    message: { ... },
    conversation: { ... },
    similarity: row.similarity,
    source: 'embeddings'
  }));
}
```

### evna Tools (MCP)

**Available tools:**

1. **`brain_boot`** - Morning brain boot with multi-source fusion
   - Semantic search (messages + notes)
   - Recent messages (last 20)
   - GitHub status (PRs, issues)
   - Daily notes (last N days)
   - Cohere reranking (optional)

2. **`semantic_search`** - Deep semantic search across conversation history
   - Delegates to `floatctl query messages`
   - Filters: project, since, threshold

3. **`active_context`** - Capture and query recent activity
   - Dual modes: capture (store) vs query (retrieve)
   - Cross-client surfacing (Desktop ↔ Claude Code)
   - Double-write pattern: hot cache + permanent storage

4. **`ask_evna`** - LLM-driven orchestrator (nested agent loop)
   - Coordinates all tools + filesystem tools
   - Multi-turn sessions (database-backed)
   - Early termination logic (3-tier quality scoring)

**MCP resources (daily notes):**
- `daily://today` - Today's daily note
- `daily://recent` - Last 3 days concatenated
- `daily://week` - Last 7 days concatenated
- `daily://list` - JSON array of last 30 days

---

## What's NOT Being Used

### Cloudflare Vectorize/AutoRAG

**Status:** ❌ NOT FOUND in evna codebase

**Evidence:**
- No `@cloudflare/*` packages in `package.json`
- No Cloudflare API calls in `src/lib/db.ts`
- No vectorize configuration files

**Conclusion:** Either never implemented or removed. pgvector is the sole vector backend.

### Chromadb

**Status:** ⚠️ UNKNOWN - not found in current floatctl or evna

**Note:** floatctl has `--features embed` for pgvector. No chroma feature flag or dependency found.

---

## Performance Characteristics

### floatctl Benchmarks (criterion)

From `ARCHITECTURE.md` and `LESSONS.md`:

**Streaming performance (3-conversation fixture):**
- `RawValueStream`: 22 µs
- `ConvStream`: 35 µs
- `Conversation::from_export`: 4.9 µs

**Real-world (772MB, 2912 conversations):**
- Convert to NDJSON: ~4s, <100MB memory
- Full extract: ~7s, <100MB memory
- Embedding batch (32 msgs): ~1-2s (OpenAI API call)

**Key optimizations:**
1. Manual JSON array parsing (not serde's StreamDeserializer)
2. `std::mem::take()` instead of cloning message arrays
3. `to_writer()` instead of `to_string() + write()`
4. Lazy regex compilation (`once_cell::Lazy`)
5. Tokenizer caching (2-3x speedup for chunking)

### evna Performance

**Vector search latency:**
- Semantic search (via floatctl): ~100-300ms (depends on embedding + pgvector query)
- Active context query (direct Supabase): ~50-100ms
- Brain boot (multi-source + Cohere): ~500-1000ms

**Token usage (ask_evna early termination):**
- Without termination: 138k+ tokens for negative searches
- With keyword matching (99% of queries): <1ms overhead, 6-13k tokens
- With LLM scoring (1% of queries): ~50-100ms, ~100 tokens

---

## Key File References

### floatctl

**CLI & Commands:**
- `floatctl-cli/src/main.rs` - All command definitions (2001 lines)

**Core functionality:**
- `floatctl-core/src/stream.rs:33-112` - JsonArrayStream (custom parser)
- `floatctl-core/src/conversation.rs` - Dual-format conversation parsing
- `floatctl-core/src/pipeline.rs` - Slug generation, artifact extraction

**Embeddings:**
- `floatctl-embed/src/lib.rs:42-115` - Token-based chunking
- `floatctl-embed/src/lib.rs:158-388` - Embedding pipeline
- `floatctl-embed/src/lib.rs:411-484` - Query implementation
- `migrations/0003_add_chunk_support.sql` - Multi-chunk schema

**Bridge:**
- `floatctl-bridge/src/index.rs` - Annotation indexing
- `floatctl-bridge/src/append.rs` - Content appending with deduplication

**Claude integration:**
- `floatctl-claude/src/stream.rs` - JSONL streaming parser
- `floatctl-claude/src/commands/list_sessions.rs` - Session discovery
- `floatctl-claude/src/commands/recent_context.rs` - Context extraction

### evna

**Core:**
- `evna/src/lib/db.ts` - Database client (floatctl delegation pattern)
- `evna/src/core/config.ts` - Shared config (query options, system prompt)

**Tools:**
- `evna/src/tools/brain-boot.ts` - Multi-source brain boot
- `evna/src/tools/pgvector-search.ts` - Dual-source search
- `evna/src/tools/ask-evna.ts` - LLM orchestrator (~600 lines)
- `evna/src/tools/index.ts` - Tool definitions (Agent SDK)

**MCP servers:**
- `evna/src/mcp-server.ts` - External MCP (tools + resources for Claude Desktop/Code)
- `evna/src/interfaces/mcp.ts` - Internal MCP (tools only for TUI/CLI agent)

**Database:**
- `evna/migrations/0001_semantic_search_function.sql` - pgvector search function
- `evna/migrations/0003_add_ask_evna_sessions.sql` - Session storage table

---

## Next Steps: float-hub Commands

**Based on this documentation, proposed `floatctl hub` commands should:**

1. **Follow existing patterns:**
   - JSON output for scripting (`--json`)
   - Boring, deterministic core with optional LLM enhancement (`--llm-model`)
   - Delegate to external tools (ollama, grep, fd) like evna delegates to floatctl

2. **Integrate with existing infrastructure:**
   - Use `floatctl embed-notes` for embedding new bridges/imprints
   - Use `floatctl bridge append` for automation
   - Follow evna's double-write pattern for context capture

3. **Avoid redundancy:**
   - Don't duplicate `bridge index/append` (already exists)
   - Don't duplicate `query` commands (already have messages/notes/active)
   - Don't create new vector backend (pgvector works, evna uses it)

4. **Fill actual gaps:**
   - Metadata validation (no existing command for frontmatter checking)
   - Inbox routing (no existing automation for file organization)
   - Dispatch parsing (:: markers not queryable yet)
   - Hook helpers (changelog append, context injection)
   - Async task queue (ollama bulk processing)

**Document will be updated as new commands are implemented.**

---

## Appendix: Environment Variables

**floatctl:**
```bash
DATABASE_URL="postgresql://user:pass@host:5433/dbname"  # pgvector connection
OPENAI_API_KEY="sk-..."                                 # Embeddings
RUST_LOG="info"                                         # Logging (use 'off' for JSON output)
FLOATCTL_BIN="/path/to/floatctl"                        # Override floatctl path (for evna)
```

**evna:**
```bash
ANTHROPIC_API_KEY="..."           # Claude API (required)
OPENAI_API_KEY="..."              # Embeddings (required)
DATABASE_URL="postgresql://..."   # Supabase/PostgreSQL (required)
SUPABASE_URL="..."                # Supabase project URL (required)
SUPABASE_SERVICE_KEY="..."        # Supabase service role key (required)
COHERE_API_KEY="..."              # Cohere reranking (optional - graceful fallback)
FLOATCTL_BIN="floatctl"           # Path to floatctl binary (defaults to 'floatctl' in PATH)
```

**floatctl evna remote (ngrok):**
```bash
EVNA_NGROK_AUTHTOKEN="..."        # ngrok auth token (or NGROK_AUTHTOKEN)
EVNA_NGROK_DOMAIN="..."           # Reserved ngrok domain (paid accounts)
EVNA_NGROK_AUTH="user:pass"       # Basic auth (format for ngrok --basic-auth)
```

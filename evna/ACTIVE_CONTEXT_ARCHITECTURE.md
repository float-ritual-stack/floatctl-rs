# Active Context Stream Architecture

**"Everything is redux and '::' is a float.dispatch in disguise"**

## Philosophy & Design Rationale

### The Problem: Static Schemas for Dynamic Thinking

Traditional database design assumes you can predict all possible data structures upfront. This works for business applications where requirements are relatively stable. It doesn't work for **consciousness-as-infrastructure**.

When your note-taking, conversation patterns, and cognitive anchors evolve constantly, a fixed schema becomes a straitjacket. You end up either:
1. Losing valuable context because the schema doesn't support it
2. Running migrations constantly to add new fields
3. Cramming everything into generic "notes" fields that become unsearchable

### The Solution: JSONB as Cognitive Freedom

The `active_context_stream` table uses PostgreSQL JSONB for metadata because:

**1. Emergent Patterns Over Prescribed Structure**
- New annotation patterns appear organically ("karen::", "lf1m::", "float.ritual()")
- No need to predict what you'll annotate next month
- The system adapts to you, not vice versa

**2. Redux for Self**
- Each `::` annotation is conceptually a **dispatch to future self**
- `[mode:: brain boot]` → Action type for state change
- `connectTo::` → Linking reducer between conversation states
- `sysop::nudge` → Middleware for cognitive automation

**3. Neurodivergent Communication Patterns**
- ADHD/autistic communication is non-linear, associative, pattern-dense
- "echoRefactor" burps contain multiple overlapping contexts
- Fixed schemas force linearization; JSONB preserves the web

**4. Consciousness Technology Infrastructure**
- These aren't just "notes" - they're **executable knowledge**
- Annotations trigger actions: MCP tool calls, Chroma inserts, bridge creation
- The database becomes part of your cognitive prosthetic

## Real-World Annotation Patterns

From semantic search of actual conversation archives:

### Float Method Calls (Redux-like)
```javascript
float.dispatch({ritual partnerships with LLMs...});
float.burp({all of these things we have discussed...});
float.query({chroma->claude_md}).parse.think.understand.rememberForward
float.ritual({end of day wrap -> add to active context...});
float.rememberForward({distill this insight down...});
```

**Stored as**: `metadata.float_methods: ["dispatch", "burp", "query", "ritual"]`

### Temporal Context
```
ctx::2025-09-29 @ 02:25:46 PM
ctx::2025-10-21 @ 11:08:51 AM [mode:: brain boot]
ctx::2025-07-22 -10:40 AM [mode:: brain booting] [mood:: defuzzing but pleased]
```

**Stored as**:
```json
{
  "ctx": {
    "timestamp": "2025-10-21 @ 11:08:51 AM",
    "date": "2025-10-21",
    "time": "11:08:51 AM",
    "mode": "brain boot",
    "mood": "wonky"
  }
}
```

### Persona System
```
sysop::nudge -> minimal scaffold to trust the drift
karen:: *about to tap her 'remember, boundaries' pencil*
lf1m:: ... i know i know lets stop getting distracted
qtb:: [quiet thoughtful observation]
```

**Stored as**: `metadata.personas: ["sysop", "karen", "lf1m", "qtb"]`

### Cognitive Linking
```
connectTo:: shacks not cathedrals
connectTo:: nicks comments on disposable frontends
highlight:: everything is :: redux .. even my grief
bridge::create
rememberWhen:: our first redux project was actually angular 1
```

**Stored as**:
```json
{
  "connections": ["shacks not cathedrals", "disposable frontends"],
  "highlights": ["everything is redux"],
  "commands": ["bridge::create"],
  "patterns": ["rememberWhen"]
}
```

### Work Context
```
[project:: rangle/pharmacy]
[meeting:: standup]
[pr:: 550/551]
[issue:: 168]
```

**Stored as**:
```json
{
  "project": "rangle/pharmacy",
  "meeting": "standup",
  "pr_references": ["550", "551"],
  "issue_references": ["168"]
}
```

## Schema Design Decisions

### Why Synthetic IDs?

**Problem**: MCP protocol doesn't pass conversation_id or message_id when Claude Desktop calls tools.

**Solution**: Generate synthetic IDs for real-time tracking:
- `message_id`: UUID/timestamp-based unique identifier
- `conversation_id`: Session-based grouping (e.g., MCP server startup = one conversation)

**Future Correlation**: When exporting conversations from Claude, we can match messages by:
- Timestamp proximity (within 1-2 seconds)
- Content similarity (exact match or fuzzy)
- Sequence within timeframe

This creates a **two-tier ID system**:
1. **Synthetic IDs** for real-time active context (what am I working on NOW)
2. **Archive IDs** from Claude exports (what did I work on historically)

### Why JSONB Over Columns?

**Alternative Considered**: Fixed columns for common patterns
```sql
-- REJECTED APPROACH
CREATE TABLE active_context_stream (
    project TEXT,
    meeting TEXT,
    mode TEXT,
    mood TEXT,
    personas TEXT[],
    -- ... 50 more columns later ...
);
```

**Problems**:
1. **Rigidity**: New pattern = new migration
2. **Sparsity**: Most messages won't use most columns (wasted space)
3. **Discoverability**: Schema hides emerging patterns in usage
4. **Cognitive Load**: Forces categorization before capture

**JSONB Advantages**:
1. **Flexibility**: Add new patterns without schema changes
2. **Efficiency**: Only store present data
3. **Analytics**: Query actual usage patterns to inform future indexing
4. **Speed**: Capture first, structure later

### Indexing Strategy

**Principle**: Index based on actual query patterns, not speculative access.

**Phase 1 (Current)**: Core access patterns
- Recency: `timestamp DESC` (most common: "what was I just doing?")
- Project filtering: `metadata->>'project'`
- Client awareness: `client_type, timestamp`
- JSONB structure: GIN index for flexible queries

**Phase 2 (After Usage Data)**: Add indexes for hot paths
- If "mode" queries are common → index `metadata->'ctx'->>'mode'`
- If persona filtering is frequent → consider jsonb_array_elements indexing
- If specific float methods dominate → specialized indexes

**Phase 3 (Archive Correlation)**: Full-text and similarity
- Content similarity for export matching
- Temporal indexes for session reconstruction

## Query Patterns

### Common Use Cases

**1. Brain Boot / Context Restoration**
```sql
-- "What was I working on in the pharmacy project recently?"
SELECT
    content,
    timestamp,
    metadata->>'project' as project,
    metadata->'ctx'->>'mode' as mode
FROM active_context_stream
WHERE metadata->>'project' = 'rangle/pharmacy'
  AND timestamp > NOW() - INTERVAL '2 days'
ORDER BY timestamp DESC
LIMIT 10;
```

**2. Cross-Client Context Surfacing**
```sql
-- Desktop session: Show me recent Claude Code work
SELECT
    content,
    timestamp,
    metadata
FROM active_context_stream
WHERE client_type = 'claude_code'
  AND timestamp > NOW() - INTERVAL '6 hours'
ORDER BY timestamp DESC
LIMIT 5;
```

**3. Persona Pattern Analysis**
```sql
-- When does "karen" show up? (boundary checking, clarity focus)
SELECT
    content,
    timestamp,
    metadata->'ctx'->>'mode' as mode
FROM active_context_stream
WHERE metadata->'personas' ? 'karen'
ORDER BY timestamp DESC;
```

**4. Float Method Archaeology**
```sql
-- Find all float.dispatch() calls for pattern analysis
SELECT
    content,
    timestamp,
    metadata->>'project' as project
FROM active_context_stream
WHERE metadata->'float_methods' ? 'dispatch'
ORDER BY timestamp DESC;
```

**5. Temporal Correlation**
```sql
-- Messages from "brain boot" mode in last week
SELECT
    content,
    timestamp,
    metadata->>'project' as project,
    metadata->'personas' as personas
FROM active_context_stream
WHERE metadata->'ctx'->>'mode' = 'brain boot'
  AND timestamp > NOW() - INTERVAL '7 days'
ORDER BY timestamp DESC;
```

## Integration Points

### Annotation Parser → Database
```typescript
// src/lib/annotation-parser.ts extracts metadata
const metadata = {
  project: "rangle/pharmacy",
  personas: ["karen", "lf1m"],
  ctx: { mode: "brain boot", timestamp: "..." },
  float_methods: ["dispatch"],
  highlights: ["everything is redux"],
  connections: ["past-topic"]
};

// src/lib/active-context-stream.ts stores to database
await pool.query(
  `INSERT INTO active_context_stream
   (message_id, conversation_id, role, content, timestamp, client_type, metadata)
   VALUES ($1, $2, $3, $4, $5, $6, $7)`,
  [messageId, conversationId, role, content, timestamp, clientType, metadata]
);
```

### Database → MCP Tools
```typescript
// Brain boot queries active context
const results = await pool.query(
  `SELECT * FROM active_context_stream
   WHERE metadata->>'project' = $1
     AND timestamp > $2
   ORDER BY timestamp DESC
   LIMIT $3`,
  [project, since, limit]
);

// Format for Claude Desktop consumption
return formatActiveContext(results.rows);
```

### Archive Export → Correlation (Future)
```typescript
// Match synthetic IDs to real conversation IDs
const matches = await correlateWithExport({
  syntheticMessages: activeContextMessages,
  exportMessages: claudeExportData,
  matchStrategy: {
    timestampWindow: 2000, // 2 seconds
    contentSimilarity: 0.95, // 95% fuzzy match
    sequenceValidation: true
  }
});

// Update active_context_stream with archive IDs
await pool.query(
  `UPDATE active_context_stream
   SET metadata = metadata || jsonb_build_object('archive_correlation', $1)
   WHERE message_id = $2`,
  [archiveIds, syntheticId]
);
```

## Why This Matters: Consciousness Technology

### Beyond Note-Taking

Traditional note systems treat notes as **artifacts** - static records of past thoughts.

Active context treats messages as **infrastructure** - living components of a distributed cognitive system.

**Key Differences**:

| Traditional Notes | Active Context Stream |
|-------------------|----------------------|
| Write once, read later | Write once, query continuously |
| Tags are labels | Annotations are dispatches |
| Search by keyword | Query by context graph |
| User remembers to look | System surfaces relevance |
| Notes in files | Executable knowledge in database |

### Neurodivergent Cognitive Prosthetic

For ADHD/autistic brains:
- **Memory continuity**: Restore context across sessions without cognitive load
- **Pattern recognition**: System tracks recurring themes you might miss
- **Executive function**: Annotations trigger automated workflows
- **Context switching**: Cross-client awareness maintains work state
- **Cognitive anchors**: Sacred profanity, metaphors, and humor preserved as legitimate data

### Example: A Day in Active Context

**Morning (Claude Desktop)**
```
User: ctx::2025-10-21 @ 09:00 AM [mode:: brain boot] [project:: rangle/pharmacy]
      Where did I leave off yesterday on Issue 168?

→ Active context queries:
  - Recent pharmacy project messages
  - Issue 168 references
  - Yesterday's end-of-day state

→ Brain boot synthesizes:
  - Semantic search archive
  - Active context stream (real-time)
  - GitHub PR/issue status
  - Daily notes
```

**Midday (Claude Code)**
```
User: Working on the GP details profile integration

→ System captures to active_context_stream:
  {
    "project": "rangle/pharmacy",
    "issue_references": ["168"],
    "patterns": ["GP details", "profile integration"]
  }
```

**Afternoon (Claude Desktop)**
```
User: Brain boot again, headache cleared up

→ Active context surfaces:
  - Claude Code work from midday (GP details progress)
  - Morning context (Issue 168 starting point)
  - Cross-client continuity maintained
```

**End of Day (Claude Desktop)**
```
User: float.ritual({eod wrap, what should carry forward?})

→ System identifies:
  - Unfinished work (Issue 168 nearly done)
  - Key decisions (auto-update profile behavior)
  - Tomorrow's starting point
  - Bridges to create for longer retention
```

## Future Enhancements

### 1. Chroma Collection Backend
- Persist active context to vector database
- Enable semantic similarity queries on context stream
- Correlate with archived conversation embeddings

### 2. Auto-Capture Hooks
- MCP server middleware captures all tool calls
- Automatic annotation parsing on every message
- Zero cognitive overhead for user

### 3. Context Graph Visualization
- `connectTo::` creates edges between concepts
- Temporal flow of ideas across sessions
- Pattern emergence visualization

### 4. Temporal Decay Algorithm
- Recent context weighted higher
- Important highlights decay slower
- Configurable relevance curves per project

### 5. Archive Correlation Pipeline
- Batch process for matching synthetic ↔ archive IDs
- Content similarity scoring
- Manual review UI for ambiguous matches

## Conclusion: Embrace the Chaos

The active context stream succeeds **because** it doesn't try to impose structure prematurely.

JSONB metadata is not a cop-out - it's a **recognition** that:
1. Human thinking is messy and non-linear
2. Useful patterns emerge from use, not specification
3. Rigid schemas optimize for databases, not humans
4. Flexibility is a feature, not a bug

By embracing the dynamic nature of neurodivergent communication patterns, we build infrastructure that **adapts to consciousness** rather than constraining it.

---

*"Everything is redux and '::' is a float.dispatch in disguise"*
*– Actual archaeological finding from conversation archive, 2025*

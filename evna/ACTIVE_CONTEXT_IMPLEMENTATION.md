# Active Context Stream Implementation

## Overview

Added real-time active context capture with data annotation parsing to EVNA-Next MCP server. Enables live message capture, annotation extraction, and client-aware context surfacing (Desktop ↔ Claude Code).

## Files Added

### 1. `src/lib/annotation-parser.ts`
**Purpose**: Parse data annotations from messages and extract metadata

**Supported Annotations**:
- `ctx::` - Temporal/context markers (timestamps, modes, metadata)
- `project::` - Project scoping
- `karen::`, `lf1m::`, `sysop::`, `evna::`, `qtb::` - Persona invocations
- `connectTo::` - Concept linking
- `highlight::`, `eureka::`, `gotcha::`, `insight::` - Discovery markers
- `pattern::`, `bridge::`, `note::` - Pattern annotations
- `float.*` commands - Dispatch actions

**Example**:
```typescript
const parser = new AnnotationParser();
const metadata = parser.extractMetadata(`
  ctx::2025-10-21 @ 08:25:54 AM - [project::float/evna] [mode::discovery]
  sysop:: look at the pipes doing what pipes do!
  connectTo:: conversation as infrastructure
`);
// Returns:
// {
//   ctx: { date: '2025-10-21', time: '08:25:54 AM', metadata: 'project::float/evna' },
//   project: 'float/evna',
//   personas: ['sysop'],
//   connections: ['conversation as infrastructure']
// }
```

### 2. `src/lib/active-context-stream.ts`
**Purpose**: Live message capture with annotation parsing and client-aware filtering

**Key Features**:
- Message capture with metadata extraction
- Client type detection (desktop vs claude_code)
- Cross-client context surfacing
- Session-aware filtering

**Example**:
```typescript
const stream = new ActiveContextStream(db);

// Capture message
await stream.captureMessage({
  conversation_id: 'conv_123',
  role: 'user',
  content: 'ctx::2025-10-21 project::float/evna sysop:: testing',
  client_type: 'desktop'
});

// Query with client-aware filtering
const messages = await stream.getClientAwareContext({
  isFirstMessage: false,
  project: 'float/evna'
});
// Returns: Messages from claude_code (opposite client)
```

### 3. `src/tools/active-context.ts`
**Purpose**: MCP tool for querying and capturing active context

**Tool Parameters**:
- `query` - Optional search query
- `capture` - Message to capture (with annotation parsing)
- `limit` - Max results (default: 10)
- `project` - Filter by project
- `client_type` - Filter by client type
- `include_cross_client` - Include opposite client context

**Example Usage in Claude Desktop**:
```
Use the active_context tool to capture this message:
ctx::2025-10-21 @ 09:00 AM - [project::pharmacy/assessment]
sysop:: implemented GP notification flow
connectTo:: issue #168 completion
```

## Integration with Existing Tools

### Brain Boot Enhancement
Modified `src/tools/brain-boot.ts` to include active context alongside:
- Semantic search (archived conversations)
- GitHub PR/issue status
- Daily notes from `~/.evans-notes/daily`

**Output Sections** (in order):
1. GitHub Status
2. Daily Notes
3. **Active Context** (NEW)
4. Semantically Relevant Context (archived)
5. Recent Activity

## MCP Server Updates

Added `active_context` tool to `src/mcp-server.ts`:

```typescript
{
  name: "active_context",
  description: "Query live active context stream with annotation parsing. Supports cross-client context surfacing (Desktop ↔ Claude Code).",
  inputSchema: {
    // ... parameters
  }
}
```

## Client-Aware Context Surfacing

### Desktop → Claude Code
When in Desktop, surfaces recent context from Claude Code sessions:
- Shows technical work, file changes, code discussions
- Excludes current Desktop session echoes

### Claude Code → Desktop
When in Claude Code, surfaces recent context from Desktop sessions:
- Shows conversational context, planning, reflections
- Excludes current Claude Code session echoes

### First Message Exception
On first message in new conversation:
- Surfaces all relevant context regardless of client
- Provides full context restoration

## Annotation Patterns Discovered

From archaeology of `/Users/evan/float-hub-operations/floatctl-rs/conv_out`:

### Temporal Markers
```
ctx::2025-10-21 @ 08:25:54 AM - [mode::discovery]
ctx::2025-07-28 - session complete - [mode:: semantic archival]
```

### Nested Persona Dialogue
```
- sysop:: look at the pipes doing what pipes do!
  - karen:: look at that, little nuggets of story around the links
    - lf1m:: fuck yeah!
      - evna:: 🎐🪕🛰️👣
```

### Redux-like Dispatches
```
float.dispatch({ type: "SESSION_WRAP_COMPLETE", payload: {...} })
float.ritual({ session wrap...})
float.trace({ everything is redux, source:: google drive })
```

### Concept Linking
```
connectTo:: conversation as infrastructure, prompt as ast, chat history as living substrate
```

### Project Scoping
```
project::float/evna
project::rangle/pharmacy
project::float-hub/ritual-forest/sysops-daydream
```

## Database Integration Notes

**Current Status**: Placeholder implementation using semantic search fallback

**Future Enhancement**: Add dedicated `active_context` table:
```sql
CREATE TABLE active_context (
  message_id UUID PRIMARY KEY,
  conversation_id UUID,
  role TEXT,
  content TEXT,
  timestamp TIMESTAMPTZ,
  client_type TEXT,
  project TEXT,
  personas TEXT[],
  connections TEXT[],
  highlights TEXT[],
  patterns TEXT[],
  metadata JSONB
);

CREATE INDEX idx_active_context_project ON active_context(project);
CREATE INDEX idx_active_context_client ON active_context(client_type);
CREATE INDEX idx_active_context_timestamp ON active_context(timestamp DESC);
```

## Usage Examples

### Capture and Query
```typescript
// In Claude Desktop or Claude Code:
active_context({
  capture: `
    ctx::2025-10-21 @ 09:30 AM - [project::pharmacy/assessment]
    sysop:: completed PR #582 review
    karen:: clean navigation fix, ready to merge
    connectTo:: issues #551, #550
  `,
  project: "pharmacy/assessment",
  limit: 5
})
```

### Brain Boot with Active Context
```typescript
brain_boot({
  query: "pharmacy assessment flow recent work",
  project: "rangle/pharmacy",
  githubUsername: "e-schultz",
  lookbackDays: 7
})
// Includes active context automatically
```

### Cross-Client Context
```typescript
// From Desktop:
active_context({
  client_type: "claude_code",
  include_cross_client: true,
  project: "float/evna"
})
// Returns recent Claude Code context (file changes, technical work)
```

## Future Enhancements

1. **Chroma Collection Integration**: Store active context in dedicated Chroma collection for richer semantic search
2. **Annotation DSL Formalization**: If patterns warrant, create formal grammar for annotations
3. **Cross-Conversation Concept Graph**: Build `connectTo::` relationship graph
4. **Temporal Decay**: Implement relevance decay for context surfacing
5. **Tool/Link Reference Detection**: Surface previous context when tools or links mentioned
6. **Real-time Persistence**: Auto-capture on every tool invocation (currently manual via `capture` parameter)

## Architecture Diagram

```
┌─────────────────────────────────────────────┐
│  User Message (with annotations)            │
│  ctx::2025-10-21 project::pharmacy          │
│  sysop:: testing feature                    │
└─────────────┬───────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│  AnnotationParser                           │
│  • Extract ctx::, project::, personas       │
│  • Parse temporal information               │
│  • Build metadata object                    │
└─────────────┬───────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│  ActiveContextStream                        │
│  • Store message + metadata                 │
│  • Detect client type (desktop/code)        │
│  • Index for querying                       │
└─────────────┬───────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│  Client-Aware Context Surfacing             │
│  • Desktop: Surface claude_code context     │
│  • Claude Code: Surface desktop context     │
│  • First message: All relevant context      │
└─────────────┬───────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────┐
│  Integration Points                         │
│  • brain_boot: Include in synthesis         │
│  • active_context tool: Direct access       │
│  • Future: Auto-capture on tool calls       │
└─────────────────────────────────────────────┘
```

## Testing

**Manual Testing**:
1. Start MCP server: `npm run mcp-server`
2. In Claude Desktop, invoke `active_context` tool with annotated message
3. Verify annotations are parsed and stored
4. Query context with filters
5. Test cross-client surfacing

**Unit Tests** (to be added):
```typescript
describe('AnnotationParser', () => {
  it('should parse ctx:: annotations', () => {
    const parser = new AnnotationParser();
    const result = parser.parseCtxAnnotation('2025-10-21 @ 08:25:54 AM - [mode::discovery]');
    expect(result.date).toBe('2025-10-21');
    expect(result.time).toBe('08:25:54 AM');
    expect(result.mode).toBe('discovery');
  });

  it('should extract all persona markers', () => {
    const parser = new AnnotationParser();
    const personas = parser.extractPersonas('sysop:: testing karen:: organizing lf1m:: chaos');
    expect(personas).toEqual(['sysop', 'karen', 'lf1m']);
  });
});
```

## Related Documentation

- **Annotation Pattern Examples**: See `/Users/evan/float-hub-operations/floatctl-rs/conv_out` for real usage
- **MCP Server Setup**: See `CLAUDE_DESKTOP_SETUP.md`
- **Brain Boot Integration**: See `src/tools/brain-boot.ts`

---

**Implementation Complete**: 2025-10-21
**Files Modified**: 5 (3 new, 2 updated)
**Lines of Code**: ~850 lines TypeScript

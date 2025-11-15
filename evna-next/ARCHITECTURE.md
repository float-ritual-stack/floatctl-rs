# EVNA-Next Architecture

This document explains the architecture of EVNA-Next and how it differs from the original EVNA implementation.

## Design Philosophy

EVNA-Next follows the same cognitive ecosystem principles as the original EVNA but adapts them for a web-based interface:

- **LLMs as fuzzy compilers**: User input → LLM parsing → tool execution
- **Remember forward**: Continuous integration of past experiences
- **Technology-mediated thought**: Externalizing memory through structured data

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                        EVNA-Next Web UI                         │
├─────────────────────────────────────────────────────────────────┤
│  Browser (React)                                                │
│  ├─ ChatInterface Component                                     │
│  │  ├─ Custom useChat hook                                      │
│  │  ├─ Streaming message display                                │
│  │  └─ Tool invocation visualization                            │
│  └─ shadcn/ui Components (Button, Card, Input)                  │
├─────────────────────────────────────────────────────────────────┤
│  Next.js 16 Server (Node.js Runtime)                            │
│  ├─ /api/chat Route (POST)                                      │
│  │  ├─ AI SDK streamText()                                      │
│  │  ├─ Tool execution                                            │
│  │  └─ Response streaming                                        │
│  └─ Lazy-loaded clients                                          │
│     ├─ Anthropic (Claude)                                        │
│     ├─ OpenAI (embeddings)                                       │
│     ├─ PostgreSQL (pgvector)                                     │
│     └─ Supabase                                                  │
├─────────────────────────────────────────────────────────────────┤
│  Shared Database Layer                                           │
│  ├─ PostgreSQL + pgvector                                        │
│  ├─ conversations, messages, embeddings                          │
│  └─ active_context_stream                                        │
└─────────────────────────────────────────────────────────────────┘
```

## Key Components

### 1. Client-Side (Browser)

**ChatInterface Component** (`components/chat-interface.tsx`)
- Custom streaming chat implementation
- Parses SSE (Server-Sent Events) from /api/chat
- Displays messages with tool invocations
- Visual feedback for brain_boot, semantic_search, active_context

**Custom useChat Hook**
- Manages message state
- Handles form submission
- Streams responses from API
- Implements client-side retry logic

### 2. Server-Side (Next.js)

**API Route** (`app/api/chat/route.ts`)
- Node.js runtime (required for pg module)
- Accepts POST with messages array
- Uses AI SDK's `streamText()` for streaming
- Exposes three tools to LLM

**Tool Definitions** (`lib/tools/`)
- `brain_boot`: Dual-source search (embeddings + active_context)
- `semantic_search`: pgvector similarity search
- `active_context`: Recent activity querying
- All use `dynamicTool()` from AI SDK v5

### 3. Database Layer

**Lazy Initialization** (`lib/db.ts`)
- Prevents build-time errors
- Clients created on first use
- Shared schema with original EVNA

**PostgreSQL Access**
- Direct connection via `pg` module
- Vector operations via pgvector extension
- Supabase for additional REST operations

## Comparison: EVNA vs EVNA-Next

| Aspect | Original EVNA | EVNA-Next |
|--------|---------------|-----------|
| **Runtime** | Bun + TypeScript | Node.js + Next.js |
| **AI SDK** | Claude Agent SDK | Vercel AI SDK v5 |
| **Interface** | CLI / TUI / MCP | Web UI (React) |
| **Deployment** | Local / Remote MCP | Web hosting (Vercel, etc.) |
| **UI Framework** | OpenTUI (terminal) | React + shadcn/ui |
| **Streaming** | Agent SDK | Vercel AI SDK |
| **Tool Calling** | Agent SDK tools | AI SDK dynamicTool() |
| **Database** | PostgreSQL/pgvector | PostgreSQL/pgvector (shared) |
| **Target Users** | CLI power users | Web users, mobile-friendly |

### Shared Components

Both implementations:
- Use same PostgreSQL/pgvector database
- Support same three core tools (brain_boot, semantic_search, active_context)
- Follow same annotation system (ctx::, project::, meeting::, mode::)
- Use Claude for LLM (via different SDKs)
- Use OpenAI for embeddings
- Support optional Cohere reranking

### When to Use Which

**Use Original EVNA (CLI/TUI/MCP) when:**
- Working in terminal-heavy workflows
- Need MCP integration with Claude Desktop/Code
- Want fastest possible interaction
- Prefer keyboard-driven interface
- Building automation scripts

**Use EVNA-Next (Web) when:**
- Need shareable UI for team members
- Want mobile/tablet access
- Prefer visual interface
- Need remote access from anywhere
- Building dashboards or integrations

## Tool Implementation Details

### AI SDK v5 Tool Pattern

```typescript
export const toolName = dynamicTool({
  description: "What the tool does",
  inputSchema: z.object({
    param: z.string().describe("Parameter description"),
  }),
  execute: async (input: any) => {
    const { param } = input;
    // Tool logic here
    return { success: true, data: ... };
  },
});
```

Key differences from Agent SDK:
- `inputSchema` instead of `parameters`
- `dynamicTool()` instead of `tool()`
- Input parameter is `any` (no automatic destructuring)
- Return value is plain object (no special formatting)

### Streaming Architecture

**Client → Server:**
```
POST /api/chat
{
  "messages": [
    { "role": "user", "content": "Brain boot" }
  ]
}
```

**Server → Client:**
```
data: {"content": "Let"}
data: {"content": " me"}
data: {"content": " search"}
data: [DONE]
```

Client reconstructs full message from chunks.

## AI Gateway Integration

Optional layer for:
- Request caching
- Rate limiting
- Analytics
- Cost tracking

Configured via environment variables:
```env
AI_GATEWAY_URL=https://gateway.ai.cloudflare.com/v1/account/gateway
AI_GATEWAY_API_KEY=...
```

When set, all Anthropic/OpenAI requests route through gateway.
When unset, direct API access is used.

## Database Schema (Shared)

```sql
-- Conversation metadata
conversations (
  id TEXT PRIMARY KEY,
  title TEXT,
  created_at TIMESTAMP,
  updated_at TIMESTAMP,
  project TEXT,
  meeting TEXT
)

-- Message content
messages (
  id TEXT PRIMARY KEY,
  conversation_id TEXT REFERENCES conversations(id),
  role TEXT,
  content TEXT,
  timestamp TIMESTAMP,
  project TEXT,
  meeting TEXT,
  mode TEXT
)

-- Vector embeddings
embeddings (
  conversation_id TEXT,
  message_id TEXT REFERENCES messages(id),
  embedding VECTOR(1536),  -- OpenAI text-embedding-3-small
  PRIMARY KEY (conversation_id, message_id)
)

-- Active context stream
active_context_stream (
  id SERIAL PRIMARY KEY,
  conversation_id TEXT,
  content TEXT,
  timestamp TIMESTAMP,
  project TEXT,
  meeting TEXT,
  mode TEXT,
  client_type TEXT  -- 'desktop' | 'claude_code' | 'web'
)
```

## Performance Characteristics

### Original EVNA
- Cold start: ~100ms (Bun)
- Tool execution: ~200-500ms (direct)
- Response time: ~1-2s (Claude)

### EVNA-Next
- Cold start: ~500ms-1s (Next.js)
- Tool execution: ~200-500ms (same DB)
- Response time: ~1-2s (same Claude)
- Network latency: +50-200ms (HTTP overhead)

Web interface adds HTTP overhead but provides broader accessibility.

## Security Considerations

### Environment Variables
- Never commit `.env` file
- Use separate keys for dev/prod
- Rotate API keys regularly

### Database Access
- Use connection pooling (pg.Pool)
- Validate all inputs
- Limit query results (prevent DoS)

### API Routes
- Rate limiting recommended (Vercel built-in)
- Input sanitization
- Error handling (don't leak secrets)

## Future Enhancements

Potential additions:
- [ ] Session persistence (save conversation history)
- [ ] Multi-user support (authentication)
- [ ] Real-time collaboration (WebSockets)
- [ ] Voice input/output (Web Speech API)
- [ ] Mobile-optimized views
- [ ] Dark mode toggle
- [ ] Export conversations (markdown/JSON)
- [ ] Integration with original EVNA's MCP tools

## Development Workflow

1. **Local Development**
   ```bash
   npm run dev  # Start dev server
   ```

2. **Type Checking**
   ```bash
   npm run build  # Compiles TypeScript
   ```

3. **Linting**
   ```bash
   npm run lint  # ESLint
   ```

4. **Production Build**
   ```bash
   npm run build
   npm start  # Production server
   ```

## Deployment Checklist

- [ ] Set all environment variables
- [ ] Test database connection
- [ ] Verify API keys work
- [ ] Test streaming responses
- [ ] Check error handling
- [ ] Monitor performance
- [ ] Set up logging (Vercel Analytics, etc.)

---

**Last Updated**: 2025-11-15  
**Version**: 1.0.0  
**Author**: Evan (QTB)

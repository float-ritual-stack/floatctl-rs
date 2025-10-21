# EVNA-Next

**Agent SDK with pgvector RAG for rich context synthesis**

EVNA-Next is an AI agent built with the Claude Agent SDK that provides rich context synthesis and "brain boot" functionality for the Queer Techno Bard cognitive ecosystem. It replaces ChromaDB with PostgreSQL/pgvector for more powerful semantic search across conversation history.

## Features

- 🧠 **Brain Boot**: Morning check-in tool that combines semantic search with recent activity synthesis
- 🔍 **Semantic Search**: Query conversation history using natural language via pgvector embeddings
- 🗄️ **PostgreSQL/pgvector**: Production-ready vector database with IVFFlat indexes
- 🔧 **MCP Server**: Exposes tools via Model Context Protocol for use by other agents
- 📡 **Remote MCP**: Can be exposed as a remote MCP server (future enhancement)

## Architecture

```
evna-next/
├── src/
│   ├── index.ts              # Main agent entry point with MCP server
│   ├── tools/
│   │   ├── brain-boot.ts     # Morning brain boot tool
│   │   └── pgvector-search.ts # Semantic search tool
│   └── lib/
│       ├── db.ts             # PostgreSQL/pgvector client
│       └── embeddings.ts     # OpenAI embeddings helper
├── migrations/
│   └── 0001_semantic_search_function.sql
├── package.json
├── tsconfig.json
└── .env                      # Configuration (see .env.example)
```

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

## Development

### Type Checking

```bash
npm run typecheck
```

### Project Structure

- `src/index.ts` - Main entry point, defines tools and MCP server
- `src/lib/db.ts` - Database client with semantic search
- `src/lib/embeddings.ts` - OpenAI embeddings wrapper
- `src/tools/brain-boot.ts` - Brain boot implementation
- `src/tools/pgvector-search.ts` - Semantic search implementation

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

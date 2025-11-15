# EVNA-Next

**Next.js Web Interface for EVNA - Context Synthesis & Semantic Search**

A modern web-based implementation of EVNA using Vercel AI SDK 6, Next.js 16, shadcn/ui, and AI Gateway integration. Built with the App Router, React Server Components, and streaming AI responses.

## Features

- 🧠 **Brain Boot**: Morning check-in tool combining semantic search with recent activity synthesis
- 🔍 **Semantic Search**: Deep search across conversation history using pgvector embeddings
- 📝 **Active Context**: Query recent activity across different clients (Desktop, Claude Code)
- ⚡ **Streaming Responses**: Real-time AI responses with Vercel AI SDK streaming
- 🎨 **Modern UI**: Beautiful interface built with shadcn/ui and Tailwind CSS
- 🔧 **Tool Integration**: Visual display of tool invocations (brain_boot, semantic_search, active_context)
- 🌐 **AI Gateway Support**: Optional integration with Cloudflare AI Gateway for API management

## Tech Stack

- **Next.js 16** - App Router with React Server Components
- **Vercel AI SDK 6** - Streaming AI responses and tool calling
- **shadcn/ui** - Beautiful, accessible UI components
- **Tailwind CSS** - Utility-first styling
- **TypeScript** - Type-safe development
- **PostgreSQL + pgvector** - Vector embeddings for semantic search
- **Anthropic Claude** - LLM provider (via AI SDK)
- **OpenAI** - Embeddings generation

## Prerequisites

- Node.js 18+ 
- PostgreSQL with pgvector extension (Supabase recommended)
- Anthropic API key (for Claude)
- OpenAI API key (for embeddings)
- Cohere API key (optional, for reranking)

## Setup

1. **Install dependencies**:
```bash
npm install
```

2. **Configure environment variables**:
```bash
cp .env.example .env
```

Edit `.env` with your credentials:
```env
ANTHROPIC_API_KEY=your_anthropic_key
OPENAI_API_KEY=your_openai_key
COHERE_API_KEY=your_cohere_key  # Optional

# Supabase/PostgreSQL
SUPABASE_URL=https://your-project.supabase.co
SUPABASE_SERVICE_KEY=your_service_key
DATABASE_URL=postgresql://user:pass@host:port/db

# Optional: AI Gateway
AI_GATEWAY_URL=https://gateway.ai.cloudflare.com/v1/your-account/your-gateway
AI_GATEWAY_API_KEY=your_gateway_key
```

3. **Database setup**:

The app uses the same PostgreSQL database as the existing EVNA implementation. Make sure you have:
- `conversations` table
- `messages` table
- `embeddings` table
- `active_context_stream` table

See `../evna/migrations/` for database schema.

4. **Run development server**:
```bash
npm run dev
```

Open [http://localhost:3000](http://localhost:3000) in your browser.

## Usage

### Chat Interface

The main interface provides a streaming chat experience with EVNA. Simply type your questions or requests:

- "Good morning! What was I working on yesterday?" (triggers brain_boot)
- "Search for conversations about authentication bugs" (triggers semantic_search)
- "Show me recent activity on the pharmacy project" (triggers active_context)

### Available Tools

EVNA has access to three main tools:

1. **brain_boot**: Morning brain boot combining semantic search + recent context
   - Parameters: query, project, lookbackDays, maxResults
   - Best for: Morning check-ins, context restoration

2. **semantic_search**: Deep semantic search across conversation history
   - Parameters: query, limit, project, since, threshold
   - Best for: Finding specific past discussions or topics

3. **active_context**: Query recent activity with annotation parsing
   - Parameters: query, limit, project
   - Best for: Recent work, cross-client context

### AI Gateway Integration

To use Cloudflare AI Gateway (or another gateway):

1. Set up your gateway at [Cloudflare Dashboard](https://dash.cloudflare.com/)
2. Add credentials to `.env`:
```env
AI_GATEWAY_URL=https://gateway.ai.cloudflare.com/v1/account/gateway
AI_GATEWAY_API_KEY=your_key
```

The gateway provides:
- Request caching
- Rate limiting
- Analytics
- Cost tracking
- Request logging

## Architecture

```
evna-next/
├── app/
│   ├── api/
│   │   └── chat/
│   │       └── route.ts          # Streaming AI endpoint
│   ├── layout.tsx                # Root layout
│   └── page.tsx                  # Main page (ChatInterface)
├── components/
│   ├── ui/                       # shadcn/ui components
│   │   ├── button.tsx
│   │   ├── card.tsx
│   │   └── input.tsx
│   └── chat-interface.tsx        # Main chat UI
├── lib/
│   ├── tools/                    # AI SDK tool definitions
│   │   ├── brain-boot.ts
│   │   ├── semantic-search.ts
│   │   ├── active-context.ts
│   │   └── index.ts
│   ├── ai-config.ts              # AI model configuration
│   ├── db.ts                     # Database client
│   ├── embeddings.ts             # OpenAI embeddings
│   └── utils.ts                  # Utility functions
└── .env                          # Environment variables
```

## Key Features Explained

### Streaming Responses

Uses Vercel AI SDK's `streamText()` for real-time streaming:
- Progressive message display
- Tool invocation visualization
- Loading states

### Tool Integration

All EVNA tools are exposed via the AI SDK's tool calling system:
- Automatic parameter validation with Zod schemas
- Structured tool results
- Visual feedback in UI when tools are invoked

### Database Integration

Direct PostgreSQL access for:
- Vector similarity search via pgvector
- Active context querying
- Cross-client context surfacing

## Development

```bash
# Development server (with hot reload)
npm run dev

# Type checking
npm run build

# Production build
npm run build && npm start

# Linting
npm run lint
```

## Comparison with Original EVNA

| Feature | Original EVNA | EVNA-Next |
|---------|--------------|-----------|
| Runtime | Bun + TypeScript | Node.js + Next.js |
| AI SDK | Claude Agent SDK | Vercel AI SDK 6 |
| Interface | CLI / TUI / MCP | Web UI |
| UI Framework | OpenTUI (terminal) | React + shadcn/ui |
| Deployment | Local / Remote MCP | Web (Vercel/any host) |
| Streaming | Agent SDK | Vercel AI SDK |

Both implementations share:
- Same PostgreSQL/pgvector database
- Same tool capabilities (brain_boot, semantic_search, active_context)
- Same annotation system (ctx::, project::, meeting::)
- Same cognitive ecosystem philosophy

## Deployment

### Vercel (Recommended)

1. Push to GitHub
2. Import project in Vercel
3. Add environment variables in Vercel dashboard
4. Deploy!

### Docker

```dockerfile
FROM node:18-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci
COPY . .
RUN npm run build
CMD ["npm", "start"]
```

### Other Platforms

Works on any Node.js hosting:
- Netlify
- Railway
- Render
- Fly.io

## License

ISC

## Related Projects

- **floatctl-rs**: Rust-based conversation processing and embedding pipeline
- **evna**: Original TypeScript/Bun EVNA with CLI/TUI/MCP interfaces

---

**Author**: Evan (QTB)  
**Built with**: Vercel AI SDK 6, Next.js 16, shadcn/ui, PostgreSQL/pgvector

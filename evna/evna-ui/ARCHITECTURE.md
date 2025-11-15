# EVNA Block Chat UI - Architecture

## Overview

The EVNA Block Chat UI is a production-ready implementation of a "block-based" chat interface built with:
- **Next.js 16** with App Router and React Server Components
- **Vercel AI SDK v6 (beta)** for streaming AI responses
- **Tiptap 3** for the sidebar continuous note editor
- **shadcn/ui** as the base component library
- **Tailwind CSS 4** for styling
- **TypeScript** and **Zod** for type safety

## Core Architectural Principles

### 1. Blocks Over Messages

Traditional chat interfaces use a linear message list. This implementation uses **blocks**:

```typescript
type Block = {
  id: string;
  blockType: 'userCommand' | 'agentResponse' | 'boardSummary' | 'structuredComponent' | 'error';
  role: 'user' | 'assistant' | 'system';
  content: string;
  metadata: BlockMetadata;
  structuredOutput?: StructuredOutput;
};
```

**Benefits:**
- Blocks are **composable** and can be nested or linked
- Each block has **stable identity** (ID) for referencing
- Blocks can be **extended** with new types without refactoring
- Blocks can contain **structured data** beyond plain text

### 2. Three-Panel Layout

The interface is divided into three main regions:

```
┌──────────────────────────────────────────────────────────┐
│  Sidebar (20%)  │  Main Blocks (50%)  │  Boards (30%)  │
│                 │                      │                 │
│  Tiptap Editor  │  Block Chat         │  BBS Boards    │
│  - Notes        │  - Commands         │  - Board List  │
│  - Markers      │  - Responses        │  - Board Detail│
│  - References   │  - Structured Data  │  - Posts       │
└──────────────────────────────────────────────────────────┘
```

Each panel is resizable using `react-resizable-panels`.

### 3. Tiptap as Document, Not Text Field

The sidebar uses **Tiptap** (not a simple textarea) because:

- **Custom nodes**: CommandMarker, BlockReference, BoardReference
- **Rich formatting**: Headings, lists, emphasis
- **Interactive components**: Can embed React components as node views
- **Extensible schema**: Add new node types without breaking existing content

Example custom node:

```typescript
export const CommandMarker = Node.create({
  name: 'commandMarker',
  group: 'block',
  content: 'inline*',
  addAttributes() {
    return {
      commandId: { default: null },
      timestamp: { default: null },
      command: { default: null },
    };
  },
  // Renders as a styled block with border and background
});
```

### 4. Structured Outputs as First-Class Citizens

AI responses can include **structured outputs** that map to React components:

```json
{
  "type": "boardSummary",
  "boardId": "board-123",
  "title": "Project Ideas",
  "items": [...]
}
```

The **dispatcher** routes these to appropriate handlers:

```typescript
dispatcher.registerHandler('boardSummary', (output) => {
  // Render a BBS board component
  // or update the boards panel
});
```

### 5. BBS-Style Boards, Not Websites

The right panel shows **structured boards** (like BBS boards), not arbitrary web pages:

```typescript
type Board = {
  id: string;
  title: string;
  tags: string[];
  posts: BoardPost[];
  createdAt: string;
  lastUpdatedAt: string;
};
```

**Why boards?**
- AI can **create and manage** boards programmatically
- Posts are **atomic**, with metadata and timestamps
- Dense, **information-rich** layout (modern CSS, not ASCII)
- Tags and search for **organization**

## Data Flow

### 1. User Command → AI Response

```
User types command
  ↓
Create userCommand block
  ↓
Send to /api/chat (streaming)
  ↓
Read stream, collect full response
  ↓
Parse for structured outputs
  ↓
Create agentResponse block
  ↓
Dispatch structured outputs
  ↓
Update UI
```

### 2. Structured Output Processing

```
AI returns JSON in response
  ↓
Dispatcher.parseResponse()
  ↓
Extract JSON from markdown code blocks
  ↓
Validate against Zod schemas
  ↓
Route to type-specific handlers
  ↓
Update blocks, boards, or sidebar
```

### 3. Sidebar Interaction

```
User writes in Tiptap editor
  ↓
Editor emits onUpdate event
  ↓
Store HTML content
  ↓
(Future: extract annotations like ctx::, project::)
  ↓
(Future: link to blocks via BlockReference nodes)
```

## Key Files and Responsibilities

### Types (`lib/types/`)

- `block.ts`: Block types, metadata, structured output schemas
- `board.ts`: Board and post schemas
- All validated with Zod for runtime safety

### AI Layer (`lib/ai/`)

- `config.ts`: Model configuration, system prompts
- `dispatcher.ts`: Structured output parsing and routing

### Tiptap (`lib/tiptap/`)

- `extensions.ts`: Custom nodes (CommandMarker, BlockReference, BoardReference)
- `getSidebarExtensions()`: Complete extension list for editor

### Board Management (`lib/boards/`)

- `store.ts`: In-memory board store (CRUD operations)
- `initializeSampleBoards()`: Seed data for demo

### Components

- `components/block-chat/`: Block rendering, command input, block list
- `components/sidebar-note/`: Tiptap editor wrapper
- `components/boards/`: Board cards, board detail view, boards panel
- `components/ui/`: Base shadcn/ui primitives (Button, Separator)

### API Routes

- `app/api/chat/route.ts`: Streaming AI endpoint using Vercel AI SDK v6

## Technology Choices & Tradeoffs

### Next.js 16 App Router

**Chosen because:**
- Server Components for optimized rendering
- Built-in API routes (no separate backend needed)
- Edge runtime support for low latency
- File-based routing

**Tradeoffs:**
- More complex mental model (Client vs Server Components)
- Some libraries not compatible with Server Components

### Vercel AI SDK v6 (Beta)

**Chosen because:**
- First-class streaming support
- Provider-agnostic (Anthropic, OpenAI, etc.)
- Built-in structured output support (when stable)
- Tight integration with Next.js

**Tradeoffs:**
- Beta API, may change
- Some features still in development
- Limited documentation compared to v3

### Tiptap 3

**Chosen because:**
- ProseMirror-based (robust, proven)
- React node views for custom components
- Extensible schema
- Actively maintained

**Tradeoffs:**
- Larger bundle size than plain textarea
- More complex API than Slate or Draft.js
- Node view types can be tricky

### In-Memory Board Store

**Current:**
- Simple Map-based store in `lib/boards/store.ts`

**Future:**
- Replace with database (PostgreSQL/pgvector like existing EVNA)
- Or keep in-memory but add persistence via localStorage/sessionStorage

## Extending the System

### Adding a New Block Type

1. Add to `BlockType` enum in `lib/types/block.ts`
2. Create renderer in `components/block-chat/block-item.tsx`
3. Add to dispatcher if it comes from AI

Example:

```typescript
// In block.ts
export const BlockType = z.enum([
  ...,
  'codeExecution', // New type
]);

// In block-item.tsx
{block.blockType === 'codeExecution' && (
  <CodeExecutionView block={block} />
)}
```

### Adding a New Tiptap Node

1. Define node in `lib/tiptap/extensions.ts`
2. Add to `getSidebarExtensions()`
3. Style in `components/sidebar-note/sidebar-editor.tsx`

Example:

```typescript
export const MeetingNote = Node.create({
  name: 'meetingNote',
  group: 'block',
  content: 'block+',
  addAttributes() {
    return {
      meetingId: { default: null },
      date: { default: null },
    };
  },
  parseHTML() {
    return [{ tag: 'div[data-meeting-note]' }];
  },
  renderHTML({ HTMLAttributes }) {
    return ['div', { 'data-meeting-note': '', ...HTMLAttributes }, 0];
  },
});
```

### Adding a New Structured Output Type

1. Define schema in `lib/types/block.ts`
2. Add to `StructuredOutputSchema` union
3. Register handler in dispatcher

Example:

```typescript
// In block.ts
export const DataVisualizationOutputSchema = z.object({
  type: z.literal('dataVisualization'),
  chartType: z.enum(['bar', 'line', 'pie']),
  data: z.array(z.object({
    label: z.string(),
    value: z.number(),
  })),
});

// In dispatcher setup
dispatcher.registerHandler('dataVisualization', (output) => {
  // Render chart component
});
```

## Future Enhancements

### Phase 1 (Core Features)
- [ ] Persistent storage (database integration)
- [ ] Block reordering (drag-and-drop)
- [ ] Block nesting (threads)
- [ ] Sidebar → Block linking via BlockReference nodes

### Phase 2 (Advanced Features)
- [ ] Multi-agent support (different AI personas)
- [ ] Board creation from AI
- [ ] Advanced Tiptap features (collaboration, comments)
- [ ] Export blocks/boards to markdown

### Phase 3 (Integration)
- [ ] Connect to existing EVNA MCP server
- [ ] Integrate with existing tools (brain_boot, semantic_search)
- [ ] pgvector integration for semantic search within UI
- [ ] Real-time collaboration (multiple users)

## Performance Considerations

### Bundle Size
- Tiptap + extensions: ~150KB (gzipped)
- AI SDK: ~50KB (gzipped)
- Total first load: ~300KB (acceptable for a work tool)

### Streaming
- AI responses stream character-by-character
- Blocks update incrementally
- No full page reloads

### State Management
- React useState for simple UI state
- Could migrate to Zustand/Jotai for complex state later
- Server state could use TanStack Query

## Security Considerations

### API Keys
- ANTHROPIC_API_KEY stored in `.env.local` (never committed)
- Edge runtime for API routes (no Node.js APIs exposed)

### Input Validation
- All structured outputs validated with Zod schemas
- Prevents malicious JSON from breaking UI

### Content Security
- Next.js default CSP headers
- No `dangerouslySetInnerHTML` (except Tiptap's controlled renderer)

## Testing Strategy

### Unit Tests
- Zod schemas (block types, board types)
- Dispatcher logic (parsing, routing)
- Board store (CRUD operations)

### Integration Tests
- Block rendering
- Tiptap editor interactions
- API route streaming

### E2E Tests
- Full user flow (command → response → block)
- Board creation and viewing
- Sidebar editing and saving

## Development Workflow

```bash
# Install dependencies
npm install

# Run development server
npm run dev

# Type check
npm run typecheck

# Lint
npm run lint

# Build for production
npm run build

# Start production server
npm start
```

## Deployment

### Vercel (Recommended)
```bash
vercel deploy
```

### Other Platforms
- Docker: Create Dockerfile with Next.js standalone output
- Node.js: `npm run build && npm start`
- Static export: Not recommended (loses API routes)

## License

ISC - Part of the floatctl-rs project

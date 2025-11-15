# EVNA Block Chat UI

A modern "block chat" interface for the EVNA AI agent, built with Next.js 16, Vercel AI SDK v6, and Tiptap.

## Architecture

This is NOT a generic chat UI. It's designed as "chat as document / blocks" with a three-panel layout:

### Three Main Regions

1. **Left Sidebar (20%)**: Continuous Tiptap note editor
   - Write freeform notes
   - Insert command markers when commands are sent
   - Reference blocks from the main area
   - Custom node views for interactive components

2. **Main Block Chat Area (50%)**: Block-based conversation
   - Each exchange is a discrete block (not linear messages)
   - Blocks can be user commands, agent responses, board summaries, structured components
   - Blocks are composable and can be rearranged
   - Supports structured outputs from AI

3. **Right Panel (30%)**: BBS-style boards
   - Dense, BBS-inspired layout (modern CSS, no ASCII)
   - Boards have posts, tags, timestamps
   - Created/referenced by AI agents via structured outputs

## Tech Stack

- **Next.js 16**: App Router, React 19, Server Actions
- **Vercel AI SDK v6 (beta)**: Streaming responses, structured outputs
- **Tiptap**: Rich text editor with custom nodes and React views
- **shadcn/ui**: Base UI components
- **Tailwind CSS 4**: Styling
- **TypeScript**: Type safety throughout
- **Zod**: Schema validation for blocks and structured outputs

## Key Concepts

### Block System

Blocks are the fundamental unit of interaction:

```typescript
type Block = {
  id: string;
  blockType: 'userCommand' | 'agentResponse' | 'boardSummary' | 'structuredComponent' | 'error';
  role: 'user' | 'assistant' | 'system';
  content: string;
  metadata: {
    timestamp: string;
    agent?: string;
    associatedBoardId?: string;
    sidebarMarkerRange?: { from: number; to: number };
  };
  structuredOutput?: StructuredOutput;
};
```

### Structured Outputs

AI agents respond with structured JSON that maps to React components:

- `boardSummary`: Display a BBS board with posts
- `noteDecoration`: Add styling to the sidebar
- More types can be added via the dispatcher

### Tiptap Custom Nodes

- `CommandMarker`: Marks when a command was issued
- `BlockReference`: References a block in the main area
- `BoardReference`: References a board

These enable linking between the sidebar, blocks, and boards.

## Project Structure

```
evna-ui/
├── app/
│   ├── api/chat/route.ts       # AI SDK streaming endpoint
│   ├── layout.tsx               # Root layout
│   └── page.tsx                 # Main three-panel page
├── components/
│   ├── block-chat/              # Block-based chat UI
│   │   ├── block-item.tsx       # Individual block renderer
│   │   ├── block-list.tsx       # List of blocks
│   │   ├── command-input.tsx    # Command input field
│   │   └── index.tsx            # Main block chat component
│   ├── sidebar-note/            # Tiptap sidebar
│   │   ├── sidebar-editor.tsx   # Tiptap editor wrapper
│   │   └── index.tsx            # Sidebar container
│   ├── boards/                  # BBS board components
│   │   ├── board-card.tsx       # Board preview card
│   │   ├── board-detail.tsx     # Full board view
│   │   └── index.tsx            # Boards panel
│   └── ui/                      # Base shadcn/ui components
│       ├── button.tsx
│       └── separator.tsx
├── lib/
│   ├── ai/
│   │   ├── config.ts            # AI model configuration
│   │   └── dispatcher.ts        # Structured output dispatcher
│   ├── tiptap/
│   │   └── extensions.ts        # Custom Tiptap nodes
│   ├── boards/
│   │   └── store.ts             # In-memory board store
│   ├── types/
│   │   ├── block.ts             # Block type definitions
│   │   ├── board.ts             # Board type definitions
│   │   └── index.ts             # Type exports
│   └── utils.ts                 # Utility functions
└── README-EVNA-UI.md            # This file
```

## Getting Started

### Prerequisites

- Node.js 18+
- Anthropic API key

### Installation

```bash
cd evna-ui
npm install
```

### Configuration

Create `.env.local`:

```bash
ANTHROPIC_API_KEY=your_anthropic_api_key_here
```

### Development

```bash
npm run dev
```

Open [http://localhost:3000](http://localhost:3000)

### Building

```bash
npm run build
npm start
```

## Usage

1. **Write notes** in the left sidebar using Tiptap
2. **Send commands** in the main area using the command input
3. **View boards** in the right panel (BBS-style)
4. **Interact with blocks** - click to select, see metadata
5. **Structured outputs** from AI automatically render as custom components

## Extending the System

### Adding New Block Types

1. Add to `BlockType` enum in `lib/types/block.ts`
2. Create renderer in `components/block-chat/block-item.tsx`
3. Add to structured output dispatcher if needed

### Adding New Tiptap Nodes

1. Define node in `lib/tiptap/extensions.ts`
2. Add to `getSidebarExtensions()`
3. Style in `components/sidebar-note/sidebar-editor.tsx`
4. Optionally create React node view for complex components

### Adding New Structured Outputs

1. Define schema in `lib/types/block.ts`
2. Add to `StructuredOutputSchema` discriminated union
3. Register handler in `lib/ai/dispatcher.ts`
4. Create React component renderer

## Architecture Decisions

### Why Blocks Instead of Messages?

Traditional chat UIs are linear and ephemeral. Blocks are:
- **Composable**: Can be rearranged and nested
- **Persistent**: Have stable IDs and metadata
- **Extensible**: New block types = new capabilities
- **Linkable**: Can reference from sidebar or other blocks

### Why Tiptap for Sidebar?

Tiptap provides:
- Custom nodes with React views (not just markdown)
- Precise range selection for linking
- Extensible schema for future features
- Professional editing experience

### Why BBS-style Boards?

Not arbitrary websites, but structured information spaces:
- AI can create and manage boards
- Posts are atomic units of information
- Dense, information-rich layout
- Tags and timestamps for organization

## Future Enhancements

- [ ] Drag-and-drop block reordering
- [ ] Block nesting (threads)
- [ ] Advanced Tiptap features (comments, collaboration)
- [ ] Persistent storage (database integration)
- [ ] Board creation from AI
- [ ] Multi-agent support
- [ ] Real-time collaboration
- [ ] Export blocks/boards to markdown

## Integration with Existing EVNA

This UI can connect to the existing EVNA MCP server:

1. The existing tools (`brain_boot`, `semantic_search`, `active_context`) are available via structured outputs
2. AI Gateway can route requests to different models/providers
3. Board data can be backed by the same PostgreSQL/pgvector database

## License

ISC - Part of the floatctl-rs project

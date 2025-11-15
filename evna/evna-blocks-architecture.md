# EVNA Blocks Architecture

## Overview

**evna-blocks** is a Next.js 16 web interface for evna that reimagines the AI interaction paradigm as a continuous, block-based workspace with BBS board integration.

**Core Concept**: Instead of traditional linear chat, the interface is a **living document** where:
- Left sidebar = continuous TipTap editor (like Notion/Linear's command interface)
- Right pane = BBS board preview/canvas
- Agent responses are **inserted as blocks** with custom React components
- Structured outputs → dynamic component rendering

## Tech Stack

- **Next.js 16** (App Router, React Server Components)
- **Vercel AI SDK 6 Beta** (Agent abstraction, structured outputs)
- **AI Elements** (shadcn/ui-based chat components)
- **TipTap** (Rich text editor with custom node views)
- **shadcn/ui** (Component library)
- **AI Gateway** (Rate limiting, caching, observability)
- **TypeScript** (Strict mode)

## Architecture Principles

### 1. Block-First Design

Traditional chat is linear:
```
User: Query
Assistant: Response
User: Follow-up
Assistant: Response
```

evna-blocks is **compositional**:
```
┌─────────────────────────────┐
│ ## Morning brain boot       │  ← User-written heading
├─────────────────────────────┤
│ [CommandMarker: /brain_boot]│  ← Command triggers agent
├─────────────────────────────┤
│ ╔═══════════════════════════╗│
│ ║ Brain Boot Results        ║│  ← Structured output component
│ ║ • Recent work: ...        ║│
│ ║ • GitHub PRs: ...         ║│
│ ║ [Show more]               ║│
│ ╚═══════════════════════════╝│
├─────────────────────────────┤
│ Notes from yesterday:       │  ← User continues writing
│ - Follow up on...           │
└─────────────────────────────┘
```

### 2. TipTap as Foundation

**Why TipTap?**
- Native block/document model (ProseMirror)
- Custom React node views (interactive components)
- Collaborative editing support (future)
- Command palette (slash commands)
- Rich content editing

**Architecture for Advanced Features**:

```typescript
// Extension system designed for growth
src/
├── editor/
│   ├── extensions/
│   │   ├── index.ts                 // Extension registry
│   │   ├── command-marker.ts        // /brain_boot → marker node
│   │   ├── agent-response-block.ts  // Container for agent output
│   │   ├── structured-output-block.ts // Dynamic component renderer
│   │   ├── bbs-board-embed.ts       // Inline board previews
│   │   └── collaboration.ts         // (Future: Yjs integration)
│   ├── nodes/
│   │   ├── command-marker/
│   │   │   ├── component.tsx        // React view
│   │   │   ├── node.ts              // ProseMirror node spec
│   │   │   └── plugin.ts            // Behavior (autocomplete, etc.)
│   │   ├── agent-response/
│   │   │   ├── component.tsx
│   │   │   ├── node.ts
│   │   │   └── types.ts             // Response data schema
│   │   └── structured-output/
│   │       ├── component.tsx        // Renders child components
│   │       ├── node.ts
│   │       └── registry.tsx         // Component type → React component
│   └── config.ts                    // Editor configuration
```

### 3. AI SDK 6 Integration Pattern

**Agent → Structured Output → TipTap Node**

```typescript
// AI SDK 6 Agent with structured output
import { agent } from 'ai';
import { z } from 'zod';

const brainBootAgent = agent({
  model: 'claude-sonnet-4',
  tools: {
    brain_boot: tool({
      // ... tool definition
    }),
  },
  output: {
    type: 'object',
    schema: z.object({
      summary: z.string(),
      sections: z.array(z.object({
        title: z.string(),
        items: z.array(z.string()),
        expandable: z.boolean(),
      })),
      boardReferences: z.array(z.object({
        id: z.string(),
        preview: z.string(),
      })),
    }),
  },
});

// Insert as TipTap node
editor.commands.insertContent({
  type: 'agentResponse',
  attrs: {
    agentId: 'brain_boot',
    data: result.output, // Structured output
  },
});
```

### 4. Three-Pane Layout

```
┌──────────────────────────────────────────────────────────────┐
│  [EVNA] Workspace                            [User] [Settings]│
├────────────┬─────────────────────────────────────────────────┤
│            │                                                  │
│  Sidebar   │  Main Editor (TipTap)                           │
│  (Pinned)  │  ┌────────────────────────────────────────┐     │
│            │  │ # November 15, 2025                    │     │
│  • Today   │  │                                        │     │
│  • Recents │  │ Morning check-in:                      │     │
│  • Search  │  │ /brain_boot show recent work on floatctl    │
│            │  │                                        │     │
│            │  │ ╔════════════════════════════════════╗ │     │
│            │  │ ║ Brain Boot: floatctl-rs           ║ │     │
│            │  │ ║ • Recent commits: PR #25          ║ │     │
│            │  │ ║ • Active work: evna optimization  ║ │     │
│            │  │ ║                      [Expand ↓]   ║ │     │
│            │  │ ╚════════════════════════════════════╝ │     │
│            │  │                                        │     │
│            │  │ Notes:                                 │     │
│            │  │ - Need to finish...                    │     │
│            │  └────────────────────────────────────────┘     │
│            │                                                  │
├────────────┼──────────────────────────────────────────────────┤
│            │  BBS Board Preview                              │
│            │  ┌────────────────────────────────────────┐     │
│            │  │ [Board: restoration]                   │     │
│            │  │                                        │     │
│            │  │ Recent activity:                       │     │
│            │  │ • Thread: Database schema changes     │     │
│            │  │ • Thread: Performance optimizations   │     │
│            │  └────────────────────────────────────────┘     │
└──────────────┴─────────────────────────────────────────────────┘
```

**Layout States**:
1. **Full Editor** - Sidebar collapsed, no board preview
2. **Editor + Board** - Sidebar collapsed, board preview active
3. **Editor + Sidebar** - Board collapsed, sidebar active
4. **Three-pane** - All visible (desktop)

### 5. Command System

**Slash Commands** (TipTap native):
```
/brain_boot    → Morning synthesis (multi-source)
/search        → Semantic search (pgvector)
/context       → Active context query
/board         → Insert board embed
/ask           → ask_evna orchestrator
```

**Implementation**:
```typescript
// Command extension
import { Extension } from '@tiptap/core';
import { Suggestion } from '@tiptap/suggestion';

export const Commands = Extension.create({
  name: 'commands',

  addOptions() {
    return {
      suggestion: {
        char: '/',
        items: ({ query }) => {
          return [
            { label: 'brain_boot', description: 'Morning synthesis' },
            { label: 'search', description: 'Semantic search' },
            { label: 'context', description: 'Active context' },
            { label: 'board', description: 'Insert board' },
            { label: 'ask', description: 'Ask evna' },
          ].filter(item =>
            item.label.toLowerCase().startsWith(query.toLowerCase())
          );
        },
        render: () => {
          // Custom command palette UI
          return CommandPaletteRenderer;
        },
      },
    };
  },
});
```

### 6. Structured Output → React Components

**Component Registry Pattern**:

```typescript
// src/components/agent-outputs/registry.tsx
import { BrainBootOutput } from './brain-boot';
import { SearchResults } from './search-results';
import { ContextTimeline } from './context-timeline';
import { BoardEmbed } from './board-embed';

export const AgentOutputRegistry = {
  brain_boot: BrainBootOutput,
  search: SearchResults,
  context: ContextTimeline,
  board_preview: BoardEmbed,
} as const;

// TipTap node renders from registry
function StructuredOutputNode({ node }: NodeViewProps) {
  const { outputType, data } = node.attrs;
  const Component = AgentOutputRegistry[outputType];

  if (!Component) return <div>Unknown output type</div>;

  return (
    <NodeViewWrapper>
      <Component data={data} />
    </NodeViewWrapper>
  );
}
```

**Example Component** (BrainBootOutput):

```typescript
// src/components/agent-outputs/brain-boot.tsx
import { useState } from 'react';
import { Card } from '@/components/ui/card';
import { Collapsible } from '@/components/ui/collapsible';

interface BrainBootData {
  summary: string;
  sections: Array<{
    title: string;
    items: string[];
    expandable: boolean;
  }>;
  boardReferences: Array<{
    id: string;
    preview: string;
  }>;
}

export function BrainBootOutput({ data }: { data: BrainBootData }) {
  const [expandedSections, setExpandedSections] = useState<Set<number>>(new Set());

  return (
    <Card className="my-4 p-4 border-l-4 border-l-blue-500">
      <div className="text-sm text-gray-500 mb-2">🧠 Brain Boot</div>
      <p className="mb-4">{data.summary}</p>

      {data.sections.map((section, idx) => (
        <Collapsible
          key={idx}
          open={expandedSections.has(idx)}
          onOpenChange={(open) => {
            const next = new Set(expandedSections);
            open ? next.add(idx) : next.delete(idx);
            setExpandedSections(next);
          }}
        >
          <CollapsibleTrigger>
            <h4 className="font-semibold">{section.title}</h4>
          </CollapsibleTrigger>
          <CollapsibleContent>
            <ul className="list-disc pl-5 mt-2">
              {section.items.map((item, i) => (
                <li key={i}>{item}</li>
              ))}
            </ul>
          </CollapsibleContent>
        </Collapsible>
      ))}

      {data.boardReferences.length > 0 && (
        <div className="mt-4">
          <div className="text-sm text-gray-500">Referenced Boards:</div>
          <div className="flex gap-2 mt-2">
            {data.boardReferences.map((board) => (
              <button
                key={board.id}
                className="px-3 py-1 bg-gray-100 rounded hover:bg-gray-200"
                onClick={() => {
                  // Show board in preview pane
                  window.dispatchEvent(new CustomEvent('show-board', {
                    detail: { boardId: board.id },
                  }));
                }}
              >
                {board.preview}
              </button>
            ))}
          </div>
        </div>
      )}
    </Card>
  );
}
```

### 7. BBS Board Integration

**Board Preview Pane**:

```typescript
// src/components/board-preview.tsx
import { useEffect, useState } from 'react';

interface Board {
  id: string;
  name: string;
  threads: Array<{
    id: string;
    title: string;
    author: string;
    timestamp: string;
    preview: string;
  }>;
}

export function BoardPreview() {
  const [activeBoard, setActiveBoard] = useState<string | null>(null);
  const [board, setBoard] = useState<Board | null>(null);

  useEffect(() => {
    // Listen for board show events from agent outputs
    const handleShowBoard = (e: CustomEvent) => {
      setActiveBoard(e.detail.boardId);
    };

    window.addEventListener('show-board', handleShowBoard as EventListener);
    return () => {
      window.removeEventListener('show-board', handleShowBoard as EventListener);
    };
  }, []);

  useEffect(() => {
    if (!activeBoard) return;

    // Fetch board data (could be from evna MCP server resource)
    fetch(`/api/boards/${activeBoard}`)
      .then(res => res.json())
      .then(setBoard);
  }, [activeBoard]);

  if (!board) {
    return (
      <div className="h-full flex items-center justify-center text-gray-400">
        No board selected
      </div>
    );
  }

  return (
    <div className="h-full overflow-y-auto p-4">
      <h2 className="text-2xl font-bold mb-4">{board.name}</h2>
      <div className="space-y-2">
        {board.threads.map(thread => (
          <Card key={thread.id} className="p-3 hover:bg-gray-50 cursor-pointer">
            <h3 className="font-semibold">{thread.title}</h3>
            <div className="text-sm text-gray-500">
              {thread.author} · {thread.timestamp}
            </div>
            <p className="text-sm mt-2">{thread.preview}</p>
          </Card>
        ))}
      </div>
    </div>
  );
}
```

### 8. AI Gateway Integration

**Configuration** (`src/lib/ai-gateway.ts`):

```typescript
import { anthropic } from '@ai-sdk/anthropic';
import { createAIGateway } from 'ai-gateway'; // Hypothetical API

export const gateway = createAIGateway({
  provider: anthropic('claude-sonnet-4'),

  // Rate limiting
  rateLimits: {
    requestsPerMinute: 50,
    tokensPerMinute: 100000,
  },

  // Caching (dedupe identical requests)
  cache: {
    enabled: true,
    ttl: 300, // 5 minutes
  },

  // Observability
  observability: {
    enabled: true,
    endpoint: process.env.OBSERVABILITY_ENDPOINT,
  },

  // Fallback models
  fallback: [
    anthropic('claude-sonnet-3-5'),
  ],
});
```

### 9. Data Flow

**User Command → Agent → TipTap Update**:

```
1. User types: /brain_boot recent work on floatctl
   ↓
2. TipTap detects command marker
   ↓
3. Insert CommandMarker node with params
   ↓
4. Trigger AI SDK 6 Agent via Server Action
   ↓
5. Agent calls evna MCP tools (brain_boot)
   ↓
6. Agent returns structured output
   ↓
7. Server Action returns to client
   ↓
8. Client inserts AgentResponse node after marker
   ↓
9. AgentResponse renders via component registry
   ↓
10. User continues writing below
```

**Implementation** (Server Action):

```typescript
// src/app/actions/run-agent.ts
'use server';

import { agent } from 'ai';
import { z } from 'zod';
import { evnaMcpClient } from '@/lib/mcp-client';

export async function runBrainBoot(query: string) {
  const brainBootAgent = agent({
    model: 'claude-sonnet-4',
    tools: {
      brain_boot: tool({
        description: 'Morning brain boot synthesis',
        parameters: z.object({
          query: z.string(),
          project: z.string().optional(),
          lookbackDays: z.number().default(7),
        }),
        execute: async (params) => {
          // Call evna MCP server
          const result = await evnaMcpClient.callTool('brain_boot', params);
          return result;
        },
      }),
    },
    output: {
      type: 'object',
      schema: z.object({
        summary: z.string(),
        sections: z.array(z.object({
          title: z.string(),
          items: z.array(z.string()),
          expandable: z.boolean(),
        })),
        boardReferences: z.array(z.object({
          id: z.string(),
          preview: z.string(),
        })),
      }),
    },
  });

  const result = await brainBootAgent.run({
    messages: [{ role: 'user', content: query }],
  });

  return result.output;
}
```

### 10. Project Structure

```
evna-blocks/
├── src/
│   ├── app/                          # Next.js 16 App Router
│   │   ├── layout.tsx                # Root layout
│   │   ├── page.tsx                  # Main workspace page
│   │   ├── actions/                  # Server Actions
│   │   │   ├── run-agent.ts          # Agent orchestration
│   │   │   └── boards.ts             # Board data fetching
│   │   └── api/                      # API routes
│   │       ├── boards/[id]/route.ts
│   │       └── mcp/route.ts          # MCP client proxy
│   │
│   ├── components/
│   │   ├── ui/                       # shadcn/ui components
│   │   ├── workspace/
│   │   │   ├── layout.tsx            # Three-pane layout
│   │   │   ├── sidebar.tsx           # Navigation sidebar
│   │   │   └── board-preview.tsx     # Board preview pane
│   │   ├── editor/
│   │   │   ├── editor.tsx            # Main TipTap editor
│   │   │   └── command-palette.tsx   # Slash command UI
│   │   └── agent-outputs/
│   │       ├── registry.tsx          # Component registry
│   │       ├── brain-boot.tsx
│   │       ├── search-results.tsx
│   │       ├── context-timeline.tsx
│   │       └── board-embed.tsx
│   │
│   ├── editor/
│   │   ├── extensions/
│   │   │   ├── index.ts              # Extension registry
│   │   │   ├── commands.ts           # Slash commands
│   │   │   ├── command-marker.ts     # Command marker node
│   │   │   ├── agent-response.ts     # Agent response container
│   │   │   └── structured-output.ts  # Dynamic component renderer
│   │   ├── nodes/
│   │   │   ├── command-marker/
│   │   │   ├── agent-response/
│   │   │   └── structured-output/
│   │   └── config.ts                 # Editor configuration
│   │
│   ├── lib/
│   │   ├── ai-gateway.ts             # AI Gateway config
│   │   ├── mcp-client.ts             # evna MCP client
│   │   ├── editor-state.ts           # Editor state management
│   │   └── utils.ts                  # Shared utilities
│   │
│   └── types/
│       ├── agent-outputs.ts          # Structured output types
│       ├── boards.ts                 # Board data types
│       └── editor.ts                 # Editor-specific types
│
├── public/                           # Static assets
├── next.config.js                    # Next.js configuration
├── tailwind.config.ts                # Tailwind configuration
├── tsconfig.json                     # TypeScript configuration
└── package.json                      # Dependencies
```

## Key Design Decisions

### 1. Why Not Agent SDK for Frontend?

**AI SDK 6 Agent vs Claude Agent SDK**:
- AI SDK 6: Designed for frontend/backend JavaScript integration
- Agent SDK: Designed for standalone agent processes (like current evna MCP)

**Decision**: Use AI SDK 6 Agents in Server Actions, call evna MCP server as a tool backend.

### 2. Continuous Note vs Chat History

**Traditional chat**: Ephemeral, conversation-scoped
**Continuous note**: Persistent, workspace-scoped

**Benefits**:
- User can organize thoughts around agent responses
- Responses become part of documentation
- Natural context preservation (scroll up to see earlier work)
- Supports non-linear workflows (edit earlier sections)

### 3. TipTap vs Other Editors

**Alternatives**: ProseMirror (low-level), Slate, Lexical, Monaco

**TipTap chosen because**:
- Built on ProseMirror (battle-tested)
- React integration
- Extension system
- Node views for custom components
- Active development

### 4. BBS Integration Strategy

**Phase 1**: Read-only preview pane
**Phase 2**: Inline board embeds (like Linear issue embeds)
**Phase 3**: Two-way sync (create threads from workspace)

## Implementation Phases

### Phase 1: Foundation (Week 1)
- [ ] Next.js 16 project setup
- [ ] TipTap editor with basic extensions
- [ ] Three-pane layout
- [ ] Command marker system
- [ ] AI SDK 6 + evna MCP integration
- [ ] Basic agent response rendering

### Phase 2: Structured Outputs (Week 2)
- [ ] Component registry
- [ ] Brain boot output component
- [ ] Search results component
- [ ] Context timeline component
- [ ] Expandable/collapsible sections

### Phase 3: BBS Integration (Week 3)
- [ ] Board preview pane
- [ ] Board data API
- [ ] Board embed nodes
- [ ] Cross-references (board ↔ workspace)

### Phase 4: Advanced Features (Week 4+)
- [ ] Collaborative editing (Yjs)
- [ ] Workspace persistence
- [ ] Export/import
- [ ] Keyboard shortcuts
- [ ] Mobile responsive

## Open Questions

1. **Workspace persistence**: Local storage? Database? File system?
2. **Multi-workspace support**: One workspace or many?
3. **Collaboration**: Single-user or multi-user from start?
4. **Board backend**: Where does BBS data come from? (evna MCP resource? Separate API?)
5. **Authentication**: Needed? Or localhost-only for now?

## Success Metrics

- **Feels like**: Linear's command interface + Notion's blocks + v0's AI integration
- **Performance**: <100ms command palette, <2s agent response
- **Reliability**: No lost work (auto-save), graceful error handling
- **Extensibility**: New agent output types in <30 min

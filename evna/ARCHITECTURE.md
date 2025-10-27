# EVNA-Next Architecture

## Overview

EVNA-Next is a multi-interface AI agent system built with the Claude Agent SDK. This document explains the architectural patterns that prevent code duplication and enable clean separation of concerns across CLI, TUI, and MCP server interfaces.

## The "Three EVNAs" Problem

### The Risk

When building an Agent SDK application with multiple interfaces (CLI, Terminal UI, MCP server), there's a natural tendency to duplicate configuration and tool definitions across each interface:

```typescript
// ❌ Anti-pattern: Duplication across interfaces

// In cli.ts:
const systemPrompt = readFileSync("evna-system-prompt.md", "utf-8");
const result = await query({
  options: { systemPrompt, model: "claude-sonnet-4-20250514", ... }
});

// In tui.ts:
const systemPrompt = readFileSync("evna-system-prompt.md", "utf-8");
const result = await query({
  options: { systemPrompt, model: "claude-sonnet-4-20250514", ... }
});

// In mcp.ts:
const brainBootTool = tool("brain_boot", "Morning brain boot...", ...);
// Duplicated in cli.ts and tui.ts
```

This leads to:
- Configuration drift (different interfaces using different models/settings)
- Tool definition duplication (changes must be manually synchronized)
- System prompt loading overhead (file read 3x, parsed 3x)
- Maintenance burden (update 3 files for every change)

### The Solution: Separation of Concerns

EVNA-Next uses a **core + interfaces** pattern:

```
src/
├── core/           # Shared business logic
│   └── config.ts   # Query options, system prompt, model config
├── interfaces/     # Thin UI adapters
│   ├── cli.ts      # 60 lines - just imports and calls
│   ├── mcp.ts      # 19 lines - factory function
│   └── tui/        # Terminal UI with shared config
├── tools/          # Tool definitions (DRY)
│   └── index.ts    # All Agent SDK tools defined once
└── index.ts        # Export-only public API
```

## Core Layer: Single Source of Truth

**File**: `src/core/config.ts`

```typescript
import { readFileSync } from "fs";
import { join } from "path";

// Load system prompt ONCE at module initialization
export const evnaSystemPrompt = readFileSync(
  join(process.cwd(), "evna-system-prompt.md"),
  "utf-8"
);

export const DEFAULT_MODEL = "claude-sonnet-4-20250514";

// Factory function ensures consistent options across all interfaces
export function createQueryOptions(mcpServer: any) {
  return {
    settingSources: ["user"] as ["user"],
    systemPrompt: {
      type: "preset" as const,
      preset: "claude_code" as const,
      append: evnaSystemPrompt,
    },
    mcpServers: {
      "evna-next": mcpServer,
    },
    model: DEFAULT_MODEL,
    permissionMode: "bypassPermissions" as const,
  };
}
```

**Benefits**:
- System prompt loaded once (not 3x)
- Model config in one place
- TypeScript const assertions ensure type safety
- Easy to update (change once, all interfaces inherit)

## Tool Definitions Layer

**File**: `src/tools/index.ts`

All Agent SDK tools defined in one place:
- `brainBootTool` - Brain boot implementation
- `semanticSearchTool` - Semantic search implementation
- `activeContextTool` - Active context stream
- `testTool` - Simple echo for testing

Database clients initialized once as singletons, shared across all tools.

## Interface Layer: Thin Adapters

### CLI Interface (`src/interfaces/cli.ts`)

60 lines - just imports shared config and calls `query()`:

```typescript
import { createQueryOptions } from "../core/config.js";
import { evnaNextMcpServer } from "./mcp.js";

export async function main() {
  const result = await query({
    prompt: generateMessages(),
    options: createQueryOptions(evnaNextMcpServer),
  });
  // Handle output...
}
```

### MCP Interface (`src/interfaces/mcp.ts`)

19 lines - factory function for MCP server:

```typescript
import { createSdkMcpServer } from "@anthropic-ai/claude-agent-sdk";
import { brainBootTool, semanticSearchTool, activeContextTool } from "../tools/index.js";

export function createEvnaMcpServer() {
  return createSdkMcpServer({
    name: "evna-next",
    version: "1.0.0",
    tools: [brainBootTool, semanticSearchTool, activeContextTool],
  });
}

export const evnaNextMcpServer = createEvnaMcpServer();
```

### TUI Interface (`src/interfaces/tui/`)

Uses same `createQueryOptions()` pattern as CLI. Changed from 19 lines of duplicated config to 2 lines importing shared config.

## System Prompt Separation Pattern

### The Token Budget Problem

MCP tool descriptions are loaded into every context window. Verbose descriptions = expensive:

**Before optimization**:
- brain_boot: 1.3k tokens
- semantic_search: 1.3k tokens
- active_context: 1.9k tokens
- **Total**: 4.5k tokens (21% of MCP tool budget)

**Problem**: Tool descriptions included:
- Workspace context (GitHub username, project repos)
- Project name normalization examples
- Tool chaining strategy matrix
- Philosophy notes ("LLMs as fuzzy compilers")
- Verbose error handling sections
- Multiple usage examples per tool

**This is EVNA-specific internal knowledge**, not operational tool documentation.

### The Solution: Internal vs External Context Boundary

**MCP Tool Descriptions** (External - for any LLM using the tools):
- Concise operational documentation
- What the tool does (one sentence)
- When to use / when NOT to use
- One clear example
- Terse error guidance
- Focus: High-signal information only

**System Prompt** (Internal - EVNA's identity):
- Workspace context (GitHub username, projects, repos, paths, meetings)
- Project name normalization rules and fuzzy matching philosophy
- Annotation system documentation (ctx::, project::, meeting::)
- Tool chaining strategy
- Proactive capture rules
- Core philosophy and response style

**File**: `evna-system-prompt.md` (99 lines)

Loaded once in `src/core/config.ts`, appended to Agent SDK system prompt via:
```typescript
systemPrompt: {
  type: "preset",
  preset: "claude_code",
  append: evnaSystemPrompt,
}
```

### Results

**After optimization**:
- brain_boot: 922 tokens (-29%)
- semantic_search: 908 tokens (-30%)
- active_context: 1.0k tokens (-47%)
- **Total**: 2.8k tokens (saved 1.7k tokens, 38% reduction)

**Key Insight**: Following MCP best practice "optimize for limited context" by moving internal knowledge to system prompt, keeping tool descriptions focused on operational essentials.

## File Path Resolution Pattern

### The Problem

When loading files at module initialization (like `evna-system-prompt.md`), path resolution can fail depending on execution context:

```typescript
// ❌ Fails in some contexts (MCP server startup, different working directories)
const __dirname = dirname(fileURLToPath(import.meta.url));
const prompt = readFileSync(join(__dirname, "..", "evna-system-prompt.md"));
```

**Issues**:
- `__dirname` resolves relative to compiled module location (`dist/core/`)
- Path traversal (`..`) breaks when directory structure changes
- Fails when MCP server started from different working directory
- Error: "ENOENT: no such file or directory"

### The Solution

Use `process.cwd()` for project root resolution:

```typescript
// ✅ Reliable - works from any execution context
const prompt = readFileSync(
  join(process.cwd(), "evna-system-prompt.md"),
  "utf-8"
);
```

**Benefits**:
- Resolves from project root regardless of compiled location
- Works when MCP server invoked by Claude Desktop
- Works in CLI mode
- Works in TUI mode
- Assumes file is at project root (convention over configuration)

## Design Principles

### 1. DRY (Don't Repeat Yourself)

**One definition, many consumers**:
- Tools defined once in `src/tools/index.ts`
- Config created once in `src/core/config.ts`
- All interfaces import and use shared definitions

### 2. Interfaces as Thin Adapters

Each interface should be minimal:
- CLI: Read input → call `query()` → output results
- MCP: Expose tools via `createSdkMcpServer()`
- TUI: Render UI → call `query()` → display results

**No business logic in interface layer**.

### 3. System Prompt for Identity, Tool Descriptions for Operations

**System Prompt** (`evna-system-prompt.md`):
- Who EVNA is
- Workspace context and grounding facts
- Internal knowledge and conventions

**Tool Descriptions** (in `src/tools/registry-zod.ts`):
- What the tool does
- When to use it
- How to call it
- What it returns

### 4. Export-Only Public API

**File**: `src/index.ts`

```typescript
// ✅ Clean exports, no business logic
export { evnaSystemPrompt, createQueryOptions } from "./core/config.js";
export { brainBootTool, semanticSearchTool, activeContextTool } from "./tools/index.js";
export { evnaNextMcpServer } from "./interfaces/mcp.js";
export { main } from "./interfaces/cli.js";
```

This creates a clean API surface for consumers while keeping implementation details internal.

## Migration Guide

If you're building a similar Agent SDK application with multiple interfaces, here's how to refactor:

### Before (Duplicated)

```
src/
├── cli.ts        # Tools + config duplicated
├── mcp.ts        # Tools + config duplicated
└── tui.ts        # Tools + config duplicated
```

### After (DRY)

```
src/
├── core/
│   └── config.ts       # Shared config
├── tools/
│   └── index.ts        # Shared tools
├── interfaces/
│   ├── cli.ts          # Thin adapter
│   ├── mcp.ts          # Thin adapter
│   └── tui/            # Thin adapter
└── index.ts            # Export-only
```

### Steps

1. **Create `src/core/config.ts`**:
   - Extract system prompt loading
   - Create `createQueryOptions()` factory function
   - Export constants (DEFAULT_MODEL, etc.)

2. **Create `src/tools/index.ts`**:
   - Move all `tool()` definitions here
   - Initialize clients as singletons
   - Export all tools

3. **Create `src/interfaces/` directory**:
   - Move CLI logic to `cli.ts`
   - Move MCP server to `mcp.ts`
   - Move TUI to `tui/`
   - Each imports from core/tools

4. **Refactor `src/index.ts`**:
   - Remove all business logic
   - Export from core/tools/interfaces
   - Clean API surface

5. **Update imports**:
   - All interfaces import `createQueryOptions()`
   - All interfaces import tools from `../tools/index.js`

## MCP Best Practices Applied

### Tool Description Optimization

**Guideline**: "Optimize for limited context - make every token count"

**Implementation**:
1. Identify internal vs external knowledge
2. Move internal knowledge to system prompt
3. Keep tool descriptions operational and concise
4. Remove redundant examples and verbose sections
5. Focus on "what/when/how" essentials

**Example Transformation**:

**Before** (1.9k tokens):
```
Description with:
- Verbose dual-mode explanation
- 4 usage examples
- Detailed error handling matrix
- Project normalization examples
- Philosophy notes
- Tool chaining strategy
- Proactive capture rule
```

**After** (1.0k tokens):
```
Description with:
- Concise purpose (1 sentence)
- When to use / not use (bullet points)
- Single clear example
- Terse return format
```

**Savings**: 47% reduction

## Benefits of This Architecture

### For Developers

- **Single point of update**: Change tools or config once, all interfaces inherit
- **Type safety**: Shared TypeScript types across interfaces
- **Easier testing**: Test core logic separately from UI adapters
- **Clear boundaries**: Core vs interface responsibilities obvious

### For EVNA

- **Token efficiency**: System prompt loaded once, used by all interfaces
- **Consistent behavior**: All interfaces use same model, tools, config
- **Maintainable**: 60% less code than duplicated approach
- **Extensible**: Adding new interface (Discord bot, Web UI) = create thin adapter

### For Users

- **Consistent experience**: Same tools/behavior whether using CLI, TUI, or MCP
- **Better performance**: Optimized tool descriptions = faster loading
- **Single configuration**: Update workspace context once in `workspace-context.json`

## Related Documentation

- [ACTIVE_CONTEXT_ARCHITECTURE.md](./ACTIVE_CONTEXT_ARCHITECTURE.md) - Active context stream design
- [ACTIVE_CONTEXT_IMPLEMENTATION.md](./ACTIVE_CONTEXT_IMPLEMENTATION.md) - Implementation details
- [TUI-IMPLEMENTATION.md](./TUI-IMPLEMENTATION.md) - Terminal UI specifics
- [CLAUDE_DESKTOP_SETUP.md](./CLAUDE_DESKTOP_SETUP.md) - MCP server setup guide

## Future Considerations

### Potential Enhancements

- **Dynamic config reloading**: Watch `workspace-context.json` for changes
- **Per-interface overrides**: Allow CLI/TUI/MCP to customize query options
- **Lazy system prompt loading**: Defer readFileSync until first use
- **Testing framework**: Unit tests for `createQueryOptions()` behavior

### Adding New Interfaces

To add a new interface (e.g., Discord bot, Web UI):

1. Create `src/interfaces/discord.ts` or `src/interfaces/web.ts`
2. Import `createQueryOptions()` from `../core/config.js`
3. Import tools from `../tools/index.js`
4. Implement UI-specific logic only
5. Export from `src/index.ts`

Total code: ~50-100 lines (vs ~300-500 with duplication)

## Philosophy

**"LLMs as Fuzzy Compilers"** - This architecture brings structure to the inherent chaos of multi-interface applications without fighting the natural flow of development. The core layer normalizes configuration, the interface layer embraces diversity of user experience.

---

**Author**: Evan (QTB)
**Version**: 1.0
**Date**: 2025-10-21
**Pattern Origin**: Preventing "Three EVNAs" fork scenario

# EVNA-Next TUI Implementation

**Status**: ✅ Complete and ready to test

## What Was Built

Interactive terminal UI (TUI) for EVNA-Next using OpenTUI, replacing JSON stdout dumps with a rich conversation interface.

### Features

✅ **Multi-line input with tab support** - No more single-line frustration
✅ **Tool call visualization** - See brain_boot, semantic_search, active_context in action
✅ **Interactive REPL** - Type prompts directly, no editing files
✅ **Token/cost tracking** - Real-time usage display in status bar
✅ **Console overlay** - Debug without disrupting UI (press `` ` ``)
✅ **Agent SDK integration** - Full query() with MCP server support

## Quick Start

```bash
# Install dependencies (already done)
bun install

# Run the TUI
bun run tui
```

## Keyboard Shortcuts

### Input Mode (default)
- **Enter** - New line
- **Ctrl+Enter** - Submit message
- **Tab** - Insert tab character (literal `\t`)
- **Arrow Keys** - Navigate cursor
- **Home/End** - Jump to line start/end
- **Backspace/Delete** - Remove characters

### Global
- **Escape** - Toggle focus (input ↔ history)
- **Ctrl+L** - Clear conversation
- **`` ` ``** - Toggle console overlay
- **Ctrl+C** - Exit application

### Console Overlay
- **Arrow keys** - Scroll when focused
- **+/-** - Resize console height
- **Ctrl+P/Ctrl+O** - Change position (top/bottom/left/right)

## Architecture

### File Structure

```
evna-next/
├── src/
│   ├── index.ts                     # Original Agent SDK setup
│   └── tools/
│       ├── brain-boot.ts            # Brain boot tool
│       ├── pgvector-search.ts       # Semantic search tool
│       ├── active-context.ts        # Active context tool
│       └── registry-zod.ts          # Tool schemas
└── tui/
    ├── tui.ts                       # Main TUI entry point ✨
    ├── types.ts                     # TypeScript types
    ├── components/
    │   ├── MultilineInput.ts        # Multi-line input with tabs
    │   ├── MessageRenderer.ts       # Agent SDK message formatter
    │   └── ConversationLoop.ts      # REPL orchestrator
    └── README.md                    # Template usage guide
```

### Integration Points

**tui.ts:18** - Imports tools from `../src/index.js`:
```typescript
import {
  brainBootTool,
  semanticSearchTool,
  activeContextTool,
  evnaNextMcpServer,
} from "../src/index.js"
```

**tui.ts:50-60** - Converts user input to async generator:
```typescript
async function* generateMessages(): AsyncGenerator<SDKUserMessage> {
  yield {
    type: "user" as const,
    session_id: "", // Filled by SDK
    message: {
      role: "user" as const,
      content: userInput,
    },
    parent_tool_use_id: null,
  }
}
```

**tui.ts:64-74** - Query Agent SDK with MCP server:
```typescript
const result = await query({
  prompt: generateMessages(),
  options: {
    settingSources: ["user"],
    mcpServers: {
      "evna-next": evnaNextMcpServer,
    },
    model: "claude-sonnet-4-20250514",
    permissionMode: "bypassPermissions", // Auto-approve tools
  },
})
```

## Usage Examples

### Example 1: Brain Boot Query

```
User input:
> what was I working on with the pharmacy project last week?

Expected behavior:
- brain_boot tool called automatically
- Searches semantic history + active context
- Displays results with timestamps
- Shows token usage in status bar
```

### Example 2: Semantic Search

```
User input:
> find conversations about GP node rendering issues

Expected behavior:
- semantic_search tool called
- Searches pgvector embeddings
- Returns ranked results with similarity scores
- Displays conversation excerpts
```

### Example 3: Active Context Capture

```
User input:
> ctx::2025-10-21 @ 04:30 PM [project::evna]
>
> TUI implementation complete! Multi-line input works perfectly.

Expected behavior:
- active_context tool called with capture parameter
- Message stored with annotations parsed
- Confirmation displayed
- Available for future queries
```

## Testing Checklist

- [ ] **Launch TUI**: `bun run tui` starts without errors
- [ ] **Multi-line input**: Enter key creates newlines, tabs work
- [ ] **Submit**: Ctrl+Enter sends message to agent
- [ ] **Tool calls**: brain_boot/semantic_search/active_context render properly
- [ ] **Token tracking**: Status bar shows input/output tokens and cost
- [ ] **Console toggle**: Backtick key shows/hides console overlay
- [ ] **Focus toggle**: Escape key switches between input and history
- [ ] **Clear conversation**: Ctrl+L clears history
- [ ] **Exit**: Ctrl+C exits cleanly

## Comparison: Before vs After

### Before (src/index.ts)
```bash
$ npm start
🧠 EVNA-Next: Agent SDK with pgvector RAG
============================================

Running brain boot with GitHub integration...

{ type: 'system', ... }  # JSON dump
{ type: 'assistant', ... }  # JSON dump
{ type: 'tool_use', ... }  # JSON dump
```

**Problems**:
- Edit index.ts to change prompt
- Restart process every time
- JSON dumps hard to read
- No multi-line input
- No interactive chat loop

### After (tui/tui.ts)
```bash
$ bun run tui
🧠 EVNA-Next TUI
================
Interactive chat loop with brain_boot, semantic_search, active_context

✅ Renderer initialized
📋 Tools: brain_boot, semantic_search, active_context
⌨️  Press ` to toggle console, Ctrl+C to exit

┌─────────────────────────────────────────┐
│ 🤖 ASSISTANT                            │
│ Let me search for that...              │
│ 🔧 Tool: brain_boot                    │
│    Input: { query: "pharmacy" }        │
│ ✅ Result: Found 5 results...          │
│ 📊 Tokens: 1234↑ 56↓                   │
├─────────────────────────────────────────┤
│ Enter your message...                   │
│ > what did we discuss about            │
│   GP node rendering last week?         │
│   [Tab works here!]                    │
│                                         │
│ (Ctrl+Enter to submit)                 │
└─────────────────────────────────────────┘
Ready | Tokens: 2500↑ 800↓ | Cost: $0.0195
```

**Improvements**:
✅ Interactive REPL - no editing files
✅ Multi-line input with tabs
✅ Formatted tool calls
✅ Token/cost tracking
✅ Console overlay for debugging
✅ Keyboard shortcuts

## Next Steps

### Immediate Testing
1. Run `bun run tui`
2. Try multi-line input with tabs
3. Test each tool (brain_boot, semantic_search, active_context)
4. Verify token tracking
5. Toggle console overlay

### Future Enhancements
- **Streaming responses**: Update TUI progressively as tokens arrive (see `references/opentui-patterns.md`)
- **Syntax highlighting**: Add Tree-sitter for code blocks in tool results
- **Conversation persistence**: Save/load conversation history
- **Keyboard macros**: Add user-configurable shortcuts
- **Custom themes**: Allow color scheme customization
- **History search**: Ctrl+R to search past conversations

### Known Limitations
- History scrolling not yet implemented (Escape toggles focus, but no scroll handlers)
- No streaming support (collects all messages before rendering)
- Token costs hardcoded for Sonnet 4.5 (needs model-specific pricing)
- No conversation save/load

## Troubleshooting

### "Module not found" errors
```bash
bun install  # Reinstall dependencies
```

### "Cannot read property 'add' of undefined"
Check that `renderer.root.add(loop)` is called before `renderer.start()`

### Tools not working
Verify `.env` file has:
```bash
SUPABASE_URL=...
SUPABASE_SERVICE_KEY=...
OPENAI_API_KEY=...
ANTHROPIC_API_KEY=...
```

### Console not toggling
Press backtick (`` ` ``) key, not single quote (`'`)

## Documentation

- **Skill reference**: `~/.claude/skills/opentui-agent-builder/SKILL.md`
- **OpenTUI patterns**: `~/.claude/skills/opentui-agent-builder/references/opentui-patterns.md`
- **Template README**: `tui/README.md`
- **OpenTUI repo**: https://github.com/sst/opentui

---

Built with the **opentui-agent-builder** skill for Claude Code. 🎨

# EVNA TUI Template

Complete boilerplate for building a terminal UI with OpenTUI and Claude Agent SDK.

## Quick Start

1. **Copy this directory** to your Agent SDK project
2. **Install dependencies:**
   ```bash
   bun install @opentui/core
   ```
3. **Customize `tui.ts`** with your Agent SDK tools
4. **Run:**
   ```bash
   bun run tui.ts
   ```

## File Structure

```
evna-tui-template/
├── tui.ts                          # Main entry point
├── types.ts                        # TypeScript type definitions
├── components/
│   ├── MultilineInput.ts          # Multi-line input with tab support
│   ├── MessageRenderer.ts         # Agent SDK message formatter
│   └── ConversationLoop.ts        # REPL orchestrator
└── README.md                       # This file
```

## Components

### MultilineInput

Custom multi-line text input component with:
- Line-based editing
- Tab character support (literal `\t`, not focus navigation)
- Ctrl+Enter to submit
- Arrow keys, Home/End navigation
- Vertical scrolling for long inputs

**Usage:**
```typescript
const input = new MultilineInput(renderer, {
  id: "input",
  width: 80,
  height: 10,
  placeholder: "Enter your message...",
})

input.on("submit", (value) => {
  console.log("User submitted:", value)
})

input.focus()
```

### MessageRenderer

Formats and displays Agent SDK messages with:
- Role-based color coding (user/assistant/system)
- Tool call visualization
- Tool result formatting (success/error states)
- Token usage display
- Thinking block rendering

**Usage:**
```typescript
const renderer = new MessageRenderer(renderer, {
  id: "messages",
  width: "100%",
})

renderer.addMessage({
  role: "assistant",
  content: [
    { type: "text", text: "Hello!" },
    { type: "tool_use", id: "123", name: "search", input: { query: "test" } }
  ],
  usage: { input_tokens: 100, output_tokens: 50 }
})
```

### ConversationLoop

Orchestrates the full REPL experience:
- Manages message history
- Handles user input
- Calls Agent SDK on submit
- Updates UI with responses
- Tracks token usage and costs
- Focus state management

**Usage:**
```typescript
const loop = new ConversationLoop(renderer, {
  onSubmit: async (userInput) => {
    return await query(userInput, tools)
  },
  formatMessage: (response) => response,
  enableConsole: true,
})

renderer.root.add(loop)
renderer.start()
```

## Keyboard Shortcuts

### Input Mode
- **Enter** - New line
- **Ctrl+Enter** - Submit message
- **Tab** - Insert tab character
- **Arrow Keys** - Navigate cursor
- **Home/End** - Jump to line start/end
- **Escape** - Toggle focus to history

### History Mode
- **Escape** - Toggle focus back to input
- **Up/Down** - Scroll (TODO: implement)

### Global
- **Ctrl+L** - Clear conversation
- **`** - Toggle console overlay
- **Ctrl+C** - Exit application

## Integration with Agent SDK

Replace the mock response in `tui.ts` with your Agent SDK setup:

```typescript
import { query, tool } from "@anthropic-ai/claude-agent-sdk"
import { z } from "zod"

// Define your tools
const myTool = tool(
  "my_tool",
  "Tool description",
  {
    param: z.string().describe("Parameter description"),
  },
  async (args) => {
    // Tool implementation
    return {
      content: [{ type: "text" as const, text: "Result" }],
    }
  }
)

const tools = [myTool]

// In ConversationLoop:
const loop = new ConversationLoop(renderer, {
  onSubmit: async (userInput: string) => {
    return await query(userInput, tools)
  },
})
```

## Customization

### Colors

Edit `MessageRenderer.ts` to change color scheme:
```typescript
const COLORS = {
  user: "#00FF00",        // Green
  assistant: "#00AAFF",   // Blue
  system: "#FFAA00",      // Orange
  // ...
}
```

### Layout

Adjust flex ratios in `ConversationLoop.ts`:
```typescript
this.history.flexGrow = 7  // 70% of space
this.input.flexGrow = 3     // 30% of space
```

### Token Costs

Update pricing in `ConversationLoop.ts` -> `calculateCost()`:
```typescript
const inputCost = (inputTokens / 1_000_000) * 3.0   // $3 per 1M tokens
const outputCost = (outputTokens / 1_000_000) * 15.0 // $15 per 1M tokens
```

## Debugging

- Press **`** to toggle the console overlay
- Use `console.log()`, `console.error()` for debugging without disrupting the TUI
- Check `renderer.console` for more console controls

## Next Steps

1. Wire up your Agent SDK tools
2. Customize colors and layout
3. Add streaming support (see `references/opentui-patterns.md`)
4. Implement history persistence
5. Add syntax highlighting for code blocks

For more patterns and examples, see the skill's `references/opentui-patterns.md`.

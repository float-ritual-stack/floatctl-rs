# EVNA Chat TUI - Full Featured Implementation

**Status**: ✅ Complete with all agentic chat UI features

## What Was Built

A fully-featured terminal chat interface for EVNA using OpenTUI, with all the expected features of a modern agentic chat UI like Claude or ChatGPT.

### Core Features

✅ **Multi-line Input Editor**
- Full text editing with cursor navigation
- Word-by-word navigation (Ctrl+Left/Right)
- Text selection with Shift+Arrow keys
- Clipboard support (Ctrl+C/X/V)
- Undo/Redo (Ctrl+Z, Ctrl+Shift+Z)
- Input history navigation (Ctrl+Up/Down)
- Auto-indent on newlines
- Line numbers

✅ **Rich Message Rendering**
- Markdown parsing (bold, italic, code, headings, lists)
- Code blocks with language labels
- Tool call visualization with inputs/outputs
- Tool result display (success/error states)
- Thinking block rendering
- Role-based color coding
- Token usage display per message

✅ **Status Bar**
- Real-time token tracking (input↑ output↓ cached💾)
- Cost calculation with model-specific pricing
- Current status indicator (Ready/Thinking/Error)
- Model name display
- Help shortcut reminder

✅ **Session Management**
- Auto-save sessions (every 60 seconds)
- Manual save (Ctrl+S or /save)
- Load previous sessions (/load or /sessions)
- New session (Ctrl+N or /new)
- Session persistence in ~/.evna/sessions/
- Automatic session pruning (keeps last 50)

✅ **Help System**
- Keyboard shortcut overlay (Ctrl+H)
- Categorized shortcuts by function
- Slash command support (/help, /clear, etc.)

✅ **Display Options**
- Toggle timestamps (Ctrl+T or /timestamps)
- Compact mode (Ctrl+M or /compact)
- Clear conversation (Ctrl+L or /clear)

## Quick Start

```bash
# From evna/ directory
bun run tui

# Or via floatctl
floatctl evna tui
```

## Keyboard Shortcuts

### Submission
| Key | Action |
|-----|--------|
| ESC | Submit message |
| Ctrl+Enter | Submit message |
| Ctrl+D | Submit message |

### Navigation
| Key | Action |
|-----|--------|
| ↑/↓ | Move cursor up/down |
| ←/→ | Move cursor left/right |
| Ctrl+←/→ | Move by word |
| Home/End | Start/end of line |
| Ctrl+Home/End | Start/end of document |
| PgUp/PgDn | Page up/down |

### Editing
| Key | Action |
|-----|--------|
| Enter | New line |
| Tab | Insert indent (2 spaces) |
| Shift+Tab | Outdent line |
| Ctrl+K | Kill to end of line |
| Ctrl+U | Kill entire line |
| Ctrl+W | Delete word backward |
| Ctrl+Z | Undo |
| Ctrl+Shift+Z | Redo |

### Selection & Clipboard
| Key | Action |
|-----|--------|
| Shift+Arrows | Extend selection |
| Ctrl+A | Select all |
| Ctrl+C | Copy selection |
| Ctrl+X | Cut selection |
| Ctrl+V | Paste |

### Input History
| Key | Action |
|-----|--------|
| Ctrl+↑ | Previous input |
| Ctrl+↓ | Next input |

### Session & Display
| Key | Action |
|-----|--------|
| Ctrl+L | Clear conversation |
| Ctrl+S | Save session |
| Ctrl+N | New session |
| Ctrl+T | Toggle timestamps |
| Ctrl+M | Toggle compact mode |
| Ctrl+H | Toggle help overlay |

### Exit
| Key | Action |
|-----|--------|
| Ctrl+C | Exit (auto-saves session) |

## Slash Commands

| Command | Aliases | Description |
|---------|---------|-------------|
| /help | /h | Show help overlay |
| /clear | /c | Clear conversation |
| /save | /s | Save current session |
| /load [id] | /l | Load session by ID |
| /sessions | /list | List recent sessions |
| /new | /n | Start new session |
| /timestamps | /ts | Toggle timestamps |
| /compact | | Toggle compact mode |
| /model [name] | | Change model display |

## Architecture

### File Structure

```
evna/src/interfaces/tui/
├── tui.ts                    # Main entry point
├── types.ts                  # Type definitions
└── components/
    ├── index.ts              # Component exports
    ├── MultilineInput.ts     # Enhanced text editor
    ├── MessageRenderer.ts    # Message display with markdown
    ├── StatusBar.ts          # Token/cost tracking
    ├── HelpOverlay.ts        # Keyboard shortcuts display
    ├── SessionManager.ts     # Persistence layer
    └── ConversationLoop.ts   # Main orchestrator
```

### Component Hierarchy

```
ConversationLoop (main orchestrator)
├── Header (title bar)
├── MessageRenderer (scrollable message history)
│   └── Message containers
│       ├── Role header
│       ├── Content blocks (text/code/tools)
│       └── Usage stats
├── MultilineInput (text editor)
├── StatusBar (stats display)
└── HelpOverlay (modal, hidden by default)
```

### Data Flow

```
User Input → MultilineInput.submit
           → ConversationLoop.handleSubmit
           → Agent SDK query()
           → Response transformer
           → MessageRenderer.addMessage
           → StatusBar.updateTokens
           → SessionManager.updateSession
```

## Session Storage

Sessions are stored as JSON in `~/.evna/sessions/`:

```json
{
  "id": "session_1735123456789_abc123",
  "name": "Chat Dec 25 10:30 AM",
  "messages": [...],
  "createdAt": 1735123456789,
  "updatedAt": 1735123556789,
  "totalTokens": {
    "input": 1500,
    "output": 800,
    "cached": 500
  }
}
```

## Model Pricing

Built-in pricing for cost estimation (as of December 2025):

| Model | Input (per 1M) | Output (per 1M) | Cache Read |
|-------|----------------|-----------------|------------|
| claude-3-5-haiku | $0.25 | $1.25 | $0.025 |
| claude-sonnet-4 | $3.00 | $15.00 | $0.30 |
| claude-opus-4 | $15.00 | $75.00 | $1.50 |

## Theming

Colors are defined in each component. Key colors:

```typescript
const COLORS = {
  // Roles
  user: "#00ff88",
  assistant: "#00aaff",
  system: "#ffaa00",
  tool: "#ff66ff",

  // UI
  text: "#e0e0e0",
  code: "#ffd700",
  border: "#404050",
  cursor: "#00ff88",
}
```

## Differences from Previous Implementation

| Feature | Before | After |
|---------|--------|-------|
| Input editing | Basic keys only | Full editor with selection, undo, clipboard |
| Message rendering | Simple text | Full markdown with code blocks |
| Token tracking | Per-session only | Per-message + cumulative |
| Session persistence | None | Auto-save + manual save/load |
| Help system | Console notes | Interactive overlay |
| History navigation | None | Ctrl+Up/Down through inputs |
| Model pricing | Hardcoded | Model-specific calculation |

## Testing Checklist

- [x] Multi-line input with tab/newlines
- [x] Word navigation (Ctrl+Left/Right)
- [x] Text selection (Shift+Arrows)
- [x] Clipboard operations (Ctrl+C/X/V)
- [x] Undo/Redo (Ctrl+Z)
- [x] Input history (Ctrl+Up/Down)
- [x] Message submission (ESC, Ctrl+Enter)
- [x] Tool call rendering
- [x] Markdown code blocks
- [x] Token/cost display
- [x] Help overlay (Ctrl+H)
- [x] Session save/load
- [x] Timestamps toggle
- [x] Compact mode toggle
- [x] Clean exit with save

## Troubleshooting

### "Module not found" errors
```bash
cd evna && bun install
```

### Sessions not saving
Check permissions on `~/.evna/sessions/`:
```bash
mkdir -p ~/.evna/sessions
chmod 755 ~/.evna/sessions
```

### Input not responding
- Make sure terminal has focus
- Check if help overlay is open (press any key to close)
- Try Ctrl+C to exit and restart

### Tools not executing
Verify `.env` has required keys:
```bash
ANTHROPIC_API_KEY=...
SUPABASE_URL=...
SUPABASE_SERVICE_KEY=...
```

## Future Enhancements

- [ ] Streaming response display (real-time tokens)
- [ ] Search through message history (Ctrl+R)
- [ ] Custom themes/color schemes
- [ ] Conversation export (markdown/JSON)
- [ ] Split view for long tool outputs
- [ ] Image attachment support
- [ ] Voice input integration

---

Built with OpenTUI (@opentui/core) for EVNA agentic assistant.

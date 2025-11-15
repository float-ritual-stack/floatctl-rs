# floatctl-tui

Terminal UI for floatctl - a vim/zellij-inspired BBS-style interface with block-based content and asynchronous agents.

## Quick Start

```bash
# Run the TUI
cargo run -p floatctl-tui

# Or install globally
cargo install --path floatctl-tui
floatctl-tui
```

## Interface Layout

```
┌─────────────────────────────────────────────────────────────┐
│ NORMAL  /recent/  [SCRATCH]                         14:23:45 │  ← Status bar
├──────────────────────┬──────────────────────────────────────┤
│                      │                                      │
│  Scratch Log         │  Board: /recent/                    │
│  (left pane)         │  (right pane)                       │
│                      │                                      │
│  Type here...        │  📝 Recent entries                   │
│                      │  🤖 Agent posts                      │
│                      │                                      │
├──────────────────────┴──────────────────────────────────────┤
│ i: insert | :: command | b: boards | Tab: switch | q: quit  │  ← Command bar
└─────────────────────────────────────────────────────────────┘
```

## Modes (Vim-inspired)

### Normal Mode (default)
- **Navigate**: `j/k` for up/down (TODO)
- **Switch panes**: `Tab`
- **Insert mode**: `i`
- **Command mode**: `:`
- **Board navigation**: `b`
- **Refresh**: `r`
- **Quit**: `q`

### Insert Mode
- **Edit scratch log**: Type freely with vim-like editing (tui-textarea)
- **Save entry**: `Ctrl-s` (TODO: implement save to BlockStore)
- **Exit**: `Esc` → returns to Normal mode

### Command Mode
- Type commands after `:` prompt
- **`:work` or `:w`** - Switch to /work/ board
- **`:tech` or `:t`** - Switch to /tech/ board
- **`:life-admin` or `:l`** - Switch to /life-admin/ board
- **`:recent` or `:r`** - Switch to /recent/ board
- **`:scratch` or `:s`** - Switch to /scratch/ board
- **`:quit` or `:q`** - Exit application
- **`Esc`** - Cancel command

### Board Navigation Mode
- **`w`** - /work/ board
- **`t`** - /tech/ board
- **`l`** - /life-admin/ board
- **`r`** - /recent/ board
- **`s`** - /scratch/ board
- **`Esc`** - Cancel

## Architecture

### Phase 1: Block Model + Storage ✅
- Block types: Text, ContextEntry, AgentPost, Component, Code, Link
- ctx:: parser with annotation extraction
- SQLite storage with JSONB and FTS5
- 16 passing unit tests

### Phase 2: TUI Interface ✅
- Paned layout (scratch left, boards right)
- Modal system (Normal/Insert/Command/BoardNav)
- Status bar (mode indicator, board, time)
- Command bar (hints, status messages)
- Scratch panel (tui-textarea integration)
- Board panel (block list rendering)

### Phase 3: Agent System (TODO)
- Async agent runtime (tokio channels)
- Background monitoring of scratch log
- Agent posting to boards
- evna, lf1m, karen agents

### Phase 4: Advanced Features (TODO)
- Save scratch entries to BlockStore
- Parse ctx:: entries on save
- Agent-driven component insertion
- Search across blocks
- TOML configuration

## Database

Stores blocks in `~/.floatctl/tui.db` (SQLite):
- All blocks stored as JSONB
- Annotations extracted for filtering
- Agent posts denormalized for board queries
- Full-text search with FTS5

## Current Limitations

- Scratch panel editing works but entries not saved to BlockStore yet
- No agent runtime implemented yet
- Board navigation works but blocks need to be manually added
- No j/k navigation in lists yet
- No search functionality yet

## Development

```bash
# Build
cargo build -p floatctl-tui

# Run tests
cargo test -p floatctl-tui

# Run with logging
RUST_LOG=debug cargo run -p floatctl-tui
```

## Next Steps

1. **Implement save functionality**: Wire up Ctrl-s to parse scratch content and save to BlockStore
2. **Add agent runtime**: Background tasks that monitor ctx:: entries
3. **Implement j/k navigation**: Scroll through board entries
4. **Add search**: `/` command for FTS5 search
5. **Custom components**: Agent-driven widget insertion

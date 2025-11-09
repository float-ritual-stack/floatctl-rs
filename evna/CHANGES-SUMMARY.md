# Recent Changes Summary

## 1. Self-Modifying System Prompt (Nov 6, 2025)

### What Changed
- System prompt now loads from `~/.evna/system-prompt.md` (with fallback to project file)
- Removed hard-coded weekly bridge injection
- Added two new tools: `update_system_prompt` and `read_system_prompt`

### Why
- Allows EVNA to experiment with self-modification
- System prompt persists across git pulls/updates
- Enables iterative refinement of identity/behavior

### Files Modified
- `src/core/config.ts` - Load from ~/.evna/ with fallback
- `src/tools/update-system-prompt.ts` - New tool for self-modification
- `src/tools/registry-zod.ts` - Tool schemas
- `src/tools/index.ts` - Tool registration
- `src/mcp-server.ts` - MCP handlers
- `setup-user-prompt.sh` - Setup script (new)

### Setup
```bash
cd evna
./setup-user-prompt.sh
```

This copies the current system prompt to `~/.evna/system-prompt.md`.

### Usage
EVNA can now:
- Read her system prompt: `read_system_prompt()`
- Update it: `update_system_prompt(content: "...", backup: true)`
- Automatic backups created with timestamps

### Important
- Changes take effect on next session (restart CLI/TUI or reload MCP)
- Backups stored in `~/.evna/system-prompt.backup.TIMESTAMP.md`
- Only update when explicitly asked by user

## 2. MCP Timeout Handling (Nov 6, 2025)

### What Changed
- Added `timeout_ms` parameter to `ask_evna` tool
- Default 25-second timeout for MCP calls
- Returns early with session_id if query takes too long

### Why
- MCP protocol is synchronous - long queries can timeout
- Prevents "connection lost" errors in Claude Desktop
- Allows graceful degradation with session resumption

### Files Modified
- `src/tools/ask-evna-agent.ts` - Timeout logic
- `src/tools/registry-zod.ts` - timeout_ms parameter
- `src/mcp-server.ts` - Default 25s timeout
- `src/tools/index.ts` - Pass through timeout_ms

### How It Works
1. MCP client calls `ask_evna` with query
2. If query takes >25 seconds, returns:
   - "🕐 Query is taking longer than expected..."
   - Session ID for resumption
   - `timed_out: true` flag
3. User calls again with `session_id` to continue/get results

### Usage
```typescript
// Default (25s timeout for MCP safety)
ask_evna({ query: "complex question" })

// Custom timeout
ask_evna({ query: "complex question", timeout_ms: 60000 })

// Resume after timeout
ask_evna({ session_id: "abc-123" })
```

### Important
- Timeout only applies when specified (CLI/TUI can omit for unlimited time)
- Session state is saved even if timed out
- Can resume with follow-up questions or retrieve results

## Testing

All changes pass typecheck:
```bash
cd evna
bun run typecheck  # ✅ No errors
```

## Next Steps

1. Run setup script to migrate system prompt to ~/.evna/
2. Test system prompt updates with EVNA
3. Monitor MCP timeout behavior with long queries
4. Consider adding progress indicators in future iterations

## 3. Claude Projects Context Injection (Nov 6, 2025)

### What Changed
- Added hook to inject recent conversation snippets from `~/.claude/projects`
- Gives EVNA "peripheral vision" into recent Desktop/Code work
- Implemented as Agent SDK hook (clean, proper pattern)

### Why
- Most ask_evna turns are 1-3 turns (context window not a concern)
- EVNA can see "have I answered this recently?"
- Detect patterns across different work streams
- Bridge between Desktop ↔ Code sessions

### Files Created
- `src/lib/claude-projects-context.ts` - Context extraction from .jsonl files
- `src/hooks/claude-projects-context.ts` - Agent SDK hook implementation

### Files Modified
- `src/tools/ask-evna-agent.ts` - Hook integration
- `src/tools/registry-zod.ts` - New parameters (include_projects_context, all_projects)
- `src/mcp-server.ts` - Pass through parameters
- `src/tools/index.ts` - Pass through parameters

### How It Works
1. **Hook triggers** on UserPromptSubmit/BeforeTurn events
2. **Reads recent .jsonl files** from ~/.claude/projects (sorted by mtime)
3. **Extracts head/tail** (default: 20 head lines, 10 tail lines)
4. **Injects into system prompt** with markdown formatting
5. **Default: evna project only** (focused context)
6. **Option: all projects** (broader scan, 5 projects, 2 files each)

### Usage
```typescript
// Default (evna project only, enabled)
ask_evna({ query: "help with Issue #656" })

// Disable context injection
ask_evna({ query: "...", include_projects_context: false })

// Include all projects (broader peripheral vision)
ask_evna({ query: "...", all_projects: true })
```

### Configuration
**Default (evna project)**:
- 1 project (-Users-evan--evna)
- 3 most recent files
- 20 head lines, 10 tail lines
- 72 hour max age

**All projects mode**:
- 5 most recently active projects
- 2 files per project
- 15 head lines, 8 tail lines
- 48 hour max age

### Benefits
- **Deduplication**: "I just answered this in Desktop 2 hours ago"
- **Cross-pollination**: See patterns across different projects
- **Continuity**: Bridge between Desktop ↔ Code sessions
- **Lightweight**: Only injects head/tail, not full conversations

### Performance
- Graceful degradation on errors
- Async file reads
- Sorted by mtime (most recent first)
- Filtered by age cutoff
- ~100-500ms overhead per turn (acceptable for 25s timeout budget)

### Future Enhancements
- Cache file stats between turns (reduce filesystem hits)
- Semantic filtering of injected content
- Project-specific injection rules
- Integration with active_context cross-client surfacing

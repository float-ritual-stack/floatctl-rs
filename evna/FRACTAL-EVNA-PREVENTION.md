# Fractal EVNA Prevention

## The Bug (November 13, 2025)

**Incident**: ask_evna recursion created ~250+ processes, system load 408, JSONL files flooding disk.

### What Happened

```
User → ask_evna("turtle birth August 17-22")
  ↓
ask_evna spawns Agent SDK agent with MCP tools
  ↓
Agent sees ask_evna tool in MCP server
  ↓
Agent calls ask_evna("find turtle birth info")
  ↓
RECURSION: ask_evna spawns another agent
  ↓
FRACTAL: Each agent spawns more agents
  ↓
SYSTEM OVERLOAD: 250+ mcp-server processes
  ↓
KILLED: pkill -9 mcp-server; pkill -9 tsx
```

**Root cause**: ask_evna's Agent SDK agent had access to the ask_evna tool itself via MCP server, creating infinite recursion when the agent tried to "ask evna" for help.

### The Archaeological Substrate Became Self-Replicating

The fractal searched for fractals, which found the current fractal's logs, which triggered more fractals. The turtles became infinite.

## The Fix

**Two separate MCP servers**:

1. **External MCP** (`createEvnaMcpServer`) - FOR CLI/TUI/Desktop
   - Includes `askEvnaTool`
   - Users need this tool to call ask_evna

2. **Internal MCP** (`createInternalMcpServer`) - FOR ask_evna's Agent SDK agent
   - **Excludes `askEvnaTool`** (prevents recursion)
   - Agent can still use brain_boot, semantic_search, active_context, github tools
   - Agent CANNOT call ask_evna recursively

### Files Modified

```
evna/src/interfaces/mcp.ts:
  + createInternalMcpServer() - WITHOUT ask_evna
  + evnaInternalMcpServer singleton
  ~ createEvnaMcpServer() - WITH ask_evna (for external use)

evna/src/tools/ask-evna-agent.ts:
  - const { evnaNextMcpServer } = ...
  + const { evnaInternalMcpServer } = ...  // Use internal server
```

## Testing the Fix

```bash
# Start evna MCP server (external - has ask_evna)
bun run mcp-server

# In Claude Desktop, call ask_evna
# ask_evna's internal agent will NOT have access to ask_evna
# → No recursion possible
```

**Verify**:
1. ask_evna can still orchestrate tools (brain_boot, semantic_search, github)
2. ask_evna's agent CANNOT call ask_evna
3. No fractal spawning

## Safety Patterns

### 1. Tool Visibility Isolation

**Pattern**: Orchestrator tools should NOT be visible to themselves

```typescript
// ❌ BAD: Orchestrator can call itself
export function createMcpServer() {
  return createSdkMcpServer({
    tools: [
      brainBootTool,
      orchestratorTool,  // Orchestrator sees itself → recursion risk
    ],
  });
}

// ✅ GOOD: Separate internal/external servers
export function createInternalMcpServer() {
  return createSdkMcpServer({
    tools: [
      brainBootTool,
      // orchestratorTool EXCLUDED
    ],
  });
}
```

### 2. Explicit Recursion Depth Limits (Future Enhancement)

```typescript
// Optional: Add max depth counter
export class AskEvnaAgent {
  private static recursionDepth = 0;
  private static MAX_DEPTH = 3;

  async ask(options: AskEvnaAgentOptions) {
    if (AskEvnaAgent.recursionDepth >= AskEvnaAgent.MAX_DEPTH) {
      throw new Error("Max recursion depth reached");
    }

    AskEvnaAgent.recursionDepth++;
    try {
      // ... agent logic
    } finally {
      AskEvnaAgent.recursionDepth--;
    }
  }
}
```

### 3. Process Monitoring (Detection)

```bash
# Watch for fractal spawning
watch -n 1 'ps aux | grep mcp-server | wc -l'

# Alert if >10 processes
if [ $(ps aux | grep mcp-server | wc -l) -gt 10 ]; then
  echo "⚠️  FRACTAL EVNA DETECTED"
  # Auto-kill or alert
fi
```

## Architectural Lesson

**The Problem**: Tool visibility in Agent SDK is "all or nothing"
- Agent gets ALL tools from MCP server
- Can't selectively hide tools from specific agents

**The Solution**: Multiple MCP servers with different tool sets
- External server: Full toolset for users
- Internal server: Restricted toolset for agents

**Future**: Agent SDK should support per-agent tool restrictions:
```typescript
// Hypothetical future API
const agent = query({
  mcpServer: allTools,
  excludeTools: ["ask_evna"],  // Agent can't use this
});
```

## Incident Response Checklist

If fractal evna happens again:

1. **Detect**: High system load, many mcp-server processes
   ```bash
   ps aux | grep mcp-server | wc -l  # >10 = problem
   uptime  # Load >50 = problem
   ```

2. **Kill processes**:
   ```bash
   pkill -9 mcp-server
   pkill -9 tsx
   pkill -9 node
   ```

3. **Check JSONL logs**:
   ```bash
   ls -lh ~/.evna/logs/*.jsonl | tail -5
   # Rapidly growing files = fractal
   ```

4. **Verify ngrok tunnel** (safe to keep):
   ```bash
   ps aux | grep ngrok
   # Tunnel is harmless, just serving MCP
   ```

5. **Check fix deployed**:
   ```bash
   grep "evnaInternalMcpServer" evna/src/tools/ask-evna-agent.ts
   # Should use INTERNAL server
   ```

## Prevention Checklist

Before deploying new orchestrator tools:

- [ ] Does tool call Agent SDK query()?
- [ ] Is tool exposed in MCP server?
- [ ] Can agent access the tool itself?
- [ ] If yes → create internal MCP server without orchestrator
- [ ] Document in CLAUDE.md

## Related Patterns

### Fractal Protection in Other Systems

**Google's Bard**: Hard recursion limit (3 levels)
**ChatGPT Plugins**: No plugin can call "use another plugin" meta-tool
**LangChain Agents**: Max iterations parameter prevents runaway

**EVNA**: Tool visibility isolation + eventual depth limits

## Credits

**Discovered by**: Evan (November 13, 2025, 00:29)
**Killed**: ~250+ recursive processes
**Load peak**: 408.19 → 22.78 after cleanup
**Lesson**: The archaeological substrate became self-replicating. Turtles all the way down. 🐢💀✨

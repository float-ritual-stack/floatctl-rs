# ask_evna Tool Implementation Plan

**Date**: 2025-10-30
**Feature**: LLM-driven orchestration layer for evna context tools
**Status**: ✅ COMPLETE - All phases done, ready for testing

## Overview

Implementing `ask_evna` - an Agent SDK-powered orchestrator that interprets natural language queries and intelligently coordinates existing evna tools (brain_boot, semantic_search, active_context).

**The Gap We're Filling**:
- **Current**: User explicitly calls tools (brain_boot, semantic_search, etc.)
- **New**: LLM-driven orchestration that interprets intent, chains tools, synthesizes results

## Implementation Status

### Phase 1: Create AskEvnaTool Class ✅ COMPLETE

**File**: `src/tools/ask-evna.ts` (CREATED)

**Architecture Decisions**:
- [x] Streaming responses? **Decision**: Start with non-streaming for simplicity
- [x] Rate limiting? **Decision**: Use Claude API defaults
- [x] Error recovery? **Decision**: Graceful degradation - catch errors in executeTools, return error as tool result
- [x] Anthropic SDK dependency? **Decision**: Added @anthropic-ai/sdk@^0.68.0 for nested orchestrator loop

**Verification Checklist**:
- [x] Class structure follows existing tool patterns (BrainBootTool, PgVectorSearchTool, ActiveContextTool)
- [x] System prompt clearly defines orchestrator role (concise, operational focus)
- [x] Tool definitions match existing tool schemas (mirrored from registry-zod.ts)
- [x] Agent loop handles tool_use stop_reason correctly (while loop continues until non-tool_use stop)
- [x] Bridge executes all three tool types (active_context, semantic_search, brain_boot)
- [x] Error handling with try-catch (in executeTools method, returns error as tool result)
- [x] TypeScript types are correct (bun run typecheck passes ✅)

**Files Modified**:
- ✅ Created: `src/tools/ask-evna.ts` (~300 lines)
- ✅ Modified: `package.json` (added @anthropic-ai/sdk dependency)

### Phase 2: Add Zod Schema ✅ COMPLETE

**File**: `src/tools/registry-zod.ts` (MODIFIED)

**Architecture Decisions**:
- [x] Optional parameters (max_turns, debug_mode)? **Decision**: No - keep simple initially (only query parameter)
- [x] Tool selection hints in schema? **Decision**: No - let agent decide based on system prompt

**Verification Checklist**:
- [x] Schema follows existing pattern (name, description, schema structure matches other tools)
- [x] Description is clear and helpful (includes Purpose, When to use, When NOT to use, Examples)
- [x] Examples cover different query types (temporal, project-specific, cross-project, blocker identification)
- [x] Zod validation is correct (single required string parameter, TypeScript passes ✅)

### Phase 3: Wire Up in Tools Index ✅ COMPLETE

**File**: `src/tools/index.ts` (MODIFIED)

**Architecture Decisions**:
- [x] Where to instantiate? **Decision**: After existing tool instances (line 56)
- [x] Which tools to expose? **Decision**: Subset (brainBoot, search, activeContext) - not r2Sync (operational, not query-oriented)

**Verification Checklist**:
- [x] Import statements correct (import { AskEvnaTool })
- [x] Instance passed correct dependencies (brainBoot, search, activeContext)
- [x] Agent SDK wrapper follows pattern (tool() wrapper matches existing tools)
- [x] Export naming consistent (askEvna instance, askEvnaTool wrapper)
- [x] TypeScript passes (bun run typecheck ✅)

### Phase 4: Register in MCP Servers ✅ COMPLETE

**Files**:
- `src/interfaces/mcp.ts` (MODIFIED - Internal MCP)
- `src/mcp-server.ts` (MODIFIED - External MCP)

**Architecture Decisions**:
- [x] Expose in both MCPs? **Decision**: Yes - internal for TUI/CLI, external for Claude Desktop

**Verification Checklist**:
- [x] Internal MCP registration correct (imported askEvnaTool, added to tools array)
- [x] External MCP registration correct (imported askEvna, added call handler case, auto-listed via toMcpTools())
- [x] Tool appears in MCP tool lists (via toMcpTools() which includes all toolSchemas)
- [x] TypeScript passes (bun run typecheck ✅)

### Phase 5: Update System Prompt (OPTIONAL)

**File**: `evna-system-prompt.md` (TO MODIFY)

**Verification Checklist**:
- [ ] Guidelines are clear
- [ ] Examples are helpful
- [ ] Doesn't contradict existing guidance

## Key Architecture Decisions Log

### Decision 1: Wrapping vs Reimplementation
**Choice**: Wrap existing tool instances
**Rationale**: Avoids duplication, maintains single source of truth, easier to maintain
**Alternative Considered**: Reimplement tool logic in ask_evna (rejected - violates DRY)

### Decision 2: System Prompt Strategy
**Choice**: Concise prompt focused on intent classification and synthesis
**Rationale**: Keep orchestrator focused, let individual tools handle details
**Alternative Considered**: Detailed instructions for each tool (rejected - too prescriptive)

### Decision 3: Tool Selection
**Choice**: Provide brain_boot, semantic_search, active_context (not r2_sync)
**Rationale**: r2_sync is operational/admin, not query-oriented
**Alternative Considered**: Include all tools (rejected - wrong abstraction level)

### Decision 4: Response Format
**Choice**: Return synthesized text response (not structured data)
**Rationale**: ask_evna is for natural language queries expecting narrative responses
**Alternative Considered**: Structured JSON with source metadata (rejected - defeats purpose)

### Decision 5: Nested Agent Loop
**Choice**: Use direct Anthropic SDK client for orchestrator loop
**Rationale**: Agent SDK's `query()` is for outer loop, nested orchestrator needs direct API access
**Implementation**: Added @anthropic-ai/sdk@^0.68.0 dependency

## Testing Strategy

### Manual Testing (TODO after all phases)
1. Test via CLI: `bun run cli` (after wiring up)
2. Test via TUI: Interactive query
3. Test via Claude Desktop: MCP tool invocation

### Test Cases
- Temporal query: "what was I working on yesterday?"
- Project-specific query: "summarize pharmacy Issue #633"
- Cross-project query: "show me all GP node work"
- Ambiguous query: "what's blocking the release?" (should prompt for project or use context)

## Success Criteria

- [x] Phase 1: AskEvnaTool class created with agent loop
- [ ] Phase 2-4: Integrated into all interfaces
- [ ] ask_evna responds to natural language queries
- [ ] Agent correctly selects appropriate tool(s)
- [ ] Multiple tools can be chained if needed
- [ ] Responses are synthesized (not raw dumps)
- [ ] Works in all interfaces (CLI, TUI, Claude Desktop)
- [ ] No duplication of existing tool logic
- [ ] Follows established code patterns

## Next Steps

1. ✅ **DONE**: Create AskEvnaTool class with orchestrator logic
2. **NEXT**: Add Zod schema to registry-zod.ts
3. Wire up in tools/index.ts
4. Register in both MCP servers
5. Manual testing across all interfaces
6. (Optional) Update evna-system-prompt.md with usage guidelines

## Implementation Complete ✅

**Summary**: All 4 core phases completed successfully. ask_evna tool is now fully integrated into evna's toolchain and available across all interfaces.

### What Was Built

**1. AskEvnaTool Class** (src/tools/ask-evna.ts)
- Nested Anthropic API loop for orchestrator agent
- System prompt focused on intent classification & synthesis
- Wraps existing tool instances (brainBoot, search, activeContext)
- Graceful error handling with tool result fallback
- ~300 lines, TypeScript clean

**2. Zod Schema** (src/tools/registry-zod.ts)
- Comprehensive tool description with examples
- Single required parameter: query (string)
- Clear guidelines on when to use ask_evna vs individual tools
- Auto-converted to MCP JSON format via toMcpTools()

**3. Tool Integration** (src/tools/index.ts)
- askEvna instance instantiated with existing tool dependencies
- askEvnaTool Agent SDK wrapper exported
- Follows established patterns

**4. MCP Registration**
- Internal MCP (src/interfaces/mcp.ts): Added to tools array for TUI/CLI
- External MCP (src/mcp-server.ts): Added call handler for Claude Desktop
- Both MCPs now expose ask_evna tool

### Files Modified

**Created (1)**:
- src/tools/ask-evna.ts (~300 lines)

**Modified (5)**:
- src/tools/registry-zod.ts (added schema)
- src/tools/index.ts (instantiation + export)
- src/interfaces/mcp.ts (internal MCP registration)
- src/mcp-server.ts (external MCP registration)
- package.json (added @anthropic-ai/sdk dependency)

**Documentation (1)**:
- docs/ask-evna-implementation-plan.md (this file)

### Commits

1. **Phase 1** (1841a70): AskEvnaTool class + implementation plan + Anthropic SDK dependency
2. **Phase 2** (b0fe55a): Zod schema definition in registry
3. **Phase 3** (a99e381): Wire up in tools/index.ts
4. **Phase 4** (969b986): Register in both MCP servers

### Verification

- ✅ TypeScript compiles cleanly (bun run typecheck)
- ✅ All phases self-verified against implementation plan
- ✅ Follows established code patterns
- ✅ No duplication of existing tool logic
- ✅ Commits between major phases

### Architecture Decisions Made

1. **Nested Agent Loop**: Use direct Anthropic SDK client (not Agent SDK's query()) for orchestrator
2. **Tool Wrapping**: Wrap existing instances, don't reimplement logic
3. **Simple Interface**: Single query parameter, no optional parameters initially
4. **Dual MCP Exposure**: Register in both internal (TUI/CLI) and external (Claude Desktop) MCPs
5. **System Prompt Strategy**: Concise, operational focus on intent classification

### Next Steps (Manual Testing)

**Ready to test via**:
1. **Claude Desktop**: MCP tool should appear in tool list
2. **CLI**: `bun run cli` (after updating example query)
3. **TUI**: `bun run tui` (interactive testing)

**Example Queries to Test**:
- "What was I working on yesterday afternoon?"
- "Summarize pharmacy Issue #633 discussion"
- "Show me all GP node work across projects"
- "What's blocking the pharmacy release?"

**Expected Behavior**:
- Orchestrator analyzes query intent
- Selects appropriate tool(s) (active_context, semantic_search, or brain_boot)
- Chains tools if needed
- Returns synthesized narrative (not raw data dump)

## Notes

- This is a personal tool for Evan - no enterprise multi-user concerns needed
- Simplicity preferred over feature completeness
- Can iterate based on usage patterns
- Nuke-and-rebuild is acceptable if needed
- Implementation completed 2025-10-30
- All phases verified: TypeScript compiles cleanly, follows existing patterns

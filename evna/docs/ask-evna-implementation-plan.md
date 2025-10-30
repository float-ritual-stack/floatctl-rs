# ask_evna Tool Implementation Plan

**Date**: 2025-10-30
**Feature**: LLM-driven orchestration layer for evna context tools
**Status**: Phase 1 Complete ✅

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

### Phase 3: Wire Up in Tools Index (PENDING)

**File**: `src/tools/index.ts` (TO MODIFY)

**Architecture Decisions**:
- [ ] Where to instantiate? **Decision**: After existing tool instances
- [ ] Which tools to expose? **Decision**: Subset (brain_boot, search, activeContext) - not r2Sync

**Verification Checklist**:
- [ ] Import statements correct
- [ ] Instance passed correct dependencies
- [ ] Agent SDK wrapper follows pattern
- [ ] Export naming consistent

### Phase 4: Register in MCP Servers (PENDING)

**Files**:
- `src/interfaces/mcp.ts` (TO MODIFY - Internal MCP)
- `src/mcp-server.ts` (TO MODIFY - External MCP)

**Architecture Decisions**:
- [ ] Expose in both MCPs? **Decision**: Yes - internal for TUI/CLI, external for Claude Desktop

**Verification Checklist**:
- [ ] Internal MCP registration correct
- [ ] External MCP registration correct
- [ ] Tool appears in MCP tool lists

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

## Notes

- This is a personal tool for Evan - no enterprise multi-user concerns needed
- Simplicity preferred over feature completeness
- Can iterate based on usage patterns
- Nuke-and-rebuild is acceptable if needed
- Phase 1 verified: TypeScript compiles cleanly, follows existing patterns

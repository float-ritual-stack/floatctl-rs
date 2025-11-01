# Changelog

All notable changes to evna-next will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

#### ask_evna Migration to Agent SDK (2025-11-01)

- **Migrated ask_evna from custom Anthropic orchestrator to Agent SDK**
  - Reduced codebase from ~1800 lines to ~110 lines (~95% reduction)
  - Deleted `src/tools/ask-evna.ts` (custom orchestrator)
  - Deleted `src/lib/search-session.ts` (early termination logic)
  - Created `src/tools/ask-evna-agent.ts` (Agent SDK wrapper)
  - Fixed circular dependency with lazy MCP server import

- **Session Management Simplified**
  - Removed custom database-backed session storage
  - Now uses Agent SDK's native session management
  - Sessions stored in-memory during MCP server lifetime
  - Pass `session_id` to resume conversations
  - Pass `fork_session: true` to branch from resume point

- **Performance Improvements**
  - Expected ~90%+ token reduction through Agent SDK context isolation
  - Agent SDK manages message history efficiently with prompt caching
  - No more repeated system prompt + growing message history on every tool call

- **New Capabilities Gained**
  - Skills support (~/.evna/skills/)
  - Slash commands (~/.evna/commands/)
  - TodoWrite integration
  - Subagent support
  - Plugin hooks (Phase 2, deferred)
  - Full Agent SDK feature set

- **Architecture Notes**
  - ask_evna is now a thin wrapper around Agent SDK's `query()` function
  - All tools accessible via MCP registration (no manual tool routing)
  - Bridge hooks and quality nudges deferred to Phase 2 (will use plugin hooks)
  - Philosophy: Use Agent SDK as-is, don't compete with frameworks

### Added

#### Bridge Management System (2025-10-31)

- **Self-Organizing Knowledge Graph** - evna can now build and maintain bridges
  - Bridges: Grep-able markdown documents in `~/float-hub/float.dispatch/bridges/`
  - Agent-driven pattern: Simple filesystem tools + prompt guidance instead of rigid logic
  - evna has full agency to create, extend, merge, and connect bridges as she sees fit

- **New Bridge Tools**:
  - `list_bridges`: List all bridge documents
  - `read_bridge`: Read bridge content by filename
  - `write_bridge`: Create/update bridge documents
  - `write_file`: General file writing capability
  - `get_current_time`: Get accurate timestamps (prevents date hallucination)
  - Bridge-aware `search_dispatch`: Can grep bridge directory for patterns

- **Bridge Document Structure**:
  - YAML frontmatter with metadata (type, created, topic, daily_root, connections)
  - Markdown body with findings, search history, connections
  - [[Wiki-links]] for connections and temporal organization
  - Slugified filenames (e.g., "grep-patterns-discovery.bridge.md")

- **System Prompt Guidance**:
  - When to create bridges (repeated searches, significant findings)
  - How to structure bridge documents (template provided)
  - Bridge operations: check, build, extend, connect, merge, search
  - Full agent authority: "These are YOUR tools. Use them when you think they're valuable."
  - **CRITICAL**: Always use `get_current_time` before creating timestamps (prevents temporal hallucination)

- **Philosophy**: Trust evna's agency. Give her tools and guidance, let her manage the knowledge graph organically.

- **Timestamp Accuracy**:
  - Added `get_current_time` tool to prevent LLM date hallucination
  - System prompt now emphasizes: "ALWAYS call get_current_time BEFORE creating/updating bridges. NEVER guess timestamps."
  - Returns both full timestamp format (YYYY-MM-DD @ HH:MM AM/PM) and date-only (YYYY-MM-DD)

#### ask_evna Orchestrator

- **ask_evna Tool Description Improvements** (2025-10-31)
  - Rewrote tool description following MCP best practices (narrowly describe functionality, include examples)
  - Made multi-turn conversation workflow prominent with numbered steps
  - Added "When NOT to use" section for clarity
  - Expanded example queries to show filesystem and bridge creation capabilities
  - Shorter opening line (agent orchestrator that coordinates multiple sources)
  - Result: Clearer understanding of when to use ask_evna vs direct tools, better multi-turn workflow visibility

- **Bridge Creation Proactivity Improvements** (2025-10-31)
  - Updated system prompt to push evna toward PROACTIVE bridge creation
  - Added specific triggers: tool usage lessons, search strategy discoveries, multi-tool orchestration insights, failed searches
  - Changed tone from passive ("when you notice") to active ("don't wait", "just do it", "create it NOW")
  - Added rule: "Will future-me benefit? → YES → CREATE BRIDGE"
  - Added guidance: "Default to bridge creation - easier to merge later than lose insights"
  - Result: evna should capture tool limitations, workarounds, and patterns without being reminded

- **Directory Tree and File Bundling Tools** (2025-10-31)
  - Added `get_directory_tree` tool for directory visualization via tree command
  - Added `bundle_files` tool for pattern-based file gathering via code2prompt
  - **Primary use case**: Temporal and pattern-based file gathering across directories
    - "Show me all notes from 2025-10-31 across directories"
    - "Bundle all *.bridge.md files"
    - "How big are all the files matching pattern X?" (token counts before viewing)
  - **Safety constraints**:
    - Path validation (must be absolute or ~/...)
    - Always enforce `--no-clipboard --output-file -` for code2prompt
    - Default depth limit of 3 for tree to prevent massive output
    - **Token limit safety wrapper** (20,000 token threshold):
      - Parses token count from code2prompt output
      - Returns summary only (token count + file tree) if over limit
      - Prevents context bombs from large file bundles
      - Tested: 82K token weekly notes blocked, returned metadata only
  - **Date pattern guidance** in system prompt:
    - Examples for single day, date ranges, entire month
    - Explains tool limitation (no OR patterns, must split ranges)
    - evna now constructs correct patterns for temporal queries
  - **Implementation**: 180 lines across executeTools() and defineTools() in ask-evna.ts
  - Result: evna can safely gather files by temporal criteria with automatic protection against oversized results

- **Grep Infrastructure Awareness** (2025-10-31)
  - Updated system prompt to reference FRONTMATTER-VOCABULARY.md and GREP-PATTERNS.md
  - Added guidelines for structural queries ("find all personas", "what types exist?")
  - Added examples for when to use grep vs semantic search
  - Result: ask_evna can now leverage grep patterns for structured queries

### Fixed

#### ask_evna Orchestrator

- **Early Termination Bug Fix** (2025-10-31)
  - Fixed bug where successful grep results were overwritten by semantic search failures
  - Added `hasAnySuccess()` helper to prevent termination when any tool succeeded
  - Updated `buildNegativeResponse()` to acknowledge partial successes
  - Result: ask_evna now correctly synthesizes mixed results instead of reporting false negatives

#### Core Features

- **Double-Write Pattern for Active Context** (2025-10-24)
  - Active context now persists to BOTH hot cache (36hr TTL) AND permanent storage
  - Migration: Added `persisted_to_long_term` and `persisted_message_id` columns to active_context_stream
  - New methods: `getOrCreateConversation()` and `createMessage()` in DatabaseClient
  - Result: Organic corpus growth from actual usage - search finds gap → discuss gap → capture fills gap
  - Test confirmed: Captures write to active_context_stream + conversations + messages with proper UUID linkage

- **Active Context Stream** ([e35540e](../../commit/e35540e))
  - JSONB-based active_context_stream table with 36-hour TTL
  - Real-time message capture for recent work
  - Project/meeting/marker extraction from annotations
  - Fuzzy project name normalization ([36e17e3](../../commit/36e17e3), [d95da84](../../commit/d95da84))

- **Dual-Source Semantic Search** ([66e772b](../../commit/66e772b))
  - Merges active_context_stream (recent) + embeddings (historical)
  - Unified interface via pgvectorTool
  - Temporal diversity in search results
  - Composite key deduplication ([2ac69c0](../../commit/2ac69c0))

- **Cohere Reranking** ([7799076](../../commit/7799076))
  - Multi-source result fusion via rerank-english-v3.0
  - Coherent temporal narrative from mixed sources
  - Cost: ~$5/month
  - 104-line implementation with inline docs

- **Semantic Filtering** ([6279271](../../commit/6279271))
  - Cosine similarity calculation for active_context messages
  - Threshold applied to BOTH active_context AND embeddings
  - True similarity scores (0.45-1.00 gradient)
  - Source field tagging ('active_context' | 'embeddings')

#### Architecture Improvements

- **Auto-Wiring Pattern** ([793c4c1](../../commit/793c4c1), [0156efa](../../commit/0156efa))
  - Zod schemas as single source of truth
  - Tool definitions generated from schemas
  - Agent SDK integration for active_context

- **MCP Best Practices** ([b9f6097](../../commit/b9f6097))
  - Enhanced tool descriptions with usage guidance
  - Proactive capture rules ([d48f33b](../../commit/d48f33b))
  - Dynamic workspace context ([d0d0755](../../commit/d0d0755))

- **Architecture Refactor** ([4785a06](../../commit/4785a06))
  - Separated core logic from interface layers
  - Clean dependency boundaries
  - Improved testability

#### Developer Experience

- **Enhanced Query Output** ([84065cc](../../commit/84065cc))
  - Rich conversation context in results
  - Detailed metadata display
  - Improved debugging information

- **Smart Truncation** ([4f1e438](../../commit/4f1e438))
  - 200 → 400 character context window
  - Sentence boundary detection
  - Preserves semantic meaning

### Fixed

#### Critical Fixes

- **UTF-8 Character Boundary Panic** ([a895403](../../commit/a895403))
  - Root cause: Naive byte slicing without char boundary detection
  - Solution: char_indices() based truncation
  - Impact: Prevents panics on multi-byte Unicode characters

- **Lossy UTF-8 Recovery** ([9d60718](../../commit/9d60718))
  - Handles token decoding errors gracefully
  - Prevents data loss on malformed UTF-8
  - Maintains embedding pipeline stability

- **Division-by-Zero Panic** ([0255450](../../commit/0255450))
  - Index optimization edge case
  - Added zero-check guard
  - Updated CLAUDE.md with fix details

#### Functional Fixes

- **Project Filter Bug** ([239014d](../../commit/239014d))
  - Extract project from ctx:: annotation blocks
  - Regex-based annotation parser
  - Enables project-scoped brain_boot queries

- **Dual-Source Balance** ([2ac69c0](../../commit/2ac69c0))
  - Fixed 30/70 allocation (active_context/embeddings)
  - Rabbit-turtle balance (recent vs historical)
  - Composite key prevents empty string ID collisions

- **Threshold Asymmetry** ([6279271](../../commit/6279271))
  - Threshold now applies to active_context (previously embeddings only)
  - Semantic filtering prevents irrelevant recent content
  - 0 results OK if nothing matches threshold

#### Code Quality

- **PR Review Findings** ([fcf5bdb](../../commit/fcf5bdb), [1fedb51](../../commit/1fedb51))
  - Security improvements
  - Error handling enhancements
  - Type safety fixes
  - Code quality improvements

- **Test Query Fix** ([d097a31](../../commit/d097a31))
  - Updated embeds_roundtrip test to match QueryRow struct
  - Ensures integration tests pass

### Changed

#### Performance Optimizations

- **Index Recreation Optimization** ([707bb19](../../commit/707bb19))
  - Remove redundant index recreation
  - Smart index management (only recreate on >20% row count change)
  - Faster query startup

- **Batch Embedding** ([6279271](../../commit/6279271))
  - Batch embed active_context messages for efficiency
  - Single API call instead of N calls
  - Reduces OpenAI API latency

#### API Improvements

- **SearchResult Interface** ([6279271](../../commit/6279271))
  - Added optional `source` field ('active_context' | 'embeddings')
  - Enables clean source detection without relying on similarity scores
  - Backwards compatible (field is optional)

- **brain_boot Implementation** ([894d99a](../../commit/894d99a))
  - Replaced db.semanticSearch with pgvectorTool.search
  - Removed redundant activeContext.queryContext call
  - Unified dual-source interface

#### Configuration

- **Normalization → Workspace Context** ([d0d0755](../../commit/d0d0755))
  - Expanded normalization.json to workspace-context.json
  - Dynamic tool descriptions based on workspace state
  - Better context awareness

### Performance

- **Query Latency**: +300ms for semantic filtering (embedding cache planned for Phase 2.3)
- **Index Optimization**: Faster startup via smart index recreation
- **Batch Embedding**: Reduced API calls for active_context filtering

### Documentation

- **TODO-SYNTHESIS.md** ([c3f4f40](../../commit/c3f4f40), [9b4b9d4](../../commit/9b4b9d4), [ae01f84](../../commit/ae01f84))
  - Phase-by-phase breakdown with time tracking
  - Session end notes for future pickup
  - 94 lines documenting all phases

- **DUAL-SOURCE-REFINEMENTS.md** (new)
  - 300+ line root cause analysis
  - 4 proposed tweaks with trade-off matrix
  - Daddy feedback integration

- **COMMIT-ANALYSIS.md** (new)
  - Full commit timeline & categorization
  - Evolution narrative (UTF-8 fix → synthesis upgrade)
  - Breaking changes analysis

- **Updated CLAUDE.md** ([0255450](../../commit/0255450))
  - Division-by-zero fix documentation
  - Architecture improvements

- **Documentation Commits** ([f50cbe6](../../commit/f50cbe6), [d313371](../../commit/d313371))
  - General documentation updates
  - Chunking improvements documentation

### Migration Guide

#### No Breaking Changes
All changes are backwards compatible:
- brain_boot API unchanged (only internal search implementation)
- Existing embeddings queries still work
- Active context stream is additive (doesn't replace anything)
- Source field is optional on SearchResult interface

#### Optional Optimization (Phase 2.3 - Planned)
To eliminate 300ms latency from semantic filtering:

1. Add embedding column to active_context_stream:
   ```sql
   ALTER TABLE active_context_stream ADD COLUMN embedding vector(1536);
   ```

2. Update db.storeActiveContext() to store embeddings at write time

3. Update pgvector-search.ts to use stored embeddings (skip API call)

Expected impact: Query latency drops from 300ms → <50ms

### Commit Summary

- **Total Commits**: 31
- **Date Range**: Oct 14 - Oct 24, 2025
- **Categories**:
  - Features: 16 commits
  - Fixes: 8 commits
  - Performance: 2 commits
  - Refactor: 1 commit
  - Documentation: 4 commits

### Development Metrics

- **Phase 0 (UTF-8 Safety)**: 6 commits over 3 hours
- **Phase 1 (Active Context Stream)**: 11 commits over 1 day
- **Phase 2 (Dual-Source Integration)**: 4 commits over 4 hours
- **Phase 3 (Brain Boot Synthesis)**: 6 commits over 94 minutes
- **Total Estimated Time**: 2.5-3.5 hours (Phase 3 only)
- **Total Actual Time**: 94 minutes (Phase 3 only)
- **Efficiency**: 5x faster than estimate

### Known Issues & Future Work

#### Phase 2.3: Embedding Cache (Planned - This Week)
- **Goal**: Eliminate 300ms latency from semantic filtering
- **Approach**: Store embeddings at write time instead of query time
- **Impact**: Query latency drops from 300ms → <50ms
- **Estimated Time**: 30-60 minutes

#### Phase 3+: Future Enhancements (Deferred)
- Dynamic allocation with temporal parsing ("today's work" vs "2020 BBS history")
- Rust CLI unification (floatctl-cli querying active_context_stream)
- Additional metadata extraction (meeting::, issue::, pr::)
- Daily note gap-filling agent (human-in-the-loop pattern)

### Related Documentation

- [PR-DESCRIPTION.md](./PR-DESCRIPTION.md) - Full PR narrative with problem → discovery → solution
- [TODO-SYNTHESIS.md](./TODO-SYNTHESIS.md) - Phase breakdown with time tracking
- [DUAL-SOURCE-REFINEMENTS.md](./DUAL-SOURCE-REFINEMENTS.md) - Root cause analysis & design decisions
- [COMMIT-ANALYSIS.md](./COMMIT-ANALYSIS.md) - Commit timeline & evolution
- [CLAUDE.md](./CLAUDE.md) - Project overview & architecture

---

## [0.1.0] - 2025-10-14

### Initial Release (Pre-Branch)
- Basic embedding pipeline
- PostgreSQL + pgvector integration
- OpenAI text-embedding-3-small embeddings
- MCP server foundation

---

**Note**: This CHANGELOG captures the journey from "fix char boundary panic" to "brain boot synthesis upgrade" - a case study in how fixing one thing reveals the next, and hermit crab methodology in action.

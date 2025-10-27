# Commit Analysis: fix/truncate-char-boundary Branch

**Branch Evolution**: Oct 14 - Oct 24, 2025
**Total Commits**: 31
**Journey**: UTF-8 fix → Active context stream → Brain boot synthesis upgrade

---

## Commit Timeline & Categorization

| SHA | Date | Category | Message |
|-----|------|----------|---------|
| c3f4f40 | 2025-10-24 01:17 | docs | Add session end notes to TODO-SYNTHESIS.md - ready for pickup tomorrow |
| 9b4b9d4 | 2025-10-24 01:12 | docs | Document Phase 2.2 completion in TODO-SYNTHESIS.md |
| 6279271 | 2025-10-24 01:11 | feat | Phase 2.2: Semantic filtering + true similarity scores |
| ae01f84 | 2025-10-24 00:55 | docs | Document Phase 2.1 dual-source balance fix |
| 2ac69c0 | 2025-10-24 00:54 | fix | Fix dual-source search: Rabbit-turtle balance + composite key dedup |
| 894d99a | 2025-10-23 23:56 | feat | Improve brain_boot semantic search: Use dual-source pgvectorTool |
| 7799076 | 2025-10-23 23:42 | feat | Add Cohere reranking for multi-source fusion in brain_boot |
| fcf5bdb | 2025-10-23 16:53 | fix | Address PR review findings: Security, error handling, and code quality |
| 4f1e438 | 2025-10-23 16:28 | feat | Improve active_context truncation: 200→400 chars with smart boundaries |
| 66e772b | 2025-10-23 15:24 | feat | Implement dual-source semantic_search: active_context + embeddings |
| 239014d | 2025-10-23 14:14 | fix | Fix project filter bug: Extract project from ctx:: annotation blocks |
| f50cbe6 | 2025-10-22 22:27 | docs | docs |
| 1fedb51 | 2025-10-21 17:33 | fix | Fix critical issues from PR review |
| 4785a06 | 2025-10-21 17:17 | refactor | Refactor EVNA architecture: Separate core logic from interface layers |
| d0d0755 | 2025-10-21 15:48 | feat | Expand normalization.json to workspace-context.json with dynamic tool descriptions |
| d48f33b | 2025-10-21 14:48 | feat | Add proactive capture rule to active_context tool description |
| b9f6097 | 2025-10-21 14:44 | feat | Enhance tool descriptions following MCP best practices |
| d95da84 | 2025-10-21 14:36 | feat | Normalize project names on capture in annotation parser |
| 36e17e3 | 2025-10-21 14:35 | feat | Add project name normalization for fuzzy matching in active context |
| 0156efa | 2025-10-21 13:48 | feat | Complete auto-wiring pattern by adding active_context to Agent SDK |
| 793c4c1 | 2025-10-21 12:49 | feat | Auto-wire tool definitions using Zod as single source of truth |
| e35540e | 2025-10-21 12:11 | feat | Implement JSONB-based active context stream persistence |
| 2d745c9 | 2025-10-21 11:37 | feat | Add active context stream and fix threshold parameter bugs in EVNA-Next |
| a2c98c8 | 2025-10-21 08:25 | feat | Add EVNA-Next MCP server and enhance floatctl query with JSON output |
| 0255450 | 2025-10-14 03:02 | fix | Fix division-by-zero panic in index optimization and update CLAUDE.md |
| d097a31 | 2025-10-14 02:56 | fix | Fix embeds_roundtrip test query to match QueryRow struct |
| 707bb19 | 2025-10-14 02:29 | perf | Optimize query performance by removing redundant index recreation |
| 84065cc | 2025-10-14 02:17 | feat | Enhance query output with conversation context and rich metadata |
| 9d60718 | 2025-10-14 01:26 | fix | Add lossy UTF-8 recovery for token decoding errors |
| a895403 | 2025-10-14 00:35 | fix | Fix UTF-8 character boundary panic in truncate function |
| d313371 | 2025-10-14 00:30 | docs | Update documentation for chunking improvements and performance optimizations |

---

## Evolution by Phase

### Phase 0: Foundation (Oct 14)
**Goal**: Fix UTF-8 character boundary panic
**Commits**: 6 (d313371 → 0255450)

- UTF-8 safe truncation with character boundary detection
- Lossy UTF-8 recovery for token decoding
- Query output enhancements with rich metadata
- Performance optimization (index recreation)
- Documentation updates

**Impact**: Resolved panics when truncating embeddings at non-character boundaries

---

### Phase 1: Active Context Stream (Oct 21)
**Goal**: Implement dual-source search infrastructure
**Commits**: 11 (a2c98c8 → f50cbe6)

**Core Features**:
- JSONB-based active_context_stream table (36-hour TTL)
- Auto-wiring pattern (Zod schemas → tool definitions)
- Project name normalization for fuzzy matching
- MCP best practices for tool descriptions
- Architecture refactor (separate core from interface layers)

**Why This Matters**: Brain boot needed BOTH recent context (active_context_stream) AND historical context (embeddings) - pure semantic search missed recent work

---

### Phase 2: Dual-Source Integration (Oct 23)
**Goal**: Merge active_context + embeddings in semantic search
**Commits**: 4 (239014d → 66e772b)

- Project filter bug fix (extract from ctx:: annotations)
- Dual-source semantic_search implementation
- Smart truncation (200→400 chars with sentence boundaries)
- Security & error handling improvements

---

### Phase 3: Brain Boot Synthesis Sprint (Oct 23-24)
**Goal**: Multi-source fusion with reranking + semantic filtering
**Commits**: 6 (7799076 → c3f4f40)

**Shipped Features**:

**Phase 3.1: Cohere Reranking** (23 min)
- Multi-source result fusion via rerank-english-v3.0
- Coherent temporal narrative from mixed sources
- Cost: ~$5/month

**Phase 3.2: Dual-Source pgvectorTool** (15 min)
- Replaced db.semanticSearch with pgvectorTool.search
- Unified interface for active + embeddings
- Removed redundant activeContext.queryContext call

**Phase 3.3: Rabbit-Turtle Balance** (22 min)
- Fixed 30/70 allocation (active_context/embeddings)
- Composite key deduplication (conversation_id + timestamp + content prefix)
- Prevents empty string ID collisions

**Phase 3.4: Semantic Filtering** (4 min)
- Applied threshold to BOTH active_context AND embeddings
- True cosine similarity scores (no fake 1.00)
- Added `source` field to SearchResult interface
- 300ms latency trade-off for relevance (embedding cache planned for Phase 2.3)

**Documentation**:
- TODO-SYNTHESIS.md (94 lines with time tracking)
- DUAL-SOURCE-REFINEMENTS.md (300+ line analysis)

---

## The Thread: How Did We Get Here?

**Oct 14**: "Fix char boundary panic" → Exposed truncate() usage in embedding pipeline
**Oct 21**: "Why not use active context stream?" → Built dual-source infrastructure
**Oct 23**: "Semantic search returns 0 results for recent work" → Integrated active_context + embeddings
**Oct 24**: "Results improving but could use tweaks" → Semantic filtering + true similarity scores

**What started as a UTF-8 fix became a complete brain boot synthesis upgrade** because:
1. Truncation panic revealed embedding pipeline architecture
2. Embedding-only search missed recent work (last 36 hours)
3. Active context stream existed but wasn't integrated
4. Integration revealed allocation + deduplication issues
5. Deduplication revealed similarity scoring issues
6. Similarity scoring revealed threshold asymmetry

**Hermit crab principle validated**: Ship working code fast, let problems reveal the path forward

---

## Breaking Changes

**None** - All changes are backwards compatible:
- brain_boot API unchanged (only internal search implementation)
- Existing embeddings queries still work
- Active context stream is additive (doesn't replace anything)
- Source field is optional on SearchResult interface

---

## Migration Notes

**Optional Optimization** (Phase 2.3 - planned):
To eliminate 300ms latency from semantic filtering:
1. Add `embedding` column to active_context_stream table
2. Store embeddings at write time (db.storeActiveContext)
3. Skip OpenAI API call when querying active_context

**No immediate action required** - Current implementation works correctly, just slower

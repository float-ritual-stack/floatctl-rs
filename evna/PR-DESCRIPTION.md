# Brain Boot Synthesis Upgrade: From UTF-8 Fix to Multi-Source Context Fusion

## Problem

**Original Issue (Oct 14)**: UTF-8 character boundary panic in truncate function
```
thread 'main' panicked at floatctl-embed/src/lib.rs:42:5:
byte index 400 is not a char boundary
```

**Root Cause**: Naive truncation at fixed byte offset without respecting UTF-8 character boundaries

**What We Thought**: "Quick fix - add char_indices() boundary detection"

**What We Discovered**: The truncation panic exposed a much larger architecture gap in how evna-next surfaces context:
- **Semantic search (embeddings)**: Great for historical deep dive, useless for "what happened this morning"
- **Active context stream**: Perfect for recent work, but isolated from semantic search
- **Brain boot**: Returned 0 results for recent queries because it only used embeddings

---

## Discovery: The Journey from Fix to Feature

### Phase 0: UTF-8 Safety (Oct 14) - 6 commits
**What shipped**: Character boundary-aware truncation + lossy UTF-8 recovery

```rust
// Before: Naive byte slicing
content[..400].to_string() // ❌ Panics on multi-byte chars

// After: UTF-8 safe truncation
pub fn truncate(s: &str, max_bytes: usize) -> String {
    s.char_indices()
        .take_while(|(idx, _)| *idx < max_bytes)
        .map(|(_, c)| c)
        .collect()
}
```

**Files changed**: `floatctl-embed/src/lib.rs`, query output enhancements, performance optimizations

**Why this mattered**: Exposed that truncation was happening in MANY places → led to Phase 1 investigation

---

### Phase 1: Active Context Stream Infrastructure (Oct 21) - 11 commits
**Problem discovered**: "Why does brain_boot return 0 results for work I did 2 hours ago?"

**Diagnosis**:
- Embeddings pipeline: Batch process, 2-24 hour lag
- Active context stream: Real-time, 36-hour TTL, **but not integrated with brain_boot**

**What shipped**:
1. **JSONB-based active_context_stream table** (e35540e)
   - 36-hour TTL for recent messages
   - Project/meeting/marker extraction
   - Fuzzy project name matching

2. **Auto-wiring pattern** (793c4c1, 0156efa)
   - Zod schemas as single source of truth
   - Tool definitions generated from schemas
   - Agent SDK integration

3. **MCP best practices** (b9f6097, d48f33b)
   - Enhanced tool descriptions
   - Proactive capture rules
   - Dynamic workspace context

4. **Architecture refactor** (4785a06)
   - Separated core logic from interface layers
   - Clean dependency boundaries

**Files changed**:
- `src/lib/db.ts` (JSONB active context)
- `src/tools/active-context.ts` (query/capture)
- `src/index.ts` (auto-wiring)
- `normalization.json` → `workspace-context.json`

---

### Phase 2: Dual-Source Integration (Oct 23) - 4 commits
**Problem**: Active context stream exists, but brain_boot doesn't use it

**What shipped**:
1. **Dual-source semantic_search** (66e772b)
   - Query BOTH active_context_stream AND embeddings
   - Merge results by timestamp
   - Deduplicate before returning

2. **Project filter bug fix** (239014d)
   - Extract project from `ctx::` annotation blocks
   - Regex-based annotation parser
   - Enables project-scoped queries

3. **Smart truncation** (4f1e438)
   - 200 → 400 chars for better context
   - Sentence boundary detection
   - Preserves semantic meaning

**Result**: brain_boot now returns results from last 36 hours ✅

**But...**: Only 3-4 results instead of 10 (next phase addresses this)

---

### Phase 3: Brain Boot Synthesis Sprint (Oct 23-24) - 6 commits
**Problem discovered during dogfooding**: "Things are improving, but could still need a few tweaks"

**Issues found**:
1. Variable result counts (3-4 instead of 10)
2. All active_context results showing `similarity: 1.00` (fake priority score)
3. Active_context allocation too aggressive (crowding out embeddings)

#### Phase 3.1: Cohere Reranking (23 min) - commit 7799076
**Why**: Mixed sources (recent + historical) need coherent fusion

**What shipped**:
- `src/lib/cohere-reranker.ts` (104 lines)
- Multi-source fusion via rerank-english-v3.0
- Coherent temporal narrative from dual sources
- Cost: ~$5/month

```typescript
const reranked = await this.reranker.fuseMultiSource(
  query,
  activeResults,
  semanticResults
);
```

#### Phase 3.2: Dual-Source pgvectorTool (15 min) - commit 894d99a
**Why**: Unified interface for active_context + embeddings

**What changed**:
```typescript
// Before: Embeddings only
const results = await this.db.semanticSearch(query, { ... });

// After: Active + embeddings
const results = await this.pgvectorTool.search(query, { ... });
```

**Impact**: Removed redundant `activeContext.queryContext()` call (already handled by pgvectorTool)

#### Phase 3.3: Rabbit-Turtle Balance (22 min) - commit 2ac69c0
**Problem**: Active_context requested 2x limit, pushed embeddings off results

**Solution**:
- **Rabbit** (active_context): 30% allocation, min 3 results
- **Turtle** (embeddings): 2x limit for deep historical search
- **Composite key dedup**: `conversation_id + timestamp + content[0:50]`

**Why composite key**: Rust CLI returns empty string IDs → all embeddings collided as "duplicates"

**Result**: Proper 🔴 3 recent + 🐢 7 historical balance

#### Phase 3.4: Semantic Filtering + True Similarity (4 min) - commit 6279271
**Problem**: Threshold applied ONLY to embeddings, active_context ALWAYS included

**Solution**:
1. **Semantic filtering for active_context**:
   ```typescript
   const queryEmbedding = await this.embeddings.embed(query);
   const activeEmbeddings = await this.embeddings.embedBatch(
     activeContextMessages.map(msg => msg.content)
   );

   const activeWithSimilarity = activeContextMessages.map((msg, idx) => ({
     msg,
     similarity: this.embeddings.cosineSimilarity(
       queryEmbedding,
       activeEmbeddings[idx]
     ),
   })).filter(({ similarity }) => similarity >= threshold);
   ```

2. **True cosine similarity scores**:
   - Added `cosineSimilarity()` helper to `embeddings.ts`
   - Replaced fake `similarity: 1.0` with actual scores
   - Honest gradient (0.45-1.00) instead of misleading perfect matches

3. **Source field tagging**:
   ```typescript
   interface SearchResult {
     message: Message;
     conversation?: Conversation;
     similarity: number;
     source?: 'active_context' | 'embeddings'; // NEW
   }
   ```

**Trade-off**: +300ms latency for query embedding
**Mitigation**: Phase 2.3 (embedding cache) will eliminate latency

**Result**: Only semantically relevant content surfaces (0 results OK if nothing matches)

---

## Documentation

**Comprehensive tracking** (hermit crab principle - document inline, ship fast):

1. **TODO-SYNTHESIS.md** (94 lines)
   - Phase breakdown with time estimates
   - Actual vs estimated timing (94 min vs 2.5-3.5 hours)
   - Session end notes for future pickup

2. **DUAL-SOURCE-REFINEMENTS.md** (300+ lines)
   - Root cause analysis (3 issues identified)
   - 4 proposed tweaks with trade-off matrix
   - Daddy feedback integration
   - Phase 2.3 planning (embedding cache)

3. **COMMIT-ANALYSIS.md** (this PR prep)
   - Commit timeline & categorization
   - Evolution narrative (how UTF-8 fix → synthesis upgrade)
   - Breaking changes analysis (none)

---

## Impact

### What Changed for Users

**brain_boot before**:
- 0 results for recent work (last 36 hours)
- All results from embeddings (2-24 hour lag)
- No temporal diversity

**brain_boot after**:
- Recent work surfaces immediately (active_context_stream)
- Historical depth from embeddings (35,810 rows)
- Coherent fusion via Cohere reranking
- Honest similarity scores (no fake 1.00)
- Semantic filtering (0 results OK if nothing matches threshold)

### Example Query Results

**Query**: "pharmacy GP node Issue 168"

**Before fix**: 0 results (work done yesterday, not yet in embeddings)

**After fix**: 10 results
- 🔴 3 recent (active_context): Yesterday's PR work, demo prep, standup notes
- 🐢 7 historical (embeddings): Original issue discussion, architecture decisions
- Similarity gradient: 0.52 → 0.89 (honest scores)
- Temporal spread: Oct 20, 21, 23, 24

---

## Breaking Changes

**None** - All changes are backwards compatible:
- ✅ brain_boot API unchanged (only internal search implementation)
- ✅ Existing embeddings queries still work
- ✅ Active context stream is additive (doesn't replace anything)
- ✅ Source field is optional on SearchResult interface

---

## Migration Notes

### Immediate (No Action Required)
Current implementation works correctly out of the box:
- Active context stream auto-captures on message store
- Dual-source search automatically enabled
- Semantic filtering applies threshold to both sources

### Optional Optimization (Phase 2.3 - Planned)
To eliminate 300ms latency from semantic filtering:

1. Add `embedding` column to active_context_stream table:
   ```sql
   ALTER TABLE active_context_stream
   ADD COLUMN embedding vector(1536);
   ```

2. Update `db.storeActiveContext()` to embed at write time:
   ```typescript
   const embedding = await this.embeddings.embed(content);
   // Store embedding alongside message
   ```

3. Update `pgvector-search.ts` to use stored embeddings:
   ```typescript
   // Skip OpenAI API call if embeddings already stored
   const activeEmbeddings = activeContextMessages.map(msg => msg.embedding);
   ```

**Expected impact**: Query latency drops from 300ms → <50ms

---

## Testing

### Dogfooding Results (Validation via evna-next usage)

**Queries tested**:
- ✅ "pharmacy GP node Issue 168" → 10 results (3 recent, 7 historical)
- ✅ "nuke-driven methodology" → 8 results (semantic filtering working)
- ✅ "echoRefactor PTOC transformation" → 0 active, 6 embeddings (recent content correctly filtered)
- ✅ "float.dispatch redux patterns" → 5 results (historical only, no recent mentions)

**Metrics**:
- Active_context coverage: Last 36 hours ✅
- Embeddings depth: 35,810 rows ✅
- Similarity scores: 0.45-1.00 gradient ✅
- Deduplication: No collisions ✅
- Semantic filtering: Irrelevant content excluded ✅

---

## What's Next (Phase 2.3 - This Week)

**Goal**: Eliminate 300ms latency from semantic filtering

**Checklist**:
- [ ] Add `embedding` column to active_context_stream table (vector type)
- [ ] Update `db.storeActiveContext()` to embed + store embedding
- [ ] Update `pgvector-search.ts` to use stored embeddings (skip API call)
- [ ] Test: Latency drops from 300ms to <50ms

**Estimated time**: 30-60 minutes

---

## Related Documentation

- [TODO-SYNTHESIS.md](./TODO-SYNTHESIS.md) - Phase breakdown with time tracking
- [DUAL-SOURCE-REFINEMENTS.md](./DUAL-SOURCE-REFINEMENTS.md) - Root cause analysis & design decisions
- [COMMIT-ANALYSIS.md](./COMMIT-ANALYSIS.md) - Full commit timeline & evolution narrative
- [CLAUDE.md](./CLAUDE.md) - Project overview & architecture

---

## Acknowledgments

**Hermit crab methodology validated**:
- Ship working code fast with inline docs
- Let problems reveal the path forward
- Document the journey, not just the destination
- 94 minutes actual vs 2.5-3.5 hours estimated

**Daddy feedback incorporated**:
- "300ms for relevance = right call - consciousness archaeology, not autocomplete"
- "0 results OK if nothing matches threshold"
- "Embedding cache (Phase 2.3) is the real win"

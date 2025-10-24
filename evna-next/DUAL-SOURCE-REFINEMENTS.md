# Dual-Source Search Refinement Analysis
**Created**: 2025-10-24 @ 1:05 AM
**Context**: Phase 2.1 shipped rabbit-turtle balance, but test results show "could still need a few tweaks"

---

## Test Results Analysis

### What Shipped (Phase 2.1 @ 12:52 AM)
- ✅ Rabbit-turtle balance: 30% active_context (min 3), 70% embeddings (2x limit)
- ✅ Composite key dedup: Fixed empty string ID collision
- ✅ Result: 🔴 3 recent + 🐢 7 historical = 10 total

### Observations from Dogfooding

**Before fix**:
- ALL results similarity 1.00 (active_context only)
- ALL from 10/24 @ 12:XX AM (meta-testing loop)
- No temporal diversity
- Embeddings invisible despite 35,810 rows

**After fix**:
- Proper similarity gradient (0.49 → 1.00)
- Temporal spread: Oct 4, 8, 19, 20, 24
- Historical embeddings NOW surfacing ✅

**But...**:
- semantic_search #1: 3 results (expected 10-12)
- semantic_search #2: 4 results (expected 12)
- semantic_search #3: 10 results ✅ (full)

---

## Issue #1: Variable Result Counts

### Root Cause
Threshold applied ONLY to embeddings (via Rust CLI), NOT to active_context.

**Current flow**:
```typescript
// src/tools/pgvector-search.ts:38-50
const activeContextMessages = await this.db.queryActiveContext({
  limit: activeLimit,  // No threshold parameter!
  project,
  since: lookbackDate,
});

const embeddingResults = await this.db.semanticSearch(query, {
  limit: limit * 2,
  project,
  since: since || lookbackDate.toISOString(),
  threshold,  // Threshold ONLY applied here (to embeddings)
});
```

**Result**:
- Active_context: ALWAYS returns 3 messages (regardless of query relevance)
- Embeddings: Returns 0-20 messages (depending on threshold + query match)
- Final: `3 active + (0-7 embeddings above threshold)` = 3-10 results

**Example**:
- Query: "bbs bulletin board systems FidoNet" (threshold 0.45)
- Active_context: 3 messages from 10/24 @ 12:XX AM (evna meta-testing)
- Embeddings: 1 message from 10/19 (similarity 0.49, barely above threshold)
- Total: 4 results (3 irrelevant recent + 1 relevant historical)

---

## Issue #2: Misleading Similarity Scores

### Root Cause
Active_context assigned `similarity: 1.0` regardless of semantic match.

**Current code**:
```typescript
// src/tools/pgvector-search.ts:74
similarity: 1.0, // Priority score for recent context
```

**Problems**:
1. **User confusion**: Output shows `(similarity: 1.00)` for ALL active_context results
   - Implies "perfect semantic match"
   - Actually means "recent, priority boosted"

2. **Cohere reranking confusion**: When brain_boot uses Cohere reranking:
   ```typescript
   // src/tools/brain-boot.ts:118-143
   const rankedResults = await this.reranker.fuseMultiSource(query, {
     semanticResults: semanticResults.map((r) => ({
       content: r.message.content,
       metadata: {
         similarity: r.similarity,  // 1.00 for active, 0.49-0.59 for embeddings
         source: r.similarity === 1.0 ? 'active_context' : 'semantic_search',
       },
     })),
   });
   ```
   - Cohere sees `similarity: 1.00` and assumes "perfect match"
   - May rank irrelevant recent content above relevant historical content

---

## Issue #3: Rigid 30% Active Allocation

### Root Cause
Fixed 30% allocation doesn't adapt to query relevance.

**Current code**:
```typescript
// src/tools/pgvector-search.ts:37
const activeLimit = Math.max(Math.floor(limit * 0.3), 3); // Always 30% or min 3
```

**Problem scenarios**:
1. **Query matches recent work well**:
   - Example: "pharmacy GP node Issue 168"
   - Recent content IS relevant
   - 30% allocation: Appropriate ✅

2. **Query about historical topics**:
   - Example: "bbs FidoNet consciousness archaeology"
   - Recent content NOT relevant (meta-testing from tonight)
   - 30% allocation: Wastes 3 slots on irrelevant content ❌
   - Better: 0-1 recent + 9-10 historical

---

## Proposed Tweaks

### Tweak #1: Semantic Filtering for Active_Context ⭐ RECOMMENDED

**What**: Calculate semantic similarity for active_context results, filter below threshold.

**How**:
```typescript
// Option A: Embed query + active_context, compute cosine similarity
const queryEmbedding = await this.embeddings.embed(query);
const activeWithSimilarity = await Promise.all(
  activeContextMessages.map(async (msg) => {
    const msgEmbedding = await this.embeddings.embed(msg.content);
    const similarity = cosineSimilarity(queryEmbedding, msgEmbedding);
    return { msg, similarity };
  })
);
const filtered = activeWithSimilarity.filter(({ similarity }) => similarity >= threshold);

// Option B: Use Rust CLI for active_context too (query against active_context_stream table)
// Requires: floatctl-cli support for querying active_context_stream with embeddings
```

**Pros**:
- Only surfaces recent content that's ACTUALLY relevant
- Prevents irrelevant recent messages from crowding results
- Unified threshold across both sources

**Cons**:
- **Latency**: Embedding active_context adds 100-300ms (3 messages × 30-100ms each)
- **Complexity**: Need OpenAI API calls for active_context (not just embeddings)
- **Cost**: 3 extra embeddings per query (~$0.0001 per query)

**Recommendation**: Ship this if latency acceptable (~300ms), defer if speed critical.

---

### Tweak #2: Dynamic Active Allocation Based on Query Match

**What**: Adjust active_context percentage based on how well query matches recent content.

**How**:
```typescript
// Calculate query match to recent content
const recentMatchScore = activeWithSimilarity.reduce((sum, { similarity }) => sum + similarity, 0) / activeWithSimilarity.length;

// Adjust allocation dynamically
let activeLimit;
if (recentMatchScore > 0.7) {
  activeLimit = Math.floor(limit * 0.3); // 30% (high match → keep recent)
} else if (recentMatchScore > 0.5) {
  activeLimit = Math.floor(limit * 0.2); // 20% (medium match → reduce recent)
} else {
  activeLimit = Math.floor(limit * 0.1); // 10% (low match → mostly historical)
}
activeLimit = Math.max(activeLimit, 1); // Min 1, not 3
```

**Pros**:
- Adapts to query relevance
- Historical queries get more room for historical results
- Recent queries still prioritize recent content

**Cons**:
- **Complexity**: Requires Tweak #1 (semantic filtering) first
- **Tuning needed**: Thresholds (0.7, 0.5) need validation
- **Less predictable**: Users may expect consistent result structure

**Recommendation**: Ship AFTER Tweak #1, validate with real usage.

---

### Tweak #3: True Semantic Similarity for Active_Context Display

**What**: Replace `similarity: 1.0` with actual cosine similarity for active_context.

**How**:
```typescript
// Instead of:
similarity: 1.0, // Priority score for recent context

// Use:
similarity: actualCosineSimilarity, // From Tweak #1 calculation
```

**Pros**:
- Honest similarity scores (no more "fake 1.00")
- Better Cohere reranking input (accurate similarity signals)
- Users understand relevance vs recency

**Cons**:
- **Breaking change**: brain_boot relies on `similarity === 1.0` to detect active_context
  ```typescript
  // src/tools/brain-boot.ts:128
  source: r.similarity === 1.0 ? 'active_context' : 'semantic_search',
  ```
- **Output format change**: Users used to seeing (similarity: 1.00) for recent

**Recommendation**: Ship WITH Tweak #1 (requires semantic similarity anyway), update brain_boot to use different marker.

---

### Tweak #4: Post-Merge Threshold Filtering (Alternative to Tweak #1)

**What**: Instead of filtering active_context pre-merge, filter ALL results post-merge.

**How**:
```typescript
// After merge, before dedup
const allResults = [...activeResults, ...embeddingResults];

// Apply threshold to ALL results (not just embeddings)
const thresholdFiltered = allResults.filter((result) => result.similarity >= threshold);

// Then deduplicate + slice
const deduplicated = deduplicateByCompositeKey(thresholdFiltered);
return deduplicated.slice(0, limit);
```

**Pros**:
- Simpler than Tweak #1 (no extra embedding calls)
- Unified threshold across both sources
- Prevents irrelevant active_context from appearing

**Cons**:
- **Requires Tweak #3 first** (need true similarity for active_context)
- **May return < limit results** (if ALL results below threshold)
- **Less control** than pre-merge filtering

**Recommendation**: Good alternative IF Tweak #1 latency unacceptable.

---

## Trade-off Matrix

| Tweak | Accuracy | Latency | Complexity | Breaking |
|-------|----------|---------|------------|----------|
| #1: Semantic filter active | ⭐⭐⭐ | ❌ +300ms | ⭐⭐ | No |
| #2: Dynamic allocation | ⭐⭐ | ❌ +300ms* | ⭐⭐⭐ | No |
| #3: True similarity scores | ⭐⭐ | ✅ Free** | ⭐ | Yes |
| #4: Post-merge threshold | ⭐⭐ | ✅ Free** | ⭐ | No*** |

\* Requires Tweak #1
\*\* If paired with Tweak #1, otherwise needs embedding calls
\*\*\* Requires Tweak #3 (which IS breaking)

---

## Recommended Ship Order

### ✅ SHIPPED: Phase 2.2 (2025-10-24 @ 1:10 AM)

**Implemented**:
1. **Tweak #1**: Semantic filtering for active_context ✅
   - Added `cosineSimilarity()` helper to embeddings.ts
   - Embed query + batch embed active_context messages
   - Filter by threshold (same as embeddings)
   - Result: Only semantically relevant recent content surfaces

2. **Tweak #3**: True similarity scores ✅
   - Replaced `similarity: 1.0` with actual cosine similarity
   - Added `source` field to SearchResult interface ('active_context' | 'embeddings')
   - Updated brain_boot to use `source` instead of `similarity === 1.0`
   - Result: Honest similarity display, better Cohere input

**Validation Results**:
- ✅ Semantic filtering working: Active_context filtered out when irrelevant
  - Test: "echoRefactor PTOC" → 0 active_context, 6 embeddings (0.45-0.54)
  - Meta-testing content from tonight correctly filtered (not relevant to historical queries)

- ✅ True similarity scores: No more fake 1.00
  - Test: "everything is redux" → embedding result shows 1.00 (actual cosine similarity)
  - Similarity range reflects real semantic match (0.45-1.00 gradient)

- ✅ Source tagging working: brain_boot can distinguish sources
  - Active_context results: `source: 'active_context'`
  - Embedding results: `source: 'embeddings'`

**Trade-offs Accepted**:
- Latency: +100-300ms (batch embedding 3 active_context messages)
- Breaking change: brain_boot source detection (updated to use `source` field)

**Files Modified**:
- `src/lib/embeddings.ts:43-63` (added cosineSimilarity helper)
- `src/tools/pgvector-search.ts:53-90` (semantic filtering + source tagging)
- `src/lib/db.ts:65-70, 162` (added source field to SearchResult, mark embeddings)
- `src/tools/brain-boot.ts:128` (use source field instead of similarity === 1.0)

**Commits**: See git log for detailed commit message

**Time**: 4 minutes (1:06 AM - 1:10 AM)

### Ship Later (Phase 3+)
3. **Tweak #2**: Dynamic allocation
   - Wait for real usage data on Tweak #1
   - Tune thresholds (0.7, 0.5) based on observed patterns
   - More complex, needs validation first

### Alternative Path (If Latency Critical)
If 300ms latency unacceptable:
- Ship **Tweak #4** (post-merge threshold) instead of Tweak #1
- Still need **Tweak #3** (true similarity) as prerequisite
- Trade-off: Slightly less accurate, but no extra API calls

---

## Implementation Notes

### Files to modify
1. `src/tools/pgvector-search.ts`:
   - Add semantic filtering for active_context (Tweak #1)
   - Replace `similarity: 1.0` with actual scores (Tweak #3)

2. `src/tools/brain-boot.ts`:
   - Update source detection (don't use `similarity === 1.0`)
   - Use metadata tag or new `source` field

3. `src/lib/embeddings.ts`:
   - Ensure `embed()` method exists and works for single messages
   - Add `cosineSimilarity()` helper if not present

### Testing checklist
- [ ] Query: "everything is redux" → expect historical "everything is redux" philosophy
- [ ] Query: "pharmacy GP node Issue 168" → expect recent pharmacy work (30% allocation justified)
- [ ] Query: "bbs FidoNet" → expect BBS archaeology, minimal recent meta-content
- [ ] Query: "echoRefactor PTOC" → expect echoRefactor usage examples (temporal spread)
- [ ] All queries return 8-10 results (not 3-4)
- [ ] Similarity scores reflect relevance (0.45-1.00 gradient)
- [ ] brain_boot still identifies active_context vs embeddings correctly

---

## Success Criteria

**Before (current state)**:
- Some queries return 3-4 results (not 10)
- Active_context ALWAYS 3 messages (regardless of relevance)
- Similarity 1.00 misleading ("recent" not "relevant")

**After (with Tweaks #1 + #3)**:
- All queries return 8-10 results consistently
- Active_context only appears when semantically relevant to query
- Similarity scores honest (0.45-1.00 reflects actual semantic match)
- Latency: +300ms (acceptable trade-off for accuracy)

---

## Open Questions

1. **Embedding cache for active_context**: Can we cache embeddings for active_context messages to reduce latency?
   - If message written to active_context_stream, also store embedding
   - Trade-off: Storage cost vs latency savings

2. **Rust CLI support for active_context**: Should floatctl-cli query active_context_stream with embeddings?
   - Would unify semantic search across both sources
   - Requires: Embeddings column in active_context_stream table

3. **Cohere reranking behavior**: How does Cohere handle `similarity: 1.00` inputs?
   - Does it trust the score or recalculate?
   - Test: Compare Cohere output with fake 1.00 vs real scores

4. **User expectations**: Do users expect 3 recent messages ALWAYS, or only when relevant?
   - Survey needed: Predictability vs relevance preference
   - May inform dynamic allocation thresholds (Tweak #2)

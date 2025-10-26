# Context Bomb Mitigation Plan

**Problem Identified:** 2025-10-24

Naive bulk imports storing entire messages (up to 99K tokens) as single records. When semantic search returns these, MCP response exceeds 25K token limit causing tool failures.

## Root Cause

**Import Pipeline:**
- floatctl-rs embeds full messages without pre-chunking
- Messages >10K tokens stored as single records
- Semantic search returns full content
- MCP serialization → BOOM if total response >25K tokens

**Largest Bombs Identified:**
- 99,231 tokens: ChatGPT JSON metadata export
- 70,889 tokens: Obsidian vault summary
- 66,180 tokens: Hellraiser AST document
- 51,052 tokens: Meta review ritual stack
- 40,836 tokens: Estate of Claude Fucks deep dive
- 37,488 tokens: Temporal awareness RFC
- 34,496 tokens: ChatKeeper conversation export

## Two-Layer Solution

### Layer 1: Import Time (floatctl-rs)

**Current State:**
- Chunking exists (`floatctl-embed/src/lib.rs:42-115`)
- Splits messages >6000 tokens with 200-token overlap
- Stores chunks: `(message_id, chunk_index)` composite key
- **Gap**: Doesn't prevent initial ingestion of 99K token messages

**Proposed Enhancement:**
```rust
// Before embedding, detect and chunk ALL messages >10K tokens
// Not just those that would exceed embedding API limits
fn should_chunk_for_retrieval(content: &str) -> bool {
    let token_count = estimate_tokens(content);
    token_count > 10_000 // Reasonable retrieval size
}

// Then apply existing chunking logic (6000 tokens + 200 overlap)
```

**Trade-off:** More chunks = more storage, but better retrieval performance

**Implementation Location:** `floatctl-embed/src/lib.rs` before embedding pipeline

---

### Layer 2: Retrieval Time (evna-next)

**Current Flow:**
```
semanticSearch()
  → Rust CLI returns full content
  → pgvector-search.ts returns SearchResult[]
  → MCP serializes to JSON
  → FAILS if >25K tokens
```

**Proposed Truncation Points:**

#### Option A: db.ts Transformation
**Location:** `src/lib/db.ts:142-163`
```typescript
return rows.map((row) => ({
  message: {
    content: smartTruncate(row.content, 2000), // Truncate at retrieval
    // ... other fields
  },
  // Store original length as metadata
  _originalLength: row.content.length,
}));
```

#### Option B: pgvector-search.ts Pre-Dedup
**Location:** `src/tools/pgvector-search.ts:92-106`
```typescript
const allResults = [...activeResults, ...embeddingResults];

// BEFORE deduplication, truncate content bombs
const truncated = allResults.map(result => {
  if (result.message.content.length > 8000) { // ~2K tokens
    return {
      ...result,
      message: {
        ...result.message,
        content: smartTruncate(result.message.content, 2000),
      },
      _truncated: true,
    };
  }
  return result;
});
```

**Why pre-dedup?** Don't truncate before semantic filtering - need full content for embedding comparison.

#### Option C: MCP Response Guard (Fastest)
**Location:** `src/tools/index.ts` or MCP server wrapper
```typescript
// Add middleware to catch oversized responses
function guardResponseSize<T>(result: T, maxTokens = 20000): T {
  const estimated = JSON.stringify(result).length / 4;
  if (estimated > maxTokens) {
    // Apply emergency truncation to result content
    return truncateSearchResults(result, maxTokens);
  }
  return result;
}
```

---

## Recommended Implementation Order

### 1. Short-term (Immediate Fix)
**Option C - MCP Response Guard**
- Catch-all safety net
- No changes to core search logic
- Degrades gracefully when limits hit
- **Effort:** 30 minutes

### 2. Medium-term (Proper Fix)
**Option B - Truncate in pgvector-search.ts**
- Happens after semantic filtering (don't truncate before embedding comparison)
- Consistent truncation across active_context + embeddings
- Reuse existing `smartTruncate()` from brain-boot.ts
- **Effort:** 1-2 hours

### 3. Long-term (Root Cause Fix)
**Layer 1 - Chunk at Import Time**
- Fix root cause (giant messages shouldn't exist as single records)
- Requires re-importing historical data with new chunking threshold
- Better search relevance (chunks surface specific sections)
- **Effort:** 4-6 hours + re-import time

---

## smartTruncate() Implementation

Already exists in `src/tools/brain-boot.ts:31-57`:

```typescript
function smartTruncate(text: string, maxLength: number): string {
  if (text.length <= maxLength) return text;

  // Search backwards from maxLength + 50 to find last sentence ending
  const searchWindow = text.substring(0, maxLength + 50);
  const lastSentence = searchWindow.lastIndexOf('.');

  if (lastSentence > maxLength * 0.7) {
    return text.substring(0, lastSentence + 1);
  }

  // Fallback: word boundary
  const lastSpace = text.lastIndexOf(' ', maxLength);
  if (lastSpace > 0) {
    return text.substring(0, lastSpace) + '...';
  }

  // Last resort: hard truncate
  return text.substring(0, maxLength) + '...';
}
```

**Recommendation:** Extract to `src/lib/truncate.ts` for reuse across:
- brain-boot.ts
- pgvector-search.ts
- active-context-stream.ts (already has copy)
- Future tools

---

## Validation Impact

**Validation Query Results (2025-10-24):**

**Successful Queries (no bombs):**
- Karen origin: 0.57 similarity ✅
- Sysop infrastructure: 0.62 similarity ✅
- Evna Montreal: 0.51 similarity ✅

**Failed Queries (hit bombs):**
- "when to activate break before hyperfocus exhaustion 40-year evolution"
- "survival mechanism BBS refuge professional boundary practice"

Both queries too broad → matched historical long-form ADHD/neurodivergent content analysis documents.

**Learning:** Specific anchor terms (Stereo nightclub, infrastructure monk, Montreal techno baptism) avoid bombs. Broad semantic concepts hit comprehensive analysis documents.

---

## Files to Modify

### Short-term (Option C)
- `src/tools/index.ts` - Add MCP response size guard wrapper

### Medium-term (Option B)
- `src/lib/truncate.ts` - Extract smartTruncate() as shared utility
- `src/tools/pgvector-search.ts` - Apply truncation before deduplication
- `src/tools/brain-boot.ts` - Import from shared truncate.ts
- `src/lib/active-context-stream.ts` - Import from shared truncate.ts

### Long-term (Layer 1)
- `floatctl-embed/src/lib.rs` - Add pre-chunking for messages >10K tokens
- Re-run embedding pipeline on historical data
- Update CLAUDE.md with new chunking strategy

---

## Success Metrics

**After short-term fix:**
- [ ] No MCP tool failures due to response size
- [ ] Graceful degradation when hitting limits
- [ ] Error logging shows which queries triggered truncation

**After medium-term fix:**
- [ ] All search results <2K tokens per message
- [ ] Validation queries complete without bombs
- [ ] Response times stable (<500ms for typical queries)

**After long-term fix:**
- [ ] No messages >10K tokens in database
- [ ] Improved search relevance (chunks surface specific sections)
- [ ] Historical data re-imported with proper chunking

---

## Related Documentation

- [CHANGELOG.md](./CHANGELOG.md) - Double-write pattern, semantic filtering
- [CLAUDE.md](./CLAUDE.md) - Architecture overview
- [TODO-SYNTHESIS.md](./TODO-SYNTHESIS.md) - Phase tracking
- Context bomb analysis: active_context capture 2025-10-24 @ 07:55 PM

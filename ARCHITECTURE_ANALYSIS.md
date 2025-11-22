# Vector Search Architecture Refactoring Analysis
## floatctl-rs TypeScript (evna) + Rust Integration

**Date**: November 16, 2025
**Scope**: Architecture maturation, naming consolidation, Rust ecosystem fit
**Status**: Research phase → Decision + Implementation blueprint

---

## Current Architecture State

### TypeScript/evna Search Landscape

#### What Exists

```
brain-boot.ts (377 lines)
├─ Manual orchestration of 5+ sources:
│  ├─ pgvectorTool.search() (dual-source: active_context + AutoRAG)
│  ├─ db.semanticSearchNotes() (AutoRAG note search)
│  ├─ db.getRecentMessages() (non-semantic recent)
│  ├─ GitHub status (if configured)
│  ├─ Daily notes (DailyNotesReader)
│  └─ Cohere reranker (multi-source fusion)
└─ All logic inline in boot() method (230+ lines of logic)

pgvector-search.ts (146 lines) - CONFUSING NAME
├─ Class: PgVectorSearchTool
├─ Actually does: Dual-source search (active_context_stream + AutoRAG wrappers)
├─ Has methods:
│  ├─ search() - orchestrates active_context + embeddings
│  ├─ formatResults() - markdown formatting with source attribution
│  └─ Semantic filtering (embed query, batch embed messages, cosine similarity)
└─ Default threshold: 0.5 (but brain-boot uses 0.3 hardcoded)

db.ts (partial, ~150 lines of search methods)
├─ semanticSearch() - Wraps AutoRAG (historical)
├─ semanticSearchNotes() - Wraps AutoRAG (note/bridge search)
├─ getRecentMessages() - PostgreSQL direct query
└─ queryActiveContext() - PostgreSQL from active_context_stream

cohere-reranker.ts (80 lines)
├─ Class: CohereReranker
├─ Method: fuseMultiSource() - Reranks heterogeneous sources
└─ Graceful fallback if COHERE_API_KEY missing

active-context-stream.ts (200+ lines)
├─ Class: ActiveContextStream
├─ Provides: Real-time message capture with annotation parsing
└─ Smart truncation: 400 chars, sentence-boundary aware

embeddings.ts (100+ lines)
├─ Class: EmbeddingsClient
├─ Methods:
│  ├─ embed() - Single vector via OpenAI
│  ├─ embedBatch() - Batch embedding with caching
│  └─ cosineSimilarity() - Similarity calculation
└─ Used by: pgvector-search for semantic filtering
```

#### The Problem

**Naming Confusion**
- Class called "PgVectorSearchTool" but doesn't use pgvector anymore
- Wraps AutoRAG (Cloudflare) for actual vector search
- Name suggests PostgreSQL vector extension, but that's not the pattern

**Scattered Orchestration**
- brain-boot.ts has 5+ hardcoded search calls
- No unified abstraction for "search results"
- Deduplication logic in pgvector-search.ts, but additional dedup in brain-boot.ts
- Threshold defaults inconsistent (0.5 vs 0.3)

**Mixing Concerns**
- Vector search logic (active_context + AutoRAG fusion) in pgvector-search.ts
- Ranking logic (Cohere) in cohere-reranker.ts
- Search result formatting in pgvector-search.ts
- Search orchestration in brain-boot.ts

**Result**: Hard to reason about, hard to test, hard to extend

---

## Refactoring Decision: Option B (Orchestrator Class)

### Why Option B?

| Aspect | A (Rename) | B (Orchestrator) | C (Module) |
|--------|-----------|------------------|-----------|
| **Effort** | 30 min | 2 hours | 4 hours |
| **Solves naming issue** | ✅ | ✅ | ✅ |
| **Consolidates logic** | ❌ | ✅✅ | ✅✅✅ |
| **Improves testability** | ⚠️ | ✅ | ✅✅ |
| **Fits current code** | ✅ | ✅✅ | ✅✅✅ |
| **Maturity win** | Small | **Large** | Very large |
| **Technical debt** | Still scattered | **Solved** | Solved + future-proof |

**Recommendation**: **Option B - Orchestrator Class**

Reasoning:
- Solves the immediate naming + scattering problem
- Doesn't over-engineer (avoid premature module extraction)
- Sets up infrastructure for future enhancements (Phase 3: Burp-Aware Brain Boot)
- Natural progression: scattered → orchestrator → module (if needed)
- Time investment aligns with scope (2 hours isn't minor, but isn't massive)

---

## Implementation Plan: Option B

### Step 1: Create VectorSearchOrchestrator Class

**File**: `evna/src/tools/vector-search-orchestrator.ts` (NEW, ~250 lines)

**Responsibilities**:
1. Coordinate dual-source search (active_context + AutoRAG)
2. Manage deduplication (single composite-key strategy)
3. Handle threshold consistency (default 0.3, allow override)
4. Format all result types (semantic, notes, recent)
5. Source attribution (active_context vs embeddings vs notes)

**Public Methods**:
```typescript
class VectorSearchOrchestrator {
  // Search operations
  async searchDualSource(options: DualSourceSearchOptions): Promise<SearchResult[]>
  async searchNotes(options: NoteSearchOptions): Promise<SearchResult[]>
  async searchRecent(options: RecentSearchOptions): Promise<SearchResult[]>

  // Multi-source coordination
  async fuseMultipleSources(query: string, sources: FusionOptions): Promise<FusionResult>

  // Utility methods
  deduplicateResults(results: SearchResult[]): SearchResult[]
  formatResults(results: SearchResult[], style?: 'markdown' | 'json'): string
}
```

**Key Decisions**:
- Single source of truth for threshold default (0.3)
- Composite deduplication key: `conversation_id::timestamp::content_prefix`
- Source attribution on all results (for brain_boot visualization)
- Graceful error handling with retry logic (matches autorag-client pattern)

### Step 2: Refactor pgvector-search.ts → dual-source-search.ts

**What Changes**:
- Rename file: `pgvector-search.ts` → `dual-source-search.ts`
- Rename class: `PgVectorSearchTool` → `DualSourceSearchTool`
- Extract logic into orchestrator (keep TL;DR here)
- Delegate to orchestrator for actual search

**Result**:
```typescript
// OLD: pgvector-search.ts (146 lines, mixed concerns)
// NEW: dual-source-search.ts (50 lines, thin wrapper)

export class DualSourceSearchTool {
  constructor(private orchestrator: VectorSearchOrchestrator) {}

  async search(options: SearchOptions): Promise<SearchResult[]> {
    return this.orchestrator.searchDualSource(options);
  }

  formatResults(results: SearchResult[]): string {
    return this.orchestrator.formatResults(results, 'markdown');
  }
}
```

### Step 3: Refactor brain-boot.ts to Use Orchestrator

**Current State** (brain-boot.ts lines 88-185):
```typescript
// 5 separate search calls with manual coordination
const semanticWithProject = await pgvectorTool.search(...);
const semanticFallback = await pgvectorTool.search(...);
const noteResults = await db.semanticSearchNotes(...);
const recentMessages = await db.getRecentMessages(...);
const githubStatus = await github.getUserStatus(...);
// ... manual deduplication, Cohere reranking, truncation
```

**New State** (brain-boot.ts lines ~88-120):
```typescript
// Single orchestrator call with declarative source specification
const fusionResult = await orchestrator.fuseMultipleSources(query, {
  semanticSearch: {
    project,
    lookbackDays,
    threshold: 0.3,
    weight: 0.4, // 40% of final ranking
  },
  noteSearch: {
    threshold: 0.25,
    weight: 0.3,
  },
  recentMessages: {
    limit: 20,
    weight: 0.1,
  },
  githubStatus: githubUsername ? { weight: 0.1 } : null,
  dailyNotes: { lookbackDays, weight: 0.1 },
  cohere: {
    apiKey: process.env.COHERE_API_KEY,
  },
});

// Result is already ranked, deduplicated, formatted
return fusionResult.summary;
```

**Benefits**:
- brain-boot.ts becomes 50 lines of orchestration (vs 230 lines of logic)
- Clear declarative specification of sources + weights
- Easier to test (mock orchestrator)
- Easier to add new sources (just add to fusion config)

### Step 4: Threshold Consistency

**Problem**: pgvector-search default 0.5, brain-boot hardcodes 0.3

**Solution**:
```typescript
// In VectorSearchOrchestrator
export interface SearchOptions {
  threshold?: number; // Defaults to 0.3 (consistent)
}

private DEFAULT_THRESHOLD = 0.3; // Single source of truth

// In brain-boot.ts, remove hardcoded 0.3
// Use orchestrator default everywhere
const fusionResult = await orchestrator.fuseMultipleSources(query, {
  semanticSearch: { /* no threshold specified = use default */ },
  noteSearch: { threshold: 0.25 }, // Override only when needed
});
```

### Step 5: Update Imports

**Files to Modify**:
1. `src/tools/index.ts` (line 71):
   - Old: `export const search = new PgVectorSearchTool(db, embeddings);`
   - New: `export const search = new DualSourceSearchTool(orchestrator);`

2. `src/tools/brain-boot.ts` (line 12, 45):
   - Old: `import { PgVectorSearchTool } from './pgvector-search.js';`
   - New: `import { VectorSearchOrchestrator } from './vector-search-orchestrator.js';`

3. `src/tools/registry-zod.ts` (line 130):
   - Update semantic_search threshold default from 0.5 to 0.3

4. Tests/CLI references:
   - Update imports in mcp-server.ts, interfaces/cli.ts, etc.

---

## Architecture: Before vs After

### BEFORE (Scattered)
```
brain-boot.ts
  ├─ Calls pgvectorTool.search() (5 ways)
  ├─ Calls db.semanticSearchNotes()
  ├─ Calls db.getRecentMessages()
  ├─ Calls github.getUserStatus()
  ├─ Calls dailyNotes.getRecentNotes()
  ├─ Manual deduplication
  ├─ Calls cohere.fuseMultiSource()
  └─ Manual formatting (500+ lines)

pgvector-search.ts
  ├─ Calls db.queryActiveContext()
  ├─ Calls db.semanticSearch()
  ├─ Semantic filtering (embed, batch embed, similarity)
  ├─ Deduplication (composite key)
  └─ formatResults()

db.ts
  ├─ semanticSearch() → AutoRAG
  ├─ semanticSearchNotes() → AutoRAG
  ├─ getRecentMessages() → SQL
  └─ queryActiveContext() → SQL

cohere-reranker.ts
  └─ fuseMultiSource() → Cohere API
```

### AFTER (Orchestrator Pattern)
```
brain-boot.ts
  └─ Calls orchestrator.fuseMultipleSources()
     └─ Returns ranked, deduplicated result

vector-search-orchestrator.ts (NEW)
  ├─ orchestrator.searchDualSource()
  │  ├─ Calls db.queryActiveContext()
  │  ├─ Calls db.semanticSearch()
  │  ├─ Semantic filtering
  │  └─ Deduplication
  ├─ orchestrator.searchNotes()
  │  ├─ Calls db.semanticSearchNotes()
  │  └─ Deduplication
  ├─ orchestrator.searchRecent()
  │  └─ Calls db.getRecentMessages()
  └─ orchestrator.fuseMultipleSources()
     ├─ Calls all search methods (parallel)
     ├─ Calls cohere.rerank() if configured
     ├─ Merges + ranks results
     └─ formatResults()

dual-source-search.ts (REFACTORED)
  └─ Thin wrapper around orchestrator

(Database, Cohere, embeddings remain unchanged)
```

**Result**: Clear separation of concerns, single responsibility per class

---

## Rust Ecosystem: Vector Search Integration

### Research Findings

**Recommendation**: Add optional in-memory HNSW index to floatctl-embed (Phase 1)

#### Why?

Your use case is ideal for HNSW:
- **Scale**: 5-50K vectors (easily fits in memory)
- **Philosophy**: Offline-first, curated indexing (perfect for batch HNSW)
- **Benefits**: 10-40x faster local queries (no network latency)
- **Risk**: Minimal (feature-gated, doesn't touch existing code)

#### Crates Evaluated

| Crate | Maturity | Dependencies | SIMD | Use Case | Recommendation |
|-------|----------|--------------|------|----------|-----------------|
| **hnsw** (rust-cv) | ✅ Stable | Minimal (smallvec, ahash) | ❌ | Pure Rust simplicity | **✅ PRIMARY** |
| **hnswlib-rs** | ✅ Battle-tested | Moderate | ✅ AVX2 | Performance focus | ✅ Alternative |
| **Hora** | ⚠️ Active dev | Moderate | ✅ Multi-algo | Feature-rich | Alternative |
| **SimSIMD** | ✅ Very active | Zero (C FFI) | ✅ AVX-512, NEON | Distance metric boost | Optional add-on |
| **pgvecto.rs** | ✅ New Rust ext | PostgreSQL | ✅ SIMD | Postgres replacement | Future consideration |

#### Architecture: Three Phases

**Phase 1: Optional In-Memory Index** (Recommended now)
```
floatctl-embed commands:
├─ --use-local-index: Query in-memory HNSW (10-40x faster)
└─ --use-pgvector: Query PostgreSQL (backward compatible, default)
```

**Phase 2: Streaming Index Build** (After Phase 1 validation)
```
Embed step refactoring:
├─ Stream NDJSON messages
├─ Batch embed with OpenAI
├─ Concurrent HNSW build (no memory overhead)
└─ Serialize HNSW index on completion
```

**Phase 3: Evna Independence** (Only if needed)
```
evna tools:
├─ brain_boot: Use local HNSW instead of pgvector
├─ semantic_search: Fall back to pgvector if no local index
└─ active_context: Keep PostgreSQL (metadata)
```

### Implementation Approach

**Phase 1: ~200 lines of Rust code**

```rust
// floatctl-embed/src/local_index.rs (NEW)
use hnsw::Hnsw;

pub struct LocalVectorIndex {
  hnsw: Hnsw<f32, cosine_distance>,
  vectors: Vec<Vec<f32>>,
}

impl LocalVectorIndex {
  pub fn new(capacity: usize) -> Self { /* ... */ }
  pub fn add_vector(&mut self, id: usize, vector: Vec<f32>) { /* ... */ }
  pub fn search(&self, query: &Vec<f32>, k: usize) -> Vec<(usize, f32)> { /* ... */ }
  pub fn save(&self, path: &str) -> Result<()> { /* serialize */ }
  pub fn load(path: &str) -> Result<Self> { /* deserialize */ }
}

// In embed command handler
if args.use_local_index {
  local_index.add_vector(message_id, embedding);
}

// After embedding completes
if args.use_local_index {
  local_index.save("~/.floatctl/indexes/messages.hnsw")?;
}
```

**Phase 1: Floatctl Integration**
```bash
# During embedding
cargo run -p floatctl-cli -- embed --in messages.ndjson --use-local-index

# Query local index
cargo run -p floatctl-cli -- query "search term" --use-local-index

# Query pgvector (existing behavior)
cargo run -p floatctl-cli -- query "search term" --use-pgvector
```

### Why Not Phase Out pgvector Yet?

1. **Infrastructure proven**: 18 months of reliable operation
2. **Operational features**: Replication, HA, persistence
3. **Easy coexistence**: Local index + pgvector work together
4. **Staged risk**: Validate Phase 1 before making permanent decisions
5. **evna flexibility**: Can use AutoRAG for historical, local HNSW for recent

---

## Clippy Analysis: Rust Code Quality

### Summary
- **29 warnings total** (across 6 crates)
- **Severity**: Minor linting issues (no bugs)
- **Autocorrectable**: 16 of 29 (via `cargo clippy --fix`)
- **Manual fixes**: 13 of 29 (e.g., collapsible patterns)

### Key Issues by Category

| Category | Count | Example | Effort |
|----------|-------|---------|--------|
| **Collapsible match** | 2 | Nested `if let` in config.rs | 5 min |
| **Useless conversions** | 4 | `.to_string()` vs `format!()` | 10 min |
| **Code style** | 8 | `push_str("\n")` → `push('\n')` | 15 min |
| **Type signatures** | 3 | `&PathBuf` → `&Path` | 10 min |
| **Derived traits** | 2 | Add `#[derive(Eq)]` | 5 min |
| **Redundant logic** | 3 | Unnecessary closures | 10 min |
| **Naming** | 2 | Variant ends with enum name | 5 min |
| **Misc** | 5 | Manual string parsing, etc. | 20 min |

### Recommended Action

**Quick fix** (80% of warnings, 30 mins total):
```bash
cargo clippy --fix --all
cargo clippy --fix --allow-dirty  # For manual ones
cargo fmt
```

**Then manual review** (20% of warnings, 20 mins):
- Review collapsible patterns in config.rs
- Verify derived trait additions
- Check renamed items

**No breaking changes** - just cleanup.

---

## Decision Summary

### TypeScript/evna: Option B (Orchestrator Class)

**What to do**:
1. Create `VectorSearchOrchestrator` class (~250 lines)
2. Refactor `pgvector-search.ts` → `dual-source-search.ts` (rename + thin wrapper)
3. Simplify `brain-boot.ts` to use orchestrator (230+ → 120 lines)
4. Fix threshold consistency (default 0.3 everywhere)

**Timeline**: ~2 hours implementation + 1 hour testing

**Benefits**:
- ✅ Fixes confusing naming (pgvector → dual-source)
- ✅ Consolidates scattered logic (single orchestrator)
- ✅ Improves testability (easier to mock)
- ✅ Enables future enhancements (Phase 3: burp-aware brain boot)
- ✅ Technical debt solved

**Risk**: Minimal (thin wrapper approach, easy to revert)

### Rust/floatctl: Phase 1 (Optional In-Memory HNSW)

**What to do**:
1. Add `hnsw = "0.11"` to floatctl-embed Cargo.toml
2. Implement `LocalVectorIndex` (~100 lines)
3. Add `--use-local-index` flag to query command
4. Benchmark Phase 1 vs pgvector on actual data

**Timeline**: ~4 hours implementation + 2 hours testing

**Benefits**:
- ✅ 10-40x faster local queries (no network latency)
- ✅ Reduces database pressure
- ✅ Minimal risk (feature-gated, doesn't replace pgvector)
- ✅ Aligned with "offline-first" philosophy

**Risk**: Very low (opt-in, pgvector remains default)

### Code Quality: Clippy Cleanup

**What to do**:
```bash
cargo clippy --fix --all
# Manual review of 13 non-auto-fixable warnings
```

**Timeline**: ~1 hour

**Benefits**:
- ✅ Clean compilation
- ✅ Best practices alignment
- ✅ Easier code review

---

## Implementation Sequence

### Week 1: TypeScript Refactoring
1. **Day 1**: Implement `VectorSearchOrchestrator` (250 lines, test coverage)
2. **Day 2**: Refactor `dual-source-search.ts` + `brain-boot.ts`
3. **Day 3**: Update imports + verify backward compatibility
4. **Day 4**: End-to-end testing (brain_boot, semantic_search, ask_evna)
5. **Day 5**: Code review + polish

### Week 2: Rust Investigation + Phase 1
1. **Day 1**: Setup HNSW crate, implement `LocalVectorIndex`
2. **Day 2**: Integrate with embed command
3. **Day 3**: Implement query routing (`--use-local-index` flag)
4. **Day 4**: Benchmark Phase 1 vs pgvector
5. **Day 5**: Documentation + decision on Phase 2

### Week 3: Code Quality
1. **Day 1-2**: Clippy fixes + code review
2. **Day 3**: Final validation
3. **Ongoing**: Observe TypeScript changes in production before proceeding to Phase 2

---

## Files to Modify (Checklist)

### TypeScript Changes

- [ ] **NEW**: `evna/src/tools/vector-search-orchestrator.ts` (250 lines)
- [ ] **RENAME**: `evna/src/tools/pgvector-search.ts` → `dual-source-search.ts`
- [ ] **REFACTOR**: `evna/src/tools/brain-boot.ts` (230+ → 120 lines)
- [ ] **UPDATE**: `evna/src/tools/index.ts` (imports + instantiation)
- [ ] **UPDATE**: `evna/src/tools/registry-zod.ts` (threshold default 0.5 → 0.3)
- [ ] **UPDATE**: `evna/src/interfaces/mcp.ts` (imports)
- [ ] **UPDATE**: `evna/src/mcp-server.ts` (imports)
- [ ] **UPDATE**: `evna/src/lib/db.ts` (if method signatures change)
- [ ] **UPDATE**: `evna/CLAUDE.md` (architecture docs)

### Rust Changes

- [ ] **UPDATE**: `floatctl-embed/Cargo.toml` (add hnsw dependency)
- [ ] **NEW**: `floatctl-embed/src/local_index.rs` (100 lines)
- [ ] **UPDATE**: `floatctl-embed/src/lib.rs` (public API)
- [ ] **UPDATE**: `floatctl-cli/src/embed.rs` (add `--use-local-index` flag)
- [ ] **UPDATE**: `floatctl-cli/src/query.rs` (add routing logic)
- [ ] **UPDATE**: `CLAUDE.md` (Phase 1 architecture notes)

### Code Quality

- [ ] **RUN**: `cargo clippy --fix --all`
- [ ] **REVIEW**: 13 manual clippy fixes
- [ ] **RUN**: `cargo fmt`
- [ ] **TEST**: `cargo test --all`

---

## Questions & Next Steps

### Questions for Clarification

1. **TypeScript prioritization**: Should Option B start immediately, or validate other improvements first?
2. **Rust timeline**: Is Phase 1 a good fit for next sprint, or should we wait?
3. **evna stability**: Any concerns about refactoring core orchestration logic?
4. **Benchmark targets**: What performance improvement would justify Phase 2 (streaming HNSW build)?

### Suggested Next Steps

1. **Review this analysis** - Validate Option B is the right call
2. **Create GitHub issue** for TypeScript refactoring with checklist
3. **Create GitHub issue** for Rust Phase 1 with phased approach
4. **Setup feature branch** for TypeScript work (base on main)
5. **Create spike PR** for Rust HNSW integration (validate approach before full implementation)

---

## Reference: Three-Option Comparison

### Option A: Quick Rename (30 minutes)
```
✅ Fixes naming (pgvector → dual-source)
❌ Doesn't solve scattering problem
❌ Threshold inconsistency remains
❌ brain-boot.ts still 230+ lines
→ Technical debt persists
```

### Option B: Orchestrator Class (2 hours) ⭐ RECOMMENDED
```
✅ Fixes naming
✅ Consolidates scattered logic
✅ Solves threshold consistency
✅ Improves testability
✅ Enables future enhancements
✅ Sets maturity baseline
→ Clean, extensible architecture
```

### Option C: Full Module Extraction (4 hours)
```
✅ All benefits of Option B
✅ Maximum separation of concerns
✅ Future-proof module structure
❌ Over-engineers for current scope
❌ Module extraction can happen later
→ Better done after observing Option B in production
```

---

**Recommendation**: **Proceed with Option B. Validate in production. Plan Option C only if Phase 3 (burp-aware brain boot) demands it.**

# Implementation Blueprint: Vector Search Architecture Refactoring

**Status**: Ready for implementation
**Owner**: Next session (cowboy mode)
**Decision**: Option B (Orchestrator Class) + Rust Phase 1 (In-Memory HNSW)

---

## TypeScript: VectorSearchOrchestrator Implementation

### Part 1: Create VectorSearchOrchestrator Class

**File**: `evna/src/tools/vector-search-orchestrator.ts` (NEW, ~300 lines)

**Purpose**: Single point of coordination for all search sources (semantic, notes, recent, GitHub, daily notes)

**Implementation template**:

```typescript
/**
 * Vector Search Orchestrator
 * Coordinates multiple search sources with unified interface
 * - Dual-source semantic search (active_context + AutoRAG)
 * - Note embeddings search
 * - Recent messages search
 * - Multi-source fusion with Cohere reranking
 */

import { DatabaseClient, SearchResult } from '../lib/db.js';
import { EmbeddingsClient } from '../lib/embeddings.js';
import { CohereReranker } from '../lib/cohere-reranker.js';

// Configuration interfaces
export interface SearchOptions {
  query: string;
  limit?: number;
  project?: string;
  since?: string;
  threshold?: number; // Defaults to 0.3 (moved from hardcoded values)
}

export interface NoteSearchOptions {
  query: string;
  limit?: number;
  threshold?: number; // Defaults to 0.25 (for notes)
}

export interface RecentSearchOptions {
  limit?: number;
  project?: string;
  since?: string;
}

export interface FusionOptions {
  semanticSearch?: SearchOptions & { weight?: number };
  noteSearch?: NoteSearchOptions & { weight?: number };
  recentMessages?: RecentSearchOptions & { weight?: number };
  githubStatus?: { weight?: number };
  dailyNotes?: { lookbackDays?: number; weight?: number };
  cohere?: { apiKey?: string };
}

export interface FusionResult {
  relevantContext: Array<{
    content: string;
    timestamp: string;
    project?: string;
    conversation?: string;
    similarity: number;
    source: string;
  }>;
  recentActivity: Array<{
    content: string;
    timestamp: string;
    project?: string;
  }>;
  sources: {
    semantic: SearchResult[];
    notes: SearchResult[];
    recent: any[];
    github?: string;
  };
}

export class VectorSearchOrchestrator {
  private DEFAULT_THRESHOLD = 0.3;
  private DEFAULT_NOTE_THRESHOLD = 0.25;

  constructor(
    private db: DatabaseClient,
    private embeddings: EmbeddingsClient,
    private reranker?: CohereReranker
  ) {}

  /**
   * Dual-source semantic search: active_context_stream + AutoRAG
   * Returns results with true cosine similarity scores and source attribution
   */
  async searchDualSource(options: SearchOptions): Promise<SearchResult[]> {
    const {
      query,
      limit = 10,
      project,
      since,
      threshold = this.DEFAULT_THRESHOLD,
    } = options;

    // Calculate lookback date
    const lookbackDate = since
      ? new Date(since)
      : new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);

    // RABBIT: Active context (30% of results, min 3)
    const activeLimit = Math.max(Math.floor(limit * 0.3), 3);
    const activeContextMessages = await this.db.queryActiveContext({
      limit: activeLimit,
      project,
      since: lookbackDate,
    });

    // TURTLE: Embeddings/AutoRAG (2x to account for overlap)
    const embeddingResults = await this.db.semanticSearch(query, {
      limit: limit * 2,
      project,
      since: since || lookbackDate.toISOString(),
      threshold,
    });

    // Semantic filtering for active_context
    const queryEmbedding = await this.embeddings.embed(query);
    const activeEmbeddings = activeContextMessages.length > 0
      ? await this.embeddings.embedBatch(
          activeContextMessages.map(msg => msg.content)
        )
      : [];

    const activeWithSimilarity = activeContextMessages
      .map((msg, idx) => ({
        msg,
        similarity: this.embeddings.cosineSimilarity(
          queryEmbedding,
          activeEmbeddings[idx]
        ),
      }))
      .filter(({ similarity }) => similarity >= threshold);

    // Convert to SearchResult format with source attribution
    const activeResults: SearchResult[] = activeWithSimilarity.map(
      ({ msg, similarity }) => ({
        message: {
          id: msg.message_id,
          conversation_id: msg.conversation_id,
          idx: 0,
          role: msg.role,
          timestamp: msg.timestamp,
          content: msg.content,
          project: msg.metadata?.project || null,
          meeting: msg.metadata?.meeting || null,
          markers: [],
        },
        conversation: {
          id: msg.conversation_id,
          conv_id: msg.conversation_id,
          title: null,
          created_at: msg.timestamp,
          markers: [],
        },
        similarity, // TRUE cosine similarity
        source: 'active_context', // Source attribution
      })
    );

    // Merge and deduplicate
    const allResults = [...activeResults, ...embeddingResults];
    return this.deduplicateResults(allResults).slice(0, limit);
  }

  /**
   * Search note embeddings (bridges, daily notes)
   * Lower threshold than semantic search (0.25 default)
   */
  async searchNotes(options: NoteSearchOptions): Promise<SearchResult[]> {
    const {
      query,
      limit = Math.ceil(10 * 0.5), // 50% allocation in brain_boot
      threshold = this.DEFAULT_NOTE_THRESHOLD,
    } = options;

    const results = await this.db.semanticSearchNotes(query, {
      limit,
      threshold,
    });

    return this.deduplicateResults(results).slice(0, limit);
  }

  /**
   * Search recent messages (non-semantic, just recency)
   */
  async searchRecent(options: RecentSearchOptions): Promise<SearchResult[]> {
    const { limit = 20, project, since } = options;

    const lookbackDate = since
      ? new Date(since)
      : new Date(Date.now() - 7 * 24 * 60 * 60 * 1000);

    return this.db.getRecentMessages({
      limit,
      project,
      since: lookbackDate.toISOString(),
    });
  }

  /**
   * Multi-source fusion with Cohere reranking
   * Orchestrates all search sources, optionally reranks with Cohere
   */
  async fuseMultipleSources(
    query: string,
    options: FusionOptions
  ): Promise<FusionResult> {
    // Execute searches in parallel
    const [semanticResults, noteResults, recentResults] = await Promise.all([
      options.semanticSearch
        ? this.searchDualSource(options.semanticSearch)
        : Promise.resolve([]),
      options.noteSearch
        ? this.searchNotes(options.noteSearch)
        : Promise.resolve([]),
      options.recentMessages
        ? this.searchRecent(options.recentMessages)
        : Promise.resolve([]),
    ]);

    // If Cohere configured, rerank across all sources
    if (this.reranker && options.cohere?.apiKey) {
      const rankedResults = await this.reranker.fuseMultiSource(
        query,
        {
          semanticResults: semanticResults.map(r => ({
            content: r.message.content,
            metadata: {
              timestamp: r.message.timestamp,
              project: r.message.project,
              conversation: r.conversation?.title || r.conversation?.conv_id,
              similarity: r.similarity,
              source: r.source || 'semantic_search',
            },
          })),
          noteEmbeddings: noteResults.map(r => ({
            content: r.message.content,
            metadata: {
              note_path: r.conversation?.title || r.conversation?.conv_id,
              similarity: r.similarity,
              source: 'note_embeddings',
            },
          })),
          dailyNotes: [], // Handled in brain_boot
          githubActivity: [], // Handled in brain_boot
        },
        10 // max results
      );

      return {
        relevantContext: rankedResults.map(r => ({
          content: r.document.text,
          timestamp: r.document.metadata?.timestamp || '',
          project: r.document.metadata?.project,
          conversation: r.document.metadata?.conversation,
          similarity: r.relevanceScore,
          source: r.document.metadata?.source || 'semantic_search',
        })),
        recentActivity: [],
        sources: {
          semantic: semanticResults,
          notes: noteResults,
          recent: recentResults,
        },
      };
    }

    // Fallback: No Cohere, just return all results
    return {
      relevantContext: semanticResults.map(r => ({
        content: r.message.content,
        timestamp: r.message.timestamp,
        project: r.message.project || undefined,
        conversation: r.conversation?.title || r.conversation?.conv_id,
        similarity: r.similarity,
        source: r.source || 'semantic_search',
      })),
      recentActivity: recentResults.map(r => ({
        content: r.content,
        timestamp: r.timestamp,
        project: r.project || undefined,
      })),
      sources: {
        semantic: semanticResults,
        notes: noteResults,
        recent: recentResults,
      },
    };
  }

  /**
   * Deduplicate search results by composite key
   * Key: conversation_id::timestamp::content_prefix (first 50 chars)
   * Fixes issue with Rust CLI empty string IDs
   */
  deduplicateResults(results: SearchResult[]): SearchResult[] {
    const seen = new Set<string>();
    return results.filter(result => {
      const key = `${result.message.conversation_id}::${result.message.timestamp}::${result.message.content.substring(0, 50)}`;
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
  }

  /**
   * Format search results as markdown
   * Used by semantic_search tool and brain_boot
   */
  formatResults(results: SearchResult[]): string {
    if (results.length === 0) {
      return '**No results found**';
    }

    const lines: string[] = [];
    lines.push(`# Search Results (${results.length} matches)\n`);

    results.forEach((result, idx) => {
      const { message, conversation, similarity, source } = result;
      const timestamp = new Date(message.timestamp).toLocaleString();
      const projectTag = message.project ? ` [${message.project}]` : '';
      const sourceTag = source === 'active_context' ? ' 🔴 Recent' : '';

      lines.push(
        `## ${idx + 1}. ${timestamp}${projectTag}${sourceTag} (similarity: ${similarity.toFixed(2)})`
      );
      lines.push(`**Conversation**: ${conversation?.title || conversation?.conv_id || 'Unknown'}`);
      lines.push(`**Role**: ${message.role}`);
      lines.push('');
      lines.push(message.content);
      lines.push('');
      lines.push('---');
      lines.push('');
    });

    return lines.join('\n');
  }
}
```

### Part 2: Refactor pgvector-search.ts → dual-source-search.ts

**File**: `evna/src/tools/dual-source-search.ts` (REFACTORED, ~50 lines)

**Implementation**:

```typescript
/**
 * Dual-Source Search Tool
 * Thin wrapper around VectorSearchOrchestrator
 * Searches active_context_stream (recent) + AutoRAG (historical)
 */

import { VectorSearchOrchestrator } from './vector-search-orchestrator.js';
import { SearchOptions } from '../lib/db.js';

export interface SearchOptions {
  query: string;
  limit?: number;
  project?: string;
  since?: string;
  threshold?: number;
}

export class DualSourceSearchTool {
  constructor(private orchestrator: VectorSearchOrchestrator) {}

  /**
   * Perform semantic search across conversation history
   * Delegates to orchestrator for actual search
   */
  async search(options: SearchOptions): Promise<any[]> {
    return this.orchestrator.searchDualSource(options);
  }

  /**
   * Format search results as markdown
   */
  formatResults(results: any[]): string {
    return this.orchestrator.formatResults(results);
  }
}
```

### Part 3: Refactor brain-boot.ts

**File**: `evna/src/tools/brain-boot.ts` (REFACTORED, ~120 lines actual orchestration logic)

**Key changes**:
1. Remove pgvectorTool (now use orchestrator directly)
2. Remove manual deduplication in lines 108-118 (handled by orchestrator)
3. Simplify Cohere reranking setup (orchestrator handles it)
4. Replace 88-185 line range with single orchestrator call

**Before (lines 88-106)**:
```typescript
// 1. Message embeddings + active_context (dual-source via pgvectorTool)
const semanticWithProject = project
  ? await this.pgvectorTool.search({
      query,
      limit: maxResults,
      project,
      since: sinceISO,
      threshold: 0.3,
    })
  : [];

const semanticFallback = semanticWithProject.length < maxResults
  ? await this.pgvectorTool.search({
      query,
      limit: maxResults * 2,
      project: undefined,
      since: sinceISO,
      threshold: 0.3,
    })
  : [];

// Merge and deduplicate (30+ lines)
const semanticResultsRaw = [...semanticWithProject, ...semanticFallback];
const seenMessages = new Set<string>();
const semanticResults = semanticResultsRaw
  .filter((r) => {
    const key = `${r.message.id}::${r.message.timestamp}`;
    if (seenMessages.has(key)) return false;
    seenMessages.add(key);
    return true;
  })
  .slice(0, maxResults);
```

**After (orchestrator approach)**:
```typescript
// Single call, orchestrator handles project filtering + deduplication
const semanticResults = await this.orchestrator.searchDualSource({
  query,
  limit: maxResults,
  project,
  since: sinceISO,
  threshold: 0.3, // Default from orchestrator, or override
});
```

### Part 4: Update Imports

**In `evna/src/tools/index.ts`** (line ~71):

```typescript
// BEFORE
export const search = new PgVectorSearchTool(db, embeddings);

// AFTER
const orchestrator = new VectorSearchOrchestrator(db, embeddings);
export const search = new DualSourceSearchTool(orchestrator);
```

**In `evna/src/tools/brain-boot.ts`** (line ~12, 45, 62):

```typescript
// BEFORE
import { PgVectorSearchTool } from './pgvector-search.js';
// ...
private pgvectorTool: PgVectorSearchTool;
// ...
this.pgvectorTool = new PgVectorSearchTool(db, embeddings);

// AFTER
import { VectorSearchOrchestrator } from './vector-search-orchestrator.js';
// ...
private orchestrator: VectorSearchOrchestrator;
// ...
this.orchestrator = new VectorSearchOrchestrator(db, embeddings);
```

**In `evna/src/tools/registry-zod.ts`** (line ~130):

```typescript
// BEFORE
threshold: z.number().optional().default(0.5).describe("..."),

// AFTER
threshold: z.number().optional().default(0.3).describe("..."),
```

### Part 5: Testing Checklist

**Unit tests** (for `VectorSearchOrchestrator`):
```typescript
// Create: evna/src/tools/__tests__/vector-search-orchestrator.test.ts

describe('VectorSearchOrchestrator', () => {
  describe('searchDualSource', () => {
    it('returns deduplicated results with source attribution')
    it('applies semantic filtering to active_context')
    it('respects threshold parameter')
    it('defaults to 0.3 threshold if not specified')
  })

  describe('deduplicateResults', () => {
    it('removes duplicate messages by composite key')
    it('preserves first occurrence if duplicates exist')
  })

  describe('formatResults', () => {
    it('generates markdown with correct format')
    it('includes source attribution (active_context vs embeddings)')
  })
})
```

**Integration tests** (for `brain-boot.ts`):
```typescript
describe('BrainBootTool', () => {
  describe('boot', () => {
    it('orchestrates multiple search sources')
    it('handles missing Cohere gracefully')
    it('filters by project when specified')
    it('applies lookback days correctly')
  })
})
```

---

## Rust: Phase 1 In-Memory HNSW Index

### Part 1: Add HNSW Dependency

**File**: `floatctl-embed/Cargo.toml`

```toml
[dependencies]
hnsw = { version = "0.11", default-features = false }
```

### Part 2: Create LocalVectorIndex

**File**: `floatctl-embed/src/local_index.rs` (NEW, ~100 lines)

```rust
use hnsw::Hnsw;
use std::fs;
use std::path::Path;
use anyhow::Result;

/// In-memory HNSW vector index for local query operations
/// Provides ~10-40x faster queries vs network pgvector calls
pub struct LocalVectorIndex {
    hnsw: Hnsw<f32, fn(&[f32], &[f32]) -> f32>,
    vectors: Vec<Vec<f32>>,
}

impl LocalVectorIndex {
    /// Create new index with given capacity
    pub fn new(capacity: usize) -> Self {
        Self {
            hnsw: Hnsw::new(12, capacity), // M=12 default (good balance)
            vectors: Vec::new(),
        }
    }

    /// Add vector to index
    pub fn add_vector(&mut self, id: usize, vector: Vec<f32>) {
        self.vectors.push(vector.clone());
        self.hnsw.add_link(id, &vector);
    }

    /// Search for K nearest neighbors
    pub fn search(&self, query: &[f32], k: usize) -> Vec<(usize, f32)> {
        let searcher = self.hnsw.searcher();
        searcher.search(query, k)
            .iter()
            .map(|(id, dist)| (*id, *dist))
            .collect()
    }

    /// Serialize index to file (after embedding completes)
    pub fn save(&self, path: &str) -> Result<()> {
        let serialized = serde_json::to_string(&self.hnsw)?;
        fs::write(path, serialized)?;
        Ok(())
    }

    /// Load index from file
    pub fn load(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let hnsw = serde_json::from_str(&content)?;
        Ok(Self {
            hnsw,
            vectors: Vec::new(), // Could reconstruct if needed
        })
    }
}
```

### Part 3: Integrate with Embed Command

**File**: `floatctl-cli/src/embed.rs` (MODIFY)

```rust
// In EmbedArgs struct
pub struct EmbedArgs {
    // ... existing fields ...

    /// Use local HNSW index instead of pgvector
    #[arg(long, help = "Build and query local HNSW index")]
    pub use_local_index: bool,
}

// In embed command handler
async fn run_embed(args: EmbedArgs) -> Result<()> {
    // ... existing embedding setup ...

    let mut local_index = if args.use_local_index {
        Some(LocalVectorIndex::new(10000))
    } else {
        None
    };

    // During embedding loop
    while let Some(line) = reader.next_line().await? {
        let embedding = openai.embed(&content).await?;

        // Store in pgvector (existing)
        store_embedding(&pool, &message_id, &embedding).await?;

        // Also add to local index if enabled (NEW)
        if let Some(ref mut index) = local_index {
            index.add_vector(parse_message_id(&message_id), embedding);
        }
    }

    // Save local index on completion
    if let Some(index) = local_index {
        let index_path = format!("{}/.floatctl/indexes/messages.hnsw",
            std::env::var("HOME")?);
        index.save(&index_path)?;
        eprintln!("Saved local index to: {}", index_path);
    }

    Ok(())
}
```

### Part 4: Add Query Routing

**File**: `floatctl-cli/src/query.rs` (MODIFY)

```rust
pub struct QueryArgs {
    // ... existing fields ...

    /// Query local HNSW index instead of pgvector
    #[arg(long)]
    pub use_local_index: bool,

    /// Fallback to pgvector if local index not available
    #[arg(long, default_value = "true")]
    pub fallback_pgvector: bool,
}

async fn run_query(args: QueryArgs) -> Result<()> {
    if args.use_local_index {
        let index_path = format!("{}/.floatctl/indexes/messages.hnsw",
            std::env::var("HOME")?);

        match LocalVectorIndex::load(Path::new(&index_path)) {
            Ok(index) => {
                // Query local index
                let query_embedding = openai.embed(&args.query).await?;
                let results = index.search(&query_embedding, args.limit);

                // Format results
                for (id, similarity) in results {
                    println!("{}: {}", id, similarity);
                }
                return Ok(());
            }
            Err(e) if args.fallback_pgvector => {
                eprintln!("Local index not found ({}), falling back to pgvector", e);
                // Continue to pgvector query below
            }
            Err(e) => return Err(e.into()),
        }
    }

    // Existing pgvector query code
    query_pgvector(args).await
}
```

### Part 5: Testing & Benchmarking

**Benchmark setup**:

```bash
# Build local index
cargo run -p floatctl-cli -- embed \
  --in messages.ndjson \
  --use-local-index

# Benchmark local vs pgvector
time cargo run -p floatctl-cli -- query "your search term" --use-local-index
time cargo run -p floatctl-cli -- query "your search term" --use-pgvector

# Expected result: local index 10-40x faster (no network latency)
```

---

## Integration Checklist

### TypeScript (Option B)

- [ ] Create `vector-search-orchestrator.ts` (~300 lines)
  - [ ] Implement all methods (searchDualSource, searchNotes, etc.)
  - [ ] Add unit tests
  - [ ] Verify semantic filtering works correctly

- [ ] Refactor `pgvector-search.ts` → `dual-source-search.ts`
  - [ ] Rename file
  - [ ] Rename class + simplify to thin wrapper
  - [ ] Verify imports still work

- [ ] Refactor `brain-boot.ts` (~240 → 120 lines of logic)
  - [ ] Replace pgvectorTool calls with orchestrator
  - [ ] Remove manual deduplication
  - [ ] Simplify Cohere setup
  - [ ] Update constructor

- [ ] Update imports across codebase
  - [ ] `tools/index.ts` (instantiation)
  - [ ] `tools/registry-zod.ts` (threshold default 0.5 → 0.3)
  - [ ] `interfaces/mcp.ts` (if needed)
  - [ ] `mcp-server.ts` (if needed)

- [ ] Run full test suite
  - [ ] `bun run typecheck` (REQUIRED)
  - [ ] `bun run test` (unit + integration)
  - [ ] Manual smoke test (brain_boot, semantic_search)

- [ ] Update documentation
  - [ ] `evna/CLAUDE.md` (architecture section)
  - [ ] Code comments in orchestrator

### Rust (Phase 1)

- [ ] Add `hnsw` dependency
  - [ ] Update `floatctl-embed/Cargo.toml`
  - [ ] Run `cargo build` to verify compilation

- [ ] Create `local_index.rs`
  - [ ] Implement `LocalVectorIndex` struct
  - [ ] Write unit tests for save/load

- [ ] Integrate with embed command
  - [ ] Add `--use-local-index` flag
  - [ ] Build index during embedding
  - [ ] Save on completion

- [ ] Integrate with query command
  - [ ] Add `--use-local-index` flag
  - [ ] Add routing logic
  - [ ] Implement fallback to pgvector

- [ ] Benchmark
  - [ ] Run clippy: `cargo clippy --all`
  - [ ] Run tests: `cargo test -p floatctl-embed`
  - [ ] Benchmark local vs pgvector queries
  - [ ] Document performance results

### Code Quality

- [ ] Run clippy fixes
  - [ ] `cargo clippy --fix --all`
  - [ ] Manual review of 13 non-auto fixes
  - [ ] `cargo fmt`

- [ ] Verify no regressions
  - [ ] `cargo test --all`
  - [ ] `bun run typecheck`

---

## Success Criteria

### TypeScript Refactoring
- ✅ VectorSearchOrchestrator exists and passes tests
- ✅ brain-boot.ts is <150 lines of orchestration logic
- ✅ All imports updated, no compilation errors
- ✅ `bun run typecheck` passes
- ✅ brain_boot, semantic_search, ask_evna all work
- ✅ Threshold consistency verified (0.3 everywhere)
- ✅ No performance regression

### Rust Phase 1
- ✅ Local HNSW index builds successfully
- ✅ Query routing works (--use-local-index flag)
- ✅ Fallback to pgvector on missing index
- ✅ Benchmark shows 10x+ speedup vs pgvector
- ✅ No compilation warnings
- ✅ Tests pass

### Code Quality
- ✅ Clippy warnings reduced from 29 to <5
- ✅ No new lint warnings introduced
- ✅ Code formatted with cargo fmt

---

## Risk Mitigation

### TypeScript Risks

**Risk**: Brain boot breaks due to orchestrator refactoring
- **Mitigation**: Keep pgvector-search.ts file intact during transition, switch imports gradually
- **Rollback**: Easy - revert to old imports if needed

**Risk**: Cohere reranking breaks in orchestrator
- **Mitigation**: Test with/without COHERE_API_KEY, verify fallback works
- **Rollback**: Copy old cohere logic if needed

**Risk**: Threshold inconsistency introduced
- **Mitigation**: Use `DEFAULT_THRESHOLD = 0.3` constant everywhere
- **Testing**: Query same test cases with old (0.5) vs new (0.3) threshold

### Rust Risks

**Risk**: HNSW index corruption/loss of data
- **Mitigation**: Keep pgvector as default, --use-local-index is opt-in
- **Rollback**: Just use --use-pgvector flag

**Risk**: HNSW serialization fails
- **Mitigation**: Handle save errors gracefully, warn user if save fails
- **Testing**: Verify save/load round-trip on 1K, 10K, 100K vectors

---

## Post-Implementation

### Observation Period (1-2 weeks)
- Monitor brain_boot performance in production
- Check for any threshold-related issues
- Validate HNSW build time + query speed with real data

### Phase 2 Planning (if needed)
- If HNSW shows 10x+ speedup: Proceed with streaming index build
- If marginal improvement: Keep pgvector as primary
- Decision gate: Real production performance data

### Documentation Updates
- Update CLAUDE.md with new architecture
- Add performance benchmarks to PR
- Document Phase 1 vs Phase 2 progression

---

## File Change Summary

### New Files
- `evna/src/tools/vector-search-orchestrator.ts` (300 lines)
- `floatctl-embed/src/local_index.rs` (100 lines)

### Modified Files
- `evna/src/tools/pgvector-search.ts` → `dual-source-search.ts` (146 → 50 lines)
- `evna/src/tools/brain-boot.ts` (377 → 250 lines, logic reduced by 50%)
- `evna/src/tools/index.ts` (instantiation change)
- `evna/src/tools/registry-zod.ts` (threshold default)
- `floatctl-embed/Cargo.toml` (add hnsw dep)
- `floatctl-cli/src/embed.rs` (add --use-local-index flag)
- `floatctl-cli/src/query.rs` (add routing logic)
- `CLAUDE.md` (both TypeScript and Rust sections)

### Deleted Files
- None (keep pgvector-search.ts as git history, rename to dual-source-search.ts)

---

**Next Step**: Review this blueprint, make any adjustments, then proceed with Part 1 (VectorSearchOrchestrator implementation).

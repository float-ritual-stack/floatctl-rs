# Brain Boot Synthesis Upgrade Plan
**Date**: 2025-10-23
**Context**: Improve brain_boot result synthesis + Claude Code SDK integration + Cohere API exploration
**Semantic Anchor**: SC-OCT23-BRAIN-BOOT-SYNTHESIS-COHERE-SDK

---

## 🎯 The Problem

**Current brain_boot architecture** (brain-boot.ts:73-101):
```typescript
// Fetches 5 parallel data sources BUT:
// ❌ Shows them as separate sections (not synthesized)
// ❌ Doesn't use dual-source semantic_search (uses embeddings-only db.semanticSearch)
// ❌ No reranking/relevance scoring across sources
// ❌ GitHub/daily notes just concatenated, not integrated
```

**Result**: Information dump, not synthesis. User gets 5 disconnected sections instead of one coherent narrative.

---

## 💡 Three-Pronged Solution

### 1. Cohere Reranking Layer (NEW!)
**Your Cohere API key**: `Qswbm3tObm8QujfmUVPogavdPcQ79q2tqLXWqYcA`

**Why Cohere?**
- Rerank API: Takes query + documents, returns relevance-sorted results
- Cross-encoder model (better than cosine similarity for ranking)
- Handles multi-source fusion (semantic search + active_context + daily notes)
- Free tier: 100 requests/min

**Integration point**: After fetching 5 parallel sources, rerank ALL results by relevance to query

### 2. Claude Code SDK Agent Integration
**Current package.json**: Already has `@anthropic-ai/claude-agent-sdk: ^0.1.14` ✅

**Opportunity**: Use SDK's agent orchestration for synthesis instead of manual string concatenation

### 3. Dual-Source Semantic Search Fix
**Current**: brain_boot calls `db.semanticSearch()` (embeddings only)
**Fix**: Use `PgVectorSearchTool.search()` (dual-source: embeddings + active_context)

---

## 🔧 Implementation Design

### Phase 1: Cohere Reranking Integration (2-3 hours)

#### 1.1 Install Cohere SDK
```bash
cd evna-next
bun add cohere-ai
```

#### 1.2 Create Cohere Client Library
**File**: `src/lib/cohere-reranker.ts` (NEW)

```typescript
/**
 * Cohere Reranking Client
 * Cross-encoder reranking for multi-source result fusion
 */

import { CohereClient } from 'cohere-ai';

export interface RerankDocument {
  text: string;
  metadata?: Record<string, any>;
}

export interface RerankResult {
  index: number;
  relevanceScore: number;
  document: RerankDocument;
}

export class CohereReranker {
  private client: CohereClient;

  constructor(apiKey: string) {
    this.client = new CohereClient({
      token: apiKey,
    });
  }

  /**
   * Rerank documents by relevance to query
   * @param query - Search query
   * @param documents - Documents to rerank
   * @param topN - Number of top results to return (default: 10)
   * @returns Reranked documents with relevance scores
   */
  async rerank(
    query: string,
    documents: RerankDocument[],
    topN: number = 10
  ): Promise<RerankResult[]> {
    // Cohere expects array of strings
    const texts = documents.map((doc) => doc.text);

    const response = await this.client.rerank({
      query,
      documents: texts,
      topN: Math.min(topN, documents.length),
      model: 'rerank-english-v3.0', // Latest Cohere rerank model
    });

    // Map back to original documents with scores
    return response.results.map((result) => ({
      index: result.index,
      relevanceScore: result.relevanceScore,
      document: documents[result.index],
    }));
  }

  /**
   * Fuse multiple result sources with reranking
   * Combines semantic search, active context, daily notes, GitHub
   */
  async fuseMultiSource(
    query: string,
    sources: {
      semanticResults: Array<{ content: string; metadata: any }>;
      activeContext: Array<{ content: string; metadata: any }>;
      dailyNotes: Array<{ content: string; metadata: any }>;
      githubActivity: Array<{ content: string; metadata: any }>;
    },
    topN: number = 10
  ): Promise<RerankResult[]> {
    // Flatten all sources into single document array with source metadata
    const allDocuments: RerankDocument[] = [
      ...sources.semanticResults.map((r) => ({
        text: r.content,
        metadata: { ...r.metadata, source: 'semantic_search' },
      })),
      ...sources.activeContext.map((r) => ({
        text: r.content,
        metadata: { ...r.metadata, source: 'active_context' },
      })),
      ...sources.dailyNotes.map((r) => ({
        text: r.content,
        metadata: { ...r.metadata, source: 'daily_notes' },
      })),
      ...sources.githubActivity.map((r) => ({
        text: r.content,
        metadata: { ...r.metadata, source: 'github' },
      })),
    ];

    // Rerank across ALL sources
    return this.rerank(query, allDocuments, topN);
  }
}
```

#### 1.3 Update Brain Boot to Use Cohere
**File**: `src/tools/brain-boot.ts` (MODIFY)

```typescript
import { CohereReranker } from '../lib/cohere-reranker.js';

export class BrainBootTool {
  private reranker: CohereReranker;

  constructor(
    private db: DatabaseClient,
    private embeddings: EmbeddingsClient,
    githubRepo?: string,
    dailyNotesDir?: string,
    cohereApiKey?: string // NEW
  ) {
    // ... existing initialization ...

    if (cohereApiKey) {
      this.reranker = new CohereReranker(cohereApiKey);
    }
  }

  async boot(options: BrainBootOptions): Promise<BrainBootResult> {
    // ... existing parallel fetch (lines 73-103) ...
    const [semanticResults, recentMessages, githubStatus, dailyNotes, activeContextMessages] = await Promise.all(promises);

    // NEW: Rerank all sources if Cohere available
    let rankedResults;
    if (this.reranker) {
      rankedResults = await this.reranker.fuseMultiSource(
        query,
        {
          semanticResults: semanticResults.map((r) => ({
            content: r.message.content,
            metadata: {
              timestamp: r.message.timestamp,
              project: r.message.project,
              conversation: r.conversation?.title,
              similarity: r.similarity,
            },
          })),
          activeContext: activeContextMessages.map((m) => ({
            content: m.content,
            metadata: {
              timestamp: m.timestamp,
              project: m.metadata.project,
              client_type: m.client_type,
            },
          })),
          dailyNotes: dailyNotes.map((note) => ({
            content: note.content,
            metadata: {
              date: note.date,
              type: 'daily_note',
            },
          })),
          githubActivity: githubStatus
            ? [{ content: githubStatus, metadata: { source: 'github' } }]
            : [],
        },
        maxResults
      );
    }

    // Generate SYNTHESIZED summary (not separate sections)
    const summary = this.generateSynthesizedSummary({
      query,
      rankedResults,
      project,
      lookbackDays,
    });

    return { summary, relevantContext: rankedResults, recentActivity: [] };
  }
}
```

---

### Phase 2: Claude Code SDK Synthesis Agent (3-4 hours)

#### 2.1 Create Synthesis Agent
**File**: `src/agents/synthesis-agent.ts` (NEW)

```typescript
/**
 * Synthesis Agent
 * Uses Claude Code SDK to generate coherent narrative from multi-source results
 */

import Anthropic from '@anthropic-ai/sdk';

export interface SynthesisInput {
  query: string;
  rankedResults: Array<{
    content: string;
    relevanceScore: number;
    metadata: {
      source: 'semantic_search' | 'active_context' | 'daily_notes' | 'github';
      timestamp?: string;
      project?: string;
      conversation?: string;
    };
  }>;
  project?: string;
  lookbackDays: number;
}

export class SynthesisAgent {
  private anthropic: Anthropic;

  constructor(apiKey: string) {
    this.anthropic = new Anthropic({ apiKey });
  }

  /**
   * Generate synthesized narrative from ranked results
   * Uses Claude 3.5 Sonnet for high-quality synthesis
   */
  async synthesize(input: SynthesisInput): Promise<string> {
    const { query, rankedResults, project, lookbackDays } = input;

    // Build synthesis prompt
    const prompt = this.buildSynthesisPrompt(input);

    const response = await this.anthropic.messages.create({
      model: 'claude-3-5-sonnet-20241022',
      max_tokens: 2048,
      messages: [
        {
          role: 'user',
          content: prompt,
        },
      ],
    });

    const textContent = response.content.find((c) => c.type === 'text');
    return textContent?.type === 'text' ? textContent.text : '';
  }

  /**
   * Build synthesis prompt from ranked results
   */
  private buildSynthesisPrompt(input: SynthesisInput): string {
    const { query, rankedResults, project, lookbackDays } = input;

    const sections: string[] = [];
    sections.push(`# Context Synthesis Request\n`);
    sections.push(`**Query**: ${query}`);
    if (project) sections.push(`**Project**: ${project}`);
    sections.push(`**Lookback**: Last ${lookbackDays} days\n`);

    sections.push(`## Ranked Context (by relevance)\n`);
    rankedResults.slice(0, 10).forEach((result, idx) => {
      const sourceEmoji = {
        semantic_search: '📚',
        active_context: '🔴',
        daily_notes: '📝',
        github: '🐙',
      }[result.metadata.source];

      sections.push(`### ${idx + 1}. ${sourceEmoji} ${result.metadata.source} (relevance: ${result.relevanceScore.toFixed(2)})`);
      if (result.metadata.timestamp) {
        sections.push(`**When**: ${new Date(result.metadata.timestamp).toLocaleString()}`);
      }
      if (result.metadata.project) {
        sections.push(`**Project**: ${result.metadata.project}`);
      }
      sections.push(`\n${result.content.substring(0, 500)}...\n`);
    });

    sections.push(`\n---\n`);
    sections.push(`## Task\n`);
    sections.push(`Synthesize the above context into a coherent narrative answering the query: "${query}"\n`);
    sections.push(`**Requirements**:`);
    sections.push(`1. **Temporal flow**: Organize by timeline (what happened when)`);
    sections.push(`2. **Pattern recognition**: Identify recurring themes across sources`);
    sections.push(`3. **Cross-referencing**: Connect GitHub PRs → daily notes → conversations`);
    sections.push(`4. **Actionable insights**: What's in progress, what's blocked, what's next`);
    sections.push(`5. **Source attribution**: Use emoji badges (📚 semantic, 🔴 recent, 📝 daily, 🐙 GitHub)`);
    sections.push(`\n**Format**: Markdown with clear sections, not bullet points`);

    return sections.join('\n');
  }
}
```

#### 2.2 Integrate Synthesis Agent into Brain Boot
**File**: `src/tools/brain-boot.ts` (MODIFY)

```typescript
import { SynthesisAgent } from '../agents/synthesis-agent.js';

export class BrainBootTool {
  private synthesisAgent?: SynthesisAgent;

  constructor(
    // ... existing params ...
    anthropicApiKey?: string // NEW
  ) {
    // ... existing initialization ...

    if (anthropicApiKey) {
      this.synthesisAgent = new SynthesisAgent(anthropicApiKey);
    }
  }

  async boot(options: BrainBootOptions): Promise<BrainBootResult> {
    // ... parallel fetch + Cohere reranking ...

    // NEW: Claude synthesis if available
    let summary;
    if (this.synthesisAgent && rankedResults) {
      summary = await this.synthesisAgent.synthesize({
        query,
        rankedResults,
        project,
        lookbackDays,
      });
    } else {
      // Fallback: manual string concatenation (current behavior)
      summary = this.generateSummary({ /* ... */ });
    }

    return { summary, relevantContext: rankedResults, recentActivity: [] };
  }
}
```

---

### Phase 3: Dual-Source Semantic Search Fix (1 hour)

#### 3.1 Use PgVectorSearchTool instead of db.semanticSearch
**File**: `src/tools/brain-boot.ts` (MODIFY lines 81-86)

```typescript
// BEFORE (embeddings only):
this.db.semanticSearch(query, {
  limit: maxResults,
  project,
  since: sinceISO,
  threshold: 0.3,
}),

// AFTER (dual-source: embeddings + active_context):
this.pgvectorTool.search({
  query,
  limit: maxResults,
  project,
  since: sinceISO,
  threshold: 0.3,
}),
```

**Why**: pgvector-search.ts already implements dual-source correctly (lines 29-89):
- Queries `active_context_stream` (recent, 🔴 priority)
- Queries `embeddings` (historical, via Rust CLI)
- Merges + deduplicates

---

## 📊 Complete Architecture Diagram

### Before (Current State)

```
brain_boot orchestrator
  ├─> db.semanticSearch() → embeddings only ❌ (empty for pharmacy)
  ├─> db.getRecentMessages() → messages table ❌ (empty for pharmacy)
  ├─> github.getUserStatus() → GitHub API ✅
  ├─> dailyNotes.getRecentNotes() → file system ✅
  └─> activeContext.queryContext() → active_context_stream ✅

      ↓ (manual string concatenation)

  5 SEPARATE SECTIONS (information dump)
```

### After (Hybrid Synthesis)

```
brain_boot orchestrator
  ├─> pgvectorTool.search() → dual-source (embeddings + active_context) ✅
  ├─> github.getUserStatus() → GitHub API ✅
  ├─> dailyNotes.getRecentNotes() → file system ✅
  └─> [REMOVED: redundant activeContext call, now in pgvectorTool]

      ↓ (Cohere reranking layer)

  Ranked multi-source fusion (top 10 by relevance)

      ↓ (Claude synthesis agent)

  COHERENT NARRATIVE (temporal flow + pattern recognition + attribution)
```

---

## 🚀 Implementation Phases

### Phase 1: Cohere Reranking (Tonight - 2-3 hours)
1. ✅ Install `cohere-ai` package
2. ✅ Create `src/lib/cohere-reranker.ts`
3. ✅ Update `brain-boot.ts` to use reranker
4. ✅ Test with pharmacy query
5. ✅ Verify relevance scores improve ranking

**Success Criteria**: brain_boot returns top 10 results ranked by Cohere relevance, mixing sources intelligently

### Phase 2: Claude Synthesis (Tomorrow - 3-4 hours)
1. ✅ Create `src/agents/synthesis-agent.ts`
2. ✅ Build synthesis prompt with source attribution
3. ✅ Integrate into `brain-boot.ts`
4. ✅ Test narrative quality vs string concatenation
5. ✅ Add caching for repeated queries (optional)

**Success Criteria**: brain_boot returns coherent narrative instead of 5 sections, with temporal flow and cross-references

### Phase 3: Dual-Source Fix (Tonight - 1 hour)
1. ✅ Import `PgVectorSearchTool` in `brain-boot.ts`
2. ✅ Replace `db.semanticSearch()` with `pgvectorTool.search()`
3. ✅ Remove redundant `activeContext.queryContext()` call
4. ✅ Test pharmacy query returns recent captures

**Success Criteria**: brain_boot semantic results include 🔴 Recent badges from active_context_stream

---

## 💰 Cost Analysis

### Cohere Rerank API
- **Free Tier**: 100 requests/min, 10,000 requests/month
- **Pricing**: $0.002 per 1,000 searches (after free tier)
- **Usage**: 1 request per brain_boot call (~10-20/day = $0.00004/day)

**Verdict**: Effectively free for personal use

### Claude Synthesis API
- **Model**: Claude 3.5 Sonnet
- **Input**: ~2,000 tokens (ranked results)
- **Output**: ~500 tokens (synthesis)
- **Cost**: ~$0.008 per brain_boot call
- **Usage**: ~10-20/day = $0.08-$0.16/day

**Verdict**: ~$5/month for high-quality synthesis

### Combined Cost
**Total**: ~$5/month (Claude dominates, Cohere negligible)

---

## 🧪 Testing Plan

### Unit Tests

```typescript
// src/lib/cohere-reranker.test.ts
describe('CohereReranker', () => {
  it('reranks documents by relevance to query', async () => {
    const reranker = new CohereReranker(process.env.COHERE_API_KEY!);

    const documents = [
      { text: 'pharmacy sprint demo prep work', metadata: { source: 'active' } },
      { text: 'unrelated blockchain stuff', metadata: { source: 'old' } },
      { text: 'GP node rendering fix PR #604', metadata: { source: 'github' } },
    ];

    const ranked = await reranker.rerank('pharmacy sprint demo', documents, 3);

    expect(ranked[0].document.text).toContain('pharmacy sprint');
    expect(ranked[0].relevanceScore).toBeGreaterThan(ranked[1].relevanceScore);
  });
});

// src/agents/synthesis-agent.test.ts
describe('SynthesisAgent', () => {
  it('generates coherent narrative from ranked results', async () => {
    const agent = new SynthesisAgent(process.env.ANTHROPIC_API_KEY!);

    const input = {
      query: 'pharmacy GP node work',
      rankedResults: [
        {
          content: 'PR #604 merged - GP node rendering fix',
          relevanceScore: 0.95,
          metadata: { source: 'github', timestamp: '2025-10-23T14:00:00Z' },
        },
        {
          content: 'Daily note: 02:42pm - GP node testing complete',
          relevanceScore: 0.88,
          metadata: { source: 'daily_notes', timestamp: '2025-10-23T14:42:00Z' },
        },
      ],
      project: 'rangle/pharmacy',
      lookbackDays: 7,
    };

    const narrative = await agent.synthesize(input);

    expect(narrative).toContain('timeline');
    expect(narrative).toContain('PR #604');
    expect(narrative).toContain('📝'); // daily notes emoji
  });
});
```

### Integration Test

```typescript
// src/tools/brain-boot.test.ts
describe('BrainBootTool with Cohere + Claude', () => {
  it('synthesizes pharmacy context with multi-source fusion', async () => {
    const db = new DatabaseClient(supabaseUrl, supabaseKey);
    const embeddings = new EmbeddingsClient(db);
    const brainBoot = new BrainBootTool(
      db,
      embeddings,
      'e-schultz/pharmacy',
      '~/.evans-notes/daily',
      process.env.COHERE_API_KEY,
      process.env.ANTHROPIC_API_KEY
    );

    const result = await brainBoot.boot({
      query: 'pharmacy sprint demo prep work PRs',
      project: 'pharmacy',
      lookbackDays: 3,
      maxResults: 10,
    });

    // Verify synthesis quality
    expect(result.summary).toContain('PR #604'); // GitHub
    expect(result.summary).toContain('sprint demo'); // Daily notes
    expect(result.summary).toMatch(/timeline|chronology/i); // Temporal flow
    expect(result.summary).toContain('🔴'); // Active context badge

    // Verify ranking
    expect(result.relevantContext.length).toBeGreaterThan(0);
    expect(result.relevantContext[0].relevanceScore).toBeGreaterThan(0.7);
  });
});
```

---

## 📝 Environment Variables

Add to `.env`:

```bash
# Existing
DATABASE_URL="postgresql://..."
OPENAI_API_KEY="sk-..."
ANTHROPIC_API_KEY="sk-ant-..."

# NEW
COHERE_API_KEY="Qswbm3tObm8QujfmUVPogavdPcQ79q2tqLXWqYcA"
```

---

## 🎯 Success Metrics

### Before vs After Comparison

**Query**: "pharmacy sprint demo prep work PRs"

#### Before (Current)
```markdown
# Brain Boot: 2025-10-23

**Query**: pharmacy sprint demo prep work PRs
**Lookback**: Last 3 days

## Semantically Relevant Context (0 results)
*No relevant context found*

## Recent Activity (0 messages)
*No recent activity*

## Daily Notes
- 04:43pm - [issue::580] → PR #607 addressing review comments
- 03:28pm - [issue::551] → PR #606 created
- 02:42pm - [project::evna-next] dual-source semantic_search complete

## Active Context (10 messages)
[10 separate message dumps with no synthesis]
```

#### After (With Cohere + Claude)
```markdown
# Brain Boot: 2025-10-23

**Query**: pharmacy sprint demo prep work PRs
**Lookback**: Last 3 days

## Synthesized Context

Over the past 3 days, you've been preparing for tomorrow's pharmacy sprint demo with three key PRs:

**Timeline & Progress:**
- **Oct 23, 10:47am** 🔴: PR #604 blocker cleared (GP node rendering fix) - ready for review
- **Oct 23, 2:42pm** 📝: Issue #551 wrapped up (switch node visibility)
- **Oct 23, 3:28pm** 🐙: PR #606 created for switch node fix
- **Oct 23, 4:43pm** 📝: PR #607 addressing review comments (auto-add to basket)

**Cross-Project Patterns:**
While focusing on pharmacy PRs, you also shipped evna-next dual-source semantic_search (PR #5, commit 66e772b) 📚 - the same pattern now powering this synthesis!

**Sprint Demo Readiness:**
✅ PR #604 (GP node) - awaiting code review
⏳ PR #606 (switch visibility) - ready soon
⚠️ PR #607 (auto-add) - needs error handling fixes per agent review

**Next Actions:**
1. Monitor PR #604 review status for demo readiness
2. Complete PR #607 error handling fixes before demo
3. Sync with Scott on final demo flow
```

---

## 🔮 Future Enhancements

### Phase 4+: Advanced Features

1. **Cohere Embeddings** (alternative to OpenAI):
   - Cohere Embed v3: Multilingual, 1024 dimensions
   - Cost: $0.0001 per 1,000 tokens (10x cheaper than OpenAI)
   - Integration: Replace `embeddings.ts` OpenAI client with Cohere

2. **Claude Caching** (reduce synthesis cost):
   - Cache ranked results prompt prefix
   - Only pay for new query text ($0.001/token vs $0.003/token)
   - 80% cost reduction for repeated brain_boot calls

3. **Streaming Synthesis** (faster UX):
   - Use Claude streaming API
   - Display synthesis as it generates (not all at once)
   - Better for long narratives (>1,000 tokens)

---

## 📚 Related Documents

- **Brain Boot Source**: `src/tools/brain-boot.ts:58-144`
- **PgVector Dual-Source**: `src/tools/pgvector-search.ts:29-89`
- **Tool Registry**: `src/tools/registry-zod.ts:56-189`
- **Architecture Gap Analysis**: (tonight's /util:er investigation)

---

**{ synthesis upgrade specification complete }**

*Generated: 2025-10-23 @ 11:11 PM*
*Session mode: synthesis_architecture → cohere_claude_integration*
*Pattern: multi-source fusion + LLM synthesis + relevance reranking*

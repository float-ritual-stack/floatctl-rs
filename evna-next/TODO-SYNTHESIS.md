# Brain Boot Synthesis TODO
**Started**: 2025-10-23 @ 11:19 PM
**Goal**: Ship Cohere reranking + dual-source fix tonight (2-3 hours)
**Defer**: Claude synthesis agent (diminishing returns, ship if Phase 1 works great)

---

## Phase 1: Cohere Reranking ⏳ (2-3 hours)
- [ ] Install cohere-ai package
- [ ] Create src/lib/cohere-reranker.ts (rerank() + fuseMultiSource())
- [ ] Update brain-boot.ts constructor (add cohereApiKey param)
- [ ] Update brain-boot.ts boot() (call reranker.fuseMultiSource after parallel fetch)
- [ ] Add COHERE_API_KEY to .env
- [ ] Test: brain_boot("pharmacy sprint demo") returns ranked results
- [ ] Commit: "Add Cohere reranking for multi-source fusion"

**Time tracking**:
- Start: 11:19 PM
- End: TBD
- Actual: TBD

---

## Phase 2: Dual-Source Fix ⏳ (30 min)
- [ ] Import PgVectorSearchTool in brain-boot.ts
- [ ] Replace db.semanticSearch() with pgvectorTool.search() (line 81)
- [ ] Remove redundant activeContext.queryContext() call (line 96)
- [ ] Test: pharmacy query returns 🔴 Recent badges
- [ ] Commit: "Fix brain_boot to use dual-source semantic search"

**Time tracking**:
- Start: TBD
- End: TBD
- Actual: TBD

---

## Phase 3: Claude Synthesis 🔮 (DEFER if tired)
- [ ] Create src/agents/synthesis-agent.ts
- [ ] Build synthesis prompt (temporal flow + source attribution)
- [ ] Wire into brain-boot.ts
- [ ] Test: narrative quality vs string concat
- [ ] Commit: "Add Claude synthesis agent for narrative generation"

**Status**: DEFERRED - ship Phases 1+2 first, evaluate ROI

---

## Architecture Decisions (Document Inline)

**Why Cohere?**
Cross-encoder reranking > cosine similarity for multi-source fusion. Cohere's rerank-english-v3.0 scores query relevance across heterogeneous sources (semantic search + active context + daily notes + GitHub) better than vector similarity alone.

**Why dual-source fix?**
pgvector-search.ts already works (queries embeddings + active_context_stream), brain_boot just wasn't using it. One-line change fixes empty pharmacy results.

**Why defer synthesis?**
Reranking solves relevance (core problem), synthesis is UX polish. Ship working reranking, evaluate if Claude narrative generation worth $5/month.

---

## Success Criteria (Tonight)
- ✅ Cohere reranking working (pharmacy query returns ranked multi-source results)
- ✅ Dual-source fix working (🔴 Recent badges appear)
- ✅ TODO-SYNTHESIS.md updated with actual time vs estimate
- ✅ Inline comments explain why (architecture choices preserved)

---

## Failure Recovery
- If Cohere integration takes >3 hours → commit what works, document blocker, defer
- If dual-source fix breaks existing behavior → revert, document why, try different approach
- If Claude synthesis feels like cathedral-building → skip it, ship Cohere reranking only

---

## Context Preservation for Future Claude

**On context loss, read these files**:
1. BRAIN-BOOT-SYNTHESIS-UPGRADE.md (full spec)
2. TODO-SYNTHESIS.md (this file - progress tracking)
3. Git log (architecture decisions in commit messages)
4. Inline code comments (non-obvious choices)

**Hermit Crab Principles**:
- Steal working patterns (pgvector-search.ts dual-source)
- Document INLINE (code comments > separate docs)
- Ship working code fast (2-3 hours, not 4 weeks)
- Todo list = living document (update as we learn)

---

**Last updated**: 2025-10-23 @ 11:19 PM (started)

# evna Architecture

**Last Updated:** 2025-10-30
**Version:** 1.0.0

## Purpose

This document defines evna's architectural vision and design principles. It serves as a validation checklist for new features and a reference for maintaining architectural consistency as evna evolves from "database proxy" to "agent orchestrator".

## Philosophy

evna evolves through **organic, empirical discovery** - not upfront theoretical design. We build enough to be useful, use it in real workflows, notice coordination gaps, and evolve architecture based on lived experience. This document captures patterns that have proven valuable through actual usage.

> "Let agents do agenty things - compose tools dynamically, reason about intent, synthesize coherent narratives. Stop treating evna as a database proxy that pushes coordination responsibility to the caller."

## Core Principles

### 1. Clean Interfaces Expose Intent, Not Implementation

**Pattern:**
```typescript
✓ CORRECT: active_context(query: "yesterday's pharmacy work")
✗ WRONG:   active_context(query, threshold, fallback_strategy, hybrid_weight...)
```

**Rationale:**
- MCP layer exposes WHAT (semantic intent)
- Agent layer decides HOW (strategy implementation)
- Internal tools handle WHERE (actual operations)

**Violation symptoms:**
- Parameters that don't affect behavior (write-only config)
- Exposing technical knobs (thresholds, weights, modes)
- Multiple variations of the same tool at MCP layer

### 2. Agent Layer Handles Coordination

**Agent responsibilities:**
- Parse user intent from natural language
- Decide strategy (chronological vs semantic vs hybrid)
- Coordinate tool invocation (compose multiple sources)
- Handle fallbacks gracefully (recent → relevant → cross-client)
- Return synthesis, not raw data

**NOT Caller responsibilities:**
- Intent interpretation ❌
- Strategy selection ❌
- Source composition ❌
- Result synthesis ❌

**Current violation example (active_context):**
Desktop Claude must manually scan chronological streams, interpret relevance, handle temporal filtering - work the agent layer should do.

### 3. Synthesis Over Excerpts

```markdown
WRONG: Truncated Chronological Stream
[10:13 AM] persona mechanics (consciousness-tech)
[10:13 AM] sparkling browncoats (consciousness-tech)
[10:13 AM] disability models (consciousness-tech)
→ Desktop Claude must scan for relevance (manual filtering)

CORRECT: Intelligent Synthesis
evna AGENT composes sources:
⏺ Recent work: pharmacy scrum prep (2025-10-30 @ 00:16)
◆ Yesterday's context: PRs #637, #641 shipped on recovery day
▲ Related pattern: recovery day protocol validated
◊ Synthesis: "Yesterday shipped 2 PRs during recovery day..."
```

**Rationale:** Agents should reason about data, not dump data for humans to reason about.

### 4. Organic Evolution Over Upfront Design

**Process that works:**
1. Build enough to be useful (MCP tools operational)
2. Use it in real workflows (Desktop/iOS Claude)
3. Notice coordination gaps (query parameter ignored)
4. Identify where agents solve actual problems
5. Evolve architecture based on lived experience

**Anti-pattern:** Building agent orchestration because it sounds cool, before empirical gaps prove it's needed.

> "Not a thing in search of a problem, but a solution to observed coordination friction."

### 5. Composition Over Monolithic Implementation

**Don't build:**
```typescript
active_context_with_semantic_temporal_github_integration()
```

**Do build:**
```typescript
Agent composes:
- active_context.query() (database layer)
- semantic_search() (pgvector layer)
- read_daily_note() (filesystem layer)
- gh_pr_status() (GitHub CLI layer)
```

**Rationale:** Let agents compose tools dynamically based on intent, not pre-bake all variations.

### 6. Progressive Enhancement + Graceful Degradation

**Fallback Hierarchy:**
```
Try: Agent orchestration with semantic scoring
  ↓ (if agent unavailable)
Try: Basic query filtering (keyword matching)
  ↓ (if no matches)
Try: Project filter only (current implementation)
  ↓ (if still empty)
Return: Most recent N messages (chronological baseline)
```

**User never sees:** "No results found" when context exists.

### 7. Make Intent Visible

**Pattern:** Show strategy BEFORE execution, not just results after.

```typescript
evna agent receives query → announces strategy
"Searching recent pharmacy context from yesterday..."
  ↓
evna agent invokes tools → streams findings
"Found 4 related messages, composing synthesis..."
  ↓
evna agent returns synthesis → presents narrative
```

**NOT:** Silent execution then data dump.

## Layer Boundaries

### Layer 1: MCP Protocol (External Interface)

**Purpose:** Clean, intent-focused APIs for external clients (Desktop Claude, iOS Claude, Agent SDK apps)

**Characteristics:**
- Consolidate by semantic intent (ONE tool per capability domain)
- Hide implementation complexity
- Simple parameters (query, project, limit)
- No strategy knobs (no threshold, fallback_mode, etc.)

**Example:**
```typescript
// MCP exposes this:
active_context(query: string, project?: string, limit?: number)

// NOT this:
active_context_recent(), active_context_semantic(), active_context_hybrid()
```

### Layer 2: Agent Orchestration (Coordination Logic)

**Purpose:** Parse intent, decide strategy, compose tools, synthesize results

**Characteristics:**
- Lives BEHIND MCP interface (invisible to caller)
- Reasons about user intent
- Composes fine-grained internal tools
- Synthesizes coherent narratives
- Handles fallbacks gracefully

**Responsibilities:**
- Intent parsing: "yesterday's pharmacy work" → {temporal: 2025-10-29, domain: pharmacy, strategy: recent}
- Strategy decision: recent vs relevant vs hybrid
- Multi-source composition: database + semantic search + daily notes + GitHub
- Synthesis generation: narrative from sources

**Example:**
```typescript
class ActiveContextAgent {
  async query(userQuery: string) {
    // 1. Parse intent
    const intent = this.parseIntent(userQuery);
    // "yesterday's pharmacy work" → {temporal, domain, strategy}

    // 2. Decide strategy
    const strategy = this.decideStrategy(intent);
    // Recent? Semantic? Hybrid?

    // 3. Compose tools
    const sources = await this.composeSources(intent, strategy);
    // queryDatabase() + semanticSearch() + readDailyNote() + ghCLI()

    // 4. Synthesize
    return this.synthesize(sources, intent);
    // Temporal narrative + pattern recognition
  }
}
```

### Layer 3: Fine-Grained Tools (Internal Operations)

**Purpose:** Single-responsibility operations that agents compose dynamically

**Characteristics:**
- Single responsibility (do one thing well)
- Composable by agents
- Internal use only (not exposed at MCP layer)

**Examples:**
```typescript
// Database operations
async function queryActiveContext(filters: Filters): Promise<Message[]>
async function captureContext(message: string): Promise<void>

// Semantic search
async function semanticSearch(query: string): Promise<SearchResults>
async function generateEmbedding(text: string): Promise<number[]>

// Filesystem operations
async function readDailyNote(date: string): Promise<string>
async function listDailyNotes(since: string): Promise<string[]>

// GitHub CLI
async function ghCLI(args: string[]): Promise<string>
async function ghPRStatus(repo: string): Promise<PR[]>

// Code search
async function grepCodebase(pattern: string): Promise<GrepResults>
```

## Tool Design Patterns

### Pattern A: Consolidated MCP + Fine-Grained Internal

**MCP Layer (External):** One tool per semantic capability
```typescript
// Expose this:
r2_sync(operation: "status" | "trigger" | "start" | "stop" | "logs")

// NOT this:
r2_sync_status(), r2_sync_trigger(), r2_sync_start(), r2_sync_stop(), r2_sync_logs()
```

**Internal Layer:** Fine-grained operations for agent composition
```typescript
class R2SyncTool {
  async status(options) { ... }   // Composable
  async trigger(options) { ... }  // Composable
  async start(options) { ... }    // Composable
  async stop(options) { ... }     // Composable
  async logs(options) { ... }     // Composable
}
```

**Agent Layer (Future):** Composes operations dynamically
```typescript
r2_sync("status and show recent errors")
  → Agent composes: status() + logs() + error parsing + synthesis
```

### Pattern B: Natural Language → Structured Intent

```typescript
// User input (natural language)
"yesterday's pharmacy work"

// Agent parses to structured intent
{
  temporal: "2025-10-29",
  domain: "pharmacy",
  strategy: "recent",
  sources: ["database", "daily_notes"]
}

// Agent composes tools based on intent
await Promise.all([
  queryDatabase({project: "pharmacy", date: "2025-10-29"}),
  readDailyNote("2025-10-29")
]);
```

### Pattern C: Multi-Source Composition

```typescript
// Agent decides source combination based on intent
const sources = {
  database: await queryActiveContext(filters),
  semantic: await semanticSearch(query),
  daily: await readDailyNote(date),
  github: await ghPRStatus(repo)
};

// Agent synthesizes narrative
return synthesize(sources, {
  temporal: true,   // Organize chronologically
  patterns: true,   // Identify recurring themes
  highlights: true  // Extract key insights
});
```

## Evolution Strategy

### Phase 1: Make Existing Parameters Functional

**Goal:** Low-hanging fruit improvements

**Actions:**
- Make query parameter actually work (currently write-only)
- Add basic keyword matching to active_context
- Implement temporal filtering from ctx:: annotations
- Parse project:: markers for better filtering

**No Agent Orchestration Yet:** Just make current interface work better

**Timeline:** Immediate utility improvement

### Phase 2: Agent Orchestration Layer

**Goal:** Migrate from database proxy to agent coordination

**Actions:**
- Add intent parsing (natural language → structured query)
- Implement strategy decision logic (recent vs relevant vs hybrid)
- Build multi-source composition (database + semantic + files + GitHub)
- Generate synthesis (narrative from sources)

**Agent Orchestration Now:** evna becomes coordinator, not proxy

**Timeline:** After Phase 1 validated in production

### Phase 3: Semantic Similarity (Optional)

**Goal:** Advanced relevance ranking

**Actions:**
- Embedding-based relevance scoring
- Hybrid ranking (temporal + semantic)
- Cross-session pattern discovery

**Advanced Capabilities:** Only if Phase 2 proves insufficient

**Timeline:** Evaluate based on Phase 2 outcomes

## Case Studies

### Case Study 1: active_context (Canonical Example)

**Current State (Proxy):**
```typescript
// MCP interface exposes query param
active_context(query: "yesterday pharmacy", project: "pharmacy", limit: 5)

// But query is ignored! Only project is used
// Returns: Chronological excerpts (raw data dump)
// Desktop Claude: Must manually scan for relevance
```

**Problems:**
1. Query parameter write-only (implementation leak)
2. No intent parsing
3. No synthesis
4. Coordination burden on caller

**Target State (Agent):**
```typescript
// MCP interface same, but query actually works
active_context(query: "yesterday's pharmacy work")

// Agent layer reasons:
{
  temporal: "2025-10-29",
  domain: "pharmacy",
  strategy: "recent+keyword",
  sources: ["database", "daily_notes", "github"]
}

// Agent composes sources:
const db = await queryActiveContext({project: "pharmacy", date: "2025-10-29"});
const daily = await readDailyNote("2025-10-29");
const gh = await ghPRStatus("rangle/pharmacy");

// Agent synthesizes:
return `
⏺ Recent work: pharmacy scrum prep (2025-10-30 @ 00:16)
◆ Yesterday's context: PRs #637, #641 shipped on recovery day
▲ Related pattern: recovery day protocol validated
◊ Synthesis: "Yesterday shipped 2 PRs during recovery day..."
`;
```

**Lessons:**
- MCP interface stays simple (just query string)
- Agent layer handles ALL coordination
- Returns synthesis, not raw data
- Caller doesn't reason about strategy

### Case Study 2: r2_sync (Applying Principles)

**Before (Tool Proliferation):**
```typescript
// MCP exposed 5 separate tools
r2_sync_status()
r2_sync_trigger()
r2_sync_start()
r2_sync_stop()
r2_sync_logs()
```

**Problems:**
1. Tool noise (5 tools cluttering list)
2. Exposed implementation details at MCP layer
3. Caller must know which tool to use
4. No room for agent composition

**After (Consolidated MCP):**
```typescript
// MCP exposes 1 tool with operation param
r2_sync(operation: "status")
r2_sync(operation: "trigger", daemon_type: "daily", wait: true)
r2_sync(operation: "logs", daemon_type: "daily", lines: 20)
```

**Benefits:**
1. Clean namespace (1 tool vs 5)
2. Intent-focused interface
3. Room for future agent composition

**Internal (Still Fine-Grained):**
```typescript
class R2SyncTool {
  async status(options) { ... }   // Composable
  async trigger(options) { ... }  // Composable
  async start(options) { ... }    // Composable
  async stop(options) { ... }     // Composable
  async logs(options) { ... }     // Composable
}
```

**Future Agent Composition:**
```typescript
r2_sync("status and show recent activity")
  → Agent composes:
     1. status() internally
     2. logs() internally
     3. Parse for errors
     4. Synthesize: "Daily daemon healthy (last sync 11:25 AM), dispatch active.
        Recent: 142 files synced to daily/, 119 to dispatch/. No errors."
```

**Lessons:**
- Consolidate at MCP layer (single entry point)
- Keep internal tools fine-grained (for composition)
- Enable future agent orchestration
- Reduce tool noise

## Anti-Patterns to Avoid

### Anti-Pattern 1: Database Proxy

**Symptoms:**
- Thin wrapper around database queries
- Exposes implementation details (table schemas, query params)
- Pushes coordination to caller
- Returns raw excerpts, not synthesis

**Example:**
```typescript
// BAD: Proxy pattern
function active_context(project, limit, client_type) {
  return db.query("SELECT * FROM active_context WHERE project = $1 LIMIT $2",
                  [project, limit]);
}
```

**Solution:** Add agent layer that reasons, composes, synthesizes.

### Anti-Pattern 2: Parameter Explosion

**Symptoms:**
- Too many configuration knobs
- Technical parameters leak into interface
- Unclear which params affect behavior

**Example:**
```typescript
// BAD: Parameter explosion
active_context(
  query,
  semantic_threshold,
  fallback_mode,
  hybrid_weight,
  temporal_boost,
  cross_client_priority
)
```

**Solution:** Hide strategy decisions in agent layer, expose simple intent-focused params.

### Anti-Pattern 3: Tool Proliferation at MCP Layer

**Symptoms:**
- Many similar tools at MCP layer
- Variations of same operation exposed separately
- Caller must know which tool to pick

**Example:**
```typescript
// BAD: Tool proliferation
r2_upload(), r2_download(), r2_sync(), r2_list(), r2_delete(), r2_status()
```

**Solution:** Consolidate by intent at MCP layer, keep fine-grained internally.

### Anti-Pattern 4: Premature Agent-ification

**Symptoms:**
- Adding agents before gaps proven empirically
- "Thing in search of a problem"
- Over-engineering simple queries

**Example:**
```typescript
// BAD: Agent for simple lookup
"Get today's date" → Agent orchestration with intent parsing
```

**Solution:** Only add agent layer when coordination gaps observed in actual usage.

### Anti-Pattern 5: Silent Coordination

**Symptoms:**
- Agent executes without showing strategy
- User doesn't see decision process
- Opaque black box

**Example:**
```typescript
// BAD: Silent execution
query("pharmacy work") → [magic happens] → results
```

**Solution:** Announce strategy, stream progress, make intent visible.

## Validation Checklist

**Before shipping new MCP tool, validate:**

- [ ] **Intent-focused interface?** Exposes what (intent), not how (implementation)
- [ ] **Agent coordinates?** Not pushing composition to caller
- [ ] **Synthesis not excerpts?** Returns coherent narrative, not raw data
- [ ] **Graceful fallbacks?** Hierarchy defined for degradation
- [ ] **Fine-grained internally?** Internal tools composable by agents
- [ ] **Progressive enhancement?** Phased rollout strategy clear
- [ ] **Empirical gap identified?** Solving observed problem, not theoretical

**Red flags:**
- ❌ Many parameters that don't affect behavior
- ❌ Multiple tool variations at MCP layer
- ❌ Coordination logic in caller
- ❌ Technical knobs exposed (thresholds, weights, modes)
- ❌ No synthesis, just data dumps
- ❌ No fallback strategy

## Summary

evna evolves from **database proxy** (thin wrapper exposing implementation details) to **agent orchestrator** (reasons about intent, composes tools, synthesizes results).

**Key architectural moves:**
1. **Consolidate at MCP layer** - ONE tool per capability domain
2. **Coordinate in agent layer** - Parse intent, decide strategy, compose sources, synthesize
3. **Keep internal tools fine-grained** - Single responsibility, composable by agents
4. **Progressive enhancement** - Phase 1 (functional params) → Phase 2 (agent layer) → Phase 3 (advanced)
5. **Organic evolution** - Build → use → notice gaps → evolve (not upfront design)

**Validation test:** Does this tool push coordination to the caller, or does evna own the complexity?

If evna owns coordination → ✅ Good architecture
If caller must compose → ❌ Proxy anti-pattern

---

**Contributing:** When adding new tools or features, refer to this doc. If architectural decisions conflict with these principles, update this doc with rationale and versioning.

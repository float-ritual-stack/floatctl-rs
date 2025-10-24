---
created: 2025-10-24 @ 10:22 AM
type: architecture_design
project: evna-next
status: draft
purpose: Design hybrid capture pattern for active_context with verbatim + summary + annotations
---

# Hybrid Capture Design for Active Context Stream

## Problem Statement

**User observation** (ctx::2025-10-24 @ 10:19 AM):
> Original shower burp had rich detail (direct edit vs event queue, eventual consistency, multi-tool log aggregation, intentional friction) but active_context capture collapsed to brief query string. Signal loss in compression.

**Current capture example**:
```typescript
// User's rich burp:
"in floatctl-rs/evna-next - there ssome new md files keeping track of recent work
also pondering about more agentic evna
and what tweaks could be made to floatctl-rs to help with this
thinking in jsol logging
also the intersection of float-hub and managing things around here
[...10+ more lines with architectural trade-offs...]"

// Kitty's compressed capture:
ctx::2025-10-24 @ 10:18 AM - [mode::burp_to_structure] - User invoked /util:er for: shower ponder on agentic evna architecture + floatctl-rs tweaks + daily note automation patterns
```

**What was lost**: Direct edit vs event queue comparison, eventual consistency philosophy, multi-tool log aggregation details, intentional friction reasoning

---

## Design Decision: Option 3 (Hybrid)

**User chose**: "tihnk 3 -- and then 'if users message is too long' (as I do dump quite alot sometimes) --> then a more strucuted summary of the burp"

**Pattern**: Store three layers:
1. **Verbatim**: Full user message text (raw_content)
2. **Structured summary**: For long burps, kitty creates extraction (summary)
3. **Annotations**: Parsed metadata (metadata JSONB)

---

## Schema Design

### Current Schema (active_context_stream)
```sql
CREATE TABLE active_context_stream (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    conversation_id TEXT NOT NULL,
    timestamp_unix BIGINT NOT NULL,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    role TEXT NOT NULL,
    content TEXT NOT NULL,
    client_type TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW()
);
```

### Proposed Schema Addition
```sql
-- Add two new fields to active_context_stream
ALTER TABLE active_context_stream
ADD COLUMN raw_content TEXT,           -- Full user message (NULL for assistant messages)
ADD COLUMN summary JSONB;              -- Structured extraction for long burps

-- Migration: Backfill existing data
UPDATE active_context_stream
SET raw_content = content
WHERE role = 'user' AND raw_content IS NULL;
```

### Field Usage Pattern

**For short user messages** (<500 chars):
```json
{
  "raw_content": "ctx::2025-10-24 @ 10:30 AM working on PR #604",
  "content": "ctx::2025-10-24 @ 10:30 AM working on PR #604",
  "summary": null,
  "metadata": {
    "ctx": { "date": "2025-10-24", "time": "10:30 AM" },
    "work_items": ["PR #604"]
  }
}
```

**For long user burps** (>500 chars):
```json
{
  "raw_content": "[full 10-paragraph shower burp verbatim]",
  "content": "ctx::2025-10-24 @ 10:18 AM - [mode::burp_to_structure] - shower ponder on agentic evna architecture...",
  "summary": {
    "intent": "deep_exploration + architecture_design",
    "topics": [
      "agentic evna evolution",
      "floatctl-rs integration",
      "daily note automation"
    ],
    "comparisons": [
      {
        "option_1": "evna edits daily note directly",
        "option_2": "evna logs to event queue → reconcile later"
      }
    ],
    "philosophies": [
      "eventual consistency model",
      "intentional friction preservation",
      "JSONL as coordination mechanism"
    ],
    "questions": [
      "What logging hooks needed in floatctl-rs?",
      "What gets edited directly vs queued?",
      "How to centralize Claude Code session logs?"
    ],
    "references": [
      "floatctl-rs/evna-next/*.md files",
      "last few days daily notes",
      "inbox contents",
      "JSONL Archaeology Handbook"
    ]
  },
  "metadata": {
    "ctx": { "date": "2025-10-24", "time": "10:18 AM" },
    "mode": "burp_to_structure",
    "project": "evna-next",
    "command": "/util:er",
    "burp_length": 1247,
    "structured_summary": true
  }
}
```

**For assistant messages**:
```json
{
  "raw_content": null,  // Don't duplicate assistant responses
  "content": "[assistant message text]",
  "summary": null,
  "metadata": {
    "response_to": "user_burp_id",
    "tools_used": ["active_context", "read", "glob"]
  }
}
```

---

## Summary Schema Design

For long burps, kitty extracts structure into `summary` JSONB:

```typescript
interface BurpSummary {
  intent: string;              // "search" | "build" | "review" | "deep" | "mixed"
  topics: string[];            // Key topics mentioned
  comparisons?: Comparison[];  // A vs B trade-offs
  philosophies?: string[];     // Principles/approaches mentioned
  questions?: string[];        // Open questions posed
  references?: string[];       // Files/docs/artifacts referenced
  constraints?: string[];      // "don't automate X", "intentional friction"
  decisions?: Decision[];      // Choices made or requested
}

interface Comparison {
  option_1: string;
  option_2: string;
  context?: string;
}

interface Decision {
  choice: string;
  rationale?: string;
  timestamp?: string;
}
```

---

## Implementation Pattern

### Phase 1: Capture Logic (TypeScript)

**File**: `src/lib/active-context-stream.ts`

```typescript
async captureMessage(params: {
  conversation_id: string;
  role: 'user' | 'assistant';
  content: string;
  client_type?: string;
}) {
  const metadata = this.parser.extractMetadata(params.content);

  let raw_content = null;
  let summary = null;
  let final_content = params.content;

  // Only for user messages
  if (params.role === 'user') {
    raw_content = params.content;

    // Long burp detection (>500 chars)
    if (params.content.length > 500) {
      summary = await this.extractBurpSummary(params.content);
      final_content = this.compressForContent(params.content, summary);
      metadata.burp_length = params.content.length;
      metadata.structured_summary = true;
    }
  }

  await this.db.query(`
    INSERT INTO active_context_stream
    (conversation_id, role, content, raw_content, summary, metadata, client_type, timestamp_unix)
    VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
  `, [
    params.conversation_id,
    params.role,
    final_content,
    raw_content,
    summary ? JSON.stringify(summary) : null,
    JSON.stringify(metadata),
    params.client_type,
    Date.now()
  ]);
}

private async extractBurpSummary(content: string): Promise<BurpSummary> {
  // Option 1: Rule-based extraction (fast, no API calls)
  const topics = this.extractTopics(content);
  const questions = this.extractQuestions(content);
  const comparisons = this.extractComparisons(content);

  // Option 2: LLM-assisted (slower, more accurate)
  // Could call Claude API with structured output prompt

  return {
    intent: this.detectIntent(content),
    topics,
    questions,
    comparisons,
    references: this.extractFileReferences(content)
  };
}

private compressForContent(raw: string, summary: BurpSummary): string {
  // Create human-readable compressed version for 'content' field
  const ctx = this.extractCtxMarker(raw);
  const topicStr = summary.topics.slice(0, 3).join(', ');
  return `${ctx} - ${summary.intent} on: ${topicStr}`;
}
```

### Phase 2: Query Enhancement

**File**: `src/tools/active-context.ts`

```typescript
async queryContext(params: {
  query?: string;
  limit?: number;
  include_summaries?: boolean;  // NEW
}) {
  const results = await this.stream.query(params);

  return results.map(msg => {
    if (params.include_summaries && msg.summary) {
      // Include structured summary in output
      return {
        ...msg,
        summary_extracted: JSON.parse(msg.summary)
      };
    }
    return msg;
  });
}
```

### Phase 3: Kitty Behavior Update

**Pattern for kitty creating capture markers**:

```typescript
// BEFORE (compressed)
ctx::2025-10-24 @ 10:18 AM - [mode::burp_to_structure] - User invoked /util:er for: shower ponder on agentic evna architecture + floatctl-rs tweaks + daily note automation patterns

// AFTER (with structured summary for long burps)
ctx::2025-10-24 @ 10:18 AM - [mode::burp_to_structure] - [project::evna-next] - User invoked /util:er exploring agentic architecture: Direct edit vs event queue approaches, eventual consistency model, JSONL coordination mechanism, multi-tool log aggregation, intentional friction preservation. Questions: floatctl-rs logging hooks, reconciliation workflow, Claude Code session log centralization. References: evna-next/*.md, daily notes, inbox, JSONL Archaeology Handbook.
```

**Capture marker template for long burps**:
```
ctx::YYYY-MM-DD @ HH:MM [AM/PM] - [mode::X] - [project::Y] - User {action} exploring {intent}: {key_topics}. Trade-offs: {comparisons}. Questions: {questions}. References: {files_mentioned}.
```

**Length guidelines**:
- Short message (<500 chars): Minimal compression, preserve detail
- Long burp (>500 chars): Extract structure, create 2-3 sentence summary
- Very long burp (>1000 chars): Use bullet points in capture marker

---

## Example: Shower Burp Capture

**Original burp** (1247 chars):
```
in floatctl-rs/evna-next - there ssome new md files keeping track of recent work, give those a read
also pondering about more agentic evna
and what tweaks could be made to floatctl-rs to help with this
thinking in jsol logging
also the intersection of float-hub and managing things around here
poke at the notes form the last few days, whats in the inbox, and also recent chat history
and maybe a bit of @artifacts/JSONL-Archaeology-Handbook.md -- i think we have a skill for that now also?
but -- a bit of friction - but some is intentional, i dont want to automate away /everything/ -- but -- keeping the daily note/etc upto date
is extending the agentic side of evna to help with some of that
thought one -> evna edits say my daily note directly ..
thought two -> evna gets better jsonl logging and capturing things --> and instead of making edits directly, writes to logs/event que that we then reconcile later (even if evna does the reconciling)
with lots of things eventual consistency is ok
remember this is tooling for me run off of my own devices
also pondering -> getting details from claude-code sessions (and other tools i use, although breen pretty consistent with sticking with claude code at the moment - but codex/opencode/a few others do enter the mix now and then)
but centralizing some of the key logs from those and exposing them --- as they help build a bigger picture of what is ging on
```

**Stored data**:

```json
{
  "raw_content": "[full burp verbatim as above]",

  "content": "ctx::2025-10-24 @ 10:18 AM - [mode::burp_to_structure] - [project::evna-next] - User exploring agentic evna architecture and daily note automation. Key trade-off: Direct edits vs event queue with eventual consistency. Wants intentional friction preserved. References: evna-next/*.md, daily notes, inbox, JSONL Archaeology Handbook.",

  "summary": {
    "intent": "deep_exploration + architecture_design",
    "topics": [
      "agentic evna evolution",
      "floatctl-rs integration points",
      "daily note automation",
      "JSONL logging patterns",
      "float-hub management"
    ],
    "comparisons": [
      {
        "option_1": "evna edits daily note directly",
        "option_2": "evna logs to event queue → reconcile later (eventual consistency)",
        "context": "daily note automation approach"
      }
    ],
    "philosophies": [
      "eventual consistency is ok for many things",
      "intentional friction preservation (don't automate away everything)",
      "tooling for personal devices only",
      "JSONL as coordination mechanism"
    ],
    "questions": [
      "What tweaks needed in floatctl-rs?",
      "How to centralize Claude Code session logs?",
      "What about other tools (codex/opencode)?",
      "What gets edited directly vs queued?"
    ],
    "references": [
      "floatctl-rs/evna-next/*.md files",
      "last few days daily notes",
      "inbox contents",
      "recent chat history",
      "artifacts/JSONL-Archaeology-Handbook.md",
      "JSONL archaeology skill"
    ],
    "constraints": [
      "don't automate away everything",
      "preserve intentional friction",
      "personal devices only"
    ]
  },

  "metadata": {
    "ctx": { "date": "2025-10-24", "time": "10:18 AM" },
    "mode": "burp_to_structure",
    "project": "evna-next",
    "command": "/util:er",
    "burp_length": 1247,
    "structured_summary": true,
    "personas": [],
    "float_methods": []
  }
}
```

**Query output** (default, without `include_summaries`):
```markdown
### User Message @ 10:18 AM

ctx::2025-10-24 @ 10:18 AM - [mode::burp_to_structure] - [project::evna-next] - User exploring agentic evna architecture and daily note automation. Key trade-off: Direct edits vs event queue with eventual consistency. Wants intentional friction preserved. References: evna-next/*.md, daily notes, inbox, JSONL Archaeology Handbook.
```

**Query output** (with `include_summaries: true`):
```markdown
### User Message @ 10:18 AM

**Intent**: Deep exploration + architecture design

**Topics**:
- Agentic evna evolution
- Floatctl-rs integration points
- Daily note automation
- JSONL logging patterns
- Float-hub management

**Trade-offs**:
- Option 1: evna edits daily note directly
- Option 2: evna logs to event queue → reconcile later (eventual consistency)

**Philosophies**:
- Eventual consistency is ok for many things
- Intentional friction preservation (don't automate away everything)
- JSONL as coordination mechanism

**Questions**:
- What tweaks needed in floatctl-rs?
- How to centralize Claude Code session logs?
- What gets edited directly vs queued?

**References**: evna-next/*.md, daily notes, inbox, JSONL Archaeology Handbook

[View full verbatim: raw_content]
```

---

## Benefits of Hybrid Approach

**1. Archaeological Value** ✅
- Full verbatim text preserved for future analysis
- No signal loss in compression
- Can always recover original context

**2. Query Performance** ✅
- Compressed `content` field for fast scanning
- Structured `summary` for faceted search
- JSONB metadata for annotation queries

**3. LLM Context Efficiency** ✅
- Default query returns compressed version (save tokens)
- Can request full verbatim when needed
- Structured summary enables better relevance filtering

**4. Multi-Format Output** ✅
- Short: Just `content` field (compressed)
- Medium: `content` + structured summary
- Full: `raw_content` (verbatim)

**5. Pattern Recognition** ✅
- Summary structure enables trend analysis
- "User always compares direct edit vs event queue"
- "User mentions intentional friction in 15% of burps"

---

## Implementation Phases

### Phase 1: Schema + Backfill (1 hour)
- [x] Add `raw_content` and `summary` columns
- [ ] Backfill existing `content` → `raw_content` for user messages
- [ ] Test migration on staging database

### Phase 2: Capture Logic (2-3 hours)
- [ ] Implement `extractBurpSummary()` (rule-based extraction)
- [ ] Add length detection (>500 chars)
- [ ] Update `captureMessage()` to populate new fields
- [ ] Write unit tests for summary extraction

### Phase 3: Query Enhancement (1 hour)
- [ ] Add `include_summaries` parameter to tool
- [ ] Format structured output in Markdown
- [ ] Test with existing queries

### Phase 4: Kitty Behavior (1-2 hours)
- [ ] Update capture marker template for long burps
- [ ] Add structured summary to markers
- [ ] Document guidelines (when to use full vs compressed)

### Phase 5: Dogfooding (ongoing)
- [ ] Use in real sessions for 1 week
- [ ] Tune summary extraction rules
- [ ] Adjust length threshold if needed
- [ ] Collect examples for documentation

---

## Open Questions

1. **LLM-assisted vs rule-based extraction?**
   - Rule-based: Fast, free, deterministic
   - LLM-assisted: More accurate, costs API tokens
   - **Proposal**: Start rule-based, add LLM as optional enhancement

2. **Length threshold?**
   - 500 chars seems reasonable for trigger
   - Could make configurable per-user
   - Monitor P50/P95/P99 message lengths in practice

3. **Summary schema evolution?**
   - JSONB allows flexible schema changes
   - Can add new fields without migration
   - Monitor what fields actually get used

4. **Verbatim storage cost?**
   - Postgres TEXT field is efficient for <10K chars
   - Could compress with pg_lz if storage becomes issue
   - Monitor table size growth

---

## Success Metrics

**After 1 week of dogfooding**:
- [ ] Zero "I said X but capture shows Y" observations
- [ ] User can query both compressed and verbatim
- [ ] Summary extraction captures 90%+ of key points
- [ ] No performance degradation in queries
- [ ] Storage cost acceptable (<2x increase)

**After 1 month**:
- [ ] Summary schema stable (no frequent changes)
- [ ] Pattern recognition queries working
- [ ] Archaeological value validated (found old context via verbatim)
- [ ] Length threshold tuned based on usage

---

## References

- Active Context Architecture: `ACTIVE_CONTEXT_ARCHITECTURE.md`
- Active Context Implementation: `ACTIVE_CONTEXT_IMPLEMENTATION.md`
- Annotation Parser: `src/lib/annotation-parser.ts`
- Active Context Stream: `src/lib/active-context-stream.ts`
- User observation: ctx::2025-10-24 @ 10:19 AM (meta_observation mode)

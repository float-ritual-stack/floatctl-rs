# floatctl hub Command Proposals

**Status:** PROPOSAL
**Created:** 2025-11-15
**Prerequisites:** Read [`CURRENT-STATE-DOCUMENTATION.md`](./CURRENT-STATE-DOCUMENTATION.md) first

---

## Philosophy: Boring, Composable, Deterministic

**LLMs as fuzzy compilers calling dumb tools:**
- Start deterministic (parse YAML, grep patterns, validate syntax)
- Add `--llm-model` flag for optional AI enhancement
- Output JSON for piping (`--json`)
- Small scope, clear purpose, composable

**Pattern:** Follow `floatctl bridge` and `floatctl claude` examples:
- Namespace: `floatctl hub <subcommand>`
- JSON output for automation
- Delegate to external tools (ollama, grep, fd)
- Security: use `execFile()` not shell execution

---

## Proposed Commands

### 1. floatctl hub metadata

**Purpose:** Validate and maintain markdown frontmatter.

#### floatctl hub metadata validate

**Problem:** Frontmatter accumulates incomplete/inconsistent fields. Manual checking is tedious.

**Implementation:** Deterministic (no LLM):
1. Walk directory with `walkdir` crate
2. Parse YAML frontmatter with `serde_yaml`
3. Validate required fields (configurable per collection type)
4. Check field formats (dates, tags, etc.)
5. Report issues as JSON or human-readable

**CLI:**
```bash
floatctl hub metadata validate PATH [OPTIONS]
  --collection <bridges|imprints|docs|daily>   # Collection-specific rules
  --fix                                         # Auto-fix common issues
  --report <json|text|summary>                  # Output format
  --required <field,field,...>                  # Additional required fields

# Examples:
floatctl hub metadata validate ~/float-hub/float.dispatch/bridges/ \
  --collection bridges \
  --report summary

floatctl hub metadata validate ~/float-hub/float.dispatch/bridges/ \
  --collection bridges \
  --json
```

**Before (manual):**
```bash
# Manual grep for missing fields
grep -L "^type:" float.dispatch/bridges/*.md
grep -L "^created:" float.dispatch/bridges/*.md
# Then inspect each file manually
```

**After (with tool):**
```bash
floatctl hub metadata validate float.dispatch/bridges/ \
  --collection bridges \
  --report summary

# Output:
# MISSING REQUIRED FIELDS:
# - evna-stuff.md: missing 'created', 'tags'
# - old-bridge.md: missing 'collection'
# INCONSISTENT:
# - some-file.md: created format '2025-11-15' should be '2025-11-15 @ HH:MM AM/PM'
# 3 files with issues, 47 valid
```

**JSON output:**
```json
{
  "valid": 47,
  "invalid": 3,
  "issues": [
    {
      "file": "evna-stuff.md",
      "missing": ["created", "tags"],
      "invalid": []
    },
    {
      "file": "some-file.md",
      "inconsistent": {
        "created": {
          "value": "2025-11-15",
          "expected_format": "YYYY-MM-DD @ HH:MM AM/PM"
        }
      }
    }
  ]
}
```

**When called:**
- **Background cron:** Daily validation report to operations/inbox/
- **Pre-commit hook:** Prevent incomplete metadata
- **Manual:** Before promoting turtle archaeology to bridges/

**Implementation notes:**
- Collection rules in `~/.config/floatctl/hub-metadata-rules.toml`:
  ```toml
  [collections.bridges]
  required = ["type", "created", "collection", "tags"]
  date_format = "YYYY-MM-DD @ HH:MM AM/PM"

  [collections.imprints]
  required = ["type", "created", "collection"]
  date_format = "YYYY-MM-DD @ HH:MM AM/PM"
  ```

**Auto-fix capabilities:**
- Trailing whitespace removal
- Ensure default field values (e.g., `draft: false`)
- Normalize date formats (if parseable)
- **Cannot auto-fix:** Missing required fields, broken YAML syntax

#### floatctl hub metadata sync

**Problem:** Bulk metadata updates when schema changes (e.g., adding new required field to all bridges).

**Implementation:** Deterministic:
1. Parse all frontmatter in directory
2. Apply field set/ensure operations
3. Preserve file structure (don't reformat unnecessarily)
4. Use `--dry-run` to preview changes

**CLI:**
```bash
floatctl hub metadata sync PATH [OPTIONS]
  --set <field=value>              # Set field to value
  --ensure <field>                 # Ensure field exists (use default from rules)
  --ensure <field=default>         # Ensure field exists with specific default
  --pattern <glob>                 # File pattern (e.g., "*.bridge.md")
  --dry-run                        # Show what would change
  --log-to <path>                  # Log changes to file (for INFRASTRUCTURE-CHANGELOG)

# Examples:
floatctl hub metadata sync float.dispatch/bridges/ \
  --ensure collection=bridges \
  --ensure draft=false \
  --dry-run

floatctl hub metadata sync float.dispatch/bridges/ \
  --ensure collection=bridges \
  --ensure draft=false \
  --log-to ~/float-hub/INFRASTRUCTURE-CHANGELOG.md
```

**Before (manual):**
```bash
# Manually edit 50+ bridge files to add new field
# Or write one-off sed/awk script that might break formatting
for f in float.dispatch/bridges/*.md; do
  # ... complex sed that breaks on edge cases
done
```

**After (with tool):**
```bash
# Dry run to preview
floatctl hub metadata sync float.dispatch/bridges/ \
  --ensure collection=bridges \
  --dry-run

# Shows:
# WOULD UPDATE 47 files:
# - evna-stuff.md: add 'collection: bridges'
# - old-bridge.md: add 'collection: bridges'
# ...

# Apply changes
floatctl hub metadata sync float.dispatch/bridges/ \
  --ensure collection=bridges

# Updated 47 files
# Logged to INFRASTRUCTURE-CHANGELOG.md (if --log-to specified)
```

**When called:**
- **Manual:** After updating METADATA-GUIDE.md schema
- **Script:** Bulk operations via operations/scripts/

**Implementation notes:**
- Parse frontmatter with `serde_yaml`
- Modify YAML tree in-place (preserve formatting)
- Write back only if changed (avoid unnecessary mtime updates)
- Log format matches INFRASTRUCTURE-CHANGELOG.md style:
  ```
  ## 2025-11-15 @ 02:30 PM EDT
  **what:** Updated 47 bridge files with `collection` field
  **why:** Metadata schema enforcement
  **who:** floatctl-hub-metadata-sync
  **where:** float.dispatch/bridges/
  ```

---

### 2. floatctl hub route

**Purpose:** Route inbox files to appropriate destinations.

#### floatctl hub route suggest

**Problem:** Files pile up in inbox/. Manual routing requires reading each file, understanding context, remembering directory structure.

**Implementation:** Start deterministic, add `--llm-model` for enhancement:

**Deterministic heuristics (99% of cases):**
1. Parse frontmatter (tags, type, context markers)
2. Match against known patterns:
   - `type: turtle-expedition` → `float.dispatch/imprints/the-curious-turtle/`
   - `tags: [consciousness-tech]` + synthesis → `float.dispatch/imprints/slutprints/`
   - `context: [rangle]` + issue number → `float.dispatch/imprints/rangle-weekly/reference/`
3. Confidence based on match strength (exact type > tag match > content keyword)

**LLM enhancement (optional, for ambiguous cases):**
```bash
floatctl hub route suggest inbox/ --llm-model llama3.2:3b
```

**CLI:**
```bash
floatctl hub route suggest FILE_OR_DIR [OPTIONS]
  --llm-model <ollama-model>           # Use local ollama for classification
  --confidence <0.0-1.0>               # Min confidence threshold (default: 0.75)
  --output <json|table>                # Output format
  --auto-route                         # Actually move files (requires --yes or interactive confirm)
  --log-to <path>                      # Log routing to INFRASTRUCTURE-CHANGELOG

# Examples:
floatctl hub route suggest inbox/ --output table

floatctl hub route suggest inbox/ --llm-model llama3.2:3b --confidence 0.8

floatctl hub route suggest inbox/ --auto-route  # Interactive confirm
floatctl hub route suggest inbox/ --auto-route --yes  # Skip confirm
```

**Before (manual):**
```bash
# You open each file in inbox/
# Read frontmatter and content
# Remember where things go
# Manually mv files one by one
# Manually update INFRASTRUCTURE-CHANGELOG.md
```

**After (with tool):**
```bash
floatctl hub route suggest inbox/ --output table

# Output:
# FILE                                    SUGGESTED_ROUTE                                      CONFIDENCE  REASON
# turtle-archaeology-2025-11-15.md        float.dispatch/imprints/the-curious-turtle/         0.92        type:turtle-expedition, tags match
# consciousness-tech-synthesis.md         float.dispatch/imprints/slutprints/                 0.75        tags:[consciousness-tech], synthesis content
# rangle-issue-700.md                     float.dispatch/imprints/rangle-weekly/reference/    0.95        context:[rangle], issue number detected
# ambiguous-file.md                       (no suggestion)                                     0.45        confidence below threshold

# Review suggestions, then:
floatctl hub route suggest inbox/ --auto-route --yes \
  --log-to ~/float-hub/INFRASTRUCTURE-CHANGELOG.md

# Moved 3 files, skipped 1 (below confidence threshold)
# Logged to INFRASTRUCTURE-CHANGELOG.md
```

**JSON output:**
```json
{
  "suggestions": [
    {
      "file": "inbox/turtle-archaeology-2025-11-15.md",
      "destination": "float.dispatch/imprints/the-curious-turtle/",
      "confidence": 0.92,
      "reason": "type:turtle-expedition, tags match",
      "method": "heuristic"
    },
    {
      "file": "inbox/ambiguous-file.md",
      "destination": null,
      "confidence": 0.45,
      "reason": "No strong heuristic match",
      "method": "heuristic"
    }
  ],
  "moved": 3,
  "skipped": 1
}
```

**When called:**
- **Background cron:** Nightly inbox scan, generate suggestions (no auto-route)
- **Manual:** After turtle archaeology delivery to inbox/
- **Hook:** User-triggered from Claude Code

**Implementation notes:**
- Routing rules in `~/.config/floatctl/hub-routing-rules.toml`:
  ```toml
  [[rules]]
  condition = { type = "turtle-expedition" }
  destination = "float.dispatch/imprints/the-curious-turtle/"
  confidence = 0.9

  [[rules]]
  condition = { tags = ["consciousness-tech"], any_of = ["synthesis", "philosophical"] }
  destination = "float.dispatch/imprints/slutprints/"
  confidence = 0.75

  [[rules]]
  condition = { context = ["rangle"], has_issue_number = true }
  destination = "float.dispatch/imprints/rangle-weekly/reference/"
  confidence = 0.95
  ```

- LLM mode calls ollama with simple prompt:
  ```
  Given this file metadata and first 500 characters, suggest which directory it should be routed to.
  Available destinations:
  - float.dispatch/imprints/the-curious-turtle/ (turtle archaeology expeditions)
  - float.dispatch/imprints/slutprints/ (consciousness tech synthesis)
  - float.dispatch/imprints/rangle-weekly/ (work project content)
  ...

  File: inbox/ambiguous-file.md
  Metadata: {...}
  Content preview: {...}

  Respond with JSON: {"destination": "...", "confidence": 0.8, "reason": "..."}
  ```

---

### 3. floatctl hub dispatch

**Purpose:** Parse and query `::` markers (ctx::, project::, issue::, mode::, meeting::).

#### floatctl hub dispatch parse

**Problem:** `::` markers are everywhere as "dispatches" but not easily queryable.

**Implementation:** Deterministic (pure regex parsing):
1. Read markdown files
2. Extract all `::` markers with regex: `(\w+)::([^\s]+)`
3. Group by dispatch type (ctx, project, issue, mode, meeting, lf1m, etc.)
4. Count occurrences, track files, first/last seen timestamps

**CLI:**
```bash
floatctl hub dispatch parse FILE_OR_DIR [OPTIONS]
  --type <ctx|project|mode|issue|all>   # Filter dispatch type
  --output <json|table|grouped>         # Output format
  --since <date>                        # Time filter (parse file mtimes)
  --recursive                           # Recursive directory scan

# Examples:
floatctl hub dispatch parse ~/.evans-notes/daily/ \
  --since 2025-11-01 \
  --output grouped

floatctl hub dispatch parse ~/float-hub/float.dispatch/bridges/ \
  --type project \
  --output json \
  --recursive
```

**Before (manual):**
```bash
# Grep for ctx:: markers across files
grep -h "ctx::" ~/.evans-notes/daily/*.md | sort | uniq
# Doesn't show frequency, file sources, or relationships
```

**After (with tool):**
```bash
floatctl hub dispatch parse ~/.evans-notes/daily/ \
  --since 2025-11-01 \
  --output grouped

# Output:
# CONTEXT DISPATCHES (Nov 1-15):
# ctx::rangle/pharmacy          47 occurrences  (files: 12)
# ctx::float-infrastructure     23 occurrences  (files: 8)
# ctx::consciousness-tech       19 occurrences  (files: 6)
#
# PROJECT DISPATCHES:
# project::rangle/pharmacy      34 occurrences  (files: 9)
# project::float-hub            12 occurrences  (files: 5)
#
# ISSUE DISPATCHES:
# issue::656                    28 occurrences  (files: 7)
# issue::633                    15 occurrences  (files: 4)
```

**JSON output:**
```json
{
  "ctx": {
    "rangle/pharmacy": {
      "count": 47,
      "files": ["2025-11-01.md", "2025-11-02.md", ...],
      "first_seen": "2025-11-01T09:00:00Z",
      "last_seen": "2025-11-15T14:30:00Z"
    },
    "float-infrastructure": {
      "count": 23,
      "files": [...],
      "first_seen": "2025-11-03T10:00:00Z",
      "last_seen": "2025-11-15T13:00:00Z"
    }
  },
  "project": {
    "rangle/pharmacy": {"count": 34, "files": [...] },
    "float-hub": {"count": 12, "files": [...] }
  },
  "issue": {
    "656": {"count": 28, "files": [...] },
    "633": {"count": 15, "files": [...] }
  }
}
```

**When called:**
- **Manual:** Understanding what contexts were active in a time period
- **Via script:** Generate context reports for brain-boot
- **Background:** Weekly dispatch analytics

**Implementation notes:**
- Regex: `(\w+)::([^\s]+)` captures `type::value`
- Parse file mtimes for `--since` filtering
- Track line numbers for detailed reporting (optional `--with-locations`)

---

### 4. floatctl hub summarize

**Purpose:** Batch generate summaries for files missing descriptions.

**Implementation:** LLM-driven (requires ollama):

**CLI:**
```bash
floatctl hub summarize FILE_OR_DIR [OPTIONS]
  --model <ollama-model>                # Default: llama3.2:3b
  --field <description|summary|tldr>    # Which field to generate
  --max-tokens <n>                      # Summary length (default: 100)
  --update-frontmatter                  # Write back to file
  --output <json|markdown>              # Results format
  --only-missing                        # Only process files missing the field
  --async                               # Queue for background processing
  --queue-id <id>                       # Queue identifier (for async mode)

# Examples:
# Summarize files missing descriptions
floatctl hub summarize inbox/ \
  --model llama3.2:3b \
  --field description \
  --only-missing \
  --update-frontmatter

# Async mode for large batches
floatctl hub summarize inbox/ \
  --model llama3.2:3b \
  --async \
  --queue-id inbox-summaries
```

**Before (manual):**
```bash
# You read each file
# Write summary by hand
# Update frontmatter manually
# Or skip it and files have no descriptions
```

**After (with tool):**
```bash
floatctl hub summarize inbox/ \
  --model llama3.2:3b \
  --field description \
  --only-missing \
  --update-frontmatter

# Processing: turtle-archaeology-2025-11-15.md
# Generated: "Archaeological findings from cursor exploration of float-hub structure, metadata patterns, and consciousness-tech archaeology"
# Updated frontmatter ✓
#
# Processing: synthesis-doc.md
# Generated: "Synthesis of evna behavioral protocols and mycelial architecture patterns"
# Updated frontmatter ✓
#
# Processed 2 files, skipped 3 (already have descriptions)
```

**Async mode:**
```bash
# Queue summarization
floatctl hub summarize inbox/ --async --queue-id inbox-nov-15

# Task queued: inbox-nov-15
# Check status: floatctl hub queue status inbox-nov-15

# Later:
floatctl hub queue status inbox-nov-15
# Status: RUNNING (processed 15/23 files)
# Elapsed: 3m 24s
```

**When called:**
- **Background job:** Nightly summarization of inbox/ files
- **Manual:** After turtle archaeology delivery
- **Hook:** Post-file-creation (optional)

**Implementation notes:**
- Call ollama API: `POST http://localhost:11434/api/generate`
- Prompt template:
  ```
  Summarize this markdown file in 1-2 sentences for a frontmatter 'description' field.
  Focus on what the document contains and its purpose.

  File: {{filename}}
  Content: {{content}}

  Summary:
  ```
- Store results in `~/.config/floatctl/hub-queue/{{queue-id}}.json` for async mode

---

### 5. floatctl hub patterns

**Purpose:** Extract convergent patterns from multiple files (e.g., turtle archaeology overlapping passes).

#### floatctl hub patterns extract

**Problem:** After turtle archaeology delivers 3+ overlapping passes, need to extract patterns that appear repeatedly = core insights.

**Implementation:** LLM-driven (requires ollama):

**CLI:**
```bash
floatctl hub patterns extract FILE_PATTERN [OPTIONS]
  --model <ollama-model>           # For semantic clustering (default: llama3.2:3b)
  --min-occurrences <n>            # Convergence threshold (default: 3)
  --output <json|markdown>         # Results format
  --output-file <path>             # Write synthesis doc
  --similarity-threshold <0-1>     # Semantic similarity for clustering (default: 0.75)

# Examples:
floatctl hub patterns extract inbox/turtle-archaeology-2025-11-* \
  --min-occurrences 3 \
  --output-file float.dispatch/imprints/the-curious-turtle/synthesis-nov-2025.md

floatctl hub patterns extract inbox/turtle-archaeology-2025-11-* \
  --min-occurrences 3 \
  --output json
```

**Before (manual):**
```bash
# Read multiple turtle archaeology files
# Manually note repeated themes
# Write synthesis document by hand
# Hope you didn't miss connections
```

**After (with tool):**
```bash
floatctl hub patterns extract inbox/turtle-archaeology-2025-11-* \
  --min-occurrences 3 \
  --output-file float.dispatch/imprints/the-curious-turtle/synthesis-nov-2025.md

# CONVERGENT PATTERNS (3+ occurrences):
# - "consciousness-tech as externalization" (4 files)
# - "mycelial architecture" (5 files)
# - "proactive context capture" (3 files)
# - "bridge promotion criteria" (4 files)
#
# Generated synthesis document with extracted quotes and sources
# Written to: float.dispatch/imprints/the-curious-turtle/synthesis-nov-2025.md
```

**Markdown synthesis output:**
```markdown
# Turtle Archaeology Synthesis: November 2025

**Generated:** 2025-11-15
**Source files:** 5 turtle archaeology expeditions
**Convergence threshold:** 3+ occurrences

## Convergent Patterns

### Consciousness-tech as Externalization (4 occurrences)

Pattern observed across multiple explorations: externalizing internal processes makes them visible and manipulable.

**Sources:**
- `inbox/turtle-archaeology-2025-11-10.md`: "Consciousness technology isn't accommodation, it's visibility..."
- `inbox/turtle-archaeology-2025-11-12.md`: "Making thought external = consciousness with better I/O..."
...

### Mycelial Architecture (5 occurrences)

Repeated observations about information flow following mycelial network patterns.

**Sources:**
- `inbox/turtle-archaeology-2025-11-10.md`: "Bridge networks function like mycelium..."
...
```

**When called:**
- **Manual:** After turtle archaeology delivery (curation step)
- **Background:** Weekly pattern extraction from imprints/the-curious-turtle/

**Implementation notes:**
- Extract all headings + paragraphs from source files
- Embed each using ollama embeddings model
- Cluster by cosine similarity (semantic grouping)
- Count occurrences per cluster
- Filter by `--min-occurrences`
- Generate synthesis with LLM using prompt:
  ```
  You are analyzing multiple overlapping exploration passes (turtle archaeology) to find convergent patterns.

  Pattern cluster: {{cluster_name}}
  Occurrences: {{occurrence_count}}
  Source quotes:
  {{quotes_with_sources}}

  Write a synthesis paragraph explaining this pattern and its significance.
  Include inline references to source files.
  ```

---

### 6. floatctl hub lint

**Purpose:** Markdown linting and cleanup.

**Implementation:** Deterministic (no LLM):

**Rules:**
- `frontmatter-format` - YAML syntax, required fields
- `heading-structure` - No skipped levels (h1 → h3 invalid)
- `link-validity` - Check internal `[[links]]` and `[markdown](links)`
- `trailing-whitespace` - Remove trailing spaces
- `consistent-lists` - Enforce bullet style (-, *, +)
- `code-fence-language` - All code blocks have language specifier

**CLI:**
```bash
floatctl hub lint PATH [OPTIONS]
  --fix                            # Auto-fix issues
  --rules <rule,rule,...>          # Specific rules to check
  --report <json|text|summary>     # Output format
  --config <path>                  # Custom lint rules config

# Examples:
floatctl hub lint float.dispatch/bridges/ --report summary

floatctl hub lint float.dispatch/bridges/ --fix --rules frontmatter-format,trailing-whitespace

floatctl hub lint float.dispatch/bridges/ --json
```

**Before (manual):**
```bash
# You manually check files or ignore issues
# Broken links accumulate
# Inconsistent formatting everywhere
```

**After (with tool):**
```bash
floatctl hub lint float.dispatch/bridges/ --report summary

# FRONTMATTER ISSUES: 3 files
# - missing-fields.md: missing 'created'
# - bad-yaml.md: YAML syntax error at line 4
#
# LINK ISSUES: 5 files
# - broken-link.md: [[nonexistent-file]] not found
# - bad-path.md: [link](./missing.md) not found
#
# FORMATTING: 12 files
# - headings.md: h1 → h3 skip at line 45
# - whitespace.md: 23 lines with trailing spaces

# Fix automatically:
floatctl hub lint float.dispatch/bridges/ --fix
# Fixed trailing whitespace in 12 files
# Fixed heading structure in 1 file
# (Skipped unfixable: broken links, missing frontmatter)
```

**When called:**
- **Pre-commit hook:** Lint staged markdown files
- **Background cron:** Weekly full-repo lint
- **Manual:** Before promoting files from inbox/

**Implementation notes:**
- Use `pulldown-cmark` for markdown parsing (AST traversal)
- Link validation: resolve `[[wikilinks]]` to actual files
- Heading structure: track heading levels, detect skips
- Auto-fix safe operations only (whitespace, list style)
- Cannot auto-fix: broken links, missing frontmatter, bad YAML

---

### 7. floatctl hub hook

**Purpose:** Hook helper functions for Claude Code integration.

#### floatctl hub hook context-inject

**Problem:** Claude Code hooks need to inject relevant context (recent work, active issues, related bridges) without manual setup.

**Implementation:** Deterministic + optional evna integration:

**CLI:**
```bash
floatctl hub hook context-inject [OPTIONS]
  --type <session-start|user-prompt>   # Hook type
  --scope <project|global>             # Context scope (if project, use $PWD)
  --max-tokens <n>                     # Context budget (default: 2000)
  --format <markdown|json>             # Output format
  --include-dispatches                 # Include :: marker summary
  --include-bridges                    # Include related bridge snippets

# Examples:
floatctl hub hook context-inject --type session-start --max-tokens 2000

floatctl hub hook context-inject --type user-prompt --scope project --include-dispatches
```

**Hook config (.claude/settings.json):**
```json
{
  "hooks": {
    "SessionStart": [{
      "hooks": [{
        "type": "command",
        "command": "floatctl hub hook context-inject --type session-start --max-tokens 2000"
      }]
    }]
  }
}
```

**Output (injected into Claude Code session):**
```markdown
## Recent Float-Hub Activity

**Last updated**: 2025-11-15 @ 02:17 PM

**Active contexts** (past 7 days):
- ctx::rangle/pharmacy (47 occurrences)
- ctx::float-infrastructure (23 occurrences)

**Recent issues**:
- issue::656 (weighted blend performance - MERGED)
- issue::676 (reactflow validation indicators - IN PROGRESS)

**Related bridges**:
- [[evna-behavioral-protocol]] (updated 2025-11-06)
- [[kitty-cowboy-agent-handoff-methodology]] (referenced 3x this week)

**Inbox items**: 5 files awaiting routing
```

**When called:**
- **SessionStart hook:** Inject recent work context
- **UserPromptSubmit hook:** Inject related bridge content based on query

**Implementation notes:**
- Parse `::` dispatches from recent daily notes (last 7 days)
- Find related bridges by tag/project match
- Count inbox files
- Format as markdown for injection
- Token counting: use tiktoken-rs for accurate budget

#### floatctl hub hook changelog-append

**Problem:** INFRASTRUCTURE-CHANGELOG.md updates are mandatory but easy to forget.

**Implementation:** Deterministic (append to file):

**CLI:**
```bash
floatctl hub hook changelog-append [OPTIONS]
  --what <description>              # One-line what
  --why <reason>                    # Why this change
  --where <path,path,...>           # Affected paths
  --who <persona>                   # Who made change (default: $USER)
  --dry-run                         # Show what would be added

# Examples:
floatctl hub hook changelog-append \
  --what "Routed 5 files from inbox to imprints" \
  --why "Turtle archaeology delivery cleanup" \
  --where "inbox/ → float.dispatch/imprints/the-curious-turtle/" \
  --who "kitty-claude"

# In a script:
floatctl hub route suggest inbox/ --auto-route --yes
floatctl hub hook changelog-append \
  --what "Auto-routed inbox files" \
  --why "Scheduled cleanup" \
  --where "inbox/" \
  --who "cron"
```

**Format appended:**
```markdown
## 2025-11-15 @ 02:30 PM EDT
**what:** Routed 5 files from inbox to imprints
**why:** Turtle archaeology delivery cleanup
**who:** kitty-claude
**where:** inbox/ → float.dispatch/imprints/the-curious-turtle/
```

**When called:**
- **Manual:** After bulk operations
- **Via script:** Wrapping file operations
- **Hook:** Post-route, post-summarize

**Implementation notes:**
- Append to `~/float-hub/INFRASTRUCTURE-CHANGELOG.md` (configurable)
- Get timezone from system
- Format timestamp in expected format
- Atomic append (file locking to prevent concurrent corruption)

---

### 8. floatctl hub queue

**Purpose:** Async task queue for long-running operations.

**Architecture:** Simple file-based queue with daemon processor.

#### floatctl hub queue add

**CLI:**
```bash
floatctl hub queue add <TASK_TYPE> [TASK_ARGS...] [OPTIONS]
  --queue-id <id>                  # Queue identifier (default: auto-generated)
  --priority <1-10>                # Priority (default: 5)

# Task types:
# - summarize FILE_PATTERN --model MODEL --field FIELD
# - embed FILE_PATTERN
# - patterns-extract FILE_PATTERN --min-occurrences N

# Examples:
floatctl hub queue add summarize inbox/*.md \
  --model llama3.2:3b \
  --field description \
  --queue-id inbox-summaries

floatctl hub queue add patterns-extract inbox/turtle-archaeology-* \
  --min-occurrences 3 \
  --queue-id turtle-patterns
```

#### floatctl hub queue status

**CLI:**
```bash
floatctl hub queue status [QUEUE_ID]

# Output:
# Queue: inbox-summaries
# Status: RUNNING
# Progress: 15/23 files processed
# Elapsed: 3m 24s
# Estimated completion: 2m 15s
# Logs: ~/.config/floatctl/logs/queue-inbox-summaries.log
```

#### floatctl hub queue list

**CLI:**
```bash
floatctl hub queue list

# Output:
# QUEUE_ID              STATUS      PROGRESS    CREATED
# inbox-summaries       RUNNING     15/23       2025-11-15 14:20:00
# turtle-patterns       PENDING     0/5         2025-11-15 14:25:00
# embedding-batch-01    COMPLETED   100/100     2025-11-15 13:00:00
```

#### floatctl hub queue logs

**CLI:**
```bash
floatctl hub queue logs <QUEUE_ID> [--follow]

# Output:
# [2025-11-15 14:23:01] Processing turtle-archaeology.md
# [2025-11-15 14:23:04] ✓ Generated description (87 tokens)
# [2025-11-15 14:23:04] Processing synthesis-doc.md
# ...
```

#### floatctl hub queue cancel

**CLI:**
```bash
floatctl hub queue cancel <QUEUE_ID>

# Output:
# Queue inbox-summaries cancelled (processed 15/23 files)
```

**When called:**
- **Manual:** Starting long-running operations
- **Via hooks:** Queue tasks from hooks without blocking
- **Background daemon:** `floatctl hub daemon` processes queue

**Implementation notes:**
- Queue state: `~/.config/floatctl/hub-queue/*.json`
- Format:
  ```json
  {
    "queue_id": "inbox-summaries",
    "task_type": "summarize",
    "args": ["inbox/*.md"],
    "options": {"model": "llama3.2:3b", "field": "description"},
    "status": "running",
    "progress": {"completed": 15, "total": 23},
    "created_at": "2025-11-15T14:20:00Z",
    "started_at": "2025-11-15T14:20:05Z",
    "log_file": "~/.config/floatctl/logs/queue-inbox-summaries.log"
  }
  ```

---

### 9. floatctl hub daemon

**Purpose:** Background daemon for processing queue and scheduled tasks.

**CLI:**
```bash
floatctl hub daemon <SUBCOMMAND>
  start     # Start daemon
  stop      # Stop daemon
  status    # Show daemon status
  logs      # Show daemon logs

# Options:
floatctl hub daemon start [OPTIONS]
  --config <path>                  # Config file (default: ~/.config/floatctl/hub-daemon.toml)
  --foreground                     # Run in foreground (don't daemonize)

floatctl hub daemon logs [OPTIONS]
  --follow                         # Tail logs (like tail -f)
  --lines <n>                      # Show last N lines (default: 50)
```

**Config file (~/.config/floatctl/hub-daemon.toml):**
```toml
[schedules]
# Cron-style scheduling
validate_metadata = { cron = "0 2 * * *", cmd = "metadata validate ~/float-hub/float.dispatch/bridges/ --report json" }
inbox_routing = { cron = "0 3 * * *", cmd = "route suggest ~/float-hub/inbox/ --output json" }
pattern_extraction = { cron = "0 4 * * 0", cmd = "patterns extract ~/float-hub/inbox/turtle-archaeology-* --min-occurrences 3" }

[queue]
poll_interval = 5  # seconds
max_concurrent = 2
log_retention_days = 30

[logging]
level = "info"
file = "~/.config/floatctl/logs/hub-daemon.log"
```

**Status output:**
```bash
floatctl hub daemon status

# Status: RUNNING
# PID: 12345
# Uptime: 3d 14h 23m
# Queue: 1 active, 0 pending
# Last metadata validation: 2025-11-15 02:00:01 (✓ success)
# Last inbox routing: 2025-11-15 03:00:04 (✓ success, 5 files)
# Config: ~/.config/floatctl/hub-daemon.toml
```

**When called:**
- **System startup:** Via launchd/systemd
- **Manual:** For debugging daemon issues

**Implementation notes:**
- Use `daemonize` crate for Unix daemon creation
- PID file: `~/.config/floatctl/hub-daemon.pid`
- Signal handling: SIGTERM for graceful shutdown, SIGHUP for config reload
- Queue processor: poll `~/.config/floatctl/hub-queue/*.json` every N seconds
- Scheduler: use `cron` crate for schedule parsing

---

## Implementation Priorities

### Phase 1: Pure Utility (No LLM, Immediate Value)

1. **floatctl hub metadata validate** - Catch missing frontmatter
2. **floatctl hub lint** - Fix formatting issues
3. **floatctl hub dispatch parse** - Understand :: usage patterns
4. **floatctl hub hook changelog-append** - Automate changelog updates

**Rationale:** All deterministic, immediate operational value, no dependencies.

### Phase 2: LLM-Assisted (Still Useful Without)

5. **floatctl hub route suggest** - Start with heuristics, add `--llm-model` later
6. **floatctl hub hook context-inject** - Deterministic dispatch parsing + optional evna query

**Rationale:** Heuristics handle 80% of cases, LLM enhances remaining 20%.

### Phase 3: Async/Background

7. **floatctl hub summarize** - Batch ollama processing
8. **floatctl hub patterns extract** - Turtle archaeology curation
9. **floatctl hub queue** - Queue management
10. **floatctl hub daemon** - Background task processor

**Rationale:** Requires queue infrastructure, but unblocks long-running operations.

---

## Composability Examples

**Find bridges with incomplete metadata → queue summarization:**
```bash
floatctl hub metadata validate float.dispatch/bridges/ --json | \
  jq -r '.issues[].file' | \
  xargs -I {} floatctl hub summarize {} --model llama3.2:3b --async
```

**Parse dispatches → generate routing suggestions:**
```bash
floatctl hub dispatch parse ~/.evans-notes/daily/ --since today --output json | \
  jq -r '.ctx | keys[]' | \
  xargs -I {} floatctl hub route suggest inbox/ --context {} --output json
```

**Validate → lint → route workflow:**
```bash
# Validate metadata
floatctl hub metadata validate inbox/ --fix

# Lint markdown
floatctl hub lint inbox/ --fix

# Route to destinations
floatctl hub route suggest inbox/ --auto-route --yes

# Log to changelog
floatctl hub hook changelog-append \
  --what "Processed and routed inbox files" \
  --why "Daily cleanup workflow" \
  --where "inbox/"
```

---

## Integration with Existing Infrastructure

### With floatctl bridge

```bash
# After routing files, index annotations
floatctl hub route suggest inbox/ --auto-route --yes
floatctl bridge index ~/float-hub/float.dispatch/imprints/ --recursive
```

### With floatctl embed-notes

```bash
# After routing bridges, embed them
floatctl hub route suggest inbox/*.bridge.md --auto-route --yes --destination bridges/
floatctl embed-notes ~/float-hub/float.dispatch/bridges/ --skip-existing
```

### With evna

```bash
# Generate context for evna brain_boot
floatctl hub dispatch parse ~/.evans-notes/daily/ --since 7d --output json > /tmp/recent-dispatches.json

# evna can read this for context injection
```

---

## Testing Strategy

### Unit Tests

**Per command:**
- Metadata validation: Test YAML parsing, field checking, date formats
- Dispatch parsing: Test regex extraction, grouping, counting
- Lint: Test each rule individually
- Queue: Test state persistence, status tracking

### Integration Tests

**End-to-end workflows:**
1. Create test inbox with sample files
2. Run `route suggest` → verify suggestions
3. Run `metadata validate` → verify detection
4. Run `summarize` → verify ollama integration
5. Run `queue add` → verify task creation

### Golden Fixtures

Store expected outputs in `tests/fixtures/`:
- `sample-bridges/` - Test bridge files with various frontmatter
- `sample-dispatches.md` - File with known :: markers
- `sample-routing-inbox/` - Files with known routing destinations

---

## Documentation to Create

1. **User Guide:** `docs/floatctl-hub-user-guide.md`
   - Quick start examples
   - Common workflows
   - Troubleshooting

2. **Hook Integration Guide:** `docs/floatctl-hub-hooks.md`
   - Claude Code hook setup
   - SessionStart examples
   - UserPromptSubmit patterns

3. **Configuration Reference:** `docs/floatctl-hub-config.md`
   - Routing rules format
   - Metadata rules format
   - Daemon config options

4. **Architecture Notes:** `docs/floatctl-hub-architecture.md`
   - Queue implementation
   - Daemon design
   - Security considerations

---

## Security Considerations

**Following existing patterns:**

1. **Use `execFile()` not shell** (like evna does for floatctl):
   ```rust
   // Good: floatctl calling ollama
   Command::new("ollama")
     .arg("generate")
     .arg("--model")
     .arg(model)  // Separate arguments, not shell-interpolated
   ```

2. **Validate file paths:**
   - Reject symlinks (like `floatctl script register`)
   - Check for `../` in paths
   - Validate file sizes

3. **Sandbox LLM calls:**
   - Timeout: 60s max
   - Max tokens: configurable, default reasonable
   - Rate limiting: configurable delay between calls

4. **Queue security:**
   - File permissions: 0600 for queue state files
   - No arbitrary code execution in queue (whitelist task types)
   - Validate task args before execution

---

## Open Questions

1. **Routing rules format:** TOML vs JSON vs embedded in code?
   - **Proposal:** TOML for readability, validation with serde

2. **Daemon vs cron:** Should `floatctl hub daemon` replace cron entirely?
   - **Proposal:** Daemon for queue + schedules, user choice for entry point

3. **Ollama required or optional?**
   - **Proposal:** Commands with `--llm-model` flag fail gracefully if ollama unavailable

4. **floatctl vs evna boundary:** Should some commands be evna tools instead?
   - **Proposal:** floatctl = boring deterministic, evna = orchestration/fusion
   - Example: `route suggest` heuristics in floatctl, but evna could call it + add reasoning

---

## Next Steps

1. **Review this proposal** - validate problem statements, CLI design
2. **Pick Phase 1 command** - implement `metadata validate` first (simplest, high value)
3. **Create spec document** - detailed implementation plan for first command
4. **Prototype** - build, test, refine
5. **Iterate** - add next command, repeat

**Document status:** PROPOSAL - ready for review and refinement.

# YAGNI Simplification Analysis: floatctl-rs

**Date**: 2026-01-10
**Total LOC analyzed**: ~24,700 lines
**Potential reduction**: ~2,500+ lines (10%+)

## Executive Summary

This codebase is well-structured but has accumulated complexity inappropriate for **single-user personal tooling**. Key issues:

1. **Massive code duplication** - Same patterns repeated 4-10x across files
2. **Over-engineered abstractions** - Enterprise patterns in personal tools
3. **Dead code** - Functions/fields marked `#[allow(dead_code)]` that should be deleted
4. **Interactive wizards** - 574+ lines of TUI when CLI flags suffice
5. **Defensive programming** - Validation for scenarios that can't occur

---

## Priority 1: High-Impact Quick Wins

### 1.1 Remove Dead Code (~100 lines)

| File | Lines | Issue |
|------|-------|-------|
| `sync.rs` | 669-711 | `check_dispatch_status()` - marked dead_code, never called |
| `sync.rs` | 740-757 | `get_log_modification_time()` - marked dead_code |
| `evna.rs` | 28-30 | `timed_out` field - marked dead_code, never read |
| `bbs.rs` | 374, 396, 419 | `persona` fields - deserialized but never accessed |
| `bbs.rs` | 440, 448 | `path`, `details` fields - never used |
| `bbs.rs` | 798-809 | `R2FileMatch` - 4 of 6 fields never used |

**Action**: Delete these. Dead code is liability, not insurance.

### 1.2 Extract Duplicated Path Resolution (~100 lines saved)

**evna.rs** - Evna binary path discovery appears **4 times** (lines 380-401, 668-689, 1188-1196, 1339-1348):

```rust
// BEFORE: 70 lines duplicated across 4 functions
let evna_path = if let Some(path) = args.path {
    path
} else {
    let home = dirs::home_dir().context("...")?;
    let candidates = vec![
        home.join("float-hub-operations").join("floatctl-rs").join("evna"),
        // ... more candidates
    ];
    candidates.into_iter().find(|p| p.exists()).ok_or_else(|| ...)?
};

// AFTER: Single helper
fn resolve_evna_path(explicit: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit { return Ok(path); }
    let home = dirs::home_dir().context("Could not determine home directory")?;
    for candidate in [
        home.join("float-hub-operations/floatctl-rs/evna"),
        home.join("float-hub-operations/evna"),
        home.join(".floatctl/evna"),
    ] {
        if candidate.exists() { return Ok(candidate); }
    }
    bail!("evna directory not found")
}
```

### 1.3 Remove Trivial Wrapper Functions (~20 lines)

**sync.rs:1010-1020** - Three wrappers that only call `trigger_via_float_box`:

```rust
// DELETE THESE:
fn trigger_daily_sync(wait: bool) -> Result<SyncResult> {
    trigger_via_float_box("daily", wait)  // Just call this directly
}
fn trigger_dispatch_sync(wait: bool) -> Result<SyncResult> { ... }
fn trigger_projects_sync(wait: bool) -> Result<SyncResult> { ... }
```

---

## Priority 2: Duplicate Pattern Extraction (~400 lines saved)

### 2.1 ASCII Tree Formatting (bbs.rs)

**10 locations** duplicate this pattern:
```rust
let is_last = i == collection.len() - 1;
let prefix = if is_last { "└─" } else { "├─" };
let cont_prefix = if is_last { "   " } else { "│  " };
println!("{} {}", prefix, title);
```

**Create helper**:
```rust
fn print_tree_item(index: usize, total: usize, title: &str, details: &[&str]) {
    let (prefix, cont) = if index == total - 1 { ("└─", "   ") } else { ("├─", "│  ") };
    println!("{} {}", prefix, title);
    for line in details { println!("{}{}", cont, line); }
}
```

### 2.2 Config Resolution Pattern (bbs.rs:671-708)

Two identical chains for endpoint/persona resolution:

```rust
// Extract to:
fn resolve_config<T: Clone>(
    arg: Option<T>,
    env_key: &str,
    config_path: impl Fn(&FloatConfig) -> Option<T>,
    error: &str,
) -> Result<T> {
    if let Some(v) = arg { return Ok(v); }
    if let Ok(v) = std::env::var(env_key) { return Ok(v.parse()?); }
    if let Ok(cfg) = FloatConfig::load() {
        if let Some(v) = config_path(&cfg) { return Ok(v); }
    }
    bail!(error)
}
```

### 2.3 Query Builder Duplication (embed/lib.rs:567-752)

186 lines of SQL construction with identical SELECT, filter, and ORDER BY patterns across 3 query modes:

```rust
// Extract common parts:
fn apply_common_filters(
    builder: &mut QueryBuilder,
    project: &Option<String>,
    days: &Option<i64>,
) {
    if let Some(p) = project {
        builder.push(" AND project = ").push_bind(p);
    }
    if let Some(d) = days {
        let cutoff = Utc::now() - Duration::days(*d);
        builder.push(" AND created_at >= ").push_bind(cutoff);
    }
}
```

### 2.4 Cognitive Tool Command Builders (evna.rs)

**5 functions** (lines 1029-1332) build CLI args identically:

```rust
// Create builder:
struct EvnaCommandBuilder {
    args: Vec<String>,
}

impl EvnaCommandBuilder {
    fn new(command: &str) -> Self {
        Self { args: vec![command.to_string()] }
    }
    fn add_opt(&mut self, flag: &str, value: Option<&str>) -> &mut Self {
        if let Some(v) = value {
            self.args.extend([format!("--{}", flag), v.to_string()]);
        }
        self
    }
    fn add_flag(&mut self, flag: &str, enabled: bool) -> &mut Self {
        if enabled { self.args.push(format!("--{}", flag)); }
        self
    }
    fn build(self) -> Vec<String> { self.args }
}
```

---

## Priority 3: Over-Engineered Features to Simplify

### 3.1 Delete the Wizard System (~574 lines)

**File**: `wizard.rs` (entire file)

**Problem**: Interactive TUI prompts for a CLI tool where the user already knows what they want.

**Evidence of YAGNI**:
- Defines `WizardFillable` trait but never implements it
- Duplicates validation that CLI already does
- Multi-line input with double-Enter detection when `cat | floatctl` is Unix-standard

**Solution**: Delete `wizard.rs`. Use sensible CLI defaults + environment variables:

```bash
# Instead of wizard:
export FLOATCTL_OUTPUT_DIR="$HOME/.floatctl/exports"
floatctl full-extract --in export.json  # uses default output dir
```

### 3.2 Simplify BBS `run_get()` Function (bbs.rs:1093-1398)

**305 lines** doing 6+ different things. Should be split:

```rust
// BEFORE: One giant function with inline search for inbox, memory, board, filesystem, R2

// AFTER:
async fn search_inbox(query: &str, persona: &str, ...) -> Vec<SearchMatch> { ... }
async fn search_memory(query: &str, persona: &str, ...) -> Vec<SearchMatch> { ... }
async fn search_board(query: &str, board: &str, ...) -> Vec<SearchMatch> { ... }

async fn run_get(...) {
    let mut matches = vec![];
    if search_types.contains(&Inbox) { matches.extend(search_inbox(...).await); }
    if search_types.contains(&Memory) { matches.extend(search_memory(...).await); }
    // ...
    display_matches(&matches);
}
```

### 3.3 Remove Board List Caching (main.rs:670-751)

**82 lines** for eager-fetching ALL boards to `/tmp` during wizard - completely unnecessary for single-user.

```rust
// DELETE: Phase 1 parallel board fetch with /tmp caching
// REPLACE WITH: Lazy load on selection, or just list names without content
```

### 3.4 Simplify Server Response Wrappers (bbs_api.rs)

**7 response structs** wrapping simple data. For single-user API:

```rust
// DELETE all these:
struct InboxListResponse { persona: String, messages: Vec<...>, total_unread: usize }
struct MemoryListResponse { ... }
struct BoardListResponse { ... }
// etc.

// INSTEAD: Return the data directly
async fn list_inbox(...) -> Json<Vec<InboxMessage>> { ... }
```

Also delete redundant `success: true` field - HTTP 200 already indicates success.

---

## Priority 4: Remove "Just In Case" Code

### 4.1 Unnecessary Validation

| Location | Issue | Fix |
|----------|-------|-----|
| embed/lib.rs:797-803 | API key empty check (already validated upstream) | Remove |
| embed/lib.rs:1068-1076 | Division by zero guard for values that can't be zero | Remove nested conditionals |
| embed/lib.rs:263-269 | Silent batch_size clamping | `bail!()` instead |
| bbs_api.rs:611-619 | Path security check for single-user | Remove |
| bbs_api.rs (5 places) | `.min(100)` rate limiting for one user | Remove caps |

### 4.2 Over-Defensive Error Handling

```rust
// embed/lib.rs:865-877 - Trust the OpenAI API contract
// BEFORE: Pre-allocate Vec<Option>, check indices, error on missing
// AFTER:
response.data.into_iter().map(|d| Vector::from(d.embedding)).collect()
```

### 4.3 Unnecessary Concurrency (embed/lib.rs:886-898)

```rust
// BEFORE: tokio::spawn for each DB insert with join_all
// AFTER: Simple sequential loop (DB pool is the bottleneck anyway)
for msg in batch.drain(..) {
    upsert_message(pool, &msg).await?;
}
```

---

## Priority 5: Incomplete Abstractions to Remove

### 5.1 QueryTable Enum (embed/lib.rs:243-248)

```rust
pub enum QueryTable {
    Messages,
    Notes,    // Only supports Semantic mode
    All,      // "Unified search not yet implemented"
}
```

**Solution**: Remove the enum. Have separate functions or a boolean flag:

```rust
pub async fn query_messages(args: QueryArgs) -> Result<()> { ... }
pub async fn query_notes(args: QueryArgs) -> Result<()> { ... }
```

### 5.2 SpecificDaemonType vs DaemonType (sync.rs:112-117)

Two enums with same variants. Use one:

```rust
// DELETE:
pub enum SpecificDaemonType { Daily, Dispatch, Projects }

// KEEP and use everywhere:
pub enum DaemonType { Daily, Dispatch, Projects, All }
```

---

## Summary by File

| File | Lines | Reducible | % |
|------|-------|-----------|---|
| bbs.rs | 1819 | ~550 | 30% |
| evna.rs | 1371 | ~500 | 36% |
| sync.rs | 1211 | ~150 | 12% |
| main.rs | 1116 | ~200 | 18% |
| wizard.rs | 574 | ~574 | 100% |
| embed/lib.rs | 1819 | ~200 | 11% |
| bbs_api.rs | 762 | ~200 | 26% |
| **Total** | **8,672** | **~2,374** | **27%** |

---

## Recommended Action Plan

### Phase 1: Quick Wins (1 hour)
1. Delete dead code (marked `#[allow(dead_code)]`)
2. Remove trivial wrapper functions in sync.rs
3. Remove unused struct fields

### Phase 2: Extract Helpers (2 hours)
1. Create `resolve_evna_path()` helper
2. Create `print_tree_item()` helper for bbs.rs
3. Create `resolve_config()` helper for config resolution

### Phase 3: Simplify Commands (4 hours)
1. Split `run_get()` into focused search functions
2. Consolidate command arg builders in evna.rs
3. Remove wizard.rs entirely

### Phase 4: Server Cleanup (2 hours)
1. Remove response wrapper structs
2. Consolidate duplicate handlers
3. Remove single-user validation overhead

---

## Final Assessment

**Total potential LOC reduction**: ~2,400 lines (27% of analyzed code)
**Complexity score**: HIGH
**Recommended action**: Proceed with simplifications in phases

The codebase works, but carries significant maintenance burden from:
- Copy-paste duplication
- Enterprise patterns in personal tooling
- Interactive features that don't match the workflow

Simplification will make the tool faster to modify, easier to debug, and more aligned with its single-user purpose.

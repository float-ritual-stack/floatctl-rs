# floatctl-rs Archaeological Categorization

**Generated**: 2025-11-21
**Search Scope**: Last 2 weeks (2025-11-08 to 2025-11-21)
**Files Analyzed**: 428 files found, 13 high-priority files read
**Sources**: ~/float-hub/float.dispatch (imprints, bridges), ~/.evans-notes/daily

---

## 🛠️ FIXES (Already Shipped)

### 1. PR #23 CodeRabbit Review Fixes
- **Source**: `bridges/floatctl-rs-issue-231232.md`
- **Date**: 2025-11-13
- **Status**: ✅ COMPLETE
- **Details**:
  - (1) eval security issue with safe parsing
  - (2) all .unwrap() calls replaced with proper error handling
  - (3) platform checks added
- **Commit**: Review cycle 2 complete
- **Priority**: N/A (shipped)

### 2. evna CLI Comprehensive Testing + Bug Fixes
- **Source**: `daily/2025-11-16.md`
- **Date**: 2025-11-16
- **Status**: ✅ COMPLETE
- **Details**: 5 bugs found and fixed:
  - sessions list (date parse)
  - agent mode (help only)
  - active stdin missing
  - dotenv noise
  - inconsistent output flags
- **Commits**: bbb5315 + 21a98ce
- **Testing**: 8/8 commands working (100% coverage)
- **Priority**: N/A (shipped)

### 3. Logging Consolidation
- **Source**: `imprints/sysops-log/2025-11-21-floatctl-logging-consolidation.md`
- **Date**: 2025-11-21
- **Status**: ✅ COMPLETE
- **Details**:
  - Consolidated logs under ~/.floatctl/logs/
  - Structured JSONL daemon logging
  - Heredoc support validation
- **Commit**: 73d5d22
- **Priority**: N/A (shipped)

---

## ✨ FEATURES (Already Shipped)

### 1. floatctl script enhancements (All 4 Phases)
- **Source**: `imprints/sysops-daydream/2025-11-automation-infrastructure/floatctl-script-enhancements-spec.md`
- **Date**: 2025-11-14
- **Status**: ✅ COMPLETE
- **Details**:
  - **Phase 1**: show/edit/--names-only commands
  - **Phase 2**: Doc block parsing, enhanced list output
  - **Phase 3**: Dynamic script name completion (zsh/bash/fish)
  - **Phase 4**: describe command, args parsing
- **Commit**: 5840167
- **Time**: ~60 minutes (estimated 70-85)
- **Priority**: N/A (shipped)

### 2. floatctl script unregister
- **Source**: `imprints/sysops-daydream/2025-11-automation-infrastructure/2025-11-15-floatctl-script-ux-improvements-handoff.md`
- **Date**: 2025-11-15
- **Status**: ✅ COMPLETE
- **Details**:
  - Added `unregister` command with --force flag
  - Discovered `register --force` already existed (but undocumented)
  - Covers script lifecycle management end-to-end
- **Commit**: 168aa93
- **Priority**: N/A (shipped)

### 3. floatctl ctx command + evna integration
- **Source**: `bridges/floatctl-ctx-architecture-2025-11-20.bridge.md`
- **Date**: 2025-11-20
- **Status**: ✅ COMPLETE
- **Details**:
  - Queues ctx:: messages locally
  - Syncs to float-box via SSH
  - Remote Ollama synthesis processing
  - Global install support (~/.bun/bin/evna wrapper)
- **Commit**: 065623b
- **Priority**: N/A (shipped)

---

## 🏗️ ARCHITECTURE IDEAS (Big Picture)

### 1. floatctl-hub ecosystem vision
- **Source**: `bridges/floatctl-hub-ecosystem-overview-2025-11-20.bridge.md`
- **Date**: 2025-11-20
- **Status**: 🔮 CONCEPTUAL
- **Details**:
  - Infrastructure layers diagram (Rust → TypeScript → Organization)
  - Proposed floatctl-hub crate
  - float-box SSH integration architecture
- **Priority**: LOW (architectural vision, no immediate need)
- **Dependencies**: None
- **Note**: Exploration fodder for future, not immediate build

### 2. floatctl-hub design principles
- **Source**: `bridges/floatctl-hub-design-principles-2025-11-20.bridge.md`
- **Date**: 2025-11-20
- **Status**: 📖 PHILOSOPHY
- **Details**:
  - FLOAT principles foundation (ritual stack, not productivity)
  - Persona-driven design (sysop/lf1m/evna)
  - "Shacks not cathedrals" philosophy
  - BBS/FidoNet/mIRC patterns
  - Store-and-forward routing
- **Priority**: LOW (guiding principles, not actionable implementation)
- **Dependencies**: None

### 3. FLOAT.dispatch architectural convergence
- **Source**: `bridges/2025-11-20-float-dispatch-architectural-convergence.bridge.md`
- **Date**: 2025-11-20
- **Status**: ⚠️ DIFFERENT PROJECT (float.dispatch, not floatctl-rs)
- **Details**:
  - Thoughts as executable entities
  - Redux for thoughts architecture
  - Semantic routing mechanisms
  - Fuzzy compiler patterns (util-er convergence)
- **Priority**: N/A (different codebase)
- **Note**: Convergence patterns with floatctl worth noting (ctx:: markers, annotation parsing, fuzzy compilation) but not directly applicable to floatctl-rs codebase

---

## 🔍 SYSTEM ASSESSMENT - Actionable Findings

**Source**: `imprints/sysops-daydream/2025-11-12-evna-float-system-assessment.md`
**Date**: 2025-11-12
**Method**: Multi-agent archaeology (3 specialized agents: bridge explorer, code reviewer, session analyst)
**Grade**: B+ for personal tool, C for team tool, F for public SaaS

### HIGH PRIORITY Issues

#### 1. Zero Test Coverage (F grade)
- **Status**: ❗ IDENTIFIED GAP
- **Details**:
  - Zero test files in `floatctl-rs/**/tests/`
  - 5 basic unit tests in main.rs (script validation only)
  - 1 integration test (embeds_roundtrip, requires Docker)
  - Custom JSON parser has no edge case coverage
  - **Risk**: High regression risk, refactoring dangerous without tests
- **Priority**: 🔴 HIGH (critical for collaboration)
- **Dependencies**: None (can start immediately)
- **Estimated Effort**: 8-16 hours
  - Stream parser edge cases
  - pgvector deduplication
  - ask_evna session management
- **Impact**: Regression risk drops from High → Medium (70% confidence in refactoring)
- **Recommended Tests**:
  ```rust
  // floatctl-core/tests/stream_tests.rs
  #[test] fn test_empty_array()
  #[test] fn test_malformed_json_recovery()
  #[test] fn test_giant_file_memory_usage()  // Criterion benchmark: peak memory <100MB for 1GB file
  ```

#### 2. Incomplete Features - Sync Daemon Skeleton (D grade)
- **Status**: ❗ IDENTIFIED GAP
- **Details**:
  - `sync.rs`: 4 TODO comments for start/stop/parsing logic
  - `embed.rs`: Notes/All table queries marked TODO
  - `append.rs`: Time-window deduplication marked TODO
  - CLI help advertises features that don't work
  - **Impact**: R2 sync daemon is skeleton code, features exist in CLI but don't work
- **Priority**: 🟡 MEDIUM (decide: complete or mark experimental)
- **Dependencies**: None
- **Estimated Effort**:
  - **Option A**: 4-6 hours (complete implementation)
  - **Option B**: 1 hour (mark as experimental in CLI help)
- **Trade-off**: Option A is complete, Option B is honest. Choose based on R2 sync usage frequency.

#### 3. Database Schema Gaps (C grade)
- **Status**: ❗ IDENTIFIED GAP
- **Details**:
  - No indexes on `ask_evna_sessions` (added Oct 31, table scans inevitable)
  - No indexes on `active_context_stream.timestamp`
  - No indexes on `messages.project`
  - Sessions accumulate indefinitely (no cleanup strategy)
  - No deduplication enforcement at schema level (composite key relies on app logic)
- **Priority**: 🟡 MEDIUM (scalability issue)
- **Dependencies**: None
- **Estimated Effort**: 1-2 hours
- **Impact**: Query latency O(N) → O(log N) at scale
- **Recommended Fix**:
  ```sql
  -- migrations/0004_add_performance_indexes.sql
  CREATE INDEX idx_sessions_last_used ON ask_evna_sessions(last_used);
  CREATE INDEX idx_active_context_timestamp ON active_context_stream(timestamp);
  CREATE INDEX idx_messages_project ON messages(project);
  ```

### MEDIUM PRIORITY Issues

#### 4. Async Orchestration Underutilized
- **Status**: ⚠️ BEHAVIORAL GAP (architecture ready, behavioral underutilization)
- **Details**:
  - **Architecture Status**: ✅ Ready (task queue pattern validated, peripheral vision working, timeout handling MCP-safe)
  - **Behavioral Status**: ⚠️ Underutilized (single-shot queries dominate, session ID reuse minimal, synchronous bias)
  - **Usage Patterns**: 10-22% evna usage in float work, 0-3% in rent work
  - **Gap**: UX friction (user must remember session_id, no automatic chirping, no queue visibility)
- **Priority**: 🟡 MEDIUM (UX improvement, not blocker)
- **Dependencies**: None
- **Estimated Effort**: 8-16 hours total
  - Automatic chirping: 4-6 hours (evna writes to active_context on task completion)
  - Queue visibility: 2-4 hours (`floatctl evna status` shows running tasks)
  - Progressive results: 2-6 hours (stream partial findings, refactor all-or-nothing pattern)
- **Impact**: Converts architecture from "ready but manual" to "automatic and invisible"

#### 5. Console Logging Pollution (C grade)
- **Status**: ⚠️ IDENTIFIED GAP (low priority, workarounds exist)
- **Details**:
  - 109 console.log/error/warn calls across 19 TypeScript files
  - MCP protocol fragility (stdout logs break JSON-RPC)
  - Workarounds exist: `RUST_LOG=off`, `windowsHide: true` (but fragile)
  - Debugging production issues impossible
- **Priority**: 🟢 LOW (workarounds functional)
- **Dependencies**: None
- **Estimated Effort**: 3-4 hours (structured logging with pino or minimal built-in)
- **Recommended Fix**:
  ```typescript
  // evna/src/lib/logger.ts
  import pino from 'pino';
  export const logger = pino({
    level: process.env.LOG_LEVEL || 'info',
    transport: { target: 'pino-pretty' }, // Dev only
  });
  ```

### LOW PRIORITY Issues

#### 6. Rent Work Coverage Gap (10x usage disparity)
- **Status**: ⚠️ BEHAVIORAL GAP (not technical)
- **Details**:
  - Float work: 10-22% evna usage (every 3-10 turns)
  - Rent work: 0-3% evna usage (every 30-50 turns or none)
  - Archaeological trail weak for paid work
  - Evidence: CLAUDE.md reminder suggests past nudge fatigue
- **Priority**: 🟢 LOW (behavioral, not technical)
- **Dependencies**: None
- **Estimated Effort**: Ongoing (behavioral change)
- **Experimental Approaches**:
  - Trigger on deliverables: Auto-capture after git commit, PR creation, issue close
  - Standup integration: Daily scrum → auto-query "what did I work on yesterday?"
  - Bridge promotion: If 3+ related commits, prompt "Create bridge for this work?"
- **Trade-off**: Risk annoying with prompts. Start with deliverable triggers (low friction).

---

## 🗑️ OUTDATED/SUPERSEDED

**Status**: ✅ None identified

All documentation from last 2 weeks appears current and valid. No stale bridges or superseded patterns found.

---

## 💡 SLASH COMMANDS (NOT FLOATCTL CORE)

### 1. /rangle:brain-boot
- **Source**: `imprints/sysops-daydream/2025-11-automation-infrastructure/2025-11-20-rangle-brain-boot-slash-command-built.md`
- **Date**: 2025-11-20
- **Status**: ✅ COMPLETE
- **Details**:
  - Proactive daily work status surfacing for rangle/pharmacy
  - Wraps floatctl + brain_boot MCP
  - Cognitive assist, not cognitive demand
  - Location: `/Users/evan/.claude/commands/rangle/brain-boot.md`
- **Note**: Demonstrates floatctl integration patterns but not a floatctl feature request
- **Pattern**: Multi-step workflow (GitHub + MCP + daily note update + evna capture)

### 2. /pharmacy:debug-db
- **Source**: `daily/2025-11-17.md`
- **Date**: 2025-11-17
- **Status**: ✅ COMPLETE
- **Details**:
  - Systematic database investigation workflow
  - Extracted from 493-turn debugging session (Issue #712)
  - 8-phase workflow: Reproduce → Snapshot → Logs → Investigation → Fix → Migration → Handoff → PR
  - Location: `~/projects/pharmacy-online/.claude/commands/pharmacy/debug-db.md`
- **Note**: Pharmacy-specific, not floatctl
- **Pattern**: "Codified ninja debugging" - extract expert workflow into reusable infrastructure

---

## 📊 SUMMARY STATS

**Total Findings**: 15 items
- **Fixes (Shipped)**: 3
- **Features (Shipped)**: 3
- **Architecture Ideas**: 3
- **Actionable Issues**: 6 (3 high, 2 medium, 1 low)
- **Outdated**: 0
- **Slash Commands (Non-Core)**: 2

**Critical Gaps**:
1. Zero test coverage (HIGH)
2. Sync daemon skeleton (MEDIUM)
3. Database indexes missing (MEDIUM)

**System Health**: B+ personal tool, strong architecture but uneven implementation maturity

---

## 🎯 NEXT STEPS

1. **Continue archaeological search**: Read main.rs to identify extraction opportunities (1800+ lines flagged)
2. **Survey Rust ecosystem**: clap patterns, modular CLI design, Agent SDK patterns
3. **Generate refactoring roadmap**: Immediate (<1 day), Enabling (1-3 days), Structural (ongoing)

---

**Categorization Complete**: 2025-11-21
**Method**: Archaeological grep → File reading → Structured categorization
**Next Phase**: Deep code review of floatctl-rs codebase

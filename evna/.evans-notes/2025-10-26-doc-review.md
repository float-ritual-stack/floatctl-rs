---
created: 2025-10-26 @ 03:35 PM
type: review
project: evna-next
purpose: Review yesterday's documentation burst - what to keep, what to archive
---

# Evna-Next Documentation Review (2025-10-26)

## Context
Yesterday (2025-10-26 @ 14:51), 18 markdown files were created in evna-next during hyper-focus session. Review to determine what's worth carrying forward vs what can be archived/consolidated.

---

## 📊 Files Created (Oct 26 14:51)

### ✅ KEEP - Core Documentation (Active Development)

**1. CLAUDE.md** (15K)
- **Purpose**: Development guidance for Claude Code instances
- **Status**: Active, being updated
- **Value**: Critical for agent coordination
- **Action**: KEEP, continue updating

**2. README.md** (8.9K)
- **Purpose**: Project overview, setup instructions
- **Status**: Active
- **Value**: Entry point for understanding evna-next
- **Action**: KEEP

**3. CHANGELOG.md** (9.6K)
- **Purpose**: Release history, feature tracking
- **Status**: Active
- **Value**: Historical record of development
- **Action**: KEEP

**4. ARCHITECTURE.md** (13K)
- **Purpose**: System architecture overview
- **Status**: Core documentation
- **Value**: High-level design reference
- **Action**: KEEP

---

### 🔄 CONSOLIDATE - Overlapping Specs/Designs

**5. TODO-SYNTHESIS.md** (18K)
- **Content**: Phase-by-phase implementation tracking (Cohere reranking, dual-source fix)
- **Status**: COMPLETED work (all checkboxes done)
- **Overlap**: Content now in CHANGELOG.md
- **Action**: **ARCHIVE** → Move to `.evans-notes/completed/` (historical reference only)

**6. BRAIN-BOOT-SYNTHESIS-UPGRADE.md** (21K)
- **Content**: Design plan for brain_boot improvements (Cohere, Claude SDK, dual-source)
- **Status**: Implementation COMPLETE (tracked in TODO-SYNTHESIS)
- **Overlap**: Duplicates ARCHITECTURE.md sections
- **Action**: **CONSOLIDATE** → Extract unique insights to ARCHITECTURE.md, archive original

**7. ACTIVE_CONTEXT_ARCHITECTURE.md** (13K)
- **Content**: Active context system design
- **Overlap**: Covered in ARCHITECTURE.md
- **Action**: **MERGE** → Consolidate with ARCHITECTURE.md, delete

**8. ACTIVE_CONTEXT_IMPLEMENTATION.md** (11K)
- **Content**: Implementation details for active_context
- **Status**: Implementation complete
- **Overlap**: Redundant with code + CHANGELOG
- **Action**: **ARCHIVE** → Move to `.evans-notes/specs/` (for archaeological reference)

**9. PHASE-2.3-SPEC.md** (4.1K)
- **Content**: Spec for Phase 2.3 (embedding at write time)
- **Status**: Completed phase
- **Overlap**: Tracked in TODO-SYNTHESIS + CHANGELOG
- **Action**: **ARCHIVE** → Move to `.evans-notes/specs/`

**10. HYBRID-CAPTURE-DESIGN.md** (17K)
- **Content**: Design for hybrid capture (verbatim + summary + annotations)
- **Status**: Draft design, not yet implemented
- **Value**: Future feature spec
- **Action**: **KEEP** → Move to `docs/specs/` (active future work)

**11. DUAL-SOURCE-REFINEMENTS.md** (13K)
- **Content**: Refinements to dual-source search
- **Status**: Implementation complete
- **Overlap**: Tracked in CHANGELOG
- **Action**: **ARCHIVE** → Historical reference only

---

### 🗑️ REMOVE - Temporary/Transient Content

**12. PR-DESCRIPTION.md** (11K)
- **Content**: Draft PR description
- **Status**: Transient (for specific PR, likely already submitted)
- **Action**: **DELETE** → One-time artifact, no ongoing value

**13. COMMIT-ANALYSIS.md** (7.5K)
- **Content**: Analysis of specific commits
- **Status**: Transient analysis
- **Action**: **DELETE** → Commit messages + CHANGELOG are sufficient

**14. CLAUDE_DESKTOP_SETUP.md** (3.1K)
- **Content**: Setup instructions for Claude Desktop
- **Status**: Likely specific to one-time setup
- **Overlap**: Should be in README.md if still relevant
- **Action**: **REVIEW** → If still relevant, merge into README, else delete

**15. evna-system-prompt.md** (3.7K)
- **Content**: System prompt for evna
- **Status**: Operational prompt (might be active)
- **Action**: **KEEP** → If in active use, else archive

---

### ⚠️ SPECIAL CASES

**16. CONTEXT-BOMB-MITIGATION.md** (7.1K)
- **Content**: Design plan for fixing MCP response size failures
- **Status**: SOLVED (deleted 73 messages >10K tokens from DB)
- **Value**: Documents the problem + solution
- **Action**: **KEEP** → Move to `docs/postmortems/` (valuable problem-solving record)

**17. TUI-IMPLEMENTATION.md** (8.1K)
- **Content**: TUI implementation plan
- **Status**: Future feature (not implemented)
- **Action**: **KEEP** → Move to `docs/specs/` (future work)

**18. 2206.01062v1.md** (1.6M!)
- **Content**: arXiv paper (likely research reference)
- **Status**: Reference material
- **Action**: **MOVE** → `docs/research/` or delete if no longer relevant
- **Note**: 1.6MB is HUGE for markdown, likely contains embedded content

---

## 📁 Recommended File Organization

```
evna-next/
├── README.md                           (keep)
├── CLAUDE.md                           (keep)
├── CHANGELOG.md                        (keep)
├── ARCHITECTURE.md                     (keep, consolidate others into)
├── evna-system-prompt.md               (keep if active)
│
├── docs/
│   ├── specs/                          (future features)
│   │   ├── HYBRID-CAPTURE-DESIGN.md   (keep - future work)
│   │   └── TUI-IMPLEMENTATION.md      (keep - future work)
│   │
│   ├── postmortems/                    (problems solved)
│   │   └── CONTEXT-BOMB-MITIGATION.md (keep - valuable record)
│   │
│   └── research/
│       └── 2206.01062v1.md            (optional - if still relevant)
│
└── .evans-notes/
    ├── completed/                      (historical tracking)
    │   ├── TODO-SYNTHESIS.md          (archive)
    │   └── BRAIN-BOOT-SYNTHESIS-UPGRADE.md (archive)
    │
    └── specs/                          (completed specs)
        ├── ACTIVE_CONTEXT_IMPLEMENTATION.md (archive)
        ├── PHASE-2.3-SPEC.md               (archive)
        └── DUAL-SOURCE-REFINEMENTS.md      (archive)
```

---

## 🎯 Recommended Actions (Priority Order)

### Immediate (5 min)
1. **DELETE** transient files:
   - `PR-DESCRIPTION.md` (no ongoing value)
   - `COMMIT-ANALYSIS.md` (no ongoing value)

### Short-term (15 min)
2. **CREATE** directory structure:
   ```bash
   mkdir -p docs/specs docs/postmortems docs/research
   mkdir -p .evans-notes/completed .evans-notes/specs
   ```

3. **MOVE** files to appropriate locations:
   - Specs → `docs/specs/`
   - Postmortems → `docs/postmortems/`
   - Completed tracking → `.evans-notes/completed/`
   - Completed specs → `.evans-notes/specs/`

### Medium-term (30 min)
4. **CONSOLIDATE** ARCHITECTURE.md:
   - Extract unique insights from BRAIN-BOOT-SYNTHESIS-UPGRADE.md
   - Merge ACTIVE_CONTEXT_ARCHITECTURE.md sections
   - Delete merged files

5. **REVIEW** CLAUDE_DESKTOP_SETUP.md:
   - If still relevant, merge into README
   - Else delete

### Optional (if needed)
6. **DECIDE** on 2206.01062v1.md (1.6MB):
   - Move to docs/research/ if actively referenced
   - Delete if not needed (can always recover from git)

---

## 🔍 Key Insights

**Documentation Pattern Observed:**
- Multiple overlapping specs created during rapid development
- Tracking documents (TODO-SYNTHESIS) served purpose during implementation
- Now that work is complete, CHANGELOG is sufficient
- Core docs (README, CLAUDE, ARCHITECTURE) remain valuable
- Spec documents valuable for future work, less so for completed work

**Sacred Profanity Preserved:**
None detected in documentation (professional tone throughout)

**Archaeology Value:**
- Completed specs = low ongoing value, high archaeological value
- Move to `.evans-notes/` preserves history without cluttering active docs
- Postmortems (CONTEXT-BOMB) = high ongoing value (lessons learned)

---

## 💡 Suggested Next Step

**Proposed bash script** to reorganize in one command:
```bash
# Create directories
mkdir -p docs/{specs,postmortems,research}
mkdir -p .evans-notes/{completed,specs}

# Delete transient files
rm PR-DESCRIPTION.md COMMIT-ANALYSIS.md

# Move to appropriate homes
mv HYBRID-CAPTURE-DESIGN.md docs/specs/
mv TUI-IMPLEMENTATION.md docs/specs/
mv CONTEXT-BOMB-MITIGATION.md docs/postmortems/
mv 2206.01062v1.md docs/research/  # optional

mv TODO-SYNTHESIS.md .evans-notes/completed/
mv BRAIN-BOOT-SYNTHESIS-UPGRADE.md .evans-notes/completed/
mv ACTIVE_CONTEXT_IMPLEMENTATION.md .evans-notes/specs/
mv PHASE-2.3-SPEC.md .evans-notes/specs/
mv DUAL-SOURCE-REFINEMENTS.md .evans-notes/specs/

# TODO: Manual consolidation
echo "MANUAL: Consolidate ACTIVE_CONTEXT_ARCHITECTURE.md into ARCHITECTURE.md"
echo "MANUAL: Review CLAUDE_DESKTOP_SETUP.md relevance"
```

**Time estimate**: 20-30 minutes for full reorganization

---

## 📍 Recommendation

**Carry Forward**:
- Core docs (README, CLAUDE, ARCHITECTURE, CHANGELOG)
- Future specs (HYBRID-CAPTURE, TUI-IMPLEMENTATION)
- Postmortems (CONTEXT-BOMB)
- System prompt (if active)

**Archive** (preserve but not clutter):
- Completed tracking docs
- Completed spec docs
- Overlapping architecture docs (after consolidation)

**Delete**:
- Transient artifacts (PR descriptions, commit analysis)
- Research paper (if not actively referenced)

**Result**: Clean documentation structure, preserved history, clear separation between active and archaeological content.

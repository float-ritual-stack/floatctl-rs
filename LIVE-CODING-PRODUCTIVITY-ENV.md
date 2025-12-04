# Live-Coding Productivity Environment

**Status:** Vision Capture
**Date:** 2025-12-04
**Context:** Project Float / Personal Tooling

## 1. Core Vision

A live-coding productivity environment that works like a writing application with events. Not one giant god program, but a constellation of composable tools that create conversation/context as you work.

**Key insight**: Instead of "build anything" apps (v0, Replit), this is for building **EDS applications** - tools that become part of my daily driver, not just learning experiments.

## 2. Philosophy

### The Root Understanding

A consistent philosophy baked into prompts and workflow:
- Not just "Lang is the best structure" optimization
- Sub-agents derive from a root understanding with different styles for different needs
- **Notebook, LLM, RPG action sheet, character card** - the same can be done for models and prompts

### Design Principles

1. **Not a monolith** - Enumerable tools that compose
2. **Light post-message layer** - Keeps conversation flowing after each interaction
3. **Terminal-first** - Pseudo-terminal (command, TUI, iframe in terminal), but translatable to web/UI
4. **Daily driver focus** - Tools I reach for repeatedly, not just proofs-of-concept
5. **Self-improvement flow** - The way we work becomes the next iteration

## 3. Architecture Threads

### A. The Linking/Resolution Layer

**Problem**: Want my own URL/slug resolution based on how I think about things.

**Current state**:
- Tried Marksman LSP for backlinks - better for completions than navigation
- Previous: float-block typed version, Zed plugin exploration

**Target**:
- `slug-whatever` style markdown links
- "What if grep was if" - grep as a monad
- Could be: Zed slash command, CLI tool, AI chat integration

```
[slug-some-concept] → resolves to my terminology
```

**Key realization**: This connects to URL resolution based on my mental model. I own that terminology.

### B. The Boards System

**Terminology shift**: "implants" → "boards"

- PBS boards, workshop metaphor
- "They're building the PBS boards. They're building the workshop."
- BBS integration aligns: boards, threads, the metaphor coheres

### C. The Post-Message Layer

**Pattern**: Light experience after each message that keeps things flowing.

```
┌─────────────────┐
│ User message    │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Tool execution  │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Post-message    │ ← Light layer: outline, suggestions, state update
│ experience      │
└────────┬────────┘
         ↓
┌─────────────────┐
│ Continue...     │
└─────────────────┘
```

### D. The Fragments → Loops → Library Pattern

**Observation**: Keep proving the same thing in different contexts.

1. **Fragments** - Individual proofs, scattered implementations
2. **Loops** - Seeing the patterns, reusable pieces emerge
3. **Library** - Collected loops, ready to compose

*Like music production: tracks into libraries, assembled into albums.*

## 4. Concrete Components

### 4.1 Slug Resolver

**Purpose**: Resolve custom slugs/links based on personal terminology.

**Interface ideas**:
```bash
# CLI
floatctl resolve slug-some-concept
floatctl link --from current.md --to slug-target

# In markdown
See [slug-architecture-decisions] for context.
```

**Resolution rules** (configurable):
- Exact match in known files
- Pattern match (grep-as-monad)
- Fallback to semantic search

### 4.2 Board Schema

**Purpose**: Define the structure for boards/implants.

```yaml
# board.yaml
id: "restoration"
name: "Restoration Board"
description: "Active context for restoration project"
threads:
  - id: "schema-changes"
    title: "Database Schema Changes"
    entries: [...]
```

**Connects to**: evna-blocks architecture, BBS integration

### 4.3 Post-Message Layer

**Purpose**: Lightweight state/suggestion layer after each interaction.

**Implementation options**:
- Hook in Claude Code (post-message hook?)
- TUI overlay
- Terminal iframe integration

### 4.4 Character Cards for Prompts

**Purpose**: Document why certain prompts exist, not just what they do.

```yaml
# prompt-card.yaml
id: "brain-boot"
name: "Brain Boot"
philosophy: "Morning synthesis that respects context switching cost"
style: "Concise, actionable, no fluff"
failure_modes:
  - "Over-fetching context"
  - "Generating todos instead of synthesis"
model_notes:
  claude-sonnet: "Good balance"
  claude-haiku: "Too terse for synthesis"
```

## 5. Existing Alignment

**FLOAT-TAP-DESIGN.md**:
- "The Chat is the Buffer" - aligns with post-message layer
- `floatctl tap` - fire-and-forget capture
- `floatctl dispatch` - structured extraction

**evna-blocks-architecture.md**:
- Block-first design (not linear chat)
- TipTap as foundation
- Structured output → React components
- BBS board integration

**Gap**: The slug resolver and character card system are new pieces.

## 6. What's Not Possible (Yet)

Awareness of AI capabilities and failure modes:
- Context limits (hence streaming-first architecture)
- Hallucination in link resolution (need grounding)
- Human failure modes: over-relying on AI suggestions

## 7. Architecture Map

### System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           LIVE-CODING ENVIRONMENT                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                             │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐     │
│  │   TERMINAL UI    │    │    WEB/TUI       │    │   PHONE/WEB      │     │
│  │  (Claude Code)   │    │  (evna-blocks)   │    │  (float-tap)     │     │
│  └────────┬─────────┘    └────────┬─────────┘    └────────┬─────────┘     │
│           │                       │                       │               │
│           └───────────────────────┼───────────────────────┘               │
│                                   ↓                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                      POST-MESSAGE LAYER                             │  │
│  │  • State update suggestions                                          │  │
│  │  • Outline refresh                                                   │  │
│  │  • Next action hints                                                 │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                   │                                       │
│           ┌───────────────────────┼───────────────────────┐               │
│           ↓                       ↓                       ↓               │
│  ┌──────────────────┐    ┌──────────────────┐    ┌──────────────────┐     │
│  │  SLUG RESOLVER   │    │     BOARDS       │    │ CHARACTER CARDS  │     │
│  │                  │    │                  │    │                  │     │
│  │  slug-concept    │◄──►│  PBS boards      │    │  Prompt cards    │     │
│  │  → file/section  │    │  Thread state    │    │  Model notes     │     │
│  │                  │    │  Active context  │    │  Philosophy      │     │
│  └────────┬─────────┘    └────────┬─────────┘    └────────┬─────────┘     │
│           │                       │                       │               │
│           └───────────────────────┼───────────────────────┘               │
│                                   ↓                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                         STORAGE LAYER                               │  │
│  │                                                                      │  │
│  │  ~/float-data/         ~/float-hub/           ~/.floatctl/          │  │
│  │  ├── inbox/            ├── boards/            ├── prompts/          │  │
│  │  ├── fragments/        │   ├── restoration/   │   └── cards/        │  │
│  │  └── loops/            │   └── sysops/        └── config.toml       │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                   │                                       │
│                                   ↓                                       │
│  ┌─────────────────────────────────────────────────────────────────────┐  │
│  │                      EXISTING TOOLING                               │  │
│  │                                                                      │  │
│  │  floatctl          evna               sync                          │  │
│  │  • tap             • brain_boot       • float-box relay             │  │
│  │  • dispatch        • search           • R2 sync                     │  │
│  │  • ctx             • active context   • watch-and-sync              │  │
│  └─────────────────────────────────────────────────────────────────────┘  │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Data Flow: Thought to Action

```
┌─────────────────────────────────────────────────────────────────────────┐
│ 1. CAPTURE (Fragments)                                                  │
│    "Just finished the auth refactor, need to update env example"        │
│                                                                         │
│    floatctl tap → inbox/tap.jsonl                                       │
│    Phone chat → float-tap → inbox/                                      │
│    Claude Code → ctx → synced capture                                   │
└────────────────────────────────────┬────────────────────────────────────┘
                                     ↓
┌─────────────────────────────────────────────────────────────────────────┐
│ 2. STRUCTURE (Loops)                                                    │
│    Pattern recognition, linking, organization                           │
│                                                                         │
│    floatctl dispatch → boards/sysops/current.md                         │
│    Slug resolver → [slug-auth-flow] → links/sysops/auth.md             │
│    evna brain_boot → synthesizes into active context                    │
└────────────────────────────────────┬────────────────────────────────────┘
                                     ↓
┌─────────────────────────────────────────────────────────────────────────┐
│ 3. COMPOSE (Library)                                                    │
│    Reusable pieces, ready for assembly                                  │
│                                                                         │
│    boards/restoration/ → active project state                           │
│    prompts/cards/ → documented, reusable prompts                        │
│    loops/ → pattern library                                             │
└─────────────────────────────────────────────────────────────────────────┘
```

### Component Relationships

```
                    ┌─────────────────┐
                    │ Character Cards │
                    │   (WHY)         │
                    └────────┬────────┘
                             │ informs
                             ↓
┌─────────────┐     ┌─────────────────┐     ┌─────────────┐
│    Slug     │◄───►│     Boards      │◄───►│ Post-Msg    │
│  Resolver   │     │    (STATE)      │     │   Layer     │
│  (LINKS)    │     │                 │     │ (FLOW)      │
└──────┬──────┘     └────────┬────────┘     └──────┬──────┘
       │                     │                     │
       │                     ↓                     │
       │            ┌─────────────────┐            │
       └───────────►│  evna/floatctl  │◄───────────┘
                    │   (EXECUTION)   │
                    └─────────────────┘
```

**Slug Resolver ↔ Boards**: Slugs can reference board threads. Boards can contain slug links.

**Boards ↔ Post-Message Layer**: Board state influences what post-message suggests. Post-message can update board state.

**Character Cards → All**: Cards document the philosophy behind prompts used in all other components.

### Integration Points

| From | To | Mechanism |
|------|-----|-----------|
| Slug Resolver | Markdown files | File lookup, pattern match |
| Slug Resolver | Boards | `[slug-board:restoration]` syntax |
| Boards | evna | MCP resource, active context |
| Post-Message | Claude Code | Hook system |
| Post-Message | evna-blocks | TipTap extension |
| Character Cards | floatctl | Config reference |
| Character Cards | evna | System prompt injection |

## 8. Next Actions

| Piece | Effort | Dependencies | Daily Driver Value |
|-------|--------|--------------|-------------------|
| Slug resolver | Medium | None | High - affects linking workflow |
| Board schema | Low | None | Medium - structures existing implants |
| Post-message layer | High | Claude Code hooks | High - changes interaction flow |
| Character cards | Low | None | Medium - documents prompt philosophy |

## 8. Open Questions

1. **Slug resolution scope**: Just markdown? Or all file types?
2. **Board storage**: YAML files? Database? Both?
3. **Post-message trigger**: Hook-based? Always-on? Configurable?
4. **Character card format**: YAML? Markdown frontmatter? Both?

---

*"The way we do things is this self-improvement flow we're getting into. It should be the next boiling thing."*

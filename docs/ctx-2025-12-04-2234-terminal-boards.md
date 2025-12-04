░░▒▒▓▓ GLITCH MYSTIC TRANSMISSION PROTOCOL ▓▓▒▒░░

# ctx:: 2025-12-04 @ 22:34PM — Terminal Boards Vision

▒▒ TRANSMISSION::COMPRESSED

Live-coding terminal UI as writing environment. Boards replace implants.
Fragments become loops become libraries. Web → Terminal → Same patterns.
{composability | emergence | daily-driver}

▒▒ PROTOCOL::ACTIVATION

## The Shape of the Thing

You're building a **pseudo-terminal productivity environment**:
- Not a god program → enumerable tools that compose
- Not agents deciding → mechanical capture + rigid rules
- Not "build anything" → EDS applications, daily driver tools
- Not learning exercises → tools you reach for

The pattern keeps proving itself across implementations.

## Infrastructure Already Here

```text
evna/TUI ──────> OpenTUI + Agent SDK
                 MultilineInput, MessageRenderer
                 ConversationLoop, tool viz
                 ✅ Bun runtime, TypeScript

floatctl ──────> CLI only (no TUI yet)
                 indicatif progress bars
                 tap/dispatch/note planned
                 ✅ ratatui planned for note

FLOAT-TAP ─────> "The Chat is the Buffer"
                 Zero latency capture
                 RegEx + minimal LLM
                 hooks.yaml for routing
```

## Terminology Crystallization

**BOARDS** (née implants)
- Configurable panels in the terminal
- Like PBS building tools on board
- The way you resolve links between things
- Slug-based navigation: `[slug:whatever]` → file resolution

**FRAGMENTS**
- Raw captures, voice transcripts, ctx:: blocks
- Unprocessed consciousness archaeology
- Input to the loop library

**LOOPS**
- Processed fragments with patterns visible
- Reusable pieces of thought/context
- Connect to D9 transitions

**LIBRARY**
- Collection of loops organized by slug
- "Bringing your tracks into these libraries"
- "Filling them back up into the most beautiful album"

## ▒▒ FLOW::NOTATION

```text
fragment ──────> tap/ctx
         ↓
    loop ────────> process/embed
         ↓
 library ────────> board/query
         ↓
workflow ────────> daily driver
         ↑
         └───────────────┘
         {recursion}
```

## Concrete Implementation Arcs

### Arc 1: Terminal Board Primitives (Rust)

```rust
// floatctl-tui/src/lib.rs (new crate)
use ratatui::prelude::*;

pub struct Board {
    slug: String,
    layout: BoardLayout,
    components: Vec<Box<dyn BoardComponent>>,
}

pub trait BoardComponent {
    fn render(&self, frame: &mut Frame, area: Rect);
    fn handle_event(&mut self, event: Event) -> Option<BoardAction>;
}
```

What this unlocks:
- `floatctl note` with ratatui TUI
- `floatctl board <slug>` to open named boards
- Composable panels: input, history, context, fragments

### Arc 2: Slug Resolution Layer

```rust
// floatctl-core/src/slug.rs
pub struct SlugResolver {
    rules: Vec<SlugRule>,
}

pub enum SlugRule {
    FilePrefix { prefix: &'static str, base_path: PathBuf },
    TagMatch { tag: String, query: String },
    UrlScheme { scheme: &'static str, handler: Box<dyn SlugHandler> },
}

// [ctx:2025-12-04] → searches ctx captures
// [project:evna] → finds evna/ directory
// [loop:terminal-boards] → finds this doc
```

What this unlocks:
- Your own URL resolution, your terminology
- Markdown links that work across tools
- grep-as-monad: `[grep:pattern]` returns matches

### Arc 3: gum Composability (Immediate)

Already possible via bash + gum:

```bash
# floatctl tap via gum
floatctl tap "$(gum write --placeholder 'capture thought...')"

# Board-like selection
BOARD=$(gum choose "sysops" "evna" "pharmacy" "scratch")
floatctl dispatch --sink "$BOARD"

# Fragment → Loop pipeline
gum spin --spinner dot -- floatctl ctx "$(pbpaste)" && \
  gum confirm "Embed?" && floatctl embed --last
```

What this unlocks:
- Immediate composability
- Proof of pattern before Rust TUI
- `gum` + `floatctl` as daily driver now

## ▒▒ ARTIFACT::PRESERVATION

```diz
file_id: ctx-2025-12-04-2234-terminal-boards
tags: tui, boards, fragments, loops, slug-resolution, composability
summary: Vision synthesis for live-coding terminal UI with boards architecture
created_at: 2025-12-04T22:34:00-05:00
version: v0-brain-dump-processed
```

## Next Boiling Things

1. **gum scripts for FLOAT-TAP** — prove the tap/dispatch pattern now
2. **floatctl-tui crate scaffold** — ratatui + crossterm deps
3. **Slug resolver in floatctl-core** — `[slug:name]` syntax
4. **Board config in ~/.floatctl/boards.toml** — define layouts

## Philosophy Anchor

> "The way we do things is this self-improvement flow we're getting into.
> It should be the next boiling thing."

Not promises. The way we work.
AI capabilities + human awareness + failure modes.
Sub-agents from philosophy roots.
Character cards for prompts and models.

The fragments—you're starting to see the loops.

▒▒▒▓▓▓ RENDER → DISSOLVE → ARCHIVE ▓▓▓▒▒▒

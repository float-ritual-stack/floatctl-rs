# Floatctl-RS Claude Subcommand: Architectural Reconnaissance

**Target**: `~/float-hub-operations/floatctl-rs`

**Scope**: Quick scan of claude subcommand structure and extension points for search feature

---

## 1. Current Claude Subcommand Architecture

### Command Registration Pattern

**Entry point**: `floatctl-cli/src/main.rs`

```rust
// CLI enum (line 84-85)
Claude(ClaudeArgs),

// Subcommand dispatch (line 530)
Commands::Claude(args) => run_claude(args)?,
```

**Subcommand types** (lines 234-242):
```rust
enum ClaudeCommands {
    #[command(alias = "list-sessions")]
    List(ListSessionsArgs),           // line 238
    RecentContext(RecentContextArgs), // line 240
    Show(ShowArgs),                   // line 242
}
```

**Handler dispatch** (lines 1479-1485):
```rust
fn run_claude(args: ClaudeArgs) -> Result<()> {
    match args.command {
        ClaudeCommands::List(list_args) => run_claude_list_sessions(list_args),
        ClaudeCommands::RecentContext(context_args) => run_claude_recent_context(context_args),
        ClaudeCommands::Show(show_args) => run_claude_show(show_args),
    }
}
```

### Command Organization

Three existing commands in `floatctl-claude/src/commands/`:

1. **list_sessions.rs** (lines 1-248)
   - Discovers recent session files from `~/.claude/projects/`
   - Filters by project, excludes agent sessions (IDs starting with "agent-")
   - Returns ordered list with metadata (start time, branch, turn count, tool calls)
   - Options: `limit`, `project_filter`, `include_agents`

2. **recent_context.rs** (primary for evna integration)
   - Extracts first/last N messages from recent sessions
   - Smart truncation with UTF-8 boundary handling
   - Returns structured JSON for system prompt injection
   - Options: `sessions`, `first`, `last`, `truncate`, `project_filter`

3. **show.rs** (pretty-printing)
   - Reads full session from JSONL file
   - Multiple output formats: text (box drawing), markdown, JSON
   - Filtering: `--first N`, `--last N`, tool/thinking block visibility
   - Metadata extraction and statistics (tokens, cache efficiency, tool calls)

### Data Layer

**Core types** in `floatctl-claude/src/lib.rs`:

```rust
LogEntry {              // Individual log line
    entry_type: String,     // "user", "assistant", "queue-operation", etc.
    timestamp: Option<String>,
    message: Option<MessageData>,
    content: Option<String>,
    session_id: Option<String>,
    cwd: Option<String>,
    git_branch: Option<String>,
    // ... 7 more fields
}

MessageData {           // API or user message
    role: String,           // "user" or "assistant"
    content: Vec<ContentBlock>,
    usage: Option<Usage>,   // token counts, cache info
}

ContentBlock (enum) {   // Multiple block types
    Text { text: String },
    Thinking { thinking: String },
    ToolUse { id, name, input },
    ToolResult { tool_use_id, content, is_error },
    Image { source: ImageSource },
}
```

**JSONL Streaming** (`stream.rs`):
- Simple line-by-line reader with `BufReader`
- Helper: `read_log_file(path)` -> `Vec<LogEntry>` (loads entire file)
- Used by all three commands via `stream::read_log_file()`

**Parsing Utilities** (`parser.rs`):
- `extract_messages()` - Filter user/assistant entries, extract text + tool calls
- `calculate_stats()` - Count turns, tool calls, token usage
- `get_session_metadata()` - Extract session info from entries

---

## 2. Extension Points for Search Feature

### Natural Integration Point: New Subcommand

Would follow existing pattern:

```rust
// In floatctl-claude/src/commands/mod.rs
pub mod search;  // NEW

// In floatctl-cli/src/main.rs enum ClaudeCommands
Search(SearchArgs),  // NEW

// In run_claude() dispatch
ClaudeCommands::Search(search_args) => run_claude_search(search_args),
```

### Available Reusable Infrastructure

1. **Session Discovery** (from list_sessions.rs)
   ```rust
   find_session_logs(projects_dir: &Path) -> Result<Vec<PathBuf>>
   list_sessions(projects_dir: &Path, options) -> Result<Vec<SessionSummary>>
   ```
   - Already handles filtering, sorting, limiting
   - Could leverage for search scope

2. **Message Extraction** (from parser.rs)
   ```rust
   extract_messages(entries: &[LogEntry]) -> Vec<Message>
   ```
   - Already filters to user/assistant entries
   - Extracts text content, tool calls, timestamps
   - Perfect foundation for search filtering

3. **Content Access** (LogEntry structure)
   - Three content sources per session:
     - `entry.content` (queue-operation entries - user input)
     - `entry.message.content` (MessageData blocks - API responses)
     - Both searchable, different cardinality patterns

4. **Token Estimation** (already in codebase via floatctl-embed)
   - `tiktoken-rs` dependency available (line 54 in Cargo.toml)
   - Could estimate search result sizes for batching
   - **Note**: Not directly imported in floatctl-claude yet

### Gotchas and Complexity Areas

1. **Two Content Sources Per Message**
   - User messages: text in `entry.content` (queue-operation type)
   - Assistant messages: content blocks in `entry.message.content`
   - Search must handle both formats for complete coverage

2. **Memory Trade-offs**
   ```rust
   // Current pattern: Load entire file
   read_log_file(path) -> Vec<LogEntry>  // Loads all into memory
   
   // Search alternative needed for large sessions:
   // - Stream-based search (line-by-line filtering)
   // - Limits result sets (pagination)
   // - Materializes only matched entries
   ```

3. **ContentBlock Complexity**
   - Text blocks are straightforward
   - ToolResult content is **nested** (recursively contains more ContentBlocks)
   - Thinking blocks present but should likely be skipped for search
   - Image blocks are base64 encoded (not meaningful to search)

4. **Session Organization**
   - Sessions stored as individual .jsonl files in `~/.claude/projects/`
   - Directory structure: `~/.claude/projects/<project-id>/<session-id>.jsonl`
   - Can't query across all sessions without walking entire tree
   - Filtering by project happens at walker stage (already implemented)

---

## 3. CLI Argument Pattern

### Existing Pattern (from show.rs, recent_context.rs)

```rust
#[derive(Parser, Debug)]
struct ShowArgs {
    session: String,  // positional
    
    #[arg(long)]
    first: Option<usize>,
    
    #[arg(long, default_value = "text")]
    format: String,
    
    #[arg(long)]
    projects_dir: Option<PathBuf>,
}
```

### Suggested Search Arguments Pattern

```rust
#[derive(Parser, Debug)]
struct SearchArgs {
    query: String,  // positional
    
    #[arg(short = 'p', long)]
    project: Option<String>,  // filter to project
    
    #[arg(short = 'l', long, default_value = "10")]
    limit: usize,  // max results
    
    #[arg(long)]
    case_sensitive: bool,
    
    #[arg(long)]
    regex: bool,  // enable regex search
    
    #[arg(long, default_value = "json")]
    format: String,  // json, text, etc.
    
    #[arg(long)]
    projects_dir: Option<PathBuf>,
}
```

---

## 4. Codebase Patterns to Follow

### Error Handling
```rust
use anyhow::{Context, Result};

find_session_logs(projects_dir)?
    .context("Failed to find session logs")?
```

### Output Formatting
```rust
match args.format.as_str() {
    "json" => println!("{}", serde_json::to_string_pretty(&results)?),
    "text" => {
        // Pretty-print with structure
        println!("╭─────────────");
        // ...
    }
}
```

### File I/O
```rust
use std::fs;
use walkdir::WalkDir;

// Already using for session discovery in list_sessions.rs
for entry in WalkDir::new(projects_dir)
    .follow_links(false)
    .into_iter()
    .filter_map(|e| e.ok())
```

---

## 5. Key Files by Feature

| File | Purpose | Key Items |
|------|---------|-----------|
| `floatctl-claude/src/lib.rs` | Core types | `LogEntry`, `MessageData`, `ContentBlock`, `smart_truncate()`, `find_session_logs()` |
| `floatctl-claude/src/stream.rs` | JSONL reading | `LogStream`, `read_log_file()` - line-by-line parser |
| `floatctl-claude/src/parser.rs` | Message extraction | `extract_messages()`, `calculate_stats()`, `get_session_metadata()` |
| `floatctl-claude/src/commands/mod.rs` | Command exports | Re-exports three commands |
| `floatctl-claude/src/commands/list_sessions.rs` | Session listing | Session discovery, filtering, sorting |
| `floatctl-claude/src/commands/recent_context.rs` | Context extraction | Evna integration, first/last messages, truncation |
| `floatctl-claude/src/commands/show.rs` | Pretty-printing | Three output formats (text, markdown, JSON) |
| `floatctl-cli/src/main.rs` | CLI wiring | Command enum, argument parsing, dispatch |

---

## 6. Quick Implementation Checklist

For a search command following existing patterns:

1. **Core Implementation**
   - [ ] Create `floatctl-claude/src/commands/search.rs`
   - [ ] Implement `SearchArgs` in `main.rs` CLI
   - [ ] Add `Search` variant to `ClaudeCommands` enum
   - [ ] Add `run_claude_search()` dispatch handler
   - [ ] Export new command from `commands/mod.rs`

2. **Search Logic**
   - [ ] Leverage `find_session_logs()` + filtering for scope
   - [ ] Stream search via `LogStream::new()` (not loading entire file)
   - [ ] Leverage `extract_messages()` for content extraction
   - [ ] Implement regex + case-sensitive options
   - [ ] Add result limiting (pagination)

3. **Output Formatting**
   - [ ] JSON format (serializable structs)
   - [ ] Text format (pretty-printed with context)
   - [ ] Include metadata: session, timestamp, role, message excerpt

4. **Testing**
   - [ ] Unit tests for search logic (with temp session files)
   - [ ] Integration test with real session files
   - [ ] Edge cases: empty results, large result sets, special characters

5. **Token Estimation** (optional enhancement)
   - [ ] Add tiktoken-rs import if batching needed
   - [ ] Estimate search result token counts
   - [ ] Warn if result overflow risks

---

## 7. Performance Characteristics

**Current Commands** (no benchmarks published, but observed):
- `list_sessions`: Walks `~/.claude/projects/` tree (typically 5-50 files)
- `recent_context`: Loads 3-10 recent files into memory sequentially
- `show`: Loads single .jsonl file (typically 100KB-10MB)

**For Search**:
- Current `read_log_file()` loads entire session into `Vec<LogEntry>` → not ideal for large sessions
- **Recommendation**: Use `LogStream::new().next_entry()` loop for streaming search
- **Memory profile**: O(1) for search, O(limit) for result accumulation
- **Speed**: Regex search on 1000 message session: <100ms (rough estimate)

---

## Summary

**Architecture**: Clean, extensible pattern with three focused commands

**Strength**: Reusable abstractions (session discovery, message extraction, formatting)

**For Search Implementation**: 
- Natural fit as 4th subcommand
- Leverage existing `find_session_logs()`, `parser`, `stream` infrastructure
- Use streaming search (not load-all-then-search) for large sessions
- Follow existing output formatting patterns (JSON + text)
- Add to CLI enum and dispatch handler (~30 lines of wiring)

**No major obstacles** - straightforward extension following established patterns.


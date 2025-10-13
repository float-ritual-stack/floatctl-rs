# Architecture

## Overview

`floatctl` is designed as a streaming-first, memory-efficient conversation archive processor. The architecture prioritizes O(1) memory usage even for multi-GB files through custom streaming JSON parsing.

## Design Principles

1. **Stream First**: Never load entire files into memory
2. **Zero-Copy When Possible**: Use `std::mem::take()` to move data instead of cloning
3. **Parse Once**: Avoid unnecessary serialization/deserialization round-trips
4. **Progress Transparency**: Always show progress for long-running operations
5. **Format Agnostic**: Handle both ChatGPT and Anthropic exports transparently

## Core Components

### Streaming Layer (`stream.rs`)

The foundation of the system's performance.

#### `JsonArrayStream`
**Purpose**: Stream individual elements from a JSON array without loading the entire array.

**Problem Solved**: `serde_json::StreamDeserializer` treats a JSON array `[...]` as a single top-level value. For a 772MB file with one array, this means loading 772MB before yielding anything.

**Solution**: Manual parsing of array structure:
1. Peek at first byte to detect `[`
2. Read opening bracket
3. For each element:
   - Skip whitespace
   - Check for `,` or `]`
   - Use `serde_json::Deserializer::from_reader()` for individual element
   - Yield element immediately (O(1) memory)

**Key Code** (floatctl-core/src/stream.rs:33-112):
```rust
pub struct JsonArrayStream {
    reader: BufReader<File>,
    started: bool,
    finished: bool,
}

impl JsonArrayStream {
    fn next_element(&mut self) -> Result<Option<Value>> {
        // Manual [, comma, ] parsing
        // Uses Deserializer::from_reader() per element
    }
}
```

#### `RawValueStream` vs `ConvStream`
- **`RawValueStream`**: Returns raw `serde_json::Value` - used when you don't need parsed `Conversation` structs (e.g., `ndjson` command)
- **`ConvStream`**: Parses into `Conversation` structs - used when you need metadata and structured access

Both support:
- JSON arrays (`[{...}, {...}]`)
- NDJSON (one JSON object per line)
- Auto-detection based on first non-whitespace byte

### Conversation Model (`conversation.rs`)

#### Dual-Format Support

**Challenge**: ChatGPT and Anthropic/Claude use different export schemas.

| Field | ChatGPT | Anthropic |
|-------|---------|-----------|
| Messages | `messages` | `chat_messages` |
| Role | `role` | `sender` |
| ID | `id` | `uuid` |
| Text | `content` | `text` |
| Timestamp | `timestamp` | `created_at` |

**Solution**: Check for both formats at parse time:
```rust
let msgs = if let Some(m) = value_mut.get_mut("messages") {
    // ChatGPT
} else if let Some(m) = value_mut.get_mut("chat_messages") {
    // Anthropic
} else {
    Vec::new()
};
```

#### Raw JSON Preservation

**Challenge**: After parsing messages with `std::mem::take()`, the array is empty in the original JSON.

**Solution**: Clone the raw JSON *before* any mutations:
```rust
pub fn from_export(value: Value) -> Result<Self> {
    let raw = value.clone();  // Preserve original
    let mut value_mut = value;
    // ... mutate value_mut for parsing ...
    Ok(Self { meta, messages, raw })  // Use preserved clone
}
```

This allows the JSON output to contain the full, unmodified conversation.

### Output Pipeline (`pipeline.rs`)

#### Slug Generation

**Requirements**:
- Date-prefixed: `YYYY-MM-DD-title`
- Filesystem-safe characters
- Strip existing date prefixes from titles
- Limit length to 100 chars

**Implementation** (floatctl-core/src/pipeline.rs:40-63):
```rust
fn generate_slug(conv: &Conversation) -> String {
    let date_str = format!("{:04}-{:02}-{:02}", ...);
    let title_without_date = strip_leading_date(title);
    let slug = slugify(title_without_date);
    format!("{}-{}", date_str, slug)
}
```

#### Artifact Extraction

**Detection**: Look for `tool_use` blocks with `name: "artifacts"` in message `content` arrays.

**Type Mapping** (floatctl-core/src/pipeline.rs:95-126):
```rust
fn artifact_type_to_extension(artifact_type: &str) -> &str {
    match artifact_type {
        "text/markdown" => "md",
        "application/vnd.ant.react" => "jsx",
        "text/html" => "html",
        "image/svg+xml" => "svg",
        // ... etc
        _ => "txt",
    }
}
```

**Extraction Process**:
1. Iterate through all messages in conversation
2. For each message, check `content` array for `tool_use` blocks
3. Filter by `name == "artifacts"`
4. Extract `input.content`, `input.title`, `input.type`
5. Map type to file extension
6. Generate filename: `{index:02}-{slugified-title}.{ext}`
7. Write to `artifacts/` subdirectory

#### Folder Structure

```
output_dir/
├── {date}-{slug}/
│   ├── {date}-{slug}.md        # Markdown
│   ├── {date}-{slug}.json      # Raw JSON (preserved)
│   ├── {date}-{slug}.ndjson    # Message records
│   └── artifacts/              # Extracted artifacts (if any)
│       ├── 00-name.jsx
│       └── 01-other.svg
└── messages.ndjson             # Aggregate of all messages
```

### Markdown Rendering

**Format**:
1. YAML frontmatter with metadata
2. Title
3. Messages with emoji role indicators
4. Artifact references
5. Horizontal rules between messages

**Features**:
- Emoji roles: 👤 User, 🤖 Assistant, ⚙️ System, 🔧 Tool
- Formatted timestamps
- Project/meeting markers from `MarkerSet`
- Artifact indicators: 📎 **Artifact**: {title}

### Commands Layer (`commands.rs`)

#### `cmd_ndjson`
**Purpose**: High-speed conversion of JSON arrays to NDJSON

**Optimizations**:
- Uses `RawValueStream` (no `Conversation` parsing)
- Uses `serde_json::to_writer()` (no intermediate String)
- Direct write to `BufWriter`

**Performance**: 772MB → 756MB in ~4 seconds

#### `cmd_full_extract`
**Purpose**: One-command workflow

**Logic**:
1. Peek at first byte of input file
2. If `[` → JSON array:
   - Create temp NDJSON file in system temp dir
   - Run `cmd_ndjson` to convert
   - Continue with NDJSON
   - Clean up temp file (unless `--keep-ndjson`)
3. If `{` → Already NDJSON:
   - Skip conversion step
4. Run `split_file` on NDJSON

This provides a simple interface while maintaining streaming performance.

#### `explode_ndjson_parallel`
**Purpose**: Fast file splitting with rayon

**Implementation**:
- Load all NDJSON lines into memory (acceptable for line metadata)
- Use rayon thread pool (max 8 threads)
- Parallel `fs::write()` for each conversation
- Progress bar with `pb.inc(1)` in parallel closure

## Data Flow

### Full Extraction Flow

```
conversations.json (772MB)
    ↓
[Auto-detect: starts with '[']
    ↓
JsonArrayStream (manual parsing)
    ↓
Temp NDJSON (756MB) ──→ conversations.ndjson
    ↓                         ↓
ConvStream ←─────────────────┘
    ↓
Conversation::from_export()
    ↓
├─→ write_conversation()
│   ├─→ generate_slug()
│   ├─→ create conv_dir/
│   ├─→ extract_artifacts()
│   │   └─→ artifacts/ subdirectory
│   ├─→ render_markdown()
│   ├─→ write JSON (raw preserved)
│   └─→ write NDJSON
│
└─→ aggregate messages.ndjson
```

### Memory Profile

**At any point in time, memory holds**:
- 1 buffered `Conversation` struct (~10-50KB)
- 1 `BufReader` buffer (8KB default)
- 1 `BufWriter` buffer (8KB default)
- Progress bar state (negligible)

**Total**: ~100MB peak for entire 2912-conversation workload

## Benchmarks

### Formal Benchmarks (criterion)

The project includes criterion benchmarks for core streaming operations:

```bash
cargo bench -p floatctl-core
```

Results on Apple M-series (3-conversation fixture):
- `RawValueStream::parse_small_array`: 22 µs
- `ConvStream::parse_small_array`: 35 µs
- `Conversation::from_export`: 4.9 µs

This confirms ~1.6x overhead for full conversation parsing vs raw JSON streaming.

### Development Measurements

See [LESSONS.md](LESSONS.md:183-191) for informal performance measurements with the 772MB real-world dataset.

## Performance Optimizations

### 1. Avoid Cloning Message Arrays
**Before**:
```rust
for (idx, raw_message) in array.iter().cloned().enumerate() { ... }
```
- For 2912 conversations × 50 messages = 145K clones
- Each clone allocates and deeply copies nested JSON

**After**:
```rust
let msgs = std::mem::take(arr);  // Move Vec out, leave []
for (idx, raw_message) in msgs.into_iter().enumerate() { ... }
```
- Zero clones, all moves

### 2. Use `to_writer()` Not `to_string()`
**Before**:
```rust
let json = serde_json::to_string(&value)?;
writeln!(output, "{}", json)?;
```
- Allocates intermediate String
- Two write operations

**After**:
```rust
serde_json::to_writer(&mut output, &value)?;
output.write_all(b"\n")?;
```
- Direct serialization to output buffer
- Single write syscall

### 3. Lazy Regex Compilation
```rust
use once_cell::sync::Lazy;
static DATE_PREFIX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(\d{4}-\d{2}-\d{2}[\s\-:]+)").unwrap()
});
```
- Compile regex once, use many times
- Thread-safe via `Lazy<T>`

## Error Handling

All errors use `anyhow::Result<T>` with context:
```rust
.with_context(|| format!("failed to open {:?}", path))?
```

This provides clear error messages with full context chain.

## Testing Strategy

### Unit Tests
- `stream.rs`: Test JSON array vs NDJSON detection
- Slug generation with edge cases
- Artifact type mapping

### Integration Tests
- `floatctl-embed/tests/`: pgvector integration
- Golden fixtures in `tests/data/`

### Performance Tests
See `LESSONS.md` for documented performance tests with real data.

## Future Considerations

### Potential Improvements
1. **Parallel conversation processing**: Process multiple conversations concurrently
2. **Incremental updates**: Only process changed conversations
3. **Compressed archives**: Direct `.zip` support without extraction
4. **Custom templates**: User-defined Markdown templates
5. **Deduplication**: Skip already-processed conversations based on hash

### Scaling Limits
Current architecture handles:
- ✅ Files up to 1GB+
- ✅ Tens of thousands of conversations
- ✅ Long-running imports (hours)

Potential bottlenecks:
- **Filesystem**: Creating millions of directories (OS dependent)
- **Single-threaded parsing**: Could parallelize conversation processing
- **pgvector ingestion**: Network/DB bottleneck for embedding storage

## Related Documentation

- **[LESSONS.md](LESSONS.md)**: Performance lessons and benchmarks
- **[README.md](README.md)**: User-facing documentation
- **Per-crate READMEs**: Implementation details for each crate

## Key Takeaways

1. **Manual JSON array parsing is required** for true streaming of array elements
2. **Clone raw data before mutations** to preserve it for output
3. **Use `to_writer()` family** of functions to avoid intermediate allocations
4. **Test with minimal examples first** when debugging performance issues
5. **Streaming ≠ just using iterators** - requires careful API selection

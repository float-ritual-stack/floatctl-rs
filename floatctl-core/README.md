# floatctl-core

Core library for streaming conversation processing and analysis.

## Purpose

`floatctl-core` provides the fundamental types and streaming infrastructure for processing LLM conversation exports. It's designed as a library crate that can be used independently of the CLI.

## Features

- **Streaming JSON parsing**: O(1) memory usage for arbitrarily large files
- **Dual-format support**: ChatGPT and Anthropic/Claude export formats
- **Conversation models**: Structured types for conversations, messages, and metadata
- **Marker extraction**: Pattern-based metadata extraction (project::, ctx::, etc.)
- **Artifact handling**: Detection and extraction of code/document artifacts
- **NDJSON utilities**: Reading and writing newline-delimited JSON

## Key Components

### Streaming (`stream.rs`)

#### `JsonArrayStream`
Custom streaming parser for JSON arrays that yields elements one at a time without loading the entire array.

```rust
use floatctl_core::stream::JsonArrayStream;

let file = std::fs::File::open("conversations.json")?;
let mut stream = JsonArrayStream::new(file);

while let Some(value) = stream.next_element()? {
    // Process each JSON value as it's streamed
    println!("Got value: {}", value);
}
```

#### `ConvStream`
High-level iterator over `Conversation` structs with auto-format detection.

```rust
use floatctl_core::stream::ConvStream;

let stream = ConvStream::from_path("export.json")?;
for result in stream {
    let conversation = result?;
    println!("Conversation: {:?}", conversation.meta.title);
}
```

#### `RawValueStream`
Low-level iterator over `serde_json::Value` for operations that don't need conversation parsing.

```rust
use floatctl_core::stream::RawValueStream;

let stream = RawValueStream::from_path("export.json")?;
for result in stream {
    let raw_value = result?;
    // Work with raw JSON without conversation struct overhead
}
```

### Conversation Model (`conversation.rs`)

```rust
use floatctl_core::{Conversation, Message, MessageRole};
use serde_json::Value;

let json: Value = serde_json::from_str(raw_json)?;
let conversation = Conversation::from_export(json)?;

// Access metadata
println!("Title: {:?}", conversation.meta.title);
println!("Created: {}", conversation.meta.created_at);
println!("Messages: {}", conversation.messages.len());

// Iterate messages
for message in &conversation.messages {
    match message.role {
        MessageRole::User => println!("User: {}", message.content),
        MessageRole::Assistant => println!("Assistant: {}", message.content),
        _ => {}
    }
}

// Access raw JSON (preserved from input)
let original_json = &conversation.raw;
```

### Markers (`markers.rs`)

Pattern-based metadata extraction from message content.

```rust
use floatctl_core::markers::extract_markers;

let text = "ctx::meeting project::rangle/pharmacy Discussed API design";
let markers = extract_markers(text);

for marker in markers.iter() {
    println!("Found marker: {}", marker);
}
// Output:
// Found marker: ctx::meeting
// Found marker: project::rangle/pharmacy
```

**Supported patterns**:
- `ctx::{value}` - Context markers
- `project::{name}` - Project associations
- `float.{pattern}` - Float-specific markers
- And more... (see regex in markers.rs)

### Artifacts (`artifacts.rs`)

Types for representing extracted code artifacts.

```rust
use floatctl_core::{Artifact, ArtifactKind};

let artifact = Artifact {
    message_idx: 5,
    title: "React Component".to_string(),
    filename: "component.jsx".to_string(),
    kind: ArtifactKind::Code,
    language: Some("javascript".to_string()),
    body: "const MyComponent = () => { ... }".to_string(),
};
```

### Commands (`commands.rs`)

High-level command implementations that can be called programmatically.

```rust
use floatctl_core::{cmd_ndjson, cmd_full_extract};
use std::path::Path;

// Convert to NDJSON
cmd_ndjson(
    Path::new("conversations.json"),
    false,  // canonical=false
    Some(Path::new("output.ndjson"))
)?;

// Full extraction workflow
let opts = SplitOptions {
    output_dir: PathBuf::from("archive"),
    emit_markdown: true,
    emit_json: true,
    emit_ndjson: true,
    dry_run: false,
    show_progress: true,
};

cmd_full_extract("conversations.json", opts, false).await?;
```

### Pipeline (`pipeline.rs`)

Core conversation processing pipeline with slug generation and artifact extraction.

```rust
use floatctl_core::pipeline::{split_file, SplitOptions};

let opts = SplitOptions {
    output_dir: PathBuf::from("./output"),
    emit_markdown: true,
    emit_json: true,
    emit_ndjson: true,
    dry_run: false,
    show_progress: true,
};

split_file("conversations.ndjson", opts).await?;
```

## Format Support

### ChatGPT Format
```json
{
  "id": "conv-123",
  "title": "My Conversation",
  "created_at": "2024-01-15T10:00:00Z",
  "messages": [
    {
      "id": "msg-1",
      "role": "user",
      "timestamp": "2024-01-15T10:00:00Z",
      "content": "Hello"
    }
  ]
}
```

### Anthropic Format
```json
{
  "uuid": "conv-123",
  "name": "My Conversation",
  "created_at": "2024-01-15T10:00:00Z",
  "chat_messages": [
    {
      "uuid": "msg-1",
      "sender": "human",
      "created_at": "2024-01-15T10:00:00Z",
      "text": "Hello",
      "content": [{"type": "text", "text": "Hello"}]
    }
  ]
}
```

Both formats are handled transparently by `Conversation::from_export()`.

## Performance Characteristics

### Formal Benchmarks

Run criterion benchmarks:
```bash
cargo bench -p floatctl-core
```

Results on Apple M-series (3-conversation fixture):

| Operation | Time | Memory |
|-----------|------|--------|
| `RawValueStream` | 22 µs | O(1) per value |
| `ConvStream` | 35 µs | O(1) per conversation |
| `Conversation::from_export` | 4.9 µs | ~10-50KB per struct |

### Large File Performance

Informal development measurements (772MB file, 2912 conversations):
- Convert to NDJSON: ~4 seconds (<100MB memory)
- Full extraction: ~7 seconds (<100MB memory)
- Streaming maintains O(1) memory usage regardless of file size

See [LESSONS.md](../LESSONS.md) for detailed analysis.

## Dependencies

Core dependencies:
- `serde`/`serde_json`: JSON parsing
- `chrono`: Date/time handling
- `uuid`: Unique identifiers
- `anyhow`: Error handling
- `regex`/`once_cell`: Pattern matching
- `indicatif`: Progress bars

Optional:
- `tokio`: Async runtime (feature-gated)

## Usage Examples

### Example 1: Stream and Filter Conversations

```rust
use floatctl_core::stream::ConvStream;
use chrono::{Utc, Duration};

let stream = ConvStream::from_path("conversations.json")?;
let cutoff = Utc::now() - Duration::days(7);

for result in stream {
    let conv = result?;
    if conv.meta.created_at > cutoff {
        println!("Recent: {:?}", conv.meta.title);
    }
}
```

### Example 2: Extract Specific Markers

```rust
use floatctl_core::stream::ConvStream;

let stream = ConvStream::from_path("conversations.json")?;

for result in stream {
    let conv = result?;

    // Check conversation-level markers
    for marker in conv.meta.markers.iter() {
        if marker.starts_with("project::rangle") {
            println!("Found Rangle project conversation");
            break;
        }
    }

    // Check message-level markers
    for msg in &conv.messages {
        for marker in msg.markers.iter() {
            if marker.contains("urgent") {
                println!("Urgent message in {:?}", conv.meta.title);
            }
        }
    }
}
```

### Example 3: Convert Format

```rust
use floatctl_core::stream::RawValueStream;
use std::fs::File;
use std::io::{BufWriter, Write};

let stream = RawValueStream::from_path("conversations.json")?;
let output = BufWriter::new(File::create("output.ndjson")?);

for result in stream {
    let value = result?;
    serde_json::to_writer(&mut output, &value)?;
    output.write_all(b"\n")?;
}
```

## Testing

```bash
cargo test -p floatctl-core
```

See `tests/` directory for unit tests and fixtures.

## See Also

- **[floatctl-cli](../floatctl-cli/README.md)**: CLI interface using this library
- **[floatctl-embed](../floatctl-embed/README.md)**: Vector embedding integration
- **[ARCHITECTURE.md](../ARCHITECTURE.md)**: Detailed system design

## License

MIT - See LICENSE file

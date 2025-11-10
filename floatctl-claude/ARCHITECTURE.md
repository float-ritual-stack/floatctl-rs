# floatctl-claude Architecture Analysis

## Log Format Discovery (2025-11-09)

### Entry Types Found (8,777 total entries from evna project)

1. **assistant** (4,639 entries - 52.8%)
2. **user** (3,235 entries - 36.8%)
3. **queue-operation** (706 entries - 8.0%)
4. **summary** (110 entries - 1.3%)
5. **file-history-snapshot** (74 entries - 0.8%)
6. **system** (13 entries - 0.1%)

### Entry Type Structures

#### queue-operation
```json
{
  "type": "queue-operation",
  "operation": "enqueue",
  "timestamp": "2025-11-09T13:39:28.040Z",
  "content": "message text",
  "sessionId": "uuid"
}
```
**Fields**: `["content","operation","sessionId","timestamp","type"]`

#### user
```json
{
  "type": "user",
  "timestamp": "2025-11-09T13:39:28.050Z",
  "sessionId": "uuid",
  "uuid": "uuid",
  "parentUuid": "uuid",
  "isSidechain": false,
  "userType": "external",
  "cwd": "/path",
  "gitBranch": "main",
  "version": "1.0.0",
  "message": {
    "role": "user",
    "content": [...]
  }
}
```
**Fields**: `["cwd","gitBranch","isSidechain","message","parentUuid","sessionId","timestamp","type","userType","uuid","version"]`

#### assistant
```json
{
  "type": "assistant",
  "timestamp": "2025-11-09T13:39:29.337Z",
  "sessionId": "uuid",
  "requestId": "uuid",
  "uuid": "uuid",
  "parentUuid": "uuid",
  "isSidechain": false,
  "userType": "external",
  "cwd": "/path",
  "gitBranch": "main",
  "version": "1.0.0",
  "message": {
    "role": "assistant",
    "content": [...],
    "usage": {
      "input_tokens": 100,
      "output_tokens": 200,
      "cache_creation_input_tokens": 0,
      "cache_read_input_tokens": 0
    }
  }
}
```
**Fields**: `["cwd","gitBranch","isSidechain","message","parentUuid","requestId","sessionId","timestamp","type","userType","uuid","version"]`

#### system
```json
{
  "type": "system",
  "subtype": "notification",
  "level": "info",
  "content": "message",
  "timestamp": "2025-11-09T13:39:28.040Z",
  "sessionId": "uuid",
  "uuid": "uuid",
  "parentUuid": "uuid",
  "isSidechain": false,
  "isMeta": true,
  "userType": "external",
  "cwd": "/path",
  "gitBranch": "main",
  "version": "1.0.0"
}
```
**Fields**: `["content","cwd","gitBranch","isMeta","isSidechain","level","parentUuid","sessionId","subtype","timestamp","type","userType","uuid","version"]`

#### file-history-snapshot
```json
{
  "type": "file-history-snapshot",
  "messageId": "uuid",
  "isSnapshotUpdate": false,
  "snapshot": {
    "messages": [...],
    "systemPrompt": "..."
  }
}
```
**Fields**: `["isSnapshotUpdate","messageId","snapshot","type"]`
**Note**: NO `timestamp` field!

#### summary
```json
{
  "type": "summary",
  "summary": "Session summary text",
  "leafUuid": "uuid"
}
```
**Fields**: `["leafUuid","summary","type"]`
**Note**: NO `timestamp` field!

### Content Block Types (4,760 total blocks)

1. **text** (2,421 blocks - 50.9%)
2. **tool_use** (2,067 blocks - 43.4%)
3. **tool_result** (2,067 blocks - 43.4%)
4. **thinking** (199 blocks - 4.2%)
5. **image** (6 blocks - 0.1%)

### Critical Finding: tool_result Content Polymorphism

Analysis of 2,067 tool_result blocks reveals **TWO FORMATS**:

1. **String content** (1,461 instances - 70.7%)
```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_xxx",
  "content": "Simple string result"
}
```

2. **Array content** (606 instances - 29.3%)
```json
{
  "type": "tool_result",
  "tool_use_id": "toolu_xxx",
  "content": [
    {
      "type": "text",
      "text": "Nested content"
    }
  ]
}
```

**This polymorphism is the root cause of parsing failures.**

## Current Implementation Problems

### Problem 1: Missing `timestamp` Field Handling

**Entry types without timestamp**:
- `file-history-snapshot` (74 entries)
- `summary` (110 entries)

**Current code**: Expects `timestamp: String` (required field)

**Fix**: Make `timestamp` optional: `timestamp: Option<String>`

### Problem 2: tool_result Content Polymorphism

**Current code**:
```rust
ContentBlock::ToolResult {
    tool_use_id: String,
    content: Vec<ContentBlock>, // WRONG - assumes always array
    is_error: bool,
}
```

**Reality**: 70.7% are strings, 29.3% are arrays

**Fix needed**: Custom deserializer to handle both formats

### Problem 3: message.content Polymorphism

**User messages**: Sometimes `content` is a string (queue-operation fallback)

**Current code**: Uses custom deserializer `deserialize_content` that handles:
- String → `vec![ContentBlock::Text { text }]`
- Array → `Vec<ContentBlock>`

**This works well and should be pattern for tool_result.**

## Architecture Decision: Graceful Degradation

### Principle: Parse What We Can, Skip What We Can't

1. **Optional fields for metadata**: timestamp, session_id, etc.
2. **Polymorphic content handling**: Custom deserializers
3. **Skip unknown entry types**: Filter in stream processing
4. **Skip malformed entries**: Log warning, continue parsing
5. **Extract text from nested structures**: Recursive helper

### Design Pattern: Untagged Enum for Polymorphism

```rust
#[derive(Deserialize)]
#[serde(untagged)]
enum ToolResultContent {
    String(String),
    Blocks(Vec<ContentBlock>),
}
```

Then convert to canonical form in custom deserializer.

## Implementation Plan

### Phase 1: Fix Core Data Structures (floatctl-claude/src/lib.rs)

1. Make `LogEntry.timestamp` optional
2. Add custom deserializer for `ContentBlock::ToolResult.content`
3. Update `extract_text_from_blocks` to handle both string and array content

### Phase 2: Update Dependent Code

1. **parser.rs**: Update `extract_messages` to handle missing timestamps
2. **parser.rs**: Update `extract_session_metadata` for optional timestamps
3. **commands/show.rs**: Handle optional timestamps in display
4. **commands/recent_context.rs**: Skip entries without timestamps

### Phase 3: Add Test Coverage

1. Create fixtures for each entry type
2. Test string tool_result content
3. Test array tool_result content
4. Test missing timestamp handling
5. Test file-history-snapshot parsing
6. Test summary parsing

### Phase 4: Improve Error Messages

1. Add context to parsing errors (entry type, line number)
2. Log warnings for skipped entries
3. Provide degraded functionality instead of hard failures

## Testing Strategy

### Real-World Data Validation

Test against actual session logs:
- `~/.claude/projects/-Users-evan--evna/` (8,777 entries)
- `~/.claude/projects/-Users-evan-float-hub-operations-floatctl-rs/` (current session)

### Success Criteria

1. Parse 100% of entry types without crashing
2. Extract messages from 95%+ of user/assistant entries
3. Handle both tool_result content formats
4. Gracefully degrade for missing optional fields
5. Provide useful error messages for truly malformed data

## Future Enhancements

### Entry Type Support

Currently ignored but present in logs:
- `file-history-snapshot`: Could extract snapshot data
- `summary`: Could use for session metadata
- `system`: Could track notifications and errors

### Content Block Support

Rare but present:
- `image`: Could extract for visual context (6 instances found)

### Performance Optimization

- Streaming parser already O(1) memory
- Could add index for fast random access by message ID
- Could cache parsed sessions for repeated queries

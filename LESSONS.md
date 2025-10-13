# Lessons Learned: Streaming JSON Performance

## Context
While implementing `floatctl ndjson` to convert a 772MB JSON array to NDJSON format, we encountered a hang that appeared to be indefinite. After extensive debugging, we achieved a **>100x speedup** (from hanging >10 minutes to completing in ~4.5 seconds).

## Key Lessons

### 1. `serde_json::StreamDeserializer` Does NOT Stream Array Elements

**The Problem:**
```rust
// This looks like it would stream array elements, but it doesn't!
let de = Deserializer::from_reader(file);
let stream = de.into_iter::<Value>();
// If file starts with '[', this reads the ENTIRE array as ONE value
```

**What Actually Happens:**
- `StreamDeserializer` is designed for **streaming multiple top-level JSON values** (like NDJSON)
- When it encounters a JSON array `[...]`, it treats the **entire array as a single top-level value**
- For a 772MB file with a single array containing 2912 objects, it loads all 772MB before yielding anything

**The Solution:**
Manually parse the array structure:
```rust
pub struct JsonArrayStream {
    reader: BufReader<File>,
    started: bool,
    finished: bool,
}

impl JsonArrayStream {
    fn next_element(&mut self) -> Result<Option<Value>> {
        // Manually handle '[', ',', ']' and stream one element at a time
        // Read individual elements using Deserializer::from_reader
    }
}
```

**Takeaway:** For true O(1) memory streaming of JSON array elements, you must manually parse the array structure. `StreamDeserializer` is for sequences of top-level values, not array element streaming.

---

### 2. Avoid Unnecessary Parsing Round-Trips

**The Problem:**
```rust
// Original flow: JSON → Conversation struct → JSON
for value in stream {
    let conv = Conversation::from_export(value)?; // Parse
    let json = serde_json::to_string(&conv.raw)?; // Back to string
    writeln!(output, "{}", json)?;
}
```

**Why It's Slow:**
- Parses timestamps, UUIDs, extracts markers, validates message structure
- Allocates intermediate `Conversation` struct with all fields
- Stores full `raw: Value` field (doubles memory usage)
- Re-serializes the same data we just parsed

**The Solution:**
```rust
// Direct passthrough: JSON Value → JSON string
for value in RawValueStream::from_path(input)? {
    let value = value?; // Already a Value, no struct parsing
    serde_json::to_writer(&mut output, &value)?; // Direct write
    output.write_all(b"\n")?;
}
```

**Takeaway:** Only parse into structured types when you need to **transform** or **validate** the data. For passthrough operations, work with `serde_json::Value` directly.

---

### 3. Avoid Gratuitous `.cloned()` in Iterations

**The Problem:**
```rust
// Original code in conversation.rs:164
for (idx, raw_message) in array.iter().cloned().enumerate() {
    // Clones every message Value unnecessarily
}
```

**Why It's Slow:**
- Each message could be large (10KB+)
- For 2912 conversations × average 50 messages = ~145K clones
- Each clone allocates and deeply copies nested JSON structures

**The Solution:**
```rust
// Move the array out first
let msgs = value.get_mut("messages")
    .and_then(|m| m.as_array_mut())
    .map(|arr| std::mem::take(arr)) // Moves Vec<Value>, leaves []
    .unwrap_or_default();

// Then iterate with ownership
for (idx, raw_message) in msgs.into_iter().enumerate() {
    // raw_message is moved, zero clones
}
```

**Takeaway:** Use `std::mem::take()` or `into_iter()` to move data instead of cloning. Only clone when you truly need multiple owners.

---

### 4. Use `to_writer()` Instead of `to_string() + write!()`

**The Problem:**
```rust
// Creates intermediate String allocation
let line = serde_json::to_string(&value)?;
writeln!(output, "{}", line)?;
```

**Why It's Slower:**
- Allocates a String for every conversation
- String → bytes conversion
- Two write operations (content + newline)

**The Solution:**
```rust
// Writes JSON directly to output buffer
serde_json::to_writer(&mut output, &value)?;
output.write_all(b"\n")?;
```

**Takeaway:** When serializing to I/O, use `to_writer()` family functions to avoid intermediate String allocations.

---

### 5. Test Small Examples First

**The Debugging Process:**
1. ❌ Tested 772MB file → hung indefinitely, unclear why
2. ❌ Added debug output → still unclear, hung before any output
3. ❌ Suspected progress bar blocking → not the issue
4. ✅ Tested with 3-element array → immediately revealed the bug
   - Expected: 3 lines of NDJSON
   - Got: 1 line containing the entire array

**Takeaway:** When debugging performance issues, create minimal reproducible examples. A 3-element test file revealed in seconds what hours of debugging a 772MB file couldn't.

---

### 6. Debug Output Should Be Strategic

**What Worked:**
```rust
eprintln!("[DEBUG] Opening file: {:?}", input_path);
eprintln!("[DEBUG] Stream created, starting iteration...");
if idx == 0 {
    eprintln!("[DEBUG] Got first conversation from stream");
}
```

- Placed at key checkpoints in the data flow
- Revealed that code never reached "Stream created" → problem was earlier
- Small test case showed output appeared → proved streaming itself was fine

**Takeaway:** Debug output is most effective when placed at **transitions** in your data flow, not just at the start/end.

---

### 7. Profile Memory Usage, Not Just CPU Time

**The Original Symptoms:**
- Command appeared "hung" (no progress, no output)
- CPU usage was moderate, not maxed out
- No error messages

**The Real Problem:**
- Allocating 772MB+ of JSON in memory
- Possibly thrashing the allocator
- Taking minutes to complete a single huge allocation

**Takeaway:** When a program "hangs" with moderate CPU usage, check memory allocation patterns. Tools like `time -v` on Linux or Activity Monitor on macOS can show peak memory usage.

---

## Performance Results

| Implementation | Time | Memory | Streaming |
|---|---|---|---|
| Original `StreamDeserializer` | >10 min (timeout) | >1.5GB | ❌ No |
| `RawValueStream` + `to_writer()` | **~4.5 sec** | <100MB | ✅ Yes |

**Speedup: >100x** (from timeout to completion)

---

## Code References

- `floatctl-core/src/stream.rs:33-112` - `JsonArrayStream` implementation
- `floatctl-core/src/commands.rs:17-77` - Optimized `cmd_ndjson`
- `floatctl-core/src/conversation.rs:143-176` - Removed `.cloned()` with `std::mem::take()`

---

## Related Resources

- [serde_json::StreamDeserializer docs](https://docs.rs/serde_json/latest/serde_json/struct.StreamDeserializer.html)
- [Discussion: Streaming array elements](https://github.com/serde-rs/json/issues/345)
- [Rust Performance Book - Allocations](https://nnethercote.github.io/perf-book/heap-allocations.html)

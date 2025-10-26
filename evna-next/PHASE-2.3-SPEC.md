# Phase 2.3 Specification: Embedding at Write Time

**Status**: Specification (not implemented)
**Created**: 2025-10-24
**Architecture**: floatctl-cli does embedding, evna orchestrates

## Problem

Active context messages written to `messages` table but not `embeddings` table, making them invisible to `semantic_search` and `floatctl query` until manual embedding pipeline runs.

## Original Architecture (Preserved)

```
floatctl-rs (Rust):
├─ Heavy lifting: tokenization, chunking, embedding API, pgvector
├─ CLI interface for all operations
└─ Source of truth for embedding logic

evna-next (TypeScript):
├─ Orchestration: knows when to call floatctl
├─ Context synthesis: brain boot, active context
└─ Thin wrapper around floatctl commands
```

## Solution: floatctl-cli embed-message Command

### Command Design

```bash
cargo run -p floatctl-cli -- embed-message \
  --message-id <uuid> \
  [--skip-if-exists]
```

**Behavior:**
1. Fetch message content from `messages` table by ID
2. Check if embedding already exists (optional: `--skip-if-exists`)
3. Apply existing chunking logic if message >6000 tokens
4. Generate embeddings via OpenAI API (reuse existing client code)
5. Insert into `embeddings` table with proper `(message_id, chunk_index)` composite key
6. Return success/failure status

**Reuses existing floatctl code:**
- `floatctl-embed/src/lib.rs:42-115` - Chunking logic
- `floatctl-embed/src/lib.rs:158-257` - Embedding client
- `floatctl-embed/src/lib.rs:411-484` - Database insertion

### Integration with evna-next

**Location**: `src/lib/db.ts:298` (TODO comment already added)

```typescript
// Step 5: Call floatctl-cli to embed the message
try {
  const { exec } = await import('child_process');
  const { promisify } = await import('util');
  const execAsync = promisify(exec);

  await execAsync(
    `cargo run --release -p floatctl-cli -- embed-message ` +
    `--message-id "${persistedMessage.id}" ` +
    `--skip-if-exists`,
    {
      cwd: '../../', // Adjust to floatctl-rs root
      env: {
        ...process.env,
        RUST_LOG: 'off',
      },
    }
  );
} catch (embeddingError) {
  console.error('[db] floatctl embed-message failed:', {
    message_id: persistedMessage.id,
    error: embeddingError instanceof Error
      ? embeddingError.message
      : String(embeddingError),
  });
  // Don't throw - embedding is optimization, not requirement
}
```

## Why This Approach

1. **Preserves architecture**: floatctl owns embedding logic, evna orchestrates
2. **Reuses battle-tested code**: Chunking, tokenization already correct in Rust
3. **Single source of truth**: Embedding behavior defined once in floatctl
4. **No duplication**: Don't reimplement OpenAI client, chunking in TypeScript
5. **Consistent behavior**: `embed` (batch) and `embed-message` (single) use same logic

## Alternative Considered (Rejected)

**TypeScript embedding in evna-next:**
- ❌ Duplicates logic already in floatctl
- ❌ Creates two sources of truth for chunking/embedding
- ❌ Violates original architecture separation
- ❌ Harder to maintain (changes need TypeScript + Rust updates)

## Implementation Order

1. Add `embed-message` command to floatctl-cli (`floatctl-cli/src/main.rs`)
2. Implement command handler reusing existing embed code
3. Test: `cargo run -p floatctl-cli -- embed-message --message-id <uuid>`
4. Update `src/lib/db.ts` to call command after Step 4
5. Test: New active_context capture → verify embedding created
6. Document in CHANGELOG.md as Phase 2.3 completion

## Success Metrics

- [ ] `embed-message` command exists and works standalone
- [ ] New active_context captures immediately searchable via `floatctl query`
- [ ] New active_context captures immediately searchable via `semantic_search` tool
- [ ] No TypeScript duplication of Rust embedding logic
- [ ] Architecture diagram still valid: floatctl does work, evna orchestrates

## Related Documentation

- [CHANGELOG.md](./CHANGELOG.md) - Phase 2.3 deferred, now specified
- [CLAUDE.md](./CLAUDE.md) - Original architecture principles
- [floatctl-rs CLAUDE.md](../../CLAUDE.md) - Embedding pipeline details

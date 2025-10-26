# Test Harness Scripts

Quick test scripts for iterative development without MCP server restarts.

## Available Scripts

### test-db-methods.ts
Test database methods directly without MCP layer.

**Usage:**
```bash
# Test queryActiveContext with project filter
npx tsx scripts/test-db-methods.ts query-active-context --project "rangle/pharmacy" --limit 5

# Test queryActiveContext without filter
npx tsx scripts/test-db-methods.ts query-active-context --limit 10

# Test getRecentMessages
npx tsx scripts/test-db-methods.ts recent-messages --project "pharmacy" --limit 5

# Test semantic search (dual-source: active_context + embeddings)
npx tsx scripts/test-db-methods.ts semantic-search --query "pharmacy project filter" --limit 5 --threshold 0.5
```

**Options:**
- `--query <text>`: Search query (required for semantic-search)
- `--project <name>`: Filter by project name
- `--limit <n>`: Number of results to return
- `--threshold <0-1>`: Similarity threshold (for semantic-search, default: 0.5)
- `--since <ISO date>`: Filter by timestamp
- `--clientType <desktop|claude_code>`: Filter by client type

### test-annotation-parser.ts
Test annotation parser directly to verify metadata extraction.

**Usage:**
```bash
npx tsx scripts/test-annotation-parser.ts
```

Tests various annotation formats:
- Direct `project::` annotations
- Project markers in `ctx::` blocks
- Multiple annotation types together

## Why These Exist

During iterative debugging (like fixing the PostgREST filter bug), restarting the MCP server for every code change gets annoying. These scripts:

1. Load code directly from `src/`
2. Bypass MCP layer
3. Give immediate feedback on code changes
4. Show actual database state and metadata structure

## Example Workflow

```bash
# 1. Make a code change to src/tools/pgvector-search.ts
vim src/tools/pgvector-search.ts

# 2. Test immediately without MCP restart
npx tsx scripts/test-db-methods.ts semantic-search --query "pharmacy project filter" --limit 5

# 3. Iterate quickly
# (edit, test, edit, test...)

# 4. When satisfied, restart MCP to load the fix
# Then test via actual MCP tools
```

**Example output:**
```
🧪 Testing Semantic Search (Dual-Source)
✅ Success: 5 results

🔴 ACTIVE CONTEXT RESULTS (Recent, last 7 days):
--- Active Result 1 ---
Timestamp: 2025-10-23T18:50:17.429+00:00
Project: evna-next
Similarity: 1.00 (priority)
Content preview: ctx::2025-10-23 @ 02:50 PM - [project::evna-next] - [status::testing]...

🗄️  EMBEDDINGS RESULTS (Historical, archived):
--- Embedding Result 1 ---
Timestamp: 2025-08-18T16:16:00.000Z
Conversation: Rangle Pharmacy Bridge Tracking
Similarity: 0.60
Content preview: yes, and clean up some of the previus ones, and where neede...
```

## Requirements

- Environment variables in `.env`:
  - `SUPABASE_URL`
  - `SUPABASE_ANON_KEY`
  - `OPENAI_API_KEY` (required for semantic-search)
- `tsx` installed (via `npm install`)

### run-migration.ts
Backfill project metadata from ctx.metadata strings to top-level field.

**Usage:**
```bash
# Preview what will be updated
npx tsx scripts/run-migration.ts preview

# Run the migration (requires typing "yes" to confirm)
npx tsx scripts/run-migration.ts execute
```

**What it does:**
- Finds records with `ctx.metadata` containing `project::` markers
- Extracts project value and populates top-level `metadata.project` field
- Enables project filtering to work on existing records

**Example:**
```json
// Before migration:
{
  "ctx": {
    "metadata": "project::rangle/pharmacy"
  }
}

// After migration:
{
  "ctx": {
    "metadata": "project::rangle/pharmacy"
  },
  "project": "rangle/pharmacy"  // ← Added by migration
}
```

## Notes

- These scripts test against **live database data**
- Changes to annotation parser only affect **new messages**
- Existing database records need migration via `run-migration.ts`
- Use these for development, not production testing

## Complete Fix Workflow

**Problem**: Project filters weren't working because project data was trapped in `ctx.metadata` strings.

**Solution (3 steps)**:

1. **Fix parser** (already done): `src/lib/annotation-parser.ts` now extracts project from ctx:: blocks
2. **Backfill existing data**: Run `npx tsx scripts/run-migration.ts execute`
3. **Restart MCP**: New messages will use fixed parser, old messages have migrated data

**Verify fix works**:
```bash
# Test with project filter
npx tsx scripts/test-db-methods.ts query-active-context --project "rangle/pharmacy" --limit 5
```

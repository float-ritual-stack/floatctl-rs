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
```

**Options:**
- `--project <name>`: Filter by project name
- `--limit <n>`: Number of results to return
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
# 1. Make a code change to src/lib/db.ts
vim src/lib/db.ts

# 2. Test immediately without MCP restart
npx tsx scripts/test-db-methods.ts query-active-context --project "rangle/pharmacy" --limit 3

# 3. Iterate quickly
# (edit, test, edit, test...)

# 4. When satisfied, restart MCP to load the fix
# Then test via actual MCP tools
```

## Requirements

- Environment variables in `.env`:
  - `SUPABASE_URL`
  - `SUPABASE_ANON_KEY`
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

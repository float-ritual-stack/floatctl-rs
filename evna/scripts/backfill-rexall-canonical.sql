-- Backfill: normalize Rexall/Catalyst project variants to canonical `rangle/rexall`
-- Added 2026-05-19 — workspace-context.json now has all the aliases configured,
-- but historical rows captured with the variants need to be normalized so
-- the project filter at the semantic-RPC path (strict =, not ILIKE fan-out)
-- finds them.
--
-- The recency-path query (queryActiveContext non-semantic) already fans out
-- aliases via expandProjectAliases — historical rows tagged with variants
-- are found IF the search hits the recency path. Only the semantic RPC
-- path needs the historical rows to already be canonical.

-- PREVIEW: count rows that would be normalized
SELECT
  metadata->>'project' as current_project,
  COUNT(*) as row_count
FROM active_context_stream
WHERE
  metadata->>'project' ILIKE '%catalyst%'
  OR metadata->>'project' ILIKE '%rexall%'
  OR metadata->>'project' ILIKE 'rangle/rexall-catalyst'
  OR metadata->>'project' ILIKE 'rangle-rexall'
  OR metadata->>'project' ILIKE 'rangle-catalyst'
  OR metadata->>'project' ILIKE 'project-catalyst'
  OR metadata->>'project' ILIKE 'project_catalyst'
GROUP BY metadata->>'project'
ORDER BY row_count DESC;

-- DRY RUN: count total affected rows (rows that would change)
SELECT COUNT(*) as rows_to_normalize
FROM active_context_stream
WHERE
  metadata->>'project' IS NOT NULL
  AND metadata->>'project' != 'rangle/rexall'
  AND (
    metadata->>'project' ILIKE '%catalyst%'
    OR metadata->>'project' ILIKE '%rexall%'
    OR metadata->>'project' ILIKE 'project-catalyst'
    OR metadata->>'project' ILIKE 'project_catalyst'
  );

-- UPDATE: normalize all Rexall/Catalyst variants → `rangle/rexall`
-- Run this AFTER reviewing the PREVIEW + DRY RUN counts above.
UPDATE active_context_stream
SET metadata = jsonb_set(
  metadata,
  '{project}',
  to_jsonb('rangle/rexall'::text),
  true
)
WHERE
  metadata->>'project' IS NOT NULL
  AND metadata->>'project' != 'rangle/rexall'
  AND (
    metadata->>'project' ILIKE '%catalyst%'
    OR metadata->>'project' ILIKE '%rexall%'
    OR metadata->>'project' ILIKE 'project-catalyst'
    OR metadata->>'project' ILIKE 'project_catalyst'
    OR metadata->>'project' ILIKE 'rangle-rexall'
    OR metadata->>'project' ILIKE 'rangle-catalyst'
  );

-- Same normalization for the `messages` table (used by recent-messages queries)
UPDATE messages
SET project = 'rangle/rexall'
WHERE
  project IS NOT NULL
  AND project != 'rangle/rexall'
  AND (
    project ILIKE '%catalyst%'
    OR project ILIKE '%rexall%'
    OR project ILIKE 'project-catalyst'
    OR project ILIKE 'project_catalyst'
    OR project ILIKE 'rangle-rexall'
    OR project ILIKE 'rangle-catalyst'
  );

-- VERIFY: should return 0 rows after the UPDATE
SELECT
  metadata->>'project' as project,
  COUNT(*) as row_count
FROM active_context_stream
WHERE
  metadata->>'project' IS NOT NULL
  AND metadata->>'project' != 'rangle/rexall'
  AND (
    metadata->>'project' ILIKE '%catalyst%'
    OR metadata->>'project' ILIKE '%rexall%'
  )
GROUP BY metadata->>'project';

-- Final tally
SELECT
  COUNT(*) as total_rangle_rexall_rows
FROM active_context_stream
WHERE metadata->>'project' = 'rangle/rexall';

-- Backfill project metadata from ctx.metadata strings
-- Extracts project:: markers from ctx.metadata and populates top-level metadata.project

-- PREVIEW: See what will be updated (run this first!)
SELECT
  message_id,
  content,
  metadata->'ctx'->>'metadata' as ctx_metadata_string,
  regexp_replace(
    (metadata->'ctx'->>'metadata'),
    '.*project::([^\]]+).*',
    '\1'
  ) as extracted_project,
  metadata->'project' as current_project_field
FROM active_context_stream
WHERE
  -- Has ctx.metadata with project:: pattern
  metadata->'ctx'->>'metadata' LIKE '%project::%'
  -- Top-level project field is NULL or empty
  AND (metadata->'project' IS NULL OR metadata->'project'::text = 'null')
ORDER BY timestamp DESC
LIMIT 20;

-- DRY RUN: Count affected rows
SELECT COUNT(*) as affected_rows
FROM active_context_stream
WHERE
  metadata->'ctx'->>'metadata' LIKE '%project::%'
  AND (metadata->'project' IS NULL OR metadata->'project'::text = 'null');

-- ACTUAL UPDATE: Backfill the project field
-- IMPORTANT: Run the PREVIEW queries above first to verify!
UPDATE active_context_stream
SET metadata = jsonb_set(
  metadata,
  '{project}',
  to_jsonb(
    -- Extract project value from ctx.metadata string
    -- Matches: "project::value" or "project::value]"
    regexp_replace(
      (metadata->'ctx'->>'metadata'),
      '.*project::([^\]]+).*',
      '\1'
    )
  ),
  true  -- create_if_missing = true
)
WHERE
  -- Only update records with project in ctx.metadata
  metadata->'ctx'->>'metadata' LIKE '%project::%'
  -- Only update if top-level project is missing
  AND (metadata->'project' IS NULL OR metadata->'project'::text = 'null');

-- VERIFY: Check updated records
SELECT
  message_id,
  metadata->'project' as project_field,
  metadata->'ctx'->>'metadata' as ctx_metadata,
  content
FROM active_context_stream
WHERE metadata->'project' IS NOT NULL
ORDER BY timestamp DESC
LIMIT 10;

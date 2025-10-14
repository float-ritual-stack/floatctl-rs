-- Performance optimization indexes
-- Created based on performance audit recommendations

-- Composite index for common query pattern: filtering by project + timestamp
-- This significantly speeds up vector searches filtered by project and date range
-- Expected impact: 5-10x speedup for queries like:
--   SELECT * FROM messages WHERE project = 'X' AND timestamp >= 'Y'
create index if not exists messages_project_timestamp_idx
on messages(project, timestamp)
where project is not null;

-- Add covering index for conversation lookups
-- Helps with JOIN operations between messages and conversations
create index if not exists messages_conversation_id_idx
on messages(conversation_id);

-- Analyze tables to update statistics for query planner
analyze conversations;
analyze messages;
analyze embeddings;

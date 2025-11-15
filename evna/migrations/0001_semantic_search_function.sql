-- ╔═══════════════════════════════════════════════════════════╗
-- ║ VESTIGIAL MIGRATION - REPLACED BY AUTORAG (Nov 15, 2025)  ║
-- ╠═══════════════════════════════════════════════════════════╣
-- ║ This migration created pgvector semantic search via       ║
-- ║ match_messages() function referencing embeddings table.   ║
-- ║                                                            ║
-- ║ REPLACED BY: AutoRAG (Cloudflare AI Search)               ║
-- ║ - Multi-document synthesis with citations                 ║
-- ║ - Metadata filtering (folder, date)                       ║
-- ║ - Actually works (vs semantic_search returns empty)       ║
-- ║                                                            ║
-- ║ EVIDENCE: Last message embedding July 25, 2025            ║
-- ║ PHILOSOPHY: "Give the bitch a zine" > comprehensive logs  ║
-- ║                                                            ║
-- ║ Function dropped: DROP FUNCTION match_messages() CASCADE  ║
-- ║ Table dropped: DROP TABLE embeddings CASCADE              ║
-- ╚═══════════════════════════════════════════════════════════╝

-- VESTIGIAL CODE BELOW - KEPT FOR ARCHAEOLOGICAL REFERENCE

/*
create or replace function match_messages (
  query_embedding vector(1536),
  match_threshold float default 0.5,
  match_count int default 10
)
returns table (
  message_id uuid,
  similarity float
)
language sql stable
as $$
  select
    embeddings.message_id,
    1 - (embeddings.vector <=> query_embedding) as similarity
  from embeddings
  where 1 - (embeddings.vector <=> query_embedding) > match_threshold
  order by embeddings.vector <=> query_embedding
  limit match_count;
$$;

-- Grant execute permission
grant execute on function match_messages to anon, authenticated, service_role;
*/

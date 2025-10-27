-- Create semantic search function for pgvector similarity search
-- This function uses the IVFFlat index for fast approximate nearest neighbor search

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

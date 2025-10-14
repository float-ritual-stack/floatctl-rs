-- Add chunk support to embeddings table
-- Allows storing multiple embeddings per message for long content

ALTER TABLE embeddings
ADD COLUMN IF NOT EXISTS chunk_index INTEGER NOT NULL DEFAULT 0,
ADD COLUMN IF NOT EXISTS chunk_count INTEGER NOT NULL DEFAULT 1,
ADD COLUMN IF NOT EXISTS chunk_text TEXT,
ADD COLUMN IF NOT EXISTS created_at TIMESTAMPTZ DEFAULT NOW(),
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ;

-- Update the unique constraint to include chunk_index
ALTER TABLE embeddings DROP CONSTRAINT IF EXISTS embeddings_pkey;
ALTER TABLE embeddings ADD PRIMARY KEY (message_id, chunk_index);

-- Add index for querying all chunks of a message
CREATE INDEX IF NOT EXISTS idx_embeddings_message_chunks
ON embeddings(message_id, chunk_index);

COMMENT ON COLUMN embeddings.chunk_index IS 'Zero-based index of this chunk (0 for first/only chunk)';
COMMENT ON COLUMN embeddings.chunk_count IS 'Total number of chunks for this message';
COMMENT ON COLUMN embeddings.chunk_text IS 'The actual text content of this chunk (for context)';

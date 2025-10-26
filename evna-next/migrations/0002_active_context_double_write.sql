-- Migration: Add double-write tracking to active_context_stream
-- Purpose: Track which active_context messages have been persisted to permanent storage
-- Pattern: Write to hot cache (36hr TTL) AND permanent storage for organic corpus growth

-- Add double-write tracking columns
ALTER TABLE active_context_stream
ADD COLUMN persisted_to_long_term BOOLEAN DEFAULT FALSE,
ADD COLUMN persisted_message_id UUID;

-- Add foreign key to messages table (if persisted)
ALTER TABLE active_context_stream
ADD CONSTRAINT fk_persisted_message
FOREIGN KEY (persisted_message_id)
REFERENCES messages(id)
ON DELETE SET NULL;

-- Index for fast queries on persisted status
CREATE INDEX idx_active_context_persisted
ON active_context_stream(persisted_to_long_term)
WHERE persisted_to_long_term = FALSE;

-- Comment for clarity
COMMENT ON COLUMN active_context_stream.persisted_to_long_term IS
'Tracks whether this message has been double-written to permanent messages table';

COMMENT ON COLUMN active_context_stream.persisted_message_id IS
'UUID reference to the permanent message record in messages table';

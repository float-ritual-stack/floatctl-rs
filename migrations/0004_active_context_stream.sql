-- ============================================================================
-- ACTIVE CONTEXT STREAM TABLE
-- ============================================================================
-- Real-time message capture with JSONB metadata for dynamic annotation parsing
--
-- PHILOSOPHY: "Everything is redux and '::' is a float.dispatch in disguise"
--
-- This table embraces the dynamic, emergent nature of neurodivergent annotation
-- patterns rather than trying to predict and lock down a fixed schema. The user
-- invents new annotation patterns constantly (ctx::, sysop::, karen::, lf1m::,
-- float.dispatch(), highlight::, connectTo::, etc.) and JSONB lets us capture
-- whatever emerges without requiring migrations.
--
-- SYNTHETIC IDs: Uses generated IDs for real-time tracking since MCP protocol
-- doesn't provide conversation/message IDs. Can correlate with archive exports
-- later via timestamp + content matching.
--
-- See: evna-next/ACTIVE_CONTEXT_ARCHITECTURE.md for full design rationale
-- ============================================================================

CREATE TABLE IF NOT EXISTS active_context_stream (
    -- Synthetic IDs for real-time tracking (MCP doesn't provide real IDs)
    message_id TEXT PRIMARY KEY,
    conversation_id TEXT NOT NULL, -- Session-based synthetic ID

    -- Core message data
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant')),
    content TEXT NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL,
    client_type TEXT CHECK (client_type IN ('desktop', 'claude_code')),

    -- All the wild and wonderful annotations go here
    -- Structure is determined by annotation-parser.ts extraction logic
    -- Example metadata structure:
    -- {
    --   "project": "rangle/pharmacy",
    --   "personas": ["karen", "lf1m", "sysop"],
    --   "ctx": {
    --     "timestamp": "2025-10-21 @ 11:08:51 AM",
    --     "mode": "brain boot",
    --     "mood": "wonky"
    --   },
    --   "float_methods": ["dispatch", "burp", "ritual"],
    --   "highlights": ["everything is redux"],
    --   "connections": ["past-conversation-topic"],
    --   "meeting": "standup",
    --   "pr_references": ["550", "551"],
    --   "issue_references": ["168"],
    --   "commands": ["sysop::nudge"],
    --   "patterns": ["echoRefactor", "neurodivergent burp"]
    -- }
    metadata JSONB DEFAULT '{}',

    -- Housekeeping
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- ============================================================================
-- INDEXES
-- ============================================================================

-- Recency-based queries (most common pattern)
CREATE INDEX idx_active_context_timestamp
    ON active_context_stream(timestamp DESC);

-- Project filtering (from metadata JSONB)
CREATE INDEX idx_active_context_project
    ON active_context_stream((metadata->>'project'))
    WHERE metadata->>'project' IS NOT NULL;

-- Client-aware context surfacing (Desktop ↔ Claude Code)
CREATE INDEX idx_active_context_client_time
    ON active_context_stream(client_type, timestamp DESC)
    WHERE client_type IS NOT NULL;

-- Conversation grouping
CREATE INDEX idx_active_context_conversation
    ON active_context_stream(conversation_id);

-- Mode filtering (ctx::mode values)
CREATE INDEX idx_active_context_mode
    ON active_context_stream((metadata->'ctx'->>'mode'))
    WHERE metadata->'ctx'->>'mode' IS NOT NULL;

-- GIN index for flexible JSONB queries
-- Enables queries like: WHERE metadata @> '{"personas": ["karen"]}'
-- Or: WHERE metadata ? 'float_methods'
CREATE INDEX idx_active_context_metadata
    ON active_context_stream USING gin(metadata);

-- Full text search on content (for future correlation with archive)
CREATE INDEX idx_active_context_content_search
    ON active_context_stream USING gin(to_tsvector('english', content));

-- ============================================================================
-- QUERY EXAMPLES
-- ============================================================================

-- Find all messages in "brain boot" mode:
-- SELECT * FROM active_context_stream
-- WHERE metadata->'ctx'->>'mode' = 'brain boot'
-- ORDER BY timestamp DESC;

-- Find messages where karen persona appeared:
-- SELECT * FROM active_context_stream
-- WHERE metadata->'personas' ? 'karen'
-- ORDER BY timestamp DESC;

-- Client-aware: Desktop messages for rangle/pharmacy project:
-- SELECT * FROM active_context_stream
-- WHERE client_type = 'desktop'
--   AND metadata->>'project' = 'rangle/pharmacy'
-- ORDER BY timestamp DESC
-- LIMIT 10;

-- Find all float.dispatch() calls:
-- SELECT * FROM active_context_stream
-- WHERE metadata->'float_methods' ? 'dispatch'
-- ORDER BY timestamp DESC;

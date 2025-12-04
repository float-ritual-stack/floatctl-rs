-- floatctl-server schema
-- Boards, threads, messages, inbox, and scratchpad

-- Boards
CREATE TABLE IF NOT EXISTS boards (
    name        TEXT PRIMARY KEY CHECK (name ~ '^[a-z0-9][a-z0-9_-]{0,63}$'),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Threads
CREATE TABLE IF NOT EXISTS threads (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    board_name  TEXT NOT NULL REFERENCES boards(name) ON DELETE CASCADE,
    title       TEXT NOT NULL CHECK (length(title) <= 256),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Thread messages
CREATE TABLE IF NOT EXISTS thread_messages (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    thread_id   UUID NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    content     TEXT NOT NULL CHECK (length(content) <= 65536),
    author      TEXT CHECK (author IS NULL OR length(author) <= 64),
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Message markers (extracted from content)
CREATE TABLE IF NOT EXISTS message_markers (
    message_id  UUID NOT NULL REFERENCES thread_messages(id) ON DELETE CASCADE,
    kind        TEXT NOT NULL CHECK (kind IN ('ctx', 'project', 'mode', 'bridge', 'float')),
    value       TEXT NOT NULL CHECK (length(value) <= 256),
    PRIMARY KEY (message_id, kind, value)
);

-- Per-persona inbox
CREATE TABLE IF NOT EXISTS inboxes (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    persona     TEXT NOT NULL CHECK (persona IN ('evna', 'kitty', 'cowboy', 'daddy')),
    content     TEXT NOT NULL CHECK (length(content) <= 65536),
    from_persona TEXT CHECK (from_persona IS NULL OR from_persona IN ('evna', 'kitty', 'cowboy', 'daddy')),
    read_at     TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Scratchpad (key-value with TTL)
CREATE TABLE IF NOT EXISTS scratchpad (
    key         TEXT PRIMARY KEY CHECK (length(key) <= 256),
    value       JSONB NOT NULL,
    expires_at  TIMESTAMPTZ,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_threads_board ON threads(board_name);
CREATE INDEX IF NOT EXISTS idx_threads_created ON threads(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_thread ON thread_messages(thread_id);
CREATE INDEX IF NOT EXISTS idx_messages_created ON thread_messages(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_markers_kind_value ON message_markers(kind, value);
CREATE INDEX IF NOT EXISTS idx_inbox_persona ON inboxes(persona, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_scratchpad_expires ON scratchpad(expires_at) WHERE expires_at IS NOT NULL;

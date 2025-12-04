CREATE TABLE IF NOT EXISTS boards (
    name TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

CREATE TABLE IF NOT EXISTS threads (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    board_name TEXT NOT NULL REFERENCES boards(name) ON DELETE CASCADE,
    title TEXT NOT NULL,
    author TEXT,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);
CREATE INDEX IF NOT EXISTS idx_threads_board ON threads (board_name, created_at DESC);

CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    thread_id INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    author TEXT,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);

CREATE TABLE IF NOT EXISTS thread_markers (
    thread_id INTEGER NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    value TEXT NOT NULL,
    UNIQUE(thread_id, kind, value)
);
CREATE INDEX IF NOT EXISTS idx_thread_markers_kind_value ON thread_markers (kind, value);

CREATE TABLE IF NOT EXISTS inbox_messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    persona TEXT NOT NULL,
    author TEXT,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
);
CREATE INDEX IF NOT EXISTS idx_inbox_persona ON inbox_messages (persona, created_at DESC);

CREATE TABLE IF NOT EXISTS common_items (
    key TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    created_at INTEGER NOT NULL DEFAULT (strftime('%s','now')),
    expires_at INTEGER
);
CREATE INDEX IF NOT EXISTS idx_common_expires_at ON common_items (expires_at);

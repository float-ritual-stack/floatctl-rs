-- SQLite schema for floatctl-tui block storage
-- Design: Simple, fast, no enterprise bloat (as requested)

-- Core blocks table - stores all block types as JSONB
CREATE TABLE IF NOT EXISTS blocks (
    id TEXT PRIMARY KEY,
    block_type TEXT NOT NULL CHECK(block_type IN ('text', 'context_entry', 'agent_post', 'component', 'code', 'link')),
    content JSONB NOT NULL CHECK(json_valid(content)), -- Full block as JSON (validated)
    timestamp TEXT NOT NULL,            -- ISO 8601 timestamp
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Index for temporal queries (recent activity, time-based filtering)
CREATE INDEX IF NOT EXISTS idx_blocks_timestamp ON blocks(timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_blocks_type ON blocks(block_type);

-- Annotations table - extracted from blocks for filtering
CREATE TABLE IF NOT EXISTS annotations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    block_id TEXT NOT NULL,
    annotation_key TEXT NOT NULL,       -- 'project', 'meeting', 'mode', etc
    annotation_value TEXT NOT NULL,
    FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE
);

-- Index for annotation-based queries (board filtering)
CREATE INDEX IF NOT EXISTS idx_annotations_key_value ON annotations(annotation_key, annotation_value);
CREATE INDEX IF NOT EXISTS idx_annotations_block ON annotations(block_id);

-- Agent posts table - denormalized for quick board queries
CREATE TABLE IF NOT EXISTS agent_posts (
    id TEXT PRIMARY KEY,
    agent_id TEXT NOT NULL,
    board_id TEXT NOT NULL,
    block_id TEXT NOT NULL,
    title TEXT,
    timestamp TEXT NOT NULL,
    FOREIGN KEY (block_id) REFERENCES blocks(id) ON DELETE CASCADE
);

-- Index for board views
CREATE INDEX IF NOT EXISTS idx_agent_posts_board ON agent_posts(board_id, timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_agent_posts_agent ON agent_posts(agent_id);

-- Links table - extracted wikilinks and references
CREATE TABLE IF NOT EXISTS links (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    source_block_id TEXT NOT NULL,
    target_type TEXT NOT NULL,          -- 'block', 'file', 'url', 'board'
    target_value TEXT NOT NULL,
    display TEXT,
    FOREIGN KEY (source_block_id) REFERENCES blocks(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_links_source ON links(source_block_id);
CREATE INDEX IF NOT EXISTS idx_links_target ON links(target_type, target_value);

-- FTS5 virtual table for full-text search
CREATE VIRTUAL TABLE IF NOT EXISTS blocks_fts USING fts5(
    block_id UNINDEXED,
    content,
    tokenize = 'porter ascii'
);

-- Triggers to keep FTS index updated
-- Uses COALESCE to handle different block types:
-- - Text/ContextEntry: $.content
-- - AgentPost: nested content (just use title for now)
-- - Component: $.data
-- - Code: $.content
-- - Link: $.display
CREATE TRIGGER IF NOT EXISTS blocks_fts_insert AFTER INSERT ON blocks
BEGIN
    INSERT INTO blocks_fts(block_id, content)
    VALUES (
        NEW.id,
        COALESCE(
            json_extract(NEW.content, '$.content'),
            json_extract(NEW.content, '$.title'),
            json_extract(NEW.content, '$.display'),
            ''
        )
    );
END;

CREATE TRIGGER IF NOT EXISTS blocks_fts_update AFTER UPDATE OF content ON blocks
BEGIN
    DELETE FROM blocks_fts WHERE block_id = OLD.id;
    INSERT INTO blocks_fts(block_id, content)
    VALUES (
        NEW.id,
        COALESCE(
            json_extract(NEW.content, '$.content'),
            json_extract(NEW.content, '$.title'),
            json_extract(NEW.content, '$.display'),
            ''
        )
    );
END;

CREATE TRIGGER IF NOT EXISTS blocks_fts_delete AFTER DELETE ON blocks
BEGIN
    DELETE FROM blocks_fts WHERE block_id = OLD.id;
END;

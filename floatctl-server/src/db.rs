//! SQLite database layer for floatctl-server BBS
//!
//! Uses rusqlite with automatic schema migrations on startup.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::error::{ServerError, ServerResult};
use crate::models::*;

/// Thread-safe database wrapper
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
    path: PathBuf,
}

impl Database {
    /// Open or create the database at the given path
    pub fn open(path: impl Into<PathBuf>) -> ServerResult<Self> {
        let path = path.into();

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let conn = Connection::open(&path)?;

        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path,
        };

        db.run_migrations()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing)
    pub fn open_in_memory() -> ServerResult<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
            path: PathBuf::from(":memory:"),
        };
        db.run_migrations()?;
        Ok(db)
    }

    /// Get the database file path
    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Get database file size in bytes
    pub fn size_bytes(&self) -> Option<u64> {
        std::fs::metadata(&self.path).ok().map(|m| m.len())
    }

    /// Run schema migrations
    fn run_migrations(&self) -> ServerResult<()> {
        let conn = self.conn.lock().unwrap();

        conn.execute_batch(SCHEMA)?;

        // Create indexes
        conn.execute_batch(INDEXES)?;

        Ok(())
    }

    // ========================================================================
    // Boards
    // ========================================================================

    pub fn list_boards(&self) -> ServerResult<Vec<Board>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT b.id, b.name, b.description, b.created_at, b.updated_at,
                   (SELECT COUNT(*) FROM threads t WHERE t.board_id = b.id) as thread_count
            FROM boards b
            ORDER BY b.updated_at DESC
            "#,
        )?;

        let boards = stmt
            .query_map([], |row| {
                Ok(Board {
                    id: parse_uuid(row.get::<_, String>(0)?),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: parse_datetime(row.get::<_, String>(3)?),
                    updated_at: parse_datetime(row.get::<_, String>(4)?),
                    thread_count: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(boards)
    }

    pub fn get_board(&self, name: &str) -> ServerResult<Option<Board>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT b.id, b.name, b.description, b.created_at, b.updated_at,
                   (SELECT COUNT(*) FROM threads t WHERE t.board_id = b.id) as thread_count
            FROM boards b
            WHERE b.name = ?
            "#,
        )?;

        let board = stmt
            .query_row([name], |row| {
                Ok(Board {
                    id: parse_uuid(row.get::<_, String>(0)?),
                    name: row.get(1)?,
                    description: row.get(2)?,
                    created_at: parse_datetime(row.get::<_, String>(3)?),
                    updated_at: parse_datetime(row.get::<_, String>(4)?),
                    thread_count: row.get(5)?,
                })
            })
            .optional()?;

        Ok(board)
    }

    pub fn create_board(&self, req: &CreateBoardRequest) -> ServerResult<Board> {
        // Check for duplicate
        if self.get_board(&req.name)?.is_some() {
            return Err(ServerError::Conflict(format!(
                "Board '{}' already exists",
                req.name
            )));
        }

        let id = Uuid::new_v4();
        let now = Utc::now();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO boards (id, name, description, created_at, updated_at) VALUES (?, ?, ?, ?, ?)",
            params![id.to_string(), req.name, req.description, format_datetime(now), format_datetime(now)],
        )?;

        Ok(Board {
            id,
            name: req.name.clone(),
            description: req.description.clone(),
            created_at: now,
            updated_at: now,
            thread_count: 0,
        })
    }

    pub fn get_board_with_threads(
        &self,
        name: &str,
        limit: i64,
    ) -> ServerResult<Option<BoardWithThreads>> {
        let board = match self.get_board(name)? {
            Some(b) => b,
            None => return Ok(None),
        };

        let threads = self.list_threads_for_board(board.id, limit, 0)?;

        Ok(Some(BoardWithThreads {
            board,
            recent_threads: threads,
        }))
    }

    // ========================================================================
    // Threads
    // ========================================================================

    pub fn list_threads_for_board(
        &self,
        board_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> ServerResult<Vec<ThreadSummary>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"
            SELECT t.id, t.board_id, t.title, t.author, t.created_at, t.updated_at,
                   (SELECT COUNT(*) FROM messages m WHERE m.thread_id = t.id) as message_count,
                   (SELECT MAX(m.created_at) FROM messages m WHERE m.thread_id = t.id) as last_message_at
            FROM threads t
            WHERE t.board_id = ?
            ORDER BY t.updated_at DESC
            LIMIT ? OFFSET ?
            "#,
        )?;

        let threads = stmt
            .query_map(params![board_id.to_string(), limit, offset], |row| {
                Ok(ThreadSummary {
                    id: parse_uuid(row.get::<_, String>(0)?),
                    board_id: parse_uuid(row.get::<_, String>(1)?),
                    title: row.get(2)?,
                    author: row.get(3)?,
                    created_at: parse_datetime(row.get::<_, String>(4)?),
                    updated_at: parse_datetime(row.get::<_, String>(5)?),
                    message_count: row.get(6)?,
                    last_message_at: row
                        .get::<_, Option<String>>(7)?
                        .map(parse_datetime),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(threads)
    }

    pub fn get_thread(&self, thread_id: Uuid) -> ServerResult<Option<Thread>> {
        let conn = self.conn.lock().unwrap();

        // Get thread metadata
        let mut stmt = conn.prepare(
            "SELECT id, board_id, title, author, created_at, updated_at FROM threads WHERE id = ?",
        )?;

        let thread_row = stmt
            .query_row([thread_id.to_string()], |row| {
                Ok((
                    parse_uuid(row.get::<_, String>(0)?),
                    parse_uuid(row.get::<_, String>(1)?),
                    row.get::<_, String>(2)?,
                    row.get::<_, Option<String>>(3)?,
                    parse_datetime(row.get::<_, String>(4)?),
                    parse_datetime(row.get::<_, String>(5)?),
                ))
            })
            .optional()?;

        let (id, board_id, title, author, created_at, updated_at) = match thread_row {
            Some(r) => r,
            None => return Ok(None),
        };

        // Get messages
        let mut stmt = conn.prepare(
            "SELECT id, thread_id, author, content, created_at FROM messages WHERE thread_id = ? ORDER BY created_at ASC",
        )?;

        let messages: Vec<Message> = stmt
            .query_map([thread_id.to_string()], |row| {
                let content: String = row.get(3)?;
                let markers = extract_markers(&content);
                Ok(Message {
                    id: parse_uuid(row.get::<_, String>(0)?),
                    thread_id: parse_uuid(row.get::<_, String>(1)?),
                    author: row.get(2)?,
                    content,
                    created_at: parse_datetime(row.get::<_, String>(4)?),
                    markers,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        // Aggregate markers from all messages
        let mut markers = ThreadMarkers::default();
        for msg in &messages {
            for marker in &msg.markers {
                if let Some((kind, value)) = marker.split_once("::") {
                    match kind {
                        "project" => {
                            if !markers.projects.contains(&value.to_string()) {
                                markers.projects.push(value.to_string());
                            }
                        }
                        "ctx" => {
                            if !markers.contexts.contains(&value.to_string()) {
                                markers.contexts.push(value.to_string());
                            }
                        }
                        "mode" => {
                            if !markers.modes.contains(&value.to_string()) {
                                markers.modes.push(value.to_string());
                            }
                        }
                        "bridge" => {
                            if !markers.bridges.contains(&value.to_string()) {
                                markers.bridges.push(value.to_string());
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(Some(Thread {
            id,
            board_id,
            title,
            author,
            created_at,
            updated_at,
            messages,
            markers,
        }))
    }

    pub fn create_thread(
        &self,
        board_id: Uuid,
        req: &CreateThreadRequest,
    ) -> ServerResult<Thread> {
        let thread_id = Uuid::new_v4();
        let now = Utc::now();

        let conn = self.conn.lock().unwrap();

        conn.execute(
            "INSERT INTO threads (id, board_id, title, author, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?)",
            params![
                thread_id.to_string(),
                board_id.to_string(),
                req.title,
                req.author,
                format_datetime(now),
                format_datetime(now)
            ],
        )?;

        // Update board's updated_at
        conn.execute(
            "UPDATE boards SET updated_at = ? WHERE id = ?",
            params![format_datetime(now), board_id.to_string()],
        )?;

        let mut messages = Vec::new();
        let mut markers = ThreadMarkers::default();

        // Create initial message if content provided
        if let Some(content) = &req.content {
            let msg_id = Uuid::new_v4();
            let msg_markers = extract_markers(content);

            conn.execute(
                "INSERT INTO messages (id, thread_id, author, content, created_at) VALUES (?, ?, ?, ?, ?)",
                params![
                    msg_id.to_string(),
                    thread_id.to_string(),
                    req.author,
                    content,
                    format_datetime(now)
                ],
            )?;

            // Aggregate markers
            for marker in &msg_markers {
                if let Some((kind, value)) = marker.split_once("::") {
                    match kind {
                        "project" => markers.projects.push(value.to_string()),
                        "ctx" => markers.contexts.push(value.to_string()),
                        "mode" => markers.modes.push(value.to_string()),
                        "bridge" => markers.bridges.push(value.to_string()),
                        _ => {}
                    }
                }
            }

            messages.push(Message {
                id: msg_id,
                thread_id,
                author: req.author.clone(),
                content: content.clone(),
                created_at: now,
                markers: msg_markers,
            });
        }

        Ok(Thread {
            id: thread_id,
            board_id,
            title: req.title.clone(),
            author: req.author.clone(),
            created_at: now,
            updated_at: now,
            messages,
            markers,
        })
    }

    pub fn add_message(&self, thread_id: Uuid, req: &CreateMessageRequest) -> ServerResult<Message> {
        let msg_id = Uuid::new_v4();
        let now = Utc::now();
        let markers = extract_markers(&req.content);

        let conn = self.conn.lock().unwrap();

        // Verify thread exists
        let thread_exists: bool = conn
            .query_row(
                "SELECT 1 FROM threads WHERE id = ?",
                [thread_id.to_string()],
                |_| Ok(true),
            )
            .optional()?
            .unwrap_or(false);

        if !thread_exists {
            return Err(ServerError::NotFound(format!(
                "Thread {} not found",
                thread_id
            )));
        }

        conn.execute(
            "INSERT INTO messages (id, thread_id, author, content, created_at) VALUES (?, ?, ?, ?, ?)",
            params![
                msg_id.to_string(),
                thread_id.to_string(),
                req.author,
                req.content,
                format_datetime(now)
            ],
        )?;

        // Update thread's updated_at
        conn.execute(
            "UPDATE threads SET updated_at = ? WHERE id = ?",
            params![format_datetime(now), thread_id.to_string()],
        )?;

        // Update board's updated_at
        conn.execute(
            r#"
            UPDATE boards SET updated_at = ?
            WHERE id = (SELECT board_id FROM threads WHERE id = ?)
            "#,
            params![format_datetime(now), thread_id.to_string()],
        )?;

        Ok(Message {
            id: msg_id,
            thread_id,
            author: req.author.clone(),
            content: req.content.clone(),
            created_at: now,
            markers,
        })
    }

    pub fn search_threads(&self, params: &ThreadQueryParams) -> ServerResult<Vec<ThreadSummary>> {
        let conn = self.conn.lock().unwrap();

        let limit = params.limit.unwrap_or(50);
        let offset = params.offset.unwrap_or(0);

        // Build query based on filters
        let query = if params.project.is_some() || params.ctx.is_some() {
            r#"
            SELECT DISTINCT t.id, t.board_id, t.title, t.author, t.created_at, t.updated_at,
                   (SELECT COUNT(*) FROM messages m WHERE m.thread_id = t.id) as message_count,
                   (SELECT MAX(m.created_at) FROM messages m WHERE m.thread_id = t.id) as last_message_at
            FROM threads t
            JOIN messages m ON m.thread_id = t.id
            WHERE (? IS NULL OR m.content LIKE '%project::' || ? || '%')
              AND (? IS NULL OR m.content LIKE '%ctx::' || ? || '%')
            ORDER BY t.updated_at DESC
            LIMIT ? OFFSET ?
            "#
        } else {
            r#"
            SELECT t.id, t.board_id, t.title, t.author, t.created_at, t.updated_at,
                   (SELECT COUNT(*) FROM messages m WHERE m.thread_id = t.id) as message_count,
                   (SELECT MAX(m.created_at) FROM messages m WHERE m.thread_id = t.id) as last_message_at
            FROM threads t
            ORDER BY t.updated_at DESC
            LIMIT ? OFFSET ?
            "#
        };

        let threads: Vec<ThreadSummary> = if params.project.is_some() || params.ctx.is_some() {
            let mut stmt = conn.prepare(query)?;
            let rows = stmt.query_map(
                params![
                    params.project,
                    params.project,
                    params.ctx,
                    params.ctx,
                    limit,
                    offset
                ],
                |row| {
                    Ok(ThreadSummary {
                        id: parse_uuid(row.get::<_, String>(0)?),
                        board_id: parse_uuid(row.get::<_, String>(1)?),
                        title: row.get(2)?,
                        author: row.get(3)?,
                        created_at: parse_datetime(row.get::<_, String>(4)?),
                        updated_at: parse_datetime(row.get::<_, String>(5)?),
                        message_count: row.get(6)?,
                        last_message_at: row
                            .get::<_, Option<String>>(7)?
                            .map(parse_datetime),
                    })
                },
            )?;
            rows.collect::<Result<Vec<_>, _>>()?
        } else {
            let mut stmt = conn.prepare(query)?;
            let rows = stmt.query_map(params![limit, offset], |row| {
                Ok(ThreadSummary {
                    id: parse_uuid(row.get::<_, String>(0)?),
                    board_id: parse_uuid(row.get::<_, String>(1)?),
                    title: row.get(2)?,
                    author: row.get(3)?,
                    created_at: parse_datetime(row.get::<_, String>(4)?),
                    updated_at: parse_datetime(row.get::<_, String>(5)?),
                    message_count: row.get(6)?,
                    last_message_at: row
                        .get::<_, Option<String>>(7)?
                        .map(parse_datetime),
                })
            })?;
            rows.collect::<Result<Vec<_>, _>>()?
        };

        Ok(threads)
    }

    // ========================================================================
    // Inbox
    // ========================================================================

    pub fn list_inbox(
        &self,
        persona: &str,
        params: &InboxQueryParams,
    ) -> ServerResult<Vec<InboxMessage>> {
        let conn = self.conn.lock().unwrap();
        let limit = params.limit.unwrap_or(50);
        let include_read = params.include_read.unwrap_or(false);

        let query = if include_read {
            r#"
            SELECT id, persona, from_persona, subject, content, created_at, read_at, thread_id, priority
            FROM inbox
            WHERE persona = ?
            ORDER BY
                CASE priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END,
                created_at DESC
            LIMIT ?
            "#
        } else {
            r#"
            SELECT id, persona, from_persona, subject, content, created_at, read_at, thread_id, priority
            FROM inbox
            WHERE persona = ? AND read_at IS NULL
            ORDER BY
                CASE priority WHEN 'urgent' THEN 0 WHEN 'high' THEN 1 WHEN 'normal' THEN 2 ELSE 3 END,
                created_at DESC
            LIMIT ?
            "#
        };

        let mut stmt = conn.prepare(query)?;
        let messages = stmt
            .query_map(params![persona, limit], |row| {
                Ok(InboxMessage {
                    id: parse_uuid(row.get::<_, String>(0)?),
                    persona: row.get(1)?,
                    from_persona: row.get(2)?,
                    subject: row.get(3)?,
                    content: row.get(4)?,
                    created_at: parse_datetime(row.get::<_, String>(5)?),
                    read_at: row.get::<_, Option<String>>(6)?.map(parse_datetime),
                    thread_id: row
                        .get::<_, Option<String>>(7)?
                        .map(parse_uuid),
                    priority: row
                        .get::<_, String>(8)?
                        .parse()
                        .unwrap_or(InboxPriority::Normal),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(messages)
    }

    pub fn send_inbox_message(
        &self,
        persona: &str,
        req: &CreateInboxMessageRequest,
    ) -> ServerResult<InboxMessage> {
        let id = Uuid::new_v4();
        let now = Utc::now();

        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"
            INSERT INTO inbox (id, persona, from_persona, subject, content, created_at, thread_id, priority)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            "#,
            params![
                id.to_string(),
                persona,
                req.from_persona,
                req.subject,
                req.content,
                format_datetime(now),
                req.thread_id.map(|u| u.to_string()),
                req.priority.to_string()
            ],
        )?;

        Ok(InboxMessage {
            id,
            persona: persona.to_string(),
            from_persona: req.from_persona.clone(),
            subject: req.subject.clone(),
            content: req.content.clone(),
            created_at: now,
            read_at: None,
            thread_id: req.thread_id,
            priority: req.priority,
        })
    }

    pub fn mark_inbox_read(&self, persona: &str, message_id: Uuid) -> ServerResult<bool> {
        let now = Utc::now();
        let conn = self.conn.lock().unwrap();

        let rows_affected = conn.execute(
            "UPDATE inbox SET read_at = ? WHERE id = ? AND persona = ?",
            params![format_datetime(now), message_id.to_string(), persona],
        )?;

        Ok(rows_affected > 0)
    }

    pub fn delete_inbox_message(&self, persona: &str, message_id: Uuid) -> ServerResult<bool> {
        let conn = self.conn.lock().unwrap();

        let rows_affected = conn.execute(
            "DELETE FROM inbox WHERE id = ? AND persona = ?",
            params![message_id.to_string(), persona],
        )?;

        Ok(rows_affected > 0)
    }

    // ========================================================================
    // Common Area
    // ========================================================================

    pub fn list_common(&self, params: &CommonQueryParams) -> ServerResult<Vec<CommonItem>> {
        let conn = self.conn.lock().unwrap();
        let limit = params.limit.unwrap_or(100);
        let include_expired = params.include_expired.unwrap_or(false);
        let now = format_datetime(Utc::now());

        let query = if let Some(prefix) = &params.prefix {
            if include_expired {
                format!(
                    r#"
                    SELECT key, value, created_at, updated_at, expires_at, created_by
                    FROM common
                    WHERE key LIKE '{}%'
                    ORDER BY updated_at DESC
                    LIMIT ?
                    "#,
                    prefix.replace('\'', "''")
                )
            } else {
                format!(
                    r#"
                    SELECT key, value, created_at, updated_at, expires_at, created_by
                    FROM common
                    WHERE key LIKE '{}%' AND (expires_at IS NULL OR expires_at > ?)
                    ORDER BY updated_at DESC
                    LIMIT ?
                    "#,
                    prefix.replace('\'', "''")
                )
            }
        } else if include_expired {
            r#"
            SELECT key, value, created_at, updated_at, expires_at, created_by
            FROM common
            ORDER BY updated_at DESC
            LIMIT ?
            "#
            .to_string()
        } else {
            r#"
            SELECT key, value, created_at, updated_at, expires_at, created_by
            FROM common
            WHERE expires_at IS NULL OR expires_at > ?
            ORDER BY updated_at DESC
            LIMIT ?
            "#
            .to_string()
        };

        let mut stmt = conn.prepare(&query)?;

        let items = if include_expired {
            stmt.query_map(params![limit], |row| {
                Ok(CommonItem {
                    key: row.get(0)?,
                    value: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    created_at: parse_datetime(row.get::<_, String>(2)?),
                    updated_at: parse_datetime(row.get::<_, String>(3)?),
                    expires_at: row.get::<_, Option<String>>(4)?.map(parse_datetime),
                    created_by: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        } else {
            stmt.query_map(params![now, limit], |row| {
                Ok(CommonItem {
                    key: row.get(0)?,
                    value: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    created_at: parse_datetime(row.get::<_, String>(2)?),
                    updated_at: parse_datetime(row.get::<_, String>(3)?),
                    expires_at: row.get::<_, Option<String>>(4)?.map(parse_datetime),
                    created_by: row.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?
        };

        Ok(items)
    }

    pub fn get_common(&self, key: &str) -> ServerResult<Option<CommonItem>> {
        let conn = self.conn.lock().unwrap();
        let now = format_datetime(Utc::now());

        let mut stmt = conn.prepare(
            r#"
            SELECT key, value, created_at, updated_at, expires_at, created_by
            FROM common
            WHERE key = ? AND (expires_at IS NULL OR expires_at > ?)
            "#,
        )?;

        let item = stmt
            .query_row(params![key, now], |row| {
                Ok(CommonItem {
                    key: row.get(0)?,
                    value: serde_json::from_str(&row.get::<_, String>(1)?).unwrap_or_default(),
                    created_at: parse_datetime(row.get::<_, String>(2)?),
                    updated_at: parse_datetime(row.get::<_, String>(3)?),
                    expires_at: row.get::<_, Option<String>>(4)?.map(parse_datetime),
                    created_by: row.get(5)?,
                })
            })
            .optional()?;

        Ok(item)
    }

    pub fn set_common(&self, req: &CreateCommonItemRequest) -> ServerResult<CommonItem> {
        let now = Utc::now();
        let expires_at = req.ttl_seconds.map(|ttl| {
            now + chrono::Duration::seconds(ttl)
        });

        let value_json = serde_json::to_string(&req.value)?;

        let conn = self.conn.lock().unwrap();

        // Upsert
        conn.execute(
            r#"
            INSERT INTO common (key, value, created_at, updated_at, expires_at, created_by)
            VALUES (?, ?, ?, ?, ?, ?)
            ON CONFLICT(key) DO UPDATE SET
                value = excluded.value,
                updated_at = excluded.updated_at,
                expires_at = excluded.expires_at
            "#,
            params![
                req.key,
                value_json,
                format_datetime(now),
                format_datetime(now),
                expires_at.map(format_datetime),
                req.created_by
            ],
        )?;

        Ok(CommonItem {
            key: req.key.clone(),
            value: req.value.clone(),
            created_at: now,
            updated_at: now,
            expires_at,
            created_by: req.created_by.clone(),
        })
    }

    pub fn delete_common(&self, key: &str) -> ServerResult<bool> {
        let conn = self.conn.lock().unwrap();
        let rows_affected = conn.execute("DELETE FROM common WHERE key = ?", params![key])?;
        Ok(rows_affected > 0)
    }

    /// Clean up expired items from common area
    pub fn cleanup_expired(&self) -> ServerResult<usize> {
        let conn = self.conn.lock().unwrap();
        let now = format_datetime(Utc::now());

        let deleted = conn.execute(
            "DELETE FROM common WHERE expires_at IS NOT NULL AND expires_at <= ?",
            params![now],
        )?;

        Ok(deleted)
    }
}

// ============================================================================
// Schema
// ============================================================================

const SCHEMA: &str = r#"
-- Boards table
CREATE TABLE IF NOT EXISTS boards (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    description TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Threads table
CREATE TABLE IF NOT EXISTS threads (
    id TEXT PRIMARY KEY,
    board_id TEXT NOT NULL REFERENCES boards(id),
    title TEXT NOT NULL,
    author TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

-- Messages table
CREATE TABLE IF NOT EXISTS messages (
    id TEXT PRIMARY KEY,
    thread_id TEXT NOT NULL REFERENCES threads(id),
    author TEXT,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL
);

-- Inbox table
CREATE TABLE IF NOT EXISTS inbox (
    id TEXT PRIMARY KEY,
    persona TEXT NOT NULL,
    from_persona TEXT,
    subject TEXT,
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    read_at TEXT,
    thread_id TEXT REFERENCES threads(id),
    priority TEXT NOT NULL DEFAULT 'normal'
);

-- Common area table
CREATE TABLE IF NOT EXISTS common (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    expires_at TEXT,
    created_by TEXT
);
"#;

const INDEXES: &str = r#"
-- Indexes for efficient queries
CREATE INDEX IF NOT EXISTS idx_threads_board ON threads(board_id);
CREATE INDEX IF NOT EXISTS idx_threads_updated ON threads(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_messages_thread ON messages(thread_id);
CREATE INDEX IF NOT EXISTS idx_messages_created ON messages(created_at);
CREATE INDEX IF NOT EXISTS idx_inbox_persona ON inbox(persona);
CREATE INDEX IF NOT EXISTS idx_inbox_unread ON inbox(persona, read_at) WHERE read_at IS NULL;
CREATE INDEX IF NOT EXISTS idx_common_expires ON common(expires_at) WHERE expires_at IS NOT NULL;
"#;

// ============================================================================
// Helpers
// ============================================================================

fn parse_uuid(s: String) -> Uuid {
    Uuid::parse_str(&s).unwrap_or_else(|_| Uuid::nil())
}

fn parse_datetime(s: String) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .unwrap_or_else(|_| Utc::now())
}

fn format_datetime(dt: DateTime<Utc>) -> String {
    dt.to_rfc3339()
}

/// Extract :: markers from content
fn extract_markers(content: &str) -> Vec<String> {
    let mut markers = Vec::new();
    let marker_pattern = regex::Regex::new(r"\b(\w+)::([\w/\-_.]+)").unwrap();

    for cap in marker_pattern.captures_iter(content) {
        markers.push(format!("{}::{}", &cap[1], &cap[2]));
    }

    markers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_markers() {
        let content = "Working on project::pharmacy with ctx::late-night mode::deep";
        let markers = extract_markers(content);
        assert_eq!(markers.len(), 3);
        assert!(markers.contains(&"project::pharmacy".to_string()));
        assert!(markers.contains(&"ctx::late-night".to_string()));
        assert!(markers.contains(&"mode::deep".to_string()));
    }

    #[test]
    fn test_database_operations() {
        let db = Database::open_in_memory().unwrap();

        // Create a board
        let board = db
            .create_board(&CreateBoardRequest {
                name: "test-board".to_string(),
                description: Some("A test board".to_string()),
            })
            .unwrap();
        assert_eq!(board.name, "test-board");

        // Create a thread
        let thread = db
            .create_thread(
                board.id,
                &CreateThreadRequest {
                    title: "Test Thread".to_string(),
                    author: Some("cowboy".to_string()),
                    content: Some("First message with project::test".to_string()),
                },
            )
            .unwrap();
        assert_eq!(thread.title, "Test Thread");
        assert_eq!(thread.messages.len(), 1);
        assert!(thread.markers.projects.contains(&"test".to_string()));

        // Add a message
        let msg = db
            .add_message(
                thread.id,
                &CreateMessageRequest {
                    author: Some("kitty".to_string()),
                    content: "Reply message".to_string(),
                },
            )
            .unwrap();
        assert_eq!(msg.author, Some("kitty".to_string()));

        // Verify thread has 2 messages
        let updated_thread = db.get_thread(thread.id).unwrap().unwrap();
        assert_eq!(updated_thread.messages.len(), 2);
    }
}

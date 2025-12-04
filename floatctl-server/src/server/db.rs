use std::path::Path;

use chrono::{DateTime, Duration, Utc};
use once_cell::sync::Lazy;
use regex::Regex;
use sqlx::{
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
    FromRow, SqlitePool,
};
use sqlx::{QueryBuilder, Sqlite};
use tokio::fs;

use crate::server::{
    error::AppError,
    models::{
        Board, CommonItem, CreateMessageRequest, CreateThreadRequest, InboxMessage, Marker,
        Message, ThreadDetail, ThreadQuery, ThreadSummary,
    },
};

#[derive(Clone)]
pub struct Database {
    pool: SqlitePool,
}

impl Database {
    pub async fn connect(path: &Path) -> Result<Self, AppError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).await?;
        }

        let options = SqliteConnectOptions::new()
            .filename(path)
            .journal_mode(SqliteJournalMode::Wal)
            .create_if_missing(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        let db = Self { pool };
        db.migrate().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<(), AppError> {
        let queries = [
            "CREATE TABLE IF NOT EXISTS boards (\n                id INTEGER PRIMARY KEY AUTOINCREMENT,\n                name TEXT NOT NULL UNIQUE,\n                created_at TEXT NOT NULL\n            )",
            "CREATE TABLE IF NOT EXISTS threads (\n                id INTEGER PRIMARY KEY AUTOINCREMENT,\n                board_id INTEGER NOT NULL,\n                title TEXT NOT NULL,\n                created_at TEXT NOT NULL,\n                FOREIGN KEY(board_id) REFERENCES boards(id)\n            )",
            "CREATE TABLE IF NOT EXISTS messages (\n                id INTEGER PRIMARY KEY AUTOINCREMENT,\n                thread_id INTEGER NOT NULL,\n                author TEXT,\n                content TEXT NOT NULL,\n                created_at TEXT NOT NULL,\n                FOREIGN KEY(thread_id) REFERENCES threads(id)\n            )",
            "CREATE TABLE IF NOT EXISTS markers (\n                id INTEGER PRIMARY KEY AUTOINCREMENT,\n                thread_id INTEGER NOT NULL,\n                kind TEXT NOT NULL,\n                value TEXT NOT NULL,\n                UNIQUE(thread_id, kind, value),\n                FOREIGN KEY(thread_id) REFERENCES threads(id)\n            )",
            "CREATE TABLE IF NOT EXISTS inbox_messages (\n                id INTEGER PRIMARY KEY AUTOINCREMENT,\n                persona TEXT NOT NULL,\n                content TEXT NOT NULL,\n                created_at TEXT NOT NULL\n            )",
            "CREATE TABLE IF NOT EXISTS common_items (\n                id INTEGER PRIMARY KEY AUTOINCREMENT,\n                key TEXT NOT NULL UNIQUE,\n                value TEXT NOT NULL,\n                created_at TEXT NOT NULL,\n                expires_at TEXT\n            )",
        ];

        for query in queries {
            sqlx::query(query).execute(&self.pool).await?;
        }
        Ok(())
    }

    pub async fn create_board(&self, name: &str) -> Result<Board, AppError> {
        let created_at = Utc::now();
        sqlx::query(
            "INSERT INTO boards (name, created_at) VALUES (?1, ?2) ON CONFLICT(name) DO NOTHING",
        )
        .bind(name)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        self.get_board(name)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("board {name}")))
    }

    pub async fn list_boards(&self) -> Result<Vec<Board>, AppError> {
        let rows = sqlx::query_as::<_, BoardRow>(
            "SELECT name, created_at FROM boards ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Board::from).collect())
    }

    pub async fn get_board(&self, name: &str) -> Result<Option<Board>, AppError> {
        let row =
            sqlx::query_as::<_, BoardRow>("SELECT name, created_at FROM boards WHERE name = ?1")
                .bind(name)
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(Board::from))
    }

    pub async fn list_threads(
        &self,
        board: Option<&str>,
        filter: ThreadQuery,
    ) -> Result<Vec<ThreadSummary>, AppError> {
        let project = filter.project;
        let mut builder: QueryBuilder<Sqlite> = QueryBuilder::new(
            "SELECT t.id, b.name as board, t.title, t.created_at FROM threads t JOIN boards b ON t.board_id = b.id",
        );

        if project.is_some() {
            builder.push(" JOIN markers m ON m.thread_id = t.id AND m.kind = 'project'");
        }

        let mut has_where = false;
        if let Some(board_name) = board {
            builder.push(" WHERE b.name = ");
            builder.push_bind(board_name);
            has_where = true;
        }

        if let Some(project) = project {
            builder.push(if has_where { " AND" } else { " WHERE" });
            builder.push(" m.value = ");
            builder.push_bind(project);
        }

        builder.push(" ORDER BY t.created_at DESC");

        let threads = builder
            .build_query_as::<ThreadRow>()
            .fetch_all(&self.pool)
            .await?;

        let mut summaries = Vec::with_capacity(threads.len());
        for thread in threads {
            let last_message_at = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
                "SELECT MAX(created_at) FROM messages WHERE thread_id = ?1",
            )
            .bind(thread.id)
            .fetch_one(&self.pool)
            .await?;

            let markers = self.load_markers(thread.id).await?;
            summaries.push(ThreadSummary {
                id: thread.id,
                board: thread.board,
                title: thread.title,
                created_at: thread.created_at,
                last_message_at,
                markers,
            });
        }

        Ok(summaries)
    }

    pub async fn create_thread(
        &self,
        board_name: &str,
        req: CreateThreadRequest,
    ) -> Result<ThreadDetail, AppError> {
        let _board = self
            .get_board(board_name)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("board {board_name}")))?;

        let created_at = Utc::now();
        let mut tx = self.pool.begin().await?;
        let thread_id: i64 = sqlx::query(
            "INSERT INTO threads (board_id, title, created_at) VALUES ((SELECT id FROM boards WHERE name = ?1), ?2, ?3)",
        )
        .bind(board_name)
        .bind(&req.title)
        .bind(created_at)
        .execute(&mut *tx)
        .await?
        .last_insert_rowid();

        if let Some(content) = req.message {
            self.insert_message(&mut tx, thread_id, &req.author, &content)
                .await?;
        }

        tx.commit().await?;

        self.get_thread(thread_id)
            .await?
            .ok_or_else(|| AppError::NotFound("thread not found after creation".into()))
    }

    pub async fn add_message(
        &self,
        thread_id: i64,
        req: CreateMessageRequest,
    ) -> Result<Message, AppError> {
        let mut tx = self.pool.begin().await?;
        let message = self
            .insert_message(&mut tx, thread_id, &req.author, &req.content)
            .await?;
        tx.commit().await?;
        Ok(message)
    }

    async fn insert_message(
        &self,
        tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
        thread_id: i64,
        author: &Option<String>,
        content: &str,
    ) -> Result<Message, AppError> {
        let created_at = Utc::now();
        let result = sqlx::query(
            "INSERT INTO messages (thread_id, author, content, created_at) VALUES (?1, ?2, ?3, ?4)",
        )
        .bind(thread_id)
        .bind(author)
        .bind(content)
        .bind(created_at)
        .execute(&mut **tx)
        .await?;

        let message_id = result.last_insert_rowid();
        let markers = extract_markers(content);
        for marker in markers {
            sqlx::query(
                "INSERT OR IGNORE INTO markers (thread_id, kind, value) VALUES (?1, ?2, ?3)",
            )
            .bind(thread_id)
            .bind(marker.kind)
            .bind(marker.value)
            .execute(&mut **tx)
            .await?;
        }

        Ok(Message {
            id: message_id,
            thread_id,
            author: author.clone(),
            content: content.to_string(),
            created_at,
        })
    }

    pub async fn get_thread(&self, id: i64) -> Result<Option<ThreadDetail>, AppError> {
        let thread = sqlx::query_as::<_, ThreadRow>(
            "SELECT t.id, b.name as board, t.title, t.created_at FROM threads t JOIN boards b ON t.board_id = b.id WHERE t.id = ?1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        let Some(thread) = thread else {
            return Ok(None);
        };

        let messages = sqlx::query_as::<_, MessageRow>(
            "SELECT id, thread_id, author, content, created_at FROM messages WHERE thread_id = ?1 ORDER BY created_at ASC",
        )
        .bind(id)
        .fetch_all(&self.pool)
        .await?;

        let markers = self.load_markers(id).await?;

        Ok(Some(ThreadDetail {
            id,
            board: thread.board,
            title: thread.title,
            created_at: thread.created_at,
            markers,
            messages: messages.into_iter().map(Message::from).collect(),
        }))
    }

    pub async fn send_inbox(&self, persona: &str, content: &str) -> Result<InboxMessage, AppError> {
        let created_at = Utc::now();
        let result = sqlx::query(
            "INSERT INTO inbox_messages (persona, content, created_at) VALUES (?1, ?2, ?3)",
        )
        .bind(persona)
        .bind(content)
        .bind(created_at)
        .execute(&self.pool)
        .await?;

        let id = result.last_insert_rowid();
        Ok(InboxMessage {
            id,
            persona: persona.to_string(),
            content: content.to_string(),
            created_at,
        })
    }

    pub async fn fetch_inbox(&self, persona: &str) -> Result<Vec<InboxMessage>, AppError> {
        let rows = sqlx::query_as::<_, InboxRow>(
            "SELECT id, persona, content, created_at FROM inbox_messages WHERE persona = ?1 ORDER BY created_at DESC",
        )
        .bind(persona)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(InboxMessage::from).collect())
    }

    pub async fn delete_inbox(&self, persona: &str, id: i64) -> Result<(), AppError> {
        let count = sqlx::query("DELETE FROM inbox_messages WHERE persona = ?1 AND id = ?2")
            .bind(persona)
            .bind(id)
            .execute(&self.pool)
            .await?
            .rows_affected();

        if count == 0 {
            return Err(AppError::NotFound(format!("inbox message {id}")));
        }
        Ok(())
    }

    pub async fn put_common(
        &self,
        key: &str,
        value: serde_json::Value,
        ttl_seconds: Option<u64>,
    ) -> Result<CommonItem, AppError> {
        let created_at = Utc::now();
        let expires_at = ttl_seconds.map(|ttl| created_at + Duration::seconds(ttl as i64));
        sqlx::query(
            "INSERT INTO common_items (key, value, created_at, expires_at) VALUES (?1, ?2, ?3, ?4)\n             ON CONFLICT(key) DO UPDATE SET value = excluded.value, expires_at = excluded.expires_at",
        )
        .bind(key)
        .bind(
            serde_json::to_string(&value)
                .map_err(|err| AppError::BadRequest(err.to_string()))?,
        )
        .bind(created_at)
        .bind(expires_at)
        .execute(&self.pool)
        .await?;

        self.get_common(key)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("common item {key}")))
    }

    pub async fn list_common(&self) -> Result<Vec<CommonItem>, AppError> {
        self.cleanup_expired().await?;
        let rows = sqlx::query_as::<_, CommonRow>(
            "SELECT id, key, value, created_at, expires_at FROM common_items ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(|row| row.try_into()).collect()
    }

    pub async fn get_common(&self, key: &str) -> Result<Option<CommonItem>, AppError> {
        self.cleanup_expired().await?;
        let row = sqlx::query_as::<_, CommonRow>(
            "SELECT id, key, value, created_at, expires_at FROM common_items WHERE key = ?1",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await?;

        row.map(|row| row.try_into()).transpose()
    }

    async fn cleanup_expired(&self) -> Result<(), AppError> {
        sqlx::query("DELETE FROM common_items WHERE expires_at IS NOT NULL AND expires_at <= ?1")
            .bind(Utc::now())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn load_markers(&self, thread_id: i64) -> Result<Vec<Marker>, AppError> {
        let rows = sqlx::query_as::<_, MarkerRow>(
            "SELECT thread_id, kind, value FROM markers WHERE thread_id = ?1",
        )
        .bind(thread_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|row| Marker {
                kind: row.kind,
                value: row.value,
            })
            .collect())
    }
}

#[derive(FromRow)]
struct BoardRow {
    name: String,
    created_at: DateTime<Utc>,
}

impl From<BoardRow> for Board {
    fn from(row: BoardRow) -> Self {
        Self {
            name: row.name,
            created_at: row.created_at,
        }
    }
}

#[derive(FromRow)]
struct ThreadRow {
    id: i64,
    board: String,
    title: String,
    created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct MessageRow {
    id: i64,
    thread_id: i64,
    author: Option<String>,
    content: String,
    created_at: DateTime<Utc>,
}

impl From<MessageRow> for Message {
    fn from(row: MessageRow) -> Self {
        Self {
            id: row.id,
            thread_id: row.thread_id,
            author: row.author,
            content: row.content,
            created_at: row.created_at,
        }
    }
}

#[derive(FromRow)]
struct InboxRow {
    id: i64,
    persona: String,
    content: String,
    created_at: DateTime<Utc>,
}

impl From<InboxRow> for InboxMessage {
    fn from(row: InboxRow) -> Self {
        Self {
            id: row.id,
            persona: row.persona,
            content: row.content,
            created_at: row.created_at,
        }
    }
}

#[derive(FromRow)]
struct MarkerRow {
    thread_id: i64,
    kind: String,
    value: String,
}

#[derive(FromRow)]
struct CommonRow {
    id: i64,
    key: String,
    value: String,
    created_at: DateTime<Utc>,
    expires_at: Option<DateTime<Utc>>,
}

impl TryFrom<CommonRow> for CommonItem {
    type Error = AppError;

    fn try_from(row: CommonRow) -> Result<Self, Self::Error> {
        let value = serde_json::from_str(&row.value)
            .map_err(|err| AppError::BadRequest(err.to_string()))?;
        Ok(CommonItem {
            id: row.id,
            key: row.key,
            value,
            created_at: row.created_at,
            expires_at: row.expires_at,
        })
    }
}

fn extract_markers(content: &str) -> Vec<Marker> {
    static MARKER_RE: Lazy<Regex> = Lazy::new(|| {
        Regex::new(r"(?P<kind>ctx|project|mode|bridge)::(?P<value>[A-Za-z0-9_-]+)").unwrap()
    });

    MARKER_RE
        .captures_iter(content)
        .filter_map(|cap| {
            let kind = cap.name("kind")?.as_str().to_string();
            let value = cap.name("value")?.as_str().to_string();
            Some(Marker { kind, value })
        })
        .collect()
}

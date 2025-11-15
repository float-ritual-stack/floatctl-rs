use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;
use std::str::FromStr;
use std::time::Duration;

use crate::block::{Annotation, Block, BlockId, BoardId};

/// SQLite-backed block storage
/// Design philosophy: Simple, fast, no enterprise bloat
pub struct BlockStore {
    pool: SqlitePool,
}

impl BlockStore {
    /// Create a new BlockStore with the given database path
    pub async fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let db_path = db_path.as_ref();

        // Ensure parent directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        // Create connection options
        let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .busy_timeout(Duration::from_secs(5)) // Prevent SQLITE_BUSY errors with concurrent access
            .synchronous(sqlx::sqlite::SqliteSynchronous::Normal) // WAL mode allows relaxed sync
            .foreign_keys(true) // Enable foreign key constraints
            .pragma("cache_size", "-64000"); // 64MB cache for better read performance

        // Create pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        // Run schema (multi-statement SQL)
        let schema = include_str!("schema.sql");
        // Use raw query execution for multi-statement SQL
        sqlx::raw_sql(schema).execute(&pool).await?;

        Ok(Self { pool })
    }

    /// Insert a block into the store
    pub async fn insert(&self, block: &Block) -> Result<()> {
        let mut tx = self.pool.begin().await?;

        // Serialize block to JSON
        let content = serde_json::to_value(block)?;
        let block_id = block.id().to_string();
        let block_type = Self::block_type_name(block);
        let timestamp = block.timestamp().to_rfc3339();

        // Insert main block
        sqlx::query("INSERT INTO blocks (id, block_type, content, timestamp) VALUES (?, ?, ?, ?)")
            .bind(&block_id)
            .bind(block_type)
            .bind(&content)
            .bind(&timestamp)
            .execute(&mut *tx)
            .await?;

        // Extract and insert annotations if ContextEntry (batch insert to avoid N+1)
        if let Block::ContextEntry { annotations, .. } = block {
            if !annotations.is_empty() {
                let mut builder = sqlx::QueryBuilder::new(
                    "INSERT INTO annotations (block_id, annotation_key, annotation_value) ",
                );
                builder.push_values(annotations.iter(), |mut b, ann| {
                    b.push_bind(&block_id)
                        .push_bind(ann.key())
                        .push_bind(ann.value());
                });
                builder.build().execute(&mut *tx).await?;
            }
        }

        // Insert agent post metadata if AgentPost
        if let Block::AgentPost {
            agent,
            board,
            title,
            ..
        } = block
        {
            sqlx::query(
                "INSERT INTO agent_posts (id, agent_id, board_id, block_id, title, timestamp) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(&block_id)
            .bind(agent.to_string())
            .bind(board.to_string())
            .bind(&block_id)
            .bind(title.as_ref())
            .bind(&timestamp)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    /// Get a block by ID
    pub async fn get(&self, id: BlockId) -> Result<Option<Block>> {
        let row = sqlx::query("SELECT content FROM blocks WHERE id = ?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(row) => {
                let content: serde_json::Value = row.try_get("content")?;
                let block: Block = serde_json::from_value(content)?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    /// Query blocks by annotation
    pub async fn query_by_annotation(
        &self,
        annotation: &Annotation,
        limit: usize,
    ) -> Result<Vec<Block>> {
        let rows = sqlx::query(
            r#"
            SELECT DISTINCT b.content
            FROM blocks b
            JOIN annotations a ON b.id = a.block_id
            WHERE a.annotation_key = ? AND a.annotation_value = ?
            ORDER BY b.timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(annotation.key())
        .bind(annotation.value())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let content: serde_json::Value = row.try_get("content")?;
                let block: Block = serde_json::from_value(content)?;
                Ok(block)
            })
            .collect()
    }

    /// Query agent posts for a board
    pub async fn query_board(&self, board: &BoardId, limit: usize) -> Result<Vec<Block>> {
        let rows = sqlx::query(
            r#"
            SELECT b.content
            FROM blocks b
            JOIN agent_posts ap ON b.id = ap.block_id
            WHERE ap.board_id = ?
            ORDER BY ap.timestamp DESC
            LIMIT ?
            "#,
        )
        .bind(board.to_string())
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let content: serde_json::Value = row.try_get("content")?;
                let block: Block = serde_json::from_value(content)?;
                Ok(block)
            })
            .collect()
    }

    /// Query recent blocks (for /recent/ board)
    pub async fn query_recent(&self, limit: usize) -> Result<Vec<Block>> {
        let rows = sqlx::query("SELECT content FROM blocks ORDER BY timestamp DESC LIMIT ?")
            .bind(limit as i64)
            .fetch_all(&self.pool)
            .await?;

        rows.iter()
            .map(|row| {
                let content: serde_json::Value = row.try_get("content")?;
                let block: Block = serde_json::from_value(content)?;
                Ok(block)
            })
            .collect()
    }

    /// Full-text search across blocks
    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<Block>> {
        // Sanitize FTS5 query by escaping special characters
        let sanitized_query = Self::sanitize_fts_query(query);

        let rows = sqlx::query(
            r#"
            SELECT b.content
            FROM blocks b
            JOIN blocks_fts fts ON b.id = fts.block_id
            WHERE blocks_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#,
        )
        .bind(sanitized_query)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|row| {
                let content: serde_json::Value = row.try_get("content")?;
                let block: Block = serde_json::from_value(content)?;
                Ok(block)
            })
            .collect()
    }

    /// Count total blocks
    pub async fn count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM blocks")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("count")?)
    }

    /// Sanitize FTS5 query to prevent injection and syntax errors
    /// Uses whitelist approach: only allows alphanumeric, spaces, and basic punctuation
    /// All input is treated as literal search terms (no FTS5 operators)
    fn sanitize_fts_query(query: &str) -> String {
        // Whitelist: alphanumeric + spaces + safe punctuation
        // Replace non-whitelisted chars with spaces to maintain word boundaries
        let cleaned: String = query
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c.is_whitespace() || matches!(c, '-' | '_' | '.') {
                    c
                } else {
                    ' ' // Replace special chars with spaces
                }
            })
            .collect();

        // Collapse multiple spaces and trim
        let normalized = cleaned.split_whitespace().collect::<Vec<_>>().join(" ");

        // Wrap in quotes to treat as literal phrase (prevents operator injection)
        // Empty string check to avoid invalid FTS5 query
        if normalized.is_empty() {
            // Return a query that matches nothing safely
            "\"\"".to_string()
        } else {
            // Quote the entire query to make it literal
            format!("\"{}\"", normalized)
        }
    }

    /// Get block type name for storage
    fn block_type_name(block: &Block) -> &'static str {
        match block {
            Block::Text { .. } => "text",
            Block::ContextEntry { .. } => "context_entry",
            Block::AgentPost { .. } => "agent_post",
            Block::Component { .. } => "component",
            Block::Code { .. } => "code",
            Block::Link { .. } => "link",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::AgentId;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_create_store() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");

        let store = BlockStore::new(&db_path).await.unwrap();
        let count = store.count().await.unwrap();
        assert_eq!(count, 0);
    }

    #[tokio::test]
    async fn test_insert_and_get() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = BlockStore::new(&db_path).await.unwrap();

        let block = Block::new_text("hello world".into());
        let id = block.id();

        store.insert(&block).await.unwrap();

        let retrieved = store.get(id).await.unwrap().unwrap();
        match retrieved {
            Block::Text { content, .. } => assert_eq!(content, "hello world"),
            _ => panic!("Wrong block type"),
        }
    }

    #[tokio::test]
    async fn test_query_by_annotation() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = BlockStore::new(&db_path).await.unwrap();

        let block = Block::new_context_entry(
            "ctx::2025-11-15".into(),
            vec!["test".into()],
            vec![Annotation::Project("test-project".into())],
        );

        store.insert(&block).await.unwrap();

        let results = store
            .query_by_annotation(&Annotation::Project("test-project".into()), 10)
            .await
            .unwrap();

        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_query_board() {
        let dir = tempdir().unwrap();
        let db_path = dir.path().join("test.db");
        let store = BlockStore::new(&db_path).await.unwrap();

        let post = Block::new_agent_post(
            AgentId::Evna,
            BoardId::Work,
            Some("Test post".into()),
            vec![Block::new_text("content".into())],
            vec![],
        );

        store.insert(&post).await.unwrap();

        let results = store.query_board(&BoardId::Work, 10).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_sanitize_fts_query_normal_input() {
        assert_eq!(
            BlockStore::sanitize_fts_query("hello world"),
            "\"hello world\""
        );
    }

    #[test]
    fn test_sanitize_fts_query_special_chars() {
        // FTS5 operators should be stripped
        assert_eq!(
            BlockStore::sanitize_fts_query("OR * DROP AND NOT"),
            "\"OR DROP AND NOT\""
        );
    }

    #[test]
    fn test_sanitize_fts_query_empty() {
        assert_eq!(BlockStore::sanitize_fts_query(""), "\"\"");
    }

    #[test]
    fn test_sanitize_fts_query_whitespace_only() {
        assert_eq!(BlockStore::sanitize_fts_query("   "), "\"\"");
    }

    #[test]
    fn test_sanitize_fts_query_collapses_spaces() {
        assert_eq!(
            BlockStore::sanitize_fts_query("hello    world   test"),
            "\"hello world test\""
        );
    }

    #[test]
    fn test_sanitize_fts_query_safe_punctuation() {
        // Allowed: alphanumeric, -, _, .
        assert_eq!(
            BlockStore::sanitize_fts_query("hello-world_test.rs"),
            "\"hello-world_test.rs\""
        );
    }

    #[test]
    fn test_sanitize_fts_query_removes_parentheses() {
        assert_eq!(
            BlockStore::sanitize_fts_query("(hello OR world)"),
            "\"hello OR world\""
        );
    }

    #[test]
    fn test_sanitize_fts_query_removes_braces() {
        assert_eq!(
            BlockStore::sanitize_fts_query("{column}:search"),
            "\"column search\""
        );
    }
}

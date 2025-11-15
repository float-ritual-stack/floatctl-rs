use anyhow::Result;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use sqlx::Row;
use std::path::Path;
use std::str::FromStr;

use crate::block::{AgentId, Annotation, Block, BlockId, BoardId};

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
        let options = SqliteConnectOptions::from_str(
            &format!("sqlite://{}", db_path.display())
        )?
        .create_if_missing(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        // Create pool
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(options)
            .await?;

        // Run schema
        let schema = include_str!("schema.sql");
        sqlx::query(schema).execute(&pool).await?;

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
        sqlx::query(
            "INSERT INTO blocks (id, block_type, content, timestamp) VALUES (?, ?, ?, ?)"
        )
        .bind(&block_id)
        .bind(&block_type)
        .bind(&content)
        .bind(&timestamp)
        .execute(&mut *tx)
        .await?;

        // Extract and insert annotations if ContextEntry
        if let Block::ContextEntry { annotations, .. } = block {
            for annotation in annotations {
                sqlx::query(
                    "INSERT INTO annotations (block_id, annotation_key, annotation_value) VALUES (?, ?, ?)"
                )
                .bind(&block_id)
                .bind(annotation.key())
                .bind(annotation.value())
                .execute(&mut *tx)
                .await?;
            }
        }

        // Insert agent post metadata if AgentPost
        if let Block::AgentPost { agent, board, title, .. } = block {
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
        let row = sqlx::query(
            "SELECT content FROM blocks WHERE id = ?"
        )
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
            "#
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
    pub async fn query_board(
        &self,
        board: &BoardId,
        limit: usize,
    ) -> Result<Vec<Block>> {
        let rows = sqlx::query(
            r#"
            SELECT b.content
            FROM blocks b
            JOIN agent_posts ap ON b.id = ap.block_id
            WHERE ap.board_id = ?
            ORDER BY ap.timestamp DESC
            LIMIT ?
            "#
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
        let rows = sqlx::query(
            "SELECT content FROM blocks ORDER BY timestamp DESC LIMIT ?"
        )
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
        let rows = sqlx::query(
            r#"
            SELECT b.content
            FROM blocks b
            JOIN blocks_fts fts ON b.id = fts.block_id
            WHERE blocks_fts MATCH ?
            ORDER BY rank
            LIMIT ?
            "#
        )
        .bind(query)
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
}

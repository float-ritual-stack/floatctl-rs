//! Thread repository - Spec 1.3
//!
//! Handles thread CRUD with:
//! - Atomic creation with first message (transaction)
//! - Paginated listing

use sqlx::{PgPool, FromRow, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::{BoardName, ThreadTitle, MessageContent, Pagination, Paginated};
use super::DbError;

/// Thread record from database
#[derive(Debug, Clone, FromRow)]
pub struct Thread {
    pub id: Uuid,
    pub board_name: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
}

/// Thread with message count for list display
#[derive(Debug, Clone)]
pub struct ThreadWithCount {
    pub id: Uuid,
    pub board_name: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub message_count: i64,
}

/// Thread repository
pub struct ThreadRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ThreadRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Create thread with optional first message (atomic).
    ///
    /// Uses transaction to ensure thread + message are created together
    /// or neither is created.
    pub async fn create_with_message(
        &self,
        board: BoardName,
        title: ThreadTitle,
        first_message: Option<(MessageContent, Option<String>)>,
    ) -> Result<Thread, DbError> {
        let mut tx = self.pool.begin().await?;

        // Verify board exists
        let board_exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM boards WHERE name = $1)",
        )
        .bind(board.as_str())
        .fetch_one(&mut *tx)
        .await?;

        if !board_exists.0 {
            return Err(DbError::NotFound {
                resource: "board",
                id: board.as_str().to_owned(),
            });
        }

        // Insert thread
        let thread: Thread = sqlx::query_as(
            r#"
            INSERT INTO threads (board_name, title)
            VALUES ($1, $2)
            RETURNING id, board_name, title, created_at
            "#,
        )
        .bind(board.as_str())
        .bind(title.as_str())
        .fetch_one(&mut *tx)
        .await?;

        // Insert first message if provided
        if let Some((content, author)) = first_message {
            let message_row = sqlx::query(
                r#"
                INSERT INTO thread_messages (thread_id, content, author)
                VALUES ($1, $2, $3)
                RETURNING id
                "#,
            )
            .bind(thread.id)
            .bind(content.as_str())
            .bind(author.as_deref())
            .fetch_one(&mut *tx)
            .await?;

            let message_id: Uuid = message_row.get("id");

            // Extract and insert markers
            for marker in content.extract_markers() {
                sqlx::query(
                    r#"
                    INSERT INTO message_markers (message_id, kind, value)
                    VALUES ($1, $2, $3)
                    ON CONFLICT DO NOTHING
                    "#,
                )
                .bind(message_id)
                .bind(marker.kind.as_str())
                .bind(&marker.value)
                .execute(&mut *tx)
                .await?;
            }
        }

        tx.commit().await?;
        Ok(thread)
    }

    /// List threads for a board with message counts.
    pub async fn list_for_board(
        &self,
        board_name: &str,
        page: Pagination,
    ) -> Result<Paginated<ThreadWithCount>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT
                t.id,
                t.board_name,
                t.title,
                t.created_at,
                COUNT(m.id) as message_count,
                COUNT(*) OVER() as total
            FROM threads t
            LEFT JOIN thread_messages m ON m.thread_id = t.id
            WHERE t.board_name = $1
            GROUP BY t.id, t.board_name, t.title, t.created_at
            ORDER BY t.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(board_name)
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(self.pool)
        .await?;

        let total = rows.first().map(|r| r.get::<i64, _>("total")).unwrap_or(0);
        let items = rows
            .into_iter()
            .map(|r| ThreadWithCount {
                id: r.get("id"),
                board_name: r.get("board_name"),
                title: r.get("title"),
                created_at: r.get("created_at"),
                message_count: r.get("message_count"),
            })
            .collect();

        Ok(Paginated {
            items,
            total,
            page: page.page,
            per_page: page.per_page,
        })
    }

    /// Get a single thread by ID.
    pub async fn get(&self, id: Uuid) -> Result<Thread, DbError> {
        let thread: Thread = sqlx::query_as(
            r#"
            SELECT id, board_name, title, created_at
            FROM threads
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound {
            resource: "thread",
            id: id.to_string(),
        })?;

        Ok(thread)
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore = "requires database"]
    async fn create_thread_transaction_rollback() {
        // If message insert fails, thread should NOT be created
        // TODO: Implement by triggering a constraint violation
    }
}

//! Board repository - Spec 1.3
//!
//! Handles board CRUD with proper patterns:
//! - create: INSERT with ON CONFLICT (idempotent)
//! - list: JOIN with thread count (no N+1)

use sqlx::{PgPool, FromRow, Row};
use chrono::{DateTime, Utc};

use crate::models::{BoardName, Pagination, Paginated};

/// Board record from database
#[derive(Debug, Clone, FromRow)]
pub struct Board {
    pub name: String,
    pub created_at: DateTime<Utc>,
}

/// Board with thread count for list display
#[derive(Debug, Clone)]
pub struct BoardWithCount {
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub thread_count: i64,
}

/// Database error type
#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error("database error: {0}")]
    Sqlx(#[from] sqlx::Error),

    #[error("not found: {resource} '{id}'")]
    NotFound { resource: &'static str, id: String },
}

/// Board repository
pub struct BoardRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> BoardRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Create a board, returning existing on conflict (idempotent).
    ///
    /// Uses CTE to insert/upsert and then JOIN for thread count in single query.
    /// Returns BoardWithCount directly to avoid double-query in handler.
    pub async fn create(&self, name: BoardName) -> Result<BoardWithCount, DbError> {
        // Single query: CTE for upsert + JOIN for thread count
        let row = sqlx::query(
            r#"
            WITH upserted AS (
                INSERT INTO boards (name) VALUES ($1)
                ON CONFLICT (name) DO UPDATE SET name = EXCLUDED.name
                RETURNING name, created_at
            )
            SELECT u.name, u.created_at, COUNT(t.id) as thread_count
            FROM upserted u
            LEFT JOIN threads t ON t.board_name = u.name
            GROUP BY u.name, u.created_at
            "#,
        )
        .bind(name.as_str())
        .fetch_one(self.pool)
        .await?;

        Ok(BoardWithCount {
            name: row.get("name"),
            created_at: row.get("created_at"),
            thread_count: row.get("thread_count"),
        })
    }

    /// List boards with thread counts.
    ///
    /// Uses LEFT JOIN to get counts in a single query (no N+1).
    pub async fn list(&self, page: Pagination) -> Result<Paginated<BoardWithCount>, DbError> {
        // Single query with COUNT(*) OVER() for total
        let rows = sqlx::query(
            r#"
            SELECT
                b.name,
                b.created_at,
                COUNT(t.id) as thread_count,
                COUNT(*) OVER() as total
            FROM boards b
            LEFT JOIN threads t ON t.board_name = b.name
            GROUP BY b.name, b.created_at
            ORDER BY b.created_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(self.pool)
        .await?;

        let total = rows.first().map(|r| r.get::<i64, _>("total")).unwrap_or(0);
        let items = rows
            .into_iter()
            .map(|r| BoardWithCount {
                name: r.get("name"),
                created_at: r.get("created_at"),
                thread_count: r.get("thread_count"),
            })
            .collect();

        Ok(Paginated {
            items,
            total,
            page: page.page,
            per_page: page.per_page,
        })
    }

    /// Get a single board by name with thread count.
    pub async fn get(&self, name: &str) -> Result<BoardWithCount, DbError> {
        let row = sqlx::query(
            r#"
            SELECT
                b.name,
                b.created_at,
                COUNT(t.id) as thread_count
            FROM boards b
            LEFT JOIN threads t ON t.board_name = b.name
            WHERE b.name = $1
            GROUP BY b.name, b.created_at
            "#,
        )
        .bind(name)
        .fetch_optional(self.pool)
        .await?
        .ok_or_else(|| DbError::NotFound {
            resource: "board",
            id: name.to_owned(),
        })?;

        Ok(BoardWithCount {
            name: row.get("name"),
            created_at: row.get("created_at"),
            thread_count: row.get("thread_count"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests - run with DATABASE_URL set
    // cargo test -p floatctl-server -- --ignored

    #[tokio::test]
    #[ignore = "requires database"]
    async fn list_boards_single_query() {
        // This test should verify via sqlx query logging
        // that list() executes exactly 1 query
        // TODO: Implement with query logging
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn create_board_idempotent() {
        // Creating same board twice should return same record
        // TODO: Implement
    }
}

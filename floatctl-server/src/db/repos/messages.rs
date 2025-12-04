//! Message repository - Spec 1.3
//!
//! Handles message CRUD with marker extraction.

use sqlx::{PgPool, FromRow, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::{MessageContent, Pagination, Paginated, MarkerKind};
use super::DbError;

/// Message record from database
#[derive(Debug, Clone, FromRow)]
pub struct Message {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub content: String,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Message with extracted markers
#[derive(Debug, Clone)]
pub struct MessageWithMarkers {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub content: String,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
    pub markers: Vec<(MarkerKind, String)>,
}

/// Message repository
pub struct MessageRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> MessageRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Add message to thread, extracting and storing markers.
    pub async fn create(
        &self,
        thread_id: Uuid,
        content: MessageContent,
        author: Option<String>,
    ) -> Result<Message, DbError> {
        // Verify thread exists
        let thread_exists: (bool,) = sqlx::query_as(
            "SELECT EXISTS(SELECT 1 FROM threads WHERE id = $1)",
        )
        .bind(thread_id)
        .fetch_one(self.pool)
        .await?;

        if !thread_exists.0 {
            return Err(DbError::NotFound {
                resource: "thread",
                id: thread_id.to_string(),
            });
        }

        let mut tx = self.pool.begin().await?;

        // Insert message
        let message: Message = sqlx::query_as(
            r#"
            INSERT INTO thread_messages (thread_id, content, author)
            VALUES ($1, $2, $3)
            RETURNING id, thread_id, content, author, created_at
            "#,
        )
        .bind(thread_id)
        .bind(content.as_str())
        .bind(author.as_deref())
        .fetch_one(&mut *tx)
        .await?;

        // Extract and insert markers
        for marker in content.extract_markers() {
            sqlx::query(
                r#"
                INSERT INTO message_markers (message_id, kind, value)
                VALUES ($1, $2, $3)
                ON CONFLICT DO NOTHING
                "#,
            )
            .bind(message.id)
            .bind(marker.kind.as_str())
            .bind(&marker.value)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(message)
    }

    /// List messages for a thread with pagination.
    ///
    /// Uses window function for total count in single query.
    pub async fn list_for_thread(
        &self,
        thread_id: Uuid,
        page: Pagination,
    ) -> Result<Paginated<Message>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                thread_id,
                content,
                author,
                created_at,
                COUNT(*) OVER() as total
            FROM thread_messages
            WHERE thread_id = $1
            ORDER BY created_at ASC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(thread_id)
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(self.pool)
        .await?;

        let total = rows.first().map(|r| r.get::<i64, _>("total")).unwrap_or(0);
        let items = rows
            .into_iter()
            .map(|r| Message {
                id: r.get("id"),
                thread_id: r.get("thread_id"),
                content: r.get("content"),
                author: r.get("author"),
                created_at: r.get("created_at"),
            })
            .collect();

        Ok(Paginated {
            items,
            total,
            page: page.page,
            per_page: page.per_page,
        })
    }

    /// Search threads by markers (AND semantics).
    pub async fn search_by_markers(
        &self,
        filters: &[(MarkerKind, String)],
        page: Pagination,
    ) -> Result<Paginated<Uuid>, DbError> {
        if filters.is_empty() {
            return Ok(Paginated {
                items: vec![],
                total: 0,
                page: page.page,
                per_page: page.per_page,
            });
        }

        // Build dynamic query with EXISTS for each filter
        // This is safe because MarkerKind is an enum, not user input
        let mut query = String::from(
            r#"
            SELECT DISTINCT t.id, COUNT(*) OVER() as total
            FROM threads t
            WHERE 1=1
            "#,
        );

        for (i, (_kind, _)) in filters.iter().enumerate() {
            query.push_str(&format!(
                r#"
                AND EXISTS (
                    SELECT 1 FROM message_markers mm
                    JOIN thread_messages m ON mm.message_id = m.id
                    WHERE m.thread_id = t.id
                    AND mm.kind = ${}
                    AND mm.value = ${}
                )
                "#,
                i * 2 + 1,
                i * 2 + 2
            ));
        }

        query.push_str(&format!(
            "ORDER BY t.id LIMIT ${} OFFSET ${}",
            filters.len() * 2 + 1,
            filters.len() * 2 + 2
        ));

        // Build and execute query - use query() not query_scalar() to get both id and total
        let mut builder = sqlx::query(&query);
        for (kind, value) in filters {
            builder = builder.bind(kind.as_str()).bind(value);
        }
        builder = builder.bind(page.limit() as i64).bind(page.offset() as i64);

        let rows = builder.fetch_all(self.pool).await?;

        let total = rows.first().map(|r| r.get::<i64, _>("total")).unwrap_or(0);
        let thread_ids: Vec<Uuid> = rows.iter().map(|r| r.get("id")).collect();

        Ok(Paginated {
            total,
            items: thread_ids,
            page: page.page,
            per_page: page.per_page,
        })
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore = "requires database"]
    async fn message_extracts_markers() {
        // Content with "ctx::review project::api" should create markers
        // TODO: Implement
    }
}

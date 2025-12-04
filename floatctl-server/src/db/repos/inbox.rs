//! Inbox repository - Spec 3.2
//!
//! Per-persona async messaging inbox.

use sqlx::{PgPool, FromRow, Row};
use uuid::Uuid;
use chrono::{DateTime, Utc};

use crate::models::{Persona, MessageContent, Pagination, Paginated};
use super::DbError;

/// Inbox message record
#[derive(Debug, Clone, FromRow)]
pub struct InboxMessage {
    pub id: Uuid,
    pub persona: String,
    pub content: String,
    pub from_persona: Option<String>,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Inbox repository
pub struct InboxRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> InboxRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Send message to a persona's inbox.
    pub async fn send(
        &self,
        to: Persona,
        content: MessageContent,
        from: Option<Persona>,
    ) -> Result<InboxMessage, DbError> {
        let message: InboxMessage = sqlx::query_as(
            r#"
            INSERT INTO inboxes (persona, content, from_persona)
            VALUES ($1, $2, $3)
            RETURNING id, persona, content, from_persona, read_at, created_at
            "#,
        )
        .bind(to.as_str())
        .bind(content.as_str())
        .bind(from.map(|p| p.as_str().to_owned()))
        .fetch_one(self.pool)
        .await?;

        Ok(message)
    }

    /// List unread messages for a persona.
    pub async fn list_unread(
        &self,
        persona: Persona,
        page: Pagination,
    ) -> Result<Paginated<InboxMessage>, DbError> {
        let rows = sqlx::query(
            r#"
            SELECT
                id,
                persona,
                content,
                from_persona,
                read_at,
                created_at,
                COUNT(*) OVER() as total
            FROM inboxes
            WHERE persona = $1 AND read_at IS NULL
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(persona.as_str())
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(self.pool)
        .await?;

        let total = rows.first().map(|r| r.get::<i64, _>("total")).unwrap_or(0);
        let items = rows
            .into_iter()
            .map(|r| InboxMessage {
                id: r.get("id"),
                persona: r.get("persona"),
                content: r.get("content"),
                from_persona: r.get("from_persona"),
                read_at: r.get("read_at"),
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

    /// Mark message as read / delete.
    ///
    /// Idempotent - returns Ok even if already deleted.
    pub async fn delete(&self, persona: Persona, message_id: Uuid) -> Result<(), DbError> {
        sqlx::query(
            r#"
            DELETE FROM inboxes
            WHERE id = $1 AND persona = $2
            "#,
        )
        .bind(message_id)
        .bind(persona.as_str())
        .execute(self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore = "requires database"]
    async fn inbox_isolation() {
        // kitty's inbox shouldn't show cowboy's messages
        // TODO: Implement
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn delete_idempotent() {
        // DELETE same message twice should succeed both times
        // TODO: Implement
    }
}

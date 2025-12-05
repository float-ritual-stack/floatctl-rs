//! Scratchpad repository - Spec 3.3
//!
//! Shared key-value store with optional TTL.

use std::sync::atomic::{AtomicI64, Ordering};
use once_cell::sync::Lazy;
use sqlx::{PgPool, FromRow};
use chrono::{DateTime, Utc, Duration};
use serde_json::Value as JsonValue;

use crate::models::Pagination;
use super::DbError;

/// Last cleanup timestamp for throttling
static LAST_CLEANUP: Lazy<AtomicI64> = Lazy::new(|| AtomicI64::new(0));

/// Minimum interval between cleanup spawns (60 seconds)
const CLEANUP_INTERVAL_SECS: i64 = 60;

/// Maximum TTL in seconds (1 year)
const MAX_TTL_SECS: i64 = 31_536_000;

/// Scratchpad item record
#[derive(Debug, Clone, FromRow)]
pub struct ScratchpadItem {
    pub key: String,
    pub value: JsonValue,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Scratchpad repository
pub struct ScratchpadRepo<'a> {
    pool: &'a PgPool,
}

impl<'a> ScratchpadRepo<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Upsert a key-value pair with optional TTL.
    ///
    /// TTL is capped at MAX_TTL_SECS (1 year) to prevent overflow.
    pub async fn upsert(
        &self,
        key: &str,
        value: JsonValue,
        ttl_seconds: Option<i64>,
    ) -> Result<ScratchpadItem, DbError> {
        // Cap TTL to prevent datetime overflow, use checked_add for safety
        let expires_at = ttl_seconds.and_then(|s| {
            let capped = s.clamp(0, MAX_TTL_SECS);
            Utc::now().checked_add_signed(Duration::seconds(capped))
        });

        let item: ScratchpadItem = sqlx::query_as(
            r#"
            INSERT INTO scratchpad (key, value, expires_at)
            VALUES ($1, $2, $3)
            ON CONFLICT (key) DO UPDATE
            SET value = EXCLUDED.value,
                expires_at = EXCLUDED.expires_at,
                updated_at = NOW()
            RETURNING key, value, expires_at, created_at, updated_at
            "#,
        )
        .bind(key)
        .bind(&value)
        .bind(expires_at)
        .fetch_one(self.pool)
        .await?;

        Ok(item)
    }

    /// Get a single item by key.
    ///
    /// Returns None if expired or not found.
    pub async fn get(&self, key: &str) -> Result<Option<ScratchpadItem>, DbError> {
        // Spawn cleanup (non-blocking)
        self.spawn_cleanup();

        let item: Option<ScratchpadItem> = sqlx::query_as(
            r#"
            SELECT key, value, expires_at, created_at, updated_at
            FROM scratchpad
            WHERE key = $1
            AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(key)
        .fetch_optional(self.pool)
        .await?;

        Ok(item)
    }

    /// List all non-expired items.
    pub async fn list(&self, page: Pagination) -> Result<Vec<ScratchpadItem>, DbError> {
        // Spawn cleanup (non-blocking)
        self.spawn_cleanup();

        let items: Vec<ScratchpadItem> = sqlx::query_as(
            r#"
            SELECT key, value, expires_at, created_at, updated_at
            FROM scratchpad
            WHERE expires_at IS NULL OR expires_at > NOW()
            ORDER BY updated_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(page.limit() as i64)
        .bind(page.offset() as i64)
        .fetch_all(self.pool)
        .await?;

        Ok(items)
    }

    /// Delete an item by key (idempotent).
    pub async fn delete(&self, key: &str) -> Result<(), DbError> {
        sqlx::query("DELETE FROM scratchpad WHERE key = $1")
            .bind(key)
            .execute(self.pool)
            .await?;
        Ok(())
    }

    /// Clean up expired items (non-blocking spawn).
    ///
    /// Throttled to run at most once per CLEANUP_INTERVAL_SECS to avoid
    /// excessive background tasks under load.
    fn spawn_cleanup(&self) {
        let now = Utc::now().timestamp();
        let last = LAST_CLEANUP.load(Ordering::Relaxed);

        // Only spawn if enough time has passed
        if now - last < CLEANUP_INTERVAL_SECS {
            return;
        }

        // Try to claim this cleanup slot (atomic compare-exchange)
        if LAST_CLEANUP
            .compare_exchange(last, now, Ordering::SeqCst, Ordering::Relaxed)
            .is_err()
        {
            return; // Another thread beat us to it
        }

        let pool: PgPool = (*self.pool).clone();
        tokio::spawn(async move {
            let _ = cleanup_expired(&pool).await;
        });
    }
}

/// Delete expired items from scratchpad.
pub async fn cleanup_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM scratchpad WHERE expires_at < NOW()")
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    #[ignore = "requires database"]
    async fn ttl_expiration() {
        // Item with ttl_seconds=1 should be gone after 2s
        // TODO: Implement
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn upsert_behavior() {
        // POST same key twice should update value
        // TODO: Implement
    }
}

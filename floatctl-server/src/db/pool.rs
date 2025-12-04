//! Database connection pool management
//!
//! Uses sqlx PgPool with explicit connection limits.
//! See Spec 1.1 in HTTP_SERVER_SPEC.md for requirements.

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Default maximum connections for the pool.
/// Kept low for single-user tooling.
const DEFAULT_MAX_CONNECTIONS: u32 = 5;

/// Create a PostgreSQL connection pool.
///
/// # Arguments
///
/// * `database_url` - PostgreSQL connection string
///
/// # Errors
///
/// Returns an error if the connection fails.
///
/// # Example
///
/// ```ignore
/// let pool = create_pool("postgres://localhost/floatctl").await?;
/// ```
pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    create_pool_with_options(database_url, DEFAULT_MAX_CONNECTIONS).await
}

/// Create a PostgreSQL connection pool with custom options.
///
/// # Arguments
///
/// * `database_url` - PostgreSQL connection string
/// * `max_connections` - Maximum number of connections in the pool
pub async fn create_pool_with_options(
    database_url: &str,
    max_connections: u32,
) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(max_connections)
        .connect(database_url)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    // Integration tests require a real database
    // Run with: DATABASE_URL=postgres://... cargo test -p floatctl-server

    #[tokio::test]
    #[ignore = "requires database"]
    async fn pool_acquires_connection() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = create_pool(&url).await.expect("pool creation failed");

        // Verify we can execute a query
        let result: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&pool)
            .await
            .expect("query failed");

        assert_eq!(result.0, 1);
    }

    #[tokio::test]
    #[ignore = "requires database"]
    async fn concurrent_pool_access() {
        let url = std::env::var("DATABASE_URL").expect("DATABASE_URL required");
        let pool = create_pool(&url).await.expect("pool creation failed");

        // Spawn 10 concurrent tasks
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let pool = pool.clone();
                tokio::spawn(async move {
                    let result: (i32,) = sqlx::query_as("SELECT $1::int")
                        .bind(i)
                        .fetch_one(&pool)
                        .await
                        .expect("concurrent query failed");
                    result.0
                })
            })
            .collect();

        // All tasks should complete successfully
        for (i, handle) in handles.into_iter().enumerate() {
            let result = handle.await.expect("task panicked");
            assert_eq!(result, i as i32);
        }
    }
}

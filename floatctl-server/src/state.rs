//! Application state shared across handlers

use sqlx::PgPool;
use std::sync::Arc;

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    inner: Arc<AppStateInner>,
}

struct AppStateInner {
    pub pool: PgPool,
}

impl AppState {
    pub fn new(pool: PgPool) -> Self {
        Self {
            inner: Arc::new(AppStateInner { pool }),
        }
    }

    pub fn pool(&self) -> &PgPool {
        &self.inner.pool
    }
}

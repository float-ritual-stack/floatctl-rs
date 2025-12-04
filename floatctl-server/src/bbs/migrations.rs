//! Database migrations for BBS tables

use sqlx::PgPool;

use crate::Result;

/// Run all BBS migrations
pub async fn run(pool: &PgPool) -> Result<()> {
    tracing::info!("Running BBS migrations...");

    // Create boards table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS bbs_boards (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            slug TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            visibility TEXT NOT NULL DEFAULT 'public',
            parent_id UUID REFERENCES bbs_boards(id) ON DELETE SET NULL,
            pinned BOOLEAN NOT NULL DEFAULT FALSE,
            archived BOOLEAN NOT NULL DEFAULT FALSE
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create threads table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS bbs_threads (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            board_id UUID NOT NULL REFERENCES bbs_boards(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            author TEXT NOT NULL,
            pinned BOOLEAN NOT NULL DEFAULT FALSE,
            locked BOOLEAN NOT NULL DEFAULT FALSE,
            status TEXT NOT NULL DEFAULT 'open',
            tags TEXT[] NOT NULL DEFAULT '{}'
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create posts table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS bbs_posts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            thread_id UUID NOT NULL REFERENCES bbs_threads(id) ON DELETE CASCADE,
            author TEXT NOT NULL,
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            reply_to UUID REFERENCES bbs_posts(id) ON DELETE SET NULL,
            edited BOOLEAN NOT NULL DEFAULT FALSE,
            deleted BOOLEAN NOT NULL DEFAULT FALSE,
            metadata JSONB
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create inbox table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS bbs_inbox (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            recipient TEXT NOT NULL,
            sender TEXT NOT NULL,
            subject TEXT,
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            read_at TIMESTAMPTZ,
            priority TEXT NOT NULL DEFAULT 'normal',
            thread_ref UUID REFERENCES bbs_threads(id) ON DELETE SET NULL,
            board_ref UUID REFERENCES bbs_boards(id) ON DELETE SET NULL,
            archived BOOLEAN NOT NULL DEFAULT FALSE
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create commons table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS bbs_commons (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            slug TEXT NOT NULL UNIQUE,
            name TEXT NOT NULL,
            description TEXT,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            common_type TEXT NOT NULL DEFAULT 'persistent',
            visibility TEXT NOT NULL DEFAULT 'public',
            expires_at TIMESTAMPTZ
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create artifacts table
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS bbs_artifacts (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            common_id UUID NOT NULL REFERENCES bbs_commons(id) ON DELETE CASCADE,
            author TEXT NOT NULL,
            artifact_type TEXT NOT NULL,
            title TEXT,
            content TEXT NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            metadata JSONB,
            pinned BOOLEAN NOT NULL DEFAULT FALSE
        )
        "#,
    )
    .execute(pool)
    .await?;

    // Create indexes for common queries
    create_indexes(pool).await?;

    tracing::info!("BBS migrations complete");
    Ok(())
}

async fn create_indexes(pool: &PgPool) -> Result<()> {
    // Board indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_bbs_boards_slug ON bbs_boards(slug)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_bbs_boards_parent ON bbs_boards(parent_id)")
        .execute(pool)
        .await?;

    // Thread indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_bbs_threads_board ON bbs_threads(board_id)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_bbs_threads_updated ON bbs_threads(updated_at DESC)",
    )
    .execute(pool)
    .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_bbs_threads_author ON bbs_threads(author)")
        .execute(pool)
        .await?;

    // Post indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_bbs_posts_thread ON bbs_posts(thread_id)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_bbs_posts_created ON bbs_posts(created_at)")
        .execute(pool)
        .await?;
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_bbs_posts_author ON bbs_posts(author)")
        .execute(pool)
        .await?;

    // Inbox indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_bbs_inbox_recipient ON bbs_inbox(recipient)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_bbs_inbox_unread ON bbs_inbox(recipient) WHERE read_at IS NULL",
    )
    .execute(pool)
    .await?;

    // Commons indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_bbs_commons_slug ON bbs_commons(slug)")
        .execute(pool)
        .await?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS idx_bbs_commons_expires ON bbs_commons(expires_at) WHERE expires_at IS NOT NULL",
    )
    .execute(pool)
    .await?;

    // Artifact indexes
    sqlx::query("CREATE INDEX IF NOT EXISTS idx_bbs_artifacts_common ON bbs_artifacts(common_id)")
        .execute(pool)
        .await?;

    Ok(())
}

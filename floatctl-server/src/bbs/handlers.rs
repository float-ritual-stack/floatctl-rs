//! BBS HTTP handlers

use axum::extract::{Path, Query, State};
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use crate::state::AppState;
use crate::{Error, Result};

use super::models::*;

// ============================================================================
// Boards
// ============================================================================

pub async fn list_boards(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<BoardSummary>>> {
    let offset = params.offset();
    let limit = params.per_page();

    let boards: Vec<Board> = sqlx::query_as(
        r#"
        SELECT * FROM bbs_boards
        WHERE archived = FALSE
        ORDER BY pinned DESC, updated_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(state.pool())
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bbs_boards WHERE archived = FALSE")
        .fetch_one(state.pool())
        .await?;

    let mut summaries = Vec::with_capacity(boards.len());
    for board in boards {
        let counts: (i64, i64) = sqlx::query_as(
            r#"
            SELECT
                COUNT(DISTINCT t.id),
                COUNT(p.id)
            FROM bbs_threads t
            LEFT JOIN bbs_posts p ON p.thread_id = t.id
            WHERE t.board_id = $1
            "#,
        )
        .bind(board.id)
        .fetch_one(state.pool())
        .await?;

        summaries.push(BoardSummary {
            board,
            thread_count: counts.0,
            post_count: counts.1,
        });
    }

    Ok(Json(Paginated {
        items: summaries,
        total: total.0,
        page: params.page(),
        per_page: limit,
    }))
}

pub async fn get_board(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<BoardSummary>> {
    let board: Board = sqlx::query_as("SELECT * FROM bbs_boards WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(state.pool())
        .await?
        .ok_or_else(|| Error::NotFound(format!("Board '{}' not found", slug)))?;

    let counts: (i64, i64) = sqlx::query_as(
        r#"
        SELECT
            COUNT(DISTINCT t.id),
            COUNT(p.id)
        FROM bbs_threads t
        LEFT JOIN bbs_posts p ON p.thread_id = t.id
        WHERE t.board_id = $1
        "#,
    )
    .bind(board.id)
    .fetch_one(state.pool())
    .await?;

    Ok(Json(BoardSummary {
        board,
        thread_count: counts.0,
        post_count: counts.1,
    }))
}

pub async fn create_board(
    State(state): State<AppState>,
    Json(input): Json<CreateBoard>,
) -> Result<Json<Board>> {
    let board: Board = sqlx::query_as(
        r#"
        INSERT INTO bbs_boards (slug, name, description, visibility, parent_id)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(&input.slug)
    .bind(&input.name)
    .bind(&input.description)
    .bind(input.visibility.unwrap_or_else(|| "public".to_string()))
    .bind(input.parent_id)
    .fetch_one(state.pool())
    .await?;

    Ok(Json(board))
}

pub async fn update_board(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Json(input): Json<UpdateBoard>,
) -> Result<Json<Board>> {
    let board: Board = sqlx::query_as(
        r#"
        UPDATE bbs_boards SET
            name = COALESCE($2, name),
            description = COALESCE($3, description),
            visibility = COALESCE($4, visibility),
            pinned = COALESCE($5, pinned),
            archived = COALESCE($6, archived),
            updated_at = NOW()
        WHERE slug = $1
        RETURNING *
        "#,
    )
    .bind(&slug)
    .bind(&input.name)
    .bind(&input.description)
    .bind(&input.visibility)
    .bind(input.pinned)
    .bind(input.archived)
    .fetch_optional(state.pool())
    .await?
    .ok_or_else(|| Error::NotFound(format!("Board '{}' not found", slug)))?;

    Ok(Json(board))
}

pub async fn delete_board(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<serde_json::Value>> {
    let result = sqlx::query("DELETE FROM bbs_boards WHERE slug = $1")
        .bind(&slug)
        .execute(state.pool())
        .await?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound(format!("Board '{}' not found", slug)));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ============================================================================
// Threads
// ============================================================================

pub async fn list_threads(
    State(state): State<AppState>,
    Path(board_slug): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<ThreadSummary>>> {
    let board: Board = sqlx::query_as("SELECT * FROM bbs_boards WHERE slug = $1")
        .bind(&board_slug)
        .fetch_optional(state.pool())
        .await?
        .ok_or_else(|| Error::NotFound(format!("Board '{}' not found", board_slug)))?;

    let offset = params.offset();
    let limit = params.per_page();

    let threads: Vec<Thread> = sqlx::query_as(
        r#"
        SELECT * FROM bbs_threads
        WHERE board_id = $1
        ORDER BY pinned DESC, updated_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(board.id)
    .bind(limit)
    .bind(offset)
    .fetch_all(state.pool())
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bbs_threads WHERE board_id = $1")
        .bind(board.id)
        .fetch_one(state.pool())
        .await?;

    let mut summaries = Vec::with_capacity(threads.len());
    for thread in threads {
        let stats: (i64, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
            r#"
            SELECT COUNT(*), MAX(created_at)
            FROM bbs_posts WHERE thread_id = $1
            "#,
        )
        .bind(thread.id)
        .fetch_one(state.pool())
        .await?;

        summaries.push(ThreadSummary {
            thread,
            post_count: stats.0,
            last_post_at: stats.1,
        });
    }

    Ok(Json(Paginated {
        items: summaries,
        total: total.0,
        page: params.page(),
        per_page: limit,
    }))
}

pub async fn get_thread(
    State(state): State<AppState>,
    Path(thread_id): Path<Uuid>,
) -> Result<Json<ThreadSummary>> {
    let thread: Thread = sqlx::query_as("SELECT * FROM bbs_threads WHERE id = $1")
        .bind(thread_id)
        .fetch_optional(state.pool())
        .await?
        .ok_or_else(|| Error::NotFound("Thread not found".to_string()))?;

    let stats: (i64, Option<chrono::DateTime<Utc>>) = sqlx::query_as(
        r#"
        SELECT COUNT(*), MAX(created_at)
        FROM bbs_posts WHERE thread_id = $1
        "#,
    )
    .bind(thread.id)
    .fetch_one(state.pool())
    .await?;

    Ok(Json(ThreadSummary {
        thread,
        post_count: stats.0,
        last_post_at: stats.1,
    }))
}

pub async fn create_thread(
    State(state): State<AppState>,
    Json(input): Json<CreateThread>,
) -> Result<Json<Thread>> {
    let mut tx = state.pool().begin().await?;

    // Create thread
    let thread: Thread = sqlx::query_as(
        r#"
        INSERT INTO bbs_threads (board_id, title, author, tags)
        VALUES ($1, $2, $3, $4)
        RETURNING *
        "#,
    )
    .bind(input.board_id)
    .bind(&input.title)
    .bind(&input.author)
    .bind(input.tags.unwrap_or_default())
    .fetch_one(&mut *tx)
    .await?;

    // Create initial post
    sqlx::query(
        r#"
        INSERT INTO bbs_posts (thread_id, author, content)
        VALUES ($1, $2, $3)
        "#,
    )
    .bind(thread.id)
    .bind(&input.author)
    .bind(&input.content)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    Ok(Json(thread))
}

pub async fn update_thread(
    State(state): State<AppState>,
    Path(thread_id): Path<Uuid>,
    Json(input): Json<UpdateThread>,
) -> Result<Json<Thread>> {
    let thread: Thread = sqlx::query_as(
        r#"
        UPDATE bbs_threads SET
            title = COALESCE($2, title),
            pinned = COALESCE($3, pinned),
            locked = COALESCE($4, locked),
            status = COALESCE($5, status),
            tags = COALESCE($6, tags),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(thread_id)
    .bind(&input.title)
    .bind(input.pinned)
    .bind(input.locked)
    .bind(&input.status)
    .bind(&input.tags)
    .fetch_optional(state.pool())
    .await?
    .ok_or_else(|| Error::NotFound("Thread not found".to_string()))?;

    Ok(Json(thread))
}

// ============================================================================
// Posts
// ============================================================================

pub async fn list_posts(
    State(state): State<AppState>,
    Path(thread_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<Post>>> {
    let offset = params.offset();
    let limit = params.per_page();

    let posts: Vec<Post> = sqlx::query_as(
        r#"
        SELECT * FROM bbs_posts
        WHERE thread_id = $1 AND deleted = FALSE
        ORDER BY created_at ASC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(thread_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(state.pool())
    .await?;

    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM bbs_posts WHERE thread_id = $1 AND deleted = FALSE",
    )
    .bind(thread_id)
    .fetch_one(state.pool())
    .await?;

    Ok(Json(Paginated {
        items: posts,
        total: total.0,
        page: params.page(),
        per_page: limit,
    }))
}

pub async fn create_post(
    State(state): State<AppState>,
    Json(input): Json<CreatePost>,
) -> Result<Json<Post>> {
    // Check thread exists and not locked
    let thread: Thread = sqlx::query_as("SELECT * FROM bbs_threads WHERE id = $1")
        .bind(input.thread_id)
        .fetch_optional(state.pool())
        .await?
        .ok_or_else(|| Error::NotFound("Thread not found".to_string()))?;

    if thread.locked {
        return Err(Error::BadRequest("Thread is locked".to_string()));
    }

    let mut tx = state.pool().begin().await?;

    let post: Post = sqlx::query_as(
        r#"
        INSERT INTO bbs_posts (thread_id, author, content, reply_to, metadata)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(input.thread_id)
    .bind(&input.author)
    .bind(&input.content)
    .bind(input.reply_to)
    .bind(&input.metadata)
    .fetch_one(&mut *tx)
    .await?;

    // Update thread updated_at
    sqlx::query("UPDATE bbs_threads SET updated_at = NOW() WHERE id = $1")
        .bind(input.thread_id)
        .execute(&mut *tx)
        .await?;

    tx.commit().await?;

    Ok(Json(post))
}

pub async fn update_post(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
    Json(input): Json<UpdatePost>,
) -> Result<Json<Post>> {
    let post: Post = sqlx::query_as(
        r#"
        UPDATE bbs_posts SET
            content = $2,
            edited = TRUE,
            updated_at = NOW()
        WHERE id = $1 AND deleted = FALSE
        RETURNING *
        "#,
    )
    .bind(post_id)
    .bind(&input.content)
    .fetch_optional(state.pool())
    .await?
    .ok_or_else(|| Error::NotFound("Post not found".to_string()))?;

    Ok(Json(post))
}

pub async fn delete_post(
    State(state): State<AppState>,
    Path(post_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let result = sqlx::query("UPDATE bbs_posts SET deleted = TRUE WHERE id = $1")
        .bind(post_id)
        .execute(state.pool())
        .await?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound("Post not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "deleted": true })))
}

// ============================================================================
// Inbox
// ============================================================================

pub async fn list_inbox(
    State(state): State<AppState>,
    Path(recipient): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<InboxMessage>>> {
    let offset = params.offset();
    let limit = params.per_page();

    let messages: Vec<InboxMessage> = sqlx::query_as(
        r#"
        SELECT * FROM bbs_inbox
        WHERE recipient = $1 AND archived = FALSE
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&recipient)
    .bind(limit)
    .bind(offset)
    .fetch_all(state.pool())
    .await?;

    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM bbs_inbox WHERE recipient = $1 AND archived = FALSE",
    )
    .bind(&recipient)
    .fetch_one(state.pool())
    .await?;

    Ok(Json(Paginated {
        items: messages,
        total: total.0,
        page: params.page(),
        per_page: limit,
    }))
}

pub async fn inbox_stats(
    State(state): State<AppState>,
    Path(recipient): Path<String>,
) -> Result<Json<InboxStats>> {
    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM bbs_inbox WHERE recipient = $1 AND archived = FALSE",
    )
    .bind(&recipient)
    .fetch_one(state.pool())
    .await?;

    let unread: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM bbs_inbox WHERE recipient = $1 AND archived = FALSE AND read_at IS NULL",
    )
    .bind(&recipient)
    .fetch_one(state.pool())
    .await?;

    let high_priority: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM bbs_inbox WHERE recipient = $1 AND archived = FALSE AND priority = 'high' AND read_at IS NULL",
    )
    .bind(&recipient)
    .fetch_one(state.pool())
    .await?;

    Ok(Json(InboxStats {
        total: total.0,
        unread: unread.0,
        high_priority: high_priority.0,
    }))
}

pub async fn send_message(
    State(state): State<AppState>,
    Json(input): Json<SendMessage>,
) -> Result<Json<InboxMessage>> {
    let msg: InboxMessage = sqlx::query_as(
        r#"
        INSERT INTO bbs_inbox (recipient, sender, subject, content, priority, thread_ref, board_ref)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(&input.recipient)
    .bind(&input.sender)
    .bind(&input.subject)
    .bind(&input.content)
    .bind(input.priority.unwrap_or_else(|| "normal".to_string()))
    .bind(input.thread_ref)
    .bind(input.board_ref)
    .fetch_one(state.pool())
    .await?;

    Ok(Json(msg))
}

pub async fn mark_read(
    State(state): State<AppState>,
    Path(msg_id): Path<Uuid>,
) -> Result<Json<InboxMessage>> {
    let msg: InboxMessage = sqlx::query_as(
        r#"
        UPDATE bbs_inbox SET read_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(msg_id)
    .fetch_optional(state.pool())
    .await?
    .ok_or_else(|| Error::NotFound("Message not found".to_string()))?;

    Ok(Json(msg))
}

pub async fn archive_message(
    State(state): State<AppState>,
    Path(msg_id): Path<Uuid>,
) -> Result<Json<serde_json::Value>> {
    let result = sqlx::query("UPDATE bbs_inbox SET archived = TRUE WHERE id = $1")
        .bind(msg_id)
        .execute(state.pool())
        .await?;

    if result.rows_affected() == 0 {
        return Err(Error::NotFound("Message not found".to_string()));
    }

    Ok(Json(serde_json::json!({ "archived": true })))
}

// ============================================================================
// Commons
// ============================================================================

pub async fn list_commons(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<Common>>> {
    let offset = params.offset();
    let limit = params.per_page();

    let commons: Vec<Common> = sqlx::query_as(
        r#"
        SELECT * FROM bbs_commons
        WHERE expires_at IS NULL OR expires_at > NOW()
        ORDER BY updated_at DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(state.pool())
    .await?;

    let total: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM bbs_commons WHERE expires_at IS NULL OR expires_at > NOW()",
    )
    .fetch_one(state.pool())
    .await?;

    Ok(Json(Paginated {
        items: commons,
        total: total.0,
        page: params.page(),
        per_page: limit,
    }))
}

pub async fn get_common(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Common>> {
    let common: Common = sqlx::query_as("SELECT * FROM bbs_commons WHERE slug = $1")
        .bind(&slug)
        .fetch_optional(state.pool())
        .await?
        .ok_or_else(|| Error::NotFound(format!("Common '{}' not found", slug)))?;

    Ok(Json(common))
}

pub async fn create_common(
    State(state): State<AppState>,
    Json(input): Json<CreateCommon>,
) -> Result<Json<Common>> {
    let expires_at = input
        .ttl_seconds
        .map(|ttl| Utc::now() + chrono::Duration::seconds(ttl));

    let common: Common = sqlx::query_as(
        r#"
        INSERT INTO bbs_commons (slug, name, description, common_type, visibility, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(&input.slug)
    .bind(&input.name)
    .bind(&input.description)
    .bind(input.common_type.unwrap_or_else(|| "persistent".to_string()))
    .bind(input.visibility.unwrap_or_else(|| "public".to_string()))
    .bind(expires_at)
    .fetch_one(state.pool())
    .await?;

    Ok(Json(common))
}

pub async fn list_artifacts(
    State(state): State<AppState>,
    Path(common_slug): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<CommonArtifact>>> {
    let common: Common = sqlx::query_as("SELECT * FROM bbs_commons WHERE slug = $1")
        .bind(&common_slug)
        .fetch_optional(state.pool())
        .await?
        .ok_or_else(|| Error::NotFound(format!("Common '{}' not found", common_slug)))?;

    let offset = params.offset();
    let limit = params.per_page();

    let artifacts: Vec<CommonArtifact> = sqlx::query_as(
        r#"
        SELECT * FROM bbs_artifacts
        WHERE common_id = $1
        ORDER BY pinned DESC, created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(common.id)
    .bind(limit)
    .bind(offset)
    .fetch_all(state.pool())
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM bbs_artifacts WHERE common_id = $1")
        .bind(common.id)
        .fetch_one(state.pool())
        .await?;

    Ok(Json(Paginated {
        items: artifacts,
        total: total.0,
        page: params.page(),
        per_page: limit,
    }))
}

pub async fn create_artifact(
    State(state): State<AppState>,
    Json(input): Json<CreateArtifact>,
) -> Result<Json<CommonArtifact>> {
    let artifact: CommonArtifact = sqlx::query_as(
        r#"
        INSERT INTO bbs_artifacts (common_id, author, artifact_type, title, content, metadata)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(input.common_id)
    .bind(&input.author)
    .bind(&input.artifact_type)
    .bind(&input.title)
    .bind(&input.content)
    .bind(&input.metadata)
    .fetch_one(state.pool())
    .await?;

    // Update common updated_at
    sqlx::query("UPDATE bbs_commons SET updated_at = NOW() WHERE id = $1")
        .bind(input.common_id)
        .execute(state.pool())
        .await?;

    Ok(Json(artifact))
}

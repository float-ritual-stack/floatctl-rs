//! BBS data models
//!
//! Core primitives for bulletin board functionality:
//! - Boards: Topic containers
//! - Threads: Discussion units
//! - Posts: Individual contributions
//! - Inbox: Personal async message queue
//! - Commons: Shared collaboration spaces

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// ============================================================================
// Boards
// ============================================================================

/// A board is a topic-organized container for threads
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Board {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Board visibility: public, private, unlisted
    pub visibility: String,
    /// Optional parent board for hierarchical organization
    pub parent_id: Option<Uuid>,
    /// Pinned boards appear at top
    pub pinned: bool,
    /// Archive boards to hide from default views
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBoard {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub visibility: Option<String>,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateBoard {
    pub name: Option<String>,
    pub description: Option<String>,
    pub visibility: Option<String>,
    pub pinned: Option<bool>,
    pub archived: Option<bool>,
}

// ============================================================================
// Threads
// ============================================================================

/// A thread is a discussion unit within a board
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Thread {
    pub id: Uuid,
    pub board_id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Original author identifier (could be agent ID, user ID, etc.)
    pub author: String,
    /// Pinned threads appear at top of board
    pub pinned: bool,
    /// Locked threads cannot receive new posts
    pub locked: bool,
    /// Thread status: open, resolved, archived
    pub status: String,
    /// Optional tags for categorization
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateThread {
    pub board_id: Uuid,
    pub title: String,
    pub author: String,
    /// Initial post content
    pub content: String,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateThread {
    pub title: Option<String>,
    pub pinned: Option<bool>,
    pub locked: Option<bool>,
    pub status: Option<String>,
    pub tags: Option<Vec<String>>,
}

// ============================================================================
// Posts
// ============================================================================

/// A post is an individual message within a thread
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Post {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub author: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Reply to another post (for threaded replies)
    pub reply_to: Option<Uuid>,
    /// Edit history preserved
    pub edited: bool,
    /// Soft delete
    pub deleted: bool,
    /// Optional structured metadata (JSON)
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatePost {
    pub thread_id: Uuid,
    pub author: String,
    pub content: String,
    pub reply_to: Option<Uuid>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatePost {
    pub content: String,
}

// ============================================================================
// Inbox
// ============================================================================

/// Inbox message for async personal communication
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct InboxMessage {
    pub id: Uuid,
    /// Recipient identifier
    pub recipient: String,
    /// Sender identifier
    pub sender: String,
    pub subject: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    /// Priority: normal, high, low
    pub priority: String,
    /// Optional thread reference for context
    pub thread_ref: Option<Uuid>,
    /// Optional board reference for context
    pub board_ref: Option<Uuid>,
    /// Archived messages hidden from default view
    pub archived: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessage {
    pub recipient: String,
    pub sender: String,
    pub subject: Option<String>,
    pub content: String,
    pub priority: Option<String>,
    pub thread_ref: Option<Uuid>,
    pub board_ref: Option<Uuid>,
}

// ============================================================================
// Commons
// ============================================================================

/// A common area for shared real-time collaboration
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Common {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Common type: scratch (ephemeral), persistent, event
    pub common_type: String,
    /// Visibility: public, private, invite
    pub visibility: String,
    /// Expiry for ephemeral commons
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommon {
    pub slug: String,
    pub name: String,
    pub description: Option<String>,
    pub common_type: Option<String>,
    pub visibility: Option<String>,
    /// Duration in seconds for ephemeral commons
    pub ttl_seconds: Option<i64>,
}

/// An artifact in a common area (shared content)
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CommonArtifact {
    pub id: Uuid,
    pub common_id: Uuid,
    pub author: String,
    /// Artifact type: text, code, image, link, file
    pub artifact_type: String,
    pub title: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// Optional metadata (language for code, dimensions for images, etc.)
    pub metadata: Option<serde_json::Value>,
    /// Pinned artifacts appear prominently
    pub pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateArtifact {
    pub common_id: Uuid,
    pub author: String,
    pub artifact_type: String,
    pub title: Option<String>,
    pub content: String,
    pub metadata: Option<serde_json::Value>,
}

// ============================================================================
// Response types
// ============================================================================

/// Thread with post count for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSummary {
    #[serde(flatten)]
    pub thread: Thread,
    pub post_count: i64,
    pub last_post_at: Option<DateTime<Utc>>,
}

/// Board with thread count for listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSummary {
    #[serde(flatten)]
    pub board: Board,
    pub thread_count: i64,
    pub post_count: i64,
}

/// Inbox stats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxStats {
    pub total: i64,
    pub unread: i64,
    pub high_priority: i64,
}

/// Pagination wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paginated<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

impl PaginationParams {
    pub fn offset(&self) -> i64 {
        let page = self.page.unwrap_or(1).max(1);
        let per_page = self.per_page();
        (page - 1) * per_page
    }

    pub fn per_page(&self) -> i64 {
        self.per_page.unwrap_or(20).clamp(1, 100)
    }

    pub fn page(&self) -> i64 {
        self.page.unwrap_or(1).max(1)
    }
}

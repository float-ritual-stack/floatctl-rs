//! Request and response models for floatctl-server BBS

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Personas
// ============================================================================

/// Known agent personas in the FLOAT system
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Persona {
    Evna,
    Kitty,
    Cowboy,
    Daddy,
    /// Generic persona for external agents
    Agent,
}

impl std::fmt::Display for Persona {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Persona::Evna => write!(f, "evna"),
            Persona::Kitty => write!(f, "kitty"),
            Persona::Cowboy => write!(f, "cowboy"),
            Persona::Daddy => write!(f, "daddy"),
            Persona::Agent => write!(f, "agent"),
        }
    }
}

impl std::str::FromStr for Persona {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "evna" => Ok(Persona::Evna),
            "kitty" => Ok(Persona::Kitty),
            "cowboy" => Ok(Persona::Cowboy),
            "daddy" => Ok(Persona::Daddy),
            "agent" => Ok(Persona::Agent),
            _ => Err(format!("Unknown persona: {}", s)),
        }
    }
}

// ============================================================================
// Boards
// ============================================================================

/// A message board (workspace area)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Board {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub thread_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateBoardRequest {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardWithThreads {
    #[serde(flatten)]
    pub board: Board,
    pub recent_threads: Vec<ThreadSummary>,
}

// ============================================================================
// Threads
// ============================================================================

/// A thread summary (for listing)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreadSummary {
    pub id: Uuid,
    pub board_id: Uuid,
    pub title: String,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub message_count: i64,
    pub last_message_at: Option<DateTime<Utc>>,
}

/// A full thread with messages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: Uuid,
    pub board_id: Uuid,
    pub title: String,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub messages: Vec<Message>,
    /// Parsed markers from all messages
    pub markers: ThreadMarkers,
}

/// Parsed markers aggregated from thread content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThreadMarkers {
    pub projects: Vec<String>,
    pub contexts: Vec<String>,
    pub modes: Vec<String>,
    pub bridges: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateThreadRequest {
    pub title: String,
    pub author: Option<String>,
    /// Initial message content (optional)
    pub content: Option<String>,
}

// ============================================================================
// Messages
// ============================================================================

/// A message within a thread
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub author: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    /// Raw marker strings found in content
    pub markers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateMessageRequest {
    pub author: Option<String>,
    pub content: String,
}

// ============================================================================
// Inbox
// ============================================================================

/// An inbox message for async agent handoffs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboxMessage {
    pub id: Uuid,
    pub persona: String,
    pub from_persona: Option<String>,
    pub subject: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
    /// Optional reference to a thread
    pub thread_id: Option<Uuid>,
    pub priority: InboxPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum InboxPriority {
    Low,
    #[default]
    Normal,
    High,
    Urgent,
}

impl std::fmt::Display for InboxPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InboxPriority::Low => write!(f, "low"),
            InboxPriority::Normal => write!(f, "normal"),
            InboxPriority::High => write!(f, "high"),
            InboxPriority::Urgent => write!(f, "urgent"),
        }
    }
}

impl std::str::FromStr for InboxPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "low" => Ok(InboxPriority::Low),
            "normal" => Ok(InboxPriority::Normal),
            "high" => Ok(InboxPriority::High),
            "urgent" => Ok(InboxPriority::Urgent),
            _ => Err(format!("Unknown priority: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateInboxMessageRequest {
    pub from_persona: Option<String>,
    pub subject: Option<String>,
    pub content: String,
    pub thread_id: Option<Uuid>,
    #[serde(default)]
    pub priority: InboxPriority,
}

// ============================================================================
// Common Area
// ============================================================================

/// An item in the common scratch area
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonItem {
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommonItemRequest {
    pub key: String,
    pub value: serde_json::Value,
    /// TTL in seconds (optional)
    pub ttl_seconds: Option<i64>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateCommonItemRequest {
    pub value: serde_json::Value,
    /// TTL in seconds (optional, resets expiry if provided)
    pub ttl_seconds: Option<i64>,
}

// ============================================================================
// CLI Wrapper
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliRequest {
    /// Command arguments (e.g., ["--in", "file.json"])
    pub args: Vec<String>,
    /// Working directory (optional)
    pub cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliResponse {
    pub success: bool,
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
    pub duration_ms: u64,
}

// ============================================================================
// Health Check
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
    pub database: DatabaseHealth,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseHealth {
    pub connected: bool,
    pub path: String,
    pub size_bytes: Option<u64>,
}

// ============================================================================
// Query Parameters
// ============================================================================

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ThreadQueryParams {
    /// Filter by project marker
    pub project: Option<String>,
    /// Filter by context marker
    pub ctx: Option<String>,
    /// Limit results
    pub limit: Option<i64>,
    /// Offset for pagination
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct InboxQueryParams {
    /// Include read messages
    pub include_read: Option<bool>,
    /// Filter by priority
    pub priority: Option<String>,
    /// Limit results
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CommonQueryParams {
    /// Filter by key prefix
    pub prefix: Option<String>,
    /// Include expired items
    pub include_expired: Option<bool>,
    /// Limit results
    pub limit: Option<i64>,
}

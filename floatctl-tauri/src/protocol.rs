//! Protocol definitions for Tauri frontend/backend communication
//!
//! These types define the data contract between Rust backend and TypeScript frontend.
//! All types implement Serialize/Deserialize for JSON transport via Tauri commands/events.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ============================================================================
// Core Item Types - Hierarchical data model (Boards → Posts → Actions)
// ============================================================================

/// The kind of navigable item in the hierarchy
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemKind {
    /// Top-level board container
    Board,
    /// Post within a board (thread/message)
    Post,
    /// Agent job or background task
    Job,
    /// File from filesystem source
    File,
    /// Search result
    SearchResult,
    /// Persona (kitty, daddy, cowboy, evna)
    Persona,
}

/// A navigable item in the hierarchical browser
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    /// Unique identifier
    pub id: String,

    /// Item type for rendering/routing
    pub kind: ItemKind,

    /// Display title
    pub title: String,

    /// Optional subtitle (e.g., timestamp, author)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subtitle: Option<String>,

    /// Available action IDs for this item
    pub actions: Vec<String>,

    /// Parent ID for hierarchy traversal
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// Whether item has children (expandable)
    pub has_children: bool,

    /// Badge/indicator text (e.g., unread count)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub badge: Option<String>,

    /// Additional metadata for item-specific rendering
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub meta: HashMap<String, serde_json::Value>,
}

impl Item {
    /// Create a new board item
    pub fn board(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            kind: ItemKind::Board,
            title: title.into(),
            subtitle: None,
            actions: vec!["view".into(), "browse".into()],
            parent_id: None,
            has_children: true,
            badge: None,
            meta: HashMap::new(),
        }
    }

    /// Create a new post item
    pub fn post(
        id: impl Into<String>,
        title: impl Into<String>,
        board_id: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind: ItemKind::Post,
            title: title.into(),
            subtitle: None,
            actions: vec!["view".into(), "edit_metadata".into(), "dispatch".into()],
            parent_id: Some(board_id.into()),
            has_children: false,
            badge: None,
            meta: HashMap::new(),
        }
    }

    /// Builder: add subtitle
    pub fn with_subtitle(mut self, subtitle: impl Into<String>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }

    /// Builder: add badge
    pub fn with_badge(mut self, badge: impl Into<String>) -> Self {
        self.badge = Some(badge.into());
        self
    }

    /// Builder: add metadata
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Serialize) -> Self {
        if let Ok(v) = serde_json::to_value(value) {
            self.meta.insert(key.into(), v);
        }
        self
    }
}

// ============================================================================
// Source - Data Provider Abstraction
// ============================================================================

/// Scope constraints for data queries
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scope {
    /// Filter by parent ID (e.g., show posts in board)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<String>,

    /// Filter by item kinds
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub kinds: Vec<ItemKind>,

    /// Active folder/project context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_context: Option<String>,

    /// Search query (for RAG sources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<String>,

    /// Limit results
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,

    /// Offset for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<usize>,
}

/// Response from a source fetch
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceResponse {
    /// Fetched items
    pub items: Vec<Item>,

    /// Total count (for pagination)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<usize>,

    /// Source identifier
    pub source: String,

    /// Whether more items available
    pub has_more: bool,
}

/// Identifies a data source type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// BBS boards and posts
    Bbs,
    /// Local filesystem
    Filesystem,
    /// RAG search results
    Search,
    /// Agent jobs
    Jobs,
    /// Fixed/static list
    Static,
}

// ============================================================================
// Action - Command Execution
// ============================================================================

/// Action that can be executed on an item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    /// Unique action identifier
    pub id: String,

    /// Display label
    pub label: String,

    /// Keyboard shortcut (e.g., "Enter", "Space", "e")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shortcut: Option<String>,

    /// Icon identifier (for UI rendering)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon: Option<String>,

    /// Whether action is destructive (shows warning)
    #[serde(default)]
    pub destructive: bool,

    /// Whether action runs in background
    #[serde(default)]
    pub background: bool,
}

impl Action {
    pub fn new(id: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            shortcut: None,
            icon: None,
            destructive: false,
            background: false,
        }
    }

    pub fn with_shortcut(mut self, shortcut: impl Into<String>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn destructive(mut self) -> Self {
        self.destructive = true;
        self
    }

    pub fn background(mut self) -> Self {
        self.background = true;
        self
    }
}

/// Request to execute an action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionRequest {
    /// Action ID to execute
    pub action_id: String,

    /// Target item ID
    pub item_id: String,

    /// Additional parameters
    #[serde(default)]
    pub params: HashMap<String, serde_json::Value>,
}

/// Result of action execution
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ActionResult {
    /// Action completed successfully
    Success {
        /// Optional message
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        /// Optional navigation target
        #[serde(skip_serializing_if = "Option::is_none")]
        navigate_to: Option<String>,
    },
    /// Action started as background job
    JobStarted {
        /// Job ID for tracking
        job_id: String,
    },
    /// Action failed
    Error {
        /// Error message
        message: String,
    },
}

// ============================================================================
// Navigation State - Mode-based UI
// ============================================================================

/// Current UI mode (vim-like modal editing)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    /// Navigation mode - keys send commands
    #[default]
    Normal,
    /// Edit mode - focus on input fields
    Edit,
    /// Visual mode - multi-select (future)
    Visual,
    /// Command mode - palette/search active
    Command,
}

/// Navigation cursor position
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cursor {
    /// Current item ID under cursor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item_id: Option<String>,

    /// Index in current list
    pub index: usize,

    /// Hierarchy depth (0 = root boards)
    pub depth: usize,

    /// Parent path for breadcrumb
    #[serde(default)]
    pub path: Vec<String>,
}

/// Full navigation state synced between backend and frontend
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NavigationState {
    /// Current UI mode
    pub mode: Mode,

    /// Cursor position
    pub cursor: Cursor,

    /// Active source kind
    pub source: Option<SourceKind>,

    /// Active view/route
    #[serde(skip_serializing_if = "Option::is_none")]
    pub view: Option<String>,
}

// ============================================================================
// Job Status - Background task tracking
// ============================================================================

/// Status of a background job
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Job is queued
    Pending,
    /// Job is running
    Running,
    /// Job completed successfully
    Completed,
    /// Job failed
    Failed,
    /// Job was cancelled
    Cancelled,
}

/// Job progress update (emitted via Tauri events)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobProgress {
    /// Job ID
    pub job_id: String,

    /// Current status
    pub status: JobStatus,

    /// Progress percentage (0-100)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,

    /// Status message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,

    /// Result data (when completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
}

// ============================================================================
// Scratch Commands - Parsed from scratch pane input
// ============================================================================

/// Command parsed from scratch pane
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ScratchCommand {
    /// RAG search: /search "query" or /rag "query"
    Search { query: String },
    /// Navigate to item: /nav path or @entity
    Navigate { target: String },
    /// Ask agent: /ask "question"
    Ask { question: String },
    /// Execute shell: /cmd command
    Shell { command: String },
    /// Unknown/passthrough
    Text { content: String },
}

impl ScratchCommand {
    /// Parse a line from scratch pane
    pub fn parse(input: &str) -> Self {
        let trimmed = input.trim();

        if let Some(rest) = trimmed
            .strip_prefix("/search ")
            .or(trimmed.strip_prefix("/rag "))
        {
            return Self::Search {
                query: rest.trim_matches('"').to_string(),
            };
        }

        if let Some(rest) = trimmed.strip_prefix("/nav ") {
            return Self::Navigate {
                target: rest.to_string(),
            };
        }

        if let Some(rest) = trimmed.strip_prefix("/ask ") {
            return Self::Ask {
                question: rest.trim_matches('"').to_string(),
            };
        }

        if let Some(rest) = trimmed.strip_prefix("/cmd ") {
            return Self::Shell {
                command: rest.to_string(),
            };
        }

        // Check for @entity pattern
        if trimmed.starts_with('@') {
            return Self::Navigate {
                target: trimmed[1..].to_string(),
            };
        }

        Self::Text {
            content: trimmed.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_builders() {
        let board = Item::board("b1", "Daily Notes").with_badge("5");
        assert_eq!(board.kind, ItemKind::Board);
        assert_eq!(board.badge, Some("5".into()));

        let post = Item::post("p1", "Morning thoughts", "b1").with_subtitle("2025-12-25");
        assert_eq!(post.kind, ItemKind::Post);
        assert_eq!(post.parent_id, Some("b1".into()));
    }

    #[test]
    fn scratch_command_parsing() {
        assert!(matches!(
            ScratchCommand::parse("/search refactoring"),
            ScratchCommand::Search { query } if query == "refactoring"
        ));

        assert!(matches!(
            ScratchCommand::parse("/nav daily-notes"),
            ScratchCommand::Navigate { target } if target == "daily-notes"
        ));

        assert!(matches!(
            ScratchCommand::parse("@today"),
            ScratchCommand::Navigate { target } if target == "today"
        ));

        assert!(matches!(
            ScratchCommand::parse("plain text"),
            ScratchCommand::Text { content } if content == "plain text"
        ));
    }

    #[test]
    fn action_result_serialization() {
        let success = ActionResult::Success {
            message: Some("Done".into()),
            navigate_to: None,
        };
        let json = serde_json::to_string(&success).unwrap();
        assert!(json.contains("\"status\":\"success\""));

        let job = ActionResult::JobStarted {
            job_id: "job-123".into(),
        };
        let json = serde_json::to_string(&job).unwrap();
        assert!(json.contains("\"status\":\"job_started\""));
    }
}

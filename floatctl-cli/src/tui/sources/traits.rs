//! Core traits for data sources and actions

use std::fmt;

use crate::tui::app::{ActionItem, ItemKind, ListItem};

/// A source provides data for the list navigator
pub trait Source: Send + Sync {
    /// Unique identifier for this source
    fn id(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Get items at the root level
    fn list(&self) -> Vec<ListItem>;

    /// Get children of an item (for hierarchical navigation)
    fn children(&self, item_id: &str) -> Option<Vec<ListItem>>;

    /// Get preview content for an item
    fn preview(&self, item_id: &str) -> Option<String>;

    /// Search/filter items
    fn search(&self, query: &str) -> Vec<ListItem>;

    /// Get available actions for an item
    fn actions(&self, item_id: &str) -> Vec<ActionItem>;
}

/// An item from a source
#[derive(Debug, Clone)]
pub struct SourceItem {
    /// Unique identifier
    pub id: String,
    /// Display title
    pub title: String,
    /// Optional description
    pub description: Option<String>,
    /// Item kind
    pub kind: ItemKind,
    /// Whether this item has children
    pub has_children: bool,
    /// Raw data (for actions)
    pub data: Option<serde_json::Value>,
}

impl From<SourceItem> for ListItem {
    fn from(item: SourceItem) -> Self {
        ListItem {
            id: item.id,
            title: item.title,
            subtitle: item.description,
            kind: item.kind,
            has_children: item.has_children,
            meta: None,
        }
    }
}

/// Context for action execution
#[derive(Debug)]
pub struct ActionContext {
    /// The item the action is being performed on
    pub item_id: String,
    /// Source that owns the item
    pub source_id: String,
    /// Additional data
    pub data: serde_json::Value,
}

/// Result of an action execution
#[derive(Debug)]
pub enum ActionResult {
    /// Action completed successfully
    Success(String),
    /// Action completed with a message to show
    Message(String),
    /// Action spawned a background job
    Job { id: String, description: String },
    /// Action failed with error
    Error(String),
    /// Action opened external editor/viewer
    External,
    /// Action caused navigation
    Navigate { kind: String, id: String, title: String },
    /// Action refreshes the current view
    Refresh,
}

impl fmt::Display for ActionResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ActionResult::Success(msg) => write!(f, "Success: {}", msg),
            ActionResult::Message(msg) => write!(f, "{}", msg),
            ActionResult::Job { id, description } => write!(f, "Job {}: {}", id, description),
            ActionResult::Error(err) => write!(f, "Error: {}", err),
            ActionResult::External => write!(f, "Opened external application"),
            ActionResult::Navigate { title, .. } => write!(f, "Navigated to {}", title),
            ActionResult::Refresh => write!(f, "Refreshed"),
        }
    }
}

/// An executable action
pub trait Action: Send + Sync {
    /// Unique identifier
    fn id(&self) -> &str;

    /// Display name
    fn name(&self) -> &str;

    /// Description
    fn description(&self) -> &str;

    /// Keyboard shortcut hint
    fn shortcut(&self) -> Option<&str> {
        None
    }

    /// Check if this action is applicable to an item
    fn applicable(&self, item: &ListItem) -> bool;

    /// Execute the action
    fn execute(&self, ctx: &ActionContext) -> ActionResult;
}

/// Registry of actions
#[derive(Default)]
pub struct ActionRegistry {
    actions: Vec<Box<dyn Action>>,
}

impl ActionRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self { actions: Vec::new() }
    }

    /// Register an action
    pub fn register(&mut self, action: Box<dyn Action>) {
        self.actions.push(action);
    }

    /// Get applicable actions for an item
    pub fn get_actions(&self, item: &ListItem) -> Vec<ActionItem> {
        self.actions
            .iter()
            .filter(|a| a.applicable(item))
            .map(|a| ActionItem {
                id: a.id().to_string(),
                name: a.name().to_string(),
                shortcut: a.shortcut().map(String::from),
                description: a.description().to_string(),
            })
            .collect()
    }

    /// Execute an action by ID
    pub fn execute(&self, action_id: &str, ctx: &ActionContext) -> Option<ActionResult> {
        self.actions
            .iter()
            .find(|a| a.id() == action_id)
            .map(|a| a.execute(ctx))
    }
}

// Common actions

/// View action - display item content
pub struct ViewAction;

impl Action for ViewAction {
    fn id(&self) -> &str {
        "view"
    }

    fn name(&self) -> &str {
        "View"
    }

    fn description(&self) -> &str {
        "View item content"
    }

    fn shortcut(&self) -> Option<&str> {
        Some("v")
    }

    fn applicable(&self, _item: &ListItem) -> bool {
        true
    }

    fn execute(&self, _ctx: &ActionContext) -> ActionResult {
        ActionResult::Success("Viewing item".to_string())
    }
}

/// Edit metadata action
pub struct EditMetadataAction;

impl Action for EditMetadataAction {
    fn id(&self) -> &str {
        "edit_metadata"
    }

    fn name(&self) -> &str {
        "Edit Metadata"
    }

    fn description(&self) -> &str {
        "Edit item metadata"
    }

    fn shortcut(&self) -> Option<&str> {
        Some("m")
    }

    fn applicable(&self, item: &ListItem) -> bool {
        matches!(item.kind, ItemKind::Post | ItemKind::Note | ItemKind::File)
    }

    fn execute(&self, _ctx: &ActionContext) -> ActionResult {
        ActionResult::Message("Metadata editing not yet implemented".to_string())
    }
}

/// Open in editor action
pub struct OpenInEditorAction;

impl Action for OpenInEditorAction {
    fn id(&self) -> &str {
        "open_editor"
    }

    fn name(&self) -> &str {
        "Open in Editor"
    }

    fn description(&self) -> &str {
        "Open in external editor"
    }

    fn shortcut(&self) -> Option<&str> {
        Some("e")
    }

    fn applicable(&self, item: &ListItem) -> bool {
        matches!(
            item.kind,
            ItemKind::File | ItemKind::Note | ItemKind::Post
        )
    }

    fn execute(&self, ctx: &ActionContext) -> ActionResult {
        // Would shell out to $EDITOR
        ActionResult::Message(format!("Would open {} in editor", ctx.item_id))
    }
}

/// Copy to clipboard action
pub struct CopyAction;

impl Action for CopyAction {
    fn id(&self) -> &str {
        "copy"
    }

    fn name(&self) -> &str {
        "Copy"
    }

    fn description(&self) -> &str {
        "Copy to clipboard"
    }

    fn shortcut(&self) -> Option<&str> {
        Some("y")
    }

    fn applicable(&self, _item: &ListItem) -> bool {
        true
    }

    fn execute(&self, ctx: &ActionContext) -> ActionResult {
        // Would use cli-clipboard
        ActionResult::Message(format!("Copied {} to clipboard", ctx.item_id))
    }
}

//! Home dashboard source

use crate::tui::app::{ActionItem, ItemKind, ListItem};
use super::traits::Source;

/// Home dashboard source - shows recent sessions, notifications, etc.
pub struct HomeSource {
    items: Vec<ListItem>,
}

impl Default for HomeSource {
    fn default() -> Self {
        Self::new()
    }
}

impl HomeSource {
    /// Create a new home source with placeholder data
    pub fn new() -> Self {
        let items = vec![
            ListItem {
                id: "recent_sessions".to_string(),
                title: "Recent Sessions".to_string(),
                subtitle: Some("View recent Claude Code sessions".to_string()),
                kind: ItemKind::Folder,
                has_children: true,
                meta: None,
            },
            ListItem {
                id: "notifications".to_string(),
                title: "Notifications".to_string(),
                subtitle: Some("3 unread".to_string()),
                kind: ItemKind::Notification,
                has_children: true,
                meta: None,
            },
            ListItem {
                id: "daily_note".to_string(),
                title: "Today's Daily Note".to_string(),
                subtitle: Some("2024-12-25".to_string()),
                kind: ItemKind::Note,
                has_children: false,
                meta: None,
            },
            ListItem {
                id: "focus".to_string(),
                title: "Current Focus".to_string(),
                subtitle: Some("Float Control TUI implementation".to_string()),
                kind: ItemKind::Task,
                has_children: false,
                meta: None,
            },
            ListItem {
                id: "quick_actions".to_string(),
                title: "Quick Actions".to_string(),
                subtitle: Some("Common operations".to_string()),
                kind: ItemKind::Folder,
                has_children: true,
                meta: None,
            },
        ];

        Self { items }
    }

    /// Load real data from system
    pub async fn load(&mut self) -> anyhow::Result<()> {
        // TODO: Load real data from:
        // - floatctl status show
        // - Recent Claude sessions
        // - BBS notifications
        // - Daily note content
        Ok(())
    }
}

impl Source for HomeSource {
    fn id(&self) -> &str {
        "home"
    }

    fn name(&self) -> &str {
        "Home"
    }

    fn list(&self) -> Vec<ListItem> {
        self.items.clone()
    }

    fn children(&self, item_id: &str) -> Option<Vec<ListItem>> {
        match item_id {
            "recent_sessions" => Some(vec![
                ListItem {
                    id: "session_1".to_string(),
                    title: "floatctl-rs".to_string(),
                    subtitle: Some("TUI implementation - 2h ago".to_string()),
                    kind: ItemKind::Session,
                    has_children: false,
                    meta: None,
                },
                ListItem {
                    id: "session_2".to_string(),
                    title: "evna-next".to_string(),
                    subtitle: Some("MCP server updates - 5h ago".to_string()),
                    kind: ItemKind::Session,
                    has_children: false,
                    meta: None,
                },
            ]),
            "notifications" => Some(vec![
                ListItem {
                    id: "notif_1".to_string(),
                    title: "New message from kitty".to_string(),
                    subtitle: Some("BBS - 1h ago".to_string()),
                    kind: ItemKind::Notification,
                    has_children: false,
                    meta: None,
                },
                ListItem {
                    id: "notif_2".to_string(),
                    title: "Sync completed".to_string(),
                    subtitle: Some("daily sync - 3h ago".to_string()),
                    kind: ItemKind::Notification,
                    has_children: false,
                    meta: None,
                },
            ]),
            "quick_actions" => Some(vec![
                ListItem {
                    id: "action_search".to_string(),
                    title: "Search".to_string(),
                    subtitle: Some("AI-powered search across archives".to_string()),
                    kind: ItemKind::Action,
                    has_children: false,
                    meta: None,
                },
                ListItem {
                    id: "action_ctx".to_string(),
                    title: "Capture Context".to_string(),
                    subtitle: Some("Add context marker".to_string()),
                    kind: ItemKind::Action,
                    has_children: false,
                    meta: None,
                },
                ListItem {
                    id: "action_bbs".to_string(),
                    title: "BBS Inbox".to_string(),
                    subtitle: Some("Check messages".to_string()),
                    kind: ItemKind::Action,
                    has_children: false,
                    meta: None,
                },
            ]),
            _ => None,
        }
    }

    fn preview(&self, item_id: &str) -> Option<String> {
        match item_id {
            "daily_note" => Some(
                "# Daily Note - 2024-12-25\n\n\
                ## Focus\n\
                - Float Control TUI implementation\n\
                - Milestone 1: core navigation\n\n\
                ## Notes\n\
                - Using ratatui for rendering\n\
                - TV-inspired but menu-driven\n"
                    .to_string(),
            ),
            "focus" => Some(
                "Current Work Focus:\n\n\
                Float Control TUI implementation\n\
                - Hierarchical navigation\n\
                - Action palette\n\
                - RAG integration\n"
                    .to_string(),
            ),
            _ => None,
        }
    }

    fn search(&self, query: &str) -> Vec<ListItem> {
        let query_lower = query.to_lowercase();
        self.items
            .iter()
            .filter(|item| {
                item.title.to_lowercase().contains(&query_lower)
                    || item
                        .subtitle
                        .as_ref()
                        .map(|s| s.to_lowercase().contains(&query_lower))
                        .unwrap_or(false)
            })
            .cloned()
            .collect()
    }

    fn actions(&self, item_id: &str) -> Vec<ActionItem> {
        match item_id {
            "daily_note" => vec![
                ActionItem {
                    id: "view".to_string(),
                    name: "View".to_string(),
                    shortcut: Some("v".to_string()),
                    description: "View daily note".to_string(),
                },
                ActionItem {
                    id: "open_editor".to_string(),
                    name: "Open in Editor".to_string(),
                    shortcut: Some("e".to_string()),
                    description: "Edit in external editor".to_string(),
                },
            ],
            "focus" => vec![
                ActionItem {
                    id: "edit_focus".to_string(),
                    name: "Edit Focus".to_string(),
                    shortcut: Some("e".to_string()),
                    description: "Change current focus".to_string(),
                },
                ActionItem {
                    id: "clear_focus".to_string(),
                    name: "Clear Focus".to_string(),
                    shortcut: Some("c".to_string()),
                    description: "Clear the focus status".to_string(),
                },
            ],
            _ if item_id.starts_with("action_") => vec![ActionItem {
                id: "execute".to_string(),
                name: "Execute".to_string(),
                shortcut: Some("Enter".to_string()),
                description: "Run this action".to_string(),
            }],
            _ => vec![
                ActionItem {
                    id: "view".to_string(),
                    name: "View".to_string(),
                    shortcut: Some("v".to_string()),
                    description: "View details".to_string(),
                },
            ],
        }
    }
}

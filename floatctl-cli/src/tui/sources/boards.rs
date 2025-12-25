//! BBS Boards source

use crate::tui::app::{ActionItem, ItemKind, ListItem};
use super::traits::Source;

/// Boards source - hierarchical BBS board/post navigation
pub struct BoardsSource {
    boards: Vec<Board>,
}

#[derive(Clone)]
struct Board {
    id: String,
    name: String,
    description: String,
    post_count: usize,
    posts: Vec<Post>,
}

#[derive(Clone)]
struct Post {
    id: String,
    title: String,
    author: String,
    date: String,
    content: String,
    tags: Vec<String>,
}

impl Default for BoardsSource {
    fn default() -> Self {
        Self::new()
    }
}

impl BoardsSource {
    /// Create a new boards source with placeholder data
    pub fn new() -> Self {
        let boards = vec![
            Board {
                id: "sysops-log".to_string(),
                name: "sysops-log".to_string(),
                description: "System operations log".to_string(),
                post_count: 15,
                posts: vec![
                    Post {
                        id: "post_1".to_string(),
                        title: "Float Control TUI Started".to_string(),
                        author: "sysop".to_string(),
                        date: "2024-12-25".to_string(),
                        content: "Started implementing the Float Control TUI with ratatui.\n\nKey features:\n- Hierarchical navigation\n- Action palette\n- RAG integration\n- TV-inspired design".to_string(),
                        tags: vec!["dev".to_string(), "tui".to_string()],
                    },
                    Post {
                        id: "post_2".to_string(),
                        title: "Sync Pipeline Fixed".to_string(),
                        author: "sysop".to_string(),
                        date: "2024-12-24".to_string(),
                        content: "Fixed the rsync -> R2 pipeline issue where files weren't being detected properly.".to_string(),
                        tags: vec!["fix".to_string(), "sync".to_string()],
                    },
                ],
            },
            Board {
                id: "sysops-ponder".to_string(),
                name: "sysops-ponder".to_string(),
                description: "Reflections and deeper thoughts".to_string(),
                post_count: 8,
                posts: vec![
                    Post {
                        id: "post_3".to_string(),
                        title: "On TUI Design Philosophy".to_string(),
                        author: "sysop".to_string(),
                        date: "2024-12-25".to_string(),
                        content: "The difference between an editor-centric and menu-centric TUI is subtle but important.\n\nEditor-centric: select file -> open in editor\nMenu-centric: select item -> show actions -> choose what to do\n\nThe latter gives more control and fits the 'float control' paradigm better.".to_string(),
                        tags: vec!["design".to_string(), "philosophy".to_string()],
                    },
                ],
            },
            Board {
                id: "common".to_string(),
                name: "common".to_string(),
                description: "General discussion board".to_string(),
                post_count: 12,
                posts: vec![
                    Post {
                        id: "post_4".to_string(),
                        title: "Welcome to BBS".to_string(),
                        author: "system".to_string(),
                        date: "2024-01-01".to_string(),
                        content: "Welcome to the Float BBS system!\n\nThis is the common board for general discussion.".to_string(),
                        tags: vec!["welcome".to_string()],
                    },
                ],
            },
        ];

        Self { boards }
    }

    /// Load real data from BBS API
    pub async fn load(&mut self, _endpoint: &str, _persona: &str) -> anyhow::Result<()> {
        // TODO: Fetch from BBS API
        // let client = reqwest::Client::new();
        // let boards_url = format!("{}/bbs/boards", endpoint);
        // ...
        Ok(())
    }

    fn find_board(&self, board_id: &str) -> Option<&Board> {
        self.boards.iter().find(|b| b.id == board_id)
    }

    fn find_post(&self, post_id: &str) -> Option<(&Board, &Post)> {
        for board in &self.boards {
            if let Some(post) = board.posts.iter().find(|p| p.id == post_id) {
                return Some((board, post));
            }
        }
        None
    }
}

impl Source for BoardsSource {
    fn id(&self) -> &str {
        "boards"
    }

    fn name(&self) -> &str {
        "Boards"
    }

    fn list(&self) -> Vec<ListItem> {
        self.boards
            .iter()
            .map(|board| ListItem {
                id: board.id.clone(),
                title: board.name.clone(),
                subtitle: Some(format!("{} ({} posts)", board.description, board.post_count)),
                kind: ItemKind::Board,
                has_children: true,
                meta: None,
            })
            .collect()
    }

    fn children(&self, item_id: &str) -> Option<Vec<ListItem>> {
        // Check if it's a board
        if let Some(board) = self.find_board(item_id) {
            return Some(
                board
                    .posts
                    .iter()
                    .map(|post| ListItem {
                        id: post.id.clone(),
                        title: post.title.clone(),
                        subtitle: Some(format!("by {} @ {}", post.author, post.date)),
                        kind: ItemKind::Post,
                        has_children: false,
                        meta: Some(post.tags.join(", ")),
                    })
                    .collect(),
            );
        }

        None
    }

    fn preview(&self, item_id: &str) -> Option<String> {
        // Board preview
        if let Some(board) = self.find_board(item_id) {
            let recent: Vec<_> = board.posts.iter().take(5).collect();
            let mut preview = format!("# {}\n{}\n\n", board.name, board.description);
            preview.push_str(&format!("Posts: {}\n\n", board.post_count));
            preview.push_str("Recent:\n");
            for post in recent {
                preview.push_str(&format!("- {} ({})\n", post.title, post.date));
            }
            return Some(preview);
        }

        // Post preview
        if let Some((_board, post)) = self.find_post(item_id) {
            let mut preview = format!("# {}\n", post.title);
            preview.push_str(&format!("by {} @ {}\n", post.author, post.date));
            if !post.tags.is_empty() {
                preview.push_str(&format!("tags: {}\n", post.tags.join(", ")));
            }
            preview.push_str("\n---\n\n");
            preview.push_str(&post.content);
            return Some(preview);
        }

        None
    }

    fn search(&self, query: &str) -> Vec<ListItem> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for board in &self.boards {
            // Search board names
            if board.name.to_lowercase().contains(&query_lower)
                || board.description.to_lowercase().contains(&query_lower)
            {
                results.push(ListItem {
                    id: board.id.clone(),
                    title: board.name.clone(),
                    subtitle: Some(board.description.clone()),
                    kind: ItemKind::Board,
                    has_children: true,
                    meta: None,
                });
            }

            // Search posts
            for post in &board.posts {
                if post.title.to_lowercase().contains(&query_lower)
                    || post.content.to_lowercase().contains(&query_lower)
                    || post.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
                {
                    results.push(ListItem {
                        id: post.id.clone(),
                        title: post.title.clone(),
                        subtitle: Some(format!("{} / {}", board.name, post.date)),
                        kind: ItemKind::Post,
                        has_children: false,
                        meta: Some(post.tags.join(", ")),
                    });
                }
            }
        }

        results
    }

    fn actions(&self, item_id: &str) -> Vec<ActionItem> {
        // Board actions
        if self.find_board(item_id).is_some() {
            return vec![
                ActionItem {
                    id: "view".to_string(),
                    name: "View Posts".to_string(),
                    shortcut: Some("Enter".to_string()),
                    description: "Browse posts in this board".to_string(),
                },
                ActionItem {
                    id: "new_post".to_string(),
                    name: "New Post".to_string(),
                    shortcut: Some("n".to_string()),
                    description: "Create a new post".to_string(),
                },
                ActionItem {
                    id: "refresh".to_string(),
                    name: "Refresh".to_string(),
                    shortcut: Some("r".to_string()),
                    description: "Refresh from server".to_string(),
                },
            ];
        }

        // Post actions
        if self.find_post(item_id).is_some() {
            return vec![
                ActionItem {
                    id: "view".to_string(),
                    name: "View".to_string(),
                    shortcut: Some("v".to_string()),
                    description: "View full post".to_string(),
                },
                ActionItem {
                    id: "edit_metadata".to_string(),
                    name: "Edit Metadata".to_string(),
                    shortcut: Some("m".to_string()),
                    description: "Edit post tags and metadata".to_string(),
                },
                ActionItem {
                    id: "refactor_note".to_string(),
                    name: "Refactor Note".to_string(),
                    shortcut: Some("R".to_string()),
                    description: "Bridge tender: split into smaller notes".to_string(),
                },
                ActionItem {
                    id: "open_editor".to_string(),
                    name: "Open in Editor".to_string(),
                    shortcut: Some("e".to_string()),
                    description: "Edit in external editor".to_string(),
                },
                ActionItem {
                    id: "copy".to_string(),
                    name: "Copy".to_string(),
                    shortcut: Some("y".to_string()),
                    description: "Copy to clipboard".to_string(),
                },
                ActionItem {
                    id: "delete".to_string(),
                    name: "Delete".to_string(),
                    shortcut: Some("d".to_string()),
                    description: "Delete this post".to_string(),
                },
            ];
        }

        vec![]
    }
}

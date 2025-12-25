//! Filesystem source for directory navigation

use std::path::{Path, PathBuf};

use crate::tui::app::{ActionItem, ItemKind, ListItem};
use super::traits::Source;

/// Filesystem source - navigate directories without exiting the app
pub struct FilesystemSource {
    root: PathBuf,
    current: PathBuf,
    show_hidden: bool,
}

impl FilesystemSource {
    /// Create a new filesystem source rooted at a directory
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let current = root.clone();
        Self {
            root,
            current,
            show_hidden: false,
        }
    }

    /// Toggle showing hidden files
    pub fn toggle_hidden(&mut self) {
        self.show_hidden = !self.show_hidden;
    }

    /// Navigate to a directory
    pub fn navigate_to(&mut self, path: impl AsRef<Path>) {
        let path = path.as_ref();
        if path.is_dir() {
            self.current = path.to_path_buf();
        }
    }

    /// Go up one directory (if not at root)
    pub fn go_up(&mut self) -> bool {
        if self.current != self.root {
            if let Some(parent) = self.current.parent() {
                if parent.starts_with(&self.root) || parent == self.root {
                    self.current = parent.to_path_buf();
                    return true;
                }
            }
        }
        false
    }

    fn list_dir(&self, path: &Path) -> Vec<ListItem> {
        let mut items = Vec::new();

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().to_string();

                // Skip hidden files unless show_hidden is true
                if name.starts_with('.') && !self.show_hidden {
                    continue;
                }

                let is_dir = path.is_dir();
                let kind = if is_dir {
                    ItemKind::Folder
                } else {
                    ItemKind::File
                };

                let subtitle = if is_dir {
                    entry
                        .metadata()
                        .ok()
                        .and_then(|_| {
                            std::fs::read_dir(&path).ok().map(|d| {
                                let count = d.count();
                                format!("{} items", count)
                            })
                        })
                } else {
                    entry.metadata().ok().map(|m| {
                        let size = m.len();
                        if size < 1024 {
                            format!("{} B", size)
                        } else if size < 1024 * 1024 {
                            format!("{:.1} KB", size as f64 / 1024.0)
                        } else {
                            format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
                        }
                    })
                };

                items.push(ListItem {
                    id: path.to_string_lossy().to_string(),
                    title: name,
                    subtitle,
                    kind,
                    has_children: is_dir,
                    meta: None,
                });
            }
        }

        // Sort: directories first, then alphabetically
        items.sort_by(|a, b| {
            let a_is_dir = a.kind == ItemKind::Folder;
            let b_is_dir = b.kind == ItemKind::Folder;
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
            }
        });

        items
    }
}

impl Source for FilesystemSource {
    fn id(&self) -> &str {
        "filesystem"
    }

    fn name(&self) -> &str {
        "Files"
    }

    fn list(&self) -> Vec<ListItem> {
        self.list_dir(&self.current)
    }

    fn children(&self, item_id: &str) -> Option<Vec<ListItem>> {
        let path = Path::new(item_id);
        if path.is_dir() {
            Some(self.list_dir(path))
        } else {
            None
        }
    }

    fn preview(&self, item_id: &str) -> Option<String> {
        let path = Path::new(item_id);

        if path.is_dir() {
            // Directory preview: list contents
            let mut preview = format!("Directory: {}\n\n", path.display());
            if let Ok(entries) = std::fs::read_dir(path) {
                let items: Vec<_> = entries.filter_map(|e| e.ok()).take(20).collect();
                for entry in &items {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let kind = if entry.path().is_dir() { "dir" } else { "file" };
                    preview.push_str(&format!("  {} ({})\n", name, kind));
                }
                if items.len() == 20 {
                    preview.push_str("  ...\n");
                }
            }
            return Some(preview);
        }

        if path.is_file() {
            // File preview: show content (if text)
            if let Ok(content) = std::fs::read_to_string(path) {
                let lines: Vec<_> = content.lines().take(50).collect();
                let truncated = lines.len() == 50;
                let mut preview = lines.join("\n");
                if truncated {
                    preview.push_str("\n\n... (truncated)");
                }
                return Some(preview);
            } else {
                // Binary file
                if let Ok(metadata) = path.metadata() {
                    return Some(format!(
                        "Binary file\nSize: {} bytes\n",
                        metadata.len()
                    ));
                }
            }
        }

        None
    }

    fn search(&self, query: &str) -> Vec<ListItem> {
        let query_lower = query.to_lowercase();

        fn search_recursive(
            dir: &Path,
            query: &str,
            results: &mut Vec<ListItem>,
            depth: usize,
        ) {
            if depth > 3 {
                return; // Limit recursion depth
            }

            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.filter_map(|e| e.ok()) {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();

                    if name.to_lowercase().contains(query) {
                        let is_dir = path.is_dir();
                        results.push(ListItem {
                            id: path.to_string_lossy().to_string(),
                            title: name,
                            subtitle: Some(path.parent().unwrap_or(&path).to_string_lossy().to_string()),
                            kind: if is_dir { ItemKind::Folder } else { ItemKind::File },
                            has_children: is_dir,
                            meta: None,
                        });
                    }

                    if path.is_dir() && results.len() < 100 {
                        search_recursive(&path, query, results, depth + 1);
                    }
                }
            }
        }

        let mut results = Vec::new();
        search_recursive(&self.current, &query_lower, &mut results, 0);
        results
    }

    fn actions(&self, item_id: &str) -> Vec<ActionItem> {
        let path = Path::new(item_id);

        if path.is_dir() {
            vec![
                ActionItem {
                    id: "view".to_string(),
                    name: "Enter".to_string(),
                    shortcut: Some("Enter".to_string()),
                    description: "Navigate into directory".to_string(),
                },
                ActionItem {
                    id: "open_terminal".to_string(),
                    name: "Open Terminal".to_string(),
                    shortcut: Some("t".to_string()),
                    description: "Open terminal in this directory".to_string(),
                },
            ]
        } else {
            vec![
                ActionItem {
                    id: "view".to_string(),
                    name: "View".to_string(),
                    shortcut: Some("v".to_string()),
                    description: "View file content".to_string(),
                },
                ActionItem {
                    id: "open_editor".to_string(),
                    name: "Open in Editor".to_string(),
                    shortcut: Some("e".to_string()),
                    description: "Edit in external editor".to_string(),
                },
                ActionItem {
                    id: "copy_path".to_string(),
                    name: "Copy Path".to_string(),
                    shortcut: Some("y".to_string()),
                    description: "Copy file path to clipboard".to_string(),
                },
                ActionItem {
                    id: "delete".to_string(),
                    name: "Delete".to_string(),
                    shortcut: Some("d".to_string()),
                    description: "Delete file".to_string(),
                },
            ]
        }
    }
}

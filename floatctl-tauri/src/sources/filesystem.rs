//! Filesystem source - Local file navigation

use crate::protocol::{Item, ItemKind, Scope, SourceResponse};
use anyhow::Result;
use std::path::PathBuf;

/// Fetch filesystem items
pub async fn fetch(scope: Scope) -> Result<SourceResponse> {
    let base_path = if let Some(ref parent) = scope.parent_id {
        PathBuf::from(parent)
    } else {
        // Default to home directory
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("/"))
    };

    let mut items = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&base_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();

            // Skip hidden files unless explicitly requested
            if name.starts_with('.') {
                continue;
            }

            let is_dir = path.is_dir();
            let item = Item {
                id: path.to_string_lossy().to_string(),
                kind: if is_dir { ItemKind::Board } else { ItemKind::File },
                title: name,
                subtitle: if is_dir {
                    Some("directory".into())
                } else {
                    path.extension()
                        .map(|ext| ext.to_string_lossy().to_string())
                },
                actions: if is_dir {
                    vec!["browse".into()]
                } else {
                    vec!["view".into(), "edit".into()]
                },
                parent_id: scope.parent_id.clone(),
                has_children: is_dir,
                badge: None,
                meta: Default::default(),
            };

            items.push(item);
        }
    }

    // Sort: directories first, then alphabetically
    items.sort_by(|a, b| {
        let a_is_dir = a.kind == ItemKind::Board;
        let b_is_dir = b.kind == ItemKind::Board;
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        }
    });

    // Apply pagination
    let total = items.len();
    let offset = scope.offset.unwrap_or(0);
    let limit = scope.limit.unwrap_or(100);

    let items: Vec<_> = items.into_iter().skip(offset).take(limit).collect();
    let has_more = offset + items.len() < total;

    Ok(SourceResponse {
        items,
        total: Some(total),
        source: "filesystem".into(),
        has_more,
    })
}

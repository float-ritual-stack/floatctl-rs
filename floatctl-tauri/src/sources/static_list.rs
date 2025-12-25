//! Static list source - Fixed data for testing and system items

use crate::protocol::{Item, ItemKind, Scope, SourceResponse};
use anyhow::Result;

/// Fetch static list items
pub async fn fetch(scope: Scope) -> Result<SourceResponse> {
    // Return persona list as default static content
    let items = vec![
        Item {
            id: "kitty".into(),
            kind: ItemKind::Persona,
            title: "Kitty".into(),
            subtitle: Some("The float cat".into()),
            actions: vec!["switch".into(), "inbox".into()],
            parent_id: None,
            has_children: false,
            badge: None,
            meta: Default::default(),
        },
        Item {
            id: "daddy".into(),
            kind: ItemKind::Persona,
            title: "Daddy".into(),
            subtitle: Some("The wise elder".into()),
            actions: vec!["switch".into(), "inbox".into()],
            parent_id: None,
            has_children: false,
            badge: None,
            meta: Default::default(),
        },
        Item {
            id: "cowboy".into(),
            kind: ItemKind::Persona,
            title: "Cowboy".into(),
            subtitle: Some("The frontier spirit".into()),
            actions: vec!["switch".into(), "inbox".into()],
            parent_id: None,
            has_children: false,
            badge: None,
            meta: Default::default(),
        },
        Item {
            id: "evna".into(),
            kind: ItemKind::Persona,
            title: "Evna".into(),
            subtitle: Some("AI assistant".into()),
            actions: vec!["switch".into(), "inbox".into()],
            parent_id: None,
            has_children: false,
            badge: None,
            meta: Default::default(),
        },
    ];

    Ok(SourceResponse {
        items,
        total: Some(4),
        source: "static".into(),
        has_more: false,
    })
}

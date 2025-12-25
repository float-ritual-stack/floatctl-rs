//! Search source - RAG search results

use crate::protocol::{Item, ItemKind, Scope, SourceResponse};
use anyhow::Result;

/// Fetch search results
pub async fn fetch(scope: Scope) -> Result<SourceResponse> {
    let query = match &scope.query {
        Some(q) if !q.is_empty() => q.clone(),
        _ => {
            return Ok(SourceResponse {
                items: vec![],
                total: Some(0),
                source: "search".into(),
                has_more: false,
            });
        }
    };

    // TODO: Connect to floatctl-search for actual RAG queries
    // For now, return mock results

    let items = vec![
        Item {
            id: "search-1".into(),
            kind: ItemKind::SearchResult,
            title: format!("Result for: {}", query),
            subtitle: Some("bridges/CB-20251225-1000-ABCD.md".into()),
            actions: vec!["view".into(), "open_source".into()],
            parent_id: None,
            has_children: false,
            badge: Some("0.95".into()),
            meta: [("score".into(), serde_json::json!(0.95))]
                .into_iter()
                .collect(),
        },
        Item {
            id: "search-2".into(),
            kind: ItemKind::SearchResult,
            title: "Related context".into(),
            subtitle: Some("daily/2025-12-24.md".into()),
            actions: vec!["view".into(), "open_source".into()],
            parent_id: None,
            has_children: false,
            badge: Some("0.82".into()),
            meta: [("score".into(), serde_json::json!(0.82))]
                .into_iter()
                .collect(),
        },
    ];

    Ok(SourceResponse {
        items,
        total: Some(2),
        source: "search".into(),
        has_more: false,
    })
}

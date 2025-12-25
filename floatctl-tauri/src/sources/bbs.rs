//! BBS source - Boards and Posts from the float-box BBS

use crate::protocol::{Item, ItemKind, Scope, SourceResponse};
use anyhow::Result;

/// Fetch BBS boards and posts
pub async fn fetch(scope: Scope) -> Result<SourceResponse> {
    // If we have a parent_id, we're fetching posts for a board
    // Otherwise, we're fetching the board list

    let items = if let Some(parent_id) = &scope.parent_id {
        // Fetch posts for board
        fetch_posts_for_board(parent_id).await?
    } else {
        // Fetch board list
        fetch_boards().await?
    };

    let total = items.len();

    Ok(SourceResponse {
        items,
        total: Some(total),
        source: "bbs".into(),
        has_more: false,
    })
}

async fn fetch_boards() -> Result<Vec<Item>> {
    // TODO: Connect to actual BBS API (floatctl-server)
    // For now, return mock data matching the BBS structure

    let boards = vec![
        Item::board("inbox-kitty", "Kitty Inbox")
            .with_badge("3")
            .with_meta("persona", "kitty"),
        Item::board("inbox-daddy", "Daddy Inbox").with_meta("persona", "daddy"),
        Item::board("inbox-evna", "Evna Inbox")
            .with_badge("1")
            .with_meta("persona", "evna"),
        Item::board("dispatch", "Dispatch Queue").with_meta("system", true),
        Item::board("daily-notes", "Daily Notes").with_meta("type", "notes"),
    ];

    Ok(boards)
}

async fn fetch_posts_for_board(board_id: &str) -> Result<Vec<Item>> {
    // TODO: Fetch from BBS API
    // Mock data for now

    let posts = match board_id {
        "inbox-kitty" => vec![
            Item::post("post-1", "Morning check-in", board_id)
                .with_subtitle("2025-12-25 08:00")
                .with_meta("unread", true),
            Item::post("post-2", "Project update needed", board_id)
                .with_subtitle("2025-12-24 16:30"),
            Item::post("post-3", "Quick question about API", board_id)
                .with_subtitle("2025-12-24 10:15"),
        ],
        "dispatch" => vec![
            Item::post("dispatch-1", "Bridge Tender: floatctl refactor", board_id)
                .with_subtitle("Pending")
                .with_badge("queued"),
        ],
        _ => vec![],
    };

    Ok(posts)
}

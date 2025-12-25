//! Tauri commands - invokable from frontend
//!
//! These commands form the IPC bridge between the WebView and Rust backend.
//! Each command is exposed via `tauri::command` and callable from TypeScript.

use crate::protocol::*;
use crate::sources;
use crate::AppState;
use std::sync::Arc;
use tauri::State;

/// Fetch items from a source with scope constraints
#[tauri::command]
pub async fn fetch_items(
    source: SourceKind,
    scope: Option<Scope>,
    state: State<'_, Arc<AppState>>,
) -> Result<SourceResponse, String> {
    let scope = scope.unwrap_or_default();

    let response = match source {
        SourceKind::Bbs => sources::bbs::fetch(scope).await,
        SourceKind::Filesystem => sources::filesystem::fetch(scope).await,
        SourceKind::Search => sources::search::fetch(scope).await,
        SourceKind::Jobs => sources::jobs::fetch(&state).await,
        SourceKind::Static => sources::static_list::fetch(scope).await,
    };

    response.map_err(|e| e.to_string())
}

/// Execute an action on an item
#[tauri::command]
pub async fn execute_action(
    request: ActionRequest,
    state: State<'_, Arc<AppState>>,
) -> Result<ActionResult, String> {
    tracing::info!(
        action = %request.action_id,
        item = %request.item_id,
        "Executing action"
    );

    // Route to appropriate handler based on action
    let result = match request.action_id.as_str() {
        "view" => handle_view_action(&request).await,
        "browse" => handle_browse_action(&request, &state).await,
        "edit_metadata" => handle_edit_metadata(&request).await,
        "dispatch" => handle_dispatch_action(&request, &state).await,
        "delete" => handle_delete_action(&request).await,
        _ => Ok(ActionResult::Error {
            message: format!("Unknown action: {}", request.action_id),
        }),
    };

    result.map_err(|e| e.to_string())
}

/// Get current navigation state
#[tauri::command]
pub async fn get_navigation_state(
    state: State<'_, Arc<AppState>>,
) -> Result<NavigationState, String> {
    let nav = state.navigation.read().await;
    Ok(nav.clone())
}

/// Set the current UI mode
#[tauri::command]
pub async fn set_mode(mode: Mode, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut nav = state.navigation.write().await;
    nav.mode = mode;
    tracing::debug!(?mode, "Mode changed");
    Ok(())
}

/// Navigate to a specific item or path
#[tauri::command]
pub async fn navigate_to(
    target: String,
    state: State<'_, Arc<AppState>>,
) -> Result<NavigationState, String> {
    let mut nav = state.navigation.write().await;

    // Parse target and update cursor
    // For now, simple implementation - can be expanded
    nav.cursor.item_id = Some(target.clone());
    nav.cursor.path.push(target);
    nav.cursor.depth = nav.cursor.path.len();

    Ok(nav.clone())
}

/// Parse a scratch pane command
#[tauri::command]
pub fn parse_scratch_command(input: String) -> ScratchCommand {
    ScratchCommand::parse(&input)
}

/// Execute a search query
#[tauri::command]
pub async fn search(query: String, scope: Option<Scope>) -> Result<SourceResponse, String> {
    let mut search_scope = scope.unwrap_or_default();
    search_scope.query = Some(query);

    sources::search::fetch(search_scope)
        .await
        .map_err(|e| e.to_string())
}

/// Get available actions for an item
#[tauri::command]
pub fn get_actions_for_item(kind: ItemKind) -> Vec<Action> {
    match kind {
        ItemKind::Board => vec![
            Action::new("view", "View Board").with_shortcut("Enter"),
            Action::new("browse", "Browse Posts").with_shortcut("l"),
        ],
        ItemKind::Post => vec![
            Action::new("view", "View Post").with_shortcut("Enter"),
            Action::new("edit_metadata", "Edit Metadata").with_shortcut("e"),
            Action::new("dispatch", "Dispatch Agent")
                .with_shortcut("d")
                .background(),
            Action::new("delete", "Delete Post")
                .with_shortcut("x")
                .destructive(),
        ],
        ItemKind::Job => vec![
            Action::new("view", "View Job").with_shortcut("Enter"),
            Action::new("cancel", "Cancel Job")
                .with_shortcut("c")
                .destructive(),
        ],
        ItemKind::File => vec![
            Action::new("view", "View File").with_shortcut("Enter"),
            Action::new("edit", "Edit File").with_shortcut("e"),
        ],
        ItemKind::SearchResult => vec![
            Action::new("view", "View Result").with_shortcut("Enter"),
            Action::new("open_source", "Open Source").with_shortcut("o"),
        ],
        ItemKind::Persona => vec![
            Action::new("switch", "Switch Persona").with_shortcut("Enter"),
            Action::new("inbox", "View Inbox").with_shortcut("i"),
        ],
    }
}

// ============================================================================
// Action Handlers
// ============================================================================

async fn handle_view_action(request: &ActionRequest) -> anyhow::Result<ActionResult> {
    Ok(ActionResult::Success {
        message: None,
        navigate_to: Some(format!("/view/{}", request.item_id)),
    })
}

async fn handle_browse_action(
    request: &ActionRequest,
    state: &State<'_, Arc<AppState>>,
) -> anyhow::Result<ActionResult> {
    let mut nav = state.navigation.write().await;
    nav.cursor.path.push(request.item_id.clone());
    nav.cursor.depth += 1;
    nav.cursor.item_id = None;
    nav.cursor.index = 0;

    Ok(ActionResult::Success {
        message: None,
        navigate_to: Some(format!("/browse/{}", request.item_id)),
    })
}

async fn handle_edit_metadata(request: &ActionRequest) -> anyhow::Result<ActionResult> {
    // TODO: Open metadata editor modal
    Ok(ActionResult::Success {
        message: Some("Opening metadata editor".into()),
        navigate_to: None,
    })
}

async fn handle_dispatch_action(
    request: &ActionRequest,
    state: &State<'_, Arc<AppState>>,
) -> anyhow::Result<ActionResult> {
    let job_id = uuid::Uuid::new_v4().to_string();

    // Create job entry
    let job = JobProgress {
        job_id: job_id.clone(),
        status: JobStatus::Pending,
        progress: Some(0),
        message: Some("Queued for dispatch".into()),
        result: None,
    };

    // Add to jobs list
    {
        let mut jobs = state.jobs.write().await;
        jobs.push(job);
    }

    // TODO: Spawn actual agent task

    Ok(ActionResult::JobStarted { job_id })
}

async fn handle_delete_action(request: &ActionRequest) -> anyhow::Result<ActionResult> {
    // TODO: Implement actual deletion with confirmation
    Ok(ActionResult::Error {
        message: "Delete requires confirmation".into(),
    })
}

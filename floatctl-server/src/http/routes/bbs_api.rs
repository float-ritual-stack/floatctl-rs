//! BBS HTTP endpoints - file-based bulletin board system
//!
//! Persona-first routing:
//! - /:persona/inbox - messaging
//! - /:persona/memories - persistent notes
//! - /:persona/boards/:name - shared posting spaces

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::bbs::{board, inbox, memory};
use crate::http::error::ApiError;
use crate::http::server::AppState;
use crate::models::Persona;

// ============================================================================
// Shared Types
// ============================================================================

/// Response wrapper for successful operations
#[derive(Serialize)]
struct SuccessResponse {
    success: bool,
    id: String,
    path: String,
}

// ============================================================================
// Inbox Endpoints
// ============================================================================

/// GET /:persona/inbox query params
#[derive(Debug, Deserialize)]
pub struct InboxListParams {
    /// Max messages to return (default 10, max 100)
    pub limit: Option<usize>,
    /// Only return unread messages
    pub unread_only: Option<bool>,
    /// Filter by sender
    pub from: Option<String>,
}

/// Inbox list response
#[derive(Serialize)]
pub struct InboxListResponse {
    pub messages: Vec<inbox::InboxMessage>,
    pub total_unread: usize,
    pub persona: String,
}

/// GET /:persona/inbox - list inbox messages with optional filters
#[instrument(skip(state), fields(persona = %persona))]
async fn list_inbox_handler(
    State(state): State<Arc<AppState>>,
    Path(persona): Path<String>,
    Query(params): Query<InboxListParams>,
) -> Result<Json<InboxListResponse>, ApiError> {
    let persona_enum = Persona::from_str_validated(&persona, &state.bbs_config.root_dir)?;
    let persona_str = persona_enum.as_str();

    let limit = params.limit.unwrap_or(10).min(100);
    let unread_only = params.unread_only.unwrap_or(false);

    let (messages, total_unread) = inbox::list_inbox(
        &state.bbs_config,
        persona_str,
        limit,
        unread_only,
        params.from.as_deref(),
    )
    .await
    .map_err(|e| ApiError::Internal {
        message: format!("inbox list failed: {}", e),
    })?;

    Ok(Json(InboxListResponse {
        messages,
        total_unread,
        persona: persona_str.to_string(),
    }))
}

/// POST /:persona/inbox request body
#[derive(Deserialize)]
pub struct SendMessageRequest {
    /// Recipient persona
    pub to: String,
    /// Message subject
    pub subject: String,
    /// Message content (markdown)
    pub content: String,
    /// Optional tags
    #[serde(default)]
    pub tags: Vec<String>,
}

/// POST /:persona/inbox - send a message
#[instrument(skip(state, req), fields(from = %from_persona, to = %req.to))]
async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(from_persona): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<SuccessResponse>), ApiError> {
    // Validate both personas against filesystem
    let from = Persona::from_str_validated(&from_persona, &state.bbs_config.root_dir)?;
    let to = Persona::from_str_validated(&req.to, &state.bbs_config.root_dir)?;

    let (message_id, path) = inbox::send_message(
        &state.bbs_config,
        from.as_str(),
        to.as_str(),
        &req.subject,
        &req.content,
        req.tags,
    )
    .await
    .map_err(|e| ApiError::Internal {
        message: format!("send message failed: {}", e),
    })?;

    tracing::info!(
        from = %from,
        to = %to,
        message_id = %message_id,
        "message sent"
    );

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse {
            success: true,
            id: message_id,
            path,
        }),
    ))
}

/// PUT /:persona/inbox/:id/read - mark message as read
#[instrument(skip(state), fields(persona = %persona, message_id = %message_id))]
async fn mark_read(
    State(state): State<Arc<AppState>>,
    Path((persona, message_id)): Path<(String, String)>,
) -> Result<Json<SuccessResponse>, ApiError> {
    let persona_enum = Persona::from_str_validated(&persona, &state.bbs_config.root_dir)?;

    inbox::mark_as_read(&state.bbs_config, persona_enum.as_str(), &message_id)
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("mark read failed: {}", e),
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        id: message_id,
        path: String::new(),
    }))
}

/// PUT /:persona/inbox/:id/unread - mark message as unread
#[instrument(skip(state), fields(persona = %persona, message_id = %message_id))]
async fn mark_unread(
    State(state): State<Arc<AppState>>,
    Path((persona, message_id)): Path<(String, String)>,
) -> Result<Json<SuccessResponse>, ApiError> {
    let persona_enum = Persona::from_str_validated(&persona, &state.bbs_config.root_dir)?;

    inbox::mark_as_unread(&state.bbs_config, persona_enum.as_str(), &message_id)
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("mark unread failed: {}", e),
        })?;

    Ok(Json(SuccessResponse {
        success: true,
        id: message_id,
        path: String::new(),
    }))
}

/// GET /:persona/inbox/:id - get a single message by ID
#[instrument(skip(state), fields(persona = %persona, message_id = %message_id))]
async fn get_message(
    State(state): State<Arc<AppState>>,
    Path((persona, message_id)): Path<(String, String)>,
) -> Result<Json<inbox::InboxMessage>, ApiError> {
    let persona_enum = Persona::from_str_validated(&persona, &state.bbs_config.root_dir)?;

    let message = inbox::get_message(&state.bbs_config, persona_enum.as_str(), &message_id)
        .await
        .map_err(|_| ApiError::NotFound {
            resource: "message",
            id: message_id.clone(),
        })?;

    Ok(Json(message))
}

// ============================================================================
// Memory Endpoints
// ============================================================================

/// GET /:persona/memories query params
#[derive(Debug, Deserialize)]
pub struct MemoryListParams {
    /// Filter by category
    pub category: Option<String>,
    /// Search query
    pub query: Option<String>,
    /// Max memories to return (default 20, max 100)
    pub limit: Option<usize>,
}

/// Memory list response
#[derive(Serialize)]
pub struct MemoryListResponse {
    pub memories: Vec<memory::Memory>,
    pub total: usize,
    pub persona: String,
}

/// GET /:persona/memories - list memories
#[instrument(skip(state), fields(persona = %persona))]
async fn list_memories(
    State(state): State<Arc<AppState>>,
    Path(persona): Path<String>,
    Query(params): Query<MemoryListParams>,
) -> Result<Json<MemoryListResponse>, ApiError> {
    let persona_enum = Persona::from_str_validated(&persona, &state.bbs_config.root_dir)?;
    let persona_str = persona_enum.as_str();

    let limit = params.limit.unwrap_or(20).min(100);

    let memories = memory::list_memories(
        &state.bbs_config,
        persona_str,
        params.category.as_deref(),
        params.query.as_deref(),
        limit,
    )
    .await
    .map_err(|e| ApiError::Internal {
        message: format!("memory list failed: {}", e),
    })?;

    let total = memories.len();

    Ok(Json(MemoryListResponse {
        memories,
        total,
        persona: persona_str.to_string(),
    }))
}

/// POST /:persona/memories request body
#[derive(Deserialize)]
pub struct SaveMemoryRequest {
    /// Memory title
    pub title: String,
    /// Memory content (markdown)
    pub content: String,
    /// Category (patterns, moments, discoveries, reflections)
    pub category: Option<String>,
    /// Optional tags
    #[serde(default)]
    pub tags: Vec<String>,
}

/// POST /:persona/memories - save a memory
#[instrument(skip(state, req), fields(persona = %persona, title = %req.title))]
async fn save_memory(
    State(state): State<Arc<AppState>>,
    Path(persona): Path<String>,
    Json(req): Json<SaveMemoryRequest>,
) -> Result<(StatusCode, Json<SuccessResponse>), ApiError> {
    let persona_enum = Persona::from_str_validated(&persona, &state.bbs_config.root_dir)?;

    let (memory_id, path) = memory::save_memory(
        &state.bbs_config,
        persona_enum.as_str(),
        &req.title,
        &req.content,
        req.category.as_deref(),
        req.tags,
    )
    .await
    .map_err(|e| ApiError::Internal {
        message: format!("save memory failed: {}", e),
    })?;

    tracing::info!(
        persona = %persona_enum,
        memory_id = %memory_id,
        "memory saved"
    );

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse {
            success: true,
            id: memory_id,
            path,
        }),
    ))
}

// ============================================================================
// Board Endpoints
// ============================================================================

/// GET /:persona/boards/:name query params
#[derive(Debug, Deserialize)]
pub struct BoardListParams {
    /// Max posts to return (default 20, max 100)
    pub limit: Option<usize>,
    /// Filter by author
    pub by_author: Option<String>,
    /// Filter by tag
    pub by_tag: Option<String>,
    /// Include full content (default false)
    pub include_content: Option<bool>,
}

/// Board list response
#[derive(Serialize)]
pub struct BoardListResponse {
    pub posts: Vec<board::BoardPost>,
    pub total: usize,
    pub board: String,
}

/// GET /:persona/boards/:name - list board posts
#[instrument(skip(state), fields(persona = %persona, board = %board_name))]
async fn list_board(
    State(state): State<Arc<AppState>>,
    Path((persona, board_name)): Path<(String, String)>,
    Query(params): Query<BoardListParams>,
) -> Result<Json<BoardListResponse>, ApiError> {
    // Validate persona (author context)
    let _persona_enum = Persona::from_str_validated(&persona, &state.bbs_config.root_dir)?;

    let limit = params.limit.unwrap_or(20).min(100);
    let include_content = params.include_content.unwrap_or(false);

    let posts = board::list_board(
        &state.bbs_config,
        &board_name,
        limit,
        params.by_author.as_deref(),
        params.by_tag.as_deref(),
        include_content,
    )
    .await
    .map_err(|e| ApiError::Internal {
        message: format!("board list failed: {}", e),
    })?;

    let total = posts.len();

    Ok(Json(BoardListResponse {
        posts,
        total,
        board: board_name,
    }))
}

/// POST /:persona/boards/:name request body
#[derive(Deserialize)]
pub struct PostToBoardRequest {
    /// Post title
    pub title: String,
    /// Post content (markdown)
    pub content: String,
    /// Imprint tag (default "field-notes")
    pub imprint: Option<String>,
    /// Optional tags
    #[serde(default)]
    pub tags: Vec<String>,
}

/// POST /:persona/boards/:name - post to board
#[instrument(skip(state, req), fields(persona = %persona, board = %board_name, title = %req.title))]
async fn post_to_board(
    State(state): State<Arc<AppState>>,
    Path((persona, board_name)): Path<(String, String)>,
    Json(req): Json<PostToBoardRequest>,
) -> Result<(StatusCode, Json<SuccessResponse>), ApiError> {
    let persona_enum = Persona::from_str_validated(&persona, &state.bbs_config.root_dir)?;

    let (post_id, path) = board::post_to_board(
        &state.bbs_config,
        &board_name,
        persona_enum.as_str(),
        &req.title,
        &req.content,
        req.imprint.as_deref(),
        req.tags,
    )
    .await
    .map_err(|e| ApiError::Internal {
        message: format!("post to board failed: {}", e),
    })?;

    tracing::info!(
        author = %persona_enum,
        board = %board_name,
        post_id = %post_id,
        "posted to board"
    );

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse {
            success: true,
            id: post_id,
            path,
        }),
    ))
}

/// GET /boards - list all available boards
#[derive(Serialize)]
pub struct BoardsListResponse {
    pub boards: Vec<String>,
}

#[instrument(skip(state))]
async fn list_all_boards(
    State(state): State<Arc<AppState>>,
) -> Result<Json<BoardsListResponse>, ApiError> {
    let boards = board::list_boards(&state.bbs_config)
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("list boards failed: {}", e),
        })?;

    Ok(Json(BoardsListResponse { boards }))
}

// ============================================================================
// Persona Endpoints
// ============================================================================

/// GET /bbs/personas - list all available personas
#[derive(Serialize)]
pub struct PersonasListResponse {
    pub personas: Vec<String>,
}

#[instrument(skip(state))]
async fn list_all_personas(
    State(state): State<Arc<AppState>>,
) -> Result<Json<PersonasListResponse>, ApiError> {
    let personas = Persona::list_all(&state.bbs_config.root_dir)
        .into_iter()
        .map(|p| p.as_str().to_string())
        .collect();

    Ok(Json(PersonasListResponse { personas }))
}

// ============================================================================
// Router
// ============================================================================

/// BBS API routes
///
/// Mounts:
/// - /:persona/inbox
/// - /:persona/memories
/// - /:persona/boards/:name
/// - /boards (list all)
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Inbox routes
        .route("/{persona}/inbox", get(list_inbox_handler))
        .route("/{persona}/inbox", post(send_message))
        .route("/{persona}/inbox/{id}", get(get_message))
        .route("/{persona}/inbox/{id}/read", put(mark_read))
        .route("/{persona}/inbox/{id}/unread", put(mark_unread))
        // Memory routes
        .route("/{persona}/memories", get(list_memories))
        .route("/{persona}/memories", post(save_memory))
        // Board routes
        .route("/{persona}/boards/{name}", get(list_board))
        .route("/{persona}/boards/{name}", post(post_to_board))
        // List all boards (not persona-scoped)
        .route("/bbs/boards", get(list_all_boards))
        // List all available personas
        .route("/bbs/personas", get(list_all_personas))
}

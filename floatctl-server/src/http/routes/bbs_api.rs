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
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::instrument;
use walkdir::WalkDir;

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
// File Search Endpoints
// ============================================================================

/// GET /bbs/files query params
#[derive(Debug, Deserialize)]
pub struct SearchFilesParams {
    /// Search query (fuzzy match on filename)
    pub q: String,
    /// Max results (default 20, max 100)
    pub limit: Option<usize>,
}

/// File match result
#[derive(Debug, Serialize)]
pub struct FileMatch {
    pub id: String,
    pub r#type: String,
    pub title: String,
    pub preview: String,
    pub date: String,
}

/// Search files response
#[derive(Serialize)]
pub struct SearchFilesResponse {
    pub matches: Vec<FileMatch>,
    pub paths_searched: Vec<String>,
}

/// GET /bbs/files - search configured filesystem paths
#[instrument(skip(state), fields(query = %params.q))]
async fn search_files(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchFilesParams>,
) -> Result<Json<SearchFilesResponse>, ApiError> {
    let query_lower = params.q.to_lowercase();
    let limit = params.limit.unwrap_or(20).min(100);
    let mut matches = Vec::new();
    let paths_searched: Vec<String> = state
        .bbs_config
        .search_paths
        .iter()
        .map(|p| p.display().to_string())
        .collect();

    for base_path in &state.bbs_config.search_paths {
        if !base_path.exists() {
            tracing::debug!(path = %base_path.display(), "search path does not exist, skipping");
            continue;
        }

        for entry in WalkDir::new(base_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if matches.len() >= limit {
                break;
            }

            let path = entry.path();
            if !path.extension().map(|e| e == "md").unwrap_or(false) {
                continue;
            }

            let filename = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

            // Fuzzy match on filename
            if filename.to_lowercase().contains(&query_lower) {
                // Extract title from frontmatter or use filename
                let title = tokio::fs::read_to_string(path)
                    .await
                    .ok()
                    .and_then(|content| {
                        if content.starts_with("---") {
                            content
                                .lines()
                                .skip(1)
                                .take_while(|l| *l != "---")
                                .find(|l| l.starts_with("title:"))
                                .map(|l| {
                                    l.trim_start_matches("title:")
                                        .trim()
                                        .trim_matches('"')
                                        .to_string()
                                })
                        } else {
                            None
                        }
                    })
                    .unwrap_or_else(|| filename.to_string());

                // Get file modified time
                let date = tokio::fs::metadata(path)
                    .await
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .map(|t| DateTime::<Utc>::from(t).to_rfc3339())
                    .unwrap_or_else(|| Utc::now().to_rfc3339());

                // Determine type from path
                let type_str = if path.to_string_lossy().contains("bridge") {
                    "file::bridge"
                } else if path.to_string_lossy().contains("imprint") {
                    "file::imprint"
                } else if path.to_string_lossy().contains("daily") {
                    "file::daily"
                } else {
                    "file"
                };

                matches.push(FileMatch {
                    id: path.display().to_string(),
                    r#type: type_str.to_string(),
                    title,
                    preview: path.display().to_string(),
                    date,
                });
            }
        }
    }

    tracing::info!(matches = %matches.len(), "file search results");

    Ok(Json(SearchFilesResponse {
        matches,
        paths_searched,
    }))
}

/// GET /bbs/files/:path - read file content
#[instrument(skip(state))]
async fn read_file(
    State(state): State<Arc<AppState>>,
    Path(file_path): Path<String>,
) -> Result<String, ApiError> {
    let path = std::path::Path::new(&file_path);

    // Security: ensure path is within search_paths
    let allowed = state.bbs_config.search_paths.iter().any(|base| {
        path.starts_with(base)
    });

    if !allowed {
        return Err(ApiError::Forbidden {
            reason: "Path not in allowed search paths".to_string(),
        });
    }

    tokio::fs::read_to_string(path)
        .await
        .map_err(|_| ApiError::NotFound {
            resource: "file",
            id: file_path,
        })
}

// ============================================================================
// R2 Search Endpoints (server-side rclone - clients don't need rclone)
// ============================================================================

/// GET /bbs/r2/search query params
#[derive(Debug, Deserialize)]
pub struct R2SearchParams {
    /// Search query (fuzzy match on filename)
    pub q: String,
    /// Max results (default 20, max 100)
    pub limit: Option<usize>,
}

/// R2 search response
#[derive(Serialize)]
pub struct R2SearchResponse {
    pub matches: Vec<FileMatch>,
    pub bucket: String,
}

/// GET /bbs/r2/search - search R2 bucket using rclone (runs on server)
#[instrument(skip(_state), fields(query = %params.q))]
async fn search_r2(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<R2SearchParams>,
) -> Result<Json<R2SearchResponse>, ApiError> {
    let pattern = format!("*{}*", params.q);
    let limit = params.limit.unwrap_or(20).min(100);

    let output = tokio::process::Command::new("rclone")
        .args(["lsf", "r2:sysops-beta/", "--include", &pattern, "-R"])
        .output()
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("rclone not available: {}", e),
        })?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(error = %stderr, "rclone search failed");
        return Err(ApiError::Internal {
            message: format!("rclone search failed: {}", stderr),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let matches: Vec<FileMatch> = stdout
        .lines()
        .filter(|l| !l.is_empty() && !l.ends_with('/'))
        .take(limit)
        .map(|path| {
            let filename = path.rsplit('/').next().unwrap_or(path);
            FileMatch {
                id: path.to_string(),
                r#type: "r2".to_string(),
                title: filename.to_string(),
                preview: path.to_string(),
                date: String::new(),
            }
        })
        .collect();

    tracing::info!(matches = %matches.len(), pattern = %pattern, "R2 search results");

    Ok(Json(R2SearchResponse {
        matches,
        bucket: "r2:sysops-beta".to_string(),
    }))
}

/// GET /bbs/r2/files/{path} - read file from R2 bucket
#[instrument(skip(_state))]
async fn read_r2_file(
    State(_state): State<Arc<AppState>>,
    Path(file_path): Path<String>,
) -> Result<String, ApiError> {
    let r2_path = format!("r2:sysops-beta/{}", file_path);

    let output = tokio::process::Command::new("rclone")
        .args(["cat", &r2_path])
        .output()
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("rclone not available: {}", e),
        })?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::debug!(error = %stderr, path = %r2_path, "rclone fetch failed");
        Err(ApiError::NotFound {
            resource: "r2 file",
            id: file_path,
        })
    }
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
        // File search (searches get_search_paths from config)
        .route("/bbs/files", get(search_files))
        .route("/bbs/files/{*path}", get(read_file))
        // R2 search (server-side rclone - clients don't need rclone installed)
        .route("/bbs/r2/search", get(search_r2))
        .route("/bbs/r2/files/{*path}", get(read_r2_file))
}

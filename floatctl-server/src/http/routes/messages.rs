//! Message endpoints - Spec 2.3

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::repos::{MessageRepo, Message};
use crate::http::error::ApiError;
use crate::http::server::AppState;
use crate::models::{MessageContent, Paginated, Pagination, PaginationParams, MarkerKind};

/// Create message request
#[derive(Deserialize)]
pub struct CreateMessageRequest {
    pub content: String,
    pub author: Option<String>,
}

/// Message response
#[derive(Serialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub content: String,
    pub author: Option<String>,
    pub created_at: String,
}

impl From<Message> for MessageResponse {
    fn from(m: Message) -> Self {
        Self {
            id: m.id,
            thread_id: m.thread_id,
            content: m.content,
            author: m.author,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

/// Marker filter query params
#[derive(Deserialize, Default)]
pub struct MarkerFilterParams {
    pub ctx: Option<String>,
    pub project: Option<String>,
    pub mode: Option<String>,
    pub bridge: Option<String>,
    #[serde(flatten)]
    pub pagination: PaginationParams,
}

impl MarkerFilterParams {
    /// Convert to list of (MarkerKind, value) filters
    pub fn to_filters(&self) -> Vec<(MarkerKind, String)> {
        let mut filters = vec![];
        if let Some(v) = &self.ctx {
            filters.push((MarkerKind::Ctx, v.clone()));
        }
        if let Some(v) = &self.project {
            filters.push((MarkerKind::Project, v.clone()));
        }
        if let Some(v) = &self.mode {
            filters.push((MarkerKind::Mode, v.clone()));
        }
        if let Some(v) = &self.bridge {
            filters.push((MarkerKind::Bridge, v.clone()));
        }
        filters
    }
}

/// GET /threads/{id}/messages - list messages for a thread
async fn list_messages(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<MessageResponse>>, ApiError> {
    let page = Pagination::from(params);
    let result = MessageRepo::new(&state.pool)
        .list_for_thread(thread_id, page)
        .await?;

    Ok(Json(Paginated {
        items: result.items.into_iter().map(MessageResponse::from).collect(),
        total: result.total,
        page: result.page,
        per_page: result.per_page,
    }))
}

/// POST /threads/{id}/messages - add a message to a thread
async fn create_message(
    State(state): State<Arc<AppState>>,
    Path(thread_id): Path<Uuid>,
    Json(req): Json<CreateMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), ApiError> {
    let content = MessageContent::new(&req.content)?;

    let message = MessageRepo::new(&state.pool)
        .create(thread_id, content, req.author)
        .await?;

    Ok((StatusCode::CREATED, Json(MessageResponse::from(message))))
}

/// GET /threads?{filters} - search threads by markers
async fn search_threads(
    State(state): State<Arc<AppState>>,
    Query(params): Query<MarkerFilterParams>,
) -> Result<Json<Paginated<Uuid>>, ApiError> {
    let filters = params.to_filters();
    let page = Pagination::from(params.pagination);

    let result = MessageRepo::new(&state.pool)
        .search_by_markers(&filters, page)
        .await?;

    Ok(Json(result))
}

/// Message routes
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/threads", get(search_threads))
        .route("/threads/{id}/messages", get(list_messages).post(create_message))
}

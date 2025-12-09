//! Inbox endpoints - Spec 3.2

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::repos::{InboxRepo, InboxMessage};
use crate::http::error::ApiError;
use crate::http::server::AppState;
use crate::models::{Persona, MessageContent, Paginated, Pagination, PaginationParams};

/// Send message request
#[derive(Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
    pub from: Option<String>,
}

/// Inbox message response
#[derive(Serialize)]
pub struct InboxMessageResponse {
    pub id: Uuid,
    pub persona: String,
    pub content: String,
    pub from_persona: Option<String>,
    pub created_at: String,
}

impl From<InboxMessage> for InboxMessageResponse {
    fn from(m: InboxMessage) -> Self {
        Self {
            id: m.id,
            persona: m.persona,
            content: m.content,
            from_persona: m.from_persona,
            created_at: m.created_at.to_rfc3339(),
        }
    }
}

/// GET /inbox/{persona} - list unread messages
async fn list_inbox(
    State(state): State<Arc<AppState>>,
    Path(persona_str): Path<String>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Paginated<InboxMessageResponse>>, ApiError> {
    let persona = Persona::from_str(&persona_str)?;
    let page = Pagination::from(params);

    let result = InboxRepo::new(&state.pool)
        .list_unread(persona, page)
        .await?;

    Ok(Json(Paginated {
        items: result.items.into_iter().map(InboxMessageResponse::from).collect(),
        total: result.total,
        page: result.page,
        per_page: result.per_page,
    }))
}

/// POST /inbox/{persona} - send message to persona
async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(persona_str): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<InboxMessageResponse>), ApiError> {
    let persona = Persona::from_str(&persona_str)?;
    let content = MessageContent::new(&req.content)?;
    let from = req.from.map(|s| Persona::from_str(&s)).transpose()?;

    let message = InboxRepo::new(&state.pool)
        .send(persona, content, from)
        .await?;

    Ok((StatusCode::CREATED, Json(InboxMessageResponse::from(message))))
}

/// DELETE /inbox/{persona}/{id} - mark as read / delete
async fn delete_message(
    State(state): State<Arc<AppState>>,
    Path((persona_str, message_id)): Path<(String, Uuid)>,
) -> Result<StatusCode, ApiError> {
    let persona = Persona::from_str(&persona_str)?;

    InboxRepo::new(&state.pool)
        .delete(persona, message_id)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

/// Inbox routes
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/inbox/{persona}", get(list_inbox).post(send_message))
        .route("/inbox/{persona}/{id}", delete(delete_message))
}

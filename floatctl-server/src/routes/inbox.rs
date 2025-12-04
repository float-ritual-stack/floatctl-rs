//! Inbox routes - Per-persona message queues for async handoffs

use axum::{
    extract::{Path, Query, State},
    Json,
};
use uuid::Uuid;

use crate::db::Database;
use crate::error::{ServerError, ServerResult};
use crate::models::{CreateInboxMessageRequest, InboxMessage, InboxQueryParams};

/// GET /inbox/:persona - Fetch messages for a persona
pub async fn list_inbox(
    State(db): State<Database>,
    Path(persona): Path<String>,
    Query(params): Query<InboxQueryParams>,
) -> ServerResult<Json<Vec<InboxMessage>>> {
    let messages = db.list_inbox(&persona, &params)?;
    Ok(Json(messages))
}

/// POST /inbox/:persona - Send a message to a persona's inbox
pub async fn send_to_inbox(
    State(db): State<Database>,
    Path(persona): Path<String>,
    Json(req): Json<CreateInboxMessageRequest>,
) -> ServerResult<Json<InboxMessage>> {
    if req.content.is_empty() {
        return Err(ServerError::BadRequest(
            "Message content cannot be empty".into(),
        ));
    }

    let message = db.send_inbox_message(&persona, &req)?;
    Ok(Json(message))
}

/// DELETE /inbox/:persona/:id - Mark message as read/processed (deletes it)
pub async fn delete_inbox_message(
    State(db): State<Database>,
    Path((persona, id)): Path<(String, Uuid)>,
) -> ServerResult<Json<serde_json::Value>> {
    let deleted = db.delete_inbox_message(&persona, id)?;

    if deleted {
        Ok(Json(serde_json::json!({
            "success": true,
            "message": "Message deleted"
        })))
    } else {
        Err(ServerError::NotFound(format!(
            "Message {} not found in {}'s inbox",
            id, persona
        )))
    }
}

/// POST /inbox/:persona/:id/read - Mark message as read (keeps it)
pub async fn mark_inbox_read(
    State(db): State<Database>,
    Path((persona, id)): Path<(String, Uuid)>,
) -> ServerResult<Json<serde_json::Value>> {
    let marked = db.mark_inbox_read(&persona, id)?;

    if marked {
        Ok(Json(serde_json::json!({
            "success": true,
            "message": "Message marked as read"
        })))
    } else {
        Err(ServerError::NotFound(format!(
            "Message {} not found in {}'s inbox",
            id, persona
        )))
    }
}

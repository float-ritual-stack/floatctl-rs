//! Common area routes - Shared scratch space for all agents

use axum::{
    extract::{Path, Query, State},
    Json,
};

use crate::db::Database;
use crate::error::{ServerError, ServerResult};
use crate::models::{CommonItem, CommonQueryParams, CreateCommonItemRequest, UpdateCommonItemRequest};

/// GET /common - List items in common area
pub async fn list_common(
    State(db): State<Database>,
    Query(params): Query<CommonQueryParams>,
) -> ServerResult<Json<Vec<CommonItem>>> {
    let items = db.list_common(&params)?;
    Ok(Json(items))
}

/// POST /common - Add item to common area
pub async fn create_common(
    State(db): State<Database>,
    Json(req): Json<CreateCommonItemRequest>,
) -> ServerResult<Json<CommonItem>> {
    if req.key.is_empty() {
        return Err(ServerError::BadRequest("Key cannot be empty".into()));
    }

    let item = db.set_common(&req)?;
    Ok(Json(item))
}

/// GET /common/:key - Get specific item
pub async fn get_common(
    State(db): State<Database>,
    Path(key): Path<String>,
) -> ServerResult<Json<CommonItem>> {
    let item = db
        .get_common(&key)?
        .ok_or_else(|| ServerError::NotFound(format!("Item '{}' not found or expired", key)))?;

    Ok(Json(item))
}

/// PUT /common/:key - Update existing item
pub async fn update_common(
    State(db): State<Database>,
    Path(key): Path<String>,
    Json(req): Json<UpdateCommonItemRequest>,
) -> ServerResult<Json<CommonItem>> {
    // Check if exists first
    if db.get_common(&key)?.is_none() {
        return Err(ServerError::NotFound(format!(
            "Item '{}' not found or expired",
            key
        )));
    }

    let create_req = CreateCommonItemRequest {
        key,
        value: req.value,
        ttl_seconds: req.ttl_seconds,
        created_by: None,
    };

    let item = db.set_common(&create_req)?;
    Ok(Json(item))
}

/// DELETE /common/:key - Delete item
pub async fn delete_common(
    State(db): State<Database>,
    Path(key): Path<String>,
) -> ServerResult<Json<serde_json::Value>> {
    let deleted = db.delete_common(&key)?;

    if deleted {
        Ok(Json(serde_json::json!({
            "success": true,
            "message": format!("Item '{}' deleted", key)
        })))
    } else {
        Err(ServerError::NotFound(format!("Item '{}' not found", key)))
    }
}

/// POST /common/cleanup - Cleanup expired items
pub async fn cleanup_common(State(db): State<Database>) -> ServerResult<Json<serde_json::Value>> {
    let count = db.cleanup_expired()?;

    Ok(Json(serde_json::json!({
        "success": true,
        "deleted_count": count
    })))
}

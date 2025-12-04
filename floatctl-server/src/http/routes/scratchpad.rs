//! Scratchpad endpoints - Spec 3.3

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{delete, get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::db::repos::{ScratchpadRepo, ScratchpadItem};
use crate::http::error::ApiError;
use crate::http::server::AppState;
use crate::models::{Pagination, PaginationParams};

/// Create/update scratchpad item request
#[derive(Deserialize)]
pub struct UpsertItemRequest {
    pub key: String,
    pub value: JsonValue,
    pub ttl_seconds: Option<i64>,
}

/// Scratchpad item response
#[derive(Serialize)]
pub struct ScratchpadItemResponse {
    pub key: String,
    pub value: JsonValue,
    pub expires_at: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<ScratchpadItem> for ScratchpadItemResponse {
    fn from(item: ScratchpadItem) -> Self {
        Self {
            key: item.key,
            value: item.value,
            expires_at: item.expires_at.map(|dt| dt.to_rfc3339()),
            created_at: item.created_at.to_rfc3339(),
            updated_at: item.updated_at.to_rfc3339(),
        }
    }
}

/// GET /common - list all scratchpad items
async fn list_items(
    State(state): State<Arc<AppState>>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<Vec<ScratchpadItemResponse>>, ApiError> {
    let page = Pagination::from(params);
    let items = ScratchpadRepo::new(&state.pool).list(page).await?;

    Ok(Json(items.into_iter().map(ScratchpadItemResponse::from).collect()))
}

/// POST /common - upsert an item
async fn upsert_item(
    State(state): State<Arc<AppState>>,
    Json(req): Json<UpsertItemRequest>,
) -> Result<(StatusCode, Json<ScratchpadItemResponse>), ApiError> {
    // Validate key length
    if req.key.len() > 256 {
        return Err(ApiError::Validation(crate::models::ValidationError::TooLong {
            field: "key",
            max: 256,
        }));
    }

    let item = ScratchpadRepo::new(&state.pool)
        .upsert(&req.key, req.value, req.ttl_seconds)
        .await?;

    Ok((StatusCode::CREATED, Json(ScratchpadItemResponse::from(item))))
}

/// GET /common/{key} - get a single item
async fn get_item(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<Json<ScratchpadItemResponse>, ApiError> {
    let item = ScratchpadRepo::new(&state.pool)
        .get(&key)
        .await?
        .ok_or_else(|| ApiError::NotFound {
            resource: "scratchpad item",
            id: key,
        })?;

    Ok(Json(ScratchpadItemResponse::from(item)))
}

/// DELETE /common/{key} - delete an item
async fn delete_item(
    State(state): State<Arc<AppState>>,
    Path(key): Path<String>,
) -> Result<StatusCode, ApiError> {
    ScratchpadRepo::new(&state.pool).delete(&key).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Scratchpad routes
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/common", get(list_items).post(upsert_item))
        .route("/common/{key}", get(get_item).delete(delete_item))
}

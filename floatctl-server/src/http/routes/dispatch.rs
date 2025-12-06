//! Dispatch capture endpoints - JSONL file storage
//!
//! Captures context dispatches from Raycast/Chrome and stores in JSONL format.
//! Replaces the Hono-based highlight-receiver service.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::http::error::ApiError;
use crate::http::server::AppState;

/// Default JSONL file path for dispatches
const DEFAULT_DISPATCH_FILE: &str = "/opt/float/bbs/inbox/dispatches.jsonl";

/// Capture dispatch request
/// Accepts both new field names (content) and legacy field names (highlighted_text)
/// for backward compatibility with Raycast sender.
#[derive(Deserialize)]
pub struct CaptureRequest {
    /// The dispatch content (ctx:: marker, highlight text, etc.)
    /// Also accepts `highlighted_text` for backward compatibility
    #[serde(alias = "highlighted_text")]
    pub content: String,
    /// Route to persona inbox (kitty, daddy, cowboy, evna)
    pub route_to: Option<String>,
    /// Optional tags
    #[serde(default)]
    pub tags: Vec<String>,
    /// Optional annotation/note
    pub annotation: Option<String>,
    /// Source URL (for web highlights)
    pub source_url: Option<String>,
    /// Source page title
    pub source_title: Option<String>,
}

/// Stored dispatch entry
#[derive(Serialize, Deserialize, Clone)]
pub struct Dispatch {
    pub id: Uuid,
    pub ts: DateTime<Utc>,
    pub content: String,
    pub route_to: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub annotation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_title: Option<String>,
}

/// Capture response
/// Includes `success: true` for backward compatibility with Raycast sender
#[derive(Serialize)]
pub struct CaptureResponse {
    pub success: bool,
    pub id: Uuid,
    pub ts: String,
    pub route_to: String,
}

/// List query parameters
#[derive(Deserialize)]
pub struct ListParams {
    /// Filter by route_to persona
    pub route_to: Option<String>,
    /// Max entries to return (default 20, max 100)
    pub limit: Option<usize>,
}

/// List response
#[derive(Serialize)]
pub struct ListResponse {
    pub dispatches: Vec<Dispatch>,
    pub count: usize,
    pub total: usize,
}

/// POST /dispatch/capture - capture a new dispatch
async fn capture_dispatch(
    State(_state): State<Arc<AppState>>,
    Json(req): Json<CaptureRequest>,
) -> Result<(StatusCode, Json<CaptureResponse>), ApiError> {
    // Validate content not empty
    if req.content.trim().is_empty() {
        return Err(ApiError::Validation(crate::models::ValidationError::Empty {
            field: "content",
        }));
    }

    // Create dispatch entry
    let dispatch = Dispatch {
        id: Uuid::new_v4(),
        ts: Utc::now(),
        content: req.content,
        route_to: req.route_to.unwrap_or_else(|| "kitty".to_string()),
        tags: req.tags,
        annotation: req.annotation,
        source_url: req.source_url,
        source_title: req.source_title,
    };

    // Serialize to JSONL line
    let line = serde_json::to_string(&dispatch).map_err(|e| ApiError::Internal {
        message: format!("serialization error: {}", e),
    })?;

    // Append to file (create if doesn't exist)
    let file_path = std::env::var("DISPATCH_FILE").unwrap_or_else(|_| DEFAULT_DISPATCH_FILE.to_string());

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&file_path)
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("failed to open dispatch file: {}", e),
        })?;

    file.write_all(format!("{}\n", line).as_bytes())
        .await
        .map_err(|e| ApiError::Internal {
            message: format!("failed to write dispatch: {}", e),
        })?;

    tracing::info!(
        dispatch_id = %dispatch.id,
        route_to = %dispatch.route_to,
        content_preview = %dispatch.content.chars().take(50).collect::<String>(),
        "dispatch captured"
    );

    Ok((
        StatusCode::CREATED,
        Json(CaptureResponse {
            success: true,
            id: dispatch.id,
            ts: dispatch.ts.to_rfc3339(),
            route_to: dispatch.route_to,
        }),
    ))
}

/// GET /dispatch/list - list recent dispatches
async fn list_dispatches(
    State(_state): State<Arc<AppState>>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResponse>, ApiError> {
    let file_path = std::env::var("DISPATCH_FILE").unwrap_or_else(|_| DEFAULT_DISPATCH_FILE.to_string());

    // Read file contents (or empty if doesn't exist)
    let content = match fs::read_to_string(&file_path).await {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            return Err(ApiError::Internal {
                message: format!("failed to read dispatch file: {}", e),
            });
        }
    };

    // Parse all lines
    let all_dispatches: Vec<Dispatch> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();

    let total = all_dispatches.len();

    // Filter by route_to if specified
    let filtered: Vec<Dispatch> = if let Some(ref route) = params.route_to {
        all_dispatches
            .into_iter()
            .filter(|d| d.route_to == *route)
            .collect()
    } else {
        all_dispatches
    };

    // Apply limit (default 20, max 100)
    let limit = params.limit.unwrap_or(20).min(100);

    // Return most recent first
    let dispatches: Vec<Dispatch> = filtered
        .into_iter()
        .rev()
        .take(limit)
        .collect();

    Ok(Json(ListResponse {
        count: dispatches.len(),
        dispatches,
        total,
    }))
}

/// GET /dispatch/{id} - get a specific dispatch by ID
async fn get_dispatch(
    State(_state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Dispatch>, ApiError> {
    let file_path = std::env::var("DISPATCH_FILE").unwrap_or_else(|_| DEFAULT_DISPATCH_FILE.to_string());

    let content = fs::read_to_string(&file_path).await.map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            ApiError::NotFound {
                resource: "dispatch",
                id: id.to_string(),
            }
        } else {
            ApiError::Internal {
                message: format!("failed to read dispatch file: {}", e),
            }
        }
    })?;

    // Find the dispatch with matching ID
    let dispatch = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| serde_json::from_str::<Dispatch>(line).ok())
        .find(|d| d.id == id)
        .ok_or_else(|| ApiError::NotFound {
            resource: "dispatch",
            id: id.to_string(),
        })?;

    Ok(Json(dispatch))
}

/// Dispatch routes
pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/dispatch/capture", post(capture_dispatch))
        .route("/dispatch/list", get(list_dispatches))
        .route("/dispatch/{id}", get(get_dispatch))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_capture_request() {
        let json = r#"{"content": "ctx::test", "route_to": "kitty"}"#;
        let req: CaptureRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "ctx::test");
        assert_eq!(req.route_to, Some("kitty".to_string()));
    }

    #[test]
    fn deserialize_minimal_request() {
        let json = r#"{"content": "test"}"#;
        let req: CaptureRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.content, "test");
        assert!(req.route_to.is_none());
        assert!(req.tags.is_empty());
    }

    #[test]
    fn serialize_dispatch() {
        let dispatch = Dispatch {
            id: Uuid::nil(),
            ts: DateTime::parse_from_rfc3339("2025-12-06T12:00:00Z")
                .unwrap()
                .with_timezone(&Utc),
            content: "test".to_string(),
            route_to: "kitty".to_string(),
            tags: vec![],
            annotation: None,
            source_url: None,
            source_title: None,
        };
        let json = serde_json::to_string(&dispatch).unwrap();
        assert!(json.contains(r#""content":"test""#));
        // Optional fields should be omitted when None
        assert!(!json.contains("annotation"));
    }
}

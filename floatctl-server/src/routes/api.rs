//! API routes for floatctl functionality

use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use crate::state::AppState;
use crate::Result;

/// API routes: /api/*
pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        // Query endpoints (wrapping floatctl-embed)
        .nest("/query", query_router())
        // Claude session endpoints
        .nest("/claude", claude_router())
        // Sync status
        .route("/sync/status", get(sync_status))
}

fn query_router() -> Router<AppState> {
    Router::new()
        .route("/messages", get(query_messages))
        .route("/notes", get(query_notes))
        .route("/active", get(query_active_context))
}

fn claude_router() -> Router<AppState> {
    Router::new()
        .route("/sessions", get(list_sessions))
        .route("/sessions/{session_id}", get(get_session))
}

// ============================================================================
// Health & Status
// ============================================================================

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}

#[derive(Serialize)]
struct StatusResponse {
    database: bool,
    #[cfg(feature = "embed")]
    embeddings: bool,
}

async fn status(State(state): State<AppState>) -> Result<Json<StatusResponse>> {
    // Test database connection
    let db_ok = sqlx::query("SELECT 1")
        .execute(state.pool())
        .await
        .is_ok();

    Ok(Json(StatusResponse {
        database: db_ok,
        #[cfg(feature = "embed")]
        embeddings: std::env::var("OPENAI_API_KEY").is_ok(),
    }))
}

// ============================================================================
// Query endpoints
// ============================================================================

#[derive(Deserialize)]
#[allow(dead_code)]
struct QueryParams {
    q: String,
    limit: Option<i32>,
}

#[derive(Serialize)]
struct QueryResult {
    query: String,
    results: Vec<SearchResult>,
}

#[derive(Serialize)]
struct SearchResult {
    id: String,
    content: String,
    similarity: f64,
    source: String,
    metadata: Option<serde_json::Value>,
}

async fn query_messages(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<QueryResult>> {
    // This would call into floatctl-embed when available
    // For now, return a placeholder indicating the endpoint exists
    let _ = state; // silence unused warning

    Ok(Json(QueryResult {
        query: params.q,
        results: vec![],
    }))
}

async fn query_notes(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<QueryResult>> {
    let _ = state;

    Ok(Json(QueryResult {
        query: params.q,
        results: vec![],
    }))
}

async fn query_active_context(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<QueryResult>> {
    let _ = state;

    Ok(Json(QueryResult {
        query: params.q,
        results: vec![],
    }))
}

// ============================================================================
// Claude sessions
// ============================================================================

#[derive(Serialize)]
struct SessionSummary {
    id: String,
    project: Option<String>,
    started_at: String,
    message_count: i32,
}

#[derive(Serialize)]
struct SessionDetail {
    id: String,
    project: Option<String>,
    started_at: String,
    messages: Vec<serde_json::Value>,
}

async fn list_sessions(State(state): State<AppState>) -> Result<Json<Vec<SessionSummary>>> {
    let _ = state;
    // Would call into floatctl-claude
    Ok(Json(vec![]))
}

async fn get_session(
    State(state): State<AppState>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> Result<Json<SessionDetail>> {
    let _ = state;

    Ok(Json(SessionDetail {
        id: session_id,
        project: None,
        started_at: chrono::Utc::now().to_rfc3339(),
        messages: vec![],
    }))
}

// ============================================================================
// Sync status
// ============================================================================

#[derive(Serialize)]
struct SyncStatusResponse {
    daemons: Vec<DaemonStatus>,
}

#[derive(Serialize)]
struct DaemonStatus {
    name: String,
    running: bool,
    last_sync: Option<String>,
}

async fn sync_status() -> Json<SyncStatusResponse> {
    // Would call into sync module to get daemon status
    Json(SyncStatusResponse { daemons: vec![] })
}

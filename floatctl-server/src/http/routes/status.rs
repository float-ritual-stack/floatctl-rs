//! System status endpoint - curllable status for agents
//!
//! GET /status - returns current system status (focus, notice, time)
//!
//! This is the curl-friendly alternative when MCP tool descriptions
//! aren't updating as expected.

use axum::{routing::get, Json, Router};
use chrono::{DateTime, Utc};
use chrono_tz::America::Toronto;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Status entry with timestamp metadata
#[derive(Debug, Serialize, Deserialize, Default)]
pub struct StatusEntry {
    pub content: String,
    pub set_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_by: Option<String>,
}

/// System status response
#[derive(Debug, Serialize, Default)]
pub struct StatusResponse {
    pub current_time: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focus: Option<StatusEntry>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notice: Option<StatusEntry>,
}

fn get_status_dir() -> PathBuf {
    // Check FLOATCTL_STATUS_DIR env var first (for container mounts)
    // Then fall back to ~/.floatctl/status/
    if let Ok(dir) = std::env::var("FLOATCTL_STATUS_DIR") {
        return PathBuf::from(dir);
    }

    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".floatctl")
        .join("status")
}

fn read_status_entry(name: &str) -> Option<StatusEntry> {
    let status_dir = get_status_dir();

    // Try JSON first
    let json_path = status_dir.join(format!("{}.json", name));
    if json_path.exists() {
        if let Ok(content) = fs::read_to_string(&json_path) {
            if let Ok(entry) = serde_json::from_str::<StatusEntry>(&content) {
                return Some(entry);
            }
        }
    }

    // Try legacy .txt
    let txt_path = status_dir.join(format!("{}.txt", name));
    if txt_path.exists() {
        if let Ok(content) = fs::read_to_string(&txt_path) {
            let trimmed = content.trim();
            if !trimmed.is_empty() {
                return Some(StatusEntry {
                    content: trimmed.to_string(),
                    set_at: "unknown".to_string(),
                    set_by: None,
                });
            }
        }
    }

    None
}

fn format_time_ago(iso_timestamp: &str) -> String {
    if iso_timestamp == "unknown" {
        return "unknown".to_string();
    }

    match DateTime::parse_from_rfc3339(iso_timestamp) {
        Ok(set_time) => {
            let now = Utc::now();
            let diff = now.signed_duration_since(set_time.with_timezone(&Utc));
            let mins = diff.num_minutes();

            if mins < 1 {
                "just now".to_string()
            } else if mins < 60 {
                format!("{}min ago", mins)
            } else if mins < 1440 {
                format!("{}h ago", mins / 60)
            } else {
                format!("{}d ago", mins / 1440)
            }
        }
        Err(_) => iso_timestamp.to_string(),
    }
}

/// GET /status
async fn status() -> Json<StatusResponse> {
    let now = Utc::now().with_timezone(&Toronto);

    Json(StatusResponse {
        current_time: now.format("%a %b %d @ %I:%M %p").to_string(),
        focus: read_status_entry("focus"),
        notice: read_status_entry("notice"),
    })
}

/// GET /status/text - human-readable format
async fn status_text() -> String {
    let now = Utc::now().with_timezone(&Toronto);
    let focus = read_status_entry("focus");
    let notice = read_status_entry("notice");

    let mut lines = vec![
        "━━━ SYSTEM STATUS ━━━".to_string(),
        format!("🕐 {} (Toronto)", now.format("%a %b %d @ %I:%M %p")),
    ];

    if let Some(f) = focus {
        let ago = format_time_ago(&f.set_at);
        let by = f.set_by.map(|s| format!(" by {}", s)).unwrap_or_default();
        lines.push(format!("[FOCUS] {} (set {}{})", f.content, ago, by));
    }

    if let Some(n) = notice {
        let ago = format_time_ago(&n.set_at);
        let by = n.set_by.map(|s| format!(" by {}", s)).unwrap_or_default();
        lines.push(format!("[NOTICE] {} (set {}{})", n.content, ago, by));
    }

    lines.push("━━━━━━━━━━━━━━━━━━━━━".to_string());

    lines.join("\n")
}

/// Status routes
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new()
        .route("/status", get(status))
        .route("/status/text", get(status_text))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn status_returns_time() {
        let response = status().await;
        assert!(!response.current_time.is_empty());
    }
}

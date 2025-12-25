//! Jobs source - Background task status

use crate::protocol::{Item, ItemKind, JobStatus, Scope, SourceResponse};
use crate::AppState;
use anyhow::Result;
use std::sync::Arc;
use tauri::State;

/// Fetch active/recent jobs
pub async fn fetch(state: &State<'_, Arc<AppState>>) -> Result<SourceResponse> {
    let jobs = state.jobs.read().await;

    let items: Vec<Item> = jobs
        .iter()
        .map(|job| {
            let status_str = match job.status {
                JobStatus::Pending => "pending",
                JobStatus::Running => "running",
                JobStatus::Completed => "completed",
                JobStatus::Failed => "failed",
                JobStatus::Cancelled => "cancelled",
            };

            Item {
                id: job.job_id.clone(),
                kind: ItemKind::Job,
                title: job.message.clone().unwrap_or_else(|| "Job".into()),
                subtitle: Some(status_str.into()),
                actions: match job.status {
                    JobStatus::Pending | JobStatus::Running => vec!["view".into(), "cancel".into()],
                    _ => vec!["view".into()],
                },
                parent_id: None,
                has_children: false,
                badge: job.progress.map(|p| format!("{}%", p)),
                meta: [("status".into(), serde_json::json!(status_str))]
                    .into_iter()
                    .collect(),
            }
        })
        .collect();

    let total = items.len();

    Ok(SourceResponse {
        items,
        total: Some(total),
        source: "jobs".into(),
        has_more: false,
    })
}

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Board {
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThreadSummary {
    pub id: i64,
    pub board: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub markers: Vec<Marker>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ThreadDetail {
    pub id: i64,
    pub board: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub markers: Vec<Marker>,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Message {
    pub id: i64,
    pub thread_id: i64,
    pub author: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InboxMessage {
    pub id: i64,
    pub persona: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CommonItem {
    pub id: i64,
    pub key: String,
    pub value: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Marker {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateBoardRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateThreadRequest {
    pub title: String,
    pub author: Option<String>,
    pub message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMessageRequest {
    pub author: Option<String>,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct SendInboxRequest {
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct CommonItemRequest {
    pub key: Option<String>,
    pub value: serde_json::Value,
    pub ttl_seconds: Option<u64>,
}

#[derive(Debug, Deserialize)]
pub struct CliRequest {
    pub args: Option<Vec<String>>,
    pub stdin: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CliResponse {
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Deserialize)]
pub struct ThreadQuery {
    pub project: Option<String>,
}

impl CommonItemRequest {
    pub fn resolved_key(self) -> (String, serde_json::Value, Option<u64>) {
        let key = self
            .key
            .unwrap_or_else(|| format!("common-{}", Uuid::new_v4().simple()));
        (key, self.value, self.ttl_seconds)
    }
}

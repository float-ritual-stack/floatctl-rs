use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize)]
pub struct Board {
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CreateBoardRequest {
    pub name: String,
}

#[derive(Serialize)]
pub struct ThreadSummary {
    pub id: i64,
    pub board_name: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub message_count: i64,
}

#[derive(Deserialize)]
pub struct CreateThreadRequest {
    pub title: String,
    pub author: String,
    pub content: String,
}

#[derive(Serialize)]
pub struct ThreadDetail {
    pub id: i64,
    pub board_name: String,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub messages: Vec<Message>,
}

#[derive(Serialize)]
pub struct Message {
    pub id: i64,
    pub thread_id: i64,
    pub author: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub markers: Vec<Marker>,
}

#[derive(Serialize)]
pub struct Marker {
    pub kind: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct CreateMessageRequest {
    pub author: String,
    pub content: String,
}

#[derive(Deserialize)]
pub struct PersonaMessageRequest {
    pub content: String,
}

#[derive(Serialize)]
pub struct InboxMessage {
    pub id: i64,
    pub persona: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub struct CommonItemRequest {
    pub key: String,
    pub content: String,
    pub ttl_seconds: Option<i64>,
}

#[derive(Serialize)]
pub struct CommonItem {
    pub id: i64,
    pub key: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct CliCommandRequest {
    #[serde(default)]
    pub args: Vec<String>,
}

#[derive(Serialize)]
pub struct CliCommandResponse {
    pub command: String,
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
    pub request_id: Uuid,
}

#[derive(Deserialize, Default)]
pub struct ThreadQuery {
    pub project: Option<String>,
    pub ctx: Option<String>,
    pub board: Option<String>,
}

use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BoardCreate {
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct Board {
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct BoardWithThreads {
    pub board: Board,
    pub threads: Vec<Thread>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ThreadCreate {
    pub title: String,
    pub content: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct Thread {
    pub id: i64,
    pub board_name: String,
    pub title: String,
    pub author: Option<String>,
    pub created_at: DateTime<Utc>,
    pub markers: Vec<Marker>,
}

#[derive(Debug, Serialize)]
pub struct ThreadDetail {
    pub thread: Thread,
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageCreate {
    pub author: Option<String>,
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct Message {
    pub id: i64,
    pub thread_id: i64,
    pub author: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub markers: Vec<Marker>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Marker {
    pub kind: String,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InboxMessageCreate {
    pub content: String,
    pub author: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct InboxMessage {
    pub id: i64,
    pub persona: String,
    pub author: Option<String>,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CommonItemCreate {
    pub key: Option<String>,
    pub content: String,
    pub ttl_seconds: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct CommonItem {
    pub key: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CliCommandRequest {
    pub args: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct CliCommandResponse {
    pub id: Uuid,
    pub command: String,
    pub status: i32,
    pub stdout: String,
    pub stderr: String,
}

pub fn ts_to_datetime(ts: i64) -> DateTime<Utc> {
    let naive = NaiveDateTime::from_timestamp_opt(ts, 0)
        .unwrap_or_else(|| NaiveDateTime::from_timestamp_opt(0, 0).expect("epoch must exist"));
    DateTime::<Utc>::from_utc(naive, Utc)
}

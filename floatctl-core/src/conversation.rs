use std::borrow::Cow;

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::markers::{extract_markers, MarkerSet};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    Assistant,
    User,
    Tool,
    Other,
}

impl MessageRole {
    pub fn from_export_value(value: &Value) -> MessageRole {
        value
            .as_str()
            .map(|s| match s {
                "system" => MessageRole::System,
                "assistant" => MessageRole::Assistant,
                "user" | "human" => MessageRole::User,
                "tool" | "function" => MessageRole::Tool,
                _ => MessageRole::Other,
            })
            .unwrap_or(MessageRole::Other)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: Uuid,
    pub idx: i32,
    pub role: MessageRole,
    pub timestamp: DateTime<Utc>,
    pub content: String,
    pub project: Option<String>,
    pub meeting: Option<String>,
    pub markers: MarkerSet,
    #[serde(skip)]
    pub raw: Value,
}

impl Message {
    pub fn from_export(idx: i32, value: Value) -> Result<Self> {
        let role = MessageRole::from_export_value(value.get("role").unwrap_or(&Value::Null));
        let timestamp = value
            .get("timestamp")
            .or_else(|| value.get("create_time"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("message missing timestamp"))?;
        let timestamp = DateTime::parse_from_rfc3339(timestamp)
            .or_else(|_| DateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S%.f %z"))
            .map(|dt| dt.with_timezone(&Utc))
            .context("failed to parse message timestamp")?;

        let text = extract_message_text(&value)?;
        let markers = extract_markers(&text);

        Ok(Self {
            id: infer_message_id(&value),
            idx,
            role,
            timestamp,
            content: text,
            project: extract_tag(&value, "project"),
            meeting: extract_tag(&value, "meeting"),
            markers,
            raw: value,
        })
    }
}

fn extract_message_text(value: &Value) -> Result<String> {
    if let Some(text) = value.get("content").and_then(|c| c.as_str()) {
        return Ok(text.to_owned());
    }

    if let Some(array) = value.get("content").and_then(|c| c.as_array()) {
        let mut joined = String::new();
        for item in array {
            if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                if !joined.is_empty() {
                    joined.push_str("\n\n");
                }
                joined.push_str(text);
            }
        }
        if !joined.is_empty() {
            return Ok(joined);
        }
    }

    if let Some(summary) = value.get("summary").and_then(|s| s.as_str()) {
        return Ok(summary.to_owned());
    }

    Ok(String::new())
}

fn extract_tag(value: &Value, key: &str) -> Option<String> {
    value
        .get("metadata")
        .and_then(|meta| meta.get(key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_owned())
}

fn infer_message_id(value: &Value) -> Uuid {
    if let Some(id) = value.get("id").and_then(|v| v.as_str()) {
        if let Ok(uuid) = Uuid::parse_str(id) {
            return uuid;
        }
    }
    Uuid::new_v4()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMeta {
    pub id: Uuid,
    pub conv_id: String,
    pub title: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: Option<DateTime<Utc>>,
    pub markers: MarkerSet,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub meta: ConversationMeta,
    pub messages: Vec<Message>,
    #[serde(skip)]
    pub raw: Value,
}

impl Conversation {
    pub fn from_export(mut value: Value) -> Result<Self> {
        // Extract messages array first to avoid borrow conflicts
        let msgs = value
            .get_mut("messages")
            .and_then(|m| m.as_array_mut())
            .map(|arr| std::mem::take(arr)) // moves Vec<Value> out
            .unwrap_or_default();

        let conv_id = value
            .get("id")
            .or_else(|| value.get("uuid"))
            .and_then(|v| v.as_str())
            .map(Cow::from)
            .unwrap_or_else(|| Cow::from(Uuid::new_v4().to_string()));

        let created_at = value
            .get("created_at")
            .or_else(|| value.get("create_time"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("conversation missing created_at"))?;
        let created_at = DateTime::parse_from_rfc3339(created_at)
            .or_else(|_| DateTime::parse_from_str(created_at, "%Y-%m-%d %H:%M:%S%.f %z"))
            .map(|dt| dt.with_timezone(&Utc))
            .context("failed to parse conversation timestamp")?;

        let mut markers = MarkerSet::default();
        let mut messages = Vec::new();
        // Process the extracted messages without cloning
        for (idx, raw_message) in msgs.into_iter().enumerate() {
            let idx = idx as i32;
            let message = Message::from_export(idx, raw_message)?;
            markers.extend(&message.markers);
            messages.push(message);
        }

        let meta = ConversationMeta {
            id: Uuid::new_v4(),
            conv_id: conv_id.into_owned(),
            title: value
                .get("title")
                .or_else(|| value.get("name"))
                .and_then(|v| v.as_str())
                .map(str::to_owned),
            created_at,
            updated_at: messages.last().map(|m| m.timestamp),
            markers,
        };

        Ok(Self {
            meta,
            messages,
            raw: value,
        })
    }
}

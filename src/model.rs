use std::collections::{BTreeSet, HashMap};

use anyhow::{Context, Result, anyhow, bail};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Anthropic,
    ChatGpt,
}

impl Source {
    pub fn as_str(&self) -> &'static str {
        match self {
            Source::Anthropic => "anthropic",
            Source::ChatGpt => "chatgpt",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Conversation {
    pub source: Source,
    pub conv_id: String,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub model: Option<String>,
    pub created: DateTime<Utc>,
    pub updated: Option<DateTime<Utc>>,
    pub participants: BTreeSet<String>,
    pub messages: Vec<Message>,
    pub raw: Value,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub message_id: Option<String>,
    pub role: Role,
    pub timestamp: Option<DateTime<Utc>>,
    pub channels: Vec<ChannelContent>,
    pub attachments: Vec<Attachment>,
    pub tool_calls: Vec<ToolCall>,
    pub artifacts: Vec<Artifact>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    Human,
    Assistant,
    System,
    Tool,
    Other,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Human => "human",
            Role::Assistant => "assistant",
            Role::System => "system",
            Role::Tool => "tool",
            Role::Other => "other",
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChannelContent {
    pub channel: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub name: String,
    #[serde(default)]
    pub args: Value,
    #[serde(default)]
    pub result: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attachment {
    pub name: Option<String>,
    pub uri: Option<String>,
    pub mime: Option<String>,
    pub sha256: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub kind: Option<String>,
    pub lang: Option<String>,
    pub code: Option<String>,
}

pub fn detect_source(value: &Value) -> Result<Source> {
    match value {
        Value::Array(items) => {
            if let Some(Value::Object(obj)) = items.first() {
                if obj.contains_key("chat_messages") {
                    Ok(Source::Anthropic)
                } else if obj.contains_key("mapping") || obj.contains_key("conversation") {
                    Ok(Source::ChatGpt)
                } else {
                    bail!("unable to detect source type from array payload");
                }
            } else {
                bail!("expected array of objects in conversations export");
            }
        }
        Value::Object(map) => {
            if map.contains_key("mapping") {
                Ok(Source::ChatGpt)
            } else if map.contains_key("chat_messages") {
                Ok(Source::Anthropic)
            } else if let Some(Value::Array(items)) = map.get("conversations") {
                detect_source(&Value::Array(items.clone()))
            } else {
                bail!("unable to detect source type from object payload");
            }
        }
        _ => bail!("unsupported JSON shape for conversations export"),
    }
}

pub fn extract_conversations(value: Value, source: Source) -> Result<Vec<Value>> {
    match source {
        Source::Anthropic => match value {
            Value::Array(items) => Ok(items),
            other => bail!("expected array for anthropic conversations, got {other:?}"),
        },
        Source::ChatGpt => {
            if let Value::Array(items) = value {
                Ok(items)
            } else if let Value::Object(mut map) = value {
                if let Some(Value::Array(items)) = map.remove("conversations") {
                    Ok(items)
                } else if let Some(Value::Array(items)) = map.remove("items") {
                    Ok(items)
                } else {
                    bail!("chatgpt export missing 'conversations' or 'items' array");
                }
            } else {
                bail!("expected array/object for chatgpt conversations");
            }
        }
    }
}

pub fn parse_conversation(source: Source, raw: Value) -> Result<Conversation> {
    match source {
        Source::Anthropic => parse_anthropic_conversation(raw),
        Source::ChatGpt => parse_chatgpt_conversation(raw),
    }
}

fn parse_anthropic_conversation(raw: Value) -> Result<Conversation> {
    #[derive(Debug, Deserialize)]
    struct AnthropicConversation {
        uuid: String,
        #[serde(default)]
        name: String,
        #[serde(default)]
        summary: String,
        #[serde(default)]
        created_at: String,
        #[serde(default)]
        updated_at: Option<String>,
        #[serde(default)]
        model: Option<String>,
        #[serde(default)]
        chat_messages: Vec<AnthropicMessage>,
    }

    #[derive(Debug, Deserialize)]
    struct AnthropicMessage {
        #[serde(default)]
        uuid: Option<String>,
        #[serde(default)]
        text: String,
        #[serde(default)]
        content: Vec<AnthropicContent>,
        #[serde(default)]
        sender: Option<String>,
        #[serde(default)]
        created_at: Option<String>,
        #[serde(default)]
        attachments: Vec<Value>,
    }

    #[derive(Debug, Deserialize)]
    struct AnthropicContent {
        #[serde(rename = "type")]
        #[serde(default)]
        kind: String,
        #[serde(default)]
        text: String,
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        input: Option<Value>,
        #[serde(default)]
        content: Option<Value>,
        #[serde(default)]
        display_content: Option<Value>,
    }

    let convo: AnthropicConversation =
        serde_json::from_value(raw.clone()).context("failed to decode anthropic conversation")?;
    let created =
        parse_datetime(&convo.created_at).context("anthropic conversation missing created_at")?;
    let updated = match convo.updated_at {
        Some(ref ts) if !ts.is_empty() => Some(parse_datetime(ts)?),
        _ => None,
    };

    let mut participants = BTreeSet::new();
    let mut messages = Vec::with_capacity(convo.chat_messages.len());
    for (idx, msg) in convo.chat_messages.into_iter().enumerate() {
        let sender = msg.sender.unwrap_or_else(|| "unknown".to_string());
        let role = match sender.as_str() {
            "human" => Role::Human,
            "assistant" => Role::Assistant,
            "system" => Role::System,
            "tool" => Role::Tool,
            _ => Role::Other,
        };
        participants.insert(role.as_str().to_string());

        let mut channels = Vec::new();
        let mut artifacts = Vec::new();
        let primary_channel = match role {
            Role::Human => "message",
            Role::Assistant => "reply",
            Role::System => "system",
            Role::Tool => "tool",
            Role::Other => "message",
        };
        if !msg.text.trim().is_empty() {
            push_channel(&mut channels, primary_channel, &msg.text);
        }

        for block in msg.content {
            if block.kind == "text" && !block.text.trim().is_empty() {
                push_channel(&mut channels, primary_channel, &block.text);
            } else if block.kind == "tool_use" && block.name.as_deref() == Some("artifacts") {
                if let Some(ref value) = block.input {
                    collect_artifacts_from_value(value, &mut artifacts);
                }
            } else if block.kind == "tool_result" && block.name.as_deref() == Some("artifacts") {
                if let Some(ref value) = block.content {
                    collect_artifacts_from_value(value, &mut artifacts);
                }
                if let Some(ref value) = block.display_content {
                    collect_artifacts_from_value(value, &mut artifacts);
                }
            }
        }

        let timestamp = match msg.created_at {
            Some(ref ts) if !ts.is_empty() => Some(parse_datetime(ts)?),
            _ => None,
        };

        let attachments = msg
            .attachments
            .into_iter()
            .map(|value| {
                let name = value
                    .get("file_name")
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());
                let mime = value
                    .get("mime_type")
                    .or_else(|| value.get("mime"))
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());
                let uri = value
                    .get("uri")
                    .or_else(|| value.get("url"))
                    .and_then(Value::as_str)
                    .map(|s| s.to_string());

                Attachment {
                    name,
                    uri,
                    mime,
                    sha256: None,
                    width: None,
                    height: None,
                }
            })
            .collect();

        messages.push(Message {
            message_id: msg.uuid.or_else(|| Some(format!("anthropic-{idx}"))),
            role,
            timestamp,
            channels,
            attachments,
            tool_calls: Vec::new(),
            artifacts,
        });
    }

    Ok(Conversation {
        source: Source::Anthropic,
        conv_id: convo.uuid,
        title: if convo.name.trim().is_empty() {
            None
        } else {
            Some(convo.name)
        },
        summary: if convo.summary.trim().is_empty() {
            None
        } else {
            Some(convo.summary)
        },
        model: convo.model,
        created,
        updated,
        participants,
        messages,
        raw,
    })
}

fn parse_chatgpt_conversation(raw: Value) -> Result<Conversation> {
    #[derive(Debug, Deserialize)]
    struct ChatGptConversation {
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        title: Option<String>,
        #[serde(default)]
        create_time: Option<f64>,
        #[serde(default)]
        update_time: Option<f64>,
        #[serde(default)]
        mapping: Map<String, Value>,
        #[serde(default)]
        conversation_id: Option<String>,
    }

    let convo: ChatGptConversation =
        serde_json::from_value(raw.clone()).context("failed to decode chatgpt conversation")?;

    let conv_id = convo
        .conversation_id
        .or(convo.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    let created = convo
        .create_time
        .map(from_unix_seconds)
        .transpose()?
        .ok_or_else(|| anyhow!("chatgpt conversation missing create_time"))?;

    let updated = convo.update_time.map(from_unix_seconds).transpose()?;

    let mut participants = BTreeSet::new();
    let mut messages = Vec::new();

    for (node_id, node) in convo.mapping.iter() {
        if !node.is_object() {
            continue;
        }
        let node_obj = node.as_object().unwrap();
        let message = node_obj.get("message").and_then(Value::as_object);
        let Some(message) = message else { continue };

        let author = message
            .get("author")
            .and_then(Value::as_object)
            .and_then(|author| author.get("role"))
            .and_then(Value::as_str)
            .unwrap_or("unknown");

        let role = match author {
            "user" => Role::Human,
            "assistant" => Role::Assistant,
            "system" => Role::System,
            "tool" => Role::Tool,
            other if other == "developer" => Role::System,
            _ => Role::Other,
        };
        participants.insert(role.as_str().to_string());

        let content = message
            .get("content")
            .and_then(Value::as_object)
            .context("chatgpt message missing content")?;

        let content_type = content
            .get("content_type")
            .and_then(Value::as_str)
            .unwrap_or("text");

        let mut channels = Vec::new();

        match content_type {
            "text" | "code" => {
                if let Some(Value::Array(parts)) = content.get("parts") {
                    for part in parts {
                        if let Some(text) = part.as_str() {
                            let channel = match role {
                                Role::Human => "message",
                                Role::Assistant => "reply",
                                Role::System => "system",
                                Role::Tool => "tool",
                                Role::Other => "message",
                            };
                            push_channel(&mut channels, channel, text);
                        } else if let Some(obj) = part.as_object() {
                            if let Some(text) = obj.get("text").and_then(Value::as_str) {
                                push_channel(&mut channels, "reply", text);
                            }
                        }
                    }
                }
            }
            "multimodal_text" => {
                if let Some(Value::Array(parts)) = content.get("parts") {
                    for part in parts {
                        if let Some(obj) = part.as_object() {
                            if let Some(text) = obj.get("text").and_then(Value::as_str) {
                                push_channel(&mut channels, "reply", text);
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        let timestamp = message
            .get("create_time")
            .and_then(Value::as_f64)
            .map(from_unix_seconds)
            .transpose()
            .context("invalid chatgpt message timestamp")?;

        let attachments = Vec::new();
        let tool_calls = Vec::new();
        let artifacts = Vec::new();

        messages.push(Message {
            message_id: Some(node_id.clone()),
            role,
            timestamp,
            channels,
            attachments,
            tool_calls,
            artifacts,
        });
    }

    messages.sort_by_key(|m| m.timestamp.unwrap_or(created));

    Ok(Conversation {
        source: Source::ChatGpt,
        conv_id,
        title: convo.title,
        summary: None,
        model: None,
        created,
        updated,
        participants,
        messages,
        raw,
    })
}

fn parse_datetime(value: &str) -> Result<DateTime<Utc>> {
    let parsed = DateTime::parse_from_rfc3339(value)
        .or_else(|_| DateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S%.f%:z"))
        .or_else(|_| DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%.f%:z"))
        .map_err(|_| anyhow!("invalid datetime '{value}'"))?;
    Ok(parsed.with_timezone(&Utc))
}

fn from_unix_seconds(seconds: f64) -> Result<DateTime<Utc>> {
    let millis = (seconds * 1000.0).round() as i64;
    DateTime::<Utc>::from_timestamp_millis(millis)
        .ok_or_else(|| anyhow!("invalid unix timestamp {seconds}"))
}

fn push_channel(channels: &mut Vec<ChannelContent>, channel: &str, text: &str) {
    let normalized = text.trim_end();
    if normalized.trim().is_empty() {
        return;
    }
    if channels
        .iter()
        .any(|existing| existing.channel == channel && existing.text.trim() == normalized.trim())
    {
        return;
    }
    channels.push(ChannelContent {
        channel: channel.to_string(),
        text: normalized.to_string(),
    });
}

fn collect_artifacts_from_value(value: &Value, artifacts: &mut Vec<Artifact>) {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
        Value::String(content) => {
            push_artifact(None, None, content, artifacts);
        }
        Value::Array(items) => {
            for item in items {
                collect_artifacts_from_value(item, artifacts);
            }
        }
        Value::Object(map) => {
            if let Some(embedded) = map.get("artifacts") {
                collect_artifacts_from_value(embedded, artifacts);
            }

            let mut handled_table = false;
            if let Some(table) = map.get("table") {
                if let Some(artifact) = table_to_artifact(table) {
                    handled_table = true;
                    artifacts.push(artifact);
                }
            }

            if let Some(content_value) = map.get("content") {
                match content_value {
                    Value::String(text) => {
                        if !handled_table {
                            let title = map.get("title").and_then(Value::as_str);
                            let lang = map.get("type").and_then(Value::as_str);
                            push_artifact(title, lang, text, artifacts);
                        }
                    }
                    other => collect_artifacts_from_value(other, artifacts),
                }
            }
        }
    }
}

fn push_artifact(
    title: Option<&str>,
    lang: Option<&str>,
    content: &str,
    artifacts: &mut Vec<Artifact>,
) {
    if content.trim().is_empty() {
        return;
    }
    artifacts.push(Artifact {
        kind: title.map(|s| s.to_string()),
        lang: lang.map(|s| s.to_string()),
        code: Some(content.to_string()),
    });
}

fn table_to_artifact(table: &Value) -> Option<Artifact> {
    let Value::Array(rows) = table else {
        return None;
    };
    let mut entries = HashMap::new();
    for row in rows {
        if let Value::Array(pair) = row {
            if pair.len() == 2 {
                if let (Some(Value::String(key)), Some(Value::String(value))) =
                    (pair.get(0), pair.get(1))
                {
                    entries.insert(key.clone(), value.clone());
                }
            }
        }
    }

    let content = entries.get("content")?.clone();
    if content.trim().is_empty() {
        return None;
    }
    let title = entries.get("title").cloned();
    let lang = entries.get("type").or_else(|| entries.get("mime")).cloned();

    Some(Artifact {
        kind: title,
        lang,
        code: Some(content),
    })
}

pub fn canonicalize_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));
            let mut new_map = Map::new();
            for (k, v) in entries {
                new_map.insert(k.clone(), canonicalize_value(v));
            }
            Value::Object(new_map)
        }
        Value::Array(items) => Value::Array(items.iter().map(canonicalize_value).collect()),
        other => other.clone(),
    }
}

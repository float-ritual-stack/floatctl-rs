use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Write};
use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::de::IoRead;
use serde_json::Deserializer;
use tokio::fs::File as AsyncFile;
use tokio::io::{AsyncBufReadExt, BufReader as AsyncBufReader};

use crate::conversation::{Conversation, Message};

pub struct ConversationReader<R: Read> {
    inner: Deserializer<IoRead<R>>,
}

impl ConversationReader<BufReader<File>> {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::open(path)?;
        Ok(Self::new(BufReader::new(file)))
    }
}

impl<R: Read> ConversationReader<R> {
    pub fn new(reader: R) -> Self {
        let inner = Deserializer::from_reader(reader);
        Self { inner }
    }

    pub fn into_iter(self) -> impl Iterator<Item = Result<Conversation>> {
        self.inner.into_iter::<serde_json::Value>().map(|item| {
            item.map_err(anyhow::Error::from)
                .and_then(Conversation::from_export)
        })
    }
}

pub struct AsyncConversationReader {
    lines: tokio::io::Lines<AsyncBufReader<AsyncFile>>,
}

impl AsyncConversationReader {
    pub async fn open(path: impl AsRef<Path>) -> Result<Self> {
        let file = AsyncFile::open(path).await?;
        let reader = AsyncBufReader::new(file);
        Ok(Self {
            lines: reader.lines(),
        })
    }

    pub async fn next(&mut self) -> Option<Result<Conversation>> {
        loop {
            match self.lines.next_line().await {
                Ok(Some(line)) => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    let value: serde_json::Value = match serde_json::from_str(&line) {
                        Ok(value) => value,
                        Err(err) => return Some(Err(err.into())),
                    };
                    return Some(Conversation::from_export(value));
                }
                Ok(None) => return None,
                Err(err) => return Some(Err(err.into())),
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum MessageRecord {
    Meta {
        conv_id: String,
        title: Option<String>,
        created_at: String,
        markers: Vec<String>,
    },
    Message {
        conv_id: String,
        idx: i32,
        message_id: String,
        role: String,
        timestamp: String,
        content: String,
        project: Option<String>,
        meeting: Option<String>,
        markers: Vec<String>,
    },
}

impl MessageRecord {
    pub fn from_conversation(conv: &Conversation) -> Vec<MessageRecord> {
        let meta = MessageRecord::Meta {
            conv_id: conv.meta.conv_id.clone(),
            title: conv.meta.title.clone(),
            created_at: conv.meta.created_at.to_rfc3339(),
            markers: conv.meta.markers.iter().cloned().collect(),
        };
        let mut records = vec![meta];
        for message in &conv.messages {
            records.push(MessageRecord::from_message(&conv.meta.conv_id, message));
        }
        records
    }

    pub fn from_message(conv_id: &str, msg: &Message) -> MessageRecord {
        MessageRecord::Message {
            conv_id: conv_id.to_owned(),
            idx: msg.idx,
            message_id: msg.id.to_string(),
            role: format!("{:?}", msg.role).to_lowercase(),
            timestamp: msg.timestamp.to_rfc3339(),
            content: msg.content.clone(),
            project: msg.project.clone(),
            meeting: msg.meeting.clone(),
            markers: msg.markers.iter().cloned().collect(),
        }
    }
}

pub struct NdjsonWriter<W: Write> {
    writer: BufWriter<W>,
}

impl NdjsonWriter<File> {
    pub fn create(path: impl AsRef<Path>) -> Result<Self> {
        let file = File::create(path)?;
        Ok(Self::new(file))
    }
}

impl<W: Write> NdjsonWriter<W> {
    pub fn new(inner: W) -> Self {
        Self {
            writer: BufWriter::new(inner),
        }
    }

    pub fn write_record(&mut self, record: &MessageRecord) -> Result<()> {
        let line = serde_json::to_string(record)?;
        self.writer.write_all(line.as_bytes())?;
        self.writer.write_all(b"\n")?;
        self.writer.flush()?;
        Ok(())
    }
}

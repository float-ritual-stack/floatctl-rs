//! Inbox file operations
//!
//! Per-persona messaging with:
//! - Message files (YAML frontmatter + markdown body)
//! - Read status tracking via `.read/` marker files

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

use super::config::BbsConfig;
use super::frontmatter::{
    generate_message_id, generate_preview, parse_frontmatter, write_with_frontmatter,
};

/// Message frontmatter (YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageFrontmatter {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub date: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Inbox message (full representation)
#[derive(Debug, Clone, Serialize)]
pub struct InboxMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub date: DateTime<Utc>,
    pub subject: String,
    pub preview: String,
    pub content: String,
    pub read: bool,
    pub path: String,
}

/// Check if a message has been read
pub async fn is_read(config: &BbsConfig, persona: &str, message_id: &str) -> bool {
    let marker_path = config.read_markers_path(persona).join(message_id);
    fs::try_exists(&marker_path).await.unwrap_or(false)
}

/// Mark message as read
pub async fn mark_as_read(config: &BbsConfig, persona: &str, message_id: &str) -> std::io::Result<()> {
    let markers_dir = config.read_markers_path(persona);
    fs::create_dir_all(&markers_dir).await?;

    let marker_path = markers_dir.join(message_id);
    let timestamp = Utc::now().to_rfc3339();
    fs::write(marker_path, timestamp).await?;

    Ok(())
}

/// Mark message as unread
pub async fn mark_as_unread(config: &BbsConfig, persona: &str, message_id: &str) -> std::io::Result<()> {
    let marker_path = config.read_markers_path(persona).join(message_id);
    if fs::try_exists(&marker_path).await.unwrap_or(false) {
        fs::remove_file(marker_path).await?;
    }
    Ok(())
}

/// Parse a message file
async fn parse_message(
    path: &Path,
    persona: &str,
    config: &BbsConfig,
) -> Result<InboxMessage, Box<dyn std::error::Error + Send + Sync>> {
    let content = fs::read_to_string(path).await?;
    let (fm, body): (MessageFrontmatter, String) = parse_frontmatter(&content)?;

    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    let read = is_read(config, persona, filename).await;

    Ok(InboxMessage {
        id: filename.to_string(),
        from: fm.from,
        to: fm.to,
        date: fm.date,
        subject: fm.subject,
        preview: generate_preview(&body, 200),
        content: body,
        read,
        path: path.display().to_string(),
    })
}

/// List inbox messages for a persona
pub async fn list_inbox(
    config: &BbsConfig,
    persona: &str,
    limit: usize,
    unread_only: bool,
    from_filter: Option<&str>,
) -> std::io::Result<(Vec<InboxMessage>, usize)> {
    let inbox_path = config.inbox_path(persona);

    // Create if doesn't exist
    fs::create_dir_all(&inbox_path).await?;

    let mut entries = fs::read_dir(&inbox_path).await?;
    let mut messages = Vec::new();
    let mut total_unread = 0;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Skip non-.md files and hidden files/dirs
        if !path.extension().map(|e| e == "md").unwrap_or(false) {
            continue;
        }
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.starts_with('.'))
            .unwrap_or(true)
        {
            continue;
        }

        let msg = match parse_message(&path, persona, config).await {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Failed to parse {}: {}", path.display(), e);
                continue;
            }
        };

        if !msg.read {
            total_unread += 1;
        }

        // Apply filters
        if unread_only && msg.read {
            continue;
        }
        if let Some(from) = from_filter {
            if msg.from != from {
                continue;
            }
        }

        messages.push(msg);
    }

    // Sort by date, most recent first
    messages.sort_by(|a, b| b.date.cmp(&a.date));

    // Apply limit
    messages.truncate(limit);

    Ok((messages, total_unread))
}

/// Get a single message by ID
pub async fn get_message(
    config: &BbsConfig,
    persona: &str,
    message_id: &str,
) -> Result<InboxMessage, Box<dyn std::error::Error + Send + Sync>> {
    let inbox_path = config.inbox_path(persona);
    let message_path = inbox_path.join(format!("{}.md", message_id));

    if !message_path.exists() {
        return Err(format!("Message '{}' not found", message_id).into());
    }

    parse_message(&message_path, persona, config).await
}

/// Send message to recipient's inbox
pub async fn send_message(
    config: &BbsConfig,
    from: &str,
    to: &str,
    subject: &str,
    content: &str,
    tags: Vec<String>,
) -> std::io::Result<(String, String)> {
    let recipient_inbox = config.inbox_path(to);
    fs::create_dir_all(&recipient_inbox).await?;

    let message_id = generate_message_id(from);
    let message_path = recipient_inbox.join(format!("{}.md", message_id));

    let frontmatter = MessageFrontmatter {
        from: from.to_string(),
        to: to.to_string(),
        subject: subject.to_string(),
        date: Utc::now(),
        tags,
    };

    let file_content = write_with_frontmatter(&frontmatter, content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    fs::write(&message_path, file_content).await?;

    Ok((message_id, message_path.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(temp_dir: &TempDir) -> BbsConfig {
        BbsConfig::with_root(temp_dir.path().to_path_buf())
    }

    #[tokio::test]
    async fn test_send_and_receive_message() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        // Send message
        let (msg_id, path) = send_message(
            &config,
            "kitty",
            "cowboy",
            "Test Subject",
            "Test body content",
            vec!["test".to_string()],
        )
        .await
        .unwrap();

        assert!(Path::new(&path).exists());
        assert!(msg_id.contains("-from-kitty"));

        // List inbox
        let (messages, unread) = list_inbox(&config, "cowboy", 10, false, None)
            .await
            .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(unread, 1);
        assert_eq!(messages[0].subject, "Test Subject");
        assert_eq!(messages[0].from, "kitty");
        assert!(!messages[0].read);
    }

    #[tokio::test]
    async fn test_mark_read_unread() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        // Send message
        let (msg_id, _) = send_message(&config, "kitty", "cowboy", "Test", "Body", vec![])
            .await
            .unwrap();

        // Should be unread initially
        assert!(!is_read(&config, "cowboy", &msg_id).await);

        // Mark as read
        mark_as_read(&config, "cowboy", &msg_id).await.unwrap();
        assert!(is_read(&config, "cowboy", &msg_id).await);

        // Mark as unread
        mark_as_unread(&config, "cowboy", &msg_id).await.unwrap();
        assert!(!is_read(&config, "cowboy", &msg_id).await);
    }

    #[tokio::test]
    async fn test_filter_unread_only() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        // Send two messages
        let (msg1, _) = send_message(&config, "kitty", "cowboy", "Msg 1", "Body 1", vec![])
            .await
            .unwrap();
        let (_msg2, _) = send_message(&config, "kitty", "cowboy", "Msg 2", "Body 2", vec![])
            .await
            .unwrap();

        // Mark first as read
        mark_as_read(&config, "cowboy", &msg1).await.unwrap();

        // Filter unread only
        let (messages, _) = list_inbox(&config, "cowboy", 10, true, None)
            .await
            .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].subject, "Msg 2");
    }

    #[tokio::test]
    async fn test_filter_by_sender() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        // Send from different senders
        send_message(&config, "kitty", "cowboy", "From Kitty", "Body", vec![])
            .await
            .unwrap();
        send_message(&config, "daddy", "cowboy", "From Daddy", "Body", vec![])
            .await
            .unwrap();

        // Filter by sender
        let (messages, _) = list_inbox(&config, "cowboy", 10, false, Some("kitty"))
            .await
            .unwrap();

        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].from, "kitty");
    }
}

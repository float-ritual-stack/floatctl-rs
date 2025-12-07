//! Board file operations
//!
//! Shared posting spaces (replaces "common" with explicit board names).
//! Each board is a directory containing posts with YAML frontmatter.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

use super::config::BbsConfig;
use super::frontmatter::{
    generate_content_id, generate_preview, parse_frontmatter, write_with_frontmatter,
};

/// Board post frontmatter (YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardFrontmatter {
    pub title: String,
    pub date: DateTime<Utc>,
    pub author: String,
    #[serde(default = "default_imprint")]
    pub imprint: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

fn default_imprint() -> String {
    "field-notes".to_string()
}

/// Board post (full representation)
#[derive(Debug, Clone, Serialize)]
pub struct BoardPost {
    pub id: String,
    pub title: String,
    pub author: String,
    pub date: DateTime<Utc>,
    pub imprint: String,
    pub tags: Vec<String>,
    pub preview: String,
    pub content: String,
    pub path: String,
}

/// Parse a board post file
async fn parse_post(
    path: &Path,
) -> Result<BoardPost, Box<dyn std::error::Error + Send + Sync>> {
    let content = fs::read_to_string(path).await?;
    let (fm, body): (BoardFrontmatter, String) = parse_frontmatter(&content)?;

    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    Ok(BoardPost {
        id: filename.to_string(),
        title: fm.title,
        author: fm.author,
        date: fm.date,
        imprint: fm.imprint,
        tags: fm.tags,
        preview: generate_preview(&body, 200),
        content: body,
        path: path.display().to_string(),
    })
}

/// List posts from a board
pub async fn list_board(
    config: &BbsConfig,
    board_name: &str,
    limit: usize,
    by_author: Option<&str>,
    by_tag: Option<&str>,
    include_content: bool,
) -> std::io::Result<Vec<BoardPost>> {
    let board_path = config.board_path(board_name);

    // Create if doesn't exist
    fs::create_dir_all(&board_path).await?;

    let mut entries = fs::read_dir(&board_path).await?;
    let mut posts = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Skip non-.md files and hidden files
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

        let mut post = match parse_post(&path).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Failed to parse post {}: {}", path.display(), e);
                continue;
            }
        };

        // Apply filters
        if let Some(author) = by_author {
            if post.author != author {
                continue;
            }
        }
        if let Some(tag) = by_tag {
            if !post.tags.iter().any(|t| t == tag) {
                continue;
            }
        }

        // Clear content if not requested (for lighter responses)
        if !include_content {
            post.content = String::new();
        }

        posts.push(post);
    }

    // Sort by date, most recent first
    posts.sort_by(|a, b| b.date.cmp(&a.date));

    // Apply limit
    posts.truncate(limit);

    Ok(posts)
}

/// Post to a board
pub async fn post_to_board(
    config: &BbsConfig,
    board_name: &str,
    author: &str,
    title: &str,
    content: &str,
    imprint: Option<&str>,
    tags: Vec<String>,
) -> std::io::Result<(String, String)> {
    let board_path = config.board_path(board_name);
    fs::create_dir_all(&board_path).await?;

    let post_id = generate_content_id(title);
    let post_path = board_path.join(format!("{}.md", post_id));

    let frontmatter = BoardFrontmatter {
        title: title.to_string(),
        date: Utc::now(),
        author: author.to_string(),
        imprint: imprint.unwrap_or("field-notes").to_string(),
        tags,
    };

    let file_content = write_with_frontmatter(&frontmatter, content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    fs::write(&post_path, file_content).await?;

    Ok((post_id, post_path.display().to_string()))
}

/// List available boards
pub async fn list_boards(config: &BbsConfig) -> std::io::Result<Vec<String>> {
    let boards_root = config.boards_root();

    // Create if doesn't exist
    fs::create_dir_all(&boards_root).await?;

    let mut entries = fs::read_dir(&boards_root).await?;
    let mut boards = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();

        // Only include directories
        if !path.is_dir() {
            continue;
        }

        // Skip hidden directories
        if path
            .file_name()
            .and_then(|n| n.to_str())
            .map(|s| s.starts_with('.'))
            .unwrap_or(true)
        {
            continue;
        }

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            boards.push(name.to_string());
        }
    }

    boards.sort();
    Ok(boards)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(temp_dir: &TempDir) -> BbsConfig {
        BbsConfig::with_root(temp_dir.path().to_path_buf())
    }

    #[tokio::test]
    async fn test_post_and_list() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        // Post to board
        let (post_id, path) = post_to_board(
            &config,
            "sysops-log",
            "kitty",
            "Test Post",
            "This is the content",
            Some("field-notes"),
            vec!["test".to_string()],
        )
        .await
        .unwrap();

        assert!(Path::new(&path).exists());
        assert!(post_id.contains("test-post"));

        // List board
        let posts = list_board(&config, "sysops-log", 10, None, None, true)
            .await
            .unwrap();

        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].title, "Test Post");
        assert_eq!(posts[0].author, "kitty");
        assert!(!posts[0].content.is_empty());
    }

    #[tokio::test]
    async fn test_filter_by_author() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        post_to_board(&config, "test-board", "kitty", "Kitty Post", "Body", None, vec![])
            .await
            .unwrap();
        post_to_board(&config, "test-board", "cowboy", "Cowboy Post", "Body", None, vec![])
            .await
            .unwrap();

        let posts = list_board(&config, "test-board", 10, Some("kitty"), None, false)
            .await
            .unwrap();

        assert_eq!(posts.len(), 1);
        assert_eq!(posts[0].author, "kitty");
    }

    #[tokio::test]
    async fn test_filter_by_tag() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        post_to_board(
            &config,
            "test-board",
            "kitty",
            "Tagged Post",
            "Body",
            None,
            vec!["important".to_string()],
        )
        .await
        .unwrap();
        post_to_board(&config, "test-board", "kitty", "Untagged Post", "Body", None, vec![])
            .await
            .unwrap();

        let posts = list_board(&config, "test-board", 10, None, Some("important"), false)
            .await
            .unwrap();

        assert_eq!(posts.len(), 1);
        assert!(posts[0].tags.contains(&"important".to_string()));
    }

    #[tokio::test]
    async fn test_list_boards() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        // Create some boards by posting to them
        post_to_board(&config, "board-a", "kitty", "Post", "Body", None, vec![])
            .await
            .unwrap();
        post_to_board(&config, "board-b", "kitty", "Post", "Body", None, vec![])
            .await
            .unwrap();

        let boards = list_boards(&config).await.unwrap();

        assert_eq!(boards.len(), 2);
        assert!(boards.contains(&"board-a".to_string()));
        assert!(boards.contains(&"board-b".to_string()));
    }

    #[tokio::test]
    async fn test_include_content_flag() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        post_to_board(&config, "test-board", "kitty", "Post", "Long content here", None, vec![])
            .await
            .unwrap();

        // Without content
        let posts = list_board(&config, "test-board", 10, None, None, false)
            .await
            .unwrap();
        assert!(posts[0].content.is_empty());
        assert!(!posts[0].preview.is_empty());

        // With content
        let posts = list_board(&config, "test-board", 10, None, None, true)
            .await
            .unwrap();
        assert!(!posts[0].content.is_empty());
    }
}

//! Memory file operations
//!
//! Per-persona persistent memories with categories:
//! - patterns: Recurring patterns discovered
//! - moments: Significant moments captured
//! - discoveries: New findings
//! - reflections: Meta-observations

use std::path::Path;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::fs;

use super::config::BbsConfig;
use super::frontmatter::{
    generate_content_id, generate_preview, parse_frontmatter, write_with_frontmatter,
};

/// Valid memory categories
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MemoryCategory {
    Patterns,
    Moments,
    Discoveries,
    Reflections,
}

impl MemoryCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Patterns => "patterns",
            Self::Moments => "moments",
            Self::Discoveries => "discoveries",
            Self::Reflections => "reflections",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "patterns" => Some(Self::Patterns),
            "moments" => Some(Self::Moments),
            "discoveries" => Some(Self::Discoveries),
            "reflections" => Some(Self::Reflections),
            _ => None,
        }
    }

    pub fn all() -> &'static [Self] {
        &[
            Self::Patterns,
            Self::Moments,
            Self::Discoveries,
            Self::Reflections,
        ]
    }
}

impl Default for MemoryCategory {
    fn default() -> Self {
        Self::Patterns
    }
}

/// Memory frontmatter (YAML)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryFrontmatter {
    pub title: String,
    pub date: DateTime<Utc>,
    pub category: String,
    pub persona: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Memory entry (full representation)
#[derive(Debug, Clone, Serialize)]
pub struct Memory {
    pub id: String,
    pub title: String,
    pub category: String,
    pub date: DateTime<Utc>,
    pub tags: Vec<String>,
    pub preview: String,
    pub content: String,
    pub path: String,
}

/// Parse a memory file
async fn parse_memory(
    path: &Path,
) -> Result<Memory, Box<dyn std::error::Error + Send + Sync>> {
    let content = fs::read_to_string(path).await?;
    let (fm, body): (MemoryFrontmatter, String) = parse_frontmatter(&content)?;

    let filename = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    Ok(Memory {
        id: filename.to_string(),
        title: fm.title,
        category: fm.category,
        date: fm.date,
        tags: fm.tags,
        preview: generate_preview(&body, 200),
        content: body,
        path: path.display().to_string(),
    })
}

/// List memories for a persona
pub async fn list_memories(
    config: &BbsConfig,
    persona: &str,
    category_filter: Option<&str>,
    query: Option<&str>,
    limit: usize,
) -> std::io::Result<Vec<Memory>> {
    let mut memories = Vec::new();

    // Determine which categories to search
    let categories: Vec<&str> = match category_filter {
        Some(cat) => vec![cat],
        None => MemoryCategory::all().iter().map(|c| c.as_str()).collect(),
    };

    for category in categories {
        let category_path = config.memories_path(persona, Some(category));

        // Skip if directory doesn't exist
        if !fs::try_exists(&category_path).await.unwrap_or(false) {
            continue;
        }

        let mut entries = fs::read_dir(&category_path).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();

            // Skip non-.md files
            if !path.extension().map(|e| e == "md").unwrap_or(false) {
                continue;
            }

            let memory = match parse_memory(&path).await {
                Ok(m) => m,
                Err(e) => {
                    tracing::warn!("Failed to parse memory {}: {}", path.display(), e);
                    continue;
                }
            };

            // Apply query filter (case-insensitive search in title, content, tags)
            if let Some(q) = query {
                let q_lower = q.to_lowercase();
                let searchable = format!(
                    "{} {} {}",
                    memory.title,
                    memory.content,
                    memory.tags.join(" ")
                )
                .to_lowercase();

                if !searchable.contains(&q_lower) {
                    continue;
                }
            }

            memories.push(memory);
        }
    }

    // Sort by date, most recent first
    memories.sort_by(|a, b| b.date.cmp(&a.date));

    // Apply limit
    memories.truncate(limit);

    Ok(memories)
}

/// Save a new memory
pub async fn save_memory(
    config: &BbsConfig,
    persona: &str,
    title: &str,
    content: &str,
    category: Option<&str>,
    tags: Vec<String>,
) -> std::io::Result<(String, String)> {
    let category_str = category
        .and_then(MemoryCategory::from_str)
        .unwrap_or_default()
        .as_str();

    let category_path = config.memories_path(persona, Some(category_str));
    fs::create_dir_all(&category_path).await?;

    let memory_id = generate_content_id(title);
    let memory_path = category_path.join(format!("{}.md", memory_id));

    let frontmatter = MemoryFrontmatter {
        title: title.to_string(),
        date: Utc::now(),
        category: category_str.to_string(),
        persona: persona.to_string(),
        tags,
    };

    let file_content = write_with_frontmatter(&frontmatter, content)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    fs::write(&memory_path, file_content).await?;

    Ok((memory_id, memory_path.display().to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(temp_dir: &TempDir) -> BbsConfig {
        BbsConfig::with_root(temp_dir.path().to_path_buf())
    }

    #[tokio::test]
    async fn test_save_and_recall_memory() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        // Save memory
        let (mem_id, path) = save_memory(
            &config,
            "kitty",
            "Test Pattern",
            "This is a pattern I discovered",
            Some("patterns"),
            vec!["test".to_string(), "pattern".to_string()],
        )
        .await
        .unwrap();

        assert!(Path::new(&path).exists());
        assert!(mem_id.contains("test-pattern"));

        // Recall memories
        let memories = list_memories(&config, "kitty", None, None, 10)
            .await
            .unwrap();

        assert_eq!(memories.len(), 1);
        assert_eq!(memories[0].title, "Test Pattern");
        assert_eq!(memories[0].category, "patterns");
    }

    #[tokio::test]
    async fn test_filter_by_category() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        // Save memories in different categories
        save_memory(&config, "kitty", "Pattern 1", "Body", Some("patterns"), vec![])
            .await
            .unwrap();
        save_memory(&config, "kitty", "Moment 1", "Body", Some("moments"), vec![])
            .await
            .unwrap();

        // Filter by category
        let patterns = list_memories(&config, "kitty", Some("patterns"), None, 10)
            .await
            .unwrap();
        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].title, "Pattern 1");

        let moments = list_memories(&config, "kitty", Some("moments"), None, 10)
            .await
            .unwrap();
        assert_eq!(moments.len(), 1);
        assert_eq!(moments[0].title, "Moment 1");
    }

    #[tokio::test]
    async fn test_search_query() {
        let temp = TempDir::new().unwrap();
        let config = test_config(&temp);

        save_memory(
            &config,
            "kitty",
            "Floatctl Pattern",
            "Something about floatctl",
            Some("patterns"),
            vec!["cli".to_string()],
        )
        .await
        .unwrap();

        save_memory(
            &config,
            "kitty",
            "BBS Discovery",
            "Something about BBS",
            Some("discoveries"),
            vec!["bbs".to_string()],
        )
        .await
        .unwrap();

        // Search in title
        let results = list_memories(&config, "kitty", None, Some("floatctl"), 10)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert!(results[0].title.contains("Floatctl"));

        // Search in content
        let results = list_memories(&config, "kitty", None, Some("BBS"), 10)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);

        // Search in tags
        let results = list_memories(&config, "kitty", None, Some("cli"), 10)
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_category_default() {
        assert_eq!(MemoryCategory::default().as_str(), "patterns");
    }

    #[test]
    fn test_category_from_str() {
        assert_eq!(
            MemoryCategory::from_str("patterns"),
            Some(MemoryCategory::Patterns)
        );
        assert_eq!(
            MemoryCategory::from_str("MOMENTS"),
            Some(MemoryCategory::Moments)
        );
        assert_eq!(MemoryCategory::from_str("invalid"), None);
    }
}

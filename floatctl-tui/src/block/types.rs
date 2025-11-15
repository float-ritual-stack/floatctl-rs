use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Unique identifier for blocks
pub type BlockId = Uuid;

/// Agent identifiers
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentId {
    Evna,
    Lf1m,
    Karen,
    Custom(String),
}

impl std::fmt::Display for AgentId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentId::Evna => write!(f, "evna"),
            AgentId::Lf1m => write!(f, "lf1m"),
            AgentId::Karen => write!(f, "karen"),
            AgentId::Custom(name) => write!(f, "{}", name),
        }
    }
}

/// Board identifiers
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BoardId {
    Scratch,
    Work,
    Tech,
    LifeAdmin,
    ND,
    Recent,
    Custom(String),
}

impl std::fmt::Display for BoardId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoardId::Scratch => write!(f, "/scratch/"),
            BoardId::Work => write!(f, "/work/"),
            BoardId::Tech => write!(f, "/tech/"),
            BoardId::LifeAdmin => write!(f, "/life-admin/"),
            BoardId::ND => write!(f, "/nd/"),
            BoardId::Recent => write!(f, "/recent/"),
            BoardId::Custom(path) => write!(f, "{}", path),
        }
    }
}

/// Annotations parsed from ctx:: entries
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Annotation {
    /// project::name
    Project(String),
    /// meeting::identifier
    Meeting(String),
    /// mode::name
    Mode(String),
    /// connectTo::target
    ConnectTo(String),
    /// lf1m::topic
    Lf1m(String),
    /// issue::number
    Issue(String),
    /// Custom annotation
    Custom { key: String, value: String },
}

impl Annotation {
    /// Parse annotation from "key::value" format
    pub fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.splitn(2, "::").collect();
        if parts.len() != 2 {
            return None;
        }

        let key = parts[0].trim();
        let value = parts[1].trim().to_string();

        match key {
            "project" => Some(Annotation::Project(value)),
            "meeting" => Some(Annotation::Meeting(value)),
            "mode" => Some(Annotation::Mode(value)),
            "connectTo" => Some(Annotation::ConnectTo(value)),
            "lf1m" => Some(Annotation::Lf1m(value)),
            "issue" => Some(Annotation::Issue(value)),
            _ => Some(Annotation::Custom {
                key: key.to_string(),
                value,
            }),
        }
    }

    /// Get the annotation key
    pub fn key(&self) -> &str {
        match self {
            Annotation::Project(_) => "project",
            Annotation::Meeting(_) => "meeting",
            Annotation::Mode(_) => "mode",
            Annotation::ConnectTo(_) => "connectTo",
            Annotation::Lf1m(_) => "lf1m",
            Annotation::Issue(_) => "issue",
            Annotation::Custom { key, .. } => key,
        }
    }

    /// Get the annotation value
    pub fn value(&self) -> &str {
        match self {
            Annotation::Project(v)
            | Annotation::Meeting(v)
            | Annotation::Mode(v)
            | Annotation::ConnectTo(v)
            | Annotation::Lf1m(v)
            | Annotation::Issue(v) => v,
            Annotation::Custom { value, .. } => value,
        }
    }
}

/// Link targets
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkTarget {
    /// Link to another block by ID
    Block(BlockId),
    /// Link to a file path
    File(String),
    /// Link to a URL
    Url(String),
    /// Link to a board
    Board(BoardId),
}

/// Hints for rendering custom components
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum RenderHint {
    /// Render inline with text
    Inline,
    /// Render as a block (full width)
    Block,
    /// Render in a popup/overlay
    Popup,
    /// Custom rendering instructions
    Custom(String),
}

/// Core block types - extensible for agent-driven components
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Block {
    /// Plain text entry
    Text {
        id: BlockId,
        content: String,
        timestamp: DateTime<Utc>,
    },

    /// ctx:: marked entry - triggers board updates
    ContextEntry {
        id: BlockId,
        marker: String,       // "ctx::2025-10-03 @ 07:36:52 PM"
        content: Vec<String>, // bullet points
        annotations: Vec<Annotation>,
        timestamp: DateTime<Utc>,
    },

    /// Agent response block - rendered as forum post
    AgentPost {
        id: BlockId,
        agent: AgentId,
        board: BoardId,
        title: Option<String>,
        content: Vec<Block>, // recursive - posts can contain blocks
        timestamp: DateTime<Utc>,
        references: Vec<BlockId>, // links to scratch entries
    },

    /// Custom component - agent-inserted via structured output
    Component {
        id: BlockId,
        component_type: String,  // "calendar", "resource_list", "thread_view"
        data: serde_json::Value, // arbitrary structured data
        render_hint: RenderHint,
        timestamp: DateTime<Utc>,
    },

    /// Code block
    Code {
        id: BlockId,
        language: Option<String>,
        content: String,
        timestamp: DateTime<Utc>,
    },

    /// Link to another block/note
    Link {
        id: BlockId,
        target: LinkTarget,
        display: String,
        timestamp: DateTime<Utc>,
    },
}

impl Block {
    /// Get the block's unique ID
    pub fn id(&self) -> BlockId {
        match self {
            Block::Text { id, .. }
            | Block::ContextEntry { id, .. }
            | Block::AgentPost { id, .. }
            | Block::Component { id, .. }
            | Block::Code { id, .. }
            | Block::Link { id, .. } => *id,
        }
    }

    /// Get the block's timestamp
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Block::Text { timestamp, .. }
            | Block::ContextEntry { timestamp, .. }
            | Block::AgentPost { timestamp, .. }
            | Block::Component { timestamp, .. }
            | Block::Code { timestamp, .. }
            | Block::Link { timestamp, .. } => *timestamp,
        }
    }

    /// Create a new text block
    pub fn new_text(content: String) -> Self {
        Block::Text {
            id: Uuid::new_v4(),
            content,
            timestamp: Utc::now(),
        }
    }

    /// Create a new context entry block
    pub fn new_context_entry(
        marker: String,
        content: Vec<String>,
        annotations: Vec<Annotation>,
    ) -> Self {
        Block::ContextEntry {
            id: Uuid::new_v4(),
            marker,
            content,
            annotations,
            timestamp: Utc::now(),
        }
    }

    /// Create a new agent post block
    pub fn new_agent_post(
        agent: AgentId,
        board: BoardId,
        title: Option<String>,
        content: Vec<Block>,
        references: Vec<BlockId>,
    ) -> Self {
        Block::AgentPost {
            id: Uuid::new_v4(),
            agent,
            board,
            title,
            content,
            timestamp: Utc::now(),
            references,
        }
    }

    /// Create a new code block
    pub fn new_code(language: Option<String>, content: String) -> Self {
        Block::Code {
            id: Uuid::new_v4(),
            language,
            content,
            timestamp: Utc::now(),
        }
    }

    /// Create a new link block
    pub fn new_link(target: LinkTarget, display: String) -> Self {
        Block::Link {
            id: Uuid::new_v4(),
            target,
            display,
            timestamp: Utc::now(),
        }
    }

    /// Create a new component block
    pub fn new_component(
        component_type: String,
        data: serde_json::Value,
        render_hint: RenderHint,
    ) -> Self {
        Block::Component {
            id: Uuid::new_v4(),
            component_type,
            data,
            render_hint,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_annotation_parse() {
        let cases = vec![
            ("project::rangle", Annotation::Project("rangle".into())),
            ("meeting::standup", Annotation::Meeting("standup".into())),
            ("mode::work", Annotation::Mode("work".into())),
            (
                "connectTo::FLOAT Block",
                Annotation::ConnectTo("FLOAT Block".into()),
            ),
            (
                "lf1m::context-switch",
                Annotation::Lf1m("context-switch".into()),
            ),
            ("issue::123", Annotation::Issue("123".into())),
        ];

        for (input, expected) in cases {
            let parsed = Annotation::parse(input).unwrap();
            assert_eq!(parsed, expected);
        }
    }

    #[test]
    fn test_annotation_invalid() {
        assert!(Annotation::parse("invalid").is_none());
        assert!(Annotation::parse("").is_none());
    }

    #[test]
    fn test_block_creation() {
        let block = Block::new_text("hello world".into());
        assert!(matches!(block, Block::Text { .. }));

        let ctx_block = Block::new_context_entry(
            "ctx::2025-11-15".into(),
            vec!["line 1".into()],
            vec![Annotation::Project("test".into())],
        );
        assert!(matches!(ctx_block, Block::ContextEntry { .. }));
    }

    #[test]
    fn test_block_id_timestamp() {
        let block = Block::new_text("test".into());
        let id = block.id();
        let timestamp = block.timestamp();

        // ID should be unique
        let block2 = Block::new_text("test2".into());
        assert_ne!(id, block2.id());

        // Timestamp should be recent
        assert!(timestamp <= Utc::now());
    }
}

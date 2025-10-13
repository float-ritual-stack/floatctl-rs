use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactKind {
    Code,
    Text,
    Binary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub message_idx: i32,
    pub title: String,
    pub filename: String,
    pub kind: ArtifactKind,
    pub language: Option<String>,
    pub body: String,
}

impl Artifact {
    pub fn new_code(
        idx: i32,
        title: impl Into<String>,
        filename: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            message_idx: idx,
            title: title.into(),
            filename: filename.into(),
            kind: ArtifactKind::Code,
            language: None,
            body: body.into(),
        }
    }
}

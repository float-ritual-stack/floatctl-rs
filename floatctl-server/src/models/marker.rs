//! Marker extraction - Spec 1.2
//!
//! Integrates with floatctl-core marker system.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Marker pattern regex
/// Matches: ctx::value, project::value, mode::value, bridge::value, float.value
static MARKER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(ctx::[^\s]+|project::[^\s]+|mode::[^\s]+|bridge::[^\s]+|float\.[^\s]+)")
        .expect("invalid marker regex")
});

/// Marker kinds supported by the server
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MarkerKind {
    Ctx,
    Project,
    Mode,
    Bridge,
    Float,
}

impl MarkerKind {
    /// Parse marker kind from prefix.
    pub fn from_prefix(prefix: &str) -> Option<Self> {
        match prefix.to_lowercase().as_str() {
            "ctx" => Some(Self::Ctx),
            "project" => Some(Self::Project),
            "mode" => Some(Self::Mode),
            "bridge" => Some(Self::Bridge),
            "float" => Some(Self::Float),
            _ => None,
        }
    }

    /// Get string representation.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Ctx => "ctx",
            Self::Project => "project",
            Self::Mode => "mode",
            Self::Bridge => "bridge",
            Self::Float => "float",
        }
    }
}

/// Extracted marker with kind and value
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Marker {
    pub kind: MarkerKind,
    pub value: String,
}

impl Marker {
    /// Extract all markers from text.
    ///
    /// # Example
    /// ```
    /// use floatctl_server::models::{Marker, MarkerKind};
    ///
    /// let markers = Marker::extract_from("ctx::review project::api");
    /// assert_eq!(markers.len(), 2);
    /// ```
    pub fn extract_from(text: &str) -> Vec<Self> {
        MARKER_RE
            .find_iter(text)
            .filter_map(|m| Self::parse(m.as_str()))
            .collect()
    }

    /// Parse a single marker string.
    ///
    /// # Format
    /// - `ctx::value` → Ctx("value")
    /// - `project::value` → Project("value")
    /// - `float.value` → Float("value")
    pub fn parse(s: &str) -> Option<Self> {
        let s = s.trim();

        // Handle float.xxx format
        if let Some(value) = s.strip_prefix("float.").or_else(|| s.strip_prefix("Float.")) {
            return Some(Self {
                kind: MarkerKind::Float,
                value: value.to_owned(),
            });
        }

        // Handle prefix::value format
        let parts: Vec<&str> = s.splitn(2, "::").collect();
        if parts.len() != 2 {
            return None;
        }

        let kind = MarkerKind::from_prefix(parts[0])?;
        let value = parts[1].to_owned();

        Some(Self { kind, value })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ctx_marker() {
        let marker = Marker::parse("ctx::review").unwrap();
        assert_eq!(marker.kind, MarkerKind::Ctx);
        assert_eq!(marker.value, "review");
    }

    #[test]
    fn parse_project_marker() {
        let marker = Marker::parse("project::api").unwrap();
        assert_eq!(marker.kind, MarkerKind::Project);
        assert_eq!(marker.value, "api");
    }

    #[test]
    fn parse_float_marker() {
        let marker = Marker::parse("float.config").unwrap();
        assert_eq!(marker.kind, MarkerKind::Float);
        assert_eq!(marker.value, "config");
    }

    #[test]
    fn parse_case_insensitive() {
        let marker = Marker::parse("CTX::REVIEW").unwrap();
        assert_eq!(marker.kind, MarkerKind::Ctx);
        assert_eq!(marker.value, "REVIEW"); // value preserves case
    }

    #[test]
    fn extract_multiple() {
        let markers = Marker::extract_from("ctx::a project::b mode::c");
        assert_eq!(markers.len(), 3);
    }

    #[test]
    fn extract_from_sentence() {
        let text = "Please review ctx::urgent the project::api changes";
        let markers = Marker::extract_from(text);
        assert_eq!(markers.len(), 2);
    }

    #[test]
    fn invalid_marker() {
        assert!(Marker::parse("invalid").is_none());
        assert!(Marker::parse("unknown::value").is_none());
    }
}

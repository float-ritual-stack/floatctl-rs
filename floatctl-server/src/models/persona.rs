//! Persona - Filesystem-validated persona wrapper
//!
//! Personas are validated against the BBS filesystem structure:
//! - {bbs_root}/inbox/{persona}/ - inbox directory exists
//! - {bbs_root}/{persona}/ - root-level persona directory exists
//!
//! Any string is valid if corresponding directory exists on filesystem.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::ValidationError;

/// Filesystem-validated persona (dynamic, not hardcoded enum)
///
/// Serializes as a bare string for JSON compatibility.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Persona(String);

impl Persona {
    /// Validate persona against filesystem structure.
    ///
    /// Valid if either:
    /// - `{bbs_root}/inbox/{persona}/` exists
    /// - `{bbs_root}/{persona}/` exists
    ///
    /// # Arguments
    /// * `s` - Persona name (case-insensitive)
    /// * `bbs_root` - BBS root directory (e.g., `/opt/float/bbs`)
    pub fn from_str_validated(s: &str, bbs_root: &Path) -> Result<Self, ValidationError> {
        let name = s.to_lowercase();

        // Reject empty names
        if name.is_empty() {
            return Err(ValidationError::Empty { field: "persona" });
        }

        // Check inbox/{persona}/ exists
        let inbox_path = bbs_root.join("inbox").join(&name);

        // OR {persona}/ exists (root-level persona dir)
        let persona_path = bbs_root.join(&name);

        if inbox_path.is_dir() || persona_path.is_dir() {
            Ok(Self(name))
        } else {
            Err(ValidationError::InvalidVariant {
                field: "persona",
                value: s.to_owned(),
            })
        }
    }

    /// Create persona without validation (for testing or trusted contexts).
    ///
    /// Use `from_str_validated` for user input.
    pub fn new_unchecked(s: &str) -> Self {
        Self(s.to_lowercase())
    }

    /// Get inner string reference.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// List all valid personas from filesystem.
    ///
    /// Scans `{bbs_root}/inbox/` for subdirectories.
    pub fn list_all(bbs_root: &Path) -> Vec<Self> {
        let inbox_dir = bbs_root.join("inbox");
        let mut personas = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&inbox_dir) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    if let Some(name) = entry.file_name().to_str() {
                        // Skip hidden directories and non-persona dirs
                        if !name.starts_with('.') {
                            personas.push(Self(name.to_lowercase()));
                        }
                    }
                }
            }
        }

        // Sort for consistent ordering
        personas.sort_by(|a, b| a.0.cmp(&b.0));
        personas
    }
}

impl std::fmt::Display for Persona {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn setup_test_bbs() -> TempDir {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create inbox directories
        fs::create_dir_all(root.join("inbox/kitty")).unwrap();
        fs::create_dir_all(root.join("inbox/daddy")).unwrap();
        fs::create_dir_all(root.join("inbox/evan")).unwrap();

        // Create root-level persona directories
        fs::create_dir_all(root.join("cowboy")).unwrap();

        temp
    }

    #[test]
    fn valid_personas_inbox() {
        let temp = setup_test_bbs();
        let root = temp.path();

        // Inbox-based personas
        assert!(Persona::from_str_validated("kitty", root).is_ok());
        assert!(Persona::from_str_validated("daddy", root).is_ok());
        assert!(Persona::from_str_validated("evan", root).is_ok());
    }

    #[test]
    fn valid_personas_root() {
        let temp = setup_test_bbs();
        let root = temp.path();

        // Root-level persona directory
        assert!(Persona::from_str_validated("cowboy", root).is_ok());
    }

    #[test]
    fn case_insensitive() {
        let temp = setup_test_bbs();
        let root = temp.path();

        let p1 = Persona::from_str_validated("KITTY", root).unwrap();
        let p2 = Persona::from_str_validated("Kitty", root).unwrap();
        let p3 = Persona::from_str_validated("kitty", root).unwrap();

        assert_eq!(p1, p2);
        assert_eq!(p2, p3);
        assert_eq!(p1.as_str(), "kitty");
    }

    #[test]
    fn invalid_persona() {
        let temp = setup_test_bbs();
        let root = temp.path();

        let err = Persona::from_str_validated("nonexistent", root).unwrap_err();
        assert!(matches!(err, ValidationError::InvalidVariant { .. }));
    }

    #[test]
    fn empty_persona_rejected() {
        let temp = setup_test_bbs();
        let root = temp.path();

        let err = Persona::from_str_validated("", root).unwrap_err();
        assert!(matches!(err, ValidationError::Empty { .. }));
    }

    #[test]
    fn list_all_personas() {
        let temp = setup_test_bbs();
        let root = temp.path();

        let personas = Persona::list_all(root);

        // Should find inbox-based personas (sorted)
        assert_eq!(personas.len(), 3);
        assert_eq!(personas[0].as_str(), "daddy");
        assert_eq!(personas[1].as_str(), "evan");
        assert_eq!(personas[2].as_str(), "kitty");
    }

    #[test]
    fn new_unchecked_works() {
        let p = Persona::new_unchecked("TestPersona");
        assert_eq!(p.as_str(), "testpersona");
    }

    #[test]
    fn serialization_roundtrip() {
        let p = Persona::new_unchecked("kitty");
        let json = serde_json::to_string(&p).unwrap();
        assert_eq!(json, "\"kitty\"");

        let parsed: Persona = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, p);
    }
}

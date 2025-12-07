//! BBS configuration - paths and environment loading
//!
//! Configuration priority (highest to lowest):
//! 1. `BBS_ROOT` environment variable
//! 2. `[bbs].root` in ~/.floatctl/config.toml
//! 3. Default: /opt/float/bbs

use std::path::PathBuf;

use floatctl_core::FloatConfig;

/// BBS configuration
#[derive(Debug, Clone)]
pub struct BbsConfig {
    /// Root directory for BBS files (e.g., /opt/float/bbs)
    pub root_dir: PathBuf,
}

impl BbsConfig {
    /// Create config from environment/config file
    ///
    /// Priority: BBS_ROOT env > config.toml [bbs].root > default
    pub fn from_env() -> Self {
        // 1. Check BBS_ROOT env var first
        if let Ok(root) = std::env::var("BBS_ROOT") {
            return Self {
                root_dir: PathBuf::from(root),
            };
        }

        // 2. Try loading from ~/.floatctl/config.toml
        if let Ok(config) = FloatConfig::load() {
            if let Some(bbs) = config.bbs {
                return Self { root_dir: bbs.root };
            }
        }

        // 3. Fall back to default
        Self {
            root_dir: PathBuf::from("/opt/float/bbs"),
        }
    }

    /// Create config with explicit root directory (for testing)
    pub fn with_root(root_dir: PathBuf) -> Self {
        Self { root_dir }
    }

    /// Inbox path for a persona
    pub fn inbox_path(&self, persona: &str) -> PathBuf {
        self.root_dir.join("inbox").join(persona)
    }

    /// Read markers path for a persona's inbox
    pub fn read_markers_path(&self, persona: &str) -> PathBuf {
        self.inbox_path(persona).join(".read")
    }

    /// Memories base path for a persona
    pub fn memories_base_path(&self, persona: &str) -> PathBuf {
        self.root_dir.join(persona).join("memories")
    }

    /// Memories path for a persona and optional category
    pub fn memories_path(&self, persona: &str, category: Option<&str>) -> PathBuf {
        let base = self.memories_base_path(persona);
        match category {
            Some(cat) => base.join(cat),
            None => base,
        }
    }

    /// Board path
    pub fn board_path(&self, board_name: &str) -> PathBuf {
        self.root_dir.join("boards").join(board_name)
    }

    /// List of all boards path
    pub fn boards_root(&self) -> PathBuf {
        self.root_dir.join("boards")
    }
}

impl Default for BbsConfig {
    fn default() -> Self {
        Self::from_env()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_are_correct() {
        let config = BbsConfig::with_root(PathBuf::from("/test/bbs"));

        assert_eq!(
            config.inbox_path("kitty"),
            PathBuf::from("/test/bbs/inbox/kitty")
        );
        assert_eq!(
            config.read_markers_path("kitty"),
            PathBuf::from("/test/bbs/inbox/kitty/.read")
        );
        assert_eq!(
            config.memories_path("kitty", Some("patterns")),
            PathBuf::from("/test/bbs/kitty/memories/patterns")
        );
        assert_eq!(
            config.memories_path("kitty", None),
            PathBuf::from("/test/bbs/kitty/memories")
        );
        assert_eq!(
            config.board_path("sysops-log"),
            PathBuf::from("/test/bbs/boards/sysops-log")
        );
    }
}

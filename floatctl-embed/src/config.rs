use std::collections::HashMap;
use std::path::PathBuf;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Load environment variables from .env files in multiple locations
///
/// Priority order (highest to lowest):
/// 1. Current directory .env
/// 2. ~/.floatctl/.env
/// 3. Environment variables already set
///
/// This allows:
/// - Global installation: `cargo install --path floatctl-cli`
/// - Global config: ~/.floatctl/.env
/// - Local overrides: ./.env in any directory
pub fn load_dotenv() -> Result<()> {
    let mut loaded_from = Vec::new();

    // Check current directory first (highest priority)
    if let Ok(path) = dotenvy::dotenv() {
        loaded_from.push(format!("current directory ({})", path.display()));
        debug!("Loaded .env from current directory: {}", path.display());
    }

    // Check ~/.floatctl/.env
    if let Some(home_dir) = dirs::home_dir() {
        let config_dir = home_dir.join(".floatctl");
        let env_file = config_dir.join(".env");

        if env_file.exists() {
            // dotenvy doesn't overwrite existing vars, so this is safe
            match dotenvy::from_path(&env_file) {
                Ok(_) => {
                    loaded_from.push(format!("~/.floatctl/.env ({})", env_file.display()));
                    debug!("Loaded .env from ~/.floatctl: {}", env_file.display());
                }
                Err(e) => {
                    debug!("Failed to load ~/.floatctl/.env: {}", e);
                }
            }
        }
    }

    if loaded_from.is_empty() {
        debug!("No .env files found (current dir or ~/.floatctl)");
        info!("Using environment variables only (no .env file found)");
    } else {
        info!("Loaded configuration from: {}", loaded_from.join(", "));
    }

    Ok(())
}

/// Get the floatctl config directory path (~/.floatctl)
pub fn config_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home| home.join(".floatctl"))
}

/// Ensure the floatctl config directory exists
pub fn ensure_config_dir() -> Result<PathBuf> {
    let config_dir = config_dir().context("Could not determine home directory")?;

    if !config_dir.exists() {
        std::fs::create_dir_all(&config_dir)
            .context(format!("Failed to create config directory: {}", config_dir.display()))?;
        info!("Created config directory: {}", config_dir.display());
    }

    Ok(config_dir)
}

/// Get the default conversation exports directory (~/.floatctl/conversation-exports)
pub fn default_exports_dir() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("conversation-exports"))
}

/// Ensure the conversation exports directory exists
pub fn ensure_exports_dir() -> Result<PathBuf> {
    let exports_dir = default_exports_dir().context("Could not determine home directory")?;

    if !exports_dir.exists() {
        std::fs::create_dir_all(&exports_dir)
            .context(format!("Failed to create exports directory: {}", exports_dir.display()))?;
        debug!("Created exports directory: {}", exports_dir.display());
    }

    Ok(exports_dir)
}

// ============================================================================
// TOML Configuration
// ============================================================================

/// Quick wins TOML configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FloatctlConfig {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub query: QueryConfig,

    #[serde(default)]
    pub embedding: EmbeddingConfig,

    #[serde(default)]
    pub projects: ProjectsConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct GeneralConfig {
    /// Default output directory for conversation exports
    /// Defaults to ~/.floatctl/conversation-exports if not specified
    #[serde(default)]
    pub default_output_dir: Option<String>,
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryConfig {
    #[serde(default = "default_query_limit")]
    pub default_limit: i64,

    #[serde(default)]
    pub threshold: Option<f64>,

    #[serde(default = "default_output_format")]
    pub output_format: String,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            default_limit: default_query_limit(),
            threshold: None,
            output_format: default_output_format(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,

    #[serde(default = "default_rate_limit_ms")]
    pub rate_limit_ms: u64,

    #[serde(default)]
    pub skip_existing: bool,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            batch_size: default_batch_size(),
            rate_limit_ms: default_rate_limit_ms(),
            skip_existing: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProjectsConfig {
    /// Project aliases for fuzzy matching
    /// Format: "canonical_name" = ["alias1", "alias2", ...]
    #[serde(default)]
    pub aliases: HashMap<String, Vec<String>>,
}

// Default value functions for serde
fn default_query_limit() -> i64 {
    10
}

fn default_output_format() -> String {
    "text".to_string()
}

fn default_batch_size() -> usize {
    32
}

fn default_rate_limit_ms() -> u64 {
    500
}

impl FloatctlConfig {
    /// Load config from TOML files
    ///
    /// Priority order (highest to lowest):
    /// 1. ./floatctl.toml (project-specific)
    /// 2. ~/.floatctl/config.toml (user defaults)
    /// 3. Built-in defaults
    pub fn load() -> Self {
        let mut config = FloatctlConfig::default();

        // Try global config first (~/.floatctl/config.toml)
        if let Some(global_config_path) = config_dir().map(|d| d.join("config.toml")) {
            if global_config_path.exists() {
                match std::fs::read_to_string(&global_config_path) {
                    Ok(contents) => match toml::from_str::<FloatctlConfig>(&contents) {
                        Ok(global_config) => {
                            debug!("Loaded global config from {}", global_config_path.display());
                            config = global_config;
                        }
                        Err(e) => {
                            warn!("Failed to parse {}: {}", global_config_path.display(), e);
                        }
                    },
                    Err(e) => {
                        debug!("Failed to read {}: {}", global_config_path.display(), e);
                    }
                }
            }
        }

        // Try local config (./floatctl.toml) - overrides global
        let local_config_path = PathBuf::from("floatctl.toml");
        if local_config_path.exists() {
            match std::fs::read_to_string(&local_config_path) {
                Ok(contents) => match toml::from_str::<FloatctlConfig>(&contents) {
                    Ok(local_config) => {
                        debug!("Loaded local config from {}", local_config_path.display());
                        // Merge local config over global (local takes precedence)
                        config = Self::merge(config, local_config);
                    }
                    Err(e) => {
                        warn!("Failed to parse {}: {}", local_config_path.display(), e);
                    }
                },
                Err(e) => {
                    debug!("Failed to read {}: {}", local_config_path.display(), e);
                }
            }
        }

        config
    }

    /// Merge two configs (right overrides left)
    fn merge(mut base: Self, overlay: Self) -> Self {
        // For simple configs, just use overlay values
        // (In a more complex system, you'd do field-by-field merging)
        base.general = overlay.general;
        base.query = overlay.query;
        base.embedding = overlay.embedding;

        // Merge project aliases (combine both)
        for (key, values) in overlay.projects.aliases {
            base.projects.aliases.insert(key, values);
        }

        base
    }

    /// Get the configured default output directory
    /// Returns the path from config, or ~/.floatctl/conversation-exports if not configured
    pub fn get_default_output_dir(&self) -> Result<PathBuf> {
        if let Some(ref custom_path) = self.general.default_output_dir {
            // Expand ~ to home directory
            let path_str = custom_path.as_str();
            if path_str.starts_with("~/") {
                let home = dirs::home_dir().context("Could not determine home directory")?;
                Ok(home.join(&path_str[2..]))
            } else if path_str == "~" {
                dirs::home_dir().context("Could not determine home directory")
            } else {
                Ok(PathBuf::from(path_str))
            }
        } else {
            // Use default ~/.floatctl/conversation-exports
            default_exports_dir().context("Could not determine home directory")
        }
    }

    /// Get project aliases for a given project name
    pub fn get_project_aliases(&self, project: &str) -> Vec<String> {
        let project_lower = project.to_lowercase();

        // Find matching canonical or alias
        for (canonical, aliases) in &self.projects.aliases {
            let all_variants: Vec<String> = std::iter::once(canonical.clone())
                .chain(aliases.iter().cloned())
                .map(|s| s.to_lowercase())
                .collect();

            if all_variants.iter().any(|v| v.contains(&project_lower) || project_lower.contains(v)) {
                return std::iter::once(canonical.clone())
                    .chain(aliases.iter().cloned())
                    .collect();
            }
        }

        // No match - return original
        vec![project.to_string()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_dir_returns_path() {
        // Should return a path on all platforms
        let dir = config_dir();
        assert!(dir.is_some());

        if let Some(path) = dir {
            assert!(path.ends_with(".floatctl"));
        }
    }

    #[test]
    fn test_ensure_config_dir_creates_if_missing() {
        // This test would require mocking the home directory
        // For now, just ensure it doesn't panic
        let result = ensure_config_dir();
        // Should either succeed or fail with a clear error
        if let Err(e) = result {
            println!("Config dir error (expected in test): {}", e);
        }
    }

    #[test]
    fn test_load_dotenv_doesnt_panic() {
        // Should never panic, even if no .env exists
        let result = load_dotenv();
        assert!(result.is_ok());
    }

    #[test]
    fn test_floatctl_config_defaults() {
        let config = FloatctlConfig::default();
        assert_eq!(config.query.default_limit, 10);
        assert_eq!(config.embedding.batch_size, 32);
        assert_eq!(config.embedding.rate_limit_ms, 500);
        assert_eq!(config.embedding.skip_existing, false);
    }

    #[test]
    fn test_floatctl_config_load_doesnt_panic() {
        // Should never panic, even if no config files exist
        let config = FloatctlConfig::load();
        assert_eq!(config.query.default_limit, 10); // Should use defaults
    }

    #[test]
    fn test_project_aliases_no_match() {
        let config = FloatctlConfig::default();
        let aliases = config.get_project_aliases("unknown-project");
        assert_eq!(aliases, vec!["unknown-project"]);
    }

    #[test]
    fn test_project_aliases_with_match() {
        let mut config = FloatctlConfig::default();
        config.projects.aliases.insert(
            "rangle/pharmacy".to_string(),
            vec!["pharmacy".to_string(), "pharm".to_string()],
        );

        let aliases = config.get_project_aliases("pharmacy");
        assert!(aliases.contains(&"rangle/pharmacy".to_string()));
        assert!(aliases.contains(&"pharmacy".to_string()));
        assert!(aliases.contains(&"pharm".to_string()));
    }

    #[test]
    fn test_default_exports_dir() {
        let exports_dir = default_exports_dir();
        assert!(exports_dir.is_some());

        if let Some(path) = exports_dir {
            assert!(path.ends_with(".floatctl/conversation-exports"));
        }
    }
}

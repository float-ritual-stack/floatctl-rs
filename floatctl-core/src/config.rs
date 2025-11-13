use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::PathBuf;

/// Centralized configuration for floatctl ecosystem
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatConfig {
    pub machine: MachineConfig,
    pub paths: PathsConfig,
    pub evna: Option<EvnaConfig>,
    pub floatctl: Option<FloatctlConfig>,
    pub r2: Option<R2Config>,
    pub integrations: Option<IntegrationsConfig>,

    /// Machine-specific overrides (keyed by machine name)
    #[serde(flatten)]
    pub machine_overrides: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MachineConfig {
    pub name: String,
    pub environment: String,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathsConfig {
    pub float_home: PathBuf,
    pub daily_notes_home: PathBuf,
    pub daily_notes: PathBuf,
    pub bridges: PathBuf,
    pub operations: PathBuf,
    pub inbox: PathBuf,
    pub dispatches: PathBuf,
    pub archives: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvnaConfig {
    pub database_url: String,
    pub system_prompt: Option<PathBuf>,
    pub mcp_server_port: Option<u16>,
    pub active_context_ttl: Option<String>,
    pub sessions_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FloatctlConfig {
    pub cache_dir: Option<PathBuf>,
    pub scripts_dir: Option<PathBuf>,
    pub log_level: Option<String>,
    pub conversation_exports: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct R2Config {
    pub enabled: bool,
    pub bucket_name: String,
    pub account_id: String,
    pub api_token: String,
    pub daily_notes_prefix: Option<String>,
    pub dispatch_prefix: Option<String>,
    pub archive_prefix: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationsConfig {
    pub github_org: Option<String>,
    pub cohere_api_key: Option<String>,
    pub openai_api_key: Option<String>,
    pub anthropic_api_key: Option<String>,
}

impl FloatConfig {
    /// Load config from ~/.floatctl/config.toml
    ///
    /// Fails hard with actionable error if config doesn't exist
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        if !config_path.exists() {
            anyhow::bail!(
                "Config not found at {:?}\n\nRun: floatctl config init",
                config_path
            );
        }

        let content = fs::read_to_string(&config_path)
            .context(format!("Failed to read config file: {:?}", config_path))?;

        let mut config: Self = toml::from_str(&content)
            .context("Failed to parse config file (invalid TOML)")?;

        // Apply machine-specific overrides
        config.apply_machine_overrides()?;

        // Expand variables (${var} substitution)
        config.expand_variables()?;

        Ok(config)
    }

    /// Get config file path: ~/.floatctl/config.toml
    pub fn config_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".floatctl/config.toml")
    }

    /// Get current machine name (from env or config)
    pub fn current_machine(&self) -> String {
        env::var("FLOATCTL_MACHINE")
            .unwrap_or_else(|_| self.machine.name.clone())
    }

    /// Apply machine-specific overrides (e.g., [paths."hetzner-box"])
    fn apply_machine_overrides(&mut self) -> Result<()> {
        let machine = self.current_machine();

        // Check for paths override
        let paths_key = format!("paths.{}", machine);
        if let Some(override_value) = self.machine_overrides.get(&paths_key) {
            if let toml::Value::Table(table) = override_value {
                // Merge paths
                if let Some(float_home) = table.get("float_home").and_then(|v| v.as_str()) {
                    self.paths.float_home = PathBuf::from(float_home);
                }
                if let Some(daily_notes_home) = table.get("daily_notes_home").and_then(|v| v.as_str()) {
                    self.paths.daily_notes_home = PathBuf::from(daily_notes_home);
                }
            }
        }

        // Check for evna override
        let evna_key = format!("evna.{}", machine);
        if let Some(override_value) = self.machine_overrides.get(&evna_key) {
            if let toml::Value::Table(table) = override_value {
                if let Some(ref mut evna) = self.evna {
                    if let Some(database_url) = table.get("database_url").and_then(|v| v.as_str()) {
                        evna.database_url = database_url.to_string();
                    }
                    if let Some(port) = table.get("mcp_server_port").and_then(|v| v.as_integer()) {
                        evna.mcp_server_port = Some(port as u16);
                    }
                }
            }
        }

        Ok(())
    }

    /// Expand ${var} references in paths
    fn expand_variables(&mut self) -> Result<()> {
        let mut vars = HashMap::new();

        // Environment variables
        vars.insert("HOME".to_string(), env::var("HOME").unwrap_or_default());
        vars.insert("DATABASE_URL".to_string(), env::var("DATABASE_URL").unwrap_or_default());
        vars.insert("R2_ACCOUNT_ID".to_string(), env::var("R2_ACCOUNT_ID").unwrap_or_default());
        vars.insert("R2_API_TOKEN".to_string(), env::var("R2_API_TOKEN").unwrap_or_default());
        vars.insert("COHERE_API_KEY".to_string(), env::var("COHERE_API_KEY").unwrap_or_default());
        vars.insert("OPENAI_API_KEY".to_string(), env::var("OPENAI_API_KEY").unwrap_or_default());
        vars.insert("ANTHROPIC_API_KEY".to_string(), env::var("ANTHROPIC_API_KEY").unwrap_or_default());

        // Config variables (for path substitution)
        vars.insert("float_home".to_string(), self.paths.float_home.display().to_string());
        vars.insert("daily_notes_home".to_string(), self.paths.daily_notes_home.display().to_string());

        // Expand paths
        self.paths.daily_notes = Self::expand_path(&self.paths.daily_notes, &vars)?;
        self.paths.bridges = Self::expand_path(&self.paths.bridges, &vars)?;
        self.paths.operations = Self::expand_path(&self.paths.operations, &vars)?;
        self.paths.inbox = Self::expand_path(&self.paths.inbox, &vars)?;
        self.paths.dispatches = Self::expand_path(&self.paths.dispatches, &vars)?;
        if let Some(ref archives) = self.paths.archives {
            self.paths.archives = Some(Self::expand_path(archives, &vars)?);
        }

        // Expand evna paths
        if let Some(ref mut evna) = self.evna {
            if let Some(ref prompt) = evna.system_prompt {
                evna.system_prompt = Some(Self::expand_path(prompt, &vars)?);
            }
            if let Some(ref sessions) = evna.sessions_dir {
                evna.sessions_dir = Some(Self::expand_path(sessions, &vars)?);
            }
            evna.database_url = Self::expand_string(&evna.database_url, &vars);
        }

        // Expand floatctl paths
        if let Some(ref mut floatctl) = self.floatctl {
            if let Some(ref cache) = floatctl.cache_dir {
                floatctl.cache_dir = Some(Self::expand_path(cache, &vars)?);
            }
            if let Some(ref scripts) = floatctl.scripts_dir {
                floatctl.scripts_dir = Some(Self::expand_path(scripts, &vars)?);
            }
            if let Some(ref exports) = floatctl.conversation_exports {
                floatctl.conversation_exports = Some(Self::expand_path(exports, &vars)?);
            }
        }

        // Expand r2 config
        if let Some(ref mut r2) = self.r2 {
            r2.account_id = Self::expand_string(&r2.account_id, &vars);
            r2.api_token = Self::expand_string(&r2.api_token, &vars);
        }

        // Expand integrations
        if let Some(ref mut integrations) = self.integrations {
            if let Some(ref key) = integrations.cohere_api_key {
                integrations.cohere_api_key = Some(Self::expand_string(key, &vars));
            }
            if let Some(ref key) = integrations.openai_api_key {
                integrations.openai_api_key = Some(Self::expand_string(key, &vars));
            }
            if let Some(ref key) = integrations.anthropic_api_key {
                integrations.anthropic_api_key = Some(Self::expand_string(key, &vars));
            }
        }

        Ok(())
    }

    /// Expand ${var} references in a path
    fn expand_path(path: &PathBuf, vars: &HashMap<String, String>) -> Result<PathBuf> {
        let path_str = path.display().to_string();
        let expanded = Self::expand_string(&path_str, vars);
        Ok(PathBuf::from(expanded))
    }

    /// Expand ${var} references in a string
    fn expand_string(s: &str, vars: &HashMap<String, String>) -> String {
        let mut result = s.to_string();

        for (key, value) in vars {
            let pattern = format!("${{{}}}", key);
            result = result.replace(&pattern, value);
        }

        result
    }

    /// Validate all paths exist and are accessible
    pub fn validate_paths(&self) -> Result<()> {
        let paths = vec![
            ("float_home", &self.paths.float_home),
            ("daily_notes_home", &self.paths.daily_notes_home),
            ("daily_notes", &self.paths.daily_notes),
            ("bridges", &self.paths.bridges),
            ("operations", &self.paths.operations),
            ("inbox", &self.paths.inbox),
            ("dispatches", &self.paths.dispatches),
        ];

        let mut errors = Vec::new();

        for (name, path) in paths {
            if !path.exists() {
                errors.push(format!("  ✗ {}: {:?} (does not exist)", name, path));
            } else if !path.is_dir() {
                errors.push(format!("  ✗ {}: {:?} (not a directory)", name, path));
            }
        }

        if !errors.is_empty() {
            anyhow::bail!("Path validation failed:\n{}", errors.join("\n"));
        }

        Ok(())
    }

    /// Save config to file
    pub fn save(&self) -> Result<()> {
        let config_path = Self::config_path();

        // Create directory if it doesn't exist
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let toml_str = toml::to_string_pretty(self)
            .context("Failed to serialize config to TOML")?;

        fs::write(&config_path, toml_str)
            .context(format!("Failed to write config file: {:?}", config_path))?;

        Ok(())
    }
}

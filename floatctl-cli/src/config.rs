use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use floatctl_core::FloatConfig;

#[derive(Parser, Debug)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub command: ConfigCommands,
}

#[derive(Subcommand, Debug)]
pub enum ConfigCommands {
    /// Initialize config from detected environment
    Init(InitArgs),
    /// Get a config value by dot-notation key
    Get(GetArgs),
    /// List all config values
    List(ListArgs),
    /// Validate all paths and configuration
    Validate,
    /// Export config as environment variables
    Export,
    /// Show config file path
    Path,
}

#[derive(Parser, Debug)]
pub struct InitArgs {
    /// Detect paths from current environment
    #[arg(long)]
    pub detect: bool,

    /// Force overwrite existing config
    #[arg(long, short)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct GetArgs {
    /// Dot-notation key (e.g., "paths.float_home")
    pub key: String,

    /// Get value for specific machine
    #[arg(long)]
    pub machine: Option<String>,
}

#[derive(Parser, Debug)]
pub struct ListArgs {
    /// Show config for specific machine
    #[arg(long)]
    pub machine: Option<String>,
}

pub fn run_config(args: ConfigArgs) -> Result<()> {
    match args.command {
        ConfigCommands::Init(args) => run_init(args),
        ConfigCommands::Get(args) => run_get(args),
        ConfigCommands::List(args) => run_list(args),
        ConfigCommands::Validate => run_validate(),
        ConfigCommands::Export => run_export(),
        ConfigCommands::Path => run_path(),
    }
}

fn run_init(args: InitArgs) -> Result<()> {
    let config_path = FloatConfig::config_path();

    // Check if config already exists
    if config_path.exists() && !args.force {
        return Err(anyhow::anyhow!(
            "Config already exists at {:?}\n\nUse --force to overwrite",
            config_path
        ));
    }

    // Create config directory
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Copy template from repository to config location
    let template_content = include_str!("../../.floatctl-config.template.toml");

    let mut config_content = template_content.to_string();

    // If --detect, try to auto-detect current paths
    if args.detect {
        println!("🔍 Detecting current environment...");

        // Detect home directory
        if let Some(home) = dirs::home_dir() {
            let home_str = home.display().to_string();
            config_content = config_content.replace("/Users/evan", &home_str);
            println!("   ✓ Detected home: {}", home_str);
        }

        // Check if float-hub exists in typical locations
        if let Some(home) = dirs::home_dir() {
            let float_hub = home.join("float-hub");
            if float_hub.exists() {
                println!("   ✓ Found float-hub at: {}", float_hub.display());
            } else {
                println!("   ⚠ float-hub not found at: {}", float_hub.display());
                println!("     You'll need to update paths.float_home manually");
            }
        }
    }

    // Write config file
    std::fs::write(&config_path, config_content)
        .context(format!("Failed to write config file: {:?}", config_path))?;

    println!("✅ Created config at: {:?}", config_path);
    println!("\nNext steps:");
    println!("  1. Edit the config: $EDITOR {:?}", config_path);
    println!("  2. Update paths to match your setup");
    println!("  3. Run: floatctl config validate");

    Ok(())
}

fn run_get(args: GetArgs) -> Result<()> {
    // Load config with optional machine override (thread-safe parameter passing)
    let config = FloatConfig::load_with_machine(args.machine.as_deref())?;

    // Parse dot-notation key
    let value = get_config_value(&config, &args.key)?;

    println!("{}", value);

    Ok(())
}

fn run_list(args: ListArgs) -> Result<()> {
    // Load config with optional machine override (thread-safe parameter passing)
    let config = FloatConfig::load_with_machine(args.machine.as_deref())?;

    // Pretty-print config as TOML
    let toml_str = toml::to_string_pretty(&config)
        .context("Failed to serialize config to TOML")?;

    println!("{}", toml_str);

    Ok(())
}

fn run_validate() -> Result<()> {
    println!("🔍 Validating configuration...");

    let config = FloatConfig::load()?;

    println!("   ✓ Config loaded successfully");
    println!("   Machine: {} ({})", config.machine.name, config.machine.environment);

    // Validate paths
    match config.validate_paths() {
        Ok(_) => {
            println!("   ✓ All paths exist and are accessible");
        }
        Err(e) => {
            eprintln!("\n❌ Path validation failed:\n{}", e);
            std::process::exit(1);
        }
    }

    // Check for raw secrets (non-blocking warnings)
    let secret_warnings = config.validate_secrets();
    if !secret_warnings.is_empty() {
        println!("\n⚠️  Security warnings:");
        for warning in &secret_warnings {
            println!("   {}", warning);
        }
        println!("\n   Config should use environment variables for secrets.");
        println!("   See template: .floatctl-config.template.toml");
    }

    // Validate evna config (if present)
    if let Some(ref evna) = config.evna {
        if evna.database_url.is_empty() || evna.database_url.starts_with("${") {
            eprintln!("   ⚠  evna.database_url not set or using unresolved env var");
        } else {
            println!("   ✓ evna.database_url is set");
        }
    }

    println!("\n✅ Configuration valid!");

    Ok(())
}

fn run_export() -> Result<()> {
    let config = FloatConfig::load()?;

    // Export as shell environment variables
    println!("export FLOAT_HOME={:?}", config.paths.float_home);
    println!("export FLOAT_DAILY_NOTES={:?}", config.paths.daily_notes);
    println!("export FLOAT_DAILY_NOTES_HOME={:?}", config.paths.daily_notes_home);
    println!("export FLOAT_BRIDGES={:?}", config.paths.bridges);
    println!("export FLOAT_OPERATIONS={:?}", config.paths.operations);
    println!("export FLOAT_INBOX={:?}", config.paths.inbox);
    println!("export FLOAT_DISPATCHES={:?}", config.paths.dispatches);

    if let Some(ref archives) = config.paths.archives {
        println!("export FLOAT_ARCHIVES={:?}", archives);
    }

    // Export evna config
    if let Some(ref evna) = config.evna {
        if !evna.database_url.is_empty() {
            println!("export EVNA_DATABASE_URL={:?}", evna.database_url);
        }
        if let Some(port) = evna.mcp_server_port {
            println!("export EVNA_MCP_PORT={}", port);
        }
    }

    // Export floatctl config
    if let Some(ref floatctl) = config.floatctl {
        if let Some(ref scripts_dir) = floatctl.scripts_dir {
            println!("export FLOATCTL_SCRIPTS_DIR={:?}", scripts_dir);
        }
        if let Some(ref cache_dir) = floatctl.cache_dir {
            println!("export FLOATCTL_CACHE_DIR={:?}", cache_dir);
        }
    }

    Ok(())
}

fn run_path() -> Result<()> {
    println!("{}", FloatConfig::config_path().display());
    Ok(())
}

// Helper to get config value by dot-notation key
fn get_config_value(config: &FloatConfig, key: &str) -> Result<String> {
    let parts: Vec<&str> = key.split('.').collect();

    match (parts.first(), parts.get(1)) {
        (Some(&"paths"), Some(&"float_home")) => Ok(config.paths.float_home.display().to_string()),
        (Some(&"paths"), Some(&"daily_notes_home")) => Ok(config.paths.daily_notes_home.display().to_string()),
        (Some(&"paths"), Some(&"daily_notes")) => Ok(config.paths.daily_notes.display().to_string()),
        (Some(&"paths"), Some(&"bridges")) => Ok(config.paths.bridges.display().to_string()),
        (Some(&"paths"), Some(&"operations")) => Ok(config.paths.operations.display().to_string()),
        (Some(&"paths"), Some(&"inbox")) => Ok(config.paths.inbox.display().to_string()),
        (Some(&"paths"), Some(&"dispatches")) => Ok(config.paths.dispatches.display().to_string()),
        (Some(&"paths"), Some(&"archives")) => {
            config.paths.archives
                .as_ref()
                .map(|p| p.display().to_string())
                .ok_or_else(|| anyhow::anyhow!("paths.archives not set"))
        }
        (Some(&"machine"), Some(&"name")) => Ok(config.machine.name.clone()),
        (Some(&"machine"), Some(&"environment")) => Ok(config.machine.environment.clone()),
        (Some(&"evna"), Some(&"database_url")) => {
            config.evna
                .as_ref()
                .map(|e| e.database_url.clone())
                .ok_or_else(|| anyhow::anyhow!("evna config not set"))
        }
        (Some(&"evna"), Some(&"mcp_server_port")) => {
            config.evna
                .as_ref()
                .and_then(|e| e.mcp_server_port)
                .map(|p| p.to_string())
                .ok_or_else(|| anyhow::anyhow!("evna.mcp_server_port not set"))
        }
        _ => Err(anyhow::anyhow!("Unknown config key: {}", key)),
    }
}

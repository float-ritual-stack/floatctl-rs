//! EVNA MCP server management and cognitive tool commands
//!
//! Commands: install, uninstall, status, remote, boot, search, active, ask, agent, sessions

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};

// === Session Persistence ===

/// Persisted session state for --continue flag
#[derive(Debug, Serialize, Deserialize)]
struct LastEvnaSession {
    session_id: String,
    timestamp: String,
    query_preview: Option<String>,
}

/// JSON output from evna ask --json
#[derive(Debug, Deserialize)]
struct EvnaAskJsonResult {
    response: String,
    session_id: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    timed_out: bool,
}

fn get_session_state_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let state_dir = home.join(".floatctl").join("state");

    // Ensure state directory exists
    if !state_dir.exists() {
        fs::create_dir_all(&state_dir)
            .context("Failed to create ~/.floatctl/state directory")?;
    }

    Ok(state_dir.join("last-evna-session.json"))
}

fn load_last_session() -> Result<Option<LastEvnaSession>> {
    let path = get_session_state_path()?;

    if !path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .context("Failed to read last session file")?;

    let session: LastEvnaSession = serde_json::from_str(&content)
        .context("Failed to parse last session JSON")?;

    Ok(Some(session))
}

fn save_last_session(session_id: &str, query: Option<&str>) -> Result<()> {
    let path = get_session_state_path()?;

    let session = LastEvnaSession {
        session_id: session_id.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
        query_preview: query.map(|q| {
            // Truncate to first 100 chars for preview
            if q.len() > 100 {
                format!("{}...", &q[..97])
            } else {
                q.to_string()
            }
        }),
    };

    let content = serde_json::to_string_pretty(&session)
        .context("Failed to serialize session")?;

    fs::write(&path, content)
        .context("Failed to write last session file")?;

    Ok(())
}

// === Arg Structs (moved from main.rs for high cohesion) ===

#[derive(Parser, Debug)]
pub struct EvnaArgs {
    #[command(subcommand)]
    pub command: EvnaCommands,
}

#[derive(Subcommand, Debug)]
pub enum EvnaCommands {
    // === MCP Management Commands ===
    /// Install evna as MCP server in Claude Desktop
    Install(EvnaInstallArgs),
    /// Uninstall evna MCP server from Claude Desktop
    Uninstall,
    /// Show evna MCP server status
    Status,
    /// Start evna as remote MCP server (Supergateway + ngrok)
    Remote(EvnaRemoteArgs),

    // === Cognitive Tool Commands (shell out to evna binary) ===
    /// Brain boot - semantic search + active context + GitHub synthesis
    Boot(EvnaBootArgs),
    /// Deep semantic search across conversation history
    Search(EvnaSearchArgs),
    /// Query or capture recent activity stream
    Active(EvnaActiveArgs),
    /// LLM-orchestrated multi-tool search
    Ask(EvnaAskArgs),
    /// Conversational agent mode (Agent SDK)
    Agent(EvnaAgentArgs),
    /// Manage Claude Code session history
    Sessions(EvnaSessionsArgs),
}

#[derive(Parser, Debug)]
pub struct EvnaInstallArgs {
    /// Path to evna directory (defaults to ../evna relative to floatctl-rs)
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// Force reinstall even if already configured
    #[arg(long)]
    pub force: bool,
}

#[derive(Parser, Debug)]
pub struct EvnaRemoteArgs {
    /// Path to evna directory (defaults to ../evna relative to floatctl-rs)
    #[arg(long)]
    pub path: Option<PathBuf>,

    /// Port for Supergateway SSE server (default: 3100)
    #[arg(long, default_value = "3100")]
    pub port: u16,

    /// Skip ngrok tunnel (only start Supergateway)
    #[arg(long)]
    pub no_tunnel: bool,

    /// ngrok authtoken (reads from ~/.ngrok2/ngrok.yml if not provided)
    #[arg(long)]
    pub ngrok_token: Option<String>,

    /// ngrok domain (for paid accounts with reserved domains)
    #[arg(long)]
    pub ngrok_domain: Option<String>,
}

// === Cognitive Tool Args (pass-through to evna binary) ===

#[derive(Parser, Debug)]
pub struct EvnaBootArgs {
    /// Natural language query describing what context to retrieve (or read from stdin if omitted)
    pub query: Option<String>,

    /// Filter by project name
    #[arg(long)]
    pub project: Option<String>,

    /// Lookback days (default: 7)
    #[arg(long)]
    pub days: Option<u32>,

    /// Maximum results (default: 10)
    #[arg(long)]
    pub limit: Option<u32>,

    /// GitHub username for PR/issue status
    #[arg(long)]
    pub github: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Minimal output
    #[arg(long)]
    pub quiet: bool,
}

#[derive(Parser, Debug)]
pub struct EvnaSearchArgs {
    /// Search query (natural language, question, or keywords; or read from stdin if omitted)
    pub query: Option<String>,

    /// Filter by project name
    #[arg(long)]
    pub project: Option<String>,

    /// Maximum results (default: 10)
    #[arg(long)]
    pub limit: Option<u32>,

    /// Similarity threshold 0-1 (default: 0.5)
    #[arg(long)]
    pub threshold: Option<f32>,

    /// Filter by timestamp (ISO 8601)
    #[arg(long)]
    pub since: Option<String>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Minimal output
    #[arg(long)]
    pub quiet: bool,
}

#[derive(Parser, Debug)]
pub struct EvnaActiveArgs {
    /// Query to filter active context (optional if --capture used)
    pub query: Option<String>,

    /// Capture message to active context stream
    #[arg(long)]
    pub capture: bool,

    /// Filter by project name
    #[arg(long)]
    pub project: Option<String>,

    /// Maximum results (default: 10)
    #[arg(long)]
    pub limit: Option<u32>,

    /// Client type filter (desktop or claude_code)
    #[arg(long)]
    pub client: Option<String>,

    /// Exclude cross-client context
    #[arg(long)]
    pub no_cross_client: bool,

    /// Disable Ollama synthesis (return raw format)
    #[arg(long)]
    pub no_synthesize: bool,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Minimal output
    #[arg(long)]
    pub quiet: bool,
}

#[derive(Parser, Debug, Clone)]
pub struct EvnaAskArgs {
    /// Natural language query for LLM orchestrator (or read from stdin if omitted)
    pub query: Option<String>,

    /// Resume session by ID
    #[arg(long)]
    pub session: Option<String>,

    /// Continue last session (auto-loads most recent session ID)
    #[arg(long, short = 'c', conflicts_with = "session")]
    pub continue_session: bool,

    /// Fork existing session
    #[arg(long)]
    pub fork: bool,

    /// Timeout in milliseconds
    #[arg(long)]
    pub timeout: Option<u32>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,

    /// Minimal output
    #[arg(long)]
    pub quiet: bool,
}

#[derive(Parser, Debug)]
pub struct EvnaAgentArgs {
    /// Natural language query for conversational agent (or read from stdin if omitted)
    pub query: Option<String>,

    /// Resume session by ID
    #[arg(long)]
    pub session: Option<String>,

    /// Claude model to use
    #[arg(long)]
    pub model: Option<String>,

    /// Maximum agent turns
    #[arg(long)]
    pub max_turns: Option<u32>,

    /// Show detailed agent reasoning and tool calls
    #[arg(long)]
    pub verbose: bool,

    /// Disable streaming
    #[arg(long)]
    pub no_stream: bool,

    /// Save session for later resume
    #[arg(long)]
    pub save_session: bool,

    /// Minimal output
    #[arg(long)]
    pub quiet: bool,
}

#[derive(Parser, Debug)]
pub struct EvnaSessionsArgs {
    /// Subcommand (list or read)
    #[arg(default_value = "list")]
    pub subcommand: String,

    /// Session ID (for 'read' subcommand)
    pub session_id: Option<String>,

    /// Number of sessions to list (default: 10)
    #[arg(long, short = 'n')]
    pub n: Option<u32>,

    /// Filter by project
    #[arg(long)]
    pub project: Option<String>,

    /// First N messages from session
    #[arg(long)]
    pub first: Option<u32>,

    /// Last N messages from session
    #[arg(long)]
    pub last: Option<u32>,

    /// Truncate long messages (chars)
    #[arg(long)]
    pub truncate: Option<u32>,

    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

// === Command Implementations ===

pub async fn run_evna(args: EvnaArgs) -> Result<()> {
    match args.command {
        // MCP Management
        EvnaCommands::Install(install_args) => evna_install(install_args).await?,
        EvnaCommands::Uninstall => evna_uninstall().await?,
        EvnaCommands::Status => evna_status().await?,
        EvnaCommands::Remote(remote_args) => evna_remote(remote_args).await?,

        // Cognitive Tools (shell out to evna binary)
        EvnaCommands::Boot(boot_args) => evna_boot(boot_args).await?,
        EvnaCommands::Search(search_args) => evna_search(search_args).await?,
        EvnaCommands::Active(active_args) => evna_active(active_args).await?,
        EvnaCommands::Ask(ask_args) => evna_ask(ask_args).await?,
        EvnaCommands::Agent(agent_args) => evna_agent(agent_args).await?,
        EvnaCommands::Sessions(sessions_args) => evna_sessions(sessions_args).await?,
    }
    Ok(())
}

async fn evna_install(args: EvnaInstallArgs) -> Result<()> {
    use serde_json::{json, Value};
    use std::fs;

    // Determine evna path
    let evna_path = if let Some(path) = args.path {
        path
    } else {
        // Try common locations in order
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let candidates = vec![
            home.join("float-hub-operations").join("floatctl-rs").join("evna"),
            home.join("float-hub-operations").join("evna"),
            home.join(".floatctl").join("evna"),
        ];

        candidates.into_iter()
            .find(|p| p.exists())
            .ok_or_else(|| anyhow!(
                "evna directory not found in common locations:\n\
                 - ~/float-hub-operations/floatctl-rs/evna\n\
                 - ~/float-hub-operations/evna\n\
                 - ~/.floatctl/evna\n\
                 \n\
                 Use --path to specify a custom location"
            ))?
    };

    // Validate evna directory exists
    if !evna_path.exists() {
        return Err(anyhow!(
            "evna directory not found at: {}\nUse --path to specify location",
            evna_path.display()
        ));
    }

    // Check for package.json to confirm it's evna
    let package_json = evna_path.join("package.json");
    if !package_json.exists() {
        return Err(anyhow!(
            "Not a valid evna directory (missing package.json): {}",
            evna_path.display()
        ));
    }

    // Get Claude Desktop config path
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let config_path = home
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");

    // Read existing config or create new one
    let mut config: Value = if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .context("Failed to read Claude Desktop config")?;
        serde_json::from_str(&content)
            .context("Failed to parse Claude Desktop config JSON")?
    } else {
        json!({})
    };

    // Check if evna is already configured
    if let Some(mcp_servers) = config.get("mcpServers") {
        if let Some(evna) = mcp_servers.get("evna") {
            if !args.force {
                println!("✅ evna is already configured in Claude Desktop");
                println!("   Config: {}", serde_json::to_string_pretty(&evna)?);
                println!("\nUse --force to reinstall");
                return Ok(());
            } else {
                println!("🔄 Reinstalling evna (--force specified)");
            }
        }
    }

    // Get absolute path for config
    let evna_path_absolute = evna_path.canonicalize()
        .context("Failed to resolve evna absolute path")?;

    // Create evna MCP server configuration
    let evna_config = json!({
        "command": "bun",
        "args": ["run", "mcp-server"],
        "cwd": evna_path_absolute.to_string_lossy(),
        "env": {
            "NODE_ENV": "production"
        }
    });

    // Ensure mcpServers object exists
    if !config.is_object() {
        config = json!({});
    }
    if config.get("mcpServers").is_none() {
        config["mcpServers"] = json!({});
    }

    // Add evna configuration
    config["mcpServers"]["evna"] = evna_config;

    // Write config back
    let config_dir = config_path.parent().unwrap();
    fs::create_dir_all(config_dir)?;

    let config_json = serde_json::to_string_pretty(&config)?;
    fs::write(&config_path, config_json)
        .context("Failed to write Claude Desktop config")?;

    println!("✅ Successfully installed evna MCP server!");
    println!("   Location: {}", evna_path_absolute.display());
    println!("   Config: {}", config_path.display());
    println!("\n📝 Next steps:");
    println!("   1. Ensure .env is configured in evna directory");
    println!("   2. Restart Claude Desktop to load the MCP server");
    println!("   3. Test with: 'Use the brain_boot tool to search for...'");

    Ok(())
}

async fn evna_uninstall() -> Result<()> {
    use serde_json::Value;
    use std::fs;

    let home = dirs::home_dir().context("Could not determine home directory")?;
    let config_path = home
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");

    if !config_path.exists() {
        println!("ℹ️  Claude Desktop config not found - nothing to uninstall");
        return Ok(());
    }

    let content = fs::read_to_string(&config_path)
        .context("Failed to read Claude Desktop config")?;
    let mut config: Value = serde_json::from_str(&content)
        .context("Failed to parse Claude Desktop config JSON")?;

    // Check if evna exists
    if let Some(mcp_servers) = config.get_mut("mcpServers") {
        if let Some(obj) = mcp_servers.as_object_mut() {
            if obj.remove("evna").is_some() {
                let config_json = serde_json::to_string_pretty(&config)?;
                fs::write(&config_path, config_json)?;
                println!("✅ Successfully uninstalled evna MCP server");
                println!("   Restart Claude Desktop to apply changes");
                return Ok(());
            }
        }
    }

    println!("ℹ️  evna is not configured - nothing to uninstall");
    Ok(())
}

async fn evna_status() -> Result<()> {
    use serde_json::Value;
    use std::fs;

    let home = dirs::home_dir().context("Could not determine home directory")?;
    let config_path = home
        .join("Library")
        .join("Application Support")
        .join("Claude")
        .join("claude_desktop_config.json");

    if !config_path.exists() {
        println!("❌ Claude Desktop config not found");
        println!("   Expected: {}", config_path.display());
        return Ok(());
    }

    let content = fs::read_to_string(&config_path)
        .context("Failed to read Claude Desktop config")?;
    let config: Value = serde_json::from_str(&content)
        .context("Failed to parse Claude Desktop config JSON")?;

    // Check if evna is configured
    if let Some(mcp_servers) = config.get("mcpServers") {
        if let Some(evna) = mcp_servers.get("evna") {
            println!("✅ evna MCP server is configured");
            println!("\n📋 Configuration:");
            println!("{}", serde_json::to_string_pretty(&evna)?);

            // Validate the path exists
            if let Some(cwd) = evna.get("cwd").and_then(|v| v.as_str()) {
                let evna_path = PathBuf::from(cwd);
                if evna_path.exists() {
                    println!("\n✅ evna directory exists: {}", evna_path.display());

                    // Check for .env file
                    let env_file = evna_path.join(".env");
                    if env_file.exists() {
                        println!("✅ .env file found");
                    } else {
                        println!("⚠️  .env file not found - configure before using");
                    }
                } else {
                    println!("\n❌ evna directory not found: {}", evna_path.display());
                }
            }

            return Ok(());
        }
    }

    println!("❌ evna is not configured");
    println!("   Run: floatctl evna install");
    Ok(())
}

/// Kill any process listening on the specified port
fn kill_process_on_port(port: u16) -> Result<()> {
    use std::process::Command;

    // Use lsof to find process ID on port
    let output = Command::new("lsof")
        .arg("-ti")
        .arg(format!(":{}", port))
        .output()?;

    if !output.status.success() {
        // No process found on port, or lsof failed
        return Ok(());
    }

    let pid_str = String::from_utf8_lossy(&output.stdout);
    let pid = pid_str.trim();

    if pid.is_empty() {
        // No process found
        return Ok(());
    }

    // Kill the process
    let kill_status = Command::new("kill")
        .arg(pid)
        .status()?;

    if !kill_status.success() {
        return Err(anyhow!("Failed to kill process {} on port {}", pid, port));
    }

    // Give it a moment to die
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}

/// Kill ngrok processes tunneling the specified port
fn kill_ngrok_for_port(port: u16) -> Result<()> {
    use std::process::Command;

    // Find all ngrok processes
    let output = Command::new("pgrep")
        .arg("-f")
        .arg(format!("ngrok.*{}", port))
        .output()?;

    if !output.status.success() {
        // No ngrok processes found for this port
        return Ok(());
    }

    let pids_str = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<&str> = pids_str.trim().split('\n').filter(|p| !p.is_empty()).collect();

    if pids.is_empty() {
        return Ok(());
    }

    // Kill ngrok processes for this port
    for pid in pids {
        Command::new("kill")
            .arg(pid)
            .status()
            .ok(); // Ignore errors, process might already be dead
    }

    // Give them a moment to die
    std::thread::sleep(std::time::Duration::from_millis(500));

    Ok(())
}

async fn evna_remote(args: EvnaRemoteArgs) -> Result<()> {
    use std::process::{Command, Stdio};

    // Determine evna path
    let evna_path = if let Some(path) = args.path {
        path
    } else {
        // Try common locations in order
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let candidates = vec![
            home.join("float-hub-operations").join("floatctl-rs").join("evna"),
            home.join("float-hub-operations").join("evna"),
            home.join(".floatctl").join("evna"),
        ];

        candidates.into_iter()
            .find(|p| p.exists())
            .ok_or_else(|| anyhow!(
                "evna directory not found in common locations:\n\
                 - ~/float-hub-operations/floatctl-rs/evna\n\
                 - ~/float-hub-operations/evna\n\
                 - ~/.floatctl/evna\n\
                 \n\
                 Use --path to specify a custom location"
            ))?
    };

    if !evna_path.exists() {
        return Err(anyhow!(
            "evna directory not found: {}\nUse --path to specify location",
            evna_path.display()
        ));
    }

    // Load .env from evna directory
    let env_file = evna_path.join(".env");
    if env_file.exists() {
        dotenvy::from_path(&env_file).ok(); // Load but don't fail if parsing errors
    }

    // Check dependencies
    println!("🔍 Checking dependencies...");

    // Check Supergateway
    let supergateway_check = Command::new("supergateway")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if supergateway_check.is_err() || !supergateway_check.unwrap().success() {
        return Err(anyhow!(
            "Supergateway not found. Install with:\n  npm install -g supergateway"
        ));
    }
    println!("✅ Supergateway found");

    // Check bun
    let bun_check = Command::new("bun")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();

    if bun_check.is_err() || !bun_check.unwrap().success() {
        return Err(anyhow!(
            "bun not found. Install with:\n  curl -fsSL https://bun.sh/install | bash"
        ));
    }
    println!("✅ bun found");

    // Check ngrok (unless --no-tunnel)
    if !args.no_tunnel {
        let ngrok_check = Command::new("ngrok")
            .arg("version")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();

        if ngrok_check.is_err() || !ngrok_check.unwrap().success() {
            return Err(anyhow!(
                "ngrok not found. Install from https://ngrok.com/download\nOr use --no-tunnel to skip"
            ));
        }
        println!("✅ ngrok found");

        // Check for ngrok authtoken
        if std::env::var("EVNA_NGROK_AUTHTOKEN").is_err()
            && std::env::var("NGROK_AUTHTOKEN").is_err()
            && args.ngrok_token.is_none()
        {
            println!("⚠️  Warning: No ngrok authtoken configured");
            println!("   Set EVNA_NGROK_AUTHTOKEN in .env or pass --ngrok-token");
            println!("   Get authtoken from: https://dashboard.ngrok.com/get-started/your-authtoken");
            println!();
        }
    }

    println!();
    println!("🚀 Starting EVNA remote MCP server");
    println!("   Directory: {}", evna_path.display());
    println!("   Port: {}", args.port);
    println!("   Transport: stdio → SSE");
    if !args.no_tunnel {
        println!("   Tunnel: ngrok");
    }
    println!();

    // Kill any existing process on the port
    println!("🧹 Checking for existing process on port {}...", args.port);
    if let Err(e) = kill_process_on_port(args.port) {
        println!("   ⚠️  Warning: Could not check/kill existing process: {}", e);
    } else {
        println!("   ✅ Port {} is clear", args.port);
    }
    println!();

    // Start Supergateway in background
    println!("📡 Starting Supergateway on port {}...", args.port);

    // Build PATH with common binary locations
    let home = dirs::home_dir()
        .context("Could not determine home directory for PATH/FLOATCTL_BIN setup")?;
    let home = home
        .to_str()
        .ok_or_else(|| anyhow!("Home directory path contains invalid UTF-8"))?
        .to_string();
    let path_dirs = vec![
        format!("{}/.cargo/bin", home),
        format!("{}/.bun/bin", home),
        "/opt/homebrew/bin".to_string(),
        "/usr/local/bin".to_string(),
        "/usr/bin".to_string(),
        "/bin".to_string(),
    ];
    let path_env = path_dirs.join(":");

    let mut supergateway_cmd = Command::new("supergateway");
    supergateway_cmd
        .arg("--stdio")
        .arg("bun run --silent mcp-server")
        .arg("--port")
        .arg(args.port.to_string())
        .current_dir(&evna_path)
        .env("PATH", &path_env)
        .env("FLOATCTL_BIN", format!("{}/.cargo/bin/floatctl", home))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut supergateway_process = supergateway_cmd
        .spawn()
        .context("Failed to start Supergateway")?;

    // Give Supergateway time to start
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Check if process is still running
    match supergateway_process.try_wait() {
        Ok(Some(status)) => {
            return Err(anyhow!(
                "Supergateway exited immediately with status: {}",
                status
            ));
        }
        Ok(None) => {
            println!("✅ Supergateway running");
            println!("   Local URL: http://localhost:{}/sse", args.port);
        }
        Err(e) => {
            return Err(anyhow!("Failed to check Supergateway status: {}", e));
        }
    }

    // Start ngrok tunnel (unless --no-tunnel)
    let mut ngrok_process = None;
    if !args.no_tunnel {
        // Kill any existing ngrok processes for this port
        println!();
        println!("🧹 Cleaning up ngrok for port {}...", args.port);
        if let Err(e) = kill_ngrok_for_port(args.port) {
            println!("   ⚠️  Warning: Could not kill ngrok: {}", e);
        } else {
            println!("   ✅ ngrok cleared");
        }

        println!();
        println!("🌐 Starting ngrok tunnel...");

        let mut ngrok_cmd = Command::new("ngrok");
        ngrok_cmd.arg("http").arg(args.port.to_string());

        // Priority: CLI arg > EVNA_NGROK_* env var > NGROK_* env var (fallback)
        if let Some(token) = args.ngrok_token
            .or_else(|| std::env::var("EVNA_NGROK_AUTHTOKEN").ok())
            .or_else(|| std::env::var("NGROK_AUTHTOKEN").ok())
        {
            ngrok_cmd.arg("--authtoken").arg(token);
        }

        // Reserved domain (CLI arg > EVNA_NGROK_DOMAIN env var)
        let domain = args.ngrok_domain
            .or_else(|| std::env::var("EVNA_NGROK_DOMAIN").ok());
        if let Some(domain) = domain.as_ref() {
            ngrok_cmd.arg("--domain").arg(domain);
            println!("   Using reserved domain: {}", domain);
        }

        // Basic auth (from env var only - too sensitive for CLI)
        if let Ok(auth) = std::env::var("EVNA_NGROK_AUTH") {
            ngrok_cmd.arg("--basic-auth").arg(auth);
            println!("   Basic auth enabled (from .env)");
        }

        ngrok_cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut ngrok = ngrok_cmd.spawn().context("Failed to start ngrok")?;

        // Give ngrok time to establish tunnel
        tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

        // Check if ngrok is still running
        match ngrok.try_wait() {
            Ok(Some(status)) => {
                // Kill Supergateway before returning
                let _ = supergateway_process.kill();
                return Err(anyhow!("ngrok exited with status: {}", status));
            }
            Ok(None) => {
                println!("✅ ngrok tunnel established");
                println!();

                // Build authenticated URL if we have domain + auth
                let auth_url = if let Some(domain) = domain.as_ref() {
                    if let Ok(auth) = std::env::var("EVNA_NGROK_AUTH") {
                        // Format: https://username:password@domain/sse
                        format!("https://{}@{}/sse", auth, domain)
                    } else {
                        // No auth: https://domain/sse
                        format!("https://{}/sse", domain)
                    }
                } else {
                    String::new()
                };

                // Copy to clipboard if we have a complete URL
                if !auth_url.is_empty() {
                    use cli_clipboard::{ClipboardContext, ClipboardProvider};
                    if let Ok(mut ctx) = ClipboardContext::new() {
                        if ctx.set_contents(auth_url.clone()).is_ok() {
                            println!("📋 Copied to clipboard: {}", auth_url);
                        }
                    }
                }

                // Show URL based on whether we have a reserved domain
                if let Some(domain) = domain {
                    println!("🎯 Public URL: https://{}/sse", domain);
                    println!();

                    // Check if we have auth credentials
                    if let Ok(auth) = std::env::var("EVNA_NGROK_AUTH") {
                        println!("   (URL with auth credentials copied to clipboard)");
                        println!();

                        // Show both config formats
                        println!("📋 Claude Desktop config (URL auth):");
                        println!(r#"   {{
     "mcpServers": {{
       "evna-remote": {{
         "url": "https://{}@{}/sse",
         "transport": "sse"
       }}
     }}
   }}"#, auth, domain);
                        println!();

                        // Base64 encode for Authorization header
                        use base64::{Engine as _, engine::general_purpose};
                        let encoded = general_purpose::STANDARD.encode(&auth);

                        println!("📋 Claude Code config (header auth):");
                        println!(r#"   {{
     "mcpServers": {{
       "evna-remote": {{
         "url": "https://{}/sse",
         "transport": "sse",
         "headers": {{
           "Authorization": "Basic {}"
         }}
       }}
     }}
   }}"#, domain, encoded);
                        println!();

                        println!("💻 Claude Code CLI command:");
                        println!(r#"   claude mcp add evna-remote https://{}/sse --transport sse --header "Authorization: Basic {}""#, domain, encoded);
                    } else {
                        // No auth
                        println!("📋 Claude Desktop config:");
                        println!(r#"   {{
     "mcpServers": {{
       "evna-remote": {{
         "url": "https://{}/sse",
         "transport": "sse"
       }}
     }}
   }}"#, domain);
                    }
                } else {
                    println!("🎯 Public URL: Check http://localhost:4040 for ngrok URL");
                    println!("   (ngrok web UI shows the public HTTPS URL)");
                    println!();
                    println!("📋 Claude Desktop config:");
                    println!(r#"   {{
     "mcpServers": {{
       "evna-remote": {{
         "url": "https://YOUR-NGROK-URL.ngrok-free.app/sse",
         "transport": "sse"
       }}
     }}
   }}"#);
                }
                ngrok_process = Some(ngrok);
            }
            Err(e) => {
                let _ = supergateway_process.kill();
                return Err(anyhow!("Failed to check ngrok status: {}", e));
            }
        }
    }

    println!();
    println!("✨ EVNA remote MCP server is online!");
    println!("   Press Ctrl+C to stop");
    println!();

    // Wait for Ctrl+C
    tokio::signal::ctrl_c().await?;

    println!();
    println!("🛑 Shutting down...");

    // Kill processes
    if let Some(mut ngrok) = ngrok_process {
        let _ = ngrok.kill();
        println!("✅ ngrok stopped");
    }

    let _ = supergateway_process.kill();
    println!("✅ Supergateway stopped");

    println!("👋 EVNA remote MCP server stopped");

    Ok(())
}

// === Cognitive Tool Handlers (shell out to evna binary) ===

async fn evna_boot(args: EvnaBootArgs) -> Result<()> {
    let mut cmd_args = vec!["boot".to_string()];

    // Add query if provided (otherwise evna will read from stdin)
    if let Some(query) = args.query {
        cmd_args.push(query);
    }

    if let Some(project) = args.project {
        cmd_args.extend(["--project".to_string(), project]);
    }
    if let Some(days) = args.days {
        cmd_args.extend(["--days".to_string(), days.to_string()]);
    }
    if let Some(limit) = args.limit {
        cmd_args.extend(["--limit".to_string(), limit.to_string()]);
    }
    if let Some(github) = args.github {
        cmd_args.extend(["--github".to_string(), github]);
    }
    if args.json {
        cmd_args.push("--json".to_string());
    }
    if args.quiet {
        cmd_args.push("--quiet".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

async fn evna_search(args: EvnaSearchArgs) -> Result<()> {
    let mut cmd_args = vec!["search".to_string()];

    // Add query if provided (otherwise evna will read from stdin)
    if let Some(query) = args.query {
        cmd_args.push(query);
    }

    if let Some(project) = args.project {
        cmd_args.extend(["--project".to_string(), project]);
    }
    if let Some(limit) = args.limit {
        cmd_args.extend(["--limit".to_string(), limit.to_string()]);
    }
    if let Some(threshold) = args.threshold {
        cmd_args.extend(["--threshold".to_string(), threshold.to_string()]);
    }
    if let Some(since) = args.since {
        cmd_args.extend(["--since".to_string(), since]);
    }
    if args.json {
        cmd_args.push("--json".to_string());
    }
    if args.quiet {
        cmd_args.push("--quiet".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

async fn evna_active(args: EvnaActiveArgs) -> Result<()> {
    let mut cmd_args = vec!["active".to_string()];

    if let Some(query) = args.query {
        cmd_args.push(query);
    }

    if args.capture {
        cmd_args.push("--capture".to_string());
    }
    if let Some(project) = args.project {
        cmd_args.extend(["--project".to_string(), project]);
    }
    if let Some(limit) = args.limit {
        cmd_args.extend(["--limit".to_string(), limit.to_string()]);
    }
    if let Some(client) = args.client {
        cmd_args.extend(["--client".to_string(), client]);
    }
    if args.no_cross_client {
        cmd_args.push("--no-cross-client".to_string());
    }
    if args.no_synthesize {
        cmd_args.push("--no-synthesize".to_string());
    }
    if args.json {
        cmd_args.push("--json".to_string());
    }
    if args.quiet {
        cmd_args.push("--quiet".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

/// Execute evna ask command with session persistence
pub async fn evna_ask(args: EvnaAskArgs) -> Result<()> {
    let mut cmd_args = vec!["ask".to_string()];

    // Clone query for later use in session persistence
    let query_for_session = args.query.clone();

    // Add query if provided (otherwise evna will read from stdin)
    if let Some(query) = args.query.clone() {
        cmd_args.push(query);
    }

    // Handle --continue flag: load last session ID
    let effective_session = if args.continue_session {
        match load_last_session() {
            Ok(Some(last)) => {
                eprintln!("\x1b[90m   Resuming session: {}\x1b[0m", last.session_id);
                Some(last.session_id)
            }
            Ok(None) => {
                eprintln!("\x1b[33m   No previous session found, starting fresh\x1b[0m");
                None
            }
            Err(e) => {
                eprintln!("\x1b[33m   Warning: couldn't load last session: {}\x1b[0m", e);
                None
            }
        }
    } else {
        args.session.clone()
    };

    if let Some(session) = effective_session.clone() {
        cmd_args.extend(["--session".to_string(), session]);
    }
    if args.fork {
        cmd_args.push("--fork".to_string());
    }
    if let Some(timeout) = args.timeout {
        cmd_args.extend(["--timeout".to_string(), timeout.to_string()]);
    }

    // Always use JSON internally to capture session_id, unless user requested quiet
    let use_internal_json = !args.quiet;

    if use_internal_json || args.json {
        cmd_args.push("--json".to_string());
    }
    if args.quiet {
        cmd_args.push("--quiet".to_string());
    }

    // If we need to capture session_id, use the capture variant
    if use_internal_json && !args.json {
        return evna_ask_with_capture(&cmd_args, query_for_session.as_deref(), args.quiet).await;
    }

    // Otherwise pass through normally
    shell_out_to_evna(&cmd_args).await
}

/// Execute evna ask with JSON capture for session persistence
async fn evna_ask_with_capture(cmd_args: &[String], query: Option<&str>, quiet: bool) -> Result<()> {
    // Find evna binary
    let evna_bin = which::which("evna").ok().or_else(|| {
        let home = dirs::home_dir()?;
        let candidates = vec![
            home.join("float-hub-operations/floatctl-rs/evna/bin/evna"),
            home.join("float-hub-operations/evna/bin/evna"),
            home.join(".floatctl/evna/bin/evna"),
            home.join(".local/bin/evna"),
        ];
        candidates.into_iter().find(|p| p.exists())
    }).context(
        "evna binary not found. Install with:\n\
         1. cd evna\n\
         2. bun install\n\
         3. chmod +x bin/evna\n\
         4. ln -s $(pwd)/bin/evna ~/.local/bin/evna"
    )?;

    // Spawn evna with captured stdout (for JSON parsing) and pass-through stderr
    let mut child = Command::new(&evna_bin)
        .args(cmd_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .context(format!("Failed to spawn evna binary: {}", evna_bin.display()))?;

    // Collect stdout
    let stdout = child.stdout.take().context("Failed to capture stdout")?;
    let reader = BufReader::new(stdout);
    let mut output = String::new();

    for line in reader.lines() {
        let line = line.context("Failed to read line from evna output")?;
        output.push_str(&line);
        output.push('\n');
    }

    let status = child.wait().context("Failed to wait for evna process")?;

    if !status.success() {
        // Still try to parse any output before failing
        if let Ok(result) = serde_json::from_str::<EvnaAskJsonResult>(&output) {
            if let Some(session_id) = result.session_id {
                let _ = save_last_session(&session_id, query);
            }
            if !quiet {
                println!("{}", result.response);
            }
        }
        std::process::exit(status.code().unwrap_or(1));
    }

    // Parse JSON output
    match serde_json::from_str::<EvnaAskJsonResult>(&output) {
        Ok(result) => {
            // Save session_id for --continue
            if let Some(ref session_id) = result.session_id {
                if let Err(e) = save_last_session(session_id, query) {
                    eprintln!("\x1b[33m   Warning: couldn't save session: {}\x1b[0m", e);
                }
            }

            // Print response (not JSON) to user
            if !quiet {
                println!("{}", result.response);

                // Print continuation hint
                if let Some(ref session_id) = result.session_id {
                    eprintln!();
                    eprintln!("\x1b[90m💾 Session: {}\x1b[0m", session_id);
                    eprintln!("\x1b[90m   Resume with: \x1b[36mfloatctl ask evna -c\x1b[90m \x1b[33m\"follow up\"\x1b[0m");
                }
            }
        }
        Err(e) => {
            // JSON parsing failed, just print raw output
            eprintln!("\x1b[33m   Warning: couldn't parse evna response as JSON: {}\x1b[0m", e);
            print!("{}", output);
        }
    }

    Ok(())
}

async fn evna_agent(args: EvnaAgentArgs) -> Result<()> {
    let mut cmd_args = vec!["agent".to_string()];

    // Add query if provided (otherwise evna will read from stdin)
    if let Some(query) = args.query {
        cmd_args.push(query);
    }

    if let Some(session) = args.session {
        cmd_args.extend(["--session".to_string(), session]);
    }
    if let Some(model) = args.model {
        cmd_args.extend(["--model".to_string(), model]);
    }
    if let Some(max_turns) = args.max_turns {
        cmd_args.extend(["--max-turns".to_string(), max_turns.to_string()]);
    }
    if args.verbose {
        cmd_args.push("--verbose".to_string());
    }
    if args.no_stream {
        cmd_args.push("--no-stream".to_string());
    }
    if args.save_session {
        cmd_args.push("--save-session".to_string());
    }
    if args.quiet {
        cmd_args.push("--quiet".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

async fn evna_sessions(args: EvnaSessionsArgs) -> Result<()> {
    let mut cmd_args = vec!["sessions".to_string(), args.subcommand];

    if let Some(session_id) = args.session_id {
        cmd_args.push(session_id);
    }

    if let Some(n) = args.n {
        cmd_args.extend(["--n".to_string(), n.to_string()]);
    }
    if let Some(project) = args.project {
        cmd_args.extend(["--project".to_string(), project]);
    }
    if let Some(first) = args.first {
        cmd_args.extend(["--first".to_string(), first.to_string()]);
    }
    if let Some(last) = args.last {
        cmd_args.extend(["--last".to_string(), last.to_string()]);
    }
    if let Some(truncate) = args.truncate {
        cmd_args.extend(["--truncate".to_string(), truncate.to_string()]);
    }
    if args.json {
        cmd_args.push("--json".to_string());
    }

    shell_out_to_evna(&cmd_args).await
}

/// Shell out to evna binary and pass through output
async fn shell_out_to_evna(args: &[String]) -> Result<()> {
    use std::process::Command;

    // Try to find evna binary in PATH first, fall back to common locations
    let evna_bin = which::which("evna").ok().or_else(|| {
        let home = dirs::home_dir()?;
        let candidates = vec![
            home.join("float-hub-operations/floatctl-rs/evna/bin/evna"),
            home.join("float-hub-operations/evna/bin/evna"),
            home.join(".floatctl/evna/bin/evna"),
            home.join(".local/bin/evna"),
        ];

        candidates.into_iter().find(|p| p.exists())
    }).context(
        "evna binary not found. Install with:\n\
         1. cd evna\n\
         2. bun install\n\
         3. chmod +x bin/evna\n\
         4. ln -s $(pwd)/bin/evna ~/.local/bin/evna"
    )?;

    // Execute evna with pass-through args (inherit stdio for user visibility)
    let status = Command::new(&evna_bin)
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()
        .context(format!("Failed to execute evna binary: {}", evna_bin.display()))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

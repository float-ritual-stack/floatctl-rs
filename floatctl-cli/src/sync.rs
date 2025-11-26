use anyhow::{Context, Result};
use chrono::TimeZone;
use chrono_tz::America::Toronto;
use clap::{Parser, Subcommand, ValueEnum};
use floatctl_core::SyncEvent;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{BufRead, BufReader};
use std::process::Command;

// Daemon startup/shutdown delay (milliseconds)
const DAEMON_OPERATION_DELAY_MS: u64 = 1000;

#[derive(Parser, Debug)]
pub struct SyncArgs {
    #[command(subcommand)]
    pub command: SyncCommands,
}

#[derive(Subcommand, Debug)]
pub enum SyncCommands {
    /// Check R2 sync daemon status
    Status(SyncStatusArgs),
    /// Manually trigger a sync
    Trigger(SyncTriggerArgs),
    /// Start sync daemon(s)
    Start(SyncStartArgs),
    /// Stop sync daemon(s)
    Stop(SyncStopArgs),
    /// View sync logs
    Logs(SyncLogsArgs),
    /// Install/update sync scripts to ~/.floatctl/
    Install(SyncInstallArgs),
}

#[derive(Parser, Debug)]
pub struct SyncStatusArgs {
    /// Which daemon to check (daily, dispatch, projects, or all)
    #[arg(long, value_enum, default_value = "all")]
    pub daemon: DaemonType,

    /// Output format (text or json)
    #[arg(long, value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// Check remote float-box status via SSH (systemd services)
    #[arg(long)]
    pub remote: bool,

    /// Remote host (default: float-box)
    #[arg(long, default_value = "float-box")]
    pub host: String,
}

#[derive(Parser, Debug)]
pub struct SyncTriggerArgs {
    /// Which daemon to trigger (daily, dispatch, or all)
    #[arg(long, value_enum, default_value = "all")]
    pub daemon: DaemonType,

    /// Wait for sync to complete before returning
    #[arg(long)]
    pub wait: bool,
}

#[derive(Parser, Debug)]
pub struct SyncStartArgs {
    /// Which daemon to start (daily, dispatch, or all)
    #[arg(long, value_enum, default_value = "all")]
    pub daemon: DaemonType,
}

#[derive(Parser, Debug)]
pub struct SyncStopArgs {
    /// Which daemon to stop (daily, dispatch, or all)
    #[arg(long, value_enum, default_value = "all")]
    pub daemon: DaemonType,
}

#[derive(Parser, Debug)]
pub struct SyncLogsArgs {
    /// Which daemon's logs to view (daily or dispatch)
    #[arg(value_enum)]
    pub daemon: SpecificDaemonType,

    /// Number of lines to show (default: 20)
    #[arg(long, short = 'n', default_value = "20")]
    pub lines: usize,

    /// Follow log output (like tail -f)
    #[arg(long, short = 'f')]
    pub follow: bool,
}

#[derive(Parser, Debug)]
pub struct SyncInstallArgs {
    /// Force reinstall even if files already exist
    #[arg(long)]
    pub force: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum DaemonType {
    Daily,
    Dispatch,
    Projects,
    All,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecificDaemonType {
    Daily,
    Dispatch,
    Projects,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DaemonStatus {
    pub name: String,
    pub running: bool,
    pub pid: Option<u32>,
    pub last_sync: Option<String>,
    pub status_message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SyncResult {
    pub daemon: String,
    pub success: bool,
    pub files_transferred: Option<usize>,
    pub bytes_transferred: Option<u64>,
    pub message: String,
}

// Command handlers

/// Execute sync command based on subcommand
///
/// Routes to appropriate handler based on the sync subcommand:
/// - `status`: Check daemon status
/// - `trigger`: Manually trigger sync
/// - `start`: Start daemon(s)
/// - `stop`: Stop daemon(s)
/// - `logs`: View sync logs
/// - `install`: Install/update scripts
///
/// # Arguments
///
/// * `args` - Sync command arguments including subcommand
///
/// # Returns
///
/// `Ok(())` on success, error on failure
pub async fn run_sync(args: SyncArgs) -> Result<()> {
    match args.command {
        SyncCommands::Status(status_args) => run_status(status_args).await,
        SyncCommands::Trigger(trigger_args) => run_trigger(trigger_args).await,
        SyncCommands::Start(start_args) => run_start(start_args).await,
        SyncCommands::Stop(stop_args) => run_stop(stop_args).await,
        SyncCommands::Logs(logs_args) => run_logs(logs_args).await,
        SyncCommands::Install(install_args) => run_install(install_args).await,
    }
}

async fn run_status(args: SyncStatusArgs) -> Result<()> {
    // Handle remote-only status via SSH
    if args.remote {
        return run_remote_status(&args.host, args.daemon, args.format).await;
    }

    // Default: show full pipeline status (MacBook → float-box → R2)
    #[cfg(target_os = "macos")]
    {
        println!("📊 Sync Pipeline Status\n");

        // 1. MacBook → float-box (rsync)
        println!("┌─ MacBook → float-box (rsync)");
        if let Some(last_sync) = get_float_box_sync_time()? {
            println!("│  ✅ Last sync: {}", last_sync);
        } else {
            println!("│  ⚠️  No sync log found");
        }
        println!("│");

        // 2. float-box → R2 (rclone via systemd)
        println!("└─ float-box → R2 (systemd services on {})", args.host);

        // Try to get remote status
        match get_remote_sync_summary(&args.host).await {
            Ok(summary) => {
                for line in summary.lines() {
                    println!("   {}", line);
                }
            }
            Err(e) => {
                println!("   ⚠️  Could not connect to {}: {}", args.host, e);
                println!("   Try: ssh {} 'systemctl --user list-timers'", args.host);
            }
        }

        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!(
            "Local daemon status checking is currently macOS-only (uses launchd/cron).\n\
            Use --remote to check float-box status via SSH.\n\
            For Linux, check systemd status: systemctl status <service>"
        );
    }
}

/// Check remote float-box sync status via SSH
async fn run_remote_status(host: &str, daemon: DaemonType, format: OutputFormat) -> Result<()> {
    println!("🔗 Checking remote sync status on {}...\n", host);

    // Build SSH command to check systemd status
    let services: Vec<&str> = match daemon {
        DaemonType::Daily => vec!["floatctl-daily-sync.service"],
        DaemonType::Dispatch => vec!["floatctl-dispatch-sync.timer"],
        DaemonType::Projects => vec!["floatctl-projects-sync.timer"],
        DaemonType::All => vec![
            "floatctl-daily-sync.service",
            "floatctl-dispatch-sync.timer",
            "floatctl-projects-sync.timer",
        ],
    };

    let mut statuses = Vec::new();

    for service in &services {
        // Check if service is active (using --user for user-level systemd services)
        let status_output = Command::new("ssh")
            .args([host, &format!("systemctl --user is-active {} 2>/dev/null || echo inactive", service)])
            .output()
            .context(format!("Failed to SSH to {}", host))?;

        let is_active = String::from_utf8_lossy(&status_output.stdout)
            .trim()
            .eq("active");

        // Get last sync from logs (grep for sync_complete events, get the last one)
        let daemon_name = service.replace("floatctl-", "").replace("-sync.service", "").replace("-sync.timer", "");
        let log_output = Command::new("ssh")
            .args([host, &format!("grep sync_complete ~/.floatctl/logs/{}.jsonl 2>/dev/null | tail -1 || echo '{{}}'", daemon_name)])
            .output()?;

        let last_sync = String::from_utf8_lossy(&log_output.stdout);
        let last_sync_time = if let Ok(event) = serde_json::from_str::<SyncEvent>(last_sync.trim()) {
            match event {
                SyncEvent::SyncComplete { timestamp, .. } => Some(format_timestamp(&timestamp)),
                _ => None,
            }
        } else {
            None
        };

        let status_message = if is_active {
            format!("Active (last sync: {})", last_sync_time.as_deref().unwrap_or("unknown"))
        } else {
            "Inactive".to_string()
        };

        statuses.push(DaemonStatus {
            name: format!("{} (remote)", daemon_name),
            running: is_active,
            pid: None,
            last_sync: last_sync_time,
            status_message,
        });
    }

    match format {
        OutputFormat::Text => print_status_text(&statuses),
        OutputFormat::Json => print_status_json(&statuses)?,
    }

    Ok(())
}

async fn run_trigger(args: SyncTriggerArgs) -> Result<()> {
    let results = match args.daemon {
        DaemonType::Daily => vec![trigger_daily_sync(args.wait)?],
        DaemonType::Dispatch => vec![trigger_dispatch_sync(args.wait)?],
        DaemonType::Projects => vec![trigger_projects_sync(args.wait)?],
        DaemonType::All => vec![
            trigger_daily_sync(args.wait)?,
            trigger_dispatch_sync(args.wait)?,
            trigger_projects_sync(args.wait)?,
        ],
    };

    for result in &results {
        if result.success {
            println!("✅ {} sync complete: {}", result.daemon, result.message);
        } else {
            eprintln!("❌ {} sync failed: {}", result.daemon, result.message);
        }
    }

    Ok(())
}

async fn run_start(args: SyncStartArgs) -> Result<()> {
    match args.daemon {
        DaemonType::Daily => start_daily_daemon()?,
        DaemonType::Dispatch => {
            println!("⚠️  Dispatch daemon is cron-based and starts automatically");
            println!("    Use 'floatctl sync trigger --daemon dispatch' to run manually");
        }
        DaemonType::Projects => {
            println!("⚠️  Projects sync runs on float-box via systemd timer");
            println!("    Use 'floatctl sync status --remote' to check status");
        }
        DaemonType::All => {
            start_daily_daemon()?;
            println!("⚠️  Dispatch daemon is cron-based and starts automatically");
            println!("⚠️  Projects sync runs on float-box via systemd timer");
        }
    }
    Ok(())
}

async fn run_stop(args: SyncStopArgs) -> Result<()> {
    match args.daemon {
        DaemonType::Daily => stop_daily_daemon()?,
        DaemonType::Dispatch => {
            println!("⚠️  Dispatch daemon is cron-based and runs periodically");
            println!("    No persistent process to stop");
        }
        DaemonType::Projects => {
            println!("⚠️  Projects sync runs on float-box via systemd timer");
            println!("    Stop on float-box: sudo systemctl stop floatctl-projects-sync.timer");
        }
        DaemonType::All => {
            stop_daily_daemon()?;
            println!("⚠️  Dispatch daemon is cron-based and runs periodically");
            println!("⚠️  Projects sync runs on float-box via systemd timer");
        }
    }
    Ok(())
}

async fn run_install(args: SyncInstallArgs) -> Result<()> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let dest_base = home.join(".floatctl");
    let dest_bin = dest_base.join("bin");
    let dest_lib = dest_base.join("lib");

    // Find scripts directory (try current directory, then parent directories)
    let scripts_dir = find_scripts_dir()?;

    println!("📦 Installing sync scripts from {}", scripts_dir.display());
    println!();

    // Create destination directories
    fs::create_dir_all(&dest_bin).context("Failed to create ~/.floatctl/bin")?;
    fs::create_dir_all(&dest_lib).context("Failed to create ~/.floatctl/lib")?;

    let mut installed = 0;
    let mut skipped = 0;

    // Install bin scripts
    for entry in fs::read_dir(scripts_dir.join("bin"))? {
        let entry = entry?;
        let src = entry.path();
        let filename = entry.file_name();
        let dest = dest_bin.join(&filename);

        if dest.exists() && !args.force {
            println!("⏭️  Skipping {} (already exists, use --force to overwrite)", filename.to_string_lossy());
            skipped += 1;
            continue;
        }

        fs::copy(&src, &dest)
            .with_context(|| format!("Failed to copy {}", filename.to_string_lossy()))?;

        // Make executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dest)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dest, perms)?;
        }

        println!("✅ Installed bin/{}", filename.to_string_lossy());
        installed += 1;
    }

    // Install lib scripts
    for entry in fs::read_dir(scripts_dir.join("lib"))? {
        let entry = entry?;
        let src = entry.path();
        let filename = entry.file_name();
        let dest = dest_lib.join(&filename);

        if dest.exists() && !args.force {
            println!("⏭️  Skipping {} (already exists, use --force to overwrite)", filename.to_string_lossy());
            skipped += 1;
            continue;
        }

        fs::copy(&src, &dest)
            .with_context(|| format!("Failed to copy {}", filename.to_string_lossy()))?;

        println!("✅ Installed lib/{}", filename.to_string_lossy());
        installed += 1;
    }

    println!();
    println!("📊 Installation complete: {} installed, {} skipped", installed, skipped);

    // Set up dispatch cron if not configured
    println!();
    setup_dispatch_cron(&home)?;

    Ok(())
}

fn find_scripts_dir() -> Result<std::path::PathBuf> {
    // Try current directory first
    let cwd = std::env::current_dir()?;
    let scripts = cwd.join("scripts");
    if scripts.exists() && scripts.is_dir() {
        return Ok(scripts);
    }

    // Try parent directories (for when running from floatctl-cli subdirectory)
    let mut current = cwd.as_path();
    for _ in 0..3 {
        if let Some(parent) = current.parent() {
            let scripts = parent.join("scripts");
            if scripts.exists() && scripts.is_dir() {
                return Ok(scripts);
            }
            current = parent;
        }
    }

    Err(anyhow::anyhow!(
        "Could not find scripts directory. Run this command from the floatctl-rs repository root."
    ))
}

async fn run_logs(args: SyncLogsArgs) -> Result<()> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let daemon_name = match args.daemon {
        SpecificDaemonType::Daily => "daily",
        SpecificDaemonType::Dispatch => "dispatch",
        SpecificDaemonType::Projects => "projects",
    };

    let log_path = home
        .join(".floatctl")
        .join("logs")
        .join(format!("{}.jsonl", daemon_name));

    if !log_path.exists() {
        eprintln!("❌ Log file not found: {}", log_path.display());
        return Ok(());
    }

    if args.follow {
        println!("⚠️  Follow mode not yet implemented");
        return Ok(());
    }

    // Read last N lines
    let content = fs::read_to_string(&log_path)
        .context(format!("Failed to read log file: {}", log_path.display()))?;

    let lines: Vec<&str> = content.lines().collect();
    let start = lines.len().saturating_sub(args.lines);
    let tail = &lines[start..];

    println!("📝 Last {} events from {} daemon:", args.lines, daemon_name);
    println!();

    for line in tail {
        if line.trim().is_empty() {
            continue;
        }

        // Try to parse and format JSONL
        if let Ok(event) = serde_json::from_str::<SyncEvent>(line) {
            println!("{}", format_sync_event(&event));
        } else {
            // Fallback to raw line if parsing fails
            println!("{}", line);
        }
    }

    Ok(())
}

fn format_timestamp(timestamp: &chrono::DateTime<chrono::Utc>) -> String {
    let toronto_time = timestamp.with_timezone(&Toronto);
    toronto_time.format("%b %d %I:%M%p").to_string().to_lowercase()
}

fn format_sync_event(event: &SyncEvent) -> String {
    use SyncEvent::*;

    match event {
        DaemonStart { timestamp, daemon: _, pid, config } => {
            let mut msg = format!("🚀 [{}] Daemon started (PID: {})", format_timestamp(timestamp), pid);
            if let Some(cfg) = config {
                msg.push_str(&format!("\n   Config: {:?}", cfg));
            }
            msg
        },
        DaemonStop { timestamp, daemon: _, reason } => {
            format!("🛑 [{}] Daemon stopped (reason: {})",
                format_timestamp(timestamp), reason)
        },
        FileChange { timestamp, daemon: _, path, debounce_ms } => {
            format!("📝 [{}] File changed: {}\n   Debouncing for {}ms",
                format_timestamp(timestamp), path, debounce_ms)
        },
        SyncStart { timestamp, daemon: _, trigger } => {
            format!("▶️  [{}] Sync started (trigger: {})",
                format_timestamp(timestamp), trigger)
        },
        SyncComplete { timestamp, daemon: _, success, files_transferred, bytes_transferred, duration_ms, transfer_rate_bps, error_message } => {
            let status = if *success { "✅" } else { "❌" };
            let mut msg = format!("{} [{}] Sync completed in {}ms",
                status, format_timestamp(timestamp), duration_ms);
            msg.push_str(&format!("\n   Files: {}, Bytes: {}", files_transferred, bytes_transferred));
            if let Some(rate) = transfer_rate_bps {
                msg.push_str(&format!(", Rate: {} bytes/sec", rate));
            }
            if let Some(err) = error_message {
                msg.push_str(&format!("\n   Error: {}", err));
            }
            msg
        },
        SyncError { timestamp, daemon: _, error_type, error_message, context } => {
            let mut msg = format!("❌ [{}] Error: {}\n   {}",
                format_timestamp(timestamp), error_type, error_message);
            if let Some(ctx) = context {
                msg.push_str(&format!("\n   Context: {:?}", ctx));
            }
            msg
        },
    }
}

// Status checking functions

#[cfg(target_os = "macos")]
fn check_daily_status() -> Result<DaemonStatus> {
    // Check if fswatch process is running for watch-and-sync.sh
    // Use ps -ef to get parent PIDs, filter for PPID=1 (launchd)
    let ps_output = Command::new("ps")
        .args(["-ef"])
        .output()
        .context("Failed to run ps command")?;

    let ps_stdout = String::from_utf8_lossy(&ps_output.stdout);

    // Find the main process (PPID=1, launched by launchd)
    let main_process = ps_stdout
        .lines()
        .find(|line| {
            line.contains("watch-and-sync.sh")
                && !line.contains("grep")
                // Column 3 is PPID in ps -ef output
                && line.split_whitespace().nth(2) == Some("1")
        });

    let watch_running = main_process.is_some();

    let (pid, last_sync) = if watch_running {
        // Extract PID from ps -ef output (column 2)
        let pid = main_process
            .and_then(|line| {
                line.split_whitespace()
                    .nth(1)
                    .and_then(|s| s.parse::<u32>().ok())
            });

        // Try JSONL first (unified logging), fall back to legacy logs
        let last_sync = if let Some(event) = get_last_sync_from_jsonl("daily")? {
            if let SyncEvent::SyncComplete { timestamp, .. } = event {
                Some(format_timestamp(&timestamp))
            } else {
                None
            }
        } else {
            // Fallback: Try legacy logs
            let watcher_sync = get_last_sync_time("autosync-watcher.log")?;
            let manual_sync = get_last_sync_time("daily-sync.log")?;

            // Return the most recent timestamp
            match (watcher_sync, manual_sync) {
                (Some(w), Some(m)) => Some(if w > m { w } else { m }),
                (Some(w), None) => Some(w),
                (None, Some(m)) => Some(m),
                (None, None) => None,
            }
        };

        (pid, last_sync)
    } else {
        (None, None)
    };

    let status_message = if watch_running {
        format!("Running (last sync: {})", last_sync.as_deref().unwrap_or("unknown"))
    } else {
        "Not running".to_string()
    };

    Ok(DaemonStatus {
        name: "daily".to_string(),
        running: watch_running,
        pid,
        last_sync,
        status_message,
    })
}

#[cfg(target_os = "macos")]
fn check_dispatch_status() -> Result<DaemonStatus> {
    // Check if crontab entry exists
    let crontab_output = Command::new("crontab")
        .args(["-l"])
        .output()
        .context("Failed to run crontab command")?;

    let crontab_stdout = String::from_utf8_lossy(&crontab_output.stdout);
    let cron_configured = crontab_stdout
        .lines()
        .any(|line| line.contains("sync-dispatch-to-r2.sh") && !line.starts_with('#'));

    // Try JSONL first (unified logging), fall back to legacy logs
    let last_sync = if let Some(event) = get_last_sync_from_jsonl("dispatch")? {
        if let SyncEvent::SyncComplete { timestamp, .. } = event {
            Some(format_timestamp(&timestamp))
        } else {
            None
        }
    } else {
        // Fallback: Try legacy log or file modification time
        get_last_sync_time("dispatch-cron.log")?
            .or_else(|| get_log_modification_time("dispatch-cron.log").ok().flatten())
    };

    let status_message = if cron_configured {
        format!(
            "Cron active (every 30 min, last sync: {})",
            last_sync.as_deref().unwrap_or("unknown")
        )
    } else {
        "Cron not configured".to_string()
    };

    Ok(DaemonStatus {
        name: "dispatch".to_string(),
        running: cron_configured,
        pid: None, // Cron doesn't have a persistent PID
        last_sync,
        status_message,
    })
}

fn get_last_sync_time(log_name: &str) -> Result<Option<String>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let log_path = home.join(".floatctl").join("logs").join(log_name);

    if !log_path.exists() {
        return Ok(None);
    }

    let content = fs::read_to_string(&log_path)?;
    let lines: Vec<&str> = content.lines().collect();

    // Find the last "sync complete" or similar message
    for line in lines.iter().rev() {
        if line.contains("sync complete") || line.contains("Sync complete") || line.contains("✅") {
            // Try to extract timestamp from log line format: [YYYY-MM-DD HH:MM:SS]
            if let Some(start) = line.find('[') {
                if let Some(end) = line.find(']') {
                    return Ok(Some(line[start + 1..end].to_string()));
                }
            }
        }
    }

    Ok(None)
}

fn get_log_modification_time(log_name: &str) -> Result<Option<String>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let log_path = home.join(".floatctl").join("logs").join(log_name);

    if !log_path.exists() {
        return Ok(None);
    }

    // Get file metadata and format modification time
    let metadata = fs::metadata(&log_path)?;
    if let Ok(modified) = metadata.modified() {
        // Format as YYYY-MM-DD HH:MM:SS
        let datetime = chrono::DateTime::<chrono::Local>::from(modified);
        return Ok(Some(datetime.format("%Y-%m-%d %H:%M:%S").to_string()));
    }

    Ok(None)
}

/// Get last sync event from JSONL log (most recent SyncComplete event)
fn get_last_sync_from_jsonl(daemon: &str) -> Result<Option<SyncEvent>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let jsonl_path = home.join(".floatctl").join("logs").join(format!("{}.jsonl", daemon));

    if !jsonl_path.exists() {
        return Ok(None);
    }

    // Read file and find last SyncComplete event
    let file = File::open(&jsonl_path)?;
    let reader = BufReader::new(file);

    let mut last_sync: Option<SyncEvent> = None;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Try to parse as SyncEvent
        if let Ok(event) = serde_json::from_str::<SyncEvent>(&line) {
            // Keep track of last sync_complete event
            if matches!(event, SyncEvent::SyncComplete { .. }) {
                last_sync = Some(event);
            }
        }
    }

    Ok(last_sync)
}

// Start/stop functions

fn start_daily_daemon() -> Result<()> {
    // Platform check - bail early on non-macOS
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!(
            "Daemon management is currently macOS-only (uses launchd).\n\
            For Linux, consider using systemd: https://systemd.io/\n\
            For Windows, consider using Windows Services or Task Scheduler."
        );
    }

    // macOS-specific daemon management
    #[cfg(target_os = "macos")]
    {
        // Check if already running
        let status = check_daily_status()?;
        if status.running {
            let pid = status.pid.expect("PID should exist when daemon is running");
            println!("✅ Daily daemon already running (PID: {})", pid);
            return Ok(());
        }

        let home = dirs::home_dir().context("Could not determine home directory")?;
        let plist_path = home
            .join("Library")
            .join("LaunchAgents")
            .join("net.floatbbs.autosync.plist");

        // Load via launchctl (starts the daemon)
        let plist_path_str = plist_path
            .to_str()
            .context("Plist path contains invalid UTF-8")?;
        let output = Command::new("launchctl")
            .args(["load", plist_path_str])
            .output()
            .context("Failed to load daemon via launchctl")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            // Ignore "service already loaded" error
            if !stderr.contains("already loaded") {
                eprintln!("❌ Failed to start daemon: {}", stderr);
                return Err(anyhow::anyhow!("launchctl load failed"));
            }
        }

        // Give it a moment to start
        std::thread::sleep(std::time::Duration::from_millis(DAEMON_OPERATION_DELAY_MS));

        // Check if it started successfully
        let status = check_daily_status()?;
        if status.running {
            let pid = status.pid.expect("PID should exist when daemon is running");
            println!("✅ Daily daemon started (PID: {})", pid);
        } else {
            println!("⚠️  Daemon start command sent, but process not detected");
            println!("    Check logs: ~/.floatctl/logs/autosync-watcher-error.log");
        }

        Ok(())
    }
}

fn stop_daily_daemon() -> Result<()> {
    // Platform check - bail early on non-macOS
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!(
            "Daemon management is currently macOS-only (uses launchd).\n\
            For Linux, consider using systemd: https://systemd.io/\n\
            For Windows, consider using Windows Services or Task Scheduler."
        );
    }

    // macOS-specific daemon management
    #[cfg(target_os = "macos")]
    {
        // Check if running
        let status = check_daily_status()?;
        if !status.running {
            println!("✅ Daily daemon already stopped");
            return Ok(());
        }

        let pid = status.pid.expect("PID should exist when daemon is running");
        let home = dirs::home_dir().context("Could not determine home directory")?;
        let plist_path = home
            .join("Library")
            .join("LaunchAgents")
            .join("net.floatbbs.autosync.plist");

        // Unload via launchctl (stops and prevents restart)
        let plist_path_str = plist_path
            .to_str()
            .context("Plist path contains invalid UTF-8")?;
        let output = Command::new("launchctl")
            .args(["unload", plist_path_str])
            .output()
            .context("Failed to unload daemon via launchctl")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            eprintln!("⚠️  launchctl unload warning: {}", stderr);
            println!("    Attempting direct process termination...");

            // Fallback: kill the process directly
            Command::new("kill")
                .args(["-TERM", &pid.to_string()])
                .output()
                .context("Failed to kill process")?;
        }

        // Give it a moment to stop
        std::thread::sleep(std::time::Duration::from_millis(DAEMON_OPERATION_DELAY_MS));

        // Check if it stopped successfully
        let status = check_daily_status()?;
        if !status.running {
            println!("✅ Daily daemon stopped");

            // Clean up PID file if it exists
            let pidfile = home.join(".floatctl").join("run").join("daily-sync.pid");
            if pidfile.exists() {
                let _ = fs::remove_file(&pidfile);
            }
        } else {
            println!("⚠️  Daemon still running after unload");
            println!("    Try: kill -9 {}", pid);
        }

        Ok(())
    }
}

// Trigger functions

fn trigger_daily_sync(wait: bool) -> Result<SyncResult> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let script_path = home.join(".floatctl").join("bin").join("sync-daily-to-r2.sh");

    if !script_path.exists() {
        return Ok(SyncResult {
            daemon: "daily".to_string(),
            success: false,
            files_transferred: None,
            bytes_transferred: None,
            message: format!("Sync script not found: {}", script_path.display()),
        });
    }

    let output = if wait {
        Command::new(&script_path)
            .env("FLOATCTL_TRIGGER", "manual")
            .output()
            .context("Failed to execute daily sync script")?
    } else {
        Command::new(&script_path)
            .env("FLOATCTL_TRIGGER", "manual")
            .spawn()
            .context("Failed to spawn daily sync script")?;
        return Ok(SyncResult {
            daemon: "daily".to_string(),
            success: true,
            files_transferred: None,
            bytes_transferred: None,
            message: "Sync triggered in background".to_string(),
        });
    };

    let success = output.status.success();
    let message = if success {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    Ok(SyncResult {
        daemon: "daily".to_string(),
        success,
        files_transferred: None, // TODO: Parse from rclone output
        bytes_transferred: None, // TODO: Parse from rclone output
        message: message.trim().to_string(),
    })
}

fn trigger_dispatch_sync(wait: bool) -> Result<SyncResult> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let script_path = home.join(".floatctl").join("bin").join("sync-dispatch-to-r2.sh");

    if !script_path.exists() {
        return Ok(SyncResult {
            daemon: "dispatch".to_string(),
            success: false,
            files_transferred: None,
            bytes_transferred: None,
            message: format!("Sync script not found: {}", script_path.display()),
        });
    }

    let output = if wait {
        Command::new(&script_path)
            .env("FLOATCTL_TRIGGER", "manual")
            .output()
            .context("Failed to execute dispatch sync script")?
    } else {
        Command::new(&script_path)
            .env("FLOATCTL_TRIGGER", "manual")
            .spawn()
            .context("Failed to spawn dispatch sync script")?;
        return Ok(SyncResult {
            daemon: "dispatch".to_string(),
            success: true,
            files_transferred: None,
            bytes_transferred: None,
            message: "Sync triggered in background".to_string(),
        });
    };

    let success = output.status.success();
    let message = if success {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    Ok(SyncResult {
        daemon: "dispatch".to_string(),
        success,
        files_transferred: None, // TODO: Parse from rclone output
        bytes_transferred: None, // TODO: Parse from rclone output
        message: message.trim().to_string(),
    })
}

fn trigger_projects_sync(wait: bool) -> Result<SyncResult> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let script_path = home.join(".floatctl").join("bin").join("sync-projects-to-r2.sh");

    if !script_path.exists() {
        return Ok(SyncResult {
            daemon: "projects".to_string(),
            success: false,
            files_transferred: None,
            bytes_transferred: None,
            message: format!("Sync script not found: {}", script_path.display()),
        });
    }

    let output = if wait {
        Command::new(&script_path)
            .env("FLOATCTL_TRIGGER", "manual")
            .output()
            .context("Failed to execute projects sync script")?
    } else {
        Command::new(&script_path)
            .env("FLOATCTL_TRIGGER", "manual")
            .spawn()
            .context("Failed to spawn projects sync script")?;
        return Ok(SyncResult {
            daemon: "projects".to_string(),
            success: true,
            files_transferred: None,
            bytes_transferred: None,
            message: "Sync triggered in background".to_string(),
        });
    };

    let success = output.status.success();
    let message = if success {
        String::from_utf8_lossy(&output.stdout).to_string()
    } else {
        String::from_utf8_lossy(&output.stderr).to_string()
    };

    Ok(SyncResult {
        daemon: "projects".to_string(),
        success,
        files_transferred: None,
        bytes_transferred: None,
        message: message.trim().to_string(),
    })
}

// Cron setup

fn setup_dispatch_cron(home: &std::path::Path) -> Result<()> {
    // Check if cron already configured
    let crontab_output = Command::new("crontab")
        .args(["-l"])
        .output()
        .context("Failed to run crontab command")?;

    let crontab_stdout = String::from_utf8_lossy(&crontab_output.stdout);
    let already_configured = crontab_stdout
        .lines()
        .any(|line| line.contains("sync-dispatch-to-r2.sh") && !line.starts_with('#'));

    if already_configured {
        println!("⏭️  Dispatch cron already configured");
        return Ok(());
    }

    // Add cron entry
    let script_path = home.join(".floatctl").join("bin").join("sync-dispatch-to-r2.sh");
    let script_path_str = script_path
        .to_str()
        .context("Script path contains invalid UTF-8")?;

    // Cron entry: every 30 minutes
    let cron_entry = format!("*/30 * * * * {}", script_path_str);

    // Get existing crontab (if any)
    let existing_crontab = if crontab_output.status.success() {
        crontab_stdout.to_string()
    } else {
        String::new()
    };

    // Append new entry
    let new_crontab = if existing_crontab.is_empty() {
        format!("{}\n", cron_entry)
    } else {
        format!("{}{}\n", existing_crontab, cron_entry)
    };

    // Write to temporary file and install via crontab
    let temp_file = home.join(".floatctl").join("crontab.tmp");
    fs::write(&temp_file, new_crontab)
        .context("Failed to write temporary crontab file")?;

    let output = Command::new("crontab")
        .arg(&temp_file)
        .output()
        .context("Failed to update crontab")?;

    // Clean up temp file
    let _ = fs::remove_file(&temp_file);

    if output.status.success() {
        println!("✅ Dispatch cron configured (every 30 minutes)");
    } else {
        println!("⚠️  Failed to configure dispatch cron");
        println!("    You can manually add: {}", cron_entry);
    }

    Ok(())
}

// Pipeline status helper functions

/// Get last sync time from MacBook → float-box rsync log
#[cfg(target_os = "macos")]
fn get_float_box_sync_time() -> Result<Option<String>> {
    let home = dirs::home_dir().context("Could not determine home directory")?;

    // Try today's log first, then yesterday's
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let yesterday = (chrono::Local::now() - chrono::Duration::days(1))
        .format("%Y-%m-%d")
        .to_string();

    for date in [&today, &yesterday] {
        let log_path = home
            .join(".floatctl")
            .join("logs")
            .join(format!("float-box-sync-{}.log", date));

        if !log_path.exists() {
            continue;
        }

        // Read file and find last sync completion
        let content = fs::read_to_string(&log_path)?;

        // Look for rsync completion markers (most recent first)
        for line in content.lines().rev() {
            // rsync output format includes timestamps like "[2025-11-25 16:28:44]"
            // Log format: "[2025-11-25 16:28:44] Sync completed successfully"
            if line.contains("Sync completed successfully") || line.contains("sync complete") {
                // Extract timestamp from log line
                if let Some(start) = line.find('[') {
                    if let Some(end) = line.find(']') {
                        let timestamp = &line[start + 1..end];
                        // Try to parse and format nicely
                        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S") {
                            let toronto_dt = chrono::TimeZone::from_local_datetime(&Toronto, &dt)
                                .single()
                                .unwrap_or_else(|| Toronto.from_utc_datetime(&dt));
                            return Ok(Some(toronto_dt.format("%b %d %I:%M%p").to_string().to_lowercase()));
                        }
                        return Ok(Some(timestamp.to_string()));
                    }
                }
            }
        }
    }

    Ok(None)
}

/// Get summary of float-box → R2 sync times via SSH
#[cfg(target_os = "macos")]
async fn get_remote_sync_summary(host: &str) -> Result<String> {
    let mut summary = String::new();

    let daemons = [
        ("daily", "Daily notes"),
        ("dispatch", "Dispatch"),
        ("projects", "Projects"),
    ];

    for (daemon, label) in daemons {
        // Get last sync from JSONL log on remote
        let cmd = format!(
            "tail -20 ~/.floatctl/logs/{}.jsonl 2>/dev/null | grep sync_complete | tail -1",
            daemon
        );

        let output = Command::new("ssh")
            .args([host, &cmd])
            .output()
            .context(format!("Failed to SSH to {}", host))?;

        let line = String::from_utf8_lossy(&output.stdout);

        if line.trim().is_empty() {
            summary.push_str(&format!("├─ {}: No sync log found\n", label));
            continue;
        }

        // Parse JSONL to extract timestamp
        if let Ok(event) = serde_json::from_str::<SyncEvent>(line.trim()) {
            if let SyncEvent::SyncComplete { timestamp, success, .. } = event {
                let status = if success { "✅" } else { "❌" };
                let time_str = format_timestamp(&timestamp);
                summary.push_str(&format!("├─ {} {}: Last sync {}\n", status, label, time_str));
            }
        } else {
            summary.push_str(&format!("├─ {}: Could not parse log\n", label));
        }
    }

    // Check systemd timer status
    let timer_output = Command::new("ssh")
        .args([host, "systemctl --user list-timers --no-pager 2>/dev/null | grep floatctl | head -3"])
        .output()?;

    let timer_status = String::from_utf8_lossy(&timer_output.stdout);
    if !timer_status.trim().is_empty() {
        summary.push_str("│\n└─ Timers active");
    }

    Ok(summary)
}

// Output formatting

fn print_status_text(statuses: &[DaemonStatus]) {
    println!("📊 R2 Sync Status\n");
    for status in statuses {
        let emoji = if status.running { "✅" } else { "❌" };
        println!("{} {}: {}", emoji, status.name, status.status_message);
        if let Some(pid) = status.pid {
            println!("   PID: {}", pid);
        }
    }
}

fn print_status_json(statuses: &[DaemonStatus]) -> Result<()> {
    let json = serde_json::to_string_pretty(statuses)?;
    println!("{}", json);
    Ok(())
}

use anyhow::{Context, Result};
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
    /// Which daemon to check (daily, dispatch, or all)
    #[arg(long, value_enum, default_value = "all")]
    pub daemon: DaemonType,

    /// Output format (text or json)
    #[arg(long, value_enum, default_value = "text")]
    pub format: OutputFormat,
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
    All,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecificDaemonType {
    Daily,
    Dispatch,
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
    let statuses = match args.daemon {
        DaemonType::Daily => vec![check_daily_status()?],
        DaemonType::Dispatch => vec![check_dispatch_status()?],
        DaemonType::All => vec![check_daily_status()?, check_dispatch_status()?],
    };

    match args.format {
        OutputFormat::Text => print_status_text(&statuses),
        OutputFormat::Json => print_status_json(&statuses)?,
    }

    Ok(())
}

async fn run_trigger(args: SyncTriggerArgs) -> Result<()> {
    let results = match args.daemon {
        DaemonType::Daily => vec![trigger_daily_sync(args.wait)?],
        DaemonType::Dispatch => vec![trigger_dispatch_sync(args.wait)?],
        DaemonType::All => vec![trigger_daily_sync(args.wait)?, trigger_dispatch_sync(args.wait)?],
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
        DaemonType::All => {
            start_daily_daemon()?;
            println!("⚠️  Dispatch daemon is cron-based and starts automatically");
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
        DaemonType::All => {
            stop_daily_daemon()?;
            println!("⚠️  Dispatch daemon is cron-based and runs periodically");
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
        DaemonStart { timestamp, daemon, pid, config } => {
            let mut msg = format!("🚀 [{}] Daemon started (PID: {})", format_timestamp(timestamp), pid);
            if let Some(cfg) = config {
                msg.push_str(&format!("\n   Config: {:?}", cfg));
            }
            msg
        },
        DaemonStop { timestamp, daemon, reason } => {
            format!("🛑 [{}] Daemon stopped (reason: {})",
                format_timestamp(timestamp), reason)
        },
        FileChange { timestamp, daemon, path, debounce_ms } => {
            format!("📝 [{}] File changed: {}\n   Debouncing for {}ms",
                format_timestamp(timestamp), path, debounce_ms)
        },
        SyncStart { timestamp, daemon, trigger } => {
            format!("▶️  [{}] Sync started (trigger: {})",
                format_timestamp(timestamp), trigger)
        },
        SyncComplete { timestamp, daemon, success, files_transferred, bytes_transferred, duration_ms, transfer_rate_bps, error_message } => {
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
        SyncError { timestamp, daemon, error_type, error_message, context } => {
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
    // Platform check
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!(
            "Daemon management is currently macOS-only (uses launchd).\n\
            For Linux, consider using systemd: https://systemd.io/\n\
            For Windows, consider using Windows Services or Task Scheduler."
        );
    }

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

fn stop_daily_daemon() -> Result<()> {
    // Platform check
    #[cfg(not(target_os = "macos"))]
    {
        anyhow::bail!(
            "Daemon management is currently macOS-only (uses launchd).\n\
            For Linux, consider using systemd: https://systemd.io/\n\
            For Windows, consider using Windows Services or Task Scheduler."
        );
    }

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

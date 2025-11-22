//! System health and maintenance commands
//!
//! Commands: health-check, cleanup

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};

// === Arg Structs (moved from main.rs for high cohesion) ===

#[derive(Parser, Debug)]
pub struct SystemArgs {
    #[command(subcommand)]
    pub command: SystemCommands,
}

#[derive(Subcommand, Debug)]
pub enum SystemCommands {
    /// Run system health diagnostics
    HealthCheck,
    /// Clean up duplicate processes and zombies
    Cleanup(CleanupArgs),
}

#[derive(Parser, Debug)]
pub struct CleanupArgs {
    /// Preview cleanup actions without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompts
    #[arg(long)]
    pub force: bool,
}

// === Command Implementations ===

pub fn run_system(args: SystemArgs) -> Result<()> {
    match args.command {
        SystemCommands::HealthCheck => run_system_health_check(),
        SystemCommands::Cleanup(cleanup_args) => run_system_cleanup(cleanup_args),
    }
}

fn run_system_health_check() -> Result<()> {
    use std::process::Command;

    let home = dirs::home_dir().context("Could not determine home directory")?;
    let script_path = home.join(".floatctl").join("bin").join("health-check.sh");

    // Validate script exists
    if !script_path.exists() {
        return Err(anyhow!(
            "System script not found: {}\n\
             Install system scripts with: floatctl sync install",
            script_path.display()
        ));
    }

    // Execute script
    let status = Command::new(&script_path)
        .status()
        .with_context(|| format!("Failed to execute health-check: {}", script_path.display()))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(anyhow!("Health check found {} issue(s)", code));
    }

    Ok(())
}

fn run_system_cleanup(args: CleanupArgs) -> Result<()> {
    use std::process::Command;

    let home = dirs::home_dir().context("Could not determine home directory")?;
    let script_path = home.join(".floatctl").join("bin").join("cleanup.sh");

    // Validate script exists
    if !script_path.exists() {
        return Err(anyhow!(
            "System script not found: {}\n\
             Install system scripts with: floatctl sync install",
            script_path.display()
        ));
    }

    // Build command with arguments
    let mut cmd = Command::new(&script_path);

    if args.dry_run {
        cmd.arg("--dry-run");
    }

    if args.force {
        cmd.arg("--force");
    }

    // Execute script
    let status = cmd
        .status()
        .with_context(|| format!("Failed to execute cleanup: {}", script_path.display()))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(anyhow!("Cleanup exited with code: {}", code));
    }

    Ok(())
}

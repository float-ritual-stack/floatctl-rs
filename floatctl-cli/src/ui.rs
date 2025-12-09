//! Unified UI helpers for floatctl CLI
//!
//! Provides consistent progress feedback across all commands with automatic
//! quiet mode detection for LLM consumption.
//!
//! # Quiet Mode
//!
//! Progress spinners and bars are automatically suppressed when:
//! - `--quiet` flag is passed
//! - `FLOATCTL_QUIET=1` environment variable is set
//! - stderr is not a TTY (piped output)
//!
//! This allows floatctl to be used cleanly by Claude Code and other tools.

use std::io::IsTerminal;
use std::sync::OnceLock;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};

/// Global quiet mode state
static QUIET_MODE: OnceLock<bool> = OnceLock::new();

/// Initialize quiet mode from flags and environment
///
/// Call this once at startup with the --quiet flag value.
/// Will also check FLOATCTL_QUIET env var and TTY status.
pub fn init_quiet_mode(quiet_flag: bool) {
    let is_quiet = quiet_flag
        || std::env::var("FLOATCTL_QUIET").map(|v| v == "1").unwrap_or(false)
        || !std::io::stderr().is_terminal();

    QUIET_MODE.set(is_quiet).ok();
}

/// Check if we're in quiet mode
pub fn is_quiet() -> bool {
    *QUIET_MODE.get().unwrap_or(&false)
}

/// Create a spinner that respects quiet mode
///
/// Returns None in quiet mode, allowing clean LLM output.
pub fn spinner(msg: impl Into<String>) -> Option<ProgressBar> {
    if is_quiet() {
        return None;
    }

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
            .template("{spinner:.cyan} {msg}")
            .expect("valid template"),
    );
    pb.set_message(msg.into());
    pb.enable_steady_tick(Duration::from_millis(80));
    Some(pb)
}

/// Create a bounded progress bar that respects quiet mode
#[allow(dead_code)]
pub fn progress_bar(len: u64, msg: impl Into<String>) -> Option<ProgressBar> {
    if is_quiet() {
        return None;
    }

    let pb = ProgressBar::new(len);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg} [{bar:30.cyan/dim}] {pos}/{len} ({eta})")
            .expect("valid template")
            .progress_chars("━╸─"),
    );
    pb.set_message(msg.into());
    Some(pb)
}

/// Finish a progress bar with a success message
pub fn finish_success(pb: Option<ProgressBar>, msg: impl Into<String>) {
    if let Some(pb) = pb {
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{msg}")
                .expect("valid template"),
        );
        pb.finish_with_message(format!("✓ {}", msg.into()));
    }
}

/// Finish a progress bar with an error message
pub fn finish_error(pb: Option<ProgressBar>, msg: impl Into<String>) {
    if let Some(pb) = pb {
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{msg}")
                .expect("valid template"),
        );
        pb.finish_with_message(format!("✗ {}", msg.into()));
    }
}

/// Run an operation with a spinner, handling success/error automatically
///
/// Shows a spinner during the operation, then success/error on completion.
/// In quiet mode, just runs the operation silently.
#[allow(dead_code)]
pub fn with_spinner<T, E: std::fmt::Display>(
    msg: impl Into<String>,
    success_msg: impl Into<String>,
    f: impl FnOnce() -> Result<T, E>,
) -> Result<T, E> {
    let msg = msg.into();
    let success_msg = success_msg.into();
    let pb = spinner(&msg);

    match f() {
        Ok(result) => {
            finish_success(pb, success_msg);
            Ok(result)
        }
        Err(e) => {
            finish_error(pb, format!("{}: {}", msg, e));
            Err(e)
        }
    }
}

/// Async version of with_spinner
#[allow(dead_code)]
pub async fn with_spinner_async<T, E: std::fmt::Display>(
    msg: impl Into<String>,
    success_msg: impl Into<String>,
    f: impl std::future::Future<Output = Result<T, E>>,
) -> Result<T, E> {
    let msg = msg.into();
    let success_msg = success_msg.into();
    let pb = spinner(&msg);

    match f.await {
        Ok(result) => {
            finish_success(pb, success_msg);
            Ok(result)
        }
        Err(e) => {
            finish_error(pb, format!("{}: {}", msg, e));
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quiet_mode_default() {
        // In tests, TTY detection should return false (not a terminal)
        // so quiet mode should be true by default
        init_quiet_mode(false);
        // Note: can't test is_quiet() reliably since OnceLock is global
    }
}

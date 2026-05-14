//! `floatctl normalize` — normalize YAML frontmatter in markdown files.
//!
//! Native Rust port of `/opt/float/bbs/scripts/yaml-normalize.py`. The
//! launchd plist (`tech.float.bbs.yaml-normalize.plist`) can be migrated
//! to invoke this command instead of the Python script — see
//! `docs/yaml-normalize-migration.md`.

use anyhow::{Context, Result};
use clap::Parser;
use floatctl_core::yaml_normalize::{normalize_str, NormalizeOutcome};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Parser, Debug)]
pub struct NormalizeArgs {
    /// One or more files or directories to normalize. Directories are walked
    /// recursively for `*.md` files.
    #[arg(value_name = "PATH", required = true)]
    paths: Vec<PathBuf>,

    /// Write changes back to disk. Default is dry-run (counts only).
    #[arg(long)]
    apply: bool,

    /// Show a unified diff for each changed file.
    #[arg(long)]
    diff: bool,

    /// Cap the number of diffs shown (with `--diff`). 0 = unlimited.
    #[arg(long, default_value = "0")]
    limit: usize,

    /// Only process files modified within the last N days.
    #[arg(long)]
    mtime: Option<u64>,
}

pub fn run_normalize(args: NormalizeArgs) -> Result<()> {
    let files = collect_files(&args.paths, args.mtime)?;

    let mut changed = 0usize;
    let mut shown = 0usize;

    for path in &files {
        let original = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("! skip (read error): {}: {}", path.display(), e);
                continue;
            }
        };
        let new = match normalize_str(&original) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("! skip (normalize error): {}: {}", path.display(), e);
                continue;
            }
        };
        if new == original {
            continue;
        }
        changed += 1;

        if args.diff && (args.limit == 0 || shown < args.limit) {
            print_unified_diff(&original, &new, &path.display().to_string());
            shown += 1;
        }

        if args.apply {
            std::fs::write(path, &new)
                .with_context(|| format!("write {}", path.display()))?;
        }
    }

    let verb = if args.apply { "applied" } else { "would change" };
    eprintln!(
        "\n{}: {} files (scanned {})",
        verb,
        changed,
        files.len()
    );
    Ok(())
}

fn collect_files(paths: &[PathBuf], mtime_days: Option<u64>) -> Result<Vec<PathBuf>> {
    let mut out: Vec<PathBuf> = Vec::new();
    let cutoff = mtime_days.map(|d| {
        SystemTime::now()
            .checked_sub(Duration::from_secs(d * 86_400))
            .unwrap_or(SystemTime::UNIX_EPOCH)
    });

    for p in paths {
        if !p.exists() {
            eprintln!("! skip (does not exist): {}", p.display());
            continue;
        }
        if p.is_file() {
            if matches_mtime(p, cutoff) {
                out.push(p.clone());
            }
            continue;
        }
        if p.is_dir() {
            let walker = walkdir::WalkDir::new(p).into_iter().filter_map(|e| e.ok());
            for entry in walker {
                let path = entry.path();
                if path.is_file()
                    && path.extension().and_then(|s| s.to_str()) == Some("md")
                    && matches_mtime(path, cutoff)
                {
                    out.push(path.to_path_buf());
                }
            }
        }
    }
    out.sort();
    Ok(out)
}

fn matches_mtime(path: &std::path::Path, cutoff: Option<SystemTime>) -> bool {
    let Some(cutoff) = cutoff else { return true };
    match std::fs::metadata(path).and_then(|m| m.modified()) {
        Ok(modified) => modified >= cutoff,
        Err(_) => true,
    }
}

fn print_unified_diff(a: &str, b: &str, fname: &str) {
    // Minimal unified diff — line-by-line. Mirrors python difflib output enough
    // to be useful at a glance. For deep audits, `git diff` the apply result.
    let a_lines: Vec<&str> = a.split('\n').collect();
    let b_lines: Vec<&str> = b.split('\n').collect();

    println!("--- {}", fname);
    println!("+++ {} (normalized)", fname);

    // naive line-pair diff; sufficient for human review of frontmatter changes
    let max_len = a_lines.len().max(b_lines.len());
    for i in 0..max_len {
        let a_line = a_lines.get(i);
        let b_line = b_lines.get(i);
        match (a_line, b_line) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => {
                println!("-{}", a);
                println!("+{}", b);
            }
            (Some(a), None) => println!("-{}", a),
            (None, Some(b)) => println!("+{}", b),
            (None, None) => {}
        }
    }
    println!();
    let _ = NormalizeOutcome::Unchanged; // keep import live
}

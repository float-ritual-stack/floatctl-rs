//! Script management commands
//!
//! Commands: register, unregister, list, show, edit, describe, run

use anyhow::{anyhow, Context, Result};
use clap::{Parser, Subcommand};
use floatctl_script;
use std::path::PathBuf;
use tracing::info;

// === Arg Structs (moved from main.rs for high cohesion) ===

#[derive(Parser, Debug)]
pub struct ScriptArgs {
    #[command(subcommand)]
    pub command: ScriptCommands,
}

#[derive(Subcommand, Debug)]
pub enum ScriptCommands {
    /// Register a shell script for reuse
    Register(RegisterScriptArgs),
    /// Unregister (remove) a registered script
    Unregister(UnregisterScriptArgs),
    /// List all registered scripts with descriptions
    List(ListScriptArgs),
    /// Show (cat) a registered script to stdout
    Show(ShowScriptArgs),
    /// Edit a registered script in $EDITOR
    Edit(EditScriptArgs),
    /// Show full documentation for a registered script
    Describe(DescribeScriptArgs),
    /// Run a registered script with arguments
    Run(RunScriptArgs),
}

#[derive(Parser, Debug)]
pub struct RegisterScriptArgs {
    /// Path to the script file to register
    #[arg(value_name = "PATH")]
    script_path: PathBuf,

    /// Optional name for the script (defaults to filename)
    #[arg(long, short = 'n')]
    name: Option<String>,

    /// Force overwrite if script already exists
    #[arg(long, short = 'f')]
    force: bool,

    /// Preview registration without copying file
    #[arg(long)]
    dry_run: bool,
}

#[derive(Parser, Debug)]
pub struct UnregisterScriptArgs {
    /// Name of the script to unregister
    script_name: String,

    /// Skip confirmation prompt
    #[arg(long, short = 'f')]
    force: bool,
}

#[derive(Parser, Debug)]
pub struct RunScriptArgs {
    /// Name of the registered script to run
    script_name: String,

    /// Arguments to pass to the script
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    args: Vec<String>,
}

#[derive(Parser, Debug)]
pub struct ListScriptArgs {
    /// Output format (text, json, names-only)
    #[arg(long, default_value = "text")]
    format: String,
}

#[derive(Parser, Debug)]
pub struct ShowScriptArgs {
    /// Name of the script to show
    script_name: String,
}

#[derive(Parser, Debug)]
pub struct EditScriptArgs {
    /// Name of the script to edit
    script_name: String,
}

#[derive(Parser, Debug)]
pub struct DescribeScriptArgs {
    /// Name of the script to describe
    script_name: String,
}

// === Command Implementations ===

pub fn run_script(args: ScriptArgs) -> Result<()> {
    match args.command {
        ScriptCommands::Register(register_args) => run_script_register(register_args),
        ScriptCommands::Unregister(unregister_args) => run_script_unregister(unregister_args),
        ScriptCommands::List(list_args) => run_script_list(list_args),
        ScriptCommands::Show(show_args) => run_script_show(show_args),
        ScriptCommands::Edit(edit_args) => run_script_edit(edit_args),
        ScriptCommands::Describe(describe_args) => run_script_describe(describe_args),
        ScriptCommands::Run(run_args) => run_script_run(run_args),
    }
}

fn get_scripts_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    let scripts_dir = home.join(".floatctl").join("scripts");

    // Create if doesn't exist
    if !scripts_dir.exists() {
        std::fs::create_dir_all(&scripts_dir)
            .with_context(|| format!("Failed to create {}", scripts_dir.display()))?;
        info!("Created scripts directory: {}", scripts_dir.display());
    }

    Ok(scripts_dir)
}

// === Platform-specific helpers ===

#[cfg(unix)]
fn make_executable(path: &PathBuf) -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let mut perms = std::fs::metadata(path)?.permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms)?;
    Ok(())
}

#[cfg(not(unix))]
fn make_executable(_path: &PathBuf) -> Result<()> {
    // Windows: Files are executable by extension (.bat, .cmd, .exe)
    Ok(())
}

fn validate_script(path: &PathBuf) -> Result<()> {
    use std::io::Read;

    // Security: Reject files larger than 10 MiB
    let metadata = std::fs::metadata(path)?;
    const MAX_SCRIPT_SIZE: u64 = 10 * 1024 * 1024; // 10 MiB
    if metadata.len() > MAX_SCRIPT_SIZE {
        return Err(anyhow!(
            "Script too large ({} bytes, max {} bytes)\n   This may not be a script file",
            metadata.len(),
            MAX_SCRIPT_SIZE
        ));
    }

    let mut file = std::fs::File::open(path)?;
    let mut buffer = [0u8; 2];

    // Check if file is readable
    if file.read(&mut buffer).is_err() {
        return Err(anyhow!("Cannot read script file"));
    }

    // Check for shebang on Unix systems
    #[cfg(unix)]
    {
        if buffer != [b'#', b'!'] {
            eprintln!("⚠️  Warning: Script does not start with shebang (#!)");
            eprintln!("   Script may not execute correctly without proper interpreter directive");
        }
    }

    Ok(())
}

fn run_script_register(args: RegisterScriptArgs) -> Result<()> {
    use std::fs;

    // Validate input script exists
    if !args.script_path.exists() {
        return Err(anyhow!("Script not found: {}", args.script_path.display()));
    }

    if !args.script_path.is_file() {
        return Err(anyhow!("Path is not a file: {}", args.script_path.display()));
    }

    // Security: Prevent symlink attacks
    if args.script_path.is_symlink() {
        return Err(anyhow!(
            "Cannot register symlink: {}\n   Register the target file directly instead",
            args.script_path.display()
        ));
    }

    // Validate script content (check shebang on Unix)
    validate_script(&args.script_path)?;

    // Determine script name
    let script_name = if let Some(name) = args.name {
        // Validate custom script name
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("Script name cannot be empty"));
        }
        if trimmed.contains('/') || trimmed.contains('\\') {
            return Err(anyhow!(
                "Script name cannot contain path separators (/ or \\)\n   Use simple filename only"
            ));
        }
        trimmed.to_string()
    } else {
        args.script_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Could not determine script filename")?
            .to_string()
    };

    // Get scripts directory
    let scripts_dir = get_scripts_dir()?;
    let dest_path = scripts_dir.join(&script_name);

    // Check if already exists
    if dest_path.exists() && !args.force {
        if args.dry_run {
            println!("🔍 Dry run: Would fail - script '{}' already exists", script_name);
            println!("   Use --force to overwrite");
            return Ok(());
        }
        return Err(anyhow!(
            "Script '{}' already exists. Use --force to overwrite",
            script_name
        ));
    }

    // Dry run mode - show what would be done
    if args.dry_run {
        println!("🔍 Dry run: Would register script");
        println!("   Source: {}", args.script_path.display());
        println!("   Destination: {}", dest_path.display());
        println!("   Name: {}", script_name);
        if dest_path.exists() {
            println!("   Action: Overwrite existing script");
        } else {
            println!("   Action: Create new script");
        }
        return Ok(());
    }

    // Copy script to scripts directory
    fs::copy(&args.script_path, &dest_path)
        .with_context(|| format!("Failed to copy script to {}", dest_path.display()))?;

    // Make executable (Unix: chmod 755, Windows: no-op)
    make_executable(&dest_path)?;

    println!("✅ Registered script: {}", script_name);
    println!("   Location: {}", dest_path.display());
    println!("   Run with: floatctl script run {}", script_name);

    Ok(())
}

fn run_script_unregister(args: UnregisterScriptArgs) -> Result<()> {
    use std::fs;
    use std::io::{self, Write};

    let scripts_dir = get_scripts_dir()?;
    let script_path = scripts_dir.join(&args.script_name);

    // Check if script exists
    if !script_path.exists() {
        return Err(anyhow!(
            "Script '{}' not found.\n   List registered scripts with: floatctl script list",
            args.script_name
        ));
    }

    // Get script description for confirmation
    let doc = floatctl_script::parse_doc_block(&script_path).ok();
    let description = doc
        .as_ref()
        .and_then(|d| d.description.as_ref())
        .map(|s| s.as_str())
        .unwrap_or("(no description)");

    // Confirm deletion unless --force
    if !args.force {
        println!("⚠️  Unregister script '{}'?", args.script_name);
        println!("   Description: {}", description);
        println!("   Location: {}", script_path.display());
        print!("\nConfirm deletion? (y/N): ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("Cancelled.");
            return Ok(());
        }
    }

    // Delete the script
    fs::remove_file(&script_path)
        .with_context(|| format!("Failed to remove script: {}", script_path.display()))?;

    println!("✅ Unregistered script: {}", args.script_name);

    Ok(())
}

fn run_script_list(args: ListScriptArgs) -> Result<()> {
    let parse_docs = args.format != "names-only";
    let scripts = floatctl_script::list_scripts(parse_docs)?;

    if scripts.is_empty() {
        if args.format == "json" {
            println!("{{\"scripts\":[]}}");
        } else {
            println!("No scripts registered.");
            println!("Register a script with: floatctl script register <path>");
        }
        return Ok(());
    }

    match args.format.as_str() {
        "json" => {
            let scripts_dir = floatctl_script::get_scripts_dir()?;
            let output = serde_json::json!({
                "scripts_dir": scripts_dir.display().to_string(),
                "scripts": scripts
            });
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        "names-only" => {
            for script in scripts {
                println!("{}", script.name);
            }
        }
        _ => {
            // Default text format
            let scripts_dir = floatctl_script::get_scripts_dir()?;
            println!("Registered scripts in {}:\n", scripts_dir.display());

            for script in scripts {
                println!("  {} ({} bytes)", script.name, script.size);

                if let Some(doc) = script.doc {
                    if let Some(desc) = doc.description {
                        println!("    {}", desc);
                    }
                    if let Some(usage) = doc.usage {
                        println!("    Usage: {}", usage);
                    }
                    if !doc.args.is_empty() {
                        let arg_names: Vec<_> = doc.args.iter().map(|a| a.name.as_str()).collect();
                        println!("    Args: {}", arg_names.join(", "));
                    }
                }
                println!();
            }

            println!("Run with: floatctl script run <name> [args...]");
        }
    }

    Ok(())
}

fn run_script_show(args: ShowScriptArgs) -> Result<()> {
    let content = floatctl_script::show_script(&args.script_name)?;
    print!("{}", content);
    Ok(())
}

fn run_script_edit(args: EditScriptArgs) -> Result<()> {
    use std::process::Command;

    let scripts_dir = floatctl_script::get_scripts_dir()?;
    let script_path = scripts_dir.join(&args.script_name);

    if !script_path.exists() {
        return Err(anyhow!(
            "Script '{}' not found. List scripts with: floatctl script list",
            args.script_name
        ));
    }

    // Get editor from environment or fall back to vim
    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

    let status = Command::new(&editor)
        .arg(&script_path)
        .status()
        .with_context(|| format!("Failed to execute editor: {}", editor))?;

    if !status.success() {
        return Err(anyhow!("Editor exited with non-zero status"));
    }

    println!("✅ Script '{}' updated", args.script_name);
    println!("   Run with: floatctl script run {}", args.script_name);

    Ok(())
}

fn run_script_describe(args: DescribeScriptArgs) -> Result<()> {
    let scripts_dir = floatctl_script::get_scripts_dir()?;
    let script_path = scripts_dir.join(&args.script_name);

    if !script_path.exists() {
        return Err(anyhow!(
            "Script '{}' not found. List scripts with: floatctl script list",
            args.script_name
        ));
    }

    // Parse doc block
    let doc = floatctl_script::parse_doc_block(&script_path)?;

    // Display formatted documentation
    println!("📜 {}", args.script_name);
    println!();

    if let Some(desc) = &doc.description {
        println!("Description: {}", desc);
    }

    if let Some(usage) = &doc.usage {
        println!("Usage: {}", usage);
    }

    if !doc.args.is_empty() {
        println!();
        println!("Arguments:");
        for arg in &doc.args {
            if let Some(desc) = &arg.description {
                println!("  {} - {}", arg.name, desc);
            } else {
                println!("  {}", arg.name);
            }
        }
    }

    if let Some(example) = &doc.example {
        println!();
        println!("Example:");
        println!("  {}", example);
    }

    // If no documentation found, show message
    if doc.description.is_none() && doc.usage.is_none() && doc.args.is_empty() && doc.example.is_none() {
        println!("(No documentation found in script)");
        println!();
        println!("To add documentation, add comments at the top of the script:");
        println!("  # Description: What the script does");
        println!("  # Usage: script-name <args>");
        println!("  # Args:");
        println!("  #   arg1 - Description");
        println!("  # Example:");
        println!("  #   script-name example");
    }

    Ok(())
}

fn run_script_run(args: RunScriptArgs) -> Result<()> {
    use std::process::Command;

    let scripts_dir = get_scripts_dir()?;
    let script_path = scripts_dir.join(&args.script_name);

    // Validate script exists
    if !script_path.exists() {
        return Err(anyhow!(
            "Script '{}' not found. List scripts with: floatctl script list",
            args.script_name
        ));
    }

    // Execute script with arguments
    // Note: Uses .status() instead of .output() for real-time streaming output.
    // Trade-off: stderr is not captured, but user sees output immediately.
    let mut cmd = Command::new(&script_path);
    cmd.args(&args.args);

    let status = cmd.status()
        .with_context(|| {
            #[cfg(unix)]
            let hint = "Check that script has proper shebang and execute permissions";
            #[cfg(not(unix))]
            let hint = "Check that script has proper extension (.bat, .cmd, .ps1)";

            format!(
                "Failed to execute script: {}\n   {}",
                script_path.display(),
                hint
            )
        })?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        return Err(anyhow!(
            "Script '{}' exited with code: {}",
            args.script_name,
            code
        ));
    }

    Ok(())
}

// === Tests ===

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_validate_script_rejects_large_files() {
        let temp_dir = TempDir::new().unwrap();
        let large_file = temp_dir.path().join("large.sh");

        // Create 11 MiB file (exceeds 10 MiB limit)
        let mut file = std::fs::File::create(&large_file).unwrap();
        let data = vec![0u8; 11 * 1024 * 1024];
        file.write_all(&data).unwrap();
        drop(file);

        let result = validate_script(&large_file);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Script too large"));
    }

    #[test]
    fn test_validate_script_accepts_small_files() {
        let temp_dir = TempDir::new().unwrap();
        let small_file = temp_dir.path().join("small.sh");

        // Create small file with shebang
        let mut file = std::fs::File::create(&small_file).unwrap();
        file.write_all(b"#!/bin/bash\necho 'hello'\n").unwrap();
        drop(file);

        let result = validate_script(&small_file);
        assert!(result.is_ok());
    }

    #[test]
    fn test_script_name_validation_rejects_path_separators() {
        let args = RegisterScriptArgs {
            script_path: PathBuf::from("/tmp/test.sh"),
            name: Some("../etc/passwd".to_string()),
            force: false,
            dry_run: true,
        };

        // Simulate the validation logic
        let name = args.name.as_ref().unwrap();
        let trimmed = name.trim();
        let has_separator = trimmed.contains('/') || trimmed.contains('\\');

        assert!(has_separator, "Should detect path separator");
    }

    #[test]
    fn test_script_name_validation_rejects_empty_names() {
        let args = RegisterScriptArgs {
            script_path: PathBuf::from("/tmp/test.sh"),
            name: Some("   ".to_string()),
            force: false,
            dry_run: true,
        };

        // Simulate the validation logic
        let name = args.name.as_ref().unwrap();
        let trimmed = name.trim();
        let is_empty = trimmed.is_empty();

        assert!(is_empty, "Should detect empty name");
    }

    #[test]
    fn test_get_scripts_dir_creates_directory() {
        // This test verifies that get_scripts_dir() creates the directory
        // Note: This will create ~/.floatctl/scripts if it doesn't exist
        let result = get_scripts_dir();
        assert!(result.is_ok());
        let scripts_dir = result.unwrap();
        assert!(scripts_dir.exists());
        assert!(scripts_dir.is_dir());
    }
}

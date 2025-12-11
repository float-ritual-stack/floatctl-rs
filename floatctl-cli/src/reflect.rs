//! CLI Introspection - `floatctl reflect` command
//!
//! Outputs the full CLI schema in JSON format, allowing agents to
//! "read the manual" programmatically before attempting a task.
//!
//! Example output:
//! ```json
//! {
//!   "name": "floatctl",
//!   "version": "0.1.0",
//!   "description": "...",
//!   "commands": [
//!     {
//!       "name": "full-extract",
//!       "description": "Full extraction workflow...",
//!       "args": [
//!         { "name": "in", "required": true, "type": "PATH", "description": "Input file" }
//!       ]
//!     }
//!   ]
//! }
//! ```

use clap::{Arg, ArgAction, Command};
use serde::{Deserialize, Serialize};

/// Full CLI schema for introspection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CliSchema {
    /// CLI binary name
    pub name: String,
    /// Version string
    pub version: String,
    /// Top-level description
    pub description: String,
    /// Global arguments (apply to all commands)
    pub global_args: Vec<ArgSchema>,
    /// Available subcommands
    pub commands: Vec<CommandSchema>,
}

/// Schema for a single command or subcommand
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandSchema {
    /// Command name (e.g., "full-extract")
    pub name: String,
    /// Command description
    pub description: String,
    /// Command arguments
    pub args: Vec<ArgSchema>,
    /// Nested subcommands (if any)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<CommandSchema>,
}

/// Schema for a single argument
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArgSchema {
    /// Argument name (e.g., "in", "out")
    pub name: String,
    /// Whether this argument is required
    pub required: bool,
    /// Value type hint (e.g., "PATH", "STRING", "NUMBER")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_type: Option<String>,
    /// Description
    pub description: String,
    /// Default value (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Whether this is a flag (boolean)
    pub is_flag: bool,
    /// Possible values (for enums)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
    /// Short form (e.g., "-q" for quiet)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<String>,
    /// Long form (e.g., "--quiet")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long: Option<String>,
}

/// Extract schema from a clap Command
pub fn extract_schema(cmd: &Command) -> CliSchema {
    CliSchema {
        name: cmd.get_name().to_string(),
        version: cmd.get_version().unwrap_or("unknown").to_string(),
        description: cmd
            .get_about()
            .map(|s| s.to_string())
            .unwrap_or_default(),
        global_args: extract_global_args(cmd),
        commands: extract_subcommands(cmd),
    }
}

/// Extract global arguments from the root command
fn extract_global_args(cmd: &Command) -> Vec<ArgSchema> {
    cmd.get_arguments()
        .filter(|arg| arg.is_global_set())
        .map(extract_arg_schema)
        .collect()
}

/// Extract subcommands recursively
fn extract_subcommands(cmd: &Command) -> Vec<CommandSchema> {
    cmd.get_subcommands()
        .filter(|sub| !sub.is_hide_set()) // Skip hidden commands
        .map(|sub| CommandSchema {
            name: sub.get_name().to_string(),
            description: sub
                .get_about()
                .map(|s| s.to_string())
                .unwrap_or_default(),
            args: sub.get_arguments().map(extract_arg_schema).collect(),
            subcommands: extract_subcommands(sub),
        })
        .collect()
}

/// Extract schema from a single Arg
fn extract_arg_schema(arg: &Arg) -> ArgSchema {
    let is_flag = matches!(arg.get_action(), ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count);

    ArgSchema {
        name: arg.get_id().to_string(),
        required: arg.is_required_set(),
        value_type: arg.get_value_names().map(|names| {
            names
                .iter()
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }),
        description: arg.get_help().map(|s| s.to_string()).unwrap_or_default(),
        default: arg.get_default_values().first().map(|v| v.to_string_lossy().to_string()),
        is_flag,
        possible_values: arg
            .get_possible_values()
            .iter()
            .filter(|v| !v.is_hide_set())
            .map(|v| v.get_name().to_string())
            .collect(),
        short: arg.get_short().map(|c| format!("-{}", c)),
        long: arg.get_long().map(|s| format!("--{}", s)),
    }
}

/// Generate a compact usage example for a command
pub fn generate_usage(schema: &CommandSchema) -> String {
    let mut parts = vec![schema.name.clone()];

    for arg in &schema.args {
        if arg.name == "help" || arg.name == "version" {
            continue;
        }

        let arg_str = if let Some(ref long) = arg.long {
            if arg.is_flag {
                format!("[{}]", long)
            } else if let Some(ref vt) = arg.value_type {
                if arg.required {
                    format!("{} <{}>", long, vt)
                } else {
                    format!("[{} <{}>]", long, vt)
                }
            } else {
                format!("[{}]", long)
            }
        } else {
            format!("<{}>", arg.name)
        };

        parts.push(arg_str);
    }

    parts.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::{Arg, Command};

    fn test_command() -> Command {
        Command::new("test")
            .version("1.0.0")
            .about("Test command")
            .arg(
                Arg::new("input")
                    .long("in")
                    .value_name("PATH")
                    .required(true)
                    .help("Input file"),
            )
            .arg(
                Arg::new("verbose")
                    .short('v')
                    .long("verbose")
                    .action(ArgAction::SetTrue)
                    .help("Enable verbose output"),
            )
            .subcommand(
                Command::new("sub")
                    .about("A subcommand")
                    .arg(Arg::new("name").help("Name argument")),
            )
    }

    #[test]
    fn test_extract_schema() {
        let cmd = test_command();
        let schema = extract_schema(&cmd);

        assert_eq!(schema.name, "test");
        assert_eq!(schema.version, "1.0.0");
        assert!(!schema.commands.is_empty());

        let sub = &schema.commands[0];
        assert_eq!(sub.name, "sub");
    }

    #[test]
    fn test_extract_arg_schema() {
        let arg = Arg::new("test")
            .long("test")
            .short('t')
            .value_name("VALUE")
            .required(true)
            .help("Test argument");

        let schema = extract_arg_schema(&arg);

        assert_eq!(schema.name, "test");
        assert!(schema.required);
        assert_eq!(schema.long, Some("--test".to_string()));
        assert_eq!(schema.short, Some("-t".to_string()));
    }

    #[test]
    fn test_generate_usage() {
        let schema = CommandSchema {
            name: "full-extract".to_string(),
            description: "Extract stuff".to_string(),
            args: vec![
                ArgSchema {
                    name: "in".to_string(),
                    required: true,
                    value_type: Some("PATH".to_string()),
                    description: "Input".to_string(),
                    default: None,
                    is_flag: false,
                    possible_values: vec![],
                    short: None,
                    long: Some("--in".to_string()),
                },
                ArgSchema {
                    name: "dry-run".to_string(),
                    required: false,
                    value_type: None,
                    description: "Dry run".to_string(),
                    default: None,
                    is_flag: true,
                    possible_values: vec![],
                    short: None,
                    long: Some("--dry-run".to_string()),
                },
            ],
            subcommands: vec![],
        };

        let usage = generate_usage(&schema);
        assert!(usage.contains("--in <PATH>"));
        assert!(usage.contains("[--dry-run]"));
    }
}

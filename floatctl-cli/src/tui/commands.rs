//! Scratch pane command parsing
//!
//! Parses commands from scratch pane input. Commands start with `/`.
//!
//! Supported commands:
//! - `/search <query>` - Search across all sources
//! - `/board <name>` - Navigate to a specific board
//! - `/rag <query>` - RAG search (scoped to active context)
//! - `/open <path>` - Open file/folder in filesystem view
//! - `/home` - Switch to home tab
//! - `/boards` - Switch to boards tab
//! - `/dash` - Switch to dashboard tab
//! - `/help` - Show help

use crate::tui::app::MainTab;

/// A parsed command from scratch input
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    /// Search across sources
    Search { query: String },
    /// Navigate to a board by name
    Board { name: String },
    /// RAG search with context scoping
    Rag { query: String },
    /// Open a file or folder
    Open { path: String },
    /// Switch to a tab
    Tab(MainTab),
    /// Show help
    Help,
    /// Clear scratch content
    Clear,
    /// Quit the application
    Quit,
    /// Unknown command
    Unknown { cmd: String, args: String },
}

/// Result of parsing scratch content
#[derive(Debug, Clone)]
pub enum ParseResult {
    /// A valid command was found
    Command(Command),
    /// Not a command (regular text or empty)
    Text(String),
    /// Incomplete command (ends with / but no command yet)
    Incomplete,
}

/// Parse scratch content to extract commands
pub fn parse_scratch(input: &str) -> ParseResult {
    let trimmed = input.trim();

    // Check if it's a command (starts with /)
    if !trimmed.starts_with('/') {
        return ParseResult::Text(trimmed.to_string());
    }

    // Remove leading / and parse
    let without_slash = &trimmed[1..];

    if without_slash.is_empty() {
        return ParseResult::Incomplete;
    }

    // Split into command and args
    let parts: Vec<&str> = without_slash.splitn(2, ' ').collect();
    let cmd = parts[0].to_lowercase();
    let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

    let command = match cmd.as_str() {
        "search" | "s" | "find" | "f" => {
            if args.is_empty() {
                Command::Search { query: String::new() }
            } else {
                Command::Search { query: args.to_string() }
            }
        }
        "board" | "b" => {
            if args.is_empty() {
                Command::Tab(MainTab::Boards)
            } else {
                Command::Board { name: args.to_string() }
            }
        }
        "rag" | "r" | "ask" => {
            Command::Rag { query: args.to_string() }
        }
        "open" | "o" | "go" => {
            Command::Open { path: args.to_string() }
        }
        "home" | "h" => Command::Tab(MainTab::Home),
        "boards" => Command::Tab(MainTab::Boards),
        "dash" | "dashboard" | "d" => Command::Tab(MainTab::Dashboard),
        "help" | "?" => Command::Help,
        "clear" | "c" => Command::Clear,
        "quit" | "q" | "exit" => Command::Quit,
        _ => Command::Unknown {
            cmd: cmd.to_string(),
            args: args.to_string(),
        },
    };

    ParseResult::Command(command)
}

/// Get command completions for partial input
pub fn get_completions(partial: &str) -> Vec<&'static str> {
    let commands = [
        "/search", "/board", "/rag", "/open",
        "/home", "/boards", "/dash",
        "/help", "/clear", "/quit",
    ];

    if partial.is_empty() || partial == "/" {
        return commands.to_vec();
    }

    let partial_lower = partial.to_lowercase();
    commands
        .into_iter()
        .filter(|cmd| cmd.to_lowercase().starts_with(&partial_lower))
        .collect()
}

/// Get help text for commands
pub fn get_help_text() -> &'static str {
    r#"Available Commands:
  /search <query>  - Search across all sources
  /board <name>    - Navigate to a board (or list boards)
  /rag <query>     - RAG search with context scoping
  /open <path>     - Open file or folder

Navigation:
  /home            - Switch to Home tab
  /boards          - Switch to Boards tab
  /dash            - Switch to Dashboard tab

Utility:
  /help            - Show this help
  /clear           - Clear scratch content
  /quit            - Quit the application

Shortcuts: /s=search, /b=board, /r=rag, /o=open, /h=home, /d=dash"#
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_search() {
        match parse_scratch("/search test query") {
            ParseResult::Command(Command::Search { query }) => {
                assert_eq!(query, "test query");
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_parse_shortcut() {
        match parse_scratch("/s test") {
            ParseResult::Command(Command::Search { query }) => {
                assert_eq!(query, "test");
            }
            _ => panic!("Expected Search command"),
        }
    }

    #[test]
    fn test_parse_tab() {
        match parse_scratch("/home") {
            ParseResult::Command(Command::Tab(MainTab::Home)) => {}
            _ => panic!("Expected Home tab"),
        }
    }

    #[test]
    fn test_parse_non_command() {
        match parse_scratch("just some text") {
            ParseResult::Text(t) => assert_eq!(t, "just some text"),
            _ => panic!("Expected Text"),
        }
    }

    #[test]
    fn test_parse_incomplete() {
        match parse_scratch("/") {
            ParseResult::Incomplete => {}
            _ => panic!("Expected Incomplete"),
        }
    }
}

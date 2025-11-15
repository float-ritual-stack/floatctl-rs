use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

use crate::block::BoardId;
use crate::db::BlockStore;
use crate::mode::{AppMode, Pane};

/// Main application state
pub struct App {
    /// Current mode
    pub mode: AppMode,

    /// Which pane has focus
    pub focused_pane: Pane,

    /// Current board being viewed
    pub current_board: BoardId,

    /// Command input buffer
    pub command_input: String,

    /// Status message (shown in command bar)
    pub status_message: Option<String>,

    /// Should quit?
    pub should_quit: bool,

    /// Block store
    pub store: BlockStore,
}

impl App {
    /// Create a new App
    pub fn new(store: BlockStore) -> Self {
        Self {
            mode: AppMode::Normal,
            focused_pane: Pane::Scratch,
            current_board: BoardId::Recent,
            command_input: String::new(),
            status_message: None,
            should_quit: false,
            store,
        }
    }

    /// Handle keyboard input
    pub async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        match self.mode {
            AppMode::Normal => self.handle_normal_mode(key).await?,
            AppMode::Insert => self.handle_insert_mode(key).await?,
            AppMode::Command => self.handle_command_mode(key).await?,
            AppMode::BoardNav => self.handle_board_nav_mode(key).await?,
        }
        Ok(())
    }

    /// Handle normal mode keys
    async fn handle_normal_mode(&mut self, key: KeyEvent) -> Result<()> {
        match (key.code, key.modifiers) {
            // Quit
            (KeyCode::Char('q'), KeyModifiers::NONE) => {
                self.should_quit = true;
            }

            // Enter insert mode
            (KeyCode::Char('i'), KeyModifiers::NONE) => {
                self.mode = AppMode::Insert;
                self.focused_pane = Pane::Scratch; // Always focus scratch panel in insert mode
                self.status_message = Some("-- INSERT --".to_string());
            }

            // Enter command mode
            (KeyCode::Char(':'), KeyModifiers::NONE) => {
                self.mode = AppMode::Command;
                self.command_input.clear();
            }

            // Enter board nav mode
            (KeyCode::Char('b'), KeyModifiers::NONE) => {
                self.mode = AppMode::BoardNav;
                self.status_message = Some("Select board: (w)ork (t)ech (l)ife-admin (r)ecent (s)cratch".to_string());
            }

            // Toggle pane focus
            (KeyCode::Tab, KeyModifiers::NONE) => {
                self.focused_pane = self.focused_pane.toggle();
            }

            // Refresh
            (KeyCode::Char('r'), KeyModifiers::NONE) => {
                self.status_message = Some("Refreshed".to_string());
            }

            _ => {}
        }
        Ok(())
    }

    /// Handle insert mode keys
    async fn handle_insert_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Exit insert mode
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.status_message = None;
            }

            // Save entry (Ctrl-s)
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.status_message = Some("Saved (TODO: implement save)".to_string());
            }

            _ => {
                // Let tui-textarea handle other keys
            }
        }
        Ok(())
    }

    /// Handle command mode keys
    async fn handle_command_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            // Cancel command
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.command_input.clear();
            }

            // Execute command
            KeyCode::Enter => {
                self.execute_command().await?;
                self.mode = AppMode::Normal;
                self.command_input.clear();
            }

            // Backspace
            KeyCode::Backspace => {
                self.command_input.pop();
            }

            // Type characters
            KeyCode::Char(c) => {
                self.command_input.push(c);
            }

            _ => {}
        }
        Ok(())
    }

    /// Handle board nav mode keys
    async fn handle_board_nav_mode(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.mode = AppMode::Normal;
                self.status_message = None;
            }

            KeyCode::Char('w') => {
                self.current_board = BoardId::Work;
                self.mode = AppMode::Normal;
                self.status_message = Some("Switched to /work/".to_string());
            }

            KeyCode::Char('t') => {
                self.current_board = BoardId::Tech;
                self.mode = AppMode::Normal;
                self.status_message = Some("Switched to /tech/".to_string());
            }

            KeyCode::Char('l') => {
                self.current_board = BoardId::LifeAdmin;
                self.mode = AppMode::Normal;
                self.status_message = Some("Switched to /life-admin/".to_string());
            }

            KeyCode::Char('r') => {
                self.current_board = BoardId::Recent;
                self.mode = AppMode::Normal;
                self.status_message = Some("Switched to /recent/".to_string());
            }

            KeyCode::Char('s') => {
                self.current_board = BoardId::Scratch;
                self.mode = AppMode::Normal;
                self.status_message = Some("Switched to /scratch/".to_string());
            }

            _ => {}
        }
        Ok(())
    }

    /// Execute a command
    async fn execute_command(&mut self) -> Result<()> {
        let cmd = self.command_input.trim();

        match cmd {
            "q" | "quit" => {
                self.should_quit = true;
            }

            "work" | "w" => {
                self.current_board = BoardId::Work;
                self.status_message = Some("Switched to /work/".to_string());
            }

            "tech" | "t" => {
                self.current_board = BoardId::Tech;
                self.status_message = Some("Switched to /tech/".to_string());
            }

            "life-admin" | "life" | "l" => {
                self.current_board = BoardId::LifeAdmin;
                self.status_message = Some("Switched to /life-admin/".to_string());
            }

            "recent" | "r" => {
                self.current_board = BoardId::Recent;
                self.status_message = Some("Switched to /recent/".to_string());
            }

            "scratch" | "s" => {
                self.current_board = BoardId::Scratch;
                self.status_message = Some("Switched to /scratch/".to_string());
            }

            "" => {}

            _ => {
                self.status_message = Some(format!("Unknown command: {}", cmd));
            }
        }

        Ok(())
    }

    /// Poll for events with timeout
    pub fn poll_event(timeout: Duration) -> Result<Option<Event>> {
        if event::poll(timeout)? {
            Ok(Some(event::read()?))
        } else {
            Ok(None)
        }
    }
}
